import { defaultBattleConfig } from "./data";
import { loadSavedBuilds } from "./main-storage";
import { repositoryFixtureCatalogEnabled } from "./runtime-capabilities";
import type { AppState, BattleConfig, TargetBuild } from "./types";

export function createInitialAppState(locationSearch: string): AppState {
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
    cardPickerScope: "common",
    pickerSearch: "",
    cardArchiveKind: "all",
    cardArchiveKey: "all",
    cardType: "all",
    frameIndex: 0,
    autoPlaying: false,
    result: null,
    battleStatus: null,
    solverMode: "orderBeam",
    solverResult: null,
    solverCollapsed: false,
    solverStatus: null,
    diagnosticResult: null,
    diagnosticStatus: null,
    fixtureImportOpen: false,
    replayImportTab: "code",
    replayImportCode: "",
    replayImportCandidates: [],
    replayImportStatus: null,
    replayImportDeveloperMode:
      repositoryFixtureCatalogEnabled && new URLSearchParams(locationSearch).get("devReplay") === "1",
    fixtureImportQuery: "",
    fixtureImportId: "",
    recentFixtureIds: [],
    importedFixture: null,
    importedFixtureId: null,
    importedFixtureOrigin: null,
    fixtureConsistency: null,
    error: null,
    savedBuilds: loadSavedBuilds(),
    saveDraftNames: { p1: "", p2: "" },
    selectedBuildIds: { p1: "", p2: "" },
  };
}

export function battleConfigSignature(config: BattleConfig): string {
  const playerSignature = (player: BattleConfig["players"]["p1"]) => [
    player.characterId,
    player.careerName,
    player.dualCareerNames,
    player.level,
    player.lifeModifier,
    player.jiFangshengInitialFateRank,
    player.talents,
    player.fateStrategies,
    player.deck,
    player.buffs,
    player.permanentBuffTempDatas,
    player.handCardIds,
  ];
  return JSON.stringify([
    config.firstPlayerSide,
    config.gameRound,
    playerSignature(config.players.p1),
    playerSignature(config.players.p2),
  ]);
}

export function targetBuildSignature(build: TargetBuild): string {
  const player = build.player;
  return JSON.stringify([
    player.characterId,
    player.careerName,
    player.dualCareerNames,
    player.level,
    player.lifeModifier,
    player.talents,
    player.fateStrategies,
    player.deck,
    player.buffs,
    player.permanentBuffTempDatas,
    player.handCardIds,
  ]);
}

export function targetParametersSignature(state: AppState): string {
  const target = state.target;
  return target
    ? `${target.damageThreshold}|${target.displayRounds}|${state.config.gameRound}`
    : "";
}

export function isTextEditingTarget(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) return false;
  if (target.isContentEditable) return true;
  return Boolean(target.closest("input, textarea, select"));
}
