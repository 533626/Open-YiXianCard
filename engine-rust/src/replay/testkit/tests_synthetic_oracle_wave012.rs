use super::support::{
    active_neighbor_card, has_active_base_card_in_deck, normalized_base_id, other_param,
    wu_xing_count_in_deck,
};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

const WATER_FIRE_HARMONY_IDS: [i64; 3] = [136, 10_136, 20_136];
const METAL_SPIRIT_METEOR_IDS: [i64; 3] = [143, 10_143, 20_143];
const CONDENSE_INTENT_LIFE_IDS: [i64; 3] = [183, 10_183, 20_183];
const CLOUD_SWORD_EMPTY_CLARITY_IDS: [i64; 3] = [184, 10_184, 20_184];
const FIVE_ELEMENTS_PROTECTION_IDS: [i64; 3] = [197, 10_197, 20_197];
const HEAVEN_TURNING_BRUSH_IDS: [i64; 3] = [200, 10_200, 20_200];
const CRANE_STEP_FLYING_CLOUD_IDS: [i64; 3] = [204, 10_204, 20_204];
const DODGE_AND_SHIFT_IDS: [i64; 3] = [391, 10_391, 20_391];

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn named_card(id: i64, base_id: i64, name: &str, other_params: &[i64]) -> CardDefinition {
    let mut card = basic_attack();
    card.id = id;
    card.base_id = Some(base_id);
    card.name = name.to_string().into();
    card.attack = None;
    card.attack_count = None;
    card.other_params = other_params.to_vec().into();
    card
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

/// One row of the data-driven wave012 card-shape tables: every loop iteration
/// of the migrated tests below runs as an individually named `#[test]` with
/// the verbatim original body and assertions. The row identity is the
/// generated test name suffix (for example `card_136`).
struct CardCase {
    card_id: i64,
}

#[test]
fn wave012_exact_physical_rows_load_and_normalize_to_ten_targets() {
    let rows = [
        WATER_FIRE_HARMONY_IDS.as_slice(),
        METAL_SPIRIT_METEOR_IDS.as_slice(),
        &[169],
        &[180],
        CONDENSE_INTENT_LIFE_IDS.as_slice(),
        CLOUD_SWORD_EMPTY_CLARITY_IDS.as_slice(),
        FIVE_ELEMENTS_PROTECTION_IDS.as_slice(),
        HEAVEN_TURNING_BRUSH_IDS.as_slice(),
        CRANE_STEP_FLYING_CLOUD_IDS.as_slice(),
        DODGE_AND_SHIFT_IDS.as_slice(),
    ]
    .concat();
    assert_eq!(rows.len(), 26);
    let bases: std::collections::BTreeSet<_> = rows
        .into_iter()
        .map(|id| normalized_base_id(&original_card(id)))
        .collect();
    assert_eq!(
        bases,
        [136, 143, 169, 180, 183, 184, 197, 200, 204, 391].into()
    );
}

#[test]
fn stable_active_topology_keeps_current_excludes_locked_and_wraps_in_reverse() {
    let current = original_card(391);
    let locked = original_card(20_391);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([
            current,
            named_card(900_001, 900_001, "火灵测试", &[]),
            named_card(900_002, 900_002, "水灵测试", &[]),
            locked,
        ]),
        3,
    ));
    state.p1.deck.queue.remove(0);
    assert!(has_active_base_card_in_deck(&state.p1, 391));
    assert_eq!(
        active_neighbor_card(&state.p1, 0, 1).unwrap().name,
        "火灵测试"
    );
    assert_eq!(active_neighbor_card(&state.p1, 2, 1).unwrap().id, 391);

    state.p1.fate.reverse_card_direction = 1;
    assert_eq!(
        active_neighbor_card(&state.p1, 0, 1).unwrap().name,
        "水灵测试"
    );

    let locked_only =
        ReplayState::test_from_fixture(&fixture(deck([basic_attack(), original_card(391)]), 1));
    assert!(!has_active_base_card_in_deck(&locked_only.p1, 391));
}

