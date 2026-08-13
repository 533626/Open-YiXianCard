import { CARD_OPTION_BY_BASE_ID, cardSlotLevelLabel } from "./data";
import type { AppState, Side } from "./types";

const LEVEL_DEBOUNCE_MS = 350;

export interface LevelControl {
  cancel(): void;
  patch(side: Side, slot: number): void;
  schedule(): void;
}

export function createLevelControl(
  app: HTMLElement,
  state: AppState,
  flush: () => void,
): LevelControl {
  let timer: ReturnType<typeof setTimeout> | null = null;
  return {
    cancel() {
      if (timer) clearTimeout(timer);
      timer = null;
    },
    patch(side, slot) {
      const deckSlot = state.config.players[side].deck[slot];
      if (!deckSlot) return;
      const button = app.querySelector<HTMLElement>(
        `.slot-level-cycle[data-action='cycle-level'][data-side='${side}'][data-slot='${slot}']`,
      );
      const card = CARD_OPTION_BY_BASE_ID.get(deckSlot.baseId);
      if (!button || !card) return;
      const label = cardSlotLevelLabel(card, deckSlot.level);
      button.textContent = label;
      button.title = `${label}，点击切换等级`;
      button.setAttribute("aria-label", `${label}，点击切换等级`);
      const controls = button.closest(".slot-level-controls");
      if (controls) controls.className = `slot-level-controls level-${deckSlot.level}`;
      const cardFace = button.closest(".deck-slot")?.querySelector<HTMLElement>(".card-face");
      if (cardFace) {
        cardFace.className = cardFace.className.replace(/\blevel-\d+\b/g, "").trim();
        cardFace.classList.add(`level-${deckSlot.level}`);
      }
    },
    schedule() {
      if (timer) clearTimeout(timer);
      timer = setTimeout(() => {
        timer = null;
        flush();
      }, LEVEL_DEBOUNCE_MS);
    },
  };
}
