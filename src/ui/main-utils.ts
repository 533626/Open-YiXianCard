import {
  CHARACTER_BY_ID,
  TALENT_OPTION_BY_ID,
  activeSlotCountForProgress,
  characterBaseTalentSlots,
  fateStrategyOptionsForCharacter,
  isCardDisabled,
  isFateStrategyImplemented,
  isFateStrategySelectableForCharacter,
  normalizePlayerTalents,
  scopedCardIndexOptions,
  slotHasDualCareerTalent,
} from "./data";
import {
  JI_FANGSHENG_CHARACTER_ID,
  defaultHpForSetup,
  defaultPhysiqueForPlayer,
  defaultMaxHpForPlayer,
  levelForGameRound,
  maxJiFangshengInitialFateRank,
  normalizePlayerLevel,
  normalizeJiFangshengInitialFateRank,
  physiqueLimitForPlayer,
  PERMANENT_PHYSIQUE_KEY,
} from "./derived-state";
import { CoreBuff } from "./domain";
import type { AppState, BattleConfig, PlayerConfig, Side, TargetBuild } from "./types";

const LINGZHI_CAREER_ID = "LingZhiShi";
const LINGZHI_PERMANENT_KEYS = new Set([
  "10008",
  "10009",
  "10010",
  "10011",
  "10013",
  "10014",
  "10017",
  "10020",
]);
const PHYSIQUE_BUFF = CoreBuff.Physique;
const CONFIG_MUTATING_ACTIONS = new Set([
  "apply-character-talents",
  "apply-solver-best",
  "adjust-jifangsheng-rank",
  "clear-deck",
  "clear-fate-strategies",
  "clear-slot",
  "clear-talent-slot",
  "cycle-level",
  "load-build",
  "pick-card",
  "pick-career",
  "pick-character",
  "pick-dual-career",
  "pick-saved-build",
  "pick-talent",
  "reset-player",
  "set-first",
  "shift-deck-slot",
  "slot-level",
  "toggle-fate-strategy",
]);

/** 改动当前卡组/角色/仙命/天衍，使当前构筑不再等于任一已保存存档。 */
const DECK_EDITING_ACTIONS = new Set([
  "adjust-jifangsheng-rank",
  "apply-character-talents",
  "apply-solver-best",
  "apply-solver-row",
  "apply-solver-baseline",
  "clear-deck",
  "clear-fate-strategies",
  "clear-slot",
  "clear-talent-slot",
  "cycle-level",
  "pick-card",
  "pick-character",
  "pick-talent",
  "reset-player",
  "shift-deck-slot",
  "toggle-fate-strategy",
]);

export function isDeckEditingAction(action: string): boolean {
  return DECK_EDITING_ACTIONS.has(action);
}

export function clearBuildSelection(state: AppState, side: Side): void {
  if (state.selectedBuildIds[side] || state.saveDraftNames[side]) {
    state.selectedBuildIds[side] = "";
    state.saveDraftNames[side] = "";
  }
}

export function clearDeckSlot(state: AppState, side: Side, slot: number): void {
  state.config.players[side].deck[slot] = { baseId: 0, level: 0 };
}

export function clearPlayerDeck(state: AppState, side: Side): void {
  const player = state.config.players[side];
  for (let index = 0; index < player.deck.length; index += 1) {
    player.deck[index] = { baseId: 0, level: 0 };
  }
}

export function shiftDeckSlot(state: AppState, side: Side, slot: number, delta: -1 | 1): void {
  const target = slot + delta;
  if (target < 0 || target >= state.config.players[side].deck.length) return;
  const deck = state.config.players[side].deck;
  const current = deck[slot]!;
  deck[slot] = deck[target]!;
  deck[target] = current;
}

