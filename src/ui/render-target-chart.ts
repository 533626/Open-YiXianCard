/**
 * 打靶模式右列结果面板 + 按卡牌来源的堆叠柱状图（手写 SVG，无图表库）。
 *
 * 同一根「回合聚合堆叠柱」也回填双方对战的伤害曲线（render-battle-flow.ts），
 * 通过 `renderStackedDamageChart` 共用；target 模式独有：阈值线、累计折线叠加
 * （overlay）/ 分面（grid）、点击柱段展开该回合明细。
 */

import {
  activeTargetBuild,
  targetPracticeState,
} from "./main-actions";
import {
  cardPalette,
  cardPaletteKey,
  targetReachedLabel,
} from "./target-practice-metrics";
import { GAME_TURN_LIMIT } from "./target-dummy";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type {
  AppState,
  TargetBuild,
  TargetCardDamage,
  TargetDamageStep,
  TargetTurnDamage,
} from "./types";

const CHART_W = 100;
const CHART_H = 64;
const CHART_PAD_TOP = 4;
const CHART_PAD_BOTTOM = 3;
const BUILD_LINE_CLASSES = ["build-line-1", "build-line-2", "build-line-3", "build-line-4"];

export interface StackedDamageChartOptions {
  /** 图表标题（图例/工具提示里的构筑名或玩家名）。 */
  readonly label?: string;
  /** 选中的回合（与时间轴对齐高亮）；null 不高亮。 */
  readonly selectedRound?: number | null;
  /** 阈值线（打靶模式）；null 不画。 */
  readonly threshold?: number | null;
  /** 构筑色 class（overlay 图例 / grid 分面头用）。 */
  readonly buildColorClass?: string | null;
  /** 额外作用域 class（当前只有双方对战曲线模块用 "flow"）。 */
  readonly scopeClass?: string;
  /** x 轴至少覆盖到的回合数（打靶模式传入 displayRounds，双方图表传入选中回合）。 */
  readonly minRounds?: number;
  /** 绝对展示窗口终点；用于打靶阶梯图保留零伤害尾部。 */
  readonly displayRounds?: number;
  /** 图表顶部用量面板式摘要读数（如双方对战的本回合伤害/累计伤害）。 */
  readonly summaryItems?: readonly {
    readonly key: string;
    readonly label: string;
    readonly value: string;
    readonly detail?: string;
    readonly tone?: "warm" | "good" | "danger" | "muted";
  }[];
}

export function renderTargetResult(state: AppState): string {
  const target = targetPracticeState(state);
  const active = activeTargetBuild(state);
  const doneBuilds = target.builds.filter((build) => build.status === "done" && build.result);
  return `
    <section class="panel target-result-panel" aria-label="打靶结果">
      <div class="panel-title">打靶</div>
      ${renderTargetStatusStrip(target.builds)}
      ${active?.result ? renderTargetSummaryBar(active, target.damageThreshold) : ""}
      ${renderTargetCharts(state, active, doneBuilds)}
      ${renderExpandedStepDetail(state, active)}
    </section>
  `;
}

/**
 * 用量面板式摘要读数：当前总量/阈值进度或本回合伤害，纯文本 + tabular 数字，
 * 不抢主体趋势图的视觉重量（OpenRouter token 用量视图的信息结构）。
 */
export function renderUsageSummary(
  items: readonly {
    readonly key: string;
    readonly label: string;
    readonly value: string;
    readonly detail?: string;
    readonly tone?: "warm" | "good" | "danger" | "muted";
  }[],
): string {
  return `
    <div class="usage-summary" aria-label="伤害读数摘要">
      ${items.map((item) => `
        <span class="usage-summary-item tone-${item.tone ?? "muted"}" title="${escapeAttribute(item.detail ?? "")}">
          <span class="usage-summary-label">${escapeHtml(item.label)}</span>
          <b class="usage-summary-value">${escapeHtml(item.value)}</b>
        </span>
      `).join("")}
    </div>
  `;
}

