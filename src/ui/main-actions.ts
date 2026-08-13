import { battleModuleFromValue } from "./battle-modules";
import {
  CARD_OPTION_BY_BASE_ID,
  DEFAULT_CAREER_ID,
  TALENT_OPTION_BY_ID,
  canPickCardForDeckSlot,
  defaultPlayerConfig,
  fateStrategyOptionsForCharacter,
  isCardDisabled,
  isFateStrategyImplemented,
  isTalentSelectableForCharacter,
  lockedBaseTalentId,
  normalizePlayerTalents,
  slotHasDualCareerTalent,
} from "./data";
import { normalizeBaseId } from "./domain";
import {
  fixtureEntryById,
  filterFixtureEntries,
  type UiFixtureEntry,
} from "./fixture-catalog";
import {
  applyImportedReplay,
} from "./replay-import";
import { loadRepositoryReplayFixture } from "./repository-replay-loader";
import {
  deleteSelectedBuild,
  loadSelectedBuild,
  saveCurrentBuild,
} from "./main-storage";
import {
  applyJiFangshengInitialFateRank,
  actionMutatesConfig,
  characterBaseTalentIds,
  clearBuildSelection,
  clearDeckSlot,
  clearPlayerDeck,
  invalidateComputedResults,
  isDeckEditingAction,
  resyncTargetMirror,
  resetCardFilters,
  sanitizePlayerScope,
  shiftDeckSlot,
  syncDualCareers,
  syncPlayerDerivedStats,
} from "./main-utils";
import { buildReplayFixture } from "./replay-fixture-builder";
import {
  isSolverSearchMethod,
  isSolverUiTask,
  SOLVER_PRESETS,
  solverMethodForMode,
  solverModeFor,
  solverTaskForMode,
  type SolverUiMode,
} from "./solver-ui";
import { visibleErrorMessage } from "./view-utils";
import type {
  AppState,
  ReplayImportTab,
  Side,
  SimulationResult,
} from "./types";
import { repositoryFixtureCatalogEnabled } from "./runtime-capabilities";
import {
  workbenchWorkerClient,
  type WorkbenchWorkerClient,
} from "./worker-client";
import type { WorkbenchSolvePayload } from "./worker-protocol";
import { solverCardPoolOptions } from "./solver-card-pool";
import {
  applySolverBaseline,
  applySolverBest,
  applySolverRow,
  cancelSolver,
  scheduleDeckDiagnostics,
  scheduleSolver,
} from "./main-solver-actions";

export {
  applySolverBaseline,
  applySolverBest,
  applySolverRow,
  buildSolverPayload,
  scheduleDeckDiagnostics,
  scheduleSolver,
} from "./main-solver-actions";
export {
  activeTargetBuild,
  addTargetBuild,
  createTargetPracticeState,
  duplicateTargetBuild,
  removeTargetBuild,
  renameTargetBuild,
  selectTargetBuild,
  setTargetCompareMode,
  switchWorkbenchMode,
  targetPracticeState,
} from "./target-workbench-state";
import {
  activeTargetBuild,
  addTargetBuild,
  duplicateTargetBuild,
  removeTargetBuild,
  renameTargetBuild,
  selectTargetBuild,
  setTargetCompareMode,
  switchWorkbenchMode,
} from "./target-workbench-state";

export type ActionContext = {
  readonly state: AppState;
  readonly render: () => void;
  readonly runBattle: () => void | Promise<void>;
  readonly cancelBattle?: () => void;
  readonly workerClient?: WorkbenchWorkerClient;
  readonly resetBattle: () => void;
  readonly stopAuto: () => void;
  readonly toggleAuto: () => void;
  readonly maybeScheduleAutoBattle?: () => void;
  readonly adjacentCompletedTurnFrameIndex: (
    result: SimulationResult,
    currentIndex: number,
    direction: -1 | 1,
  ) => number;
  /** 打靶模式：跑指定构筑的打靶推演（main.ts 持有 worker 生命周期）。 */
  readonly runTargetPractice?: (buildId: string) => void | Promise<void>;
  readonly cancelTargetPractice?: () => void;
};

