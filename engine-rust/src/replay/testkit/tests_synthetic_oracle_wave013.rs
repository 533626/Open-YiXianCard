use super::support::{div_ceil, normalized_base_id, other_param};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

const CUT_GRASS_IDS: [i64; 3] = [265, 10_265, 20_265];
const EARTH_MOUNTAINS_IDS: [i64; 3] = [274, 10_274, 20_274];
const RAKSHASA_POUNCE_IDS: [i64; 3] = [297, 10_297, 20_297];
const LOTUS_ROOT_IDS: [i64; 3] = [298, 10_298, 20_298];
const KUN_HEXAGRAM_IDS: [i64; 3] = [307, 10_307, 20_307];
const TWO_POLARITY_IDS: [i64; 3] = [320, 10_320, 20_320];
const IMMORTAL_BINDING_VINE_IDS: [i64; 2] = [343, 10_343];
const CLOUD_DANCE_IDS: [i64; 5] = [1_000_072, 1_010_072, 1_020_072, 1_030_072, 1_040_072];
const FRENZY_FIRST_FORM_IDS: [i64; 5] = [1_000_073, 1_010_073, 1_020_073, 1_030_073, 1_040_073];
const EARTH_CLIFF_IDS: [i64; 5] = [7_000_091, 7_010_091, 7_020_091, 7_030_091, 7_040_091];

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

/// One row of the data-driven wave013 card-shape tables: every loop iteration
/// of the migrated tests below runs as an individually named `#[test]` with
/// the verbatim original body and assertions. The row identity is the
/// generated test name suffix (for example `card_265`).
struct CardCase {
    card_id: i64,
}

#[test]
fn wave013_exact_physical_rows_load_realms_and_normalize_to_ten_targets() {
    let rows = [
        CUT_GRASS_IDS.as_slice(),
        EARTH_MOUNTAINS_IDS.as_slice(),
        RAKSHASA_POUNCE_IDS.as_slice(),
        LOTUS_ROOT_IDS.as_slice(),
        KUN_HEXAGRAM_IDS.as_slice(),
        TWO_POLARITY_IDS.as_slice(),
        IMMORTAL_BINDING_VINE_IDS.as_slice(),
        CLOUD_DANCE_IDS.as_slice(),
        FRENZY_FIRST_FORM_IDS.as_slice(),
        EARTH_CLIFF_IDS.as_slice(),
    ]
    .concat();
    assert_eq!(rows.len(), 35);
    let bases: std::collections::BTreeSet<_> = rows
        .iter()
        .map(|id| normalized_base_id(&original_card(*id)))
        .collect();
    assert_eq!(
        bases,
        [265, 274, 297, 298, 307, 320, 343, 1_000_072, 1_000_073, 7_000_091,].into()
    );
    for (index, id) in CLOUD_DANCE_IDS.into_iter().enumerate() {
        assert_eq!(
            original_config::original_card_realm_level(id),
            Some(index as i64 + 1)
        );
    }
    for (index, id) in FRENZY_FIRST_FORM_IDS.into_iter().enumerate() {
        assert_eq!(
            original_config::original_card_realm_level(id),
            Some(index as i64 + 1)
        );
    }
    for (index, id) in EARTH_CLIFF_IDS.into_iter().enumerate() {
        assert_eq!(
            original_config::original_card_realm_level(id),
            Some(index as i64 + 1)
        );
    }
}

const CUT_GRASS_CASES: &[CardCase] = &[
    CardCase {
        card_id: CUT_GRASS_IDS[0],
    },
    CardCase {
        card_id: CUT_GRASS_IDS[1],
    },
    CardCase {
        card_id: CUT_GRASS_IDS[2],
    },
];

fn run_cut_grass_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.anima = 6;
    state.p1.astrology.hexagram = 5;
    state.p2.core.hp = 70;
    state.p2.core.max_hp = 100;

    state.test_execute_one_card(PlayerSide::P1);

    let post_cost_anima = 5;
    let loss = 30
        + other_param(&card, 0)
        + post_cost_anima * other_param(&card, 1)
        + 5 * other_param(&card, 2);
    assert_eq!(state.p1.core.anima, 0, "card {card_id}");
    assert_eq!(state.p1.astrology.hexagram, 0, "card {card_id}");
    assert_eq!(state.p2.core.max_hp, 100 - loss, "card {card_id}");
    assert_eq!(state.p2.core.hp, 100 - loss, "card {card_id}");
    assert_eq!(state.p2.core.lost_max_hp_count, loss, "card {card_id}");
}

