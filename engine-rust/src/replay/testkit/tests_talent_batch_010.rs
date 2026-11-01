use super::*;
use crate::fixture::{
    BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck_with(cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    super::test_support::resize_deck(cards, basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 100, Some(0), 1, Some(6))
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

/// Typed rows for the data-driven talent-010 tables: every loop iteration of
/// the migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `talent_60`).
struct TalentGainCase {
    talent: i64,
    expected_gain: i64,
}

struct TalentRewardCase {
    talent: i64,
    expected_defense: i64,
    expected_hp_gain: i64,
    expected_sharpness: i64,
    expected_anima: i64,
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

const TALENT_60_GAIN_CASES: &[TalentGainCase] = &[
    TalentGainCase {
        talent: 60,
        expected_gain: 10,
    },
    TalentGainCase {
        talent: 10_060,
        expected_gain: 10,
    },
    TalentGainCase {
        talent: 20_060,
        expected_gain: 15,
    },
    TalentGainCase {
        talent: 30_060,
        expected_gain: 20,
    },
];

fn run_talent_60_gain_case(case: &TalentGainCase) {
    let talent = case.talent;
    let expected_gain = case.expected_gain;
    let mut state = state_with_talents(vec![talent]);
    let card = state.test_actor_card(PlayerSide::P1, 0);
    state.apply_selected_card_hooks(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.max_hp, 100 + expected_gain, "talent {talent}");
    assert_eq!(state.p1.core.hp, 100, "talent {talent}");
}

crate::data_driven_cases! {
    table: TALENT_60_GAIN_CASES,
    run: run_talent_60_gain_case,
    suite: "replay::tests_talent_batch_010::talent_60_rows_apply_their_exact_first_slot_max_hp_gain",
    valid: talent_60_rows_apply_their_exact_case_table_is_valid,
    cases: [
        talent_60_rows_apply_their_exact_first_slot_max_hp_gain_talent_60 => 0,
        talent_60_rows_apply_their_exact_first_slot_max_hp_gain_talent_10060 => 1,
        talent_60_rows_apply_their_exact_first_slot_max_hp_gain_talent_20060 => 2,
        talent_60_rows_apply_their_exact_first_slot_max_hp_gain_talent_30060 => 3,
    ]
}

#[test]
fn talent_60_rows_apply_their_exact_first_slot_max_hp_gain_combined() {
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

const TALENT_79_REWARD_CASES: &[TalentRewardCase] = &[
    TalentRewardCase {
        talent: 79,
        expected_defense: 6,
        expected_hp_gain: 0,
        expected_sharpness: 0,
        expected_anima: 0,
    },
    TalentRewardCase {
        talent: 10_079,
        expected_defense: 0,
        expected_hp_gain: 6,
        expected_sharpness: 0,
        expected_anima: 0,
    },
    TalentRewardCase {
        talent: 20_079,
        expected_defense: 0,
        expected_hp_gain: 0,
        expected_sharpness: 8,
        expected_anima: 0,
    },
    TalentRewardCase {
        talent: 30_079,
        expected_defense: 0,
        expected_hp_gain: 0,
        expected_sharpness: 0,
        expected_anima: 4,
    },
];

fn run_talent_79_reward_case(case: &TalentRewardCase) {
    let talent = case.talent;
    let expected = (
        case.expected_defense,
        case.expected_hp_gain,
        case.expected_sharpness,
        case.expected_anima,
    );
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

crate::data_driven_cases! {
    table: TALENT_79_REWARD_CASES,
    run: run_talent_79_reward_case,
    suite: "replay::tests_talent_batch_010::talent_79_rows_reward_every_element_activation_including_repeats",
    valid: talent_79_rows_reward_every_element_case_table_is_valid,
    cases: [
        talent_79_rows_reward_every_element_activation_including_repeats_talent_79 => 0,
        talent_79_rows_reward_every_element_activation_including_repeats_talent_10079 => 1,
        talent_79_rows_reward_every_element_activation_including_repeats_talent_20079 => 2,
        talent_79_rows_reward_every_element_activation_including_repeats_talent_30079 => 3,
    ]
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

struct TalentFormationCase {
    branch: FormationBranch,
    expected: (i64, i64, i64, i64, i64, i64),
}

const TALENT_130_FORMATION_CASES: &[TalentFormationCase] = &[
    TalentFormationCase {
        branch: FormationBranch::Water,
        expected: (0, 2, 0, 50, 100, 100),
    },
    TalentFormationCase {
        branch: FormationBranch::Earth,
        expected: (2, 0, 0, 50, 100, 100),
    },
    TalentFormationCase {
        branch: FormationBranch::Fire,
        expected: (0, 0, 0, 50, 98, 98),
    },
    TalentFormationCase {
        branch: FormationBranch::Metal,
        expected: (0, 0, 2, 50, 100, 100),
    },
    TalentFormationCase {
        branch: FormationBranch::Wood,
        expected: (0, 0, 0, 52, 100, 100),
    },
];

fn run_talent_130_formation_case(case: &TalentFormationCase) {
    let branch = case.branch;
    let expected = case.expected;
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

crate::data_driven_cases! {
    table: TALENT_130_FORMATION_CASES,
    run: run_talent_130_formation_case,
    suite: "replay::tests_talent_batch_010::talent_130_extends_all_five_standard_formations_to_their_generating_element",
    valid: talent_130_extends_all_five_standard_case_table_is_valid,
    cases: [
        talent_130_extends_all_five_standard_formations_to_their_generating_element_branch_water => 0,
        talent_130_extends_all_five_standard_formations_to_their_generating_element_branch_earth => 1,
        talent_130_extends_all_five_standard_formations_to_their_generating_element_branch_fire => 2,
        talent_130_extends_all_five_standard_formations_to_their_generating_element_branch_metal => 3,
        talent_130_extends_all_five_standard_formations_to_their_generating_element_branch_wood => 4,
    ]
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
