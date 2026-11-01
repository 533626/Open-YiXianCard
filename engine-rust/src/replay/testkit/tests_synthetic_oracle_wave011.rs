use super::support::{normalized_base_id, other_param};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

const HUNDRED_BEAST_FORMATION_IDS: [i64; 3] = [49, 10_049, 20_049];
const DRIPPING_WATER_IDS: [i64; 3] = [54, 10_054, 20_054];
const WAVE_CUTTING_IDS: [i64; 3] = [58, 10_058, 20_058];
const ICE_SNOW_LOTUS_IDS: [i64; 2] = [333, 10_333];
const INSTANT_SHADOW_STRIKE_IDS: [i64; 3] = [338, 10_338, 20_338];
const SHARPENING_TALISMAN_IDS: [i64; 2] = [379, 10_379];
const DREAM_LI_HEXAGRAM_IDS: [i64; 5] = [4_000_080, 4_010_080, 4_020_080, 4_030_080, 4_040_080];
const DREAM_OPEN_SEA_BODY_FORGE_IDS: [i64; 5] =
    [10_000_072, 10_010_072, 10_020_072, 10_030_072, 10_040_072];
const DREAM_FORGE_MUSCLES_IDS: [i64; 5] =
    [10_000_075, 10_010_075, 10_020_075, 10_030_075, 10_040_075];

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn deck(cards: impl IntoIterator<Item = CardDefinition>) -> Vec<CardDefinition> {
    super::test_support::resize_deck(cards.into_iter().collect(), basic_attack())
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 500, None, active_slot_count, None)
}

fn fixture(p1_cards: Vec<CardDefinition>, active_slot_count: usize) -> BattleFixture {
    super::test_support::default_fixture(
        player(p1_cards, active_slot_count),
        player(deck([basic_attack()]), 1),
    )
}

/// One row of the data-driven wave011 card-shape tables: every loop iteration
/// of the migrated tests below runs as an individually named `#[test]` with
/// the verbatim original body and assertions. The row identity is the
/// generated test name suffix (for example `card_49`).
struct CardCase {
    card_id: i64,
}

#[test]
fn wave011_exact_physical_rows_load_and_normalize_to_ten_targets() {
    let rows = [
        HUNDRED_BEAST_FORMATION_IDS.as_slice(),
        DRIPPING_WATER_IDS.as_slice(),
        WAVE_CUTTING_IDS.as_slice(),
        ICE_SNOW_LOTUS_IDS.as_slice(),
        INSTANT_SHADOW_STRIKE_IDS.as_slice(),
        &[342],
        SHARPENING_TALISMAN_IDS.as_slice(),
        DREAM_LI_HEXAGRAM_IDS.as_slice(),
        DREAM_OPEN_SEA_BODY_FORGE_IDS.as_slice(),
        DREAM_FORGE_MUSCLES_IDS.as_slice(),
    ]
    .concat();
    assert_eq!(rows.len(), 32);
    let bases: Vec<_> = rows
        .into_iter()
        .map(|id| normalized_base_id(&original_card(id)))
        .collect();
    assert_eq!(
        bases,
        [
            49, 49, 49, 54, 54, 54, 58, 58, 58, 333, 333, 338, 338, 338, 342, 379, 379, 4_000_080,
            4_000_080, 4_000_080, 4_000_080, 4_000_080, 10_000_072, 10_000_072, 10_000_072,
            10_000_072, 10_000_072, 10_000_075, 10_000_075, 10_000_075, 10_000_075, 10_000_075,
        ]
    );
}

const HUNDRED_BEAST_FORMATION_CASES: &[CardCase] = &[
    CardCase {
        card_id: HUNDRED_BEAST_FORMATION_IDS[0],
    },
    CardCase {
        card_id: HUNDRED_BEAST_FORMATION_IDS[1],
    },
    CardCase {
        card_id: HUNDRED_BEAST_FORMATION_IDS[2],
    },
];