function renderTargetStatusStrip(builds: readonly TargetBuild[]): string {
  if (builds.length <= 1 && builds[0]?.status !== "error") return "";
  return `
    <div class="target-status-strip" aria-label="各构筑状态">
      ${builds.map((build) => `
        <span class="target-status-chip${build.status && build.status !== "idle" ? ` status-${build.status}` : ""}" title="${escapeAttribute(build.name)}">
          ${escapeHtml(build.name)}：${statusText(build)}
        </span>
      `).join("")}
    </div>
  `;
}

function statusText(build: TargetBuild): string {
  switch (build.status ?? "idle") {
    case "running": return "推演中";
    case "error": return build.errorMessage ?? "失败";
    case "done": return build.result
      ? `累计 ${build.result.totalDamage} 伤 · 第 ${build.result.reachedTurn} 回合 ${targetReachedLabel(build.result)}`
      : "完成";
    default: return "未运行";
  }
}

function renderTargetSummaryBar(build: TargetBuild, threshold: number): string {
  const result = build.result!;
  const reached = result.stopReason === "threshold";
  const percent = Math.min(100, Math.round((result.totalDamage / threshold) * 100));
  return `
    <div class="target-result-summary ${reached ? "reached" : ""}">
      <div class="target-summary-grid">
        ${renderUsageSummary([
          {
            key: "total",
            label: "累计伤害",
            value: String(result.totalDamage),
            detail: `累计伤害（截至当前「显示至回合」窗口终点）`,
            tone: reached ? "good" : "warm",
          },
          {
            key: "threshold",
            label: "阈值",
            value: `${threshold} 伤`,
            detail: `累计伤害达到 ${threshold} 即停止推演`,
          },
          {
            key: "reached",
            label: "达标回合",
            value: `R${result.reachedTurn}`,
            detail: `首个累计伤害 ≥ 阈值的回合；未达标时为回合上限 R${GAME_TURN_LIMIT}`,
            tone: reached ? "good" : "danger",
          },
          {
            key: "state",
            label: "状态",
            value: targetReachedLabel(result),
            detail: reached
              ? `第 ${result.reachedTurn} 回合累计伤害达到阈值 ${threshold}`
              : `打满 ${GAME_TURN_LIMIT} 回合仍未达到阈值 ${threshold}`,
            tone: reached ? "good" : "muted",
          },
        ])}
      </div>
      <div class="target-progress" role="progressbar" aria-valuenow="${percent}" aria-valuemin="0" aria-valuemax="100" aria-label="阈值达成进度 ${percent}%">
        <span class="target-progress-fill" style="width:${percent}%"></span>
      </div>
    </div>
  `;
}

function renderTargetCharts(
  state: AppState,
  active: TargetBuild | undefined,
  doneBuilds: readonly TargetBuild[],
): string {
  const target = state.target!;
  if (doneBuilds.length === 0) {
    const failed = target.builds.find((build) => build.status === "error");
    const running = target.builds.find((build) => build.status === "running");
    return `
      <div class="target-chart-empty" aria-live="polite">
        ${failed
          ? `<div class="target-result-empty error" role="alert"><strong>推演失败</strong><span>${escapeHtml(failed.errorMessage ?? "打靶推演失败，未生成可归因 trace")}</span><button type="button" class="build-action rerun" data-action="run-target-practice" data-build-id="${escapeAttribute(failed.id)}">重试</button></div>`
          : running
            ? `<div class="battle-running-status" role="status"><span class="solver-spinner" aria-hidden="true"></span><span>推演中，木桩不出伤害，累计达到阈值即停</span></div>`
            : `<div class="simulator-intro">配置构筑后自动推演：对手为高血量静默木桩（0 攻击 0 伤害），累计伤害 ≥ 阈值或打满回合上限即停。</div>`}
      </div>
    `;
  }
  if (target.compareMode === "grid") {
    return `
      <div class="target-grid" aria-label="分面对比">
        ${doneBuilds.map((build, index) => `
          <div class="target-grid-cell">
            <div class="target-grid-head ${BUILD_LINE_CLASSES[index % BUILD_LINE_CLASSES.length] ?? "build-line-1"}">
              <span>${escapeHtml(build.name)}</span>
              <b>${statusText(build)}</b>
            </div>
            ${renderDamageStepChart(build.result!.steps, cardPalette(cardIdsOfSteps(build.result!.steps)), {
              label: build.name,
              threshold: target.damageThreshold,
              displayRounds: target.displayRounds,
              selectedStep: target.expandedStepBuildId === build.id ? target.expandedStep ?? undefined : undefined,
              buildId: build.id,
            })}
          </div>
        `).join("")}
      </div>
    `;
  }
  // overlay：累计折线叠加 + 聚焦构筑的阶梯曲线。
  const overlay = renderCumulativeOverlay(doneBuilds, target.damageThreshold, target.displayRounds);
  const stacked = active?.result
    ? renderDamageStepChart(active.result.steps, cardPalette(cardIdsOfSteps(active.result.steps)), {
      label: active.name,
      threshold: target.damageThreshold,
      displayRounds: target.displayRounds,
      selectedStep: target.expandedStepBuildId === active.id ? target.expandedStep ?? undefined : undefined,
      buildId: active.id,
    })
    : "";
  return `
    <div class="target-overlay" aria-label="叠加对比">
      ${overlay}
      ${stacked}
    </div>
  `;
}

