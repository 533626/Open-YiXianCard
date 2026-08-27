use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn basic_attack() -> CardDefinition {
    CardDefinition {
        id: 0,
        base_id: Some(0),
        name: "普通攻击".to_string(),
        card_type: None,
        attack: Some(3),
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: Vec::new(),
    }
}

fn deck() -> Vec<CardDefinition> {
    vec![basic_attack(); DECK_SIZE]
}

fn player() -> FixturePlayer {
    FixturePlayer {
        level: 1,
        base_max_hp: 100,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 1,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards: deck(),
    }
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
        max_actor_turns: Some(1),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers { p1, p2 },
    }
}

#[test]
fn fate_strategy_326_sets_yan_qi_at_battle_start() {
    let mut p1 = player();
    p1.fate_strategies = vec![326];

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.yan_qi, 1);
}

#[test]
fn fate_strategy_396_seeds_one_jing_lei_layer_at_battle_start() {
    let mut p1 = player();
    p1.fate_strategies = vec![396];

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.ke_yin_jing_lei, 1);
}

#[test]
fn fate_strategy_384_counts_opening_hand_and_used_cloud_swords_with_cap() {
    let mut p1 = player();
    p1.fate_strategies = vec![384];
    p1.hand_cards = vec![1_000_026, 1_000_042, 0, 1_000_026];
    p1.last_round_used_card_base_ids = vec![1_000_042, 1_000_026, 0, 1_000_042];

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.anima, 3);

    let mut enhanced = player();
    enhanced.fate_strategies = vec![384];
    enhanced.talents = vec![222];
    enhanced.hand_cards = vec![1_000_026; 10];
    let enhanced_state = ReplayState::test_from_fixture(&fixture(enhanced, player()));
    assert_eq!(enhanced_state.p1.core.anima, 5);
}

#[test]
fn fate_strategy_396_thunder_hook_clears_layers_adds_wound_and_shatters_attack() {
    let mut p1 = player();
    p1.fate_strategies = vec![396];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    let thunder = original_config::original_card_definition(4_000_030).expect("missing 落雷术");
    state.p2.core.defense = 5;

    // 原版 SuiFang(340) 是整卡行动期碎防（CardActionBase.cs:3738-3742 才
    // 移除），故钩子与攻击都必须在 effect invocation 帧内执行。
    let kind = super::effect_invocation::EffectInvocationKind::Played;
    state.begin_effect_invocation(
        PlayerSide::P1,
        &thunder,
        &thunder,
        &thunder,
        0,
        0,
        kind,
        false,
    );
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &thunder, 0, false);

    assert_eq!(state.p1.fate.ke_yin_jing_lei, 0);
    assert_eq!(state.p2.status.external_injury, 1);
    // 第一段攻击碎防：防御 5 全碎（3×2>=5 → 3+ceil(5/2)=6，吸收 5）。
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(state.p2.core.defense, 0);
    // 同一卡行动内的后续攻击段仍然碎防（原版 340 不按段消耗）。
    state.p2.core.defense = 5;
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(state.p2.core.defense, 0);
    state.end_effect_invocation(PlayerSide::P1, kind);
    // 卡行动结束后碎防不再生效（AfterCardAction 已移除 340）。
    state.p2.core.defense = 5;
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(state.p2.core.defense, 2);
}

#[test]
fn fate_strategy_396_non_thunder_card_does_not_consume_jing_lei() {
    let mut p1 = player();
    p1.fate_strategies = vec![396];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    let card = basic_attack();
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &card, 0, false);

    assert_eq!(state.p1.fate.ke_yin_jing_lei, 1);
    assert_eq!(state.p2.status.external_injury, 0);
    // 非雷牌不发放碎防：防御 5 对攻击 3 正常吸收 3 剩 2。
    state.p2.core.defense = 5;
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(state.p2.core.defense, 2);
}

