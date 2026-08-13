import { describe, expect, test } from "bun:test";
import { buildBattleExplanation } from "../battle-explanation";
import type {
  CounterfactualReport,
  RuleImpactCheckpoint,
  RuleImpactReport,
} from "../battle-explanation";

function contribution(
  partial: Partial<RuleImpactCheckpoint["contribution"]>,
): RuleImpactCheckpoint["contribution"] {
  const filled = {
    hp: 0,
    defense: 0,
    guard: 0,
    resource: 0,
    debuff: 0,
    tempo: 0,
    ...partial,
  };
  return {
    ...filled,
    total: filled.hp + filled.defense + filled.guard + filled.resource + filled.debuff +
      filled.tempo,
    hpLossPreventedByGuard: partial.hpLossPreventedByGuard ?? 0,
    hpLossPreventedByDefense: partial.hpLossPreventedByDefense ?? 0,
  };
}

function card(
  actionIndex: number,
  cardName: string,
  partial: Partial<RuleImpactCheckpoint["contribution"]>,
): RuleImpactCheckpoint {
  return {
    checkpointIndex: actionIndex,
    kind: "cardCompleted",
    actorTurn: actionIndex,
    actor: "p1",
    cardActionIndex: actionIndex,
    cardId: 100 + actionIndex,
    cardName,
    contribution: contribution(partial),
  };
}

function report(
  checkpoints: readonly RuleImpactCheckpoint[],
  auditDeltaForSide = 0,
): RuleImpactReport {
  const total = checkpoints.reduce((sum, point) => sum + point.contribution.total, 0);
  return {
    schemaVersion: "canonical-rule-impact-v1",
    side: "p1",
    startValueForSide: 0,
    terminalValueForSide: total,
    terminalDeltaForSide: total,
    auditDeltaForSide,
    checkpoints,
    cards: [],
  };
}

function counterfactual(
  elements: CounterfactualReport["elements"],
): CounterfactualReport {
  return {
    schemaVersion: "canonical-counterfactual-v1",
    side: "p1",
    baselineTerminalHpDeltaForSide: 8,
    elements,
  };
}

