import {
  BASIC_ATTACK,
  adaptOriginalCardConfig,
  adaptOriginalCardType,
  buildCardArchiveOptions,
  cardRealmLabel,
  normalizeBaseId,
  sortCardsByArchive,
  type CardDefinition,
  type OriginalCardConfig,
} from "../domain";
import type {
  CardOption,
  CardVariantMode,
  CardVariantOption,
  CareerOption,
  DeckSlotConfig,
} from "../types";
import {
  ORIGINAL_CARD_CONFIGS,
  archiveByBaseId,
  archiveCards,
  characterNameById,
  coverageByBaseId,
  type UiOriginalCardConfig,
} from "./source";
import { cardTypeLabel } from "./constants";
import { formatOriginalDetail } from "./text-format";

const UI_DISABLED_CARD_BASE_IDS = new Set([11, 216, 217]);

export const CARD_CONFIG_BY_ID = new Map(
  ORIGINAL_CARD_CONFIGS.map((card) => [card.id, card] as const),
);

/**
 * 境界排序与标签。梦卡变体按境界（LianQi..HuaShen）分级，level 0..4 对应这五个境界；
 * FanXu 在战斗卡组不出现，仅用于排序兜底。
 */
const REALM_ORDER: Readonly<Record<string, number>> = {
  LianQi: 0,
  ZhuJi: 1,
  JinDan: 2,
  YuanYing: 3,
  HuaShen: 4,
  FanXu: 5,
};

/**
 * 判定一组变体的分级方式。梦境卡的 5 个变体没有 rarity 字段（全部为 0），
 * 但各自带不同境界（config.level.name）与不同整牌 id；普通卡的变体靠 rarity 区分。
 * - 只有 1 个变体（noUpgrade 卡，如遗迹法器）→ single：无等级切换。
 * - 多变体且 rarity 全 0、境界互异 → realm：按境界序保留全部。
 * - 其余 → rarity：按 rarity 0/1/2 分阶。
 */
function detectVariantMode(variants: readonly CardVariantOption[]): CardVariantMode {
  if (variants.length <= 1) return "single";
  const allRarityZero = variants.every((v) => v.rarity === 0);
  if (!allRarityZero) return "rarity";
  const realms = variants
    .map((v) => v.config.level?.name)
    .filter((name): name is string => typeof name === "string");
  if (realms.length !== variants.length) return "rarity";
  const distinctRealms = new Set(realms);
  return distinctRealms.size === variants.length ? "realm" : "rarity";
}

/**
 * 去重并排序变体。rarity 模式按 rarity 去重升序；realm 模式按境界序去重，
 * 不再用 rarity（全为 0）去重，否则 5 个境界变体会被压成 1 个。
 */
function dedupVariants(
  variants: readonly CardVariantOption[],
  mode: CardVariantMode,
): readonly CardVariantOption[] {
  if (mode === "rarity") {
    return [...variants]
      .sort((a, b) => a.rarity - b.rarity)
      .filter((v, i, all) => all.findIndex((c) => c.rarity === v.rarity) === i);
  }
  return [...variants]
    .sort((a, b) => (REALM_ORDER[a.realm ?? ""] ?? 900) - (REALM_ORDER[b.realm ?? ""] ?? 900))
    .filter((v, i, all) => all.findIndex((c) => c.realm === v.realm) === i);
}

/** 主变体：rarity 模式取 rarity=0；realm 模式取境界序最低（炼气）。 */
function pickPrimaryVariant(
  variants: readonly CardVariantOption[],
  mode: CardVariantMode,
): CardVariantOption {
  if (mode === "rarity") {
    return variants.find((v) => v.rarity === 0) ?? variants[0]!;
  }
  return variants[0]!;
}

export const CARD_OPTIONS: readonly CardOption[] = buildCardOptions();
export const CARD_OPTION_BY_BASE_ID = new Map(
  CARD_OPTIONS.map((card) => [card.baseId, card] as const),
);
export const CARD_INDEX_OPTIONS: readonly CardOption[] = buildCardIndexOptions();
export const CARD_ARCHIVE_OPTIONS = buildCardArchiveOptions(CARD_OPTIONS);
export const CAREER_OPTIONS: readonly CareerOption[] = CARD_ARCHIVE_OPTIONS
  .filter((option) => option.kind === "career")
  .map((option) => ({
    id: option.key.slice("career:".length),
    name: option.label,
  }));
export const DEFAULT_CAREER_ID =
  CAREER_OPTIONS.find((career) => career.id === "LianDanShi")?.id ??
  CAREER_OPTIONS[0]?.id ??
  null;
export const ALL_CARD_DEFINITIONS: readonly CardDefinition[] = CARD_OPTIONS
  .flatMap((card) => card.variants.map((variant) => variant.definition));

