use crate::{PlayerSide, ReplayEventKind, ReplayPlayerSnapshot, ReplayPreventionState, ReplayRun};
use serde::Serialize;
use std::collections::BTreeMap;

use super::value::{round_score, value_channels_for_side, ValueChannels};

pub const RULE_IMPACT_SCHEMA_VERSION: &str = "canonical-rule-impact-v1";
/// Field name and sign, in the exact order of `snapshot_features`.
///
/// The sign travels with the field instead of being a positional index range:
/// `NEGATIVE_FEATURE_FIELDS` in analysis/value/value-features.ts is keyed by name,
/// and a range would silently re-point at the wrong fields the first time this
/// array gains an entry.
const VALUE_FEATURE_FIELDS: [(&str, FeatureSign); 28] = [
    ("hp", FeatureSign::Own),
    ("maxHp", FeatureSign::Own),
    ("defense", FeatureSign::Own),
    ("guard", FeatureSign::Own),
    ("anima", FeatureSign::Own),
    ("swordIntent", FeatureSign::Own),
    ("momentum", FeatureSign::Own),
    ("agility", FeatureSign::Own),
    ("hexagram", FeatureSign::Own),
    ("starPower", FeatureSign::Own),
    ("attackBonus", FeatureSign::Own),
    ("physique", FeatureSign::Own),
    ("cloudChain", FeatureSign::Own),
    ("waterMomentum", FeatureSign::Own),
    ("sharpness", FeatureSign::Own),
    ("cloudSea", FeatureSign::Own),
    ("activatedMetal", FeatureSign::Own),
    ("activatedWater", FeatureSign::Own),
    ("activatedWood", FeatureSign::Own),
    ("activatedFire", FeatureSign::Own),
    ("activatedEarth", FeatureSign::Own),
    ("internalInjury", FeatureSign::Opponent),
    ("weakness", FeatureSign::Opponent),
    ("flaw", FeatureSign::Opponent),
    ("attackReduction", FeatureSign::Opponent),
    ("entangle", FeatureSign::Opponent),
    ("externalInjury", FeatureSign::Opponent),
    ("actionAgainCount", FeatureSign::Own),
];

/// Which side a higher raw value favours.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum FeatureSign {
    Own,
    Opponent,
}

const FEATURE_BUCKET_NAMES: [&str; 3] = ["early", "mid", "late"];

/// Checkpoint-level attribution for a canonical Rust replay.
///
/// A card-completed checkpoint includes every rule hook that resolved before
/// that checkpoint. It does not claim a finer per-buff causal source than the
/// replay protocol records.
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverRuleImpactReport {
    pub schema_version: &'static str,
    pub source: &'static str,
    pub value_profile: &'static str,
    pub side: PlayerSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub battle_seed: Option<u32>,
    pub start_value_for_side: f64,
    pub terminal_value_for_side: f64,
    pub terminal_delta_for_side: f64,
    pub attributed_delta_for_side: f64,
    /// Must stay zero: the contributions this report actually publishes sum back
    /// to the battleStart-to-end value change. Checked against
    /// `checkpoints[].contribution.total`, not against the internal consecutive
    /// deltas — telescoping those equals `terminal - start` by construction, so
    /// comparing them could never fail. See `published_delta_total`.
    pub audit_delta_for_side: f64,
    /// Canonical actor-turn trajectory and terminal features consumed by the
    /// λ calibration pipeline. Keys match analysis/value/value-features.ts.
    pub feature_sample_count: usize,
    pub features: BTreeMap<String, f64>,
    pub checkpoints: Vec<SolverRuleImpactCheckpoint>,
    pub cards: Vec<SolverRuleImpactCard>,
}

