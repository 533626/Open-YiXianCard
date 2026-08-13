import {
  CAREER_OPTIONS,
  EMPTY_CHARACTER_ID,
  canPickCardForDeckSlot,
  cardDetailText,
  cardDerivationTalentIds,
  cardSeriesKey,
  cardsGroupedForPicker,
  characterBaseTalentSlots,
  characterGroups,
  deckUsageCounts,
  derivedCardOption,
  fateStrategyDisplayName,
  fateStrategyGroupsForCharacter,
  fateStrategySummary,
  isBattleIrrelevantTalent,
  isCardDisabled,
  isFateStrategyImplemented,
  isTalentMissingBattle,
  normalizePlayerTalents,
  scopedCardIndexOptions,
  slotHasDualCareerTalent,
  talentChoiceGroupsForSlot,
  talentDetailText,
} from "./data";
import type { CardPickerGroup } from "./data";
import { renderCardFace } from "./render-card-face";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type {
  AppState,
  CardOption,
  FateStrategyGroup,
  FateStrategyOption,
  Side,
  TalentOption,
  TalentSlotOption,
} from "./types";

export function renderCardPopup(state: AppState): string {
  if (state.pickerMode !== "card") return "";
  const side = state.activeSide;
  const player = state.config.players[side];
  const scopedCards = scopedCardIndexOptions(player.characterId, player.careerName, player.talents, player.dualCareerNames)
    .map((card) => derivedCardOption(card, player.talents));
  const usage = deckUsageCounts(player.deck);
  // 副职牌按已选副职（主副职 + 副职兼修仙命开出的兼修副职）分组，避免两个副职的牌混在一组。
  const allGroups = cardsGroupedForPicker(filterCards(state, scopedCards), {
    primary: player.careerName,
    duals: player.dualCareerNames,
  });
  const scope = state.cardPickerScope ?? "common";
  const groups = state.cardSearch.trim()
    ? allGroups
    : allGroups.filter((group) => cardPickerGroupScope(group.id) === scope);
  return `
    <div class="picker-popup-backdrop" data-action="close-card-picker"></div>
    <section class="picker-popup build-picker-popup card-popup" aria-label="构筑选择">
      <div class="picker-popup-head">
        <div class="card-picker-heading">
          ${pickerPopupTitle(side, "构筑选择", "卡牌")}
          ${renderBuildPickerTabs(state)}
        </div>
        <div class="picker-popup-tools">
          <input
            class="card-picker-search picker-search"
            type="search"
            id="cardSearch"
            name="cardSearch"
            aria-label="搜卡"
            data-picker-search="card"
            value="${escapeAttribute(state.cardSearch)}"
            placeholder="搜索"
          />
          ${pickerCloseButton("close-card-picker")}
        </div>
      </div>
      <nav class="card-picker-scopes" aria-label="卡牌范围">
        ${renderCardPickerScope("common", "常用", scope)}
        ${renderCardPickerScope("season", "赛季", scope)}
        ${renderCardPickerScope("special", "特殊", scope)}
      </nav>
      ${groups.length === 0
        ? `<div class="empty-picker-note">无可选卡牌</div>`
        : `
          <div class="card-picker-library" data-card-picker-scroll="1">
            ${groups.map((group) => renderCardPickerSection(state, group, usage)).join("")}
          </div>
        `}
    </section>
  `;
}

function renderCardPickerScope(
  value: "common" | "season" | "special",
  label: string,
  selected: string,
): string {
  return `<button
    type="button"
    class="card-picker-scope${selected === value ? " selected" : ""}"
    data-action="set-card-picker-scope"
    data-scope="${value}"
    aria-pressed="${selected === value}"
  >${label}</button>`;
}

function cardPickerGroupScope(groupId: string): "common" | "season" | "special" {
  if (groupId.startsWith("season-")) return "season";
  if (groupId.startsWith("chance-") || groupId === "secret") return "special";
  return "common";
}

