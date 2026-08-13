import { describe, expect, test } from "bun:test";
import { defaultPlayerConfig } from "../data";
import {
  renderCumulativeOverlay,
  renderDamageStepChart,
  renderStackedDamageChart,
  renderTargetResult,
  renderUsageSummary,
} from "../render-target-chart";
import { renderTargetSetupPane } from "../render-target-setup";
import { cardPalette, cardPaletteKey } from "../target-practice-metrics";
import { baseState } from "./layout-test-helpers";
import type {
  AppState,
  TargetBuild,
  TargetDamageStep,
  TargetPracticeState,
  TargetTurnDamage,
} from "../types";

function perTurn(): readonly TargetTurnDamage[] {
  return [
    { round: 1, total: 25, byCard: [
      { cardId: 82, cardName: "万玄破魔掌", damage: 15 },
      { cardId: null, cardName: null, damage: 10 },
    ] },
    { round: 2, total: 0, byCard: [] },
    { round: 3, total: 100, byCard: [
      { cardId: 82, cardName: "万玄破魔掌", damage: 100 },
    ] },
  ];
}

type PartialTargetResult = Omit<NonNullable<TargetBuild["result"]>, "steps">;

function withSteps(result: PartialTargetResult): TargetBuild["result"] {
  const steps: TargetDamageStep[] = [];
  let cumulative = 0;
  let actorTurn = 1;
  for (const turn of result.perTurn) {
    for (const entry of turn.byCard) {
      cumulative += entry.damage;
      steps.push({
        round: turn.round,
        actorTurn,
        cardId: entry.cardId,
        cardName: entry.cardName,
        damage: entry.damage,
        cumulative,
      });
      actorTurn += 2;
    }
  }
  return { ...result, steps };
}

function build(id: string, name: string, result?: PartialTargetResult): TargetBuild {
  return {
    id,
    name,
    player: defaultPlayerConfig("p1", 4_000_004, 16),
    result: result ? withSteps(result) : null,
    status: result ? "done" : "idle",
    errorMessage: null,
  };
}

function targetState(overrides: Partial<TargetPracticeState> = {}): TargetPracticeState {
  return {
    builds: [build("b1", "构筑 A")],
    activeBuildId: "b1",
    damageThreshold: 120,
    displayRounds: 1,
    displayRoundMin: 1,
    displayRoundPending: false,
    compareMode: "overlay",
    expandedStep: null,
    expandedStepBuildId: null,
    duelP1Player: null,
    ...overrides,
  };
}

function targetAppState(overrides: Partial<TargetPracticeState> = {}): AppState {
  const state = baseState();
  state.workbenchMode = "target";
  state.target = targetState(overrides);
  return state;
}

