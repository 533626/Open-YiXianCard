// Covers all seven admitted batch-008 bodies.
use super::cards_dream_mirage::DreamMirageValue;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::resize_deck(vec![active], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 100, None, 1, None)
}

fn fixture(active: CardDefinition) -> BattleFixture {
    super::test_support::default_fixture(player(deck(active)), player(deck(basic_attack())))
}

/// Typed rows for the data-driven pilot tables: every loop iteration of the
/// migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `card_1000071`).
struct AttackSegmentsCase {
    card_id: i64,
    attack: i64,
    segments: i64,
}

struct RhinoCase {
    card_id: i64,
    printed_defense: i64,
    expected_anima: i64,
}

struct DryHexagramCase {
    card_id: i64,
    hexagram: i64,
    expected_action_again: bool,
}

struct CloudDancePilotCase {
    card_id: i64,
    anima: i64,
    defense: i64,
    sword_intent: i64,
}

struct NoEdgeCase {
    card_id: i64,
    attack: i64,
    multiplier: i64,
}

struct WoodSproutCase {
    card_id: i64,
    attack: i64,
    segments: i64,
    positive_bonus: i64,
}

const DREAM_FLYING_FANG_CASES: &[AttackSegmentsCase] = &[
    AttackSegmentsCase {
        card_id: 1_000_071,
        attack: 6,
        segments: 1,
    },
    AttackSegmentsCase {
        card_id: 1_010_071,
        attack: 8,
        segments: 1,
    },
    AttackSegmentsCase {
        card_id: 1_020_071,
        attack: 10,
        segments: 1,
    },
    AttackSegmentsCase {
        card_id: 1_030_071,
        attack: 4,
        segments: 2,
    },
    AttackSegmentsCase {
        card_id: 1_040_071,
        attack: 4,
        segments: 3,
    },
];

fn run_dream_flying_fang_case(case: &AttackSegmentsCase) {
    let card_id = case.card_id;
    let attack = case.attack;
    let segments = case.segments;
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    state.p1.sword.sword_intent = 2;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        state.p2.core.hp,
        100 - (attack + 2) * segments,
        "card {card_id}"
    );
    assert_eq!(state.p1.sword.sword_intent, 2, "card {card_id}");
    assert_eq!(
        state.active_effect_pending_sword_intent(),
        0,
        "card {card_id}"
    );
    assert_eq!(
        state.active_effect_deferred_sword_intent_restore(),
        0,
        "card {card_id}"
    );
    assert_eq!(
        state.p1.turn.attack_segments_performed, segments,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: DREAM_FLYING_FANG_CASES,
    run: run_dream_flying_fang_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes",
    valid: dream_flying_fang_restores_pending_case_table_is_valid,
    cases: [
        dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_card_1000071 => 0,
        dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_card_1010071 => 1,
        dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_card_1020071 => 2,
        dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_card_1030071 => 3,
        dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_card_1040071 => 4,
    ]
}

#[test]
fn dream_flying_fang_restores_pending_sword_intent_for_all_five_shapes_no_intent() {
    let mut no_intent = ReplayState::test_from_fixture(&fixture(original_card(1_000_071)));
    no_intent.test_execute_one_card(PlayerSide::P1);
    assert_eq!(no_intent.p2.core.hp, 94);
    assert_eq!(no_intent.p1.sword.sword_intent, 0);
}

#[test]
fn dream_flying_fang_restores_after_circulation_preserves_the_pending_spend() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(1_000_071)));
    state.p1.sword.sword_intent = 3;
    state.p1.sword.sword_intent_circulation = 1;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 91);
    assert_eq!(state.p1.sword.sword_intent, 6);
    assert_eq!(state.p1.sword.sword_intent_circulation, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.active_effect_deferred_sword_intent_restore(), 0);
}

#[test]
fn mirage_flying_fang_keeps_pending_restore_plus_its_extra_intent() {
    let mut card_fixture = fixture(original_card(263));
    card_fixture.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&card_fixture);
    state.p1.sword.sword_intent = 3;

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, 93);
    assert_eq!(state.p1.sword.sword_intent, 4);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.active_effect_deferred_sword_intent_restore(), 0);
}

const SPIRIT_RHINO_CASES: &[RhinoCase] = &[
    RhinoCase {
        card_id: 325,
        printed_defense: 8,
        expected_anima: 3,
    },
    RhinoCase {
        card_id: 10_325,
        printed_defense: 12,
        expected_anima: 4,
    },
    RhinoCase {
        card_id: 20_325,
        printed_defense: 16,
        expected_anima: 5,
    },
];

