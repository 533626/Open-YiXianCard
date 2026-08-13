import {
  TALENT_OPTION_BY_ID,
  describeTalent,
  fateStrategyDisplayName,
  fateStrategyOptionsForCharacter,
  isFateStrategyImplemented,
  talentDetailText,
  talentPickerColumn,
} from "./data";
import { JI_FANGSHENG_CHARACTER_ID, maxJiFangshengInitialFateRank } from "./derived-state";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState, FateStrategyOption, PlayerConfig, Side } from "./types";

export function renderTalentRow(state: AppState, side: Side, player: PlayerConfig): string {
  return `
    <div class="talent-row">
      ${player.talents.map((talentId, index) => renderTalentSlot(state, side, player, talentId, index)).join("")}
    </div>
  `;
}

function renderTalentSlot(
  state: AppState,
  side: Side,
  player: PlayerConfig,
  talentId: number,
  index: number,
): string {
  const locked = index === 0;
  const empty = !talentId || !TALENT_OPTION_BY_ID.has(talentId);
  const disabledByLevel = !locked && index >= player.level;
  const editing = !locked && state.pickerMode === "talent" && state.activeSide === side && state.selectedTalentSlot === index;
  const name = empty ? "" : escapeHtml(describeTalent(talentId));
  const column = empty ? "" : talentPickerClass(talentId);
  const title = empty ? "" : escapeAttribute(talentDetailText(talentId));
  if (locked) {
    return `
      <div class="talent-slot locked ${column}" ${title ? `title="${title}"` : ""}>
        <span class="talent-slot-name">${name}</span>
        ${renderLockedTalentControls(side, player)}
      </div>
    `;
  }
  return `
    <button
      type="button"
      class="talent-slot ${column}${empty ? " empty" : ""}${editing ? " editing" : ""}${disabledByLevel ? " locked-by-level" : ""}"
      data-action="select-talent-slot"
      data-side="${side}"
      data-slot="${index}"
      ${title ? `title="${title}"` : ""}
      ${disabledByLevel ? "disabled" : ""}
    >
      <span class="talent-slot-name">${name}</span>
    </button>
  `;
}

export function renderFateStrategyStrip(state: AppState, side: Side, player: PlayerConfig): string {
  const options = fateStrategyOptionsForCharacter(player.characterId);
  if (options.length === 0) return "";
  const implementedOptions = options.filter(isFateStrategyImplemented);
  const optionById = new Map(options.map((option) => [option.id, option] as const));
  const selected = player.fateStrategies
    .map((id) => optionById.get(id))
    .filter((option): option is FateStrategyOption => option !== undefined);
  const editing = state.pickerMode === "fate" && state.activeSide === side;
  return `
    <div class="season-fate-strip ${editing ? "editing" : ""}">
      <button type="button" class="season-fate-open" data-action="open-fate-picker" data-side="${side}">
        <span class="season-fate-icon"></span>
        <span class="season-fate-title">天衍</span>
        <span class="season-fate-count">${selected.length}/${implementedOptions.length}</span>
      </button>
      <div class="season-fate-chips">
        ${selected.length === 0
          ? ""
          : selected.map((option) => `
            <button type="button" class="season-fate-chip" data-action="toggle-fate-strategy" data-side="${side}" data-fate-strategy-id="${option.id}" title="取消">${escapeHtml(fateStrategyDisplayName(option))}</button>
          `).join("")}
      </div>
    </div>
  `;
}

function renderLockedTalentControls(side: Side, player: PlayerConfig): string {
  if (player.characterId !== JI_FANGSHENG_CHARACTER_ID) return "";
  const maxRank = maxJiFangshengInitialFateRank(player.gameRound);
  const rank = player.jiFangshengInitialFateRank;
  return `
    <span class="locked-talent-rank" aria-label="档位">
      <button type="button" data-action="adjust-jifangsheng-rank" data-side="${side}" data-delta="-1" ${rank <= 0 ? "disabled" : ""}>▼</button>
      <b>${rank}</b>
      <button type="button" data-action="adjust-jifangsheng-rank" data-side="${side}" data-delta="1" ${rank >= maxRank ? "disabled" : ""}>▲</button>
    </span>
  `;
}

function talentPickerClass(talentId: number): string {
  const option = TALENT_OPTION_BY_ID.get(talentId);
  if (!option) return "";
  return talentPickerColumn(option) ?? "";
}
