//! build 25621897（仙魔赛季）换代中既有卡的最小契约：只覆盖本次源码 / 配置改动点。

use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::MirageRonghuiValue;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    super::test_support::test_card(id, base_id, name)
}

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::resize_deck(vec![active], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 1, 30, None, 1, None)
}

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
    super::test_support::default_fixture(player(p1_cards), player(p2_cards))
}

/// Typed rows for the data-driven season tables: every loop iteration of the
/// migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `strategy_1001`).
struct RarityDefenseCase {
    rarity: i64,
    expected_defense: i64,
}

struct StrategyCase {
    strategy: i64,
}

struct CompoundHpCase {
    strategy: i64,
    expected_hp: i64,
}

struct CloudElementCase {
    element: Element,
}

struct TemporaryAttackCase {
    temporary: bool,
}

struct TierCase {
    tier: i64,
}

struct FateVecCase {
    fate: &'static [i64],
}

struct CardProbeCase {
    id: i64,
}

struct CallbackTierCase {
    tier: i64,
    base: i64,
}

struct HiddenAttackCountCase {
    card_id: i64,
    attack_count: i64,
}

#[test]
fn xian_mo_5011_consumes_next_pu_gong_for_weakness_and_attack_reduction() {
    // 26125017 new: battle-start XiaCiPuGongXiaoHao(793) += 1; the next 普攻
    // gives dst 虚弱(101) += 3 / 减攻(103) += 1 and skips itself (once).
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![5011];
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.fate.xian_mo_5011_pending_pu_gong, 1);
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &basic_attack(), 0, false);
    assert_eq!(state.p1.fate.xian_mo_5011_pending_pu_gong, 0);
    assert_eq!(state.p2.status.weakness, 3);
    assert_eq!(state.p2.status.attack_reduction, 1);
    assert!(state.p1.deck.slots[0].skipped);
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &basic_attack(), 1, false);
    assert_eq!(state.p2.status.weakness, 3);
    assert_eq!(state.p2.status.attack_reduction, 1);
    assert!(!state.p1.deck.slots[1].skipped);
}

#[test]
fn star_shift_269_marks_persistent_effect_once_regardless_of_immediate_slots() {
    // Card_269.cs：立即成为星位 otherParams[0] 格，持续标记 HuanDouZhuanJiaXingWei 固定 +1。
    let mut star_shift = card(20_269, 269, "幻•斗转星移");
    star_shift.rarity = Some(2);
    star_shift.other_params = vec![3, 6, 2].into();
    let battle = fixture(deck(star_shift), deck(basic_attack()));

    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::StarShift),
        1
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::StarShiftAttack),
        6
    );
}

#[test]
fn steadfast_291_raises_momentum_limit_before_momentum() {
    // Card_291.cs：QiShiShangXian += otherParams[0] 先于 QiShi += otherParams[0]。
    let mut steadfast = card(291, 291, "幻•岿然不动");
    steadfast.other_params = vec![3, 1, 1].into();
    let battle = fixture(deck(steadfast), deck(basic_attack()));

    let mut state = ReplayState::test_from_fixture(&battle);
    let limit_before = state.p1.beng.momentum_limit;
    state.test_configure_p1(|player| player.beng.momentum = limit_before);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.beng.momentum_limit, limit_before + 3);
    assert_eq!(state.p1.beng.momentum, limit_before + 3);
}

const EARTH_317_RARITY_CASES: &[RarityDefenseCase] = &[
    RarityDefenseCase {
        rarity: 1,
        expected_defense: 10,
    },
    RarityDefenseCase {
        rarity: 2,
        expected_defense: 25,
    },
];

fn run_earth_317_rarity_case(case: &RarityDefenseCase) {
    // Card_317.cs：rarity 2 写 HuanTuLingZhenYiChuShangXian；OnTurnEnded 持有时水势加防不截断。
    let rarity = case.rarity;
    let expected_defense = case.expected_defense;
    let mut formation = card(10_317 + (rarity - 1) * 10_000, 317, "幻•土灵阵");
    formation.rarity = Some(rarity);
    formation.other_params = vec![1, 10].into();
    let battle = fixture(deck(formation), deck(basic_attack()));

    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        state.mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::MirageWaterDefenseUncapped
        ),
        i64::from(rarity == 2)
    );
    state.test_configure_p1(|player| {
        player.elements.water_momentum = 25;
        player.core.defense = 0;
    });
    state.apply_mirage_water_defense_after_turn_end_water_damage(PlayerSide::P1);
    assert_eq!(state.p1.core.defense, expected_defense, "rarity {rarity}");
}

crate::data_driven_cases! {
    table: EARTH_317_RARITY_CASES,
    run: run_earth_317_rarity_case,
    suite: "replay::tests_build_2026_09_season::earth_formation_317_level_three_removes_turn_end_defense_cap",
    valid: earth_formation_317_level_three_case_table_is_valid,
    cases: [
        earth_formation_317_level_three_removes_turn_end_defense_cap_rarity_1 => 0,
        earth_formation_317_level_three_removes_turn_end_defense_cap_rarity_2 => 1,
    ]
}

const OPENING_SWITCH_CASES: &[StrategyCase] = &[
    StrategyCase { strategy: 1001 },
    StrategyCase { strategy: 1002 },
    StrategyCase { strategy: 1003 },
    StrategyCase { strategy: 1004 },
    StrategyCase { strategy: 1005 },
    StrategyCase { strategy: 1006 },
    StrategyCase { strategy: 1008 },
];

