use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(BASIC_ATTACK_ID)
}

fn deck() -> Vec<CardDefinition> {
    let mut cards = vec![basic_attack()];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn deck_with(first: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![first];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn player() -> FixturePlayer {
    player_with(deck())
}

fn player_with(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 50,
        extra_max_hp: Some(0),
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: Some(6),
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: BTreeMap::new(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: BTreeMap::new(),
        talent_card_params: BTreeMap::new(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards,
    }
}

fn fixture() -> BattleFixture {
    fixture_with(player(), player())
}

fn fixture_with(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
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

fn common_card_state(card_id: i64, actor_anima: i64, target_defense: i64) -> ReplayState {
    let mut p1 = player_with(deck_with(original_card(card_id)));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    p1.initial_anima = actor_anima;
    p2.initial_defense = target_defense;
    ReplayState::test_from_fixture(&fixture_with(p1, p2))
}

#[test]
fn dream_majesty_in_deck_replaces_percent_momentum_before_the_card_is_used() {
    let mut cards = deck();
    cards[1] = original_card(10_030_084);
    let mut state = ReplayState::test_from_fixture(&fixture_with(player_with(cards), player()));
    state.p1.beng.momentum_limit = 10;
    state.p1.beng.momentum = 10;

    state.apply_attack(PlayerSide::P1, 17, 0);

    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p1.beng.momentum, 9);
}

#[test]
fn leaf_blade_flower_grants_persistent_defense_piercing_after_its_own_attack() {
    let mut state = common_card_state(9_020_018, 0, 12);

    state.test_execute_one_card(PlayerSide::P1);

    // ApplyDamage's low-attack branch doubles 4 to 8 when 4 * 2 < 10.
    state.p2.core.hp = 50;
    state.p2.core.defense = 10;
    state.apply_attack(PlayerSide::P1, 4, 0);
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 2);

    // At the threshold, 4 gains ceil(6 / 2) before the ordinary defense loss.
    state.p2.core.hp = 50;
    state.p2.core.defense = 6;
    state.apply_attack(PlayerSide::P1, 4, 0);
    assert_eq!(state.p2.core.hp, 49);
    assert_eq!(state.p2.core.defense, 0);
}

#[test]
fn turn_wide_ignore_defense_preserves_persistent_attack_charges() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p1.turn.current_turn_ignore_defense = 1;
    state.p1.turn.ignore_defense_attacks = 2;
    state.p2.core.defense = 20;

    state.apply_attack(PlayerSide::P1, 5, 0);
    assert_eq!(state.p2.core.hp, 45);
    assert_eq!(state.p2.core.defense, 20);
    assert_eq!(state.p1.turn.ignore_defense_attacks, 2);

    state.p1.turn.current_turn_ignore_defense = 0;
    state.apply_attack(PlayerSide::P1, 5, 0);
    assert_eq!(state.p2.core.hp, 40);
    assert_eq!(state.p1.turn.ignore_defense_attacks, 1);
}

#[test]
fn metal_iron_bone_reduces_non_attack_damage() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;

    state.apply_damage(PlayerSide::P1, 8, false, false, false);

    assert_eq!(state.p2.core.hp, 47);
}

#[test]
fn dismantle_fist_stance_reduces_non_attack_damage_after_iron_bone() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;
    state.p2.fate.dismantle_move = 1;
    state.p2.beng.quan_stance = 1;

    state.apply_damage(PlayerSide::P1, 13, false, false, false);

    assert_eq!(state.p2.core.hp, 46);
}

#[test]
fn attack_damage_does_not_apply_iron_bone_twice() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.elements.metal_iron_bone = 1;

    state.apply_damage(PlayerSide::P1, 8, true, false, false);

    assert_eq!(state.p2.core.hp, 42);
}

#[test]
fn guard_makes_same_hit_defense_absorption_worth_zero_hp() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    state.p2.core.defense = 6;
    state.p2.core.guard = 1;

    state.apply_damage(PlayerSide::P1, 20, false, false, false);

    // Original ordering is unchanged: defense is spent first, then guard cancels
    // the remaining 14. The counterfactual telemetry must not claim the spent 6
    // defense saved HP because guard would also have cancelled a full 20-point hit.
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p2.prevention.hp_loss_prevented_by_guard, 14);
    assert_eq!(state.p2.prevention.hp_loss_prevented_by_defense, 0);
}

