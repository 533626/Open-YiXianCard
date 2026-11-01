use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    super::test_support::test_card(id, base_id, name)
}

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::resize_deck(vec![active], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, None, 1, None)
}

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
    super::test_support::default_fixture(player(p1_cards), player(p2_cards))
}

/// Typed rows for the data-driven verified tables: every loop iteration of
/// the migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `hexagram_7`).
struct CalamityCase {
    hexagram: i64,
}

struct QiHealingCase {
    card_id: i64,
    expected_anima: i64,
    expected_max_hp: i64,
    expected_hp: i64,
}

const HEAVENLY_FATE_CALAMITY_CASES: &[CalamityCase] =
    &[CalamityCase { hexagram: 7 }, CalamityCase { hexagram: 9 }];

fn run_heavenly_fate_calamity_case(case: &CalamityCase) {
    let hexagram = case.hexagram;
    let calamity =
        original_card_definition_by_id(4_000_058).expect("missing heavenly fate calamity");
    let mut off_fixture = fixture(deck(calamity.clone()), deck(basic_attack()));
    off_fixture.players.p1.initial_anima = 2;
    let mut off = ReplayState::test_from_fixture(&off_fixture);
    off.p1.astrology.hexagram = hexagram;
    off.test_execute_one_card(PlayerSide::P1);
    assert_eq!(off.p2.core.hp, 50);
}

crate::data_driven_cases! {
    table: HEAVENLY_FATE_CALAMITY_CASES,
    run: run_heavenly_fate_calamity_case,
    suite: "replay::tests_synthetic_oracle_verified::heavenly_fate_calamity_requires_exactly_eight_hexagram_and_bypasses_guard",
    valid: heavenly_fate_calamity_requires_exactly_case_table_is_valid,
    cases: [
        heavenly_fate_calamity_requires_exactly_eight_hexagram_and_bypasses_guard_hexagram_7 => 0,
        heavenly_fate_calamity_requires_exactly_eight_hexagram_and_bypasses_guard_hexagram_9 => 1,
    ]
}

#[test]
fn heavenly_fate_calamity_requires_exactly_eight_hexagram_and_bypasses_guard_exact_eight() {
    let calamity =
        original_card_definition_by_id(4_000_058).expect("missing heavenly fate calamity");
    let mut exact_fixture = fixture(deck(calamity.clone()), deck(basic_attack()));
    exact_fixture.players.p1.initial_anima = 2;
    exact_fixture.players.p2.initial_guard = 2;
    let mut exact = ReplayState::test_from_fixture(&exact_fixture);
    exact.p1.astrology.hexagram = 8;

    assert!(!exact.test_execute_one_card(PlayerSide::P1));
    assert_eq!(exact.p1.core.anima, 0);
    assert_eq!(exact.p2.core.hp, 8);
    assert_eq!(exact.p2.core.guard, 2);
}

#[test]
fn break_cocoon_clears_every_negative_layer_including_lost_mind() {
    let cocoon = original_card_definition_by_id(10_000_062).expect("missing break cocoon");
    let mut cocoon_fixture = fixture(deck(cocoon), deck(basic_attack()));
    cocoon_fixture.players.p1.initial_agility = 4;
    cocoon_fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 100);
    let mut state = ReplayState::test_from_fixture(&cocoon_fixture);
    state.p1.core.hp = 30;
    state.p1.status.internal_injury = 2;
    state.p1.status.weakness = 1;
    state.p1.status.lost_mind = 2;
    state.p1.status.recovery = 4;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.turn.agility, 9);
    assert_eq!(state.p1.core.physique, 10);
    assert_eq!(state.p1.core.max_hp, 60);
    assert_eq!(state.p1.core.hp, 40);
    assert_eq!(state.negative_status_stack_count(PlayerSide::P1), 0);
    assert_eq!(state.p1.status.recovery, 4);
}

#[test]
fn ultimate_dark_crow_sword_uses_post_gain_anima_for_defense() {
    let crow = original_card_definition_by_id(1_000_097).expect("missing dark crow sword");
    let mut crow_fixture = fixture(deck(crow), deck(basic_attack()));
    crow_fixture.players.p1.initial_anima = 2;
    let mut state = ReplayState::test_from_fixture(&crow_fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.core.defense, 12);
    assert_eq!(state.p2.core.hp, 44);
}

#[test]
fn ultimate_two_polarity_uses_post_gain_total_without_generic_double_gain() {
    let polarity = original_card_definition_by_id(4_000_099).expect("missing two polarity");
    let mut polarity_fixture = fixture(deck(polarity), deck(basic_attack()));
    polarity_fixture.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&polarity_fixture);
    state.p1.core.hp = 30;
    state.p1.astrology.hexagram = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.astrology.hexagram, 5);
    assert_eq!(state.p1.core.defense, 10);
    assert_eq!(state.p1.core.hp, 40);
}

#[test]
fn cloud_sword_evening_glow_reads_chain_before_the_common_cloud_hook() {
    let evening_glow = original_card_definition_by_id(10_047).expect("missing evening glow");
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(evening_glow), deck(basic_attack())));
    state.p1.sword.cloud_chain = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.sword.sword_intent, 10);
    assert_eq!(state.p1.sword.cloud_chain, 4);
}

