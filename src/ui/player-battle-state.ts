import { ELEMENT_LABELS } from "./data";
import buffCategoryArchive from "../../shared/data/buff-category-archive.json";
import { buffLabel, escapeHtml } from "./view-utils";
import type { PlayerView } from "./types";

/**
 * 原作 BuffCategory（buff-category-archive.json，来自解码 BuffConfig.json）：
 * BattleCharacterUI.RefreshBuff 按分类决定显示位置 —— Positive/Negative → 角色区
 * （MainBuffDisplay）、Permanent → 仙命区（TalentDisplay）、其余（Neutral）→ 卡牌区
 * （BuffDisplay）；Hidden 与 BuffType.TiPo / TiPoShangXian 直接 return，永不显示。
 */
type BuffCategory = "Positive" | "Negative" | "Neutral" | "Hidden" | "Permanent";

/** 原作三区（RefreshBuff 的 BuffDisplayType：角色 / 仙命 / 卡牌）。 */
export type BattleStatZone = "character" | "talent" | "card";

const ZONE_LABELS: Readonly<Record<BattleStatZone, string>> = {
  character: "角色",
  talent: "仙命",
  card: "卡牌",
};

type BuffArchiveEntry = {
  readonly id: number;
  readonly category: BuffCategory;
  readonly name: string | null;
};

const generatedBuffCategoryArchive = buffCategoryArchive as {
  readonly buffs: Readonly<Record<string, BuffArchiveEntry>>;
};

/**
 * UI buff 键 → 原版 BuffType 枚举名。rustBuffs / CoreBuff 使用 camelCase 别名，
 * 分类表按原版枚举名索引，先经此表锚定（拼音/低频字别名必须锚定原版 BuffType，
 * 见 AGENTS.md「不要只根据拼音名解释 Buff」；ID 见 buff-category-archive.json）。
 */
const BUFF_TYPE_BY_UI_KEY: Readonly<Record<string, string>> = {
  recovery: "HuiFu", // CoreBuff.Recovery → BuffType.HuiFu(248) 恢复
  internalInjury: "NeiShang", // → BuffType.NeiShang(100) 内伤
  weakness: "XuRuo", // → BuffType.XuRuo(101) 虚弱
  flaw: "PoZhan", // → BuffType.PoZhan(102) 破绽
  attackBonus: "JiaGong", // → BuffType.JiaGong(2) 加攻
  attackReduction: "JianGong", // → BuffType.JianGong(103) 减攻（负面）
  entangle: "KunFu", // → BuffType.KunFu(104) 困缚
  externalInjury: "WaiShang", // → BuffType.WaiShang(105) 外伤
  physique: "TiPo", // → BuffType.TiPo(10023) 体魄（stat 条显示，见 STAT_DISPLAYED_BUFF_KEYS）
  TiPoShangXian: "TiPoShangXian", // → BuffType.TiPoShangXian(10024) 体魄上限（同上）
  swordIntent: "JianYi", // → BuffType.JianYi(6) 剑意
  swordEnergy: "JianQi", // → BuffType.JianQi(625) 剑气
};

/**
 * 拳/棍架势只显示架势不显示数值层数（RefreshBuff 对 QuanJiaShi / GunJiaShi
 * 调用 ClearValueLabel）。注意 ZuiQuanJiaShi（醉拳架势）不在此列，原版照常显示数值。
 */
const STANCE_NO_VALUE_KEYS = new Set(["QuanJiaShi", "GunJiaShi"]);

/**
 * 已作为角色属性在 stat 条显示的 buff 键，不在 buff 状态项里重复出现。
 * 体魄/体魄上限按用户约束在状态条保留显示（不对齐原作 RefreshBuff 的排除规则），
 * 其显示位置是 stat 条；setup 面板另有独立体魄输入框。
 */
const STAT_DISPLAYED_BUFF_KEYS = new Set(["physique", "TiPoShangXian"]);

const TOP_RESOURCE_BUFFS = [
  { key: "GuaXiang", label: "卦象" },
  { key: "XingLi", label: "星力" },
] as const;

const MISC_BUFFS = new Set([
  "AddHpCount",
  "LoseHpCount",
  "LoseHpTimesCount",
  "ActualDamage",
  "BENLUNGONGJICISHU",
  "BenChangZhanDouTiPoJiShu",
  "ZhanDouGongJiJiShu",
  "UsedCardCount",
  "WoundedCount",
  "LoseDefCount",
]);

export type BattleStatItem = {
  readonly label: string;
  readonly value: string | number;
  readonly hot?: boolean;
};

