/**
 * 打靶模式的伤害归因与聚合（纯函数，无 DOM / 无 Worker 依赖）。
 *
 * 数据来源是现有 `yixian_trace_json` 的 `HookStep`，不依赖新引擎 API：
 * - `mainEffect` 步骤带逐段攻击采样 `attackSegments`（Rust
 *   `ReplayHookTraceStep.attack_segments`，按 event_index 精确挂到该步骤的卡），
 *   每段 `damage = (hpBefore - hpAfter) + (defBefore - defAfter)`，归到
 *   `step.cardId/cardName`。被防御吸收的部分与击穿防御的部分都计入。
 * - 非攻击步骤（turnStart/turnEnd/afterCard 等）不出攻击段，但 `changes` 里有
 *   `{group:"核心", key:"hp"/"defense"}` 的减少量（毒/内伤/持续 buff 滴血、
 *   回合结算伤害），归到 `step.cardId ?? null` → 「持续/其他」桶。
 * - mainEffect 步骤若没有攻击段（巨鼎落这类直接改血卡），其 `changes` 减少量
 *   是唯一的伤害来源，同样归到该卡。同一步骤只走一条通道（段 或 changes），
 *   不会重复计数。
 *
 * 聚合按「回合」：`battleRound(actorTurn) = ceil(actorTurn / 2)`（双方各动一次
 * 算一回合，与 render-battle-progress 同口径）。
 */

import { normalizeBaseId } from "./domain";
import type { HookStep } from "./hook-trace";
import { battleRound } from "./render-battle-progress";
import { GAME_TURN_LIMIT } from "./target-dummy";
import type {
  Side,
  TargetCardDamage,
  TargetDamageStep,
  TargetPracticeResult,
  TargetTurnDamage,
} from "./types";

/** 「持续/其他」桶（无卡牌归因的回合结算伤害）的调色板键。 */
export const OTHER_DAMAGE_KEY = "null";

/**
 * 引擎基础战斗最大行动回合数：engine-rust/src/replay.rs `DEFAULT_MAX_ACTOR_TURNS`。
 * 打靶模式照搬此常量（= GAME_TURN_LIMIT*2 = 64），引擎始终跑满，
 * UI 按「显示至回合」（绝对有效回合数）裁剪展示窗口。
 */

const CARD_COLOR_CLASSES = [
  "card-1", "card-2", "card-3", "card-4",
  "card-5", "card-6", "card-7", "card-8",
] as const;

/** 卡牌调色板键：同 baseId 的变体跨构筑/跨模式同色（按 baseId 哈希到固定槽位）。 */
export function cardPaletteKey(cardId: number | null): string {
  if (cardId === null || cardId <= 0) return OTHER_DAMAGE_KEY;
  return String(normalizeBaseId(cardId));
}

/**
 * 稳定按卡分配颜色槽位：同一张卡（按 baseId）跨回合同色、跨构筑同色；
 * 无卡归因（回合结算伤害）固定用「持续/其他」色。
 */
export function cardPalette(
  cardIds: readonly (number | null)[],
): ReadonlyMap<string, string> {
  const palette = new Map<string, string>();
  for (const cardId of cardIds) {
    const key = cardPaletteKey(cardId);
    if (palette.has(key)) continue;
    palette.set(key, OTHER_DAMAGE_KEY === key
      ? "card-other"
      : CARD_COLOR_CLASSES[hashSlot(key)]);
  }
  return palette;
}

function hashSlot(key: string): number {
  let hash = 0;
  for (let index = 0; index < key.length; index += 1) {
    hash = (hash * 31 + key.charCodeAt(index)) >>> 0;
  }
  return hash % CARD_COLOR_CLASSES.length;
}

export function opponentSide(side: Side): Side {
  return side === "p1" ? "p2" : "p1";
}

/**
 * 归因并聚合 `sourceSide` 一方对另一方的逐回合伤害。
 * 与双方对战伤害曲线共用（duel 模式两侧各算一次，target 模式只算我方对木桩侧）。
 */
export function computeDamagePerTurn(
  hookSteps: readonly HookStep[] | undefined,
  sourceSide: Side,
): readonly TargetTurnDamage[] {
  const targetSide = opponentSide(sourceSide);
  const byRound = new Map<number, Map<string, TargetCardDamage>>();
  for (const step of hookSteps ?? []) {
    if (step.actor !== sourceSide) continue;
    const damage = stepDamage(step, targetSide);
    if (damage <= 0) continue;
    const round = battleRound(step.actorTurn);
    const cards = byRound.get(round) ?? new Map<string, TargetCardDamage>();
    const key = cardPaletteKey(step.cardId);
    const existing = cards.get(key);
    const entry: TargetCardDamage = existing ?? {
      cardId: step.cardId,
      cardName: step.cardName,
      damage: 0,
    };
    cards.set(key, { ...entry, damage: entry.damage + damage });
    byRound.set(round, cards);
  }
  const perTurn: TargetTurnDamage[] = [...byRound.entries()]
    .sort(([left], [right]) => left - right)
    .map(([round, cards]) => {
      const byCard = [...cards.values()]
        .sort((left, right) =>
          right.damage - left.damage ||
          String(left.cardId ?? -1).localeCompare(String(right.cardId ?? -1)))
        .map((entry) => ({ ...entry }));
      return {
        round,
        total: byCard.reduce((sum, entry) => sum + entry.damage, 0),
        byCard,
      };
    });
  return perTurn;
}