#[test]
fn fate_strategy_398_skips_fifth_grid_and_heals_for_star_chess() {
    let mut p1 = player();
    p1.active_slot_count = 6;
    p1.fate_strategies = vec![398];
    p1.cards[4] = original_config::original_card_definition(4_000_093).expect("missing 星弈•长");
    p1.cards[5] = original_config::original_card_definition(129).expect("missing 灯影蚀焰");

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 90;
    for _ in 0..5 {
        state.test_execute_one_card(PlayerSide::P1);
    }

    let completed = state
        .test_events()
        .iter()
        .filter(|event| event.card_id == Some(129))
        .count();
    assert_eq!(completed, 1);
    assert_eq!(state.p1.core.hp, 95);
}

#[test]
fn fate_strategy_398_does_not_heal_when_star_chess_break_skips_fifth_grid() {
    // 原版 BattleExecuter.cs:1813-1816 的星弈断 while 排在 1857-1864 的
    // FS398 之前：第 5 格若先被星弈断（XingYi_Duan）跳掉，FS398 的 while
    // 重入时 currentCard 已是下一张牌，gridNumber != 4，不会加血。
    // 锚点：hf-32308000 a96197fcfaf754d8/round-15 turn 9（p2 星弈•断 →
    // p1 XingYi_Duan，turn 9 跳第 5 格不加血，原版 hp 70→70）。
    let mut p1 = player();
    p1.active_slot_count = 6;
    p1.fate_strategies = vec![398];
    p1.cards[4] = original_config::original_card_definition(4_000_093).expect("missing 星弈•长");
    p1.cards[5] = original_config::original_card_definition(129).expect("missing 灯影蚀焰");

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 90;
    for _ in 0..4 {
        state.test_execute_one_card(PlayerSide::P1);
    }
    // 第 5 次取牌前才获得星弈断层（模拟对方星弈•断命中）：星弈断的 while
    // 在 FS398 之前，第 5 格被它跳掉时 FS398 循环重入看到的已是下一张牌。
    state.p1.astrology.star_chess_break = 1;
    state.test_execute_one_card(PlayerSide::P1);

    let completed = state
        .test_events()
        .iter()
        .filter(|event| event.card_id == Some(129))
        .count();
    assert_eq!(completed, 1);
    // 星弈断先吃掉第 5 格：不加血，星弈断层数归零。
    assert_eq!(state.p1.astrology.star_chess_break, 0);
    assert_eq!(state.p1.core.hp, 90);
}

#[test]
fn fate_strategy_407_grants_three_layers_and_consumes_one_on_anima_gain() {
    let mut p1 = player();
    p1.fate_strategies = vec![407];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.wu_you_ling_niang, 3);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (100, 100));

    state.gain_anima(PlayerSide::P1, 1);

    assert_eq!(state.p1.fate.wu_you_ling_niang, 2);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (104, 104));
    assert_eq!(state.p1.core.anima, 1);
}

#[test]
fn fate_strategy_395_adds_one_to_each_of_three_hexagram_gains_then_exhausts() {
    let mut p1 = player();
    p1.fate_strategies = vec![395];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.wu_jing_gua_yan, 3);
    state.gain_hexagram(PlayerSide::P1, 1);
    state.gain_hexagram(PlayerSide::P1, 1);
    state.gain_hexagram(PlayerSide::P1, 1);
    assert_eq!(state.p1.astrology.hexagram, 6);
    assert_eq!(state.p1.fate.wu_jing_gua_yan, 0);

    state.gain_hexagram(PlayerSide::P1, 1);
    assert_eq!(state.p1.astrology.hexagram, 7);
}

#[test]
fn fate_strategy_407_exhausts_after_three_anima_gain_calls() {
    let mut p1 = player();
    p1.fate_strategies = vec![407];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    state.gain_anima(PlayerSide::P1, 1);
    state.gain_anima(PlayerSide::P1, 2);
    state.gain_anima(PlayerSide::P1, 3);
    state.gain_anima(PlayerSide::P1, 4);

    assert_eq!(state.p1.fate.wu_you_ling_niang, 0);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (112, 112));
    assert_eq!(state.p1.core.anima, 10);
}

