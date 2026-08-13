use super::{original_build_profile, original_config};
use crate::fixture::{
    BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide};
use crate::{EngineError, Result};

/// Deterministic synthetic fixture for engine/solver contract tests.
///
/// This is deliberately code-owned and is not replay evidence or a member of
/// the certified corpus, so an intentionally empty corpus does not disable the
/// Rust self-test suite.
pub fn engine_contract_fixture() -> Result<BattleFixture> {
    let cards = |ids: &[i64]| -> Result<Vec<CardDefinition>> {
        ids.iter()
            .map(|id| {
                original_config::original_card_definition(*id).ok_or_else(|| {
                    EngineError::InvalidFixture(format!(
                        "engine contract fixture card is absent from original config: {id}",
                    ))
                })
            })
            .collect()
    };
    let player = |cards: Vec<CardDefinition>| FixturePlayer {
        level: 5,
        base_max_hp: 80,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        talent_resonance_id: None,
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        used_ke_yin_cards: Vec::new(),
        cards,
    };
    let p1 = cards(&[
        1_000_010, 1_000_005, 1_000_012, 1_000_004, 1_000_002, 1_000_003, 1_000_001, 1_000_009,
    ])?;
    let p2 = cards(&[
        1_000_021, 1_000_007, 1_000_006, 1_000_008, 1_000_013, 1_000_017, 1_000_018, 1_000_019,
    ])?;
    let fixture = BattleFixture {
        schema_version: 1,
        source: Some(FixtureSource {
            steam_build: Some(original_build_profile::project_target_steam_build().to_string()),
            ..FixtureSource::default()
        }),
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
        max_actor_turns: None,
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers {
            p1: player(p1),
            p2: player(p2),
        },
    };
    fixture.validate()?;
    Ok(fixture)
}
