use crate::fixture::SOLVER_STARTING_PERTURBATION_FIELDS;
use crate::{
    run_replay_fixture_with_events, BattleFixture, EngineError, PlayerSide, ReplayDecisionEvent,
    ReplayEvent, ReplayPlayerSnapshot, ReplayRun, SolverStartingPerturbation,
};
use serde::{Deserialize, Serialize};

pub const COUNTERFACTUAL_SCHEMA_VERSION: &str = "canonical-counterfactual-v1";

/// One opening-state quantity to remove before the first canonical checkpoint.
///
/// The intervention reuses the analysis-only `solverStartingPerturbations` path,
/// so it can never enter exact replay comparison. `amount` is positive here; the
/// runner turns it into a negative state perturbation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualElement {
    pub id: String,
    pub label: String,
    pub side: PlayerSide,
    pub field: String,
    pub amount: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum CounterfactualDivergenceReason {
    DecisionTape,
    EventSequence,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualElementResult {
    pub element: CounterfactualElement,
    pub first_divergence_actor_turn: Option<i64>,
    pub first_divergence_checkpoint_index: Option<usize>,
    pub first_divergence_reason: Option<CounterfactualDivergenceReason>,
    /// Counterfactual minus baseline at the last checkpoint before divergence.
    /// Negative means removing the element made this side's HP gap worse.
    pub pre_divergence_hp_delta_change_for_side: i64,
    /// Counterfactual minus baseline at each run's terminal state.
    pub terminal_hp_delta_change_for_side: i64,
    pub counterfactual_terminal_hp_delta_for_side: i64,
    pub baseline_winner: PlayerSide,
    pub counterfactual_winner: PlayerSide,
    pub winner_changed: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CounterfactualReport {
    pub schema_version: &'static str,
    pub side: PlayerSide,
    pub baseline_terminal_hp_delta_for_side: i64,
    pub elements: Vec<CounterfactualElementResult>,
}

pub fn explain_fixture_counterfactuals(
    fixture: &BattleFixture,
    side: PlayerSide,
    elements: &[CounterfactualElement],
) -> Result<CounterfactualReport, String> {
    if elements.is_empty() {
        return Err("counterfactual elements must not be empty".to_string());
    }
    let baseline = run_replay_fixture_with_events(fixture).map_err(engine_error)?;
    let baseline_start = baseline
        .events
        .first()
        .ok_or_else(|| "counterfactual baseline emitted no battleStart checkpoint".to_string())?;
    let mut results = Vec::with_capacity(elements.len());
    for element in elements {
        validate_element(element, baseline_start)?;
        let mut counterfactual_fixture = fixture.clone();
        counterfactual_fixture
            .source
            .get_or_insert_default()
            .solver_starting_perturbations
            .push(SolverStartingPerturbation {
                side: element.side,
                field: element.field.clone(),
                amount: -element.amount,
            });
        let counterfactual =
            run_replay_fixture_with_events(&counterfactual_fixture).map_err(engine_error)?;
        results.push(compare_runs(
            side,
            element.clone(),
            &baseline,
            &counterfactual,
        )?);
    }
    Ok(CounterfactualReport {
        schema_version: COUNTERFACTUAL_SCHEMA_VERSION,
        side,
        baseline_terminal_hp_delta_for_side: terminal_hp_delta(&baseline, side)?,
        elements: results,
    })
}

fn validate_element(
    element: &CounterfactualElement,
    baseline_start: &ReplayEvent,
) -> Result<(), String> {
    if element.id.trim().is_empty() {
        return Err("counterfactual element id must not be empty".to_string());
    }
    if element.label.trim().is_empty() {
        return Err(format!(
            "counterfactual element {} label must not be empty",
            element.id
        ));
    }
    if element.amount <= 0 {
        return Err(format!(
            "counterfactual element {} amount must be greater than zero",
            element.id
        ));
    }
    if !SOLVER_STARTING_PERTURBATION_FIELDS.contains(&element.field.as_str()) {
        return Err(format!(
            "counterfactual element {} uses unsupported field {}",
            element.id, element.field
        ));
    }
    if element.field.starts_with("activated") {
        return Err(format!(
            "counterfactual element {} cannot remove activation field {}",
            element.id, element.field
        ));
    }
    let snapshot = snapshot_for_side(baseline_start, element.side);
    let available = snapshot_field(snapshot, &element.field).ok_or_else(|| {
        format!(
            "counterfactual element {} field {} has no snapshot projection",
            element.id, element.field
        )
    })?;
    if element.amount > available.max(0) {
        return Err(format!(
            "counterfactual element {} removes {} {}, but battleStart only has {}",
            element.id, element.amount, element.field, available
        ));
    }
    Ok(())
}

fn compare_runs(
    side: PlayerSide,
    element: CounterfactualElement,
    baseline: &ReplayRun,
    counterfactual: &ReplayRun,
) -> Result<CounterfactualElementResult, String> {
    let event_divergence = first_event_divergence(&baseline.events, &counterfactual.events);
    let decision_divergence =
        first_decision_divergence(&baseline.decision_events, &counterfactual.decision_events);
    let divergence = match (decision_divergence, event_divergence) {
        (Some(decision), Some(event)) if decision.actor_turn <= event.actor_turn => Some(decision),
        (_, Some(event)) => Some(event),
        (Some(decision), None) => Some(decision),
        (None, None) => None,
    };
    let baseline_prefix = clean_prefix_event(baseline, divergence);
    let counterfactual_prefix = clean_prefix_event(counterfactual, divergence);
    let pre_divergence_change =
        hp_delta(counterfactual_prefix, side) - hp_delta(baseline_prefix, side);
    let baseline_terminal = terminal_hp_delta(baseline, side)?;
    let counterfactual_terminal = terminal_hp_delta(counterfactual, side)?;
    Ok(CounterfactualElementResult {
        element,
        first_divergence_actor_turn: divergence.map(|value| value.actor_turn),
        first_divergence_checkpoint_index: divergence.and_then(|value| value.checkpoint_index),
        first_divergence_reason: divergence.map(|value| value.reason),
        pre_divergence_hp_delta_change_for_side: pre_divergence_change,
        terminal_hp_delta_change_for_side: counterfactual_terminal - baseline_terminal,
        counterfactual_terminal_hp_delta_for_side: counterfactual_terminal,
        baseline_winner: baseline.summary.winner_side,
        counterfactual_winner: counterfactual.summary.winner_side,
        winner_changed: baseline.summary.winner_side != counterfactual.summary.winner_side,
    })
}

#[derive(Debug, Clone, Copy)]
struct Divergence {
    actor_turn: i64,
    checkpoint_index: Option<usize>,
    reason: CounterfactualDivergenceReason,
}

fn first_event_divergence(
    baseline: &[ReplayEvent],
    counterfactual: &[ReplayEvent],
) -> Option<Divergence> {
    let common_len = baseline.len().min(counterfactual.len());
    for index in 0..common_len {
        if !same_event_identity(&baseline[index], &counterfactual[index]) {
            return Some(Divergence {
                actor_turn: baseline[index].turn.min(counterfactual[index].turn),
                checkpoint_index: Some(index),
                reason: CounterfactualDivergenceReason::EventSequence,
            });
        }
    }
    if baseline.len() == counterfactual.len() {
        return None;
    }
    let event = baseline
        .get(common_len)
        .or_else(|| counterfactual.get(common_len))?;
    Some(Divergence {
        actor_turn: event.turn,
        checkpoint_index: Some(common_len),
        reason: CounterfactualDivergenceReason::EventSequence,
    })
}

fn first_decision_divergence(
    baseline: &[ReplayDecisionEvent],
    counterfactual: &[ReplayDecisionEvent],
) -> Option<Divergence> {
    let common_len = baseline.len().min(counterfactual.len());
    for index in 0..common_len {
        if baseline[index] != counterfactual[index] {
            return Some(Divergence {
                actor_turn: baseline[index]
                    .actor_turn
                    .min(counterfactual[index].actor_turn),
                checkpoint_index: None,
                reason: CounterfactualDivergenceReason::DecisionTape,
            });
        }
    }
    if baseline.len() == counterfactual.len() {
        return None;
    }
    let decision = baseline
        .get(common_len)
        .or_else(|| counterfactual.get(common_len))?;
    Some(Divergence {
        actor_turn: decision.actor_turn,
        checkpoint_index: None,
        reason: CounterfactualDivergenceReason::DecisionTape,
    })
}

fn clean_prefix_event(run: &ReplayRun, divergence: Option<Divergence>) -> &ReplayEvent {
    let Some(divergence) = divergence else {
        return run
            .events
            .last()
            .expect("validated replay has a terminal event");
    };
    run.events
        .iter()
        .rev()
        .find(|event| event.turn < divergence.actor_turn)
        .unwrap_or_else(|| {
            run.events
                .first()
                .expect("validated replay has a battleStart event")
        })
}

fn same_event_identity(left: &ReplayEvent, right: &ReplayEvent) -> bool {
    left.turn == right.turn
        && left.kind == right.kind
        && left.actor == right.actor
        && left.slot == right.slot
        && left.card_id == right.card_id
}

fn terminal_hp_delta(run: &ReplayRun, side: PlayerSide) -> Result<i64, String> {
    run.events
        .last()
        .map(|event| hp_delta(event, side))
        .ok_or_else(|| "counterfactual replay emitted no terminal checkpoint".to_string())
}

fn hp_delta(event: &ReplayEvent, side: PlayerSide) -> i64 {
    let raw = event.p1.hp - event.p2.hp;
    if side == PlayerSide::P1 {
        raw
    } else {
        -raw
    }
}

fn snapshot_for_side(event: &ReplayEvent, side: PlayerSide) -> &ReplayPlayerSnapshot {
    if side == PlayerSide::P1 {
        &event.p1
    } else {
        &event.p2
    }
}

fn snapshot_field(snapshot: &ReplayPlayerSnapshot, field: &str) -> Option<i64> {
    Some(match field {
        "hp" => snapshot.hp,
        "maxHp" => snapshot.max_hp,
        "defense" => snapshot.defense,
        "guard" => snapshot.guard,
        "anima" => snapshot.anima,
        "momentum" => snapshot.momentum,
        "agility" => snapshot.agility,
        "swordIntent" => snapshot.sword_intent,
        "sharpness" => snapshot.sharpness,
        "attackBonus" => snapshot.attack_bonus,
        "physique" => snapshot.physique,
        "internalInjury" => snapshot.internal_injury,
        "weakness" => snapshot.weakness,
        "flaw" => snapshot.flaw,
        "attackReduction" => snapshot.attack_reduction,
        "entangle" => snapshot.entangle,
        "externalInjury" => snapshot.external_injury,
        "hexagram" => snapshot.hexagram,
        "starPower" => snapshot.star_power,
        "cloudChain" => snapshot.cloud_chain,
        "waterMomentum" => snapshot.water_momentum,
        "cloudSea" => snapshot.cloud_sea,
        "activatedMetal" => snapshot.activated_metal,
        "activatedWater" => snapshot.activated_water,
        "activatedWood" => snapshot.activated_wood,
        "activatedFire" => snapshot.activated_fire,
        "activatedEarth" => snapshot.activated_earth,
        _ => return None,
    })
}

fn engine_error(error: EngineError) -> String {
    error.to_string()
}