export function getCardVariant(slot: DeckSlotConfig): CardVariantOption {
  const card = CARD_OPTION_BY_BASE_ID.get(slot.baseId) ?? CARD_OPTION_BY_BASE_ID.get(0);
  if (!card) throw new Error(`未知卡牌 baseId：${slot.baseId}`);
  if (card.variantMode === "single") return card.variants[0]!;
  if (card.variantMode === "realm") {
    // 梦境卡：level 是境界序号（0=炼气 … 4=化神），直接按下标取变体。
    return card.variants[slot.level] ?? card.variants[0]!;
  }
  return (
    card.variants.find((variant) => variant.rarity === slot.level) ??
    card.variants.find((variant) => variant.rarity === 0) ??
    card.variants[0]!
  );
}

/** 普通卡（rarity 分级）的阶位标签，level 0/1/2 → 一阶/二阶/三阶。 */
export function cardLevelLabel(level: number): string {
  if (level === 1) return "二阶";
  if (level === 2) return "三阶";
  return "一阶";
}

/** 境界分级卡（梦境卡）的境界标签，level 0..4 → 炼气/筑基/金丹/元婴/化神。 */
export function cardRealmLevelLabel(level: number): string {
  const realms = ["LianQi", "ZhuJi", "JinDan", "YuanYing", "HuaShen"] as const;
  return cardRealmLabel(realms[level] ?? "");
}

/**
 * 卡牌在指定 level 的分级标签。根据卡牌的 variantMode 决定显示阶位还是境界。
 * single 模式返回空字符串——单变体卡不展示阶位/境界标签。
 */
export function cardSlotLevelLabel(card: CardOption, level: number): string {
  if (card.variantMode === "single") return "";
  return card.variantMode === "realm" ? cardRealmLevelLabel(level) : cardLevelLabel(level);
}

/** 单个变体的分级标签：rarity 模式用阶位，realm/single 模式用境界名或空。 */
function variantLabel(card: CardOption, variant: CardVariantOption): string {
  if (card.variantMode === "realm" && variant.realm) return cardRealmLabel(variant.realm);
  if (card.variantMode === "single") return card.realmLabel ?? "";
  return cardLevelLabel(variant.rarity);
}

const NUMERIC_DETAIL_FIELD_KEYS = [
  "attack",
  "randomAttack",
  "attackCount",
  "def",
  "randomDef",
  "anima",
  "jianYi",
  "physique",
  "guaXiang",
] as const satisfies readonly (keyof OriginalCardConfig)[];

// Hover detail is level-independent: it always shows every registered
// rarity variant's effect text, not just the deck slot's current level.
// When every variant shares the same desc template (the common case), the
// text is rendered once with each differing number shown as v1/v2/v3
// instead of repeating the whole line per level.
export function cardDetailText(card: CardOption, fallbackConfig?: OriginalCardConfig | null): string {
  if (card.baseId === 0) return card.name;
  if (card.variants.length === 0) {
    const desc = fallbackConfig ? formatCardConfigDetail(fallbackConfig, card.desc) : "";
    return desc ? `${card.name}\n${desc}` : card.name;
  }
  const templates = card.variants.map((variant) => variant.config.desc ?? card.desc);
  const sameTemplate = templates.every((template) => template === templates[0]);
  if (sameTemplate) {
    const desc = formatMergedCardDetail(card.variants, templates[0]!);
    return desc ? `${card.name}\n${desc}` : card.name;
  }
  const blocks = card.variants
    .map((variant) => {
      const desc = formatCardConfigDetail(variant.config, card.desc);
      const label = variantLabel(card, variant);
      return desc ? `${label}：${desc}` : null;
    })
    .filter((line): line is string => line !== null);
  return blocks.length > 0 ? `${card.name}\n${blocks.join("\n")}` : card.name;
}

function formatCardConfigDetail(config: OriginalCardConfig, fallbackTemplate: string): string {
  return formatOriginalDetail(config.desc ?? fallbackTemplate, config.otherParams ?? [], {
    attack: config.attack,
    randomAttack: config.randomAttack,
    attackCount: config.attackCount,
    def: config.def,
    randomDef: config.randomDef,
    anima: config.anima,
    jianYi: config.jianYi,
    physique: config.physique,
    guaXiang: config.guaXiang,
  });
}

function formatMergedCardDetail(variants: readonly CardVariantOption[], template: string): string {
  const fields: Record<string, number | string | undefined> = {};
  for (const key of NUMERIC_DETAIL_FIELD_KEYS) {
    fields[key] = mergeLevelValues(variants.map((variant) => variant.config[key] as number | undefined));
  }
  const paramCount = Math.max(0, ...variants.map((variant) => variant.config.otherParams?.length ?? 0));
  const otherParams: (number | string)[] = [];
  for (let index = 0; index < paramCount; index += 1) {
    const merged = mergeLevelValues(variants.map((variant) => variant.config.otherParams?.[index]));
    if (merged !== undefined) otherParams[index] = merged;
  }
  return formatOriginalDetail(template, otherParams, fields);
}

