use super::cards_mirage_ronghui::MirageRonghuiValue;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    original_card(BASIC_ATTACK_ID)
}

fn deck() -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![basic_attack()], basic_attack())
}

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![first], basic_attack())
}

fn player() -> FixturePlayer {
    player_with(deck())
}

fn player_with(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6))
}

fn fixture() -> BattleFixture {
    fixture_with(player(), player())
}

fn fixture_with(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
}

fn common_card_state(card_id: i64, actor_anima: i64, target_defense: i64) -> ReplayState {
    let mut p1 = player_with(deck_with(original_card(card_id)));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    p1.initial_anima = actor_anima;
    p2.initial_defense = target_defense;
    ReplayState::test_from_fixture(&fixture_with(p1, p2))
}

/// Typed rows for the data-driven damage tables: every loop iteration of the
/// migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `card_145`).
struct SimpleAttackCase {
    card_id: i64,
    expected_segments: i64,
    expected_hp: i64,
}

struct RankedAttackCase {
    card_id: i64,
    initial_anima: i64,
    expected_anima: i64,
    expected_segments: i64,
    expected_hp: i64,
}

struct EscapeCase {
    card_id: i64,
    active_element_count: i64,
    expected_defense: i64,
    expected_action_again: bool,
}

struct CardHpCase {
    card_id: i64,
    expected_hp: i64,
}

struct HorseHoofCase {
    card_id: i64,
    roll: i64,
    expected_hp: i64,
}

#[test]
fn dream_majesty_in_deck_replaces_percent_momentum_before_the_card_is_used() {
    let mut cards = deck();
    cards[1] = original_card(10_030_084);
    let mut state = ReplayState::test_from_fixture(&fixture_with(player_with(cards), player()));
    state.p1.beng.momentum_limit = 10;
    state.p1.beng.momentum = 10;

    state.apply_attack(PlayerSide::P1, 17, 0);

    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p1.beng.momentum, 9);
}

#[test]
fn leaf_blade_flower_grants_persistent_defense_piercing_after_its_own_attack() {
    let mut state = common_card_state(9_020_018, 0, 12);

    state.test_execute_one_card(PlayerSide::P1);

    // ApplyDamage's low-attack branch doubles 4 to 8 when 4 * 2 < 10.
    state.p2.core.hp = 50;
    state.p2.core.defense = 10;
    state.apply_attack(PlayerSide::P1, 4, 0);
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 2);

    // At the threshold, 4 gains ceil(6 / 2) before the ordinary defense loss.
    state.p2.core.hp = 50;
    state.p2.core.defense = 6;
    state.apply_attack(PlayerSide::P1, 4, 0);
    assert_eq!(state.p2.core.hp, 49);
    assert_eq!(state.p2.core.defense, 0);
}

#[test]
fn turn_wide_ignore_defense_preserves_persistent_attack_charges() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p1.turn.current_turn_ignore_defense = 1;
    state.p1.turn.ignore_defense_attacks = 2;
    state.p2.core.defense = 20;

    state.apply_attack(PlayerSide::P1, 5, 0);
    assert_eq!(state.p2.core.hp, 45);
    assert_eq!(state.p2.core.defense, 20);
    assert_eq!(state.p1.turn.ignore_defense_attacks, 2);

    state.p1.turn.current_turn_ignore_defense = 0;
    state.apply_attack(PlayerSide::P1, 5, 0);
    assert_eq!(state.p2.core.hp, 40);
    assert_eq!(state.p1.turn.ignore_defense_attacks, 1);
}

#[test]
fn metal_iron_bone_reduces_non_attack_damage() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;

    state.apply_damage(PlayerSide::P1, 8, false, false, false);

    assert_eq!(state.p2.core.hp, 47);
}

#[test]
fn dismantle_fist_stance_reduces_non_attack_damage_after_iron_bone() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;
    state.p2.fate.dismantle_move = 1;
    state.p2.beng.quan_stance = 1;

    state.apply_damage(PlayerSide::P1, 13, false, false, false);

    assert_eq!(state.p2.core.hp, 46);
}

#[test]
fn attack_damage_does_not_apply_iron_bone_twice() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;

    state.apply_damage(PlayerSide::P1, 8, true, false, false);

    assert_eq!(state.p2.core.hp, 42);
}