const SOLVER_VALUE_SCORING = { scoreProfile: "value-v0" as const };
export function handleAction(event: Event, context: ActionContext): void {
  const target = event.currentTarget as HTMLElement;
  const action = target.dataset.action;
  const side = target.dataset.side as Side | undefined;
  const slot = target.dataset.slot === undefined ? null : Number(target.dataset.slot);
  if (!action) return;
  // cycle-level / slot-level 由调用方（main.ts 监听器）防抖处理 invalidate + render，
  // 这里不再走统一的 invalidate + render 路径。
  if (action === "cycle-level" || action === "slot-level") {
    if (action === "cycle-level" && side && slot !== null) cycleCardLevel(context.state, side, slot);
    if (action === "slot-level" && side && slot !== null) {
      setCardLevel(context.state, side, slot, Number((target as HTMLSelectElement).value));
    }
    if (isDeckEditingAction(action)) clearBuildSelection(context.state, side ?? context.state.activeSide);
    return;
  }
  if (actionMutatesConfig(action) && action !== "apply-solver-best") {
    invalidateComputedResults(context.state);
  }

  if (action === "run") {
    void context.runBattle();
    return;
  }
  if (action === "diagnose-decks") {
    void scheduleDeckDiagnostics(context);
    return;
  }
  if (action === "cancel-battle") {
    context.cancelBattle?.();
    return;
  }
  if (action === "reset") return context.resetBattle();
  if (action === "toggle-fixture-import") {
    context.state.fixtureImportOpen = !context.state.fixtureImportOpen;
  }
  if (action === "set-replay-import-tab") {
    const tab = target.dataset.importTab as ReplayImportTab | undefined;
    if (tab === "code" || tab === "computer" || tab === "package") {
      context.state.replayImportTab = tab;
    }
  }
  if (action === "import-local-replay-round") {
    const candidateId = target.dataset.replayCandidateId;
    const candidate = context.state.replayImportCandidates
      ?.find((item) => item.id === candidateId);
    if (candidate) {
      applyImportedReplay(context.state, candidate.fixture, { origin: "local" });
      // 导入会整体替换 config.players.p1：打靶模式下必须重新挂镜像。
      resyncTargetMirror(context.state);
    }
    context.render();
    return;
  }
  if (action === "import-fixture") {
    void importFixture(context, false);
    return;
  }
  if (action === "import-fixture-and-run") {
    void importFixture(context, true);
    return;
  }
  if (action === "quick-fixture") {
    const fixtureId = target.dataset.fixtureId;
    if (fixtureId) void importFixtureById(context, fixtureId, true);
    return;
  }
  if (action === "set-solver-task") setSolverTask(target, context.state);
  if (action === "set-solver-method") setSolverMethod(target, context.state);
  if (action === "solve-active") {
    void scheduleSolver(context);
    return;
  }
  if (action === "cancel-solver") {
    cancelSolver(context);
    return;
  }
  if (action === "apply-solver-best") {
    applySolverBest(context.state);
    invalidateComputedResults(context.state);
  }
  if (action === "apply-solver-row") {
    applySolverRow(target, context.state);
    invalidateComputedResults(context.state);
  }
  if (action === "apply-solver-baseline") {
    applySolverBaseline(context.state);
    invalidateComputedResults(context.state);
  }

  if (action === "set-first" && side) context.state.config.firstPlayerSide = side;
  if (action === "switch-workbench-mode") {
    const mode = target.dataset.mode;
    if (mode === "duel" || mode === "target") switchWorkbenchMode(context.state, mode);
  }
  if (action === "select-target-build") {
    const buildId = target.dataset.buildId;
    if (buildId) selectTargetBuild(context.state, buildId);
  }
  if (action === "add-target-build") addTargetBuild(context.state);
  if (action === "remove-target-build") {
    const buildId = target.dataset.buildId;
    if (buildId) removeTargetBuild(context.state, buildId);
  }
  if (action === "duplicate-target-build") {
    const buildId = target.dataset.buildId;
    if (buildId) duplicateTargetBuild(context.state, buildId);
  }
  if (action === "rename-target-build") {
    const buildId = target.dataset.buildId;
    if (buildId) renameTargetBuild(context.state, buildId, (target as HTMLInputElement).value);
  }
  if (action === "run-target-practice") {
    const buildId = target.dataset.buildId;
    if (buildId) void context.runTargetPractice?.(buildId);
  }
  if (action === "cancel-target-practice") context.cancelTargetPractice?.();
  if (action === "set-target-compare-mode") {
    const mode = target.dataset.mode;
    if (mode === "overlay" || mode === "grid") setTargetCompareMode(context.state, mode);
  }
  if (action === "toggle-target-step" && context.state.target) {
    const step = Number(target.dataset.step);
    const buildId = target.dataset.buildId ?? activeTargetBuild(context.state)?.id ?? null;
    if (Number.isInteger(step) && step >= 0 && buildId) {
      const targetState = context.state.target;
      if (!targetState.builds.some((build) => build.id === buildId)) return;
      if (targetState.activeBuildId !== buildId) selectTargetBuild(context.state, buildId);
      const sameSelection = targetState.expandedStepBuildId === buildId && targetState.expandedStep === step;
      targetState.expandedStep = sameSelection ? null : step;
      targetState.expandedStepBuildId = sameSelection ? null : buildId;
    }
  }
  if (action === "select-build" && side) selectBuild(target as HTMLSelectElement, context.state, side);
  if (action === "save-build" && side) saveCurrentBuild(context.state, side);
  if (action === "load-build" && side) loadSelectedBuild(context.state, side);
  if (action === "delete-build" && side) deleteSelectedBuild(context.state, side);
  if (action === "toggle-build-archive" && side) {
    event.stopPropagation();
    const root = document.getElementById("app");
    const wrap = target.closest(".build-archive-wrap");
    const wasOpen = wrap?.classList.contains("open") ?? false;
    root?.querySelectorAll(".build-archive-wrap.open").forEach((element) => element.classList.remove("open"));
    if (!wasOpen) wrap?.classList.add("open");
    return;
  }
  if (action === "pick-saved-build" && side) {
    const buildId = target.dataset.buildId;
    const build = buildId
      ? context.state.savedBuilds.find((entry) => entry.id === buildId)
      : undefined;
    if (build) {
      context.state.saveDraftNames[side] = build.name;
      context.state.selectedBuildIds[side] = build.id;
      loadSelectedBuild(context.state, side);
    }
  }
  if (action === "set-picker-mode") setPickerMode(target, context.state);
  if (
    action === "close-card-picker" ||
    action === "close-talent-picker" ||
    action === "close-fate-picker" ||
    action === "close-career-picker" ||
    action === "close-character-picker"
  ) {
    context.state.pickerMode = "none";
    // 构筑弹窗全部收回即立即调度自动推演，不等下一次渲染周期。
    context.maybeScheduleAutoBattle?.();
  }

  if (action === "select-slot" && side && slot !== null) selectSlot(context.state, side, slot);
  if (action === "set-card-picker-scope") {
    const scope = target.dataset.scope;
    if (scope === "common" || scope === "season" || scope === "special") {
      context.state.cardPickerScope = scope;
    }
  }
  if (action === "select-talent-slot" && side && slot !== null && slot > 0) selectTalentSlot(context.state, side, slot);
  if (action === "open-fate-picker" && side) openFatePicker(context.state, side);
  if (action === "open-character-picker" && side) openCharacterPicker(context.state, side);
  if (action === "pick-character") pickCharacter(target, context.state);
  if (action === "pick-career") pickCareer(target, context.state);
  if (action === "pick-dual-career") pickDualCareer(target, context.state);
  if (action === "clear-slot" && side && slot !== null) clearSlot(context.state, side, slot);
  if (action === "clear-deck" && side) clearDeck(context.state, side);
  if (action === "adjust-jifangsheng-rank" && side) adjustJiFangshengRank(target, context.state, side);
  if (action === "shift-deck-slot" && side && slot !== null) {
    const delta = Number(target.dataset.delta) === 1 ? 1 : -1;
    shiftDeckSlot(context.state, side, slot, delta);
    context.state.activeSide = side;
    context.state.selectedSlot = slot + delta;
  }
  if (action === "reset-card-picker") {
    resetCardFilters(context.state);
    context.state.pickerMode = "card";
  }
  if (action === "pick-card") pickCard(target, context.state);
  if (action === "pick-talent") pickTalent(target, context.state);
  if (action === "toggle-fate-strategy") toggleFateStrategy(target, context.state, side);
  if (action === "clear-fate-strategies" && side) clearFateStrategies(context.state, side);
  if (action === "clear-talent-slot" && side && slot !== null && slot > 0) clearTalentSlot(context.state, side, slot);
  if (action === "apply-character-talents" && side) applyCharacterTalents(context.state, side);
  if (action === "reset-player" && side) resetPlayer(context.state, side);
  if (action === "jump-frame" && context.state.result) jumpFrame(target, context.state);
  if (action === "select-battle-module") {
    const module = battleModuleFromValue(target.dataset.module);
    if (module) context.state.battleModule = module;
  }
  if (action === "select-trajectory-metric") {
    const metric = target.dataset.metric;
    // 曲线选项卡的 生命/伤害 分段开关：动作名显式说明它既切到曲线模块又选口径，
    // 不再在共享 handler 里隐式副作用地切换 battleModule。
    if (metric === "life" || metric === "damage") {
      context.state.battleModule = "trajectory";
      context.state.flowMetric = metric;
    }
  }
  if (action === "toggle-auto") context.toggleAuto();
  if (action === "show-setup") {
    context.stopAuto();
    context.state.view = "setup";
  }
  if (action === "show-battle" && context.state.result) context.state.view = "battle";
  if (isDeckEditingAction(action)) {
    clearBuildSelection(context.state, side ?? context.state.activeSide);
  }
  // 打靶模式镜像收口：个别动作（选角色/重置/读档/导入）会整体替换 config.players.p1，
  // 动作结束后必须保证它仍是当前聚焦构筑的 player，否则后续编辑会写丢。
  resyncTargetMirror(context.state);
  context.render();
}