// Same value across every level -> show it once; differing values -> "v1/v2/v3" in level order.
function mergeLevelValues(values: readonly (number | undefined)[]): number | string | undefined {
  const defined = values.filter((value): value is number => value !== undefined);
  if (defined.length === 0) return undefined;
  const unique = [...new Set(defined)];
  return unique.length === 1 ? unique[0] : defined.join("/");
}

export function describeCard(card: CardOption): string {
  if (card.baseId === 0) return card.name;
  return [
    card.name,
    card.archiveLabel === "通用" ? "" : card.archiveLabel,
    ...(card.realmLabel ? [card.realmLabel] : []),
    card.type === "normal" ? "" : cardTypeLabel(card.type),
  ].filter(Boolean).join(" · ");
}

function buildCardOptions(): readonly CardOption[] {
  const byBaseId = new Map<number, CardVariantOption[]>();
  for (const config of ORIGINAL_CARD_CONFIGS) {
    if (config.id < 0) continue;
    const type = adaptOriginalCardType(config.cardType);
    const baseId = normalizeBaseId(config.id);
    if (!isGeneratedRegisteredCard(baseId)) continue;
    const definition = adaptOriginalCardConfig(config);

    const variants = byBaseId.get(baseId) ?? [];
    variants.push({
      id: config.id,
      rarity: config.rarity ?? 0,
      ...(config.level?.name ? { realm: config.level.name } : {}),
      label: `Lv${(config.rarity ?? 0) + 1}`,
      config,
      definition,
    });
    byBaseId.set(baseId, variants);
  }

  const options: CardOption[] = [];
  for (const [baseId, variants] of byBaseId) {
    const variantMode = detectVariantMode(variants);
    const deduped = dedupVariants(variants, variantMode);
    const primary = pickPrimaryVariant(deduped, variantMode);
    const config = primary.config as UiOriginalCardConfig;
    const coverage = coverageByBaseId.get(baseId);
    const generatedArchive = archiveByBaseId.get(baseId);
    const archive = generatedArchive ?? {
      archiveKind: config.sect ? "sect" : config.career ? "career" : "common",
      archiveKey: config.sect?.name
        ? `sect:${config.sect.name}`
        : config.career?.name
          ? `career:${config.career.name}`
          : "common",
      archiveLabel: config.sect?.name ?? config.career?.name ?? "通用",
    };
    const realm = generatedArchive?.realm ?? config.level?.name;
    const realmLabel = generatedArchive?.realmLabel ??
      (config.level?.name ? cardRealmLabel(config.level.name) : undefined);
    options.push({
      baseId,
      name: primary.definition.name,
      group: coverage?.group ?? config.sect?.name ?? config.career?.name ??
        (config.seasonMechanics?.length ? "天衍仙命" : "通用"),
      groupType: coverage?.groupType ?? (config.sect ? "sect" : config.career ? "career" : "season"),
      implemented: true,
      archiveKind: archive.archiveKind,
      archiveKey: archive.archiveKey,
      archiveLabel: archive.archiveLabel,
      ...(realm !== undefined ? { realm } : {}),
      ...(realmLabel !== undefined ? { realmLabel } : {}),
      type: primary.definition.type,
      desc: config.desc ?? "",
      variantMode,
      variants: deduped,
    });
  }

  return sortCardsByArchive(options);
}

function isGeneratedRegisteredCard(baseId: number): boolean {
  if (baseId === 0) return true;
  return archiveByBaseId.get(baseId)?.registered === true ||
    coverageByBaseId.get(baseId)?.registered === true;
}

function buildCardIndexOptions(): readonly CardOption[] {
  const options = archiveCards
    .filter((card) => card.simulationScope === "battle" && card.obsolete !== true)
    .map((card): CardOption => {
      const implemented = CARD_OPTION_BY_BASE_ID.get(card.baseId);
      if (implemented) return implemented;
      return {
        baseId: card.baseId,
        name: card.name,
        group: card.sect ?? card.career ?? card.archiveLabel,
        groupType: card.archiveKind,
        implemented: false,
        archiveKind: card.archiveKind,
        archiveKey: card.archiveKey,
        archiveLabel: card.archiveLabel,
        ...(card.realm !== undefined ? { realm: card.realm } : {}),
        ...(card.realmLabel !== undefined ? { realmLabel: card.realmLabel } : {}),
        type: card.type,
        desc: "",
        variants: [],
        variantMode: "rarity",
      };
    });
  return sortCardsByArchive(options);
}

export function isCardDisabled(card: CardOption): boolean {
  return !card.implemented || UI_DISABLED_CARD_BASE_IDS.has(card.baseId);
}

export function basicAttackDefinition(): CardDefinition {
  return BASIC_ATTACK;
}
