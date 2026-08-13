import { describe, expect, test } from "bun:test";
import {
  invalidateComputedResults,
  reorderDeckSlot,
} from "../main-utils";
import { renderPlayerDeck } from "../render-player-deck";
import {
  baseState,
  battleFrame,
  playerView,
  simulationResult,
} from "./layout-test-helpers";

function deckIds(state: ReturnType<typeof baseState>): number[] {
  return state.config.players.p1.deck.slice(0, 4).map((slot) => slot.baseId);
}

describe("构筑牌槽拖拽", () => {
  test("落在牌中央时任意两格互换", () => {
    const state = baseState();
    state.config.players.p1.deck.splice(0, 4,
      { baseId: 1, level: 0 },
      { baseId: 2, level: 0 },
      { baseId: 3, level: 0 },
      { baseId: 4, level: 0 },
    );

    expect(reorderDeckSlot(state, "p1", 0, 3, "swap")).toBe(3);
    expect(deckIds(state)).toEqual([4, 2, 3, 1]);
  });

  test("落在牌边缘时插入并顺移中间牌", () => {
    const state = baseState();
    state.config.players.p1.deck.splice(0, 4,
      { baseId: 1, level: 0 },
      { baseId: 2, level: 0 },
      { baseId: 3, level: 0 },
      { baseId: 4, level: 0 },
    );

    expect(reorderDeckSlot(state, "p1", 0, 3, "insert-before")).toBe(2);
    expect(deckIds(state)).toEqual([2, 3, 1, 4]);
    expect(reorderDeckSlot(state, "p1", 3, 0, "insert-after")).toBe(1);
    expect(deckIds(state)).toEqual([2, 4, 3, 1]);
  });

  test("改构筑时清掉旧战斗、求解和诊断投影", () => {
    const state = baseState();
    state.result = simulationResult([battleFrame([])]);
    state.frameIndex = 1;
    state.battleStatus = { state: "running", requestId: "battle-old" };
    const fixtureSummary = {
      winnerSide: "p1" as const,
      actorTurnCount: 1,
      hpDeltaP1MinusP2: 1,
      finalHp: { p1: 1, p2: 0 },
    };
    state.fixtureConsistency = {
      engine: fixtureSummary,
      ui: fixtureSummary,
      engineMatch: true,
      expectedMatch: true,
    };
    state.solverResult = { mode: "order" } as never;
    state.solverStatus = {
      mode: "order",
      state: "running",
      requestId: "solver-old",
      maxEvaluations: 1,
    };
    state.diagnosticResult = { p1: [], p2: [] } as never;
    state.diagnosticStatus = { state: "running", requestId: "diagnostic-old" };

    invalidateComputedResults(state);

    expect(state.result).toBeNull();
    expect(state.frameIndex).toBe(0);
    expect(state.battleStatus).toBeNull();
    expect(state.fixtureConsistency).toBeNull();
    expect(state.solverResult).toBeNull();
    expect(state.solverStatus).toBeNull();
    expect(state.diagnosticResult).toBeNull();
    expect(state.diagnosticStatus).toBeNull();
  });

  test("运行态临时升级牌保留醒目标志", () => {
    const state = baseState();
    const player = state.config.players.p1;
    const runtime = playerView({
      slots: [{
        index: 0,
        cardId: 10_000,
        baseId: 0,
        name: "普通攻击",
        skipped: false,
        hadUsed: false,
        temporarilyUpgraded: true,
      }],
    });

    const html = renderPlayerDeck({
      state,
      side: "p1",
      player,
      frame: battleFrame([], { actorId: "p1", sourceSlot: 0 }),
      runtime,
    });

    expect(html).toContain("deck-slot temporarily-upgraded active");
    expect(html).toContain("临时升级");
  });
});
