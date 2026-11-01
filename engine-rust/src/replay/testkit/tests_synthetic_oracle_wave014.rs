use super::support::{normalized_base_id, other_param};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};
use std::collections::{BTreeMap, BTreeSet};

const GIANT_PENG_IDS: [i64; 5] = [1_000_081, 1_010_081, 1_020_081, 1_030_081, 1_040_081];
const EARTH_FIEND_IDS: [i64; 5] = [1_000_082, 1_010_082, 1_020_082, 1_030_082, 1_040_082];
const FALLING_FLOWERS_IDS: [i64; 5] = [4_000_074, 4_010_074, 4_020_074, 4_030_074, 4_040_074];
const METAL_EDGE_IDS: [i64; 5] = [7_000_085, 7_010_085, 7_020_085, 7_030_085, 7_040_085];
const WATER_SURGING_IDS: [i64; 5] = [7_000_103, 7_010_103, 7_020_103, 7_030_103, 7_040_103];
const ASURA_ROAR_IDS: [i64; 5] = [10_000_070, 10_010_070, 10_020_070, 10_030_070, 10_040_070];
const SOARING_WAVES_IDS: [i64; 5] = [10_000_073, 10_010_073, 10_020_073, 10_030_073, 10_040_073];
const BENG_QUAN_RUSH_IDS: [i64; 5] = [10_000_074, 10_010_074, 10_020_074, 10_030_074, 10_040_074];
const MOUNTAIN_PALM_IDS: [i64; 5] = [10_000_079, 10_010_079, 10_020_079, 10_030_079, 10_040_079];
const MAJESTIC_MOMENTUM_IDS: [i64; 5] =
    [10_000_086, 10_010_086, 10_020_086, 10_030_086, 10_040_086];

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
    let mut permanent_buff_temp_datas = BTreeMap::new();
    permanent_buff_temp_datas.insert("10024".to_string(), 100);
    FixturePlayer {
        level: 5,
        base_max_hp: 500,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        xian_mo_strategies: Vec::new(),
        xian_mo_temp_datas: Default::default(),
        active_slot_count,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas,
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

fn fixture(p1_cards: Vec<CardDefinition>, active_slot_count: usize) -> BattleFixture {
    super::test_support::default_fixture(
        player(p1_cards, active_slot_count),
        player(deck([basic_attack()]), 1),
    )
}

fn state_for(card: &CardDefinition) -> ReplayState {
    ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1))
}

fn realm(card_id: i64) -> i64 {
    original_config::original_card_realm_level(card_id)
        .unwrap_or_else(|| panic!("missing realm for card {card_id}"))
}

/// One row of the data-driven wave014 card-shape tables: every loop iteration
/// of the migrated tests below runs as an individually named `#[test]` with
/// the verbatim original body and assertions. The row identity is the
/// generated test name suffix (for example `card_1000081`).
struct CardCase {
    card_id: i64,
}

#[test]
fn wave014_loads_fifty_physical_rows_for_ten_logical_targets() {
    let families = [
        GIANT_PENG_IDS.as_slice(),
        EARTH_FIEND_IDS.as_slice(),
        FALLING_FLOWERS_IDS.as_slice(),
        METAL_EDGE_IDS.as_slice(),
        WATER_SURGING_IDS.as_slice(),
        ASURA_ROAR_IDS.as_slice(),
        SOARING_WAVES_IDS.as_slice(),
        BENG_QUAN_RUSH_IDS.as_slice(),
        MOUNTAIN_PALM_IDS.as_slice(),
        MAJESTIC_MOMENTUM_IDS.as_slice(),
    ];
    let rows = families.concat();
    assert_eq!(rows.len(), 50);
    let bases: BTreeSet<_> = rows
        .iter()
        .map(|id| normalized_base_id(&original_card(*id)))
        .collect();
    assert_eq!(
        bases,
        [
            1_000_081, 1_000_082, 4_000_074, 7_000_085, 7_000_103, 10_000_070, 10_000_073,
            10_000_074, 10_000_079, 10_000_086,
        ]
        .into()
    );
    for family in families {
        for (index, card_id) in family.iter().enumerate() {
            assert_eq!(realm(*card_id), index as i64 + 1, "card {card_id}");
        }
    }
}

