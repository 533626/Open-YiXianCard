import {
  CHARACTER_BY_ID,
  normalizePlayerTalents,
} from "./data";
import { adjacentCompletedTurnFrameIndex } from "./battle-keyboard";
import { LEVEL_OPTIONS, derivePlayerBattleStats } from "./derived-state";
import { renderBattleHp } from "./player-battle-state";
import { renderPlayerDeck } from "./render-player-deck";
import { renderPhysiqueField, renderSetupBody, renderSetupToolbar } from "./render-player-setup";
import { renderFateStrategyStrip, renderTalentRow } from "./render-player-talents";
import {
  escapeAttribute,
  escapeHtml,
} from "./view-utils";
import type { AppState, BattleFrame, Side } from "./types";

export function renderPlayerPanel(state: AppState, side: Side): string {
  const player = state.config.players[side];
  if (!state.config.sourceKind) normalizePlayerTalents(player);
  const frame = battleFrame(state);
  const runtime = frame?.players[side];
  const battleActiveSlots = playedSlotsForCurrentTurn(state, side);
  const previousRuntime = previousBattleFrame(state)?.players[side] ?? null;
  const character = CHARACTER_BY_ID.get(player.characterId);
  const isDuanXuan = character?.sectName === "DuanXuanZong";
  const setupActive = state.activeSide === side;
  const displayName = character?.name ?? "选择角色";
  const setupStats = derivePlayerBattleStats(player);
  const panelClasses = [
    "player-panel",
    frame ? "has-battle" : "",
    setupActive ? "active" : "",
  ].filter(Boolean).join(" ");
  return `
    <article class="${panelClasses}" data-side="${side}">
      ${renderPanelHead({
        state,
        displayName,
        side,
        player,
        setupActive,
        hp: setupStats.hp,
        maxHp: setupStats.maxHp,
        isDuanXuan,
        runtime,
        previousRuntime,
        characterEditing: state.pickerMode === "character" && state.activeSide === side,
      })}
      <div class="player-build-topline">
        ${character ? renderTalentRow(state, side, player) : ""}
        ${character ? renderFateStrategyStrip(state, side, player) : ""}
      </div>
      ${renderPlayerDeck({ state, side, player, frame, runtime, battleActiveSlots })}
      ${renderSetupBody(state, side, player, runtime, previousRuntime)}
    </article>
  `;
}

/**
 * 回放导航停在 turnEnd 时，完成态本身没有 sourceSlot。左栏应保留该 actorTurn
 * 已实际打出的全部槽位；再次行动链不能只亮最后一张。
 */
export function playedSlotsForCurrentTurn(state: AppState, side: Side): readonly number[] {
  if (!state.result) return [];
  const current = battleFrame(state);
  if (!current || current.actorId !== side || current.actorTurn <= 0) return [];
  const slots = new Set<number>();
  for (let index = state.frameIndex; index >= 0; index -= 1) {
    const frame = state.result.frames[index];
    if (!frame || frame.actorTurn !== current.actorTurn) break;
    if (frame.actorId === side && frame.sourceSlot !== null) slots.add(frame.sourceSlot);
  }
  return [...slots].sort((left, right) => left - right);
}

/** @deprecated Use renderPlayerPanel */
export function renderSetupPlayerPanel(state: AppState, side: Side): string {
  return renderPlayerPanel(state, side);
}

function battleFrame(state: AppState): BattleFrame | null {
  if (!state.result) return null;
  return state.result.frames[state.frameIndex] ?? state.result.frames[0] ?? null;
}

function previousBattleFrame(state: AppState): BattleFrame | null {
  if (!state.result || state.frameIndex <= 0) return null;
  const previousIndex = adjacentCompletedTurnFrameIndex(
    state.result,
    state.frameIndex,
    -1,
  );
  if (previousIndex >= state.frameIndex) return null;
  return state.result.frames[previousIndex] ?? null;
}

function renderPanelHead(options: {
  readonly state: AppState;
  readonly displayName: string;
  readonly side: Side;
  readonly player: AppState["config"]["players"][Side];
  readonly setupActive?: boolean;
  readonly hp: number;
  readonly maxHp: number;
  readonly isDuanXuan: boolean;
  readonly runtime: import("./types").PlayerView | null;
  readonly previousRuntime: import("./types").PlayerView | null;
  readonly characterEditing?: boolean;
}): string {
  return `
    <header class="player-panel-head">
      <div class="player-head-leading">
        <button
          type="button"
          class="player-name-trigger ${options.setupActive ? "selected" : ""} ${options.characterEditing ? "editing" : ""}"
          data-action="open-character-picker"
          data-side="${options.side}"
          title="${escapeAttribute(options.displayName)}"
        >${escapeHtml(options.displayName)}</button>
        ${compactLevelSelect(options.side, options.player.level)}
        <button type="button" class="player-reset-btn" data-action="reset-player" data-side="${options.side}">重置</button>
        <div class="player-hp-setup" aria-label="战前生命设置">
          ${compactMaxHpDisplay("上限", options.maxHp)}
          ${compactNumberField("修正", `player-${options.side}-lifeModifier`, options.player.lifeModifier, -9999, 9999)}
          ${options.isDuanXuan ? renderPhysiqueField(options.side, options.player) : ""}
          ${
    options.runtime
      ? renderBattleHp(
        options.runtime,
        undefined,
        options.runtime.hp - (options.previousRuntime?.hp ?? options.runtime.hp),
      )
      : ""
  }
        </div>
      </div>
      ${renderSetupToolbar(options.state, options.side, options.player)}
    </header>
  `;
}

function compactNumberField(label: string, id: string, value: number, min: number, max: number): string {
  return `
    <label class="hp-field">
      <span>${label}</span>
      <input type="number" id="${id}" value="${value}" min="${min}" max="${max}" />
    </label>
  `;
}

function compactLevelSelect(side: Side, value: number): string {
  return `
    <label class="hp-field player-level-field">
      <span>境界</span>
      <select id="player-${side}-level">
        ${LEVEL_OPTIONS.map((option) => `
          <option value="${option.level}" ${value === option.level ? "selected" : ""}>${escapeHtml(option.label)}</option>
        `).join("")}
      </select>
    </label>
  `;
}

function compactMaxHpDisplay(label: string, value: number): string {
  return `
    <span class="hp-field hp-field-readonly">
      <span>${label}</span>
      <span class="hp-readonly-value">${value}</span>
    </span>
  `;
}
