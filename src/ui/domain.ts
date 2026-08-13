import type {
  CardElement,
  CardTrait,
  OriginalCardConfig,
  OriginalEnumValue,
} from "../../shared/contracts";

export type { CardElement, CardTrait, OriginalCardConfig, OriginalEnumValue };

export type PlayerId = string;
export type BuffState = Partial<Record<string, number>>;
export type CardType = "normal" | "sustain" | "consume" | "refine" | "change";
export type CardArchiveKind =
  | "sect"
  | "career"
  | "exclusive"
  | "chance"
  | "secret"
  | "fate"
  | "season"
  | "common";

export interface CardDefinition {
  readonly id: number;
  readonly baseId?: number;
  readonly name: string;
  readonly rarity?: number;
  readonly desc?: string;
  readonly hidden?: boolean;
  readonly subcategoryName?: string;
  readonly careerName?: string;
  readonly type: CardType;
  readonly traits: readonly CardTrait[];
  readonly animaCost?: number;
  readonly animaGain?: number;
  readonly chargeQi?: number;
  readonly hpCost?: number;
  readonly actionAgain?: boolean;
  readonly attack?: number;
  readonly randomAttack?: number;
  readonly attackCount?: number;
  readonly realmLevel?: number;
  readonly defense?: number;
  readonly randomDefense?: number;
  readonly damage?: number;
  readonly physique?: number;
  readonly swordIntent?: number;
  readonly hexagram?: number;
  readonly otherParams?: readonly number[];
}

export interface RuleEvent {
  readonly sequence: number;
  readonly type:
    | "phase"
    | "resource"
    | "buff"
    | "effectLocal"
    | "queue"
    | "damage"
    | "guard"
    | "checkpoint"
    | "card";
  readonly actorId?: PlayerId;
  readonly targetId?: PlayerId;
  readonly name: string;
  readonly before?: number;
  readonly after?: number;
  readonly amount?: number;
  readonly detail?: Readonly<Record<string, number | string | boolean>>;
}

export interface OriginalReplayPlayerFixture {
  readonly level: number;
  readonly baseMaxHp: number;
  readonly extraMaxHp: number;
  readonly battleStartHp?: number;
  readonly characterId: number;
  readonly careerId?: number;
  readonly talents: readonly number[];
  readonly talentResonanceId?: number;
  readonly activeSlotCount: number;
  readonly initialDefense?: number;
  readonly initialAnima?: number;
  readonly initialGuard?: number;
  readonly initialMomentum?: number;
  readonly initialMomentumLimit?: number;
  readonly initialAgility?: number;
  readonly initialBattleBuffs?: Readonly<Partial<Record<string, number>>>;
  readonly handCards?: readonly number[];
  readonly usedKeYinCards?: readonly number[];
  readonly lastRoundUsedCardBaseIds?: readonly number[];
  readonly lastRoundLife?: number;
  readonly lastRoundExp?: number;
  readonly talentCardParams?: Readonly<Record<string, readonly number[]>>;
  readonly currentTalentTempDatas?: Readonly<Record<string, number>>;
  readonly talentTempDatas?: Readonly<Record<string, number>>;
  readonly currentPermanentBuffTempDatas?: Readonly<Record<string, number>>;
  readonly permanentBuffTempDatas: Readonly<Record<string, number>>;
  readonly fateStrategies?: readonly number[];
  readonly cards: readonly OriginalCardConfig[];
}

export interface HistoricalCardOverride {
  readonly side: "p1" | "p2";
  readonly slotIndex: number;
  readonly patch: Partial<Omit<OriginalCardConfig, "id" | "baseId" | "name">>;
  readonly reason: string;
  readonly evidence: string;
}

export interface OriginalReplayFixture {
  readonly firstPlayerSide: "p1" | "p2";
  readonly decisionTape?: readonly number[];
  readonly randomFallbackTape?: readonly number[];
  readonly catalogCards?: readonly OriginalCardConfig[];
  readonly historicalCardOverrides?: readonly HistoricalCardOverride[];
  readonly maxActorTurns?: number;
  readonly source?: {
    readonly round?: number;
    readonly steamBuild?: string;
    readonly seasonMechanism?: number;
    readonly historicalSeasonMechanisms?: readonly "talentResonance"[];
    readonly syntheticDecisionSeed?: number;
    readonly syntheticDecisionSides?: readonly ("p1" | "p2")[];
    readonly syntheticDecisionFallbackSeed?: number;
  };
  readonly players: Readonly<Record<"p1" | "p2", OriginalReplayPlayerFixture>>;
}

export enum CoreBuff {
  Recovery = "recovery",
  InternalInjury = "internalInjury",
  ExternalInjury = "externalInjury",
  AttackBonus = "attackBonus",
  Physique = "physique",
  PhysiqueLimit = "TiPoShangXian",
  FistStance = "QuanJiaShi",
  Hexagram = "GuaXiang",
  LastStandIntent = "SiZhanZhiZhi",
  CannotAct = "BuNengXingDong",
}

const CARD_TYPES: Readonly<Record<number, CardType>> = {
  0: "normal",
  1: "consume",
  2: "refine",
  3: "sustain",
  4: "change",
};

export function adaptOriginalCardType(cardType: OriginalEnumValue | null | undefined): CardType {
  const value = cardType?.value ?? 0;
  const mapped = CARD_TYPES[value];
  if (!mapped) throw new Error(`未知原版卡牌类型 ${value}`);
  return mapped;
}

