use crate::fixture::{
    apply_historical_card_patch, BattleFixture, BattleFixtureView, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide};
use crate::{
    original_card_definition_by_id, ReplayDecisionEvent, ReplayDecisionProvider, ReplaySummary,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

use super::FAILED_SCORE;

/// Wire selector kept so requests can keep sending `"scoreProfile": "hpDelta"` and results keep
/// echoing it. hpDelta is the only profile: the value-v0 profile (weights, area terms, rule-impact
/// attribution) was removed on 2026-10-21; `git log -- engine-rust/src/solver/value.rs` recovers it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum ScoreProfile {
    #[serde(rename = "hpDelta")]
    #[default]
    HpDelta,
}

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
    /// 终局时本方体魄（未截上限）。体魄跨轮保留（锻玄宗突破路之一），驱动侧据此给阵型的
    /// 长期收益计价；只是读数，不参与 winner / actorTurn / hpDelta。
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physique_for_side: Option<f64>,
    pub score: f64,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub decision_events: Vec<ReplayDecisionEvent>,
    #[serde(skip)]
    pub(crate) uncaptured_synthetic_seeds: Vec<u32>,
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
    p1_physique: i64,
    p2_physique: i64,
    decision_events: Vec<ReplayDecisionEvent>,
    uncaptured_synthetic_seeds: Vec<u32>,
    warnings: Vec<String>,
}

pub fn evaluate_fixture(
    fixture: &BattleFixture,
    side: PlayerSide,
    score_profile: ScoreProfile,
) -> SolverEvaluation {
    evaluate_fixture_view(&BattleFixtureView::new(fixture), side, score_profile)
}

pub(crate) fn evaluate_fixture_view(
    fixture: &BattleFixtureView<'_>,
    side: PlayerSide,
    score_profile: ScoreProfile,
) -> SolverEvaluation {
    match crate::replay::evaluate_replay_terminal_fallible(fixture) {
        Ok(run) => evaluation_from_summary(
            side,
            score_profile,
            SuccessfulEvaluationInput {
                summary: run.summary,
                p1_hp: run.hp[0],
                p2_hp: run.hp[1],
                p1_physique: run.physique[0],
                p2_physique: run.physique[1],
                decision_events: run.decision_events,
                uncaptured_synthetic_seeds: run.uncaptured_synthetic_seeds,
                warnings: Vec::new(),
            },
        ),
        Err(error) => failed_evaluation(side, score_profile, error),
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
    let mut variant = BattleFixtureView::new(fixture);
    variant.with_deck(side, cards, hand_card_ids.as_deref());
    evaluate_fixture_view(&variant, side, score_profile)
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
    // 换阵型只克隆一次，再逐种子改决策回退种子（与「先按种子克隆、再换阵型」等价：
    // 两步改的是不同字段）。原写法每个种子要克隆整个 fixture 两次。
    let mut variant = BattleFixtureView::new(fixture);
    variant.with_deck(side, cards, hand_card_ids.as_deref());
    aggregate_seed_evaluations(
        seeds
            .iter()
            .map(|seed| {
                variant.fallback_seed_override = Some(*seed);
                evaluate_fixture_view(&variant, side, score_profile)
            })
            .collect(),
        seeds,
    )
}

pub(crate) fn evaluate_fixture_view_deck(
    fixture: &BattleFixtureView<'_>,
    side: PlayerSide,
    cards: &[CardDefinition],
    score_profile: ScoreProfile,
    seeds: Option<&[u32]>,
) -> SolverEvaluation {
    let mut variant = fixture.reborrow();
    variant.with_deck(side, cards, None);
    let Some(seeds) = seeds else {
        return evaluate_fixture_view(&variant, side, score_profile);
    };
    aggregate_seed_evaluations(
        seeds
            .iter()
            .map(|seed| {
                variant.fallback_seed_override = Some(*seed);
                evaluate_fixture_view(&variant, side, score_profile)
            })
            .collect(),
        seeds,
    )
}

fn with_typed_decision_fallback(fixture: &BattleFixture, seed: u32) -> BattleFixture {
    let mut seeded = fixture.clone();
    set_typed_decision_fallback(&mut seeded, seed);
    seeded
}

pub(super) fn set_typed_decision_fallback(fixture: &mut BattleFixture, seed: u32) {
    let source = fixture.source.get_or_insert_with(FixtureSource::default);
    source.synthetic_decision_fallback_seed = Some(seed);
}

fn aggregate_seed_evaluations(
    mut evaluations: Vec<SolverEvaluation>,
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
    let mut synthetic_decision_seeds_used = evaluations
        .iter()
        .flat_map(|item| item.decision_events.iter())
        .filter(|event| event.provider == ReplayDecisionProvider::SeededSynthetic)
        .filter_map(|event| event.seed)
        .chain(
            evaluations
                .iter()
                .flat_map(|item| item.uncaptured_synthetic_seeds.iter().copied()),
        )
        .collect::<Vec<_>>();
    synthetic_decision_seeds_used.sort_unstable();
    synthetic_decision_seeds_used.dedup();
    let used_synthetic_decisions = !synthetic_decision_seeds_used.is_empty();
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
        physique_for_side: evaluations
            .iter()
            .map(|item| item.physique_for_side)
            .sum::<Option<f64>>()
            .map(|total| total / count),
        score: average_score,
        decision_events: evaluations
            .iter_mut()
            .flat_map(|item| std::mem::take(&mut item.decision_events))
            .collect(),
        uncaptured_synthetic_seeds: Vec::new(),
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
            synthetic_decision_seeds_used,
            used_synthetic_decisions,
        }),
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
    fixture_view_deck_candidates(&BattleFixtureView::new(fixture), side)
}