export function renderTalentPopup(state: AppState): string {
  if (state.pickerMode !== "talent" || state.selectedTalentSlot <= 0) return "";
  const side = state.activeSide;
  const player = state.config.players[side];
  normalizePlayerTalents(player);
  const query = (state.pickerSearch ?? "").trim().toLowerCase();
  const baseSlots = characterBaseTalentSlots(player.characterId);
  const stages = baseSlots
    .map((baseSlot, slot) => ({
      baseSlot,
      slot,
      groups: filterTalentGroups(
        talentChoiceGroupsForSlot(player.characterId, baseSlot.id, baseSlot.levelName),
        query,
      ),
    }))
    .filter((stage) => stage.slot > 0 && stage.slot < player.level);
  return `
    <div class="picker-popup-backdrop" data-action="close-talent-picker"></div>
    <section class="picker-popup build-picker-popup identity-popup talent-popup" aria-label="构筑选择">
      <div class="picker-popup-head">
        <div class="build-picker-heading">
          ${pickerPopupTitle(side, "构筑选择", "仙命")}
          ${renderBuildPickerTabs(state)}
        </div>
        <div class="picker-popup-tools">
          ${pickerSearch("搜仙命", "talent", state.pickerSearch ?? "", "搜索")}
          ${pickerCloseButton("close-talent-picker")}
        </div>
      </div>
      ${stages.every((stage) => stage.groups.length === 0)
        ? `<div class="empty-picker-note">无可选仙命</div>`
        : `
          <div class="talent-stages-grid">
            ${stages.map((stage) => renderTalentStage(state, stage)).join("")}
          </div>
        `}
    </section>
  `;
}

export function renderCharacterPopup(state: AppState): string {
  if (state.pickerMode !== "character") return "";
  const side = state.activeSide;
  const player = state.config.players[side];
  const query = (state.pickerSearch ?? "").trim().toLowerCase();
  const groups = characterGroups()
    .map((group) => ({ ...group, characters: group.characters.filter((character) =>
      !query || `${character.name} ${character.sectName}`.toLowerCase().includes(query)) }))
    .filter((group) => group.characters.length > 0);
  return `
    <div class="picker-popup-backdrop" data-action="close-character-picker"></div>
    <section class="picker-popup build-picker-popup identity-popup character-popup" aria-label="构筑选择">
      <div class="picker-popup-head">
        <div class="build-picker-heading">
          ${pickerPopupTitle(side, "构筑选择", "角色")}
          ${renderBuildPickerTabs(state)}
        </div>
        <div class="picker-popup-tools">
          ${pickerSearch("搜角色", "character", state.pickerSearch ?? "", "搜索")}
          ${pickerCloseButton("close-character-picker")}
        </div>
      </div>
      ${groups.length === 0 ? `<div class="empty-picker-note">无匹配角色</div>` : `<div class="picker-popup-grid character-popup-grid" style="--picker-cols: ${groups.length}">
        ${groups.map((group) => renderCharacterPickerColumn(group, player.characterId)).join("")}
      </div>`}
    </section>
  `;
}

export function renderCareerPopup(state: AppState): string {
  if (state.pickerMode !== "career") return "";
  const side = state.activeSide;
  const player = state.config.players[side];
  const dualSlots = [1, 2, 3, 4].filter((slot) => slotHasDualCareerTalent(player, slot));
  const columns: { label: string; count: number; content: string }[] = [
    {
      label: "主副职",
      count: CAREER_OPTIONS.length,
      content: CAREER_OPTIONS.map((career) => {
        const isPrimary = player.careerName === career.id;
        const isDual = Object.values(player.dualCareerNames).includes(career.id);
        return renderCareerCandidate(career.name, isPrimary, `pick-career`, side, {
          careerId: career.id,
          extraClass: isDual ? " dual" : "",
        });
      }).join(""),
    },
    ...dualSlots.map((slot) => {
      const slotLabel = slot === 1 ? "筑基" : slot === 2 ? "金丹" : slot === 3 ? "元婴" : "化神";
      const current = player.dualCareerNames[slot] ?? "";
      // 兼修副职没有「未选」态：只有 7 个副职可选。已被主副职或其他兼修槽
      // 占用的副职禁用（与 syncDualCareers 的去重一致），避免选了又被静默清掉。
      const taken = new Set<string>();
      if (player.careerName) taken.add(player.careerName);
      for (const other of [1, 2, 3, 4]) {
        if (other !== slot && player.dualCareerNames[other]) {
          taken.add(player.dualCareerNames[other]!);
        }
      }
      const items = CAREER_OPTIONS.map((career) =>
        renderCareerCandidate(career.name, current === career.id, "pick-dual-career", side, {
          slot,
          careerId: career.id,
          disabled: taken.has(career.id) && current !== career.id,
        }),
      );
      return { label: `${slotLabel}兼修`, count: items.length, content: items.join("") };
    }),
  ];
  return `
    <div class="picker-popup-backdrop" data-action="close-career-picker"></div>
    <section class="picker-popup build-picker-popup identity-popup career-popup" aria-label="构筑选择">
      <div class="picker-popup-head">
        <div class="build-picker-heading">
          ${pickerPopupTitle(side, "构筑选择", "副职")}
          ${renderBuildPickerTabs(state)}
        </div>
        <div class="picker-popup-tools">
          ${pickerCloseButton("close-career-picker")}
        </div>
      </div>
      <div class="picker-popup-grid career-popup-grid" style="--picker-cols: ${columns.length}">
        ${columns.map((col) => renderPickerColumn(col.label, col.count, col.content, " career-realm-col")).join("")}
      </div>
    </section>
  `;
}

