import type {
  OriginalCardConfig,
  OriginalReplayPlayerFixture,
} from "./domain";
import { normalizeBaseId } from "./domain";
import { CARD_CONFIG_BY_ID } from "./data";
import { baseMaxHpForLevel } from "./derived-state";
import type { ReplayFixtureWithExpected } from "./fixture-contract";
import { recordCodePair } from "./replay-record-code";
import type { Side } from "./types";

type ScalarKind =
  | "bool"
  | "identifier"
  | "int"
  | "int64"
  | "int_map"
  | "nested"
  | "packed_int"
  | "selection_list"
  | "string"
  | "talent_data_map";

type FieldSpec = readonly [name: string, kind: ScalarKind, nested?: Schema];
type Schema = Readonly<Record<number, FieldSpec>>;
type JsonRecord = Record<string, unknown>;

const SELECTION_DATA: Schema = {
  1: ["id", "int"],
  2: ["pendings", "packed_int"],
  3: ["selected", "int"],
};

const BATTLE_TALENT_DATA: Schema = {
  1: ["commonParams", "packed_int"],
};

const FATE_STRATEGY_DATA: Schema = {
  1: ["strategies", "selection_list"],
  2: ["counters", "int_map"],
};

const LAST_ROUND: Schema = {
  1: ["life", "int"],
  2: ["extraMaxHp", "int"],
  3: ["exp", "int"],
  4: ["level", "int"],
  5: ["talents", "packed_int"],
  6: ["usedCards", "packed_int"],
  7: ["handCards", "packed_int"],
  8: ["talentTempDatas", "int_map"],
  9: ["permanentBuffTempDatas", "int_map"],
  10: ["unlockGrids", "int"],
  15: ["usedKeYinCards", "packed_int"],
  16: ["fateStrategies", "packed_int"],
};

const PUBLIC_PLAYER: Schema = {
  1: ["uid", "identifier"],
  4: ["extraMaxHp", "int"],
  6: ["level", "int"],
  8: ["career", "int"],
  12: ["characterId", "int"],
  13: ["talents", "packed_int"],
  14: ["talentTempDatas", "int_map"],
  17: ["permanentBuffTempDatas", "int_map"],
  22: ["talentDatas", "talent_data_map"],
  200: ["lastRound", "nested", LAST_ROUND],
};

const PRIVATE_PLAYER: Schema = {
  1: ["handCards", "packed_int"],
  2: ["usedCards", "packed_int"],
  3: ["unlockGrids", "int"],
  10: ["talentDatas", "talent_data_map"],
  19: ["fateStrategyData", "nested", FATE_STRATEGY_DATA],
};

const PLAYER_DATA: Schema = {
  1: ["public", "nested", PUBLIC_PLAYER],
  2: ["private", "nested", PRIVATE_PLAYER],
};

const BATTLE_RESULT: Schema = {
  1: ["p1", "nested", PLAYER_DATA],
  2: ["p2", "nested", PLAYER_DATA],
  3: ["battleParams", "packed_int"],
  4: ["actorTurnCount", "int"],
  5: ["hpDelta", "int"],
  7: ["round", "int"],
  8: ["firstPlayerId", "identifier"],
  9: ["winnerId", "identifier"],
  16: ["gameMode", "int"],
  17: ["subMode", "int"],
  18: ["seasonMechanism", "int"],
};

const RECENT_BATTLE_INFO: Schema = {
  2: ["gameMode", "int"],
  4: ["beginTs", "int64"],
  5: ["endTs", "int64"],
  8: ["battleRank", "int"],
  11: ["version", "string"],
  20: ["codeId", "int64"],
  24: ["isVersusPractice", "bool"],
  25: ["subMode", "int"],
  32: ["xianFu", "bool"],
  34: ["script", "int"],
  100: ["roundStats", "nested", BATTLE_RESULT],
};

const CARD_FIELDS = [
  "id",
  "name",
  "cardType",
  "anima",
  "attack",
  "randomAttack",
  "attackCount",
  "def",
  "randomDef",
  "damage",
  "jianYi",
  "guaXiang",
  "actionAgain",
  "hpCost",
  "physique",
  "otherParams",
  "seasonMechanics",
] as const satisfies readonly (keyof OriginalCardConfig)[];

export interface DecodedOriginalReplayRound {
  readonly round: number;
  readonly firstPlayerSide: Side;
  readonly winnerSide: Side;
  readonly actorTurnCount: number;
  readonly hpDeltaP1MinusP2: number;
  readonly p1CharacterId: number;
  readonly p2CharacterId: number;
  readonly recordCodes: readonly string[];
  readonly fixture: ReplayFixtureWithExpected;
}

