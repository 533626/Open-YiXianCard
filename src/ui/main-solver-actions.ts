import { normalizeBaseId } from "./domain";
import { lockedBaseTalentId, normalizePlayerTalents, CARD_OPTION_BY_BASE_ID } from "./data";
import { buildReplayFixture } from "./replay-fixture-builder";
import { SOLVER_PRESETS, type SolverUiMode } from "./solver-ui";
import { visibleErrorMessage } from "./view-utils";
import type { AppState, Side } from "./types";
import { syncDualCareers } from "./main-utils";
import {
  workbenchWorkerClient,
  type WorkbenchWorkerClient,
} from "./worker-client";
import type { WorkbenchSolvePayload } from "./worker-protocol";
import { solverCardPoolOptions } from "./solver-card-pool";
import type { ActionContext } from "./main-actions";
import type { CardOption, CardVariantOption } from "./types";

const SOLVER_VALUE_SCORING = { scoreProfile: "value-v0" as const };

/** 取卡牌的默认变体：single 取唯一变体，rarity 取 rarity=0（一阶），realm 取境界序最低（炼气）。 */
function baseVariant(card: CardOption): CardVariantOption | undefined {
  if (card.variantMode === "single") return card.variants[0];
  if (card.variantMode === "realm") return card.variants[0];
  return card.variants.find((item) => item.rarity === 0) ?? card.variants[0];
}

export async function scheduleSolver(context: ActionContext): Promise<void> {
  if (context.state.solverStatus?.state === "running") return;
  if (context.state.battleStatus?.state === "running") {
    context.state.error = "请先取消正在运行的战斗推演";
    context.render();
    return;
  }
  const mode = context.state.solverMode ?? "orderBeam";
  const preset = SOLVER_PRESETS[mode];
  const startedAt = nowMs();
  const client = context.workerClient ?? workbenchWorkerClient;
  let task: ReturnType<WorkbenchWorkerClient["solve"]>;
  try {
    task = client.solve(buildSolverPayload(context.state, mode));
  } catch (error) {
    const message = visibleErrorMessage(error);
    context.state.solverStatus = {
      mode,
      state: "error",
      elapsedMs: Math.round(nowMs() - startedAt),
      maxEvaluations: preset.maxEvaluations,
      message,
    };
    context.state.error = message;
    context.render();
    return;
  }
  context.state.solverStatus = {
    mode,
    state: "running",
    startedAt,
    maxEvaluations: preset.maxEvaluations,
    requestId: task.requestId,
  };
  context.state.solverResult = null;
  context.state.solverCollapsed = false;
  context.state.error = null;
  context.render();
  focusAction("cancel-solver");

  try {
    const result = await task.result;
    if (!isCurrentSolverRun(context.state, task.requestId)) return;
    context.state.solverResult = result;
    context.state.solverStatus = {
      mode,
      state: "done",
      elapsedMs: Math.round(nowMs() - startedAt),
      maxEvaluations: preset.maxEvaluations,
      evaluatedCount: result.evaluatedCount,
      requestId: task.requestId,
    };
    context.state.error = null;
  } catch (error) {
    if (!isCurrentSolverRun(context.state, task.requestId)) return;
    const message = visibleErrorMessage(error);
    context.state.solverStatus = {
      mode,
      state: "error",
      elapsedMs: Math.round(nowMs() - startedAt),
      maxEvaluations: preset.maxEvaluations,
      message,
      requestId: task.requestId,
    };
    context.state.error = message;
  }
  context.render();
}
export async function scheduleDeckDiagnostics(context: ActionContext): Promise<void> {
  if (context.state.diagnosticStatus?.state === "running") return;
  if (context.state.battleStatus?.state === "running" || context.state.solverStatus?.state === "running") {
    context.state.error = "请先取消正在运行的计算";
    context.render();
    return;
  }
  const client = context.workerClient ?? workbenchWorkerClient;
  let task: ReturnType<WorkbenchWorkerClient["diagnose"]>;
  try {
    task = client.diagnose(structuredClone(context.state.config));
  } catch (error) {
    context.state.diagnosticStatus = { state: "error", message: visibleErrorMessage(error) };
    context.render();
    return;
  }
  context.state.diagnosticStatus = { state: "running", requestId: task.requestId };
  context.state.diagnosticResult = null;
  context.state.error = null;
  context.render();
  try {
    const result = await task.result;
    if (context.state.diagnosticStatus?.requestId !== task.requestId) return;
    context.state.diagnosticResult = result;
    context.state.diagnosticStatus = { state: "done", requestId: task.requestId };
  } catch (error) {
    if (context.state.diagnosticStatus?.requestId !== task.requestId) return;
    context.state.diagnosticStatus = { state: "error", message: visibleErrorMessage(error) };
  }
  context.render();
}