async function importFixture(context: ActionContext, runAfterImport: boolean): Promise<void> {
  try {
    if (!repositoryFixtureCatalogEnabled) {
      throw new Error("托管站无内置回放；请显式选择本地版本化 JSON。");
    }
    const entry = selectedFixtureEntry(context.state);
    if (!entry) throw new Error("请选择 fixture");
    await importFixtureById(context, entry.id, runAfterImport);
  } catch (error) {
    context.state.error = visibleErrorMessage(error);
    context.render();
  }
}

async function importFixtureById(context: ActionContext, id: string, runAfterImport: boolean): Promise<void> {
  try {
    if (!repositoryFixtureCatalogEnabled) {
      throw new Error("托管站无内置回放；请显式选择本地版本化 JSON。");
    }
    const loaded = await loadRepositoryReplayFixture(id);
    applyImportedReplay(context.state, loaded.fixture, {
      origin: "catalog",
      id: loaded.entry.id,
    });
    // 导入整体替换 config：打靶模式下重新挂镜像，避免后续编辑写丢。
    resyncTargetMirror(context.state);
    context.render();
    if (runAfterImport) void context.runBattle();
  } catch (error) {
    context.state.error = visibleErrorMessage(error);
    context.render();
  }
}

function selectedFixtureEntry(state: AppState): UiFixtureEntry | undefined {
  return fixtureEntryById(state.fixtureImportId ?? "") ??
    fixtureEntryById(state.fixtureImportQuery ?? "") ??
    filterFixtureEntries(state.fixtureImportQuery ?? "")[0];
}

