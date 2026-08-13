import { describe, expect, test } from "bun:test";
import {
  type RuleEvent,
} from "../domain";
import { openingFrame } from "../battle-opening";
import { resourceFlowFrames } from "../render-battle-flow";
import {
  frameCardLabel,
  framePositionLabel,
  timelinePointLabel,
  timelinePoints,
} from "../render-battle-progress";
import { buildFrames } from "../simulator-frames";
import type { BattleFrame } from "../types";
import { playerView } from "./layout-test-helpers";

describe("UI 完成态阶段帧文案", () => {
  test("事件流没有开局效果时仍保留无状态变化的战斗开始结算帧", () => {
    const frames = buildTestFrames([
      { sequence: 1, type: "phase", actorId: "p1", name: "turnStart" },
      { sequence: 2, type: "card", actorId: "p1", name: "cardSelected", detail: { cardId: 0, sourceSlot: 0 } },
      { sequence: 3, type: "card", actorId: "p1", name: "cardCompleted", detail: { cardId: 0, sourceSlot: 0 } },
    ]);
    const opening = openingFrame({ frames });

    expect(opening?.title).toBe("战斗开始结算");
    expect(opening?.events).toEqual([]);
    expect(opening?.players).toEqual(frames[0]?.players);
    expect(frames.findIndex((frame) => frame.title === "战斗开始结算"))
      .toBeLessThan(frames.findIndex((frame) => frame.title === "第 1 回合开始结算"));
  });

  test("标题、事件详情与播放控件统一表达已完成的阶段结算", () => {
    const frames = completedPhaseFrames();
    const phaseFrames = frames.filter((frame) => frame.actionIndex === null);

    expect(phaseFrames.map((frame) => frame.title)).toEqual([
      "初始状态",
      "战斗开始结算",
      "第 1 回合开始结算",
      "第 1 回合结束结算",
      "第 1 回合开始结算",
      "战斗结束",
    ]);
    expect(openingFrame({ frames })?.title).toBe("战斗开始结算");

    const completed = phaseFrames.slice(1, -1);
    const rendered = completed.map((frame) => [
      frameCardLabel(frame),
      framePositionLabel(frame),
    ].join("\n")).join("\n");

    for (const frame of completed) {
      expect(frameCardLabel(frame)).toBe(frame.title);
      expect(framePositionLabel(frame)).toBe(frame.title);
    }
    expect(rendered).not.toContain("战斗开始效果");
    expect(rendered).not.toContain("开局结算");
    expect(rendered).not.toMatch(/第 \d+ 回合开始(?!结算)/);
    expect(rendered).not.toMatch(/第 \d+ 回合结束(?!结算)/);

    // 阶段帧不再各占一个时间轴点：一个点就是一方的一次完整行动，阶段结算折进去。
    const points = timelinePoints(frames);
    expect(points.map((point) => point.actorTurn)).toEqual([0, 1, 2]);
    expect(points.every((point) => point.frames.length > 0)).toBe(true);
    expect(timelinePointLabel(points[0]!)).toBe("开局：战前状态与战斗开始结算");
    expect(timelinePointLabel(points[1]!)).toContain("第 1 回合");
    expect(timelinePointLabel(points[1]!)).toContain("第 1 动");
    expect(points.map((point) => point.actor)).toEqual([null, "p1", "p2"]);
    expect(resourceFlowFrames(frames).map((frame) => frame.index)).toEqual(
      frames.map((frame) => frame.index),
    );
  });

  test("完成态帧命名不改写原始阶段事件语义", () => {
    const frames = completedPhaseFrames();
    const turnStart = frames.find((frame) => frame.title === "第 1 回合开始结算")!;
    const turnEnd = frames.find((frame) => frame.title === "第 1 回合结束结算")!;

    // 帧标题写"完成态"，底层原始事件名必须保持原样，不能被标题改写。
    expect(turnStart.events.find((event) => event.type === "phase")?.name).toBe("turnStart");
    expect(turnEnd.events.find((event) => event.type === "phase")?.name).toBe("turnEnd");
  });

  test("终局阶段钩子保留完成态标题而不是被战斗结束覆盖", () => {
    const turnStartLethal = buildTestFrames([
      { sequence: 1, type: "phase", actorId: "p1", name: "battleStart" },
      { sequence: 2, type: "phase", actorId: "p1", name: "turnStart" },
      { sequence: 3, type: "resource", targetId: "p1", name: "hp", before: 10, after: 0 },
    ], "p2");
    const turnEndLethal = buildTestFrames([
      { sequence: 1, type: "phase", actorId: "p1", name: "battleStart" },
      { sequence: 2, type: "phase", actorId: "p1", name: "turnStart" },
      { sequence: 3, type: "card", actorId: "p1", name: "cardSelected", detail: { cardId: 0, sourceSlot: 0 } },
      { sequence: 4, type: "card", actorId: "p1", name: "cardCompleted", detail: { cardId: 0, sourceSlot: 0 } },
      { sequence: 5, type: "phase", actorId: "p1", name: "turnEnd" },
      { sequence: 6, type: "resource", targetId: "p2", name: "hp", before: 10, after: 0 },
    ]);

    expect(turnStartLethal.at(-1)?.title).toBe("第 1 回合开始结算");
    expect(turnEndLethal.at(-1)?.title).toBe("第 1 回合结束结算");
    expect(turnStartLethal.at(-1)?.winnerId).toBe("p2");
    expect(turnEndLethal.at(-1)?.winnerId).toBe("p1");
  });
});

function completedPhaseFrames(): readonly BattleFrame[] {
  const events: readonly RuleEvent[] = [
    { sequence: 1, type: "phase", actorId: "p1", name: "battleStart" },
    { sequence: 2, type: "phase", actorId: "p1", name: "turnStart" },
    { sequence: 3, type: "card", actorId: "p1", name: "cardSelected", detail: { cardId: 0, sourceSlot: 0 } },
    { sequence: 4, type: "card", actorId: "p1", name: "cardCompleted", detail: { cardId: 0, sourceSlot: 0 } },
    { sequence: 5, type: "phase", actorId: "p1", name: "turnEnd" },
    { sequence: 6, type: "phase", actorId: "p2", name: "turnStart" },
    { sequence: 7, type: "card", actorId: "p2", name: "cardSelected", detail: { cardId: 0, sourceSlot: 0 } },
    { sequence: 8, type: "card", actorId: "p2", name: "cardCompleted", detail: { cardId: 0, sourceSlot: 0 } },
  ];
  return buildTestFrames(events);
}

function buildTestFrames(events: readonly RuleEvent[], winnerId: "p1" | "p2" = "p1"): readonly BattleFrame[] {
  return buildFrames({
    p1: playerView(),
    p2: playerView({ id: "p2", name: "李燚", side: "p2" }),
  }, events, winnerId, 16);
}
