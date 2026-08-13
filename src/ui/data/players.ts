import { CoreBuff } from "../domain";
import {
  DEFAULT_JI_FANGSHENG_INITIAL_FATE_RANK,
  JI_FANGSHENG_CHARACTER_ID,
  defaultHpForSetup,
  defaultPhysiqueForPlayer,
  levelForGameRound,
  maxJiFangshengInitialFateRank,
  physiqueLimitForPlayer,
} from "../derived-state";
import { DEFAULT_CAREER_ID } from "./cards";
import { DEFAULT_GAME_ROUND, groupLabel } from "./constants";
import { characterTalentRows } from "./source";
import { characterBaseTalentSlots } from "./talents";
import type {
  BattleConfig,
  CharacterOption,
  PlayerConfig,
  Side,
} from "../types";

export const CHARACTER_OPTIONS: readonly CharacterOption[] = buildCharacterOptions();
export const CHARACTER_BY_ID = new Map(
  CHARACTER_OPTIONS.map((character) => [character.id, character] as const),
);
export const EMPTY_CHARACTER_ID = 0;
const LI_MAN_CHARACTER_ID = 4_000_005; // 李㵘

export function activeSlotCountForGameRound(gameRound: number): number {
  const round = Number.isFinite(gameRound) ? Math.max(1, Math.trunc(gameRound)) : 1;
  return Math.min(8, round + 2);
}

export function activeSlotCountForLevel(level: number): number {
  if (level <= 1 || Number.isNaN(level)) return 3;
  if (level === 2) return 4;
  if (level === 3) return 5;
  if (level === 4) return 6;
  return 8;
}

export function activeSlotCountForProgress(gameRound: number, level: number): number {
  return Math.max(activeSlotCountForGameRound(gameRound), activeSlotCountForLevel(level));
}

export function defaultBattleConfig(): BattleConfig {
  const gameRound = DEFAULT_GAME_ROUND;
  return {
    firstPlayerSide: "p1",
    gameRound,
    maxActorTurns: 64,
    decisionTape: [],
    randomFallbackTape: [],
    players: {
      p1: defaultPlayerConfig("p1", EMPTY_CHARACTER_ID, gameRound),
      p2: defaultPlayerConfig("p2", EMPTY_CHARACTER_ID, gameRound),
    },
  };
}

export function defaultPlayerConfig(
  side: Side,
  characterId: number,
  gameRound = DEFAULT_GAME_ROUND,
): PlayerConfig {
  const recommendedTalents = characterBaseTalentSlots(characterId).map((slot) => slot.id);
  const initialRank = characterId === JI_FANGSHENG_CHARACTER_ID
    ? Math.min(DEFAULT_JI_FANGSHENG_INITIAL_FATE_RANK, maxJiFangshengInitialFateRank(gameRound))
    : 0;
  const level = levelForGameRound(gameRound);
  const baseHp = defaultHpForSetup(level, gameRound, 0);
  const initial: PlayerConfig = {
    side,
    label: side === "p1" ? "玩家一" : "玩家二",
    characterId,
    careerName: DEFAULT_CAREER_ID,
    dualCareerNames: {},
    level,
    gameRound,
    hp: baseHp,
    maxHp: baseHp,
    lifeModifier: 0,
    activeSlotCount: activeSlotCountForProgress(gameRound, level),
    talentResonanceId: null,
    jiFangshengInitialFateRank: initialRank,
    defense: 0,
    anima: 0,
    momentum: 0,
    momentumLimit: 6,
    agility: 0,
    guard: 0,
    buffs: {},
    starSlots: [2, 5],
    activatedElements: [],
    lastElement: null,
    talents: [...recommendedTalents],
    fateStrategies: [],
    lingWuCardBaseIds: [],
    handCardIds: [],
    lastRoundUsedCardBaseIds: [],
    lastRoundLife: 0,
    lastRoundExp: 0,
    talentCardParams: {},
    talentTempDatas: {},
    permanentBuffTempDatas: {},
    deck: Array.from({ length: 8 }, () => ({ baseId: 0, level: 0 })),
  };
  const character = CHARACTER_BY_ID.get(characterId);
  if (character?.sectName === "DuanXuanZong") {
    initial.buffs[CoreBuff.PhysiqueLimit] = physiqueLimitForPlayer(initial, gameRound);
    initial.buffs[CoreBuff.Physique] = defaultPhysiqueForPlayer(initial, gameRound);
    initial.maxHp = baseHp + (initial.buffs[CoreBuff.Physique] ?? 0);
  }
  if (characterId === LI_MAN_CHARACTER_ID) {
    initial.buffs[CoreBuff.FistStance] = 1;
  }
  return {
    ...initial,
  };
}

export function cloneBattleConfig(config: BattleConfig): BattleConfig {
  return structuredClone(config) as BattleConfig;
}

export function characterGroups(): readonly {
  readonly label: string;
  readonly characters: readonly CharacterOption[];
}[] {
  const groups = new Map<string, CharacterOption[]>();
  for (const character of CHARACTER_OPTIONS) {
    const list = groups.get(character.sectName) ?? [];
    list.push(character);
    groups.set(character.sectName, list);
  }
  return [...groups.entries()].map(([sect, characters]) => ({
    label: groupLabel(sect),
    characters,
  }));
}

function buildCharacterOptions(): readonly CharacterOption[] {
  const byId = new Map<number, CharacterOption>();
  for (const row of characterTalentRows) {
    const existing = byId.get(row.characterId);
    const talentIds = [row.talentId];
    if (!existing) {
      byId.set(row.characterId, {
        id: row.characterId,
        name: row.characterName,
        sectName: row.sectName,
        talentIds,
      });
      continue;
    }
    byId.set(row.characterId, {
      ...existing,
      talentIds: [...new Set([...existing.talentIds, ...talentIds])].sort(
        (left, right) => left - right,
      ),
    });
  }
  return [...byId.values()].sort(
    (left, right) =>
      left.sectName.localeCompare(right.sectName, "zh") ||
      left.name.localeCompare(right.name, "zh"),
  );
}
