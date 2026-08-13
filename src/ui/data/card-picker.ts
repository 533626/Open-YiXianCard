import { cardRealmLabel } from "../domain";
import {
  CARD_INDEX_OPTIONS,
  CARD_OPTIONS,
  CARD_OPTION_BY_BASE_ID,
  CAREER_OPTIONS,
  isCardDisabled,
} from "./cards";
import { CHARACTER_BY_ID } from "./players";
import { isCardUnlockedByTalents } from "./derivations";
import type { CardOption, CharacterOption, DeckSlotConfig } from "../types";

const LIMITED_DECK_CARD_TYPES = new Set(["consume", "sustain"]);
const LIMITED_DECK_CARD_TYPE_LIMIT = 2;

export interface CardPickerGroup {
  readonly id: string;
  readonly label: string;
  readonly cards: readonly CardOption[];
  /** 分组标签下的小徽标，目前只有副职分组使用（主 / 筑兼 …）。 */
  readonly badge?: string;
  readonly badgeTitle?: string;
}

/**
 * 卡池里副职牌的归属上下文：主副职一个，兼修副职由「副职兼修」仙命所在境界槽开出。
 * 原版除副职兼修外只能有 1 个副职，所以这里的每一项都对应一组独立的副职牌池。
 *
 * 契约：传入 scope 时，卡池（scopedCardOptions / scopedCardIndexOptions 的产出）
 * 已经按主副职 + 兼修副职过滤过；分组只反映已选副职，未选副职的牌不会成组。
 * primary 为 null（未选副职）时不产生主副职组。
 */
export interface CareerPickerScope {
  readonly primary: string | null;
  readonly duals: Readonly<Record<number, string>>;
}

const DUAL_CAREER_SLOT_LABELS: Readonly<Record<number, { short: string; full: string }>> = {
  1: { short: "筑兼", full: "筑基兼修" },
  2: { short: "金兼", full: "金丹兼修" },
  3: { short: "元兼", full: "元婴兼修" },
  4: { short: "化兼", full: "化神兼修" },
};

const SECT_REALM_BUCKETS = [
  { id: "HuaShen", label: "化神", realms: ["HuaShen", "FanXu"] as const },
  { id: "YuanYing", label: "元婴", realms: ["YuanYing"] as const },
  { id: "JinDan", label: "金丹", realms: ["JinDan"] as const },
  { id: "ZhuJi", label: "筑基", realms: ["ZhuJi"] as const },
  { id: "LianQi", label: "炼气", realms: ["LianQi"] as const },
] as const;

const ARCHIVE_PICKER_BUCKETS = [
  { id: "exclusive", kind: "exclusive", label: "专属" },
  { id: "chance-artifact", kind: "chance", archiveKey: "chance:artifact", label: "法宝" },
  { id: "chance-pet", kind: "chance", archiveKey: "chance:pet", label: "灵宠" },
  { id: "chance-entry", kind: "chance", archiveKey: "chance:entry", label: "机缘" },
  { id: "secret", kind: "secret", label: "秘术" },
  { id: "fate", kind: "fate", label: "仙命" },
  { id: "career", kind: "career", label: "副职" },
  { id: "common", kind: "common", label: "通用" },
] as const;

const SEASON_PICKER_BUCKETS = [
  { id: "season-luck", label: "气运" },
  { id: "season-life-shop", label: "命坊" },
  { id: "season-fate-branch", label: "命运分支" },
  { id: "season-relic", label: "遗迹法器" },
  { id: "season-ronghui", label: "共鸣仙命" },
  { id: "season-mirage", label: "幻景" },
  { id: "season-sigil", label: "刻印" },
  { id: "season-dream", label: "梦境" },
  { id: "season-fate-strategy", label: "天衍仙命" },
  { id: "season-activity", label: "活动" },
  { id: "season-other", label: "其他赛季" },
] as const;

export function scopedCardOptions(
  characterId: number,
  careerName: string | null,
  talentIds: readonly number[] = [],
  dualCareerNames: Readonly<Record<number, string>> = {},
): readonly CardOption[] {
  const character = CHARACTER_BY_ID.get(characterId);
  const cards = CARD_OPTIONS.filter((card) =>
    cardAllowedForScope(card, character, careerName, dualCareerNames));
  return cards.filter((card) => isCardUnlockedByTalents(card.baseId, talentIds));
}