/** 步骤序列里的卡牌 id 列表（图例调色板用）。 */
function cardIdsOfSteps(steps: readonly TargetDamageStep[]): readonly (number | null)[] {
  return steps.map((step) => step.cardId);
}

/** overlay 模式：各构筑累计伤害折线叠加在同一坐标系；点击折线切换聚焦构筑。 */
export function renderCumulativeOverlay(
  builds: readonly TargetBuild[],
  threshold: number,
  displayRounds?: number,
): string {
  const roundCount = Math.max(
    1,
    displayRounds ?? 0,
    ...builds.map((build) => Math.max(maxRound(build.result!.perTurn), build.result!.reachedTurn)),
  );
  const cumulativeTotals = builds.map((build) =>
    build.result!.perTurn.reduce((sum, turn) => sum + turn.total, 0),
  );
  const maxY = Math.max(threshold, ...cumulativeTotals, 1);
  const slot = CHART_W / roundCount;
  const yFor = (value: number): number =>
    CHART_PAD_TOP + (CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM) * (1 - value / maxY);
  return `
    <div class="stacked-chart cumulative-overlay">
      ${renderChartHead("累计伤害对比", `阈值 ${threshold}`)}
      <div class="stacked-chart-body">
        <svg class="stacked-chart-svg" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" role="img" aria-label="累计伤害对比">
          <line class="threshold-line" x1="0" y1="${yFor(threshold)}" x2="${CHART_W}" y2="${yFor(threshold)}"></line>
          ${builds.map((build, index) => {
            const colorClass = BUILD_LINE_CLASSES[index % BUILD_LINE_CLASSES.length] ?? "build-line-1";
            let cumulative = 0;
            const points: string[] = [`0,${yFor(0)}`];
            for (const turn of build.result!.perTurn) {
              cumulative += turn.total;
              const x = (turn.round - 1) * slot + slot / 2;
              points.push(`${x.toFixed(2)},${yFor(cumulative).toFixed(2)}`);
            }
            const lastRound = maxRound(build.result!.perTurn);
            const plateauRound = Math.max(lastRound, build.result!.reachedTurn, displayRounds ?? 0);
            if (lastRound < plateauRound) {
              // x 轴延伸到绝对展示终点：无伤尾回合保持累计值平台。
              const plateauX = (plateauRound - 1) * slot + slot / 2;
              points.push(`${plateauX.toFixed(2)},${yFor(cumulative).toFixed(2)}`);
            }
            return `
              <polyline
                class="cumulative-line ${colorClass}"
                points="${escapeAttribute(points.join(" "))}"
                data-build-id="${escapeAttribute(build.id)}"
                data-action="select-target-build"
                data-keyboard-action="select-target-build"
                tabindex="0"
                role="button"
                aria-label="${escapeAttribute(`切换到${build.name}，第 ${build.result!.reachedTurn} 回合累计 ${build.result!.totalDamage} 伤`)}"
                title="${escapeAttribute(`${build.name}：第 ${build.result!.reachedTurn} 回合累计 ${build.result!.totalDamage} 伤`)}"
              ></polyline>
            `;
          })}
        </svg>
        ${renderRoundTicks(roundCount)}
      </div>
      <div class="stacked-chart-foot">
        ${builds.map((build, index) => `
          <button
            type="button"
            class="build-legend-item ${BUILD_LINE_CLASSES[index % BUILD_LINE_CLASSES.length] ?? "build-line-1"}"
            data-action="select-target-build"
            data-build-id="${escapeAttribute(build.id)}"
            title="点击切换聚焦构筑"
          >
            <span class="build-line-swatch"></span>${escapeHtml(build.name)}
          </button>
        `).join("")}
      </div>
    </div>
  `;
}

