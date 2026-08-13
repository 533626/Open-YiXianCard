import {
  CARD_CONFIG_BY_ID,
  CARD_OPTION_BY_BASE_ID,
  describeTalent,
} from "./data";
import { normalizeBaseId } from "./domain";
import {
  solverModeLabel,
  solverTaskForMode,
  type SolverUiMode,
  type SolverUiTask,
} from "./solver-ui";
import { escapeAttribute, escapeHtml } from "./view-utils";
import {
  classifyValueRankComparison,
  sortDeckResults,
  type ValueRankComparison,
} from "./solver-contract";
import type { AppState } from "./types";

export type SolverResult = NonNullable<AppState["solverResult"]>;
export type SolverItem = SolverResult["results"][number];
export type SolverBaseline = SolverResult["baseline"];
export type SolverCard = SolverResult["baselineDeck"][number];

interface SolverSourceCard {
  readonly number: number;
  readonly id: number;
  readonly name: string;
  readonly origin: "field" | "hand" | "pool";
}

interface SolverSourceContext {
  readonly task: SolverUiTask;
  readonly field: readonly SolverSourceCard[];
  readonly hand: readonly SolverSourceCard[];
  readonly pool: readonly SolverSourceCard[];
}

interface SolverOrderPresentation {
  readonly text: string;
  readonly tokens: readonly string[];
  readonly usedHand: readonly SolverSourceCard[];
  readonly unusedHand: readonly SolverSourceCard[];
  readonly usedPool: readonly SolverSourceCard[];
}

export function solverHandCardName(id: number): string {
  return CARD_CONFIG_BY_ID.get(id)?.name ??
    CARD_OPTION_BY_BASE_ID.get(normalizeBaseId(id))?.name ??
    `#${id}`;
}

function displaySolverCardName(card: { readonly id: number; readonly name: string }): string {
  return card.name.startsWith("card:") ? solverHandCardName(card.id) : card.name;
}
export function renderSolverResult(
  result: SolverResult,
  uiMode: SolverUiMode | undefined,
  state: AppState,
): string {
  const baseline = result.baseline;
  const top = result.results[0];
  const task = solverResultTask(result, uiMode);
  const sources = solverSourceContext(result, state, task);
  const hpDeltaBest = bestResultByHpDelta(result.results);
  const hpDeltaBestDeckKey = top && hpDeltaBest && hpDeltaBest.deckKey !== top.deckKey
    ? hpDeltaBest.deckKey
    : undefined;
  return `
    <div class="solver-result-popover">
      <div class="solver-summary">
        <span>${escapeHtml(uiMode ? solverModeLabel(uiMode) : modeLabel(result.mode))}</span>
        <span>${escapeHtml(confidenceLabel(result.confidence))}</span>
        <span>评估 ${result.evaluatedCount}</span>
        ${result.skippedDuplicateCount === 0 ? "" : `<span>跳重 ${result.skippedDuplicateCount}</span>`}
        <span>候选牌 ${result.candidateCardCount}</span>
        ${result.candidateTalentCount === undefined ? "" : `<span>候选仙命 ${result.candidateTalentCount}</span>`}
        ${result.usedSyntheticDecisions ? `<span class="solver-synthetic-audit">含合成随机判定（seed ${escapeHtml((result.syntheticDecisionSeedsUsed ?? result.seedsUsed ?? []).join(","))}），非原版决策</span>` : ""}
      </div>
      ${top ? renderSolverWhy(top, baseline, hpDeltaBest) : ""}
      <div class="solver-list-title"><b>候选比较</b><span>${escapeHtml(solverSourceLegend(sources))}</span></div>
      <div class="solver-results">
        ${top
          ? solverDisplayRows(result).map((row) =>
            row.kind === "baseline"
              ? renderSolverBaseline(result, sources)
              : renderSolverCandidateRow(row.item, result, baseline, sources, hpDeltaBestDeckKey)
          ).join("")
          : `<div class="solver-empty">没有找到可展示的建议。请先处理卡组诊断问题，或提高搜索预算。</div>`}
      </div>
      ${result.results[0]?.talentChanges?.length ? `
        <div class="solver-talents">
          <b>仙命构筑</b>
          ${result.results[0].talentChanges.map((change) => `
            <span>${change.slot}: ${escapeHtml(formatTalent(change.from))} -> ${escapeHtml(formatTalent(change.to))}</span>
          `).join("")}
        </div>
      ` : ""}
      ${result.marginalChanges?.length ? `
        <div class="solver-marginal">
          <b>单卡边际收益</b>
          ${result.marginalChanges.slice(0, 3).map((change) => `
            <span>槽位 ${change.slot}：换出 ${escapeHtml(change.from.name)} / 换入 ${escapeHtml(change.to.name)} · 边际收益 ${formatSigned(change.gain)}</span>
          `).join("")}
        </div>
      ` : ""}
    </div>
  `;
}

