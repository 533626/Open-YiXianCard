use crate::model::PlayerSide;

/// Observation-only receipt for one stable phase inside original OnTurnEnded.
/// Kept separate from ReplayDetailedStep so existing hook/event indices remain
/// unchanged while tail mismatches can identify the exact shared hook.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTurnEndHookReceipt {
    pub turn: i64,
    pub actor: PlayerSide,
    pub hook: &'static str,
    pub before: ReplayTurnEndHookPair,
    pub after: ReplayTurnEndHookPair,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTurnEndHookPair {
    pub p1: ReplayTurnEndHookSnapshot,
    pub p2: ReplayTurnEndHookSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayTurnEndHookSnapshot {
    pub hp: i64,
    pub max_hp: i64,
    pub defense: i64,
    pub anima: i64,
    pub guard: i64,
    pub physique: i64,
    pub momentum: i64,
    pub water_momentum: i64,
    pub attack_bonus: i64,
    pub internal_injury: i64,
    pub weakness: i64,
    pub flaw: i64,
    pub attack_reduction: i64,
    pub entangle: i64,
    pub external_injury: i64,
    pub lose_hp_count: i64,
    pub lose_hp_times_count: i64,
}
