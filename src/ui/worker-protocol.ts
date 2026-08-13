import type {
  ExactDeckSearchResult,
  SolverScoringOptions,
} from "./solver-contract";
import type { OriginalReplayFixture } from "./domain";
import type {
  BattleConfig,
  BattleFrame,
  PlayerConfig,
  Side,
  SimulationResult,
} from "./types";
import type { ReplayFixtureWithExpected } from "./fixture-contract";
import type { FixtureConsistencyReport } from "./fixture-consistency";
import type { DeckDiagnosticResult } from "./deck-diagnostics";
import type { HookStep } from "./hook-trace";
import type { DecodedOriginalReplay } from "./original-replay-bin";

export type WorkbenchWorkerOperation =
  | "simulate"
  | "solve"
  | "diagnose"
  | "decode-replay"
  | "target-practice"
  | "protocol";

export type WorkbenchWorkerErrorKind =
  | "invalid-request"
  | "simulation-failed"
  | "solver-failed"
  | "diagnostic-failed"
  | "replay-decode-failed"
  | "worker-unavailable"
  | "worker-crashed"
  | "invalid-response"
  | "cancelled";

interface WorkbenchSolveBase {
  readonly fixture: OriginalReplayFixture;
  readonly side: Side;
  readonly scoring: SolverScoringOptions;
  readonly topN: number;
  readonly maxEvaluations: number;
  readonly battleSeeds?: readonly number[];
  readonly visitOrder: "canonical" | "stratified";
  readonly visitSeed: number;
}

export type WorkbenchSolvePayload = WorkbenchSolveBase & {
  readonly mode: "order" | "hand" | "pool";
};

export type WorkbenchWorkerRequest =
  | {
    readonly type: "decode-replay";
    readonly requestId: string;
    readonly payload: { readonly bytes: Uint8Array };
  }
  | {
    readonly type: "diagnose";
    readonly requestId: string;
    readonly payload: { readonly config: BattleConfig };
  }
  | {
    readonly type: "simulate";
    readonly requestId: string;
    readonly payload: {
      /** Detached clone: UI edits cannot mutate an in-flight battle. */
      readonly config: BattleConfig;
      /** Present only while an imported fixture still exactly matches config. */
      readonly comparisonFixture?: ReplayFixtureWithExpected;
    };
  }
  | {
    readonly type: "solve";
    readonly requestId: string;
    readonly payload: WorkbenchSolvePayload;
  }
  | {
    readonly type: "target-practice";
    readonly requestId: string;
    readonly payload: {
      /** 打靶构筑槽位 id，多构筑并发时按它回写各自结果。 */
      readonly buildId: string;
      /** Detached clone: UI 编辑不能影响在途推演。 */
      readonly build: PlayerConfig;
      readonly gameRound: number;
    };
  };

export type WorkbenchWorkerResponse =
  | {
    readonly type: "decode-replay-success";
    readonly requestId: string;
    readonly result: DecodedOriginalReplay;
  }
  | {
    readonly type: "diagnose-success";
    readonly requestId: string;
    readonly result: DeckDiagnosticResult;
  }
  | {
    readonly type: "simulate-success";
    readonly requestId: string;
    readonly result: SimulationResult;
    readonly fixtureConsistency: FixtureConsistencyReport | null;
  }
  | {
    readonly type: "solve-success";
    readonly requestId: string;
    readonly result: ExactDeckSearchResult;
  }
  | {
    readonly type: "target-practice-success";
    readonly requestId: string;
    readonly result: TargetPracticeOutcome;
  }
  | {
    readonly type: "failure";
    readonly requestId: string;
    readonly operation: WorkbenchWorkerOperation;
    readonly error: {
      readonly kind: WorkbenchWorkerErrorKind;
      readonly message: string;
    };
  };

/**
 * 打靶推演产出：frames 供引擎上限判定，hookSteps 供伤害归因。
 *
 * 计划初稿里的 `summary` 字段（引擎汇总）有意省略：UI 侧
 * `computeTargetPracticeResult` 从 hookSteps/frames 直接算出同样的
 * totalDamage/reachedTurn/stopReason，多传一份引擎汇总只会增大 payload 且
 * 存在与 UI 判定不一致的第二真相源。
 */
export interface TargetPracticeOutcome {
  readonly buildId: string;
  readonly frames: readonly BattleFrame[];
  readonly hookSteps: readonly HookStep[];
}

export interface WorkbenchSimulationOutcome {
  readonly result: SimulationResult;
  readonly fixtureConsistency: FixtureConsistencyReport | null;
}
