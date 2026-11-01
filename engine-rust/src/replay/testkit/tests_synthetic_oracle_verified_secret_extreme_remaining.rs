use super::cards_dream_mirage::DreamMirageValue;
use super::effect_invocation::EffectInvocationKind;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck_with(cards: &[i64]) -> Vec<CardDefinition> {
    super::test_support::resize_deck(
        cards.iter().map(|id| original_card(*id)).collect(),
        basic_attack(),
    )
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 100, None, active_slot_count, None)
}

fn fixture(p1_cards: &[i64], active_slot_count: usize) -> BattleFixture {
    super::test_support::default_fixture(
        player(deck_with(p1_cards), active_slot_count),
        player(deck_with(&[0]), 1),
    )
}

#[test]
fn endless_collapse_executes_five_same_slot_temporary_cards_without_paying_their_costs() {
    let mut fixture = fixture(&[10_000_065], 1);
    fixture.decision_tape = vec![10_000_001, 10_000_002, 10_000_003, 10_000_001, 10_000_002];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.hp = 200;
    state.p2.core.max_hp = 200;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 91);
    assert_eq!(state.p2.core.hp, 156);
    assert_eq!(state.p1.core.defense, 6);
    assert_eq!(state.p1.beng.beng_quan_defense, 2);
    assert_eq!(state.p1.beng.beng_quan_bounce, 1);
    assert_eq!(state.p1.turn.hp_cost_cards_used, 6);
    assert_eq!(state.p1.turn.used_card_count, 6);
    assert_eq!(state.active_effect_actual_damage(), 0);
    assert_eq!(state.p1.deck.slots[0].card.id, 10_000_065);
    assert!(state.p1.deck.slots[0].skipped);
    assert!(state.decision_tape.is_empty());
}

#[test]
fn endless_collapse_settles_sword_intent_per_card_without_consuming_outer_repeat() {
    let mut fixture = fixture(&[10_000_065], 1);
    fixture.decision_tape = vec![10_000_003; 5];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.sword.sword_intent = 3;
    state.p1.beng.beng_quan_double_shadow = 1;
    state.p2.core.hp = 500;
    state.p2.core.max_hp = 500;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.turn.attack_segments_performed, 5);
    assert_eq!(state.p2.core.hp, 457);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalActualDamage,),
        43,
    );
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.p1.beng.beng_quan_double_shadow, 1);
}

#[test]
fn temporary_semantics_endless_collapse_clears_and_classifies_each_nested_effect() {
    let mut fixture = fixture(&[10_000_065], 1);
    fixture.decision_tape = vec![10_000_003; 5];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.hp = 500;
    state.p2.core.max_hp = 500;

    let outer = original_card(10_000_065);
    let physical = state.p1.deck.slots[0].card.clone();
    let is_beng = state.is_dream_mirage_intrinsic_beng_quan(PlayerSide::P1, &outer);
    state.begin_effect_invocation(
        PlayerSide::P1,
        &outer,
        &outer,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        is_beng,
    );

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_extreme_remaining_card_effect(
            PlayerSide::P1,
            &outer,
            0,
            10_000_065,
        ),
        Some(false),
    );
    assert_eq!(state.active_effect_attacks(), 0);
    assert_eq!(state.active_effect_actual_damage(), 0);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalActualDamage,),
        40,
    );

    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn endless_collapse_temporary_cards_skip_first_slot_and_swift_burn_hooks() {
    let mut fixture = fixture(&[10_000_065], 1);
    fixture.players.p1.talents = vec![57, 149, 140];
    fixture.decision_tape = vec![20; 5];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 92);
    assert_eq!(state.p1.core.max_hp, 101);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.sword.water_month_sword_formation, 1);
    assert_eq!(state.p1.elements.swift_burn_seal, 1);
}