fn run_opening_switch_case(case: &StrategyCase) {
    let strategy = case.strategy;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![strategy];
    let state = ReplayState::<true>::from_fixture(&battle, true).unwrap();
    for side in [PlayerSide::P1, PlayerSide::P2] {
        let actor = state.actor(side);
        assert_eq!(actor.core.anima, if strategy == 1001 { 2 } else { 0 });
        assert_eq!(actor.core.attack_bonus, i64::from(strategy == 1002));
        let expected_hp = match strategy {
            1003 => 34,
            1006 => 26,
            _ => 30,
        };
        assert_eq!(actor.core.hp, expected_hp, "strategy {strategy} HP");
        assert_eq!(actor.core.max_hp, expected_hp, "strategy {strategy} max HP");
        assert_eq!(
            actor.status.internal_injury,
            if strategy == 1004 { 3 } else { 0 }
        );
        assert_eq!(
            actor.turn.action_again_blocked_turns,
            if strategy == 1005 { 1 } else { 0 }
        );
        assert_eq!(
            actor.music.chaotic_mind_tune,
            if strategy == 1008 { 2 } else { 0 }
        );
    }
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert(strategy.to_string(), 1);
    let state = ReplayState::<true>::from_fixture(&battle, true).unwrap();
    for side in [PlayerSide::P1, PlayerSide::P2] {
        let actor = state.actor(side);
        assert_eq!(
            (actor.core.hp, actor.core.max_hp, actor.core.anima),
            (30, 30, 0)
        );
        assert_eq!(
            (actor.core.attack_bonus, actor.status.internal_injury),
            (0, 0)
        );
        assert_eq!(
            (
                actor.turn.action_again_blocked_turns,
                actor.music.chaotic_mind_tune
            ),
            (0, 0)
        );
    }
}

crate::data_driven_cases! {
    table: OPENING_SWITCH_CASES,
    run: run_opening_switch_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled",
    valid: xian_mo_opening_switches_apply_exact_case_table_is_valid,
    cases: [
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1001 => 0,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1002 => 1,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1003 => 2,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1004 => 3,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1005 => 4,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1006 => 5,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_strategy_1008 => 6,
    ]
}

const OPENING_COMPOUND_CASES: &[CompoundHpCase] = &[
    CompoundHpCase {
        strategy: 1003,
        expected_hp: 39,
    },
    CompoundHpCase {
        strategy: 1006,
        expected_hp: 23,
    },
];

fn run_opening_compound_case(case: &CompoundHpCase) {
    // Two owners compound in actor-boundary order, flooring each 15% independently.
    let strategy = case.strategy;
    let expected_hp = case.expected_hp;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![strategy];
    battle.players.p2.xian_mo_strategies = vec![strategy];
    let state = ReplayState::<true>::from_fixture(&battle, true).unwrap();
    assert_eq!(state.p1.core.hp, expected_hp);
    assert_eq!(state.p2.core.hp, expected_hp);
}

crate::data_driven_cases! {
    table: OPENING_COMPOUND_CASES,
    run: run_opening_compound_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_opening_switches_compound",
    valid: xian_mo_opening_switches_compound_case_table_is_valid,
    cases: [
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_compound_1003 => 0,
        xian_mo_opening_switches_apply_exact_parameters_to_both_sides_and_can_be_disabled_compound_1006 => 1,
    ]
}

#[test]
fn xian_mo_15018_samples_hp_before_permanent_return_origin_grass() {
    // Original HF pa_a814d789 first divergence: source 103/153;
    // 12% HP sample is 88.
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p2.base_max_hp = 88;
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10023".into(), 50);
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10024".into(), 59);
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10008".into(), 5);
    battle.players.p2.hand_cards = vec![0];
    battle.players.p2.xian_mo_strategies = vec![15018];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (103, 153));
    assert_eq!(
        state.mirage_ronghui_value(PlayerSide::P2, MirageRonghuiValue::LastTurnStartHp),
        88
    );
}

#[test]
fn shared_xian_mo_resource_hooks_stack_with_fate_effects_and_preserve_bypasses() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    // 7133 (霜华守护) is effective from round 13 (26125017).
    battle.source = Some(crate::fixture::FixtureSource {
        round: Some(13),
        ..Default::default()
    });
    battle.players.p1.xian_mo_strategies = vec![7076, 7133, 7048];
    battle.players.p1.fate_strategies = vec![394, 382, 310];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.modify_actor_max_hp(PlayerSide::P1, 10);
    assert_eq!(
        state.p1.core.max_hp, 42,
        "independent +1 modifiers both apply"
    );
    state.p1.core.hp = 10;
    state.modify_actor_hp(PlayerSide::P1, 1, false, false);
    assert_eq!(state.p1.core.hp, 13);
    state.gain_defense(PlayerSide::P1, 1);
    assert_eq!(state.p1.core.defense, 3);
    state.add_direct_element_activation(PlayerSide::P1, Element::Metal);
    state.add_direct_element_activation(PlayerSide::P1, Element::Metal);
    assert_eq!(
        state.p1.core.anima, 1,
        "shared per-turn marker prevents double reward"
    );
    state.add_direct_element_activation(PlayerSide::P1, Element::Metal);
    assert_eq!(state.p1.core.anima, 1);

    battle.players.p1.fate_strategies.clear();
    battle.players.p1.xian_mo_strategies = vec![28001, 1005];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.status.entangle = 1;
    state.p1.music.immortal_binding_tune = 1;
    state.modify_extra_actions(PlayerSide::P1, 1);
    assert!(state.test_consume_action_again(PlayerSide::P1, &basic_attack(), 0));
    assert_eq!(state.p1.status.entangle, 1);
    assert_eq!(state.p1.turn.action_again_blocked_turns, 1);
}