#[test]
fn common_simple_attacks_follow_configured_attack_count() {
    let cases = [
        (145, 1, 77),
        (146, 2, 74),
        (147, 3, 71),
        (148, 4, 68),
        (149, 5, 65),
        (150, 7, 59),
    ];

    for (card_id, expected_segments, expected_hp) in cases {
        let mut state = common_card_state(card_id, 0, 0);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p2.core.hp,
                state.p1.turn.attack_segments_performed,
                state.p2.status.internal_injury,
                state.p2.status.external_injury
            ),
            (expected_hp, expected_segments, 0, 0),
            "card {card_id}"
        );
    }
}

#[test]
fn verified_simple_attack_cards_follow_exact_rank_config() {
    let cases = [
        (1_000_007, 1, 0, 1, 70),
        (1_010_007, 1, 0, 1, 67),
        (1_020_007, 1, 0, 1, 64),
        (1_000_019, 2, 0, 1, 64),
        (1_010_019, 2, 0, 1, 60),
        (1_020_019, 2, 0, 1, 56),
        (1_000_032, 0, 0, 3, 71),
        (1_010_032, 0, 0, 3, 68),
        (1_020_032, 0, 0, 3, 65),
        (9_000_003, 0, 0, 1, 74),
        (9_010_003, 0, 0, 2, 72),
        (9_020_003, 0, 0, 3, 71),
        (10_000_029, 4, 0, 1, 62),
        (10_010_029, 4, 0, 1, 58),
        (10_020_029, 4, 0, 1, 54),
        (10_000_035, 0, 0, 1, 69),
        (10_010_035, 0, 0, 1, 64),
        (10_020_035, 0, 0, 1, 59),
    ];

    for (card_id, initial_anima, expected_anima, expected_segments, expected_hp) in cases {
        let mut state = common_card_state(card_id, initial_anima, 0);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p1.core.anima,
                state.p2.core.hp,
                state.p1.turn.attack_segments_performed,
                state.p2.status.internal_injury,
                state.p2.status.external_injury,
            ),
            (expected_anima, expected_hp, expected_segments, 0, 0),
            "card {card_id}"
        );
    }
}

#[test]
fn hungry_tiger_cost_reduction_applies_before_its_printed_attack() {
    let mut state = common_card_state(10_000_029, 1, 0);
    state.p1.status.internal_injury = 2;
    state.p1.status.flaw = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p1.core.anima,
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
        ),
        (0, 62, 1)
    );
}

#[test]
fn continuous_collapse_repeats_and_preserves_inherited_beng_quan_effects() {
    let mut state = common_card_state(10_000_035, 0, 0);
    state.p1.beng.beng_quan_defense = 3;
    state.p1.beng.beng_quan_double_shadow = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.core.defense,
            state.p1.turn.attack_segments_performed,
            state.p1.beng.beng_quan_defense,
            state.p1.beng.beng_quan_double_shadow,
        ),
        (58, 6, 2, 3, 1)
    );
}

#[test]
fn five_elements_escape_uses_printed_defense_and_exact_two_element_threshold() {
    let cases = [
        (7_000_055, 0, 8, false),
        (7_000_055, 1, 8, false),
        (7_000_055, 2, 8, true),
        (7_010_055, 2, 13, true),
        (7_020_055, 2, 18, true),
    ];

    for (card_id, active_element_count, expected_defense, expected_action_again) in cases {
        let mut state = common_card_state(card_id, 0, 0);
        if active_element_count >= 1 {
            state.activate_element(PlayerSide::P1, Element::Metal);
        }
        if active_element_count >= 2 {
            state.activate_element(PlayerSide::P1, Element::Water);
        }

        let action_again = state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p1.core.defense,
                action_again,
                state.p1.turn.action_again_count,
            ),
            (
                expected_defense,
                expected_action_again,
                i64::from(expected_action_again),
            ),
            "card {card_id}, active elements {active_element_count}"
        );
    }
}

#[test]
fn space_spirit_field_skips_once_in_the_last_two_slots_then_gains_anima() {
    let field = original_card(9_000_015);
    let mut cards = deck();
    cards[6] = field;
    let mut state = ReplayState::test_from_fixture(&fixture_with(player_with(cards), player()));
    state.p1.deck.queue = vec![6, 7, 0, 1, 2, 3, 4, 5];

    state.test_execute_one_card(PlayerSide::P1);

    assert!(state.p1.deck.slots[6].used);
    assert_eq!((state.p1.core.anima, state.p2.core.hp), (0, 47));

    state.p1.deck.queue = vec![6, 0, 1, 2, 3, 4, 5, 7];
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!((state.p1.core.anima, state.p2.core.hp), (4, 47));
}