const GIANT_PENG_CASES: &[CardCase] = &[
    CardCase {
        card_id: GIANT_PENG_IDS[0],
    },
    CardCase {
        card_id: GIANT_PENG_IDS[1],
    },
    CardCase {
        card_id: GIANT_PENG_IDS[2],
    },
    CardCase {
        card_id: GIANT_PENG_IDS[3],
    },
    CardCase {
        card_id: GIANT_PENG_IDS[4],
    },
];

fn run_giant_peng_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.anima = 2;

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    let attack_count = card.attack_count.unwrap_or(1).max(0);
    assert_eq!(
        state.p2.core.hp,
        500 - card.attack.unwrap_or(0) * attack_count,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.anima,
        2 + card.anima.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(action_again, realm(card_id) >= 4, "card {card_id}");
    assert_eq!(state.p1.turn.agility, 0, "card {card_id}");
}

crate::data_driven_cases! {
    table: GIANT_PENG_CASES,
    run: run_giant_peng_case,
    suite: "replay::tests_synthetic_oracle_wave014::giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility",
    valid: giant_peng_attacks_then_uses_post_gain_anima_case_table_is_valid,
    cases: [
        giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_card_1000081 => 0,
        giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_card_1010081 => 1,
        giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_card_1020081 => 2,
        giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_card_1030081 => 3,
        giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_card_1040081 => 4,
    ]
}

#[test]
fn giant_peng_attacks_then_uses_post_gain_anima_for_high_realm_agility_equality_guard() {
    let card = original_card(1_030_081);
    let mut equality = state_for(&card);
    assert!(!equality.test_execute_one_card(PlayerSide::P1));
    assert_eq!(equality.p1.core.anima, card.anima.unwrap_or(0));
    assert_eq!(equality.p1.turn.agility, 0);
}

const EARTH_FIEND_CASES: &[CardCase] = &[
    CardCase {
        card_id: EARTH_FIEND_IDS[0],
    },
    CardCase {
        card_id: EARTH_FIEND_IDS[1],
    },
    CardCase {
        card_id: EARTH_FIEND_IDS[2],
    },
    CardCase {
        card_id: EARTH_FIEND_IDS[3],
    },
    CardCase {
        card_id: EARTH_FIEND_IDS[4],
    },
];

fn run_earth_fiend_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.attack_bonus = 10;

    state.test_execute_one_card(PlayerSide::P1);

    let actual_damage = (card.attack.unwrap_or(0) + 10) * card.attack_count.unwrap_or(1).max(0);
    let uncapped = actual_damage / other_param(&card, 1).max(1);
    let expected = if realm(card_id) <= 4 {
        uncapped.min(other_param(&card, 0).max(0))
    } else {
        uncapped
    };
    assert_eq!(state.p2.core.hp, 500 - actual_damage, "card {card_id}");
    assert_eq!(state.p2.status.internal_injury, expected, "card {card_id}");
    assert_eq!(state.active_effect_actual_damage(), 0, "card {card_id}");
    assert_eq!(state.active_effect_wounded_count(), 0, "card {card_id}");
}

crate::data_driven_cases! {
    table: EARTH_FIEND_CASES,
    run: run_earth_fiend_case,
    suite: "replay::tests_synthetic_oracle_wave014::earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four",
    valid: earth_fiend_reads_this_card_attack_ledger_case_table_is_valid,
    cases: [
        earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_card_1000082 => 0,
        earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_card_1010082 => 1,
        earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_card_1020082 => 2,
        earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_card_1030082 => 3,
        earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_card_1040082 => 4,
    ]
}

#[test]
fn earth_fiend_reads_this_card_attack_ledger_caps_only_realms_one_through_four_blocked() {
    let card = original_card(1_040_082);
    let mut blocked = state_for(&card);
    blocked.p2.core.defense = 100;
    blocked.test_execute_one_card(PlayerSide::P1);
    assert_eq!(blocked.p2.core.hp, 500);
    assert_eq!(blocked.p2.status.internal_injury, 0);
}

const FALLING_FLOWERS_CASES: &[CardCase] = &[
    CardCase {
        card_id: FALLING_FLOWERS_IDS[0],
    },
    CardCase {
        card_id: FALLING_FLOWERS_IDS[1],
    },
    CardCase {
        card_id: FALLING_FLOWERS_IDS[2],
    },
    CardCase {
        card_id: FALLING_FLOWERS_IDS[3],
    },
    CardCase {
        card_id: FALLING_FLOWERS_IDS[4],
    },
];

