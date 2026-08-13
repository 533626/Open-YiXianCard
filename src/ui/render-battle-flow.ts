import { timelinePoints } from "./render-battle-progress";
import { battleRound } from "./render-battle-progress";
import { renderStackedDamageChart } from "./render-target-chart";
import { cardPalette, computeDamagePerTurn } from "./target-practice-metrics";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { TimelinePoint } from "./render-battle-progress";
import type { HookStep } from "./hook-trace";
import type { BattleFrame, FlowMetric, Side, TargetTurnDamage } from "./types";

/**
 * 生命曲线 / 伤害曲线（模块头部 toggle 切换）。
 *
 * x 轴的一个点 = 时间轴上的一个点 = 一方的一次完整行动，所以逐动前后翻的时候
 * 游标正好一格一格走。
 *
 * 伤害曲线用 yixian_trace_json 的钩子链做**精确归因**（与打靶模式同一套
 * computeDamagePerTurn）：按回合（双方各动一次）聚合，柱内分段 = 各卡贡献。
 * 曾经的「每动 hp+防 差分估计」折线在动作方为 0 的点上必然一轮高一轮 0，
 * 已由回合聚合堆叠柱替换。
 */
type FlowMetricRow = {
  readonly key: string;
  readonly label: string;
  readonly values: readonly number[];
  readonly current: number;
  readonly title: string;
  /**
   * `floor` 把 0 画在底部：生命和伤害都是非负量，围着中线画会白掉下半幅。
   * 差分行用 `zero` 围中线画，正负一眼可见。
   */
  readonly scale: { readonly mode: "floor" | "zero"; readonly max: number };
};

const FLOW_WIDTH = 100;
const FLOW_HEIGHT = 56;
const FLOW_ZERO_Y = 28;
const FLOW_AMPLITUDE = 22;
const FLOW_CURSOR_TOP = 4;
const FLOW_CURSOR_BOTTOM = 52;

export function renderResourceFlow(
  frames: readonly BattleFrame[],
  selectedFrameIndex: number,
  metric: FlowMetric = "life",
  hookSteps?: readonly HookStep[],
): string {
  // 没有任何行动帧时不画曲线：只有阶段帧的时间轴画出来是一条无意义的直线。
  if (!frames.some((frame) => frame.actionIndex !== null)) return "";
  const points = timelinePoints(frames);
  if (points.length === 0) return "";
  const selectedIndex = nearestPointIndex(points, selectedFrameIndex);
  const damage = metric === "damage";
  if (damage) return renderDamageFlow(frames, selectedFrameIndex, hookSteps);
  const metrics = lifeMetrics(points, selectedIndex);
  return `
    <section class="battle-module resource-flow" aria-label="生命曲线">
      <header class="resource-flow-head">
        <span>双方生命与生命差</span>
      </header>
      <div class="resource-flow-chart">
        ${metrics.map((row) => renderMetricRow(row, selectedIndex)).join("")}
      </div>
    </section>
  `;
}

/** 伤害曲线：按回合聚合的堆叠柱状图（每段 = 一张卡的贡献）。 */
function renderDamageFlow(
  frames: readonly BattleFrame[],
  selectedFrameIndex: number,
  hookSteps: readonly HookStep[] | undefined,
): string {
  const first = frames[0]!;
  const sides: readonly { side: Side; name: string }[] = [
    { side: "p1", name: first.players.p1.name },
    { side: "p2", name: first.players.p2.name },
  ];
  const selectedFrame = frames[selectedFrameIndex] ?? first;
  const selectedRound = battleRound(selectedFrame.actorTurn);
  if (!hookSteps) {
    // 归因完全依赖 trace：没有钩子链就明示不可用，不画会误导成「0 伤」的空图。
    return `
      <section class="battle-module resource-flow" aria-label="伤害曲线">
        <header class="resource-flow-head">
          <span>每回合伤害（按卡牌来源）</span>
        </header>
        <div class="resource-flow-unavailable" role="status">
          钩子链不可用：本次战斗未采集到 yixian_trace_json，无法按卡牌归因伤害。
        </div>
      </section>
    `;
  }
  return `
    <section class="battle-module resource-flow" aria-label="伤害曲线">
      <header class="resource-flow-head">
        <span>每回合伤害（按卡牌来源）</span>
      </header>
      <div class="resource-flow-chart">
        ${sides.map(({ side, name }) => {
          const perTurn = computeDamagePerTurn(hookSteps, side);
          const roundTotal = perTurn.find((turn) => turn.round === selectedRound)?.total ?? 0;
          const cumulative = perTurn.reduce((sum, turn) => sum + turn.total, 0);
          return renderStackedDamageChart(perTurn, cardPalette(cardIdsOf(perTurn)), {
            label: name,
            selectedRound,
            minRounds: selectedRound,
            scopeClass: "flow",
            summaryItems: [
              {
                key: `${side}-round`,
                label: "本回合伤害",
                value: String(roundTotal),
                detail: `${name} 第 ${selectedRound} 回合对对方造成的伤害（trace 精确归因）`,
                tone: "warm",
              },
              {
                key: `${side}-total`,
                label: "累计伤害",
                value: String(cumulative),
                detail: `${name} 全场累计伤害（截至最后有伤害的回合）`,
              },
            ],
          });
        }).join("")}
      </div>
    </section>
  `;
}

function cardIdsOf(perTurn: readonly TargetTurnDamage[]): readonly (number | null)[] {
  return perTurn.flatMap((turn) => turn.byCard.map((entry) => entry.cardId));
}

