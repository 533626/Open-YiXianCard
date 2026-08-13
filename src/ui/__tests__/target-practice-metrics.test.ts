import { describe, expect, test } from "bun:test";
import type { HookAttackSegment, HookFieldChange, HookStep } from "../hook-trace";
import {
  cardPalette,
  cardPaletteKey,
  computeDamagePerTurn,
  computeTargetPracticeResult,
  OTHER_DAMAGE_KEY,
  targetReachedLabel,
} from "../target-practice-metrics";

function step(overrides: Partial<HookStep> = {}): HookStep {
  return {
    frameIndex: 0,
    category: "mainEffect",
    categoryLabel: "牌面结算",
    actorTurn: 1,
    actor: "p1",
    slot: 0,
    cardId: 82,
    cardName: "万玄破魔掌",
    changes: [],
    attackSegments: [],
    ...overrides,
  };
}

function segment(overrides: Partial<HookAttackSegment> = {}): HookAttackSegment {
  return {
    target: "p2",
    hitIndex: 0,
    hpBefore: 100,
    hpAfter: 90,
    defBefore: 10,
    defAfter: 0,
    ...overrides,
  };
}

function change(overrides: Partial<HookFieldChange> = {}): HookFieldChange {
  return {
    group: "核心",
    key: "hp",
    label: "生命",
    before: 50,
    after: 40,
    side: "p2",
    ...overrides,
  };
}