export function collectBattleStatItems(player: PlayerView): readonly BattleStatItem[] {
  const items: BattleStatItem[] = [];
  const push = (label: string, value: string | number, hot = false): void => {
    if (value === 0 || value === "0" || value === "0/0") return;
    items.push({ label, value, hot });
  };
  push("防", player.defense);
  push("灵", player.anima);
  for (const item of TOP_RESOURCE_BUFFS) {
    const value = player.buffs[item.key] ?? 0;
    if (value > 0) items.push({ label: item.label, value, hot: true });
  }
  // 体魄保留在状态条显示（用户约束：不对齐原作 RefreshBuff 的 TiPo 排除规则）。
  const physique = player.buffs.physique ?? 0;
  if (physique > 0) items.push({ label: "体魄", value: physique });
  if (player.momentum > 0) {
    items.push({
      label: "气势",
      value: player.momentumLimit > 0 ? `${player.momentum}/${player.momentumLimit}` : player.momentum,
      hot: true,
    });
  } else if (player.momentumLimit > 0 && player.momentumLimit !== 6) {
    push("气势上限", player.momentumLimit);
  }
  push("身法", player.agility);
  push("护体", player.guard);
  if (player.lastElement !== null) {
    items.push({ label: "五行", value: ELEMENT_LABELS[player.lastElement] ?? "-" });
  }
  return items;
}

export type BattleStatusItem = {
  readonly label: string;
  readonly value: number | string;
  readonly kind: "buff" | "sustain";
  /** 原作三区：角色（Positive/Negative）/ 仙命（Permanent）/ 卡牌（其余）。 */
  readonly zone: BattleStatZone;
  /** 拳/棍架势：原作 ClearValueLabel 只显示图标（文本标签），不渲染数值层数。 */
  readonly noValue?: boolean;
};

export function collectBattleStatusItems(player: PlayerView): readonly BattleStatusItem[] {
  const items: BattleStatusItem[] = [];
  for (const [key, value] of Object.entries(player.buffs)) {
    if (value === 0) continue;
    if (STAT_DISPLAYED_BUFF_KEYS.has(key)) continue;
    if (TOP_RESOURCE_BUFFS.some((item) => item.key === key)) continue;
    if (MISC_BUFFS.has(key)) continue;
    const entry = buffArchiveEntry(key);
    const category = entry?.category;
    if (category === "Hidden") continue;
    const label = buffLabel(key);
    if (label === "状态" && category === undefined) continue;
    items.push({
      // 分类已知但 UI 无词条的（如永久消耗品），用分类表里的原文名兜底。
      label: label === "状态" && entry?.name ? entry.name : label,
      value,
      kind: "buff",
      zone: buffZone(category),
      ...(STANCE_NO_VALUE_KEYS.has(key) ? { noValue: true } : {}),
    });
  }
  // 持续效果归卡牌区：原作持续牌效果图标挂在 BuffDisplay（卡牌区）。
  items.push(...Object.entries(player.sustainValues).map(([key, values]) => ({
    label: buffLabel(key),
    value: values.join("/"),
    kind: "sustain" as const,
    zone: "card" as const,
  })));
  return items;
}

/** 供契约测试锁定「UI buff 键 → 归档分类」的解析完整性（防裸键兜底漏过滤）。 */
export function buffArchiveEntry(buffKey: string): BuffArchiveEntry | undefined {
  const typeName = buffArchiveEnum(buffKey);
  return generatedBuffCategoryArchive.buffs[typeName];
}

/**
 * UI buff 键 → 原版 BuffType 枚举名（archiveEnum）。camelCase 别名经
 * `BUFF_TYPE_BY_UI_KEY` 锚定，archiveEnum 风格的键原样返回。供契约测试
 * 桥接 `rustBuffs` 发射键 ↔ `buff-id-rust-key-map.json` 的 `archiveEnum`。
 */
export function buffArchiveEnum(buffKey: string): string {
  return BUFF_TYPE_BY_UI_KEY[buffKey] ?? buffKey;
}

function buffZone(category: BuffCategory | undefined): BattleStatZone {
  if (category === "Positive" || category === "Negative") return "character";
  if (category === "Permanent") return "talent";
  return "card"; // Neutral 与未分类（浏览器内部计数）→ 卡牌区
}