#[test]
fn hidden_basic_attack_three_ranks_follow_configured_damage() {
    let cases = [(286, 77), (10_286, 74), (20_286, 71)];

    for (card_id, expected_hp) in cases {
        let mut state = common_card_state(card_id, 0, 0);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (state.p2.core.hp, state.p1.turn.attack_segments_performed),
            (expected_hp, 1),
            "card {card_id}"
        );
    }
}

#[test]
fn rakshasa_pounce_attacks_then_gains_anima_and_self_external_injury() {
    let cases = [(71, 72), (10_071, 69), (20_071, 66)];

    for (card_id, expected_hp) in cases {
        let mut state = common_card_state(card_id, 0, 0);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p2.core.hp,
                state.p1.core.anima,
                state.p1.status.external_injury
            ),
            (expected_hp, 1, 1),
            "card {card_id}"
        );
    }
}

#[test]
fn rakshasa_pounce_external_injury_is_prevented_by_exorcism() {
    let mut state = common_card_state(10_071, 0, 0);
    state.p1.fate.exorcism = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.fate.exorcism, 0);
    assert_eq!(state.p1.status.external_injury, 0);
}

#[test]
fn sky_breaking_claw_pays_anima_then_attacks_and_self_internal_injury() {
    let cases = [(72, 67), (10_072, 64), (20_072, 61)];

    for (card_id, expected_hp) in cases {
        let mut state = common_card_state(card_id, 1, 0);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p2.core.hp,
                state.p1.core.anima,
                state.p1.status.internal_injury
            ),
            (expected_hp, 0, 1),
            "card {card_id}"
        );
    }
}

#[test]
fn hidden_weapon_applies_injuries_before_damage_and_consumes_defense() {
    let cases = [(330, 78), (10_330, 75), (20_330, 72)];

    for (card_id, expected_hp) in cases {
        let mut card = original_card(card_id);
        card.card_type = Some(OriginalEnumValue {
            value: CARD_TYPE_CONSUME,
            name: "Consume".to_string(),
        });
        let mut p1 = player_with(deck_with(card));
        let mut p2 = player();
        p1.base_max_hp = 80;
        p2.base_max_hp = 80;
        p2.initial_defense = 3;
        let mut state = ReplayState::test_from_fixture(&fixture_with(p1, p2));
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p2.core.defense,
                state.p2.core.hp,
                state.p2.status.internal_injury,
                state.p2.status.external_injury,
                state.p1.deck.slots[0].skipped
            ),
            (0, expected_hp, 1, 1, true),
            "card {card_id}"
        );
    }
}

#[test]
fn horse_hoof_uses_original_fallback_random_attack_range() {
    let cases = [(367, 5, 75), (10_367, 9, 71), (20_367, 13, 67)];

    for (card_id, roll, expected_hp) in cases {
        let mut p1 = player_with(deck_with(original_card(card_id)));
        let mut p2 = player();
        p1.base_max_hp = 80;
        p2.base_max_hp = 80;
        let mut battle = fixture_with(p1, p2);
        battle.decision_tape = vec![roll];
        let mut state = ReplayState::test_from_fixture(&battle);
        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (
                state.p2.core.hp,
                state.p1.turn.attack_segments_performed,
                state.decision_tape.len()
            ),
            (expected_hp, 1, 0),
            "card {card_id}"
        );
    }

    let mut p1 = player_with(deck_with(original_card(20_367)));
    let mut p2 = player();
    p1.base_max_hp = 80;
    p2.base_max_hp = 80;
    let mut battle = fixture_with(p1, p2);
    battle.random_fallback_tape = vec![9];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.fail_on_missing_decision = true;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.turn.attack_segments_performed,
            state.random_fallback_tape.len(),
            state.evaluation_error
        ),
        (71, 1, 0, None)
    );
}

