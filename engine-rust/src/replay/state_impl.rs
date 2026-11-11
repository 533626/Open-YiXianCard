//! Replay execution state (split from `replay`).

use super::active_mechanisms;
use super::effect_invocation;
use super::{ActiveMechanisms, BattleError, PlayerSide, ReplayCardExecution, ReplayDecisionEvent, ReplayObservationRuntime, ReplayPlayer, ReplayTerminationCause, TraceAttributionBlock};

#[derive(Clone)]
pub(crate) struct ReplayState<const OBSERVE: bool = true> {
    pub(crate) p1: ReplayPlayer,
    pub(crate) p2: ReplayPlayer,
    pub(crate) first_player: PlayerSide,
    pub(crate) battle_round: Option<i64>,
    pub(crate) current_actor: PlayerSide,
    pub(crate) actor_turn: i64,
    pub(crate) max_actor_turns: i64,
    pub(crate) decision_tape: Vec<i64>,
    pub(crate) random_fallback_tape: Vec<i64>,
    /// Battle-start hand card ids per side (fixture handCards). The battle
    /// itself only tracks hand counts; synthetic server values that depend
    /// on hand composition (e.g. 残云封天剑) derive from this snapshot.
    pub(crate) initial_hand_card_ids: [Vec<i64>; 2],
    pub(crate) synthetic_decision_seed: Option<u32>,
    pub(crate) synthetic_decision_sides: Vec<PlayerSide>,
    pub(crate) synthetic_decision_fallback_seed: Option<u32>,
    pub(crate) robust_decisions: bool,
    pub(crate) decision_occurrence: u64,
    pub(crate) card_execution_occurrence: u64,
    pub(crate) current_card_execution: Option<ReplayCardExecution>,
    pub(crate) decision_events: Vec<ReplayDecisionEvent>,
    pub(crate) capture_decision_events: bool,
    pub(crate) uncaptured_synthetic_seeds: Vec<u32>,
    pub(crate) effect_invocation_stack: Vec<effect_invocation::EffectInvocationFrame>,
    /// Which explicit phase table is currently running, when no effect
    /// invocation is active. Receipts recorded inside a phase block are
    /// attributed to the matching phase step (BattleStart/TurnStart/TurnEnd).
    pub(crate) attribution_block: Option<TraceAttributionBlock>,
    pub(crate) fail_on_missing_decision: bool,
    pub(crate) evaluation_error: Option<BattleError>,
    pub(crate) observation: ReplayObservationRuntime,
    pub(crate) termination_cause: Option<ReplayTerminationCause>,
    pub(crate) completed_checkpoint_count: usize,
    pub(crate) active_mechanisms: ActiveMechanisms,
}

impl<const OBSERVE: bool> std::fmt::Debug for ReplayState<OBSERVE> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("ReplayState")
            .field("p1", &self.p1)
            .field("p2", &self.p2)
            .field("first_player", &self.first_player)
            .field("battle_round", &self.battle_round)
            .field("current_actor", &self.current_actor)
            .field("actor_turn", &self.actor_turn)
            .field("max_actor_turns", &self.max_actor_turns)
            .field("decision_tape", &self.decision_tape)
            .field("random_fallback_tape", &self.random_fallback_tape)
            .field("initial_hand_card_ids", &self.initial_hand_card_ids)
            .field("synthetic_decision_seed", &self.synthetic_decision_seed)
            .field("synthetic_decision_sides", &self.synthetic_decision_sides)
            .field("synthetic_decision_fallback_seed", &self.synthetic_decision_fallback_seed)
            .field("robust_decisions", &self.robust_decisions)
            .field("decision_occurrence", &self.decision_occurrence)
            .field("card_execution_occurrence", &self.card_execution_occurrence)
            .field("current_card_execution", &self.current_card_execution)
            .field("decision_events", &self.decision_events)
            .field("capture_decision_events", &self.capture_decision_events)
            .field("uncaptured_synthetic_seeds", &self.uncaptured_synthetic_seeds)
            .field("effect_invocation_stack", &self.effect_invocation_stack)
            .field("attribution_block", &self.attribution_block)
            .field("fail_on_missing_decision", &self.fail_on_missing_decision)
            .field("evaluation_error", &self.evaluation_error)
            .field("observation", &self.observation)
            .field("termination_cause", &self.termination_cause)
            .field("completed_checkpoint_count", &self.completed_checkpoint_count)
            .finish()
    }
}

/// Explicit phase table used to attribute

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    #[inline(always)]
    pub(crate) fn refresh_active_mechanisms(&mut self) {
        #[cfg(feature = "perf-ceiling")]
        let _timer = crate::performance::Timer::new(14);
        self.active_mechanisms = active_mechanisms::detect_active_mechanisms(self);
    }

    pub(crate) fn missing_card_effect(&mut self, card_id: i64, base_id: i64, reason: &str) {
        if self.evaluation_error.is_none() {
            self.evaluation_error = Some(BattleError::MissingRule {
                card_id,
                base_id,
                reason: reason.into(),
                turn: self.actor_turn,
            });
        }
    }

    pub(crate) fn missing_decision(&mut self, reason: &str) {
        if self.fail_on_missing_decision && self.evaluation_error.is_none() {
            self.evaluation_error = Some(BattleError::MissingDecision {
                reason: reason.into(),
                turn: self.actor_turn,
            });
        }
    }
}