function renderCareerCandidate(
  name: string,
  selected: boolean,
  action: string,
  side: Side,
  attrs: { careerId: string; slot?: number; extraClass?: string; disabled?: boolean },
): string {
  const slotAttr = attrs.slot !== undefined ? ` data-slot="${attrs.slot}"` : "";
  return `<button type="button" class="deck-candidate career-candidate${selected ? " selected" : ""}${attrs.extraClass ?? ""}" data-action="${action}" data-side="${side}"${slotAttr} data-career-id="${escapeAttribute(attrs.careerId)}" aria-pressed="${selected}"${attrs.disabled ? " disabled" : ""}><span class="cand-name">${escapeHtml(name)}</span></button>`;
}

export function renderFateStrategyPopup(state: AppState): string {
  if (state.pickerMode !== "fate") return "";
  const side = state.activeSide;
  const player = state.config.players[side];
  const groups = fateStrategyGroupsForCharacter(player.characterId);
  const selected = new Set(player.fateStrategies);
  const optionCount = groups.reduce((sum, group) => sum + group.options.length, 0);
  return `
    <div class="picker-popup-backdrop" data-action="close-fate-picker"></div>
    <section class="picker-popup build-picker-popup identity-popup fate-popup" aria-label="构筑选择">
      <div class="picker-popup-head">
        <div class="build-picker-heading">
          ${pickerPopupTitle(side, "构筑选择", "天衍")}
          ${renderBuildPickerTabs(state)}
        </div>
        <div class="picker-popup-tools">
          <button
            type="button"
            data-action="clear-fate-strategies"
            data-side="${side}"
            ${player.fateStrategies.length === 0 ? "disabled" : ""}
          >清空</button>
          ${pickerCloseButton("close-fate-picker")}
        </div>
      </div>
      ${optionCount === 0
        ? `<div class="empty-picker-note">无可选天衍仙命</div>`
        : `
          <div class="picker-popup-grid fate-popup-grid" style="--picker-cols: ${groups.length}">
            ${groups.map((group) => renderFateStrategyPickerColumn(group, selected)).join("")}
          </div>
        `}
    </section>
  `;
}

function renderCardPickerSection(
  state: AppState,
  group: CardPickerGroup,
  usage: ReadonlyMap<number, number>,
): string {
  const badgeTitle = group.badgeTitle ?? group.badge ?? "";
  return `
    <section class="card-picker-group" data-card-group="${escapeAttribute(group.id)}">
      <div class="card-picker-group-label" aria-label="分类：${escapeAttribute(group.label)}${group.badgeTitle ? `（${escapeAttribute(group.badgeTitle)}）` : ""}">
        <span class="card-picker-group-name">${[...group.label].map((character) => `<span>${escapeHtml(character)}</span>`).join("")}</span>
        ${group.badge
          ? `<span class="card-picker-group-badge" title="${escapeAttribute(badgeTitle)}">${escapeHtml(group.badge)}</span>`
          : ""}
      </div>
      <div class="card-picker-cards">
        ${group.cards.map((card) => renderDeckCandidate(state, card, usage)).join("")}
      </div>
    </section>
  `;
}