const CLOUD_ELEMENT_COST_CASES: &[CloudElementCase] = &[
    CloudElementCase {
        element: Element::Metal,
    },
    CloudElementCase {
        element: Element::Water,
    },
    CloudElementCase {
        element: Element::Wood,
    },
    CloudElementCase {
        element: Element::Fire,
    },
    CloudElementCase {
        element: Element::Earth,
    },
];

fn cloud_element_cost_state() -> ReplayState {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![11001, 11003, 7051];
    battle.players.p1.fate_strategies = vec![321];
    battle.players.p2.xian_mo_strategies = vec![7110];
    battle.players.p2.fate_strategies = vec![329];
    ReplayState::test_from_fixture(&battle)
}

fn run_cloud_element_cost_case(case: &CloudElementCase) {
    let element = case.element;
    let state = cloud_element_cost_state();
    assert!(super::support::has_cloud_chain(&state.p1));
    assert!(state.check_wu_xing(PlayerSide::P1, element));
}

crate::data_driven_cases! {
    table: CLOUD_ELEMENT_COST_CASES,
    run: run_cloud_element_cost_case,
    suite: "replay::tests_build_2026_09_season::shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries",
    valid: shared_xian_mo_cloud_element_cost_case_table_is_valid,
    cases: [
        shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_element_metal => 0,
        shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_element_water => 1,
        shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_element_wood => 2,
        shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_element_fire => 3,
        shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_element_earth => 4,
    ]
}

#[test]
fn shared_xian_mo_cloud_element_cost_and_second_actor_hooks_use_their_original_boundaries_anima_cost(
) {
    let state = cloud_element_cost_state();
    let mirror = original_card_definition_by_id(173).unwrap();
    assert_eq!(
        super::support::effective_anima_cost(&mirror, &state.p1, Some(0)),
        0
    );
    let kun = original_card_definition_by_id(1_000_036).unwrap();
    assert_eq!(
        super::support::effective_anima_cost(&kun, &state.p1, Some(0)),
        1
    );
    let water = original_card_definition_by_id(7_010_019).unwrap();
    assert_eq!(
        super::support::effective_anima_cost(&water, &state.p1, Some(0)),
        0
    );
    assert_eq!(state.p1.fate.next_rear_move_bypass, 0);
    assert_eq!(state.p2.fate.next_rear_move_bypass, 2);
}

const NORMAL_ATTACK_EXHAUST_CASES: &[TemporaryAttackCase] = &[
    TemporaryAttackCase { temporary: false },
    TemporaryAttackCase { temporary: true },
];

fn run_normal_attack_exhaust_case(case: &TemporaryAttackCase) {
    let temporary = case.temporary;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![5004, 5007, 5008, 13012];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &basic_attack(), 0, temporary);
    assert_eq!(
        (
            state.p1.core.guard,
            state.p1.core.attack_bonus,
            state.p1.turn.agility
        ),
        (3, 3, 14)
    );
    assert_eq!(state.p1.deck.slots[0].skipped, !temporary);
    assert_eq!(
        state.p1.beng.momentum_before_attack,
        if temporary { 0 } else { 3 }
    );
    state.p1.deck.slots[0].used = true;
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &basic_attack(), 0, temporary);
    assert_eq!(
        (
            state.p1.core.guard,
            state.p1.core.attack_bonus,
            state.p1.turn.agility
        ),
        (3, 3, 28)
    );
    assert_eq!(
        state.p1.beng.momentum_before_attack,
        if temporary { 0 } else { 3 }
    );
}

crate::data_driven_cases! {
    table: NORMAL_ATTACK_EXHAUST_CASES,
    run: run_normal_attack_exhaust_case,
    suite: "replay::tests_build_2026_09_season::normal_attack_strategies_copy_only_once_and_exhaust_only_the_real_card",
    valid: normal_attack_strategies_copy_only_once_case_table_is_valid,
    cases: [
        normal_attack_strategies_copy_only_once_and_exhaust_only_the_real_card_temporary_false => 0,
        normal_attack_strategies_copy_only_once_and_exhaust_only_the_real_card_temporary_true => 1,
    ]
}

#[test]
fn normal_attack_strategies_copy_only_once_and_exhaust_only_the_real_card_copy_once() {
    let cloud = original_card_definition_by_id(1_020_002).unwrap();
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.cards[0] = cloud;
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![5005];
    let mut state = ReplayState::test_from_fixture(&battle);
    let mut drawn = super::DrawnCard {
        source_slot: 1,
        card: basic_attack().into(),
        fallback_basic_attack: false,
        skipped_slots: Vec::new(),
        skipped_opening_slots: Vec::new(),
        fate_398_skipped_fifth_grid: false,
    };
    state.apply_verified_xian_mo_normal_attack_copy(PlayerSide::P1, &mut drawn);
    assert_eq!(
        state.test_actor_card(PlayerSide::P1, 1).id,
        1_010_002,
        "copy caps a regular rank-three card to rank two"
    );
    assert_eq!(drawn.card.id, 1_010_002);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 1_020_002);
    assert_eq!(state.p1.fate.normal_attack_copy_previous, 0);

    // 26125017: gridNumber != 0 — slot 0 has no previous grid, so a slot-0
    // 普攻 neither copies nor consumes the charge.
    let mut state0 = ReplayState::test_from_fixture(&battle);
    let mut drawn0 = super::DrawnCard {
        source_slot: 0,
        card: basic_attack().into(),
        fallback_basic_attack: false,
        skipped_slots: Vec::new(),
        skipped_opening_slots: Vec::new(),
        fate_398_skipped_fifth_grid: false,
    };
    state0.apply_verified_xian_mo_normal_attack_copy(PlayerSide::P1, &mut drawn0);
    assert_eq!(drawn0.card.id, super::BASIC_ATTACK_ID);
    assert_eq!(state0.p1.fate.normal_attack_copy_previous, 1);
}

