import type {
  BuffState,
  CardArchiveKind,
  CardDefinition,
  CardElement,
  OriginalCardConfig,
  OriginalReplayFixture,
  PlayerId,
  RuleEvent,
} from "./domain";
import type { ExactDeckSearchResult } from "./solver-contract";
import type { BattleExplanation } from "./battle-explanation";
import type { HookStep } from "./hook-trace";
import type { BattleModuleId } from "./battle-modules";
import type {
  FixtureConsistencyReport,
} from "./fixture-consistency";
import type { ReplayFixtureWithExpected } from "./fixture-contract";
import type { SolverUiMode } from "./solver-ui";
import type { DeckDiagnosticResult } from "./deck-diagnostics";

export type Side = "p1" | "p2";
export type FlowMetric = "life" | "damage";
export type ImportedFixtureOrigin = "catalog" | "local";
export type ReplayImportTab = "code" | "computer" | "package";
export type WorkbenchMode = "duel" | "target";
export type TargetCompareMode = "overlay" | "grid";
export type TargetBuildStatus = "idle" | "running" | "done" | "error";
export type TargetStopReason = "threshold" | "turnLimit";

export interface ReplayImportCandidate {
  readonly id: string;
  readonly recordId: string;
  readonly recordIndex: number;
  readonly recordTimestamp: number | null;
  readonly gameVersion: string;
  readonly recordCodes: readonly string[];
  readonly round: number;
  readonly firstPlayerSide: Side;
  readonly winnerSide: Side;
  readonly actorTurnCount: number;
  readonly hpDeltaP1MinusP2: number;
  readonly p1CharacterId: number;
  readonly p2CharacterId: number;
  readonly fixture: ReplayFixtureWithExpected;
}

export interface ReplayImportStatus {
  readonly state: "idle" | "scanning" | "ready" | "error";
  readonly message: string;
}

/**
 * 卡牌变体的分级方式。
 * - `rarity`：普通卡按 rarity 0/1/2 分一阶/二阶/三阶，DeckSlotConfig.level 取 0..2。
 * - `realm`：梦境卡按境界（LianQi/ZhuJi/JinDan/YuanYing/HuaShen）分级，
 *   每个变体是独立整牌 id（境界编码在 id 万位），无 rarity 字段；
 *   DeckSlotConfig.level 取 0..4 对应五个境界变体。
 * - `single`：只有一个变体（noUpgrade 卡，如遗迹法器），无等级切换，
 *   level 恒为 0，不显示阶位/境界控件。
 */
export type CardVariantMode = "rarity" | "realm" | "single";

export interface CardVariantOption {
  readonly id: number;
  readonly rarity: number;
  /** 境界分级变体的境界名（仅 realm 模式变体有值，如 "LianQi"）。 */
  readonly realm?: string;
  readonly label: string;
  readonly config: OriginalCardConfig;
  readonly definition: CardDefinition;
}

export interface CardOption {
  readonly baseId: number;
  readonly name: string;
  readonly group: string;
  readonly groupType: string;
  readonly implemented: boolean;
  readonly archiveKind: CardArchiveKind;
  readonly archiveKey: string;
  readonly archiveLabel: string;
  readonly realm?: string;
  readonly realmLabel?: string;
  readonly type: string;
  readonly desc: string;
  readonly variants: readonly CardVariantOption[];
  /** 变体分级方式，决定 level 如何映射到具体变体与标签。 */
  readonly variantMode: CardVariantMode;
}

export interface CharacterOption {
  readonly id: number;
  readonly name: string;
  readonly sectName: string;
  readonly talentIds: readonly number[];
}

export interface CareerOption {
  readonly id: string;
  readonly name: string;
}

export interface TalentOption {
  readonly id: number;
  readonly name: string;
  readonly desc?: string;
  readonly otherParams?: readonly number[];
  readonly levelName?: string;
  readonly status?: string;
  readonly archiveKind?: string;
  readonly archiveKey?: string;
  readonly archiveLabel?: string;
}

export interface TalentGroup {
  readonly id: string;
  readonly label: string;
  readonly options: readonly TalentOption[];
  readonly open: boolean;
}

export interface TalentSlotOption extends TalentOption {
  readonly locked: boolean;
  readonly label: string;
}