export function scopedCardIndexOptions(
  characterId: number,
  careerName: string | null,
  talentIds: readonly number[] = [],
  dualCareerNames: Readonly<Record<number, string>> = {},
): readonly CardOption[] {
  const character = CHARACTER_BY_ID.get(characterId);
  const cards = CARD_INDEX_OPTIONS.filter((card) =>
    cardAllowedForScope(card, character, careerName, dualCareerNames));
  return cards.filter((card) => isCardUnlockedByTalents(card.baseId, talentIds));
}

export function cardsGroupedForPicker(
  cards: readonly CardOption[],
  careerScope?: CareerPickerScope,
): readonly CardPickerGroup[] {
  const groups = new Map<string, CardOption[]>();
  for (const card of cards) {
    const column = cardPickerColumn(card, careerScope !== undefined);
    if (!column) continue;
    const list = groups.get(column) ?? [];
    list.push(card);
    groups.set(column, list);
  }
  const careerBuckets = careerPickerBuckets(careerScope);
  const allBuckets = [
    ...ARCHIVE_PICKER_BUCKETS
      .filter((bucket) => bucket.id !== "career")
      .map((bucket) => ({ id: bucket.id, label: bucket.label })),
    ...careerBuckets,
    ...SECT_REALM_BUCKETS.map((bucket) => ({ id: bucket.id, label: bucket.label })),
    ...SEASON_PICKER_BUCKETS,
  ];
  const bucketById = new Map(allBuckets.map((bucket) => [bucket.id, bucket] as const));
  const priorityOrder = [
    "exclusive",
    "HuaShen",
    "YuanYing",
    "season-fate-strategy",
    "JinDan",
    "ZhuJi",
    "LianQi",
    "fate",
    "career",
    "common",
    "season-luck",
    "season-life-shop",
    "season-fate-branch",
    "season-relic",
    "season-ronghui",
    "season-mirage",
    "season-sigil",
    "season-dream",
    "season-activity",
    "season-other",
    "chance-entry",
    "chance-artifact",
    "chance-pet",
    "secret",
  ] as const;
  // 副职位次展开成「每个已选副职一组」，其余分组次序不变。
  const expandedOrder = priorityOrder.flatMap((id) =>
    id === "career" ? careerBuckets.map((bucket) => bucket.id) : [id],
  );
  const prioritized = new Set<string>(expandedOrder);
  const order = [
    ...expandedOrder
      .map((id) => bucketById.get(id))
      .filter((bucket) => bucket !== undefined),
    ...allBuckets.filter((bucket) => !prioritized.has(bucket.id)),
  ];
  return order
    .filter((bucket) => groups.has(bucket.id))
    .map((bucket) => ({
      ...bucket,
      cards: (groups.get(bucket.id) ?? []).sort(
        isCareerPickerBucket(bucket.id) ? careerCardPickerSort : cardPickerSort,
      ),
    }));
}

function isCareerPickerBucket(bucketId: string): boolean {
  return bucketId === "career" || bucketId.startsWith("career:");
}

function careerPickerBuckets(
  scope: CareerPickerScope | undefined,
): readonly { id: string; label: string; badge?: string; badgeTitle?: string }[] {
  if (!scope) return [{ id: "career", label: "副职" }];
  const buckets: { id: string; label: string; badge?: string; badgeTitle?: string }[] = [];
  const seen = new Set<string>();
  const push = (careerId: string | null | undefined, badge?: string, badgeTitle?: string): void => {
    if (!careerId) return;
    const id = `career:${careerId}`;
    if (seen.has(id)) return;
    seen.add(id);
    buckets.push({
      id,
      label: careerPickerLabel(careerId),
      ...(badge !== undefined ? { badge } : {}),
      ...(badgeTitle !== undefined ? { badgeTitle } : {}),
    });
  };
  push(scope.primary, "主", "主副职");
  for (const slot of [1, 2, 3, 4]) {
    const slotLabel = DUAL_CAREER_SLOT_LABELS[slot];
    push(scope.duals[slot], slotLabel?.short, slotLabel?.full);
  }
  // 不在这里给「卡池里出现但未选中的副职」兜底成组：未选副职的牌由
  // scopedCardOptions / scopedCardIndexOptions 过滤，进不了卡池；
  // 一旦出现未选副职的组，用户会以为该副职已生效（默认副职炼丹师被混入其他副职）。
  return buckets;
}

function careerPickerLabel(careerId: string): string {
  return CAREER_OPTIONS.find((career) => career.id === careerId)?.name ?? careerId;
}


