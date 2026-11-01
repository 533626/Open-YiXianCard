use super::cards_synthetic_oracle_verified_secret_misc::{
    first_strike_enables_rear_move, lost_max_hp_layers_from_delta,
    thunder_mindset_attack_bonus_percent, unceasing_momentum_refill,
    vitality_bloom_overflow_max_hp_gain,
};
use super::effect_invocation::EffectInvocationKind;
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
    super::test_support::make_player(cards, 5, 50, None, 1, None)
}

fn fixture(active: CardDefinition) -> BattleFixture {
    super::test_support::default_fixture(player(deck(active)), player(deck(basic_attack())))
}

#[test]
fn wood_spirit_vine_dispatches_typed_body_and_reads_battle_add_hp_count() {
    let vine = original_card(7_000_064);
    let mut active = ReplayState::test_from_fixture(&fixture(vine.clone()));
    active.p1.hp_mutation.add_hp_count = 21;
    active.activate_element(PlayerSide::P1, Element::Wood);

    assert!(active.probe_has_typed_card_effect(PlayerSide::P1, &vine, 0));
    assert!(!active.test_execute_one_card(PlayerSide::P1));
    assert_eq!(active.p2.core.hp, 30);
    assert_eq!(active.p1.turn.attack_segments_performed, 5);

    let mut turn_only = ReplayState::test_from_fixture(&fixture(vine.clone()));
    turn_only.p1.dream_mirage.turn_hp_gained = 21;
    turn_only.activate_element(PlayerSide::P1, Element::Wood);
    assert!(!turn_only.test_execute_one_card(PlayerSide::P1));
    assert_eq!(turn_only.p2.core.hp, 42);
    assert_eq!(turn_only.p1.turn.attack_segments_performed, 2);

    let mut wood_inactive = ReplayState::test_from_fixture(&fixture(vine));
    wood_inactive.p1.hp_mutation.add_hp_count = 21;
    assert!(!wood_inactive.test_execute_one_card(PlayerSide::P1));
    assert_eq!(wood_inactive.p2.core.hp, 42);
    assert_eq!(wood_inactive.p1.turn.attack_segments_performed, 2);
}

#[test]
fn vitality_bloom_installs_after_printed_gain_then_expands_for_later_overflow() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(4_000_048)));
    state.p1.core.hp = 45;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.hp, 52);
    assert_eq!(state.p1.core.max_hp, 57);
    assert_eq!(state.p1.fate.vitality_bloom, 1);

    state.modify_actor_hp(PlayerSide::P1, 10, false, false);
    assert_eq!(state.p1.core.hp, 62);
    assert_eq!(state.p1.core.max_hp, 62);
    assert_eq!(vitality_bloom_overflow_max_hp_gain(&state.p1, 3), 3);
}

#[test]
fn first_strike_attacks_before_enabling_first_use_rear_move() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(4_000_049)));

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.p1.fate.first_strike, 1);
    assert!(first_strike_enables_rear_move(&state.p1));
    assert!(state.check_rear_move(PlayerSide::P1, false));
}

#[test]
fn iron_tree_blossoms_doubles_both_gains_only_for_zero_percent_roll() {
    let mut regular_fixture = fixture(original_card(4_000_057));
    regular_fixture.decision_tape = vec![1];
    let mut regular = ReplayState::test_from_fixture(&regular_fixture);
    regular.p1.core.hp = 30;
    regular.test_execute_one_card(PlayerSide::P1);
    assert_eq!((regular.p1.core.hp, regular.p1.core.max_hp), (45, 52));

    let mut lucky_fixture = fixture(original_card(4_000_057));
    lucky_fixture.decision_tape = vec![0];
    let mut lucky = ReplayState::test_from_fixture(&lucky_fixture);
    lucky.p1.core.hp = 30;
    lucky.test_execute_one_card(PlayerSide::P1);
    assert_eq!((lucky.p1.core.hp, lucky.p1.core.max_hp), (54, 54));
}

#[test]
fn meteor_heaven_fall_spends_all_star_power_before_repeating_damage() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(4_000_065)));
    state.p1.astrology.star_power = 3;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.astrology.star_power, 0);
    assert_eq!(state.p2.core.hp, 26);
}

