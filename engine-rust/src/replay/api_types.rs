//! Public replay API types (split from `replay`).

use crate::fixture::{BattleFixture, FinalLife};
use crate::model::PlayerSide;

use super::life_settle::LifeSettleDetail;
use super::{ReplayDecisionEvent, ReplayEvent, ReplayPlayerSnapshot, ReplayPreventionPair, ReplayUiEvent};

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySummary {
    pub winner_side: PlayerSide,
    pub actor_turn_count: i64,
    pub hp_delta_p1_minus_p2: i64,
    pub final_life: Option<FinalLife>,
    pub life_damage: Option<i64>,
    pub life_detail: Option<LifeSettleDetail>,
}

impl ReplaySummary {
    pub fn matches_fixture(&self, fixture: &BattleFixture) -> bool {
        self.winner_side == fixture.expected.winner_side
            && self.actor_turn_count == fixture.expected.actor_turn_count
            && self.hp_delta_p1_minus_p2 == fixture.expected.hp_delta_p1_minus_p2
            && fixture.expected.final_life.as_ref().map(|expected| Some(expected) == self.final_life.as_ref()).unwrap_or(true)
            && fixture.expected.life_damage.map(|expected| Some(expected) == self.life_damage).unwrap_or(true)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRun {
    pub summary: ReplaySummary,
    pub termination_cause: ReplayTerminationCause,
    pub completed_checkpoint_count: usize,
    pub events: Vec<ReplayEvent>,
    /// One entry per `events` element, same index. Kept beside the events instead of
    /// inside their snapshots so the oracle parity protocol stays exactly 29 fields.
    pub prevention: Vec<ReplayPreventionPair>,
    pub decision_events: Vec<ReplayDecisionEvent>,
}

/// Browser-facing replay stream with live state that intentionally stays outside
/// the exact 29-field original-oracle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReplayUiRun {
    pub summary: ReplaySummary,
    pub termination_cause: ReplayTerminationCause,
    pub completed_checkpoint_count: usize,
    pub events: Vec<ReplayUiEvent>,
    pub decision_events: Vec<ReplayDecisionEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ReplayTerminationCause {
    TurnStartLethal,
    CardLethal,
    ActionAgainLethal,
    TurnEndLethal,
    MaxTurn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvaluationRun {
    pub summary: ReplaySummary,
    pub p1: ReplayPlayerSnapshot,
    pub p2: ReplayPlayerSnapshot,
    pub decision_events: Vec<ReplayDecisionEvent>,
}
