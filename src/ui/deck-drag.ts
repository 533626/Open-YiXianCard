import {
  invalidateComputedResults,
  reorderDeckSlot,
} from "./main-utils";
import type { AppState, Side } from "./types";

type DeckDropMode = "swap" | "insert-before" | "insert-after";

export function bindDeckDrag(
  app: HTMLElement,
  state: AppState,
  render: () => void,
): void {
  let dragged: { readonly side: Side; readonly slot: number } | null = null;
  const handles = app.querySelectorAll<HTMLElement>("[data-deck-drag-handle]");
  const slots = app.querySelectorAll<HTMLElement>(".player-deck .deck-slot");
  const clearDropClasses = (): void => {
    slots.forEach((slot) => {
      slot.classList.remove("drag-source", "drop-swap", "drop-before", "drop-after");
    });
    app.querySelectorAll(".player-deck.drag-active").forEach((deck) =>
      deck.classList.remove("drag-active"));
  };

  handles.forEach((handle) => {
    handle.addEventListener("dragstart", (event) => {
      const side = handle.dataset.side as Side | undefined;
      const slot = handle.dataset.slot === undefined ? null : Number(handle.dataset.slot);
      if (!side || slot === null || !Number.isInteger(slot)) {
        event.preventDefault();
        return;
      }
      dragged = { side, slot };
      handle.closest(".deck-slot")?.classList.add("drag-source");
      handle.closest(".player-deck")?.classList.add("drag-active");
      if (event.dataTransfer) {
        event.dataTransfer.effectAllowed = "move";
        event.dataTransfer.setData("text/plain", `${side}:${slot}`);
      }
    });
    handle.addEventListener("dragend", () => {
      dragged = null;
      clearDropClasses();
    });
  });

  slots.forEach((slotElement) => {
    slotElement.addEventListener("dragover", (event) => {
      const targetSide = playerSide(slotElement);
      if (!dragged || targetSide !== dragged.side) return;
      event.preventDefault();
      if (event.dataTransfer) event.dataTransfer.dropEffect = "move";
      slotElement.classList.remove("drop-swap", "drop-before", "drop-after");
      slotElement.classList.add(`drop-${dropMode(event, slotElement).replace("insert-", "")}`);
    });
    slotElement.addEventListener("dragleave", (event) => {
      if (event.relatedTarget instanceof Node && slotElement.contains(event.relatedTarget)) return;
      slotElement.classList.remove("drop-swap", "drop-before", "drop-after");
    });
    slotElement.addEventListener("drop", (event) => {
      const payload = dragged;
      const target = Number(slotElement.dataset.slot);
      if (!payload || playerSide(slotElement) !== payload.side || !Number.isInteger(target)) return;
      event.preventDefault();
      const selectedSlot = reorderDeckSlot(
        state,
        payload.side,
        payload.slot,
        target,
        dropMode(event, slotElement),
      );
      dragged = null;
      clearDropClasses();
      if (selectedSlot === null) return;
      invalidateComputedResults(state);
      state.activeSide = payload.side;
      state.selectedSlot = selectedSlot;
      state.pickerMode = "none";
      render();
    });
  });
}

function playerSide(slot: HTMLElement): Side | undefined {
  return slot.closest<HTMLElement>(".player-panel")?.dataset.side as Side | undefined;
}

function dropMode(event: DragEvent, slot: HTMLElement): DeckDropMode {
  const { left, width } = slot.getBoundingClientRect();
  const ratio = width <= 0 ? 0.5 : (event.clientX - left) / width;
  if (ratio < 0.24) return "insert-before";
  if (ratio > 0.76) return "insert-after";
  return "swap";
}