#[derive(Debug, Clone, Copy, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverRuleImpactContribution {
    pub hp: f64,
    pub defense: f64,
    pub guard: f64,
    pub resource: f64,
    pub debuff: f64,
    pub tempo: f64,
    pub total: f64,
    /// 这一段里护体取消掉的生命损失（点数，不是 value 分）。
    ///
    /// **不计入 `total`。** value-v0 比的是终局 `P1 HP - P2 HP`：挡掉的伤害已经体现在
    /// 双方实际生命里，再按 HP_WEIGHT 折一次就是双算。这一项只回答"这一步替我省了多少
    /// 生命"，让被护体吸收的大额伤害不再作为纯负的 `guard` 消耗出现。
    pub hp_loss_prevented_by_guard: f64,
    /// 同上，防御在伤害变成生命损失之前吸收掉的部分。
    pub hp_loss_prevented_by_defense: f64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverRuleImpactCheckpoint {
    pub checkpoint_index: usize,
    pub kind: ReplayEventKind,
    pub actor_turn: i64,
    pub actor: PlayerSide,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_action_index: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_id: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_slot: Option<usize>,
    pub contribution: SolverRuleImpactContribution,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverRuleImpactCard {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub card_id: Option<i64>,
    pub card_name: String,
    pub count: usize,
    pub contribution: SolverRuleImpactContribution,
}

pub(crate) fn compute_rule_impact(
    run: &ReplayRun,
    side: PlayerSide,
    battle_seed: Option<u32>,
) -> SolverRuleImpactReport {
    let Some(first) = run.events.first() else {
        return empty_report(side, battle_seed);
    };

    let start_channels = value_channels_for_side(&first.p1, &first.p2, side);
    let mut previous = start_channels;
    let mut previous_prevention = prevention_for_side(run, 0, side);
    let mut card_action_index = 0_usize;
    let mut checkpoints = Vec::with_capacity(run.events.len().saturating_sub(1));
    let mut cards = BTreeMap::<(Option<i64>, String), SolverRuleImpactCard>::new();

    for (event_index, event) in run.events.iter().enumerate().skip(1) {
        let current = value_channels_for_side(&event.p1, &event.p2, side);
        let delta = current.delta_from(previous);
        let prevention_now = prevention_for_side(run, event_index, side);
        let contribution = contribution_from_channels(
            delta,
            prevention_now.saturating_sub_state(previous_prevention),
        );
        let is_card = event.kind == ReplayEventKind::CardCompleted;
        if is_card {
            card_action_index += 1;
            let card_name = event
                .card_name
                .clone()
                .unwrap_or_else(|| "card:?".to_string());
            let aggregate = cards
                .entry((event.card_id, card_name.clone()))
                .or_insert_with(|| SolverRuleImpactCard {
                    card_id: event.card_id,
                    card_name,
                    count: 0,
                    contribution: SolverRuleImpactContribution::default(),
                });
            aggregate.count += 1;
            aggregate.contribution = add_contribution(aggregate.contribution, contribution);
        }
        checkpoints.push(SolverRuleImpactCheckpoint {
            checkpoint_index: event_index,
            kind: event.kind,
            actor_turn: event.turn,
            actor: event.actor,
            card_action_index: is_card.then_some(card_action_index),
            card_id: is_card.then_some(event.card_id).flatten(),
            card_name: is_card.then(|| event.card_name.clone()).flatten(),
            source_slot: is_card.then_some(event.slot).flatten(),
            contribution,
        });
        previous = current;
        previous_prevention = prevention_now;
    }

    let start_value = start_channels.total();
    let terminal_value = previous.total();
    let terminal_delta = terminal_value - start_value;
    let attributed_delta = published_delta_total(&checkpoints);
    let (feature_sample_count, features) = compute_feature_vector(run, side);
    let mut card_summaries = cards.into_values().collect::<Vec<_>>();
    card_summaries.sort_by(|left, right| {
        right
            .contribution
            .total
            .abs()
            .total_cmp(&left.contribution.total.abs())
            .then_with(|| left.card_id.cmp(&right.card_id))
            .then_with(|| left.card_name.cmp(&right.card_name))
    });

    SolverRuleImpactReport {
        schema_version: RULE_IMPACT_SCHEMA_VERSION,
        source: "rust-canonical-replay-checkpoints",
        value_profile: "value-v0-terminal",
        side,
        battle_seed,
        start_value_for_side: round_score(start_value),
        terminal_value_for_side: round_score(terminal_value),
        terminal_delta_for_side: round_score(terminal_delta),
        attributed_delta_for_side: round_score(attributed_delta),
        audit_delta_for_side: round_score(terminal_delta - attributed_delta),
        feature_sample_count,
        features,
        checkpoints,
        cards: card_summaries,
    }
}

/// Sums the contributions the report actually publishes.
///
/// The audit deliberately re-reads `checkpoints[].contribution.total` instead of
/// keeping a private running total of the same consecutive channel deltas: a sum
/// of consecutive differences telescopes to `terminal - start` no matter what
/// the report emits, so auditing against it can never fail. Reading the emitted
/// values back makes a dropped, duplicated or mis-rounded published checkpoint
/// observable in `audit_delta_for_side`.
fn published_delta_total(checkpoints: &[SolverRuleImpactCheckpoint]) -> f64 {
    checkpoints
        .iter()
        .map(|checkpoint| checkpoint.contribution.total)
        .sum()
}

/// A run without events publishes no checkpoints, so the audit is satisfied by
/// having nothing to reconstruct. This is the one case where a zero audit is not
/// evidence about attribution.
fn empty_report(side: PlayerSide, battle_seed: Option<u32>) -> SolverRuleImpactReport {
    SolverRuleImpactReport {
        schema_version: RULE_IMPACT_SCHEMA_VERSION,
        source: "rust-canonical-replay-checkpoints",
        value_profile: "value-v0-terminal",
        side,
        battle_seed,
        start_value_for_side: 0.0,
        terminal_value_for_side: 0.0,
        terminal_delta_for_side: 0.0,
        attributed_delta_for_side: 0.0,
        audit_delta_for_side: 0.0,
        feature_sample_count: 0,
        features: BTreeMap::new(),
        checkpoints: Vec::new(),
        cards: Vec::new(),
    }
}

fn compute_feature_vector(run: &ReplayRun, side: PlayerSide) -> (usize, BTreeMap<String, f64>) {
    let samples = run
        .events
        .iter()
        .filter(|event| event.kind == ReplayEventKind::TurnStart)
        .skip(1)
        .collect::<Vec<_>>();
    let mut features = BTreeMap::<String, f64>::new();
    let mut bucket_counts = [0_usize; 3];
    for (sample_index, event) in samples.iter().enumerate() {
        let bucket_index = feature_bucket(sample_index, samples.len());
        bucket_counts[bucket_index] += 1;
        for (field_index, (field, _)) in VALUE_FEATURE_FIELDS.iter().enumerate() {
            let key = format!("{}.{field}", FEATURE_BUCKET_NAMES[bucket_index]);
            *features.entry(key).or_default() +=
                feature_value(&event.p1, &event.p2, side, field_index);
        }
    }
    for (bucket_index, count) in bucket_counts.into_iter().enumerate() {
        if count == 0 {
            continue;
        }
        for (field, _) in VALUE_FEATURE_FIELDS {
            let key = format!("{}.{field}", FEATURE_BUCKET_NAMES[bucket_index]);
            if let Some(value) = features.get_mut(&key) {
                *value = round_score(*value / count as f64);
            }
        }
    }
    if let Some(terminal) = run.events.last() {
        for (field_index, (field, _)) in VALUE_FEATURE_FIELDS.iter().enumerate() {
            features.insert(
                format!("terminal.{field}"),
                round_score(feature_value(&terminal.p1, &terminal.p2, side, field_index)),
            );
        }
    }
    (samples.len(), features)
}

fn feature_bucket(index: usize, sample_count: usize) -> usize {
    let position = index + 1;
    if position * 3 <= sample_count {
        0
    } else if position * 3 <= sample_count * 2 {
        1
    } else {
        2
    }
}

fn feature_value(
    p1: &ReplayPlayerSnapshot,
    p2: &ReplayPlayerSnapshot,
    side: PlayerSide,
    field_index: usize,
) -> f64 {
    let (own, opponent) = match side {
        PlayerSide::P1 => (snapshot_features(p1), snapshot_features(p2)),
        PlayerSide::P2 => (snapshot_features(p2), snapshot_features(p1)),
    };
    match VALUE_FEATURE_FIELDS[field_index].1 {
        FeatureSign::Own => (own[field_index] - opponent[field_index]) as f64,
        FeatureSign::Opponent => (opponent[field_index] - own[field_index]) as f64,
    }
}

fn snapshot_features(snapshot: &ReplayPlayerSnapshot) -> [i64; 28] {
    [
        snapshot.hp,
        snapshot.max_hp,
        snapshot.defense,
        snapshot.guard,
        snapshot.anima,
        snapshot.sword_intent,
        snapshot.momentum,
        snapshot.agility,
        snapshot.hexagram,
        snapshot.star_power,
        snapshot.attack_bonus,
        snapshot.physique,
        snapshot.cloud_chain,
        snapshot.water_momentum,
        snapshot.sharpness,
        snapshot.cloud_sea,
        snapshot.activated_metal,
        snapshot.activated_water,
        snapshot.activated_wood,
        snapshot.activated_fire,
        snapshot.activated_earth,
        snapshot.internal_injury,
        snapshot.weakness,
        snapshot.flaw,
        snapshot.attack_reduction,
        snapshot.entangle,
        snapshot.external_injury,
        snapshot.action_again_count,
    ]
}

/// 护体/防御吸收量按被解释方视角取用：这是"我少掉了多少生命"。
fn prevention_for_side(
    run: &ReplayRun,
    event_index: usize,
    side: PlayerSide,
) -> ReplayPreventionState {
    let Some(pair) = run.prevention.get(event_index) else {
        return ReplayPreventionState::default();
    };
    match side {
        PlayerSide::P1 => pair.p1,
        PlayerSide::P2 => pair.p2,
    }
}

fn contribution_from_channels(
    channels: ValueChannels,
    prevented: ReplayPreventionState,
) -> SolverRuleImpactContribution {
    SolverRuleImpactContribution {
        hp: round_score(channels.hp),
        defense: round_score(channels.defense),
        guard: round_score(channels.guard),
        resource: round_score(channels.resources),
        debuff: round_score(channels.debuff),
        tempo: round_score(channels.tempo),
        total: round_score(channels.total()),
        hp_loss_prevented_by_guard: prevented.hp_loss_prevented_by_guard as f64,
        hp_loss_prevented_by_defense: prevented.hp_loss_prevented_by_defense as f64,
    }
}

fn add_contribution(
    left: SolverRuleImpactContribution,
    right: SolverRuleImpactContribution,
) -> SolverRuleImpactContribution {
    SolverRuleImpactContribution {
        hp: round_score(left.hp + right.hp),
        defense: round_score(left.defense + right.defense),
        guard: round_score(left.guard + right.guard),
        resource: round_score(left.resource + right.resource),
        debuff: round_score(left.debuff + right.debuff),
        tempo: round_score(left.tempo + right.tempo),
        total: round_score(left.total + right.total),
        hp_loss_prevented_by_guard: left.hp_loss_prevented_by_guard
            + right.hp_loss_prevented_by_guard,
        hp_loss_prevented_by_defense: left.hp_loss_prevented_by_defense
            + right.hp_loss_prevented_by_defense,
    }
}
