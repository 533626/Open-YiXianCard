use super::*;

#[test]
fn external_repeat_sources_are_checked_after_each_prior_effect() {
    let expected = || FixtureExpected {
        winner_side: PlayerSide::P1,
        actor_turn_count: 1,
        hp_delta_p1_minus_p2: 0,
        final_hp: None,
    };

    // The Plum effect itself installs 崩拳•双影. Its later source branch must
    // see that new layer, yielding Plum + Double Shadow + primary.
    let mut double_shadow = test_card(10_000_060, 10_000_060, "崩拳•双影动态重复");
    double_shadow.attack = Some(1);
    let mut beng_state = ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(double_shadow),
        filler_cards(basic_attack_test_card()),
        expected(),
    ));
    beng_state.p1.fate.plum_blossom_twice = 1;
    beng_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(beng_state.p1.turn.used_card_count, 3);
    assert_eq!(beng_state.p1.turn.attack_segments_performed, 3);
    assert_eq!(beng_state.p1.beng.beng_quan_double_shadow, 2);

    // The same contract applies to the later 聚焰 branch.
    let mut gather_flame = test_card(270, 270, "幻•火灵聚炎动态重复");
    gather_flame.other_params = vec![0, 0];
    let mut fire_state = ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(gather_flame),
        filler_cards(basic_attack_test_card()),
        expected(),
    ));
    fire_state.activate_element(PlayerSide::P1, Element::Fire);
    fire_state.p1.fate.plum_blossom_twice = 1;
    fire_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(fire_state.p1.turn.used_card_count, 3);
    assert_eq!(
        fire_state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::RepeatNextFireOrEarth,),
        2,
    );
}

#[test]
fn profound_spirit_healing_reads_anima_and_debuffs_after_inner_injury() {
    let card = original_card_definition_by_id(10_000_091).expect("missing 玄灵愈体 card config");
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
    state.test_configure_p1(|player| {
        player.core.hp = 100;
        player.core.max_hp = 100;
    });
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.status.internal_injury, 2);
    assert_eq!(state.p1.core.max_hp, 105);
    assert_eq!(state.p1.core.hp, 105);
}

#[test]
fn profound_spirit_healing_respects_exorcism_before_counting_debuffs() {
    let card = original_card_definition_by_id(10_000_091).expect("missing 玄灵愈体 card config");
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
    state.test_configure_p1(|player| {
        player.core.hp = 100;
        player.core.max_hp = 100;
        player.fate.exorcism = 2;
    });
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.status.internal_injury, 0);
    assert_eq!(state.p1.fate.exorcism, 0);
    assert_eq!(state.p1.core.max_hp, 103);
    assert_eq!(state.p1.core.hp, 103);
}

#[test]
fn spirit_formation_echo_base_lifecycle_keeps_primary_action_again_snapshot() {
    let mut upgraded = original_card_definition_by_id(9_020_026)
        .expect("missing rarity-two clear-intestine purple fern");
    upgraded.name = "测试灵阵".to_string();
    let mut fixture = minimal_fixture(
        filler_cards(upgraded),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.fate_strategies = vec![135];
    let mut state = ReplayState::test_from_fixture(&fixture);

    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.turn.used_card_count, 2);
    assert_eq!(state.p1.turn.action_again_count, 1);
    assert_eq!(state.p1.formations.spirit_formation_echo, 0);
}