#[test]
fn guard_makes_same_hit_defense_absorption_worth_zero_hp() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.core.defense = 6;
    state.p2.core.guard = 1;

    state.apply_damage(PlayerSide::P1, 20, false, false, false);

    // Original ordering is unchanged: defense is spent first, then guard cancels
    // the remaining 14. The counterfactual telemetry must not claim the spent 6
    // defense saved HP because guard would also have cancelled a full 20-point hit.
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p2.prevention.hp_loss_prevented_by_guard, 14);
    assert_eq!(state.p2.prevention.hp_loss_prevented_by_defense, 0);
}

const COMMON_SIMPLE_ATTACK_CASES: &[SimpleAttackCase] = &[
    SimpleAttackCase {
        card_id: 145,
        expected_segments: 1,
        expected_hp: 77,
    },
    SimpleAttackCase {
        card_id: 146,
        expected_segments: 2,
        expected_hp: 74,
    },
    SimpleAttackCase {
        card_id: 147,
        expected_segments: 3,
        expected_hp: 71,
    },
    SimpleAttackCase {
        card_id: 148,
        expected_segments: 4,
        expected_hp: 68,
    },
    SimpleAttackCase {
        card_id: 149,
        expected_segments: 5,
        expected_hp: 65,
    },
    SimpleAttackCase {
        card_id: 150,
        expected_segments: 7,
        expected_hp: 59,
    },
];

fn run_common_simple_attack_case(case: &SimpleAttackCase) {
    let card_id = case.card_id;
    let expected_segments = case.expected_segments;
    let expected_hp = case.expected_hp;
    let mut state = common_card_state(card_id, 0, 0);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
            state.p2.status.internal_injury,
            state.p2.status.external_injury
        ),
        (expected_hp, expected_segments, 0, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: COMMON_SIMPLE_ATTACK_CASES,
    run: run_common_simple_attack_case,
    suite: "replay::tests_damage::common_simple_attacks_follow_configured_attack_count",
    valid: common_simple_attacks_case_table_is_valid,
    cases: [
        common_simple_attacks_follow_configured_attack_count_card_145 => 0,
        common_simple_attacks_follow_configured_attack_count_card_146 => 1,
        common_simple_attacks_follow_configured_attack_count_card_147 => 2,
        common_simple_attacks_follow_configured_attack_count_card_148 => 3,
        common_simple_attacks_follow_configured_attack_count_card_149 => 4,
        common_simple_attacks_follow_configured_attack_count_card_150 => 5,
    ]
}

const VERIFIED_SIMPLE_ATTACK_CASES: &[RankedAttackCase] = &[
    RankedAttackCase {
        card_id: 1_000_007,
        initial_anima: 1,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 70,
    },
    RankedAttackCase {
        card_id: 1_010_007,
        initial_anima: 1,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 67,
    },
    RankedAttackCase {
        card_id: 1_020_007,
        initial_anima: 1,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 64,
    },
    RankedAttackCase {
        card_id: 1_000_019,
        initial_anima: 2,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 64,
    },
    RankedAttackCase {
        card_id: 1_010_019,
        initial_anima: 2,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 60,
    },
    RankedAttackCase {
        card_id: 1_020_019,
        initial_anima: 2,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 56,
    },
    RankedAttackCase {
        card_id: 1_000_032,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 3,
        expected_hp: 71,
    },
    RankedAttackCase {
        card_id: 1_010_032,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 3,
        expected_hp: 68,
    },
    RankedAttackCase {
        card_id: 1_020_032,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 3,
        expected_hp: 65,
    },
    RankedAttackCase {
        card_id: 9_000_003,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 74,
    },
    RankedAttackCase {
        card_id: 9_010_003,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 2,
        expected_hp: 72,
    },
    RankedAttackCase {
        card_id: 9_020_003,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 3,
        expected_hp: 71,
    },
    RankedAttackCase {
        card_id: 10_000_029,
        initial_anima: 4,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 62,
    },
    RankedAttackCase {
        card_id: 10_010_029,
        initial_anima: 4,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 58,
    },
    RankedAttackCase {
        card_id: 10_020_029,
        initial_anima: 4,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 54,
    },
    RankedAttackCase {
        card_id: 10_000_035,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 69,
    },
    RankedAttackCase {
        card_id: 10_010_035,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 64,
    },
    RankedAttackCase {
        card_id: 10_020_035,
        initial_anima: 0,
        expected_anima: 0,
        expected_segments: 1,
        expected_hp: 59,
    },
];

