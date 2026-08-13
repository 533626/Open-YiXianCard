import { CoreBuff } from "./domain";
import talentBattleGainsJson from "../../shared/data/initial-talent-battle-gains.json";
import levelConfigJson from "../../shared/data/level-config.json";
import type { PlayerConfig } from "./types";

type TalentGainMap = Readonly<Record<string, number>>;

type TalentBattleGainsData = {
  readonly maxHpTalentGains: TalentGainMap;
  readonly physiqueLimitTalentGains: TalentGainMap;
  readonly initialPhysiqueTalentGains: TalentGainMap;
};

const TALENT_BATTLE_GAINS = talentBattleGainsJson as TalentBattleGainsData;
const MAX_HP_TALENT_GAINS = numericMap(TALENT_BATTLE_GAINS.maxHpTalentGains);
const PHYSIQUE_LIMIT_TALENT_GAINS = numericMap(TALENT_BATTLE_GAINS.physiqueLimitTalentGains);
const INITIAL_PHYSIQUE_TALENT_GAINS = numericMap(TALENT_BATTLE_GAINS.initialPhysiqueTalentGains);

/**
 * 体魄的永久 buff 键。引擎只从 `permanentBuffTempDatas` 这一路读体魄
 * （`engine-rust/src/replay/support.rs::permanent_physique_key`），
 * `player.buffs[CoreBuff.Physique]` 只用于界面显示。两者必须同步写，
 * 否则界面上的体魄上限一进战斗就消失。
 */
export const PERMANENT_PHYSIQUE_KEY = "10023";

export const JI_FANGSHENG_CHARACTER_ID = 4_000_004;
export const DEFAULT_JI_FANGSHENG_INITIAL_FATE_RANK = 4;
export const PHYSIQUE_FREE_GAP = 5;

export const LEVEL_OPTIONS = [
  { level: 1, label: "炼气" },
  { level: 2, label: "筑基" },
  { level: 3, label: "金丹" },
  { level: 4, label: "元婴" },
  { level: 5, label: "化神" },
] as const;

export type DerivedPlayerBattleStats = {
  readonly hp: number;
  readonly maxHp: number;
  readonly maxHpWithoutPhysique: number;
  readonly extraMaxHp: number;
};

type LevelConfigRecord = {
  readonly level: {
    readonly value: number;
    readonly name: string;
  };
  readonly baseMaxHp: number;
};

const LEVEL_CONFIGS = (levelConfigJson as { readonly records: readonly LevelConfigRecord[] })
  .records
  .filter((record) => Number.isFinite(record.level.value) && Number.isFinite(record.baseMaxHp))
  .sort((left, right) => left.level.value - right.level.value);

export function levelForGameRound(gameRound: number): number {
  const round = normalizedGameRound(gameRound);
  if (round >= 12) return 5;
  if (round >= 8) return 4;
  if (round >= 5) return 3;
  if (round >= 2) return 2;
  return 1;
}

export function normalizePlayerLevel(level: number): number {
  if (!Number.isFinite(level)) return 1;
  return Math.min(5, Math.max(1, Math.trunc(level)));
}

export function baseMaxHpForLevel(level: number): number {
  const requestedLevel = Number.isFinite(level) ? Math.max(1, Math.trunc(level)) : 1;
  let selected = LEVEL_CONFIGS[0];
  for (const config of LEVEL_CONFIGS) {
    if (config.level.value > requestedLevel) break;
    selected = config;
  }
  if (!selected) throw new Error("缺少 LevelConfig 配置");
  return selected.baseMaxHp;
}

export function defaultExtraMaxHpForGameRound(gameRound: number): number {
  return (normalizedGameRound(gameRound) - 1) * 2;
}

export function defaultHpForGameRound(gameRound: number): number {
  return baseMaxHpForLevel(levelForGameRound(gameRound)) + defaultExtraMaxHpForGameRound(gameRound);
}

export function defaultHpForSetup(level: number, gameRound: number, lifeModifier = 0): number {
  const modifier = Number.isFinite(lifeModifier) ? Math.trunc(lifeModifier) : 0;
  return Math.max(1, baseMaxHpForLevel(normalizePlayerLevel(level)) + defaultExtraMaxHpForGameRound(gameRound) + modifier);
}

export function defaultMaxHpForPlayer(player: PlayerConfig, gameRound: number): number {
  return defaultHpForSetup(player.level, gameRound, player.lifeModifier) + (player.buffs[CoreBuff.Physique] ?? 0);
}

export function physiqueLimitForGameRound(gameRound: number): number {
  return normalizedGameRound(gameRound) * 5 + 1;
}

export function maxJiFangshengInitialFateRank(gameRound: number): number {
  return Math.min(4, Math.max(0, normalizedGameRound(gameRound) - 1));
}

export function normalizeJiFangshengInitialFateRank(
  value: number,
  gameRound = 99,
): number {
  const maxRank = maxJiFangshengInitialFateRank(gameRound);
  if (!Number.isFinite(value)) return maxRank;
  return Math.min(maxRank, Math.max(0, Math.trunc(value)));
}

export function physiqueLimitBonusFromPlayer(player: PlayerConfig): number {
  const characterBonus = player.characterId === JI_FANGSHENG_CHARACTER_ID
    ? normalizeJiFangshengInitialFateRank(player.jiFangshengInitialFateRank, player.gameRound)
    : 0;
  return characterBonus + sumTalentMap(player.talents, PHYSIQUE_LIMIT_TALENT_GAINS);
}

export function physiqueLimitForPlayer(player: PlayerConfig, gameRound = player.gameRound): number {
  return physiqueLimitForGameRound(gameRound) + physiqueLimitBonusFromPlayer(player);
}

export function defaultPhysiqueForPlayer(player: PlayerConfig, gameRound = player.gameRound): number {
  const round = normalizedGameRound(gameRound);
  const characterBonus = player.characterId === JI_FANGSHENG_CHARACTER_ID
    ? normalizeJiFangshengInitialFateRank(player.jiFangshengInitialFateRank, round)
    : 0;
  const baseLimit = physiqueLimitForGameRound(round) + characterBonus;
  const talentPhysique = sumTalentMap(player.talents, INITIAL_PHYSIQUE_TALENT_GAINS);
  const limit = physiqueLimitForPlayer(player, round);
  const basePhysique = round <= 1 ? 0 : baseLimit - PHYSIQUE_FREE_GAP;
  return Math.min(limit, Math.max(0, basePhysique + talentPhysique));
}

export function derivePlayerBattleStats(player: PlayerConfig): DerivedPlayerBattleStats {
  const extraMaxHp = maxHpGainFromInitialTalents(player.talents);
  const physique = player.buffs[CoreBuff.Physique] ?? 0;
  return {
    hp: player.hp + extraMaxHp,
    maxHp: player.maxHp + extraMaxHp,
    maxHpWithoutPhysique: Math.max(0, player.maxHp - physique),
    extraMaxHp,
  };
}

export function maxHpGainFromInitialTalents(talents: readonly number[]): number {
  return sumTalentMap(talents, MAX_HP_TALENT_GAINS);
}

function sumTalentMap(talents: readonly number[], gains: ReadonlyMap<number, number>): number {
  return talents.reduce((sum, talentId) => sum + (gains.get(talentId) ?? 0), 0);
}

function numericMap(values: TalentGainMap): ReadonlyMap<number, number> {
  return new Map(
    Object.entries(values).map(([key, value]) => [Number(key), value]),
  );
}

function normalizedGameRound(gameRound: number): number {
  return Number.isFinite(gameRound) ? Math.max(1, Math.trunc(gameRound)) : 1;
}
