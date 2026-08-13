import { renderBattleResult } from "./render-battle";
import { renderSolverPanel } from "./render-solver";
import { renderFixtureImportPanel } from "./render-fixture-import";
import { renderSetupPane, renderSetupPickers } from "./render-setup";
import { renderTargetResult } from "./render-target-chart";
import { renderTargetSetupPane } from "./render-target-setup";
import { workbenchEvidenceMode } from "./evidence-mode";
import { readReleaseMetadata } from "./release-metadata";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState } from "./types";

export function renderApp(state: AppState): string {
  const evidenceMode = workbenchEvidenceMode(state);
  const releaseMetadata = readReleaseMetadata();
  const targetMode = state.workbenchMode === "target";
  const pageClass = [
    "combined-page",
    state.result ? "has-battle" : "setup-empty",
  ].filter(Boolean).join(" ");
  return `
    <div class="workbench">
      <header class="topbar">
        <div class="topbar-brand">
          <h1>Open-YiXianCard</h1>
        </div>
        ${renderWorkbenchModeSwitch(state)}
        <div class="topbar-status">
          <span
            class="release-snapshot ${releaseMetadata.bound ? "bound" : "unbound"}"
            title="${escapeAttribute(releaseMetadata.detail)}"
          >${escapeHtml(releaseMetadata.label)}</span>
          <span
            class="workbench-mode ${evidenceMode.kind}"
            title="${escapeAttribute(evidenceMode.detail)}"
          >${escapeHtml(evidenceMode.label)}</span>
        </div>
      </header>
      ${state.error ? `<div class="error-bar">${escapeHtml(state.error)}</div>` : ""}
      ${targetMode ? "" : renderFixtureImportPanel(state)}
      <main class="${pageClass}">
        <section class="combined-setup">
          ${targetMode
            ? renderTargetSetupPane(state)
            : renderSetupPane(state, { pickers: "none" })}
        </section>
        <section class="combined-battle">
          ${targetMode ? renderTargetResult(state) : renderBattleResult(state)}
          ${
    // 战斗前右列没有模块选项卡，求解仍然要能用：它是"先算出该带什么牌"的入口。
    !targetMode && !state.result
      ? `<div class="battle-module-body standalone" data-module="advice">${
        renderSolverPanel(state)
      }</div>`
      : ""
  }
        </section>
        <div class="setup-picker-host">${renderSetupPickers(state)}</div>
      </main>
    </div>
  `;
}

/** 顶栏顶层模式切换：[双方对战 | 打靶模式]。 */
function renderWorkbenchModeSwitch(state: AppState): string {
  const mode = state.workbenchMode;
  return `
    <div class="workbench-mode-switch" role="group" aria-label="工作台模式">
      <button
        type="button"
        class="mode-opt ${mode === "duel" ? "active" : ""}"
        data-action="switch-workbench-mode"
        data-mode="duel"
        aria-pressed="${mode === "duel" ? "true" : "false"}"
        title="双方对战：配置双方角色、副职、仙命与卡组并推演"
      >双方对战</button>
      <button
        type="button"
        class="mode-opt ${mode === "target" ? "active" : ""}"
        data-action="switch-workbench-mode"
        data-mode="target"
        aria-pressed="${mode === "target" ? "true" : "false"}"
        title="打靶模式：单侧构筑对静默木桩，按卡牌来源统计每回合伤害，支持多构筑并行对比"
      >打靶模式</button>
    </div>
  `;
}