export function deckUsageCounts(deck: readonly DeckSlotConfig[]): ReadonlyMap<number, number> {
  const counts = new Map<number, number>();
  for (const slot of deck) {
    if (slot.baseId === 0) continue;
    counts.set(slot.baseId, (counts.get(slot.baseId) ?? 0) + 1);
  }
  return counts;
}

export function canPickCardForDeckSlot(
  card: CardOption,
  deck: readonly DeckSlotConfig[],
  slotIndex: number,
): boolean {
  if (isCardDisabled(card)) return false;
  if (!isLimitedDeckCardType(card)) return true;
  let used = 0;
  for (const [index, slot] of deck.entries()) {
    if (index === slotIndex || slot.baseId === 0) continue;
    const existing = CARD_OPTION_BY_BASE_ID.get(slot.baseId);
    if (existing && isLimitedDeckCardType(existing)) used += 1;
  }
  return used < LIMITED_DECK_CARD_TYPE_LIMIT;
}

export function isLimitedDeckCardType(card: CardOption): boolean {
  return LIMITED_DECK_CARD_TYPES.has(card.type);
}

export function cardVisualType(card: CardOption): "dx" | "qx" | "normal" {
  if (card.baseId === 0) return "normal";
  if (card.group === "DuanXuanZong") return "dx";
  if (card.group === "QiXingGe") return "qx";
  return "normal";
}

export { cardRealmLabel };

function cardPickerColumn(card: CardOption, splitCareer = false): string | null {
  if (card.baseId === 0) return null;
  if (card.archiveKind === "common" && isNumericPlaceholderCard(card)) return null;
  if (card.archiveKind === "sect") {
    const bucket = SECT_REALM_BUCKETS.find((entry) =>
      card.realm !== undefined && (entry.realms as readonly string[]).includes(card.realm),
    );
    return bucket?.id ?? null;
  }
  if (card.archiveKind === "season") return seasonCardPickerColumn(card);
  if (card.archiveKey === "fate:5") return "season-ronghui";
  // 副职牌按所属副职单独成组：archiveKey 本身就是 `career:<副职>`。
  if (splitCareer && card.archiveKind === "career") return card.archiveKey;
  const bucket = ARCHIVE_PICKER_BUCKETS.find((entry) =>
    entry.kind === card.archiveKind &&
    (!("archiveKey" in entry) || entry.archiveKey === card.archiveKey),
  );
  return bucket?.id ?? null;
}

function isNumericPlaceholderCard(card: CardOption): boolean {
  return /^\d+$/.test(card.name.trim());
}

function seasonCardPickerColumn(card: CardOption): string {
  const { archiveKey } = card;
  if (archiveKey.startsWith("season:past:relic:")) return "season-relic";
  if (archiveKey.startsWith("season:history:mirage:")) return "season-mirage";
  if (archiveKey.startsWith("season:history:dream:")) return "season-dream";
  if (archiveKey.startsWith("season:base:fate-strategy:")) return "season-fate-strategy";
  if (archiveKey.startsWith("season:activity:")) return "season-activity";
  if (archiveKey.includes(":luck")) return "season-luck";
  if (archiveKey.includes(":life-shop")) return "season-life-shop";
  if (archiveKey.includes(":fate-branch")) return "season-fate-branch";
  if (archiveKey.includes(":relic")) return "season-relic";
  if (archiveKey.includes(":sigil")) return "season-sigil";
  return "season-other";
}

function cardPickerSort(left: CardOption, right: CardOption): number {
  const availability = compareCardAvailability(left, right);
  if (availability !== 0) return availability;
  // 同境界分组内把同系列牌（星弈•、崩拳•、云剑•、卦等）聚到一起并置顶，
  // 让用户一眼看到一张牌的整条进阶链，而不是被普通牌打散。
  const seriesDiff = cardSeriesRank(left) - cardSeriesRank(right);
  if (seriesDiff !== 0) return seriesDiff;
  const leftSeries = cardSeriesKey(left);
  const rightSeries = cardSeriesKey(right);
  if (leftSeries !== rightSeries) {
    return leftSeries.localeCompare(rightSeries, "zh");
  }
  return left.name.localeCompare(right.name, "zh");
}

