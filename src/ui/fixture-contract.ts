import type { OriginalReplayFixture } from "./domain";
import { battleConfigFromReplayFixture } from "./replay-fixture-config";
import type { BattleConfig, Side } from "./types";

export interface FixtureExpectedResult {
  readonly winnerSide: Side;
  readonly actorTurnCount: number;
  readonly hpDeltaP1MinusP2: number;
  readonly finalHp?: Readonly<Record<Side, number>>;
}

export type ReplayFixtureWithExpected = OriginalReplayFixture & {
  readonly expected?: FixtureExpectedResult;
};

export function parseReplayFixtureJson(text: string): ReplayFixtureWithExpected {
  const parsed = JSON.parse(text) as ReplayFixtureWithExpected;
  validateReplayFixtureShape(parsed);
  return parsed;
}

export function importReplayFixtureConfig(fixture: ReplayFixtureWithExpected): BattleConfig {
  return battleConfigFromReplayFixture(fixture);
}

export function configMatchesImportedFixture(
  fixture: ReplayFixtureWithExpected,
  config: BattleConfig,
): boolean {
  return stableStringify(config) === stableStringify(importReplayFixtureConfig(fixture));
}

function validateReplayFixtureShape(fixture: ReplayFixtureWithExpected): void {
  if (fixture.firstPlayerSide !== "p1" && fixture.firstPlayerSide !== "p2") {
    throw new Error("fixture.firstPlayerSide 必须是 p1 或 p2");
  }
  if (!fixture.players?.p1 || !fixture.players?.p2) {
    throw new Error("fixture.players 必须包含 p1 和 p2");
  }
  for (const side of ["p1", "p2"] as const) {
    const player = fixture.players[side];
    if (!Array.isArray(player.cards) || player.cards.length === 0) {
      throw new Error(`fixture.players.${side}.cards 不能为空`);
    }
    if (!Array.isArray(player.talents)) {
      throw new Error(`fixture.players.${side}.talents 必须是数组`);
    }
    if (!player.permanentBuffTempDatas) {
      throw new Error(`fixture.players.${side}.permanentBuffTempDatas 不能为空`);
    }
  }
}

function stableStringify(value: unknown): string {
  return JSON.stringify(sortJsonValue(value));
}

function sortJsonValue(value: unknown): unknown {
  if (Array.isArray(value)) return value.map(sortJsonValue);
  if (!value || typeof value !== "object") return value;
  return Object.fromEntries(
    Object.entries(value as Record<string, unknown>)
      .sort(([left], [right]) => left.localeCompare(right))
      .map(([key, entry]) => [key, sortJsonValue(entry)]),
  );
}
