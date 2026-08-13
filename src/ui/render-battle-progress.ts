import {
  escapeAttribute,
  escapeHtml,
} from "./view-utils";
import type { BattleFrame, Side } from "./types";

/**
 * 时间轴上的一个点 = 一方的一次完整行动。
 *
 * 一次 actorTurn 里的回合开始结算、出牌（含再动追加的那几张）、回合结束结算都
 * 落在同一个点上，跳转目标是这次行动结算完的那一帧。方向键、‹ › 按钮和时间轴
 * 共用这套粗粒度锚点 —— 把每个阶段帧都摊成一个点会让一场战斗排出几十个
 * 「阶」，既读不出谁在行动，也读不出打到哪了。
 */
export interface TimelinePoint {
  readonly actorTurn: number;
  readonly round: number;
  /** 行动方；开局点（actorTurn 0）没有行动方。 */
  readonly actor: Side | null;
  readonly actorName: string | null;
  readonly frames: readonly BattleFrame[];
  readonly jumpFrame: BattleFrame;
  readonly cardNames: readonly string[];
  readonly actionRange: readonly [number, number] | null;
}

// Round marks and timeline dots are rendered as two separate rows (layers)
// rather than sharing one row, so a narrow single-action round never
// squeezes its "Rn" label into the neighboring dot's mark. Both rows use
// the same per-point grid columns, so each round label spans exactly the
// columns occupied by its dots and stays centered over that group.
export function renderProgressTrack(timelineFrames: readonly BattleFrame[], selectedIndex: number): string {
  const points = timelinePoints(timelineFrames);
  const rounds = groupPointsByRound(points);
  return `
    <div class="progress-round-labels">
      ${rounds.map((group) => renderProgressRoundLabel(group)).join("")}
    </div>
    <div class="progress-dots">
      ${points.map((point) => renderProgressDot(point, selectedIndex)).join("")}
    </div>
  `;
}

export function timelinePoints(frames: readonly BattleFrame[]): readonly TimelinePoint[] {
  const groups = new Map<number, BattleFrame[]>();
  for (const frame of frames) {
    const list = groups.get(frame.actorTurn) ?? [];
    list.push(frame);
    groups.set(frame.actorTurn, list);
  }
  return [...groups.entries()].map(([actorTurn, grouped]) => {
    const actorFrame = grouped.find((frame) => frame.actorId === "p1" || frame.actorId === "p2");
    const actor = actorTurn === 0 ? null : (actorFrame?.actorId as Side | undefined) ?? null;
    const actions = grouped
      .map((frame) => frame.actionIndex)
      .filter((index): index is number => index !== null);
    return {
      actorTurn,
      round: battleRound(actorTurn),
      actor,
      actorName: actor && actorFrame ? actorFrame.players[actor].name : null,
      frames: grouped,
      jumpFrame: grouped.at(-1)!,
      // 只算真正出掉的牌：阶段帧也可能带着牌名，把它们算进来会让一次出牌看起来出了两张。
      cardNames: grouped
        .filter((frame) => frame.actionIndex !== null)
        .map((frame) => frame.cardName)
        .filter((name): name is string => name !== null),
      actionRange: actions.length > 0 ? [actions[0]!, actions.at(-1)!] : null,
    };
  });
}

function groupPointsByRound(
  points: readonly TimelinePoint[],
): readonly { readonly round: number; readonly count: number }[] {
  const groups: { round: number; count: number }[] = [];
  for (const point of points) {
    const current = groups.at(-1);
    if (current && current.round === point.round) current.count += 1;
    else groups.push({ round: point.round, count: 1 });
  }
  return groups;
}

function renderProgressRoundLabel(
  group: { readonly round: number; readonly count: number },
): string {
  return `
    <span
      class="progress-round-mark"
      style="grid-column:span ${group.count}"
      data-round="${group.round}"
      title="第 ${group.round} 回合"
    >${group.round > 0 ? `R${group.round}` : ""}</span>
  `;
}

function renderProgressDot(point: TimelinePoint, selectedIndex: number): string {
  const selected = point.frames.some((frame) => frame.index === selectedIndex);
  // 用一/二 标行动方，跟左上角「对局 一 二」同一套记号。不取名字首字：
  // 「玩家一/玩家二」这类默认标签首字相同，一整条时间轴会全是同一个「玩」。
  const mark = point.actor === null ? "初" : point.actor === "p1" ? "一" : "二";
  return `
    <button
      type="button"
      class="battle-progress-dot ${selected ? "selected" : ""}"
      data-action="jump-frame"
      data-frame="${point.jumpFrame.index}"
      data-round="${point.round}"
      data-actor-turn="${point.actorTurn}"
      ${point.actor ? `data-actor="${point.actor}"` : ""}
      title="${escapeAttribute(timelinePointLabel(point))}"
    >
      <span class="progress-action">${escapeHtml(mark)}</span>
    </button>
  `;
}

export function timelinePointLabel(point: TimelinePoint): string {
  if (point.actor === null) return "开局：战前状态与战斗开始结算";
  const actions = point.actionRange === null
    ? "未出牌"
    : point.actionRange[0] === point.actionRange[1]
    ? `第 ${point.actionRange[0]} 动`
    : `第 ${point.actionRange[0]}–${point.actionRange[1]} 动`;
  const cards = point.cardNames.length > 0 ? ` · ${point.cardNames.join("、")}` : "";
  return `第 ${point.round} 回合 · ${point.actorName ?? point.actor} · ${actions}${cards}`;
}

export function framePositionLabel(frame: BattleFrame, actionCount?: number): string {
  if (frame.actionIndex === null) return frame.index === 0 ? "初始状态" : frame.title;
  const actionText = actionCount === undefined ? `${frame.actionIndex} 动` : `${frame.actionIndex} / ${actionCount} 动`;
  return `第 ${battleRound(frame.actorTurn)} 回合 · 第 ${actionText}`;
}

export function battleRound(actorTurn: number): number {
  return Math.max(1, Math.ceil(actorTurn / 2));
}

export function frameCardLabel(frame: BattleFrame): string {
  if (frame.cardName) return frame.cardName;
  if (frame.actionIndex === null) return frame.index === 0 ? "初始" : frame.title;
  return "—";
}
