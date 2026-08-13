//! Rust port of the yixiancard battle engine.
//!
//! This crate is the canonical battle implementation used by native tooling
//! and the browser through its raw WebAssembly ABI.

pub mod data;
pub mod fixture;
pub mod identity;
pub mod model;
pub mod replay;
pub mod solver;
#[cfg(target_arch = "wasm32")]
mod wasm;

pub use data::{load_battle_data_snapshot, BattleDataCounts, BattleDataSnapshot};
pub use fixture::{
    candidate_fixture_path, load_fixture_file, BattleFixture, FixtureExpected, FixturePlayer,
    FixtureSource, SolverStartingPerturbation,
};
pub use identity::{engine_identity, EngineIdentity};
pub use model::{CardDefinition, PlayerSide, DECK_SIZE};
pub use replay::{
    engine_contract_fixture, evaluate_replay_fixture_fallible, original_card_definition_by_id,
    run_replay_fixture, run_replay_fixture_file, run_replay_fixture_with_detailed_events,
    run_replay_fixture_with_events, run_replay_fixture_with_parity_events,
    run_replay_fixture_with_ui_events, trace_replay_fixture_hooks, BattleError,
    ReplayAttackSegment, ReplayDecisionDomain, ReplayDecisionEvent, ReplayDecisionIntegerRange,
    ReplayDecisionKind, ReplayDecisionProvider, ReplayDetailEntry, ReplayDetailedEvent,
    ReplayDetailedRun, ReplayDetailedStep, ReplayEvaluationRun, ReplayEvent, ReplayEventKind,
    ReplayHookCategory, ReplayHookTrace, ReplayHookTraceChange, ReplayHookTraceStep,
    ReplayPlayerSnapshot, ReplayPreventionPair, ReplayPreventionState, ReplayRun, ReplaySummary,
    ReplayTerminationCause, ReplayTurnEndHookPair, ReplayTurnEndHookReceipt,
    ReplayTurnEndHookSnapshot, ReplayUiCardSlotSnapshot, ReplayUiEvent, ReplayUiPlayerSnapshot,
    ReplayUiRun,
};

#[derive(Debug, thiserror::Error)]
pub enum EngineError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid fixture: {0}")]
    InvalidFixture(String),
    #[error("battle error: {0}")]
    Battle(#[from] BattleError),
}

pub type Result<T> = std::result::Result<T, EngineError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_engine_owned_contract_fixture() {
        let fixture = engine_contract_fixture().expect("contract fixture builds");
        fixture.validate().expect("contract fixture validates");
        assert_eq!(fixture.players.p1.cards.len(), DECK_SIZE);
        assert_eq!(fixture.players.p2.cards.len(), DECK_SIZE);
        assert!(fixture.expected.actor_turn_count > 0);
    }

    #[test]
    fn battle_errors_are_not_reported_as_invalid_fixtures() {
        let error = EngineError::Battle(BattleError::Invariant {
            message: "state corrupt".to_string(),
        });
        assert_eq!(error.to_string(), "battle error: state corrupt");
    }
}