export function reorderDeckSlot(
  state: AppState,
  side: Side,
  from: number,
  target: number,
  mode: "swap" | "insert-before" | "insert-after",
): number | null {
  const deck = state.config.players[side].deck;
  if (
    !Number.isInteger(from)
    || !Number.isInteger(target)
    || from < 0
    || target < 0
    || from >= deck.length
    || target >= deck.length
  ) return null;
  if (mode === "swap") {
    if (from === target) return from;
    [deck[from], deck[target]] = [deck[target]!, deck[from]!];
    return target;
  }

  let insertionIndex = target + (mode === "insert-after" ? 1 : 0);
  const [moved] = deck.splice(from, 1);
  if (!moved) return null;
  if (from < insertionIndex) insertionIndex -= 1;
  insertionIndex = Math.max(0, Math.min(deck.length, insertionIndex));
  deck.splice(insertionIndex, 0, moved);
  return insertionIndex;
}

/** 场上已摆牌数：`baseId > 0` 的卡槽才算数，空槽不计。 */
export function configuredFieldCardCount(player: PlayerConfig): number {
  return player.deck.filter((slot) => slot.baseId > 0).length;
}

/**
 * 轻量就绪闸（只管自动推演，不影响显式“开战”或导入后跑）：
 * 手动构筑要求双方都选了角色且双方场上各至少 1 张牌，避免刚选角色、牌还没摆
 * 就先跑一场空对局；导入对局直接放行——其卡组由 fixture 承载，不必再要求摆牌。
 */
export function battleAutoRunReady(state: AppState): boolean {
  const p1 = state.config.players.p1;
  const p2 = state.config.players.p2;
  if (p1.characterId <= 0 || p2.characterId <= 0) return false;
  if (state.importedFixture) return true;
  return configuredFieldCardCount(p1) >= 1 && configuredFieldCardCount(p2) >= 1;
}

/**
 * 自动推演调度闸：既要满足就绪闸，也不能有任何选择浮层挡着——
 * 选卡/选仙命/选角色期间都不推演，等用户手动关闭浮层后再更新。
 * 选牌本身不推迟，只是右侧推演等浮层退出再跑。
 */
export function shouldScheduleAutoBattle(state: AppState): boolean {
  return state.pickerMode === "none" && battleAutoRunReady(state);
}

export function invalidateComputedResults(state: AppState): void {
  state.result = null;
  state.battleStatus = null;
  state.frameIndex = 0;
  state.fixtureConsistency = null;
  state.solverResult = null;
  state.solverStatus = null;
  state.diagnosticResult = null;
  state.diagnosticStatus = null;
  // 打靶模式：构筑内容编辑只影响当前聚焦构筑（镜像），其它构筑的结果仍然有效；
  // 共享参数（阈值/回合上限/修炼轮）变更走 invalidateAllTargetBuilds。
  invalidateActiveTargetBuild(state);
}

/**
 * 打靶模式：只作废当前聚焦构筑的结果。构筑内容编辑（选卡/仙命/副职/等级等）
 * 只影响它，不应连带清掉其它构筑仍有效的图。
 */
export function invalidateActiveTargetBuild(state: AppState): void {
  const target = state.target;
  if (!target) return;
  const active = target.builds.find((build) => build.id === target.activeBuildId)
    ?? target.builds[0];
  if (active) {
    invalidateTargetBuildResult(active);
    if (target.expandedStepBuildId === active.id) {
      target.expandedStep = null;
      target.expandedStepBuildId = null;
    }
  }
}

/** 打靶模式：作废全部构筑结果（阈值/回合上限/修炼轮等共享变更、模式切换）。 */
export function invalidateAllTargetBuilds(state: AppState): void {
  const target = state.target;
  for (const build of target?.builds ?? []) invalidateTargetBuildResult(build);
  if (target) {
    target.expandedStep = null;
    target.expandedStepBuildId = null;
  }
}

function invalidateTargetBuildResult(build: TargetBuild): void {
  build.result = null;
  build.errorMessage = null;
  if (build.status === "error") build.status = "idle";
  // running 不动：在途推演由 runTargetPractice 的签名/令牌校验决定是否作废。
}

/**
 * 打靶模式镜像收口：`config.players.p1` 必须是当前聚焦构筑的 player（同一对象）。
 * 个别路径（存档下拉读档、导入回放等）会整体替换 config.players.p1，绕过
 * handleAction 的收口，必须在这里重新挂接，否则后续编辑写丢。
 */
