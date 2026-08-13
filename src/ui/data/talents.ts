import { cardRealmLabel } from "../domain";
import { characterTalentRows, talentArchiveById, talentArchiveRows } from "./source";
import {
  derivedTalentChoiceIds,
  isDerivedTalentChoiceForCharacter,
} from "./derivations";
import { formatOriginalDetail } from "./text-format";
import type {
  PlayerConfig,
  TalentGroup,
  TalentOption,
  TalentSlotOption,
} from "../types";

export const SELECTABLE_TALENT_IDS = new Set(talentArchiveRows.map((talent) => talent.id));
export const TALENT_OPTION_BY_ID = new Map(
  talentArchiveRows.map((row) => [row.id, toTalentOption(row.id, row.name)] as const),
);

/** 副职兼修仙命 ID，与 Rust TUI card_pool.rs DUAL_CAREER_TALENT_IDS 一致。 */
export const DUAL_CAREER_TALENT_IDS: readonly number[] = [188, 10_188, 20_188, 30_188];

/** 判断某仙命是否为副职兼修（允许选择第二个副职）。 */
export function isDualCareerTalent(talentId: number): boolean {
  return DUAL_CAREER_TALENT_IDS.includes(talentId);
}

/**
 * 判断玩家在指定境界槽（1~4，0 是固定槽）是否拥有副职兼修仙命，
 * 若是则该槽可以指定一个兼修副职。
 */
export function slotHasDualCareerTalent(player: PlayerConfig, slot: number): boolean {
  if (slot < 1 || slot >= player.talents.length) return false;
  const talentId = player.talents[slot];
  if (!talentId) return false;
  return isDualCareerTalent(talentId);
}

export interface TalentPickerGroup {
  readonly id: string;
  readonly label: string;
  readonly options: readonly TalentOption[];
}

const TALENT_PICKER_BUCKETS = [
  { id: "exclusive", label: "专属" },
  { id: "sect", label: "门派" },
  { id: "common", label: "通用" },
] as const;

export function talentGroupsForCharacter(characterId: number): readonly TalentGroup[] {
  const character = characterInfo(characterId);
  if (!character) return [];
  const characterTalentIds = new Set(characterTalentRows
    .filter((row) => row.characterId === characterId)
    .map((row) => row.talentId));
  const characterOptions = characterTalentRows
    .filter((row) => row.characterId === characterId && characterTalentIds.has(row.talentId))
    .map((row) => toTalentOption(row.talentId, row.name))
    .filter((option, index, all) =>
      all.findIndex((candidate) => candidate.id === option.id) === index,
    );
  const sectOptions = talentArchiveRows
    .filter((talent) =>
      talent.archiveKey === `sect:${character.sectName}` &&
      talent.name.trim() !== "",
    )
    .map((talent) => toTalentOption(talent.id, talent.name))
    .filter((option, index, all) =>
      all.findIndex((candidate) => candidate.id === option.id) === index,
    )
    .sort((left, right) => left.id - right.id);
  return [
    {
      id: `exclusive:${character.id}`,
      label: `${character.name}仙命`,
      options: characterOptions,
      open: true,
    },
    {
      id: `sect:${character.sectName}`,
      label: `${character.sectName}通用仙命`,
      options: sectOptions,
      open: false,
    },
  ].filter((group) => group.options.length > 0);
}

export function characterBaseTalentSlots(characterId: number): readonly TalentSlotOption[] {
  const rows = characterTalentRows
    .filter((row) => row.characterId === characterId && row.slot === "talents")
    .sort((left, right) => levelOrder(left.levelName) - levelOrder(right.levelName));
  return rows.map((row, index) => ({
    ...toTalentOption(row.talentId, row.name),
    levelName: row.levelName,
    locked: index === 0,
    label: index === 0 ? "固定" : cardRealmLabel(row.levelName),
  }));
}

export function lockedBaseTalentId(characterId: number): number {
  return characterBaseTalentSlots(characterId)[0]?.id ?? 0;
}

