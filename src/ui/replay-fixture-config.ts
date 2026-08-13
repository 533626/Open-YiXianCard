import {
  normalizeBaseId,
  CoreBuff,
  type OriginalReplayFixture,
  type OriginalReplayPlayerFixture,
} from "./domain";
import {
  CARD_OPTION_BY_BASE_ID,
  defaultPlayerConfig,
} from "./data";
import { maxHpGainFromInitialTalents, PERMANENT_PHYSIQUE_KEY } from "./derived-state";
import type {
  BattleConfig,
  PlayerConfig,
  Side,
} from "./types";

export function battleConfigFromReplayFixture(fixture: OriginalReplayFixture): BattleConfig {
  const gameRound = fixture.source?.round ?? 1;
  return {
    sourceKind: "original-fixture",
    firstPlayerSide: fixture.firstPlayerSide,
    gameRound,
    maxActorTurns: fixture.maxActorTurns ?? 64,
    decisionTape: [...(fixture.decisionTape ?? [])],
    randomFallbackTape: [...(fixture.randomFallbackTape ?? [])],
    replayMetadata: {
      ...(fixture.source === undefined ? {} : { source: structuredClone(fixture.source) }),
      ...(fixture.catalogCards === undefined ? {} : { catalogCards: structuredClone(fixture.catalogCards) }),
      ...(fixture.historicalCardOverrides === undefined
        ? {}
        : { historicalCardOverrides: structuredClone(fixture.historicalCardOverrides) }),
    },
    players: {
      p1: playerConfigFromReplayFixture("p1", fixture.players.p1, gameRound),
      p2: playerConfigFromReplayFixture("p2", fixture.players.p2, gameRound),
    },
  };
}

function playerConfigFromReplayFixture(
  side: Side,
  fixture: OriginalReplayPlayerFixture,
  gameRound: number,
): PlayerConfig {
  const player = defaultPlayerConfig(side, fixture.characterId, gameRound);
  const permanentBuffTempDatas = copyNumberRecord(fixture.permanentBuffTempDatas);
  const talents = [...fixture.talents];
  const talentMaxHp = maxHpGainFromInitialTalents(talents);
  const permanentPhysique = permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY] ?? 0;
  const targetHp = fixture.baseMaxHp + fixture.extraMaxHp;
  player.level = fixture.level;
  player.hp = targetHp - talentMaxHp;
  player.maxHp = targetHp + permanentPhysique - talentMaxHp;
  player.activeSlotCount = fixture.activeSlotCount;
  player.defense = Math.max(0, fixture.initialDefense ?? 0);
  player.anima = Math.max(0, fixture.initialAnima ?? 0);
  player.momentum = Math.max(0, fixture.initialMomentum ?? 0);
  player.momentumLimit = fixture.initialMomentumLimit ?? 6;
  player.agility = Math.max(0, fixture.initialAgility ?? 0);
  player.guard = Math.max(0, fixture.initialGuard ?? 0);
  player.starSlots = [2, 5];
  player.activatedElements = [];
  player.lastElement = null;
  player.talents = talents;
  player.talentResonanceId = fixture.talentResonanceId ?? null;
  player.fateStrategies = [...(fixture.fateStrategies ?? [])];
  player.handCardIds = [...(fixture.handCards ?? [])];
  player.lastRoundUsedCardBaseIds = [...(fixture.lastRoundUsedCardBaseIds ?? [])];
  player.lastRoundLife = fixture.lastRoundLife ?? 0;
  player.lastRoundExp = fixture.lastRoundExp ?? 0;
  player.talentCardParams = copyNumberArrayRecord(fixture.talentCardParams ?? {});
  player.talentTempDatas = copyNumberRecord(fixture.talentTempDatas ?? {});
  player.permanentBuffTempDatas = permanentBuffTempDatas;
  player.lingWuCardBaseIds = replayLingWuCardBaseIds(fixture);
  player.buffs = replayInitialBuffs(fixture, permanentBuffTempDatas);
  player.deck = normalizeReplayDeck(fixture.cards);
  return player;
}

function replayLingWuCardBaseIds(fixture: OriginalReplayPlayerFixture): number[] {
  if (!fixture.talents.includes(192)) return [];
  return (fixture.talentCardParams?.["189"] ?? []).map(normalizeBaseId);
}

function normalizeReplayDeck(cards: OriginalReplayPlayerFixture["cards"]): PlayerConfig["deck"] {
  const deck = cards.slice(0, 8).map((card) => deckSlotFromReplayCard(card));
  while (deck.length < 8) deck.push({ baseId: 0, level: 0 });
  return deck;
}

function deckSlotFromReplayCard(
  card: OriginalReplayPlayerFixture["cards"][number],
): PlayerConfig["deck"][number] {
  for (const option of CARD_OPTION_BY_BASE_ID.values()) {
    const variantIndex = option.variants.findIndex((variant) =>
      variant.config.id === card.id ||
      (card.baseId !== undefined && variant.config.baseId === card.baseId && variant.config.id === card.id)
    );
    if (variantIndex !== -1) {
      return { baseId: option.baseId, level: variantIndex, originalConfig: { ...card } };
    }
  }
  const option = CARD_OPTION_BY_BASE_ID.get(card.baseId ?? card.id);
  if (!option) {
    return {
      baseId: normalizeBaseId(card.baseId ?? card.id),
      level: 0,
      originalConfig: { ...card },
    };
  }
  return { baseId: option.baseId, level: 0, originalConfig: { ...card } };
}

function replayInitialBuffs(
  fixture: OriginalReplayPlayerFixture,
  permanent: Readonly<Record<string, number>>,
): PlayerConfig["buffs"] {
  const initialPhysique = permanent[PERMANENT_PHYSIQUE_KEY] ?? 0;
  return {
    ...(fixture.characterId === 4_000_005 ? { [CoreBuff.FistStance]: 1 } : {}),
    ...(fixture.talents.includes(171)
      ? {
          [CoreBuff.AttackBonus]: 1,
          [CoreBuff.ExternalInjury]: 1,
        }
      : {}),
    ...(initialPhysique > 0 ? { [CoreBuff.Physique]: initialPhysique } : {}),
    ...(permanent["10024"] !== undefined ? { [CoreBuff.PhysiqueLimit]: permanent["10024"] } : {}),
    ...(permanent["17"] !== undefined ? { [CoreBuff.LastStandIntent]: permanent["17"] } : {}),
  };
}

function copyNumberRecord(
  record: Readonly<Record<string, number>>,
): Record<string, number> {
  return Object.fromEntries(Object.entries(record));
}

function copyNumberArrayRecord(
  record: Readonly<Record<string, readonly number[]>>,
): Record<string, number[]> {
  return Object.fromEntries(
    Object.entries(record).map(([key, values]) => [key, [...values]]),
  );
}