/**
 * 按回合聚合的堆叠柱状图（target 模式与双方对战伤害曲线共用）。
 * 每根柱 = 一回合（双方各动一次），柱内分段 = 各卡贡献，颜色按卡牌稳定分配。
 */
export function renderStackedDamageChart(
  perTurn: readonly TargetTurnDamage[],
  palette: ReadonlyMap<string, string>,
  options: StackedDamageChartOptions = {},
): string {
  const roundCount = Math.max(1, maxRound(perTurn), options.minRounds ?? 0, options.selectedRound ?? 0);
  const filled = fillRounds(perTurn, roundCount);
  const totals = filled.map((turn) => turn.total);
  const cumulativeTotal = totals.reduce((sum, total) => sum + total, 0);
  // 同一坐标系同时承载单回合柱高和累计趋势，纵轴必须覆盖累计总量，
  // 否则累计点会落到 viewBox 外而被裁掉。
  const maxY = Math.max(options.threshold ?? 0, ...totals, cumulativeTotal, 1);
  const slot = CHART_W / roundCount;
  const barWidth = Math.max(1.4, slot * 0.66);
  const yFor = (value: number): number =>
    CHART_PAD_TOP + (CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM) * (1 - value / maxY);
  const label = options.label ? `${options.label} · ` : "";
  const totalDamage = perTurn.reduce((sum, turn) => sum + turn.total, 0);
  // 累计趋势线：从 0 起逐回合累计，直线延伸到配置终点回合（含缺失回合的
  // 平台段），不提前停在最后一个有伤害的回合。
  const cumulativePoints: string[] = [];
  let runningTotal = 0;
  for (const turn of filled) {
    runningTotal += turn.total;
    const x = (turn.round - 1) * slot + slot / 2;
    cumulativePoints.push(`${x.toFixed(2)},${yFor(runningTotal).toFixed(2)}`);
  }
  const baseY = yFor(0).toFixed(2);
  const areaPoints = [`0,${baseY}`, ...cumulativePoints, `${CHART_W},${baseY}`].join(" ");
  return `
    <div class="stacked-chart ${options.scopeClass ? `scope-${options.scopeClass}` : ""}">
      ${options.summaryItems ? renderUsageSummary(options.summaryItems) : ""}
      ${renderChartHead(
        options.label ? `${options.label} · 每回合伤害（按卡牌来源）` : "每回合伤害（按卡牌来源）",
        renderStackedChartNote(totalDamage, options),
      )}
      <div class="stacked-chart-body">
        <svg class="stacked-chart-svg" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" role="img" aria-label="${escapeAttribute(options.label ? `${options.label}每回合伤害与累计趋势` : "每回合伤害与累计趋势")}，显示至 R${roundCount}">
          ${options.threshold !== null && options.threshold !== undefined
            ? `<line class="threshold-line" x1="0" y1="${yFor(options.threshold)}" x2="${CHART_W}" y2="${yFor(options.threshold)}"></line>`
            : ""}
          ${filled.map((turn) => {
            let cumulative = 0;
            const x = (turn.round - 1) * slot + (slot - barWidth) / 2;
            const selected = options.selectedRound === turn.round;
            if (turn.byCard.length === 0) {
              // 零伤害回合：淡色占位柱保持 x 轴连续，选中时同样可见。
              return `
                <g class="round-bar${selected ? " selected" : ""}" data-round="${turn.round}">
                  <rect
                    class="bar-empty${selected ? " selected" : ""}"
                    x="${x.toFixed(2)}"
                    y="${CHART_PAD_TOP}"
                    width="${barWidth.toFixed(2)}"
                    height="${(CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM).toFixed(2)}"
                    data-round="${turn.round}"
                  ></rect>
                </g>
              `;
            }
            return `
              <g class="round-bar${selected ? " selected" : ""}" data-round="${turn.round}">
                ${turn.byCard.map((entry) => {
                  const key = cardPaletteKey(entry.cardId);
                  const height = (entry.damage / maxY) * (CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM);
                  const y = yFor(cumulative + entry.damage);
                  cumulative += entry.damage;
                  return `
                    <rect
                      class="bar-seg ${palette.get(key) ?? "card-other"}${selected ? " selected" : ""}"
                      x="${x.toFixed(2)}"
                      y="${y.toFixed(2)}"
                      width="${barWidth.toFixed(2)}"
                      height="${Math.max(0.4, height).toFixed(2)}"
                      data-round="${turn.round}"
                      data-card="${escapeAttribute(key)}"
                      title="${escapeAttribute(`${label}第 ${turn.round} 回合 · ${entry.cardName ?? "持续/其他"} ${entry.damage} 伤`)}"
                    ></rect>
                  `;
                }).join("")}
              </g>
            `;
          }).join("")}
          <g class="cumulative-trend" aria-hidden="true">
            <polygon class="cumulative-area" points="${escapeAttribute(areaPoints)}"></polygon>
            <polyline class="cumulative-trend-line" points="${escapeAttribute(cumulativePoints.join(" "))}"></polyline>
          </g>
        </svg>
        ${renderRoundTicks(roundCount)}
      </div>
      ${renderCardLegend(perTurn, palette)}
    </div>
  `;
}

