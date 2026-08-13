import {
  CARD_OPTION_BY_BASE_ID,
  cardDerivationTalentIds,
  cardDetailText,
  cardSlotLevelLabel,
  derivedCardOption,
  sideLabel,
  talentDetailText,
} from "./data";
import { adaptOriginalCardType } from "./domain";
import { renderCardFace } from "./render-card-face";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState, BattleFrame, CardOption, PlayerConfig, PlayerView, Side } from "./types";

export function renderDeckToolbar(state: AppState, side: Side): string {
  const slot = state.activeSide === side ? state.selectedSlot : 0;
  const canShiftLeft = slot > 0;
  const canShiftRight = slot < 7;
  return `
    <div class="deck-toolbar" aria-label="卡组微调">
      <span class="deck-toolbar-label">卡组</span>
      <div class="deck-toolbar-actions">
        <button type="button" class="deck-tool" data-action="shift-deck-slot" data-side="${side}" data-slot="${slot}" data-delta="-1" title="与左侧卡槽交换" ${canShiftLeft ? "" : "disabled"}>◀</button>
        <button type="button" class="deck-tool" data-action="shift-deck-slot" data-side="${side}" data-slot="${slot}" data-delta="1" title="与右侧卡槽交换" ${canShiftRight ? "" : "disabled"}>▶</button>
        <button type="button" class="deck-tool" data-action="clear-slot" data-side="${side}" data-slot="${slot}" title="清空当前格">清格</button>
        <button type="button" class="deck-tool" data-action="clear-deck" data-side="${side}" title="清空全部卡槽">清空</button>
      </div>
      <span class="deck-toolbar-hint">第 ${slot + 1} 格 · 拖中间交换 / 拖边缘插入 · 右键清格</span>
    </div>
  `;
}

export function renderPlayerDeck(options: {
  readonly state: AppState;
  readonly side: Side;
  readonly player: PlayerConfig;
  readonly frame: BattleFrame | null;
  readonly runtime?: PlayerView;
  readonly battleActiveSlots?: readonly number[];
}): string {
  return `
    <div class="player-deck" aria-label="构筑牌组">
      ${options.player.deck.map((slot, index) => renderDeckSlot({ ...options, configSlot: slot, index })).join("")}
    </div>
  `;
}

