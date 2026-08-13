import {
  ELEMENT_OPTIONS,
} from "./data";
import { loadSelectedBuild } from "./main-storage";
import { activeTargetBuild } from "./main-actions";
import { GAME_TURN_LIMIT } from "./target-dummy";
import {
  applyGameRoundDefaults,
  applyPhysiqueValue,
  clampGameRound,
  invalidateAllTargetBuilds,
  invalidateComputedResults,
  normalizeBattleConfig,
  resyncTargetMirror,
  resetCardFilters,
  syncPlayerDerivedStats,
} from "./main-utils";
import {
  numericValue,
  parseJsonRecord,
  parseNumberList,
} from "./view-utils";
import type { AppState, PlayerConfig, Side } from "./types";

const PHYSIQUE_BUFF = "physique";

export type FieldContext = {
  readonly state: AppState;
  readonly render: () => void;
};

const WHEEL_NUMBER_SELECTOR = "#battle-gameRound, input[type='number'][id^='player-'][id$='-lifeModifier'], input[type='number'][data-buff='physique'], #battle-targetThreshold";

export function bindWheelNumberInputs(root: ParentNode): void {
  root.querySelectorAll<HTMLInputElement>(WHEEL_NUMBER_SELECTOR).forEach((input) => {
    input.addEventListener("wheel", (event) => {
      event.preventDefault();
      const step = event.deltaY < 0 ? 1 : -1;
      const min = input.min === "" ? Number.NEGATIVE_INFINITY : Number(input.min);
      const max = input.max === "" ? Number.POSITIVE_INFINITY : Number(input.max);
      const current = Number(input.value);
      const base = Number.isFinite(current) ? current : 0;
      const next = Math.min(max, Math.max(min, base + step));
      if (next === base) return;
      input.value = String(next);
      input.dispatchEvent(new Event("change", { bubbles: true }));
    }, { passive: false });
  });
}

export function handleSaveNameInput(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.saveName as Side | undefined;
  if (!side) return;
  context.state.saveDraftNames[side] = target.value;
}

export function handleBuildArchivePick(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.buildArchive as Side | undefined;
  if (!side) return;
  const name = target.value.trim();
  context.state.saveDraftNames[side] = target.value;
  const build = context.state.savedBuilds.find((entry) => entry.name === name);
  if (!build) {
    context.state.selectedBuildIds[side] = "";
    return;
  }
  context.state.selectedBuildIds[side] = build.id;
  loadSelectedBuild(context.state, side);
  // 读档会整体替换 config.players[side]：打靶模式下绕过 handleAction 收口，
  // 必须在这里重挂镜像，否则后续编辑写丢、构筑签名不更新。
  resyncTargetMirror(context.state);
  context.render();
}

export function handleNamedField(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement;
  const { state } = context;
  const id = target.id;
  try {
    if (id === "battle-first") state.config.firstPlayerSide = target.value as Side;
    if (id === "battle-gameRound") {
      state.config.gameRound = clampGameRound(numericValue(target));
      applyGameRoundDefaults(state.config);
    }
    if (id === "battle-targetThreshold" && state.target) {
      state.target.damageThreshold = clampTargetParam(numericValue(target), 1, 9999);
    }
    // 绝对展示回合数：有结果时有效范围是 [reachedTurn, 32]，无结果时用初始 1
    // 占位（控件本身应显示禁用等待态，由调用方按是否有结果切换）。
    if (id === "battle-targetDisplayRounds" && state.target) {
      const targetState = state.target;
      const active = activeTargetBuild(state);
      const reachedTurn = active?.result?.reachedTurn ?? targetState.displayRounds;
      const min = Math.min(Math.max(1, reachedTurn), GAME_TURN_LIMIT);
      const requested = clampTargetParam(numericValue(target), min, GAME_TURN_LIMIT);
      targetState.displayRounds = requested;
      targetState.displayRoundMin = min;
      targetState.displayRoundPending = true;
    }
    if (id === "battle-maxActorTurns") state.config.maxActorTurns = numericValue(target);
    if (id === "battle-decisionTape") state.config.decisionTape = parseNumberList(target.value);
    if (id === "battle-randomFallbackTape") state.config.randomFallbackTape = parseNumberList(target.value);
    for (const side of ["p1", "p2"] as const) {
      const prefix = `player-${side}-`;
      if (!id.startsWith(prefix)) continue;
      setPlayerNamedField(state, side, id.slice(prefix.length), target.value, target);
    }
    invalidateComputedResults(state);
    // 阈值/显示至回合/修炼轮是所有构筑共享的参数：作废全部结果并触发全部重跑，
    // generic 失效只动聚焦构筑。
    if (
      state.workbenchMode === "target" &&
      (id === "battle-targetThreshold" || id === "battle-targetDisplayRounds" || id === "battle-gameRound")
    ) {
      invalidateAllTargetBuilds(state);
    }
    state.error = null;
  } catch (error) {
    state.error = error instanceof Error ? error.message : String(error);
  }
  context.render();
}