fn run_hundred_beast_formation_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));

    state.test_execute_one_card(PlayerSide::P1);

    let anima = card.anima.unwrap_or(0);
    assert_eq!(state.p1.core.anima, anima, "card {card_id}");
    assert_eq!(state.p2.core.hp, 500 - anima, "card {card_id}");
    assert_eq!(
        state.p1.sword.hundred_beast_spirit_sword_formation, 1,
        "card {card_id}"
    );
    assert!(
        state
            .p1
            .sword
            .hundred_beast_spirit_sword_formation_triggered,
        "card {card_id}"
    );
    assert!(state.p1.deck.slots[0].skipped, "card {card_id}");
}

crate::data_driven_cases! {
    table: HUNDRED_BEAST_FORMATION_CASES,
    run: run_hundred_beast_formation_case,
    suite: "replay::tests_synthetic_oracle_wave011::hundred_beast_formation_installs_and_triggers_on_itself_for_all_shapes",
    valid: hundred_beast_formation_installs_and_triggers_case_table_is_valid,
    cases: [
        hundred_beast_formation_installs_and_triggers_on_itself_for_all_shapes_card_49 => 0,
        hundred_beast_formation_installs_and_triggers_on_itself_for_all_shapes_card_10049 => 1,
        hundred_beast_formation_installs_and_triggers_on_itself_for_all_shapes_card_20049 => 2,
    ]
}

#[test]
fn hundred_beast_formation_is_once_per_turn_resets_and_marks_zero_damage() {
    let spirit_sword = original_card(1_000_020);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([spirit_sword.clone()]), 1));
    state.p1.core.anima = 3;
    state.p1.sword.hundred_beast_spirit_sword_formation = 2;

    state.apply_hundred_beast_spirit_sword_formation_after_card(PlayerSide::P1, &spirit_sword);
    assert_eq!(state.p2.core.hp, 494);
    state.apply_hundred_beast_spirit_sword_formation_after_card(PlayerSide::P1, &spirit_sword);
    assert_eq!(state.p2.core.hp, 494);
    state.reset_hundred_beast_spirit_sword_formation_marker(PlayerSide::P1);
    state.apply_hundred_beast_spirit_sword_formation_after_card(PlayerSide::P1, &spirit_sword);
    assert_eq!(state.p2.core.hp, 488);

    state.p1.core.anima = 0;
    state.reset_hundred_beast_spirit_sword_formation_marker(PlayerSide::P1);
    state.apply_hundred_beast_spirit_sword_formation_after_card(PlayerSide::P1, &spirit_sword);
    assert_eq!(state.p2.core.hp, 488);
    assert!(
        state
            .p1
            .sword
            .hundred_beast_spirit_sword_formation_triggered
    );

    state.p1.deck.slots[0].card = basic_attack().into();
    state.test_play_actor_turn();
    assert!(
        !state
            .p1
            .sword
            .hundred_beast_spirit_sword_formation_triggered
    );
}

const DREAM_LI_HEXAGRAM_CASES: &[CardCase] = &[
    CardCase {
        card_id: DREAM_LI_HEXAGRAM_IDS[0],
    },
    CardCase {
        card_id: DREAM_LI_HEXAGRAM_IDS[1],
    },
    CardCase {
        card_id: DREAM_LI_HEXAGRAM_IDS[2],
    },
    CardCase {
        card_id: DREAM_LI_HEXAGRAM_IDS[3],
    },
    CardCase {
        card_id: DREAM_LI_HEXAGRAM_IDS[4],
    },
];

fn run_dream_li_hexagram_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.astrology.hexagram = 3;
    state.p2.core.hp = 200;
    state.p2.core.max_hp = 200;
    state.p2.core.defense = 50;

    state.test_execute_one_card(PlayerSide::P1);

    let expected_loss = other_param(&card, 0) + 3 * other_param(&card, 1);
    assert_eq!(state.p1.astrology.hexagram, 0, "card {card_id}");
    assert_eq!(state.p2.core.hp, 200 - expected_loss, "card {card_id}");
    assert_eq!(state.p2.core.defense, 50, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_LI_HEXAGRAM_CASES,
    run: run_dream_li_hexagram_case,
    suite: "replay::tests_synthetic_oracle_wave011::dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes",
    valid: dream_li_hexagram_consumes_the_snapshot_case_table_is_valid,
    cases: [
        dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes_card_4000080 => 0,
        dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes_card_4010080 => 1,
        dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes_card_4020080 => 2,
        dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes_card_4030080 => 3,
        dream_li_hexagram_consumes_the_snapshot_before_direct_hp_loss_for_all_shapes_card_4040080 => 4,
    ]
}