crate::data_driven_cases! {
    table: CUT_GRASS_CASES,
    run: run_cut_grass_case,
    suite: "replay::tests_synthetic_oracle_wave013::cut_grass_spends_post_cost_resources_and_reduces_exact_max_hp",
    valid: cut_grass_spends_post_cost_resources_case_table_is_valid,
    cases: [
        cut_grass_spends_post_cost_resources_and_reduces_exact_max_hp_card_265 => 0,
        cut_grass_spends_post_cost_resources_and_reduces_exact_max_hp_card_10265 => 1,
        cut_grass_spends_post_cost_resources_and_reduces_exact_max_hp_card_20265 => 2,
    ]
}

const EARTH_MOUNTAINS_CASES: &[CardCase] = &[
    CardCase {
        card_id: EARTH_MOUNTAINS_IDS[0],
    },
    CardCase {
        card_id: EARTH_MOUNTAINS_IDS[1],
    },
    CardCase {
        card_id: EARTH_MOUNTAINS_IDS[2],
    },
];

fn run_earth_mountains_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.defense = 5;
    state.p1.elements.activated_earth = 1;

    state.test_execute_one_card(PlayerSide::P1);

    let total = 5 + card.defense.unwrap_or(0);
    let lost = div_ceil(total, 2);
    assert_eq!(state.p1.core.defense, total - lost, "card {card_id}");
    assert_eq!(state.p2.core.hp, 500 - lost, "card {card_id}");
    assert_eq!(state.p1.turn.next_turn_defense, lost, "card {card_id}");
}

crate::data_driven_cases! {
    table: EARTH_MOUNTAINS_CASES,
    run: run_earth_mountains_case,
    suite: "replay::tests_synthetic_oracle_wave013::earth_mountains_halves_post_gain_defense_and_check_wu_xing_stores_it",
    valid: earth_mountains_halves_post_gain_defense_case_table_is_valid,
    cases: [
        earth_mountains_halves_post_gain_defense_and_check_wu_xing_stores_it_card_274 => 0,
        earth_mountains_halves_post_gain_defense_and_check_wu_xing_stores_it_card_10274 => 1,
        earth_mountains_halves_post_gain_defense_and_check_wu_xing_stores_it_card_20274 => 2,
    ]
}

const RAKSHASA_POUNCE_W13_CASES: &[CardCase] = &[
    CardCase {
        card_id: RAKSHASA_POUNCE_IDS[0],
    },
    CardCase {
        card_id: RAKSHASA_POUNCE_IDS[1],
    },
    CardCase {
        card_id: RAKSHASA_POUNCE_IDS[2],
    },
];

fn run_rakshasa_pounce_w13_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p2.status.internal_injury = 2;
    state.p2.status.weakness = 1;
    state.p2.status.flaw = 3;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.anima,
        card.anima.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p2.status.internal_injury, 0, "card {card_id}");
    assert_eq!(state.p2.status.weakness, 0, "card {card_id}");
    assert_eq!(state.p2.status.flaw, 0, "card {card_id}");
    assert_eq!(state.p2.status.external_injury, 6, "card {card_id}");
    let attack_count = card.attack_count.unwrap_or(1);
    let expected_damage_per_hit = card.attack.unwrap_or(0) + 6;
    assert_eq!(
        state.p2.core.hp,
        500 - expected_damage_per_hit * attack_count,
        "card {card_id}"
    );
    assert_eq!(state.p1.turn.ignore_defense_attacks, 0, "card {card_id}");
}

crate::data_driven_cases! {
    table: RAKSHASA_POUNCE_W13_CASES,
    run: run_rakshasa_pounce_w13_case,
    suite: "replay::tests_synthetic_oracle_wave013::rakshasa_converts_every_debuff_layer_before_attack_and_multi_hits",
    valid: rakshasa_converts_every_debuff_layer_case_table_is_valid,
    cases: [
        rakshasa_converts_every_debuff_layer_before_attack_and_multi_hits_card_297 => 0,
        rakshasa_converts_every_debuff_layer_before_attack_and_multi_hits_card_10297 => 1,
        rakshasa_converts_every_debuff_layer_before_attack_and_multi_hits_card_20297 => 2,
    ]
}