#[test]
fn fate_strategy_cards_follow_three_slot_active_ring_in_both_directions() {
    let mut mixed = named_card(7_000_101, 7_000_101, "混元化灵", &[]);
    mixed.anima = Some(0);
    let water = original_card(7_000_006);
    let metal = original_card(7_000_001);
    let locked_fire = original_card(7_000_009);
    let mut mixed_deck = deck([mixed, water, metal]);
    mixed_deck[7] = locked_fire;

    let mut forward = ReplayState::test_from_fixture(&fixture(mixed_deck.clone(), 3));
    forward.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        forward.p1.elements.activated_elements,
        vec![Element::Metal, Element::Water]
    );

    let mut reverse = ReplayState::test_from_fixture(&fixture(mixed_deck, 3));
    reverse.p1.fate.reverse_card_direction = 1;
    reverse.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        reverse.p1.elements.activated_elements,
        vec![Element::Water, Element::Metal]
    );

    let gaze = named_card(4_000_090, 4_000_090, "望星诀", &[2]);
    let mut forward = ReplayState::test_from_fixture(&fixture(deck([gaze.clone()]), 3));
    forward.p1.astrology.star_slots.clear();
    forward.test_execute_one_card(PlayerSide::P1);
    assert_eq!(forward.p1.astrology.star_slots, vec![1, 2]);

    let mut reverse = ReplayState::test_from_fixture(&fixture(deck([gaze]), 3));
    reverse.p1.astrology.star_slots.clear();
    reverse.p1.fate.reverse_card_direction = 1;
    reverse.test_execute_one_card(PlayerSide::P1);
    assert_eq!(reverse.p1.astrology.star_slots, vec![2, 1]);
}

const WATER_FIRE_HARMONY_CASES: &[CardCase] = &[
    CardCase {
        card_id: WATER_FIRE_HARMONY_IDS[0],
    },
    CardCase {
        card_id: WATER_FIRE_HARMONY_IDS[1],
    },
    CardCase {
        card_id: WATER_FIRE_HARMONY_IDS[2],
    },
];

fn run_water_fire_harmony_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));

    assert!(
        state.test_execute_one_card(PlayerSide::P1),
        "card {card_id}"
    );

    assert_eq!(
        state.p1.elements.water_momentum,
        other_param(&card, 0),
        "card {card_id}"
    );
    assert_eq!(state.p1.elements.activated_water, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_fire, 1, "card {card_id}");
}

crate::data_driven_cases! {
    table: WATER_FIRE_HARMONY_CASES,
    run: run_water_fire_harmony_case,
    suite: "replay::tests_synthetic_oracle_wave012::water_fire_harmony_adds_momentum_activates_both_elements_and_acts_again",
    valid: water_fire_harmony_adds_momentum_case_table_is_valid,
    cases: [
        water_fire_harmony_adds_momentum_activates_both_elements_and_acts_again_card_136 => 0,
        water_fire_harmony_adds_momentum_activates_both_elements_and_acts_again_card_10136 => 1,
        water_fire_harmony_adds_momentum_activates_both_elements_and_acts_again_card_20136 => 2,
    ]
}

#[test]
fn water_fire_harmony_direct_buffs_apply_talent_138_without_generated_callbacks() {
    let card = original_card(136);
    let mut battle = fixture(deck([card.clone()]), 1);
    battle.players.p1.talents = vec![137, 138];
    let mut state = ReplayState::test_from_fixture(&battle);
    let target_hp = state.p2.core.hp;
    let target_max_hp = state.p2.core.max_hp;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.elements.activated_water, 1);
    assert_eq!(state.p1.elements.activated_fire, 1);
    assert_eq!(state.p2.core.hp, target_hp - 6);
    assert_eq!(state.p2.core.max_hp, target_max_hp - 6);
}

const METAL_SPIRIT_METEOR_CASES: &[CardCase] = &[
    CardCase {
        card_id: METAL_SPIRIT_METEOR_IDS[0],
    },
    CardCase {
        card_id: METAL_SPIRIT_METEOR_IDS[1],
    },
    CardCase {
        card_id: METAL_SPIRIT_METEOR_IDS[2],
    },
];

fn run_metal_spirit_meteor_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut active = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    active.p1.core.defense = 48;
    active.lose_defense(PlayerSide::P1, 48);
    active.p1.elements.activated_metal = 1;

    assert!(
        active.test_execute_one_card(PlayerSide::P1),
        "card {card_id}"
    );
    let bonus = 48 / other_param(&card, 0);
    assert_eq!(
        active.p2.core.hp,
        500 - card.attack.unwrap_or(0) - bonus,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: METAL_SPIRIT_METEOR_CASES,
    run: run_metal_spirit_meteor_case,
    suite: "replay::tests_synthetic_oracle_wave012::metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again",
    valid: metal_spirit_meteor_uses_lost_defense_case_table_is_valid,
    cases: [
        metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again_card_143 => 0,
        metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again_card_10143 => 1,
        metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again_card_20143 => 2,
    ]
}