function renderStackedChartNote(
  totalDamage: number,
  options: StackedDamageChartOptions,
): string | undefined {
  if (options.threshold !== null && options.threshold !== undefined) {
    return `阈值 ${options.threshold} 伤 · 累计 ${totalDamage} 伤`;
  }
  if (options.scopeClass === "flow") return `累计 ${totalDamage} 伤`;
  return undefined;
}

function renderChartHead(title: string, note: string | undefined): string {
  return `
    <header class="stacked-chart-head">
      <span>${escapeHtml(title)}</span>
      ${note ? `<span class="stacked-chart-note">${escapeHtml(note)}</span>` : ""}
    </header>
  `;
}

function renderRoundTicks(roundCount: number): string {
  const step = Math.max(1, Math.ceil(roundCount / 8));
  const ticks: number[] = [];
  for (let round = 1; round <= roundCount; round += step) ticks.push(round);
  if (ticks.at(-1) !== roundCount) ticks.push(roundCount);
  return `
    <div class="round-ticks" aria-hidden="true">
      ${ticks.map((round) => `
        <span class="round-tick" style="left:${(((round - 1) / Math.max(1, roundCount - 1)) * 100).toFixed(2)}%">R${round}</span>
      `).join("")}
    </div>
  `;
}

/**
 * 卡牌图例：按调色板键（baseId）聚合。同一键下可能出现多个变体名（同一张卡
 * 的不同阶位），每个变体先按整牌 id 累计**真实总伤害**，再取累计最高的变体名，
 * 与 `cardPaletteKey` 的 baseId 口径一致。比较的是变体累计而非单条 entry。
 */