type SolverDisplayRow =
  | { readonly kind: "baseline" }
  | { readonly kind: "candidate"; readonly item: SolverItem };

function solverDisplayRows(result: SolverResult): readonly SolverDisplayRow[] {
  const baseline = {
    row: { kind: "baseline" } as const,
    evaluation: result.baseline,
    deckKey: `baseline:${result.baselineDeck.map((card) => card.id).join(",")}`,
  };
  const insertionIndex = result.results.findIndex((item) => {
    const candidate = {
      row: { kind: "candidate", item } as const,
      evaluation: item.evaluation,
      deckKey: item.deckKey,
    };
    return sortDeckResults([candidate, baseline], 2)[0]?.row.kind === "baseline";
  });
  const index = insertionIndex === -1 ? result.results.length : insertionIndex;
  return [
    ...result.results.slice(0, index).map((item) => ({ kind: "candidate", item } as const)),
    { kind: "baseline" },
    ...result.results.slice(index).map((item) => ({ kind: "candidate", item } as const)),
  ];
}

function renderSolverCandidateRow(
  item: SolverItem,
  result: SolverResult,
  baseline: SolverBaseline,
  sources: SolverSourceContext,
  hpDeltaBestDeckKey?: string,
): string {
  const order = solverOrderPresentation(item, sources);
  return `
    <button
      type="button"
      class="solver-row ${item.evaluation.winForSide ? "is-win" : "is-loss"}"
      data-action="apply-solver-row"
      data-deck-key="${escapeAttribute(item.deckKey)}"
      title="${escapeAttribute(`点击写回当前构筑>>>${solverRowTitle(item, baseline, sources, order, hpDeltaBestDeckKey)}`)}"
    >
      <b class="solver-rank">第${item.rank}</b>
      <span class="solver-badge">${escapeHtml(resultBadge(item, baseline, hpDeltaBestDeckKey))}</span>
      <code class="solver-order-digits" aria-label="牌序 ${escapeAttribute(order.tokens.join(" "))}">${escapeHtml(order.text)}</code>
      <strong class="solver-outcome">${item.evaluation.winForSide ? "胜" : "负"}</strong>
      <span class="solver-row-metric">hpDelta <b>${formatSigned(item.evaluation.hpDeltaForSide)}</b></span>
      <span class="solver-row-metric">actorTurn <b>${item.evaluation.actorTurn}</b></span>
      <span class="solver-row-change">${escapeHtml(formatSourceChangeCount(item, sources, order))}</span>
    </button>
  `;
}

function renderSolverBaseline(result: SolverResult, sources: SolverSourceContext): string {
  const digits = result.baselineDeck.map((_, index) => index + 1).join("");
  return `
    <button
      type="button"
      class="solver-baseline"
      data-action="apply-solver-baseline"
      title="${escapeAttribute(`点击回退到求解基准构筑>>>${solverSourceTitle(sources)}`)}"
    >
      <span aria-hidden="true"></span>
      <b>基准</b>
      <code>${escapeHtml(digits || "—")}</code>
      <em>${result.baseline.winForSide ? "胜" : "负"} · hpDelta ${formatSigned(result.baseline.hpDeltaForSide)} · actorTurn ${result.baseline.actorTurn}</em>
      <span aria-hidden="true"></span>
    </button>
  `;
}

function solverResultTask(result: SolverResult, uiMode?: SolverUiMode): SolverUiTask {
  if (uiMode) return solverTaskForMode(uiMode);
  if (result.mode === "hand") return "hand";
  if (result.mode === "beam") return "pool";
  return "order";
}

function solverSourceContext(
  result: SolverResult,
  state: AppState,
  task: SolverUiTask,
): SolverSourceContext {
  const field = result.baselineDeck.map((card, index) =>
    solverSourceCard(card, index + 1, "field")
  );
  const hand = task === "hand"
    ? state.config.players[result.side].handCardIds.map((id, index) => ({
        number: field.length + index + 1,
        id,
        name: solverHandCardName(id),
        origin: "hand" as const,
      }))
    : [];
  const pool = task === "pool"
    ? solverPoolSources(result, field.length + 1)
    : [];
  return { task, field, hand, pool };
}

