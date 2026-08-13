import type { ReplayFixtureWithExpected } from "./fixture-contract";
import type {
  FixtureConsistencyReport,
  FixtureRunSummary,
} from "./fixture-consistency";
import type { Side, SimulationResult } from "./types";

/** Compare the canonical Rust Worker result directly with fixture evidence. */
export function compareRustFixtureResult(
  fixture: ReplayFixtureWithExpected,
  result: SimulationResult,
): FixtureConsistencyReport {
  const summary = summarize(result);
  const expectedMatch = fixture.expected === undefined
    ? undefined
    : fixture.expected.winnerSide === summary.winnerSide &&
      fixture.expected.actorTurnCount === summary.actorTurnCount &&
      fixture.expected.hpDeltaP1MinusP2 === summary.hpDeltaP1MinusP2 &&
      (fixture.expected.finalHp === undefined ||
        (fixture.expected.finalHp.p1 === summary.finalHp.p1 &&
          fixture.expected.finalHp.p2 === summary.finalHp.p2));
  return {
    engine: summary,
    ui: summary,
    engineMatch: true,
    ...(expectedMatch === undefined ? {} : { expectedMatch }),
  };
}

function summarize(result: SimulationResult): FixtureRunSummary {
  const finalFrame = result.frames.at(-1);
  if (!finalFrame) throw new Error("Rust Worker 战斗结果没有最终帧");
  return {
    winnerSide: result.winnerId as Side | null,
    actorTurnCount: result.finalActorTurn,
    hpDeltaP1MinusP2: finalFrame.players.p1.hp - finalFrame.players.p2.hp,
    finalHp: {
      p1: finalFrame.players.p1.hp,
      p2: finalFrame.players.p2.hp,
    },
  };
}