function lifeMetrics(
  points: readonly TimelinePoint[],
  selectedIndex: number,
): readonly FlowMetricRow[] {
  const hpP1 = points.map((point) => point.jumpFrame.players.p1.hp);
  const hpP2 = points.map((point) => point.jumpFrame.players.p2.hp);
  // 两条生命曲线共用上界，否则各自归一化后"谁掉得更狠"完全看不出来。
  const hpMax = Math.max(1, ...hpP1, ...hpP2);
  const diff = hpP1.map((value, index) => value - hpP2[index]!);
  return [
    metric("hp-p1", "玩家一生命", hpP1, selectedIndex, "Rust canonical 帧中的玩家一生命原值（与玩家二共用纵轴上界）", { mode: "floor", max: hpMax }),
    metric("hp-p2", "玩家二生命", hpP2, selectedIndex, "Rust canonical 帧中的玩家二生命原值（与玩家一共用纵轴上界）", { mode: "floor", max: hpMax }),
    metric(
      "hp",
      "生命差",
      diff,
      selectedIndex,
      "派生差值：玩家一生命 - 玩家二生命，正值表示玩家一领先",
      { mode: "zero", max: Math.max(1, ...diff.map(Math.abs)) },
    ),
  ];
}

export function resourceFlowFrames(frames: readonly BattleFrame[]): readonly BattleFrame[] {
  return frames.filter((frame, index, all) =>
    all.findIndex((candidate) => candidate.index === frame.index) === index
  );
}

function metric(
  key: string,
  label: string,
  values: readonly number[],
  selectedIndex: number,
  title: string,
  scale: FlowMetricRow["scale"],
): FlowMetricRow {
  return {
    key,
    label,
    values,
    current: values[selectedIndex] ?? 0,
    title,
    scale,
  };
}

function renderMetricRow(metric: FlowMetricRow, selectedIndex: number): string {
  const selectedX = xForIndex(selectedIndex, metric.values.length);
  const selectedY = yForValue(metric.values[selectedIndex] ?? 0, metric.scale);
  return `
    <div class="flow-metric ${escapeAttribute(metric.key)}" title="${escapeAttribute(metric.title)}">
      <span class="flow-label">${escapeHtml(metric.label)}</span>
      <b class="flow-value ${metric.scale.mode === "zero" ? valueClass(metric.current) : "fact"}">${
    metric.scale.mode === "zero" ? formatSigned(metric.current) : String(metric.current)
  }</b>
      <svg class="flow-spark" viewBox="0 0 ${FLOW_WIDTH} ${FLOW_HEIGHT}" preserveAspectRatio="none" aria-hidden="true">
        ${
    metric.scale.mode === "zero"
      ? `<line class="flow-zero" x1="0" y1="${FLOW_ZERO_Y}" x2="${FLOW_WIDTH}" y2="${FLOW_ZERO_Y}"></line>`
      : ""
  }
        <polygon class="flow-area" points="${escapeAttribute(areaPolygon(metric))}"></polygon>
        <polyline class="flow-line" points="${escapeAttribute(polyline(metric))}"></polyline>
        <line class="flow-cursor" x1="${selectedX}" y1="${FLOW_CURSOR_TOP}" x2="${selectedX}" y2="${FLOW_CURSOR_BOTTOM}"></line>
        <circle class="flow-marker" cx="${selectedX}" cy="${selectedY}" r="2.4"></circle>
      </svg>
    </div>
  `;
}

function nearestPointIndex(
  points: readonly TimelinePoint[],
  selectedFrameIndex: number,
): number {
  const exact = points.findIndex((point) =>
    point.frames.some((frame) => frame.index === selectedFrameIndex)
  );
  if (exact >= 0) return exact;
  for (let index = points.length - 1; index >= 0; index -= 1) {
    if (points[index]!.jumpFrame.index <= selectedFrameIndex) return index;
  }
  return 0;
}

function polyline(metric: FlowMetricRow): string {
  return flowPoints(metric).join(" ");
}

function areaPolygon(metric: FlowMetricRow): string {
  const points = flowPoints(metric);
  if (points.length === 0) return "";
  const base = metric.scale.mode === "floor" ? FLOW_CURSOR_BOTTOM : FLOW_ZERO_Y;
  return [`0,${base}`, ...points, `${FLOW_WIDTH},${base}`].join(" ");
}

function flowPoints(metric: FlowMetricRow): readonly string[] {
  const values = metric.values;
  if (values.length === 0) return [];
  if (values.length === 1) {
    const y = yForValue(values[0]!, metric.scale);
    return [`0,${y}`, `${FLOW_WIDTH},${y}`];
  }
  return values
    .map((value, index) => `${xForIndex(index, values.length)},${yForValue(value, metric.scale)}`);
}

function xForIndex(index: number, count: number): string {
  if (count <= 1) return formatCoord(FLOW_WIDTH / 2);
  return formatCoord((index / (count - 1)) * FLOW_WIDTH);
}

function yForValue(value: number, scale: FlowMetricRow["scale"]): string {
  if (scale.mode === "floor") {
    const normalized = Math.max(0, Math.min(1, value / scale.max));
    return formatCoord(
      FLOW_CURSOR_BOTTOM - normalized * (FLOW_CURSOR_BOTTOM - FLOW_CURSOR_TOP),
    );
  }
  const normalized = Math.max(-1, Math.min(1, value / scale.max));
  return formatCoord(FLOW_ZERO_Y - normalized * FLOW_AMPLITUDE);
}

function formatCoord(value: number): string {
  return Number.isInteger(value) ? String(value) : value.toFixed(2);
}

function valueClass(value: number): string {
  if (value > 0) return "positive";
  if (value < 0) return "negative";
  return "neutral";
}

function formatSigned(value: number): string {
  if (value > 0) return `+${value}`;
  return String(value);
}