fn run_falling_flowers_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.hp_mutation.add_hp_count = 10;
    state.p1.deck.slots[0].used = true;

    state.test_execute_one_card(PlayerSide::P1);

    let healing = other_param(&card, 1).max(0);
    let rear_injury = if realm(card_id) >= 4 {
        (10 + healing) / other_param(&card, 2).max(1)
    } else {
        0
    };
    assert_eq!(state.p1.core.hp, 400 + healing, "card {card_id}");
    assert_eq!(
        state.p1.hp_mutation.add_hp_count,
        10 + healing,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.dream_mirage.turn_hp_gained, healing,
        "card {card_id}"
    );
    assert_eq!(
        state.p2.status.internal_injury,
        other_param(&card, 0).max(0) + rear_injury,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: FALLING_FLOWERS_CASES,
    run: run_falling_flowers_case,
    suite: "replay::tests_synthetic_oracle_wave014::falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger",
    valid: falling_flowers_orders_high_realm_healing_case_table_is_valid,
    cases: [
        falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger_card_4000074 => 0,
        falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger_card_4010074 => 1,
        falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger_card_4020074 => 2,
        falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger_card_4030074 => 3,
        falling_flowers_orders_high_realm_healing_before_rear_move_and_reads_ledger_card_4040074 => 4,
    ]
}

#[test]
fn falling_flowers_gates_low_healing_and_counts_requested_high_realm_overheal() {
    let low_card = original_card(4_020_074);
    let mut low = state_for(&low_card);
    low.p1.core.hp = 400;
    low.test_execute_one_card(PlayerSide::P1);
    assert_eq!(low.p1.core.hp, 400);
    assert_eq!(low.p2.status.internal_injury, other_param(&low_card, 0));

    let high_card = original_card(4_030_074);
    let mut high = state_for(&high_card);
    high.p1.deck.slots[0].used = true;
    high.test_execute_one_card(PlayerSide::P1);
    assert_eq!(high.p1.core.hp, 500);
    assert_eq!(high.p1.hp_mutation.add_hp_count, 5);
    assert_eq!(high.p1.dream_mirage.turn_hp_gained, 5);
    assert_eq!(high.p1.dream_mirage.hp_gain_event_count, 1);
    assert_eq!(high.p2.status.internal_injury, 2);
}

const METAL_EDGE_CASES: &[CardCase] = &[
    CardCase {
        card_id: METAL_EDGE_IDS[0],
    },
    CardCase {
        card_id: METAL_EDGE_IDS[1],
    },
    CardCase {
        card_id: METAL_EDGE_IDS[2],
    },
    CardCase {
        card_id: METAL_EDGE_IDS[3],
    },
    CardCase {
        card_id: METAL_EDGE_IDS[4],
    },
];

fn run_metal_edge_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.attack_bonus = 4;

    state.test_execute_one_card(PlayerSide::P1);

    let actual_damage = card.attack.unwrap_or(0) + 4;
    let parameter = other_param(&card, 0);
    let expected_water = if realm(card_id) <= 3 {
        parameter
    } else {
        actual_damage / parameter.max(1)
    };
    assert_eq!(state.p2.core.hp, 500 - actual_damage, "card {card_id}");
    assert_eq!(state.p1.core.anima, card.anima.unwrap_or(0));
    assert_eq!(
        state.p1.elements.water_momentum, expected_water,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: METAL_EDGE_CASES,
    run: run_metal_edge_case,
    suite: "replay::tests_synthetic_oracle_wave014::metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage",
    valid: metal_edge_attacks_before_anima_case_table_is_valid,
    cases: [
        metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_card_7000085 => 0,
        metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_card_7010085 => 1,
        metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_card_7020085 => 2,
        metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_card_7030085 => 3,
        metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_card_7040085 => 4,
    ]
}

#[test]
fn metal_edge_attacks_before_anima_and_converts_only_positive_actual_damage_blocked() {
    let card = original_card(7_040_085);
    let mut blocked = state_for(&card);
    blocked.p2.core.defense = 100;
    blocked.test_execute_one_card(PlayerSide::P1);
    assert_eq!(blocked.p2.core.hp, 500);
    assert_eq!(blocked.p1.core.anima, card.anima.unwrap_or(0));
    assert_eq!(blocked.p1.elements.water_momentum, 0);
}