#[test]
fn metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again_inactive_metal() {
    let card = original_card(143);
    let mut inactive = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    inactive.p1.core.defense = 48;
    inactive.lose_defense(PlayerSide::P1, 48);
    assert!(!inactive.test_execute_one_card(PlayerSide::P1));
    assert_eq!(inactive.p2.core.hp, 500 - card.attack.unwrap_or(0) - 6);
}

#[test]
fn metal_spirit_meteor_uses_lost_defense_and_pure_metal_action_again_locked_slot() {
    let locked_unconditional = named_card(7_030_077, 7_030_077, "五行天髓诀·锁定格", &[]);
    let mut locked = ReplayState::test_from_fixture(&fixture(
        deck([original_card(143), locked_unconditional]),
        1,
    ));
    locked.p1.core.defense = 48;
    locked.lose_defense(PlayerSide::P1, 48);
    assert!(!locked.test_execute_one_card(PlayerSide::P1));
}

#[test]
fn wu_xing_count_uses_active_names_special_cards_ke_yin_and_fate_bonus() {
    let cards = deck([
        original_card(169),
        named_card(900_010, 900_010, "金灵水灵测试", &[]),
        named_card(900_011, 900_011, "木灵测试", &[]),
        named_card(900_012, 900_012, "火灵测试", &[]),
        named_card(900_013, 900_013, "土灵测试", &[]),
        named_card(900_014, 292, "幻天印测试", &[0, 2]),
        named_card(900_015, 7_000_101, "元始五行变化测试", &[3]),
        named_card(900_016, 900_016, "金灵重复测试", &[]),
    ]);
    let mut source = fixture(cards, 8);
    source.players.p1.fate_strategies = vec![147];
    source.players.p1.used_ke_yin_cards = vec![40_076, 50_076];
    let mut state = ReplayState::test_from_fixture(&source);

    assert_eq!(wu_xing_count_in_deck(&state.p1), 13);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, 500 - (2 + 13 * 2) * 3);
}

#[test]
fn wu_xing_count_excludes_locked_duplicates_and_uses_last_mirage_value() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([
            named_card(900_020, 292, "火灵测试", &[0, 4]),
            named_card(900_021, 292, "火灵重复测试", &[0, 2]),
            named_card(900_022, 900_022, "金灵锁定测试", &[]),
        ]),
        2,
    ));
    state.p1.deck.queue.remove(0);
    assert_eq!(wu_xing_count_in_deck(&state.p1), 3);
}

#[test]
fn hundred_herbs_cauldron_grants_anima_then_max_hp_and_hp() {
    let card = original_card(180);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 100;
    state.p1.core.max_hp = 200;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, card.anima.unwrap_or(0));
    assert_eq!(state.p1.core.max_hp, 200 + other_param(&card, 0));
    assert_eq!(state.p1.core.hp, 100 + other_param(&card, 0));
}

const CONDENSE_INTENT_LIFE_CASES: &[CardCase] = &[
    CardCase {
        card_id: CONDENSE_INTENT_LIFE_IDS[0],
    },
    CardCase {
        card_id: CONDENSE_INTENT_LIFE_IDS[1],
    },
    CardCase {
        card_id: CONDENSE_INTENT_LIFE_IDS[2],
    },
];

