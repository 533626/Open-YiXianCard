use super::*;
#[test]

fn nested_temporary_effect_frames_preserve_origin_effective_and_physical_identity() {
    let physical = basic_attack_test_card();
    let fixture = minimal_fixture(
        filler_cards(physical.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let outer = test_card(777, 777, "外层效果");
    let first = test_card(778, 145, "第一层临时效果");
    let second = test_card(779, 146, "第二层临时效果");
    state.begin_effect_invocation(
        PlayerSide::P1,
        &outer,
        &outer,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        true,
    );
    state.set_active_effect_after_action(true);
    state.set_active_effect_shatter_defense(2);
    state.set_active_effect_pending_sword_intent(3);
    state.add_active_effect_deferred_sword_intent_restore(1);
    state.add_active_effect_actual_damage(5);
    state.add_active_effect_wounded_count(1);
    let outer_frame = state.active_effect_frame().expect("outer frame").clone();
    let shared_local = std::rc::Rc::clone(&outer_frame.local);

    let mut first_spec = TemporaryInvocationSpec::physical(0);
    first_spec.invocation_slot = 7;
    first_spec.inherit_parent_beng_quan = true;
    state
        .with_temporary_effect_invocation(PlayerSide::P1, &first, first_spec, |state, _| {
            let first_frame = state.active_effect_frame().expect("first temporary frame");
            assert!(std::rc::Rc::ptr_eq(&shared_local, &first_frame.local));
            assert_eq!(first_frame.origin.card_id, outer.id);
            assert_eq!(first_frame.effective.card_id, first.id);
            assert_eq!(first_frame.physical.card.card_id, physical.id);
            assert_eq!(first_frame.invocation_slot, 7);
            assert_eq!(first_frame.kind, EffectInvocationKind::Temporary);
            assert_eq!(first_frame.phase, EffectInvocationPhase::BeforeBody);
            assert!(first_frame.effective.is_beng_quan);
            let first_frame = first_frame.clone();
            state.gain_active_effect_shatter_defense(1);
            state.update_active_effect_pending_sword_intent(4);
            state.add_active_effect_actual_damage(2);
            state.add_active_effect_wounded_count(1);

            let mut second_spec = TemporaryInvocationSpec::physical(0);
            second_spec.invocation_slot = 6;
            state
                .with_temporary_effect_invocation(
                    PlayerSide::P1,
                    &second,
                    second_spec,
                    |state, _| {
                        let second_frame =
                            state.active_effect_frame().expect("second temporary frame");
                        assert!(std::rc::Rc::ptr_eq(&shared_local, &second_frame.local));
                        assert_eq!(second_frame.origin.card_id, first.id);
                        assert_eq!(second_frame.effective.card_id, second.id);
                        assert_eq!(second_frame.physical.card.card_id, physical.id);
                        assert_eq!(second_frame.invocation_slot, 6);
                        assert!(!second_frame.effective.is_beng_quan);
                    },
                )
                .expect("second temporary invocation");
            assert_eq!(state.active_effect_frame(), Some(&first_frame));
            assert_eq!(state.p1.deck.slots[0].card.id, first.id);
        })
        .expect("first temporary invocation");

    assert_eq!(state.active_effect_frame(), Some(&outer_frame));
    // Unwinding restores the direct parent's effective CardConfig. The
    // physical identity stays separately anchored in the invocation frame.
    assert_eq!(state.p1.deck.slots[0].card.id, outer.id);
    assert_eq!(outer_frame.physical.card.card_id, physical.id);
    assert_eq!(state.active_effect_shatter_defense(), 3);
    assert_eq!(state.active_effect_pending_sword_intent(), 4);
    assert_eq!(state.active_effect_deferred_sword_intent_restore(), 1);
    assert_eq!(state.active_effect_actual_damage(), 7);
    assert_eq!(state.active_effect_wounded_count(), 2);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
    assert!(state.effect_invocation_stack.is_empty());
}

#[test]
fn settlement_consumes_and_clears_all_invocation_local_ledgers() {
    let card = test_card(780, 780, "效果结算局部账契约");
    let fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let physical = state.p1.deck.slots[0].card.clone();
    state.begin_effect_invocation(
        PlayerSide::P1,
        &card,
        &card,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        false,
    );
    state.p1.sword.sword_intent = 4;
    state.set_active_effect_shatter_defense(3);
    state.set_active_effect_pending_sword_intent(3);
    state.add_active_effect_deferred_sword_intent_restore(1);
    state.add_active_effect_actual_damage(7);
    state.add_active_effect_wounded_count(2);

    state.settle_sword_intent_after_card_effect(PlayerSide::P1);

    assert_eq!(state.p1.sword.sword_intent, 2);
    assert_eq!(state.active_effect_shatter_defense(), 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.active_effect_deferred_sword_intent_restore(), 0);
    assert_eq!(state.active_effect_actual_damage(), 0);
    assert_eq!(state.active_effect_wounded_count(), 0);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn next_attack_shatter_is_consumed_only_when_it_is_the_fallback_source() {
    for blocker in ["current-effect", "formation", "leaf"] {
        let card = test_card(781, 781, "碎防来源优先级契约");
        let mut fixture = minimal_fixture(
            filler_cards(card.clone()),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        fixture.players.p2.initial_defense = 30;
        let mut state = ReplayState::test_from_fixture(&fixture);
        let physical = state.p1.deck.slots[0].card.clone();
        state.begin_effect_invocation(
            PlayerSide::P1,
            &card,
            &card,
            &physical,
            0,
            0,
            EffectInvocationKind::Played,
            false,
        );
        state.p1.turn.next_attack_shatter_defense = 1;
        match blocker {
            "current-effect" => state.set_active_effect_shatter_defense(1),
            "formation" => state.p1.formations.shatter_formation = 1,
            "leaf" => state.p1.status.leaf_blade_flower = 1,
            _ => unreachable!(),
        }

        state.apply_attack(PlayerSide::P1, 3, 0);
        assert_eq!(
            state.p1.turn.next_attack_shatter_defense, 1,
            "{blocker} must cover the attack without spending the pending trigger"
        );

        state.set_active_effect_shatter_defense(0);
        state.p1.formations.shatter_formation = 0;
        state.p1.status.leaf_blade_flower = 0;
        state.apply_attack(PlayerSide::P1, 3, 0);
        assert_eq!(
            state.p1.turn.next_attack_shatter_defense, 0,
            "{blocker} removed: pending trigger must now be consumed"
        );
        state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
    }
}

#[test]
fn generic_temporary_effect_runs_pre_body_hooks_and_preserves_had_used_for_rear_move() {
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut cloud = test_card(9_999_992, 145, "云剑·临时");
    cloud.anima = Some(0);
    let mut cloud_state = ReplayState::test_from_fixture(&fixture);
    cloud_state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::SpiritCatCloud, 2);

    cloud_state.apply_temporary_card_effect(PlayerSide::P1, &cloud, 0);

    assert_eq!(cloud_state.p1.core.anima, 2);

    let rear_move = original_card_definition_by_id(12).expect("missing 飞鸿踏雪");
    let mut first_use = ReplayState::test_from_fixture(&fixture);
    assert!(!first_use.apply_temporary_card_effect(PlayerSide::P1, &rear_move, 0));
    assert!(first_use.p1.deck.slots[0].used);

    let mut repeated = ReplayState::test_from_fixture(&fixture);
    repeated.p1.deck.slots[0].used = true;
    assert!(repeated.apply_temporary_card_effect(PlayerSide::P1, &rear_move, 0));
}

#[test]
fn after_card_attack_does_not_inherit_main_card_attack_windows() {
    let card = test_card(170, 170, "牌后窗口清理契约");
    let mut fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p2.initial_defense = 10;
    let mut state = ReplayState::test_from_fixture(&fixture);
    let physical = state.p1.deck.slots[0].card.clone();
    state.begin_effect_invocation(
        PlayerSide::P1,
        &card,
        &card,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        false,
    );
    state.p1.sword.sharpness = 4;
    state.p1.status.weakness = 1;
    state.p1.status.mystic_soul = 1;
    state.set_active_effect_shatter_defense(1);
    state.p1.elements.no_sharpness_for_attack = 1;
    state.p1.turn.guaranteed_wound = 1;
    state.p1.beng.momentum = 1;
    state.p1.beng.momentum_multiplier = 3;
    state.p1.turn.current_turn_ignore_defense = 1;
    state.p1.turn.ignore_weakness_attacks = 1;
    state.p1.beng.beng_quan_chuo = 2;
    state.p1.beng.consumed_beng_quan_chuo = 2;
    state.p1.beng.triggered_startled_touch = 5;
    state.modify_mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CrashFistStarSeize, 1);
    state.modify_mirage_ronghui_value(
        PlayerSide::P1,
        MirageRonghuiValue::CrashFistStarSeizeConsumed,
        1,
    );
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFist, 1);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFistConsumed, 1);

    state.apply_regular_after_card_effect_hooks(PlayerSide::P1, &card, 0, false);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.sword.sharpness, 4);
    assert_eq!(state.p1.status.mystic_soul, 0);
    assert_eq!(state.active_effect_shatter_defense(), 0);
    assert_eq!(state.p1.elements.no_sharpness_for_attack, 0);
    assert_eq!(state.p1.turn.guaranteed_wound, 0);
    assert_eq!(state.p1.beng.momentum_multiplier, 0);
    assert_eq!(state.p1.turn.current_turn_ignore_defense, 0);
    assert_eq!(state.p1.turn.ignore_weakness_attacks, 0);
    assert_eq!(state.p1.beng.beng_quan_chuo, 0);
    assert_eq!(state.p1.beng.consumed_beng_quan_chuo, 0);
    assert_eq!(
        state.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CrashFistStarSeize),
        0
    );
    assert_eq!(
        state.mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::CrashFistStarSeizeConsumed,
        ),
        0
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFist),
        0
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFistConsumed),
        0
    );
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn dream_forge_fist_consumes_on_attacking_card_even_when_defense_absorbs_all_damage() {
    let card = test_card(171, 171, "梦锻拳后续攻击");
    let fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let physique_before = state.p1.core.physique;
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFist, 1);

    state.apply_dream_mirage_forge_fist_damage_to_physique(PlayerSide::P1, 0, 0);

    assert_eq!(state.p1.core.physique, physique_before);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFistConsumed),
        1
    );
    state.complete_dream_mirage_forge_fist_card(PlayerSide::P1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamForgeFist),
        0
    );
}