const WATER_SURGING_CASES: &[CardCase] = &[
    CardCase {
        card_id: WATER_SURGING_IDS[0],
    },
    CardCase {
        card_id: WATER_SURGING_IDS[1],
    },
    CardCase {
        card_id: WATER_SURGING_IDS[2],
    },
    CardCase {
        card_id: WATER_SURGING_IDS[3],
    },
    CardCase {
        card_id: WATER_SURGING_IDS[4],
    },
];

fn run_water_surging_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.anima = 4;
    state.p1.elements.water_momentum = 3;

    state.test_execute_one_card(PlayerSide::P1);

    let attack = card.attack.unwrap_or(0) + 7;
    let growth = if realm(card_id) >= 4 {
        attack / other_param(&card, 0).max(1)
    } else {
        0
    };
    assert_eq!(state.p2.core.hp, 500 - attack, "card {card_id}");
    assert_eq!(state.p1.core.max_hp, 500 + growth, "card {card_id}");
    assert_eq!(state.p1.core.hp, 400 + growth, "card {card_id}");
}

crate::data_driven_cases! {
    table: WATER_SURGING_CASES,
    run: run_water_surging_case,
    suite: "replay::tests_synthetic_oracle_wave014::water_surging_snapshots_anima_and_water_then_grows_from_actual_damage",
    valid: water_surging_snapshots_anima_and_water_case_table_is_valid,
    cases: [
        water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_card_7000103 => 0,
        water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_card_7010103 => 1,
        water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_card_7020103 => 2,
        water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_card_7030103 => 3,
        water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_card_7040103 => 4,
    ]
}

#[test]
fn water_surging_snapshots_anima_and_water_then_grows_from_actual_damage_blocked() {
    let card = original_card(7_040_103);
    let mut blocked = state_for(&card);
    blocked.p1.core.hp = 400;
    blocked.p2.core.defense = 100;
    blocked.test_execute_one_card(PlayerSide::P1);
    assert_eq!(blocked.p2.core.hp, 500);
    assert_eq!(blocked.p1.core.max_hp, 500);
    assert_eq!(blocked.p1.core.hp, 400);
}

const ASURA_ROAR_CASES: &[CardCase] = &[
    CardCase {
        card_id: ASURA_ROAR_IDS[0],
    },
    CardCase {
        card_id: ASURA_ROAR_IDS[1],
    },
    CardCase {
        card_id: ASURA_ROAR_IDS[2],
    },
    CardCase {
        card_id: ASURA_ROAR_IDS[3],
    },
    CardCase {
        card_id: ASURA_ROAR_IDS[4],
    },
];

fn run_asura_roar_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.physique = 20;
    state.p1.core.physique_limit = 100;

    state.test_execute_one_card(PlayerSide::P1);

    let physique = 20 + card.physique.unwrap_or(0);
    let internal_injury = if realm(card_id) <= 3 {
        other_param(&card, 1)
    } else {
        physique / other_param(&card, 1).max(1)
    };
    assert_eq!(state.p1.core.physique, physique, "card {card_id}");
    assert_eq!(
        state.p1.core.max_hp,
        500 + card.physique.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.hp,
        400 + other_param(&card, 0),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.status.internal_injury, internal_injury,
        "card {card_id}"
    );
    assert_eq!(
        state.p2.status.internal_injury, internal_injury,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: ASURA_ROAR_CASES,
    run: run_asura_roar_case,
    suite: "replay::tests_synthetic_oracle_wave014::asura_roar_uses_post_gain_physique_for_bilateral_internal_injury",
    valid: asura_roar_uses_post_gain_physique_case_table_is_valid,
    cases: [
        asura_roar_uses_post_gain_physique_for_bilateral_internal_injury_card_10000070 => 0,
        asura_roar_uses_post_gain_physique_for_bilateral_internal_injury_card_10010070 => 1,
        asura_roar_uses_post_gain_physique_for_bilateral_internal_injury_card_10020070 => 2,
        asura_roar_uses_post_gain_physique_for_bilateral_internal_injury_card_10030070 => 3,
        asura_roar_uses_post_gain_physique_for_bilateral_internal_injury_card_10040070 => 4,
    ]
}