fn run_spirit_rhino_case(case: &RhinoCase) {
    let card_id = case.card_id;
    let printed_defense = case.printed_defense;
    let expected_anima = case.expected_anima;
    let mut card_fixture = fixture(original_card(card_id));
    card_fixture.players.p1.initial_defense = 5;
    let mut state = ReplayState::test_from_fixture(&card_fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.defense, 5 + printed_defense, "card {card_id}");
    assert_eq!(state.p1.core.anima, expected_anima, "card {card_id}");
}

crate::data_driven_cases! {
    table: SPIRIT_RHINO_CASES,
    run: run_spirit_rhino_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::mirage_spirit_rhino_formation_uses_post_gain_total_defense_for_all_shapes",
    valid: mirage_spirit_rhino_formation_case_table_is_valid,
    cases: [
        mirage_spirit_rhino_formation_uses_post_gain_total_defense_for_all_shapes_card_325 => 0,
        mirage_spirit_rhino_formation_uses_post_gain_total_defense_for_all_shapes_card_10325 => 1,
        mirage_spirit_rhino_formation_uses_post_gain_total_defense_for_all_shapes_card_20325 => 2,
    ]
}

const DREAM_THUNDER_CASES: &[AttackSegmentsCase] = &[
    AttackSegmentsCase {
        card_id: 4_000_071,
        attack: 7,
        segments: 1,
    },
    AttackSegmentsCase {
        card_id: 4_010_071,
        attack: 9,
        segments: 1,
    },
    AttackSegmentsCase {
        card_id: 4_020_071,
        attack: 2,
        segments: 2,
    },
    AttackSegmentsCase {
        card_id: 4_030_071,
        attack: 1,
        segments: 3,
    },
    AttackSegmentsCase {
        card_id: 4_040_071,
        attack: 1,
        segments: 4,
    },
];

fn run_dream_thunder_case(case: &AttackSegmentsCase) {
    let card_id = case.card_id;
    let attack = case.attack;
    let segments = case.segments;
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    state.p1.astrology.hexagram = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        state.p2.core.hp,
        100 - (attack + 3) * segments,
        "card {card_id}"
    );
    assert_eq!(state.p1.astrology.hexagram, 3, "card {card_id}");
    assert_eq!(
        state.p1.turn.attack_segments_performed, segments,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: DREAM_THUNDER_CASES,
    run: run_dream_thunder_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment",
    valid: dream_thunder_uses_one_hexagram_snapshot_case_table_is_valid,
    cases: [
        dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment_card_4000071 => 0,
        dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment_card_4010071 => 1,
        dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment_card_4020071 => 2,
        dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment_card_4030071 => 3,
        dream_thunder_uses_one_hexagram_snapshot_for_every_configured_segment_card_4040071 => 4,
    ]
}

const DREAM_DRY_HEXAGRAM_CASES: &[DryHexagramCase] = &[
    DryHexagramCase {
        card_id: 4_000_086,
        hexagram: 2,
        expected_action_again: false,
    },
    DryHexagramCase {
        card_id: 4_010_086,
        hexagram: 3,
        expected_action_again: false,
    },
    DryHexagramCase {
        card_id: 4_020_086,
        hexagram: 4,
        expected_action_again: false,
    },
    DryHexagramCase {
        card_id: 4_030_086,
        hexagram: 1,
        expected_action_again: true,
    },
    DryHexagramCase {
        card_id: 4_040_086,
        hexagram: 2,
        expected_action_again: true,
    },
];

fn run_dream_dry_hexagram_case(case: &DryHexagramCase) {
    let card_id = case.card_id;
    let hexagram = case.hexagram;
    let expected_action_again = case.expected_action_again;
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(card_id)));

    assert_eq!(
        state.test_execute_one_card(PlayerSide::P1),
        expected_action_again,
        "card {card_id}"
    );
    assert_eq!(state.p1.astrology.hexagram, hexagram, "card {card_id}");
    assert_eq!(
        state.p1.turn.action_again_count,
        i64::from(expected_action_again),
        "card {card_id}"
    );
    assert!(state.p1.deck.slots[0].skipped, "card {card_id}");
}