#[test]
fn wan_mo_shi_xin_qu_self_damage_precedes_frenzy_sword_zero_lifesteal_per_segment() {
    // 原版每段攻击管道顺序（build 24610558，与旧 build 24466094 反编译一致的历史行为）：
    // 万魔噬心曲自伤在 CalculateAttack（BattleCharacter.cs:11757-11759），先于
    // ApplyDamage（BattleCharacter.cs:10882-10888）内的狂剑灵石吸血；反震（FanZhenXinFa
    // 217，:10998-11005）在吸血之后。引擎原先「吸血→反震→自伤」与原版相反。
    // 锚定 eswmbqw/round-19：满血 123，云剑•无妄（1010028）2 段 ×6，万魔噬心曲 4，
    // 狂剑灵石 50%（每段 6×50%=3）：
    //   原版顺序：123 −4 +3 −4 +3 = 121
    //   引擎错序：123 +0 −4 +3 −4 = 118（满血首段吸血被 maxHp 截断）
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(1_010_028))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 123;
        player.core.max_hp = 123;
        player.music.chaotic_mind_tune = 4;
        player.sword.frenzy_sword_zero = 50;
        // 云剑•无妄须狂龙吞云在身才归为狂剑（IsKuangJian：HasBuff(357) && 卡名含「云剑」）。
        player.sword.frenzy_dragon_swallows_cloud = 1;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        (state.p1.core.hp, state.p2.core.hp),
        (121, 50 - 12),
        "两段后自伤先于吸血：p1 应 121（错序为 118），p2 吃满 2×6"
    );
}

#[test]
fn wan_mo_shi_xin_qu_full_hp_segment_lifesteal_is_not_truncated_by_max_hp() {
    // 满血截断场景：狂剑•一式（1010022，1 段，attack=8，名含「狂剑」天然归狂剑类）
    // 满血 123 攻击，万魔噬心曲 4、狂剑灵石 50%（8×50%=4）：
    //   原版顺序：123 −4 +4 = 123（吸血全额入账）
    //   引擎错序：123 +0 −4 = 119（满血吸血被截断为 0）
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(1_010_022))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 123;
        player.core.max_hp = 123;
        player.music.chaotic_mind_tune = 4;
        player.sword.frenzy_sword_zero = 50;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(
        state.p1.core.hp, 123,
        "单段先自伤后吸血，吸血不被满血上限截断（错序为 119）"
    );
}

#[test]
fn frenzy_sword_zero_does_not_lifesteal_from_after_card_formation_attack() {
    // BattleCharacter.ApplyDamage (build 24610558:10882, 24466094:10820)
    // gates KuangJianLingShi with !HasBuff(AfterCardAciton). The printed
    // frenzy-sword hit can lifesteal, but a Zhou Tian formation follow-up in
    // OnAfterExecuted cannot inherit that identity for lifesteal.
    let mut state = ReplayState::test_from_fixture(&fixture_with(
        player_with(deck_with(original_card(2))),
        player(),
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 50;
        player.core.max_hp = 100;
        player.sword.frenzy_sword_zero = 30;
        player.formations.heaven_cycle_sword_formation = 1;
        player.formations.heaven_cycle_sword_formation_damage = 5;
    });

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 50, "牌后阵法追击不得触发狂剑灵石吸血");
    assert_eq!(
        state.p2.core.hp, 42,
        "炎舞 2 攻，再叠 1 外伤后结算周天剑阵 5 攻"
    );
}

#[test]
fn cloud_way_talent_grades_apply_separate_defense_gains_for_nishi_and_he_ba_huang() {
    // CardActionBase.cs:2383-2386（OnBeforeExecuted 天赋参数 switch：
    // num3 = current % 10000 逐天赋命中 case 69）：云之道每个档位
    // （10069/20069/30069）独立执行一次 ModifyDef，不能汇总成一次加防。
    // NiShi（BattleCharacter.cs:10148）与 HeBaHuang（:10205-10209）都挂在
    // ModifyDef 上按次结算：汇总 7 → 反弹 floor(7/2)=3、八荒退 3+2+2；
    // 拆分 2+2+3 → 反弹 1+1+1、八荒退 1+1+1。
    // oracle 锚点：mirror-32299000 2ae5ddcc93eaebab/round-15 cp12
    // （游龙后 p2.def 16→4 = 15 伤 - 3 退款；引擎汇总 +7 时 16→10）。
    let mut p1 = player_with(deck_with(original_card(1_000_042)));
    p1.talents = vec![18, 66, 10_069, 20_069, 30_069];
    let mut p2 = player();
    p2.initial_defense = 16;
    let mut state = ReplayState::test_from_fixture(&fixture_with(p1, p2));
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p2.elements.earth_eight_wastes = 3;
    state.p1.sword.cloud_chain = 3;
    state.p1.dream_mirage.cloud_sword_used_count = 3;

    assert!(!state.test_execute_one_card(PlayerSide::P1));

    // 伤害：云之道 2/2/3 三档各反弹 1 + 游龙 2×2 攻 + 连云 +3 反弹 1 = 8；
    // 合八荒 3 层退款前三次防御损失 1+1+1 = 3 → 16 - 8 + 3 = 11。
    assert_eq!(state.p2.core.defense, 11);
}