export interface FateStrategyOption {
  readonly id: number;
  readonly name: string;
  readonly archiveKey: string;
  readonly archiveLabel: string;
  readonly section: string;
  readonly sectionLabel: string;
  readonly categoryLabel: string;
  readonly status: string;
}

export interface FateStrategyGroup {
  readonly id: string;
  readonly label: string;
  readonly options: readonly FateStrategyOption[];
}

export interface DeckSlotConfig {
  baseId: number;
  level: number;
  originalConfig?: OriginalCardConfig;
}

export interface PlayerConfig {
  readonly side: Side;
  label: string;
  characterId: number;
  careerName: string | null;
  /** 各境界槽（1~4）的兼修副职，仅副职兼修仙命所在槽有值。 */
  dualCareerNames: Record<number, string>;
  level: number;
  gameRound: number;
  hp: number;
  maxHp: number;
  lifeModifier: number;
  activeSlotCount: number;
  talentResonanceId: number | null;
  jiFangshengInitialFateRank: number;
  defense: number;
  anima: number;
  momentum: number;
  momentumLimit: number;
  agility: number;
  guard: number;
  buffs: BuffState;
  starSlots: number[];
  activatedElements: CardElement[];
  lastElement: CardElement | null;
  talents: number[];
  fateStrategies: number[];
  lingWuCardBaseIds: number[];
  handCardIds: number[];
  lastRoundUsedCardBaseIds: number[];
  lastRoundLife: number;
  lastRoundExp: number;
  talentCardParams: Record<string, number[]>;
  talentTempDatas: Record<string, number>;
  permanentBuffTempDatas: Record<string, number>;
  deck: DeckSlotConfig[];
}

export interface BattleConfig {
  sourceKind?: "original-fixture";
  firstPlayerSide: Side;
  gameRound: number;
  maxActorTurns: number;
  decisionTape: number[];
  randomFallbackTape: number[];
  replayMetadata?: ReplayFixtureMetadata;
  players: Record<Side, PlayerConfig>;
}

export interface ReplayFixtureMetadata {
  readonly source?: OriginalReplayFixture["source"];
  readonly catalogCards?: OriginalReplayFixture["catalogCards"];
  readonly historicalCardOverrides?: OriginalReplayFixture["historicalCardOverrides"];
}

export interface SavedBuild {
  readonly id: string;
  name: string;
  updatedAt: string;
  player: SavedPlayerConfig;
}

export interface SavedPlayerConfig {
  characterId: number;
  careerName: string | null;
  dualCareerNames?: Record<number, string>;
  level: number;
  hp: number;
  maxHp: number;
  lifeModifier?: number;
  talentResonanceId: number | null;
  jiFangshengInitialFateRank?: number;
  defense: number;
  anima: number;
  momentum: number;
  momentumLimit: number;
  agility: number;
  guard: number;
  buffs: BuffState;
  starSlots: number[];
  activatedElements: CardElement[];
  lastElement: CardElement | null;
  talents: number[];
  fateStrategies: number[];
  lingWuCardBaseIds: number[];
  handCardIds: number[];
  lastRoundUsedCardBaseIds: number[];
  lastRoundLife: number;
  lastRoundExp: number;
  talentCardParams: Record<string, number[]>;
  talentTempDatas: Record<string, number>;
  permanentBuffTempDatas: Record<string, number>;
  deck: DeckSlotConfig[];
}

export interface SlotView {
  readonly index: number;
  cardId: number;
  baseId: number;
  name: string;
  skipped: boolean;
  hadUsed: boolean;
  temporarilyUpgraded: boolean;
}

export interface PlayerView {
  readonly id: PlayerId;
  readonly name: string;
  readonly side: Side;
  hp: number;
  maxHp: number;
  defense: number;
  anima: number;
  momentum: number;
  momentumLimit: number;
  agility: number;
  guard: number;
  buffs: Record<string, number>;
  sustainValues: Record<string, readonly number[]>;
  starSlots: number[];
  activatedElements: CardElement[];
  lastElement: CardElement | null;
  cardQueue: number[];
  slots: SlotView[];
}

