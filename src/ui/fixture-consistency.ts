import type {
  FixtureExpectedResult,
  ReplayFixtureWithExpected,
} from "./fixture-contract";
import type {
  Side,
} from "./types";

export type {
  FixtureExpectedResult,
  ReplayFixtureWithExpected,
} from "./fixture-contract";
export {
  configMatchesImportedFixture,
  importReplayFixtureConfig,
  parseReplayFixtureJson,
} from "./fixture-contract";

export interface FixtureRunSummary {
  readonly winnerSide: Side | null;
  readonly actorTurnCount: number;
  readonly hpDeltaP1MinusP2: number;
  readonly finalHp: Readonly<Record<Side, number>>;
}

export interface FixtureConsistencyReport {
  readonly engine: FixtureRunSummary;
  readonly ui: FixtureRunSummary;
  readonly engineMatch: boolean;
  readonly expectedMatch?: boolean;
}