fn run_verified_simple_attack_case(case: &RankedAttackCase) {
    let card_id = case.card_id;
    let initial_anima = case.initial_anima;
    let expected_anima = case.expected_anima;
    let expected_segments = case.expected_segments;
    let expected_hp = case.expected_hp;
    let mut state = common_card_state(card_id, initial_anima, 0);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p1.core.anima,
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
            state.p2.status.internal_injury,
            state.p2.status.external_injury,
        ),
        (expected_anima, expected_hp, expected_segments, 0, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: VERIFIED_SIMPLE_ATTACK_CASES,
    run: run_verified_simple_attack_case,
    suite: "replay::tests_damage::verified_simple_attack_cards_follow_exact_rank_config",
    valid: verified_simple_attack_cards_case_table_is_valid,
    cases: [
        verified_simple_attack_cards_follow_exact_rank_config_card_1000007 => 0,
        verified_simple_attack_cards_follow_exact_rank_config_card_1010007 => 1,
        verified_simple_attack_cards_follow_exact_rank_config_card_1020007 => 2,
        verified_simple_attack_cards_follow_exact_rank_config_card_1000019 => 3,
        verified_simple_attack_cards_follow_exact_rank_config_card_1010019 => 4,
        verified_simple_attack_cards_follow_exact_rank_config_card_1020019 => 5,
        verified_simple_attack_cards_follow_exact_rank_config_card_1000032 => 6,
        verified_simple_attack_cards_follow_exact_rank_config_card_1010032 => 7,
        verified_simple_attack_cards_follow_exact_rank_config_card_1020032 => 8,
        verified_simple_attack_cards_follow_exact_rank_config_card_9000003 => 9,
        verified_simple_attack_cards_follow_exact_rank_config_card_9010003 => 10,
        verified_simple_attack_cards_follow_exact_rank_config_card_9020003 => 11,
        verified_simple_attack_cards_follow_exact_rank_config_card_10000029 => 12,
        verified_simple_attack_cards_follow_exact_rank_config_card_10010029 => 13,
        verified_simple_attack_cards_follow_exact_rank_config_card_10020029 => 14,
        verified_simple_attack_cards_follow_exact_rank_config_card_10000035 => 15,
        verified_simple_attack_cards_follow_exact_rank_config_card_10010035 => 16,
        verified_simple_attack_cards_follow_exact_rank_config_card_10020035 => 17,
    ]
}

#[test]
fn hungry_tiger_cost_reduction_applies_before_its_printed_attack() {
    let mut state = common_card_state(10_000_029, 1, 0);
    state.p1.status.internal_injury = 2;
    state.p1.status.flaw = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p1.core.anima,
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
        ),
        (0, 62, 1)
    );
}

#[test]
fn continuous_collapse_repeats_and_preserves_inherited_beng_quan_effects() {
    let mut state = common_card_state(10_000_035, 0, 0);
    state.p1.beng.beng_quan_defense = 3;
    state.p1.beng.beng_quan_double_shadow = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.core.defense,
            state.p1.turn.attack_segments_performed,
            state.p1.beng.beng_quan_defense,
            state.p1.beng.beng_quan_double_shadow,
        ),
        (58, 6, 2, 3, 1)
    );
}

const FIVE_ELEMENTS_ESCAPE_CASES: &[EscapeCase] = &[
    EscapeCase {
        card_id: 7_000_055,
        active_element_count: 0,
        expected_defense: 8,
        expected_action_again: false,
    },
    EscapeCase {
        card_id: 7_000_055,
        active_element_count: 1,
        expected_defense: 8,
        expected_action_again: false,
    },
    EscapeCase {
        card_id: 7_000_055,
        active_element_count: 2,
        expected_defense: 8,
        expected_action_again: true,
    },
    EscapeCase {
        card_id: 7_010_055,
        active_element_count: 2,
        expected_defense: 13,
        expected_action_again: true,
    },
    EscapeCase {
        card_id: 7_020_055,
        active_element_count: 2,
        expected_defense: 18,
        expected_action_again: true,
    },
];