fn run_condense_intent_life_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.core.hp = 100;
    state.p1.core.max_hp = 200;
    state.p1.sword.sword_intent = 2;

    state.test_execute_one_card(PlayerSide::P1);

    let total_intent = 2 + card.sword_intent.unwrap_or(0);
    assert_eq!(state.p1.sword.sword_intent, total_intent, "card {card_id}");
    assert_eq!(
        state.p1.core.max_hp,
        200 + other_param(&card, 1),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.hp,
        100 + total_intent * other_param(&card, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: CONDENSE_INTENT_LIFE_CASES,
    run: run_condense_intent_life_case,
    suite: "replay::tests_synthetic_oracle_wave012::condense_intent_life_adds_sword_intent_once_before_healing",
    valid: condense_intent_life_adds_sword_intent_case_table_is_valid,
    cases: [
        condense_intent_life_adds_sword_intent_once_before_healing_card_183 => 0,
        condense_intent_life_adds_sword_intent_once_before_healing_card_10183 => 1,
        condense_intent_life_adds_sword_intent_once_before_healing_card_20183 => 2,
    ]
}

const CLOUD_SWORD_EMPTY_CLARITY_CASES: &[CardCase] = &[
    CardCase {
        card_id: CLOUD_SWORD_EMPTY_CLARITY_IDS[0],
    },
    CardCase {
        card_id: CLOUD_SWORD_EMPTY_CLARITY_IDS[1],
    },
    CardCase {
        card_id: CLOUD_SWORD_EMPTY_CLARITY_IDS[2],
    },
];

fn run_cloud_sword_empty_clarity_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p1.sword.cloud_chain = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p2.core.hp,
        500 - card.attack.unwrap_or(0) * card.attack_count.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p2.status.weakness,
        other_param(&card, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: CLOUD_SWORD_EMPTY_CLARITY_CASES,
    run: run_cloud_sword_empty_clarity_case,
    suite: "replay::tests_synthetic_oracle_wave012::cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness",
    valid: cloud_sword_empty_clarity_finishes_all_attacks_case_table_is_valid,
    cases: [
        cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness_card_184 => 0,
        cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness_card_10184 => 1,
        cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness_card_20184 => 2,
    ]
}

#[test]
fn cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness_absent_chain() {
    let card = original_card(184);
    let mut absent = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    absent.test_execute_one_card(PlayerSide::P1);
    assert_eq!(absent.p2.status.weakness, 0);
}

#[test]
fn cloud_sword_empty_clarity_finishes_all_attacks_before_cloud_weakness_virtual_cloud() {
    let card = original_card(184);
    let mut virtual_cloud = fixture(deck([card]), 1);
    virtual_cloud.players.p1.talents = vec![14];
    let mut virtual_state = ReplayState::test_from_fixture(&virtual_cloud);
    virtual_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(virtual_state.p2.status.weakness, 2);
}

const FIVE_ELEMENTS_PROTECTION_CASES: &[CardCase] = &[
    CardCase {
        card_id: FIVE_ELEMENTS_PROTECTION_IDS[0],
    },
    CardCase {
        card_id: FIVE_ELEMENTS_PROTECTION_IDS[1],
    },
    CardCase {
        card_id: FIVE_ELEMENTS_PROTECTION_IDS[2],
    },
];

fn run_five_elements_protection_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([card.clone(), named_card(900_030, 900_030, "火灵测试", &[])]),
        2,
    ));
    state.p1.core.defense = 1;
    state.p1.core.guard = 2;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.defense,
        1 + card.defense.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.guard,
        2 + other_param(&card, 0) + 1,
        "card {card_id}"
    );
    assert_eq!(state.p1.elements.activated_fire, 1, "card {card_id}");
}

crate::data_driven_cases! {
    table: FIVE_ELEMENTS_PROTECTION_CASES,
    run: run_five_elements_protection_case,
    suite: "replay::tests_synthetic_oracle_wave012::five_elements_protection_uses_first_use_and_reverse_active_neighbor",
    valid: five_elements_protection_uses_first_use_case_table_is_valid,
    cases: [
        five_elements_protection_uses_first_use_and_reverse_active_neighbor_card_197 => 0,
        five_elements_protection_uses_first_use_and_reverse_active_neighbor_card_10197 => 1,
        five_elements_protection_uses_first_use_and_reverse_active_neighbor_card_20197 => 2,
    ]
}

#[test]
fn five_elements_protection_uses_first_use_and_reverse_active_neighbor_reverse_direction() {
    let card = original_card(197);
    let mut reverse = ReplayState::test_from_fixture(&fixture(
        deck([
            card,
            named_card(900_031, 900_031, "火灵测试", &[]),
            named_card(900_032, 900_032, "水灵金灵测试", &[]),
            named_card(900_033, 900_033, "土灵锁定测试", &[]),
        ]),
        3,
    ));
    reverse.p1.fate.reverse_card_direction = 1;
    reverse.p1.deck.slots[0].used = true;
    reverse.test_execute_one_card(PlayerSide::P1);
    assert_eq!(reverse.p1.core.guard, 1);
    assert_eq!(reverse.p1.elements.activated_metal, 1);
    assert_eq!(reverse.p1.elements.activated_water, 0);
    assert_eq!(reverse.p1.elements.activated_earth, 0);
}

const HEAVEN_TURNING_BRUSH_CASES: &[CardCase] = &[
    CardCase {
        card_id: HEAVEN_TURNING_BRUSH_IDS[0],
    },
    CardCase {
        card_id: HEAVEN_TURNING_BRUSH_IDS[1],
    },
    CardCase {
        card_id: HEAVEN_TURNING_BRUSH_IDS[2],
    },
];

