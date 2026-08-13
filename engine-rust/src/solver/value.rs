use crate::{PlayerSide, ReplayEventKind, ReplayPlayerSnapshot, ReplayRun};
use serde::{Deserialize, Serialize};

// The weight table is generated, not hand-written: `value_weights.rs` comes from
// `analysis/generated/value-weights-v1.json` via `bun run report:value-weights`,
// and `analysis/value/value-weights.generated.ts` is the same artifact rendered for
// TypeScript, so the two languages cannot drift apart one-sidedly. Repricing means
// re-running the training pipeline, not editing a constant.
use super::value_weights::{
    ACTION_AGAIN_WEIGHT, ACTIVATED_ELEMENT_WEIGHT, AGILITY_WEIGHT, ANIMA_WEIGHT, AREA_WEIGHT,
    ATTACK_BONUS_WEIGHT, ATTACK_REDUCTION_WEIGHT, CLOUD_CHAIN_WEIGHT,
    CLOUD_SEA_CHAIN_RESERVE_WEIGHT, DEFENSE_WEIGHT, ENTANGLE_WEIGHT, EXTERNAL_INJURY_WEIGHT,
    FLAW_WEIGHT, GUARD_WEIGHT, HEXAGRAM_WEIGHT, HP_WEIGHT, INTERNAL_INJURY_WEIGHT, MAX_HP_WEIGHT,
    MOMENTUM_WEIGHT, PHYSIQUE_WEIGHT, SHARPNESS_WEIGHT, STAR_POWER_WEIGHT, SWORD_INTENT_WEIGHT,
    TERMINAL_RESOURCE_DISCOUNT, WATER_MOMENTUM_WEIGHT, WEAKNESS_WEIGHT,
};

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Deserialize, Serialize)]
pub enum ScoreProfile {
    #[serde(rename = "hpDelta")]
    #[default]
    HpDelta,
    #[serde(rename = "value-v0")]
    ValueV0,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverValueMetrics {
    pub terminal_value_for_side: f64,
    pub terminal_hp_for_side: f64,
    pub terminal_shield_for_side: f64,
    pub terminal_defense_for_side: f64,
    pub terminal_guard_for_side: f64,
    pub terminal_resource_for_side: f64,
    pub terminal_debuff_for_side: f64,
    pub terminal_tempo_for_side: f64,
    /// Terminal action-again count difference for the scored side, unweighted.
    /// Keeps the tempo dimension identifiable for the stage-two ranking search
    /// even after `ACTION_AGAIN_WEIGHT` trains to zero.
    pub terminal_tempo_count_for_side: f64,
    pub area_score_for_side: f64,
    pub hp_area_for_side: f64,
    pub resource_area_for_side: f64,
    pub debuff_area_for_side: f64,
    pub hp_area_score_for_side: f64,
    pub resource_area_score_for_side: f64,
    pub debuff_area_score_for_side: f64,
    pub area_sample_count: f64,
    pub audit_mismatch_fields: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct ValueChannels {
    pub hp: f64,
    pub defense: f64,
    pub guard: f64,
    pub resources: f64,
    pub debuff: f64,
    pub tempo: f64,
}

impl ValueChannels {
    pub(crate) fn total(self) -> f64 {
        self.hp + self.defense + self.guard + self.resources + self.debuff + self.tempo
    }

    pub(crate) fn delta_from(self, before: Self) -> Self {
        Self {
            hp: self.hp - before.hp,
            defense: self.defense - before.defense,
            guard: self.guard - before.guard,
            resources: self.resources - before.resources,
            debuff: self.debuff - before.debuff,
            tempo: self.tempo - before.tempo,
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ValueAreaChannels {
    hp: f64,
    resources: f64,
    debuff: f64,
}

pub fn compute_value_metrics(run: &ReplayRun, side: PlayerSide) -> SolverValueMetrics {
    let side_sign = side_sign(side);
    let mut hp_area = 0.0;
    let mut resource_area = 0.0;
    let mut debuff_area = 0.0;
    let mut area_sample_count = 0_usize;
    let mut saw_actor_turn = false;

    for event in &run.events {
        if event.kind != ReplayEventKind::TurnStart {
            continue;
        }
        if saw_actor_turn {
            let area = value_area_contribution(&event.p1, &event.p2);
            hp_area += area.hp;
            resource_area += area.resources;
            debuff_area += area.debuff;
            area_sample_count += 1;
        }
        saw_actor_turn = true;
    }

    let (final_p1, final_p2) = final_snapshots(run);
    if saw_actor_turn {
        let area = value_area_contribution(final_p1, final_p2);
        hp_area += area.hp;
        resource_area += area.resources;
        debuff_area += area.debuff;
        area_sample_count += 1;
    }

    let terminal = value_channels_for_side(final_p1, final_p2, side);
    // Non-HP channels are discounted at the terminal state: the perturbation-trained
    // prices are decision-state marginal values, and a resource you can no longer
    // spend is not worth its opening price. The per-channel fields below stay
    // undiscounted so the discount itself remains identifiable when it trains to
    // zero, which means `terminal_value_for_side` is deliberately not the sum of the
    // reported channels.
    let terminal_value_for_side = terminal.hp
        + TERMINAL_RESOURCE_DISCOUNT
            * (terminal.defense + terminal.guard + terminal.resources + terminal.debuff)
        + terminal.tempo;

    let hp_area_mean = mean_or_zero(hp_area, area_sample_count);
    let resource_area_mean = mean_or_zero(resource_area, area_sample_count);
    let debuff_area_mean = mean_or_zero(debuff_area, area_sample_count);
    let hp_area_for_side = side_sign * hp_area_mean / 10.0;
    let resource_area_for_side = side_sign * resource_area_mean / 10.0;
    let debuff_area_for_side = side_sign * debuff_area_mean / 10.0;
    let hp_area_score_for_side = hp_area_for_side * AREA_WEIGHT;
    let resource_area_score_for_side = resource_area_for_side * AREA_WEIGHT;
    let debuff_area_score_for_side = debuff_area_for_side * AREA_WEIGHT;
    let area_score_for_side =
        hp_area_score_for_side + resource_area_score_for_side + debuff_area_score_for_side;

    SolverValueMetrics {
        terminal_value_for_side: round_score(terminal_value_for_side),
        terminal_hp_for_side: round_score(terminal.hp),
        terminal_shield_for_side: round_score(terminal.defense + terminal.guard),
        terminal_defense_for_side: round_score(terminal.defense),
        terminal_guard_for_side: round_score(terminal.guard),
        terminal_resource_for_side: round_score(terminal.resources),
        terminal_debuff_for_side: round_score(terminal.debuff),
        terminal_tempo_for_side: round_score(terminal.tempo),
        terminal_tempo_count_for_side: side_sign
            * (final_p1.action_again_count - final_p2.action_again_count) as f64,
        area_score_for_side: round_score(area_score_for_side),
        hp_area_for_side: round_score(hp_area_for_side),
        resource_area_for_side: round_score(resource_area_for_side),
        debuff_area_for_side: round_score(debuff_area_for_side),
        hp_area_score_for_side: round_score(hp_area_score_for_side),
        resource_area_score_for_side: round_score(resource_area_score_for_side),
        debuff_area_score_for_side: round_score(debuff_area_score_for_side),
        area_sample_count: area_sample_count as f64,
        audit_mismatch_fields: Vec::new(),
    }
}

pub fn value_score(metrics: &SolverValueMetrics) -> f64 {
    round_score(metrics.terminal_value_for_side + metrics.area_score_for_side)
}

fn final_snapshots(run: &ReplayRun) -> (&ReplayPlayerSnapshot, &ReplayPlayerSnapshot) {
    let final_event = run
        .events
        .last()
        .expect("replay run must contain at least battleEnd event");
    (&final_event.p1, &final_event.p2)
}

pub(crate) fn value_channels_for_side(
    p1: &ReplayPlayerSnapshot,
    p2: &ReplayPlayerSnapshot,
    side: PlayerSide,
) -> ValueChannels {
    let sign = side_sign(side);
    let left = player_value_channels(p1);
    let right = player_value_channels(p2);
    ValueChannels {
        hp: sign * (left.hp - right.hp) / 10.0,
        defense: sign * (left.defense - right.defense) / 10.0,
        guard: sign * (left.guard - right.guard) / 10.0,
        resources: sign * (left.resources - right.resources) / 10.0,
        debuff: sign * (left.debuff - right.debuff) / 10.0,
        tempo: sign * (left.tempo - right.tempo) / 10.0,
    }
}

/// Area channels stay on the same scale as the matching terminal channels: each
/// one is a weighted difference that the caller divides by 10. The hp channel
/// must apply `HP_WEIGHT` explicitly even though the caller's `/ 10.0` cancels it
/// today; leaving the weight implicit would silently desync the area term from
/// the terminal term the moment `HP_WEIGHT` is retuned.
fn value_area_contribution(
    p1: &ReplayPlayerSnapshot,
    p2: &ReplayPlayerSnapshot,
) -> ValueAreaChannels {
    ValueAreaChannels {
        hp: (p1.hp - p2.hp) as f64 * HP_WEIGHT,
        resources: resource_value(p1) - resource_value(p2),
        debuff: debuff_penalty(p2) - debuff_penalty(p1),
    }
}

fn player_value_channels(snapshot: &ReplayPlayerSnapshot) -> ValueChannels {
    ValueChannels {
        hp: snapshot.hp as f64 * HP_WEIGHT,
        defense: snapshot.defense as f64 * DEFENSE_WEIGHT,
        guard: snapshot.guard as f64 * GUARD_WEIGHT,
        resources: resource_value(snapshot),
        debuff: -debuff_penalty(snapshot),
        tempo: snapshot.action_again_count as f64 * ACTION_AGAIN_WEIGHT,
    }
}

fn resource_value(snapshot: &ReplayPlayerSnapshot) -> f64 {
    snapshot.max_hp as f64 * MAX_HP_WEIGHT
        + snapshot.anima as f64 * ANIMA_WEIGHT
        + snapshot.sword_intent as f64 * SWORD_INTENT_WEIGHT
        + snapshot.momentum as f64 * MOMENTUM_WEIGHT
        + snapshot.agility as f64 * AGILITY_WEIGHT
        + snapshot.hexagram as f64 * HEXAGRAM_WEIGHT
        + snapshot.star_power as f64 * STAR_POWER_WEIGHT
        + snapshot.attack_bonus as f64 * ATTACK_BONUS_WEIGHT
        + snapshot.physique as f64 * PHYSIQUE_WEIGHT
        + snapshot.cloud_chain as f64 * CLOUD_CHAIN_WEIGHT
        + snapshot.water_momentum as f64 * WATER_MOMENTUM_WEIGHT
        + snapshot.sharpness as f64 * SHARPNESS_WEIGHT
        + cloud_sea_value(snapshot)
        + activated_element_value(snapshot)
}

fn cloud_sea_value(snapshot: &ReplayPlayerSnapshot) -> f64 {
    if snapshot.cloud_chain <= 0 {
        return 0.0;
    }
    snapshot.cloud_sea as f64 * CLOUD_SEA_CHAIN_RESERVE_WEIGHT
}

fn activated_element_value(snapshot: &ReplayPlayerSnapshot) -> f64 {
    (snapshot.activated_metal.min(1)
        + snapshot.activated_water.min(1)
        + snapshot.activated_wood.min(1)
        + snapshot.activated_fire.min(1)
        + snapshot.activated_earth.min(1)) as f64
        * ACTIVATED_ELEMENT_WEIGHT
}

fn debuff_penalty(snapshot: &ReplayPlayerSnapshot) -> f64 {
    snapshot.internal_injury as f64 * INTERNAL_INJURY_WEIGHT
        + snapshot.weakness as f64 * WEAKNESS_WEIGHT
        + snapshot.flaw as f64 * FLAW_WEIGHT
        + snapshot.attack_reduction as f64 * ATTACK_REDUCTION_WEIGHT
        + snapshot.entangle as f64 * ENTANGLE_WEIGHT
        + snapshot.external_injury as f64 * EXTERNAL_INJURY_WEIGHT
}

fn side_sign(side: PlayerSide) -> f64 {
    match side {
        PlayerSide::P1 => 1.0,
        PlayerSide::P2 => -1.0,
    }
}

fn mean_or_zero(total: f64, count: usize) -> f64 {
    if count == 0 {
        0.0
    } else {
        total / count as f64
    }
}

pub(crate) fn round_score(value: f64) -> f64 {
    let scaled = value * 10.0;
    // TS 参考实现用 JS Math.round：.5 一律向 +∞ 取整，而 f64::round 远离零取整，
    // 负半值（如 -4.5）会差 0.1。
    let rounded_scaled = if scaled.fract() == -0.5 {
        scaled.trunc()
    } else {
        scaled.round()
    };
    let rounded = rounded_scaled / 10.0;
    if rounded == 0.0 {
        0.0
    } else {
        rounded
    }
}