describe("堆叠柱状图 SVG", () => {
  test("按回合出柱、按卡分段着色、图例只列出现的卡", () => {
    const palette = cardPalette([82, null]);
    const html = renderStackedDamageChart(perTurn(), palette, {
      label: "构筑 A",
      threshold: 120,
    });
    expect(html).toContain('<svg class="stacked-chart-svg"');
    expect(html).toContain("构筑 A · 每回合伤害（按卡牌来源）");
    expect(html).toContain(`class="bar-seg ${palette.get(cardPaletteKey(82))}"`);
    expect(html).toContain('class="bar-seg card-other"');
    expect(html).toContain('data-round="3"');
    expect(html).toContain("第 3 回合 · 万玄破魔掌 100 伤");
    expect(html).toContain("阈值");
    expect(html).toContain("持续/其他");
    expect(html).toContain("万玄破魔掌");
    expect(html).toContain("R1");
    expect(html).toContain("R3");
  });

  test("选中回合的柱段带 selected 标记", () => {
    const palette = cardPalette([82, null]);
    const html = renderStackedDamageChart(perTurn(), palette, { selectedRound: 3 });
    expect(html).toContain(`class="bar-seg ${palette.get(cardPaletteKey(82))} selected"`);
  });

  test("无阈值不画阈值线；零伤害回合画淡色占位柱保持 x 轴连续", () => {
    const plain = renderStackedDamageChart(perTurn(), cardPalette([]), { scopeClass: "flow" });
    expect(plain).not.toContain("threshold-line");
    expect(plain).toContain("stacked-chart-svg");
    expect(plain).toContain('class="bar-empty"');
    expect(plain).toMatch(/data-round="2"[\s\S]*bar-empty/);
    const withThreshold = renderStackedDamageChart([], cardPalette([]), { threshold: 120 });
    expect(withThreshold).toContain("threshold-line");
    expect(withThreshold).toContain('class="bar-empty"');
  });

  test("选中无伤尾回合时 x 轴补柱并带 selected", () => {
    const html = renderStackedDamageChart(perTurn(), cardPalette([82, null]), {
      selectedRound: 6,
    });
    expect(html).toContain('class="bar-empty selected"');
    expect(html).toContain('data-round="6"');
    expect(html).toContain("R6");
  });

  test("minRounds 把 x 轴延伸到终局回合（阈值/上限上下文）", () => {
    const html = renderStackedDamageChart(perTurn(), cardPalette([82, null]), { minRounds: 6 });
    expect(html).toContain('data-round="6"');
    expect(html).toContain("R6");
    expect(html).toContain('class="bar-empty"');
  });

  test("累计趋势层唯一且总量超过单回合峰值时仍在 viewBox 内", () => {
    const high = [
      { round: 1, total: 100, byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 100 }] },
      { round: 2, total: 100, byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 100 }] },
    ] satisfies readonly TargetTurnDamage[];
    const html = renderStackedDamageChart(high, cardPalette([82]), {});
    expect(html.match(/class="cumulative-area"/g)).toHaveLength(1);
    expect(html.match(/class="cumulative-trend-line"/g)).toHaveLength(1);
    const points = html.match(/class="cumulative-trend-line" points="([^"]+)"/)?.[1] ?? "";
    for (const point of points.split(" ")) {
      const y = Number(point.split(",")[1]);
      expect(y).toBeGreaterThanOrEqual(4);
      expect(y).toBeLessThanOrEqual(61);
    }
  });

  test("summaryItems 渲染用量面板式摘要读数；标题带累计总数", () => {
    const html = renderStackedDamageChart(perTurn(), cardPalette([82, null]), {
      summaryItems: [
        { key: "round", label: "本回合伤害", value: "100", tone: "warm" },
        { key: "total", label: "累计伤害", value: "125" },
      ],
    });
    expect(html).toContain('class="usage-summary"');
    expect(html).toContain('class="usage-summary-value">100<');
    expect(html).toContain('class="usage-summary-value">125<');
    expect(html).toContain('tone-warm');
    expect(html).not.toContain("累计 125 伤");
  });

  test("flow 作用域标题 note 带累计总数", () => {
    expect(renderStackedDamageChart(perTurn(), cardPalette([82, null]), { scopeClass: "flow" })).toContain("累计 125 伤");
  });

  test("renderUsageSummary 输出 tabular 数字读数，纯文本不依赖颜色", () => {
    const html = renderUsageSummary([{ key: "a", label: "累计伤害", value: "125", tone: "good" }]);
    expect(html).toContain('class="usage-summary"');
    expect(html).toContain('class="usage-summary-label">累计伤害<');
    expect(html).toContain('class="usage-summary-value">125<');
    expect(html).toContain("tone-good");
  });

  test("同一 baseId 变体共用一个图例项，取累计伤害最高的名字", () => {
    const variants: readonly TargetTurnDamage[] = [{
      round: 1,
      total: 8,
      byCard: [
        { cardId: 82, cardName: "万玄破魔掌", damage: 3 },
        { cardId: 10_082, cardName: "万玄破魔掌·二阶", damage: 5 },
      ],
    }];
    const html = renderStackedDamageChart(variants, cardPalette([82, 10_082]), {});
    const legend = html.slice(html.indexOf('class="card-legend"'));
    expect(legend.match(/legend-item/g)).toHaveLength(1);
    expect(legend).toContain("</span>万玄破魔掌·二阶");
    expect(legend).not.toContain("</span>万玄破魔掌\n");
  });

  test("同 baseId 变体图例比较的是变体累计伤害，不是单条 entry", () => {
    const variants: readonly TargetTurnDamage[] = [
      { round: 1, total: 8, byCard: [
        { cardId: 82, cardName: "万玄破魔掌", damage: 3 },
        { cardId: 10_082, cardName: "万玄破魔掌·二阶", damage: 5 },
      ] },
      { round: 2, total: 4, byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 4 }] },
    ];
    const html = renderStackedDamageChart(variants, cardPalette([82, 10_082]), {});
    const legend = html.slice(html.indexOf('class="card-legend"'));
    expect(legend.match(/legend-item/g)).toHaveLength(1);
    expect(legend).toContain("</span>万玄破魔掌\n");
    expect(legend).not.toContain("万玄破魔掌·二阶");
  });
});