const DREAM_ICE_SNOW_LOTUS_CASES: &[CardCase] = &[
    CardCase {
        card_id: ICE_SNOW_LOTUS_IDS[0],
    },
    CardCase {
        card_id: ICE_SNOW_LOTUS_IDS[1],
    },
];

fn run_dream_ice_snow_lotus_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.defense = 2;
    state.p1.core.guard = 3;

    state.test_execute_one_card(PlayerSide::P1);

    let first_use_bonus = i64::from(card_id == 10_333);
    assert_eq!(
        state.p1.core.defense,
        2 + card.defense.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.guard,
        3 + other_param(&card, 0) + first_use_bonus,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: DREAM_ICE_SNOW_LOTUS_CASES,
    run: run_dream_ice_snow_lotus_case,
    suite: "replay::tests_synthetic_oracle_wave011::dream_ice_snow_lotus_adds_huashen_first_use_guard_only_once",
    valid: dream_ice_snow_lotus_adds_huashen_first_use_case_table_is_valid,
    cases: [
        dream_ice_snow_lotus_adds_huashen_first_use_guard_only_once_card_333 => 0,
        dream_ice_snow_lotus_adds_huashen_first_use_guard_only_once_card_10333 => 1,
    ]
}

#[test]
fn dream_ice_snow_lotus_adds_huashen_first_use_guard_only_once_reused_card() {
    let card = original_card(10_333);
    let mut reused = ReplayState::test_from_fixture(&fixture(deck([card]), 1));
    reused.p1.deck.slots[0].used = true;
    reused.test_execute_one_card(PlayerSide::P1);
    assert_eq!(reused.p1.core.guard, 1);
}

#[test]
fn dream_purity_charm_reduces_every_present_negative_type_then_adds_exorcism() {
    let card = original_card(342);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card]), 1));
    state.p1.core.anima = 2;
    state.p1.status.internal_injury = 5;
    state.p1.status.weakness = 5;
    state.p1.status.flaw = 5;
    state.p1.status.attack_reduction = 5;
    state.p1.status.entangle = 5;
    state.p1.status.external_injury = 5;
    state.p1.status.meditation = 5;
    state.p1.status.lost_mind = 5;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 5);
    assert_eq!(state.p1.status.internal_injury, 1);
    assert_eq!(state.p1.status.weakness, 1);
    assert_eq!(state.p1.status.flaw, 1);
    assert_eq!(state.p1.status.attack_reduction, 1);
    assert_eq!(state.p1.status.entangle, 1);
    assert_eq!(state.p1.status.external_injury, 1);
    assert_eq!(state.p1.status.meditation, 1);
    assert_eq!(state.p1.status.lost_mind, 1);
    assert_eq!(state.p1.fate.exorcism, 4);
}

const DREAM_SHARPENING_TALISMAN_CASES: &[CardCase] = &[
    CardCase {
        card_id: SHARPENING_TALISMAN_IDS[0],
    },
    CardCase {
        card_id: SHARPENING_TALISMAN_IDS[1],
    },
];