/**
 * 卡牌的系列归属，用于在选卡浮层把同系列牌聚在一起。
 * 星弈•点 / 崩拳•伏虎 / 云剑•崩雪 这类带「•」的牌取「•」前的词干作为系列；
 * 乾卦/坤卦/巽卦… 这组单字+卦的牌归入「卦」系列；其余牌无系列（返回空串）。
 */
export function cardSeriesKey(card: CardOption): string {
  const name = card.name.trim();
  const dot = name.indexOf("•");
  if (dot > 0) return name.slice(0, dot);
  if (name.length === 2 && name.endsWith("卦")) return "卦";
  return "";
}

/** 系列牌在分组内排在普通牌之前；多系列之间按名称稳定排序。 */
function cardSeriesRank(card: CardOption): number {
  return cardSeriesKey(card) === "" ? 1 : 0;
}

function careerCardPickerSort(left: CardOption, right: CardOption): number {
  const availability = compareCardAvailability(left, right);
  if (availability !== 0) return availability;
  // 副职卡按境界从高到低排列（化神→元婴→金丹→筑基→炼气），
  // 高境界副职卡使用频率更高，优先展示。FanXu（返虚）不在此列。
  const realmOrder: Readonly<Record<string, number>> = {
    HuaShen: 5,
    YuanYing: 4,
    JinDan: 3,
    ZhuJi: 2,
    LianQi: 1,
  };
  const realmDifference = (realmOrder[right.realm ?? ""] ?? 0) -
    (realmOrder[left.realm ?? ""] ?? 0);
  if (realmDifference !== 0) return realmDifference;
  return left.name.localeCompare(right.name, "zh");
}

function compareCardAvailability(left: CardOption, right: CardOption): number {
  const leftDisabled = isCardDisabled(left);
  const rightDisabled = isCardDisabled(right);
  if (leftDisabled !== rightDisabled) return leftDisabled ? 1 : -1;
  if (left.implemented !== right.implemented) return left.implemented ? -1 : 1;
  return 0;
}

/**
 * 卡牌是否进入角色牌池。character 未解析（未选角色或角色 id 失效）时，
 * 只放行与角色归属无关的卡：通用牌、已选副职（主副职 + 兼修副职）牌、
 * 奇遇/秘术通用牌、赛季通用牌与赛季副职牌；门派、专属、门派秘术等
 * 需要角色归属的卡一律不放行，避免「没选角色/副职时全卡池都出现」。
 */
function cardAllowedForScope(
  card: CardOption,
  character: CharacterOption | undefined,
  careerName: string | null,
  dualCareerNames: Readonly<Record<number, string>> = {},
): boolean {
  if (card.archiveKey === "common") return true;
  // 奇遇/秘术通用牌与角色、副职无关，任何牌池都放行。
  if (["chance:artifact", "chance:pet", "chance:entry", "secret:common", "secret:tianyan"]
    .includes(card.archiveKey)) return true;
  if (character) {
    if (card.archiveKey === `sect:${character.sectName}`) return true;
    if (card.archiveKey === `exclusive:${character.id}`) return true;
    if (card.archiveKey === `fate:sect:${character.sectName}`) return true;
    if (card.archiveKey === `fate:sect-card:${character.sectName}`) return true;
    if (card.archiveKey === `secret:sect:${character.sectName}`) return true;
  }
  if (careerName && card.archiveKey === `career:${careerName}`) return true;
  if (careerName && card.archiveKey === `secret:career:${careerName}`) return true;
  // 副职兼修：允许兼修副职的牌池
  for (const dualKey of Object.values(dualCareerNames)) {
    if (dualKey && card.archiveKey === `career:${dualKey}`) return true;
    if (dualKey && card.archiveKey === `secret:career:${dualKey}`) return true;
  }
  if (!card.archiveKey.startsWith("season:")) return false;
  if (card.archiveKey.endsWith(":common") || card.archiveKey.endsWith(":unscoped")) return true;
  const requiredExclusive = card.archiveKey.match(/:exclusive:(\d+)/)?.[1];
  const requiredSect = card.archiveKey.match(/:sect:([^:]+)/)?.[1];
  const requiredCareer = card.archiveKey.match(/:career:([^:]+)/)?.[1];
  if (requiredExclusive && (!character || Number(requiredExclusive) !== character.id)) return false;
  if (requiredSect && (!character || requiredSect !== character.sectName)) return false;
  if (requiredCareer && requiredCareer !== careerName && !Object.values(dualCareerNames).includes(requiredCareer)) return false;
  return Boolean(requiredExclusive || requiredSect || requiredCareer);
}