#[test]
fn asura_roar_routes_physique_and_healing_through_adaptation() {
    let card = original_card(10_000_070);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.physique_limit = 100;
    state.p1.turn.adaptation = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.turn.battle_physique_gain_count, 2);
    assert_eq!(state.p1.core.max_hp, 503);
    assert_eq!(state.p1.core.hp, 406);
}

const SOARING_WAVES_CASES: &[CardCase] = &[
    CardCase {
        card_id: SOARING_WAVES_IDS[0],
    },
    CardCase {
        card_id: SOARING_WAVES_IDS[1],
    },
    CardCase {
        card_id: SOARING_WAVES_IDS[2],
    },
    CardCase {
        card_id: SOARING_WAVES_IDS[3],
    },
    CardCase {
        card_id: SOARING_WAVES_IDS[4],
    },
];

fn run_soaring_waves_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.anima = 3;

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    let post_anima = 3 + card.anima.unwrap_or(0);
    let agility = other_param(&card, 1) + if realm(card_id) >= 4 { post_anima } else { 0 };
    assert_eq!(state.p1.core.anima, post_anima, "card {card_id}");
    assert_eq!(
        state.p1.core.hp,
        400 + other_param(&card, 0),
        "card {card_id}"
    );
    assert_eq!(action_again, agility >= 10, "card {card_id}");
    assert_eq!(
        state.p1.turn.agility,
        agility - if action_again { 10 } else { 0 },
        "card {card_id}"
    );
    assert!(state.p1.deck.slots[0].skipped, "card {card_id}");
}

crate::data_driven_cases! {
    table: SOARING_WAVES_CASES,
    run: run_soaring_waves_case,
    suite: "replay::tests_synthetic_oracle_wave014::soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle",
    valid: soaring_waves_uses_post_gain_anima_case_table_is_valid,
    cases: [
        soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle_card_10000073 => 0,
        soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle_card_10010073 => 1,
        soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle_card_10020073 => 2,
        soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle_card_10030073 => 3,
        soaring_waves_uses_post_gain_anima_for_agility_and_common_consume_lifecycle_card_10040073 => 4,
    ]
}

const BENG_QUAN_RUSH_CASES: &[CardCase] = &[
    CardCase {
        card_id: BENG_QUAN_RUSH_IDS[0],
    },
    CardCase {
        card_id: BENG_QUAN_RUSH_IDS[1],
    },
    CardCase {
        card_id: BENG_QUAN_RUSH_IDS[2],
    },
    CardCase {
        card_id: BENG_QUAN_RUSH_IDS[3],
    },
    CardCase {
        card_id: BENG_QUAN_RUSH_IDS[4],
    },
];

fn run_beng_quan_rush_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.turn.agility = 6;
    state.p1.beng.beng_quan_chuo = 3;

    state.test_execute_one_card(PlayerSide::P1);

    let extra_attack = if realm(card_id) <= 4 {
        other_param(&card, 0)
    } else {
        6 / other_param(&card, 0).max(1)
    };
    assert_eq!(
        state.p1.core.hp,
        500 - card.hp_cost.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p2.core.hp,
        500 - card.attack.unwrap_or(0) - extra_attack - 3,
        "card {card_id}"
    );
    assert_eq!(state.p1.beng.beng_quan_chuo, extra_attack, "card {card_id}");
    assert_eq!(state.p1.beng.consumed_beng_quan_chuo, 0);
}

crate::data_driven_cases! {
    table: BENG_QUAN_RUSH_CASES,
    run: run_beng_quan_rush_case,
    suite: "replay::tests_synthetic_oracle_wave014::beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo",
    valid: beng_quan_rush_pays_hp_cost_case_table_is_valid,
    cases: [
        beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo_card_10000074 => 0,
        beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo_card_10010074 => 1,
        beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo_card_10020074 => 2,
        beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo_card_10030074 => 3,
        beng_quan_rush_pays_hp_cost_consumes_old_chuo_and_keeps_new_chuo_card_10040074 => 4,
    ]
}

