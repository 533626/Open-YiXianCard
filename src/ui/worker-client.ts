import type { ExactDeckSearchResult } from "./solver-contract";
import type { BattleConfig, PlayerConfig, SimulationResult } from "./types";
import type { ReplayFixtureWithExpected } from "./fixture-contract";
import type {
  TargetPracticeOutcome,
  WorkbenchSolvePayload,
  WorkbenchWorkerErrorKind,
  WorkbenchWorkerOperation,
  WorkbenchWorkerRequest,
  WorkbenchWorkerResponse,
  WorkbenchSimulationOutcome,
} from "./worker-protocol";
import type { DeckDiagnosticResult } from "./deck-diagnostics";
import type { DecodedOriginalReplay } from "./original-replay-bin";

declare const __OPEN_YIXIAN_WORKBENCH_WORKER_URL__: string | undefined;

export const DEV_WORKBENCH_WORKER_URL = "/public/build/workbench-worker.js";

export interface WorkbenchWorkerLike {
  postMessage(message: WorkbenchWorkerRequest): void;
  terminate(): void;
  addEventListener(type: "message", listener: (event: { readonly data: unknown }) => void): void;
  addEventListener(type: "error", listener: (event: {
    readonly message?: string;
    preventDefault?(): void;
  }) => void): void;
}

export type WorkbenchWorkerFactory = () => WorkbenchWorkerLike;

export interface WorkbenchWorkerTask<T> {
  readonly requestId: string;
  readonly result: Promise<T>;
}

export class WorkbenchWorkerClientError extends Error {
  constructor(
    readonly kind: WorkbenchWorkerErrorKind,
    message: string,
    readonly operation: WorkbenchWorkerOperation,
  ) {
    super(message);
    this.name = "WorkbenchWorkerClientError";
  }
}

interface PendingRequest {
  readonly operation: Exclude<WorkbenchWorkerOperation, "protocol">;
  readonly generation: number;
  readonly resolve: (
    value: WorkbenchSimulationOutcome
      | ExactDeckSearchResult
      | DeckDiagnosticResult
      | DecodedOriginalReplay
      | TargetPracticeOutcome
  ) => void;
  readonly reject: (reason: WorkbenchWorkerClientError) => void;
}

export class WorkbenchWorkerClient {
  private worker: WorkbenchWorkerLike | null = null;
  private generation = 0;
  private nextRequestId = 1;
  private readonly pending = new Map<string, PendingRequest>();
  private disposed = false;

  constructor(private readonly factory: WorkbenchWorkerFactory = defaultWorkbenchWorkerFactory) {}

  simulate(
    config: BattleConfig,
    comparisonFixture?: ReplayFixtureWithExpected,
  ): WorkbenchWorkerTask<WorkbenchSimulationOutcome> {
    return this.start<WorkbenchSimulationOutcome>({
      type: "simulate",
      requestId: this.createRequestId(),
      payload: {
        config,
        ...(comparisonFixture ? { comparisonFixture } : {}),
      },
    }, "simulate");
  }

  solve(payload: WorkbenchSolvePayload): WorkbenchWorkerTask<ExactDeckSearchResult> {
    return this.start<ExactDeckSearchResult>({
      type: "solve",
      requestId: this.createRequestId(),
      payload,
    }, "solve");
  }

  diagnose(config: BattleConfig): WorkbenchWorkerTask<DeckDiagnosticResult> {
    return this.start<DeckDiagnosticResult>({
      type: "diagnose",
      requestId: this.createRequestId(),
      payload: { config },
    }, "diagnose");
  }

  decodeReplay(bytes: Uint8Array): WorkbenchWorkerTask<DecodedOriginalReplay> {
    return this.start<DecodedOriginalReplay>({
      type: "decode-replay",
      requestId: this.createRequestId(),
      payload: { bytes },
    }, "decode-replay");
  }

  /** 打靶推演：payload 中的 build 是脱离 UI 的克隆，编辑不能影响在途推演。 */
  targetPractice(payload: {
    readonly buildId: string;
    readonly build: PlayerConfig;
    readonly gameRound: number;
  }): WorkbenchWorkerTask<TargetPracticeOutcome> {
    return this.start<TargetPracticeOutcome>({
      type: "target-practice",
      requestId: this.createRequestId(),
      payload: {
        ...payload,
        build: structuredClone(payload.build),
      },
    }, "target-practice");
  }

  cancelAll(message = "操作已取消"): number {
    const pending = [...this.pending.values()];
    this.pending.clear();
    this.replaceWorker(true);
    for (const request of pending) {
      request.reject(new WorkbenchWorkerClientError("cancelled", message, request.operation));
    }
    return pending.length;
  }

  dispose(): void {
    if (this.disposed) return;
    this.disposed = true;
    const pending = [...this.pending.values()];
    this.pending.clear();
    this.worker?.terminate();
    this.worker = null;
    this.generation += 1;
    for (const request of pending) {
      request.reject(new WorkbenchWorkerClientError("cancelled", "Worker client 已关闭", request.operation));
    }
  }

  private start<T extends
    | WorkbenchSimulationOutcome
    | ExactDeckSearchResult
    | DeckDiagnosticResult
    | DecodedOriginalReplay
    | TargetPracticeOutcome>(
    request: WorkbenchWorkerRequest,
    operation: PendingRequest["operation"],
  ): WorkbenchWorkerTask<T> {
    let resolvePromise!: (value: T) => void;
    let rejectPromise!: (reason: WorkbenchWorkerClientError) => void;
    const result = new Promise<T>((resolve, reject) => {
      resolvePromise = resolve;
      rejectPromise = reject;
    });
    const pending: PendingRequest = {
      operation,
      generation: this.generation,
      resolve: (value) => resolvePromise(value as T),
      reject: rejectPromise,
    };

    try {
      const worker = this.ensureWorker();
      const generation = this.generation;
      this.pending.set(request.requestId, { ...pending, generation });
      worker.postMessage(request);
    } catch (error) {
      this.pending.delete(request.requestId);
      rejectPromise(asClientError(error, operation));
    }

    return { requestId: request.requestId, result };
  }