function renderCardLegend(perTurn: readonly TargetTurnDamage[], palette: ReadonlyMap<string, string>): string {
  const variantTotals = new Map<string, { cardId: number | null; name: string; damage: number }>();
  for (const turn of perTurn) {
    for (const entry of turn.byCard) {
      const variantKey = `${cardPaletteKey(entry.cardId)}|${entry.cardId ?? "null"}`;
      const existing = variantTotals.get(variantKey);
      variantTotals.set(variantKey, {
        cardId: entry.cardId,
        name: entry.cardName ?? "持续/其他",
        damage: (existing?.damage ?? 0) + entry.damage,
      });
    }
  }
  const byKey = new Map<string, { key: string; name: string; damage: number }>();
  for (const variant of variantTotals.values()) {
    const key = cardPaletteKey(variant.cardId);
    const existing = byKey.get(key);
    // 同伤害并列时保留先出现的名字（稳定）。
    if (!existing || variant.damage > existing.damage) {
      byKey.set(key, { key, name: variant.name, damage: variant.damage });
    }
  }
  if (byKey.size === 0) return "";
  return `
    <div class="card-legend" aria-label="卡牌颜色图例">
      ${[...byKey.values()].map((entry) => `
        <span class="legend-item">
          <span class="legend-swatch ${palette.get(entry.key) ?? "card-other"}"></span>${escapeHtml(entry.name)}
        </span>
      `).join("")}
    </div>
  `;
}

function renderExpandedStepDetail(state: AppState, active: TargetBuild | undefined): string {
  const target = state.target!;
  if (!active?.result || target.expandedStepBuildId !== active.id || target.expandedStep === null) return "";
  const step = active.result.steps[target.expandedStep];
  if (!step) return "";
  return `
    <div class="target-round-detail" aria-label="第 ${step.round} 回合 · ${step.cardName ?? "持续/其他"} 伤害明细">
      <div class="target-round-detail-head">
        <span>第 ${step.round} 回合 · ${escapeHtml(step.cardName ?? "持续/其他")} · +${step.damage}（累计 ${step.cumulative}）</span>
        <button type="button" class="build-action" data-action="toggle-target-step" data-step="${target.expandedStep}" data-build-id="${escapeAttribute(active.id)}" title="收起明细">收起</button>
      </div>
      <ul>
        <li>
          <span class="legend-swatch ${cardPalette([step.cardId]).get(cardPaletteKey(step.cardId)) ?? "card-other"}"></span>
          <span>${escapeHtml(step.cardName ?? "持续/其他")}</span>
          <b>${step.damage} 伤</b>
          <span class="round-detail-pct">累计 ${step.cumulative}</span>
        </li>
      </ul>
    </div>
  `;
}

/**
 * 阶梯式累计伤害曲线（打靶模式专用）。
 * x 轴 = 出牌事件序列（每张牌/每次结算伤害一个台阶），y 轴 = 累计伤害（23→65→96→133）。
 * 每个台阶的上升段按该步归因的卡牌着色（无卡归因如内伤/持续 → 「持续/其他」色），
 * 台阶高度 = 该步伤害增量；回合边界用更粗的垂直分隔线标记，同回合多张牌是中间小台阶。
 * hover 一个台阶显示该步详情（回合 · 卡名 · 增量 · 累计）。阈值 120 画水平线。
 */