function solverSourceCard(
  card: SolverCard,
  number: number,
  origin: SolverSourceCard["origin"],
): SolverSourceCard {
  return { number, id: card.id, name: card.name, origin };
}

function solverPoolSources(
  result: SolverResult,
  firstNumber: number,
): readonly SolverSourceCard[] {
  const baselineCounts = cardCounts(result.baselineDeck.map((card) => card.id));
  const maxExtraCounts = new Map<number, number>();
  const exemplarById = new Map<number, SolverCard>();
  const firstSeen: number[] = [];
  for (const item of result.results) {
    const candidateCounts = cardCounts(item.deck.map((card) => card.id));
    for (const card of item.deck) {
      const extraCount = Math.max(
        0,
        (candidateCounts.get(card.id) ?? 0) - (baselineCounts.get(card.id) ?? 0),
      );
      if (extraCount <= (maxExtraCounts.get(card.id) ?? 0)) continue;
      if (!exemplarById.has(card.id)) firstSeen.push(card.id);
      exemplarById.set(card.id, card);
      maxExtraCounts.set(card.id, extraCount);
    }
  }
  const sources: SolverSourceCard[] = [];
  for (const id of firstSeen) {
    const card = exemplarById.get(id)!;
    for (let occurrence = 0; occurrence < (maxExtraCounts.get(id) ?? 0); occurrence += 1) {
      sources.push(solverSourceCard(card, firstNumber + sources.length, "pool"));
    }
  }
  return sources;
}

function cardCounts(ids: readonly number[]): Map<number, number> {
  const counts = new Map<number, number>();
  for (const id of ids) counts.set(id, (counts.get(id) ?? 0) + 1);
  return counts;
}

function solverOrderPresentation(
  item: SolverItem,
  sources: SolverSourceContext,
): SolverOrderPresentation {
  const leftoverCounts = sources.task === "hand"
    ? cardCounts(item.leftoverHandCardIds)
    : new Map<number, number>();
  const unusedHand: SolverSourceCard[] = [];
  const usedHand = sources.hand.filter((source) => {
    const count = leftoverCounts.get(source.id) ?? 0;
    if (count === 0) return true;
    leftoverCounts.set(source.id, count - 1);
    unusedHand.push(source);
    return false;
  });
  const available = [
    ...sources.field,
    ...(sources.task === "hand" ? usedHand : sources.pool),
  ];
  const usedSourceNumbers = new Set<number>();
  const mappedSources = item.deck.map((card) => {
    const source = available.find((candidate) =>
      !usedSourceNumbers.has(candidate.number) && candidate.id === card.id
    );
    if (source) usedSourceNumbers.add(source.number);
    return source;
  });
  const tokens = mappedSources.map((source) => source ? String(source.number) : "?");
  const usedPool = mappedSources.filter(
    (source): source is SolverSourceCard => source?.origin === "pool",
  );
  return {
    text: sources.task === "order" && tokens.every((token) => token.length === 1)
      ? tokens.join("")
      : tokens.join(" "),
    tokens,
    usedHand,
    unusedHand,
    usedPool,
  };
}

function solverSourceLegend(sources: SolverSourceContext): string {
  const fieldRange = sourceRange(sources.field);
  if (sources.task === "hand") {
    return `编号：基准 ${fieldRange} · 手牌 ${sourceRange(sources.hand)} · 手N=换入数`;
  }
  if (sources.task === "pool") {
    return `编号：基准 ${fieldRange} · 卡池新增 ${sourceRange(sources.pool)}`;
  }
  return `数字 = 求解基准牌槽位 ${fieldRange}`;
}

function sourceRange(sources: readonly SolverSourceCard[]): string {
  if (sources.length === 0) return "—";
  if (sources.length === 1) return String(sources[0]!.number);
  return `${sources[0]!.number}–${sources.at(-1)!.number}`;
}

function solverSourceTitle(sources: SolverSourceContext): string {
  const fieldLabel = sources.field.some((source) => source.id === 0)
    ? "求解基准牌（含普通攻击补位）"
    : "求解基准牌";
  return [
    "牌序编号说明",
    "",
    ...sourceSection(fieldLabel, sources.field),
    ...(sources.hand.length > 0 ? ["", ...sourceSection("当前手牌", sources.hand)] : []),
    ...(sources.pool.length > 0 ? ["", ...sourceSection("卡池新增牌", sources.pool)] : []),
    "",
    "读法：候选牌序中的每个数字都指向这里的一张来源牌。",
  ].join("\n");
}

function sourceSection(
  label: string,
  sources: readonly SolverSourceCard[],
): readonly string[] {
  if (sources.length === 0) return [`${label}：无`];
  return [
    `${label}：`,
    ...sources.map((source) => `  ${source.number} = ${source.name}`),
  ];
}