export interface DecodedOriginalReplay {
  readonly gameVersion: string;
  readonly beginTimestamp: number | null;
  readonly endTimestamp: number | null;
  readonly recordCodes: readonly string[];
  readonly rounds: readonly DecodedOriginalReplayRound[];
}

export function decodeOriginalReplayBin(bytes: Uint8Array): DecodedOriginalReplay {
  if (bytes.byteLength === 0) throw new Error("原版对局文件为空");
  const record = decodeMessage(bytes, RECENT_BATTLE_INFO);
  const battles = recordList(record.roundStats);
  if (battles.length === 0) throw new Error("文件中没有可导入的对局轮次");
  const battleRank = numberValue(record.battleRank);
  const codeId = bigintValue(record.codeId);
  const recordCodesForAllRounds = codeId === null
    ? []
    : recordCodePair(codeId, 0, battleRank);
  const rounds = battles.map((battle) => {
    const round = buildRound(battle);
    return {
      ...round,
      recordCodes: codeId === null
        ? []
        : unique([
            ...recordCodePair(codeId, round.round, battleRank),
            ...recordCodesForAllRounds,
          ]),
    };
  });
  const recordCodes = unique(rounds.flatMap((round) => round.recordCodes));
  return {
    gameVersion: stringValue(record.version),
    beginTimestamp: safeTimestamp(record.beginTs),
    endTimestamp: safeTimestamp(record.endTs),
    recordCodes,
    rounds,
  };
}

function buildRound(battle: JsonRecord): DecodedOriginalReplayRound {
  const p1 = recordValue(battle.p1, "缺少玩家一数据");
  const p2 = recordValue(battle.p2, "缺少玩家二数据");
  const p1Public = recordValue(p1.public, "缺少玩家一公开数据");
  const p2Public = recordValue(p2.public, "缺少玩家二公开数据");
  const p1Uid = stringValue(p1Public.uid);
  const p2Uid = stringValue(p2Public.uid);
  const firstPlayerId = stringValue(battle.firstPlayerId);
  const winnerId = stringValue(battle.winnerId);
  const firstPlayerSide = sideForUid(firstPlayerId, p1Uid, p2Uid, "先手");
  const winnerSide = sideForUid(winnerId, p1Uid, p2Uid, "胜者");
  const round = numberValue(battle.round);
  const actorTurnCount = numberValue(battle.actorTurnCount);
  const hpDeltaP1MinusP2 = numberValue(battle.hpDelta);
  const seasonMechanism = numberValue(battle.seasonMechanism);
  const fixture: ReplayFixtureWithExpected = {
    firstPlayerSide,
    decisionTape: numberList(battle.battleParams),
    source: { round, seasonMechanism },
    expected: { winnerSide, actorTurnCount, hpDeltaP1MinusP2 },
    players: {
      p1: buildPlayerFixture(p1),
      p2: buildPlayerFixture(p2),
    },
  };
  return {
    round,
    firstPlayerSide,
    winnerSide,
    actorTurnCount,
    hpDeltaP1MinusP2,
    p1CharacterId: numberValue(p1Public.characterId),
    p2CharacterId: numberValue(p2Public.characterId),
    recordCodes: [],
    fixture,
  };
}

function buildPlayerFixture(player: JsonRecord): OriginalReplayPlayerFixture {
  const publicData = recordValue(player.public, "缺少玩家公开数据");
  const privateData = recordValue(player.private, "缺少玩家私有数据");
  const lastRound = recordValue(publicData.lastRound, "缺少上一轮构筑数据");
  const level = numberValue(lastRound.level);
  const cardIds = numberList(privateData.usedCards);
  if (cardIds.length !== 8) {
    throw new Error(`原版斗法牌组必须为 8 格，当前记录为 ${cardIds.length} 格`);
  }
  const talentCardParams = collectTalentCardParams(publicData, privateData);
  const careerId = optionalNumber(publicData.career);
  const usedKeYinCards = numberList(lastRound.usedKeYinCards);
  const fateStrategies = numberList(lastRound.fateStrategies);
  return {
    level,
    baseMaxHp: baseMaxHpForLevel(level),
    extraMaxHp: numberValue(lastRound.extraMaxHp),
    characterId: numberValue(publicData.characterId),
    ...(careerId === undefined ? {} : { careerId }),
    talents: numberList(lastRound.talents),
    activeSlotCount: optionalNumber(privateData.unlockGrids) ?? cardIds.length,
    handCards: numberList(lastRound.handCards),
    ...(usedKeYinCards.length === 0 ? {} : { usedKeYinCards }),
    lastRoundLife: numberValue(lastRound.life),
    lastRoundExp: numberValue(lastRound.exp),
    lastRoundUsedCardBaseIds: numberList(lastRound.usedCards).map(normalizeBaseId),
    ...(Object.keys(talentCardParams).length === 0 ? {} : { talentCardParams }),
    currentTalentTempDatas: numberMap(publicData.talentTempDatas),
    talentTempDatas: numberMap(lastRound.talentTempDatas),
    currentPermanentBuffTempDatas: numberMap(publicData.permanentBuffTempDatas),
    permanentBuffTempDatas: numberMap(lastRound.permanentBuffTempDatas),
    ...(fateStrategies.length === 0 ? {} : { fateStrategies }),
    cards: cardIds.map(cardConfigForId),
  };
}

