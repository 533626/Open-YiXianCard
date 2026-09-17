use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::CardDefinition;

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck() -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![basic_attack()], basic_attack())
}

fn player() -> FixturePlayer {
    super::test_support::make_player(deck(), 5, 50, Some(0), 8, Some(6))
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
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
