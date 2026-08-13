import type {
  OriginalReplayFixture,
  OriginalReplayPlayerFixture,
} from "./domain";
import {
  activeSlotCountForGameRound,
  getCardVariant,
} from "./data";
import { derivePlayerBattleStats } from "./derived-state";
import type { BattleConfig, PlayerConfig } from "./types";

/**
 * Converts editable UI state into the neutral replay fixture consumed by
 * analysis. This adapter deliberately contains no battle execution.
 */
export function buildReplayFixture(config: BattleConfig): OriginalReplayFixture {
  const source = config.sourceKind === "original-fixture"
    ? { ...config.replayMetadata?.source, round: config.gameRound }
    : { round: config.gameRound };
  return {
    firstPlayerSide: config.firstPlayerSide,
    decisionTape: config.decisionTape,
    randomFallbackTape: config.randomFallbackTape,
    ...(config.sourceKind === "original-fixture" && config.replayMetadata?.catalogCards
      ? { catalogCards: config.replayMetadata.catalogCards }
      : {}),
    ...(config.sourceKind === "original-fixture" && config.replayMetadata?.historicalCardOverrides
      ? { historicalCardOverrides: config.replayMetadata.historicalCardOverrides }
      : {}),
    maxActorTurns: config.maxActorTurns,
    source,
    players: {
      p1: buildFixturePlayer(config, config.players.p1),
      p2: buildFixturePlayer(config, config.players.p2),
    },
  };
}

function buildFixturePlayer(
  config: BattleConfig,
  player: PlayerConfig,
): OriginalReplayPlayerFixture {
  const stats = derivePlayerBattleStats(player);
  return {
    level: player.level,
    baseMaxHp: stats.maxHpWithoutPhysique,
    extraMaxHp: stats.extraMaxHp,
    characterId: player.characterId,
    talents: player.talents,
    ...(player.talentResonanceId === null
      ? {}
      : { talentResonanceId: player.talentResonanceId }),
    activeSlotCount: activeSlotCountForGameRound(config.gameRound),
    initialDefense: player.defense,
    initialAnima: player.anima,
    initialGuard: player.guard,
    initialMomentum: player.momentum,
    initialMomentumLimit: player.momentumLimit,
    initialAgility: player.agility,
    handCards: player.handCardIds,
    lastRoundUsedCardBaseIds: player.lastRoundUsedCardBaseIds,
    lastRoundLife: player.lastRoundLife,
    lastRoundExp: player.lastRoundExp,
    talentCardParams: player.talentCardParams,
    talentTempDatas: player.talentTempDatas,
    permanentBuffTempDatas: player.permanentBuffTempDatas,
    fateStrategies: player.fateStrategies,
    cards: player.deck.map((slot) =>
      config.sourceKind === "original-fixture" && slot.originalConfig
        ? slot.originalConfig
        : getCardVariant(slot).config
    ),
  };
}