#[test]
fn endless_collapse_skips_non_temporary_dark_void_and_clears_each_rear_move_check() {
    let mut dark_fixture = fixture(&[10_000_065], 1);
    dark_fixture.decision_tape = vec![1_020_051; 5];
    let mut dark_state = ReplayState::test_from_fixture(&dark_fixture);

    dark_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(dark_state.p2.core.hp, 100);
    assert_eq!(dark_state.p1.sword.dark_void_sword_formation_art, 65);

    let mut rear_fixture = fixture(&[10_000_065], 1);
    rear_fixture.decision_tape = vec![12; 5];
    let mut rear_state = ReplayState::test_from_fixture(&rear_fixture);

    rear_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(rear_state.p1.fate.used_rear_move_check, 0);
}

#[test]
fn endless_collapse_reports_the_first_missing_required_decision() {
    let fixture = fixture(&[10_000_065], 1);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_execute_one_card(PlayerSide::P1);

    assert!(state.evaluation_error.as_ref().is_some_and(|error| error
        .to_string()
        .contains("card:10000065:temporaryBengQuan:1")));
}

#[test]
fn return_to_simplicity_freezes_post_gain_physique_for_basic_attacks() {
    let fixture = fixture(&[10_000_066, 0], 2);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 110;
    state.p1.core.max_hp = 110;
    state.p1.core.physique = 10;
    state.p1.core.physique_limit = 5;

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.beng.return_to_simplicity, 3);
    assert_eq!(
        (
            state.p1.core.physique,
            state.p1.core.hp,
            state.p1.core.max_hp
        ),
        (15, 115, 115)
    );

    state.apply_physique_amount(PlayerSide::P1, 5);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.turn.agility, 3);
    assert_eq!(state.p2.core.hp, 94);
}

#[test]
fn return_to_simplicity_defaults_a_missing_physique_step_to_one() {
    let mut card = original_card(10_000_066);
    card.other_params = Default::default();
    let mut state = ReplayState::test_from_fixture(&fixture(&[0], 1));
    state.p1.core.physique = 10;
    state.p1.core.physique_limit = 5;

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_extreme_remaining_card_effect(
            PlayerSide::P1,
            &card,
            0,
            10_000_066,
        ),
        Some(false)
    );

    assert_eq!(state.p1.core.physique, 15);
    assert_eq!(state.p1.beng.return_to_simplicity, 15);
}

#[test]
fn ultimate_quiet_mindset_only_raises_max_hp_then_heals_on_anima_gain() {
    let fixture = fixture(&[4_000_098], 1);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 70;
    state.p1.core.max_hp = 100;

    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (70, 120));
    assert_eq!(state.p1.fate.quiet_mindset, 1);

    state.gain_anima(PlayerSide::P1, 2);
    assert_eq!((state.p1.core.hp, state.p1.core.anima), (72, 2));
}

#[test]
fn ultimate_wood_patrol_applies_thorn_before_each_hit_and_uses_frozen_wood_check() {
    let active_fixture = fixture(&[7_000_105], 1);
    let mut state = ReplayState::test_from_fixture(&active_fixture);
    state.p1.core.hp = 80;
    state.p1.core.max_hp = 100;
    state.p1.core.anima = 1;
    state.p1.hp_mutation.add_hp_count = 1;
    state.activate_element(PlayerSide::P1, Element::Wood);

    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!((state.p1.core.hp, state.p1.core.anima), (82, 0));
    assert_eq!(state.p2.core.hp, 96);
    assert_eq!(state.p1.elements.wood_thorn, 1);

    let inactive_fixture = fixture(&[7_000_105], 1);
    let mut inactive = ReplayState::test_from_fixture(&inactive_fixture);
    inactive.p1.core.anima = 1;
    assert!(!inactive.test_execute_one_card(PlayerSide::P1));
    assert_eq!(inactive.p1.elements.last_element, Some(Element::Wood));
}

#[test]
fn wood_thorn_heals_after_lone_night_attack_value_is_frozen() {
    let fixture = fixture(&[7_000_105], 1);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 49;
    state.p1.core.max_hp = 100;
    state.p1.core.anima = 1;
    state.p1.status.lone_night_wolf = 4;
    state.activate_element(PlayerSide::P1, Element::Wood);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 51);
    assert_eq!(state.p2.core.hp, 92);
}