#[test]
fn fate_strategy_407_does_not_consume_on_zero_final_anima_gain() {
    let mut p1 = player();
    p1.fate_strategies = vec![407];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.dream_mirage.half_anima_gain = 1;

    state.gain_anima(PlayerSide::P1, 1);

    assert_eq!(state.p1.fate.wu_you_ling_niang, 3);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (100, 100));
    assert_eq!(state.p1.core.anima, 0);
}

#[test]
fn fate_strategy_407_does_not_consume_when_204_converts_anima_to_physique() {
    let mut p1 = player();
    p1.fate_strategies = vec![407];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.identity.talents = vec![204];

    state.gain_anima(PlayerSide::P1, 1);

    assert_eq!(state.p1.fate.wu_you_ling_niang, 3);
    // Talent 204's physique conversion itself raises max HP by one; no
    // additional +4/+4 from Fate 407 is applied.
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (100, 101));
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.physique, 1);
}

#[test]
fn fate_strategy_402_consumes_on_a_star_slot_and_adds_internal_injury() {
    let mut p1 = player();
    p1.fate_strategies = vec![402];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.astrology.star_slots = vec![0];

    assert_eq!(state.p1.fate.xing_yuan, 3);
    state.apply_xing_yuan_before_card_hook(PlayerSide::P1, 0);
    state.test_apply_card_effect(PlayerSide::P1, &basic_attack(), 0);

    assert_eq!(state.p1.fate.xing_yuan, 2);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn fate_strategy_402_skips_non_star_slots_and_disabled_switch() {
    let mut active = player();
    active.fate_strategies = vec![402];
    let mut active_state = ReplayState::test_from_fixture(&fixture(active, player()));
    active_state.p1.astrology.star_slots = vec![1];
    active_state.apply_xing_yuan_before_card_hook(PlayerSide::P1, 0);
    active_state.test_apply_card_effect(PlayerSide::P1, &basic_attack(), 0);
    assert_eq!(active_state.p1.fate.xing_yuan, 3);
    assert_eq!(active_state.p2.status.internal_injury, 0);

    let mut disabled = player();
    disabled.fate_strategies = vec![402];
    disabled
        .fate_strategy_temp_datas
        .insert("402".to_string(), 1);
    let mut disabled_state = ReplayState::test_from_fixture(&fixture(disabled, player()));
    disabled_state.p1.astrology.star_slots = vec![0];
    disabled_state.apply_xing_yuan_before_card_hook(PlayerSide::P1, 0);
    disabled_state.test_apply_card_effect(PlayerSide::P1, &basic_attack(), 0);
    assert_eq!(disabled_state.p1.fate.xing_yuan, 0);
    assert_eq!(disabled_state.p2.status.internal_injury, 0);
}

#[test]
fn fate_strategy_402_checks_star_slot_before_card_body_mutates_slots() {
    let mut p1 = player();
    p1.fate_strategies = vec![402];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    let tian_yuan = original_config::original_card_definition(4_000_045).expect("missing 天元心法");

    // 天元心法 makes every grid a star slot in its body.  XingYuan is an
    // OnBeforeExecuted hook, so slot 0 is not retroactively eligible here.
    state.apply_xing_yuan_before_card_hook(PlayerSide::P1, 0);
    state.test_apply_card_effect(PlayerSide::P1, &tian_yuan, 0);
    assert!(state.p1.astrology.star_slots.contains(&0));
    assert_eq!(state.p1.fate.xing_yuan, 3);
    assert_eq!(state.p2.status.internal_injury, 0);

    state.apply_xing_yuan_before_card_hook(PlayerSide::P1, 0);
    state.test_apply_card_effect(PlayerSide::P1, &basic_attack(), 0);
    assert_eq!(state.p1.fate.xing_yuan, 2);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn fate_strategy_423_seeds_one_shot_momentum_bonus_at_battle_start() {
    let mut p1 = player();
    p1.fate_strategies = vec![423];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.beng.pending_momentum_bonus, 2);
    state.modify_momentum(PlayerSide::P1, 1);
    assert_eq!(state.p1.beng.momentum, 3);
    assert_eq!(state.p1.beng.pending_momentum_bonus, 0);
    state.modify_momentum(PlayerSide::P1, 1);
    assert_eq!(state.p1.beng.momentum, 4);
}

#[test]
fn fate_strategy_326_heals_after_first_hp_or_max_hp_gain() {
    let mut hp_state = ReplayState::test_from_fixture(&fixture(player(), player()));
    hp_state.p1.core.hp = 50;
    hp_state.p1.fate.yan_qi = 1;

    hp_state.modify_actor_hp(PlayerSide::P1, 10, false, false);

    assert_eq!(hp_state.p1.core.hp, 80);
    assert_eq!(hp_state.p1.fate.yan_qi, 0);

    let mut max_hp_state = ReplayState::test_from_fixture(&fixture(player(), player()));
    max_hp_state.p1.core.hp = 50;
    max_hp_state.p1.fate.yan_qi = 1;
    max_hp_state.p1.identity.talents.push(64);

    max_hp_state.modify_actor_max_hp(PlayerSide::P1, 10);

    assert_eq!(max_hp_state.p1.core.max_hp, 110);
    assert_eq!(max_hp_state.p1.core.hp, 72);
    assert_eq!(max_hp_state.p1.core.defense, 1);
    assert_eq!(max_hp_state.p1.dream_mirage.hp_gain_event_count, 1);
    assert_eq!(max_hp_state.p1.fate.yan_qi, 0);
}

#[test]
fn fate_strategy_394_adds_one_to_positive_hp_and_max_hp_changes() {
    let mut p1 = player();
    p1.fate_strategies = vec![394];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 50;

    assert_eq!(state.modify_actor_hp(PlayerSide::P1, 10, false, false), 11);
    assert_eq!(state.p1.core.hp, 61);
    assert_eq!(state.modify_actor_max_hp(PlayerSide::P1, 10).applied, 11);
    assert_eq!(state.p1.core.max_hp, 111);

    assert_eq!(state.modify_actor_hp(PlayerSide::P1, -3, false, false), -3);
    assert_eq!(state.modify_actor_max_hp(PlayerSide::P1, -3).applied, -3);
}

#[test]
fn fate_strategy_394_respects_missing_zero_and_nonzero_switch_values() {
    for temp_data in [None, Some(0)] {
        let mut p1 = player();
        p1.fate_strategies = vec![394];
        if let Some(value) = temp_data {
            p1.fate_strategy_temp_datas.insert("394".to_string(), value);
        }
        let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
        state.p1.core.hp = 50;
        assert_eq!(state.modify_actor_hp(PlayerSide::P1, 10, false, false), 11);
        assert_eq!(state.modify_actor_max_hp(PlayerSide::P1, 10).applied, 11);
    }

    let mut p1 = player();
    p1.fate_strategies = vec![394];
    p1.fate_strategy_temp_datas.insert("394".to_string(), 1);
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 50;
    assert_eq!(state.modify_actor_hp(PlayerSide::P1, 10, false, false), 10);
    assert_eq!(state.modify_actor_max_hp(PlayerSide::P1, 10).applied, 10);
}

#[test]
fn fate_strategy_326_is_present_before_gui_yuan_cao_opening_gain() {
    let mut p1 = player();
    p1.base_max_hp = 62;
    p1.extra_max_hp = Some(24);
    p1.battle_start_hp = Some(46);
    p1.talents = vec![10_120];
    p1.fate_strategies = vec![326, 340];
    p1.permanent_buff_temp_datas.insert("10008".to_string(), 3);
    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(
        (state.p1.core.hp, state.p1.core.max_hp, state.p1.fate.yan_qi),
        (110, 117, 0)
    );
}

#[test]
fn fate_strategy_336_grants_agility_after_first_anima_gain() {
    let mut p1 = player();
    p1.fate_strategies = vec![336];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.feng_ling_zhan_yi, 5);

    state.gain_anima(PlayerSide::P1, 1);
    state.gain_anima(PlayerSide::P1, 1);

    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.turn.agility, 5);
    assert_eq!(state.p1.fate.feng_ling_zhan_yi, 0);
}