describe("overlay 累计折线", () => {
  test("每套构筑一条累计折线 + 阈值线 + 构筑图例", () => {
    const done = build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "threshold", reachedTurn: 3 });
    const html = renderCumulativeOverlay([done], 120);
    expect(html).toContain("累计伤害对比");
    expect(html).toContain('class="cumulative-line build-line-1"');
    expect(html).toContain('tabindex="0"');
    expect(html).toContain('role="button"');
    expect(html).toContain("threshold-line");
    expect(html).toContain("构筑 A");
    expect(html).toContain('data-action="select-target-build"');
  });

  test("displayRounds 传入后 overlay 保留无伤尾部窗口", () => {
    const done = build("b1", "构筑 A", {
      perTurn: [{ round: 4, total: 120, byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 120 }] }],
      totalDamage: 120,
      stopReason: "threshold",
      reachedTurn: 4,
    });
    const html = renderCumulativeOverlay([done], 120, 6);
    expect(html).toContain("R6");
    expect(html).toContain("91.67");
  });

  test("折线延伸到 reachedTurn：最后一个伤害回合后以最终累计值水平平台化", () => {
    const done = build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "turnLimit", reachedTurn: 6 });
    const html = renderCumulativeOverlay([done], 120);
    const point = html.match(/class="cumulative-line[^>]*points="([^"]+)"/)?.[1]?.split(" ").at(-1) ?? "";
    expect(point.startsWith("91.67,")).toBe(true);
    const y = Number(point.split(",")[1]);
    expect(y).toBeGreaterThanOrEqual(4);
    expect(y).toBeLessThanOrEqual(61);
    expect(html).toContain("R6");
  });
});