export function cancelSolver(context: ActionContext): void {
  const status = context.state.solverStatus;
  if (status?.state !== "running") return;
  const client = context.workerClient ?? workbenchWorkerClient;
  client.cancelAll("求解已取消");
  context.state.solverStatus = {
    mode: status.mode,
    state: "error",
    elapsedMs: status.startedAt === undefined ? undefined : Math.round(nowMs() - status.startedAt),
    maxEvaluations: status.maxEvaluations,
    message: "求解已取消",
  };
  context.state.error = null;
  context.render();
  focusAction("solve-active");
}

function isCurrentSolverRun(state: AppState, requestId: string): boolean {
  return state.solverStatus?.state === "running" && state.solverStatus.requestId === requestId;
}

function nowMs(): number {
  return globalThis.performance?.now?.() ?? Date.now();
}

export function buildSolverPayload(state: AppState, mode: SolverUiMode): WorkbenchSolvePayload {
  normalizePlayerTalents(state.config.players.p1);
  normalizePlayerTalents(state.config.players.p2);
  const preset = SOLVER_PRESETS[mode];
  const fixture = preset.task === "pool"
    ? buildCardPoolFixture(state)
    : buildReplayFixture(state.config);
  const side = state.activeSide;
  if (preset.task === "hand" && fixture.players[side].handCards.length === 0) {
    throw new Error("当前对局没有可求解的手牌，请先导入含手牌的对局 JSON");
  }
  const base = {
    fixture,
    side,
    scoring: SOLVER_VALUE_SCORING,
    topN: 5,
    battleSeeds: [1, 2, 3] as const,
    maxEvaluations: preset.maxEvaluations,
    visitOrder: preset.method === "heuristic" ? "stratified" as const : "canonical" as const,
    visitSeed: 20_260_725,
  } as const;
  return {
    ...base,
    mode: preset.task,
  };
}

function buildCardPoolFixture(state: AppState): WorkbenchSolvePayload["fixture"] {
  const fixture = buildReplayFixture(state.config);
  const side = state.activeSide;
  const player = state.config.players[side];
  const deckCopies = new Map<number, number>();
  for (const slot of player.deck) {
    if (slot.baseId === 0) continue;
    deckCopies.set(slot.baseId, (deckCopies.get(slot.baseId) ?? 0) + 1);
  }
  const poolOptions = solverCardPoolOptions(player);
  const catalogCards = poolOptions.flatMap((card) => {
    const variant = baseVariant(card);
    return variant ? [variant.config] : [];
  });
  const handCards = poolOptions.flatMap((card) => {
    const variant = baseVariant(card);
    if (!variant) return [];
    const availableCopies = Math.max(0, 1 - (deckCopies.get(card.baseId) ?? 0));
    return Array.from({ length: availableCopies }, () => variant.id);
  });
  return {
    ...fixture,
    catalogCards,
    players: {
      ...fixture.players,
      [side]: {
        ...fixture.players[side],
        handCards,
      },
    },
  };
}

function focusAction(action: string): void {
  if (typeof document === "undefined") return;
  queueMicrotask(() => {
    document.querySelector<HTMLButtonElement>(`[data-action='${action}']`)?.focus({ preventScroll: true });
  });
}



export function applySolverDeck(
  state: AppState,
  deck: readonly { readonly id: number; readonly rarity?: number }[],
  talentIds: readonly number[] | undefined,
  side: Side,
): void {
  const player = state.config.players[side];
  player.deck = deck.map((card) => {
    const baseId = card.id === 0 ? 0 : normalizeBaseId(card.id);
    const cardOption = baseId !== 0 ? CARD_OPTION_BY_BASE_ID.get(baseId) : undefined;
    // 单变体卡 level 恒为 0；梦境卡 level 是境界序（从整牌 id 反查变体下标）。
    if (cardOption?.variantMode === "single") return { baseId, level: 0 };
    if (cardOption?.variantMode === "realm" && card.id !== 0) {
      const realmIndex = cardOption.variants.findIndex((v) => v.id === card.id);
      return { baseId, level: realmIndex >= 0 ? realmIndex : 0 };
    }
    return { baseId, level: card.rarity ?? 0 };
  });
  if (talentIds) {
    player.talents = Array.from({ length: 5 }, (_, index) => talentIds[index] ?? 0);
    player.talents[0] = lockedBaseTalentId(player.characterId);
  }
  syncDualCareers(player);
  state.activeSide = side;
  state.result = null;
  state.pickerMode = "none";
}

export function applySolverBest(state: AppState): void {
  const result = state.solverResult;
  const best = result?.results[0];
  if (!result || !best) return;
  applySolverDeck(state, best.deck, best.talentIds, result.side);
}

export function applySolverRow(target: HTMLElement, state: AppState): void {
  const result = state.solverResult;
  if (!result) return;
  const deckKey = target.dataset.deckKey;
  if (!deckKey) return;
  const item = result.results.find((candidate) => candidate.deckKey === deckKey);
  if (!item) return;
  applySolverDeck(state, item.deck, item.talentIds, result.side);
}

export function applySolverBaseline(state: AppState): void {
  const result = state.solverResult;
  if (!result) return;
  applySolverDeck(state, result.baselineDeck, result.baselineTalents, result.side);
}
