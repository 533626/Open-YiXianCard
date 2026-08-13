import { describe, expect, test } from "bun:test";
import type { ExactDeckSearchResult } from "../solver-contract";
import { defaultBattleConfig, defaultPlayerConfig } from "../data";
import {
  DEV_WORKBENCH_WORKER_URL,
  WorkbenchWorkerClient,
  WorkbenchWorkerClientError,
  resolveWorkbenchWorkerUrl,
  type WorkbenchWorkerLike,
} from "../worker-client";
import type {
  WorkbenchSolvePayload,
  WorkbenchWorkerRequest,
  WorkbenchWorkerResponse,
} from "../worker-protocol";
import type { OriginalReplayFixture } from "../domain";
import { simulationResult } from "./layout-test-helpers";

class FakeWorker implements WorkbenchWorkerLike {
  readonly messages: WorkbenchWorkerRequest[] = [];
  terminated = false;
  private readonly messageListeners: Array<(event: { data: unknown }) => void> = [];
  private readonly errorListeners: Array<(event: { message?: string; preventDefault?(): void }) => void> = [];

  postMessage(message: WorkbenchWorkerRequest): void {
    this.messages.push(message);
  }

  terminate(): void {
    this.terminated = true;
  }

  addEventListener(type: "message" | "error", listener: ((event: never) => void)): void {
    if (type === "message") this.messageListeners.push(listener as (event: { data: unknown }) => void);
    else this.errorListeners.push(listener as (event: { message?: string }) => void);
  }

  respond(response: WorkbenchWorkerResponse): void {
    for (const listener of this.messageListeners) listener({ data: response });
  }

  crash(message: string): void {
    for (const listener of this.errorListeners) listener({ message });
  }
}

