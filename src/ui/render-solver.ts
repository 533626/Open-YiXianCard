import {
  SOLVER_METHOD_ORDER,
  SOLVER_METHODS,
  SOLVER_PRESETS,
  SOLVER_TASK_ORDER,
  SOLVER_TASKS,
  solverMethodForMode,
  solverModeLabel,
  solverTaskForMode,
  type SolverUiTask,
  type SolverUiMode,
} from "./solver-ui";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState } from "./types";
import { solverCardPoolOptions } from "./solver-card-pool";
import {
  formatBudget,
  formatElapsed,
  formatNumber,
  renderSolverResult,
  solverHandCardName,
} from "./render-solver-result";

export function renderSolverPanel(state: AppState): string {
  const result = state.solverResult;
  const activeMode = state.solverMode ?? "orderBeam";
  const activeTask = solverTaskForMode(activeMode);
  const activeMethod = solverMethodForMode(activeMode);
  const running = state.solverStatus?.state === "running";
  const battleRunning = state.battleStatus?.state === "running";
  const computeLocked = running || battleRunning;
  const taskContext = solverTaskContext(state, activeTask);
  const solveDisabled = computeLocked || !taskContext.available;
  const panelClass = [
    "solver-panel",
    result ? "has-result" : "",
    running ? "busy" : "",
    state.solverStatus ? "has-status" : "",
  ].filter(Boolean).join(" ");
  return `
    <section class="${panelClass}" id="winning-advice" aria-label="求解建议" aria-busy="${running ? "true" : "false"}">
      <header class="solver-workbench-head">
        <div>
          <b>求解建议</b>
          <span>${escapeHtml(taskContext.source)}</span>
        </div>
      </header>
      <nav class="solver-task-tabs" aria-label="求解任务">
        ${SOLVER_TASK_ORDER.map((task) => `
          <button
            type="button"
            class="${activeTask === task ? "active" : ""}"
            data-action="set-solver-task"
            data-solver-task="${task}"
            aria-pressed="${activeTask === task ? "true" : "false"}"
            ${computeLocked ? "disabled" : ""}
            title="${escapeAttribute(SOLVER_TASKS[task].hint)}"
          ><b>${escapeHtml(SOLVER_TASKS[task].label)}</b><span>${escapeHtml(taskTabMeta(state, task))}</span></button>
        `).join("")}
      </nav>
      <div class="solver-command-bar">
        <fieldset class="solver-methods" aria-label="搜索方式" ${computeLocked ? "disabled" : ""}>
          ${SOLVER_METHOD_ORDER.map((method) => {
            const unavailable = activeTask === "pool" && method === "exhaustive";
            return `
            <button
              type="button"
              class="${[
                activeMethod === method ? "active" : "",
                unavailable ? "unavailable" : "",
              ].filter(Boolean).join(" ")}"
              data-action="set-solver-method"
              data-solver-method="${method}"
              aria-pressed="${activeMethod === method ? "true" : "false"}"
              ${unavailable ? 'aria-disabled="true" disabled' : ""}
              title="${escapeAttribute(unavailable ? "卡池组合空间过大，仅支持启发式搜索" : SOLVER_METHODS[method].hint)}"
            >${escapeHtml(SOLVER_METHODS[method].label)}</button>
          `}).join("")}
        </fieldset>
        <div class="solver-task-summary">
          <b>${escapeHtml(SOLVER_PRESETS[activeMode].shortLabel)}</b>
        </div>
        <div class="solver-primary-actions">
          ${activeTask === "hand" && !taskContext.available ? `
            <button type="button" class="solver-import" data-action="toggle-fixture-import">导入对局</button>
          ` : ""}
          <button type="button" class="solver-run primary" data-action="solve-active" ${solveDisabled ? "disabled" : ""}>${running ? "求解中" : "开始求解"}</button>
          ${running ? `
            <button
              type="button"
              class="solver-cancel"
              data-action="cancel-solver"
              title="取消当前求解（Esc）"
            >取消</button>
          ` : ""}
        </div>
      </div>
      ${activeTask === "hand" ? renderSolverHand(state) : ""}
      ${renderSolverStatus(state)}
      ${result ? renderSolverResult(result, state.solverStatus?.mode, state) : renderSolverEmpty(taskContext.hint)}
    </section>
  `;
}

