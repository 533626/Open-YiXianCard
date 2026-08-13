import {
  CAREER_OPTIONS,
  slotHasDualCareerTalent,
} from "./data";
import { physiqueLimitForPlayer } from "./derived-state";
import {
  escapeAttribute,
  escapeHtml,
} from "./view-utils";
import { CoreBuff } from "./domain";
import { renderBattleStatRibbon } from "./player-battle-state";
import type { AppState, PlayerConfig, Side } from "./types";

const PHYSIQUE_BUFF = CoreBuff.Physique;

const LINGZHI_CAREER_ID = "LingZhiShi";
const LINGZHI_REFINES = [
  { id: "10008", name: "生命上限", effect: "自身生命上限与当前生命" },
  { id: "10010", name: "恢复", effect: "自身恢复" },
  { id: "10011", name: "加攻", effect: "自身加攻" },
  { id: "10013", name: "内伤", effect: "对方内伤" },
  { id: "10009", name: "开局伤害", effect: "对方失去生命" },
  { id: "10014", name: "防", effect: "自身防" },
  { id: "10017", name: "削上限", effect: "对方生命上限减少" },
  { id: "10020", name: "影枭灵芝", effect: "自身失去生命并再次行动" },
] as const;

export function renderSetupToolbar(
  state: AppState,
  side: Side,
  player: PlayerConfig,
): string {
  return `
    <div class="player-setup-tools">
      ${renderCareerSummary(side, player)}
      ${renderBuildSaveRow(state, side)}
    </div>
  `;
}

export function renderSetupBody(
  state: AppState,
  side: Side,
  player: PlayerConfig,
  runtime: import("./types").PlayerView | null = null,
  previousRuntime: import("./types").PlayerView | null = null,
): string {
  return `
    <div class="player-panel-setup">
      ${
    runtime
      ? `<div class="player-slot-status">${
        renderBattleStatRibbon(runtime, {
          inline: true,
          includeHp: false,
          previous: previousRuntime,
        })
      }</div>`
      : ""
  }
      ${renderLingzhiRefinePanel(side, player)}
    </div>
  `;
}

export function renderPhysiqueField(side: Side, player: PlayerConfig): string {
  const value = player.buffs[PHYSIQUE_BUFF] ?? 0;
  const limit = player.buffs[CoreBuff.PhysiqueLimit] ?? physiqueLimitForPlayer(player);
  return `
    <label class="field physique-field opening-physique-field">
      <span>体魄</span>
      <div class="field-with-suffix">
        <input
          type="number"
          value="${value}"
          min="0"
          max="${limit}"
          data-buff="${escapeAttribute(PHYSIQUE_BUFF)}"
          data-side="${side}"
        />
        <b>/${limit}</b>
      </div>
    </label>
  `;
}

function renderBuildSaveRow(state: AppState, side: Side): string {
  const selectedId = state.selectedBuildIds[side];
  const canDelete = Boolean(selectedId || state.savedBuilds.some((build) => build.name === state.saveDraftNames[side].trim()));
  return `
    <div class="build-toolbar">
      <div class="build-archive-wrap">
        <input
          type="text"
          class="build-archive-bar"
          value="${escapeAttribute(state.saveDraftNames[side])}"
          placeholder="存档名 / 选择已有"
          aria-label="存档"
          autocomplete="off"
          data-save-name="${side}"
          data-build-archive="${side}"
        />
        <button
          type="button"
          class="build-archive-toggle"
          data-action="toggle-build-archive"
          data-side="${side}"
          aria-label="展开存档列表"
          title="选择已有存档"
        >▾</button>
        <div class="build-archive-menu" role="listbox" aria-label="已有存档">
          ${state.savedBuilds.length === 0
            ? `<div class="build-archive-empty">暂无存档</div>`
            : state.savedBuilds.map((build) => `
              <button
                type="button"
                class="build-archive-option${build.id === selectedId ? " selected" : ""}"
                data-action="pick-saved-build"
                data-side="${side}"
                data-build-id="${escapeAttribute(build.id)}"
                role="option"
                ${build.id === selectedId ? 'aria-selected="true"' : ""}
              >${escapeHtml(build.name)}</button>
            `).join("")}
        </div>
      </div>
      <button type="button" class="build-action save" data-action="save-build" data-side="${side}" title="保存当前构筑">存</button>
      <button type="button" class="build-action delete" data-action="delete-build" data-side="${side}" ${canDelete ? "" : "disabled"} title="删除选中存档">删</button>
    </div>
  `;
}

function renderCareerSummary(side: Side, player: PlayerConfig): string {
  const careerName = CAREER_OPTIONS.find((c) => c.id === player.careerName)?.name ?? "炼丹师";
  const duals = [1, 2, 3, 4]
    .filter((slot) => slotHasDualCareerTalent(player, slot) && player.dualCareerNames[slot])
    .map((slot) => {
      const slotLabel = slot === 1 ? "筑基" : slot === 2 ? "金丹" : slot === 3 ? "元婴" : "化神";
      const name = CAREER_OPTIONS.find((c) => c.id === player.dualCareerNames[slot])?.name ?? "";
      return `${slotLabel}:${name}`;
    });
  const dualText = duals.length > 0 ? `+${duals.join(" ")}` : "";
  return `
    <button type="button" class="setup-career-trigger" data-action="set-picker-mode" data-mode="career" data-side="${side}" title="选择副职">
      <span class="setup-field-label">副职</span>
      <span class="setup-career-name">${escapeHtml(careerName)}${escapeHtml(dualText)}</span>
    </button>
  `;
}

function buffNumberField(label: string, side: Side, buff: string, value: number): string {
  return `
    <label class="field">
      <span>${label}</span>
      <input type="number" value="${value}" min="0" max="9999" data-side="${side}" data-buff="${escapeAttribute(buff)}" />
    </label>
  `;
}

function renderLingzhiRefinePanel(side: Side, player: PlayerConfig): string {
  if (player.careerName !== LINGZHI_CAREER_ID) return "";
  return `
    <div class="lingzhi-refine-panel">
      <div class="sub-row-title">
        <b>灵植炼化</b>
        <span>写入战前永久效果</span>
      </div>
      <div class="lingzhi-refine-grid">
        ${LINGZHI_REFINES.map((item) => `
          <label class="field lingzhi-refine-field" title="${escapeAttribute(item.effect)}">
            <span>${escapeHtml(item.name)}</span>
            <input
              type="number"
              min="0"
              max="9999"
              value="${player.permanentBuffTempDatas[item.id] ?? 0}"
              data-side="${side}"
              data-permanent-buff="${item.id}"
            />
          </label>
        `).join("")}
      </div>
    </div>
  `;
}