describe("阶梯式累计伤害曲线（打靶主图）", () => {
  const steps: readonly TargetDamageStep[] = [
    { round: 1, actorTurn: 1, cardId: 82, cardName: "万玄破魔掌", damage: 23, cumulative: 23 },
    { round: 1, actorTurn: 1, cardId: null, cardName: null, damage: 42, cumulative: 65 },
    { round: 2, actorTurn: 3, cardId: 11, cardName: "玄冰诀", damage: 31, cumulative: 96 },
    { round: 2, actorTurn: 3, cardId: 82, cardName: "万玄破魔掌", damage: 37, cumulative: 133 },
  ];

  test("selectedStep 与 buildId 同步到热点", () => {
    const html = renderDamageStepChart(steps, cardPalette(steps.map((step) => step.cardId)), { selectedStep: 1, buildId: "b1" });
    expect(html).toContain('class="step-hotspot selected"');
    expect(html).toContain('data-build-id="b1"');
  });

  test("23→65→96→133 台阶序列：每步一个上升段 + 平台 + 末点，hover 显示卡名与累计", () => {
    const html = renderDamageStepChart(steps, cardPalette(steps.map((step) => step.cardId)), { threshold: 120 });
    expect(html).toContain("累计伤害（按出牌台阶）");
    expect(html.match(/class="step-rise /g)).toHaveLength(4);
    expect(html.match(/class="step-plateau"/g)).toHaveLength(3);
    expect(html).toContain('class="step-endpoint"');
    expect(html).toContain('data-step="0"');
    expect(html).toContain("+23（累计 23）");
    expect(html).toContain("+37（累计 133）");
    expect(html).toContain('class="step-rise card-other"');
    expect(html.match(/class="step-round-boundary"/g)).toHaveLength(2);
    expect(html).toContain("threshold-line");
    expect(html).toContain("R1");
    expect(html).toContain("R2");
  });

  test("展开热点带键盘语义；空数据渲染空态不炸", () => {
    const html = renderDamageStepChart(steps, cardPalette(steps.map((step) => step.cardId)), { threshold: 120 });
    expect(html).toContain('data-step="0" data-build-id="" data-action="toggle-target-step"');
    expect(html).toContain('tabindex="0"');
    expect(html).toContain('role="button"');
    expect(html).toContain('aria-label="第 1 回合');
    const empty = renderDamageStepChart([], cardPalette([]), {});
    expect(empty).toContain("未产生伤害");
    expect(empty).not.toContain("step-rise");
  });
});

describe("打靶结果面板", () => {
  test("无结果：空态引导", () => expect(renderTargetResult(targetAppState())).toContain("配置构筑后自动推演"));

  test("done：摘要读数 + 达成标签 + 堆叠柱", () => {
    const state = targetAppState({ builds: [build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "threshold", reachedTurn: 3 })] });
    const html = renderTargetResult(state);
    expect(html).toContain('class="usage-summary"');
    expect(html).toContain('class="usage-summary-value">125<');
    expect(html).toContain('class="usage-summary-value">120 伤<');
    expect(html).toContain('class="usage-summary-value">R3<');
    expect(html).toContain("已达成");
    expect(html).toContain("stacked-chart-svg");
    expect(html).toContain("target-progress-fill");
    expect(html).toContain('aria-label="阈值达成进度 100%"');
  });

  test("grid 模式：每套构筑一个分面", () => {
    const state = targetAppState({
      compareMode: "grid",
      builds: [
        build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 50, stopReason: "turnLimit", reachedTurn: 32 }),
        build("b2", "构筑 B", { perTurn: perTurn(), totalDamage: 120, stopReason: "threshold", reachedTurn: 3 }),
      ],
    });
    const html = renderTargetResult(state);
    expect(html).toContain('class="target-grid"');
    expect(html).toMatch(/target-grid-cell/g);
    expect(html).toContain('data-action="toggle-target-step"');
    expect(html).toContain("构筑 B");
    expect(html).toContain("已达成");
    expect(html).toContain("未达成");
  });

  test("overlay 模式：累计折线 + 聚焦构筑的堆叠柱；running 显示推演中", () => {
    const runningBuild = build("b2", "构筑 B");
    runningBuild.status = "running";
    const state = targetAppState({ builds: [build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "threshold", reachedTurn: 3 }), runningBuild] });
    const html = renderTargetResult(state);
    expect(html).toContain("cumulative-overlay");
    expect(html).toContain("cumulative-line");
    expect(html).toContain("推演中");
    expect(html).toContain("target-status-strip");
  });

  test("单构筑 no-trace error 在右侧结果面板明示并提供重试", () => {
    const failed = build("b1", "构筑 A");
    failed.status = "error";
    failed.errorMessage = "钩子链不可用：打靶伤害归因需要 trace";
    const html = renderTargetResult(targetAppState({ builds: [failed] }));
    expect(html).toContain('role="alert"');
    expect(html).toContain("钩子链不可用");
    expect(html).toContain('data-action="run-target-practice"');
  });

  test("error 状态在状态条显示错误信息，不崩溃", () => {
    const failed = build("b1", "构筑 A");
    failed.status = "error";
    failed.errorMessage = "打靶模拟失败：缺少原版决策";
    const other = build("b2", "构筑 B", { perTurn: perTurn(), totalDamage: 125, stopReason: "threshold", reachedTurn: 3 });
    const html = renderTargetResult(targetAppState({ builds: [failed, other] }));
    expect(html).toContain("打靶模拟失败：缺少原版决策");
    expect(html).toContain("target-status-chip status-error");
  });

  test("displayRounds=6 时阶梯图保留 R5/R6 零伤尾部刻度", () => {
    const state = targetAppState({
      displayRounds: 6,
      builds: [build("b1", "构筑 A", { perTurn: [{ round: 4, total: 120, byCard: [{ cardId: 82, cardName: "万玄破魔掌", damage: 120 }] }], totalDamage: 120, stopReason: "threshold", reachedTurn: 4 })],
    });
    const html = renderTargetResult(state);
    expect(html).toContain("R5");
    expect(html).toContain("R6");
    expect(html).toContain("显示至 R6");
  });
});