fn run_dream_sharpening_talisman_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut ordinary = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    ordinary.p2.core.hp = 100;
    ordinary.p2.core.max_hp = 100;
    ordinary.p2.core.defense = 5;
    ordinary.test_execute_one_card(PlayerSide::P1);
    assert_eq!(ordinary.p2.core.defense, 0, "card {card_id}");
    assert_eq!(
        ordinary.p2.core.hp,
        100 - (other_param(&card, 0) - 5),
        "card {card_id}"
    );

    let mut execution = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    execution.p2.core.hp = other_param(&card, 1);
    execution.p2.core.max_hp = 100;
    execution.p2.core.defense = 20;
    execution.p2.core.guard = 2;
    execution.test_execute_one_card(PlayerSide::P1);
    assert_eq!(execution.p2.core.hp, -100, "card {card_id}");
    assert_eq!(execution.p2.core.defense, 20, "card {card_id}");
    assert_eq!(execution.p2.core.guard, 2, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_SHARPENING_TALISMAN_CASES,
    run: run_dream_sharpening_talisman_case,
    suite: "replay::tests_synthetic_oracle_wave011::dream_sharpening_talisman_uses_damage_above_threshold_and_sets_hp_at_it",
    valid: dream_sharpening_talisman_uses_damage_case_table_is_valid,
    cases: [
        dream_sharpening_talisman_uses_damage_above_threshold_and_sets_hp_at_it_card_379 => 0,
        dream_sharpening_talisman_uses_damage_above_threshold_and_sets_hp_at_it_card_10379 => 1,
    ]
}

const DRIPPING_WATER_CASES: &[CardCase] = &[
    CardCase {
        card_id: DRIPPING_WATER_IDS[0],
    },
    CardCase {
        card_id: DRIPPING_WATER_IDS[1],
    },
    CardCase {
        card_id: DRIPPING_WATER_IDS[2],
    },
];

fn run_dripping_water_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.deck.slots[0].used = true;
    state.p2.status.weakness = 1;

    assert!(
        state.test_execute_one_card(PlayerSide::P1),
        "card {card_id}"
    );
    assert_eq!(state.p2.core.hp, 499, "card {card_id}");
    assert_eq!(
        state.p2.status.internal_injury,
        other_param(&card, 0),
        "card {card_id}"
    );
    assert_eq!(state.p1.turn.action_again_count, 1, "card {card_id}");
}

crate::data_driven_cases! {
    table: DRIPPING_WATER_CASES,
    run: run_dripping_water_case,
    suite: "replay::tests_synthetic_oracle_wave011::dripping_water_samples_action_again_before_rear_move_internal_injury_for_all_shapes",
    valid: dripping_water_samples_action_again_case_table_is_valid,
    cases: [
        dripping_water_samples_action_again_before_rear_move_internal_injury_for_all_shapes_card_54 => 0,
        dripping_water_samples_action_again_before_rear_move_internal_injury_for_all_shapes_card_10054 => 1,
        dripping_water_samples_action_again_before_rear_move_internal_injury_for_all_shapes_card_20054 => 2,
    ]
}

#[test]
fn dripping_water_samples_action_again_before_rear_move_internal_injury_for_all_shapes_no_prior_weakness(
) {
    let card = original_card(54);
    let mut no_prior = ReplayState::test_from_fixture(&fixture(deck([card]), 1));
    no_prior.p1.deck.slots[0].used = true;
    assert!(!no_prior.test_execute_one_card(PlayerSide::P1));
    assert_eq!(no_prior.p2.status.internal_injury, 6);
    assert_eq!(no_prior.p1.turn.action_again_count, 0);
}

const WAVE_CUTTING_CASES: &[CardCase] = &[
    CardCase {
        card_id: WAVE_CUTTING_IDS[0],
    },
    CardCase {
        card_id: WAVE_CUTTING_IDS[1],
    },
    CardCase {
        card_id: WAVE_CUTTING_IDS[2],
    },
];

