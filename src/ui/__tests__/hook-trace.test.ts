import { describe, expect, test } from "bun:test";
import {
  adaptHookTrace,
  changedFieldCount,
  hookStepsByActorTurn,
  hookStepsForFrame,
} from "../hook-trace";
import type { RustHookTraceStep } from "../hook-trace";

function step(overrides: Partial<RustHookTraceStep> = {}): RustHookTraceStep {
  return {
    eventIndex: 0,
    category: "mainEffect",
    turn: 1,
    actor: "p1",
    slot: null,
    cardId: null,
    cardName: null,
    p1Changes: [],
    p2Changes: [],
    attackSegments: [],
    ...overrides,
  };
}

describe("Rust 钩子链适配", () => {
  test("事件下标映射到帧下标，双方改动合并后带上行动方", () => {
    const steps = adaptHookTrace({
      steps: [
        step({
          eventIndex: 2,
          category: "turnStart",
          turn: 3,
          actor: "p2",
          p1Changes: [{ group: "核心", key: "hp", label: "生命", before: 40, after: 33 }],
          p2Changes: [{ group: "核心", key: "anima", label: "灵气", before: 1, after: 3 }],
        }),
      ],
    }, 6);

    expect(steps).toHaveLength(1);
    expect(steps[0]!.frameIndex).toBe(2);
    expect(steps[0]!.actorTurn).toBe(3);
    expect(steps[0]!.categoryLabel).toBe("回合开始");
    expect(steps[0]!.changes.map((change) => [change.side, change.key])).toEqual([
      ["p1", "hp"],
      ["p2", "anima"],
    ]);
    expect(changedFieldCount(steps as never[])).toBe(2);
  });

  /**
   * parity 事件流会省掉未走完那一回合的回合结束边界，所以 detailed 侧的尾部下标
   * 可能落在帧序列之外。把它平移进来会把钩子记到别的牌上，只能丢掉。
   */
  test("落在帧序列之外的步骤直接丢掉，而不是平移下标", () => {
    const steps = adaptHookTrace({
      steps: [
        step({ eventIndex: 0 }),
        step({ eventIndex: 1, category: "battleEnd" }),
        step({ eventIndex: 2, category: "battleEnd" }),
      ],
    }, 2);

    expect(steps.map((item) => item.frameIndex)).toEqual([0, 1]);
  });

  test("按帧和按 actorTurn 取用同一份步骤", () => {
    const steps = adaptHookTrace({
      steps: [
        step({ eventIndex: 0, turn: 1, category: "turnStart" }),
        step({ eventIndex: 1, turn: 1, category: "mainEffect", cardName: "云剑•游龙" }),
        step({ eventIndex: 2, turn: 2, category: "turnStart", actor: "p2" }),
      ],
    }, 8);

    expect(hookStepsForFrame(steps, 1).map((item) => item.cardName)).toEqual(["云剑•游龙"]);
    expect(hookStepsForFrame(undefined, 2)).toEqual([]);
    expect(hookStepsByActorTurn(steps).map((group) => [group.actorTurn, group.steps.length]))
      .toEqual([[1, 2], [2, 1]]);
  });

  test("画龙点睛临时升级保留独立钩子、牌名和层数变化", () => {
    const steps = adaptHookTrace({
      steps: [
        step({
          category: "temporaryUpgrade",
          slot: 1,
          cardId: 10_000,
          cardName: "普通攻击",
          p1Changes: [{
            group: "仙命",
            key: "paintFinishingTouch",
            label: "画龙点睛",
            before: 1,
            after: 0,
          }],
        }),
      ],
    }, 2);

    expect(steps).toHaveLength(1);
    expect(steps[0]).toMatchObject({
      category: "temporaryUpgrade",
      categoryLabel: "临时升级",
      slot: 1,
      cardId: 10_000,
      cardName: "普通攻击",
    });
    expect(steps[0]!.changes).toContainEqual({
      side: "p1",
      group: "仙命",
      key: "paintFinishingTouch",
      label: "画龙点睛",
      before: 1,
      after: 0,
    });
  });
});