export function resyncTargetMirror(state: AppState): void {
  if (state.workbenchMode !== "target" || !state.target) return;
  const target = state.target;
  const active = target.builds.find((build) => build.id === target.activeBuildId)
    ?? target.builds[0];
  if (active) active.player = state.config.players.p1;
}

export function actionMutatesConfig(action: string): boolean {
  return CONFIG_MUTATING_ACTIONS.has(action);
}

/**
 * 同步兼修副职：只有副职兼修仙命所在槽位（1~4）可以保留兼修值，
 * 其余槽位的值清空。与主副职重复的兼修也清空。
 */
export function syncDualCareers(player: PlayerConfig): void {
  for (const slot of [1, 2, 3, 4]) {
    if (!slotHasDualCareerTalent(player, slot)) {
      delete player.dualCareerNames[slot];
    }
  }
  // 与主副职重复的兼修无意义；跨槽也不重复选同一副职
  const seen = new Set<string>();
  if (player.careerName) seen.add(player.careerName);
  for (const slot of [1, 2, 3, 4]) {
    const key = player.dualCareerNames[slot];
    if (!key) continue;
    if (seen.has(key)) delete player.dualCareerNames[slot];
    else seen.add(key);
  }
}

export function sanitizePlayerScope(player: PlayerConfig): void {
  // 清理兼修副职：移除非副职兼修槽位上的值，以及与主副职重复的值。
  syncDualCareers(player);
  const allowedCards = new Set(
    scopedCardIndexOptions(player.characterId, player.careerName, player.talents, player.dualCareerNames)
      .filter((card) => card.implemented && !isCardDisabled(card))
      .map((card) => card.baseId),
  );
  for (const [index, slot] of player.deck.entries()) {
    if (slot.baseId !== 0 && !allowedCards.has(slot.baseId)) {
      player.deck[index] = { baseId: 0, level: 0 };
    }
  }
  const allowedStrategies = new Set(
    fateStrategyOptionsForCharacter(player.characterId)
      .filter(isFateStrategyImplemented)
      .map((option) => option.id),
  );
  player.fateStrategies = player.fateStrategies
    .filter((strategyId) =>
      allowedStrategies.has(strategyId) &&
      isFateStrategySelectableForCharacter(player.characterId, strategyId),
    )
    .sort((left, right) => left - right);
  normalizePlayerTalents(player);
  if (player.careerName !== LINGZHI_CAREER_ID) {
    for (const key of LINGZHI_PERMANENT_KEYS) delete player.permanentBuffTempDatas[key];
  }
  syncPlayerDerivedStats(player, player.gameRound, player.buffs[PHYSIQUE_BUFF] === undefined);
}

export function characterBaseTalentIds(characterId: number): number[] {
  return characterBaseTalentSlots(characterId).map((option) => option.id);
}

export function resetCardFilters(state: AppState): void {
  state.cardArchiveKind = "all";
  state.cardArchiveKey = "all";
  state.cardType = "all";
  state.cardSearch = "";
  state.pickerSearch = "";
}

export function clampGameRound(value: number): number {
  if (Number.isNaN(value) || value <= 1) return 1;
  if (value >= 99) return 99;
  return Math.trunc(value);
}

export function normalizeBattleConfig(config: BattleConfig): void {
  syncBattleProgress(config);
}

export function syncBattleProgress(config: BattleConfig): void {
  const gameRound = clampGameRound(config.gameRound);
  config.gameRound = gameRound;
  for (const player of Object.values(config.players)) {
    syncPlayerDerivedStats(player, gameRound, false);
  }
}

export function applyGameRoundDefaults(config: BattleConfig): void {
  syncBattleProgress(config);
  const level = levelForGameRound(config.gameRound);
  for (const player of Object.values(config.players)) {
    player.level = level;
    syncPlayerDerivedStats(player, config.gameRound, false);
  }
}

export function applyPhysiqueValue(player: PlayerConfig, nextValue: number): void {
  const limit = physiqueLimitForPlayer(player, player.gameRound);
  const nextPhysique = Math.min(limit, Math.max(0, Math.trunc(nextValue)));
  if (nextPhysique === 0) delete player.buffs[PHYSIQUE_BUFF];
  else player.buffs[PHYSIQUE_BUFF] = nextPhysique;
  syncPlayerDerivedStats(player, player.gameRound, false);
}

