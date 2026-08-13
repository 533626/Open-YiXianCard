import {
  runRustEngineSimulation,
  runRustSolver,
  runRustTargetPractice,
} from "./rust-wasm-engine";
import { compareRustFixtureResult } from "./rust-fixture-consistency";
import { buildTargetPracticeFixture } from "./target-dummy";
import type {
  TargetPracticeOutcome,
  WorkbenchSolvePayload,
  WorkbenchWorkerErrorKind,
  WorkbenchWorkerOperation,
  WorkbenchWorkerRequest,
  WorkbenchWorkerResponse,
} from "./worker-protocol";
import type {
  BattleConfig,
  SimulationResult,
} from "./types";
import type { ExactDeckSearchResult } from "./solver-contract";
import type { ReplayFixtureWithExpected } from "./fixture-contract";
import type { FixtureConsistencyReport } from "./fixture-consistency";
import { diagnoseBattleConfig, type DeckDiagnosticResult } from "./deck-diagnostics";
import {
  decodeOriginalReplayBin,
  type DecodedOriginalReplay,
} from "./original-replay-bin";

export interface WorkbenchWorkerDependencies {
  readonly diagnose?: (config: Extract<WorkbenchWorkerRequest, { type: "diagnose" }>["payload"]["config"]) => DeckDiagnosticResult;
  readonly decodeReplay?: (bytes: Uint8Array) => DecodedOriginalReplay;
  readonly simulate: (config: Extract<WorkbenchWorkerRequest, { type: "simulate" }>["payload"]["config"]) => SimulationResult | Promise<SimulationResult>;
  readonly compareFixture: (
    fixture: ReplayFixtureWithExpected,
    result: SimulationResult,
  ) => FixtureConsistencyReport;
  readonly solve: (payload: WorkbenchSolvePayload) => ExactDeckSearchResult | Promise<ExactDeckSearchResult>;
  /** 打靶推演：接收 buildTargetPracticeFixture 构造出的 BattleConfig（与 simulate
   * 的 payload 类型无关，避免意外耦合）；默认走 rust-wasm-engine 的强制 trace 通道。 */
  readonly targetPractice?: (
    config: BattleConfig,
  ) => Omit<TargetPracticeOutcome, "buildId"> | Promise<Omit<TargetPracticeOutcome, "buildId">>;
}

const DEFAULT_DEPENDENCIES: WorkbenchWorkerDependencies = {
  diagnose: diagnoseBattleConfig,
  decodeReplay: decodeOriginalReplayBin,
  simulate: runRustEngineSimulation,
  compareFixture: compareRustFixtureResult,
  solve: runRustSolver,
};