const LOTUS_ROOT_CASES: &[CardCase] = &[
    CardCase {
        card_id: LOTUS_ROOT_IDS[0],
    },
    CardCase {
        card_id: LOTUS_ROOT_IDS[1],
    },
    CardCase {
        card_id: LOTUS_ROOT_IDS[2],
    },
];

fn run_lotus_root_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 50;
    state.p1.core.max_hp = 100;

    state.test_execute_one_card(PlayerSide::P1);

    let recovery = other_param(&card, 0);
    let base = other_param(&card, 1);
    assert_eq!(state.p1.status.recovery, recovery, "card {card_id}");
    assert_eq!(
        state.p1.core.max_hp,
        100 + base + recovery,
        "card {card_id}"
    );
    assert_eq!(state.p1.core.hp, 50, "card {card_id}");

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.status.recovery, recovery * 2, "card {card_id}");
    assert_eq!(
        state.p1.core.max_hp,
        100 + base + recovery + base + recovery * 2,
        "card {card_id}"
    );
    assert_eq!(state.p1.core.hp, 50 + base + recovery * 2, "card {card_id}");
}

crate::data_driven_cases! {
    table: LOTUS_ROOT_CASES,
    run: run_lotus_root_case,
    suite: "replay::tests_synthetic_oracle_wave013::lotus_root_uses_post_gain_recovery_and_heals_only_on_rear_move",
    valid: lotus_root_uses_post_gain_recovery_case_table_is_valid,
    cases: [
        lotus_root_uses_post_gain_recovery_and_heals_only_on_rear_move_card_298 => 0,
        lotus_root_uses_post_gain_recovery_and_heals_only_on_rear_move_card_10298 => 1,
        lotus_root_uses_post_gain_recovery_and_heals_only_on_rear_move_card_20298 => 2,
    ]
}

const KUN_HEXAGRAM_CASES: &[CardCase] = &[
    CardCase {
        card_id: KUN_HEXAGRAM_IDS[0],
    },
    CardCase {
        card_id: KUN_HEXAGRAM_IDS[1],
    },
    CardCase {
        card_id: KUN_HEXAGRAM_IDS[2],
    },
];

fn run_kun_hexagram_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([card.clone(), basic_attack(), basic_attack()]),
        3,
    ));
    state.p1.astrology.hexagram = 2;

    state.test_execute_one_card(PlayerSide::P1);

    let total = 2 + card.hexagram.unwrap_or(0);
    let charges = 1 + total / other_param(&card, 0).max(1);
    assert_eq!(
        state.p1.core.defense,
        card.defense.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.astrology.hexagram, total, "card {card_id}");
    assert_eq!(
        state.p1.fate.paint_finishing_touch, charges,
        "card {card_id}"
    );

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.deck.slots[1].card.id, 10_000, "card {card_id}");
    assert_eq!(
        state.p1.fate.paint_finishing_touch,
        charges - 1,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: KUN_HEXAGRAM_CASES,
    run: run_kun_hexagram_case,
    suite: "replay::tests_synthetic_oracle_wave013::kun_hexagram_uses_post_gain_value_and_upgrades_the_next_card",
    valid: kun_hexagram_uses_post_gain_value_case_table_is_valid,
    cases: [
        kun_hexagram_uses_post_gain_value_and_upgrades_the_next_card_card_307 => 0,
        kun_hexagram_uses_post_gain_value_and_upgrades_the_next_card_card_10307 => 1,
        kun_hexagram_uses_post_gain_value_and_upgrades_the_next_card_card_20307 => 2,
    ]
}

const TWO_POLARITY_CASES: &[CardCase] = &[
    CardCase {
        card_id: TWO_POLARITY_IDS[0],
    },
    CardCase {
        card_id: TWO_POLARITY_IDS[1],
    },
    CardCase {
        card_id: TWO_POLARITY_IDS[2],
    },
];

