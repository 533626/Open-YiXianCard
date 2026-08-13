import { describe, expect, test } from "bun:test";
import { renderResourceFlow } from "../render-battle-flow";
import type { HookStep } from "../hook-trace";
import { battleFrame, playerView } from "./layout-test-helpers";

function flowFrames(): ReturnType<typeof battleFrame>[] {
  return [
    battleFrame([], {
      index: 0,
      actorTurn: 0,
      actionIndex: null,
      actorId: null,
      title: "初始状态",
      players: {
        p1: playerView({ hp: 100, defense: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 100, defense: 0 }),
      },
    }),
    battleFrame([], {
      index: 1,
      actorTurn: 1,
      actionIndex: 1,
      actorId: "p1",
      title: "第 1 动",
      players: {
        p1: playerView({ hp: 100, defense: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90, defense: 0 }),
      },
    }),
    battleFrame([], {
      index: 2,
      actorTurn: 2,
      actionIndex: 2,
      actorId: "p2",
      title: "第 2 动",
      players: {
        p1: playerView({ hp: 80, defense: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90, defense: 20 }),
      },
    }),
    battleFrame([], {
      index: 3,
      actorTurn: 3,
      actionIndex: 3,
      actorId: "p1",
      title: "第 3 动",
      players: {
        p1: playerView({ hp: 80, defense: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 75, defense: 15 }),
      },
    }),
  ];
}

describe("生命曲线模块", () => {
  test("默认显示生命曲线，模块内部不再放第二个 toggle", () => {
    const html = renderResourceFlow(flowFrames(), 1);
    expect(html).toContain('aria-label="生命曲线"');
    expect(html).toContain("玩家一生命");
    expect(html).toContain("玩家二生命");
    expect(html).toContain("生命差");
    expect(html).not.toContain("玩家一伤害");
    // toggle 已上移到模块选项卡（render-battle.ts 的 .trajectory-switch），
    // 模块内部只留口径提示，不再重复出现生命/伤害选择控件。
    expect(html).not.toContain("select-trajectory-metric");
    expect(html).not.toContain("trajectory-option");
  });

  test("伤害曲线按回合聚合堆叠柱：每段按卡牌来源，trace 精确归因", () => {
    const steps: readonly HookStep[] = [{
      frameIndex: 1,
      category: "mainEffect",
      categoryLabel: "牌面结算",
      actorTurn: 1,
      actor: "p1",
      slot: 2,
      cardId: 82,
      cardName: "万玄破魔掌",
      changes: [],
      attackSegments: [{
        target: "p2",
        hitIndex: 0,
        hpBefore: 100,
        hpAfter: 90,
        defBefore: 10,
        defAfter: 0,
      }],
    }];
    const html = renderResourceFlow(flowFrames(), 3, "damage", steps);
    expect(html).toContain('aria-label="伤害曲线"');
    expect(html).toContain("每回合伤害（按卡牌来源）");
    // 两侧各一个按卡牌来源的堆叠柱（标签用帧内角色名）。
    expect(html).toContain("姬方生 · 每回合伤害（按卡牌来源）");
    expect(html).toContain("李燚 · 每回合伤害（按卡牌来源）");
    // 第 1 回合 p1 打掉 p2 10 生命 + 10 防御 → 20 伤，归到万玄破魔掌。
    expect(html).toContain("第 1 回合 · 万玄破魔掌 20 伤");
    // 不再是每动差分估计折线。
    expect(html).not.toContain("每动对对方造成的伤害估计");
    expect(html).not.toContain("flow-metric dmg-p1");
    expect(html).not.toContain("伤害差");
  });

  test("双方伤害图带用量面板摘要：本回合伤害 + 累计伤害，与选中回合同步", () => {
    const steps: readonly HookStep[] = [
      {
        frameIndex: 1,
        category: "mainEffect",
        categoryLabel: "牌面结算",
        actorTurn: 1,
        actor: "p1",
        slot: 2,
        cardId: 82,
        cardName: "万玄破魔掌",
        changes: [],
        attackSegments: [{ target: "p2", hitIndex: 0, hpBefore: 100, hpAfter: 90, defBefore: 10, defAfter: 0 }],
      },
      {
        frameIndex: 3,
        category: "mainEffect",
        categoryLabel: "牌面结算",
        actorTurn: 3,
        actor: "p1",
        slot: 2,
        cardId: 82,
        cardName: "万玄破魔掌",
        changes: [],
        attackSegments: [{ target: "p2", hitIndex: 0, hpBefore: 100, hpAfter: 95, defBefore: 0, defAfter: 0 }],
      },
    ];
    // 选中第 2 帧（actorTurn 2 → R1）：p1 第 1 回合伤害 = 20。
    const html = renderResourceFlow(flowFrames(), 2, "damage", steps);
    expect(html).toContain('class="usage-summary"');
    // 两侧各一组读数；p1 侧选中回合 R1 的伤害 = 20，累计 = 25。
    expect(html).toContain("本回合伤害");
    expect(html).toContain("累计伤害");
    // 每侧一个 usage-summary（姬方生 / 李燚）。
    expect(html.match(/class="usage-summary"/g)).toHaveLength(2);
    // 累计趋势层在两幅图里都有。
    expect(html.match(/class="cumulative-area"/g)).toHaveLength(2);
    // 选中回合游标仍在柱层：selectedRound 与 timeline 同步。
    expect(html).toContain('class="bar-seg card-');
  });

  test("伤害曲线在钩子链不可用时明示，不画误导成「0 伤」的空图", () => {
    const html = renderResourceFlow(flowFrames(), 3, "damage", undefined);
    expect(html).toContain('aria-label="伤害曲线"');
    expect(html).toContain("resource-flow-unavailable");
    expect(html).toContain("无法按卡牌归因伤害");
    expect(html).not.toContain("stacked-chart-svg");
  });

  test("没有行动帧时两种口径都不渲染", () => {
    const stageOnly = battleFrame([], { actionIndex: null });
    expect(renderResourceFlow([stageOnly], 0)).toBe("");
    expect(renderResourceFlow([stageOnly], 0, "damage")).toBe("");
  });
});
