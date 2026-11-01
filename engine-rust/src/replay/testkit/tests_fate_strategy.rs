use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn deck() -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![basic_attack()], basic_attack())
}

fn player() -> FixturePlayer {
    super::test_support::make_player(deck(), 1, 100, None, 1, None)
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
}

/// Typed rows for the data-driven fate-strategy tables: every loop iteration
/// of the migrated tests below runs as an individually named `#[test]` with
/// the verbatim original body and assertions. The row identity is the
/// generated test name suffix (for example `fate_true_xianmo_true`).
struct QiXingRequireCase {
    fate: bool,
    xianmo: bool,
    consumed: i64,
    adds_star: bool,
}

struct QiXingBothCase {
    is_temporary: bool,
    charges: i64,
    next_is_star: bool,
    remaining: i64,
    anima: i64,
}

struct FateSwitchCase {
    temp_data: Option<i64>,
}

struct BentBowCase {
    card_id: i64,
    gain: i64,
}

const QI_XING_REQUIRE_CASES: &[QiXingRequireCase] = &[
    QiXingRequireCase {
        fate: true,
        xianmo: true,
        consumed: 2,
        adds_star: true,
    },
    QiXingRequireCase {
        fate: true,
        xianmo: false,
        consumed: 1,
        adds_star: true,
    },
    QiXingRequireCase {
        fate: false,
        xianmo: true,
        consumed: 1,
        adds_star: true,
    },
    QiXingRequireCase {
        fate: false,
        xianmo: false,
        consumed: 0,
        adds_star: false,
    },
];

fn run_qi_xing_require_case(case: &QiXingRequireCase) {
    let star = original_config::original_card_definition(4_000_010).expect("星弈•夹");
    // (fate 331, xianmo 7056, charges consumed, star added)
    let fate = case.fate;
    let xianmo = case.xianmo;
    let consumed = case.consumed;
    let adds_star = case.adds_star;
    let mut p1 = player();
    p1.active_slot_count = 8;
    if fate {
        p1.fate_strategies = vec![331];
    }
    if xianmo {
        p1.xian_mo_strategies = vec![7056];
    }
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.fate.qi_xing_lian_zhu = 7;
    state.p1.astrology.star_slots.clear();
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &star, 0, false);
    assert_eq!(
        state.p1.fate.qi_xing_lian_zhu,
        7 - consumed,
        "fate={fate} xianmo={xianmo}"
    );
    assert_eq!(
        state.p1.astrology.star_slots.contains(&1),
        adds_star,
        "fate={fate} xianmo={xianmo}"
    );
}

crate::data_driven_cases! {
    table: QI_XING_REQUIRE_CASES,
    run: run_qi_xing_require_case,
    suite: "replay::tests_fate_strategy::qi_xing_lian_zhu_callbacks_require_fate_331_and_xianmo_7056",
    valid: qi_xing_lian_zhu_callbacks_require_case_table_is_valid,
    cases: [
        qi_xing_lian_zhu_callbacks_require_fate_331_and_xianmo_7056_fate_true_xianmo_true => 0,
        qi_xing_lian_zhu_callbacks_require_fate_331_and_xianmo_7056_fate_true_xianmo_false => 1,
        qi_xing_lian_zhu_callbacks_require_fate_331_and_xianmo_7056_fate_false_xianmo_true => 2,
        qi_xing_lian_zhu_callbacks_require_fate_331_and_xianmo_7056_fate_false_xianmo_false => 3,
    ]
}

#[test]
fn fate_strategy_326_sets_yan_qi_at_battle_start() {
    let mut p1 = player();
    p1.fate_strategies = vec![326];

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.fate.yan_qi, 1);
}

const QI_XING_BOTH_CASES: &[QiXingBothCase] = &[
    QiXingBothCase {
        is_temporary: false,
        charges: 7,
        next_is_star: false,
        remaining: 5,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: false,
        charges: 7,
        next_is_star: true,
        remaining: 5,
        anima: 2,
    },
    QiXingBothCase {
        is_temporary: false,
        charges: 2,
        next_is_star: false,
        remaining: 0,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: false,
        charges: 1,
        next_is_star: false,
        remaining: 0,
        anima: 0,
    },
    QiXingBothCase {
        is_temporary: false,
        charges: 1,
        next_is_star: true,
        remaining: 0,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: false,
        charges: 0,
        next_is_star: false,
        remaining: 0,
        anima: 0,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 7,
        next_is_star: false,
        remaining: 5,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 7,
        next_is_star: true,
        remaining: 5,
        anima: 2,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 2,
        next_is_star: false,
        remaining: 0,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 1,
        next_is_star: false,
        remaining: 0,
        anima: 0,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 1,
        next_is_star: true,
        remaining: 0,
        anima: 1,
    },
    QiXingBothCase {
        is_temporary: true,
        charges: 0,
        next_is_star: false,
        remaining: 0,
        anima: 0,
    },
];

