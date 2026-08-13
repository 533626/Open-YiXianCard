use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

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

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![first];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
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
fn star_chess_twin_swallows_uses_star_slot_and_rear_move_bonus() {
    let card = CardDefinition {
        id: 20_053,
        base_id: Some(53),
        name: "星弈·双飞燕".to_string(),
        card_type: None,
        attack: Some(6),
        random_attack: None,
        random_defense: None,
        attack_count: Some(1),
        defense: None,
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: Some(true),
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![8],
    };
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(card)),
        player(deck_with(basic_attack())),
    ));
    state.p1.astrology.star_slots = vec![0];
    state.p1.deck.slots[0].used = true;

    assert!(state.test_execute_one_card(PlayerSide::P1));
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_hp, 22);
    assert_eq!(snapshot.action_again_count, 1);
}

#[test]
fn fire_hexagram_lowers_current_hp_before_max_hp() {
    let card = CardDefinition {
        id: 4_000_034,
        base_id: Some(4_000_034),
        name: "离卦".to_string(),
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
        hexagram: Some(3),
        rarity: None,
        career_name: None,
        other_params: vec![3],
    };
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(card)),
        player(deck_with(basic_attack())),
    ));

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.astrology.hexagram, 3);
    assert_eq!(state.p2.core.hp, 47);
    assert_eq!(state.p2.core.max_hp, 47);
    assert_eq!(state.p2.turn.lose_hp_count, 3);
    assert_eq!(state.p2.turn.lose_hp_times_count, 1);
}