export function createWorkbenchWorkerHandler(
  dependencies: WorkbenchWorkerDependencies = DEFAULT_DEPENDENCIES,
): (request: unknown) => Promise<WorkbenchWorkerResponse> {
  return async (request) => {
    if (!isWorkbenchWorkerRequest(request)) {
      return failure(
        requestIdFrom(request),
        "protocol",
        "invalid-request",
        "Worker 请求格式无效",
      );
    }

    if (request.type === "simulate") {
      try {
        const result = await dependencies.simulate(request.payload.config);
        return {
          type: "simulate-success",
          requestId: request.requestId,
          result,
          fixtureConsistency: request.payload.comparisonFixture
            ? await dependencies.compareFixture(request.payload.comparisonFixture, result)
            : null,
        };
      } catch (error) {
        return failure(
          request.requestId,
          "simulate",
          "simulation-failed",
          `战斗模拟失败：${errorMessage(error)}`,
        );
      }
    }

    if (request.type === "diagnose") {
      try {
        return {
          type: "diagnose-success",
          requestId: request.requestId,
          result: await (dependencies.diagnose ?? diagnoseBattleConfig)(request.payload.config),
        };
      } catch (error) {
        return failure(request.requestId, "diagnose", "diagnostic-failed", `卡组诊断失败：${errorMessage(error)}`);
      }
    }

    if (request.type === "decode-replay") {
      try {
        return {
          type: "decode-replay-success",
          requestId: request.requestId,
          result: (dependencies.decodeReplay ?? decodeOriginalReplayBin)(request.payload.bytes),
        };
      } catch (error) {
        return failure(
          request.requestId,
          "decode-replay",
          "replay-decode-failed",
          `原版对局解码失败：${errorMessage(error)}`,
        );
      }
    }

    if (request.type === "target-practice") {
      try {
        const config = buildTargetPracticeFixture(
          request.payload.build,
          request.payload.gameRound,
        );
        const outcome = await (dependencies.targetPractice ?? runRustTargetPractice)(config);
        if (!outcome.hookSteps || outcome.hookSteps.length === 0) {
          throw new Error("钩子链不可用：打靶伤害归因需要 yixian_trace_json 数据");
        }
        return {
          type: "target-practice-success",
          requestId: request.requestId,
          result: {
            buildId: request.payload.buildId,
            ...outcome,
          },
        };
      } catch (error) {
        return failure(
          request.requestId,
          "target-practice",
          "simulation-failed",
          `打靶模拟失败：${errorMessage(error)}`,
        );
      }
    }

    try {
      return {
        type: "solve-success",
        requestId: request.requestId,
        result: await routeSolve(request.payload, dependencies),
      };
    } catch (error) {
      return failure(
        request.requestId,
        "solve",
        "solver-failed",
        `求解失败：${errorMessage(error)}`,
      );
    }
  };
}

export const handleWorkbenchWorkerRequest = createWorkbenchWorkerHandler();

function routeSolve(
  payload: WorkbenchSolvePayload,
  dependencies: WorkbenchWorkerDependencies,
): ExactDeckSearchResult | Promise<ExactDeckSearchResult> {
  return dependencies.solve(payload);
}

function isWorkbenchWorkerRequest(value: unknown): value is WorkbenchWorkerRequest {
  if (!value || typeof value !== "object") return false;
  const request = value as Partial<WorkbenchWorkerRequest>;
  if (typeof request.requestId !== "string" || request.requestId.length === 0) return false;
  if (request.type === "simulate" || request.type === "diagnose") return isRecord(request.payload);
  if (request.type === "decode-replay") {
    return isRecord(request.payload) &&
      (request.payload as { readonly bytes?: unknown }).bytes instanceof Uint8Array;
  }
  if (request.type === "target-practice") {
    if (!isRecord(request.payload)) return false;
    const payload = request.payload as Partial<Extract<WorkbenchWorkerRequest, { type: "target-practice" }>["payload"]>;
    return (
      typeof payload.buildId === "string" &&
      payload.buildId.length > 0 &&
      isRecord(payload.build) &&
      Number.isFinite(payload.gameRound)
    );
  }
  if (request.type !== "solve" || !isRecord(request.payload)) return false;
  const payload = request.payload as Partial<WorkbenchSolvePayload>;
  return (
    (payload.mode === "order" || payload.mode === "hand" || payload.mode === "pool") &&
    (payload.side === "p1" || payload.side === "p2") &&
    isRecord(payload.fixture) &&
    (payload.visitOrder === "canonical" || payload.visitOrder === "stratified") &&
    Number.isFinite(payload.visitSeed) &&
    Number.isFinite(payload.maxEvaluations) &&
    Number.isFinite(payload.topN)
  );
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}

function requestIdFrom(value: unknown): string {
  if (!isRecord(value) || typeof value.requestId !== "string") return "unknown";
  return value.requestId;
}

function failure(
  requestId: string,
  operation: WorkbenchWorkerOperation,
  kind: WorkbenchWorkerErrorKind,
  message: string,
): WorkbenchWorkerResponse {
  return {
    type: "failure",
    requestId,
    operation,
    error: { kind, message },
  };
}

function errorMessage(error: unknown): string {
  if (error instanceof Error && error.message.trim()) return error.message;
  const message = String(error).trim();
  return message || "未知错误";
}
