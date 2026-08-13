import type { BattleFrame, Side } from "./types";

export const BATTLE_START_SETTLEMENT_TITLE = "战斗开始结算";

export type OpeningResourceKey = "defense" | "anima" | "guard" | "momentum" | "agility";

export const OPENING_RESOURCE_LABELS: readonly {
  readonly key: OpeningResourceKey;
  readonly label: string;
}[] = [
  { key: "defense", label: "防" },
  { key: "anima", label: "灵气" },
  { key: "guard", label: "护体" },
  { key: "momentum", label: "气势" },
  { key: "agility", label: "身法" },
];

export function openingFrame(
  result: { readonly frames: readonly BattleFrame[] } | null,
): BattleFrame | null {
  if (!result) return null;
  // Rust 路径帧 0 就是战斗开始结算帧（战前不再占一帧），TS 路径帧 0 是
  // “初始状态”帧；isBattleStartSettlementFrame 按 title 精确匹配，不需要 index 条件。
  return result.frames.find((frame) =>
    isBattleStartSettlementFrame(frame)
  ) ?? null;
}

export function isBattleStartSettlementFrame(frame: BattleFrame): boolean {
  return frame.actionIndex === null
    && frame.actorTurn === 0
    && frame.title === BATTLE_START_SETTLEMENT_TITLE;
}

export function openingResourceDeltas(
  initial: BattleFrame | null,
  opening: BattleFrame | null,
  side: Side,
): readonly { readonly key: OpeningResourceKey; readonly label: string; readonly value: number }[] {
  if (!initial || !opening) return [];
  const before = initial.players[side];
  const after = opening.players[side];
  return OPENING_RESOURCE_LABELS
    .map(({ key, label }) => ({ key, label, value: after[key] - before[key] }))
    .filter((item) => item.value !== 0);
}