fn run_wave_cutting_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    assert_eq!(
        state.p1.elements.activated_elements,
        vec![Element::Metal, Element::Water],
        "card {card_id}"
    );
    assert_eq!(state.p1.elements.activated_metal, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_water, 1, "card {card_id}");
    state.p1.core.anima = 1;

    state.test_execute_one_card(PlayerSide::P1);

    let final_anima = 1 + card.anima.unwrap_or(0);
    assert_eq!(state.p1.core.anima, final_anima, "card {card_id}");
    assert_eq!(
        state.p1.sword.sharpness,
        final_anima * other_param(&card, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: WAVE_CUTTING_CASES,
    run: run_wave_cutting_case,
    suite: "replay::tests_synthetic_oracle_wave011::wave_cutting_seal_opens_metal_then_water_and_uses_post_gain_anima_for_all_shapes",
    valid: wave_cutting_seal_opens_metal_then_water_case_table_is_valid,
    cases: [
        wave_cutting_seal_opens_metal_then_water_and_uses_post_gain_anima_for_all_shapes_card_58 => 0,
        wave_cutting_seal_opens_metal_then_water_and_uses_post_gain_anima_for_all_shapes_card_10058 => 1,
        wave_cutting_seal_opens_metal_then_water_and_uses_post_gain_anima_for_all_shapes_card_20058 => 2,
    ]
}

const DREAM_OPEN_SEA_BODY_FORGE_CASES: &[CardCase] = &[
    CardCase {
        card_id: DREAM_OPEN_SEA_BODY_FORGE_IDS[0],
    },
    CardCase {
        card_id: DREAM_OPEN_SEA_BODY_FORGE_IDS[1],
    },
    CardCase {
        card_id: DREAM_OPEN_SEA_BODY_FORGE_IDS[2],
    },
    CardCase {
        card_id: DREAM_OPEN_SEA_BODY_FORGE_IDS[3],
    },
    CardCase {
        card_id: DREAM_OPEN_SEA_BODY_FORGE_IDS[4],
    },
];

fn run_dream_open_sea_body_forge_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut card_fixture = fixture(deck([card.clone()]), 1);
    card_fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 5);
    let mut state = ReplayState::test_from_fixture(&card_fixture);
    state.p1.core.anima = 3;
    state.p1.core.defense = 1;
    state.p2.core.hp = 300;
    state.p2.core.max_hp = 300;
    let anima_bonus = if card_id >= 10_030_072 { 3 } else { 0 };

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p2.core.hp,
        300 - card.attack.unwrap_or(0) - anima_bonus,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.defense,
        1 + card.defense.unwrap_or(0) + anima_bonus,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.physique,
        card.physique.unwrap_or(0) + anima_bonus,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.max_hp,
        500 + card.physique.unwrap_or(0) + anima_bonus,
        "card {card_id}"
    );
    assert_eq!(state.p1.core.hp, 500, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_OPEN_SEA_BODY_FORGE_CASES,
    run: run_dream_open_sea_body_forge_case,
    suite: "replay::tests_synthetic_oracle_wave011::dream_open_sea_body_forge_freezes_current_anima_for_high_shapes",
    valid: dream_open_sea_body_forge_freezes_current_anima_case_table_is_valid,
    cases: [
        dream_open_sea_body_forge_freezes_current_anima_for_high_shapes_card_10000072 => 0,
        dream_open_sea_body_forge_freezes_current_anima_for_high_shapes_card_10010072 => 1,
        dream_open_sea_body_forge_freezes_current_anima_for_high_shapes_card_10020072 => 2,
        dream_open_sea_body_forge_freezes_current_anima_for_high_shapes_card_10030072 => 3,
        dream_open_sea_body_forge_freezes_current_anima_for_high_shapes_card_10040072 => 4,
    ]
}

const DREAM_FORGE_MUSCLES_CASES: &[CardCase] = &[
    CardCase {
        card_id: DREAM_FORGE_MUSCLES_IDS[0],
    },
    CardCase {
        card_id: DREAM_FORGE_MUSCLES_IDS[1],
    },
    CardCase {
        card_id: DREAM_FORGE_MUSCLES_IDS[2],
    },
    CardCase {
        card_id: DREAM_FORGE_MUSCLES_IDS[3],
    },
    CardCase {
        card_id: DREAM_FORGE_MUSCLES_IDS[4],
    },
];