fn run_two_polarity_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 50;
    state.p1.core.max_hp = 100;
    state.p1.core.anima = 4;
    state.p1.astrology.hexagram = 2;

    state.test_execute_one_card(PlayerSide::P1);

    let post_print_anima = 4 + card.anima.unwrap_or(0);
    let gain = (post_print_anima + 2) * other_param(&card, 0);
    assert_eq!(state.p1.core.defense, gain, "card {card_id}");
    assert_eq!(state.p1.core.hp, 50 + gain, "card {card_id}");
    assert_eq!(state.p1.core.anima, 2, "card {card_id}");
    assert_eq!(
        state.p1.astrology.hexagram, post_print_anima,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: TWO_POLARITY_CASES,
    run: run_two_polarity_case,
    suite: "replay::tests_synthetic_oracle_wave013::two_polarity_snapshots_post_print_resources_then_swaps_them",
    valid: two_polarity_snapshots_post_print_resources_case_table_is_valid,
    cases: [
        two_polarity_snapshots_post_print_resources_then_swaps_them_card_320 => 0,
        two_polarity_snapshots_post_print_resources_then_swaps_them_card_10320 => 1,
        two_polarity_snapshots_post_print_resources_then_swaps_them_card_20320 => 2,
    ]
}

const IMMORTAL_BINDING_VINE_CASES: &[CardCase] = &[
    CardCase {
        card_id: IMMORTAL_BINDING_VINE_IDS[0],
    },
    CardCase {
        card_id: IMMORTAL_BINDING_VINE_IDS[1],
    },
];

fn run_immortal_binding_vine_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.status.external_injury, other_param(&card, 0));
    assert_eq!(
        state.p1.turn.next_attack_shatter_defense,
        other_param(&card, 1)
    );
    assert_eq!(state.p2.status.entangle, other_param(&card, 2));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.status.external_injury, other_param(&card, 0) * 2);
    assert_eq!(state.p2.status.entangle, other_param(&card, 2));
}

crate::data_driven_cases! {
    table: IMMORTAL_BINDING_VINE_CASES,
    run: run_immortal_binding_vine_case,
    suite: "replay::tests_synthetic_oracle_wave013::immortal_binding_vine_applies_entangle_only_on_first_use",
    valid: immortal_binding_vine_applies_entangle_case_table_is_valid,
    cases: [
        immortal_binding_vine_applies_entangle_only_on_first_use_card_343 => 0,
        immortal_binding_vine_applies_entangle_only_on_first_use_card_10343 => 1,
    ]
}

const CLOUD_DANCE_CASES: &[CardCase] = &[
    CardCase {
        card_id: CLOUD_DANCE_IDS[0],
    },
    CardCase {
        card_id: CLOUD_DANCE_IDS[1],
    },
    CardCase {
        card_id: CLOUD_DANCE_IDS[2],
    },
    CardCase {
        card_id: CLOUD_DANCE_IDS[3],
    },
    CardCase {
        card_id: CLOUD_DANCE_IDS[4],
    },
];

fn run_cloud_dance_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.sword.sword_intent = 6;

    state.test_execute_one_card(PlayerSide::P1);

    let sword_intent = 6 + card.sword_intent.unwrap_or(0);
    let realm = original_config::original_card_realm_level(card_id).unwrap();
    let expected_frenzy = if realm < 4 {
        0
    } else {
        sword_intent / other_param(&card, 0).max(1) + i64::from(realm == 5)
    };
    assert_eq!(
        state.p1.core.anima,
        card.anima.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.sword.sword_intent, sword_intent, "card {card_id}");
    assert_eq!(
        state.p1.sword.frenzy_sword, expected_frenzy,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: CLOUD_DANCE_CASES,
    run: run_cloud_dance_case,
    suite: "replay::tests_synthetic_oracle_wave013::cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds",
    valid: cloud_dance_uses_post_gain_sword_intent_case_table_is_valid,
    cases: [
        cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds_card_1000072 => 0,
        cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds_card_1010072 => 1,
        cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds_card_1020072 => 2,
        cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds_card_1030072 => 3,
        cloud_dance_uses_post_gain_sword_intent_and_exact_realm_thresholds_card_1040072 => 4,
    ]
}

