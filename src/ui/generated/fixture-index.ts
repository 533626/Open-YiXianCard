import fixtureIndex from "./fixture-index.json";

export interface UiFixtureEntry {
  readonly id: string;
  readonly path: string;
  readonly matchId: string;
  readonly round: number;
  readonly steamBuild: string | null;
  readonly expectedWinner: string | null;
  readonly p1CharacterId: number | null;
  readonly p2CharacterId: number | null;
}

export const UI_FIXTURE_INDEX = fixtureIndex as readonly UiFixtureEntry[];
