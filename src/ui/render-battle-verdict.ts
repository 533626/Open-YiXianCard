import { formatSigned } from "./battle-explanation";
import { renderFixtureConsistencyBadge } from "./render-fixture-import";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { BattleExplanation, ChannelShare, TurningPoint } from "./battle-explanation";
import type { AppState, BattleFrame, Side } from "./types";

/**
 * 结论层：先回答"谁赢、赢多少、怎么赢的"，引擎透视与曲线排在它后面的模块里。
 *
 * "主要靠生命通道积累优势"这种话不回答任何问题，所以赢法句子必须点名是哪几张牌的
 * 结算点累出了这份优势，以及对手哪张牌打回最多。
 */
export function renderBattleVerdict(
  state: AppState,
  finalFrame: BattleFrame,
  actionCount: number,
): string {
  const winnerSide = state.result?.winnerId as Side | null | undefined;
  const winnerName = winnerSide
    ? finalFrame.players[winnerSide]?.name ?? winnerSide
    : "无胜者";
  const hpDelta = finalFrame.players.p1.hp - finalFrame.players.p2.hp;
  const hpDeltaForWinner = winnerSide === "p2" ? -hpDelta : hpDelta;
  const explanation = state.result?.explanation;
  const consistencyBadge = renderFixtureConsistencyBadge(state);
  return `
    <div class="battle-verdict" aria-label="战斗结论">
      <div class="verdict-head">
        <strong class="verdict-winner ${winnerSide ?? "none"}">${escapeHtml(winnerName)}${
    winnerSide ? " 胜" : ""
  }</strong>
        <span class="verdict-metric"><span>动数</span><b>${actionCount}</b></span>
        <span class="verdict-metric"><span>生命差</span><b>${
    formatSigned(hpDeltaForWinner)
  }</b></span>
        ${
    explanation
      ? `<span class="verdict-metric" title="value-v0 终局价值变化，胜方视角"><span>价值</span><b>${
        formatSigned(explanation.valueDelta)
      }</b></span>`
      : ""
  }
        ${consistencyBadge}
      </div>
      ${explanation ? renderExplanation(explanation) : renderExplanationFallback(winnerSide)}
    </div>
  `;
}

function renderExplanation(explanation: BattleExplanation): string {
  if (explanation.auditDelta !== 0) {
    return `
      <p class="verdict-headline degraded" role="status">
        归因与终局价值差 ${
      formatSigned(explanation.auditDelta)
    }，本场解释不可信，请只看上方胜负与生命差。
      </p>
    `;
  }
  return `
    <p class="verdict-headline">${escapeHtml(explanation.headline)}</p>
    ${renderCounterfactuals(explanation.counterfactuals)}
    ${renderChannelBar(explanation.channels)}
    ${renderTurningPoints(explanation.turningPoints)}
  `;
}

function renderExplanationFallback(winnerSide: Side | null | undefined): string {
  return `
    <p class="verdict-headline muted">${
    winnerSide ? "本场未能生成赢法解释。" : "没有胜者，无赢法可解释。"
  }</p>
  `;
}

function renderCounterfactuals(
  counterfactuals: BattleExplanation["counterfactuals"],
): string {
  if (counterfactuals.length === 0) return "";
  return `
    <div
      class="verdict-counterfactuals"
      aria-label="反事实重跑"
      title="逐项从首个 canonical 观察点移除状态，用同一 decision/random tape 重跑。分叉前是干净前缀；终局值在分叉后只表示整条线路的变化。"
    >
      <span class="counterfactual-label">反事实重跑</span>
      ${counterfactuals.map((item) => {
    const divergence = item.firstDivergenceActorTurn === null
      ? "全程未分叉"
      : `第 ${item.firstDivergenceActorTurn} 动分叉`;
    const winner = item.winnerChanged
      ? ` · 胜方变为 ${item.counterfactualWinner.toUpperCase()}`
      : "";
    return `
          <span class="counterfactual-item">
            <b>去掉${escapeHtml(item.element.label)}</b>
            <span>分叉前 ${formatSigned(item.preDivergenceHpDeltaChangeForSide)}</span>
            <span>终局 ${formatSigned(item.terminalHpDeltaChangeForSide)}</span>
            <i>${escapeHtml(divergence)}${escapeHtml(winner)}</i>
          </span>
        `;
  }).join("")}
    </div>
  `;
}

function renderChannelBar(channels: readonly ChannelShare[]): string {
  if (channels.length === 0) return "";
  const gains = channels.filter((channel) => channel.delta > 0);
  const losses = channels.filter((channel) => channel.delta < 0);
  return `
    <div class="verdict-channels" aria-label="价值通道构成">
      <div class="verdict-channel-bar">
        ${
    gains.map((channel) => `
          <span
            class="verdict-channel-slice channel-${channel.key}"
            style="--channel-share:${(channel.share * 100).toFixed(1)}%"
            title="${escapeAttribute(`${channel.label} ${formatSigned(channel.delta)}`)}"
          ></span>
        `).join("")
  }
      </div>
      <div class="verdict-channel-legend">
        ${gains.map(renderChannelChip).join("")}
        ${
    losses.length > 0
      ? `<span class="verdict-channel-loss">失分 ${losses.map(renderChannelChip).join("")}</span>`
      : ""
  }
      </div>
    </div>
  `;
}

function renderChannelChip(channel: ChannelShare): string {
  return `
    <span class="verdict-channel-chip channel-${channel.key}">
      <i class="channel-dot" aria-hidden="true"></i>
      ${escapeHtml(channel.label)}
      <b>${formatSigned(channel.delta)}</b>
    </span>
  `;
}

function renderTurningPoints(points: readonly TurningPoint[]): string {
  if (points.length === 0) return "";
  return `
    <ol class="verdict-turns" aria-label="关键转折点">
      ${
    points.map((point) => `
        <li>
          <span class="turn-action" title="第 ${point.actorTurn} 动作回合">第 ${point.actionIndex} 动</span>
          <span class="turn-card">
            ${point.byOpponent ? '<i class="turn-by-opponent">对手</i>' : ""}
            ${escapeHtml(point.cardName)}
          </span>
          ${
      point.leadingChannel
        ? `<span class="turn-channel channel-${point.leadingChannel.key}">${
          escapeHtml(point.leadingChannel.label)
        }</span>`
        : ""
    }
          <b class="turn-delta ${point.delta > 0 ? "gain" : "loss"}">${
      formatSigned(point.delta)
    }</b>
        </li>
      `).join("")
  }
    </ol>
  `;
}
