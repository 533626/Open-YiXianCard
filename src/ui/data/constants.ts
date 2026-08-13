import { CARD_ARCHIVE_KIND_OPTIONS, cardArchiveKindLabel } from "../domain";
import type { Side } from "../types";

export { CARD_ARCHIVE_KIND_OPTIONS, cardArchiveKindLabel };

export const GROUP_NAMES: Readonly<Record<string, string>> = {
  YunLingJianZong: "云灵剑宗",
  QiXingGe: "七星阁",
  WuXingDaoMeng: "五行道盟",
  DuanXuanZong: "锻玄宗",
  LianDanShi: "炼丹师",
  FuZhouShi: "符咒师",
  QinShi: "琴师",
  HuaShi: "画师",
  ZhenFaShi: "阵法师",
  LingZhiShi: "灵植师",
  MingLiShi: "命理师",
};

const CARD_TYPE_NAMES: Readonly<Record<string, string>> = {
  normal: "普通",
  sustain: "持续",
  consume: "消耗",
  refine: "炼化",
  change: "置换",
};

export const ELEMENT_OPTIONS = ["metal", "water", "wood", "fire", "earth"] as const;
export const ELEMENT_LABELS: Readonly<Record<(typeof ELEMENT_OPTIONS)[number], string>> = {
  metal: "金",
  water: "水",
  wood: "木",
  fire: "火",
  earth: "土",
};

export const DEFAULT_GAME_ROUND = 16;

export function groupLabel(group: string): string {
  return GROUP_NAMES[group] ?? group;
}

export function cardTypeLabel(type: string): string {
  return CARD_TYPE_NAMES[type] ?? type;
}

export function sideLabel(side: Side): string {
  return side === "p1" ? "玩家一" : "玩家二";
}