export function renderBattleStatRibbon(
  player: PlayerView,
  options: {
    readonly inline?: boolean;
    readonly includeHp?: boolean;
    readonly previous?: PlayerView | null;
  } = {},
): string {
  const hasPrevious = options.previous !== undefined && options.previous !== null;
  const previousStatItems = options.previous ? collectBattleStatItems(options.previous) : [];
  const previousStatusItems = options.previous ? collectBattleStatusItems(options.previous) : [];
  const statItems = includeVanishedItems(
    collectBattleStatItems(player),
    previousStatItems,
    (item) => ({ ...item, value: item.label === "五行" ? "-" : 0 }),
  );
  const statusItems = includeVanishedStatuses(
    statusesByZone(collectBattleStatusItems(player)),
    statusesByZone(previousStatusItems),
  );
  const previousStats = new Map(previousStatItems.map((item) => [item.label, item.value]));
  const previousStatuses = new Map(previousStatusItems.map((item) => [item.label, item.value]));
  const hpPercent = player.maxHp > 0
    ? Math.max(0, Math.min(100, Math.round((player.hp / player.maxHp) * 100)))
    : 0;
  const renderStat = (item: BattleStatItem): string => `
    <span class="live-stat${item.hot ? " hot" : ""}">
      <em>${escapeHtml(item.label)}</em>${
    escapeHtml(transitionValue(item.value, previousStats.get(item.label), hasPrevious))
  }
    </span>`;
  const renderStatus = (item: BattleStatusItem): string => `
    <span class="live-status ${item.kind}${item.noValue ? " no-value" : ""}">
      <em>${escapeHtml(item.label)}</em>${
    // 拳/棍架势（noValue）：不渲染数值层数。消失帧（includeVanishedStatuses 补的
    // value=0 项）同样无数值提示 —— 这是有意的：ClearValueLabel 语义下原版从不显示
    // 架势数值，活体架势与消失帧外观一致，避免出现本不该存在的「0」或过渡。
    item.noValue
      ? ""
      : `<b>${escapeHtml(transitionValue(item.value, previousStatuses.get(item.label), hasPrevious))}</b>`
  }
    </span>`;
  // 原作三区顺序：角色区（Positive/Negative）→ 仙命区（Permanent）→ 卡牌区（其余）。
  // 核心属性（防/灵/气势/身法/护体/五行/卦象/星力）属角色状态条，置角色区首位。
  const renderZone = (zone: BattleStatZone): string => {
    const statusPart = statusItems[zone].map(renderStatus).join("");
    const statPart = zone === "character" ? statItems.map(renderStat).join("") : "";
    if (!statPart && !statusPart) return "";
    // 只有核心属性（如 setup 面板）时不加区标签；出现分类 buff 时按原版三区标注。
    const tag = zone === "character" && statusItems[zone].length === 0
      ? ""
      : `<em class="live-zone-tag">${ZONE_LABELS[zone]}</em>`;
    return `<span class="live-stat-zone zone-${zone}">${tag}${statPart}${statusPart}</span>`;
  };
  return `
    <div class="player-live-stats${options.inline ? " inline" : ""}">
      <div class="player-live-stats-main">
        ${renderZone("character")}${renderZone("talent")}${renderZone("card")}
      </div>
      ${
    options.includeHp === false
      ? ""
      : renderBattleHp(
        player,
        hpPercent,
        hasPrevious ? player.hp - options.previous!.hp : 0,
      )
  }
    </div>
  `;
}

export function renderBattleHp(
  player: PlayerView,
  hpPercent?: number,
  hpDelta?: number,
): string {
  const percent = hpPercent ?? (player.maxHp > 0
    ? Math.max(0, Math.min(100, Math.round((player.hp / player.maxHp) * 100)))
    : 0);
  // 满血曾经也是纯红，在这套深色配色里读起来像报错；按剩余比例分档着色。
  const level = percent <= 25 ? "critical" : percent <= 55 ? "hurt" : "healthy";
  return `
    <span
      class="live-stat live-hp-end hp-${level}"
      aria-label="战斗中生命"
      style="--hp-pct: ${percent}%"
    >
      <span class="live-hp-fill"></span>
      <span class="live-hp-text">${player.hp}<span class="hp-sep">/</span>${player.maxHp}</span>
    </span>
    ${
    hpDelta === undefined
      ? ""
      : `<span
          class="live-hp-delta ${hpDelta > 0 ? "gain" : hpDelta < 0 ? "loss" : "neutral"}"
          data-hp-delta="${hpDelta}"
          title="相对上个导航点的生命变化"
        >${hpDelta > 0 ? "+" : ""}${hpDelta}</span>`
  }
  `;
}

type ZoneStatuses = Readonly<Record<BattleStatZone, readonly BattleStatusItem[]>>;

function statusesByZone(items: readonly BattleStatusItem[]): ZoneStatuses {
  const zones = { character: [] as BattleStatusItem[], talent: [] as BattleStatusItem[], card: [] as BattleStatusItem[] };
  for (const item of items) zones[item.zone].push(item);
  return zones;
}

function includeVanishedStatuses(
  current: ZoneStatuses,
  previous: ZoneStatuses,
): ZoneStatuses {
  const result = { character: [] as BattleStatusItem[], talent: [] as BattleStatusItem[], card: [] as BattleStatusItem[] };
  for (const zone of ["character", "talent", "card"] as const) {
    const currentLabels = new Set(current[zone].map((item) => item.label));
    result[zone] = [
      ...current[zone],
      ...previous[zone]
        .filter((item) => !currentLabels.has(item.label))
        .map((item) => ({ ...item, value: 0 })),
    ];
  }
  return result;
}

function includeVanishedItems<T extends { readonly label: string }>(
  current: readonly T[],
  previous: readonly T[],
  vanished: (item: T) => T,
): readonly T[] {
  const currentLabels = new Set(current.map((item) => item.label));
  return [
    ...current,
    ...previous.filter((item) => !currentLabels.has(item.label)).map(vanished),
  ];
}

function transitionValue(
  current: string | number,
  previous: string | number | undefined,
  hasPreviousFrame: boolean,
): string {
  if (!hasPreviousFrame) return String(current);
  const from = previous ?? (typeof current === "number" ? 0 : "-");
  return String(from) === String(current) ? String(current) : `${from}→${current}`;
}