describe("战斗赢法结论层", () => {
  test("按通道绝对贡献排序并给出占比", () => {
    const explanation = buildBattleExplanation(report([
      card(1, "云剑·追风", { hp: 12, defense: 2 }),
      card(2, "连环剑阵", { hp: 6, resource: 4, debuff: -1 }),
    ]));

    expect(explanation.channels.map((channel) => channel.key)).toEqual([
      "hp",
      "resource",
      "defense",
      "debuff",
    ]);
    expect(explanation.channels[0]?.delta).toBe(18);
    expect(explanation.channels[0]?.share).toBeCloseTo(18 / 25, 5);
    expect(explanation.valueDelta).toBe(23);
  });

  test("转折点取绝对值最大的结算点，但按动序展示", () => {
    const explanation = buildBattleExplanation(report([
      card(1, "起手", { hp: 2 }),
      card(2, "小亏", { hp: -9 }),
      card(3, "决胜", { hp: 14 }),
      card(4, "补刀", { hp: 5 }),
    ]));

    expect(explanation.turningPoints.map((point) => point.actionIndex)).toEqual([2, 3, 4]);
    expect(explanation.turningPoints[0]?.delta).toBe(-9);
    // 赢法句子要回答"怎么取得的"：点名贡献最大的牌，而不是只报一个通道占比。
    expect(explanation.headline).toContain("生命优势");
    expect(explanation.headline).toContain("「决胜」");
    expect(explanation.headline).toContain("「补刀」");
    expect(explanation.leadingCards[0]).toMatchObject({
      cardName: "决胜",
      byOpponent: false,
      channelDelta: 14,
    });
    expect(explanation.headline).toContain("决胜");
  });

  test("吸收遥测不再冒充反事实生命收益", () => {
    const explanation = buildBattleExplanation(report([
      card(1, "云剑·追风", { hp: 9 }),
      card(2, "护体挡刀", { guard: -60, hpLossPreventedByGuard: 40 }),
    ]));

    // 通道条只认 value 分：护体这一段仍然是 -60，挡掉的 40 点生命不混进去。
    expect(explanation.channels.find((channel) => channel.key === "guard")?.delta).toBe(-60);
    expect(explanation.headline).not.toContain("挡下");
    expect(explanation.counterfactuals).toEqual([]);
  });

  test("反事实逐项保留零边际贡献与分叉标注", () => {
    const explanation = buildBattleExplanation(
      report([card(1, "全挡住", { hpLossPreventedByDefense: 18 })]),
      counterfactual([
        {
          element: {
            id: "opening-defense",
            label: "开局防御 2",
            side: "p1",
            field: "defense",
            amount: 2,
          },
          firstDivergenceActorTurn: null,
          firstDivergenceCheckpointIndex: null,
          firstDivergenceReason: null,
          preDivergenceHpDeltaChangeForSide: 0,
          terminalHpDeltaChangeForSide: 0,
          counterfactualTerminalHpDeltaForSide: 8,
          baselineWinner: "p1",
          counterfactualWinner: "p1",
          winnerChanged: false,
        },
        {
          element: {
            id: "opening-guard",
            label: "开局护体 1 层",
            side: "p1",
            field: "guard",
            amount: 1,
          },
          firstDivergenceActorTurn: 17,
          firstDivergenceCheckpointIndex: 20,
          firstDivergenceReason: "eventSequence",
          preDivergenceHpDeltaChangeForSide: -6,
          terminalHpDeltaChangeForSide: -28,
          counterfactualTerminalHpDeltaForSide: -20,
          baselineWinner: "p1",
          counterfactualWinner: "p2",
          winnerChanged: true,
        },
      ]),
    );

    expect(explanation.channels).toEqual([]);
    expect(explanation.headline).toBe("本场没有产生正向价值积累。");
    expect(explanation.counterfactuals.map((item) => item.terminalHpDeltaChangeForSide))
      .toEqual([0, -28]);
    expect(explanation.counterfactuals[1]?.firstDivergenceActorTurn).toBe(17);
  });

  test("零贡献通道不进结论，也不产生赢法句子", () => {
    const explanation = buildBattleExplanation(report([card(1, "空转", {})]));

    expect(explanation.channels).toEqual([]);
    expect(explanation.turningPoints).toEqual([]);
    expect(explanation.headline).toBe("本场没有产生正向价值积累。");
  });

  test("审计非零时不给赢法结论，交由渲染层显式降级", () => {
    const explanation = buildBattleExplanation(
      report([card(1, "云剑·追风", { hp: 12 })], 3.5),
    );

    expect(explanation.auditDelta).toBe(3.5);
    // 归因对不上终局价值变化时，上层必须报告降级而不是照常展示这句话。
    expect(explanation.channels.length).toBeGreaterThan(0);
  });

  test("对手结算点标注行动方，且不进赢法句子", () => {
    const opponentPeak: RuleImpactCheckpoint = {
      ...card(2, "对手大招", { hp: -30 }),
      actor: "p2",
    };
    const explanation = buildBattleExplanation(report([
      card(1, "云剑·追风", { hp: 40 }),
      opponentPeak,
    ]));

    expect(explanation.turningPoints.map((point) => point.byOpponent)).toEqual([false, true]);
    // "主要记在…的结算点上"只能引用己方的牌。
    expect(explanation.headline).toMatch(/主要记在「云剑·追风」[^；]*的结算点上/u);
    // 对手那一刀可以出现，但必须带「对手」前缀并说成"打回"，不能读成我方赢法。
    expect(explanation.headline).toContain("对手「对手大招」");
    expect(explanation.headline).toContain("打回最多");
    expect(explanation.headline.indexOf("对手大招"))
      .toBeGreaterThan(explanation.headline.indexOf("云剑·追风"));
  });

  test("只统计 cardCompleted，回合钩子不冒充成一张牌", () => {
    const explanation = buildBattleExplanation(report([
      {
        checkpointIndex: 1,
        kind: "turnStart",
        actorTurn: 1,
        actor: "p1",
        contribution: contribution({ resource: 20 }),
      },
      card(2, "云剑·追风", { hp: 4 }),
    ]));

    expect(explanation.turningPoints.map((point) => point.cardName)).toEqual(["云剑·追风"]);
    // 通道统计仍然包含回合钩子：它是这场价值变化的一部分。
    expect(explanation.channels.map((channel) => channel.key)).toEqual(["resource", "hp"]);
  });
});