#[test]
fn dragon_first_damage_and_first_attack_marks_use_shared_damage_paths_once() {
    // 26125017: 3001 no longer consumes on first damage; battle-start grants
    // the opponent JianGong(103) += otherParams[0] = 1 directly.
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![3001];
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.status.attack_reduction, 1);
    state.p2.core.defense = 100;
    state.apply_attack(PlayerSide::P1, 10, 0);
    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.status.attack_reduction, 1);
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p2.xian_mo_strategies = vec![3004];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.apply_damage(PlayerSide::P1, 1, false, false, false);
    assert_eq!(
        state.p2.fate.dragon_first_attack_stealth, 1,
        "non-Attack preserves the mark"
    );
    state.apply_attack(PlayerSide::P1, 10, 0);
    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p2.fate.dragon_first_attack_stealth, 0);
    assert_eq!(state.p2.elements.water_stealth, 1);
}

#[test]
fn dragon_first_anima_bonus_precedes_multiplier_and_is_consumed_by_positive_requests_only() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![3011];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.chance.qi_cai_ling_he = 2;
    state.gain_anima(PlayerSide::P1, 0);
    assert_eq!(state.p1.fate.next_anima_gain_bonus, 2);
    state.gain_anima(PlayerSide::P1, 1);
    assert_eq!(state.p1.core.anima, 6);
    assert_eq!(state.p1.fate.next_anima_gain_bonus, 0);
    state.gain_anima(PlayerSide::P1, 1);
    assert_eq!(state.p1.core.anima, 8);
}

const FIRST_SLOT_ENCHANT_CASES: &[StrategyCase] = &[
    StrategyCase { strategy: 13004 },
    StrategyCase { strategy: 13008 },
    StrategyCase { strategy: 13009 },
    StrategyCase { strategy: 13010 },
    StrategyCase { strategy: 13011 },
    StrategyCase { strategy: 13013 },
    StrategyCase { strategy: 13014 },
    StrategyCase { strategy: 13015 },
    StrategyCase { strategy: 13016 },
    StrategyCase { strategy: 13017 },
    StrategyCase { strategy: 13018 },
    StrategyCase { strategy: 13019 },
];

fn run_first_slot_enchant_case(case: &StrategyCase) {
    let strategy = case.strategy;
    let water = original_card_definition_by_id(7_000_019).unwrap();
    {
        let mut battle = fixture(deck(water.clone()), deck(basic_attack()));
        battle.players.p1.active_slot_count = 4;
        battle.players.p1.xian_mo_strategies = vec![strategy];
        let mut state = ReplayState::test_from_fixture(&battle);
        state.p1.core.hp = 10;
        state.p1.turn.lose_hp_count = 50;
        state.p1.astrology.hexagram = 2;
        state.p1.sword.sword_intent = 3;
        state.p2.status.weakness = 2;
        // 13017/13014 早于星弈连单独执行（CardActionBase.OnBeforeExecuted
        // :3300 先于 :3676），此处走新入口以保留原断言。
        if strategy == 13017 {
            state.apply_fumo_13017_before_star_hooks(PlayerSide::P1, 0);
        } else if strategy == 13014 {
            state.apply_fumo_13014_before_star_hooks(PlayerSide::P1, 0);
        } else {
            state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &water, 0, false);
        }
        match strategy {
            13004 => assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (20, 40)),
            13008 => assert_eq!((state.p1.core.guard, state.p1.core.temporary_guard), (2, 2)),
            13009 => assert_eq!(
                (
                    state.p1.core.defense,
                    state.p1.sword.water_month_sword_formation
                ),
                (6, 2)
            ),
            13010 => assert!(
                state.p1.astrology.star_slots.contains(&1)
                    && state.p1.astrology.star_slots.contains(&2)
                    && state.p1.astrology.star_slots.contains(&3)
            ),
            13011 => assert_eq!(state.p1.core.anima, 2),
            13013 => assert_eq!(
                (
                    state.p1.astrology.hexagram,
                    state.p1.core.max_hp,
                    state.p1.core.hp
                ),
                (3, 33, 13)
            ),
            13014 => assert_eq!((state.p1.core.physique, state.p1.core.defense), (2, 0)),
            13015 => {
                assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (24, 24));
                assert_eq!(
                    state.dream_mirage_value(PlayerSide::P2, DreamMirageValue::ConsumeNextCard),
                    1
                );
            }
            13016 => assert_eq!((state.p1.sword.sword_intent, state.p2.core.hp), (5, 28)),
            13017 => assert_eq!(
                (state.p2.status.internal_injury, state.p1.core.defense),
                (1, 3)
            ),
            13018 => assert_eq!(
                state.dream_mirage_value(
                    PlayerSide::P1,
                    DreamMirageValue::ExtraWaterMomentumTurnEnd
                ),
                1
            ),
            13019 => assert_eq!(state.p1.core.hp, 16),
            _ => unreachable!(),
        }
        // Temporary cards and the same slot's later repetitions must not add
        // a second enchantment, including heals and resource gain hooks.
        let before = (state.p1.snapshot(), state.p2.snapshot());
        state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &water, 0, true);
        assert_eq!((state.p1.snapshot(), state.p2.snapshot()), before);
        state.p1.deck.slots[0].used = true;
        state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &water, 0, false);
        assert_eq!((state.p1.snapshot(), state.p2.snapshot()), before);
    }
}