#[test]
fn hound_and_alchemy_read_explicit_rarity_instead_of_id_segments() {
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
    let mut state = ReplayState::test_from_fixture(&fixture);

    let mut embryo = test_card(19, 19, "澄心剑胚显式稀有度");
    embryo.rarity = Some(2);
    state.p1.chance.you_ming_xu_hun_quan = 1;
    let transformed = state.apply_you_ming_xu_hun_quan_replacement(
        PlayerSide::P1,
        DrawnCard {
            source_slot: 0,
            card: embryo,
            fallback_basic_attack: false,
            skipped_slots: Vec::new(),
            skipped_opening_slots: Vec::new(),
            fate_398_skipped_fifth_grid: false,
        },
    );
    assert_eq!(transformed.card.id, 20_000);

    let mut upgraded_id_zero_rarity = test_card(10_000, 0, "升级段但零稀有度");
    upgraded_id_zero_rarity.rarity = Some(0);
    state.p1.chance.you_ming_xu_hun_quan = 1;
    let transformed = state.apply_you_ming_xu_hun_quan_replacement(
        PlayerSide::P1,
        DrawnCard {
            source_slot: 0,
            card: upgraded_id_zero_rarity.clone(),
            fallback_basic_attack: false,
            skipped_slots: Vec::new(),
            skipped_opening_slots: Vec::new(),
            fate_398_skipped_fifth_grid: false,
        },
    );
    assert_eq!(transformed.card.id, 0);

    let mut alchemy_state = ReplayState::test_from_fixture(&fixture);
    alchemy_state.p1.ronghui.alchemy_pot = 1;
    let transformed = alchemy_state.apply_ronghui_alchemy_pot_transform(
        PlayerSide::P1,
        DrawnCard {
            source_slot: 0,
            card: upgraded_id_zero_rarity,
            fallback_basic_attack: false,
            skipped_slots: Vec::new(),
            skipped_opening_slots: Vec::new(),
            fate_398_skipped_fifth_grid: false,
        },
    );
    assert_eq!(transformed.card.id, 10_000);
    assert_eq!(
        (alchemy_state.p1.core.hp, alchemy_state.p1.core.max_hp),
        (21, 21),
    );
    assert_eq!(
        (alchemy_state.p2.core.hp, alchemy_state.p2.core.max_hp),
        (39, 39),
    );
}

#[test]
fn resonance_ten_spirit_sword_preserves_real_chain_and_updates_temp_flag() {
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
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.identity.talent_resonance_id = Some(10);
    state.p1.sword.cloud_chain = 3;
    let spirit_sword = original_card_definition_by_id(49).expect("missing spirit sword card");

    state.apply_card_classification_completed_hooks(PlayerSide::P1, &spirit_sword);
    assert_eq!(state.p1.sword.cloud_chain, 3);
    assert!(state.p1.identity.talent_resonance_temp_flags.contains(&10));

    state.apply_card_classification_completed_hooks(PlayerSide::P1, &basic_attack_test_card());
    assert_eq!(state.p1.sword.cloud_chain, 0);
    assert!(!state.p1.identity.talent_resonance_temp_flags.contains(&10));
}

#[test]
fn hp_cost_post_payment_hooks_include_one_shot_refund_and_full_combo() {
    let costly_beng = test_card(10_000_093, 10_000_093, "耗命组合顺序契约");
    let fixture = minimal_fixture(
        filler_cards(costly_beng.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 50;
    state.p1.core.max_hp = 100;
    state.p1.turn.blood_shadow = 1;
    state.p1.beng.beng_quan_bounce = 1;
    state.p1.beng.beng_quan_return_profound = 1;
    state.p1.beng.next_beng_quan_hp_cost_damage = 1;
    state.p1.identity.fate_strategies.push(347);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpCostRefund, 1);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::HpGainDefense, 1);
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpGainDefense, 1);

    state.apply_after_hp_cost_hooks(PlayerSide::P1, &costly_beng, 4, true);

    assert_eq!(state.p1.core.hp, 62);
    assert_eq!(state.p1.turn.agility, 4);
    assert_eq!(state.p1.core.defense, 8);
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p2.core.hp, 22);
    assert_eq!(state.p1.turn.blood_shadow, 0);
    assert_eq!(state.p1.beng.beng_quan_bounce, 0);
    assert_eq!(state.p1.beng.beng_quan_return_profound, 1);
    assert_eq!(state.p1.beng.next_beng_quan_hp_cost_damage, 0);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextHpCostRefund),
        0,
    );
    assert_eq!(state.p1.fate.hot_blood_to_qi_triggered, 1);
}

#[test]
fn card_322_missing_opponent_same_slot_runs_empty_body_without_error() {
    let replica = test_card(9_000_322, 322, "逍遥•复刻 baseId 契约");
    let fixture = minimal_fixture(
        filler_cards(replica.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.deck.slots.clear();

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.deck.slots[0].card.id, replica.id);
    assert!(state.p1.deck.slots[0].used);
    assert_eq!(state.p2.core.hp, 30);
}
