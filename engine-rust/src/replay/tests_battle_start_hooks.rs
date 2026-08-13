use super::*;

#[test]
fn fate_strategy_battle_start_hooks_apply_through_fixture_startup() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.fate_strategies = vec![89, 138, 143, 146];

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(state.p1.sword.cloud_chain, 1);
    assert_eq!(state.p1.sword.cloud_sea, 2);
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Wood));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Earth));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Fire));
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.core.defense, 4);
    assert_eq!(state.p2.core.hp, 28);
    assert_eq!(state.p2.core.max_hp, 28);
}

#[test]
fn talent_199_activates_bottle_element_for_wood_spirit_revival() {
    let revival = CardDefinition {
        id: 7_000_018,
        base_id: Some(7_000_018),
        name: "木灵•复苏".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(2),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![3, 1],
    };
    let mut fixture = minimal_fixture(
        filler_cards(crate::replay::support::basic_attack_card()),
        filler_cards(revival.clone()),
        FixtureExpected {
            winner_side: PlayerSide::P2,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.first_player_side = PlayerSide::P2;
    fixture.players.p2.talents = vec![199];
    fixture
        .players
        .p2
        .talent_card_params
        .insert("199".to_string(), vec![7_000_018]);
    fixture.players.p2.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    assert!(state
        .p2
        .elements
        .activated_elements
        .contains(&super::super::Element::Wood));
    state.p2.core.hp = 92;
    state.p2.core.max_hp = 94;
    state.p2.core.anima = 1;
    state.p2.deck.queue = vec![0];
    state.current_actor = PlayerSide::P2;
    let _ = state.test_execute_one_card(PlayerSide::P2);
    assert_eq!(state.p2.core.hp, 94);
}

#[test]
fn talent_199_resolves_out_of_deck_card_names_without_effect_dispatch() {
    for (stored_card_id, expected_element) in [
        (7_000_094, None),
        (7_000_097, Some(super::super::Element::Wood)),
    ] {
        let mut fixture = minimal_fixture(
            filler_cards(basic_attack_test_card()),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        fixture.players.p1.talents = vec![199];
        fixture
            .players
            .p1
            .talent_card_params
            .insert("199".to_string(), vec![stored_card_id]);

        let state = ReplayState::test_from_fixture(&fixture);

        assert_eq!(
            state.p1.elements.activated_elements.first().copied(),
            expected_element,
            "stored card {stored_card_id} should resolve by original config name"
        );
    }
}

#[test]
fn wood_spirit_all_growth_grants_hp_and_turn_start_attack_bonus() {
    let card = CardDefinition {
        id: 10_134,
        base_id: Some(134),
        name: "木灵•万物生".to_string(),
        card_type: Some(crate::model::OriginalEnumValue {
            value: 3,
            name: "Sustain".to_string(),
        }),
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(-1),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![9, 1],
    };
    let mut fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(crate::replay::support::basic_attack_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.anima = 2;
    state.p1.deck.queue = vec![0];
    state.current_actor = PlayerSide::P1;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 39);
    assert_eq!(state.p1.core.max_hp, 39);
    assert_eq!(state.p1.elements.wood_spirit_all_growth, 1);
    assert_eq!(state.p1.elements.wood_spirit_all_growth_attack, 1);
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.attack_bonus, 1);
}
