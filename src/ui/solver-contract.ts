import type { OriginalCardConfig } from "../../shared/contracts";
import type { BattleDecisionEvent } from "../../shared/contracts";
import type { OriginalReplayFixture } from "./domain";

export type SolverSide = "p1" | "p2";
export type SolverMode = "order" | "hand" | "beam" | "talent";
export type SolverConfidence = "exact" | "heuristic" | "truncated";
export type SolverScoreProfile = "hpDelta" | "value-v0";

export interface SolverScoringOptions {
  readonly scoreProfile?: SolverScoreProfile;
}

export interface SolverSeedAggregate {
  readonly rankingPolicy: "win-count-then-average-score";
  readonly seedsUsed: readonly number[];
  readonly winCount: number;
  readonly averageScore: number;
  readonly syntheticDecisionSeedsUsed: readonly number[];
  readonly usedSyntheticDecisions: boolean;
}

export interface SolverValueMetrics {
  readonly terminalValueForSide: number;
  readonly terminalHpForSide: number;
  readonly terminalShieldForSide: number;
  readonly terminalDefenseForSide: number;
  readonly terminalGuardForSide: number;
  readonly terminalResourceForSide: number;
  readonly terminalDebuffForSide: number;
  readonly terminalTempoForSide: number;
  readonly terminalTempoCountForSide: number;
  readonly areaScoreForSide: number;
  readonly hpAreaForSide: number;
  readonly resourceAreaForSide: number;
  readonly debuffAreaForSide: number;
  readonly hpAreaScoreForSide: number;
  readonly resourceAreaScoreForSide: number;
  readonly debuffAreaScoreForSide: number;
  readonly areaSampleCount: number;
  readonly auditMismatchFields: readonly string[];
}

export interface SolverEvaluation {
  readonly side: SolverSide;
  readonly scoreProfile: SolverScoreProfile;
  readonly winnerSide: SolverSide;
  readonly winForSide: boolean;
  readonly actorTurn: number;
  readonly p1Hp: number;
  readonly p2Hp: number;
  readonly hpDeltaForSide: number;
  readonly score: number;
  readonly failed?: boolean;
  readonly valueMetrics?: SolverValueMetrics;
  readonly warnings: readonly string[];
  readonly completedCards: readonly { readonly actionIndex: number; readonly actorId: string; readonly cardId: number; readonly cardName: string }[];
  readonly decisionEvents?: readonly BattleDecisionEvent[];
  readonly seedAggregate?: SolverSeedAggregate;
}

export interface SolverDeckResult {
  readonly rank: number;
  readonly confidence: SolverConfidence;
  readonly deck: readonly OriginalCardConfig[];
  readonly leftoverHandCardIds: readonly number[];
  readonly evaluation: SolverEvaluation;
  readonly changedSlots: readonly { readonly slot: number; readonly from: OriginalCardConfig; readonly to: OriginalCardConfig }[];
  readonly talentIds?: readonly number[];
  readonly talentChanges?: readonly { readonly slot: number; readonly from: number; readonly to: number }[];
  readonly deckKey: string;
}

export interface ExactDeckSearchResult {
  readonly mode: SolverMode;
  readonly side: SolverSide;
  readonly confidence: SolverConfidence;
  readonly evaluatedCount: number;
  readonly skippedDuplicateCount: number;
  readonly candidateCardCount: number;
  readonly candidateTalentCount?: number;
  readonly baseline: SolverEvaluation;
  readonly baselineDeck: readonly OriginalCardConfig[];
  readonly baselineTalents?: readonly number[];
  readonly results: readonly SolverDeckResult[];
  readonly marginalChanges?: readonly { readonly slot: number; readonly from: OriginalCardConfig; readonly to: OriginalCardConfig; readonly evaluation: SolverEvaluation; readonly gain: number }[];
  readonly seedsUsed?: readonly number[];
  readonly syntheticDecisionSeedsUsed?: readonly number[];
  readonly usedSyntheticDecisions?: boolean;
}

export interface ExactDeckSearchOptions {
  readonly fixture: OriginalReplayFixture;
  readonly side: SolverSide;
  readonly mode: SolverMode;
  readonly scoring?: SolverScoringOptions;
  readonly topN?: number;
  readonly maxEvaluations?: number;
  readonly battleSeeds?: readonly number[];
}

export function sortDeckResults<T extends { readonly evaluation: SolverEvaluation; readonly deckKey?: string }>(results: readonly T[], topN: number): T[] {
  return [...results].sort((left, right) =>
    (right.evaluation.seedAggregate?.winCount ?? Number(right.evaluation.winForSide)) -
      (left.evaluation.seedAggregate?.winCount ?? Number(left.evaluation.winForSide)) ||
    Number(right.evaluation.winForSide) - Number(left.evaluation.winForSide) ||
    right.evaluation.score - left.evaluation.score ||
    right.evaluation.hpDeltaForSide - left.evaluation.hpDeltaForSide ||
    left.evaluation.actorTurn - right.evaluation.actorTurn ||
    compareDeckKeys(left.deckKey, right.deckKey)
  ).slice(0, topN);
}

export type ValueRankCategory = "same-top" | "value-lost-win" | "value-found-win" | "winner-split" | "value-hp-regret" | "value-slower-close" | "value-faster-close" | "different-aligned";
export interface ValueRankComparison {
  readonly category: ValueRankCategory;
  readonly valueScoreDelta: number;
  readonly hpDeltaRegret: number;
  readonly actorTurnDelta: number;
}
export function classifyValueRankComparison(hpDeltaTop: { readonly deckKey: string; readonly evaluation: SolverEvaluation }, valueTop: { readonly deckKey: string; readonly evaluation: SolverEvaluation }, hpDeltaTopValueEvaluation: SolverEvaluation): ValueRankComparison {
  const sameTop = hpDeltaTop.deckKey === valueTop.deckKey;
  const hpRegret = hpDeltaTop.evaluation.hpDeltaForSide - valueTop.evaluation.hpDeltaForSide;
  const actorDelta = valueTop.evaluation.actorTurn - hpDeltaTop.evaluation.actorTurn;
  let category: ValueRankCategory = "different-aligned";
  if (sameTop) category = "same-top";
  else if (hpDeltaTop.evaluation.winForSide && !valueTop.evaluation.winForSide) category = "value-lost-win";
  else if (!hpDeltaTop.evaluation.winForSide && valueTop.evaluation.winForSide) category = "value-found-win";
  else if (hpDeltaTop.evaluation.winnerSide !== valueTop.evaluation.winnerSide) category = "winner-split";
  else if (hpDeltaTop.evaluation.winForSide && valueTop.evaluation.winForSide) {
    if (hpRegret >= 1) category = "value-hp-regret";
    else if (actorDelta >= 1) category = "value-slower-close";
    else if (actorDelta <= -1) category = "value-faster-close";
  }
  return {
    category,
    valueScoreDelta: valueTop.evaluation.score - hpDeltaTopValueEvaluation.score,
    hpDeltaRegret: hpRegret,
    actorTurnDelta: actorDelta,
  };
}

function compareDeckKeys(left?: string, right?: string): number {
  if (left === undefined || right === undefined || left === right) return 0;
  return left < right ? -1 : 1;
}