function renderTalentStage(
  state: AppState,
  stage: {
    readonly baseSlot: TalentSlotOption;
    readonly slot: number;
    readonly groups: readonly { id: string; label: string; options: readonly TalentOption[] }[];
  },
): string {
  const used = new Set(state.config.players[state.activeSide].talents.filter((talentId, index) =>
    index > 0 && index !== stage.slot && talentId > 0,
  ));
  // 仙命浮层排序：该槽位角色默认仙命置顶，「抽 N 张牌」类沉底。
  // 默认仙命本是角色专属、被 talentChoiceGroupsForSlot 过滤掉而不在选项里，这里
  // 注入到列头，让用户看得到、点得回；仅在没有搜索词时注入，避免搜索结果里混入不匹配项。
  const defaultId = stage.baseSlot.id;
  const hasQuery = (state.pickerSearch ?? "").trim() !== "";
  const entries = stage.groups.flatMap((group) =>
    group.options.map((option) => ({ columnId: group.id, option })),
  );
  const defaultInOptions = entries.some((entry) => entry.option.id === defaultId);
  const head = defaultInOptions || hasQuery || stage.groups.length === 0
    ? []
    : [{ columnId: "exclusive", option: stage.baseSlot }];
  const ordered = [
    ...head,
    ...entries.filter((entry) => entry.option.id === defaultId),
    ...entries.filter((entry) => entry.option.id !== defaultId && !isBattleIrrelevantTalent(entry.option)),
    ...entries.filter((entry) => entry.option.id !== defaultId && isBattleIrrelevantTalent(entry.option)),
  ];
  const optionCount = ordered.length;
  return `<section class="talent-stage${state.selectedTalentSlot === stage.slot ? " active" : ""}" data-talent-stage="${stage.slot}">
    <div class="talent-stage-title">
      <span>${escapeHtml(stage.baseSlot.label ?? stage.baseSlot.levelName ?? "仙命")}</span>
      <b>${optionCount}</b>
    </div>
    <div class="talent-stage-list">
      ${stage.groups.length === 0
        ? `<div class="empty-picker-note">无匹配项</div>`
        : ordered.map((entry) =>
          renderTalentCandidate(state, entry.columnId, entry.option, used, stage.slot)).join("")}
    </div>
  </section>`;
}

function renderCharacterPickerColumn(
  group: { readonly label: string; readonly characters: readonly { readonly id: number; readonly name: string; readonly sectName: string }[] },
  selectedId: number,
): string {
  return renderPickerColumn(group.label, group.characters.length,
    group.characters.map((character) => renderCharacterCandidate(character, selectedId === character.id)).join(""),
    " character-realm-col",
  );
}

function renderCharacterCandidate(
  character: { readonly id: number; readonly name: string; readonly sectName: string },
  selected: boolean,
): string {
  const typeClass = character.sectName === "DuanXuanZong"
    ? "dx"
    : character.sectName === "QiXingGe"
      ? "qx"
      : "normal";
  return `
    <button
      type="button"
      class="deck-candidate character-candidate ${typeClass}${selected ? " selected" : ""}"
      data-action="pick-character"
      data-character-id="${character.id}"
      aria-pressed="${selected}"
    >
      <span class="cand-name">${escapeHtml(character.name)}</span>
    </button>
  `;
}

function renderFateStrategyPickerColumn(
  group: FateStrategyGroup,
  selected: ReadonlySet<number>,
): string {
  return renderPickerColumn(group.label, group.options.length,
    group.options.map((option) => renderFateStrategyCandidate(option, selected.has(option.id))).join(""),
    " fate-realm-col",
  );
}

function renderPickerColumn(label: string, count: number, content: string, extraClass = ""): string {
  return `
    <div class="deck-realm-col${extraClass}">
      <div class="deck-realm-title">
        ${escapeHtml(label)}
        <span class="deck-realm-count">${count}</span>
      </div>
      <div class="deck-realm-list">${content}</div>
    </div>
  `;
}

function pickerPopupTitle(side: "p1" | "p2", title: string, context = ""): string {
  return `<div class="picker-popup-title">
    <span>${side === "p1" ? "玩家一" : "玩家二"}${context ? ` · ${escapeHtml(context)}` : ""}</span>
    <strong>${escapeHtml(title)}</strong>
  </div>`;
}