crate::data_driven_cases! {
    table: FIRST_SLOT_ENCHANT_CASES,
    run: run_first_slot_enchant_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions",
    valid: xian_mo_first_slot_enchantments_case_table_is_valid,
    cases: [
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13004 => 0,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13008 => 1,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13009 => 2,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13010 => 3,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13011 => 4,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13013 => 5,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13014 => 6,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13015 => 7,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13016 => 8,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13017 => 9,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13018 => 10,
        xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_strategy_13019 => 11,
    ]
}

#[test]
fn xian_mo_first_slot_enchantments_preserve_source_amounts_and_exclusions_metal_13005() {
    let metal = original_card_definition_by_id(7_000_016).unwrap();
    let mut battle = fixture(deck(metal.clone()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![13005];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &metal, 0, false);
    assert_eq!(state.p1.core.defense, 5);
}

#[test]
fn xian_mo_13003_upgrades_the_physical_card_only_on_its_first_use() {
    let cloud = original_card_definition_by_id(1_000_002).unwrap();
    let mut battle = fixture(deck(cloud), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![13003];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 1_010_002);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 1_010_002);
}

const NEW_CLOUD_EXTREME_CASES: &[TierCase] = &[
    TierCase { tier: 0 },
    TierCase { tier: 10_000 },
    TierCase { tier: 20_000 },
];

fn run_new_cloud_extreme_case(case: &TierCase) {
    let tier = case.tier;
    let moon = original_card_definition_by_id(311 + tier).unwrap();
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(moon.clone()), deck(basic_attack())));
    state.p1.sword.sword_intent = 3;
    state.test_apply_card_effect(PlayerSide::P1, &moon, 0);
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.p1.core.attack_bonus, 3);
    assert_eq!(
        state.p2.core.hp,
        30 - (moon.attack.unwrap() + 3) * moon.attack_count.unwrap()
    );

    let shadow = original_card_definition_by_id(1_000_093 + tier).unwrap();
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(shadow.clone()), deck(basic_attack())));
    state.p1.core.anima = 10;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.turn.successful_action_again_total, 0);
    state.modify_extra_actions(PlayerSide::P1, 1);
    assert!(state.test_consume_action_again(PlayerSide::P1, &basic_attack(), 0));
    state.test_play_actor_turn();
    assert_eq!(
        state.p1.turn.action_again_count, 0,
        "turn end clears the per-turn counter"
    );
    assert_eq!(state.p1.turn.successful_action_again_total, 2);
}

crate::data_driven_cases! {
    table: NEW_CLOUD_EXTREME_CASES,
    run: run_new_cloud_extreme_case,
    suite: "replay::tests_build_2026_09_season::new_cloud_and_extreme_cards_keep_conversion_and_battle_lifetime_action_counts",
    valid: new_cloud_and_extreme_cards_case_table_is_valid,
    cases: [
        new_cloud_and_extreme_cards_keep_conversion_and_battle_lifetime_action_counts_tier_0 => 0,
        new_cloud_and_extreme_cards_keep_conversion_and_battle_lifetime_action_counts_tier_10000 => 1,
        new_cloud_and_extreme_cards_keep_conversion_and_battle_lifetime_action_counts_tier_20000 => 2,
    ]
}

#[test]
fn xian_mo_1009_upgrades_visible_last_three_slots_once_before_actor_openings() {
    let cards = vec![
        basic_attack(),
        original_card_definition_by_id(1_000_002).unwrap(),
        original_card_definition_by_id(430).unwrap(),
        original_card_definition_by_id(1_010_002).unwrap(),
    ];
    let mut battle = fixture(cards.clone(), cards);
    battle.players.p1.active_slot_count = 4;
    battle.players.p2.active_slot_count = 4;
    battle.players.p1.xian_mo_strategies = vec![1009];
    // BattleExecuter:1002 用 IsSwitchActive(side, 1009)，它先要求该侧持有策略；
    // 持有者关闭开关后，未持有的一侧缺省 tempData 也不算开启。
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("1009".into(), 1);
    let state = ReplayState::test_from_fixture(&battle);
    for side in [PlayerSide::P1, PlayerSide::P2] {
        assert_eq!(state.test_actor_card(side, 1).id, 1_000_002);
        assert_eq!(state.test_actor_card(side, 3).id, 1_010_002);
    }
    battle.players.p1.xian_mo_temp_datas.clear();
    let state = ReplayState::test_from_fixture(&battle);
    for side in [PlayerSide::P1, PlayerSide::P2] {
        for (index, expected) in [0, 1_020_002, 430, 1_020_002].into_iter().enumerate() {
            assert_eq!(state.test_actor_card(side, index).id, expected);
        }
    }
    battle
        .players
        .p2
        .xian_mo_temp_datas
        .insert("1009".into(), 1);
    let state = ReplayState::test_from_fixture(&battle);
    // 未持有一侧的开关取值不参与判定：持有者仍开启则双方照常升阶。
    assert_eq!(state.test_actor_card(PlayerSide::P1, 1).id, 1_020_002);
    assert_eq!(state.test_actor_card(PlayerSide::P2, 3).id, 1_020_002);
}

#[test]
fn xian_mo_7064_removes_all_shared_tear_stacks_at_twenty_hp_and_7135_respects_switches() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![7064];
    battle.players.p1.fate_strategies = vec![344];
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.fate.vermilion_bird_tear, 2);
    state.p1.core.hp = 21;
    state.test_play_actor_turn();
    assert_eq!(
        (state.p1.fate.vermilion_bird_tear, state.p1.core.guard),
        (2, 0)
    );
    state.p1.core.hp = 20;
    state.test_play_actor_turn();
    assert_eq!(
        (state.p1.fate.vermilion_bird_tear, state.p1.core.guard),
        (0, 1)
    );

    battle.players.p1.fate_strategies.clear();
    battle.players.p1.xian_mo_strategies = vec![7135];
    battle.players.p2.xian_mo_strategies = vec![7135];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.fate.resonance_mystic_heart_enter_profound, 2);
    assert_eq!(state.p2.fate.resonance_mystic_heart_enter_profound, 2);
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("7135".into(), 1);
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.fate.resonance_mystic_heart_enter_profound, 1);
    assert_eq!(state.p2.fate.resonance_mystic_heart_enter_profound, 1);
}