function formatChangeCount(item: SolverItem): string {
  const cardChanges = item.changedSlots.length;
  const talentChanges = item.talentChanges?.length ?? 0;
  if (cardChanges === 0 && talentChanges === 0) return "—";
  return [
    cardChanges === 0 ? "" : `换${cardChanges}`,
    talentChanges === 0 ? "" : `命${talentChanges}`,
  ].filter(Boolean).join(" · ");
}

function formatSourceChangeCount(
  item: SolverItem,
  sources: SolverSourceContext,
  order: SolverOrderPresentation,
): string {
  if (sources.task === "hand") {
    const talentChanges = item.talentChanges?.length ?? 0;
    return `手${order.usedHand.length}${talentChanges > 0 ? ` · 命${talentChanges}` : ""}`;
  }
  if (sources.task === "pool") {
    const talentChanges = item.talentChanges?.length ?? 0;
    return `池${order.usedPool.length}${talentChanges > 0 ? ` · 命${talentChanges}` : ""}`;
  }
  return formatChangeCount(item);
}

function renderSolverWhy(
  top: SolverItem,
  baseline: SolverBaseline,
  hpDeltaBest: SolverItem | undefined,
): string {
  if (!top.evaluation.valueMetrics || !baseline.valueMetrics) {
    return "";
  }
  const comparison = hpDeltaBest?.evaluation.valueMetrics
    ? classifyValueRankComparison(
      { deckKey: hpDeltaBest.deckKey, evaluation: hpDeltaBest.evaluation },
      { deckKey: top.deckKey, evaluation: top.evaluation },
      hpDeltaBest.evaluation,
    )
    : null;
  const title = solverWhyTitle(top, baseline, comparison);
  const hpDeltaBestText = hpDeltaBest && hpDeltaBest.deckKey !== top.deckKey
    ? `<span title="${escapeAttribute(`列表血差最优: 第${hpDeltaBest.rank}，HP ${formatSigned(hpDeltaBest.evaluation.hpDeltaForSide)}`)}">血差最优 第${hpDeltaBest.rank}</span>`
    : "";
  return `
    <div class="solver-why" title="${escapeAttribute(title)}">
      <b>为什么</b>
      <span>Value ${formatDelta(top.evaluation.score - baseline.score)}</span>
      <span>HP ${formatDelta(top.evaluation.hpDeltaForSide - baseline.hpDeltaForSide)}</span>
      ${comparison ? `<span class="solver-value-chip" title="${escapeAttribute(valueRankExplanation(comparison))}">${escapeHtml(comparison.category)}</span>` : ""}
      ${hpDeltaBestText}
    </div>
  `;
}

function modeLabel(mode: string): string {
  const labels: Readonly<Record<string, string>> = {
    order: "牌序",
    hand: "手牌",
    beam: "卡池",
    talent: "仙命",
  };
  return labels[mode] ?? "建议";
}

function confidenceLabel(confidence: string): string {
  const labels: Readonly<Record<string, string>> = {
    exact: "精确",
    heuristic: "启发",
    truncated: "截断",
  };
  return labels[confidence] ?? "结果";
}

function resultBadge(
  item: SolverItem,
  baseline: SolverBaseline,
  hpDeltaBestDeckKey?: string,
): string {
  if (item.rank === 1) return "推荐";
  if (hpDeltaBestDeckKey && item.deckKey === hpDeltaBestDeckKey) return "血差最优";
  if (!item.evaluation.winForSide) return "未胜";
  if (item.evaluation.actorTurn < baseline.actorTurn) return "更快";
  if (item.evaluation.hpDeltaForSide > baseline.hpDeltaForSide) return "血差";
  return "可选";
}