function renderBuildPickerTabs(state: AppState): string {
  const player = state.config.players[state.activeSide];
  const hasCharacter = player.characterId !== EMPTY_CHARACTER_ID;
  const fateOptionIds = new Set(
    fateStrategyGroupsForCharacter(player.characterId)
      .flatMap((group) => group.options)
      .map((option) => option.id),
  );
  const selectedFateCount = player.fateStrategies.filter((id) => fateOptionIds.has(id)).length;
  return `<nav class="build-picker-tabs" aria-label="构筑选择类型">
    ${buildPickerTab("character", "角色", state.pickerMode)}
    ${buildPickerTab("talent", "仙命", state.pickerMode, !hasCharacter)}
    ${buildPickerTab("career", "副职", state.pickerMode, !hasCharacter)}
    ${buildPickerTab("fate", "天衍", state.pickerMode, !hasCharacter, `${selectedFateCount}/${fateOptionIds.size}`)}
    ${buildPickerTab("card", "卡牌", state.pickerMode)}
  </nav>`;
}

function buildPickerTab(
  mode: "character" | "talent" | "career" | "fate" | "card",
  label: string,
  selected: AppState["pickerMode"],
  disabled = false,
  count = "",
): string {
  const guides = {
    character: [
      "角色选择",
      "",
      "作用：切换当前玩家角色。",
      "联动：角色会限定可选仙命，并重置该侧与角色绑定的构筑状态。",
      "检查：切换后请重新确认仙命、生命上限与牌池范围。",
    ].join("\n"),
    talent: [
      "仙命选择",
      "",
      "作用：设置当前角色各境界的仙命。",
      "槽位：第一格为角色固定仙命，其余格随境界开放。",
      "操作：点击已选仙命可替换；构筑诊断会提示不合法组合。",
    ].join("\n"),
    career: [
      "副职选择",
      "",
      "作用：选择主副职和兼修副职。",
      "主副职：决定副职牌池范围。",
      "兼修：只有选了副职兼修仙命的境界槽可以兼修第二个副职。",
      "兼修副职必选：选中副职兼修仙命后直接进入本页选择。",
    ].join("\n"),
    fate: [
      "天衍策略",
      "",
      "作用：选择本局使用的天衍策略。",
      "计数：显示已选数量与当前角色可用总数。",
      "范围：这里只配置战斗输入，不模拟战斗外获得过程。",
    ].join("\n"),
    card: [
      "卡牌选择",
      "",
      "作用：给当前卡槽选择战斗牌。",
      "筛选：可搜索，并按卡牌范围与类型缩小结果。",
      "等级：在上方卡槽直接切换等级。",
      "范围：未实现或仅记录牌会由构筑诊断明确提示。",
    ].join("\n"),
  } as const;
  return `<button type="button" class="build-picker-tab${selected === mode ? " selected" : ""}" data-action="set-picker-mode" data-mode="${mode}" aria-pressed="${selected === mode}" title="${escapeAttribute(guides[mode])}" ${disabled ? "disabled" : ""}><span>${label}</span>${count ? `<span class="build-picker-tab-count">${count}</span>` : ""}</button>`;
}

function pickerCloseButton(action: string): string {
  return `<button type="button" class="picker-popup-close" data-action="${action}" aria-label="关闭" title="关闭">×</button>`;
}

function pickerSearch(label: string, mode: string, value: string, placeholder = label): string {
  return `<input class="picker-search" type="search" aria-label="${label}" placeholder="${placeholder}" value="${escapeAttribute(value)}" data-picker-search="${mode}" />`;
}

function filterTalentGroups(
  groups: readonly { id: string; label: string; options: readonly TalentOption[] }[],
  query: string,
): readonly { id: string; label: string; options: readonly TalentOption[] }[] {
  if (!query) return groups;
  return groups
    .map((group) => ({ ...group, options: group.options.filter((option) =>
      `${option.name} ${option.id}`.toLowerCase().includes(query)) }))
    .filter((group) => group.options.length > 0);
}