crate::data_driven_cases! {
    table: DREAM_DRY_HEXAGRAM_CASES,
    run: run_dream_dry_hexagram_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::dream_dry_hexagram_gains_once_and_uses_only_configured_action_again",
    valid: dream_dry_hexagram_gains_once_case_table_is_valid,
    cases: [
        dream_dry_hexagram_gains_once_and_uses_only_configured_action_again_card_4000086 => 0,
        dream_dry_hexagram_gains_once_and_uses_only_configured_action_again_card_4010086 => 1,
        dream_dry_hexagram_gains_once_and_uses_only_configured_action_again_card_4020086 => 2,
        dream_dry_hexagram_gains_once_and_uses_only_configured_action_again_card_4030086 => 3,
        dream_dry_hexagram_gains_once_and_uses_only_configured_action_again_card_4040086 => 4,
    ]
}

const MIRAGE_CLOUD_DANCE_PILOT_CASES: &[CloudDancePilotCase] = &[
    CloudDancePilotCase {
        card_id: 309,
        anima: 2,
        defense: 4,
        sword_intent: 2,
    },
    CloudDancePilotCase {
        card_id: 10_309,
        anima: 3,
        defense: 6,
        sword_intent: 3,
    },
    CloudDancePilotCase {
        card_id: 20_309,
        anima: 4,
        defense: 8,
        sword_intent: 4,
    },
];

fn run_mirage_cloud_dance_pilot_case(case: &CloudDancePilotCase) {
    let card_id = case.card_id;
    let anima = case.anima;
    let defense = case.defense;
    let sword_intent = case.sword_intent;
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    state.p1.turn.used_card_count = 4;
    state.p1.core.anima = 1;
    state.p1.core.defense = 5;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 1 + anima, "card {card_id}");
    assert_eq!(state.p1.core.defense, 5 + defense, "card {card_id}");
    assert_eq!(
        state.p1.sword.sword_intent,
        sword_intent + 4,
        "card {card_id}"
    );
    assert_eq!(state.p1.turn.used_card_count, 5, "card {card_id}");
}

crate::data_driven_cases! {
    table: MIRAGE_CLOUD_DANCE_PILOT_CASES,
    run: run_mirage_cloud_dance_pilot_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::mirage_cloud_dance_reads_only_previously_completed_cards_for_all_shapes",
    valid: mirage_cloud_dance_reads_only_previously_case_table_is_valid,
    cases: [
        mirage_cloud_dance_reads_only_previously_completed_cards_for_all_shapes_card_309 => 0,
        mirage_cloud_dance_reads_only_previously_completed_cards_for_all_shapes_card_10309 => 1,
        mirage_cloud_dance_reads_only_previously_completed_cards_for_all_shapes_card_20309 => 2,
    ]
}

const NO_EDGE_CHAIN_CASES: &[NoEdgeCase] = &[
    NoEdgeCase {
        card_id: 288,
        attack: 6,
        multiplier: 2,
    },
    NoEdgeCase {
        card_id: 10_288,
        attack: 6,
        multiplier: 3,
    },
    NoEdgeCase {
        card_id: 20_288,
        attack: 6,
        multiplier: 4,
    },
];

fn run_no_edge_chain_case(case: &NoEdgeCase) {
    let card_id = case.card_id;
    let attack = case.attack;
    let multiplier = case.multiplier;
    let mut inactive = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    assert!(!inactive.test_execute_one_card(PlayerSide::P1));
    assert_eq!(inactive.p2.core.hp, 100, "inactive card {card_id}");
    assert_eq!(
        inactive.p1.turn.attack_segments_performed, 0,
        "inactive card {card_id}"
    );

    let mut active = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    active.p1.sword.cloud_chain = 1;
    active.p1.turn.attack_segments_performed = 3;
    assert!(!active.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        active.p2.core.hp,
        100 - attack - multiplier * 3,
        "card {card_id}"
    );
    assert_eq!(
        active.p1.turn.attack_segments_performed, 4,
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: NO_EDGE_CHAIN_CASES,
    run: run_no_edge_chain_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::mirage_cloud_sword_no_edge_requires_chain_and_scales_from_prior_segments",
    valid: mirage_cloud_sword_no_edge_requires_chain_case_table_is_valid,
    cases: [
        mirage_cloud_sword_no_edge_requires_chain_and_scales_from_prior_segments_card_288 => 0,
        mirage_cloud_sword_no_edge_requires_chain_and_scales_from_prior_segments_card_10288 => 1,
        mirage_cloud_sword_no_edge_requires_chain_and_scales_from_prior_segments_card_20288 => 2,
    ]
}

#[test]
fn mirage_cloud_sword_no_edge_accepts_talent_14_as_cloud_chain() {
    let mut card_fixture = fixture(original_card(20_288));
    card_fixture.players.p1.talents = vec![14];
    let mut state = ReplayState::test_from_fixture(&card_fixture);
    state.p1.turn.attack_segments_performed = 2;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 86);
    assert_eq!(state.p1.turn.attack_segments_performed, 3);
}