describe("显示至回合滑条（绝对有效回合数）", () => {
  function setupHtml(overrides: Partial<TargetPracticeState> = {}): string {
    const state = baseState();
    state.workbenchMode = "target";
    state.target = targetState(overrides);
    return renderTargetSetupPane(state);
  }

  test("无结果时滑条为禁用等待态：不伪造有效回合，min=1/max=32/step=1", () => {
    const html = setupHtml();
    expect(html).toContain("显示至回合");
    expect(html).toContain('id="battle-targetDisplayRounds"');
    expect(html).toContain('type="range"');
    expect(html).toContain('min="1"');
    expect(html).toContain('max="32"');
    expect(html).toContain('step="1"');
    expect(html).toContain("disabled");
    expect(html).toContain('aria-label="显示至回合"');
    expect(html).toContain('aria-valuetext="等待推演"');
    expect(html).toContain("等待推演");
    expect(html).not.toContain("battle-targetExtraRounds");
    expect(html).not.toContain("额外回合");
  });

  test("第 4 回合达标：滑条从 R4 开始，min=4、初始 value=4，不提供 0..3", () => {
    const html = setupHtml({
      displayRounds: 4,
      builds: [build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "threshold", reachedTurn: 4 })],
    });
    const slider = html.slice(html.indexOf('id="battle-targetDisplayRounds"'));
    const sliderBlock = slider.slice(0, slider.indexOf("</label>"));
    expect(sliderBlock).toContain('min="4"');
    expect(sliderBlock).toContain('value="4"');
    expect(sliderBlock).toContain('max="32"');
    expect(sliderBlock).toContain('step="1"');
    expect(sliderBlock).not.toContain('value="1"');
    expect(sliderBlock).not.toContain("disabled");
    expect(html).toContain("R4 / 32");
  });

  test("pending range 保留可操作且显示等待中的 R6", () => {
    const html = setupHtml({ displayRounds: 6, displayRoundMin: 4, displayRoundPending: true });
    expect(html).toContain('value="6"');
    expect(html).toContain('min="4"');
    expect(html).not.toMatch(/type="range"[^>]*disabled/);
    expect(html).toContain("R6 / 32 · 重算中");
  });

  test("达标回合变化时当前值钳到新范围：reachedTurn=6 时 value 从 4 被钳到 6", () => {
    const html = setupHtml({
      displayRounds: 4,
      builds: [build("b1", "构筑 A", { perTurn: perTurn(), totalDamage: 125, stopReason: "turnLimit", reachedTurn: 6 })],
    });
    expect(html).toContain('min="6"');
    expect(html).toContain('value="6"');
  });
});