#[test]
fn xian_mo_switch_1005_blocks_one_owner_turn_and_clears_pending_extra_action() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![1005];
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.turn.action_again_blocked_turns, 1);
    state.modify_extra_actions(PlayerSide::P1, 1);
    assert!(!state.test_consume_action_again(PlayerSide::P1, &basic_attack(), 0));
    assert_eq!(state.p1.turn.extra_actions, 0);
    assert_eq!(state.p1.turn.action_again_count, 0);
    state.p1.tick_turn_end_statuses();
    state.modify_extra_actions(PlayerSide::P1, 1);
    assert!(state.test_consume_action_again(PlayerSide::P1, &basic_attack(), 0));
    assert_eq!(state.p1.turn.action_again_count, 1);
    assert_eq!(state.p2.turn.action_again_blocked_turns, 1);
}

#[test]
fn xian_mo_7017_and_fate_109_consume_shared_sword_formation_guard_in_both_source_hooks() {
    let sword_formation = original_card_definition_by_id(48).unwrap();
    let mut battle = fixture(deck(sword_formation), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![7017];
    battle.players.p1.fate_strategies = vec![109];
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.fate.sword_formation_guard, 2);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.sword_formation_guard, 0);
    assert_eq!((state.p1.core.defense, state.p1.core.guard), (18, 2));
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!((state.p1.core.defense, state.p1.core.guard), (26, 2));
}

const XIAN_MO_7035_FATE_CASES: &[FateVecCase] =
    &[FateVecCase { fate: &[] }, FateVecCase { fate: &[148] }];

fn run_xian_mo_7035_fate_case(case: &FateVecCase) {
    let fate = case.fate.to_vec();
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![7035];
    battle.players.p1.fate_strategies = fate;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.gain_attack_bonus(PlayerSide::P1, 4);
    assert_eq!(
        (state.p1.core.attack_bonus, state.p1.elements.wood_thorn),
        (3, 1)
    );
}

crate::data_driven_cases! {
    table: XIAN_MO_7035_FATE_CASES,
    run: run_xian_mo_7035_fate_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19",
    valid: xian_mo_7035_shares_attack_conversion_case_table_is_valid,
    cases: [
        xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19_fate_empty => 0,
        xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19_fate_148 => 1,
    ]
}

const XIAN_MO_23045_CARD_CASES: &[CardProbeCase] = &[
    CardProbeCase { id: 0 },
    CardProbeCase { id: 19 },
    CardProbeCase { id: 10_019 },
];

fn run_xian_mo_23045_card_case(case: &CardProbeCase) {
    let id = case.id;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![23045];
    let mut state = ReplayState::test_from_fixture(&battle);
    let card = original_card_definition_by_id(id).unwrap();
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &card, 0, false);
    assert_eq!(state.p1.core.anima, i64::from(id == 19));
}

crate::data_driven_cases! {
    table: XIAN_MO_23045_CARD_CASES,
    run: run_xian_mo_23045_card_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_23045_requires_exact_card_19",
    valid: xian_mo_23045_requires_exact_card_19_case_table_is_valid,
    cases: [
        xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19_card_0 => 0,
        xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19_card_19 => 1,
        xian_mo_7035_shares_attack_conversion_and_23045_requires_exact_card_19_card_10019 => 2,
    ]
}

#[test]
fn xian_mo_7129_upgrades_only_the_first_eligible_description() {
    let anima_card = original_card_definition_by_id(24).unwrap();
    let mut battle = fixture(
        vec![basic_attack(), anima_card.clone(), anima_card],
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 3;
    battle.players.p1.xian_mo_strategies = vec![7129];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 0);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 1).id, 10_024);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 2).id, 24);
}

const CALLBACK_TIER_CASES: &[CallbackTierCase] = &[
    CallbackTierCase { tier: 0, base: 430 },
    CallbackTierCase { tier: 0, base: 431 },
    CallbackTierCase { tier: 0, base: 432 },
    CallbackTierCase { tier: 0, base: 433 },
    CallbackTierCase { tier: 0, base: 434 },
    CallbackTierCase { tier: 0, base: 439 },
    CallbackTierCase { tier: 0, base: 448 },
    CallbackTierCase {
        tier: 10_000,
        base: 430,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 431,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 432,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 433,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 434,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 439,
    },
    CallbackTierCase {
        tier: 10_000,
        base: 448,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 430,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 431,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 432,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 433,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 434,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 439,
    },
    CallbackTierCase {
        tier: 20_000,
        base: 448,
    },
];

fn run_callback_tier_case(case: &CallbackTierCase) {
    let tier = case.tier;
    let base = case.base;
    let card = original_card_definition_by_id(tier + base).unwrap();
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.attack_bonus = 9;
    state.p2.core.defense = 3;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    match base {
        430 => {
            assert_eq!(state.p2.core.hp, 30 - (card.other_params[0] - 3));
            assert_eq!(state.p2.core.defense, 0);
            assert_eq!(state.p1.turn.attack_segments_performed, 0);
        }
        431 => {
            assert_eq!(state.p1.core.defense, card.defense.unwrap());
            assert_eq!(state.p1.core.max_hp, 30 + card.other_params[0]);
            assert_eq!(state.p1.core.hp, 10);
        }
        432 => {
            assert_eq!(state.p1.core.anima, card.anima.unwrap());
            assert_eq!(state.p1.core.defense, card.defense.unwrap());
        }
        433 | 434 => assert_eq!(state.p1.core.hp, 10 + card.other_params[0]),
        439 => {
            assert_eq!(state.p1.core.physique, card.physique.unwrap());
            assert_eq!(
                state.p1.fate.mystic_heart_enter_profound,
                card.other_params[0]
            );
            assert_eq!(state.p1.turn.agility, card.other_params[1]);
        }
        448 => assert_eq!(state.p1.fate.paint_finishing_touch, card.other_params[0]),
        _ => unreachable!(),
    }
}