fn run_heaven_turning_brush_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([
            card.clone(),
            named_card(900_040, 900_040, "金灵木灵水灵火灵土灵测试", &[]),
        ]),
        2,
    ));

    assert!(
        state.test_execute_one_card(PlayerSide::P1),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.anima,
        card.anima.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.defense,
        card.defense.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.elements.activated_metal, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_wood, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_water, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_fire, 1, "card {card_id}");
    assert_eq!(state.p1.elements.activated_earth, 1, "card {card_id}");
}

crate::data_driven_cases! {
    table: HEAVEN_TURNING_BRUSH_CASES,
    run: run_heaven_turning_brush_case,
    suite: "replay::tests_synthetic_oracle_wave012::heaven_turning_brush_activates_all_next_name_elements_then_acts_again",
    valid: heaven_turning_brush_activates_all_next_case_table_is_valid,
    cases: [
        heaven_turning_brush_activates_all_next_name_elements_then_acts_again_card_200 => 0,
        heaven_turning_brush_activates_all_next_name_elements_then_acts_again_card_10200 => 1,
        heaven_turning_brush_activates_all_next_name_elements_then_acts_again_card_20200 => 2,
    ]
}

#[test]
fn heaven_turning_brush_activates_all_next_name_elements_then_acts_again_no_element() {
    let mut no_element =
        ReplayState::test_from_fixture(&fixture(deck([original_card(200), basic_attack()]), 2));
    assert!(!no_element.test_execute_one_card(PlayerSide::P1));
}

const CRANE_STEP_FLYING_CLOUD_CASES: &[CardCase] = &[
    CardCase {
        card_id: CRANE_STEP_FLYING_CLOUD_IDS[0],
    },
    CardCase {
        card_id: CRANE_STEP_FLYING_CLOUD_IDS[1],
    },
    CardCase {
        card_id: CRANE_STEP_FLYING_CLOUD_IDS[2],
    },
];

fn run_crane_step_flying_cloud_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    state.p2.core.defense = 100;

    assert!(
        state.test_execute_one_card(PlayerSide::P1),
        "card {card_id}"
    );
    assert_eq!(
        state.p1.core.anima,
        card.anima.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.turn.agility, 0, "card {card_id}");
    let charges = other_param(&card, 0);
    assert_eq!(
        state.p1.turn.ignore_defense_attacks, charges,
        "card {card_id}"
    );

    for _ in 0..charges {
        state.apply_attack(PlayerSide::P1, 5, 0);
    }
    assert_eq!(state.p2.core.hp, 500 - charges * 5, "card {card_id}");
    assert_eq!(state.p2.core.defense, 100, "card {card_id}");
    state.apply_attack(PlayerSide::P1, 5, 0);
    assert_eq!(state.p2.core.hp, 500 - charges * 5, "card {card_id}");
    assert_eq!(state.p2.core.defense, 95, "card {card_id}");
}

crate::data_driven_cases! {
    table: CRANE_STEP_FLYING_CLOUD_CASES,
    run: run_crane_step_flying_cloud_case,
    suite: "replay::tests_synthetic_oracle_wave012::crane_step_grants_resources_acts_again_and_spends_ignore_defense_per_attack",
    valid: crane_step_grants_resources_acts_again_case_table_is_valid,
    cases: [
        crane_step_grants_resources_acts_again_and_spends_ignore_defense_per_attack_card_204 => 0,
        crane_step_grants_resources_acts_again_and_spends_ignore_defense_per_attack_card_10204 => 1,
        crane_step_grants_resources_acts_again_and_spends_ignore_defense_per_attack_card_20204 => 2,
    ]
}

const DODGE_AND_SHIFT_CASES: &[CardCase] = &[
    CardCase {
        card_id: DODGE_AND_SHIFT_IDS[0],
    },
    CardCase {
        card_id: DODGE_AND_SHIFT_IDS[1],
    },
    CardCase {
        card_id: DODGE_AND_SHIFT_IDS[2],
    },
];