fn run_five_elements_escape_case(case: &EscapeCase) {
    let card_id = case.card_id;
    let active_element_count = case.active_element_count;
    let expected_defense = case.expected_defense;
    let expected_action_again = case.expected_action_again;
    let mut state = common_card_state(card_id, 0, 0);
    if active_element_count >= 1 {
        state.activate_element(PlayerSide::P1, Element::Metal);
    }
    if active_element_count >= 2 {
        state.activate_element(PlayerSide::P1, Element::Water);
    }

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p1.core.defense,
            action_again,
            state.p1.turn.action_again_count,
        ),
        (
            expected_defense,
            expected_action_again,
            i64::from(expected_action_again),
        ),
        "card {card_id}, active elements {active_element_count}"
    );
}

crate::data_driven_cases! {
    table: FIVE_ELEMENTS_ESCAPE_CASES,
    run: run_five_elements_escape_case,
    suite: "replay::tests_damage::five_elements_escape_uses_printed_defense_and_exact_two_element_threshold",
    valid: five_elements_escape_case_table_is_valid,
    cases: [
        five_elements_escape_uses_printed_defense_and_exact_two_element_threshold_card_7000055_elem_0 => 0,
        five_elements_escape_uses_printed_defense_and_exact_two_element_threshold_card_7000055_elem_1 => 1,
        five_elements_escape_uses_printed_defense_and_exact_two_element_threshold_card_7000055_elem_2 => 2,
        five_elements_escape_uses_printed_defense_and_exact_two_element_threshold_card_7010055_elem_2 => 3,
        five_elements_escape_uses_printed_defense_and_exact_two_element_threshold_card_7020055_elem_2 => 4,
    ]
}

#[test]
fn space_spirit_field_skips_once_in_the_last_two_slots_then_gains_anima() {
    let field = original_card(9_000_015);
    let mut cards = deck();
    cards[6] = field;
    let mut state = ReplayState::test_from_fixture(&fixture_with(player_with(cards), player()));
    state.p1.deck.queue = vec![6, 7, 0, 1, 2, 3, 4, 5];

    state.test_execute_one_card(PlayerSide::P1);

    assert!(state.p1.deck.slots[6].used);
    assert_eq!((state.p1.core.anima, state.p2.core.hp), (0, 47));

    state.p1.deck.queue = vec![6, 0, 1, 2, 3, 4, 5, 7];
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!((state.p1.core.anima, state.p2.core.hp), (4, 47));
}

const HIDDEN_BASIC_ATTACK_CASES: &[CardHpCase] = &[
    CardHpCase {
        card_id: 286,
        expected_hp: 77,
    },
    CardHpCase {
        card_id: 10_286,
        expected_hp: 74,
    },
    CardHpCase {
        card_id: 20_286,
        expected_hp: 71,
    },
];

fn run_hidden_basic_attack_case(case: &CardHpCase) {
    let card_id = case.card_id;
    let expected_hp = case.expected_hp;
    let mut state = common_card_state(card_id, 0, 0);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (state.p2.core.hp, state.p1.turn.attack_segments_performed),
        (expected_hp, 1),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: HIDDEN_BASIC_ATTACK_CASES,
    run: run_hidden_basic_attack_case,
    suite: "replay::tests_damage::hidden_basic_attack_three_ranks_follow_configured_damage",
    valid: hidden_basic_attack_three_ranks_case_table_is_valid,
    cases: [
        hidden_basic_attack_three_ranks_follow_configured_damage_card_286 => 0,
        hidden_basic_attack_three_ranks_follow_configured_damage_card_10286 => 1,
        hidden_basic_attack_three_ranks_follow_configured_damage_card_20286 => 2,
    ]
}

#[test]
fn hidden_basic_attack_inherits_card_0_xiaoyao_tune_and_guqin() {
    // Card_286.OnExecuted 只调用 Card_0.OnExecuted。逍遥曲加在
    // cardConfig.attack 上；逍遥古琴把 actionAgain 设为真。
    // oracle：pa_5dc49f56，286 在逍遥曲 9 下打出 12 伤。
    let mut tuned = common_card_state(286, 0, 1);
    tuned.p1.music.xiaoyao_tune = 9;
    assert!(!tuned.test_execute_one_card(PlayerSide::P1));
    assert_eq!(tuned.p2.core.hp, 69); // 80 - (3 + 9 - 1)

    let mut plain = common_card_state(0, 0, 1);
    plain.p1.music.xiaoyao_tune = 9;
    plain.test_execute_one_card(PlayerSide::P1);
    assert_eq!(plain.p2.core.hp, 69);

    let mut guqin = common_card_state(286, 0, 0);
    guqin.p1.music.xiaoyao_guqin = 1;
    assert!(guqin.test_execute_one_card(PlayerSide::P1));
    assert_eq!(guqin.p2.core.hp, 77);
}