function clampTargetParam(value: number, min: number, max: number): number {
  if (!Number.isFinite(value)) return min;
  return Math.min(max, Math.max(min, Math.trunc(value)));
}

export function handleElements(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.side as Side;
  const element = target.dataset.element as (typeof ELEMENT_OPTIONS)[number];
  const elements = new Set(context.state.config.players[side].activatedElements);
  if (target.checked) elements.add(element);
  else elements.delete(element);
  context.state.config.players[side].activatedElements = [...elements];
  invalidateComputedResults(context.state);
  context.render();
}

export function handleIdListCheckbox(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.side as Side;
  const field = target.dataset.idList as "talents" | "fateStrategies";
  const values = new Set(context.state.config.players[side][field]);
  const id = Number(target.value);
  if (target.checked) values.add(id);
  else values.delete(id);
  context.state.config.players[side][field] = [...values].sort((left, right) => left - right);
  invalidateComputedResults(context.state);
  context.render();
}

export function handleBuffInput(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.side as Side;
  const buff = target.dataset.buff!;
  const value = Number(target.value);
  const player = context.state.config.players[side];
  if (buff === PHYSIQUE_BUFF) {
    applyPhysiqueValue(player, Number.isNaN(value) ? 0 : value);
  } else if (value === 0 || Number.isNaN(value)) delete player.buffs[buff];
  else player.buffs[buff] = value;
  invalidateComputedResults(context.state);
  context.render();
}

export function handlePermanentBuffInput(event: Event, context: FieldContext): void {
  const target = event.currentTarget as HTMLInputElement;
  const side = target.dataset.side as Side | undefined;
  const key = target.dataset.permanentBuff;
  if (!side || !key) return;
  const value = Math.max(0, numericValue(target));
  const player = context.state.config.players[side];
  if (value > 0) player.permanentBuffTempDatas[key] = value;
  else delete player.permanentBuffTempDatas[key];
  context.state.activeSide = side;
  invalidateComputedResults(context.state);
  context.render();
}

function setPlayerNamedField(
  state: AppState,
  side: Side,
  key: string,
  value: string,
  target: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement,
): void {
  const player = state.config.players[side];
  if (key === "label") player.label = value;
  else if (key === "lastElement") player.lastElement = value.trim() === "" ? null : (value as PlayerConfig["lastElement"]);
  else if (key === "talentResonanceId") player.talentResonanceId = value.trim() === "" ? null : Number(value);
  else if (["starSlots", "fateStrategies", "lingWuCardBaseIds", "handCardIds", "lastRoundUsedCardBaseIds"].includes(key)) {
    (player as unknown as Record<string, number[]>)[key] = parseNumberList(value);
  } else if (["talentCardParams", "talentTempDatas", "permanentBuffTempDatas"].includes(key)) {
    (player as unknown as Record<string, unknown>)[key] = parseJsonRecord(value);
  } else if (key in player) {
    const nextValue = numericValue(target);
    (player as unknown as Record<string, number>)[key] = nextValue;
    if (key === "lifeModifier" || key === "level") {
      syncPlayerDerivedStats(player, state.config.gameRound, false);
    }
  }
}
