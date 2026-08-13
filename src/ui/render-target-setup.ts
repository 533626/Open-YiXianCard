/**
 * 打靶模式左列：全局参数（累计阈值/显示至回合/对比模式）+ 多构筑槽列表。
 *
 * 多构筑的「镜像」约定见 main-actions.ts `switchWorkbenchMode`：打靶模式下
 * `config.players.p1` 就是当前聚焦构筑的 player（同一对象），既有玩家面板/
 * picker/出牌动作全部无改动地写回该构筑。因此这里只对聚焦构筑渲染完整面板，
 * 其余构筑渲染紧凑摘要卡（名称/角色/副职/卡组/结果一行），点击即切换聚焦。
 */

import {
  CARD_OPTION_BY_BASE_ID,
  CAREER_OPTIONS,
  CHARACTER_BY_ID,
} from "./data";
import { renderPlayerPanel } from "./render-player-panel";
import { targetReachedLabel } from "./target-practice-metrics";
import {
  activeTargetBuild,
  targetPracticeState,
} from "./main-actions";
import { GAME_TURN_LIMIT } from "./target-dummy";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState, TargetBuild, TargetCompareMode, TargetPracticeState } from "./types";

export function renderTargetSetupPane(state: AppState): string {
  const target = targetPracticeState(state);
  const active = activeTargetBuild(state);
  const anyRunning = target.builds.some((build) => build.status === "running");
  return `
    <section class="setup-pane target-setup-pane" aria-label="打靶模式构筑">
      <div class="setup-command-row">
        <div class="setup-match-controls target-params" aria-label="打靶参数">
          <span class="setup-command-label" title="累计伤害达到该值即停止推演">累计阈值</span>
          <label class="hp-field">
            <input type="number" id="battle-targetThreshold" value="${target.damageThreshold}" min="1" max="9999" />
          </label>
          ${renderDisplayRoundControl(target, active)}
          <span class="setup-command-label" title="多构筑对比显示方式">对比</span>
          ${renderCompareToggle(target.compareMode)}
          <label class="hp-field global-round-field">
            <span>修炼轮</span>
            <input type="number" id="battle-gameRound" value="${state.config.gameRound}" min="1" max="99" />
          </label>
        </div>
        <div class="setup-command-actions" aria-label="打靶操作">
          ${anyRunning ? `
            <button type="button" class="setup-cancel-action" data-action="cancel-target-practice" title="取消全部打靶推演">取消</button>
          ` : ""}
          <button type="button" class="setup-tool-action" data-action="reset" title="清空全部打靶构筑">重置</button>
        </div>
      </div>
      <div class="target-hint" title="与双方对战同一规则：卡组不足 8 张时，空卡槽在推演中按普通攻击（3 攻）结算，其伤害会归到「普通攻击」名下">
        空卡槽按普通攻击结算（与双方对战一致）
      </div>
      <div class="target-builds" aria-label="打靶构筑列表">
        ${target.builds.map((build) => renderTargetBuildCard(state, build, build.id === active?.id)).join("")}
      </div>
      <button
        type="button"
        class="target-add-build"
        data-action="add-target-build"
        title="${target.builds.length >= TARGET_BUILD_SOFT_LIMIT
          ? `建议最多 ${TARGET_BUILD_SOFT_LIMIT} 套：Worker 单线程排队执行，构筑过多会排队`
          : "新增一套打靶构筑（建议最多 4 套，Worker 单线程排队执行）"}"
        ${target.builds.length >= TARGET_BUILD_SOFT_LIMIT ? "disabled" : ""}
      >+ 新增构筑</button>
    </section>
  `;
}

/** 多构筑软上限：Worker 单线程串行执行引擎调用，超过建议数会排队。 */
const TARGET_BUILD_SOFT_LIMIT = 4;