#[test]
fn fate_strategy_336_precedes_anima_opening_card() {
    let mut opening = original_card_definition_by_id(11_000_009).expect("missing 探灵 opening");
    opening.other_params = vec![1];
    let mut p1 = player();
    p1.fate_strategies = vec![336];
    p1.cards[0] = opening;

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.turn.agility, 5);
    assert_eq!(state.p1.fate.feng_ling_zhan_yi, 0);
}

#[test]
fn fate_strategy_340_grants_max_hp_and_heals_after_action_again() {
    let mut p1 = player();
    p1.fate_strategies = vec![340];
    p1.initial_agility = 10;

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.hp, 100);
    assert_eq!(state.p1.core.max_hp, 120);

    state.p1.core.hp = 50;
    let card = state.test_actor_card(PlayerSide::P1, 0);

    assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.p1.core.hp, 52);
}

#[test]
fn fate_strategy_163_grants_anima_once_per_turn_after_physique_gain() {
    let mut p1 = player();
    p1.fate_strategies = vec![163];

    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));

    state.apply_physique_amount(PlayerSide::P1, 1);
    state.apply_physique_amount(PlayerSide::P1, 1);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.fate.chan_xin_ju_ling_triggered, 1);

    state.p1.fate.chan_xin_ju_ling_triggered = 0;
    state.apply_physique_amount(PlayerSide::P1, 1);

    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.fate.chan_xin_ju_ling_triggered, 1);
}

