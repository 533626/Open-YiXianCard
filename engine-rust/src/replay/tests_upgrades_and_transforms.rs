use super::*;

#[test]
fn plum_blossom_twice_grants_repeat_buff_from_mei_kai_er_du() {
    let card = CardDefinition {
        id: 4_000_041,
        base_id: Some(4_000_041),
        name: "梅开二度".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(-1),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![2],
    };
    let mut fixture = minimal_fixture(
        filler_cards(crate::replay::support::basic_attack_card()),
        filler_cards(card.clone()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.first_player_side = PlayerSide::P2;
    fixture.players.p2.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.anima = 2;
    state.p2.deck.queue = vec![0];
    state.current_actor = PlayerSide::P2;
    state.test_apply_card_effect(PlayerSide::P2, &card, 0);
    assert_eq!(
        state.p2.fate.plum_blossom_twice, 1,
        "direct apply_card_effect"
    );
    state.p2.fate.plum_blossom_twice = 0;
    let _ = state.test_execute_one_card(PlayerSide::P2);
    assert_eq!(
        state.p2.fate.plum_blossom_twice, 1,
        "execute_card_transaction"
    );
}

#[test]
fn repeated_card_effects_mark_used_after_each_complete_effect() {
    let cicada = original_card_definition_by_id(4_010_036)
        .expect("missing current-build golden cicada sheds its shell");
    let mut fixture = minimal_fixture(
        filler_cards(cicada),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.base_max_hp = 100;
    fixture.players.p1.initial_anima = 1;
    fixture.players.p2.base_max_hp = 100;
    fixture.players.p2.initial_defense = 60;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 50;
    state.p2.core.hp = 50;
    state.p1.fate.plum_blossom_twice = 1;
    state.p1.fate.yellow_bird_behind = 10;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.defense, 24);
    assert_eq!(state.p1.core.guard, 1);
    assert_eq!(state.p1.core.hp, 74);
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 40);
    assert_eq!(state.p1.fate.used_rear_move_check, 0);
    assert!(state.p1.deck.slots[0].used);
}

#[test]
fn temporary_upgrades_wait_until_anima_cost_succeeds() {
    let cicada = original_card_definition_by_id(4_000_036)
        .expect("missing current-build golden cicada sheds its shell");
    let paint_fixture = minimal_fixture(
        filler_cards(cicada.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut paint = ReplayState::test_from_fixture(&paint_fixture);
    paint.p1.fate.paint_finishing_touch = 1;

    assert!(!paint.test_execute_one_card(PlayerSide::P1));
    assert_eq!(paint.p1.fate.paint_finishing_touch, 1);
    assert_eq!(paint.p1.deck.slots[0].card.id, cicada.id);
    assert!(!paint.p1.deck.slots[0].used);
    assert_eq!(paint.p1.deck.queue.first(), Some(&0));

    paint.test_execute_one_card(PlayerSide::P1);
    assert_eq!(paint.p1.fate.paint_finishing_touch, 0);
    assert_eq!(paint.p1.deck.slots[0].card.id, cicada.id + 10_000);
    assert!(paint.p1.deck.slots[0].used);

    let wood_shadow = original_card_definition_by_id(7_000_017)
        .expect("missing current-build wood spirit sparse shadow");
    let generating_fixture = minimal_fixture(
        filler_cards(wood_shadow.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut generating = ReplayState::test_from_fixture(&generating_fixture);
    generating.p1.fate.generating_interaction_upgrade = 1;
    generating.p1.elements.last_element = Some(Element::Water);

    assert!(!generating.test_execute_one_card(PlayerSide::P1));
    assert_eq!(generating.p1.fate.generating_interaction_upgrade, 1);
    assert_eq!(generating.p1.deck.slots[0].card.id, wood_shadow.id);
    assert!(!generating.p1.deck.slots[0].used);
    assert_eq!(generating.p1.deck.queue.first(), Some(&0));

    generating.test_execute_one_card(PlayerSide::P1);
    assert_eq!(generating.p1.fate.generating_interaction_upgrade, 0);
    assert_eq!(generating.p1.deck.slots[0].card.id, wood_shadow.id + 10_000);
    assert!(generating.p1.deck.slots[0].used);
}

#[test]
fn paint_finishing_touch_does_not_upgrade_no_upgrade_sword_embryo() {
    let sword_embryo = original_card_definition_by_id(19).expect("missing original 澄心剑胚");
    let mut fixture = minimal_fixture(
        filler_cards(sword_embryo),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 28,
            final_hp: None,
        },
    );
    fixture.players.p2.base_max_hp = 100;
    fixture.players.p1.talents = vec![92, 10_093, 10_096];
    fixture
        .players
        .p1
        .talent_temp_datas
        .insert("92".to_string(), 15);

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.fate.paint_finishing_touch = 1;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.deck.slots[0].card.id, 19);
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    assert_eq!(state.p2.core.hp, 72);
}

#[test]
fn meditation_pays_anima_shortage_before_the_card_is_rejected() {
    let step = original_card_definition_by_id(10_000_034)
        .expect("missing current-build sky-breaking step");
    let mut fixture = minimal_fixture(
        filler_cards(step),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.character_id = Some(4_000_003);
    fixture.players.p1.talents = vec![179];
    fixture.players.p1.fate_strategies = vec![160];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.max_hp = 50;
    state.p1.core.hp = 40;

    state.test_execute_one_card(PlayerSide::P1);

    assert!(state.p1.deck.slots[0].used);
    assert_eq!(state.p1.status.meditation, 0);
    assert_eq!(state.p1.core.hp, 43);
}

#[test]
fn execute_internal_card_transforms_wait_until_printed_cost_succeeds() {
    let mut replica = test_card(322, 322, "逍遥•复刻");
    replica.anima = Some(-1);
    let mut copied = test_card(9_999_997, 145, "同格契约牌");
    copied.attack = Some(1);
    let replica_fixture = minimal_fixture(
        filler_cards(replica.clone()),
        filler_cards(copied.clone()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut replica_state = ReplayState::test_from_fixture(&replica_fixture);

    assert!(!replica_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(replica_state.p1.deck.slots[0].card.id, replica.id);
    assert!(!replica_state.p1.deck.slots[0].used);
    assert_eq!(replica_state.p1.deck.queue.first(), Some(&0));

    replica_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(replica_state.p1.deck.slots[0].card.id, copied.id);
    assert!(replica_state.p1.deck.slots[0].used);

    let previous = original_card_definition_by_id(4_000_036)
        .expect("missing current-build golden cicada sheds its shell");
    let upgraded_basic = original_card_definition_by_id(10_000)
        .expect("missing current-build upgraded basic attack");
    let mut ordered_fixture = minimal_fixture(
        filler_cards(replica.clone()),
        filler_cards(upgraded_basic),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    ordered_fixture.players.p1.active_slot_count = 2;
    ordered_fixture.players.p1.cards[1] = previous.clone();
    let mut ordered = ReplayState::test_from_fixture(&ordered_fixture);
    ordered.p1.ronghui.five_emperors_upgrade = 1;
    ordered.p1.ronghui.alchemy_pot = 1;
    ordered.p1.ronghui.free_and_easy_tune = 1;
    ordered.p1.chance.you_ming_xu_hun_quan = 1;
    ordered.p1.fate.paint_finishing_touch = 1;

    assert!(!ordered.test_execute_one_card(PlayerSide::P1));
    assert_eq!(ordered.p1.ronghui.five_emperors_upgrade, 1);
    assert_eq!(ordered.p1.ronghui.alchemy_pot, 1);
    assert_eq!(ordered.p1.ronghui.free_and_easy_tune, 1);
    assert_eq!(ordered.p1.chance.you_ming_xu_hun_quan, 1);
    assert_eq!(ordered.p1.fate.paint_finishing_touch, 1);
    assert_eq!(ordered.p1.deck.slots[0].card.id, replica.id);
    assert_eq!((ordered.p1.core.hp, ordered.p1.core.max_hp), (30, 30));

    ordered.test_execute_one_card(PlayerSide::P1);
    assert_eq!(ordered.p1.ronghui.five_emperors_upgrade, 0);
    assert_eq!(ordered.p1.ronghui.alchemy_pot, 0);
    assert_eq!(ordered.p1.ronghui.free_and_easy_tune, 1);
    assert_eq!(ordered.p1.chance.you_ming_xu_hun_quan, 0);
    assert_eq!(ordered.p1.fate.paint_finishing_touch, 1);
    assert_eq!(ordered.p1.deck.slots[0].card.id, 10_000);
    assert_eq!((ordered.p1.core.hp, ordered.p1.core.max_hp), (30, 30));

    let mut costly_basic = basic_attack_test_card();
    costly_basic.anima = Some(-1);
    let mut tune_hound_fixture = minimal_fixture(
        filler_cards(costly_basic),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    tune_hound_fixture.players.p1.active_slot_count = 2;
    tune_hound_fixture.players.p1.initial_anima = 1;
    tune_hound_fixture.players.p1.cards[1] = previous;
    let mut tune_hound = ReplayState::test_from_fixture(&tune_hound_fixture);
    tune_hound.p1.ronghui.free_and_easy_tune = 1;
    tune_hound.p1.chance.you_ming_xu_hun_quan = 1;
    tune_hound.p1.ronghui.five_emperors_upgrade = 1;
    tune_hound.p1.ronghui.alchemy_pot = 1;

    tune_hound.test_execute_one_card(PlayerSide::P1);

    assert_eq!(tune_hound.p1.ronghui.free_and_easy_tune, 0);
    assert_eq!(tune_hound.p1.chance.you_ming_xu_hun_quan, 0);
    assert_eq!(tune_hound.p1.ronghui.five_emperors_upgrade, 0);
    assert_eq!(tune_hound.p1.ronghui.alchemy_pot, 0);
    assert_eq!(tune_hound.p1.deck.slots[0].card.id, 10_000);
}

#[test]
fn generating_upgrade_precedes_paint_and_alchemy_transforms() {
    let wood_shadow = original_card_definition_by_id(7_010_017)
        .expect("missing rarity-one wood spirit sparse shadow");
    let mut fixture = minimal_fixture(
        filler_cards(wood_shadow.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.initial_anima = 1;
    fixture.players.p2.initial_defense = 100;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.elements.last_element = Some(Element::Water);
    state.p1.fate.generating_interaction_upgrade = 1;
    state.p1.fate.paint_finishing_touch = 1;
    state.p1.ronghui.alchemy_pot = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.fate.generating_interaction_upgrade, 0);
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    assert_eq!(state.p1.ronghui.alchemy_pot, 0);
    assert_eq!(state.p1.deck.slots[0].card.id, wood_shadow.id);
}

#[test]
fn alchemy_life_transfer_waits_until_printed_cost_succeeds() {
    let mut costly_basic = basic_attack_test_card();
    costly_basic.anima = Some(-1);
    let fixture = minimal_fixture(
        filler_cards(costly_basic),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.ronghui.alchemy_pot = 1;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.ronghui.alchemy_pot, 1);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (30, 30));
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (30, 30));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.ronghui.alchemy_pot, 0);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (21, 21));
    assert_eq!(state.p2.core.max_hp, 39);
}

#[test]
fn transformed_cloud_and_beng_mindset_hooks_run_per_effect() {
    let mut replica = test_card(322, 322, "逍遥•复刻");
    replica.anima = Some(-1);
    let mut copied = test_card(9_999_996, 1_000_005, "云剑•契约");
    copied.hp_cost = Some(5);
    copied.defense = Some(1);
    let mut fixture = minimal_fixture(
        filler_cards(replica.clone()),
        filler_cards(copied.clone()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents = vec![15];
    fixture.players.p1.initial_momentum_limit = Some(99);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.sword.cloud_sword_heart = 2;
    state.p1.beng.beng_mei_mindset = 3;
    state.p1.fate.plum_blossom_twice = 1;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.deck.slots[0].card.id, replica.id);
    assert_eq!(state.p1.sword.cloud_sword_heart, 2);
    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.turn.extra_actions, 0);
    assert_eq!(state.p1.core.hp, 30);

    let action_again = state.test_execute_one_card(PlayerSide::P1);
    assert!(action_again);
    assert_eq!(state.p1.deck.slots[0].card.id, copied.id);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.sword.cloud_sword_heart, 0);
    assert_eq!(state.p1.beng.momentum, 6);
    assert_eq!(state.p1.core.hp, 30);
}

#[test]
fn temporary_cloud_hooks_run_but_beng_mindset_and_virtual_chain_stay_distinct() {
    let mut cloud = test_card(9_999_995, 1_000_005, "云剑•临时契约");
    cloud.hp_cost = Some(5);
    cloud.defense = Some(1);
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
    fixture.players.p1.talents = vec![14, 15];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.sword.cloud_sword_heart = 1;
    state.p1.beng.beng_mei_mindset = 3;

    assert_eq!(state.p1.sword.cloud_chain, 0);
    assert!(support::has_cloud_chain(&state.p1));
    state.apply_temporary_card_effect(PlayerSide::P1, &cloud, 0);

    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.sword.cloud_sword_heart, 0);
    assert_eq!(state.p1.turn.extra_actions, 1);
    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.core.hp, 30);
}