  private ensureWorker(): WorkbenchWorkerLike {
    if (this.disposed) {
      throw new WorkbenchWorkerClientError("worker-unavailable", "Worker client 已关闭", "protocol");
    }
    if (this.worker) return this.worker;
    try {
      const worker = this.factory();
      const generation = this.generation;
      worker.addEventListener("message", (event) => this.handleMessage(event.data, generation));
      worker.addEventListener("error", (event) => {
        event.preventDefault?.();
        this.handleWorkerCrash(event.message ?? "Worker 运行失败", generation);
      });
      this.worker = worker;
      return worker;
    } catch (error) {
      throw new WorkbenchWorkerClientError(
        "worker-unavailable",
        `当前环境无法启动计算 Worker：${errorMessage(error)}`,
        "protocol",
      );
    }
  }

  private handleMessage(value: unknown, generation: number): void {
    if (generation !== this.generation || !isWorkbenchWorkerResponse(value)) return;
    const pending = this.pending.get(value.requestId);
    if (!pending || pending.generation !== generation) return;
    this.pending.delete(value.requestId);

    if (value.type === "failure") {
      pending.reject(new WorkbenchWorkerClientError(
        value.error.kind,
        value.error.message,
        value.operation,
      ));
      return;
    }
    if (
      (pending.operation === "simulate" && value.type !== "simulate-success") ||
      (pending.operation === "solve" && value.type !== "solve-success") ||
      (pending.operation === "diagnose" && value.type !== "diagnose-success") ||
      (pending.operation === "decode-replay" && value.type !== "decode-replay-success") ||
      (pending.operation === "target-practice" && value.type !== "target-practice-success")
    ) {
      pending.reject(new WorkbenchWorkerClientError(
        "invalid-response",
        "Worker 返回了与请求类型不匹配的结果",
        pending.operation,
      ));
      return;
    }
    pending.resolve(value.type === "simulate-success"
      ? {
        result: value.result,
        fixtureConsistency: value.fixtureConsistency,
      }
      : value.result);
  }

  private handleWorkerCrash(message: string, generation: number): void {
    if (generation !== this.generation) return;
    const affected = [...this.pending.entries()]
      .filter(([, request]) => request.generation === generation);
    for (const [requestId] of affected) this.pending.delete(requestId);
    this.replaceWorker(true);
    for (const [, request] of affected) {
      request.reject(new WorkbenchWorkerClientError(
        "worker-crashed",
        `计算 Worker 异常退出：${message}`,
        request.operation,
      ));
    }
  }

  private replaceWorker(recreate: boolean): void {
    this.worker?.terminate();
    this.worker = null;
    this.generation += 1;
    if (!recreate || this.disposed) return;
    try {
      this.ensureWorker();
    } catch {
      // The next request reports a stable worker-unavailable error.
    }
  }

  private createRequestId(): string {
    const id = this.nextRequestId;
    this.nextRequestId += 1;
    return `workbench-${id}`;
  }
}

export function resolveWorkbenchWorkerUrl(): string {
  return typeof __OPEN_YIXIAN_WORKBENCH_WORKER_URL__ === "string" &&
      __OPEN_YIXIAN_WORKBENCH_WORKER_URL__.length > 0
    ? __OPEN_YIXIAN_WORKBENCH_WORKER_URL__
    : DEV_WORKBENCH_WORKER_URL;
}

export function defaultWorkbenchWorkerFactory(): WorkbenchWorkerLike {
  if (typeof Worker !== "function") {
    throw new Error("此浏览器不支持 Web Worker");
  }
  return new Worker(resolveWorkbenchWorkerUrl(), {
    type: "module",
    name: "open-yixiancard-workbench",
  }) as unknown as WorkbenchWorkerLike;
}

export const workbenchWorkerClient = new WorkbenchWorkerClient();

function asClientError(
  error: unknown,
  operation: PendingRequest["operation"],
): WorkbenchWorkerClientError {
  if (error instanceof WorkbenchWorkerClientError) {
    return new WorkbenchWorkerClientError(error.kind, error.message, operation);
  }
  return new WorkbenchWorkerClientError(
    "worker-unavailable",
    `当前环境无法启动计算 Worker：${errorMessage(error)}`,
    operation,
  );
}

function isWorkbenchWorkerResponse(value: unknown): value is WorkbenchWorkerResponse {
  if (!value || typeof value !== "object") return false;
  const response = value as Partial<WorkbenchWorkerResponse>;
  if (typeof response.requestId !== "string") return false;
  if (
    response.type === "simulate-success" ||
    response.type === "solve-success" ||
    response.type === "diagnose-success" ||
    response.type === "decode-replay-success" ||
    response.type === "target-practice-success"
  ) {
    if (response.result === null || typeof response.result !== "object") return false;
    return response.type !== "simulate-success" ||
      response.fixtureConsistency === null ||
      (response.fixtureConsistency !== undefined && typeof response.fixtureConsistency === "object");
  }
  if (response.type !== "failure" || !response.error || typeof response.error !== "object") return false;
  const error = response.error as { readonly kind?: unknown; readonly message?: unknown };
  return typeof error.kind === "string" && typeof error.message === "string";
}

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  const message = String(error).trim();
  return message || "未知错误";
}