crate::data_driven_cases! {
    table: CALLBACK_TIER_CASES,
    run: run_callback_tier_case,
    suite: "replay::tests_build_2026_09_season::xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts",
    valid: xian_mo_callback_cards_preserve_exact_case_table_is_valid,
    cases: [
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_430 => 0,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_431 => 1,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_432 => 2,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_433 => 3,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_434 => 4,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_439 => 5,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_448 => 6,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10430 => 7,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10431 => 8,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10432 => 9,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10433 => 10,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10434 => 11,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10439 => 12,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_10448 => 13,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20430 => 14,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20431 => 15,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20432 => 16,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20433 => 17,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20434 => 18,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20439 => 19,
        xian_mo_callback_cards_preserve_exact_damage_healing_and_resource_contracts_card_20448 => 20,
    ]
}

#[test]
fn xian_mo_strategy_1007_grants_paint_to_both_sides_when_switch_is_active() {
    // XianMoStrategyFunctions.cs:882-886; XianMoStrategyConfig 1007 otherParams=[1].
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![1007];
    battle.players.p2.xian_mo_strategies = vec![1007];
    let state = ReplayState::<true>::from_fixture(&battle, true)
        .expect("implemented xianMo strategy should pass strict construction");
    assert_eq!(state.p1.fate.paint_finishing_touch, 2);
    assert_eq!(state.p2.fate.paint_finishing_touch, 2);

    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("1007".into(), 1);
    let state = ReplayState::<true>::from_fixture(&battle, true)
        .expect("disabled switch should not block the battle");
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    assert_eq!(state.p2.fate.paint_finishing_touch, 1);

    // 13002 按 OnPlayCard 源码实现（4 次 GetNextParam）；每个战斗相关仙魔策略都已实现，不再有 fail-closed 样例。
    battle.players.p1.xian_mo_strategies = vec![];
    battle.players.p2.xian_mo_strategies = vec![13002];
    assert!(ReplayState::<true>::from_fixture(&battle, true).is_ok());

    // 12003 AddCardPool：战斗外（卡池），不影响战斗 → 放行。
    battle.players.p2.xian_mo_strategies = vec![12003];
    assert!(ReplayState::<true>::from_fixture(&battle, true).is_ok());
}

#[test]
fn xian_mo_strategy_5009_repeats_fu_mo_grid_card_at_or_below_zhu_ji() {
    // 伏魔 5009 (CardActionBase.cs:1470→IL_3658→IL_3703→IL_37d2)：primary
    // 之后把伏魔格上的筑基及以下非隐藏牌再执行一次，无递归。
    // 取证见 fu_mo_grid_repeats_card 文档（R-855f195a）。
    let card = original_card_definition_by_id(1_000_002).expect("missing LianQi 云剑•飞刺");
    let mut twin = fixture(deck(card.clone()), deck(basic_attack()));
    let mut baseline_state = ReplayState::test_from_fixture(&twin);
    baseline_state.test_execute_one_card(PlayerSide::P1);
    let played_slot = baseline_state
        .test_events()
        .iter()
        .find(|event| {
            event.kind == super::ReplayEventKind::CardCompleted && event.actor == PlayerSide::P1
        })
        .and_then(|event| event.slot)
        .expect("baseline must complete one selected card");

    twin.players.p1.xian_mo_strategies = vec![5009];
    twin.players
        .p1
        .xian_mo_temp_datas
        .insert("5009".into(), played_slot as i64);
    let mut twin_state = ReplayState::test_from_fixture(&twin);
    twin_state.test_execute_one_card(PlayerSide::P1);
    // 云剑•飞刺 5 攻 + 连云追加 3 攻：primary 打 5 并建立连云，再执行打
    // 5+3=8（原版每次 ExecuteEffect 独立 snapshot 连云）。基线 30-5=25，
    // 伏魔格命中则 30-5-8=17。
    assert_eq!(baseline_state.p2.core.hp, 25);
    assert_eq!(twin_state.p2.core.hp, 17);
    assert_eq!(twin_state.test_actor_card(PlayerSide::P1, 0).id, card.id);

    // 伏魔格不命中：与基线一致，不重复。
    let mut off_grid = fixture(deck(card.clone()), deck(basic_attack()));
    off_grid.players.p1.xian_mo_strategies = vec![5009];
    off_grid
        .players
        .p1
        .xian_mo_temp_datas
        .insert("5009".into(), ((played_slot + 1) % 8) as i64);
    let mut off_state = ReplayState::test_from_fixture(&off_grid);
    off_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(off_state.p2.core.hp, baseline_state.p2.core.hp);
}

const HIDDEN_435_COUNT_CASES: &[HiddenAttackCountCase] = &[
    HiddenAttackCountCase {
        card_id: 435,
        attack_count: 1,
    },
    HiddenAttackCountCase {
        card_id: 10_435,
        attack_count: 2,
    },
    HiddenAttackCountCase {
        card_id: 20_435,
        attack_count: 3,
    },
];