fn run_dodge_and_shift_case(case: &CardCase) {
    let card_id = case.card_id;
    let card = original_card(card_id);
    let mut state = ReplayState::test_from_fixture(&fixture(deck([card.clone()]), 1));
    let gain = other_param(&card, 0);

    let acted_again = state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.hp,
        500 - card.hp_cost.unwrap_or(0),
        "card {card_id}"
    );
    assert_eq!(state.p1.core.defense, gain, "card {card_id}");
    assert_eq!(acted_again, gain >= 10, "card {card_id}");
    assert_eq!(
        state.p1.turn.agility,
        if gain >= 10 { gain - 10 } else { gain }
    );
}

crate::data_driven_cases! {
    table: DODGE_AND_SHIFT_CASES,
    run: run_dodge_and_shift_case,
    suite: "replay::tests_synthetic_oracle_wave012::card_391_applies_to_self_and_external_agility_only_while_active",
    valid: card_391_applies_to_self_and_external_case_table_is_valid,
    cases: [
        card_391_applies_to_self_and_external_agility_only_while_active_card_391 => 0,
        card_391_applies_to_self_and_external_agility_only_while_active_card_10391 => 1,
        card_391_applies_to_self_and_external_agility_only_while_active_card_20391 => 2,
    ]
}

#[test]
fn card_391_applies_to_self_and_external_agility_only_while_active_external_gain() {
    let mut external =
        ReplayState::test_from_fixture(&fixture(deck([original_card(204), original_card(391)]), 2));
    external.test_execute_one_card(PlayerSide::P1);
    assert_eq!(external.p1.core.defense, 10);
}

#[test]
fn card_391_applies_to_self_and_external_agility_only_while_active_locked_slot() {
    let mut locked =
        ReplayState::test_from_fixture(&fixture(deck([original_card(204), original_card(391)]), 1));
    locked.test_execute_one_card(PlayerSide::P1);
    assert_eq!(locked.p1.core.defense, 0);
}

#[test]
fn card_391_uses_full_defense_pipeline_once_before_agility_damage() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck([original_card(391), original_card(20_391)]),
        2,
    ));
    state.p1.sword.metal_ring = 2;
    state.p1.turn.adaptation = 1;
    state.p1.turn.agility_gain_damage = 1;
    state.p2.core.defense = 3;

    state.gain_agility(PlayerSide::P1, 3);

    assert_eq!(state.p1.turn.agility, 3);
    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.hp, 500);
}

#[test]
fn card_391_settles_feng_ling_and_spirit_turtle_through_the_shared_pipeline() {
    let mut state = ReplayState::test_from_fixture(&fixture(deck([original_card(391)]), 1));
    state.p1.turn.spirit_turtle_footwork = 2;
    state.p1.fate.feng_ling_zhan_yi = 3;
    state.p1.sword.metal_ring = 1;
    state.p1.turn.adaptation = 1;
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p1.turn.agility_gain_damage = 1;

    state.apply_damage(PlayerSide::P2, 1, false, false, false);

    assert_eq!(state.p1.core.hp, 499);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.turn.agility, 5);
    assert_eq!(state.p1.core.defense, 11);
    assert_eq!(state.p1.fate.feng_ling_zhan_yi, 0);
    assert_eq!(state.p2.core.hp, 490);
}

#[test]
fn card_391_spirit_turtle_settles_between_drain_and_capped_healing() {
    let drain = original_card(9_020_019);
    let mut source = fixture(deck([drain]), 1);
    source.players.p1.initial_anima = 1;
    source.players.p2.cards = deck([original_card(391)]);
    source.players.p2.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.core.hp = 499;
    state.p2.turn.spirit_turtle_footwork = 2;
    state.p2.turn.agility_gain_damage = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 500);
    assert_eq!(state.p2.core.hp, 490);
    assert_eq!(state.p2.turn.agility, 2);
    assert_eq!(state.p2.core.defense, 2);
}

#[test]
fn card_391_nested_cross_player_agility_damage_preserves_call_order_and_terminates() {
    let mut source = fixture(deck([original_card(391)]), 1);
    source.players.p2.cards = deck([original_card(391)]);
    source.players.p2.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.turn.spirit_turtle_footwork = 2;
    state.p1.turn.agility_gain_damage = 1;
    state.p2.turn.spirit_turtle_footwork = 2;
    state.p2.turn.agility_gain_damage = 1;

    state.modify_actor_hp(PlayerSide::P2, -1, false, false);

    assert_eq!(state.p1.core.hp, 498);
    assert_eq!(state.p2.core.hp, 499);
    assert_eq!(state.p1.turn.agility, 2);
    assert_eq!(state.p2.turn.agility, 2);
    assert_eq!(state.p1.core.defense, 2);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.turn.lost_defense_count, 2);
}
