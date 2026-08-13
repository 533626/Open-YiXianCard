use super::*;

#[test]
fn temporary_effect_skips_entire_adjacent_after_card_hook() {
    let mut temporary = test_card(9_999_994, 1_000_005, "临时相邻后钩子契约");
    temporary.defense = Some(1);
    let star_point =
        original_card_definition_by_id(4_030_089).expect("missing high dream star-point card");
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 2;
    fixture.players.p1.cards[1] = star_point;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.star_slots.push(0);

    state.apply_temporary_card_effect(PlayerSide::P1, &temporary, 0);

    assert_eq!(state.p2.status.internal_injury, 0);
    state.apply_dream_mirage_adjacent_after_card_hooks(PlayerSide::P1, 0);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn temporary_after_skips_ordinary_segment_but_keeps_common_tail() {
    // The test exercises lifecycle hooks, so use the audited printed-defense
    // carrier instead of inventing an executable Card_* id.
    let mut card = test_card(9_999_993, 1_000_005, "临时 OnAfter 分段契约");
    card.defense = Some(1);
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
    let arm = |state: &mut ReplayState| {
        state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoard, 1);
        state.p1.formations.fortune_avoid_misfortune = 1;
        state.p1.formations.fortune_avoid_misfortune_defense = 3;
        state.p1.beng.triggered_startled_touch = 2;
    };

    let mut temporary = ReplayState::test_from_fixture(&fixture);
    arm(&mut temporary);
    temporary.apply_temporary_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(temporary.p1.core.defense, 1);
    assert_eq!(temporary.p1.core.anima, 0);
    assert_eq!(temporary.p1.astrology.star_power, 0);
    assert_eq!(temporary.p1.formations.fortune_avoid_misfortune, 1);
    assert_eq!(
        temporary.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoardTriggered),
        0
    );
    assert_eq!(
        temporary.p2.core.hp, 28,
        "common startled-touch tail still runs"
    );

    let mut ordinary = ReplayState::test_from_fixture(&fixture);
    arm(&mut ordinary);
    ordinary.apply_regular_after_card_effect_hooks(PlayerSide::P1, &card, 0, false);

    assert_eq!(ordinary.p1.core.defense, 3);
    assert_eq!(ordinary.p1.core.anima, 1);
    assert_eq!(ordinary.p1.astrology.star_power, 1);
    assert_eq!(ordinary.p1.formations.fortune_avoid_misfortune, 0);
    assert_eq!(
        ordinary.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoardTriggered),
        1
    );
    assert_eq!(ordinary.p2.core.hp, 28);
}

#[test]
fn dream_star_board_consumes_two_charges_and_skips_star_slots() {
    let dream_star_board =
        original_card_definition_by_id(4_020_084).expect("missing dream star board card");
    let ordinary_card = basic_attack_test_card();
    let fixture = minimal_fixture(
        filler_cards(dream_star_board.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.deck.slots[1].card = ordinary_card.clone();
    state.p1.astrology.star_slots.push(0);
    state.test_apply_card_effect(PlayerSide::P1, &dream_star_board, 0);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoardLowRealm),
        2
    );
    for _ in 0..3 {
        state.test_apply_card_effect(PlayerSide::P1, &ordinary_card, 1);
    }
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoardLowRealm),
        0
    );

    let mut star_slot_state = ReplayState::test_from_fixture(&fixture);
    star_slot_state.p1.deck.slots[1].card = ordinary_card.clone();
    star_slot_state.p1.astrology.star_slots.push(0);
    star_slot_state.p1.astrology.star_slots.push(1);
    star_slot_state.test_apply_card_effect(PlayerSide::P1, &dream_star_board, 0);
    star_slot_state.test_apply_card_effect(PlayerSide::P1, &ordinary_card, 1);
    assert_eq!(star_slot_state.p1.core.anima, 0);
    assert_eq!(star_slot_state.p1.astrology.star_power, 0);
    assert_eq!(
        star_slot_state
            .dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamStarBoardLowRealm,),
        2
    );
}

#[test]
fn gather_flame_adds_a_complete_execute_effect_lifecycle() {
    let mut fire = test_card(197, 197, "火灵•完整生命周期契约");
    fire.defense = Some(1);
    fire.hp_cost = Some(1);
    fire.career_name = Some("QinShi".to_string());
    fire.other_params = vec![2];
    let mut fixture = minimal_fixture(
        filler_cards(fire),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.initial_momentum_limit = Some(99);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::RepeatNextFireOrEarth, 1);
    state.p1.beng.beng_mei_mindset = 2;
    state.p1.formations.fortune_avoid_misfortune = 2;
    state.p1.formations.fortune_avoid_misfortune_defense = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));

    assert_eq!(state.p1.core.hp, 29, "printed HP cost is still paid once");
    assert_eq!(state.p1.beng.momentum, 4, "before hook runs twice");
    assert_eq!(state.p1.core.defense, 8, "body and after hook run twice");
    assert_eq!(state.p1.core.guard, 5, "second body observes hadUsed");
    assert_eq!(state.p1.formations.fortune_avoid_misfortune, 0);
    assert_eq!(state.p1.music.music_cards_played, 2);
    assert_eq!(state.p1.turn.used_card_count, 2);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::RepeatNextFireOrEarth),
        0
    );
    assert!(state.p1.deck.slots[0].used);
}

#[test]
fn next_beng_quan_physique_is_consumed_before_a_non_beng_quan_card() {
    let ordinary = basic_attack_test_card();
    let fixture = minimal_fixture(
        filler_cards(ordinary),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextBengQuanPhysique, 2);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.max_hp, 32);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextBengQuanPhysique),
        0
    );
}

