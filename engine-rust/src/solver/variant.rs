use crate::fixture::{apply_historical_card_patch, BattleFixture, FixtureSource};
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::{evaluate_replay_fixture_fallible, run_replay_fixture_with_events_fallible};
use crate::{
    original_card_definition_by_id, ReplayDecisionEvent, ReplayDecisionProvider, ReplayRun,
    ReplaySummary,
};
use serde::Serialize;
use std::collections::BTreeSet;

use super::rule_impact::{compute_rule_impact, SolverRuleImpactReport};
use super::value::{compute_value_metrics, value_score, ScoreProfile, SolverValueMetrics};
use super::FAILED_SCORE;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverEvaluation {
    pub side: PlayerSide,
    pub score_profile: ScoreProfile,
    pub winner: PlayerSide,
    pub win_for_side: bool,
    pub actor_turn: f64,
    pub p1_hp: f64,
    pub p2_hp: f64,
    pub hp_delta_for_side: f64,
    pub score: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value_metrics: Option<SolverValueMetrics>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub rule_impacts: Vec<SolverRuleImpactReport>,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub decision_events: Vec<ReplayDecisionEvent>,
    pub failed: bool,
    pub warnings: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seed_aggregate: Option<SolverSeedAggregate>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverSeedAggregate {
    pub ranking_policy: &'static str,
    pub seeds_used: Vec<u32>,
    pub win_count: usize,
    pub average_score: f64,
    pub synthetic_decision_seeds_used: Vec<u32>,
    pub used_synthetic_decisions: bool,
}

struct SuccessfulEvaluationInput {
    summary: ReplaySummary,
    p1_hp: i64,
    p2_hp: i64,
    value_metrics: Option<SolverValueMetrics>,
    rule_impacts: Vec<SolverRuleImpactReport>,
    decision_events: Vec<ReplayDecisionEvent>,
    warnings: Vec<String>,
}

pub fn evaluate_fixture(
    fixture: &BattleFixture,
    side: PlayerSide,
    score_profile: ScoreProfile,
) -> SolverEvaluation {
    match score_profile {
        ScoreProfile::HpDelta => match evaluate_replay_fixture_fallible(fixture) {
            Ok(run) => evaluation_from_summary(
                side,
                score_profile,
                SuccessfulEvaluationInput {
                    summary: run.summary,
                    p1_hp: run.p1.hp,
                    p2_hp: run.p2.hp,
                    value_metrics: None,
                    rule_impacts: Vec::new(),
                    decision_events: run.decision_events,
                    warnings: Vec::new(),
                },
            ),
            Err(error) => failed_evaluation(side, score_profile, error),
        },
        ScoreProfile::ValueV0 => match run_replay_fixture_with_events_fallible(fixture) {
            Ok(run) => evaluation_from_event_run(side, score_profile, run, Vec::new()),
            Err(error) => failed_evaluation(side, score_profile, error),
        },
    }
}

pub fn evaluate_fixture_across_battle_seeds(
    fixture: &BattleFixture,
    side: PlayerSide,
    score_profile: ScoreProfile,
    battle_seeds: Option<&[u32]>,
) -> SolverEvaluation {
    let Some(seeds) = battle_seeds else {
        return evaluate_fixture(fixture, side, score_profile);
    };
    if seeds.is_empty() {
        return failed_evaluation(
            side,
            score_profile,
            "battle seeds reached evaluation without public-entry normalization".to_string(),
        );
    }
    aggregate_seed_evaluations(
        seeds
            .iter()
            .map(|seed| {
                evaluate_fixture(
                    &with_typed_decision_fallback(fixture, *seed),
                    side,
                    score_profile,
                )
            })
            .collect(),
        seeds,
    )
}

pub fn evaluate_fixture_deck(
    fixture: &BattleFixture,
    side: PlayerSide,
    cards: &[CardDefinition],
    hand_card_ids: Option<Vec<i64>>,
    score_profile: ScoreProfile,
) -> SolverEvaluation {
    let variant = create_fixture_variant(fixture, side, cards, hand_card_ids);
    evaluate_fixture(&variant, side, score_profile)
}

pub fn evaluate_fixture_deck_across_battle_seeds(
    fixture: &BattleFixture,
    side: PlayerSide,
    cards: &[CardDefinition],
    hand_card_ids: Option<Vec<i64>>,
    score_profile: ScoreProfile,
    battle_seeds: Option<&[u32]>,
) -> SolverEvaluation {
    let Some(seeds) = battle_seeds else {
        return evaluate_fixture_deck(fixture, side, cards, hand_card_ids, score_profile);
    };
    if seeds.is_empty() {
        return failed_evaluation(
            side,
            score_profile,
            "battle seeds reached evaluation without public-entry normalization".to_string(),
        );
    }
    aggregate_seed_evaluations(
        seeds
            .iter()
            .map(|seed| {
                evaluate_fixture_deck(
                    &with_typed_decision_fallback(fixture, *seed),
                    side,
                    cards,
                    hand_card_ids.clone(),
                    score_profile,
                )
            })
            .collect(),
        seeds,
    )
}

fn with_typed_decision_fallback(fixture: &BattleFixture, seed: u32) -> BattleFixture {
    let mut seeded = fixture.clone();
    let source = seeded.source.get_or_insert_with(FixtureSource::default);
    source.synthetic_decision_fallback_seed = Some(seed);
    seeded
}

fn aggregate_seed_evaluations(
    evaluations: Vec<SolverEvaluation>,
    seeds: &[u32],
) -> SolverEvaluation {
    let count = evaluations.len() as f64;
    // The two callers reject an empty seed slice before constructing evaluations.
    let first = evaluations
        .first()
        .expect("non-empty battle seeds produce at least one evaluation");
    let average =
        |select: fn(&SolverEvaluation) -> f64| evaluations.iter().map(select).sum::<f64>() / count;
    let win_count = evaluations.iter().filter(|item| item.win_for_side).count();
    let wins = win_count * 2 >= evaluations.len();
    let synthetic_decision_seeds_used = evaluations
        .iter()
        .flat_map(|item| item.decision_events.iter())
        .filter(|event| event.provider == ReplayDecisionProvider::SeededSynthetic)
        .filter_map(|event| event.seed)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let average_score = average(|item| item.score);
    SolverEvaluation {
        side: first.side,
        score_profile: first.score_profile,
        winner: if wins {
            first.side
        } else {
            opposite_side(first.side)
        },
        win_for_side: wins,
        actor_turn: average(|item| item.actor_turn),
        p1_hp: average(|item| item.p1_hp),
        p2_hp: average(|item| item.p2_hp),
        hp_delta_for_side: average(|item| item.hp_delta_for_side),
        score: average_score,
        value_metrics: first
            .value_metrics
            .as_ref()
            .map(|_| average_value_metrics(&evaluations)),
        rule_impacts: Vec::new(),
        decision_events: evaluations
            .iter()
            .flat_map(|item| item.decision_events.clone())
            .collect(),
        failed: evaluations.iter().any(|item| item.failed),
        warnings: evaluations
            .iter()
            .flat_map(|item| item.warnings.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        seed_aggregate: Some(SolverSeedAggregate {
            ranking_policy: "win-count-then-average-score",
            seeds_used: seeds.to_vec(),
            win_count,
            average_score,
            synthetic_decision_seeds_used: synthetic_decision_seeds_used.clone(),
            used_synthetic_decisions: !synthetic_decision_seeds_used.is_empty(),
        }),
    }
}

fn average_value_metrics(evaluations: &[SolverEvaluation]) -> SolverValueMetrics {
    let metrics = evaluations
        .iter()
        .map(|item| item.value_metrics.as_ref().expect("value metrics"))
        .collect::<Vec<_>>();
    let count = metrics.len() as f64;
    let average = |select: fn(&SolverValueMetrics) -> f64| {
        metrics.iter().map(|item| select(item)).sum::<f64>() / count
    };
    SolverValueMetrics {
        terminal_value_for_side: average(|item| item.terminal_value_for_side),
        terminal_hp_for_side: average(|item| item.terminal_hp_for_side),
        terminal_shield_for_side: average(|item| item.terminal_shield_for_side),
        terminal_defense_for_side: average(|item| item.terminal_defense_for_side),
        terminal_guard_for_side: average(|item| item.terminal_guard_for_side),
        terminal_resource_for_side: average(|item| item.terminal_resource_for_side),
        terminal_debuff_for_side: average(|item| item.terminal_debuff_for_side),
        terminal_tempo_for_side: average(|item| item.terminal_tempo_for_side),
        terminal_tempo_count_for_side: average(|item| item.terminal_tempo_count_for_side),
        area_score_for_side: average(|item| item.area_score_for_side),
        hp_area_for_side: average(|item| item.hp_area_for_side),
        resource_area_for_side: average(|item| item.resource_area_for_side),
        debuff_area_for_side: average(|item| item.debuff_area_for_side),
        hp_area_score_for_side: average(|item| item.hp_area_score_for_side),
        resource_area_score_for_side: average(|item| item.resource_area_score_for_side),
        debuff_area_score_for_side: average(|item| item.debuff_area_score_for_side),
        area_sample_count: average(|item| item.area_sample_count),
        audit_mismatch_fields: metrics
            .iter()
            .flat_map(|item| item.audit_mismatch_fields.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
    }
}

fn opposite_side(side: PlayerSide) -> PlayerSide {
    match side {
        PlayerSide::P1 => PlayerSide::P2,
        PlayerSide::P2 => PlayerSide::P1,
    }
}

pub(crate) fn create_fixture_variant(
    fixture: &BattleFixture,
    side: PlayerSide,
    cards: &[CardDefinition],
    hand_card_ids: Option<Vec<i64>>,
) -> BattleFixture {
    let mut variant = fixture.clone();
    let player = match side {
        PlayerSide::P1 => &mut variant.players.p1,
        PlayerSide::P2 => &mut variant.players.p2,
    };
    player.cards = cards.to_vec();
    if let Some(hand_card_ids) = hand_card_ids {
        player.hand_cards = hand_card_ids;
    }
    variant
        .historical_card_overrides
        .retain(|override_entry| override_entry.side != side);
    variant
}

pub(crate) fn fixture_deck_candidates(
    fixture: &BattleFixture,
    side: PlayerSide,
) -> Vec<CardDefinition> {
    let player = match side {
        PlayerSide::P1 => &fixture.players.p1,
        PlayerSide::P2 => &fixture.players.p2,
    };
    player
        .cards
        .iter()
        .enumerate()
        .map(|(slot_index, card)| {
            let completed = complete_original_card(card);
            if let Some(override_entry) = fixture
                .historical_card_overrides
                .iter()
                .find(|candidate| candidate.side == side && candidate.slot_index == slot_index)
            {
                apply_historical_card_patch(completed, &override_entry.patch)
            } else {
                completed
            }
        })
        .collect()
}

pub(crate) fn fixture_hand_candidates(
    fixture: &BattleFixture,
    side: PlayerSide,
) -> Vec<CardDefinition> {
    let player = match side {
        PlayerSide::P1 => &fixture.players.p1,
        PlayerSide::P2 => &fixture.players.p2,
    };
    player
        .hand_cards
        .iter()
        .map(|card_id| {
            let mut card =
                original_card_definition_by_id(*card_id).unwrap_or_else(|| CardDefinition {
                    id: *card_id,
                    base_id: None,
                    name: format!("card:{card_id}"),
                    card_type: None,
                    attack: None,
                    random_attack: None,
                    random_defense: None,
                    attack_count: None,
                    defense: None,
                    damage: None,
                    anima: None,
                    hp_cost: None,
                    action_again: None,
                    physique: None,
                    sword_intent: None,
                    hexagram: None,
                    rarity: None,
                    career_name: None,
                    other_params: Vec::new(),
                });
            card.name = format!("card:{card_id}");
            card
        })
        .collect()
}

fn complete_original_card(card: &CardDefinition) -> CardDefinition {
    let mut completed = original_card_definition_by_id(card.id).unwrap_or_else(|| card.clone());
    completed.id = card.id;
    completed.base_id = card.base_id.or(completed.base_id);
    completed.name = card.name.clone();
    completed.card_type = card.card_type.clone().or(completed.card_type);
    completed.attack = card.attack.or(completed.attack);
    completed.random_attack = card.random_attack.or(completed.random_attack);
    completed.random_defense = card.random_defense.or(completed.random_defense);
    completed.attack_count = card.attack_count.or(completed.attack_count);
    completed.defense = card.defense.or(completed.defense);
    completed.damage = card.damage.or(completed.damage);
    completed.anima = card.anima.or(completed.anima);
    completed.hp_cost = card.hp_cost.or(completed.hp_cost);
    completed.action_again = card.action_again.or(completed.action_again);
    completed.physique = card.physique.or(completed.physique);
    completed.sword_intent = card.sword_intent.or(completed.sword_intent);
    completed.hexagram = card.hexagram.or(completed.hexagram);
    if !card.other_params.is_empty() {
        completed.other_params = card.other_params.clone();
    }
    completed
}

fn evaluation_from_event_run(
    side: PlayerSide,
    score_profile: ScoreProfile,
    run: ReplayRun,
    warnings: Vec<String>,
) -> SolverEvaluation {
    let final_event = run
        .events
        .last()
        .expect("replay run must contain battleEnd");
    let value_metrics =
        (score_profile == ScoreProfile::ValueV0).then(|| compute_value_metrics(&run, side));
    let decision_events = run.decision_events;
    evaluation_from_summary(
        side,
        score_profile,
        SuccessfulEvaluationInput {
            summary: run.summary,
            p1_hp: final_event.p1.hp,
            p2_hp: final_event.p2.hp,
            value_metrics,
            rule_impacts: Vec::new(),
            decision_events,
            warnings,
        },
    )
}

fn evaluation_from_summary(
    side: PlayerSide,
    score_profile: ScoreProfile,
    input: SuccessfulEvaluationInput,
) -> SolverEvaluation {
    let SuccessfulEvaluationInput {
        summary,
        p1_hp,
        p2_hp,
        value_metrics,
        rule_impacts,
        decision_events,
        warnings,
    } = input;
    let hp_delta_for_side = match side {
        PlayerSide::P1 => p1_hp - p2_hp,
        PlayerSide::P2 => p2_hp - p1_hp,
    };
    let score = value_metrics
        .as_ref()
        .map(value_score)
        .unwrap_or(hp_delta_for_side as f64);
    SolverEvaluation {
        side,
        score_profile,
        winner: summary.winner_side,
        win_for_side: summary.winner_side == side,
        actor_turn: summary.actor_turn_count as f64,
        p1_hp: p1_hp as f64,
        p2_hp: p2_hp as f64,
        hp_delta_for_side: hp_delta_for_side as f64,
        score,
        value_metrics,
        rule_impacts,
        decision_events,
        failed: false,
        warnings,
        seed_aggregate: None,
    }
}

/// Canonical checkpoint attribution for one fixture, without running a search.
///
/// The browser and TUI need the same attribution the analysis pipeline consumes,
/// and recomputing value channels on the consumer side would fork the weight
/// table. Callers get `canonical-rule-impact-v1` verbatim.
pub fn explain_fixture_rule_impact(
    fixture: &BattleFixture,
    side: PlayerSide,
) -> Result<SolverRuleImpactReport, String> {
    capture_rule_impacts_for_fixture(fixture, side, None)?
        .into_iter()
        .next()
        .ok_or_else(|| "rule impact capture produced no report".to_string())
}

pub(crate) fn capture_rule_impacts_for_fixture(
    fixture: &BattleFixture,
    side: PlayerSide,
    battle_seeds: Option<&[u32]>,
) -> Result<Vec<SolverRuleImpactReport>, String> {
    let seeds = battle_seeds
        .map(|items| items.iter().copied().map(Some).collect::<Vec<_>>())
        .unwrap_or_else(|| vec![None]);
    seeds
        .into_iter()
        .map(|seed| {
            let replay_fixture = seed
                .map(|value| with_typed_decision_fallback(fixture, value))
                .unwrap_or_else(|| fixture.clone());
            run_replay_fixture_with_events_fallible(&replay_fixture)
                .map(|run| compute_rule_impact(&run, side, seed))
        })
        .collect()
}

fn failed_evaluation(
    side: PlayerSide,
    score_profile: ScoreProfile,
    error: String,
) -> SolverEvaluation {
    SolverEvaluation {
        side,
        score_profile,
        winner: match side {
            PlayerSide::P1 => PlayerSide::P2,
            PlayerSide::P2 => PlayerSide::P1,
        },
        win_for_side: false,
        actor_turn: 0.0,
        p1_hp: 0.0,
        p2_hp: 0.0,
        hp_delta_for_side: FAILED_SCORE as f64,
        score: FAILED_SCORE as f64,
        value_metrics: None,
        rule_impacts: Vec::new(),
        decision_events: Vec::new(),
        failed: true,
        warnings: vec![format!("error:{error}")],
        seed_aggregate: None,
    }
}