function cardConfigForId(cardId: number): OriginalCardConfig {
  const config = CARD_CONFIG_BY_ID.get(cardId);
  if (!config) throw new Error(`当前规则快照缺少卡牌 ${cardId}`);
  const compact: Partial<OriginalCardConfig> = {};
  for (const field of CARD_FIELDS) {
    const value = config[field];
    if (value !== undefined) Object.assign(compact, { [field]: value });
  }
  return compact as OriginalCardConfig;
}

function collectTalentCardParams(
  ...sources: readonly JsonRecord[]
): Readonly<Record<string, readonly number[]>> {
  const result: Record<string, readonly number[]> = {};
  for (const source of sources) {
    for (const item of recordList(source.talentDatas)) {
      const id = optionalNumber(item.id);
      const params = numberList(item.commonParams);
      if (id !== undefined && params.length > 0) result[String(id)] = params;
    }
  }
  return result;
}

function decodeMessage(bytes: Uint8Array, schema: Schema): JsonRecord {
  const result: JsonRecord = {};
  for (const field of parseMessage(bytes)) {
    const spec = schema[field.number];
    if (!spec) continue;
    const [name, kind, nested] = spec;
    if (kind === "int" || kind === "bool") {
      const value = int32(varintField(field));
      result[name] = kind === "bool" ? value !== 0 : value;
    } else if (kind === "int64") {
      result[name] = varintField(field);
    } else if (kind === "string" || kind === "identifier") {
      result[name] = decodeUtf8(bytesField(field));
    } else if (kind === "packed_int") {
      result[name] = decodePackedInts(bytesField(field));
    } else if (kind === "int_map") {
      const [key, value] = decodeIntMap(bytesField(field));
      const map = (result[name] ?? {}) as Record<string, number>;
      map[String(key)] = value;
      result[name] = map;
    } else if (kind === "nested") {
      if (!nested) throw new Error(`字段 ${name} 缺少嵌套 schema`);
      const decoded = decodeMessage(bytesField(field), nested);
      if (name === "roundStats") {
        ((result[name] ??= []) as JsonRecord[]).push(decoded);
      } else {
        result[name] = decoded;
      }
    } else if (kind === "selection_list") {
      ((result[name] ??= []) as JsonRecord[]).push(
        decodeMessage(bytesField(field), SELECTION_DATA),
      );
    } else if (kind === "talent_data_map") {
      ((result[name] ??= []) as JsonRecord[]).push(
        decodeTalentDataMap(bytesField(field)),
      );
    }
  }
  return result;
}

function decodeTalentDataMap(bytes: Uint8Array): JsonRecord {
  let id = 0;
  let params: JsonRecord = {};
  for (const field of parseMessage(bytes)) {
    if (field.number === 1) id = int32(varintField(field));
    if (field.number === 2) params = decodeMessage(bytesField(field), BATTLE_TALENT_DATA);
  }
  return { id, ...params };
}

function decodePackedInts(bytes: Uint8Array): number[] {
  const values: number[] = [];
  let offset = 0;
  while (offset < bytes.byteLength) {
    const item = readVarint(bytes, offset);
    values.push(int32(item.value));
    offset = item.offset;
  }
  return values;
}

function decodeIntMap(bytes: Uint8Array): readonly [number, number] {
  let key = 0;
  let value = 0;
  for (const field of parseMessage(bytes)) {
    if (field.number === 1) key = int32(varintField(field));
    if (field.number === 2) value = int32(varintField(field));
  }
  return [key, value];
}

interface WireField {
  readonly number: number;
  readonly wireType: number;
  readonly value: bigint | Uint8Array;
}