function solverRowTitle(
  item: SolverItem,
  baseline: SolverBaseline,
  sources: SolverSourceContext,
  order: SolverOrderPresentation,
  hpDeltaBestDeckKey?: string,
): string {
  const cardChanges = item.changedSlots.length === 0
    ? "卡牌: 无变化"
    : `卡牌: ${item.changedSlots.map((change) =>
        `${change.slot}: ${displaySolverCardName(change.from)} -> ${displaySolverCardName(change.to)}`
      ).join("; ")}`;
  const talentChanges = !item.talentChanges?.length
    ? "仙命: 无变化"
    : `仙命: ${item.talentChanges.map((change) =>
        `${change.slot}: ${formatTalent(change.from)} -> ${formatTalent(change.to)}`
      ).join("; ")}`;
  return [
    `第${item.rank} · ${resultBadge(item, baseline, hpDeltaBestDeckKey)}`,
    "",
    "牌序",
    `  ${order.tokens.join(" ")}`,
    "",
    ...sourceSection("求解基准牌", sources.field),
    ...(sources.hand.length > 0 ? [
      "",
      ...sourceSection("已换入手牌", order.usedHand),
      ...sourceSection("未换入手牌", order.unusedHand),
    ] : []),
    ...(sources.pool.length > 0 ? [
      "",
      ...sourceSection("使用的卡池牌", order.usedPool),
    ] : []),
    "",
    "战斗结果",
    `  胜负：${item.evaluation.winForSide ? "胜" : "负"}`,
    `  hpDelta：${formatSigned(item.evaluation.hpDeltaForSide)}`,
    `  actorTurn：${item.evaluation.actorTurn}（${formatTurnDelta(item.evaluation.actorTurn - baseline.actorTurn)}）`,
    `  评分：${item.evaluation.score}（相对基准 ${formatSigned(item.evaluation.score - baseline.score)}）`,
    "",
    "变化明细",
    cardChanges,
    talentChanges,
  ].join("\n");
}

function bestResultByHpDelta(results: readonly SolverItem[]): SolverItem | undefined {
  return results.reduce<SolverItem | undefined>((best, item) => {
    if (!best) return item;
    if (item.evaluation.hpDeltaForSide > best.evaluation.hpDeltaForSide) return item;
    if (item.evaluation.hpDeltaForSide === best.evaluation.hpDeltaForSide && item.rank < best.rank) return item;
    return best;
  }, undefined);
}

function solverWhyTitle(
  top: SolverItem,
  baseline: SolverBaseline,
  comparison: ValueRankComparison | null,
): string {
  const topMetrics = top.evaluation.valueMetrics!;
  const baselineMetrics = baseline.valueMetrics!;
  return [
    `相对基准: Value ${formatDelta(top.evaluation.score - baseline.score)} (终局 ${formatDelta(topMetrics.terminalValueForSide - baselineMetrics.terminalValueForSide)} / 过程 ${formatDelta(topMetrics.areaScoreForSide - baselineMetrics.areaScoreForSide)}), HP ${formatDelta(top.evaluation.hpDeltaForSide - baseline.hpDeltaForSide)}`,
    comparison ? valueRankExplanation(comparison) : "",
  ].filter(Boolean).join(" | ");
}

function valueRankExplanation(comparison: ValueRankComparison): string {
  switch (comparison.category) {
    case "same-top":
      return "Value 排名和血差最优是同一套。";
    case "value-lost-win":
      return "血差最优能赢，但 Value top 未赢，需复核权重。";
    case "value-found-win":
      return "Value top 能赢，血差最优未赢。";
    case "winner-split":
      return "Value top 和血差最优的胜方不同。";
    case "value-hp-regret":
      return `Value 分 ${formatDelta(comparison.valueScoreDelta)}，但比血差最优少 ${formatNumber(comparison.hpDeltaRegret)} HP。`;
    case "value-slower-close":
      return `Value 分 ${formatDelta(comparison.valueScoreDelta)}，但比血差最优慢 ${formatNumber(comparison.actorTurnDelta)} 回合终结。`;
    case "value-faster-close":
      return `Value 分 ${formatDelta(comparison.valueScoreDelta)}，且比血差最优快 ${formatNumber(-comparison.actorTurnDelta)} 回合终结。`;
    case "different-aligned":
      return "Value top 与血差最优不是同一套，但未触发重点后悔分类。";
  }
}

export function formatNumber(value: number): string {
  const normalized = Object.is(value, -0) ? 0 : value;
  return Number.isInteger(normalized) ? String(normalized) : normalized.toFixed(1);
}

export function formatBudget(value: number | undefined): string {
  return value === undefined ? "-" : formatNumber(value);
}

export function formatElapsed(value: number | undefined): string {
  if (value === undefined) return "-";
  return value < 1000 ? `${value}ms` : `${(value / 1000).toFixed(1)}s`;
}

function formatSigned(value: number): string {
  return value > 0 ? `+${formatNumber(value)}` : formatNumber(value);
}

function formatDelta(value: number): string {
  return value >= 0 ? `+${formatNumber(value)}` : formatNumber(value);
}

function formatTurnDelta(value: number): string {
  if (value < 0) return `更快${-value}`;
  if (value > 0) return `更慢${value}`;
  return "同回合";
}

function formatTalent(talentId: number): string {
  return talentId === 0 ? "-" : describeTalent(talentId);
}
