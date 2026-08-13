use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use crate::{EngineError, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fs;
use std::path::Path;

fn default_active_slot_count() -> usize {
    DECK_SIZE
}

/// Exact BattleBuffType names currently represented by ReplayHpMutationState.
/// Fixture validation and runtime adaptation share this list so unsupported
/// original state cannot be silently discarded.
pub const ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES: [&str; 5] = [
    "XiaHuiHeKaiShiQianBuZaiSunShiShengMing",
    "ShiYu",
    "HongZaoZong",
    "DanHuangZong",
    "XianDanHuangZong",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BattleFixture {
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(default)]
    pub source: Option<FixtureSource>,
    #[serde(rename = "firstPlayerSide")]
    pub first_player_side: PlayerSide,
    #[serde(rename = "decisionTape", default)]
    pub decision_tape: Vec<i64>,
    #[serde(rename = "randomFallbackTape", default)]
    pub random_fallback_tape: Vec<i64>,
    pub expected: FixtureExpected,
    #[serde(rename = "maxActorTurns", default)]
    pub max_actor_turns: Option<i64>,
    #[serde(rename = "historicalCardOverrides", default)]
    pub historical_card_overrides: Vec<HistoricalCardOverride>,
    #[serde(rename = "catalogCards", default)]
    pub catalog_cards: Vec<CardDefinition>,
    pub players: FixturePlayers,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct FixtureSource {
    pub round: Option<i64>,
    pub game_version: Option<String>,
    pub steam_build: Option<String>,
    pub recent_battle_file: Option<String>,
    pub season_mechanism: Option<i64>,
    pub eligible_for_base_rule_replay: Option<bool>,
    pub synthetic_decision_seed: Option<u32>,
    pub synthetic_decision_sides: Vec<PlayerSide>,
    pub synthetic_decision_fallback_seed: Option<u32>,
    /// Analysis-only perturbations applied after ordinary battle-start setup
    /// and before the first canonical observation checkpoint.
    pub solver_starting_perturbations: Vec<SolverStartingPerturbation>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverStartingPerturbation {
    pub side: PlayerSide,
    pub field: String,
    pub amount: i64,
}

pub const SOLVER_STARTING_PERTURBATION_FIELDS: [&str; 27] = [
    "hp",
    "maxHp",
    "defense",
    "guard",
    "anima",
    "momentum",
    "agility",
    "swordIntent",
    "sharpness",
    "attackBonus",
    "physique",
    "internalInjury",
    "weakness",
    "flaw",
    "attackReduction",
    "entangle",
    "externalInjury",
    "hexagram",
    "starPower",
    "cloudChain",
    "waterMomentum",
    "cloudSea",
    "activatedMetal",
    "activatedWater",
    "activatedWood",
    "activatedFire",
    "activatedEarth",
];

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct HistoricalCardOverride {
    pub side: PlayerSide,
    #[serde(rename = "slotIndex")]
    pub slot_index: usize,
    pub patch: HistoricalCardPatch,
    #[serde(default)]
    pub reason: String,
    #[serde(default)]
    pub evidence: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase", default)]
pub struct HistoricalCardPatch {
    pub attack: Option<i64>,
    pub anima: Option<i64>,
    #[serde(alias = "def")]
    pub defense: Option<i64>,
    pub damage: Option<i64>,
    #[serde(rename = "randomAttack")]
    pub random_attack: Option<i64>,
    #[serde(rename = "attackCount")]
    pub attack_count: Option<i64>,
    #[serde(rename = "hpCost")]
    pub hp_cost: Option<i64>,
    pub physique: Option<i64>,
    #[serde(rename = "otherParams")]
    pub other_params: Option<Vec<i64>>,
}

impl BattleFixture {
    pub fn validate(&self) -> Result<()> {
        if self.schema_version != 1 {
            return Err(EngineError::InvalidFixture(format!(
                "unsupported schemaVersion {}",
                self.schema_version
            )));
        }
        for (side, player) in [("p1", &self.players.p1), ("p2", &self.players.p2)] {
            if player.active_slot_count == 0 || player.active_slot_count > DECK_SIZE {
                return Err(EngineError::InvalidFixture(format!(
                    "{side} activeSlotCount must be between 1 and {DECK_SIZE}, got {}",
                    player.active_slot_count
                )));
            }
            if player.active_slot_count > player.cards.len() {
                return Err(EngineError::InvalidFixture(format!(
                    "{side} activeSlotCount {} exceeds cards length {}",
                    player.active_slot_count,
                    player.cards.len()
                )));
            }
            if player.cards.len() != DECK_SIZE {
                return Err(EngineError::InvalidFixture(format!(
                    "{side} has {} cards, expected {DECK_SIZE}",
                    player.cards.len()
                )));
            }
            if player.base_max_hp <= 0 {
                return Err(EngineError::InvalidFixture(format!(
                    "{side} baseMaxHp must be positive"
                )));
            }
            if let Some(battle_start_hp) = player.battle_start_hp {
                let max_hp = player.base_max_hp + player.extra_max_hp.unwrap_or(0);
                if battle_start_hp <= 0 || battle_start_hp > max_hp {
                    return Err(EngineError::InvalidFixture(format!(
                        "{side} battleStartHp must be in 1..={max_hp}"
                    )));
                }
            }
            let unsupported_buffs = player
                .initial_battle_buffs
                .keys()
                .filter(|name| !ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES.contains(&name.as_str()))
                .cloned()
                .collect::<Vec<_>>();
            if !unsupported_buffs.is_empty() {
                return Err(EngineError::InvalidFixture(format!(
                    "{side} initialBattleBuffs contains unsupported BuffType: {}; supported: {}",
                    unsupported_buffs.join(", "),
                    ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES.join(", ")
                )));
            }
        }
        if let Some(source) = &self.source {
            for perturbation in &source.solver_starting_perturbations {
                if !SOLVER_STARTING_PERTURBATION_FIELDS.contains(&perturbation.field.as_str()) {
                    return Err(EngineError::InvalidFixture(format!(
                        "unsupported solver starting perturbation field: {}",
                        perturbation.field
                    )));
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixtureExpected {
    #[serde(rename = "winnerSide")]
    pub winner_side: PlayerSide,
    #[serde(rename = "actorTurnCount")]
    pub actor_turn_count: i64,
    #[serde(rename = "hpDeltaP1MinusP2")]
    pub hp_delta_p1_minus_p2: i64,
    #[serde(rename = "finalHp", default)]
    pub final_hp: Option<FinalHp>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FinalHp {
    pub p1: i64,
    pub p2: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixturePlayers {
    pub p1: FixturePlayer,
    pub p2: FixturePlayer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FixturePlayer {
    pub level: i64,
    #[serde(rename = "baseMaxHp")]
    pub base_max_hp: i64,
    #[serde(rename = "extraMaxHp", default)]
    pub extra_max_hp: Option<i64>,
    /// Persistent current HP sampled for battle-start rule predicates. Combat
    /// temporary HP still follows the original max-HP initialization path.
    #[serde(rename = "battleStartHp", default)]
    pub battle_start_hp: Option<i64>,
    #[serde(rename = "characterId", default)]
    pub character_id: Option<i64>,
    #[serde(default)]
    pub talents: Vec<i64>,
    #[serde(rename = "talentResonanceId", default)]
    pub talent_resonance_id: Option<i64>,
    #[serde(rename = "fateStrategies", default)]
    pub fate_strategies: Vec<i64>,
    #[serde(rename = "fateStrategyTempDatas", default)]
    pub fate_strategy_temp_datas: BTreeMap<String, i64>,
    #[serde(rename = "activeSlotCount", default = "default_active_slot_count")]
    pub active_slot_count: usize,
    #[serde(rename = "initialDefense", default)]
    pub initial_defense: i64,
    #[serde(rename = "initialAnima", default)]
    pub initial_anima: i64,
    #[serde(rename = "initialGuard", default)]
    pub initial_guard: i64,
    #[serde(rename = "initialMomentum", default)]
    pub initial_momentum: i64,
    #[serde(rename = "initialMomentumLimit", default)]
    pub initial_momentum_limit: Option<i64>,
    #[serde(rename = "initialAgility", default)]
    pub initial_agility: i64,
    #[serde(rename = "initialBattleBuffs", default)]
    pub initial_battle_buffs: BTreeMap<String, i64>,
    #[serde(rename = "permanentBuffTempDatas", default)]
    pub permanent_buff_temp_datas: BTreeMap<String, i64>,
    #[serde(rename = "talentTempDatas", default)]
    pub talent_temp_datas: BTreeMap<String, i64>,
    #[serde(rename = "talentCardParams", default)]
    pub talent_card_params: BTreeMap<String, Vec<i64>>,
    #[serde(rename = "lastRoundUsedCardBaseIds", default)]
    pub last_round_used_card_base_ids: Vec<i64>,
    #[serde(rename = "lastRoundLife", default)]
    pub last_round_life: Option<i64>,
    #[serde(rename = "lastRoundExp", default)]
    pub last_round_exp: i64,
    #[serde(rename = "handCards", default)]
    pub hand_cards: Vec<i64>,
    #[serde(rename = "usedKeYinCards", default)]
    pub used_ke_yin_cards: Vec<i64>,
    pub cards: Vec<CardDefinition>,
}

pub fn apply_historical_card_patch(
    mut card: CardDefinition,
    patch: &HistoricalCardPatch,
) -> CardDefinition {
    if let Some(attack) = patch.attack {
        card.attack = Some(attack);
    }
    if let Some(anima) = patch.anima {
        card.anima = Some(anima);
    }
    if let Some(defense) = patch.defense {
        card.defense = Some(defense);
    }
    if let Some(damage) = patch.damage {
        card.damage = Some(damage);
    }
    if let Some(random_attack) = patch.random_attack {
        card.random_attack = Some(random_attack);
    }
    if let Some(attack_count) = patch.attack_count {
        card.attack_count = Some(attack_count);
    }
    if let Some(hp_cost) = patch.hp_cost {
        card.hp_cost = Some(hp_cost);
    }
    if let Some(physique) = patch.physique {
        card.physique = Some(physique);
    }
    if let Some(other_params) = &patch.other_params {
        card.other_params = other_params.clone();
    }
    card
}

pub fn load_fixture_file(path: impl AsRef<Path>) -> Result<BattleFixture> {
    let text = fs::read_to_string(path)?;
    let fixture: BattleFixture = serde_json::from_str(&text)?;
    fixture.validate()?;
    Ok(fixture)
}

/// Resolve a candidate fixture path by id (`<source>/<round>`), falling back to
/// the in-flight admission batch under `fixtures/incoming/<batchId>` when the
/// candidates tree lacks the file and `REPLAY_ADMISSION_INFLIGHT_BATCH_ID` is
/// set. Mirrors the TypeScript `candidateFixturePath` inflight fallback so the
/// Rust engine/tests can load not-yet-admitted fixtures during `freeze:replay
/// --full --inflight-batch <id>`; without the env the candidates path is returned
/// as before.
pub fn candidate_fixture_path(root: &Path, case_id: &str) -> std::path::PathBuf {
    let candidates_path = root
        .join("battle-evaluator/fixtures/candidates")
        .join(format!("{case_id}.json"));
    if candidates_path.exists() {
        return candidates_path;
    }
    if let Ok(batch_id) = std::env::var("REPLAY_ADMISSION_INFLIGHT_BATCH_ID") {
        let batch_id = batch_id.trim();
        if !batch_id.is_empty() {
            let inflight_path = root
                .join("battle-evaluator/fixtures/incoming")
                .join(batch_id)
                .join(format!("{case_id}.json"));
            if inflight_path.exists() {
                return inflight_path;
            }
        }
    }
    candidates_path
}