export function applyJiFangshengInitialFateRank(player: PlayerConfig, nextValue: number): void {
  if (player.characterId !== JI_FANGSHENG_CHARACTER_ID) return;
  player.jiFangshengInitialFateRank = normalizeJiFangshengInitialFateRank(nextValue, player.gameRound);
  syncPlayerDerivedStats(player, player.gameRound, false);
}

export function defaultJiFangshengInitialFateRank(characterId: number, gameRound = 99): number {
  return characterId === JI_FANGSHENG_CHARACTER_ID ? maxJiFangshengInitialFateRank(gameRound) : 0;
}

export function syncPlayerDerivedStats(
  player: PlayerConfig,
  gameRound: number,
  resetPhysique: boolean,
): void {
  player.gameRound = clampGameRound(gameRound);
  player.level = normalizePlayerLevel(player.level);
  player.lifeModifier = Number.isFinite(player.lifeModifier) ? Math.trunc(player.lifeModifier) : 0;
  player.activeSlotCount = activeSlotCountForProgress(player.gameRound, player.level);
  syncTalentSlotsForLevel(player);
  syncDuanXuanPhysiqueDefaults(player, player.gameRound, resetPhysique);
  const baseHp = defaultHpForSetup(player.level, player.gameRound, player.lifeModifier);
  player.hp = baseHp;
  player.maxHp = Math.max(0, defaultMaxHpForPlayer(player, player.gameRound));
  syncNonDuanXuanHpMax(player);
}

export function syncDuanXuanPhysiqueDefaults(
  player: PlayerConfig,
  gameRound: number,
  resetPhysique: boolean,
): void {
  player.jiFangshengInitialFateRank = defaultJiFangshengInitialFateRank(player.characterId, gameRound) === 0
    ? 0
    : normalizeJiFangshengInitialFateRank(player.jiFangshengInitialFateRank, gameRound);
  if (!isDuanXuanPlayer(player)) {
    delete player.buffs[CoreBuff.PhysiqueLimit];
    delete player.buffs[PHYSIQUE_BUFF];
    mirrorPhysiqueToEngineChannel(player, 0);
    return;
  }
  const limit = physiqueLimitForPlayer(player, gameRound);
  player.buffs[CoreBuff.PhysiqueLimit] = limit;
  if (resetPhysique) {
    player.buffs[PHYSIQUE_BUFF] = defaultPhysiqueForPlayer(player, gameRound);
  }
  const current = player.buffs[PHYSIQUE_BUFF] ?? 0;
  const clamped = Math.min(limit, Math.max(0, Math.trunc(current)));
  player.buffs[PHYSIQUE_BUFF] = clamped;
  mirrorPhysiqueToEngineChannel(player, clamped);
}

/**
 * `buffs[Physique]` 只喂界面；引擎读的是 `permanentBuffTempDatas`
 * （`engine-rust/src/replay/support.rs::permanent_physique_key`）。
 * 两路镜像只在这一个地方做：曾经界面上设好的体魄在开战瞬间消失，
 * 就是因为写入点绕过了这里。非断玄宗角色必须连永久通道一起清掉，
 * 否则会给引擎一个界面上根本看不到的体魄。
 */
function mirrorPhysiqueToEngineChannel(player: PlayerConfig, physique: number): void {
  if (physique > 0) player.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY] = physique;
  else delete player.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY];
}

export function syncNonDuanXuanHpMax(player: PlayerConfig): void {
  if (isDuanXuanPlayer(player)) return;
  player.maxHp = player.hp;
}

export function isDuanXuanPlayer(player: PlayerConfig): boolean {
  return CHARACTER_BY_ID.get(player.characterId)?.sectName === "DuanXuanZong";
}

function syncTalentSlotsForLevel(player: PlayerConfig): void {
  normalizePlayerTalents(player);
  for (let index = Math.max(1, player.level); index < player.talents.length; index += 1) {
    player.talents[index] = 0;
  }
}