/**
 * 「显示至回合」滑条：绝对有效回合数，不是「额外回合」。
 * - 有结果时有效范围是 `[reachedTurn, 32]`：第 4 回合达标就从 4 开始显示，
 *   滑条只能向后拖到 5/6…，不提供 0..reachedTurn-1 这类没有完整结果的窗口。
 * - 无结果（尚未推演/推演中/失败）时显示禁用态与「等待推演」，不伪造一个
 *   没有依据的 min/value。
 * - 拖动期间只由 input 事件更新读数（临时 DOM 预览）；change（松手/键盘提交）
 *   才写状态、作废结果并触发一次自动重算，避免每个像素都排 Worker。
 */
function renderDisplayRoundControl(
  target: TargetPracticeState,
  active: TargetBuild | undefined,
): string {
  const reachedTurn = active?.result?.reachedTurn;
  const hasResult = active?.status === "done" && reachedTurn !== undefined;
  const pending = target.displayRoundPending === true;
  if (!hasResult && !pending) {
    return `
      <span
        class="setup-command-label"
        title="有结果时从打到阈值的回合起显示，最多到游戏常量 32 回合上限；未出结果时不可调"
      >显示至回合</span>
      <label class="hp-field display-round-field" title="等待推演完成后可用">
        <input
          type="range"
          id="battle-targetDisplayRounds"
          value="${target.displayRounds}"
          min="1"
          max="${GAME_TURN_LIMIT}"
          step="1"
          disabled
          aria-label="显示至回合"
          aria-valuetext="等待推演"
        />
        <span class="display-round-readout waiting">等待推演</span>
      </label>
    `;
  }
  const minValue = Math.min(
    GAME_TURN_LIMIT,
    Math.max(1, hasResult ? reachedTurn! : target.displayRoundMin ?? 1),
  );
  const current = Math.min(GAME_TURN_LIMIT, Math.max(minValue, target.displayRounds));
  return `
    <span
      class="setup-command-label"
      title="显示到打到阈值的回合起，最多到游戏常量 32 回合上限；拖动时预览读数，松手后重算"
    >显示至回合</span>
    <label class="hp-field display-round-field">
      <input
        type="range"
        id="battle-targetDisplayRounds"
        value="${current}"
        min="${minValue}"
        max="${GAME_TURN_LIMIT}"
        step="1"
        aria-label="显示至回合"
        aria-valuetext="${current} / ${GAME_TURN_LIMIT}"
        title="显示至回合：R${current} / ${GAME_TURN_LIMIT}（从达标回合 R${minValue} 起可调）"
      />
      <span class="display-round-readout${pending ? " waiting" : ""}" data-display-round-readout="1">R${current} / ${GAME_TURN_LIMIT}${pending ? " · 重算中" : ""}</span>
    </label>
  `;
}

function renderCompareToggle(mode: TargetCompareMode): string {
  const active = mode === "overlay";
  return `
    <span class="first-toggle target-compare-toggle" role="group" aria-label="对比模式">
      <button
        type="button"
        class="toggle-opt ${active ? "active" : ""}"
        data-action="set-target-compare-mode"
        data-mode="overlay"
        aria-pressed="${active ? "true" : "false"}"
        title="各构筑的累计伤害曲线叠加在同一坐标系，直观对比谁先达成阈值"
      >叠加</button>
      <button
        type="button"
        class="toggle-opt ${active ? "" : "active"}"
        data-action="set-target-compare-mode"
        data-mode="grid"
        aria-pressed="${active ? "false" : "true"}"
        title="每套构筑一个分面堆叠柱状图"
      >网格</button>
    </span>
  `;
}

function renderTargetBuildCard(state: AppState, build: TargetBuild, active: boolean): string {
  const statusClass = build.status && build.status !== "idle" ? ` status-${build.status}` : "";
  return `
    <article class="target-build-card ${active ? "active" : ""}${statusClass}" data-build-id="${escapeAttribute(build.id)}">
      <header class="target-build-head">
        <span class="target-build-status-dot" title="${statusTitle(build)}"></span>
        <input
          type="text"
          class="target-build-name"
          value="${escapeAttribute(build.name)}"
          aria-label="构筑名称"
          autocomplete="off"
          data-target-build-name="1"
          data-action="rename-target-build"
          data-build-id="${escapeAttribute(build.id)}"
        />
        <button type="button" class="build-action duplicate" data-action="duplicate-target-build" data-build-id="${escapeAttribute(build.id)}" title="复制这套构筑">⧉</button>
        <button type="button" class="build-action delete" data-action="remove-target-build" data-build-id="${escapeAttribute(build.id)}" title="删除这套构筑" ${state.target!.builds.length <= 1 ? "disabled" : ""}>✕</button>
      </header>
      ${active ? renderActiveBuildBody(state, build) : renderInactiveBuildSummary(state, build)}
      ${renderBuildResultLine(build)}
    </article>
  `;
}

