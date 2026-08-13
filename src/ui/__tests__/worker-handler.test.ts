import { describe, expect, test } from "bun:test";
import type { ExactDeckSearchResult } from "../solver-contract";
import { defaultBattleConfig, defaultPlayerConfig } from "../data";
import { createWorkbenchWorkerHandler } from "../worker-handler";
import type {
  WorkbenchSolvePayload,
  WorkbenchWorkerRequest,
} from "../worker-protocol";
import type { FixtureConsistencyReport } from "../fixture-consistency";
import type { OriginalReplayFixture } from "../domain";
import { simulationResult } from "./layout-test-helpers";

describe("workbench Worker handler", () => {
  test("按 discriminant 路由战斗并在 Worker 内生成 fixture 一致性", async () => {
    const result = simulationResult([]);
    const report = consistencyReport();
    const calls: string[] = [];
    const handler = createWorkbenchWorkerHandler({
      simulate: (config) => {
        calls.push(`simulate:${config.gameRound}`);
        return result;
      },
      compareFixture: (_fixture, compared) => {
        calls.push(`compare:${compared.actionCount}`);
        return report;
      },
      solve: () => solverResult("order"),
    });

    const response = await handler({
      type: "simulate",
      requestId: "battle-7",
      payload: {
        config: defaultBattleConfig(),
        comparisonFixture: {} as OriginalReplayFixture,
      },
    } satisfies WorkbenchWorkerRequest);

    expect(response).toEqual({
      type: "simulate-success",
      requestId: "battle-7",
      result,
      fixtureConsistency: report,
    });
    expect(calls).toEqual(["simulate:16", "compare:0"]);
  });

  test("三种 Rust solver 任务都路由到同一 canonical 依赖并保留 requestId", async () => {
    const calls: string[] = [];
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: (payload) => record(payload.mode),
    });
    const modes = ["order", "hand", "pool"] as const;
    for (const mode of modes) {
      const response = await handler({
        type: "solve",
        requestId: `solve-${mode}`,
        payload: solvePayload(mode),
      });
      expect(response.type).toBe("solve-success");
      expect(response.requestId).toBe(`solve-${mode}`);
    }
    expect(calls).toEqual([...modes]);

    function record(mode: typeof modes[number]): ExactDeckSearchResult {
      calls.push(mode);
      return solverResult("order");
    }
  });

  test("卡组诊断经 Worker 独立路由", async () => {
    const handler = createWorkbenchWorkerHandler({
      diagnose: () => ({
        configSignature: "test",
        sides: {
          p1: { side: "p1", configuredCount: 0, effectiveCount: 0, issues: [{ kind: "不足8张", side: "p1", detail: "当前 0/8" }], simulatable: false },
          p2: { side: "p2", configuredCount: 0, effectiveCount: 0, issues: [{ kind: "不足8张", side: "p2", detail: "当前 0/8" }], simulatable: false },
        },
        issueCount: 2,
        simulatable: false,
      }),
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
    });
    const response = await handler({
      type: "diagnose",
      requestId: "diagnose-1",
      payload: { config: defaultBattleConfig() },
    });
    expect(response.type).toBe("diagnose-success");
    if (response.type === "diagnose-success") {
      expect(response.result.issueCount).toBe(2);
      expect(response.result.sides.p1.issues[0]?.kind).toBe("不足8张");
    }
  });

  test("原版 .bin 解码经 Worker 独立路由", async () => {
    const handler = createWorkbenchWorkerHandler({
      decodeReplay: (bytes) => ({
        gameVersion: `bytes-${bytes.byteLength}`,
        beginTimestamp: null,
        endTimestamp: null,
        recordCodes: [],
        rounds: [],
      }),
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
    });
    const response = await handler({
      type: "decode-replay",
      requestId: "decode-1",
      payload: { bytes: new Uint8Array([1, 2, 3]) },
    });
    expect(response).toEqual({
      type: "decode-replay-success",
      requestId: "decode-1",
      result: {
        gameVersion: "bytes-3",
        beginTimestamp: null,
        endTimestamp: null,
        recordCodes: [],
        rounds: [],
      },
    });
  });

  test("异常只返回稳定 failure，不伪造战斗结果", async () => {
    const handler = createWorkbenchWorkerHandler({
      simulate: () => { throw new Error("invalid deck"); },
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
    });
    const response = await handler({
      type: "simulate",
      requestId: "battle-fail",
      payload: { config: defaultBattleConfig() },
    });

    expect(response).toEqual({
      type: "failure",
      requestId: "battle-fail",
      operation: "simulate",
      error: {
        kind: "simulation-failed",
        message: "战斗模拟失败：invalid deck",
      },
    });
    expect("result" in response).toBe(false);
  });

  test("非法请求返回同 requestId 的 protocol failure", async () => {
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
    });
    const response = await handler({ type: "unknown", requestId: "bad-1" });
    expect(response).toEqual({
      type: "failure",
      requestId: "bad-1",
      operation: "protocol",
      error: { kind: "invalid-request", message: "Worker 请求格式无效" },
    });
  });

  test("成功响应但空 trace steps 走稳定 no-trace failure，不生成 0 伤结果", async () => {
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
      targetPractice: () => ({ frames: [], hookSteps: [] }),
    });
    const response = await handler({
      type: "target-practice",
      requestId: "target-empty-trace",
      payload: {
        buildId: "b1",
        build: defaultPlayerConfig("p1", 4_000_004, 16),
        gameRound: 16,
      },
    } satisfies WorkbenchWorkerRequest);
    expect(response).toEqual({
      type: "failure",
      requestId: "target-empty-trace",
      operation: "target-practice",
      error: {
        kind: "simulation-failed",
        message: "打靶模拟失败：钩子链不可用：打靶伤害归因需要 yixian_trace_json 数据",
      },
    });
  });

  test("打靶请求经 Worker 独立路由：构造 fixture 后调 targetPractice 依赖并回传 buildId", async () => {
    const calls: { readonly maxActorTurns: number; readonly firstPlayerSide: string; readonly gameRound: number }[] = [];
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
      targetPractice: (config) => {
        calls.push({
          maxActorTurns: config.maxActorTurns,
          firstPlayerSide: config.firstPlayerSide,
          gameRound: config.gameRound,
        });
        return { frames: [], hookSteps: [{
          frameIndex: 0,
          category: "battleStart",
          categoryLabel: "战斗开始",
          actorTurn: 0,
          actor: "p1",
          slot: null,
          cardId: null,
          cardName: null,
          changes: [],
          attackSegments: [],
        }] };
      },
    });
    const response = await handler({
      type: "target-practice",
      requestId: "target-1",
      payload: {
        buildId: "b1",
        build: defaultPlayerConfig("p1", 4_000_004, 16),
        gameRound: 16,
      },
    } satisfies WorkbenchWorkerRequest);
    expect(response.type).toBe("target-practice-success");
    if (response.type === "target-practice-success") {
      expect(response.result.buildId).toBe("b1");
      expect(response.result.frames).toEqual([]);
    }
    // fixture：maxActorTurns = 32*2，我方先手。
    expect(calls[0]?.maxActorTurns).toBe(64);
    expect(calls[0]?.firstPlayerSide).toBe("p1");
    expect(calls[0]?.gameRound).toBe(16);
  });

  test("打靶失败只返回稳定 failure：operation target-practice、kind simulation-failed", async () => {
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
      targetPractice: () => { throw new Error("boom"); },
    });
    const response = await handler({
      type: "target-practice",
      requestId: "target-fail",
      payload: {
        buildId: "b1",
        build: defaultPlayerConfig("p1", 4_000_004, 16),
        gameRound: 16,
      },
    });
    expect(response).toEqual({
      type: "failure",
      requestId: "target-fail",
      operation: "target-practice",
      error: { kind: "simulation-failed", message: "打靶模拟失败：boom" },
    });
    expect("result" in response).toBe(false);
  });

  test("非法打靶请求（空 buildId）返回 protocol failure", async () => {
    const handler = createWorkbenchWorkerHandler({
      simulate: () => simulationResult([]),
      compareFixture: () => consistencyReport(),
      solve: () => solverResult("order"),
    });
    const response = await handler({
      type: "target-practice",
      requestId: "target-bad",
      payload: { buildId: "", build: {}, gameRound: 16 },
    } as unknown as WorkbenchWorkerRequest);
    expect(response.type).toBe("failure");
    if (response.type === "failure") {
      expect(response.error.kind).toBe("invalid-request");
      expect(response.operation).toBe("protocol");
    }
  });
});

function solvePayload(mode: WorkbenchSolvePayload["mode"]): WorkbenchSolvePayload {
  const base = {
    fixture: {} as OriginalReplayFixture,
    side: "p1" as const,
    scoring: { scoreProfile: "value-v0" as const },
    topN: 5,
    maxEvaluations: 10,
    visitOrder: "canonical" as const,
    visitSeed: 7,
  };
  return { ...base, mode };
}

function solverResult(mode: ExactDeckSearchResult["mode"]): ExactDeckSearchResult {
  return { mode, evaluatedCount: 0 } as ExactDeckSearchResult;
}

function consistencyReport(): FixtureConsistencyReport {
  const summary = {
    winnerSide: "p1" as const,
    actorTurnCount: 1,
    hpDeltaP1MinusP2: 1,
    finalHp: { p1: 2, p2: 1 },
  };
  return { engine: summary, ui: summary, engineMatch: true, expectedMatch: true };
}