#[test]
fn public_adjacent_beng_quan_is_snapshotted_before_cost() {
    let mut hp_cost_card = test_card(9_999_999, 145, "test hp-cost card");
    hp_cost_card.hp_cost = Some(4);
    let adjacent_beng_tian = original_card_definition_by_id(10_030_080)
        .expect("missing high dream heaven-crushing step");
    let mut fixture = minimal_fixture(
        filler_cards(hp_cost_card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 2;
    fixture.players.p1.cards[1] = adjacent_beng_tian;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.beng.next_beng_quan_hp_cost_damage = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 26);
    assert_eq!(state.p2.core.hp, 26);
    assert_eq!(state.p1.beng.next_beng_quan_hp_cost_damage, 0);
}

#[test]
fn hp_cost_hooks_use_printed_cost_and_public_adjacent_beng_identity() {
    let mut split_mountain = test_card(10_000_005, 10_000_005, "劈山掌费用契约");
    split_mountain.hp_cost = Some(4);
    let adjacent_beng_tian = original_card_definition_by_id(10_030_080)
        .expect("missing high dream heaven-crushing step");
    let mut fixture = minimal_fixture(
        filler_cards(split_mountain),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 2;
    fixture.players.p1.cards[1] = adjacent_beng_tian;
    fixture.players.p2.initial_defense = 100;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 20;
    state.p1.core.physique = 6;
    state.p1.turn.blood_shadow = 1;
    state.p1.beng.next_beng_quan_hp_cost_damage = 1;
    state.p1.beng.beng_quan_bounce = 1;
    state.p1.beng.beng_quan_return_profound = 1;
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::HpGainDefense, 3);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpGainDefense, 1);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 27, "actual cost is 4 - physique / 2 = 1");
    assert_eq!(state.p1.turn.agility, 4);
    assert_eq!(state.p1.core.defense, 8);
    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 86);
    assert_eq!(state.p1.turn.blood_shadow, 0);
    assert_eq!(state.p1.beng.next_beng_quan_hp_cost_damage, 0);
    assert_eq!(state.p1.beng.beng_quan_bounce, 0);
    assert_eq!(state.p1.beng.beng_quan_return_profound, 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::HpGainDefense),
        3
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpGainDefense),
        0
    );
}

#[test]
fn zero_printed_hp_cost_does_not_trigger_runtime_cost_hooks() {
    let runtime_cost_only = test_card(10_000_093, 10_000_093, "运行时费用契约");
    let fixture = minimal_fixture(
        filler_cards(runtime_cost_only.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 20;
    state.p1.turn.blood_shadow = 1;
    state.p1.beng.next_beng_quan_hp_cost_damage = 1;
    state.p1.beng.beng_quan_bounce = 1;
    state.p1.beng.beng_quan_return_profound = 1;
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::HpGainDefense, 2);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpGainDefense, 1);
    state.modify_actor_hp(PlayerSide::P1, -8, true, true);
    state.apply_after_hp_cost_hooks(PlayerSide::P1, &runtime_cost_only, 0, true);

    assert_eq!(state.p1.core.hp, 12);
    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p1.turn.agility, 0);
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.turn.blood_shadow, 1);
    assert_eq!(state.p1.beng.next_beng_quan_hp_cost_damage, 1);
    assert_eq!(state.p1.beng.beng_quan_bounce, 1);
    assert_eq!(state.p1.beng.beng_quan_return_profound, 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::HpGainDefense),
        2
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpGainDefense),
        1
    );
}

#[test]
fn private_before_card_hooks_run_per_effect_and_not_on_cost_failure() {
    let mut before_card = test_card(9_999_998, 145, "木灵阵测试");
    before_card.anima = Some(-1);
    let make_fixture = |initial_anima| {
        let mut fixture = minimal_fixture(
            filler_cards(before_card.clone()),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        fixture.players.p1.active_slot_count = 2;
        fixture.players.p1.initial_anima = initial_anima;
        fixture.players.p1.cards[1].name = "土灵测试".to_string();
        fixture
    };
    let arm_hooks = |state: &mut ReplayState| {
        state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::FiveElementsMarrow, 1);
        state.modify_mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::CounterElementAnima,
            1,
        );
        state.modify_mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::CounterElementDefense,
            2,
        );
    };

    let mut shortage = ReplayState::test_from_fixture(&make_fixture(0));
    arm_hooks(&mut shortage);

    assert!(!shortage.test_execute_one_card(PlayerSide::P1));
    assert_eq!(shortage.p1.turn.agility, 0);
    assert_eq!(shortage.p1.core.defense, 0);
    assert_eq!(shortage.p1.core.anima, 1);
    assert!(!shortage.p1.deck.slots[0].used);

    let mut repeated = ReplayState::test_from_fixture(&make_fixture(1));
    arm_hooks(&mut repeated);
    repeated.p1.fate.plum_blossom_twice = 1;

    let action_again = repeated.test_execute_one_card(PlayerSide::P1);

    assert!(action_again);
    assert_eq!(repeated.p1.turn.action_again_count, 1);
    // 五行天髓按原版在第一次灵阵效果时消耗 1 层（CardActionBase.
    // OnBeforeExecuted:2956-2962），故两次效果只 +10 敏捷；随后敏捷
    // 再次行动（10≥10）把 10 点敏捷耗尽，终值 0 而非旧实现的 10。
    assert_eq!(repeated.p1.turn.agility, 0);
    assert_eq!(repeated.p1.core.defense, 4);
    assert_eq!(repeated.p1.core.anima, 2);
    assert!(repeated.p1.deck.slots[0].used);
}