export function renderDamageStepChart(
  steps: readonly TargetDamageStep[],
  palette: ReadonlyMap<string, string>,
  options: { label?: string; threshold?: number; selectedStep?: number; displayRounds?: number; buildId?: string } = {},
): string {
  const n = steps.length;
  const displayRounds = Math.max(1, Math.trunc(options.displayRounds ?? 0));
  if (n === 0) {
    const emptyRoundCount = Math.max(1, displayRounds);
    return `
      <div class="stacked-chart step-chart">
        ${renderChartHead(options.label ? `${options.label} · 累计伤害` : "累计伤害", undefined)}
        <div class="stacked-chart-body">
          <svg class="stacked-chart-svg" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" role="img" aria-label="累计伤害：未产生伤害"></svg>
          <div class="stacked-chart-empty">未产生伤害${options.displayRounds ? ` · 显示至 R${emptyRoundCount}` : ""}</div>
        </div>
      </div>
    `;
  }
  const maxRoundInSteps = steps.reduce((max, step) => Math.max(max, step.round), 0);
  const roundCount = Math.max(displayRounds, maxRoundInSteps, 1);
  const maxStepCumulative = steps.reduce((max, step) => Math.max(max, step.cumulative), 0);
  const maxY = Math.max(options.threshold ?? 0, maxStepCumulative, 1);
  const padX = 4;
  const plotW = CHART_W - padX * 2;
  // Keep event positions in their round slots and reserve zero-damage tail slots
  // through displayRounds, so R5/R6 remains visible after an R4 threshold hit.
  const slotW = plotW / Math.max(1, roundCount);
  const stepCounts = new Map<number, number>();
  for (const step of steps) stepCounts.set(step.round, (stepCounts.get(step.round) ?? 0) + 1);
  const stepSeen = new Map<number, number>();
  const stepX = steps.map((step) => {
    const ordinal = stepSeen.get(step.round) ?? 0;
    stepSeen.set(step.round, ordinal + 1);
    const count = stepCounts.get(step.round) ?? 1;
    return padX + (step.round - 1) * slotW + (slotW * (ordinal + 1)) / (count + 1);
  });
  const xFor = (index: number): number => stepX[index] ?? padX;
  const xForRound = (round: number): number => padX + (round - 1) * slotW + slotW * 0.5;
  const plotRight = padX + plotW;
  const yFor = (value: number): number =>
    CHART_PAD_TOP + (CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM) * (1 - value / maxY);
  const label = options.label ? `${options.label} · ` : "";

  // 阶梯：每步先竖直上升到累计值，再水平平台到下一步。上升段按卡色填充。
  const segments: string[] = [];
  const hotspots: string[] = [];
  let prevY = yFor(0);
  for (let index = 0; index < n; index += 1) {
    const step = steps[index]!;
    const x = xFor(index);
    const safeCumulative = Math.max(0, step.cumulative);
    const y = yFor(safeCumulative);
    const key = cardPaletteKey(step.cardId);
    const colorClass = palette.get(key) ?? "card-other";
    const nextX = index + 1 < n ? xFor(index + 1) : plotRight;
    // y 轴倒置（yFor：累计越大 y 越小）。正常出牌伤害增量为正 → y < prevY。
    // y > prevY 表示累计下降（伤害增量不可能为负），防御性跳过。
    if (y > prevY) {
      segments.push(`<!-- 累计下降异常 step ${index} -->`);
    } else {
      // 上升段（竖直矩形，从 prevY 到 y，宽度做成可见的色块）
      const riseHeight = Math.max(0.4, prevY - y);
      segments.push(
        `<rect class="step-rise ${colorClass}" x="${(x - 0.6).toFixed(2)}" y="${y.toFixed(2)}" width="1.6" height="${riseHeight.toFixed(2)}"></rect>`,
      );
    }
    // 水平平台（从该步 x 到下一步 x 的累计值水平线）
    if (index + 1 < n) {
      segments.push(
        `<line class="step-plateau" x1="${x.toFixed(2)}" y1="${y.toFixed(2)}" x2="${nextX.toFixed(2)}" y2="${y.toFixed(2)}"></line>`,
      );
    }
    // 回合边界：该步是某回合的第一步（前一步不同回合）→ 画垂直分隔线
    const prev = index > 0 ? steps[index - 1] : null;
    if (!prev || prev.round !== step.round) {
      segments.push(
        `<line class="step-round-boundary" x1="${x.toFixed(2)}" y1="${CHART_PAD_TOP}" x2="${x.toFixed(2)}" y2="${(CHART_H - CHART_PAD_BOTTOM).toFixed(2)}"></line>`,
      );
    }
    // hover 热点：覆盖该步到下一步的区域，透明可点
    const detail = `${label}第 ${step.round} 回合 · ${step.cardName ?? "持续/其他"} +${step.damage}（累计 ${safeCumulative}）`;
    hotspots.push(
      `<rect class="step-hotspot${options.selectedStep === index ? " selected" : ""}" x="${x.toFixed(2)}" y="${CHART_PAD_TOP}" width="${Math.max(2, nextX - x).toFixed(2)}" height="${(CHART_H - CHART_PAD_TOP - CHART_PAD_BOTTOM).toFixed(2)}" data-step="${index}" data-build-id="${escapeAttribute(options.buildId ?? "")}" data-action="toggle-target-step" tabindex="0" role="button" aria-label="${escapeAttribute(detail)}" title="${escapeAttribute(detail)}"></rect>`,
    );
    prevY = y;
  }
  // 末点标记
  const lastStep = steps.at(-1)!;
  const lastX = xFor(n - 1);
  const lastY = yFor(Math.max(0, lastStep.cumulative));
  segments.push(`<circle class="step-endpoint" cx="${lastX.toFixed(2)}" cy="${lastY.toFixed(2)}" r="1.6"></circle>`);

  // 回合刻度覆盖实际台阶与 displayRounds 尾部。
  const roundLabels: string[] = [];
  const tickStep = Math.max(1, Math.ceil(roundCount / 8));
  for (let round = 1; round <= roundCount; round += tickStep) {
    const pct = ((xForRound(round) - padX) / plotW) * 100;
    roundLabels.push(`<span class="round-tick" style="left:${pct.toFixed(2)}%">R${round}</span>`);
  }
  if (roundLabels.length === 0 || !roundLabels.at(-1)?.includes(`>R${roundCount}<`)) {
    const pct = ((xForRound(roundCount) - padX) / plotW) * 100;
    roundLabels.push(`<span class="round-tick" style="left:${pct.toFixed(2)}%">R${roundCount}</span>`);
  }

  return `
    <div class="stacked-chart step-chart">
      ${renderChartHead(
        options.label ? `${options.label} · 累计伤害（按出牌台阶）` : "累计伤害（按出牌台阶）",
        options.threshold === undefined ? undefined : `阈值 ${options.threshold} 伤`,
      )}
      <div class="stacked-chart-body">
        <svg class="stacked-chart-svg" viewBox="0 0 ${CHART_W} ${CHART_H}" preserveAspectRatio="none" role="img" aria-label="${escapeAttribute(options.label ? `${options.label}累计伤害阶梯曲线` : "累计伤害阶梯曲线")}，显示至 R${roundCount}">
          ${options.threshold !== undefined && options.threshold !== null
            ? `<line class="threshold-line" x1="0" y1="${yFor(options.threshold)}" x2="${CHART_W}" y2="${yFor(options.threshold)}"></line>`
            : ""}
          ${segments.join("")}
          ${hotspots.join("")}
        </svg>
        <div class="round-ticks" aria-hidden="true">${roundLabels.join("")}</div>
      </div>
      ${renderCardLegend(fromSteps(steps), palette)}
    </div>
  `;
}

