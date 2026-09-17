use super::cards_dream_mirage::DreamMirageValue;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 100, None, active_slot_count, None)
}

fn fixture() -> BattleFixture {
    let mut p1_cards = vec![
        original_card(7_020_077),
        original_card(7_000_018),
        original_card(7_000_021),
    ];
    p1_cards.resize_with(DECK_SIZE, || original_card(0));
    let mut p2_cards = vec![original_card(0)];
    p2_cards.resize_with(DECK_SIZE, || original_card(0));
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
        players: FixturePlayers {
            p1: player(p1_cards, 3),
            p2: player(p2_cards, 1),
        },
    }
}

#[test]
fn dream_five_elements_spike_low_realm_attacks_then_counts_deck_elements_for_defense() {
    let mut state = ReplayState::test_from_fixture(&fixture());

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 94);
    assert_eq!(state.p1.core.defense, 6);
}

#[test]
fn dream_five_elements_marrow_uses_the_printed_finite_count() {
    let mut p1_cards = vec![
        original_card(7_030_087),
        original_card(7_000_023),
        original_card(7_010_011),
        original_card(7_010_011),
    ];
    p1_cards.resize_with(DECK_SIZE, || original_card(0));
    let mut p2_cards = vec![original_card(0)];
    p2_cards.resize_with(DECK_SIZE, || original_card(0));
    let mut battle = fixture();
    battle.max_actor_turns = None;
    battle.players.p1.active_slot_count = 4;
    battle.players.p1.cards = p1_cards;
    battle.players.p2.cards = p2_cards;

    let mut state = ReplayState::test_from_fixture(&battle);
    assert!(!state.test_execute_one_card(PlayerSide::P1));

    // Card_7000087 stores otherParams[1] (three for 7030087), then the
    // original consumes one charge per successful spirit seal/formation.
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::FiveElementsMarrow),
        3
    );
    let spirit_seal = original_card(7_010_011);
    for remaining in [2, 1, 0] {
        state.p1.turn.agility = 0;
        state.apply_dream_mirage_before_card_hooks(PlayerSide::P1, &spirit_seal, 0);
        assert_eq!(state.p1.turn.agility, 10);
        assert_eq!(
            state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::FiveElementsMarrow),
            remaining
        );
    }
    state.p1.turn.agility = 0;
    state.apply_dream_mirage_before_card_hooks(PlayerSide::P1, &spirit_seal, 0);
    assert_eq!(state.p1.turn.agility, 0);

    let mut infinite_battle = fixture();
    infinite_battle.players.p1.active_slot_count = 1;
    infinite_battle.players.p1.cards[0] = original_card(7_040_087);
    let mut infinite = ReplayState::test_from_fixture(&infinite_battle);
    assert!(!infinite.test_execute_one_card(PlayerSide::P1));
    for _ in 0..4 {
        infinite.p1.turn.agility = 0;
        infinite.apply_dream_mirage_before_card_hooks(PlayerSide::P1, &spirit_seal, 0);
        assert_eq!(infinite.p1.turn.agility, 10);
    }
}