fn run_qi_xing_both_case(case: &QiXingBothCase) {
    let star = original_config::original_card_definition(4_000_010).expect("星弈•夹");
    let is_temporary = case.is_temporary;
    let charges = case.charges;
    let next_is_star = case.next_is_star;
    let remaining = case.remaining;
    let anima = case.anima;
    let mut p1 = player();
    p1.active_slot_count = 8;
    p1.fate_strategies = vec![331];
    p1.xian_mo_strategies = vec![7056];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    assert_eq!(state.p1.fate.qi_xing_lian_zhu, 14);
    state.p1.fate.qi_xing_lian_zhu = charges;
    state.p1.astrology.star_slots = if next_is_star { vec![1] } else { vec![] };

    state.apply_before_execute_effect_hooks(PlayerSide::P1, &star, 0, is_temporary);

    assert_eq!(state.p1.fate.qi_xing_lian_zhu, remaining);
    assert_eq!(state.p1.core.anima, anima);
    assert_eq!(
        state.p1.astrology.star_slots,
        if charges > 0 || next_is_star {
            vec![1]
        } else {
            vec![]
        }
    );
    assert_eq!(state.p2.core.anima, 0);
    assert_eq!(
        state.p2.core.hp, 100,
        "both callbacks precede the card body"
    );
}

crate::data_driven_cases! {
    table: QI_XING_BOTH_CASES,
    run: run_qi_xing_both_case,
    suite: "replay::tests_fate_strategy::qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks",
    valid: qi_xing_lian_zhu_runs_both_on_play_case_table_is_valid,
    cases: [
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_7_star_false => 0,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_7_star_true => 1,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_2_star_false => 2,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_1_star_false => 3,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_1_star_true => 4,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_false_charges_0_star_false => 5,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_7_star_false => 6,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_7_star_true => 7,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_2_star_false => 8,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_1_star_false => 9,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_1_star_true => 10,
        qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_temp_true_charges_0_star_false => 11,
    ]
}

#[test]
fn qi_xing_lian_zhu_runs_both_on_play_callbacks_with_independent_charge_checks_basic_attack() {
    let mut p1 = player();
    p1.active_slot_count = 8;
    p1.fate_strategies = vec![331];
    p1.xian_mo_strategies = vec![7056];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &basic_attack(), 0, false);
    assert_eq!(state.p1.fate.qi_xing_lian_zhu, 14);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.astrology.star_slots, vec![2, 5]);
}