function parseMessage(bytes: Uint8Array): WireField[] {
  const fields: WireField[] = [];
  let offset = 0;
  while (offset < bytes.byteLength) {
    const tag = readVarint(bytes, offset);
    offset = tag.offset;
    const number = Number(tag.value >> 3n);
    const wireType = Number(tag.value & 7n);
    if (number <= 0) throw new Error("原版对局 protobuf 字段编号无效");
    if (wireType === 0) {
      const value = readVarint(bytes, offset);
      offset = value.offset;
      fields.push({ number, wireType, value: value.value });
      continue;
    }
    if (wireType === 1 || wireType === 5) {
      const size = wireType === 1 ? 8 : 4;
      const end = checkedEnd(offset, size, bytes.byteLength);
      fields.push({ number, wireType, value: bytes.slice(offset, end) });
      offset = end;
      continue;
    }
    if (wireType === 2) {
      const length = readVarint(bytes, offset);
      offset = length.offset;
      const size = Number(length.value);
      if (!Number.isSafeInteger(size)) throw new Error("原版对局字段长度过大");
      const end = checkedEnd(offset, size, bytes.byteLength);
      fields.push({ number, wireType, value: bytes.slice(offset, end) });
      offset = end;
      continue;
    }
    throw new Error(`不支持的原版对局 protobuf wire type：${wireType}`);
  }
  return fields;
}

function readVarint(
  bytes: Uint8Array,
  start: number,
): { readonly value: bigint; readonly offset: number } {
  let value = 0n;
  let shift = 0n;
  let offset = start;
  while (offset < bytes.byteLength && shift < 70n) {
    const byte = bytes[offset]!;
    offset += 1;
    value |= BigInt(byte & 0x7f) << shift;
    if (byte < 0x80) return { value, offset };
    shift += 7n;
  }
  throw new Error("原版对局包含损坏的 protobuf varint");
}

function checkedEnd(offset: number, size: number, total: number): number {
  const end = offset + size;
  if (!Number.isSafeInteger(end) || end > total) {
    throw new Error("原版对局 protobuf 字段越界");
  }
  return end;
}

function varintField(field: WireField): bigint {
  if (field.wireType !== 0 || typeof field.value !== "bigint") {
    throw new Error(`字段 ${field.number} 不是 varint`);
  }
  return field.value;
}

function bytesField(field: WireField): Uint8Array {
  if (field.wireType !== 2 || !(field.value instanceof Uint8Array)) {
    throw new Error(`字段 ${field.number} 不是长度字段`);
  }
  return field.value;
}

function int32(value: bigint): number {
  const unsigned = Number(value & 0xffffffffn);
  return unsigned >= 0x80000000 ? unsigned - 0x100000000 : unsigned;
}

function decodeUtf8(bytes: Uint8Array): string {
  return new TextDecoder("utf-8", { fatal: true }).decode(bytes);
}

function recordValue(value: unknown, error: string): JsonRecord {
  if (!value || typeof value !== "object" || Array.isArray(value)) throw new Error(error);
  return value as JsonRecord;
}

function recordList(value: unknown): JsonRecord[] {
  return Array.isArray(value)
    ? value.filter((item): item is JsonRecord =>
        item !== null && typeof item === "object" && !Array.isArray(item))
    : [];
}

function numberList(value: unknown): number[] {
  return Array.isArray(value)
    ? value.filter((item): item is number => Number.isInteger(item))
    : [];
}

function numberMap(value: unknown): Record<string, number> {
  if (!value || typeof value !== "object" || Array.isArray(value)) return {};
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .filter((entry): entry is [string, number] => Number.isInteger(entry[1])),
  );
}

function numberValue(value: unknown): number {
  return typeof value === "number" && Number.isInteger(value) ? value : 0;
}

function optionalNumber(value: unknown): number | undefined {
  return typeof value === "number" && Number.isInteger(value) ? value : undefined;
}

function stringValue(value: unknown): string {
  return typeof value === "string" ? value : "";
}

function bigintValue(value: unknown): bigint | null {
  return typeof value === "bigint" ? value : null;
}

function safeTimestamp(value: unknown): number | null {
  if (typeof value !== "bigint") return null;
  const number = Number(value);
  return Number.isSafeInteger(number) ? number : null;
}

function sideForUid(value: string, p1Uid: string, p2Uid: string, label: string): Side {
  if (value !== "" && value === p1Uid) return "p1";
  if (value !== "" && value === p2Uid) return "p2";
  throw new Error(`${label} ID 不属于回放双方`);
}


function unique(values: readonly string[]): readonly string[] {
  return [...new Set(values.filter(Boolean))];
}