#[test]
fn fate_strategy_344_grants_guard_once_at_low_hp_turn_start() {
    let mut p1 = player();
    p1.fate_strategies = vec![344];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 20;

    state.execute_actor_turn();

    assert_eq!(state.p1.core.guard, 1);
    assert_eq!(state.p1.fate.vermilion_bird_tear, 0);

    state.current_actor = PlayerSide::P1;
    state.p1.core.hp = 20;
    state.execute_actor_turn();

    assert_eq!(state.p1.core.guard, 1);
}

#[test]
fn fate_strategy_397_grants_star_power_with_exp_advantage_branch() {
    // 灯灵星耀（FateStrategyFunctions.cs:543-546，OnBattleStart 内）：
    // 星力 +1；自身 lastRoundExp ≥ 对方 + otherParams[0]=5 再 +1。
    // 真分支 oracle 锚点：mirror-32219000-human-01 64e07edecaeef655/round-12
    // cp0（p2 星力 4 = 天元心法 2 + 397×2，exp 61 ≥ 46+5）。
    let mut advantaged = player();
    advantaged.fate_strategies = vec![397];
    advantaged.last_round_exp = 61;
    let mut behind = player();
    behind.last_round_exp = 46;
    let state = ReplayState::test_from_fixture(&fixture(advantaged, behind));
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p2.astrology.star_power, 0);

    // 假分支：exp 未领先 ≥5 时只 +1。
    let mut equal = player();
    equal.fate_strategies = vec![397];
    equal.last_round_exp = 46;
    let mut ahead = player();
    ahead.last_round_exp = 61;
    let state = ReplayState::test_from_fixture(&fixture(equal, ahead));
    assert_eq!(state.p1.astrology.star_power, 1);
}

