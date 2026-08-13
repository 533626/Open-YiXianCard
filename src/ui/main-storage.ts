import {
  activeSlotCountForProgress,
  normalizePlayerTalents,
} from "./data";
import {
  defaultJiFangshengInitialFateRank,
  resetCardFilters,
  sanitizePlayerScope,
  syncPlayerDerivedStats,
} from "./main-utils";
import type {
  AppState,
  PlayerConfig,
  SavedBuild,
  SavedPlayerConfig,
  Side,
  TargetBuild,
} from "./types";

const SAVED_BUILDS_KEY = "yixiancard:v2:saved-builds";
const TARGET_BUILDS_KEY = "yixiancard:v2:target-builds";

export function saveCurrentBuild(state: AppState, side: Side): void {
  const player = state.config.players[side];
  const selectedId = state.selectedBuildIds[side];
  const draftName = state.saveDraftNames[side].trim();
  const existing = state.savedBuilds.find((build) => build.id === selectedId)
    ?? (draftName ? state.savedBuilds.find((build) => build.name === draftName) : undefined);
  const name = draftName || existing?.name || defaultBuildName(player);
  const id = existing?.id ?? createBuildId();
  const build: SavedBuild = {
    id,
    name,
    updatedAt: new Date().toISOString(),
    player: toSavedPlayerConfig(player),
  };
  state.savedBuilds = [
    build,
    ...state.savedBuilds.filter((entry) => entry.id !== id),
  ].sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
  state.selectedBuildIds[side] = id;
  state.saveDraftNames[side] = name;
  persistSavedBuilds(state.savedBuilds);
}

export function loadSelectedBuild(state: AppState, side: Side): void {
  const build = state.savedBuilds.find((entry) => entry.id === state.selectedBuildIds[side]);
  if (!build) return;
  applySavedPlayerConfig(state, side, structuredClone(build.player));
  state.saveDraftNames[side] = build.name;
}

export function deleteSelectedBuild(state: AppState, side: Side): void {
  const draftName = state.saveDraftNames[side].trim();
  const selectedId = state.selectedBuildIds[side]
    || state.savedBuilds.find((build) => build.name === draftName)?.id
    || "";
  if (!selectedId) return;
  state.savedBuilds = state.savedBuilds.filter((build) => build.id !== selectedId);
  state.selectedBuildIds[side] = "";
  if (state.saveDraftNames[side].trim() === draftName) state.saveDraftNames[side] = "";
  persistSavedBuilds(state.savedBuilds);
}