/**
 * 单个钩子步骤对 `targetSide` 造成的伤害：
 * - mainEffect 有攻击段 → 段合计 + changes 的**残余**减少（同一步骤里段没覆盖的
 *   直接改血部分，如「攻击 + 追加直接伤害」的卡）：残余 = max(0, changes 减少 - 段合计)，
 *   纯攻击步骤里 changes ≤ 段合计（回血等会使净变化更小），残余恒为 0，不会重复计数。
 * - 其余（mainEffect 无段 = 直接改血卡；turnStart/turnEnd/afterCard 等）→
 *   数 `changes` 里 hp/defense 的减少量。
 */
function stepDamage(step: HookStep, targetSide: Side): number {
  if (step.category === "mainEffect" && step.attackSegments.length > 0) {
    const segmentDamage = step.attackSegments.reduce((sum, segment) => {
      if (segment.target !== targetSide) return sum;
      return sum + Math.max(0, segment.hpBefore - segment.hpAfter)
        + Math.max(0, segment.defBefore - segment.defAfter);
    }, 0);
    return segmentDamage + Math.max(0, changesReduction(step, targetSide) - segmentDamage);
  }
  return changesReduction(step, targetSide);
}

/** `changes` 里 targetSide 一方 hp/defense 的合计减少量。 */
function changesReduction(step: HookStep, targetSide: Side): number {
  return step.changes.reduce((sum, change) => {
    if (change.side !== targetSide) return sum;
    if (change.key !== "hp" && change.key !== "defense") return sum;
    return sum + Math.max(0, change.before - change.after);
  }, 0);
}

/**
 * 按出牌事件序列展开的伤害台阶（23→65→96→133 累计过程）。
 * 每个伤害事件（一张牌的出牌或一次回合结算伤害）= 一个台阶：
 * - 台阶高度 = 该步对木桩的伤害增量（不分卡种叠，就是这个数）；
 * - 颜色按该步归因到的卡牌着色（无卡归因如内伤/持续伤害 → 「持续/其他」色）；
 * - `cumulative` 是该步结算后的累计伤害（阶梯曲线的 y 值）。
 * 大步进 = 回合边界（`battleRound` 切换），同回合多张牌 = 中间小台阶。
 */
export function computeDamageSteps(
  hookSteps: readonly HookStep[] | undefined,
  sourceSide: Side,
): readonly TargetDamageStep[] {
  const targetSide = opponentSide(sourceSide);
  const steps: TargetDamageStep[] = [];
  let cumulative = 0;
  for (const step of hookSteps ?? []) {
    if (step.actor !== sourceSide) continue;
    const damage = stepDamage(step, targetSide);
    if (damage <= 0) continue;
    cumulative += damage;
    steps.push({
      round: battleRound(step.actorTurn),
      actorTurn: step.actorTurn,
      cardId: step.cardId,
      cardName: step.cardName,
      damage,
      cumulative,
    });
  }
  return steps;
}

/** 打靶终局判定的中文标签。 */
export function targetReachedLabel(result: TargetPracticeResult): string {
  return result.stopReason === "threshold" ? "已达成" : "未达成";
}

/**
 * 打靶结果：累计伤害第一个 ≥ 阈值的回合为 `reachedTurn`（stopReason
 * "threshold"）；否则打满游戏常量回合上限 32（stopReason "turnLimit"）。
 * 引擎始终跑满 64 actorTurn，`frames` 只用于取引擎实际跑到的 actorTurn。
 *
 * `displayRounds`：展示窗口终点（**绝对有效回合数**，1..GAME_TURN_LIMIT）。
 * 结果按 `min(displayRounds, GAME_TURN_LIMIT)` 裁剪，但至少显示到达标回合：
 * 第 4 回合达标时即使传入更小的值也保证窗口覆盖到 4，不提供 0..3 这类没有
 * 完整结果意义的窗口。调用方（滑块）只应提供 `>= reachedTurn` 的值。
 */
export function computeTargetPracticeResult(
  hookSteps: readonly HookStep[] | undefined,
  frames: readonly { readonly actorTurn: number }[] | undefined,
  damageThreshold: number,
  displayRounds: number,
): TargetPracticeResult {
  const threshold = Math.max(1, Math.trunc(damageThreshold) || 1);
  const requested = Math.trunc(displayRounds);
  const requestedRounds = Number.isFinite(requested) && requested >= 1 ? requested : 1;
  const allSteps = computeDamageSteps(hookSteps, "p1");
  const perTurn = computeDamagePerTurn(hookSteps, "p1");
  let reachedTurn = GAME_TURN_LIMIT;
  let stopReason: TargetPracticeResult["stopReason"] = "turnLimit";
  let cumulative = 0;
  for (const turn of perTurn) {
    cumulative += turn.total;
    if (cumulative >= threshold) {
      reachedTurn = turn.round;
      stopReason = "threshold";
      break;
    }
  }
  // 展示窗口至少覆盖到达标回合（第 4 回合达标就从 4 开始显示），
  // 但不超过游戏常量上限 32。
  const displayRound = Math.min(
    GAME_TURN_LIMIT,
    Math.max(reachedTurn, requestedRounds),
  );
  const visibleSteps = allSteps.filter((step) => step.round <= displayRound);
  const visiblePerTurn = perTurn.filter((turn) => turn.round <= displayRound);
  const visibleCumulative = visibleSteps.at(-1)?.cumulative
    ?? visiblePerTurn.reduce((sum, turn) => sum + turn.total, 0);
  return {
    perTurn: visiblePerTurn,
    steps: visibleSteps,
    totalDamage: visibleCumulative,
    stopReason,
    reachedTurn,
  };
}