#[test]
fn ultimate_beng_tian_step_makes_basic_attack_inherit_and_consume_beng_quan_chuo() {
    let fixture = fixture(&[10_000_098, 0], 2);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.beng.beng_quan_chuo = 2;

    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.beng.beng_tian_step, 1);
    assert_eq!(state.p1.turn.agility, 0);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, 95);
    assert_eq!(state.p1.beng.beng_quan_chuo, 0);
}

#[test]
fn ultimate_beng_tian_step_extends_cost_and_repeat_hooks_to_non_beng_quan_cards() {
    let cost_fixture = fixture(&[10_000_003, 10_000_098, 10_000_012], 3);
    let mut cost_state = ReplayState::test_from_fixture(&cost_fixture);
    cost_state.test_execute_one_card(PlayerSide::P1);
    cost_state.test_execute_one_card(PlayerSide::P1);
    cost_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(cost_state.p1.beng.beng_quan_bounce, 0);
    assert_eq!(cost_state.p1.core.hp, 98);

    let repeat_fixture = fixture(&[10_000_060, 10_000_098, 0], 3);
    let mut repeat_state = ReplayState::test_from_fixture(&repeat_fixture);
    repeat_state.test_execute_one_card(PlayerSide::P1);
    repeat_state.test_execute_one_card(PlayerSide::P1);
    repeat_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(repeat_state.p1.beng.beng_quan_double_shadow, 0);
    assert_eq!(repeat_state.p1.turn.attack_segments_performed, 3);
}

#[test]
fn hao_ran_zheng_qi_consumes_full_reduce_loop_including_no_status_markers() {
    // Card_310.cs (幻•浩然正气) loops exactly otherParams[0] times and reads a
    // GetNextParam each iteration. Trailing -1 picks remove nothing but must
    // still be consumed, otherwise the leftover marker desyncs later decisions.
    let mut fixture = fixture(&[310], 1);
    fixture.decision_tape = vec![100, 100, -1, -1];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.status.internal_injury = 2;

    state.test_execute_one_card(PlayerSide::P1);

    // Both real reductions applied and every otherParams[0] pick consumed.
    assert_eq!(state.p1.status.internal_injury, 0);
    assert!(
        state.decision_tape.is_empty(),
        "trailing -1 markers must be consumed to keep the tape aligned"
    );
}

#[test]
fn endless_collapse_skips_negative_pick_as_noop() {
    // A -1 pick means the game found no eligible 崩拳 (cardItem.InitData(-1) is a
    // no-op ExecuteEffect), so it must be skipped rather than executed as the
    // record-only 走火入魔 placeholder.
    let mut fixture = fixture(&[10_000_065], 1);
    fixture.decision_tape = vec![-1, 10_000_003, 10_000_003, 10_000_003, 10_000_003];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.hp = 200;
    state.p2.core.max_hp = 200;

    state.test_execute_one_card(PlayerSide::P1);

    // One skipped pick + four executed temporaries (plus the outer card).
    assert_eq!(state.p1.turn.used_card_count, 5);
    assert!(state.decision_tape.is_empty());
}

#[test]
fn low_tier_meng_kuang_er_on_play_builds_kuang_jian() {
    // Card_1000087 at or below JinDan folds otherParams[1] straight into the
    // KuangJian buff (sword.frenzy_sword), not a detached used-count.
    let mut fixture = fixture(&[1_000_087], 1);
    fixture.players.p1.battle_start_hp = Some(50);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.hp = 200;
    state.p2.core.max_hp = 200;
    let before = state.p1.sword.frenzy_sword;

    state.test_execute_one_card(PlayerSide::P1);

    // +1 from the shared 狂剑-use hook and +1 from the card's own otherParams[1]
    // both land on the frenzy-sword counter (KuangJian), not a dead used-count.
    assert_eq!(state.p1.sword.frenzy_sword - before, 2);
}