describe("WorkbenchWorkerClient", () => {
  test("开发 URL 使用固定 fallback", () => {
    expect(resolveWorkbenchWorkerUrl()).toBe(DEV_WORKBENCH_WORKER_URL);
  });

  test("按 requestId 接受乱序响应，忽略未知响应", async () => {
    const worker = new FakeWorker();
    const client = new WorkbenchWorkerClient(() => worker);
    const first = client.solve(solvePayload());
    const second = client.solve(solvePayload());
    const firstResult = solverResult(1);
    const secondResult = solverResult(2);

    worker.respond({
      type: "solve-success",
      requestId: "unknown",
      result: solverResult(99),
    });
    worker.respond({ type: "solve-success", requestId: second.requestId, result: secondResult });
    worker.respond({ type: "solve-success", requestId: first.requestId, result: firstResult });

    expect(await second.result).toBe(secondResult);
    expect(await first.result).toBe(firstResult);
    client.dispose();
  });

  test("原版回放解码结果按 requestId 返回", async () => {
    const worker = new FakeWorker();
    const client = new WorkbenchWorkerClient(() => worker);
    const task = client.decodeReplay(new Uint8Array([1, 2]));
    const result = {
      gameVersion: "test",
      beginTimestamp: null,
      endTimestamp: null,
      recordCodes: [],
      rounds: [],
    };
    worker.respond({
      type: "decode-replay-success",
      requestId: task.requestId,
      result,
    });
    expect(await task.result).toEqual(result);
    expect(worker.messages[0]?.type).toBe("decode-replay");
    client.dispose();
  });

  test("取消会 terminate 并立即重建，旧代结果不得回写", async () => {
    const workers: FakeWorker[] = [];
    const client = new WorkbenchWorkerClient(() => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    });
    const stale = client.simulate(defaultBattleConfig());
    const staleRejection = rejectionOf(stale.result);
    expect(client.cancelAll()).toBe(1);
    expect(workers).toHaveLength(2);
    expect(workers[0]?.terminated).toBe(true);

    workers[0]?.respond({
      type: "simulate-success",
      requestId: stale.requestId,
      result: simulationResult([]),
      fixtureConsistency: null,
    });
    const error = await staleRejection;
    expect(error).toBeInstanceOf(WorkbenchWorkerClientError);
    expect(error.kind).toBe("cancelled");

    const fresh = client.simulate(defaultBattleConfig());
    const freshResult = simulationResult([]);
    workers[1]?.respond({
      type: "simulate-success",
      requestId: fresh.requestId,
      result: freshResult,
      fixtureConsistency: null,
    });
    expect(await fresh.result).toEqual({ result: freshResult, fixtureConsistency: null });
    client.dispose();
  });

  test("Worker 不可用时明确报错且不执行同步 fallback", async () => {
    let factoryCalls = 0;
    const client = new WorkbenchWorkerClient(() => {
      factoryCalls += 1;
      throw new Error("blocked");
    });
    const task = client.simulate(defaultBattleConfig());
    const error = await rejectionOf(task.result);
    expect(error.kind).toBe("worker-unavailable");
    expect(error.message).toContain("无法启动计算 Worker");
    expect(factoryCalls).toBe(1);
  });

  test("Worker crash 拒绝当前请求并重建", async () => {
    const workers: FakeWorker[] = [];
    const client = new WorkbenchWorkerClient(() => {
      const worker = new FakeWorker();
      workers.push(worker);
      return worker;
    });
    const task = client.solve(solvePayload());
    const rejected = rejectionOf(task.result);
    workers[0]?.crash("boom");
    const error = await rejected;
    expect(error.kind).toBe("worker-crashed");
    expect(workers[0]?.terminated).toBe(true);
    expect(workers).toHaveLength(2);
    client.dispose();
  });

  test("打靶请求发出克隆的 build，结果按 buildId 回传", async () => {
    const worker = new FakeWorker();
    const client = new WorkbenchWorkerClient(() => worker);
    const build = defaultPlayerConfig("p1", 4_000_004, 16);
    const task = client.targetPractice({ buildId: "b1", build, gameRound: 16 });
    const sent = worker.messages[0]!;
    expect(sent.type).toBe("target-practice");
    if (sent.type === "target-practice") {
      expect(sent.payload.buildId).toBe("b1");
      expect(sent.payload.gameRound).toBe(16);
      // Detached clone：UI 编辑不能影响在途推演。
      build.deck[0] = { baseId: 99, level: 0 };
      expect(sent.payload.build.deck[0]?.baseId).toBe(0);
    }
    worker.respond({
      type: "target-practice-success",
      requestId: task.requestId,
      result: { buildId: "b1", frames: [], hookSteps: [] },
    });
    expect(await task.result).toEqual({ buildId: "b1", frames: [], hookSteps: [] });
    client.dispose();
  });

  test("Worker 空 trace failure 走打靶失败通道", async () => {
    const worker = new FakeWorker();
    const client = new WorkbenchWorkerClient(() => worker);
    const task = client.targetPractice({
      buildId: "b1",
      build: defaultPlayerConfig("p1", 4_000_004, 16),
      gameRound: 16,
    });
    const rejected = rejectionOf(task.result);
    worker.respond({
      type: "failure",
      requestId: task.requestId,
      operation: "target-practice",
      error: {
        kind: "simulation-failed",
        message: "打靶模拟失败：钩子链不可用：打靶伤害归因需要 yixian_trace_json 数据",
      },
    });
    const error = await rejected;
    expect(error.kind).toBe("simulation-failed");
    expect(error.operation).toBe("target-practice");
    client.dispose();
  });

  test("打靶失败与其它操作一样走 failure 拒绝通道", async () => {
    const worker = new FakeWorker();
    const client = new WorkbenchWorkerClient(() => worker);
    const task = client.targetPractice({
      buildId: "b1",
      build: defaultPlayerConfig("p1", 4_000_004, 16),
      gameRound: 16,
    });
    const rejected = rejectionOf(task.result);
    worker.respond({
      type: "failure",
      requestId: task.requestId,
      operation: "target-practice",
      error: { kind: "simulation-failed", message: "打靶模拟失败" },
    });
    const error = await rejected;
    expect(error.kind).toBe("simulation-failed");
    expect(error.operation).toBe("target-practice");
    client.dispose();
  });
});

function solvePayload(): WorkbenchSolvePayload {
  return {
    mode: "order",
    fixture: {} as OriginalReplayFixture,
    side: "p1",
    scoring: { scoreProfile: "value-v0" },
    topN: 5,
    maxEvaluations: 10,
    visitOrder: "canonical",
    visitSeed: 7,
  };
}

function solverResult(evaluatedCount: number): ExactDeckSearchResult {
  return { mode: "order", evaluatedCount } as ExactDeckSearchResult;
}

async function rejectionOf(promise: Promise<unknown>): Promise<WorkbenchWorkerClientError> {
  try {
    await promise;
  } catch (error) {
    return error as WorkbenchWorkerClientError;
  }
  throw new Error("expected promise rejection");
}
