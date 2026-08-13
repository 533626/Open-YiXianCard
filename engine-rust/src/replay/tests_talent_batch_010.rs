use super::*;
use crate::fixture::{
    BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(0)
}

fn deck_with(mut cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: Some(0),
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
        cards,
    }
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: Some(FixtureSource {
            steam_build: Some(
                super::original_build_profile::project_target_steam_build().to_string(),
            ),
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
        max_actor_turns: Some(1),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers { p1, p2 },
    }
}

fn state_with_talents(talents: Vec<i64>) -> ReplayState {
    let mut p1 = player(deck_with(vec![basic_attack()]));
    p1.talents = talents;
    ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))))
}

#[test]
fn talents_3_26_and_33_apply_exact_opening_resources_and_zero_based_star_slot() {
    let mut p1 = player(deck_with(vec![basic_attack()]));
    p1.talents = vec![3, 26, 33, 106];
    p1.initial_defense = 4;
    p1.initial_guard = 2;
    let active =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(active.p1.core.defense, 12);
    assert_eq!(active.p1.core.guard, 3);
    assert_eq!(active.p1.astrology.star_slots, vec![2, 5, 6]);
    assert_eq!(
        active
            .p1
            .astrology
            .star_slots
            .iter()
            .filter(|slot| **slot == 6)
            .count(),
        1
    );

    let control = state_with_talents(Vec::new());
    assert_eq!(control.p1.core.defense, 0);
    assert_eq!(control.p1.core.guard, 0);
    assert_eq!(control.p1.astrology.star_slots, vec![2, 5]);
}

#[test]
fn talent_36_rewards_each_positive_hexagram_gain_call_not_each_point() {
    let mut active = state_with_talents(vec![36]);
    active.gain_hexagram(PlayerSide::P1, 1);
    active.gain_hexagram(PlayerSide::P1, 3);
    assert_eq!(active.p1.astrology.hexagram, 4);
    assert_eq!(active.p1.core.anima, 4);

    active.gain_hexagram(PlayerSide::P1, 0);
    active.gain_hexagram(PlayerSide::P1, -2);
    assert_eq!(active.p1.astrology.hexagram, 4);
    assert_eq!(active.p1.core.anima, 4);

    let mut control = state_with_talents(Vec::new());
    control.gain_hexagram(PlayerSide::P1, 3);
    assert_eq!(control.p1.astrology.hexagram, 3);
    assert_eq!(control.p1.core.anima, 0);
}

#[test]
fn talent_60_rows_apply_their_exact_first_slot_max_hp_gain() {
    for (talent, expected_gain) in [(60, 10), (10_060, 10), (20_060, 15), (30_060, 20)] {
        let mut state = state_with_talents(vec![talent]);
        let card = state.test_actor_card(PlayerSide::P1, 0);
        state.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
        assert_eq!(state.p1.core.max_hp, 100 + expected_gain, "talent {talent}");
        assert_eq!(state.p1.core.hp, 100, "talent {talent}");
    }

    let mut combined = state_with_talents(vec![60, 10_060, 20_060, 30_060]);
    let card = combined.test_actor_card(PlayerSide::P1, 0);
    combined.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
    assert_eq!(combined.p1.core.max_hp, 155);
    assert_eq!(combined.p1.core.hp, 100);
}

#[test]
fn talent_60_repeats_on_slot_zero_and_does_not_leak_to_slot_one() {
    let mut state = state_with_talents(vec![60]);
    let card = state.test_actor_card(PlayerSide::P1, 0);
    state.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
    state.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
    state.apply_selected_card_hooks(PlayerSide::P1, &card, 1);

    assert_eq!(state.p1.core.max_hp, 120);
    assert_eq!(state.p1.core.hp, 100);
}

#[test]
fn talent_79_rows_reward_every_element_activation_including_repeats() {
    for (talent, expected) in [
        (79, (6, 0, 0, 0)),
        (10_079, (0, 6, 0, 0)),
        (20_079, (0, 0, 8, 0)),
        (30_079, (0, 0, 0, 4)),
    ] {
        let mut state = state_with_talents(vec![talent]);
        state.p1.core.hp = 60;
        state.activate_element(PlayerSide::P1, Element::Metal);
        state.activate_element(PlayerSide::P1, Element::Metal);

        assert_eq!(state.p1.core.defense, expected.0, "talent {talent}");
        assert_eq!(state.p1.core.hp, 60 + expected.1, "talent {talent}");
        assert_eq!(state.p1.sword.sharpness, expected.2, "talent {talent}");
        assert_eq!(state.p1.core.anima, expected.3, "talent {talent}");
        assert_eq!(state.p1.elements.activated_metal, 2, "talent {talent}");
    }
}

#[test]
fn talent_79_combined_rows_cover_all_five_elements_and_control_is_negative() {
    let mut active = state_with_talents(vec![79, 10_079, 20_079, 30_079]);
    active.p1.core.hp = 50;
    for element in [
        Element::Metal,
        Element::Water,
        Element::Wood,
        Element::Fire,
        Element::Earth,
        Element::Metal,
    ] {
        active.activate_element(PlayerSide::P1, element);
    }
    assert_eq!(active.p1.core.defense, 18);
    assert_eq!(active.p1.core.hp, 68);
    assert_eq!(active.p1.sword.sharpness, 24);
    assert_eq!(active.p1.core.anima, 12);

    let mut control = state_with_talents(Vec::new());
    control.p1.core.hp = 50;
    control.activate_element(PlayerSide::P1, Element::Metal);
    control.activate_element(PlayerSide::P1, Element::Metal);
    assert_eq!(control.p1.core.defense, 0);
    assert_eq!(control.p1.core.hp, 50);
    assert_eq!(control.p1.sword.sharpness, 0);
    assert_eq!(control.p1.core.anima, 0);
}