function renderSolverHand(state: AppState): string {
  const hand = state.config.players[state.activeSide].handCardIds;
  const fieldCount = solverFieldSourceCount(state);
  const grouped = new Map<number, { readonly id: number; readonly numbers: number[] }>();
  hand.forEach((id, index) => {
    const group = grouped.get(id);
    if (group) {
      group.numbers.push(fieldCount + index + 1);
    } else {
      grouped.set(id, { id, numbers: [fieldCount + index + 1] });
    }
  });
  const handGuide = [
    "当前手牌",
    "",
    "来源：导入对局记录里的当前手牌。",
    "编号：接在求解基准牌之后；基准包含回放里的“普通攻击”补位牌。",
    "判断：候选牌序中出现手牌编号，表示该牌确实被换入；行末“手N”是换入总数。",
  ].join("\n");
  return `
    <div class="solver-hand-input" title="${escapeAttribute(handGuide)}">
      <b>当前手牌</b>
      <div>
        ${grouped.size === 0
          ? `<span class="solver-hand-empty">未导入手牌</span>`
          : [...grouped.values()].map(({ id, numbers }) => `
            <span class="solver-hand-card" title="${escapeAttribute([
              "手牌编号",
              "",
              `编号：${numbers.join("、")}`,
              `卡牌：${solverHandCardName(id)}`,
              `卡牌 ID：${id}`,
              "",
              "候选牌序出现上述编号时，表示这张手牌被换入场上八张牌。",
            ].join("\n"))}">
              <em>${numbers.join("/")}</em>${escapeHtml(solverHandCardName(id))}${numbers.length > 1 ? `<strong>×${numbers.length}</strong>` : ""}
            </span>
          `).join("")}
      </div>
    </div>
  `;
}

function solverFieldSourceCount(state: AppState): number {
  const result = state.solverResult;
  if (result?.mode === "hand" && result.side === state.activeSide && result.baselineDeck.length > 0) {
    return result.baselineDeck.length;
  }
  const player = state.config.players[state.activeSide];
  return player.deck
    .filter((slot) => slot.baseId > 0 || slot.originalConfig !== undefined)
    .length;
}


function solverTaskContext(
  state: AppState,
  task: ReturnType<typeof solverTaskForMode>,
): {
  readonly available: boolean;
  readonly source: string;
  readonly countLabel: string;
  readonly hint: string;
} {
  const player = state.config.players[state.activeSide];
  if (task === "order") {
    return {
      available: true,
      source: "当前构筑",
      countLabel: `${player.activeSlotCount} 张场上牌`,
      hint: "固定用牌，只比较出牌顺序。",
    };
  }
  if (task === "hand") {
    const count = player.handCardIds.length;
    const source = state.importedFixtureId
      ? `已导入 ${state.importedFixtureId}`
      : state.importedFixture ? "已导入本地对局" : "尚未导入对局";
    return {
      available: count > 0,
      source,
      countLabel: `${count} 张手牌`,
      hint: count > 0
        ? "从场上牌与手牌中重组 8 张，并比较牌序。"
        : "导入含手牌的对局 JSON 后，可计算换入手牌的最优方案。",
    };
  }
  const count = solverPoolCardCount(state);
  return {
    available: count > 0,
    source: "当前角色 / 门派 / 副职",
    countLabel: `${count} 种已实现牌`,
    hint: "仅启发式：每种一阶牌 1 张，从当前角色、门派与副职的已实现卡池中搜索。",
  };
}

function taskTabMeta(
  state: AppState,
  task: ReturnType<typeof solverTaskForMode>,
): string {
  if (task === "order") return `${state.config.players[state.activeSide].activeSlotCount} 张`;
  if (task === "hand") return `${state.config.players[state.activeSide].handCardIds.length} 手牌`;
  return `${solverPoolCardCount(state)} 种`;
}

function solverPoolCardCount(state: AppState): number {
  const player = state.config.players[state.activeSide];
  return solverCardPoolOptions(player).length;
}

function renderSolverEmpty(hint: string): string {
  return `
    <div class="solver-empty-state">
      <b>等待求解</b>
      <span>${escapeHtml(hint)}</span>
      <span>结果会直接显示在这里，无需展开或切换面板。</span>
    </div>
  `;
}

function renderSolverStatus(state: AppState): string {
  const status = state.solverStatus;
  if (!status) return "";
  const label = solverModeLabel(status.mode);
  if (status.state === "running") {
    return `
      <div class="solver-status running" role="status" aria-live="polite">
        <span class="solver-spinner" aria-hidden="true"></span>
        <b>${escapeHtml(label)}</b>
        <span>预算 ${escapeHtml(formatBudget(status.maxEvaluations))}</span>
        <span>Worker 计算中 · 可继续查看帧与滚动</span>
      </div>
    `;
  }
  if (status.state === "error") {
    return `
      <div class="solver-status error">
        <b>${escapeHtml(label)}</b>
        <span>${escapeHtml(formatElapsed(status.elapsedMs))}</span>
        <span>${escapeHtml(status.message ?? "求解失败")}</span>
      </div>
    `;
  }
  return `
    <div class="solver-status done">
      <b>${escapeHtml(label)}</b>
      <span>${escapeHtml(formatElapsed(status.elapsedMs))}</span>
      <span>评估 ${escapeHtml(formatNumber(status.evaluatedCount ?? state.solverResult?.evaluatedCount ?? 0))}${status.maxEvaluations ? ` / ${escapeHtml(formatNumber(status.maxEvaluations))}` : ""}</span>
    </div>
  `;
}