pub(crate) fn fixture_view_deck_candidates(
    fixture: &BattleFixtureView<'_>,
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
                    name: format!("card:{card_id}").into(),
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
                    other_params: Vec::new().into(),
                });
            card.name = format!("card:{card_id}").into();
            card
        })
        .collect()
}

fn complete_original_card(card: &CardDefinition) -> CardDefinition {
    let mut completed = original_card_definition_by_id(card.id).unwrap_or_else(|| card.clone());
    completed.id = card.id;
    completed.base_id = card.base_id.or(completed.base_id);
    completed.name = card.name;
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

fn evaluation_from_summary(
    side: PlayerSide,
    score_profile: ScoreProfile,
    input: SuccessfulEvaluationInput,
) -> SolverEvaluation {
    let SuccessfulEvaluationInput {
        summary,
        p1_hp,
        p2_hp,
        p1_physique,
        p2_physique,
        decision_events,
        uncaptured_synthetic_seeds,
        warnings,
    } = input;
    let hp_delta_for_side = match side {
        PlayerSide::P1 => p1_hp - p2_hp,
        PlayerSide::P2 => p2_hp - p1_hp,
    };
    SolverEvaluation {
        side,
        score_profile,
        winner: summary.winner_side,
        win_for_side: summary.winner_side == side,
        actor_turn: summary.actor_turn_count as f64,
        p1_hp: p1_hp as f64,
        p2_hp: p2_hp as f64,
        hp_delta_for_side: hp_delta_for_side as f64,
        physique_for_side: Some(match side {
            PlayerSide::P1 => p1_physique,
            PlayerSide::P2 => p2_physique,
        } as f64),
        score: hp_delta_for_side as f64,
        decision_events,
        uncaptured_synthetic_seeds,
        failed: false,
        warnings,
        seed_aggregate: None,
    }
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
        physique_for_side: None,
        score: FAILED_SCORE as f64,
        decision_events: Vec::new(),
        uncaptured_synthetic_seeds: Vec::new(),
        failed: true,
        warnings: vec![format!("error:{error}")],
        seed_aggregate: None,
    }
}
