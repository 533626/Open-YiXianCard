import { sideLabel } from "./data";
import { renderCardPopup, renderCareerPopup, renderCharacterPopup, renderFateStrategyPopup, renderTalentPopup } from "./render-pickers";
import { renderPlayerPanel } from "./render-player-panel";
import { renderDeckDiagnosticPanel } from "./render-deck-diagnostics";
import type { AppState } from "./types";
import { escapeAttribute } from "./view-utils";

export function renderSetupPage(state: AppState): string {
  return `
    <main class="layout setup-layout">
      ${renderSetupPane(state)}
    </main>
  `;
}

export function renderSetupPickers(state: AppState): string {
  return `
    ${renderCardPopup(state)}
    ${renderTalentPopup(state)}
    ${renderCareerPopup(state)}
    ${renderFateStrategyPopup(state)}
    ${renderCharacterPopup(state)}
  `;
}

export function renderSetupPane(state: AppState, options?: { readonly pickers?: "inline" | "none" }): string {
  const pickers = options?.pickers === "none" ? "" : renderSetupPickers(state);
  const setupGuide = [
    "对局构筑",
    "",
    "顺序：设置先手与修炼轮，再配置双方角色、副职、仙命与场上牌。",
    "牌组：每位玩家最多八张；卡槽内可直接切换卡牌等级。",
    "生命：上限由构筑自动推导，“修正”只调整本次战斗输入。",
    "检查：阻断战斗或求解的问题会在构筑诊断中出现。",
    "运行：配置完成后自动推演。",
  ].join("\n");
  return `
    <section class="setup-pane">
      <section
        class="free-build-panel"
        id="free-build"
        aria-label="自由构筑"
      >
      <div class="setup-command-row ${state.result ? "battle-complete" : ""}">
        <div class="setup-match-controls" aria-label="对局参数">
          <span
            class="setup-command-label"
            title="${escapeAttribute(setupGuide)}"
          >先手</span>
          ${renderFirstToggle(state)}
          ${renderGlobalBattleFields(state)}
        </div>
        ${renderWorkbenchActions(state)}
      </div>
      <div class="players-grid">
        ${renderPlayerPanel(state, "p1")}
        ${renderPlayerPanel(state, "p2")}
      </div>
      </section>
      ${renderDeckDiagnosticPanel(state)}
      ${pickers}
    </section>
  `;
}



function renderWorkbenchActions(state: AppState): string {
  const battleRunning = state.battleStatus?.state === "running";
  return `
    <div class="setup-command-actions" aria-label="工作台操作" aria-busy="${battleRunning ? "true" : "false"}">
      <button type="button" class="setup-tool-action" data-action="toggle-fixture-import" title="导入原版回放">导入</button>
      <button type="button" class="setup-tool-action" data-action="reset" title="重置全部配置">重置</button>
      ${battleRunning ? `
        <button
          type="button"
          class="setup-cancel-action"
          data-action="cancel-battle"
          title="取消当前战斗推演（Esc）"
        >取消</button>
      ` : ""}
    </div>
  `;
}

function renderGlobalBattleFields(state: AppState): string {
  return `
    <label class="hp-field global-round-field">
      <span>修炼轮</span>
      <input type="number" id="battle-gameRound" value="${state.config.gameRound}" min="1" max="99" />
    </label>
  `;
}

export function renderFirstToggle(state: AppState): string {
  const first = state.config.firstPlayerSide;
  return `
    <div class="first-picker" aria-label="先手">
      <div class="first-toggle">
        <button type="button" class="toggle-opt ${first === "p1" ? "active" : ""}" data-action="set-first" data-side="p1" title="${sideLabel("p1")}">${sideLabel("p1").replace("玩家", "")}</button>
        <button type="button" class="toggle-opt ${first === "p2" ? "active" : ""}" data-action="set-first" data-side="p2" title="${sideLabel("p2")}">${sideLabel("p2").replace("玩家", "")}</button>
      </div>
    </div>
  `;
}