fn run_hidden_435_count_case(case: &HiddenAttackCountCase) {
    let card_id = case.card_id;
    let attack_count = case.attack_count;
    let active = original_card_definition_by_id(card_id)
        .unwrap_or_else(|| panic!("missing current-build CardConfig {card_id}"));
    let battle = fixture(deck(active.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, card_id);
    let before = state.p2.core.hp;

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(
        state.evaluation_error.is_none(),
        "card {card_id}: {:?}",
        state.evaluation_error
    );
    assert_eq!(
        state.p2.core.hp,
        before - 5 * attack_count,
        "card {card_id}"
    );
    assert!(
        action_again,
        "card {card_id} should grant its configured repeat"
    );
    assert_eq!(state.p1.dream_mirage.action_again_limit, 2);
    assert_eq!(state.p1.turn.keyin_shan_feng, 1);
    assert_eq!(state.p1.turn.action_again_count, 1);

    state.apply_turn_end_phases(PlayerSide::P1);
    assert_eq!(state.p1.dream_mirage.action_again_limit, 1);
    assert_eq!(state.p1.turn.keyin_shan_feng, 0);
}

crate::data_driven_cases! {
    table: HIDDEN_435_COUNT_CASES,
    run: run_hidden_435_count_case,
    suite: "replay::tests_build_2026_09_season::hidden_card_435_uses_attack_count_and_clears_its_turn_limit_at_turn_end",
    valid: hidden_card_435_uses_attack_count_case_table_is_valid,
    cases: [
        hidden_card_435_uses_attack_count_and_clears_its_turn_limit_at_turn_end_card_435 => 0,
        hidden_card_435_uses_attack_count_and_clears_its_turn_limit_at_turn_end_card_10435 => 1,
        hidden_card_435_uses_attack_count_and_clears_its_turn_limit_at_turn_end_card_20435 => 2,
    ]
}

#[test]
fn card_22_grants_hexagram_before_star_power_so_talent_61_stacks_with_the_card() {
    // Card_22.OnExecuted（25621897–25981480 一致）：灵气 → 卦象 → 星位 → 星力。卦象先触发仙命 61
    // 的首次星力，再叠加牌面星力；原版 oracle 实测 星力 2。
    let mut battle = fixture(
        deck(original_card_definition_by_id(10_022).unwrap()),
        deck(basic_attack()),
    );
    battle.players.p1.talents = vec![61];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p1.astrology.hexagram, 1);
}

#[test]
fn zongzi_94_heals_and_grants_xian_guo_bao_zong() {
    let card = original_card_definition_by_id(94).unwrap();
    assert_eq!((card.id, card.other_params.to_vec()), (94, vec![8, 1]));
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 18);
    assert_eq!(state.p1.fate.xian_guo_bao_zong, 1);
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn zongzi_100_purges_selected_negative_status() {
    let card = original_card_definition_by_id(100).unwrap();
    assert_eq!((card.id, card.other_params.to_vec()), (100, vec![6, 1]));
    let mut battle = fixture(deck(card.clone()), deck(basic_attack()));
    battle.decision_tape = vec![100];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 10;
    state.p1.status.internal_injury = 3;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 16);
    assert_eq!(state.p1.status.internal_injury, 2);
    assert!(state.decision_tape.is_empty());
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn zongzi_100_minus_one_param_is_heal_only() {
    let card = original_card_definition_by_id(100).unwrap();
    let mut battle = fixture(deck(card.clone()), deck(basic_attack()));
    battle.decision_tape = vec![-1];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 10;
    state.p1.status.internal_injury = 3;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 16);
    assert_eq!(state.p1.status.internal_injury, 3);
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn zongzi_102_double_heal_then_anima_bonus_heal() {
    let card = original_card_definition_by_id(102).unwrap();
    assert_eq!((card.id, card.other_params.to_vec()), (102, vec![1, 5, 4]));
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.anima = 5;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.status.internal_injury, 1);
    assert_eq!(state.p1.core.anima, 4);
    assert_eq!(state.p1.core.hp, 10 + 5 + 5 + 4);
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn zongzi_102_without_anima_skips_bonus_heal() {
    let card = original_card_definition_by_id(102).unwrap();
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.anima = 0;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 10 + 5 + 5);
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn zongzi_103_grants_anima_hp_and_bixie() {
    let card = original_card_definition_by_id(103).unwrap();
    assert_eq!((card.id, card.other_params.to_vec()), (103, vec![3, 1]));
    assert_eq!(card.anima, Some(1));
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.anima = 0;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.core.hp, 13);
    assert_eq!(state.p1.fate.exorcism, 1);
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn xian_guo_bao_zong_converts_negative_gain_to_recovery() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.fate.xian_guo_bao_zong = 1;
    state.p1.fate.exorcism = 2;
    let receipt = state.add_actor_negative_status(PlayerSide::P1, 100, 2);
    assert_eq!(receipt.applied, 0);
    assert_eq!(state.p1.status.internal_injury, 0);
    assert_eq!(state.p1.status.recovery, 2);
    assert_eq!(state.p1.fate.xian_guo_bao_zong, 0);
    // 转换 return 在辟邪之前：辟邪层数不动。
    assert_eq!(state.p1.fate.exorcism, 2);
}

#[test]
fn xian_guo_bao_zong_skips_meditation() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.fate.xian_guo_bao_zong = 1;
    let receipt = state.add_actor_negative_status(PlayerSide::P1, 367, 2);
    assert_eq!(receipt.applied, 2);
    assert_eq!(state.p1.status.meditation, 2);
    assert_eq!(state.p1.status.recovery, 0);
    assert_eq!(state.p1.fate.xian_guo_bao_zong, 1);
}
