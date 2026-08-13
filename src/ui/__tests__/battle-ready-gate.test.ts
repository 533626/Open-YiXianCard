import { describe, expect, test } from "bun:test";
import { defaultBattleConfig } from "../data";
import { battleAutoRunReady, shouldScheduleAutoBattle } from "../main-utils";
import type { AppState } from "../types";
import type { ReplayFixtureWithExpected } from "../fixture-contract";

function freshState(): AppState {
  return {
    view: "setup",
    workbenchMode: "duel",
    target: null,
    config: defaultBattleConfig(),
    activeSide: "p1",
    pickerMode: "none",
    selectedSlot: 0,
    selectedTalentSlot: 1,
    cardSearch: "",
    cardArchiveKind: "all",
    cardArchiveKey: "all",
    cardType: "all",
    frameIndex: 0,
    autoPlaying: false,
    result: null,
    solverResult: null,
    solverCollapsed: false,
    error: null,
    savedBuilds: [],
    saveDraftNames: { p1: "", p2: "" },
    selectedBuildIds: { p1: "", p2: "" },
  };
}

/** 任意正数角色 ID 即可让就绪闸的“已选角色”分支通过；不需要真实角色配置。 */
const CHARACTER_ID = 4_000_001;

function selectBothCharacters(state: AppState): void {
  state.config.players.p1.characterId = CHARACTER_ID;
  state.config.players.p2.characterId = CHARACTER_ID;
}

function placeCard(
  state: AppState,
  side: "p1" | "p2",
  slot: number,
  baseId = 100,
): void {
  state.config.players[side].deck[slot] = { baseId, level: 0 };
}

describe("自动推演就绪闸 battleAutoRunReady", () => {
  test("空构筑（没选角色、没摆牌、无导入）不自动跑", () => {
    expect(battleAutoRunReady(freshState())).toBe(false);
  });

  test("只选了角色但场上没牌不自动跑，避免空对局", () => {
    const state = freshState();
    selectBothCharacters(state);
    expect(battleAutoRunReady(state)).toBe(false);
  });

  test("只有一方摆了牌不自动跑", () => {
    const state = freshState();
    selectBothCharacters(state);
    placeCard(state, "p1", 0);
    expect(battleAutoRunReady(state)).toBe(false);

    const other = freshState();
    selectBothCharacters(other);
    placeCard(other, "p2", 0);
    expect(battleAutoRunReady(other)).toBe(false);
  });

  test("双方各 ≥1 张场上牌才自动跑", () => {
    const state = freshState();
    selectBothCharacters(state);
    placeCard(state, "p1", 0);
    placeCard(state, "p2", 0);
    expect(battleAutoRunReady(state)).toBe(true);
  });

  test("导入对局直接放行，不要求手动摆牌", () => {
    const state = freshState();
    selectBothCharacters(state);
    state.importedFixture = {} as ReplayFixtureWithExpected;
    expect(battleAutoRunReady(state)).toBe(true);
  });

  test("缺一方角色时即使双方都摆了牌也不自动跑", () => {
    const state = freshState();
    state.config.players.p1.characterId = CHARACTER_ID;
    placeCard(state, "p1", 0);
    placeCard(state, "p2", 0);
    expect(battleAutoRunReady(state)).toBe(false);
  });
});

describe("弹层打开时不自动推演 shouldScheduleAutoBattle", () => {
  function readyState(): AppState {
    const state = freshState();
    selectBothCharacters(state);
    placeCard(state, "p1", 0);
    placeCard(state, "p2", 0);
    return state;
  }

  test("就绪但任何选择弹层打开都不调度自动推演", () => {
    for (const mode of ["card", "talent", "fate", "character"] as const) {
      const state = readyState();
      state.pickerMode = mode;
      expect(shouldScheduleAutoBattle(state)).toBe(false);
    }
  });

  test("弹层关闭且就绪才调度自动推演", () => {
    const state = readyState();
    state.pickerMode = "none";
    expect(shouldScheduleAutoBattle(state)).toBe(true);
  });

  test("弹层关闭但就绪闸未通过（没摆牌）仍不调度", () => {
    const state = freshState();
    selectBothCharacters(state);
    state.pickerMode = "none";
    expect(shouldScheduleAutoBattle(state)).toBe(false);
  });
});