#[derive(Clone, Copy)]
enum FormationBranch {
    Water,
    Earth,
    Fire,
    Metal,
    Wood,
}

fn set_formation(state: &mut ReplayState, branch: FormationBranch, value: i64) {
    match branch {
        FormationBranch::Water => state.p1.elements.water_formation = value,
        FormationBranch::Earth => state.p1.elements.earth_formation = value,
        FormationBranch::Fire => state.p1.elements.fire_formation = value,
        FormationBranch::Metal => state.p1.elements.metal_formation = value,
        FormationBranch::Wood => state.p1.elements.wood_array = value,
    }
}

fn generated_card_id(branch: FormationBranch) -> i64 {
    match branch {
        FormationBranch::Water => 7_000_003, // water generates wood
        FormationBranch::Earth => 7_000_001, // metal triggers earth formation
        FormationBranch::Fire => 7_000_011,  // earth triggers fire formation
        FormationBranch::Metal => 7_000_006, // water triggers metal formation
        FormationBranch::Wood => 7_000_009,  // fire triggers wood formation
    }
}

fn formation_observation(state: &ReplayState) -> (i64, i64, i64, i64, i64, i64) {
    (
        state.p1.core.defense,
        state.p1.core.anima,
        state.p1.sword.sharpness,
        state.p1.core.hp,
        state.p2.core.hp,
        state.p2.core.max_hp,
    )
}

#[test]
fn talent_130_extends_all_five_standard_formations_to_their_generating_element() {
    for (branch, expected) in [
        (FormationBranch::Water, (0, 2, 0, 50, 100, 100)),
        (FormationBranch::Earth, (2, 0, 0, 50, 100, 100)),
        (FormationBranch::Fire, (0, 0, 0, 50, 98, 98)),
        (FormationBranch::Metal, (0, 0, 2, 50, 100, 100)),
        (FormationBranch::Wood, (0, 0, 0, 52, 100, 100)),
    ] {
        let mut active = state_with_talents(vec![130]);
        active.p1.core.hp = 50;
        set_formation(&mut active, branch, 2);
        let card = original_card(generated_card_id(branch));
        active.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
        assert_eq!(formation_observation(&active), expected);

        let mut control = state_with_talents(Vec::new());
        control.p1.core.hp = 50;
        set_formation(&mut control, branch, 2);
        control.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
        assert_eq!(formation_observation(&control), (0, 0, 0, 50, 100, 100));
    }
}

#[test]
fn talent_130_extends_card_action_base_3180_only_from_wood_to_fire() {
    let fire = original_card(7_000_009);
    let wood = original_card(7_000_003);

    let mut fire_with_talent = state_with_talents(vec![130]);
    fire_with_talent.p1.core.hp = 50;
    fire_with_talent.p1.core.attack_bonus = 3;
    fire_with_talent.p1.elements.wood_healing_formation = 2;
    fire_with_talent.apply_selected_card_hooks(PlayerSide::P1, &fire, 0);
    assert_eq!(fire_with_talent.p1.core.hp, 56);

    let mut fire_control = state_with_talents(Vec::new());
    fire_control.p1.core.hp = 50;
    fire_control.p1.core.attack_bonus = 3;
    fire_control.p1.elements.wood_healing_formation = 2;
    fire_control.apply_selected_card_hooks(PlayerSide::P1, &fire, 0);
    assert_eq!(fire_control.p1.core.hp, 50);

    let mut wood_control = state_with_talents(Vec::new());
    wood_control.p1.core.hp = 50;
    wood_control.p1.core.attack_bonus = 3;
    wood_control.p1.elements.wood_healing_formation = 2;
    wood_control.apply_selected_card_hooks(PlayerSide::P1, &wood, 0);
    assert_eq!(wood_control.p1.core.hp, 56);
}

#[test]
fn talent_20093_adapts_and_executes_only_fixture_card_19() {
    let mut card_19 = original_card(19);
    card_19.defense = Some(4);
    let mut p1 = player(deck_with(vec![card_19]));
    p1.talents = vec![20_093];
    let source = fixture(p1, player(deck_with(vec![basic_attack()])));

    assert_eq!(source.players.p1.cards[0].attack, Some(7));
    assert_eq!(source.players.p1.cards[0].defense, Some(4));
    let mut state = ReplayState::test_from_fixture(&source);
    let adapted = state.test_actor_card(PlayerSide::P1, 0);
    assert_eq!(adapted.id, 19);
    assert_eq!(adapted.attack, Some(5));
    assert_eq!(adapted.defense, Some(9));
    state.test_apply_card_effect(PlayerSide::P1, &adapted, 0);
    assert_eq!(state.p1.core.defense, 9);
    assert_eq!(state.p2.core.hp, 95);

    let mut control_card = original_card(19);
    control_card.defense = Some(4);
    let control = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![control_card])),
        player(deck_with(vec![basic_attack()])),
    ));
    let control_card = control.test_actor_card(PlayerSide::P1, 0);
    assert_eq!(control_card.attack, Some(7));
    assert_eq!(control_card.defense, Some(4));

    let mut unrelated = original_card(20);
    unrelated.attack = Some(7);
    unrelated.defense = Some(4);
    let mut unrelated_player = player(deck_with(vec![unrelated]));
    unrelated_player.talents = vec![20_093];
    let unrelated_state = ReplayState::test_from_fixture(&fixture(
        unrelated_player,
        player(deck_with(vec![basic_attack()])),
    ));
    let unrelated_card = unrelated_state.test_actor_card(PlayerSide::P1, 0);
    assert_eq!(unrelated_card.attack, Some(7));
    assert_eq!(unrelated_card.defense, Some(4));
}