function setSolverTask(target: HTMLElement, state: AppState): void {
  const task = target.dataset.solverTask;
  if (!isSolverUiTask(task)) return;
  const current = state.solverMode ?? "orderBeam";
  state.solverMode = solverModeFor(
    task,
    task === "pool" ? "heuristic" : solverMethodForMode(current),
  );
  state.solverResult = null;
  state.solverStatus = null;
}

function setSolverMethod(target: HTMLElement, state: AppState): void {
  const method = target.dataset.solverMethod;
  if (!isSolverSearchMethod(method)) return;
  const current = state.solverMode ?? "orderBeam";
  const task = solverTaskForMode(current);
  if (task === "pool" && method === "exhaustive") return;
  state.solverMode = solverModeFor(task, method);
  state.solverResult = null;
  state.solverStatus = null;
}

function selectBuild(target: HTMLSelectElement, state: AppState, side: Side): void {
  const selectedId = target.value;
  state.selectedBuildIds[side] = selectedId;
  state.saveDraftNames[side] = state.savedBuilds.find((build) => build.id === selectedId)?.name ??
    state.saveDraftNames[side];
}

function setPickerMode(target: HTMLElement, state: AppState): void {
  const mode = target.dataset.mode as AppState["pickerMode"];
  if (mode === "none" || mode === "card" || mode === "talent" || mode === "fate" || mode === "career" || mode === "character") {
    state.pickerMode = mode;
  }
}