/** 聚焦构筑：完整复用双方对战的玩家面板（镜像写回，side 恒 p1）。 */
function renderActiveBuildBody(state: AppState, build: TargetBuild): string {
  return `
    <div class="target-build-panel">
      ${renderPlayerPanel(state, "p1")}
    </div>
  `;
}

function renderInactiveBuildSummary(state: AppState, build: TargetBuild): string {
  const character = CHARACTER_BY_ID.get(build.player.characterId);
  const careerName = CAREER_OPTIONS.find((career) => career.id === build.player.careerName)?.name ?? "炼丹师";
  const deckNames = build.player.deck
    .filter((slot) => slot.baseId > 0)
    .map((slot) => CARD_OPTION_BY_BASE_ID.get(slot.baseId)?.name ?? `卡牌 ${slot.baseId}`)
    .join("、");
  return `
    <button type="button" class="target-build-summary" data-action="select-target-build" data-build-id="${escapeAttribute(build.id)}" title="切换到这套构筑继续编辑">
      <span class="target-summary-line">
        <b>${escapeHtml(character?.name ?? "未选角色")}</b>
        <span>${escapeHtml(careerName)}</span>
      </span>
      <span class="target-summary-deck">${deckNames ? escapeHtml(deckNames) : "未摆牌"}</span>
      <span class="target-summary-hint">点击编辑 →</span>
    </button>
  `;
}

function renderBuildResultLine(build: TargetBuild): string {
  const status = build.status ?? "idle";
  if (status === "running") {
    return `
      <div class="target-build-result running" role="status">
        <span class="solver-spinner" aria-hidden="true"></span>
        <span>推演中…</span>
        <button type="button" class="build-action" data-action="cancel-target-practice" title="取消全部打靶推演">取消</button>
      </div>
    `;
  }
  if (status === "error") {
    return `
      <div class="target-build-result error">
        <span>${escapeHtml(build.errorMessage ?? "推演失败")}</span>
        <button type="button" class="build-action rerun" data-action="run-target-practice" data-build-id="${escapeAttribute(build.id)}" title="重试这次推演">重试</button>
      </div>
    `;
  }
  if (status === "done" && build.result) {
    const result = build.result;
    return `
      <div class="target-build-result done">
        <span>累计 <b>${result.totalDamage}</b> 伤 · 第 <b>${result.reachedTurn}</b> 回合 · ${targetReachedLabel(result)}</span>
        <button type="button" class="build-action rerun" data-action="run-target-practice" data-build-id="${escapeAttribute(build.id)}" title="重新推演这套构筑">重跑</button>
      </div>
    `;
  }
  return `
    <div class="target-build-result idle">
      <span>${build.player.characterId > 0 ? "改动后自动推演" : "先选角色并摆牌"}</span>
      <button type="button" class="build-action rerun" data-action="run-target-practice" data-build-id="${escapeAttribute(build.id)}" title="手动推演这套构筑" ${build.player.characterId > 0 ? "" : "disabled"}>运行</button>
    </div>
  `;
}

function statusTitle(build: TargetBuild): string {
  switch (build.status ?? "idle") {
    case "running": return "推演中";
    case "done": return build.result
      ? `已完成：累计 ${build.result.totalDamage} 伤，第 ${build.result.reachedTurn} 回合 ${targetReachedLabel(build.result)}`
      : "已完成";
    case "error": return "推演失败";
    default: return "未运行";
  }
}
