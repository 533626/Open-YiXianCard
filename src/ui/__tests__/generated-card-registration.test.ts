import { describe, expect, test } from "bun:test";
import {
  adaptOriginalCardType,
  normalizeBaseId,
  adaptOriginalCardConfig,
} from "../domain";
import { CARD_OPTIONS } from "../data/cards";
import {
  ORIGINAL_CARD_CONFIGS,
  archiveByBaseId,
  coverageByBaseId,
} from "../data/source";

describe("UI generated card registration boundary", () => {
  test("generated registered cards all resolve to a public card config", () => {
    const generatedAccepted = new Set(CARD_OPTIONS.map((card) => card.baseId));
    expect(generatedAccepted.size).toBe(
      [...archiveByBaseId.values()].filter((card) => card.registered).length,
    );
    expect([...generatedAccepted].filter((baseId) =>
      baseId !== 0 &&
      archiveByBaseId.get(baseId)?.registered !== true &&
      coverageByBaseId.get(baseId)?.registered !== true
    )).toEqual([]);
    for (const card of CARD_OPTIONS) {
      const config = ORIGINAL_CARD_CONFIGS.find((item) => normalizeBaseId(item.id) === card.baseId);
      expect(config, `card ${card.baseId}`).toBeDefined();
      expect(String(adaptOriginalCardType(config?.cardType))).toBe(card.type);
      expect(adaptOriginalCardConfig(config!).baseId).toBe(card.baseId);
    }
  });
});