#[test]
fn beng_quan_rush_sees_post_cost_momentum_hook_but_realm_five_requires_agility() {
    let low_card = original_card(10_000_074);
    let mut low = state_for(&low_card);
    low.p1.beng.beng_mei_mindset = 1;
    low.test_execute_one_card(PlayerSide::P1);
    assert_eq!(low.p1.core.hp, 500 - low_card.hp_cost.unwrap_or(0));
    assert_eq!(low.p1.beng.beng_quan_chuo, 2);
    assert_eq!(low.p1.beng.momentum, 0);
    assert_eq!(low.p2.core.hp, 487);

    let high_card = original_card(10_040_074);
    let mut high = state_for(&high_card);
    high.p1.beng.momentum = 1;
    high.test_execute_one_card(PlayerSide::P1);
    assert_eq!(high.p1.beng.beng_quan_chuo, 0);
    assert_eq!(high.p1.beng.momentum, 0);
    assert_eq!(
        high.p2.core.hp,
        500 - high_card.attack.unwrap_or(0) * 110 / 100
    );
}

#[test]
fn beng_quan_rush_hp_cost_physique_uses_adaptation_max_hp_pipeline() {
    let card = original_card(10_000_074);
    let mut state = state_for(&card);
    state.p1.core.physique_limit = 100;
    state.p1.turn.adaptation = 1;
    state.p1.identity.fate_strategies.push(149);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.turn.battle_physique_gain_count, 1);
    assert_eq!(state.p1.core.max_hp, 502);
    assert_eq!(state.p1.core.hp, 500 - card.hp_cost.unwrap_or(0));
}

const MOUNTAIN_PALM_CASES: &[CardCase] = &[
    CardCase {
        card_id: MOUNTAIN_PALM_IDS[0],
    },
    CardCase {
        card_id: MOUNTAIN_PALM_IDS[1],
    },
    CardCase {
        card_id: MOUNTAIN_PALM_IDS[2],
    },
    CardCase {
        card_id: MOUNTAIN_PALM_IDS[3],
    },
    CardCase {
        card_id: MOUNTAIN_PALM_IDS[4],
    },
];

fn run_mountain_palm_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.physique = 30;
    state.p1.core.physique_limit = 100;
    state.p1.turn.battle_physique_gain_count = 30;

    state.test_execute_one_card(PlayerSide::P1);

    let attack = card.attack.unwrap_or(0) + 30 / other_param(&card, 1).max(1);
    let gained_momentum = if realm(card_id) >= 4 {
        30 / other_param(&card, 0).max(1)
    } else {
        0
    };
    let damage = attack * (100 + gained_momentum * 10) / 100;
    assert_eq!(state.p1.core.hp, 498, "card {card_id}");
    assert_eq!(state.p2.core.hp, 500 - damage, "card {card_id}");
    assert_eq!(
        state.p1.beng.momentum,
        (gained_momentum - i64::from(gained_momentum > 0)).max(0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: MOUNTAIN_PALM_CASES,
    run: run_mountain_palm_case,
    suite: "replay::tests_synthetic_oracle_wave014::mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack",
    valid: mountain_palm_reads_gain_counter_case_table_is_valid,
    cases: [
        mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack_card_10000079 => 0,
        mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack_card_10010079 => 1,
        mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack_card_10020079 => 2,
        mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack_card_10030079 => 3,
        mountain_palm_reads_gain_counter_then_high_realms_add_momentum_before_attack_card_10040079 => 4,
    ]
}

#[test]
fn mountain_palm_keeps_current_physique_separate_from_battle_gain_counter() {
    let card = original_card(10_040_079);
    let mut counter_only = state_for(&card);
    counter_only.p1.turn.battle_physique_gain_count = 30;
    counter_only.test_execute_one_card(PlayerSide::P1);
    assert_eq!(counter_only.p2.core.hp, 463);
    assert_eq!(counter_only.p1.beng.momentum, 0);

    let mut physique_only = state_for(&card);
    physique_only.p1.core.physique = 30;
    physique_only.p1.core.physique_limit = 100;
    physique_only.test_execute_one_card(PlayerSide::P1);
    assert_eq!(physique_only.p2.core.hp, 492);
    assert_eq!(physique_only.p1.beng.momentum, 1);
}

#[test]
fn mountain_palm_momentum_overflow_uses_defense_and_sustain_damage_hooks() {
    let card = original_card(10_040_079);
    let mut state = state_for(&card);
    state.p1.core.physique = 30;
    state.p1.core.physique_limit = 100;
    state.p1.turn.battle_physique_gain_count = 30;
    state.p1.beng.momentum_limit = 1;
    state.p1.fate.sheng_qi_ling_ren = 2;

    state.test_execute_one_card(PlayerSide::P1);

    let attack = card.attack.unwrap_or(0) + 30 / other_param(&card, 1).max(1);
    let attack_damage = attack * 110 / 100;
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p2.core.hp, 500 - attack_damage - 4);
}

