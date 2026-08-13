import { battleStepShortcutDirection } from "./battle-keyboard";
import { clearDeckSlot } from "./main-utils";
import type { LevelControl } from "./main-level-control";
import { isTextEditingTarget } from "./main-state";
import type { AppState } from "./types";

export interface MainKeyboardContext {
  readonly app: HTMLElement;
  readonly state: AppState;
  readonly levelControl: LevelControl;
  readonly render: () => void;
  readonly cancelBattle: () => void;
  readonly maybeScheduleAutoBattle: () => void;
  readonly stepBattleFrame: (direction: -1 | 1) => void;
}

export function handleMainKeyDown(event: KeyboardEvent, context: MainKeyboardContext): void {
  const { app, state } = context;
  if (event.key === "Escape" && state.pickerMode !== "none") {
    event.preventDefault();
    state.pickerMode = "none";
    context.render();
    context.maybeScheduleAutoBattle();
    return;
  }
  if (isTextEditingTarget(event.target)) return;
  if (event.key === "Escape" && state.battleStatus?.state === "running") {
    event.preventDefault();
    context.cancelBattle();
    return;
  }
  if (event.key === "Escape" && state.solverStatus?.state === "running") {
    event.preventDefault();
    app.querySelector<HTMLButtonElement>("[data-action='cancel-solver']")?.click();
    return;
  }
  if (state.pickerMode === "card") {
    const player = state.config.players[state.activeSide];
    if (event.key === "Delete" || event.key === "Backspace") {
      event.preventDefault();
      clearDeckSlot(state, state.activeSide, state.selectedSlot);
      context.render();
      return;
    }
    if (event.key === "1" || event.key === "2" || event.key === "3") {
      event.preventDefault();
      player.deck[state.selectedSlot]!.level = Number(event.key) - 1;
      context.levelControl.patch(state.activeSide, state.selectedSlot);
      context.levelControl.schedule();
      return;
    }
    if (event.key === "/") {
      event.preventDefault();
      app.querySelector<HTMLInputElement>("#cardSearch")?.focus();
      return;
    }
  }
  const direction = battleStepShortcutDirection(event.key, state, false);
  if (direction !== null) {
    event.preventDefault();
    context.stepBattleFrame(direction);
  }
}
