import { cardSlotLevelLabel, cardVisualType } from "./data";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { CardOption } from "./types";

export type CardFaceState = "normal" | "editing" | "active" | "used" | "skipped" | "disabled";

export interface CardFaceOptions {
  readonly card?: CardOption | null;
  readonly level?: number;
  readonly subLabel?: string;
  readonly empty?: boolean;
  readonly inactive?: boolean;
  readonly selected?: boolean;
  readonly unimplemented?: boolean;
  readonly state?: CardFaceState;
  readonly title?: string;
  readonly actionAttrs?: string;
  readonly disabled?: boolean;
  readonly as?: "button" | "div";
  readonly showLevelDots?: boolean;
}

export function renderCardFace(options: CardFaceOptions): string {
  const tag = options.as ?? "button";
  const card = options.card ?? null;
  const empty = options.empty ?? !card;
  const visualType = card ? cardVisualType(card) : "normal";
  const cardTypeClass = card && !empty ? `type-${card.type}` : "";
  const classes = [
    "card-face",
    visualType,
    cardTypeClass,
    empty ? "empty" : "",
    options.inactive ? "inactive" : "",
    options.selected ? "selected" : "",
    options.unimplemented ? "unimplemented" : "",
    options.state ? `state-${options.state}` : "",
    options.level !== undefined ? `level-${options.level}` : "",
  ].filter(Boolean).join(" ");
  const attrs = [
    `class="${classes}"`,
    options.title ? `title="${escapeAttribute(options.title)}"` : "",
    options.actionAttrs ?? "",
    options.disabled ? "disabled" : "",
  ].filter(Boolean).join(" ");
  return `<${tag} ${attrs}>${renderCardFaceInner(options, empty, card)}</${tag}>`;
}

function renderCardFaceInner(options: CardFaceOptions, empty: boolean, card: CardOption | null): string {
  const showLevel = options.level !== undefined && options.showLevelDots !== false;
  const level = options.level ?? 0;
  const name = empty ? "" : escapeHtml(card?.name ?? "");
  const subLabel = options.subLabel ? `<span class="card-face-sub">${escapeHtml(options.subLabel)}</span>` : "";
  // 梦境卡按境界分级（5 档），用文字标签展示境界；单变体卡不展示等级；普通卡按阶位（3 档）用圆点。
  const variantMode = card?.variantMode;
  const isRealmCard = variantMode === "realm";
  const isSingleCard = variantMode === "single";
  const levelLabel = card && isRealmCard ? cardSlotLevelLabel(card, level) : "";
  const dots = [0, 1, 2].map((dot) =>
    `<span class="card-face-dot${dot <= level ? " filled" : ""}"></span>`
  ).join("");
  const levelMarkup = isRealmCard
    ? `<span class="card-face-level card-face-realm" aria-label="境界">${escapeHtml(levelLabel)}</span>`
    : `<span class="card-face-level" aria-label="等级">${dots}</span>`;
  const showLevelMarkup = showLevel && !isSingleCard;
  return `
    <span class="card-face-type"></span>
    <span class="card-face-body">
      <span class="card-face-name">${name}</span>
      ${subLabel}
    </span>
    ${empty ? `<span class="card-face-plus">+</span>` : ""}
    ${empty || !showLevelMarkup ? "" : levelMarkup}
  `;
}