export interface BattleFrame {
  readonly index: number;
  readonly gameRound: number;
  readonly actionIndex: number | null;
  readonly title: string;
  readonly actorId: PlayerId | null;
  readonly actorTurn: number;
  readonly sourceSlot: number | null;
  readonly cardId: number | null;
  readonly cardName: string | null;
  readonly winnerId: PlayerId | null;
  readonly players: Record<Side, PlayerView>;
  readonly events: RuleEvent[];
  readonly summaries: string[];
}

export interface SimulationResult {
  readonly frames: BattleFrame[];
  readonly events: RuleEvent[];
  readonly winnerId: PlayerId | null;
  readonly warnings: string[];
  readonly finalActorTurn: number;
  readonly actionCount: number;
  /** 胜方视角的 canonical rule-impact 结论层；解释失败时缺省，不影响战斗结果。 */
  readonly explanation?: BattleExplanation;
  /** Rust canonical 钩子链；取不到时缺省，不影响战斗结果。 */
  readonly hookSteps?: readonly HookStep[];
}

/** 打靶模式下「一张卡在一个回合内的伤害贡献」。 */
export interface TargetCardDamage {
  /** 归因卡牌的整牌 id；非攻击步骤（回合开始/结束结算）无卡牌时为 null → 「持续/其他」桶。 */
  readonly cardId: number | null;
  readonly cardName: string | null;
  readonly damage: number;
}

/**
 * 一次出牌事件对木桩造成的伤害增量（打靶阶梯曲线的一个台阶）。
 * 每个伤害事件 = 一张牌的出牌（mainEffect）或一次回合结算伤害（turnStart/turnEnd）。
 * `cumulative` 是该步结算后的累计伤害（23→65→96→133 过程）。
 */
export interface TargetDamageStep {
  /** 该步所属回合（`battleRound = ceil(actorTurn/2)`）。 */
  readonly round: number;
  /** 该步的引擎 actorTurn（用于排序与回合边界判定）。 */
  readonly actorTurn: number;
  /** 该步归因到的卡牌（无卡归因如内伤/持续伤害为 null → 「持续/其他」色）。 */
  readonly cardId: number | null;
  readonly cardName: string | null;
  /** 该步对木桩造成的伤害增量（台阶高度）。 */
  readonly damage: number;
  /** 该步结算后的累计伤害（23→65→96→133 曲线值）。 */
  readonly cumulative: number;
}

/** 打靶模式/伤害曲线按「回合」（双方各动一次）聚合的一根柱。 */
export interface TargetTurnDamage {
  readonly round: number;
  readonly total: number;
  readonly byCard: readonly TargetCardDamage[];
}

/** 单套构筑打靶的完整结果。 */
export interface TargetPracticeResult {
  /** 每回合（round = ceil(actorTurn/2)）的伤害；只含实际发生战斗的回合。 */
  readonly perTurn: readonly TargetTurnDamage[];
  /** 按出牌事件序列的伤害台阶（23→65→96→133 累计过程），用于阶梯曲线。 */
  readonly steps: readonly TargetDamageStep[];
  readonly totalDamage: number;
  readonly stopReason: TargetStopReason;
  /** 首个累计伤害 ≥ 阈值的回合；未达标时为 turnLimit。 */
  readonly reachedTurn: number;
}

/** 打靶模式中的一套玩家构筑（我方 = p1，对手恒为静默木桩）。 */
export interface TargetBuild {
  readonly id: string;
  name: string;
  player: PlayerConfig;
  result?: TargetPracticeResult | null;
  status?: TargetBuildStatus;
  errorMessage?: string | null;
}

