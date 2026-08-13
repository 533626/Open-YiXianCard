import { readFileSync } from "node:fs";
import { resolve } from "node:path";
import { describe, expect, test } from "bun:test";
import {
  fixtureEntryById,
  filterFixtureEntries,
  type UiFixtureEntry,
} from "../fixture-catalog";
import { handleAction } from "../main-actions";
import { renderApp } from "../render";
import { renderFixtureImportPanel } from "../render-fixture-import";
import { renderCardPopup } from "../render-pickers";
import { defaultPlayerConfig } from "../data";
import { renderPlayerPanel } from "../render-player-panel";
import { renderSolverPanel } from "../render-solver";
import {
  baseState,
  solverEvaluation,
  solverValueMetrics,
} from "./layout-test-helpers";

const publicCatalogEntry: UiFixtureEntry = {
  id: "public-contract/round-01",
  path: "fixtures/candidates/public-contract/round-01.json",
  matchId: "public-contract",
  round: 1,
  steamBuild: "test-build",
  expectedWinner: "p1",
  p1CharacterId: 4_000_004,
  p2CharacterId: 4_000_005,
};

describe("UI 手输卡组布局契约", () => {
  test("fixture 内部回放字段不占用主界面", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    state.config.players.p2 = defaultPlayerConfig("p2", 4_000_005, state.config.gameRound);
    const activeHtml = renderPlayerPanel(state, "p1");
    expect(activeHtml).not.toContain('class="replay-field-panel"');
    expect(activeHtml).not.toContain("回放字段");
    expect(activeHtml).not.toContain("上轮用牌");
    expect(activeHtml).not.toContain("apply-character-talents");
    expect(activeHtml).not.toContain(">默认<");
    expect(activeHtml).not.toContain("card-face-runtime");
    expect(activeHtml).not.toContain("<details");
    expect(activeHtml).not.toContain("<summary");
    const inactiveHtml = renderPlayerPanel(state, "p2");
    expect(inactiveHtml).not.toContain('class="replay-field-panel"');
    expect(inactiveHtml).not.toContain('class="deck-toolbar"');
    expect(inactiveHtml).toContain('class="player-panel-setup"');
    expect(inactiveHtml).not.toContain('class="setup-initial-label"');
    expect(inactiveHtml).toContain('class="player-hp-setup"');
    expect(inactiveHtml).not.toContain('class="player-live-hp"');
    expect(inactiveHtml).toContain('class="talent-row"');
  });

  test("每位玩家渲染 8 个横向卡槽按钮", () => {
    const html = renderPlayerPanel(baseState(), "p1");
    const deckButtons = html.match(/<button\s+class="card-face/g) ?? [];
    expect(deckButtons.length).toBe(8);
    expect(html).toContain('class="player-deck"');
    expect(html).not.toContain('class="deck-toolbar"');
    expect(html).not.toContain('class="card-face-level"');
  });

  test("玩家面板保留侧标识供点击切换", () => {
    const html = renderPlayerPanel(baseState(), "p2");
    expect(html).toContain('class="player-panel');
    expect(html).toContain('data-side="p2"');
    expect(html).not.toContain('data-action="select-side"');
  });

  test("战前生命上限只读显示", () => {
    const html = renderPlayerPanel(baseState(), "p1");
    expect(html).toContain("hp-field-readonly");
    expect(html).toContain('class="hp-readonly-value"');
    expect(html).not.toMatch(/id="player-p1-hp"/);
    expect(html).not.toMatch(/id="player-p1-maxHp"/);
    expect(html).toMatch(/id="player-p1-lifeModifier"/);
    expect(html).toMatch(/id="player-p1-level"/);
  });

  test("姬方生初始仙命档位在固定仙命格内调整", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    const html = renderPlayerPanel(state, "p1");
    expect(html).toContain('class="locked-talent-rank"');
    expect(html).toContain('data-action="adjust-jifangsheng-rank"');
    expect(html).not.toContain("初始仙命");
  });

  test("求解区直接显示三类任务与启发式/穷举选择，不再套折叠层", () => {
    const html = renderSolverPanel(baseState());
    expect(html).not.toContain('data-action="toggle-solver"');
    expect(html).not.toContain('data-action="set-solver-mode"');
    expect(html).toContain('data-action="set-solver-task"');
    expect(html).toContain('data-action="set-solver-method"');
    expect(html).toContain('data-action="solve-active"');
    expect(html).toContain('aria-label="求解建议"');
    expect(html).toContain("场上牌排序");
    expect(html).toContain("对局手牌");
    expect(html).toContain("卡池求解");
    expect(html).toContain("启发式");
    expect(html).toContain("穷举");
    expect(html).toContain("无需展开或切换面板");
    expect(html.match(/data-action="solve-/g)?.length).toBe(1);
  });

  test("卡池求解的穷举选项置灰且只显示启发式说明", () => {
    const state = baseState();
    state.solverMode = "poolBeam";
    const html = renderSolverPanel(state);
    expect(html).toContain('class="unavailable"');
    expect(html).toMatch(
      /data-solver-method="exhaustive"[\s\S]*aria-disabled="true" disabled/,
    );
    expect(html).toContain("卡池组合空间过大，仅支持启发式搜索");
    expect(html).toContain("仅启发式：每种一阶牌 1 张");
  });

  test("手牌求解直接列出导入对局的当前手牌", () => {
    const state = baseState();
    state.solverMode = "handBeam";
    state.config.players.p1.handCardIds = [3, 3, 4];
    state.solverResult = {
      mode: "hand",
      side: "p1",
      confidence: "heuristic",
      evaluatedCount: 1,
      skippedDuplicateCount: 0,
      candidateCardCount: 4,
      baseline: solverEvaluation(0),
      baselineDeck: [
        { id: 1, baseId: 1, name: "场上一" },
        { id: 2, baseId: 2, name: "场上二" },
      ],
      results: [{
        rank: 1,
        confidence: "heuristic",
        deck: [
          { id: 1, baseId: 1, name: "场上一" },
          { id: 3, baseId: 3, name: "手牌一" },
        ],
        leftoverHandCardIds: [3, 4],
        evaluation: solverEvaluation(1),
        changedSlots: [{
          slot: 2,
          from: { id: 2, baseId: 2, name: "场上二" },
          to: { id: 3, baseId: 3, name: "手牌一" },
        }],
        deckKey: "hand-test",
      }],
    };
    const html = renderSolverPanel(state);
    expect(html).toContain('class="solver-hand-input"');
    expect(html).toContain(">当前手牌<");
    expect(html).toContain("手牌编号");
    expect(html).toContain("编号：3、4");
    expect(html).toContain("<strong>×2</strong>");
    expect(html).toContain("来源：导入对局记录里的当前手牌。");
    expect(html).toContain("候选牌序出现上述编号时");
    expect(html).toContain(">1 4</code>");
    expect(html).toContain("编号：基准 1–2 · 手牌 3–5 · 手N=换入数");
    expect(html).toContain('class="solver-row-change">手1</span>');
    expect(html).toContain("已换入手牌：");
    expect(html).toContain("4 =");
    expect(html).toContain("未换入手牌：");
  });

  test("手牌编号在求解前后都计入回放的普通攻击补位", () => {
    const state = baseState();
    state.solverMode = "handBeam";
    state.config.players.p1.activeSlotCount = 7;
    state.config.players.p1.deck = [
      ...Array.from({ length: 7 }, (_, index) => ({
        baseId: index + 1,
        level: 1,
      })),
      {
        baseId: 0,
        level: 1,
        originalConfig: { id: 0, baseId: 0, name: "普通攻击" },
      },
    ];
    state.config.players.p1.handCardIds = [3, 4];

    const html = renderSolverPanel(state);
    expect(html).toContain("<em>9</em>");
    expect(html).toContain("<em>10</em>");
    expect(html).toContain("基准包含回放里的“普通攻击”补位牌");
  });

  test("求解结果直接跟在操作区下方且不提供折叠开关", () => {
    const state = baseState();
    state.solverMode = "orderBeam";
    state.solverStatus = {
      mode: "orderBeam",
      state: "done",
      elapsedMs: 42,
      evaluatedCount: 1,
      maxEvaluations: 2_000,
    };
    state.solverResult = {
      mode: "order",
      side: "p1",
      confidence: "exact",
      evaluatedCount: 1,
      skippedDuplicateCount: 0,
      candidateCardCount: 8,
      seedsUsed: [1, 2, 3],
      syntheticDecisionSeedsUsed: [1, 2, 3],
      usedSyntheticDecisions: true,
      baseline: solverEvaluation(0),
      baselineDeck: [
        { id: 1, baseId: 1, name: "测试牌" },
        { id: 2, baseId: 2, name: "旧牌" },
      ],
      results: [{
        rank: 1,
        confidence: "exact",
        deck: [
          { id: 2, baseId: 2, name: "旧牌" },
          { id: 1, baseId: 1, name: "测试牌" },
        ],
        leftoverHandCardIds: [],
        evaluation: solverEvaluation(1),
        changedSlots: [{
          slot: 1,
          from: { id: 2, baseId: 2, name: "旧牌" },
          to: { id: 1, baseId: 1, name: "测试牌" },
        }],
        deckKey: "test",
      }],
    };
    const resultHtml = renderSolverPanel(state);
    expect(resultHtml).not.toContain('data-action="toggle-solver"');
    expect(resultHtml).toContain('class="solver-badge"');
    expect(resultHtml).toContain('data-action="apply-solver-row"');
    expect(resultHtml).toContain('data-deck-key="test"');
    expect(resultHtml).toContain('data-action="apply-solver-baseline"');
    expect(resultHtml).toContain("42ms");
    expect(resultHtml).toContain("评估 1 / 2000");
    expect(resultHtml).toContain("推荐");
    expect(resultHtml).toContain(">胜<");
    expect(resultHtml).toContain('class="solver-order-digits"');
    expect(resultHtml).toContain(">21</code>");
    expect(resultHtml).toContain("数字 = 求解基准牌槽位 1–2");
    expect(resultHtml).toContain('class="solver-baseline"');
    expect(resultHtml).toContain('data-action="apply-solver-baseline"');
    expect(resultHtml).toMatch(/class="solver-row[\s\S]*class="solver-baseline"/);
    expect(resultHtml).not.toContain("winning-recommendation");
    expect(resultHtml).not.toContain("recommendation-block");
    expect(resultHtml).toContain("含合成随机判定（seed 1,2,3），非原版决策");
    expect(resultHtml).toContain("hpDelta <b>+10</b>");
    expect(resultHtml).toContain("actorTurn <b>1</b>");
    expect(resultHtml).toContain("1: 旧牌 -&gt; 测试牌");
    expect(resultHtml).not.toContain("旧牌 / 测试牌");
    expect(resultHtml).not.toContain('class="solver-why"');
    state.solverResult = { ...state.solverResult, usedSyntheticDecisions: false };
    expect(renderSolverPanel(state)).not.toContain("非原版决策");
  });

  test("求解运行中显示等待态并禁用求解控件", () => {
    const state = baseState();
    state.solverMode = "orderBeam";
    state.solverStatus = {
      mode: "orderBeam",
      state: "running",
      maxEvaluations: 2_000,
      startedAt: 10,
      runId: 1,
    };

    const html = renderSolverPanel(state);
    expect(html).toContain('class="solver-panel busy has-status"');
    expect(html).toContain('aria-busy="true"');
    expect(html).toContain('class="solver-spinner"');
    expect(html).toContain("Worker 计算中");
    expect(html).toContain("预算 2000");
    expect(html).toContain('data-solver-task="order"');
    expect(html).toContain('data-solver-method="heuristic"');
    expect(html).toContain('<fieldset class="solver-methods" aria-label="搜索方式" disabled>');
    expect(html).toContain('class="solver-run primary" data-action="solve-active" disabled');
    expect(html).toContain('class="solver-cancel"');
    expect(html).toContain('data-action="cancel-solver"');
    expect(html).toContain('role="status" aria-live="polite"');
  });

  test("求解结果展示 value 为什么与列表血差最优分歧", () => {
    const state = baseState();
    state.solverResult = {
      mode: "beam",
      side: "p1",
      confidence: "heuristic",
      evaluatedCount: 2,
      skippedDuplicateCount: 0,
      candidateCardCount: 9,
      baseline: solverEvaluation(20, {
        scoreProfile: "value-v0",
        hpDeltaForSide: 10,
        valueMetrics: solverValueMetrics(20, { terminalHpForSide: 10 }),
      }),
      baselineDeck: [],
      results: [
        {
          rank: 1,
          confidence: "heuristic",
          deck: [{ id: 1, baseId: 1, name: "价值牌" }],
          leftoverHandCardIds: [],
          evaluation: solverEvaluation(31, {
            scoreProfile: "value-v0",
            hpDeltaForSide: 12,
            valueMetrics: solverValueMetrics(31, { terminalHpForSide: 12 }),
          }),
          changedSlots: [],
          deckKey: "value-top",
        },
        {
          rank: 2,
          confidence: "heuristic",
          deck: [{ id: 3, baseId: 3, name: "血差牌" }],
          leftoverHandCardIds: [],
          evaluation: solverEvaluation(29, {
            scoreProfile: "value-v0",
            hpDeltaForSide: 18,
            valueMetrics: solverValueMetrics(29, { terminalHpForSide: 18 }),
          }),
          changedSlots: [],
          deckKey: "hp-top",
        },
      ],
    };

    const html = renderSolverPanel(state);
    expect(html).toContain('class="solver-why"');
    expect(html).toContain("为什么");
    expect(html).toContain("Value +11");
    expect(html).toContain("HP +2");
    expect(html).toContain("value-hp-regret");
    expect(html).toContain("血差最优 第2");
    expect(html).toContain(">血差最优<");
    expect(html).toContain("比血差最优少 6 HP");
  });

  test("combined-page 右侧战斗日志区占 60%", () => {
    const baseCss = readFileSync(resolve(import.meta.dir, "../styles/base.css"), "utf8");
    const responsiveCss = readFileSync(resolve(import.meta.dir, "../styles/responsive.css"), "utf8");
    expect(baseCss).toMatch(
      /\.combined-page\s*\{[\s\S]*grid-template-columns:\s*minmax\(460px,\s*0\.4fr\)\s*minmax\(620px,\s*0\.6fr\)/,
    );
    expect(responsiveCss).toMatch(
      /@media \(max-width:\s*1280px\)[\s\S]*\.combined-page\s*\{[\s\S]*grid-template-columns:\s*minmax\(360px,\s*0\.4fr\)\s*minmax\(0,\s*0\.6fr\)/,
    );
  });

  test("左侧构筑列永不被遮挡，功能都在右列", () => {
    const state = baseState();
    const html = renderApp(state);
    expect(html).toContain('class="combined-page setup-empty"');
    // 四个工作流入口、全高覆盖与 Esc 收起整套机制已删除：左列任何时候都能直接调。
    expect(html).not.toContain("workflow-nav");
    expect(html).not.toContain("side-panel-expanded");
    expect(html).not.toContain("has-workflow-overlay");
    expect(html).not.toContain("data-action=\"toggle-side-panel\"");
    expect(html).not.toContain("data-action=\"show-side-panel\"");
    // 获胜建议与引擎透视都落在右列：战斗前它是独立面板，战斗后是模块选项卡之一。
    expect(html).toMatch(
      /<section class="combined-battle">[\s\S]*class="battle-module-body standalone" data-module="advice"/,
    );

    const setupLayoutCss = readFileSync(resolve(import.meta.dir, "../styles/setup-layout.css"), "utf8");
    expect(setupLayoutCss).not.toContain("has-workflow-overlay");
  });

  test("顶部命令带保持响应式，求解任务与主操作同排", () => {
    const setupLayoutCss = readFileSync(resolve(import.meta.dir, "../styles/setup-layout.css"), "utf8");
    const solverCss = readFileSync(resolve(import.meta.dir, "../styles/setup-solver.css"), "utf8");
    const responsiveCss = readFileSync(resolve(import.meta.dir, "../styles/responsive.css"), "utf8");
    expect(setupLayoutCss).toMatch(
      /\.setup-command-row\s*\{[\s\S]*grid-template-columns:\s*auto minmax\(0, 1fr\) auto/,
    );
    expect(solverCss).toMatch(
      /\.solver-command-bar\s*\{[\s\S]*grid-template-columns:\s*auto minmax\(140px, 1fr\) auto/,
    );
    expect(solverCss).toMatch(
      /\.combined-page \.solver-panel\s*\{[\s\S]*display:\s*flex;[\s\S]*flex-direction:\s*column;[\s\S]*align-items:\s*stretch/,
    );
    expect(responsiveCss).toMatch(
      /@media \(max-width:\s*1280px\)[\s\S]*\.setup-command-row\.has-solver \.solver-panel\s*\{[\s\S]*grid-column:\s*1 \/ -1/,
    );
  });

  test("入口 HTML 缓存版本与 combined 视图一致", () => {
    const indexHtml = readFileSync(resolve(import.meta.dir, "../../../index.html"), "utf8");
    const setupCss = readFileSync(resolve(import.meta.dir, "../styles/setup.css"), "utf8");
    const cardFaceCss = readFileSync(resolve(import.meta.dir, "../styles/card-face.css"), "utf8");
    const battleCss = readFileSync(resolve(import.meta.dir, "../styles/battle.css"), "utf8");
    expect(indexHtml).toMatch(/\/public\/build\/main\.js\?v=\d+/);
    expect(indexHtml).toContain('/src/ui/styles/base.css?v=0');
    expect(indexHtml).toContain('/src/ui/styles/battle.css?v=1');
    expect(indexHtml).toContain('/src/ui/styles/responsive.css?v=2');
    expect(indexHtml).toContain('/src/ui/styles/setup.css?v=1');
    expect(setupCss).toContain('./player-panel-battle.css?v=0');
    expect(battleCss).toContain('./player-panel-battle.css?v=0');
    expect(battleCss).toContain('./battle-verdict.css?v=0');
    expect(battleCss).toContain('./battle-layout.css?v=1');
    expect(battleCss).toContain('./player-panel-deck.css?v=0');
    expect(battleCss).toContain('./battle-inspector.css?v=4');
    expect(battleCss).toContain('./battle-trajectory.css?v=3');
    expect(setupCss).toContain('./setup-solver.css?v=0');
    expect(setupCss).toContain('./setup-layout.css?v=0');
    expect(setupCss).toContain('./player-panel-deck.css?v=0');
    expect(setupCss).toContain('./setup-picker.css?v=1');
    expect(setupCss).toContain('./setup-picker-card.css?v=3');
    expect(setupCss).toContain('./setup-picker-candidates.css?v=1');
    expect(setupCss).toContain('./card-face.css?v=0');
    expect(cardFaceCss).toMatch(
      /\.card-popup \.card-face-name\s*\{[\s\S]*font-size:\s*clamp\(16px, 0\.9vw, 17px\)/,
    );
    expect(`${indexHtml}\n${setupCss}\n${battleCss}`).not.toContain("ui53");
    const appHtml = renderApp(baseState());
    expect(appHtml).toContain('class="combined-page setup-empty"');
    expect(appHtml).toContain("对局构筑");
    expect(appHtml).toContain("顺序：设置先手与修炼轮");
    expect(appHtml).toContain("检查：阻断战斗或求解的问题");
    expect(appHtml).toContain('class="combined-setup"');
    expect(appHtml).toContain('class="combined-battle"');
    expect(appHtml).toContain('class="setup-picker-host"');
    expect(appHtml).not.toContain("激活卡槽");
    expect(appHtml).toContain('data-action="toggle-fixture-import"');
    expect(appHtml).toContain('class="setup-command-actions"');
    expect(appHtml).toContain('class="setup-command-row "');
    expect(appHtml).toContain('class="setup-match-controls" aria-label="对局参数"');
    expect(appHtml).toContain(">先手</span>");
    expect(appHtml).not.toContain('class="first-picker-label"');
    // 开始战斗按钮已删除：构筑变动自动推演，命令行只剩导入/重置（推演中加取消）。
    expect(appHtml).not.toContain('data-action="run"');
    // 空态右列改为模拟器功能介绍与使用指南，不再有动态待命提示与卡组进度指标。
    expect(appHtml).toContain("弈仙牌战斗模拟器");
    expect(appHtml).toContain('class="simulator-intro"');
    expect(appHtml).toContain('class="simulator-intro-features"');
    expect(appHtml).toContain('class="simulator-intro-guide"');
    expect(appHtml).not.toContain("战斗待命");
    expect(appHtml).not.toContain("选好双方角色并各摆 1 张场上牌即自动推演");
    expect(appHtml).not.toContain('class="battle-ready-grid"');
    expect(appHtml).not.toContain('class="battle-ready-metric');
    expect(appHtml).not.toContain('class="empty-state"');
    // 引擎透视只在有战斗结果时出现；空态下右列只该有模拟器说明与求解建议。
    expect(appHtml).toMatch(/自由构筑[\s\S]*求解建议/);
    expect(appHtml).not.toContain("引擎透视");
    expect(appHtml).toMatch(/setup-match-controls[\s\S]*setup-command-actions[\s\S]*solver-panel/);
    expect(appHtml).not.toContain('class="top-actions"');
    expect(appHtml).toMatch(/<section class="combined-battle">[\s\S]*<\/section>\s*<div class="setup-picker-host">/);
  });

  test("战斗运行态公开 Worker 状态和可聚焦取消操作", () => {
    const state = baseState();
    state.battleStatus = {
      state: "running",
      startedAt: 10,
      requestId: "battle-1",
    };
    const html = renderApp(state);
    expect(html).toContain('aria-busy="true"');
    expect(html).toContain('data-action="cancel-battle"');
    expect(html).toContain("计算已离开主线程");
    expect(html).toContain("Esc");
    expect(html).not.toContain('data-action="run"');
  });

  test("角色、仙命、天衍与卡牌浮层使用统一视觉层级", () => {
    const state = baseState();
    state.pickerMode = "character";
    const characterHtml = renderApp(state);
    expect(characterHtml).toContain('class="picker-popup build-picker-popup identity-popup character-popup"');
    expect(characterHtml).toContain('class="picker-popup-title"');
    expect(characterHtml).toContain('class="picker-popup-close"');
    expect(characterHtml).toContain('class="build-picker-tabs"');
    expect(characterHtml).toContain('data-mode="character"');
    expect(characterHtml).toContain('data-mode="talent"');
    expect(characterHtml).toContain('data-mode="fate"');
    expect(characterHtml).toContain('data-mode="card"');
    expect(characterHtml).toContain("角色选择");
    expect(characterHtml).toContain("作用：切换当前玩家角色。");
    expect(characterHtml).toContain("联动：角色会限定可选仙命");
    expect(characterHtml).toContain("仙命选择");
    expect(characterHtml).toContain("槽位：第一格为角色固定仙命");
    expect(characterHtml).toContain("天衍策略");
    expect(characterHtml).toContain("范围：这里只配置战斗输入");
    expect(characterHtml).toContain("卡牌选择");
    expect(characterHtml).toContain("筛选：可搜索");

    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    state.pickerMode = "talent";
    state.selectedTalentSlot = 3;
    const talentHtml = renderApp(state);
    expect(talentHtml).toContain('class="picker-popup build-picker-popup identity-popup talent-popup"');
    expect(talentHtml).toContain("构筑选择");

    state.pickerMode = "fate";
    const fateHtml = renderApp(state);
    expect(fateHtml).toContain('class="picker-popup build-picker-popup identity-popup fate-popup"');
    expect(fateHtml).toContain('data-mode="fate"');
    expect(fateHtml).toMatch(/data-mode="fate"[^>]*>[\s\S]*?天衍[\s\S]*?class="build-picker-tab-count">0\/\d+</);
    expect(fateHtml).toContain('data-action="clear-fate-strategies"');

    const pickerCss = ["setup-picker.css", "setup-picker-card.css", "setup-picker-candidates.css"]
      .map((file) => readFileSync(resolve(import.meta.dir, `../styles/${file}`), "utf8"))
      .join("\n");
    const responsiveCss = readFileSync(resolve(import.meta.dir, "../styles/responsive.css"), "utf8");
    expect(pickerCss).toMatch(/\.identity-popup \.cand-name\s*\{[\s\S]*font-size:\s*15px/);
    expect(pickerCss).toMatch(/\.character-popup\s*\{[\s\S]*height:\s*min\(372px/);
    // 卡牌/角色/仙命/副职/天衍浮层统一为右侧栏（钉在第 2 列），遮罩只盖右列且不压黑
    expect(pickerCss).toMatch(/\.combined-page > \.setup-picker-host \.card-popup,[\s\S]*?\.combined-page > \.setup-picker-host \.character-popup,[\s\S]*?\.combined-page > \.setup-picker-host \.talent-popup,[\s\S]*?\.combined-page > \.setup-picker-host \.career-popup,[\s\S]*?\.combined-page > \.setup-picker-host \.fate-popup\s*\{[\s\S]*?grid-column:\s*2;/);
    expect(pickerCss).toMatch(/\.combined-page > \.setup-picker-host \.picker-popup-backdrop\s*\{[\s\S]*?grid-column:\s*2;[\s\S]*?background:\s*transparent;/);
    expect(responsiveCss).toMatch(/\.character-popup \.character-popup-grid\s*\{[\s\S]*repeat\(4/);
  });

  test("兼修副职没有未选选项，已占用副职在兼修列禁用", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    // 金丹槽(2)与元婴槽(3)都开副职兼修；金丹兼修符咒师、元婴兼修琴师
    state.config.players.p1.talents[2] = 10_188;
    state.config.players.p1.talents[3] = 20_188;
    state.config.players.p1.dualCareerNames[2] = "FuZhouShi";
    state.config.players.p1.dualCareerNames[3] = "QinShi";
    state.pickerMode = "career";
    const html = renderApp(state);

    expect(html).toContain("金丹兼修");
    expect(html).toContain("元婴兼修");
    // 没有「未选」选项
    expect(html).not.toContain("未选");
    // 主副职炼丹师与另一兼修槽已占的副职，在本兼修列里禁用
    expect(html).toMatch(/data-slot="2"[^>]*data-career-id="LianDanShi"[^>]*disabled/);
    expect(html).toMatch(/data-slot="2"[^>]*data-career-id="QinShi"[^>]*disabled/);
    expect(html).not.toMatch(/data-slot="2"[^>]*data-career-id="FuZhouShi"[^>]*disabled/);
    expect(html).toMatch(/data-slot="3"[^>]*data-career-id="LianDanShi"[^>]*disabled/);
    expect(html).toMatch(/data-slot="3"[^>]*data-career-id="FuZhouShi"[^>]*disabled/);
    expect(html).not.toMatch(/data-slot="3"[^>]*data-career-id="QinShi"[^>]*disabled/);
  });

  test("玩家导入优先本机记录，工程 catalog 只在显式回放实验室模式出现", () => {
      const state = baseState();
      state.fixtureImportOpen = true;
      state.replayImportDeveloperMode = true;
      state.fixtureImportQuery = publicCatalogEntry.matchId;
      const html = renderFixtureImportPanel(state, [publicCatalogEntry]);
      expect(html).toContain('class="fixture-import-panel replay-import-dialog"');
      expect(html).toContain('data-original-replay-directory="1"');
      expect(html).toContain('data-original-replay-files="1"');
      expect(html).toContain("文件只在当前浏览器本地解析，不会上传");
      expect(html).toContain("复制给 AI 助手");
      expect(html).toContain('class="fixture-dev-catalog"');
      expect(html).toContain('data-fixture-query="1"');
      expect(html).not.toContain('class="fixture-select"');
      expect(html).toContain('data-action="import-fixture"');
      expect(html).toMatch(/data-action="import-fixture"[^>]*>导入</);
      expect(html).toContain("导入并战斗");
      expect(html).toContain(publicCatalogEntry.id);
      expect(html).not.toContain("<textarea");
      expect(fixtureEntryById(publicCatalogEntry.id, [publicCatalogEntry])?.path).toBe(publicCatalogEntry.path);
      expect(filterFixtureEntries(publicCatalogEntry.matchId, 80, [publicCatalogEntry]).some(
        (entry) => entry.id === publicCatalogEntry.id
      )).toBe(true);
  });

  test("选卡弹层复用卡面组件", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_004, state.config.gameRound);
    state.pickerMode = "card";
    const html = renderCardPopup(state);
    expect(html).toContain('class="picker-popup build-picker-popup card-popup"');
    expect(html).toContain('class="card-face');
    expect(html).toContain('class="card-picker-library" data-card-picker-scroll="1"');
    expect(html).toContain('class="card-picker-scopes"');
    expect(html).toContain('data-scope="common"');
    expect(html).toContain('data-scope="season"');
    expect(html).toContain('data-scope="special"');
    expect(html).toContain('class="card-picker-group"');
    expect(html).toContain('class="card-picker-cards"');
    // 顶部八槽已移除，只留左侧构筑区八槽；浮层内不再渲染卡槽选择。
    expect(html).not.toContain('class="card-picker-deck"');
    expect(html).not.toContain('data-action="select-picker-slot"');
    // 持续与普通牌合流，不再按功能类型分层；同系列牌靠 data-series 色条区分。
    expect(html).not.toContain('class="card-picker-type-rows"');
    expect(html).not.toContain('class="card-picker-type-row"');
    expect(html).not.toContain('class="card-picker-type-label');
    expect(html).toContain('data-series="');
    expect(html).not.toContain('class="card-picker-level-rail');
    expect(html).toContain('data-picker-search="card"');
    expect(html).toContain("构筑选择");
    expect(html).toContain('class="build-picker-tabs"');
    expect(html).not.toContain("快捷键提示");
    expect(html).not.toContain("当前第");
    expect(html.indexOf("专属")).toBeLessThan(html.indexOf("化神"));
    expect(html.indexOf("化神")).toBeLessThan(html.indexOf("元婴"));
    expect(html).not.toContain("天衍仙命");
    expect(html).not.toContain("法宝");
    expect(html).not.toContain("通用");
    expect(html).not.toContain("--picker-cols");
    expect(html).not.toContain('class="deck-candidate normal');
    expect(html).not.toContain('class="card-face-level"');
  });

  test("选卡网格十列竖长卡、分类直写竖排、系列色条区分", () => {
    const pickerCss = readFileSync(resolve(import.meta.dir, "../styles/setup-picker-card.css"), "utf8");
    expect(pickerCss).toMatch(
      /\.card-picker-cards\s*\{[\s\S]*grid-template-columns:\s*repeat\(10, minmax\(0, 1fr\)\)/,
    );
    expect(pickerCss).toMatch(
      /\.card-picker-group-label\s*\{[^}]*display:\s*flex;[^}]*flex-direction:\s*column;/,
    );
    expect(pickerCss).toMatch(/\.card-picker-group-name\s*\{[^}]*display:\s*grid;[^}]*place-items:\s*center/);
    expect(pickerCss).toMatch(/\.card-picker-group-badge\s*\{/);
    expect(pickerCss).toMatch(
      /\.card-picker-cards \.card-face-name\s*\{[\s\S]*text-align:\s*center/,
    );
    // 系列牌左侧色条：同系列同色，进阶链一眼可辨。
    expect(pickerCss).toMatch(/\.card-popup \.card-picker-cards \.card-face\[data-series\]/);
    expect(pickerCss).toMatch(/data-series="星弈"/);
    expect(pickerCss).not.toMatch(/\.card-picker-slot-level/);
    expect(pickerCss).not.toMatch(/\.card-picker-type-rows/);
  });

  test("选卡范围按常用、赛季、特殊分层，搜索在当前角色牌池范围内跨分层", () => {
    const state = baseState();
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_005, state.config.gameRound);
    state.pickerMode = "card";

    const commonHtml = renderCardPopup(state);
    expect(commonHtml).toContain("化神");
    expect(commonHtml).not.toContain("遗迹法器");
    expect(commonHtml).not.toContain("法宝");

    state.cardPickerScope = "season";
    const seasonHtml = renderCardPopup(state);
    expect(seasonHtml).toContain("遗迹法器");
    // 赛季范围不应出现常用范围的门派分组标签（化神境门派牌），但梦卡境界标签里的"化神"是正确的。
    expect(seasonHtml).not.toContain('data-card-group="HuaShen"');

    state.cardPickerScope = "special";
    const specialHtml = renderCardPopup(state);
    expect(specialHtml).toContain("法宝");
    expect(specialHtml).not.toContain('data-card-group="HuaShen"');

    state.cardPickerScope = "common";
    state.cardSearch = "梦崩拳封";
    const separatorFreeHtml = renderCardPopup(state);
    expect(separatorFreeHtml).toContain("梦•崩拳封");

    state.cardSearch = "极崩天步";
    expect(renderCardPopup(state)).toContain("极•崩天步");

    state.cardSearch = "极迎";
    expect(renderCardPopup(state)).toContain("极•迎风掌");

    // 搜索角色牌池外的副职牌时不应出现该牌（琴师(QinShi) 副职的牌不在默认范围）
    state.cardSearch = "破音";
    const outOfScopeHtml = renderCardPopup(state);
    expect(outOfScopeHtml).not.toContain('data-base-id="5000001"');
  });

  test("初始构筑不预选角色", () => {
    const state = baseState();
    expect(state.config.players.p1.characterId).toBe(0);
    expect(state.config.players.p2.characterId).toBe(0);

    const html = renderPlayerPanel(state, "p1");
    expect(html).toContain("选择角色");
    expect(html).not.toContain('class="talent-row"');
  });

  test("选卡浮层不再渲染顶部八槽，卡面居中且无等级调节", () => {
    const state = baseState();
    state.pickerMode = "card";
    const emptyHtml = renderCardPopup(state);
    const baseId = Number(emptyHtml.match(/data-action="pick-card" data-base-id="(\d+)"/)?.[1]);
    state.config.players.p1.deck[0] = { baseId, level: 1 };
    const html = renderCardPopup(state);
    expect(html).not.toContain("card-picker-deck");
    expect(html).not.toContain("card-picker-slot-level");
    expect(html).not.toContain("adjust-card-level");
    expect(html).not.toContain('class="card-picker-type-row"');
    const pickerCss = readFileSync(resolve(import.meta.dir, "../styles/setup-picker-card.css"), "utf8");
    expect(pickerCss).toMatch(
      /\.card-picker-cards \.card-face-name\s*\{[\s\S]*text-align:\s*center/,
    );
    expect(pickerCss).not.toMatch(/\.card-picker-deck/);
  });

  test("选卡后保持工作区并自动前进到下一空槽", () => {
    const state = baseState();
    state.pickerMode = "card";
    state.selectedSlot = 0;
    const html = renderCardPopup(state);
    const baseId = Number(html.match(/data-action="pick-card" data-base-id="(\d+)"/)?.[1]);
    expect(Number.isInteger(baseId)).toBe(true);
    let renderCount = 0;
    handleAction({
      currentTarget: { dataset: { action: "pick-card", baseId: String(baseId) } },
    } as unknown as Event, {
      state,
      render: () => { renderCount += 1; },
      runBattle: () => {},
      resetBattle: () => {},
      stopAuto: () => {},
      toggleAuto: () => {},
      adjacentCompletedTurnFrameIndex: () => 0,
    });

    expect(state.config.players.p1.deck[0]?.baseId).toBe(baseId);
    expect(state.pickerMode).toBe("card");
    expect(state.selectedSlot).toBe(1);
    expect(renderCount).toBe(1);
  });

  test("左侧构筑区卡槽切换替换目标时保留搜索词", () => {
    const state = baseState();
    state.pickerMode = "card";
    state.cardSearch = "剑";
    handleAction({
      currentTarget: { dataset: { action: "select-slot", side: "p1", slot: "4" } },
    } as unknown as Event, {
      state,
      render: () => {},
      runBattle: () => {},
      resetBattle: () => {},
      stopAuto: () => {},
      toggleAuto: () => {},
      adjacentCompletedTurnFrameIndex: () => 0,
    });

    expect(state.selectedSlot).toBe(4);
    expect(state.cardSearch).toBe("剑");
    expect(state.pickerMode).toBe("card");
  });

  test("选卡搜索时才展开通用牌", () => {
    const state = baseState();
    state.pickerMode = "card";
    state.cardSearch = "普通攻击";
    const html = renderCardPopup(state);

    expect(html).toContain("通用");
    expect(html).toContain("普通攻击");
  });

  test("存档区为单条输入加存删按钮", () => {
    const html = renderPlayerPanel(baseState(), "p1");
    expect(html).toContain('class="build-archive-wrap"');
    expect(html).toContain('class="build-archive-bar"');
    expect(html).toContain('class="build-archive-toggle"');
    expect(html).toContain('data-action="toggle-build-archive"');
    expect(html).toContain('class="build-archive-menu"');
    expect(html).toContain('data-build-archive="p1"');
    expect(html).not.toContain("<datalist");
    expect(html).not.toContain('list="build-list-p1"');
    expect(html).toContain('data-action="save-build"');
    expect(html).not.toContain('data-action="import-build-file"');
    expect(html).not.toContain('data-action="export-build-file"');
    expect(html).not.toContain('data-build-file-import="1"');
    expect(html).toContain('data-action="delete-build"');
    expect(html).not.toContain('data-action="load-build"');
    expect(html).not.toContain('class="build-load-select"');
  });
});