function selectSlot(state: AppState, side: Side, slot: number): void {
  state.activeSide = side;
  state.pickerMode = "card";
  state.selectedSlot = slot;
  state.pickerSearch = "";
}

function selectTalentSlot(state: AppState, side: Side, slot: number): void {
  if (slot >= state.config.players[side].level) return;
  state.activeSide = side;
  state.pickerMode = "talent";
  state.selectedTalentSlot = slot;
  state.pickerSearch = "";
}

function openFatePicker(state: AppState, side: Side): void {
  state.activeSide = side;
  state.pickerMode = "fate";
}

function openCharacterPicker(state: AppState, side: Side): void {
  state.activeSide = side;
  state.pickerMode = "character";
  state.pickerSearch = "";
}

function pickCharacter(target: HTMLElement, state: AppState): void {
  const characterId = Number(target.dataset.characterId);
  if (!Number.isInteger(characterId)) return;
  const side = state.activeSide;
  const previous = state.config.players[side];
  const next = defaultPlayerConfig(side, characterId, state.config.gameRound);
  next.label = previous.label;
  next.careerName = previous.careerName ?? DEFAULT_CAREER_ID;
  next.dualCareerNames = { ...previous.dualCareerNames };
  sanitizePlayerScope(next);
  state.config.players[side] = next;
  resetCardFilters(state);
  state.selectedTalentSlot = 1;
  state.pickerMode = "talent";
}

function pickCareer(target: HTMLElement, state: AppState): void {
  const side = (target.dataset.side ?? state.activeSide) as Side;
  const careerId = target.dataset.careerId;
  if (!careerId) return;
  const player = state.config.players[side];
  player.careerName = careerId;
  syncDualCareers(player);
  sanitizePlayerScope(player);
  resetCardFilters(state);
  state.activeSide = side;
  invalidateComputedResults(state);
  state.pickerMode = "career";
}

function pickDualCareer(target: HTMLElement, state: AppState): void {
  const side = (target.dataset.side ?? state.activeSide) as Side;
  const slot = Number(target.dataset.slot);
  const careerId = target.dataset.careerId ?? "";
  if (!slot || slot < 1 || slot > 4) return;
  const player = state.config.players[side];
  if (careerId === "") delete player.dualCareerNames[slot];
  else player.dualCareerNames[slot] = careerId;
  syncDualCareers(player);
  sanitizePlayerScope(player);
  resetCardFilters(state);
  state.activeSide = side;
  invalidateComputedResults(state);
  state.pickerMode = "career";
}

export function cycleCardLevel(state: AppState, side: Side, slot: number): void {
  const deckSlot = state.config.players[side].deck[slot]!;
  const card = CARD_OPTION_BY_BASE_ID.get(deckSlot.baseId);
  // 单变体卡无等级切换；梦境卡按境界（5 档）循环；普通卡/未知卡按阶位（3 档）循环。
  if (card?.variantMode === "single") return;
  const max = card?.variantMode === "realm" ? card.variants.length : 3;
  deckSlot.level = (deckSlot.level + 1) % Math.max(1, max);
}

/** 设置指定槽位的卡牌等级（slot-level 下拉选择），不触发 invalidate / render。 */
export function setCardLevel(state: AppState, side: Side, slot: number, level: number): void {
  state.config.players[side].deck[slot]!.level = level;
}

function adjustJiFangshengRank(target: HTMLElement, state: AppState, side: Side): void {
  const player = state.config.players[side];
  const delta = Number(target.dataset.delta);
  applyJiFangshengInitialFateRank(player, player.jiFangshengInitialFateRank + delta);
  state.activeSide = side;
  state.result = null;
}

function clearSlot(state: AppState, side: Side, slot: number): void {
  clearDeckSlot(state, side, slot);
  state.activeSide = side;
  state.selectedSlot = slot;
  state.pickerMode = "none";
}

function clearDeck(state: AppState, side: Side): void {
  clearPlayerDeck(state, side);
  state.activeSide = side;
  state.pickerMode = "none";
}

