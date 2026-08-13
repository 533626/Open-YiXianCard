use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn basic_attack() -> CardDefinition {
    original_card_definition_by_id(0).expect("missing basic attack")
}

fn deck() -> Vec<CardDefinition> {
    let mut cards = vec![basic_attack()];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn player() -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 50,
        extra_max_hp: Some(0),
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: Some(6),
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: BTreeMap::new(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: BTreeMap::new(),
        talent_card_params: BTreeMap::new(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards: deck(),
    }
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
        max_actor_turns: Some(1),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers { p1, p2 },
    }
}

#[test]
fn talent_183_start_physique_counts_as_battle_physique_gain() {
    let mut p1 = player();
    p1.talents = vec![183, 184];
    p1.fate_strategies = vec![166];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 4);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.physique, 6);
    assert_eq!(state.p1.core.hp, 51);
    assert_eq!(state.p1.core.defense, 2);
    assert_eq!(state.p1.turn.battle_physique_gain_count, 2);
}

#[test]
fn talent_183_opening_heals_only_new_physique_overflow() {
    let mut p1 = player();
    p1.talents = vec![183];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 5);
    p1.permanent_buff_temp_datas.insert("10024".to_string(), 5);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.physique, 6);
    assert_eq!(state.p1.core.max_hp, 56);
    assert_eq!(state.p1.core.hp, 51);
}

#[test]
fn robust_bones_variants_stack_start_hp_healing() {
    let mut p1 = player();
    p1.talents = vec![10_176, 20_176];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 35);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.max_hp, 85);
    assert_eq!(state.p1.core.hp, 58);
}

#[test]
fn permanent_power_loss_grass_initializes_attack_reduction() {
    let mut p1 = player();
    p1.permanent_buff_temp_datas.insert("10018".to_string(), 2);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.status.attack_reduction, 0);
    assert_eq!(state.p2.status.attack_reduction, 2);
}
