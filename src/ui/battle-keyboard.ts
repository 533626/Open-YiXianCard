import { timelinePoints } from "./render-battle-progress";
import type { AppState, SimulationResult } from "./types";

export function battleStepShortcutDirection(
  key: string,
  state: Pick<AppState, "result" | "view">,
  editing: boolean,
): -1 | 1 | null {
  if (editing || !state.result || state.view !== "battle") return null;
  if (key === "ArrowLeft" || key === "ArrowUp") return -1;
  if (key === "ArrowRight" || key === "ArrowDown") return 1;
  return null;
}

/**
 * 回放步进锚点是每个时间轴点的末帧：开局点（战斗开始结算）之后是各行动回合的
 * 末帧——正常回合就是“回合结束结算”，若致死导致原始事件流没有 turnEnd，则
 * 保留该 actorTurn 的最后实际帧，避免终局无法抵达。
 *
 * 开局点必须参与方向键步进与状态对比：否则从战前初始状态直接跳到第一动结束，
 * 左侧状态条会把战斗开始效果（如卜卦加卦象）的闪烁显示在第一动帧，而不是
 * 初始结算帧。
 */
export function allStepFrameIndexes(result: SimulationResult): readonly number[] {
  return timelinePoints(result.frames)
    .filter((point) => point.actorTurn >= 0)
    .map((point) => point.jumpFrame.index);
}

/**
 * 完整行动的末帧锚点（不含开局点）：战斗完成后停在这里，让用户直接看到第一个
 * 行动回合的结果；开局结算用 ← 回看。
 */
export function completedTurnFrameIndexes(
  result: SimulationResult,
): readonly number[] {
  return timelinePoints(result.frames)
    .filter((point) => point.actorTurn > 0)
    .map((point) => point.jumpFrame.index);
}

export function firstCompletedTurnFrameIndex(result: SimulationResult): number {
  return completedTurnFrameIndexes(result)[0] ?? result.frames[0]?.index ?? 0;
}

export function adjacentCompletedTurnFrameIndex(
  result: SimulationResult,
  currentIndex: number,
  direction: -1 | 1,
): number {
  const indexes = allStepFrameIndexes(result);
  const initialIndex = result.frames[0]?.index ?? currentIndex;
  if (indexes.length === 0) return initialIndex;
  if (direction < 0) {
    return [...indexes].reverse().find((index) => index < currentIndex) ?? initialIndex;
  }
  return indexes.find((index) => index > currentIndex) ?? indexes.at(-1)!;
}