/** 从步骤序列派生卡牌 id 列表（图例调色板用）。 */
function fromSteps(steps: readonly TargetDamageStep[]): readonly TargetTurnDamage[] {
  const byRound = new Map<number, Map<string, TargetCardDamage>>();
  for (const step of steps) {
    const cards = byRound.get(step.round) ?? new Map<string, TargetCardDamage>();
    const key = cardPaletteKey(step.cardId);
    const existing = cards.get(key);
    cards.set(key, {
      cardId: step.cardId,
      cardName: step.cardName,
      damage: (existing?.damage ?? 0) + step.damage,
    });
    byRound.set(step.round, cards);
  }
  return [...byRound.entries()]
    .sort(([left], [right]) => left - right)
    .map(([round, cards]) => ({
      round,
      total: [...cards.values()].reduce((sum, entry) => sum + entry.damage, 0),
      byCard: [...cards.values()],
    }));
}

function maxRound(perTurn: readonly TargetTurnDamage[]): number {
  return perTurn.reduce((max, turn) => Math.max(max, turn.round), 0);
}

/** 补齐缺失回合（0 伤柱），保证 x 轴连续。 */
function fillRounds(perTurn: readonly TargetTurnDamage[], roundCount: number): readonly TargetTurnDamage[] {
  const byRound = new Map(perTurn.map((turn) => [turn.round, turn] as const));
  const filled: TargetTurnDamage[] = [];
  for (let round = 1; round <= roundCount; round += 1) {
    const turn = byRound.get(round);
    filled.push(turn ?? { round, total: 0, byCard: [] });
  }
  return filled;
}
