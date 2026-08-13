use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide, DECK_SIZE};

fn basic_attack() -> CardDefinition {
    CardDefinition {
        id: 0,
        base_id: Some(0),
        name: "普通攻击".to_string(),
        card_type: None,
        attack: Some(3),
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
    }
}

fn test_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    CardDefinition {
        id,
        base_id: Some(base_id),
        name: name.to_string(),
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
    }
}

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![first];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 1,
        base_max_hp: 30,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 1,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards,
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
