use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![first], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6))
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
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