#[test]
fn qi_xing_lian_zhu_spirit_gathering_follows_payment_and_skips_failed_plays() {
    let star = original_config::original_card_definition(4_000_010).expect("星弈•夹");
    let mut p1 = player();
    p1.active_slot_count = 8;
    p1.cards = super::test_support::fill_deck(vec![star], basic_attack());
    p1.fate_strategies = vec![331];
    p1.xian_mo_strategies = vec![7056];
    let fixture = fixture(p1, player());
    let mut shortage = ReplayState::test_from_fixture(&fixture);
    shortage.p1.astrology.star_slots.clear();

    assert!(!shortage.test_execute_one_card(PlayerSide::P1));

    assert_eq!(shortage.p1.core.anima, 1, "shortage fallback only");
    assert_eq!(shortage.p1.fate.qi_xing_lian_zhu, 14);
    assert_eq!(shortage.p1.astrology.star_slots, Vec::<usize>::new());
    assert!(!shortage.p1.deck.slots[0].used);
    assert_eq!(shortage.p2.core.hp, 100);

    let mut paid = ReplayState::test_from_fixture(&fixture);
    paid.p1.astrology.star_slots.clear();
    paid.p1.core.anima = 1;

    assert!(!paid.test_execute_one_card(PlayerSide::P1));

    assert_eq!(
        paid.p1.core.anima, 1,
        "pay 1, then gain 1 from the second callback"
    );
    assert_eq!(paid.p1.fate.qi_xing_lian_zhu, 12);
    assert_eq!(paid.p1.astrology.star_slots, vec![1]);
    assert!(paid.p1.deck.slots[0].used);
    assert_eq!(paid.p2.core.hp, 94);
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

    // 26125017: the hook moved from before-card to CalculateAttack, so the
    // layers survive the before-hooks and clear on the first attack segment.
    assert_eq!(state.p1.fate.ke_yin_jing_lei, 1);
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(state.p1.fate.ke_yin_jing_lei, 0);
    assert_eq!(state.p2.status.external_injury, 1);
    // 第一段攻击碎防：防御 5 全碎（3×2>=5 → 3+ceil(5/2)=6，吸收 5）。
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
    // 锚点：hf-32308000 pa_fad1b6a6 turn 9（p2 星弈•断 →
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
    state.p1.identity.talents = vec![204].into();

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

const FATE_394_SWITCH_CASES: &[FateSwitchCase] = &[
    FateSwitchCase { temp_data: None },
    FateSwitchCase { temp_data: Some(0) },
];

fn run_fate_394_switch_case(case: &FateSwitchCase) {
    let temp_data = case.temp_data;
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

crate::data_driven_cases! {
    table: FATE_394_SWITCH_CASES,
    run: run_fate_394_switch_case,
    suite: "replay::tests_fate_strategy::fate_strategy_394_respects_missing_zero_and_nonzero_switch_values",
    valid: fate_strategy_394_respects_switch_values_case_table_is_valid,
    cases: [
        fate_strategy_394_respects_missing_zero_and_nonzero_switch_values_temp_missing => 0,
        fate_strategy_394_respects_missing_zero_and_nonzero_switch_values_temp_zero => 1,
    ]
}

#[test]
fn fate_strategy_394_respects_missing_zero_and_nonzero_switch_values_temp_one() {
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
    opening.other_params = vec![1].into();
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
    // 真分支 oracle 锚点：mirror-32219000-human-01 pa_62c67ebb
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
    // oracle 锚点：mirror-32219000-human-01 pa_0f7f2fd3 cp0
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
    palm_card.name = "迎风掌".to_string().into();
    let mut non_palm_card = basic_attack();
    non_palm_card.name = "连环拳".to_string().into();

    // 1. Fate 428 + 掌牌：气势不递减
    let mut p1 = player();
    p1.fate_strategies = vec![428];
    p1.cards = vec![palm_card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.modify_momentum(PlayerSide::P1, 3);
    assert_eq!(state.p1.beng.momentum, 3);
    state.execute_actor_turn();
    assert_eq!(
        state.p1.beng.momentum, 3,
        "palm card under fate 428 should retain momentum"
    );

    // 2. Fate 428 + 非掌牌：气势正常消耗 1 点
    let mut p1_non_palm = player();
    p1_non_palm.fate_strategies = vec![428];
    p1_non_palm.cards = vec![non_palm_card.clone(); DECK_SIZE];
    let mut state_non_palm = ReplayState::test_from_fixture(&fixture(p1_non_palm, player()));
    state_non_palm.modify_momentum(PlayerSide::P1, 3);
    state_non_palm.execute_actor_turn();
    assert_eq!(
        state_non_palm.p1.beng.momentum, 2,
        "non-palm card under fate 428 should consume 1 momentum"
    );

    // 3. 无 Fate 428 + 掌牌：气势正常消耗 1 点
    let mut p1_no_fate = player();
    p1_no_fate.cards = vec![palm_card; DECK_SIZE];
    let mut state_no_fate = ReplayState::test_from_fixture(&fixture(p1_no_fate, player()));
    state_no_fate.modify_momentum(PlayerSide::P1, 3);
    state_no_fate.execute_actor_turn();
    assert_eq!(
        state_no_fate.p1.beng.momentum, 2,
        "palm card without fate 428 should consume 1 momentum"
    );
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
        c.other_params = vec![1, 2, 3, 1].into();
        c
    });
    card.attack = Some(8);
    card.other_params = vec![1, 2, 3, 1].into();

    let mut p1 = player();
    p1.cards = vec![card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.anima = 5; // 5 anima -> 5 / 2 = 2 bonus attack -> total attack 8 + 2 = 10
    state.execute_actor_turn();

    // p2 takes 10 damage: 100 - 10 = 90
    assert_eq!(state.p2.core.hp, 90);
}

#[test]
fn card_10000092_ling_kong_fei_sao_hp_cost_is_two() {
    // CardConfig 凌空飞扫 hpCost = 2；对局内降级（厄劫缠身）重查 catalog 同样是 2。
    let card = original_card_definition_by_id(10_000_092).unwrap();
    assert_eq!(card.hp_cost, Some(2));

    let mut p1 = player();
    p1.cards = vec![card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.hp = 100;
    state.execute_actor_turn();
    assert_eq!(state.p1.core.hp, 98); // 100 - 2 = 98
}

const BENT_BOW_CASES: &[BentBowCase] = &[
    BentBowCase {
        card_id: 4_000_097,
        gain: 12,
    },
    BentBowCase {
        card_id: 4_010_097,
        gain: 20,
    },
    BentBowCase {
        card_id: 4_020_097,
        gain: 28,
    },
];

fn run_bent_bow_case(case: &BentBowCase) {
    // Card_4000097.cs 先机 ModifyMaxHp/ModifyHp(otherParams[0])；后招 attack + AddHpCount/otherParams[1]。
    let card_id = case.card_id;
    let gain = case.gain;
    let card = original_card_definition_by_id(card_id)
        .unwrap_or_else(|| panic!("missing original card {card_id}"));
    assert_eq!(card.attack, Some(8), "card {card_id}");
    assert_eq!(card.other_params, vec![gain, 4, 10], "card {card_id}");
}

crate::data_driven_cases! {
    table: BENT_BOW_CASES,
    run: run_bent_bow_case,
    suite: "replay::tests_fate_strategy::card_4000097_bent_bow_heal_amount",
    valid: card_4000097_bent_bow_heal_amount_case_table_is_valid,
    cases: [
        card_4000097_bent_bow_heal_amount_card_4000097 => 0,
        card_4000097_bent_bow_heal_amount_card_4010097 => 1,
        card_4000097_bent_bow_heal_amount_card_4020097 => 2,
    ]
}

#[test]
fn card_4000097_bent_bow_heal_amount_first_play() {
    // 4010097 首次打出（先机生效、后招不成立），
    // 46/97 → maxHp 117、hp 66、本场累计回血 +20，对方不受击。
    // oracle 锚点：pa_373320dc f4eu5d4/round-12 cp26。
    let card = original_card_definition_by_id(4_010_097).expect("missing 弯弓射虎 4010097");
    let mut p1 = player();
    p1.cards = vec![card.clone(); DECK_SIZE];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.p1.core.max_hp = 97;
    state.p1.core.hp = 46;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.max_hp, 117);
    assert_eq!(state.p1.core.hp, 66);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 20);
    assert_eq!(state.p2.core.hp, 100);
}

#[test]
fn card_1000034_spirit_gathering_tiebreak_flag_is_idempotent_under_echo() {
    // 原版源码与 oracle 已确认的幂等旗置契约：
    // Card_1000034.cs 用 `SetBuffValue(BanDianLingQi, 1)`——奇数 mind stance
    // 的补整标志是幂等置位。回响阵纹（8000012）在同一回合以临时牌重复执行
    // 聚灵心法时不得累加；旧 `+= 1` 会在下一个奇数回合多 +1 灵气
    // （灵气与后续防御的差异来自同一旗置时点）。
    // BattleCharacter.cs 回合开始侧对奇数 num7 做旗置 ±1 后整除，与
    // half_anima∈{0,1} 恰好互补，故只需锁登记侧幂等。
    let mindset = original_card_definition_by_id(1_000_034).expect("missing 聚灵心法 1000034");
    assert_eq!(
        mindset.other_params,
        vec![1],
        "只有 otherParams[0]==1 的档位置旗"
    );
    let mut state = ReplayState::test_from_fixture(&fixture(player(), player()));
    state.test_apply_card_effect(PlayerSide::P1, &mindset, 0);
    state.test_apply_card_effect(PlayerSide::P1, &mindset, 0);
    assert_eq!(
        state.p1.fate.spirit_gathering_mindset, 2,
        "重复执行仍累加 stance 层数"
    );
    assert_eq!(
        state.p1.fate.half_anima, 1,
        "BanDianLingQi 是 Set 语义，不随重复执行累加"
    );
}

#[test]
fn fate_strategy_128_water_spirit_card_grants_sharpness() {
    // FateStrategyFunctions.cs（金灵分支之后、320 分支之前）：
    // HasFateStrategy(128) && cardConfig.name.Contains("水灵")
    //   → ModifyBuffValue(BuffType.FengRui, 1)。
    let mut water_card = basic_attack();
    water_card.name = "水灵•春雨".to_string().into();

    let mut p1 = player();
    p1.fate_strategies = vec![128];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    state.apply_selected_card_hooks(PlayerSide::P1, &water_card, 0);
    assert_eq!(state.p1.sword.sharpness, 1);

    // 对照：无 Fate 128 时水灵牌不加锋锐。
    let mut state_no_fate = ReplayState::test_from_fixture(&fixture(player(), player()));
    state_no_fate.apply_selected_card_hooks(PlayerSide::P1, &water_card, 0);
    assert_eq!(state_no_fate.p1.sword.sharpness, 0);

    // 对照：持 Fate 128 打金灵牌不加锋锐，旧行为（+1 灵气）保持。
    let mut gold_card = basic_attack();
    gold_card.name = "金灵•铁骨".to_string().into();
    let mut p1_gold = player();
    p1_gold.fate_strategies = vec![128];
    let mut state_gold = ReplayState::test_from_fixture(&fixture(p1_gold, player()));
    state_gold.apply_selected_card_hooks(PlayerSide::P1, &gold_card, 0);
    assert_eq!(state_gold.p1.sword.sharpness, 0);
    assert_eq!(state_gold.p1.core.anima, 1);
}