const FRENZY_FIRST_FORM_CASES: &[CardCase] = &[
    CardCase {
        card_id: FRENZY_FIRST_FORM_IDS[0],
    },
    CardCase {
        card_id: FRENZY_FIRST_FORM_IDS[1],
    },
    CardCase {
        card_id: FRENZY_FIRST_FORM_IDS[2],
    },
    CardCase {
        card_id: FRENZY_FIRST_FORM_IDS[3],
    },
    CardCase {
        card_id: FRENZY_FIRST_FORM_IDS[4],
    },
];

fn run_frenzy_first_form_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 400;
    state.p1.core.max_hp = 500;
    state.p1.sword.frenzy_sword = 3;

    state.test_execute_one_card(PlayerSide::P1);

    let drain = 3 * other_param(&card, 1);
    assert_eq!(
        state.p1.core.max_hp,
        500 + other_param(&card, 0),
        "card {card_id}"
    );
    assert_eq!(state.p1.core.hp, 400 + drain, "card {card_id}");
    assert_eq!(
        state.p2.core.hp,
        500 - card.attack.unwrap_or(0) - drain,
        "card {card_id}"
    );
    assert_eq!(state.p1.sword.frenzy_sword, 4, "card {card_id}");
}

crate::data_driven_cases! {
    table: FRENZY_FIRST_FORM_CASES,
    run: run_frenzy_first_form_case,
    suite: "replay::tests_synthetic_oracle_wave013::frenzy_first_form_drains_from_pre_completion_count",
    valid: frenzy_first_form_drains_from_pre_completion_case_table_is_valid,
    cases: [
        frenzy_first_form_drains_from_pre_completion_count_card_1000073 => 0,
        frenzy_first_form_drains_from_pre_completion_count_card_1010073 => 1,
        frenzy_first_form_drains_from_pre_completion_count_card_1020073 => 2,
        frenzy_first_form_drains_from_pre_completion_count_card_1030073 => 3,
        frenzy_first_form_drains_from_pre_completion_count_card_1040073 => 4,
    ]
}

const EARTH_CLIFF_CASES: &[CardCase] = &[
    CardCase {
        card_id: EARTH_CLIFF_IDS[0],
    },
    CardCase {
        card_id: EARTH_CLIFF_IDS[1],
    },
    CardCase {
        card_id: EARTH_CLIFF_IDS[2],
    },
    CardCase {
        card_id: EARTH_CLIFF_IDS[3],
    },
    CardCase {
        card_id: EARTH_CLIFF_IDS[4],
    },
];

fn run_earth_cliff_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 400;
    state.p1.core.max_hp = 500;

    state.test_execute_one_card(PlayerSide::P1);

    let lost = div_ceil(card.defense.unwrap_or(0), 2);
    let realm = original_config::original_card_realm_level(card_id).unwrap();
    let growth = if realm >= 4 { lost } else { 0 };
    assert_eq!(
        state.p1.core.defense,
        card.defense.unwrap_or(0) - lost,
        "card {card_id}"
    );
    assert_eq!(state.p2.core.hp, 500 - lost, "card {card_id}");
    assert_eq!(state.p1.core.max_hp, 500 + growth, "card {card_id}");
    assert_eq!(state.p1.core.hp, 400 + growth, "card {card_id}");
}

crate::data_driven_cases! {
    table: EARTH_CLIFF_CASES,
    run: run_earth_cliff_case,
    suite: "replay::tests_synthetic_oracle_wave013::earth_cliff_high_realms_gain_equal_hp_and_max_hp",
    valid: earth_cliff_high_realms_gain_equal_hp_case_table_is_valid,
    cases: [
        earth_cliff_high_realms_gain_equal_hp_and_max_hp_card_7000091 => 0,
        earth_cliff_high_realms_gain_equal_hp_and_max_hp_card_7010091 => 1,
        earth_cliff_high_realms_gain_equal_hp_and_max_hp_card_7020091 => 2,
        earth_cliff_high_realms_gain_equal_hp_and_max_hp_card_7030091 => 3,
        earth_cliff_high_realms_gain_equal_hp_and_max_hp_card_7040091 => 4,
    ]
}
