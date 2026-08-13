import {
  CHARACTER_BY_ID,
  isCardDisabled,
  scopedCardOptions,
} from "./data";
import type { CardOption, PlayerConfig } from "./types";

/**
 * Browser pool scope mirrors the ordinary restricted solver pool: current
 * character exclusives, current sect, and current side career. Chance,
 * secret, generic season, and unrelated career cards stay out unless they are
 * already present in the imported/current deck.
 */
export function solverCardPoolOptions(
  player: PlayerConfig,
): readonly CardOption[] {
  const character = CHARACTER_BY_ID.get(player.characterId);
  if (!character) return [];
  const allowedKeys = new Set([
    `sect:${character.sectName}`,
    `exclusive:${character.id}`,
    ...(player.careerName ? [`career:${player.careerName}`] : []),
  ]);
  // 副职兼修的副职牌池也加入 solver 搜索范围
  for (const dualKey of Object.values(player.dualCareerNames)) {
    if (dualKey) allowedKeys.add(`career:${dualKey}`);
  }
  return scopedCardOptions(
    player.characterId,
    player.careerName,
    player.talents,
    player.dualCareerNames,
  ).filter((card) =>
    card.baseId !== 0 &&
    !isCardDisabled(card) &&
    allowedKeys.has(card.archiveKey)
  );
}