#[test]
fn lian_beng_keeps_crash_fist_star_seize_for_the_next_beng_quan() {
    // IsLianBeng 时 OnAfterExecuted 不扣 BengQuanDuoXing。连崩自己仍按
    // 实际伤害的 60% 回血，下一张崩拳再回血并消耗标记。
    let chain = original_card(10_000_035);
    let thrust = original_card(10_000_015);
    let mut cards = deck();
    cards[0] = chain.clone();
    cards[1] = thrust.clone();
    let mut p1 = player_with(cards);
    p1.base_max_hp = 80;
    let mut p2 = player();
    p2.base_max_hp = 80;
    let mut state = ReplayState::test_from_fixture(&fixture_with(p1, p2));
    state.p1.core.hp = 20;
    state.modify_mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CrashFistStarSeize, 1);

    state.test_execute_one_card(PlayerSide::P1);

    let chain_heal = chain.attack.unwrap_or(0) * 3 / 5;
    assert!(chain_heal > 0);
    assert_eq!(
        state.p1.core.hp,
        20 - chain.hp_cost.unwrap_or(0) + chain_heal
    );
    assert_eq!(
        state.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CrashFistStarSeize),
        1
    );

    state.test_execute_one_card(PlayerSide::P1);

    let thrust_heal = thrust.attack.unwrap_or(0) * 3 / 5;
    assert_eq!(
        state.p1.core.hp,
        20 - chain.hp_cost.unwrap_or(0) + chain_heal - thrust.hp_cost.unwrap_or(0) + thrust_heal
    );
    assert_eq!(
        state.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CrashFistStarSeize),
        0
    );
}

const RAKSHASA_POUNCE_DAMAGE_CASES: &[CardHpCase] = &[
    CardHpCase {
        card_id: 71,
        expected_hp: 72,
    },
    CardHpCase {
        card_id: 10_071,
        expected_hp: 69,
    },
    CardHpCase {
        card_id: 20_071,
        expected_hp: 66,
    },
];

fn run_rakshasa_pounce_damage_case(case: &CardHpCase) {
    let card_id = case.card_id;
    let expected_hp = case.expected_hp;
    let mut state = common_card_state(card_id, 0, 0);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.core.anima,
            state.p1.status.external_injury
        ),
        (expected_hp, 1, 1),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: RAKSHASA_POUNCE_DAMAGE_CASES,
    run: run_rakshasa_pounce_damage_case,
    suite: "replay::tests_damage::rakshasa_pounce_attacks_then_gains_anima_and_self_external_injury",
    valid: rakshasa_pounce_attacks_case_table_is_valid,
    cases: [
        rakshasa_pounce_attacks_then_gains_anima_and_self_external_injury_card_71 => 0,
        rakshasa_pounce_attacks_then_gains_anima_and_self_external_injury_card_10071 => 1,
        rakshasa_pounce_attacks_then_gains_anima_and_self_external_injury_card_20071 => 2,
    ]
}

#[test]
fn rakshasa_pounce_external_injury_is_prevented_by_exorcism() {
    let mut state = common_card_state(10_071, 0, 0);
    state.p1.fate.exorcism = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.fate.exorcism, 0);
    assert_eq!(state.p1.status.external_injury, 0);
}

const SKY_BREAKING_CLAW_CASES: &[CardHpCase] = &[
    CardHpCase {
        card_id: 72,
        expected_hp: 67,
    },
    CardHpCase {
        card_id: 10_072,
        expected_hp: 64,
    },
    CardHpCase {
        card_id: 20_072,
        expected_hp: 61,
    },
];

fn run_sky_breaking_claw_case(case: &CardHpCase) {
    let card_id = case.card_id;
    let expected_hp = case.expected_hp;
    let mut state = common_card_state(card_id, 1, 0);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.core.anima,
            state.p1.status.internal_injury
        ),
        (expected_hp, 0, 1),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: SKY_BREAKING_CLAW_CASES,
    run: run_sky_breaking_claw_case,
    suite: "replay::tests_damage::sky_breaking_claw_pays_anima_then_attacks_and_self_internal_injury",
    valid: sky_breaking_claw_case_table_is_valid,
    cases: [
        sky_breaking_claw_pays_anima_then_attacks_and_self_internal_injury_card_72 => 0,
        sky_breaking_claw_pays_anima_then_attacks_and_self_internal_injury_card_10072 => 1,
        sky_breaking_claw_pays_anima_then_attacks_and_self_internal_injury_card_20072 => 2,
    ]
}