export interface TargetPracticeState {
  readonly builds: TargetBuild[];
  /** 当前编辑/选牌聚焦的那套；picker 路由按它决定写回哪套。 */
  activeBuildId: string;
  /** 累计伤害阈值（终点判据），全构筑共用。默认 120。 */
  damageThreshold: number;
  /**
   * 绝对展示回合数：结果裁剪到的回合窗口终点（1..GAME_TURN_LIMIT=32）。
   * 有结果时有效范围为 `[reachedTurn, 32]`：第 4 回合达标就从 4 开始显示，
   * 不提供 0..reachedTurn-1 这类没有完整结果意义的窗口。无结果/推演中不
   * 应使用此值（控件显示禁用等待态）。引擎始终跑满 32 回合（= 64 actorTurn，
   * 原作基础战斗整场上限），UI 按此值裁剪展示窗口。
   */
  displayRounds: number;
  /** 当前有效回合下界；推演重跑期间保留上一份达标回合，避免 range 提交时丢焦点。 */
  displayRoundMin?: number;
  /** range 已提交、等待新的 trace 结果期间保持可操作。 */
  displayRoundPending?: boolean;
  /** 多构筑对比显示方式。 */
  compareMode: TargetCompareMode;
  /** 当前展开明细的出牌台阶序号（点击阶梯段展开该步详情）；null 不展开。 */
  expandedStep: number | null;
  /** 当前展开台阶所属构筑，避免切换构筑后复用旧 index。 */
  expandedStepBuildId?: string | null;
  /** 进入打靶模式时暂存的双方对战 p1 构筑，切回时恢复。 */
  duelP1Player: PlayerConfig | null;
}

export type PickerMode = "none" | "card" | "talent" | "fate" | "career" | "character";
export type CardPickerScope = "common" | "season" | "special";

export interface SolverRunStatus {
  readonly mode: SolverUiMode;
  readonly state: "running" | "done" | "error";
  readonly startedAt?: number;
  readonly elapsedMs?: number;
  readonly maxEvaluations?: number;
  readonly evaluatedCount?: number;
  readonly message?: string;
  readonly requestId?: string;
  /** Kept for old serialized/test states; live Worker runs use requestId. */
  readonly runId?: number;
}

export interface BattleRunStatus {
  readonly state: "running" | "done" | "error";
  readonly startedAt?: number;
  readonly elapsedMs?: number;
  readonly message?: string;
  readonly requestId?: string;
}

export interface DiagnosticRunStatus {
  readonly state: "running" | "done" | "error";
  readonly message?: string;
  readonly requestId?: string;
}

export interface AppState {
  view: "setup" | "battle";
  /** 顶层工作台模式：「双方对战」或「打靶模式」。 */
  workbenchMode: WorkbenchMode;
  /** 打靶模式状态；未进入过打靶模式时为 null。 */
  target?: TargetPracticeState | null;
  config: BattleConfig;
  activeSide: Side;
  pickerMode: PickerMode;
  selectedSlot: number;
  selectedTalentSlot: number;
  cardSearch: string;
  cardPickerScope?: CardPickerScope;
  /** Shared query for character/talent picker search; cleared on each open. */
  pickerSearch?: string;
  cardArchiveKind: CardArchiveKind | "all";
  cardArchiveKey: string;
  cardType: string;
  frameIndex: number;
  autoPlaying: boolean;
  result: SimulationResult | null;
  battleStatus?: BattleRunStatus | null;
  /** 右列当前显示的模块；缺省走 DEFAULT_BATTLE_MODULE。 */
  battleModule?: BattleModuleId;
  /** 生命曲线模块的显示口径；缺省显示生命曲线。 */
  flowMetric?: FlowMetric;
  solverMode?: SolverUiMode;
  solverResult: ExactDeckSearchResult | null;
  solverCollapsed: boolean;
  solverStatus?: SolverRunStatus | null;
  diagnosticResult?: DeckDiagnosticResult | null;
  diagnosticStatus?: DiagnosticRunStatus | null;
  fixtureImportOpen?: boolean;
  replayImportTab?: ReplayImportTab;
  replayImportCode?: string;
  replayImportCandidates?: readonly ReplayImportCandidate[];
  replayImportStatus?: ReplayImportStatus | null;
  replayImportDeveloperMode?: boolean;
  fixtureImportQuery?: string;
  fixtureImportId?: string;
  recentFixtureIds?: string[];
  importedFixture?: ReplayFixtureWithExpected | null;
  importedFixtureId?: string | null;
  importedFixtureOrigin?: ImportedFixtureOrigin | null;
  fixtureConsistency?: FixtureConsistencyReport | null;
  error: string | null;
  savedBuilds: SavedBuild[];
  saveDraftNames: Record<Side, string>;
  selectedBuildIds: Record<Side, string>;
}
