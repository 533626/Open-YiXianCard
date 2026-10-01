import {
  xianMoStrategyGroupsData,
  xianMoStrategyRows,
  xianMoStrategyById,
  type XianMoStrategyRow,
} from "./source";
import type { XianMoStrategyGroup, XianMoStrategyOption } from "../types";

export function xianMoStrategyGroups(): readonly XianMoStrategyGroup[] {
  return xianMoStrategyGroupsData.map((g) => ({
    id: g.id,
    label: g.label,
    options: g.strategies.map(toOption),
  }));
}

export function xianMoStrategyOptions(): readonly XianMoStrategyOption[] {
  return xianMoStrategyRows.map(toOption);
}

export const XIAN_MO_STRATEGY_OPTION_BY_ID = new Map(
  xianMoStrategyOptions().map((opt) => [opt.id, opt] as const),
);

function toOption(row: XianMoStrategyRow): XianMoStrategyOption {
  return {
    id: row.id,
    name: row.name,
    desc: row.desc,
    category: row.category,
    categoryLabel: row.categoryLabel,
    isBattleEffect: row.isBattleEffect,
    affectsBattle: row.affectsBattle,
    npcId: row.npcId,
    minRound: row.minRound,
    maxRound: row.maxRound,
    otherParams: row.otherParams,
  };
}

export function isFuMoStrategy(strategyId: number): boolean {
  const row = xianMoStrategyById.get(strategyId);
  return row?.category === "FuMo";
}

export function isSwitchStrategy(strategyId: number): boolean {
  const row = xianMoStrategyById.get(strategyId);
  return row?.category === "Switch";
}

export function xianMoStrategyDisplayName(strategyId: number): string {
  const row = xianMoStrategyById.get(strategyId);
  return row?.name ?? `策略_${strategyId}`;
}

export function xianMoStrategySummary(strategyId: number): string {
  const row = xianMoStrategyById.get(strategyId);
  if (!row) return "";
  return row.desc;
}

export function getFuMoGrid(
  tempDatas: Readonly<Record<string, number>> | undefined,
  strategyId: number,
): number {
  if (!tempDatas) return 0;
  return tempDatas[String(strategyId)] ?? 0;
}

export function isSwitchActive(
  tempDatas: Readonly<Record<string, number>> | undefined,
  strategyId: number,
): boolean {
  if (!tempDatas) return true;
  const val = tempDatas[String(strategyId)];
  return val === undefined || val === 0;
}