function renderDeckSlot(options: {
  readonly state: AppState;
  readonly side: Side;
  readonly player: PlayerConfig;
  readonly frame: BattleFrame | null;
  readonly runtime?: PlayerView;
  readonly battleActiveSlots?: readonly number[];
  readonly configSlot: PlayerConfig["deck"][number];
  readonly index: number;
}): string {
  const runtimeSlot = options.runtime?.slots[options.index];
  const isEmpty = options.configSlot.baseId === 0 && !runtimeSlot?.cardId;
  const configuredCard = isEmpty
    ? null
    : CARD_OPTION_BY_BASE_ID.get(options.configSlot.baseId)
      ?? originalConfigCardOption(options.configSlot);
  const runtimeCard = runtimeSlot && runtimeSlot.cardId !== 0
    ? CARD_OPTION_BY_BASE_ID.get(runtimeSlot.baseId) ?? runtimeCardOption(runtimeSlot)
    : null;
  const rawCard = runtimeCard ?? configuredCard;
  const card = rawCard ? derivedCardOption(rawCard, options.player.talents) : null;
  const runtimeLevel = runtimeCard?.variants.find((variant) => variant.id === runtimeSlot?.cardId)?.rarity;
  const faceLevel = runtimeLevel ?? options.configSlot.level;
  const editing = options.state.pickerMode === "card"
    && options.state.activeSide === options.side
    && options.state.selectedSlot === options.index;
  const inactive = options.index >= options.player.activeSlotCount;
  const activeSlots = options.battleActiveSlots
    ?? (options.frame?.sourceSlot === null || options.frame?.sourceSlot === undefined
      ? []
      : [options.frame.sourceSlot]);
  const battleActive = Boolean(
    options.frame
    && options.runtime
    && options.frame.actorId === options.runtime.id
    && activeSlots.includes(options.index),
  );
  const battleClasses = [
    runtimeSlot?.skipped ? "skipped" : "",
    runtimeSlot?.temporarilyUpgraded ? "temporarily-upgraded" : "",
    battleActive ? "active" : "",
    runtimeSlot?.hadUsed ? "used" : "",
  ].filter(Boolean).join(" ");
  const actionAttrs = [
    `data-action="select-slot"`,
    `data-side="${options.side}"`,
    `data-slot="${options.index}"`,
    isEmpty ? "" : `draggable="true"`,
    isEmpty ? "" : `data-deck-drag-handle="1"`,
    `aria-label="${sideLabel(options.side)}第 ${options.index + 1} 格"`,
  ].filter(Boolean).join(" ");
  const faceState = battleActive ? "active" : runtimeSlot?.skipped ? "skipped" : runtimeSlot?.hadUsed ? "used" : editing ? "editing" : "normal";
  return `
    <div
      class="deck-slot${isEmpty ? " empty" : ""}${editing ? " editing" : ""}${inactive ? " inactive" : ""}${battleClasses ? ` ${battleClasses}` : ""}"
      data-slot="${options.index}"
    >
      ${renderCardFace({
        as: "button",
        card,
        level: isEmpty ? undefined : faceLevel,
        subLabel: runtimeSlot?.temporarilyUpgraded ? "临时升级" : undefined,
        showLevelDots: false,
        empty: isEmpty,
        inactive,
        selected: editing,
        state: faceState,
        title: card
          ? playerCardDetailText(
              card,
              options.player,
              runtimeSlot
                && (
                  runtimeSlot.baseId !== options.configSlot.baseId
                  || (
                    options.configSlot.originalConfig !== undefined
                    && runtimeSlot.cardId !== options.configSlot.originalConfig.id
                  )
                )
                ? undefined
                : options.configSlot.originalConfig,
            )
          : undefined,
        actionAttrs,
        disabled: inactive,
      })}
      ${isEmpty || inactive || card?.variantMode === "single" ? "" : renderSlotLevelControls(options.side, options.index, options.configSlot.level, card)}
    </div>
  `;
}

function runtimeCardOption(slot: PlayerView["slots"][number]): CardOption {
  return {
    baseId: slot.baseId,
    name: slot.name,
    group: "战斗运行态",
    groupType: "runtime",
    implemented: true,
    variantMode: "single",
    archiveKind: "common",
    archiveKey: "runtime",
    archiveLabel: "战斗运行态",
    type: "",
    desc: "战斗中被替换或生成的当前牌。",
    variants: [],
  };
}

function playerCardDetailText(
  card: CardOption,
  player: PlayerConfig,
  originalConfig?: PlayerConfig["deck"][number]["originalConfig"],
): string {
  const base = cardDetailText(card, originalConfig);
  const derivations = cardDerivationTalentIds(card.baseId, player.talents)
    .map(talentDetailText)
    .filter((detail) => detail !== "");
  return derivations.length > 0 ? `${base}\n\n仙命派生\n${derivations.join("\n")}` : base;
}

function originalConfigCardOption(slot: PlayerConfig["deck"][number]): CardOption | null {
  if (!slot.originalConfig) return null;
  const name = slot.originalConfig.name || `原始牌 ${slot.originalConfig.id}`;
  return {
    baseId: slot.baseId,
    name,
    group: "原始回放",
    groupType: "fixture",
    implemented: true,
    variantMode: "single",
    archiveKind: "common",
    archiveKey: "fixture",
    archiveLabel: "原始回放",
    type: adaptOriginalCardType(slot.originalConfig.cardType),
    desc: slot.originalConfig.desc ?? "",
    variants: [],
  };
}

function renderSlotLevelControls(side: Side, index: number, level: number, card: CardOption): string {
  const label = cardSlotLevelLabel(card, level);
  return `
    <div class="slot-level-controls level-${level}" aria-label="卡牌等级">
      <button
        type="button"
        class="slot-level-cycle"
        data-action="cycle-level"
        data-side="${side}"
        data-slot="${index}"
        title="${escapeAttribute(`${label}，点击切换等级`)}"
        aria-label="${escapeAttribute(`${label}，点击切换等级`)}"
      >${escapeHtml(label)}</button>
    </div>
  `;
}