describe("打靶伤害归因", () => {
  test("mainEffect 攻击段精确归到出牌卡：hp+防 减少都计入，非目标侧段忽略", () => {
    const steps = [
      step({
        actorTurn: 1,
        cardId: 82,
        cardName: "万玄破魔掌",
        attackSegments: [
          segment({ hitIndex: 0, hpBefore: 100, hpAfter: 90, defBefore: 10, defAfter: 0 }),
          segment({ hitIndex: 1, hpBefore: 90, hpAfter: 85, defBefore: 0, defAfter: 0 }),
          // 对方（木桩反打/我方自伤）段不计入我方对 p2 的伤害。
          segment({ target: "p1", hpBefore: 50, hpAfter: 40 }),
        ],
      }),
      // 非我方步骤不归因。
      step({ actor: "p2", actorTurn: 2, cardId: 1, cardName: "敌方牌", attackSegments: [segment()] }),
    ];
    const perTurn = computeDamagePerTurn(steps, "p1");
    expect(perTurn).toEqual([
      {
        round: 1,
        total: 25,
        byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 25 }],
      },
    ]);
  });

  test("mainEffect 无攻击段时读 changes（巨鼎落这类直接改血卡）", () => {
    const steps = [
      step({
        actorTurn: 1,
        cardId: 11,
        cardName: "巨鼎落",
        changes: [change({ before: 60, after: 45 })],
      }),
    ];
    const perTurn = computeDamagePerTurn(steps, "p1");
    expect(perTurn[0]?.byCard).toEqual([{ cardId: 11, cardName: "巨鼎落", damage: 15 }]);
  });

  test("mainEffect 同时有攻击段与直接改血：段 + 残余，不重复计数", () => {
    // 段合计 10（hp 100→90），changes 净减少 15 → 残余 5 补计。
    const mixed = [
      step({
        cardId: 888,
        cardName: "攻+伤",
        attackSegments: [segment({ hpBefore: 100, hpAfter: 90, defBefore: 0, defAfter: 0 })],
        changes: [change({ before: 100, after: 85 })],
      }),
    ];
    expect(computeDamagePerTurn(mixed, "p1")[0]?.total).toBe(15);
    // 纯攻击步骤：changes 净减少 ≤ 段合计（段已含防吸收），残余恒 0。
    const pure = [
      step({
        cardId: 82,
        cardName: "纯攻",
        attackSegments: [segment({ hpBefore: 100, hpAfter: 90, defBefore: 10, defAfter: 0 })],
        changes: [change({ before: 100, after: 90 })],
      }),
    ];
    expect(computeDamagePerTurn(pure, "p1")[0]?.total).toBe(20);
  });

  test("持续伤害（turnStart/turnEnd 无卡步骤）归入「持续/其他」桶", () => {
    const steps = [
      step({
        category: "turnEnd",
        categoryLabel: "回合结束",
        actorTurn: 4,
        cardId: null,
        cardName: null,
        changes: [
          change({ key: "hp", before: 80, after: 70 }),
          change({ key: "defense", before: 5, after: 0 }),
        ],
      }),
      step({
        category: "turnStart",
        categoryLabel: "回合开始",
        actorTurn: 5,
        cardId: null,
        cardName: null,
        changes: [change({ key: "hp", before: 70, after: 72 })], // 回血不计
      }),
    ];
    const perTurn = computeDamagePerTurn(steps, "p1");
    expect(perTurn).toEqual([
      {
        round: 2,
        total: 15,
        byCard: [{ cardId: null, cardName: null, damage: 15 }],
      },
    ]);
    expect(cardPaletteKey(null)).toBe(OTHER_DAMAGE_KEY);
  });

  test("afterCard 步骤的 changes 也归因到该卡", () => {
    const steps = [
      step({
        category: "afterCard",
        categoryLabel: "牌后结算",
        actorTurn: 3,
        cardId: 9000,
        cardName: "暗伤",
        changes: [change({ before: 50, after: 44 })],
      }),
    ];
    const perTurn = computeDamagePerTurn(steps, "p1");
    expect(perTurn[0]?.byCard[0]).toEqual({ cardId: 9000, cardName: "暗伤", damage: 6 });
  });

  test("duel 模式两侧独立归因：p2 步骤只算对 p1 的减少", () => {
    const steps = [
      step({ actor: "p2", actorTurn: 2, cardId: 7, cardName: "敌方攻击", changes: [change({ side: "p1", before: 100, after: 70 })] }),
      step({ actor: "p2", actorTurn: 2, cardId: 7, cardName: "敌方攻击", changes: [change({ side: "p2", before: 50, after: 40 })] }),
    ];
    expect(computeDamagePerTurn(steps, "p2")).toEqual([
      {
        round: 1,
        total: 30,
        byCard: [{ cardId: 7, cardName: "敌方攻击", damage: 30 }],
      },
    ]);
  });

  test("同卡跨回合聚合、同回合多卡并列且按伤害降序", () => {
    const steps = [
      step({ actorTurn: 1, cardId: 1, cardName: "甲", changes: [change({ before: 10, after: 5 })] }),
      step({ actorTurn: 1, cardId: 2, cardName: "乙", changes: [change({ before: 20, after: 10 })] }),
      step({ actorTurn: 3, cardId: 1, cardName: "甲", changes: [change({ before: 30, after: 25 })] }),
    ];
    const perTurn = computeDamagePerTurn(steps, "p1");
    expect(perTurn).toEqual([
      {
        round: 1,
        total: 15,
        byCard: [
          { cardId: 2, cardName: "乙", damage: 10 },
          { cardId: 1, cardName: "甲", damage: 5 },
        ],
      },
      {
        round: 2,
        total: 5,
        byCard: [{ cardId: 1, cardName: "甲", damage: 5 }],
      },
    ]);
  });
});

