use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide};

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn test_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    super::test_support::test_card(id, base_id, name)
}

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![first], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 1, 30, None, 1, None)
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
}

#[test]
fn upgraded_abundant_momentum_grants_opening_momentum_and_limit() {
    let mut p1 = player(deck_with(basic_attack()));
    p1.talents = vec![30_145]; // 气势充沛

    let state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));

    assert_eq!(state.p1.beng.momentum, 1);
    assert_eq!(state.p1.beng.momentum_limit, 9);
}

#[test]
fn deity_rear_move_response_gains_five_defense_hp_and_max_hp_on_first_check() {
    let mut flying_tread = test_card(12, 12, "飞鸿踏雪");
    flying_tread.anima = Some(3);
    flying_tread.other_params = vec![0];
    let mut p1 = player(deck_with(flying_tread));
    p1.talents = vec![30_071]; // 后发制人

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 5);
    assert_eq!(state.p1.core.max_hp, 35);
    assert_eq!(state.p1.core.hp, 35);
    assert_eq!(state.p1.core.anima, 3);
}

#[test]
fn rear_move_response_stacks_each_present_talent_rank() {
    let mut flying_tread = test_card(12, 12, "飞鸿踏雪");
    flying_tread.anima = Some(3);
    flying_tread.other_params = vec![0];
    let mut p1 = player(deck_with(flying_tread));
    p1.talents = vec![64, 20_071, 30_071];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 11);
    assert_eq!(state.p1.core.max_hp, 39);
    assert_eq!(state.p1.core.hp, 39);
}

#[test]
fn devouring_ancient_vine_drains_hp_and_sets_action_again_drain() {
    let mut devouring_vine = test_card(9_020_019, 9_000_019, "噬仙古藤");
    devouring_vine.card_type = Some(OriginalEnumValue {
        value: 3,
        name: "Sustain".to_string(),
    });
    devouring_vine.anima = Some(-1);
    devouring_vine.other_params = vec![10, 6];
    let mut p1 = player(deck_with(devouring_vine));
    p1.initial_anima = 1;

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    state.p1.core.hp = 20;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.music.devouring_ancient_vine, 6);
    assert_eq!(state.p2.core.hp, 20);
}

#[test]
fn deity_regenerative_body_gains_two_physique_and_heals_six_on_first_slot() {
    let mut p1 = player(deck_with(basic_attack()));
    p1.base_max_hp = 20;
    p1.talents = vec![30_149]; // 再生之躯

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    state.p1.core.hp = 10;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.max_hp, 22);
    assert_eq!(state.p1.core.hp, 16);
    assert_eq!(state.p2.core.hp, 27);
}

#[test]
fn cost_hp_fate_strategies_gain_physique_and_once_per_turn_anima() {
    // Card_10000074 / 梦·崩拳突 is a current-build executable hp-cost card.
    // Keep this contract on a catalog-backed card so fail-closed admission is
    // exercised before the cost and FateStrategy hooks run.
    let hot_blood_cost =
        original_card_definition_by_id(10_000_074).expect("missing current-build 梦·崩拳突");
    let mut p1 = player(deck_with(hot_blood_cost));
    p1.base_max_hp = 40;
    p1.fate_strategies = vec![149, 347]; // 魂体不竭 / 热血化气

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 36);
    assert_eq!(state.p1.core.max_hp, 41);
    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.core.anima, 1);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 32);
    assert_eq!(state.p1.core.max_hp, 42);
    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.anima, 1);
}
