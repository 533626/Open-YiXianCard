import type { SolverEvaluation, SolverValueMetrics } from "../solver-contract";
import { CURRENT_STEAM_BUILD } from "../../../shared/data/current-build";
import { defaultBattleConfig } from "../data";
import type { ReplayFixtureWithExpected } from "../fixture-contract";
import type { AppState, BattleFrame, PlayerView, SimulationResult } from "../types";

/** Public tests build the smallest valid replay contract in memory: no corpus file required. */
export function publicReplayFixture(): ReplayFixtureWithExpected & { readonly schemaVersion: 1 } {
  const player = (characterId: number) => ({
    level: 1,
    baseMaxHp: 40,
    extraMaxHp: 0,
    characterId,
    talents: [],
    activeSlotCount: 1,
    permanentBuffTempDatas: {},
    cards: [{ id: 10_000, baseId: 10_000, name: "普通攻击", attack: 3 }],
  });
  return {
    schemaVersion: 1,
    firstPlayerSide: "p1",
    source: { round: 1, steamBuild: CURRENT_STEAM_BUILD },
    players: { p1: player(4_000_004), p2: player(4_000_005) },
    expected: {
      winnerSide: "p1",
      actorTurnCount: 1,
      hpDeltaP1MinusP2: 3,
    },
  };
}

export function baseState(): AppState {
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

export function playerView(overrides: Partial<PlayerView> = {}): PlayerView {
  return {
    id: "p1",
    name: "姬方生",
    side: "p1",
    hp: 50,
    maxHp: 100,
    defense: 2,
    anima: 0,
    momentum: 0,
    momentumLimit: 6,
    agility: 0,
    guard: 0,
    buffs: {},
    sustainValues: {},
    starSlots: [],
    activatedElements: [],
    lastElement: null,
    cardQueue: [],
    slots: [],
    ...overrides,
  };
}

export function battleFrame(events: BattleFrame["events"], overrides: Partial<BattleFrame> = {}): BattleFrame {
  const players = overrides.players ?? {
    p1: playerView(),
    p2: playerView({ id: "p2", name: "李燚", side: "p2" }),
  };
  return {
    index: 1,
    gameRound: 16,
    actionIndex: 1,
    title: "第 1 动 · 万玄破魔掌",
    actorId: "p1",
    actorTurn: 1,
    sourceSlot: 2,
    cardId: 82,
    cardName: "万玄破魔掌",
    winnerId: null,
    players,
    events,
    summaries: [],
    ...overrides,
  };
}

export function simulationResult(frames: BattleFrame[], actionCount = frames.length): SimulationResult {
  return {
    winnerId: "p1",
    actionCount,
    frames,
    events: frames.flatMap((frame) => frame.events),
    warnings: [],
    finalActorTurn: frames.at(-1)?.actorTurn ?? 0,
  };
}

export function solverEvaluation(score: number, overrides: Partial<SolverEvaluation> = {}): SolverEvaluation {
  return {
    side: "p1" as const,
    scoreProfile: "hpDelta" as const,
    winnerSide: "p1" as const,
    winForSide: true,
    actorTurn: 1,
    p1Hp: 50,
    p2Hp: 40,
    hpDeltaForSide: 10,
    score,
    warnings: [],
    completedCards: [],
    ...overrides,
  };
}

export function solverValueMetrics(
  terminalValueForSide: number,
  overrides: Partial<SolverValueMetrics> = {},
): SolverValueMetrics {
  return {
    terminalValueForSide,
    terminalHpForSide: 10,
    terminalShieldForSide: 0,
    terminalDefenseForSide: 0,
    terminalGuardForSide: 0,
    terminalResourceForSide: 0,
    terminalDebuffForSide: 0,
    terminalTempoForSide: 0,
    terminalTempoCountForSide: 0,
    areaScoreForSide: 0,
    hpAreaForSide: 0,
    resourceAreaForSide: 0,
    debuffAreaForSide: 0,
    hpAreaScoreForSide: 0,
    resourceAreaScoreForSide: 0,
    debuffAreaScoreForSide: 0,
    areaSampleCount: 1,
    auditMismatchFields: [],
    ...overrides,
  };
}