describe("打靶终局判定", () => {
  const frames = (actorTurn: number) => [{ actorTurn }];

  test("累计 ≥ 阈值即停：reachedTurn 取首个达标回合", () => {
    const steps = [
      step({ actorTurn: 1, changes: [change({ before: 50, after: 0 })] }), // 50
      step({ actorTurn: 3, changes: [change({ before: 100, after: 40 })] }), // 60 → 110
      step({ actorTurn: 5, changes: [change({ before: 100, after: 90 })] }), // 10 → 120 ≥ 120
    ];
    const result = computeTargetPracticeResult(steps, frames(6), 120, 0);
    expect(result.stopReason).toBe("threshold");
    expect(result.reachedTurn).toBe(3);
    expect(result.totalDamage).toBe(120);
    expect(targetReachedLabel(result)).toBe("已达成");
  });

  test("未达标 → turnLimit（打满游戏常量 32 回合）", () => {
    const steps = [step({ actorTurn: 1, changes: [change({ before: 50, after: 40 })] })];
    const result = computeTargetPracticeResult(steps, frames(64), 120, 0);
    expect(result.stopReason).toBe("turnLimit");
    expect(result.reachedTurn).toBe(32);
    expect(targetReachedLabel(result)).toBe("未达成");
  });

  test("默认 displayRounds=1：只显示到打到 120 的那一回合（达标回合起，不出现更早的无效窗口）", () => {
    // 第 3 回合（actorTurn 5）打到 120；第 4 回合还有后续伤害。
    const steps = [
      step({ actorTurn: 1, changes: [change({ before: 50, after: 0 })] }), // 50
      step({ actorTurn: 3, changes: [change({ before: 100, after: 40 })] }), // 60 → 110
      step({ actorTurn: 5, changes: [change({ before: 100, after: 90 })] }), // 10 → 120
      step({ actorTurn: 7, changes: [change({ before: 100, after: 0 })] }), // 100 后续伤害
    ];
    const result = computeTargetPracticeResult(steps, frames(8), 120, 1);
    expect(result.perTurn).toHaveLength(3); // 只到第 3 回合
    expect(result.perTurn.at(-1)?.round).toBe(3);
    expect(result.totalDamage).toBe(120);
  });

  test("displayRounds=4：往后看到第 4 回合（显示到达标回合后一回合）", () => {
    const steps = [
      step({ actorTurn: 1, changes: [change({ before: 50, after: 0 })] }),
      step({ actorTurn: 3, changes: [change({ before: 100, after: 40 })] }),
      step({ actorTurn: 5, changes: [change({ before: 100, after: 90 })] }), // 第 3 回合达标 120
      step({ actorTurn: 7, changes: [change({ before: 100, after: 0 })] }), // 第 4 回合 +100
    ];
    const result = computeTargetPracticeResult(steps, frames(8), 120, 4);
    expect(result.perTurn).toHaveLength(4);
    expect(result.perTurn.at(-1)?.round).toBe(4);
    expect(result.totalDamage).toBe(220); // 120 + 100
  });

  test("displayRounds 必须 ≥ reachedTurn：传更小的值也保证窗口覆盖到达标回合", () => {
    // 第 3 回合达标；传入 displayRounds=1（0..2 是无效窗口）→ 仍从第 3 回合显示。
    const steps = [
      step({ actorTurn: 1, changes: [change({ before: 50, after: 0 })] }), // 50
      step({ actorTurn: 3, changes: [change({ before: 100, after: 40 })] }), // 60 → 110
      step({ actorTurn: 5, changes: [change({ before: 100, after: 90 })] }), // 10 → 120
      step({ actorTurn: 7, changes: [change({ before: 100, after: 0 })] }), // 第 4 回合 +100
    ];
    const result = computeTargetPracticeResult(steps, frames(8), 120, 1);
    expect(result.reachedTurn).toBe(3);
    expect(result.perTurn.at(-1)?.round).toBe(3);
    expect(result.totalDamage).toBe(120);
  });

  test("displayRounds 不超过游戏常量 32 上限", () => {
    const steps = [step({ actorTurn: 1, changes: [change({ before: 50, after: 40 })] })];
    const result = computeTargetPracticeResult(steps, frames(64), 120, 99);
    // 未达标 → reachedTurn=32；displayRounds=99 被钳到 32，但无更多伤害回合。
    expect(result.stopReason).toBe("turnLimit");
    expect(result.reachedTurn).toBe(32);
  });

  test("空钩子链：0 伤、回合上限未达成", () => {
    const result = computeTargetPracticeResult([], frames(30), 120, 0);
    expect(result.perTurn).toEqual([]);
    expect(result.totalDamage).toBe(0);
    expect(result.stopReason).toBe("turnLimit");
    expect(result.reachedTurn).toBe(32);
  });
});

describe("卡牌调色板", () => {
  test("同 baseId 的变体同色、不同卡可稳定分配、无卡归因固定「持续/其他」", () => {
    const palette = cardPalette([82, 82, 11, null]);
    expect(palette.size).toBe(3);
    expect(palette.get(cardPaletteKey(82))).toBe(palette.get(cardPaletteKey(82)));
    expect(palette.get(OTHER_DAMAGE_KEY)).toBe("card-other");
    // 同一输入序列结果稳定。
    expect([...cardPalette([82, 11, null]).values()]).toEqual([...cardPalette([82, 11, null]).values()]);
    // 只取出现的卡：图例不列未出现的卡。
    expect(cardPalette([1]).has(cardPaletteKey(2))).toBe(false);
  });
});