function pickCard(target: HTMLElement, state: AppState): void {
  const baseId = Number(target.dataset.baseId);
  const card = CARD_OPTION_BY_BASE_ID.get(baseId);
  const deck = state.config.players[state.activeSide].deck;
  if (!card || isCardDisabled(card) || !canPickCardForDeckSlot(card, deck, state.selectedSlot)) return;
  const targetSlot = state.selectedSlot;
  deck[targetSlot] = { baseId, level: deck[targetSlot]?.level ?? 0 };
  // 选卡后直接前进到下一槽，不判断下槽是否已调；已是最后一格则原地停留。
  state.selectedSlot = Math.min(
    targetSlot + 1,
    state.config.players[state.activeSide].activeSlotCount - 1,
  );
  state.pickerMode = "card";
}

function pickTalent(target: HTMLElement, state: AppState): void {
  const talentId = Number(target.dataset.talentId);
  const option = TALENT_OPTION_BY_ID.get(talentId);
  if (!option) return;
  const player = state.config.players[state.activeSide];
  if (!isTalentSelectableForCharacter(player.characterId, talentId)) return;
  normalizePlayerTalents(player);
  const requestedSlot = Number(target.dataset.talentSlot);
  const slot = Number.isInteger(requestedSlot) && requestedSlot > 0
    ? requestedSlot
    : state.selectedTalentSlot;
  if (slot <= 0 || slot >= player.level) return;
  const existing = player.talents.indexOf(talentId);
  if (existing !== -1 && existing !== slot) player.talents[existing] = player.talents[slot] ?? 0;
  player.talents[slot] = talentId;
  player.talents[0] = lockedBaseTalentId(player.characterId);
  sanitizePlayerScope(player);
  syncPlayerDerivedStats(player, state.config.gameRound, true);
  state.selectedTalentSlot = Math.min(slot + 1, player.level - 1);
  // 副职兼修仙命没有「未选」态：选完直接进副职页补齐兼修副职。
  state.pickerMode =
    slotHasDualCareerTalent(player, slot) && !player.dualCareerNames[slot]
      ? "career"
      : "talent";
}

function toggleFateStrategy(target: HTMLElement, state: AppState, side: Side | undefined): void {
  if (side) state.activeSide = side;
  const strategyId = Number(target.dataset.fateStrategyId);
  if (!Number.isInteger(strategyId)) return;
  const player = state.config.players[state.activeSide];
  const option = fateStrategyOptionsForCharacter(player.characterId)
    .find((candidate) => candidate.id === strategyId);
  if (!option || !isFateStrategyImplemented(option)) return;
  const selected = new Set(player.fateStrategies);
  if (selected.has(strategyId)) selected.delete(strategyId);
  else selected.add(strategyId);
  player.fateStrategies = [...selected].sort((left, right) => left - right);
  state.pickerMode = "fate";
}

function clearFateStrategies(state: AppState, side: Side): void {
  state.config.players[side].fateStrategies = [];
  state.activeSide = side;
  state.pickerMode = "fate";
}

function clearTalentSlot(state: AppState, side: Side, slot: number): void {
  const player = state.config.players[side];
  if (slot >= player.level) return;
  normalizePlayerTalents(player);
  player.talents[slot] = 0;
  player.talents[0] = lockedBaseTalentId(player.characterId);
  syncPlayerDerivedStats(player, state.config.gameRound, true);
  state.selectedTalentSlot = slot;
  state.pickerMode = "talent";
}

function applyCharacterTalents(state: AppState, side: Side): void {
  const player = state.config.players[side];
  player.talents = characterBaseTalentIds(player.characterId);
  normalizePlayerTalents(player);
  syncDualCareers(player);
  syncPlayerDerivedStats(player, state.config.gameRound, true);
  state.activeSide = side;
  state.selectedTalentSlot = 1;
  state.pickerMode = "none";
}

function resetPlayer(state: AppState, side: Side): void {
  const characterId = state.config.players[side].characterId;
  state.config.players[side] = defaultPlayerConfig(side, characterId, state.config.gameRound);
  state.activeSide = side;
  state.pickerMode = "none";
}

function jumpFrame(target: HTMLElement, state: AppState): void {
  const frame = Number(target.dataset.frame);
  if (!Number.isInteger(frame) || !state.result) return;
  state.frameIndex = Math.max(0, Math.min(state.result.frames.length - 1, frame));
}