/** 李㵘得炁后炼气固定槽展示/入战为灵炁奔涌(209)，与原版 CorrectLianQiTalent 一致。 */
const LI_MAN_CHARACTER_ID = 4_000_005;
const TALENT_FAN_QU = 204;
const TALENT_DE_QI = 208;
const TALENT_LING_QI_BEN_YONG = 209;

export function normalizePlayerTalents(player: PlayerConfig): void {
  const defaults = characterBaseTalentSlots(player.characterId).map((slot) => slot.id);
  const lockedId = lockedBaseTalentId(player.characterId);
  while (player.talents.length < 5) {
    player.talents.push(defaults[player.talents.length] ?? 0);
  }
  player.talents.length = 5;
  // 化神选了得炁后，炼气槽凡躯在原版入战列表里会变成灵炁奔涌；UI 若继续锁 204
  // 会让凡躯把全部加灵转成体魄，高耗灵牌永久「卡灵」。
  const hasDeQi =
    player.characterId === LI_MAN_CHARACTER_ID &&
    player.talents.slice(1).includes(TALENT_DE_QI);
  player.talents[0] = hasDeQi ? TALENT_LING_QI_BEN_YONG : lockedId;
  for (let index = 1; index < 5; index += 1) {
    const current = player.talents[index] ?? 0;
    if (!current || !isTalentSelectableForCharacter(player.characterId, current)) {
      player.talents[index] = defaults[index] ?? 0;
    }
  }
  // 脏存档：其它槽位若仍写着 204，一并改掉。
  if (hasDeQi) {
    for (let index = 1; index < 5; index += 1) {
      if (player.talents[index] === TALENT_FAN_QU) {
        player.talents[index] = TALENT_LING_QI_BEN_YONG;
      }
    }
  }
}

export function isTalentSelectableForCharacter(characterId: number, talentId: number): boolean {
  if (talentId <= 0 || !SELECTABLE_TALENT_IDS.has(talentId)) return false;
  if (isDerivedTalentChoiceForCharacter(characterId, talentId)) return true;
  // 灵炁奔涌(209)：得炁后炼气槽的战斗内形态（CorrectLianQiTalent），archive 标
  // unclassified 故不在专属列表，但仍是李㵘合法入战仙命；玩家不可手动点选。
  if (characterId === LI_MAN_CHARACTER_ID && talentId === TALENT_LING_QI_BEN_YONG) {
    return true;
  }
  const character = characterInfo(characterId);
  const archive = talentArchiveById.get(talentId);
  if (!character || !archive) return false;
  return (
    archive.archiveKind === "common" ||
    archive.archiveKey === "common" ||
    archive.archiveKey === `sect:${character.sectName}` ||
    archive.archiveKey === `exclusive:${character.id}`
  );
}

export function talentChoiceGroupsForSlot(
  characterId: number,
  parentTalentId: number,
  levelName?: string,
): readonly TalentPickerGroup[] {
  const derivedIds = derivedTalentChoiceIds(characterId, parentTalentId);
  const standardGroups = talentsGroupedForPicker(scopedTalentOptions(characterId, levelName))
    .filter((group) => group.id !== "exclusive");
  if (derivedIds.length > 0) {
    return [
      {
        id: "exclusive",
        label: TALENT_OPTION_BY_ID.get(parentTalentId)?.name ?? "专属仙命",
        options: derivedIds
          .map((talentId) => TALENT_OPTION_BY_ID.get(talentId))
          .filter((option): option is TalentOption => option !== undefined),
      },
      ...standardGroups,
    ];
  }
  return standardGroups;
}

export function scopedTalentOptions(characterId: number, levelName?: string): readonly TalentOption[] {
  const character = characterInfo(characterId);
  if (!character) return [];
  const seen = new Set<number>();
  const options: TalentOption[] = [];
  for (const row of talentArchiveRows) {
    if (levelName && row.levelName !== levelName) continue;
    if (!SELECTABLE_TALENT_IDS.has(row.id)) continue;
    const allowed =
      row.archiveKind === "common" ||
      row.archiveKey === `sect:${character.sectName}` ||
      row.archiveKey === `exclusive:${characterId}`;
    if (!allowed || seen.has(row.id)) continue;
    seen.add(row.id);
    options.push(toTalentOption(row.id, row.name));
  }
  return options.sort(talentOptionSort);
}