#[test]
fn momentum_sustain_damage_enters_the_ordinary_damage_mitigation_kernel() {
    let card = original_card(10_040_079);
    let mut state = state_for(&card);
    state.p1.fate.sheng_qi_ling_ren = 10;
    state.p2.elements.metal_iron_bone = 1;

    state.apply_sheng_qi_ling_ren_damage(PlayerSide::P1, 1);

    assert_eq!(state.p2.core.hp, 495);
}

const MAJESTIC_MOMENTUM_CASES: &[CardCase] = &[
    CardCase {
        card_id: MAJESTIC_MOMENTUM_IDS[0],
    },
    CardCase {
        card_id: MAJESTIC_MOMENTUM_IDS[1],
    },
    CardCase {
        card_id: MAJESTIC_MOMENTUM_IDS[2],
    },
    CardCase {
        card_id: MAJESTIC_MOMENTUM_IDS[3],
    },
    CardCase {
        card_id: MAJESTIC_MOMENTUM_IDS[4],
    },
];

fn run_majestic_momentum_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.physique_limit = 100;
    state.p1.core.anima = 1;
    state.p1.beng.momentum = 3;

    state.test_execute_one_card(PlayerSide::P1);

    let high_realm = realm(card_id) >= 4;
    let physique_gain = if high_realm {
        3 * other_param(&card, 1)
    } else {
        other_param(&card, 0)
    };
    let hp_gain = if high_realm {
        3 * other_param(&card, 2)
    } else {
        other_param(&card, 1)
    };
    assert_eq!(state.p1.core.physique, physique_gain, "card {card_id}");
    assert_eq!(state.p1.core.max_hp, 500 + physique_gain, "card {card_id}");
    assert_eq!(state.p1.core.hp, 400 + hp_gain, "card {card_id}");
    assert_eq!(
        state.p1.turn.agility,
        if high_realm { other_param(&card, 3) } else { 0 },
        "card {card_id}"
    );
    assert_eq!(
        state.p1.beng.momentum,
        if high_realm { 0 } else { 3 },
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: MAJESTIC_MOMENTUM_CASES,
    run: run_majestic_momentum_case,
    suite: "replay::tests_synthetic_oracle_wave014::majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches",
    valid: majestic_momentum_ignores_metadata_physique_case_table_is_valid,
    cases: [
        majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_card_10000086 => 0,
        majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_card_10010086 => 1,
        majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_card_10020086 => 2,
        majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_card_10030086 => 3,
        majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_card_10040086 => 4,
    ]
}

#[test]
fn majestic_momentum_ignores_metadata_physique_and_uses_exact_realm_branches_inactive() {
    let card = original_card(10_000_086);
    let mut inactive = state_for(&card);
    inactive.p1.core.hp = 400;
    inactive.test_execute_one_card(PlayerSide::P1);
    assert_eq!(inactive.p1.core.physique, 0);
    assert_eq!(inactive.p1.core.max_hp, 500);
    assert_eq!(inactive.p1.core.hp, 400);
}

#[test]
fn majestic_momentum_exhausts_through_common_modifier_but_scales_from_captured_value() {
    let card = original_card(10_040_086);
    let mut state = state_for(&card);
    state.p1.core.hp = 400;
    state.p1.core.physique_limit = 100;
    state.p1.beng.momentum = 3;
    state.p1.beng.unceasing_momentum = 2;
    state.p1.fate.sheng_qi_ling_ren = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.beng.momentum, 2);
    assert_eq!(state.p2.core.hp, 497);
    assert_eq!(state.p1.core.physique, 6);
    assert_eq!(state.p1.core.max_hp, 506);
    assert_eq!(state.p1.core.hp, 409);
    assert_eq!(state.p1.turn.agility, 5);
}