#[test]
fn thunder_mindset_applies_percent_only_to_thunder_card_body_attacks() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(4_000_064)));
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.astrology.hexagram, 1);
    assert_eq!(state.p1.astrology.thunder_mindset, 40);

    assert_eq!(
        thunder_mindset_attack_bonus_percent(&state.p1, "震雷剑", false),
        40
    );
    assert_eq!(
        thunder_mindset_attack_bonus_percent(&state.p1, "普通攻击", false),
        0
    );
    assert_eq!(
        thunder_mindset_attack_bonus_percent(&state.p1, "震雷剑", true),
        0
    );

    let mut thunder_context = basic_attack();
    thunder_context.name = "震雷剑".to_string().into();
    let physical = state.p1.deck.slots[0].card.clone();
    state.begin_effect_invocation(
        PlayerSide::P1,
        &thunder_context,
        &thunder_context,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        false,
    );
    state.apply_attack(PlayerSide::P1, 10, 0);
    assert_eq!(state.p2.core.hp, 36);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn shared_after_card_attacks_do_not_receive_thunder_mindset_twice() {
    let mut thunder = basic_attack();
    thunder.name = "震雷剑".to_string().into();
    let mut state = ReplayState::test_from_fixture(&fixture(thunder));
    state.p1.astrology.thunder_mindset = 40;
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 10;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 36);
    assert!(state.effect_invocation_stack.is_empty());
}

#[test]
fn burning_sky_flame_uses_floor_of_tracked_lost_max_hp_on_every_hit() {
    let mut active_fixture = fixture(original_card(7_020_045));
    active_fixture.players.p1.initial_anima = 3;
    let mut active = ReplayState::test_from_fixture(&active_fixture);
    active.p2.core.hp = 100;
    active.p2.core.max_hp = 100;
    active.activate_element(PlayerSide::P1, Element::Fire);
    active.modify_target_max_hp(PlayerSide::P1, -8);
    active.p2.core.hp = 100.min(active.p2.core.max_hp);
    assert_eq!(active.p2.core.lost_max_hp_count, 8);

    assert!(!active.test_execute_one_card(PlayerSide::P1));
    assert_eq!(active.p2.core.hp, 65);

    let mut inactive_fixture = fixture(original_card(7_020_045));
    inactive_fixture.players.p1.initial_anima = 3;
    let mut inactive = ReplayState::test_from_fixture(&inactive_fixture);
    inactive.p2.core.hp = 92;
    inactive.p2.core.max_hp = 92;
    inactive.p2.core.lost_max_hp_count = 8;
    inactive.test_execute_one_card(PlayerSide::P1);
    assert_eq!(inactive.p2.core.hp, 71);

    assert_eq!(lost_max_hp_layers_from_delta(-8), 8);
    assert_eq!(lost_max_hp_layers_from_delta(4), 0);
}

#[test]
fn burning_sky_flame_defaults_a_missing_lost_max_hp_step_to_one() {
    let mut card = original_card(7_020_045);
    card.other_params = Default::default();
    let mut state = ReplayState::test_from_fixture(&fixture(basic_attack()));
    state.p2.core.hp = 100;
    state.p2.core.max_hp = 100;
    state.p2.core.lost_max_hp_count = 8;
    state.activate_element(PlayerSide::P1, Element::Fire);

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_misc_card_effect(
            PlayerSide::P1,
            &card,
            0,
            7_000_045,
        ),
        Some(true)
    );

    assert_eq!(state.p2.core.hp, 55);
    assert_eq!(state.p1.turn.attack_segments_performed, 3);
}

#[test]
fn lost_max_hp_tracks_the_full_negative_request_before_zero_clamping() {
    let mut state = ReplayState::test_from_fixture(&fixture(basic_attack()));
    state.p2.core.hp = 3;
    state.p2.core.max_hp = 3;

    assert_eq!(state.modify_actor_max_hp(PlayerSide::P2, -10).applied, -3);
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (0, 0));
    assert_eq!(state.p2.core.lost_max_hp_count, 10);
}

