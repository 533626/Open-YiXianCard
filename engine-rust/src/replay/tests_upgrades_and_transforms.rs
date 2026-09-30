use super::*;

#[test]
fn plum_blossom_twice_grants_repeat_buff_from_mei_kai_er_du() {
    let card = CardDefinition {
        id: 4_000_041,
        base_id: Some(4_000_041),
        name: "梅开二度".to_string().into(),
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
        other_params: vec![2].into(),
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

#[test]
fn generating_interaction_upgrade_respects_no_upgrade_cards() {
    let dream_fire = original_card_definition_by_id(7_020_083)
        .expect("missing original 梦•火灵瞬燃 3档");
    let fixture = minimal_fixture(
        filler_cards(dream_fire.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.fate.generating_interaction_upgrade = 1;
    state.p1.elements.last_element = Some(Element::Wood);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        state.p1.deck.slots[0].card.id, dream_fire.id,
        "noUpgrade cards cannot be upgraded by generating interaction"
    );
}

#[test]
fn dan_ka_gong_ji_ji_shu_persists_across_turn_end_attack_and_triggers_heaven_cycle_sword_formation() {
    let non_attack_card = original_card_definition_by_id(7_000_061)
        .expect("missing original 木灵•暗香");
    let fixture = minimal_fixture(
        filler_cards(non_attack_card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 5;
    // Simulate turn-end attack (e.g. Fate 137 water momentum attack setting dan_ka_gong_ji_ji_shu)
    state.p1.turn.dan_ka_gong_ji_ji_shu = 1;

    let initial_p2_hp = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.formations.heaven_cycle_sword_formation, 0,
        "heaven cycle sword formation should be triggered by dan_ka_gong_ji_ji_shu carry"
    );
    assert_eq!(
        state.p2.core.hp,
        initial_p2_hp - 5,
        "p2 should take 5 damage from heaven cycle sword formation"
    );
    assert_eq!(
        state.p1.turn.dan_ka_gong_ji_ji_shu, 0,
        "dan_ka_gong_ji_ji_shu should be cleared after card after-hooks"
    );
}


#[test]
fn paint_finishing_touch_runs_before_frenzy_sword_upgrade_and_blocks_it() {
    // CardActionBase.Execute：画龙点睛（IL_1421）先把 rarity=0 的狂剑•一式升到 1010022
    // 并回写 cardConfig，之后的升级下次狂剑（IL_17ac）读到 rarity=1 不再触发——671 与
    // 生命都不消耗。oracle：hf-latest-33331000 3dfe77e1e41cabd9/round-13。
    let frenzy = original_card_definition_by_id(1_000_022).expect("狂剑•一式");
    let fixture = minimal_fixture(
        filler_cards(frenzy.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.fate.paint_finishing_touch = 2;
    state.p1.sword.upgrade_next_frenzy_sword = 1;
    let hp_before = state.p1.core.hp;

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.deck.slots[0].card.id, 1_010_022);
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    assert_eq!(state.p1.sword.upgrade_next_frenzy_sword, 1);
    assert_eq!(state.p1.core.hp, hp_before);
}

fn metal_return_edge_state(with_dream_five_elements: bool) -> ReplayState {
    let edge = original_card_definition_by_id(7_000_099).expect("金灵•回锋刃");
    let fixture = minimal_fixture(
        filler_cards(edge),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    // 原版 GetBattleDeckIdList 只含已解锁格，五行刺须在开放格内。
    state.p1.deck.active_slot_count = 2;
    if with_dream_five_elements {
        state.p1.deck.slots[1].card =
            original_card_definition_by_id(7_040_077).expect("梦•五行刺");
    }
    state.p1.sword.sharpness = 2;
    state.p2.core.defense = 0;
    state
}

#[test]
fn metal_return_edge_uses_check_wu_xing_deck_override() {
    // BattleCharacter.cs:10810 的回锋返还走 CardActionBase.CheckWuXing(JiHuoJinLing)，
    // 其末尾「战斗牌组含 7030077/7040077 即视为激活」同样生效（CardActionBase.cs:5350-5362）。
    // 6攻×2：每段消耗 2 锋锐后返还 ceil(2×60%)=2 → 伤害 (6+2)×2，锋锐剩 2。
    // oracle：hf-latest-33334000 618dbaaf1fd47540/round-14。
    let mut state = metal_return_edge_state(true);
    let hp_before = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(hp_before - state.p2.core.hp, 16);
    assert_eq!(state.p1.sword.sharpness, 2);

    // 对照：无五行刺、未激活金灵 → 不返还，只有首段吃到锋锐。
    let mut plain = metal_return_edge_state(false);
    let hp_before = plain.p2.core.hp;
    plain.test_execute_one_card(PlayerSide::P1);
    assert_eq!(hp_before - plain.p2.core.hp, 14);
    assert_eq!(plain.p1.sword.sharpness, 0);
}

#[test]
fn earth_shake_halves_defense_only_with_earth_activated() {
    // Card_7000046 土灵•撼地：减半对方灵气与防御都在 CheckWuXing(JiHuoTuLing) 分支内。
    // oracle：hf-latest-33331000 ef6706d7e604abc7/round-07（未激活土灵，p2 防御 9−6=3，不再减半）。
    let shake = original_card_definition_by_id(7_000_046).expect("土灵•撼地");
    let run = |earth: bool| {
        let fixture = minimal_fixture(
            filler_cards(shake.clone()),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        let mut state = ReplayState::test_from_fixture(&fixture);
        state.p2.core.defense = 9;
        state.p2.core.anima = 4;
        if earth {
            state.p1.elements.activated_earth = 1;
        }
        state.test_execute_one_card(PlayerSide::P1);
        (state.p2.core.defense, state.p2.core.anima)
    };
    assert_eq!(run(false), (3, 4));
    assert_eq!(run(true), (1, 2));
}

#[test]
fn dream_counter_shock_reflects_after_frenzy_sword_lifesteal() {
    // BattleCharacter.ApplyDamage：狂剑吸血（:10882）先于梦•反震心法扣攻击者生命（:11006）。
    // 满血时吸血先溢出作废、再被反弹扣血；旧顺序（先反弹后吸血）会把血回满。
    // oracle：hf-latest-33331000 67dd94826528fc59/round-17。
    let frenzy = original_card_definition_by_id(1_000_022).expect("狂剑•一式");
    let fixture = minimal_fixture(
        filler_cards(frenzy),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.sword.frenzy_sword_zero = 100;
    state.p1.core.hp = state.p1.core.max_hp;
    state.p2.core.defense = 0;
    state.modify_dream_mirage_value(
        PlayerSide::P2,
        super::super::cards_dream_mirage::DreamMirageValue::DreamReflection,
        1,
    );
    let max_hp = state.p1.core.max_hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert!(state.p1.core.hp < max_hp, "反弹须在吸血之后结算，满血吸血溢出后仍应掉血");
}

#[test]
fn stance_switch_cards_do_nothing_without_a_stance() {
    // Card_222 转势 / Card_220：效果按拳、棍架势分支，SwitchJiaShi 在两者皆无时不加架势。
    // oracle：hf-latest-33333000 bbcfba241c4acf94/round-13（幻羽鹦把转势复制给无架势方）。
    for card_id in [10_222, 220] {
        let card = original_card_definition_by_id(card_id).expect("stance switch card");
        let fixture = minimal_fixture(
            filler_cards(card),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        let mut state = ReplayState::test_from_fixture(&fixture);
        let hp_before = state.p2.core.hp;
        state.test_execute_one_card(PlayerSide::P1);
        assert_eq!(state.p1.turn.agility, 0, "card {card_id}");
        assert_eq!(state.p1.beng.quan_stance, 0, "card {card_id}");
        assert_eq!(state.p1.beng.gun_stance, 0, "card {card_id}");
        assert_eq!(state.p2.core.hp, hp_before, "card {card_id}");
    }
}

#[test]
fn second_actor_opening_reads_deck_downgraded_by_first_actor() {
    // BattleCharacter.TriggerOpening 读当前牌组：先手方厄劫缠身开局把次位方同格吉运初显
    // 11010005 降为 11000005，次位方开局随后只加 4（而非 5）。
    // oracle：hf-latest-33333000 6cda5802fa858719/round-18。
    let curse = original_card_definition_by_id(11_010_018).expect("厄劫缠身");
    let fortune = original_card_definition_by_id(11_010_005).expect("吉运初显");
    let fixture = minimal_fixture(
        filler_cards(curse),
        filler_cards(fortune),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let base_max_hp = {
        let mut plain = fixture.clone();
        plain.players.p1.cards = filler_cards(basic_attack_test_card());
        plain.players.p2.cards = filler_cards(basic_attack_test_card());
        ReplayState::test_from_fixture(&plain).p2.core.max_hp
    };
    let state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(fixture.first_player_side, PlayerSide::P1);
    assert_eq!(state.p2.deck.slots[0].card.id, 11_000_005);
    assert_eq!(state.p2.core.max_hp, base_max_hp + 4);
}