function renderDeckCandidate(
  state: AppState,
  card: CardOption,
  usage: ReadonlyMap<number, number>,
): string {
  const count = usage.get(card.baseId) ?? 0;
  const isSelected = state.config.players[state.activeSide].deck[state.selectedSlot]?.baseId === card.baseId;
  const disabled = !canPickCardForDeckSlot(
    card,
    state.config.players[state.activeSide].deck,
    state.selectedSlot,
  );
  const stateName = disabled ? "disabled" : count > 0 ? "used" : "normal";
  const series = cardSeriesKey(card);
  return renderCardFace({
    as: "button",
    card,
    subLabel: card.archiveKind === "season" ? compactArchiveLabel(card.archiveLabel) : undefined,
    selected: isSelected,
    unimplemented: !card.implemented,
    state: stateName,
    title: derivedCardDetailText(card, state.config.players[state.activeSide].talents),
    actionAttrs: `data-action="pick-card" data-base-id="${card.baseId}"${series ? ` data-series="${escapeAttribute(series)}"` : ""}`,
    disabled,
  });
}

function derivedCardDetailText(card: CardOption, talentIds: readonly number[]): string {
  const base = cardDetailText(card);
  const derivations = cardDerivationTalentIds(card.baseId, talentIds)
    .map(talentDetailText)
    .filter((detail) => detail !== "");
  return derivations.length > 0 ? `${base}\n\n仙命派生\n${derivations.join("\n")}` : base;
}

export function renderTalentCandidate(
  state: AppState,
  columnId: string,
  option: TalentOption,
  usage: ReadonlySet<number>,
  slotOverride?: number,
): string {
  const player = state.config.players[state.activeSide];
  const slot = slotOverride ?? state.selectedTalentSlot;
  const selectedId = player.talents[slot] ?? 0;
  const isSelected = selectedId === option.id;
  const isUsed = usage.has(option.id) && !isSelected;
  const missingBattle = isTalentMissingBattle(option);
  return `
    <button
      type="button"
      class="deck-candidate talent-candidate ${columnId}${isUsed ? " used" : ""}${isSelected ? " selected" : ""}${missingBattle ? " unimplemented" : ""}"
      data-action="pick-talent"
      data-talent-id="${option.id}"
      data-talent-slot="${slot}"
      title="${escapeAttribute(talentDetailText(option.id))}"
      aria-pressed="${isSelected}"
    >
      <span class="cand-name">${escapeHtml(option.name)}</span>
      ${isUsed ? `<span class="cand-status">已用</span>` : missingBattle ? `<span class="cand-status">占位</span>` : ""}
      <span class="cand-sub">${escapeHtml(talentSummary(option.id))}</span>
    </button>
  `;
}

function talentSummary(talentId: number): string {
  const detail = talentDetailText(talentId);
  return detail.split("\n").slice(1).join(" ");
}

function renderFateStrategyCandidate(option: FateStrategyOption, selected: boolean): string {
  const implemented = isFateStrategyImplemented(option);
  return `
    <button
      type="button"
      class="deck-candidate fate-candidate ${selected ? "selected" : ""}${implemented ? "" : " unimplemented"}"
      data-action="toggle-fate-strategy"
      data-fate-strategy-id="${option.id}"
      ${implemented ? "" : "disabled"}
    >
      <span class="cand-name">${escapeHtml(fateStrategyDisplayName(option))}</span>
      <span class="cand-sub">${escapeHtml(fateStrategySummary(option))}</span>
    </button>
  `;
}

function filterCards(state: AppState, cards: readonly CardOption[]): readonly CardOption[] {
  const query = normalizeCardSearchText(state.cardSearch);
  const visibleCards = query
    ? cards
    : cards.filter((card) => card.archiveKind !== "common" && card.archiveKey !== "common");
  return visibleCards.filter((card) =>
    !query ||
    normalizeCardSearchText(card.name).includes(query) ||
    String(card.baseId).includes(query),
  );
}

function normalizeCardSearchText(value: string): string {
  return value
    .normalize("NFKC")
    .toLowerCase()
    .replace(/[\s·•・‧∙⋅.．。\-_—]/gu, "");
}

function compactArchiveLabel(label: string): string {
  return label
    .replace(/^(.+?)专属过往/, "专属过往")
    .replace(/^(.+?)过往/, "过往")
    .replace(/^(.+?)天衍仙命/, "天衍仙命");
}