export function isTalentMissingBattle(option: TalentOption): boolean {
  return option.status === "missing-battle";
}

/**
 * 「战斗无关」仙命：在仙命浮层里沉到列表末尾。判定口径与用户认知一致——
 * 抽通用牌（火灵传承、感悟丛生、崩拳传承…这类「抽 N 张 XX 牌」的随机抽牌）与
 * 纯加命元/修为等非战斗仙命走底部；而「给专属牌」（获得 1 张【某 ID】这类指定卡）
 * 与已接战斗逻辑的 implemented 仙命都留在顶部。
 */
export function isBattleIrrelevantTalent(option: TalentOption): boolean {
  if (option.status === "implemented") return false;
  return !/获得.*【/.test(option.desc ?? "");
}

export function talentPickerColumn(
  option: TalentOption,
): (typeof TALENT_PICKER_BUCKETS)[number]["id"] | null {
  const archive = talentArchiveById.get(option.id);
  if (!archive) return null;
  if (archive.archiveKind === "common" || archive.archiveKey === "common") return "common";
  if (archive.archiveKind === "exclusive" || archive.archiveKey?.startsWith("exclusive:")) return "exclusive";
  if (archive.archiveKind === "sect" || archive.archiveKey?.startsWith("sect:")) return "sect";
  return null;
}

export function talentsGroupedForPicker(
  options: readonly TalentOption[],
): readonly TalentPickerGroup[] {
  const groups = new Map<string, TalentOption[]>();
  for (const option of options) {
    const column = talentPickerColumn(option);
    if (!column) continue;
    const list = groups.get(column) ?? [];
    list.push(option);
    groups.set(column, list);
  }
  return TALENT_PICKER_BUCKETS
    .filter((bucket) => groups.has(bucket.id))
    .map((bucket) => ({
      id: bucket.id,
      label: bucket.label,
      options: (groups.get(bucket.id) ?? []).sort(talentOptionSort),
    }));
}

export function describeTalent(talentId: number): string {
  return TALENT_OPTION_BY_ID.get(talentId)?.name ?? `仙命${talentId}`;
}

export function talentDetailText(talentId: number): string {
  const option = TALENT_OPTION_BY_ID.get(talentId);
  if (!option) return "";
  const desc = formatOriginalDetail(option.desc ?? "", option.otherParams ?? []);
  return desc ? `${option.name}\n${desc}` : option.name;
}

function toTalentOption(id: number, fallbackName: string): TalentOption {
  const archive = talentArchiveById.get(id);
  return {
    id,
    name: fallbackName || archive?.name || "天命",
    ...(archive?.desc ? { desc: archive.desc } : {}),
    ...(archive?.otherParams ? { otherParams: archive.otherParams } : {}),
    ...(archive?.levelName ? { levelName: archive.levelName } : {}),
    ...(archive?.status ? { status: archive.status } : {}),
    ...(archive?.archiveKind ? { archiveKind: archive.archiveKind } : {}),
    ...(archive?.archiveKey ? { archiveKey: archive.archiveKey } : {}),
    ...(archive?.archiveLabel ? { archiveLabel: archive.archiveLabel } : {}),
  };
}

function talentOptionSort(left: TalentOption, right: TalentOption): number {
  const leftMissing = isTalentMissingBattle(left);
  const rightMissing = isTalentMissingBattle(right);
  if (leftMissing !== rightMissing) return leftMissing ? 1 : -1;
  return left.id - right.id;
}

function levelOrder(levelName: string): number {
  return {
    LianQi: 0,
    ZhuJi: 1,
    JinDan: 2,
    YuanYing: 3,
    HuaShen: 4,
  }[levelName] ?? 99;
}

function characterInfo(characterId: number):
  | { readonly id: number; readonly name: string; readonly sectName: string }
  | undefined {
  const row = characterTalentRows.find((candidate) => candidate.characterId === characterId);
  return row ? { id: row.characterId, name: row.characterName, sectName: row.sectName } : undefined;
}
