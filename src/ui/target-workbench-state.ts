import {
  EMPTY_CHARACTER_ID,
  defaultPlayerConfig,
  normalizePlayerTalents,
} from "./data";
import { loadTargetBuilds, persistTargetBuilds } from "./main-storage";
import {
  clampGameRound,
  invalidateAllTargetBuilds,
  invalidateComputedResults,
  syncPlayerDerivedStats,
} from "./main-utils";
import { TARGET_DAMAGE_THRESHOLD_DEFAULT } from "./target-dummy";
import type {
  AppState,
  TargetBuild,
  TargetCompareMode,
  TargetPracticeState,
  WorkbenchMode,
} from "./types";

export function targetPracticeState(state: AppState): TargetPracticeState {
  if (!state.target) state.target = createTargetPracticeState(state);
  return state.target;
}

export function createTargetPracticeState(state: AppState): TargetPracticeState {
  const round = state.config.gameRound;
  const builds = loadTargetBuilds(round);
  if (builds.length === 0) builds.push(emptyTargetBuild(round, 1));
  return {
    builds,
    activeBuildId: builds[0]!.id,
    damageThreshold: TARGET_DAMAGE_THRESHOLD_DEFAULT,
    displayRounds: 1,
    displayRoundMin: 1,
    displayRoundPending: false,
    compareMode: "overlay",
    expandedStep: null,
    expandedStepBuildId: null,
    duelP1Player: null,
  };
}

function emptyTargetBuild(gameRound: number, index: number): TargetBuild {
  return {
    id: createTargetBuildId(),
    name: `构筑 ${index}`,
    player: defaultPlayerConfig("p1", EMPTY_CHARACTER_ID, gameRound),
    result: null,
    status: "idle",
    errorMessage: null,
  };
}

export function activeTargetBuild(state: AppState): TargetBuild | undefined {
  const target = state.target;
  if (!target) return undefined;
  return target.builds.find((build) => build.id === target.activeBuildId) ?? target.builds[0];
}

export function switchWorkbenchMode(state: AppState, mode: WorkbenchMode): void {
  if (state.workbenchMode === mode) return;
  if (mode === "target") {
    const target = targetPracticeState(state);
    target.duelP1Player = state.config.players.p1;
    const active = activeTargetBuild(state);
    if (active) {
      syncTargetBuildPlayer(state, active);
      state.config.players.p1 = active.player;
    }
    state.view = "setup";
    state.frameIndex = 0;
    state.battleModule = undefined;
    state.flowMetric = undefined;
  } else {
    const target = state.target;
    if (target) {
      if (target.duelP1Player) state.config.players.p1 = target.duelP1Player;
      target.duelP1Player = null;
    }
  }
  state.workbenchMode = mode;
  state.pickerMode = "none";
  state.error = null;
  invalidateComputedResults(state);
  invalidateAllTargetBuilds(state);
}

export function selectTargetBuild(state: AppState, buildId: string): void {
  const target = state.target;
  const build = target?.builds.find((candidate) => candidate.id === buildId);
  if (!target || !build) return;
  target.activeBuildId = buildId;
  if (target.expandedStepBuildId !== buildId) {
    target.expandedStep = null;
    target.expandedStepBuildId = null;
  }
  if (state.workbenchMode === "target") {
    syncTargetBuildPlayer(state, build);
    state.config.players.p1 = build.player;
  }
}

export function addTargetBuild(state: AppState): void {
  const target = targetPracticeState(state);
  const build = emptyTargetBuild(state.config.gameRound, target.builds.length + 1);
  target.builds.push(build);
  selectTargetBuild(state, build.id);
  persistTargetBuilds(target.builds);
}

export function removeTargetBuild(state: AppState, buildId: string): void {
  const target = state.target;
  if (!target || target.builds.length <= 1) return;
  const index = target.builds.findIndex((build) => build.id === buildId);
  if (index < 0) return;
  target.builds.splice(index, 1);
  if (target.activeBuildId === buildId) {
    if (target.expandedStepBuildId === buildId) {
      target.expandedStep = null;
      target.expandedStepBuildId = null;
    }
    const next = target.builds[Math.max(0, index - 1)] ?? target.builds[0];
    if (next) selectTargetBuild(state, next.id);
  }
  persistTargetBuilds(target.builds);
}

export function duplicateTargetBuild(state: AppState, buildId: string): void {
  const target = state.target;
  const source = target?.builds.find((build) => build.id === buildId);
  if (!target || !source) return;
  const clone: TargetBuild = {
    id: createTargetBuildId(),
    name: `${source.name} 副本`,
    player: structuredClone(source.player),
    result: null,
    status: "idle",
    errorMessage: null,
  };
  target.builds.push(clone);
  selectTargetBuild(state, clone.id);
  persistTargetBuilds(target.builds);
}

export function renameTargetBuild(state: AppState, buildId: string, name: string): void {
  const target = state.target;
  const build = target?.builds.find((candidate) => candidate.id === buildId);
  if (!target || !build) return;
  build.name = name.trim() || build.name;
  persistTargetBuilds(target.builds);
}

export function setTargetCompareMode(state: AppState, mode: TargetCompareMode): void {
  if (state.target) state.target.compareMode = mode;
}

function syncTargetBuildPlayer(state: AppState, build: TargetBuild): void {
  build.player.gameRound = clampGameRound(state.config.gameRound);
  normalizePlayerTalents(build.player);
  syncPlayerDerivedStats(build.player, state.config.gameRound, false);
}

function createTargetBuildId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `target-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}
