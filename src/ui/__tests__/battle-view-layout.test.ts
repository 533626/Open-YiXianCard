import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "bun:test";
import { CoreBuff, type RuleEvent } from "../domain";
import { isSourceMapped, sourceLabel } from "../battle-event-hooks";
import {
  adjacentCompletedTurnFrameIndex,
  allStepFrameIndexes,
  battleStepShortcutDirection,
  completedTurnFrameIndexes,
  firstCompletedTurnFrameIndex,
} from "../battle-keyboard";
import { adaptHookTrace } from "../hook-trace";
import battleEventLabels from "../generated/battle-event-labels.json";
import {
  collectBattleStatItems,
  collectBattleStatusItems,
  renderBattleStatRibbon,
} from "../player-battle-state";
import { renderBattleResult } from "../render-battle";
import { renderPlayerDeck } from "../render-player-deck";
import { playedSlotsForCurrentTurn, renderPlayerPanel } from "../render-player-panel";
import { renderApp } from "../render";
import { rustBuffs, type RustSnapshot } from "../rust-wasm-engine";
import { buffLabel } from "../view-utils";
import { baseState, battleFrame, playerView, simulationResult } from "./layout-test-helpers";

describe("UI 战斗视图布局契约", () => {
  test("战斗状态按真实层数独立显示", () => {
    const player = playerView({
      buffs: {
        [CoreBuff.Recovery]: 8,
        [CoreBuff.InternalInjury]: 5,
        UnknownInternalCounter: 2,
      },
    });

    expect(collectBattleStatusItems(player)).toEqual([
      { label: "恢复", value: 8, kind: "buff", zone: "character" },
      { label: "内伤", value: 5, kind: "buff", zone: "character" },
    ]);
    const previous = playerView({
      anima: 3,
      buffs: {
        [CoreBuff.Recovery]: 8,
        [CoreBuff.InternalInjury]: 2,
        [CoreBuff.AttackBonus]: 4,
      },
    });
    player.anima = 2;
    const html = renderBattleStatRibbon(player, { previous });
    expect(html).toContain('class="player-live-stats"');
    expect(html).toContain("防");
    expect(html).toContain("恢复");
    expect(html).toContain(">8<");
    expect(html).toContain("内伤");
    expect(html).toContain("2→5");
    expect(html).toContain("灵</em>3→2");
    expect(html).toContain("加攻");
    expect(html).toContain("4→0");
  });

  test("Rust 运行态资源和状态使用现有中文投影键", () => {
    const snapshot: RustSnapshot = {
      hp: 50,
      maxHp: 50,
      defense: 0,
      anima: 0,
      guard: 0,
      physique: 2,
      swordIntent: 3,
      sharpness: 4,
      cloudChain: 5,
      cloudSea: 6,
      momentum: 3,
      agility: 0,
      waterMomentum: 7,
      activatedMetal: 0,
      activatedWater: 0,
      activatedWood: 1,
      activatedFire: 0,
      activatedEarth: 0,
      hexagram: 8,
      starPower: 9,
      attackBonus: 0,
      internalInjury: 0,
      weakness: 0,
      flaw: 0,
      attackReduction: 0,
      entangle: 0,
      externalInjury: 0,
      lostMind: 10,
      actionAgainCount: 11,
      quanStance: 0,
      gunStance: 0,
      // 全量暴露缺口（档 1a/1b）字段：本测试用零值（rustBuffs 过滤零值键）。
      metalRing: 0,
      swordEnergy: 0,
      waterMonthSwordFormation: 0,
      waterFormation: 0,
      metalFormation: 0,
      earthFormation: 0,
      fireFormation: 0,
      springFlow: 0,
      waterStealth: 0,
      metalIronBone: 0,
      earthEightWastes: 0,
      woodArray: 0,
      turtleFormation: 0,
      shatterFormation: 0,
      thunderFormation: 0,
      evilGuFormation: 0,
      spiritGatheringFormation: 0,
      heavenCycleSwordFormation: 0,
      heavenForceFormation: 0,
      flowerMazeFormation: 0,
      immovableFormation: 0,
      eightGatesFormation: 0,
      sixYaoFormation: 0,
      bengQuanCunJin: 0,
      bengQuanReturnProfound: 0,
      dreamBengQuanChain: 0,
      immortalBindingTune: 0,
      illusoryTune: 0,
      heartbreakTune: 0,
      wildDanceTune: 0,
      rejuvenationTune: 0,
      xiaoyaoTune: 0,
      xiaoyaoGuqin: 0,
      chaoticMindTune: 0,
      lingGuaArt: 0,
      starMoonFan: 0,
      infiniteHexagramPlate: 0,
      allGoesWell: 0,
      recovery: 0,
      meditation: 0,
      bloodCalamity: 0,
      loneNightWolf: 0,
      leafBladeFlower: 0,
      quietMindset: 0,
      reflectMindset: 0,
      graftFlowersToTree: 0,
      tide: 0,
      dismantleMove: 0,
      allThingsInauspicious: 0,
      fateCycle: 0,
      yellowBirdBehind: 0,
      exorcism: 0,
      iceSnowLotus: 0,
      leafShieldFlower: 0,
      paintFinishingTouch: 0,
      nextTurnDefense: 0,
      ignoreDefenseAttacks: 0,
      nextAttackShatterDefense: 0,
      momentumLimit: 9,
      lastElement: "wood",
      cardQueue: [1, 0],
      slots: [],
    };
    const player = playerView({
      defense: 0,
      momentum: snapshot.momentum,
      momentumLimit: snapshot.momentumLimit,
      lastElement: snapshot.lastElement,
      buffs: rustBuffs(snapshot),
    });

    // 体魄按用户约束保留显示；连云（BuffType.LianYun=304）原版 BuffConfig 分类为
    // Hidden，按 RefreshBuff 规则不建图标。
    expect(collectBattleStatItems(player)).toEqual([
      { label: "卦象", value: 8, hot: true },
      { label: "星力", value: 9, hot: true },
      { label: "体魄", value: 2 },
      { label: "气势", value: "3/9", hot: true },
      { label: "五行", value: "木" },
    ]);
    expect(collectBattleStatusItems(player).map(({ label, value, zone }) => [label, value, zone])).toEqual([
      ["剑意", 3, "character"], // BuffType.JianYi(6) Positive → 角色区
      ["锋锐", 4, "character"], // BuffType.FengRui(8) Positive → 角色区
      ["云海", 6, "character"], // BuffType.YunHai(624) Positive → 角色区
      ["水势", 7, "character"], // BuffType.ShuiShi(10) Positive → 角色区
      ["食滞", 10, "character"], // BuffType.ShiZhi(393) Negative → 角色区
      ["激活木灵", 1, "card"], // BuffType.JiHuoMuLing(238) Neutral → 卡牌区（五行累计激活次数）
    ]);
    expect(rustBuffs(snapshot)).not.toHaveProperty("ExActionAgain");
  });

  test("战斗牌槽显示 Rust 当前牌，而不是继续显示初始构筑", () => {
    const state = baseState();
    const player = state.config.players.p1;
    const runtime = playerView({
      slots: [{
        index: 0,
        cardId: 99_010_001,
        baseId: 99_000_001,
        name: "运行态替牌",
        skipped: true,
        hadUsed: true,
        temporarilyUpgraded: false,
      }],
    });

    const html = renderPlayerDeck({
      state,
      side: "p1",
      player,
      frame: battleFrame([], { actorId: "p1", sourceSlot: 0 }),
      runtime,
    });

    expect(html).toContain("运行态替牌");
    expect(html).toContain("deck-slot skipped active used");
  });

  test("事件来源字段映射为原作中文", () => {
    expect(sourceLabel("damage:defense")).toBe("伤害 防");
    expect(sourceLabel("buff:physique")).toBe("状态 体魄");
    expect(sourceLabel("talent:204")).toBe("仙命 凡躯");
    expect(sourceLabel("card:204")).toBe("牌面 踏鹤飞云");
    expect(sourceLabel("card:82:physique")).toBe("牌面 万玄破魔掌 · 体魄");
    expect(sourceLabel("card:82:battlePhysiqueGain")).toBe("牌面 万玄破魔掌 · 本场体魄增加");
    expect(sourceLabel("fateStrategy:135:woodArray")).toBe("天衍 灵阵回响 · 木阵");
    expect(sourceLabel("deathCheckpoint:firePhoenixRevive")).toBe("濒死检查 · 浴火凤凰");
    expect(sourceLabel("attackSegment:actualDamage")).toBe("攻击结算 · 实际伤害");
    expect(sourceLabel("card:82:talent:138:totalDefenseGained")).toBe("牌面 万玄破魔掌 · 仙命 五行爆炎 · 累计获得防");
    expect(sourceLabel("buff:spiritTurtleFootwork")).toBe("状态 灵玄迷踪步");
    expect(sourceLabel("buff:sixYaoFormation")).toBe("状态 六爻绝阵");
    expect(sourceLabel("buff:fateCycle")).toBe("状态 命运轮回");
    expect(sourceLabel("buff:hundredBirdTrailingShadowArt")).toBe("状态 百鸟曳影诀");
    expect(isSourceMapped("buff:spiritTurtleFootwork")).toBe(true);
    expect(sourceLabel("future:unmapped")).toBe("机制来源");
    expect(isSourceMapped("future:unmapped")).toBe(false);
  });

  test("内部 Buff 标识使用原作名称，不按拼音猜写", () => {
    expect(buffLabel("LingGuiMiZongBu")).toBe("灵玄迷踪步");
    expect(buffLabel("LingGuiMiZongBuShengXiao")).toBe("灵玄迷踪步生效");
    expect(buffLabel("JianYiLiuZhuan")).toBe("剑意流转");
    expect(buffLabel("MengDuanQuan")).toBe("梦•锻拳");
    expect(buffLabel("MengDuanQuanJieShu")).toBe("梦•锻拳结算");
    expect(buffLabel("JianZhenHuTi")).toBe("护体剑阵");
    expect(buffLabel("TempHuTi")).toBe("料敌机先临时护体");
    expect(buffLabel("XingYueYuShan")).toBe("星月折扇");
    expect(buffLabel("ZhaiHuaFeiYe")).toBe("摘花飞叶");
    expect(battleEventLabels.buffLabels.LingGuiMiZongBu).toEqual({
      label: "灵玄迷踪步",
      sourceKind: "card",
      sourceId: 10_000_050,
    });
    expect(battleEventLabels.buffLabels.XingYueYuShan).toEqual({
      label: "星月折扇",
      sourceKind: "card",
      sourceId: 23,
    });
    expect(battleEventLabels.buffLabels.BengQuanFanXuan.label).toBe("崩拳返玄");
    expect(battleEventLabels.buffLabels.AnXingBianFu.label).toBe("暗星蝙蝠");
    expect(battleEventLabels.buffLabels.ChanXinJuLingYiChuFa.label).toBe("禅心聚灵已触发");
    expect(battleEventLabels.buffLabels.DuanGuPlusHp.label).toBe("幻•锻骨生命回复");
    expect(battleEventLabels.buffLabels.DuanGuPlusDmg.label).toBe("幻•锻骨伤害");
    expect(battleEventLabels.buffLabels.QiCaiLingHe.label).toBe("七彩灵鹤");
    expect(battleEventLabels.buffLabels.JiaGongZhuanMuCi.label).toBe("梦•木灵阵加攻转木刺");
    expect(battleEventLabels.buffLabels.QiChenDanTianPlus.label).toBe("幻•气沉丹田攻击强化");
    expect(battleEventLabels.buffLabels.ChanPlus).toEqual({
      label: "幻•崩拳缠",
      sourceKind: "card",
      sourceId: 319,
    });
    expect(battleEventLabels.buffLabels.MengLingXuanHuaShenQi.label).toBe("梦•灵玄迷踪步化神效果");
    expect(battleEventLabels.buffLabels.MengLingXuanChuFaCiShu.label).toBe("梦•灵玄迷踪步本轮触发次数");
    expect(battleEventLabels.buffLabels.MingYeMiZongBuShengXiao.label).toBe("冥夜迷踪步生效");
    expect(battleEventLabels.buffLabels.LuoHuaYouYiPlus).toEqual({
      label: "幻•落花有意",
      sourceKind: "card",
      sourceId: 289,
    });
    expect(battleEventLabels.buffLabels.TuLingZhenPlus).toEqual({
      label: "幻•土灵阵",
      sourceKind: "card",
      sourceId: 317,
    });
    expect(battleEventLabels.buffLabels.HuanQinZhuoJianZaiCiXingDong).toEqual({
      label: "幻•勤拙剑",
      sourceKind: "card",
      sourceId: 323,
    });
    expect(battleEventLabels.buffLabels.MuCi).toEqual({
      label: "木刺",
      sourceKind: "internal-counter",
    });
    expect(battleEventLabels.buffLabels.TiaoZhiShangZhang.label).toBe("跳至");
    expect(battleEventLabels.buffLabels.XiaZhangPaiSuanZuoKuangJian.label).toBe("下张牌算作狂剑");
    expect(battleEventLabels.buffLabels.XiaZhangPaiSuanZuoKuangJianCengJi.label)
      .toBe("下张牌算作狂剑生效层级");
    expect(battleEventLabels.talentLabels["204"]).toEqual({
      label: "凡躯",
      sourceKind: "talent",
      sourceId: 204,
    });
    expect(battleEventLabels.fateStrategyLabels["135"]).toEqual({
      label: "灵阵回响",
      sourceKind: "fate-strategy",
      sourceId: 135,
    });
    expect(battleEventLabels.sourceTokenLabels.sixYaoFormation).toEqual({
      label: "六爻绝阵",
      sourceKind: "card",
      sourceId: 4_000_014,
    });
  });

  test("右列用模块选项卡，选中的模块独占下方空间", () => {
    const state = baseState();
    state.result = simulationResult([battleFrame([])], 1);
    const html = renderBattleResult(state);

    expect(html).toContain('class="battle-module-tabs"');
    expect(html).toContain('data-action="select-battle-module"');
    expect(html).toContain('data-module="insight"');
    expect(html).toContain('data-module="trajectory"');
    expect(html).toContain('data-module="advice"');
    // 引擎体只有一份；生命曲线/获胜建议复用固定的 companion 槽位。
    expect(html.match(/class="battle-module-body"/gu)).toHaveLength(1);
    expect(html).toContain('<div class="battle-module-body" data-module="insight"');
    expect(html).toContain('class="insight-companion" data-module="trajectory"');
    // 折叠分组与「逐动明细」独立入口都不复存在。
    expect(html).not.toContain("battle-detail-group");
    expect(html).not.toContain("逐动明细");
    expect(html).not.toContain('class="current-action-panel"');
  });

  test("时间轴与帧导航不属于任何模块，切模块也能逐动看", () => {
    const state = baseState();
    state.result = simulationResult([battleFrame([])], 1);
    for (const module of ["insight", "trajectory", "advice"] as const) {
      state.battleModule = module;
      const html = renderBattleResult(state);
      const railIndex = html.indexOf('class="battle-progress-rail"');
      const bodyIndex = html.indexOf('class="battle-module-body"');
      expect(railIndex).toBeGreaterThan(-1);
      expect(html).toContain('data-action="jump-frame"');
      // 轨道必须排在模块体之前，也就是不在模块内部。
      expect(railIndex).toBeLessThan(bodyIndex);
      expect(html).toContain(
        `class="insight-companion" data-module="${
          module === "advice" ? "advice" : "trajectory"
        }"`,
      );
      expect(html).toContain('<div class="battle-module-body" data-module="insight"');
    }
  });

  test("空战斗区显示模拟器功能介绍与使用指南，不再有动态提示与卡组进度", () => {
    const state = baseState();
    state.config.players.p1.deck[0] = { baseId: 1, level: 0 };
    state.config.players.p2.deck[0] = { baseId: 2, level: 0 };
    state.config.players.p2.deck[1] = { baseId: 3, level: 0 };

    const html = renderBattleResult(state);
    expect(html).toContain('aria-label="模拟器说明"');
    expect(html).toContain("弈仙牌战斗模拟器");
    expect(html).toContain('class="simulator-intro-features"');
    expect(html).toContain('class="simulator-intro-guide"');
    expect(html).not.toContain("战斗准备状态");
    expect(html).not.toContain("玩家一卡组");
    expect(html).not.toContain("选好双方角色并各摆 1 张场上牌即自动推演");
    expect(html).not.toContain("检查先手与轮次");
    expect(html).not.toContain('class="empty-state"');
  });

  test("引擎透视按步进轮播：一次步进渲染该 actorTurn 的全部钩子", () => {
    const state = baseState();
    state.result = {
      ...simulationResult([
        battleFrame([], { index: 0, actionIndex: null, title: "战斗开始结算" }),
        battleFrame([], { index: 1, actionIndex: null, title: "第 1 回合开始结算" }),
        battleFrame([], { index: 2, actionIndex: 1, title: "第 1 动 · 万玄破魔掌" }),
      ], 1),
      hookSteps: adaptHookTrace({
        steps: [
          {
            eventIndex: 0,
            category: "turnStart",
            turn: 1,
            actor: "p1",
            slot: null,
            cardId: null,
            cardName: null,
            p1Changes: [
              { group: "核心", key: "anima", label: "灵气", before: 0, after: 2 },
            ],
            p2Changes: [],
          },
          {
            eventIndex: 1,
            category: "mainEffect",
            turn: 1,
            actor: "p1",
            slot: 3,
            cardId: 82,
            cardName: "万玄破魔掌",
            p1Changes: [],
            p2Changes: [
              { group: "核心", key: "hp", label: "生命", before: 50, after: 43 },
            ],
          },
        ],
      }, 3),
    };
    state.frameIndex = 2;

    const html = renderBattleResult(state);
    expect(html).toContain('class="panel battle-view insight-split module-trajectory"');
    expect(html).toContain('class="battle-module engine-insight"');
    expect(html).toContain('class="insight-companion"');
    expect(html.match(/aria-label="生命曲线"/gu)).toHaveLength(1);
    // 右侧日志按步进轮播：一次步进 = 一个 actorTurn 的全部钩子，整场不再铺开。
    expect(html).not.toContain('class="engine-turn"');
    expect(html).toContain('class="engine-step-list"');
    // 标题只标回合；行动方在左侧时间轴圆点 tooltip，打出的牌在下方钩子链里高亮，头部不重复。
    expect(html).toContain("<b>第 1 回合</b>");
    expect(html).not.toContain("未出牌");
    expect(html).toContain("2 个钩子 · 2 处改动");
    expect(html).not.toContain("<em>本回合</em>");
    // 一次步进里回合开始与牌面结算的钩子都在，没有只留末帧。
    expect(html).toContain(">回合开始<");
    expect(html).toContain(">牌面结算<");
    // 用「一/二」跟时间轴的点保持同一套记号，完整名字留在 title 里。
    // 改动按行动方分组：侧标签「一/二」在 .hook-change-side-tag，字段名在 subject。
    expect(html).toContain('class="hook-change-side actor" data-side="p1"');
    expect(html).toContain('class="hook-change-side-tag" data-audit-ignore="repeated-log-field">一</span>');
    expect(html).toContain('class="hook-change-subject" data-audit-ignore="repeated-log-field">灵气</span>');
    expect(html).toContain('data-audit-ignore="repeated-log-value">0→2</b>');
    expect(html).toContain('class="hook-change-side" data-side="p2"');
    expect(html).toContain('class="hook-change-side-tag" data-audit-ignore="repeated-log-field">二</span>');
    expect(html).toContain('class="hook-change-subject" data-audit-ignore="repeated-log-field">生命</span>');
    expect(html).toContain('data-audit-ignore="repeated-log-value">50→43</b>');
    expect(html).toContain('data-frame="2"');
    expect(html).toContain('data-hook="mainEffect"');
    expect(html).not.toContain("当前帧状态");
    // 引擎透视不再复述整份状态快照。
    expect(html).not.toContain("双方逐回合状态快照");
    // 引擎透视曾被一条"临时隐藏动作日志"的类名规则一并藏掉，那块面板因此长期是空的。
    const battleLogCss = readFileSync(resolve(import.meta.dir, "../styles/battle-log.css"), "utf8");
    expect(battleLogCss).not.toContain("display: none");
    expect(battleLogCss).not.toMatch(/^\.battle-progress-panel/mu);
    const battleLayoutCss = readFileSync(
      resolve(import.meta.dir, "../styles/battle-layout.css"),
      "utf8",
    );
    expect(battleLayoutCss).toMatch(
      /@container \(min-width: 900px\)[\s\S]*\.battle-view\.insight-split[\s\S]*grid-template-columns:/,
    );
    expect(battleLayoutCss).toMatch(
      /\.battle-view\.insight-split > \.battle-module-body[\s\S]*grid-column:\s*2;[\s\S]*grid-row:\s*1 \/ -1/,
    );
    expect(battleLayoutCss).toMatch(
      /\.battle-view\.insight-split > \.insight-companion[\s\S]*grid-column:\s*1;[\s\S]*grid-row:\s*4/,
    );
    const inspectorCss = readFileSync(
      resolve(import.meta.dir, "../styles/battle-inspector.css"),
      "utf8",
    );
    expect(inspectorCss).toMatch(
      /\.engine-step-list\s*\{[\s\S]*overflow-x:\s*hidden;[\s\S]*overflow-y:\s*auto/,
    );
    expect(inspectorCss).toMatch(
      /\.engine-insight-current b\s*\{[\s\S]*white-space:\s*nowrap/,
    );
    expect(inspectorCss).toMatch(
      /\.engine-insight-counts\s*\{[\s\S]*text-overflow:\s*ellipsis;[\s\S]*white-space:\s*nowrap/,
    );
    expect(inspectorCss).toMatch(
      /\.engine-insight-head > strong\s*\{[\s\S]*white-space:\s*nowrap/,
    );
    expect(inspectorCss).not.toMatch(/\.engine-turn\s*\{/);
  });

  test("同名同等级连续出牌按出牌事件分组，不误合并牌名", () => {
    const state = baseState();
    state.result = {
      ...simulationResult([
        battleFrame([], { index: 0, actionIndex: null, title: "战斗开始结算" }),
        battleFrame([], { index: 1, actionIndex: 1, title: "第 1 动 · 鹤步" }),
        battleFrame([], { index: 2, actionIndex: 2, title: "第 2 动 · 鹤步" }),
      ], 2),
      hookSteps: adaptHookTrace({
        steps: [
          {
            eventIndex: 1,
            category: "mainEffect",
            turn: 1,
            actor: "p1",
            slot: 2,
            cardId: 100,
            cardName: "鹤步",
            p1Changes: [{ group: "核心", key: "anima", label: "灵气", before: 0, after: 1 }],
            p2Changes: [],
          },
          {
            eventIndex: 1,
            category: "afterCard",
            turn: 1,
            actor: "p1",
            slot: 2,
            cardId: 100,
            cardName: "鹤步",
            p1Changes: [],
            p2Changes: [],
          },
          {
            eventIndex: 2,
            category: "mainEffect",
            turn: 1,
            actor: "p1",
            slot: 5,
            cardId: 100,
            cardName: "鹤步",
            p1Changes: [{ group: "核心", key: "anima", label: "灵气", before: 1, after: 2 }],
            p2Changes: [],
          },
          {
            eventIndex: 2,
            category: "afterCard",
            turn: 1,
            actor: "p1",
            slot: 5,
            cardId: 100,
            cardName: "鹤步",
            p1Changes: [],
            p2Changes: [],
          },
        ],
      }, 3),
    };
    state.frameIndex = 2;

    const html = renderBattleResult(state);
    // 两张鹤步按出牌事件（frameIndex）分成两组，组头各带卡槽号区分。
    expect(html.match(/class="hook-card-group"/gu)).toHaveLength(2);
    expect(html.match(/hook-card-group-name[^>]*>鹤步/g)).toHaveLength(2);
    expect(html).toContain(">鹤步<span class=\"hook-card-group-slot\"> · 第 3 格</span>");
    expect(html).toContain(">鹤步<span class=\"hook-card-group-slot\"> · 第 6 格</span>");
    // 第一张鹤步的 mainEffect 钩子不混入第二张的结算。
    const firstGroup = html.slice(html.indexOf("hook-card-group"), html.indexOf("第 6 格"));
    expect(firstGroup).toContain('data-frame="1"');
    expect(firstGroup).not.toContain('data-frame="2"');
  });

  test("钩子链取不到时说明为空，而不是假装引擎没走钩子", () => {
    const state = baseState();
    state.result = simulationResult([
      battleFrame([], { index: 0, actionIndex: null, title: "战斗开始结算" }),
      battleFrame([], { index: 1, actionIndex: 1, title: "第 1 动 · 万玄破魔掌" }),
    ], 1);
    state.frameIndex = 1;

    const html = renderBattleResult(state);
    expect(html).toContain("这一步没有取到钩子");
    expect(html).toContain("0 个钩子 · 0 处改动");
  });

  test("时间轴与方向键步进独立于模块选择", () => {
    const state = baseState();
    state.view = "battle";
    state.result = simulationResult([battleFrame([])], 9);
    state.fixtureConsistency = {
      engine: { winnerSide: "p1", actorTurnCount: 1, hpDeltaP1MinusP2: 1, finalHp: { p1: 2, p2: 1 } },
      ui: { winnerSide: "p1", actorTurnCount: 1, hpDeltaP1MinusP2: 1, finalHp: { p1: 2, p2: 1 } },
      engineMatch: true,
      expectedMatch: true,
    };
    const html = renderBattleResult(state);
    expect(html).toContain('class="battle-progress-rail"');
    expect(html).toContain('data-action="jump-frame"');
    expect(html).toContain("内容：按步进轮播，一次步进展示该回合（actorTurn）的全部钩子与字段变化。");
    expect(html).toContain("导航：方向键按初始态、相邻回合结束依次移动");
    expect(html).toContain("范围：没有改动的钩子也列出，看的是引擎实际走了哪些钩子。");
    expect(html).toContain("内容：展示双方生命、生命差与整场变化轨迹");
    expect(html).toContain("任务：场上牌排序、对局手牌重组或卡池构筑");
    expect(html).toContain("编号：候选数字可追溯到场上牌、手牌或卡池来源");
    // 结论层排在轨道之前。一致性徽章是结论可信度的断言，归结论头，不再独自占一行。
    expect(html).toMatch(
      /class="battle-verdict"[\s\S]*class="verdict-winner[\s\S]*class="fixture-consistency[\s\S]*class="battle-progress-rail"/,
    );
    expect(html).toContain('class="fixture-consistency ok"');
    expect(html).not.toContain('class="battle-now"');
    expect(html).not.toContain('class="winner"');
    expect(html).not.toContain('class="battle-section-title"');
    expect(html).not.toContain('class="battle-nav"');
    expect(html).not.toContain('class="battle-now-meta"');
    expect(html).not.toContain('class="battle-round-mark"');
    expect(html).not.toContain('class="battle-card-name"');
    // 明细层级：逐动明细默认展开，重型曲线与遥测默认收起。
    // 时间轴不在任何模块里，所以轨道必须排在模块选项卡之前。
    expect(html.indexOf('class="battle-progress-rail"'))
      .toBeLessThan(html.indexOf('class="battle-module-tabs"'));
    expect(html).not.toContain('class="battle-action-total"');
    expect(html).not.toMatch(/>上一动</);
    expect(html).not.toContain('class="battle-now-pos"');
    expect(html).not.toContain('class="battle-now-title"');

    // 没有 fixture 一致性报告时，徽章不渲染；结论层与轨道之间不再留出空行。
    const noBadgeState = baseState();
    noBadgeState.view = "battle";
    noBadgeState.result = simulationResult([battleFrame([])], 9);
    const noBadgeHtml = renderBattleResult(noBadgeState);
    expect(noBadgeHtml).not.toContain('class="fixture-consistency');
    expect(noBadgeHtml).not.toContain('class="battle-now"');
    expect(noBadgeHtml).toContain('class="battle-progress-rail"');

    expect(battleStepShortcutDirection("ArrowLeft", state, false)).toBe(-1);
    expect(battleStepShortcutDirection("ArrowRight", state, false)).toBe(1);
    expect(battleStepShortcutDirection("ArrowUp", state, false)).toBe(-1);
    expect(battleStepShortcutDirection("ArrowDown", state, false)).toBe(1);
    expect(battleStepShortcutDirection("ArrowRight", state, true)).toBeNull();
  });

  test("方向键与箭头按钮按初始态、相邻回合结束依次步进", () => {
    const result = simulationResult([
      battleFrame([], {
        index: 0,
        actorTurn: 0,
        actionIndex: null,
        actorId: null,
        cardId: null,
        cardName: null,
        title: "战斗开始结算",
      }),
      battleFrame([], {
        index: 1,
        actorTurn: 1,
        actionIndex: 1,
        title: "第 1 动 · 测试牌一",
      }),
      battleFrame([], {
        index: 2,
        actorTurn: 1,
        actionIndex: null,
        cardId: null,
        cardName: null,
        title: "第 1 回合结束结算",
      }),
      battleFrame([], {
        index: 3,
        actorTurn: 2,
        actionIndex: 2,
        actorId: "p2",
        title: "第 2 动 · 测试牌二",
      }),
      battleFrame([], {
        index: 4,
        actorTurn: 2,
        actionIndex: null,
        actorId: "p2",
        cardId: null,
        cardName: null,
        title: "第 1 回合结束结算",
      }),
      // 致死回合可能没有 turnEnd；仍把该 actorTurn 的最后实际帧作为终点。
      battleFrame([], {
        index: 5,
        actorTurn: 3,
        actionIndex: 3,
        title: "第 3 动 · 致死牌",
      }),
    ], 3);

    // 完整行动锚点不含开局点：战斗完成后停在第一动结束帧，开局结算用 ← 回看。
    expect(completedTurnFrameIndexes(result)).toEqual([2, 4, 5]);
    expect(firstCompletedTurnFrameIndex(result)).toBe(2);
    // 步进锚点含开局点（战斗开始结算）：从战前初始状态前进先到开局结算，
    // 左侧状态条的开局效果闪烁因此落在初始结算帧而不是第一动。
    expect(allStepFrameIndexes(result)).toEqual([0, 2, 4, 5]);
    expect(adjacentCompletedTurnFrameIndex(result, 0, 1)).toBe(2);
    expect(adjacentCompletedTurnFrameIndex(result, 2, -1)).toBe(0);
    expect(adjacentCompletedTurnFrameIndex(result, 2, 1)).toBe(4);
    expect(adjacentCompletedTurnFrameIndex(result, 4, -1)).toBe(2);
    expect(adjacentCompletedTurnFrameIndex(result, 1, -1)).toBe(0);
    expect(adjacentCompletedTurnFrameIndex(result, 1, 1)).toBe(2);
    expect(adjacentCompletedTurnFrameIndex(result, 5, 1)).toBe(5);

    const state = baseState();
    state.view = "battle";
    state.result = result;
    state.frameIndex = 2;
    const html = renderBattleResult(state);
    // 方向键步进锚点仍是“一方完整行动的末帧”，轨道点跳转也走同一份锚点。
    expect(html).toContain('aria-label="动作进度"');
    expect(html).toContain('data-action="jump-frame"');
    expect(html).not.toContain("multi-card");
  });

  test("回合结束导航高亮该行动方本次行动实际打出的全部牌", () => {
    const state = baseState();
    state.view = "battle";
    state.result = simulationResult([
      battleFrame([], {
        index: 0,
        actorTurn: 0,
        actionIndex: null,
        actorId: null,
        sourceSlot: null,
        cardId: null,
        cardName: null,
        title: "战斗开始结算",
      }),
      battleFrame([], {
        index: 1,
        actorTurn: 1,
        actionIndex: 1,
        sourceSlot: 2,
        title: "第 1 动 · 测试牌一",
      }),
      battleFrame([], {
        index: 2,
        actorTurn: 1,
        actionIndex: 2,
        sourceSlot: 5,
        title: "第 2 动 · 再次行动",
      }),
      battleFrame([], {
        index: 3,
        actorTurn: 1,
        actionIndex: null,
        sourceSlot: null,
        cardId: null,
        cardName: null,
        title: "第 1 回合结束结算",
      }),
    ], 2);
    state.frameIndex = 3;

    expect(playedSlotsForCurrentTurn(state, "p1")).toEqual([2, 5]);
    expect(playedSlotsForCurrentTurn(state, "p2")).toEqual([]);
    const html = renderPlayerPanel(state, "p1");
    expect(html).toMatch(/class="deck-slot[^"]*active[^"]*"\s+data-slot="2"/);
    expect(html).toMatch(/class="deck-slot[^"]*active[^"]*"\s+data-slot="5"/);

    state.result = simulationResult([
      battleFrame([], {
        index: 0,
        actorTurn: 0,
        actionIndex: null,
        actorId: null,
        sourceSlot: null,
        cardId: null,
        cardName: null,
        title: "战斗开始结算",
      }),
      battleFrame([], {
        index: 1,
        actorTurn: 1,
        actionIndex: 1,
        actorId: "p1",
        sourceSlot: 0,
        title: "第 1 动 · 第一张牌",
      }),
      battleFrame([], {
        index: 2,
        actorTurn: 1,
        actionIndex: null,
        actorId: "p1",
        sourceSlot: null,
        cardId: null,
        cardName: null,
        title: "第 1 回合结束结算",
      }),
    ], 1);
    state.frameIndex = 2;
    expect(playedSlotsForCurrentTurn(state, "p1")).toEqual([0]);
    expect(renderPlayerPanel(state, "p1"))
      .toMatch(/class="deck-slot[^"]*active[^"]*"\s+data-slot="0"/);
    expect(renderPlayerPanel(state, "p2")).not.toContain("deck-slot active");
  });
});

describe("顶层工作台模式切换", () => {
  test("打靶模式渲染独立左列与右列结果面板，双方对战保持原样", () => {
    const target = baseState();
    target.workbenchMode = "target";
    target.target = {
      builds: [],
      activeBuildId: "",
      damageThreshold: 120,
      displayRounds: 1,
      displayRoundMin: 1,
      displayRoundPending: false,
      compareMode: "overlay",
      expandedStep: null,
      expandedStepBuildId: null,
      duelP1Player: null,
    };
    const targetHtml = renderApp(target);
    expect(targetHtml).toContain("打靶模式");
    expect(targetHtml).toContain("data-action=\"switch-workbench-mode\"");
    expect(targetHtml).toContain("target-setup-pane");
    expect(targetHtml).toContain("+ 新增构筑");
    expect(targetHtml).toContain("target-result-panel");
    // 显示至回合滑条取代旧的「额外回合」数字输入。
    expect(targetHtml).toContain('id="battle-targetDisplayRounds"');
    expect(targetHtml).toContain('type="range"');
    expect(targetHtml).not.toContain("battle-targetExtraRounds");
    expect(targetHtml).not.toContain("额外回合");
    // 打靶模式不渲染双方对战专属面板。
    expect(targetHtml).not.toContain("free-build-panel");
    expect(targetHtml).not.toContain("fixture-import-panel");

    const duel = baseState();
    const duelHtml = renderApp(duel);
    expect(duelHtml).toContain("free-build-panel");
    expect(duelHtml).not.toContain("target-setup-pane");
    expect(duelHtml).not.toContain("target-result-panel");
  });
});