const HIDDEN_WEAPON_CASES: &[CardHpCase] = &[
    CardHpCase {
        card_id: 330,
        expected_hp: 78,
    },
    CardHpCase {
        card_id: 10_330,
        expected_hp: 75,
    },
    CardHpCase {
        card_id: 20_330,
        expected_hp: 72,
    },
];

fn run_hidden_weapon_case(case: &CardHpCase) {
    let card_id = case.card_id;
    let expected_hp = case.expected_hp;
    let mut card = original_card(card_id);
    card.card_type = Some(OriginalEnumValue {
        value: CARD_TYPE_CONSUME,
        name: "Consume".to_string().into(),
    });
    let mut p1 = player_with(deck_with(card));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    p2.initial_defense = 3;
    let mut state = ReplayState::test_from_fixture(&fixture_with(p1, p2));
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.defense,
            state.p2.core.hp,
            state.p2.status.internal_injury,
            state.p2.status.external_injury,
            state.p1.deck.slots[0].skipped
        ),
        (0, expected_hp, 1, 1, true),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: HIDDEN_WEAPON_CASES,
    run: run_hidden_weapon_case,
    suite: "replay::tests_damage::hidden_weapon_applies_injuries_before_damage_and_consumes_defense",
    valid: hidden_weapon_case_table_is_valid,
    cases: [
        hidden_weapon_applies_injuries_before_damage_and_consumes_defense_card_330 => 0,
        hidden_weapon_applies_injuries_before_damage_and_consumes_defense_card_10330 => 1,
        hidden_weapon_applies_injuries_before_damage_and_consumes_defense_card_20330 => 2,
    ]
}

const HORSE_HOOF_CASES: &[HorseHoofCase] = &[
    HorseHoofCase {
        card_id: 367,
        roll: 5,
        expected_hp: 75,
    },
    HorseHoofCase {
        card_id: 10_367,
        roll: 9,
        expected_hp: 71,
    },
    HorseHoofCase {
        card_id: 20_367,
        roll: 13,
        expected_hp: 67,
    },
];

fn run_horse_hoof_case(case: &HorseHoofCase) {
    let card_id = case.card_id;
    let roll = case.roll;
    let expected_hp = case.expected_hp;
    let mut p1 = player_with(deck_with(original_card(card_id)));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    let mut battle = fixture_with(p1, p2);
    battle.decision_tape = vec![roll];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
            state.decision_tape.len()
        ),
        (expected_hp, 1, 0),
        "card {card_id}"
    );
}

crate::data_driven_cases! {
    table: HORSE_HOOF_CASES,
    run: run_horse_hoof_case,
    suite: "replay::tests_damage::horse_hoof_uses_original_fallback_random_attack_range",
    valid: horse_hoof_case_table_is_valid,
    cases: [
        horse_hoof_uses_original_fallback_random_attack_range_card_367 => 0,
        horse_hoof_uses_original_fallback_random_attack_range_card_10367 => 1,
        horse_hoof_uses_original_fallback_random_attack_range_card_20367 => 2,
    ]
}

#[test]
fn horse_hoof_uses_original_fallback_random_attack_range_random_fallback() {
    let mut p1 = player_with(deck_with(original_card(20_367)));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    let mut battle = fixture_with(p1, p2);
    battle.random_fallback_tape = vec![9];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.fail_on_missing_decision = true;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
            state.random_fallback_tape.len(),
            state.evaluation_error
        ),
        (71, 1, 0, None)
    );
}