#[test]
fn check_wu_xing_covers_used_element_generation_long_ma_and_deck_override() {
    let mut state = ReplayState::test_from_fixture(&fixture(basic_attack()));

    assert!(!state.check_wu_xing(PlayerSide::P1, Element::Wood));
    state.p1.elements.activated_earth = 1;
    assert!(state.check_wu_xing(PlayerSide::P1, Element::Earth));
    assert!(!state.check_wu_xing(PlayerSide::P1, Element::Fire));
    state.p1.elements.activated_earth = 0;

    for (used, generated) in [
        (Element::Metal, Element::Water),
        (Element::Water, Element::Wood),
        (Element::Wood, Element::Fire),
        (Element::Fire, Element::Earth),
        (Element::Earth, Element::Metal),
    ] {
        state.p1.elements.last_element = Some(used);
        assert!(state.check_wu_xing(PlayerSide::P1, used));
        assert!(state.check_wu_xing(PlayerSide::P1, generated));
    }

    state.p1.identity.talents.push(137);
    state.p1.elements.last_element = Some(Element::Fire);
    for required in [
        Element::Metal,
        Element::Water,
        Element::Wood,
        Element::Earth,
    ] {
        assert!(state.check_wu_xing(PlayerSide::P1, required));
    }
    state.p1.elements.last_element = Some(Element::Metal);
    assert!(state.check_wu_xing(PlayerSide::P1, Element::Fire));
    assert!(!state.check_wu_xing(PlayerSide::P1, Element::Wood));

    state.p1.identity.talents.clear();
    state.p1.elements.last_element = None;
    state.p1.elements.long_ma_spirit = 1;
    assert!(state.check_wu_xing(PlayerSide::P1, Element::Wood));
    state.p1.elements.long_ma_spirit = 0;
    for dream_card_id in [7_030_077, 7_040_077] {
        let mut changed = state.p1.deck.slots[0].card.definition().clone();
        changed.id = dream_card_id;
        state.p1.deck.slots[0].card = changed.into();
        assert!(state.check_wu_xing(PlayerSide::P1, Element::Wood));
    }
}

#[test]
fn kiln_earth_keeps_branches_independent_and_earth_reads_fire_loss_first() {
    let kiln = original_card(7_000_049);

    let mut fire = ReplayState::test_from_fixture(&fixture(kiln.clone()));
    fire.activate_element(PlayerSide::P1, Element::Fire);
    fire.test_execute_one_card(PlayerSide::P1);
    assert_eq!((fire.p2.core.hp, fire.p2.core.max_hp), (42, 42));
    assert_eq!(fire.p2.core.lost_max_hp_count, 8);
    assert_eq!(fire.p1.core.defense, 0);

    let mut earth = ReplayState::test_from_fixture(&fixture(kiln.clone()));
    earth.activate_element(PlayerSide::P1, Element::Earth);
    earth.p2.core.lost_max_hp_count = 6;
    earth.test_execute_one_card(PlayerSide::P1);
    assert_eq!((earth.p2.core.hp, earth.p2.core.max_hp), (50, 50));
    assert_eq!(earth.p1.core.defense, 7);

    let mut both = ReplayState::test_from_fixture(&fixture(kiln));
    both.activate_element(PlayerSide::P1, Element::Fire);
    both.activate_element(PlayerSide::P1, Element::Earth);
    both.test_execute_one_card(PlayerSide::P1);
    assert_eq!(both.p2.core.lost_max_hp_count, 8);
    assert_eq!(both.p1.core.defense, 8);
}

#[test]
fn kiln_earth_card_body_honors_the_unconditional_dream_deck_override() {
    let mut dream_fixture = fixture(original_card(7_000_049));
    dream_fixture.players.p1.cards[1] = original_card(7_030_077);
    dream_fixture.players.p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&dream_fixture);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (42, 42));
    assert_eq!(state.p2.core.lost_max_hp_count, 8);
    assert_eq!(state.p1.core.defense, 8);
}

#[test]
fn wood_kindles_fire_heals_before_spending_ceiling_half_hp_for_attack() {
    let mut state = ReplayState::test_from_fixture(&fixture(original_card(7_000_051)));
    state.p1.core.hp = 40;
    state.p1.core.max_hp = 100;
    state.p2.core.hp = 100;
    state.p2.core.max_hp = 100;
    state.activate_element(PlayerSide::P1, Element::Wood);
    state.activate_element(PlayerSide::P1, Element::Fire);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.hp, 28);
    assert_eq!(state.p2.core.hp, 71);
}

#[test]
fn unceasing_momentum_fills_empty_pool_then_refills_when_it_reaches_zero() {
    let card = original_card(10_000_053);
    let mut empty = ReplayState::test_from_fixture(&fixture(card.clone()));
    empty.test_execute_one_card(PlayerSide::P1);
    assert_eq!(empty.p1.beng.momentum, 4);
    assert_eq!(empty.p1.beng.unceasing_momentum, 4);
    empty.modify_momentum(PlayerSide::P1, -4);
    assert_eq!(empty.p1.beng.momentum, 4);

    let mut occupied_fixture = fixture(card);
    occupied_fixture.players.p1.initial_momentum = 2;
    let mut occupied = ReplayState::test_from_fixture(&occupied_fixture);
    occupied.test_execute_one_card(PlayerSide::P1);
    assert_eq!(occupied.p1.beng.momentum, 2);
    assert_eq!(occupied.p1.beng.unceasing_momentum, 4);
    assert_eq!(unceasing_momentum_refill(&occupied.p1), 0);
}