export function normalizeBaseId(cardId: number): number {
  if (cardId === 0 || cardId === 10_000 || cardId === 20_000 || cardId === 1_000_000) return 0;
  return cardId - Math.trunc((cardId % 1_000_000) / 10_000) * 10_000;
}

export function adaptOriginalCardConfig(config: OriginalCardConfig): CardDefinition {
  return Object.freeze({
    id: config.id,
    baseId: config.baseId ?? normalizeBaseId(config.id),
    name: config.name,
    type: adaptOriginalCardType(config.cardType),
    traits: config.traits ?? [],
    ...(config.desc === undefined ? {} : { desc: config.desc }),
    ...(config.hidden === undefined ? {} : { hidden: config.hidden }),
    ...(config.rarity === undefined ? {} : { rarity: config.rarity }),
    ...(config.subcategory?.name === undefined ? {} : { subcategoryName: config.subcategory.name }),
    ...(config.career?.name === undefined ? {} : { careerName: config.career.name }),
    ...(config.anima === undefined || config.anima >= 0 ? {} : { animaCost: -config.anima }),
    ...(config.anima === undefined || config.anima <= 0 ? {} : { animaGain: config.anima }),
    ...(config.chargeQi === undefined ? {} : { chargeQi: config.chargeQi }),
    ...(config.hpCost === undefined ? {} : { hpCost: config.hpCost }),
    ...(config.actionAgain === undefined ? {} : { actionAgain: config.actionAgain }),
    ...(config.attack === undefined ? {} : { attack: config.attack }),
    ...(config.randomAttack === undefined ? {} : { randomAttack: config.randomAttack }),
    ...(config.attackCount === undefined ? {} : { attackCount: config.attackCount }),
    ...(config.level === undefined ? {} : { realmLevel: config.level.value }),
    ...(config.def === undefined && config.defense === undefined ? {} : { defense: config.def ?? config.defense }),
    ...(config.randomDef === undefined && config.randomDefense === undefined ? {} : { randomDefense: config.randomDef ?? config.randomDefense }),
    ...(config.damage === undefined ? {} : { damage: config.damage }),
    ...(config.physique === undefined ? {} : { physique: config.physique }),
    ...(config.jianYi === undefined ? {} : { swordIntent: config.jianYi }),
    ...(config.guaXiang === undefined ? {} : { hexagram: config.guaXiang }),
    ...(config.otherParams === undefined ? {} : { otherParams: [...config.otherParams] }),
  });
}

export const BASIC_ATTACK: CardDefinition = Object.freeze({
  id: 0,
  baseId: 0,
  name: "普通攻击",
  type: "normal",
  traits: [],
  attack: 3,
});

const REALM_NAMES: Readonly<Record<string, string>> = {
  LianQi: "炼气",
  ZhuJi: "筑基",
  JinDan: "金丹",
  YuanYing: "元婴",
  HuaShen: "化神",
  FanXu: "返虚",
};

export function cardRealmLabel(realm: string | undefined): string {
  if (!realm) return "未知境界";
  return REALM_NAMES[realm] ?? realm;
}

export const CARD_ARCHIVE_KIND_OPTIONS: readonly {
  readonly kind: CardArchiveKind | "all";
  readonly label: string;
}[] = [
  { kind: "all", label: "全部索引" },
  { kind: "sect", label: "门派" },
  { kind: "career", label: "副职" },
  { kind: "exclusive", label: "专属" },
  { kind: "chance", label: "机缘" },
  { kind: "secret", label: "秘术" },
  { kind: "fate", label: "仙命" },
  { kind: "season", label: "赛季特有" },
  { kind: "common", label: "通用" },
];

export function cardArchiveKindLabel(kind: CardArchiveKind): string {
  return CARD_ARCHIVE_KIND_OPTIONS.find((option) => option.kind === kind)?.label ?? kind;
}

const ARCHIVE_ORDER = new Map(CARD_ARCHIVE_KIND_OPTIONS.map((option, index) => [option.kind, index]));

export function sortCardsByArchive<T extends {
  readonly archiveKind: CardArchiveKind;
  readonly archiveLabel: string;
  readonly baseId: number;
}>(cards: readonly T[]): T[] {
  return [...cards].sort((left, right) =>
    (ARCHIVE_ORDER.get(left.archiveKind) ?? 99) - (ARCHIVE_ORDER.get(right.archiveKind) ?? 99) ||
    left.archiveLabel.localeCompare(right.archiveLabel, "zh-Hans-CN") ||
    left.baseId - right.baseId
  );
}

export function buildCardArchiveOptions(cards: readonly {
  readonly archiveKind: CardArchiveKind;
  readonly archiveKey: string;
  readonly archiveLabel: string;
}[]): { readonly kind: CardArchiveKind; readonly key: string; readonly label: string }[] {
  const options = new Map<string, { readonly kind: CardArchiveKind; readonly key: string; readonly label: string }>();
  for (const card of cards) {
    options.set(card.archiveKey, {
      kind: card.archiveKind,
      key: card.archiveKey,
      label: card.archiveLabel,
    });
  }
  return [...options.values()].sort((left, right) =>
    (ARCHIVE_ORDER.get(left.kind) ?? 99) - (ARCHIVE_ORDER.get(right.kind) ?? 99) ||
    left.label.localeCompare(right.label, "zh-Hans-CN")
  );
}