fn run_dream_forge_muscles_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut card_fixture = fixture(deck([card.clone()]), 1);
    card_fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 5);
    let mut state = ReplayState::test_from_fixture(&card_fixture);
    state.p1.core.hp = 99;
    state.p1.core.max_hp = 99;
    state.p1.core.defense = 1;
    let updated_max_hp = 99 + card.physique.unwrap_or(0);
    let defense = if card_id >= 10_030_075 {
        updated_max_hp * other_param(&card, 0) / 100
    } else {
        card.defense.unwrap_or(0)
    };

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.physique,
        card.physique.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.core.max_hp, updated_max_hp, "card {card_id}");
    assert_eq!(state.p1.core.defense, 1 + defense, "card {card_id}");
    assert_eq!(state.p1.core.hp, 99, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_FORGE_MUSCLES_CASES,
    run: run_dream_forge_muscles_case,
    suite: "replay::tests_synthetic_oracle_wave011::dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes",
    valid: dream_forge_muscles_uses_updated_max_hp_case_table_is_valid,
    cases: [
        dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes_card_10000075 => 0,
        dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes_card_10010075 => 1,
        dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes_card_10020075 => 2,
        dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes_card_10030075 => 3,
        dream_forge_muscles_uses_updated_max_hp_only_for_high_shapes_card_10040075 => 4,
    ]
}

const INSTANT_SHADOW_STRIKE_CASES: &[CardCase] = &[
    CardCase {
        card_id: INSTANT_SHADOW_STRIKE_IDS[0],
    },
    CardCase {
        card_id: INSTANT_SHADOW_STRIKE_IDS[1],
    },
    CardCase {
        card_id: INSTANT_SHADOW_STRIKE_IDS[2],
    },
];

fn run_instant_shadow_strike_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.hp,
        500 - card.hp_cost.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.turn.agility, 8, "card {card_id}");
    assert_eq!(
        state.p1.fate.instant_shadow_strike,
        other_param(&card, 1),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: INSTANT_SHADOW_STRIKE_CASES,
    run: run_instant_shadow_strike_case,
    suite: "replay::tests_synthetic_oracle_wave011::instant_shadow_strike_pays_its_own_cost_then_installs_agility_and_layers_for_all_shapes",
    valid: instant_shadow_strike_pays_its_own_cost_case_table_is_valid,
    cases: [
        instant_shadow_strike_pays_its_own_cost_then_installs_agility_and_layers_for_all_shapes_card_338 => 0,
        instant_shadow_strike_pays_its_own_cost_then_installs_agility_and_layers_for_all_shapes_card_10338 => 1,
        instant_shadow_strike_pays_its_own_cost_then_installs_agility_and_layers_for_all_shapes_card_20338 => 2,
    ]
}

#[test]
fn instant_shadow_strike_skips_four_raw_hp_cost_cards_without_completing_them() {
    let shadow = original_card(20_338);
    let hp_cost = original_card(10_000_001);
    let cards = deck([
        shadow,
        hp_cost.clone(),
        hp_cost.clone(),
        hp_cost.clone(),
        hp_cost,
        basic_attack(),
    ]);
    let mut state = ReplayState::test_from_fixture(&fixture(cards, 6));

    state.test_execute_one_card(PlayerSide::P1);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 484);
    assert_eq!(state.p2.core.hp, 473);
    assert_eq!(state.p1.turn.agility, 8);
    assert_eq!(state.p1.fate.instant_shadow_strike, 0);
    assert_eq!(state.p1.deck.queue, vec![0, 1, 2, 3, 4, 5]);
    assert!(state.p1.deck.slots[1..5]
        .iter()
        .all(|slot| !slot.used && !slot.skipped));
    assert_eq!(state.p1.turn.hp_cost_cards_used, 1);
    assert_eq!(state.p1.turn.used_card_count, 2);
    let completed: Vec<_> = state
        .test_events()
        .iter()
        .filter(|event| event.kind == ReplayEventKind::CardCompleted)
        .filter_map(|event| event.card_id)
        .collect();
    assert_eq!(completed, vec![20_338, 0]);
}