export function loadSavedBuilds(): SavedBuild[] {
  try {
    const raw = localStorage.getItem(SAVED_BUILDS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(isSavedBuild)
      .sort((left, right) => right.updatedAt.localeCompare(left.updatedAt));
  } catch {
    return [];
  }
}

function toSavedPlayerConfig(player: PlayerConfig): SavedPlayerConfig {
  return {
    characterId: player.characterId,
    careerName: player.careerName,
    dualCareerNames: { ...player.dualCareerNames },
    level: player.level,
    hp: player.hp,
    maxHp: player.maxHp,
    lifeModifier: player.lifeModifier,
    talentResonanceId: player.talentResonanceId,
    jiFangshengInitialFateRank: player.jiFangshengInitialFateRank,
    defense: player.defense,
    anima: player.anima,
    momentum: player.momentum,
    momentumLimit: player.momentumLimit,
    agility: player.agility,
    guard: player.guard,
    buffs: structuredClone(player.buffs),
    starSlots: [...player.starSlots],
    activatedElements: [...player.activatedElements],
    lastElement: player.lastElement,
    talents: [...player.talents],
    fateStrategies: [...player.fateStrategies],
    lingWuCardBaseIds: [...player.lingWuCardBaseIds],
    handCardIds: [...player.handCardIds],
    lastRoundUsedCardBaseIds: [...player.lastRoundUsedCardBaseIds],
    lastRoundLife: player.lastRoundLife,
    lastRoundExp: player.lastRoundExp,
    talentCardParams: structuredClone(player.talentCardParams),
    talentTempDatas: structuredClone(player.talentTempDatas),
    permanentBuffTempDatas: structuredClone(player.permanentBuffTempDatas),
    deck: player.deck.map((slot) => ({ ...slot })),
  };
}

function applySavedPlayerConfig(state: AppState, side: Side, saved: SavedPlayerConfig): void {
  const previous = state.config.players[side];
  const next: PlayerConfig = {
    side,
    label: previous.label,
    ...saved,
    dualCareerNames: { ...(saved.dualCareerNames ?? {}) },
    gameRound: state.config.gameRound,
    lifeModifier: saved.lifeModifier ?? 0,
    activeSlotCount: activeSlotCountForProgress(state.config.gameRound, saved.level),
    jiFangshengInitialFateRank: saved.jiFangshengInitialFateRank
      ?? defaultJiFangshengInitialFateRank(saved.characterId, state.config.gameRound),
  };
  applyPreparedPlayerConfig(state, side, next);
}

function applyPreparedPlayerConfig(state: AppState, side: Side, player: PlayerConfig): void {
  normalizePlayerTalents(player);
  syncPlayerDerivedStats(player, state.config.gameRound, false);
  sanitizePlayerScope(player);
  state.config.players[side] = player;
  state.activeSide = side;
  state.pickerMode = "none";
  state.result = null;
  state.importedFixture = null;
  state.importedFixtureId = null;
  state.fixtureConsistency = null;
  resetCardFilters(state);
}

function defaultBuildName(player: PlayerConfig): string {
  const duals = Object.values(player.dualCareerNames).filter(Boolean);
  const suffix = duals.length > 0 ? `+${duals.join("+")}` : "";
  return `${player.characterId}-${player.careerName ?? "career"}${suffix}`;
}

function createBuildId(): string {
  return typeof crypto !== "undefined" && "randomUUID" in crypto
    ? crypto.randomUUID()
    : `build-${Date.now().toString(36)}-${Math.random().toString(36).slice(2, 8)}`;
}

function persistSavedBuilds(savedBuilds: readonly SavedBuild[]): void {
  localStorage.setItem(SAVED_BUILDS_KEY, JSON.stringify(savedBuilds));
}

function isSavedBuild(value: unknown): value is SavedBuild {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Partial<SavedBuild>;
  return (
    typeof record.id === "string" &&
    typeof record.name === "string" &&
    typeof record.updatedAt === "string" &&
    typeof record.player === "object" &&
    record.player !== null
  );
}

/** 打靶构筑持久化：只存 id/name/player，结果与运行状态不落盘。 */
export function persistTargetBuilds(builds: readonly TargetBuild[]): void {
  localStorage.setItem(
    TARGET_BUILDS_KEY,
    JSON.stringify(builds.map((build) => ({
      id: build.id,
      name: build.name,
      player: toSavedPlayerConfig(build.player),
    }))),
  );
}

export function loadTargetBuilds(gameRound: number): TargetBuild[] {
  try {
    const raw = localStorage.getItem(TARGET_BUILDS_KEY);
    if (!raw) return [];
    const parsed = JSON.parse(raw) as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed
      .filter(isSavedTargetBuild)
      .map((saved) => ({
        id: saved.id,
        name: saved.name,
        player: targetPlayerFromSaved(saved.player, gameRound),
      }));
  } catch {
    return [];
  }
}

function targetPlayerFromSaved(saved: SavedPlayerConfig, gameRound: number): PlayerConfig {
  const next: PlayerConfig = {
    side: "p1",
    label: "玩家一",
    ...saved,
    dualCareerNames: { ...(saved.dualCareerNames ?? {}) },
    gameRound,
    lifeModifier: saved.lifeModifier ?? 0,
    activeSlotCount: activeSlotCountForProgress(gameRound, saved.level),
    jiFangshengInitialFateRank: saved.jiFangshengInitialFateRank
      ?? defaultJiFangshengInitialFateRank(saved.characterId, gameRound),
  };
  normalizePlayerTalents(next);
  syncPlayerDerivedStats(next, gameRound, false);
  return next;
}

function isSavedTargetBuild(value: unknown): value is { readonly id: string; readonly name: string; readonly player: SavedPlayerConfig } {
  if (typeof value !== "object" || value === null) return false;
  const record = value as Partial<{ readonly id: string; readonly name: string; readonly player: SavedPlayerConfig }>;
  return (
    typeof record.id === "string" &&
    typeof record.name === "string" &&
    typeof record.player === "object" &&
    record.player !== null
  );
}