#[test]
fn fate_strategy_340_grants_max_hp_at_battle_start_before_yan_qi_heal() {
    // 乘雁而行（FateStrategyFunctions.cs:487-489，OnBattleStart 内）：
    // ModifyMaxHp(otherParams[0]=20)；326 砚气在其后消耗，回 newMaxHp×20/100
    // （BattleCharacter.cs:9981-9986），Talent64 在回血时 +1 防御。
    // oracle 锚点：mirror-32219000-human-01 a6b2ce98f7989074/round-12 cp0
    // （p1 maxHp 104 = 84+20，hp 104 = 84 + 104×20/100，defense 1）。
    let mut p1 = player();
    p1.fate_strategies = vec![326, 340];
    p1.talents = vec![64];
    let state = ReplayState::test_from_fixture(&fixture(p1, player()));
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (120, 120));
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p1.fate.yan_qi, 0);
}

#[test]
fn fate_strategy_428_palm_card_retains_momentum_on_attack() {
    // 24963639 BattleCharacter.cs:11579/:11702:
    // HasFateStrategy(428) && m_CurrentUsingCard.name.Contains("掌") 时，
    // 攻击计算气势不递减（-0 而不是 -1）。
    let mut palm_card = basic_attack();
    palm_card.name = "迎风掌".to_string();
    let mut non_palm_card = basic_attack();
    non_palm_card.name = "连环拳".to_string();

    // 1. Fate 428 + 掌牌：气势不递减
    let mut p1 = player();
    p1.fate_strategies = vec![428];
    p1.cards = vec![palm_card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.modify_momentum(PlayerSide::P1, 3);
    assert_eq!(state.p1.beng.momentum, 3);
    state.execute_actor_turn();
    assert_eq!(state.p1.beng.momentum, 3, "palm card under fate 428 should retain momentum");

    // 2. Fate 428 + 非掌牌：气势正常消耗 1 点
    let mut p1_non_palm = player();
    p1_non_palm.fate_strategies = vec![428];
    p1_non_palm.cards = vec![non_palm_card.clone(); DECK_SIZE];
    let mut state_non_palm = ReplayState::test_from_fixture(&fixture(p1_non_palm, player()));
    state_non_palm.modify_momentum(PlayerSide::P1, 3);
    state_non_palm.execute_actor_turn();
    assert_eq!(state_non_palm.p1.beng.momentum, 2, "non-palm card under fate 428 should consume 1 momentum");

    // 3. 无 Fate 428 + 掌牌：气势正常消耗 1 点
    let mut p1_no_fate = player();
    p1_no_fate.cards = vec![palm_card; DECK_SIZE];
    let mut state_no_fate = ReplayState::test_from_fixture(&fixture(p1_no_fate, player()));
    state_no_fate.modify_momentum(PlayerSide::P1, 3);
    state_no_fate.execute_actor_turn();
    assert_eq!(state_no_fate.p1.beng.momentum, 2, "palm card without fate 428 should consume 1 momentum");
}

#[test]
fn card_10000092_ling_kong_fei_sao_uses_half_anima_attack() {
    // 24963639 Card_10000092.cs:
    // attack = card.attack + anima / 2 (整除 2).
    let mut card = original_card_definition_by_id(10_000_092).unwrap_or_else(|| {
        let mut c = basic_attack();
        c.id = 10_000_092;
        c.base_id = Some(10_000_092);
        c.attack = Some(8);
        c.other_params = vec![1, 2, 3, 1];
        c
    });
    card.attack = Some(8);
    card.other_params = vec![1, 2, 3, 1];

    let mut p1 = player();
    p1.cards = vec![card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.anima = 5; // 5 anima -> 5 / 2 = 2 bonus attack -> total attack 8 + 2 = 10
    state.execute_actor_turn();

    // p2 takes 10 damage: 100 - 10 = 90
    assert_eq!(state.p2.core.hp, 90);
}