#[test]
fn cloud_sword_evening_glow_reapplies_printed_intent_for_repeated_bodies() {
    let evening_glow = original_card_definition_by_id(10_047).expect("missing evening glow");
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(evening_glow.clone()), deck(basic_attack())));
    state.p1.sword.cloud_chain = 3;

    state.test_apply_card_effect(PlayerSide::P1, &evening_glow, 0);
    state.test_apply_card_effect(PlayerSide::P1, &evening_glow, 0);

    assert_eq!(state.p1.sword.sword_intent, 22);
    assert_eq!(state.p1.sword.cloud_chain, 5);
}

#[test]
fn azure_dragon_formation_uses_post_gain_total_defense() {
    let formation = original_card_definition_by_id(10_048).expect("missing azure dragon formation");
    let mut formation_fixture = fixture(deck(formation), deck(basic_attack()));
    formation_fixture.players.p1.initial_defense = 5;
    let mut state = ReplayState::test_from_fixture(&formation_fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.defense, 17);
    assert_eq!(state.p1.sword.sword_intent, 5);
    assert_eq!(state.p1.sword.sword_formation_count, 1);
}

#[test]
fn mystic_spirit_counts_its_new_attack_reduction_and_lost_mind_for_agility() {
    let possession = original_card_definition_by_id(336).expect("missing mystic spirit");
    let mut possession_fixture = fixture(deck(possession), deck(basic_attack()));
    possession_fixture.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&possession_fixture);
    state.p1.status.internal_injury = 1;
    state.p1.status.weakness = 2;
    state.p1.status.attack_reduction = 1;
    state.p1.status.lost_mind = 1;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.core.attack_bonus, 3);
    assert_eq!(state.p1.status.attack_reduction, 4);
    assert_eq!(state.p1.turn.agility, 8);
}

const DREAM_QI_HEALING_CASES: &[QiHealingCase] = &[
    QiHealingCase {
        card_id: 4_000_083,
        expected_anima: 6,
        expected_max_hp: 110,
        expected_hp: 32,
    },
    QiHealingCase {
        card_id: 4_010_083,
        expected_anima: 6,
        expected_max_hp: 110,
        expected_hp: 34,
    },
    QiHealingCase {
        card_id: 4_020_083,
        expected_anima: 6,
        expected_max_hp: 110,
        expected_hp: 36,
    },
    QiHealingCase {
        card_id: 4_030_083,
        expected_anima: 6,
        expected_max_hp: 115,
        expected_hp: 51,
    },
    QiHealingCase {
        card_id: 4_040_083,
        expected_anima: 7,
        expected_max_hp: 120,
        expected_hp: 53,
    },
];

fn run_dream_qi_healing_case(case: &QiHealingCase) {
    let card_id = case.card_id;
    let expected_anima = case.expected_anima;
    let expected_max_hp = case.expected_max_hp;
    let expected_hp = case.expected_hp;
    let healing = original_card_definition_by_id(card_id)
        .unwrap_or_else(|| panic!("missing dream qi healing {card_id}"));
    let mut healing_fixture = fixture(deck(healing), deck(basic_attack()));
    healing_fixture.players.p1.base_max_hp = 100;
    healing_fixture.players.p1.initial_anima = 4;
    let mut state = ReplayState::test_from_fixture(&healing_fixture);
    state.p1.core.hp = 30;
    state.p1.astrology.star_power = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, expected_anima, "card {card_id}");
    assert_eq!(state.p1.core.max_hp, expected_max_hp, "card {card_id}");
    assert_eq!(state.p1.core.hp, expected_hp, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_QI_HEALING_CASES,
    run: run_dream_qi_healing_case,
    suite: "replay::tests_synthetic_oracle_verified::dream_qi_healing_uses_each_card_realm_and_post_gain_resources",
    valid: dream_qi_healing_uses_each_card_realm_case_table_is_valid,
    cases: [
        dream_qi_healing_uses_each_card_realm_and_post_gain_resources_card_4000083 => 0,
        dream_qi_healing_uses_each_card_realm_and_post_gain_resources_card_4010083 => 1,
        dream_qi_healing_uses_each_card_realm_and_post_gain_resources_card_4020083 => 2,
        dream_qi_healing_uses_each_card_realm_and_post_gain_resources_card_4030083 => 3,
        dream_qi_healing_uses_each_card_realm_and_post_gain_resources_card_4040083 => 4,
    ]
}

#[test]
fn dream_qi_healing_uses_three_parameter_shape_without_realm_metadata() {
    let mut healing = card(90_000_083, 4_000_083, "梦•气疗术");
    healing.anima = Some(2);
    healing.other_params = vec![15, 2, 3].into();
    let mut healing_fixture = fixture(deck(healing), deck(basic_attack()));
    healing_fixture.players.p1.base_max_hp = 75;
    let mut state = ReplayState::test_from_fixture(&healing_fixture);
    state.p1.core.hp = 66;
    state.p1.astrology.star_power = 1;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.core.max_hp, 90);
    assert_eq!(state.p1.core.hp, 73);
}

#[test]
fn lost_mind_is_visible_in_snapshots_and_detail_entries() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.add_actor_negative_status(PlayerSide::P1, 393, 2);

    assert_eq!(state.negative_status_stack_count(PlayerSide::P1), 2);
    assert_eq!(state.p1.snapshot().lost_mind, 2);
    assert!(state
        .p1
        .detail_entries()
        .iter()
        .any(|entry| entry.key == "lostMind" && entry.value == 2));

    state.reduce_all_actor_negative_statuses(PlayerSide::P1, i64::MAX);
    assert_eq!(state.p1.status.lost_mind, 0);
}