#[test]
fn qi_sinks_dantian_expires_before_shared_follow_up_but_not_for_temporary_cards() {
    let card = test_card(172, 172, "气沉丹田顺序契约");
    let fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut ordinary = ReplayState::test_from_fixture(&fixture);
    ordinary.p1.core.anima = 10;
    ordinary.p2.core.hp = 100;
    ordinary.p2.core.max_hp = 100;
    ordinary.modify_mirage_ronghui_value(
        PlayerSide::P1,
        MirageRonghuiValue::MirageAnimaAttackCards,
        1,
    );

    let physical = ordinary.p1.deck.slots[0].card.clone();
    ordinary.begin_effect_invocation(
        PlayerSide::P1,
        &card,
        &card,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        false,
    );
    ordinary.apply_attack(PlayerSide::P1, 10, 0);
    ordinary.p1.beng.triggered_startled_touch = 3;
    ordinary.apply_regular_after_card_effect_hooks(PlayerSide::P1, &card, 0, false);

    assert_eq!(ordinary.p2.core.hp, 77);
    assert_eq!(ordinary.active_effect_attacks(), 0);
    assert_eq!(
        ordinary.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::MirageAnimaAttackCards,),
        0
    );
    ordinary.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);

    let mut temporary = ReplayState::test_from_fixture(&fixture);
    temporary.p1.core.anima = 10;
    temporary.p2.core.hp = 100;
    temporary.p2.core.max_hp = 100;
    temporary.modify_mirage_ronghui_value(
        PlayerSide::P1,
        MirageRonghuiValue::MirageAnimaAttackCards,
        1,
    );
    let mut temporary_attack = test_card(9_999_993, 145, "气沉丹田临时攻击");
    temporary_attack.attack = Some(10);

    temporary.apply_temporary_card_effect(PlayerSide::P1, &temporary_attack, 0);

    assert_eq!(temporary.p2.core.hp, 80);
    assert_eq!(
        temporary.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::MirageAnimaAttackCards,),
        1
    );
}