#[test]
fn mirage_cloud_sword_no_edge_accepts_active_resonance_10_and_long_ma_spirit() {
    let mut inactive_resonance = ReplayState::test_from_fixture(&fixture(original_card(20_288)));
    inactive_resonance.p1.identity.talent_resonance_id = Some(10);
    inactive_resonance.p1.turn.attack_segments_performed = 2;
    inactive_resonance.test_execute_one_card(PlayerSide::P1);
    assert_eq!(inactive_resonance.p2.core.hp, 100);

    let mut active_resonance = ReplayState::test_from_fixture(&fixture(original_card(20_288)));
    active_resonance.p1.identity.talent_resonance_id = Some(10);
    active_resonance
        .p1
        .identity
        .talent_resonance_temp_flags
        .push(10);
    active_resonance.p1.turn.attack_segments_performed = 2;
    active_resonance.test_execute_one_card(PlayerSide::P1);
    assert_eq!(active_resonance.p2.core.hp, 86);
    assert_eq!(active_resonance.p1.turn.attack_segments_performed, 3);

    let mut long_ma = ReplayState::test_from_fixture(&fixture(original_card(20_288)));
    long_ma.p1.elements.long_ma_spirit = 1;
    long_ma.p1.turn.attack_segments_performed = 2;
    long_ma.test_execute_one_card(PlayerSide::P1);
    assert_eq!(long_ma.p2.core.hp, 86);
    assert_eq!(long_ma.p1.turn.attack_segments_performed, 3);
}

const WOOD_SPROUT_CASES: &[WoodSproutCase] = &[
    WoodSproutCase {
        card_id: 7_000_088,
        attack: 3,
        segments: 2,
        positive_bonus: 1,
    },
    WoodSproutCase {
        card_id: 7_010_088,
        attack: 4,
        segments: 2,
        positive_bonus: 1,
    },
    WoodSproutCase {
        card_id: 7_020_088,
        attack: 5,
        segments: 2,
        positive_bonus: 2,
    },
    WoodSproutCase {
        card_id: 7_030_088,
        attack: 3,
        segments: 3,
        positive_bonus: 2,
    },
    WoodSproutCase {
        card_id: 7_040_088,
        attack: 3,
        segments: 4,
        positive_bonus: 2,
    },
];

fn run_wood_sprout_case(case: &WoodSproutCase) {
    let card_id = case.card_id;
    let attack = case.attack;
    let segments = case.segments;
    let positive_bonus = case.positive_bonus;
    let mut empty = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    assert!(!empty.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        empty.p2.core.hp,
        100 - attack * segments,
        "empty card {card_id}"
    );

    let mut positive = ReplayState::test_from_fixture(&fixture(original_card(card_id)));
    positive.p1.hp_mutation.add_hp_count = 20;
    assert!(!positive.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        positive.p2.core.hp,
        100 - (attack + positive_bonus) * segments,
        "positive card {card_id}"
    );
}

crate::data_driven_cases! {
    table: WOOD_SPROUT_CASES,
    run: run_wood_sprout_case,
    suite: "replay::tests_synthetic_oracle_dream_mirage_pilot::admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches",
    valid: admitted_dream_wood_sprout_covers_branches_case_table_is_valid,
    cases: [
        admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches_card_7000088 => 0,
        admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches_card_7010088 => 1,
        admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches_card_7020088 => 2,
        admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches_card_7030088 => 3,
        admitted_dream_wood_sprout_covers_empty_fixed_and_divided_hp_gain_branches_card_7040088 => 4,
    ]
}

#[test]
fn dream_cloud_thick_earth_reads_every_positive_cloud_chain_gain() {
    let card = original_card(1_040_078);
    let mut state = ReplayState::test_from_fixture(&fixture(card.clone()));
    state.gain_cloud_chain(PlayerSide::P1, 3);

    assert!(!state.test_execute_one_card(PlayerSide::P1));

    assert_eq!(state.p1.core.defense, 21);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::CloudSwordUsedCount),
        4,
    );
}
