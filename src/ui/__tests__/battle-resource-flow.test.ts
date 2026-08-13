import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "bun:test";
import { CoreBuff } from "../domain";
import { openingResourceDeltas } from "../battle-opening";
import { defaultPlayerConfig } from "../data";
import { renderBattleResult } from "../render-battle";
import { renderResourceFlow } from "../render-battle-flow";
import { renderFixtureConsistencyBadge } from "../render-fixture-import";
import { renderPlayerPanel } from "../render-player-panel";
import { baseState, battleFrame, playerView, simulationResult } from "./layout-test-helpers";

describe("UI 战斗视图资源走势与玩家面板", () => {
  test("战斗面板展示资源走势与 fixture 一致性徽标", () => {
    const state = baseState();
    state.result = simulationResult([
      battleFrame([], {
        index: 1,
        actionIndex: 1,
        players: {
          p1: playerView({ hp: 88, anima: 1, defense: 2 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 92 }),
        },
      }),
      battleFrame([], {
        index: 2,
        actionIndex: 2,
        cardName: "普通攻击",
        players: {
          p1: playerView({ hp: 88, anima: 2, defense: 2, buffs: { [CoreBuff.Hexagram]: 1, [CoreBuff.AttackBonus]: 4 } }),
          p2: playerView({
            id: "p2",
            name: "李燚",
            side: "p2",
            hp: 80,
            buffs: { [CoreBuff.InternalInjury]: 3 },
          }),
        },
      }),
    ], 2);
    state.frameIndex = 2;
    state.importedFixtureOrigin = "catalog";
    state.fixtureConsistency = {
      engine: {
        winnerSide: "p1",
        actorTurnCount: 2,
        hpDeltaP1MinusP2: 8,
        finalHp: { p1: 88, p2: 80 },
      },
      ui: {
        winnerSide: "p1",
        actorTurnCount: 2,
        hpDeltaP1MinusP2: 8,
        finalHp: { p1: 88, p2: 80 },
      },
      engineMatch: true,
      expectedMatch: true,
    };

    state.battleModule = "trajectory";
    const html = renderBattleResult(state);
    expect(html).toContain('class="battle-module resource-flow"');
    expect(html).toContain("生命曲线");
    // 曲线选项卡是 生命/伤害 分段开关（toggle 上移后模块内部不再重复）。
    expect(html).toContain('class="trajectory-switch active"');
    expect(html).toMatch(/data-metric="life"[\s\S]{0,80}aria-pressed="true"/);
    expect(html).toMatch(/data-metric="damage"[\s\S]{0,80}aria-pressed="false"/);
    // 可见标签短到不被裁切，完整口径留在 title 里，由 audit:ui 的 text-clipped 规则守着。
    expect(html).toContain(">玩家一生命<");
    expect(html).toContain(">生命差<");
    expect(html).toContain("Rust canonical 帧中的玩家一生命原值");
    expect(html).toContain("正值表示玩家一领先");
    expect(html).not.toContain("P1 2/0/0/0/1/0/4");
    expect(html).toContain('class="fixture-consistency ok"');
    expect(html).toContain("expected exact");
    expect(html).toContain("expected 来源：准入 catalog fixture");

    expect(html).toContain('style="--progress-action-count:1"');

    const battleLayoutCss = readFileSync(resolve(import.meta.dir, "../styles/battle-layout.css"), "utf8");
    expect(battleLayoutCss).toMatch(/\.battle-progress-track\s*\{[\s\S]*overflow-x:\s*auto/);
    expect(battleLayoutCss).toMatch(/\.progress-round-labels,[\s\S]*\.progress-dots\s*\{[\s\S]*--progress-action-count/);
    expect(battleLayoutCss).toMatch(/grid-template-columns:\s*repeat\(var\(--progress-action-count/);
    expect(html).toContain('data-round="1"');
    expect(battleLayoutCss).toMatch(/\.battle-progress-dot\s*\{[\s\S]*min-width:\s*16px[\s\S]*height:\s*24px/);
  });

  test("本地 JSON 的 expected 只表示用户输入匹配，不冒充准入证据", () => {
    const state = baseState();
    state.importedFixtureOrigin = "local";
    const summary = {
      winnerSide: "p1" as const,
      actorTurnCount: 2,
      hpDeltaP1MinusP2: 8,
      finalHp: { p1: 88, p2: 80 },
    };
    state.fixtureConsistency = {
      engine: summary,
      ui: summary,
      engineMatch: true,
      expectedMatch: true,
    };

    const matched = renderFixtureConsistencyBadge(state);
    expect(matched).toContain("输入 expected 匹配 · 未认证");
    expect(matched).toContain("Engine-vs-UI exact");
    expect(matched).toContain("expected 来源：用户本地输入；未经过原作回放准入认证");
    expect(matched).not.toContain("expected exact");
    expect(matched).not.toContain("准入 catalog fixture");

    state.fixtureConsistency = { ...state.fixtureConsistency, expectedMatch: false };
    const mismatched = renderFixtureConsistencyBadge(state);
    expect(mismatched).toContain("输入 expected 不匹配 · 未认证");
    expect(mismatched).toContain('class="fixture-consistency bad"');
    expect(mismatched).not.toContain("expected mismatch");
  });

  test("开局资源只由原始初值和开局效果产生，不保留角色护体硬编码", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    expect(state.config.players.p1.guard).toBe(0);
    expect(state.config.players.p2.guard).toBe(0);
    const html = renderPlayerPanel(state, "p1");
    expect(html).not.toContain('class="setup-initial-row"');
    expect(html).not.toContain('class="setup-opening-stats"');
    expect(html).not.toContain("自动派生");
    expect(html).toContain('data-buff="physique"');
    expect(html).not.toContain('id="player-p1-defense"');
    expect(html).not.toContain('id="player-p1-anima"');
    expect(html).not.toContain('id="player-p1-guard"');
  });

  test("原版 fixture 渲染不规范化或覆盖原始 talent", () => {
    const state = baseState();
    state.config.sourceKind = "original-fixture";
    state.config.players.p1.talents = [134, 98, 99, 90, 30053];
    renderPlayerPanel(state, "p1");
    expect(state.config.players.p1.talents[0]).toBe(134);
  });

  test("资源走势包含初始状态与战斗开始结算，并可回退到开局帧", () => {
    const initial = battleFrame([], {
      index: 0,
      actionIndex: null,
      actorId: null,
      actorTurn: 0,
      title: "初始状态",
      players: {
        p1: playerView({ defense: 0, anima: 0, guard: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", defense: 0, anima: 0, guard: 0 }),
      },
    });
    const opening = battleFrame([
      { sequence: 1, type: "resource", targetId: "p1", name: "anima", before: 0, after: 1, detail: { source: "talent:13" } },
      { sequence: 2, type: "resource", targetId: "p1", name: "defense", before: 0, after: 4, detail: { source: "fateStrategy:143" } },
    ], {
      index: 1,
      actionIndex: null,
      actorId: "p1",
      actorTurn: 0,
      title: "战斗开始结算",
      players: {
        p1: playerView({ defense: 4, anima: 1, guard: 0 }),
        p2: playerView({ id: "p2", name: "李燚", side: "p2", defense: 0, anima: 0, guard: 0 }),
      },
    });
    const action = battleFrame([], {
      index: 2,
      actionIndex: 1,
      players: {
        p1: playerView({ defense: 4, anima: 1 }),
        p2: playerView({ id: "p2", name: "李燚", defense: 0, anima: 0 }),
      },
    });
    expect(openingResourceDeltas(initial, opening, "p1")).toEqual([
      { key: "defense", label: "防", value: 4 },
      { key: "anima", label: "灵气", value: 1 },
    ]);
    const html = renderResourceFlow([initial, opening, action], 1);
    // 开局帧折进 actorTurn 0 那个点，仍然可以回退到它；副标题已移除，改用点存在性验证。
    expect(html).toContain('class="flow-metric hp-p1"');
    expect(html).toContain('class="flow-metric hp"');
    // 资源差/防护差/负面差三条派生曲线已删除：等权相加既不是原作数值也不是 value 权重。
    expect(html).not.toContain("资源差");
    expect(html).not.toContain("防护差");
    expect(html).not.toContain("负面差");
  });

  test("资源走势只在行动帧存在时渲染，并保留完成态阶段帧", () => {
    expect(renderResourceFlow([battleFrame([], { actionIndex: null })], 1)).toBe("");

    const frames = [
      battleFrame([], {
        index: 1,
        actionIndex: 1,
        players: {
          p1: playerView({ hp: 90 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90 }),
        },
      }),
      battleFrame([], {
        index: 2,
        actionIndex: null,
        title: "中间事件",
        players: {
          p1: playerView({ hp: 90 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90 }),
        },
      }),
      battleFrame([], {
        index: 3,
        actionIndex: 2,
        cardName: "普通攻击",
        players: {
          p1: playerView({ hp: 90, buffs: { [CoreBuff.AttackBonus]: 3 } }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 84 }),
        },
      }),
    ];

    const html = renderResourceFlow(frames, 2);
    expect(html).toContain('class="battle-module resource-flow"');
    expect(html).toContain('viewBox="0 0 100 56"');
    expect(html).toContain('class="flow-area"');
    expect(html).toContain("玩家一生命");
    expect(html).toContain("生命差");
    expect(html).toContain('x1="0"');
    expect(html).not.toContain("防护差");
  });

  test("战斗中生命显示在属性行最右端", () => {
    const state = baseState();
    state.result = simulationResult([battleFrame([])], 1);
    const html = renderPlayerPanel(state, "p1");
    expect(html).toContain("live-hp-end");
    expect(html).toContain('class="player-slot-status"');
    expect(html).toContain('class="live-hp-fill"');
    expect(html).toContain('data-hp-delta="0"');
    expect(html).toContain('data-hp-delta="0"');
    expect(html).not.toContain(">Δ0<");
    expect(html).toContain("--hp-pct: 50%");
    expect(html).toMatch(/50[\s\S]*100/);
    expect(html).not.toContain("<em>战</em>");
    expect(html).not.toContain('class="player-battle-hp-row"');
    expect(html).not.toContain('class="acting"');
  });

  test("生命胶囊变化量比较相邻导航点，不被无变化的结束帧归零", () => {
    const state = baseState();
    state.result = simulationResult([
      battleFrame([], {
        index: 0,
        actorTurn: 0,
        actionIndex: null,
        actorId: null,
        players: {
          p1: playerView({ hp: 100 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 100 }),
        },
      }),
      battleFrame([], {
        index: 1,
        actorTurn: 1,
        actionIndex: 1,
        actorId: "p1",
        sourceSlot: 0,
        players: {
          p1: playerView({ hp: 100 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90 }),
        },
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
        players: {
          p1: playerView({ hp: 100 }),
          p2: playerView({ id: "p2", name: "李燚", side: "p2", hp: 90 }),
        },
      }),
    ], 1);
    state.frameIndex = 2;

    const p1 = renderPlayerPanel(state, "p1");
    const p2 = renderPlayerPanel(state, "p2");
    expect(p1).toContain('data-hp-delta="0"');
    expect(p2).toContain('data-hp-delta="-10"');
    expect(p2).toContain("相对上个导航点的生命变化");
  });

  test("combined 玩家区纵向堆叠以保留卡槽宽度，且开战前后不重排", () => {
    const setupLayoutCss = readFileSync(resolve(import.meta.dir, "../styles/setup-layout.css"), "utf8");
    expect(setupLayoutCss).toMatch(
      /\.combined-page \.players-grid\s*\{[\s\S]*grid-template-columns:\s*1fr/,
    );
    expect(setupLayoutCss).toMatch(
      /\.combined-page \.players-grid\s*\{[\s\S]*grid-template-rows:\s*repeat\(2,\s*minmax\(0,\s*1fr\)\)/,
    );
    // setup-empty must not reflow players-grid to side-by-side — same board before/after battle.
    expect(setupLayoutCss).not.toMatch(/\.combined-page\.setup-empty \.players-grid/);
    expect(setupLayoutCss).toMatch(
      /\.combined-page \.setup-pane\s*\{[\s\S]*overflow:\s*hidden/,
    );
    expect(setupLayoutCss).not.toMatch(/\.combined-page \.setup-pane\s*\{[\s\S]*overflow-y:\s*auto/);

    const baseCss = readFileSync(resolve(import.meta.dir, "../styles/base.css"), "utf8");
    expect(baseCss).not.toMatch(/\.combined-page\.setup-empty\s*\{[\s\S]*grid-template-columns/);
    expect(baseCss).not.toMatch(/\.combined-page\.setup-empty \.combined-battle\s*\{[\s\S]*display:\s*none/);

    const deckCss = readFileSync(resolve(import.meta.dir, "../styles/player-panel-deck.css"), "utf8");
    expect(deckCss).toMatch(
      /\.slot-level-cycle\s*\{[\s\S]*width:\s*100%/,
    );
    expect(deckCss).toMatch(/\.combined-page \.slot-level-cycle\s*\{[\s\S]*height:\s*18px/);
  });
});