#[test]
fn fate_on_play_updates_star_slots_before_body_and_precedes_counter_element() {
    let star = test_card(171, 171, "星弈测试");
    let mut star_fixture = minimal_fixture(
        filler_cards(star.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    star_fixture.players.p1.active_slot_count = 3;
    let mut star_state = ReplayState::test_from_fixture(&star_fixture);
    star_state.p1.astrology.star_slots.clear();
    star_state.p1.fate.qi_xing_lian_zhu = 1;

    star_state.apply_before_execute_effect_hooks(PlayerSide::P1, &star, 0, true);

    assert!(star_state.p1.astrology.star_slots.contains(&1));
    assert_eq!(star_state.p1.fate.qi_xing_lian_zhu, 0);

    let mut reverse_star_state = ReplayState::test_from_fixture(&star_fixture);
    reverse_star_state.p1.astrology.star_slots.clear();
    reverse_star_state.p1.fate.qi_xing_lian_zhu = 1;
    reverse_star_state.p1.fate.reverse_card_direction = 1;

    reverse_star_state.apply_before_execute_effect_hooks(PlayerSide::P1, &star, 0, true);

    assert_eq!(reverse_star_state.p1.astrology.star_slots, vec![2]);
    assert_eq!(reverse_star_state.p1.fate.qi_xing_lian_zhu, 0);

    let fire = test_card(7_000_069, 7_000_069, "火灵秘印");
    let metal = test_card(7_000_001, 7_000_001, "金灵印");
    let mut cards = filler_cards(fire.clone());
    cards[1] = metal;
    let mut counter_fixture = minimal_fixture(
        cards,
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    counter_fixture.players.p1.active_slot_count = 2;
    counter_fixture.players.p1.initial_anima = 2;
    counter_fixture.players.p1.fate_strategies = vec![345];
    let mut counter = ReplayState::test_from_fixture(&counter_fixture);
    counter.modify_mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::CounterElementAnima, 3);

    counter.apply_before_execute_effect_hooks(PlayerSide::P1, &fire, 0, false);

    assert_eq!(counter.p2.core.hp, 28);
    assert_eq!(counter.p2.core.max_hp, 28);
    assert_eq!(counter.p1.core.anima, 5);
}