#[test]
fn instant_shadow_strike_uses_printed_cost_and_bypasses_normal_cost_hooks() {
    let shadow = original_card(338);
    let raw_cost = original_card(10_000_005);
    let printed = raw_cost.hp_cost.unwrap_or(0);
    let mut card_fixture = fixture(deck([shadow, raw_cost, basic_attack()]), 3);
    card_fixture.players.p1.talents.push(174);
    let mut state = ReplayState::test_from_fixture(&card_fixture);
    state.p1.core.physique = printed * 2;

    state.test_execute_one_card(PlayerSide::P1);
    state.p1.turn.blood_shadow = 1;
    let hp_before = state.p1.core.hp;
    let max_hp_before = state.p1.core.max_hp;
    let agility_before = state.p1.turn.agility;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, hp_before - printed);
    assert_eq!(state.p1.core.max_hp, max_hp_before);
    assert_eq!(state.p1.turn.blood_shadow, 1);
    assert_eq!(state.p1.turn.agility, agility_before);
    assert_eq!(state.p1.turn.hp_cost_cards_used, 1);
    assert_eq!(state.p1.fate.instant_shadow_strike, 1);
    assert!(!state.p1.deck.slots[1].used);
    assert!(!state.p1.deck.slots[1].skipped);
}

#[test]
fn instant_shadow_strike_checks_printed_hp_cost_before_soul_hound_transform() {
    let shadow = original_card(338);
    let hp_cost = original_card(10_000_001);
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck([shadow, hp_cost, basic_attack()]), 3));
    state.test_execute_one_card(PlayerSide::P1);
    state.p1.chance.you_ming_xu_hun_quan = 1;
    let hp_before = state.p1.core.hp;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.chance.you_ming_xu_hun_quan, 0);
    assert_eq!(state.p1.fate.instant_shadow_strike, 1);
    assert_eq!(state.p1.core.hp, hp_before - 3);
    assert_eq!(state.p2.core.hp, 491);
    assert_eq!(normalized_base_id(&state.p1.deck.slots[1].card), 10_000_001);
    assert!(!state.p1.deck.slots[1].used);
    assert_eq!(normalized_base_id(&state.p1.deck.slots[2].card), 0);
    assert!(state.p1.deck.slots[2].used);
}

#[test]
fn instant_shadow_strike_settles_prior_queue_skip_openings_before_raw_cost() {
    let shadow = original_card(338);
    let opening = original_card(58);
    let hp_cost = original_card(10_000_001);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([shadow, opening, hp_cost, basic_attack()]),
        4,
    ));
    assert_eq!(state.p1.elements.activated_metal, 1);
    assert_eq!(state.p1.elements.activated_water, 1);
    state.test_execute_one_card(PlayerSide::P1);
    state.p1.fate.fate_cycle = 1;
    state.p1.fate.fate_cycle_slots[0] = 2;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.elements.activated_metal, 2);
    assert_eq!(state.p1.elements.activated_water, 2);
    assert_eq!(state.p1.fate.fate_cycle, 0);
    assert_eq!(state.p1.fate.instant_shadow_strike, 1);
    assert!(!state.p1.deck.slots[1].used);
    assert!(!state.p1.deck.slots[2].used);
    assert!(!state.p1.deck.slots[2].skipped);
}

#[test]
fn instant_shadow_strike_stops_at_non_hp_cost_without_consuming_a_layer_then_resumes() {
    let shadow = original_card(338);
    let hp_cost = original_card(10_000_001);
    let cards = deck([
        shadow,
        basic_attack(),
        hp_cost.clone(),
        hp_cost,
        basic_attack(),
    ]);
    let mut state = ReplayState::test_from_fixture(&fixture(cards, 5));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.instant_shadow_strike, 2);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.instant_shadow_strike, 2);
    assert_eq!(state.p1.deck.queue[0], 2);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.instant_shadow_strike, 0);
    assert_eq!(state.p1.core.hp, 492);
    assert_eq!(state.p2.core.hp, 482);
    assert_eq!(state.p1.deck.queue, vec![0, 1, 2, 3, 4]);
}