#[test]
fn wan_mo_shi_xin_qu_self_damage_precedes_frenzy_sword_zero_lifesteal_per_segment() {
    // 原版每段攻击管道顺序（build 24610558，与旧 build 24466094 反编译一致的历史行为）：
    // 万魔噬心曲自伤在 CalculateAttack（BattleCharacter.cs:11757-11759），先于
    // ApplyDamage（BattleCharacter.cs:10882-10888）内的狂剑灵石吸血；反震（FanZhenXinFa
    // 217，:10998-11005）在吸血之后。
    // 锚定 pa_43f28c44/round-19：满血 123，云剑•无妄（1010028）2 段 ×6，万魔噬心曲 4，
    // 狂剑灵石 50%（每段 6×50%=3）：原版顺序 123 −4 +3 −4 +3 = 121。
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(1_010_028))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 123;
        player.core.max_hp = 123;
        player.music.chaotic_mind_tune = 4;
        player.sword.frenzy_sword_zero = 50;
        // 云剑•无妄须狂龙吞云在身才归为狂剑（IsKuangJian：HasBuff(357) && 卡名含「云剑」）。
        player.sword.frenzy_dragon_swallows_cloud = 1;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (state.p1.core.hp, state.p2.core.hp),
        (121, 50 - 12),
        "两段后自伤先于吸血：p1 应 121（错序为 118），p2 吃满 2×6"
    );
}

#[test]
fn wan_mo_shi_xin_qu_full_hp_segment_lifesteal_is_not_truncated_by_max_hp() {
    // 满血截断场景：狂剑•一式（1010022，1 段，attack=8，名含「狂剑」天然归狂剑类）
    // 满血 123 攻击，万魔噬心曲 4、狂剑灵石 50%（8×50%=4）：
    //   原版顺序：123 −4 +4 = 123（吸血全额入账，不被 maxHp 截断）
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(1_010_022))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 123;
        player.core.max_hp = 123;
        player.music.chaotic_mind_tune = 4;
        player.sword.frenzy_sword_zero = 50;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.hp, 123,
        "单段先自伤后吸血，吸血不被满血上限截断（错序为 119）"
    );
}

#[test]
fn frenzy_sword_zero_does_not_lifesteal_from_after_card_formation_attack() {
    // BattleCharacter.ApplyDamage (build 24610558:10882, 24466094:10820)
    // gates KuangJianLingShi with !HasBuff(AfterCardAciton). The printed
    // frenzy-sword hit can lifesteal, but a Zhou Tian formation follow-up in
    // OnAfterExecuted cannot inherit that identity for lifesteal.
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(2))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 50;
        player.core.max_hp = 100;
        player.sword.frenzy_sword_zero = 30;
        player.formations.heaven_cycle_sword_formation = 1;
        player.formations.heaven_cycle_sword_formation_damage = 5;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 50, "牌后阵法追击不得触发狂剑灵石吸血");
    assert_eq!(
        state.p2.core.hp, 42,
        "炎舞 2 攻，再叠 1 外伤后结算周天剑阵 5 攻"
    );
}

#[test]
fn cloud_way_talent_grades_apply_separate_defense_gains_for_nishi_and_he_ba_huang() {
    // CardActionBase.cs:2383-2386（OnBeforeExecuted 天赋参数 switch：
    // num3 = current % 10000 逐天赋命中 case 69）：云之道每个档位
    // （10069/20069/30069）独立执行一次 ModifyDef，不能汇总成一次加防。
    // NiShi（BattleCharacter.cs:10148）与 HeBaHuang（:10205-10209）都挂在
    // ModifyDef 上按次结算：汇总 7 → 反弹 floor(7/2)=3、八荒退 3+2+2；
    // 拆分 2+2+3 → 反弹 1+1+1、八荒退 1+1+1。
    // oracle 锚点：mirror-32299000 pa_0fa3de62 cp12
    // （游龙后 p2.def 16→4 = 15 伤 - 3 退款；引擎汇总 +7 时 16→10）。
    let mut p1 = player_with(deck_with(original_card(1_000_042)));
    p1.talents = vec![18, 66, 10_069, 20_069, 30_069];
    let mut p2 = player();
    p2.initial_defense = 16;
    let mut state = ReplayState::test_from_fixture(&fixture_with(p1, p2));
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p2.elements.earth_eight_wastes = 3;
    state.p1.sword.cloud_chain = 3;
    state.p1.dream_mirage.cloud_sword_used_count = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));

    // 伤害：云之道 2/2/3 三档各反弹 1 + 游龙 2×2 攻 + 连云 +3 反弹 1 = 8；
    // 合八荒 3 层退款前三次防御损失 1+1+1 = 3 → 16 - 8 + 3 = 11。
    assert_eq!(state.p2.core.defense, 11);
}
