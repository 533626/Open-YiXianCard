// Build 24610558 新卡（synthetic oracle batch-027 转正）：403 云剑•猫影、
// 407 雷闪二度、422 紫芒星爆、7000108 极•金灵阵。
//
// 证据链：四卡在 HF 最新 shard 32219000 与 yiwen 批次 0 出现，由
// synthetic-original-oracle（batch-027，客户端真实执行，forward/reverse 全
// passed，14 case 全 stable）提供 witness；源码证据见
// battle-evaluator/oracle/synthetic-batch-027-build-24610558-new-cards.ts
// （private）与 build-24610558 反编译：
//   Card_422.cs（星力+otherParams[0]、ZiMangXingBao+1）、
//   Card_403.cs（生命及上限+otherParams[0]、YunJianMaoYing+otherParams[1]）、
//   Card_407.cs（卦象+guaXiang、LeiShanErDu+1）、
//   Card_7000108.cs（JiHuoJinLing+1、JinLingZhen+otherParams[0]），
// 以及共享结算 hook（BattleCharacter.cs:8811-8816/:9332-9337、
// CardActionBase.cs:1506-1519/:4413-4427/:5127-5142/:2741-2743）。
//
// 422 同时是 capability hexagramLossDoesNotSpendCard422StarPower 的
// reverify 关键：三处共享结算触发点（CheckAnima 星力代替、星力流失→加攻、
// 卦象消耗先扣星力）从 HasCardInDeck(422) 改写为 HasBuff(ZiMangXingBao)
// （BUILD_24589371_RULE_DELTA §2，§3-b）。本文件测试全部锚定 batch-027
// 各 pair 的 witness 数值。
//
// 注：test_execute_one_card 返回「本牌是否触发再次行动」而非「是否出牌」；
// 出牌结果一律以状态断言。

use super::tests::test_card;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn original_card(id: i64) -> CardDefinition {
    original_config::original_card_definition(id)
        .unwrap_or_else(|| panic!("missing original card {id}"))
}

fn basic_attack() -> CardDefinition {
    let mut attack = test_card(0, 0, "普通攻击");
    attack.attack = Some(3);
    attack
}

fn deck(ids: &[i64]) -> Vec<CardDefinition> {
    super::test_support::resize_deck(
        ids.iter().map(|id| original_card(*id)).collect(),
        basic_attack(),
    )
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 30,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: Some(1_000_001),
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        xian_mo_strategies: Vec::new(),
        xian_mo_temp_datas: Default::default(),
        active_slot_count,
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
        cards,
    }
}

fn fixture(p1_ids: &[i64], p1_active: usize, decisions: Vec<i64>) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: decisions,
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
        players: FixturePlayers {
            p1: player(deck(p1_ids), p1_active),
            p2: player(vec![basic_attack(); DECK_SIZE], 1),
        },
    }
}

fn state(p1_ids: &[i64], p1_active: usize) -> ReplayState {
    ReplayState::test_from_fixture(&fixture(p1_ids, p1_active, Vec::new()))
}

// ---- 配置档位（CardConfig，变体归一化）-----------------------------------

#[test]
fn new_cards_403_407_422_7000108_configs_match_build_24610558() {
    // 403 云剑•猫影：生命及上限+otherParams[0]、云剑追加 otherParams[1] 攻。
    // 24811621 起 otherParams=[4,4]（24705509 为 [4,3]，24666769 为 [2,3]）。
    let base = original_card(403);
    assert_eq!(base.other_params, vec![4, 4]);
    assert_eq!(base.card_type.as_ref().map(|t| t.value), Some(3)); // Sustain
    assert_eq!(original_card(10_403).other_params, vec![8, 6]);
    assert_eq!(original_card(20_403).other_params, vec![12, 8]);

    // 407 雷闪二度：卦象+guaXiang、灵气-1（anima 配置）、LeiShanErDu+1。
    let base = original_card(407);
    assert_eq!(base.hexagram, Some(2));
    assert_eq!(base.anima, Some(-1));
    assert_eq!(original_card(10_407).hexagram, Some(4));
    assert_eq!(original_card(20_407).hexagram, Some(6));

    // 422 紫芒星爆：星力+otherParams[0]、ZiMangXingBao+1（Sustain）。
    // 24705509 起 otherParams=[2]（24666769 为 [1]）。
    let base = original_card(422);
    assert_eq!(base.other_params, vec![2]);
    assert_eq!(base.card_type.as_ref().map(|t| t.value), Some(3));
    assert_eq!(original_card(10_422).other_params, vec![3]);
    assert_eq!(original_card(20_422).other_params, vec![4]);

    // 7000108 极•金灵阵：JiHuoJinLing+1、JinLingZhen+otherParams[0]、
    // actionAgain=true（Sustain）。
    let base = original_card(7_000_108);
    assert_eq!(base.other_params, vec![1]);
    assert_eq!(base.action_again, Some(true));
    assert_eq!(base.card_type.as_ref().map(|t| t.value), Some(3));
    assert_eq!(original_card(7_010_108).other_params, vec![2]);
    assert_eq!(original_card(7_020_108).other_params, vec![3]);
    assert_eq!(original_card(7_010_108).action_again, Some(true));
    assert_eq!(original_card(7_020_108).action_again, Some(true));
}

// ---- pair 1 — 403 云剑•猫影：生命及上限+otherParams[0] + 持续「每次使用云剑追加 otherParams[1] 攻」----

#[test]
fn cloud_sword_cat_shadow_granted_buff_attacks_after_each_cloud_sword() {
    // batch-027 pair 1（yun-jian-after-attack-bonus）：treatment [403, 8]
    // vs control [0, 8]。witness：p1.hp/maxHp 差分 +2；p2.hp decrease 差分
    // 3（403 自身是云剑：打出时先 hp/maxHp+2、YunJianMaoYing+3，随后
    // OnAfterExecuted 追加 3 攻；再打 8 云剑•猫爪 4攻×2 后追加 3 攻）。
    // 24811621 起 403 otherParams=[4,4]，生命及上限+4，云剑追加 4 攻。
    let mut treatment = state(&[403, 8], 2);
    treatment.test_execute_one_card(PlayerSide::P1); // 403
    treatment.test_execute_one_card(PlayerSide::P1); // 8
    assert_eq!(treatment.p1.core.hp, 34);
    assert_eq!(treatment.p1.core.max_hp, 34);
    assert_eq!(treatment.p1.sword.yun_jian_mao_ying, 4);
    // 403 自身追加 4 + 8 云剑•猫爪 4攻×2 + 追加 4 = 16。
    assert_eq!(treatment.p2.core.hp, 30 - 16);

    let mut control = state(&[0, 8], 2);
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 8
    assert_eq!(control.p1.core.hp, 30);
    assert_eq!(control.p1.core.max_hp, 30);
    assert_eq!(control.p1.sword.yun_jian_mao_ying, 0);
    // 普通攻击 3 + 8 云剑•猫爪 8 = 11。
    assert_eq!(control.p2.core.hp, 30 - 11);
}

// ---- pair 2 — 407 雷闪二度：卦象+2、LeiShanErDu+1、灵气-1；雷卡双效 ----

#[test]
fn thunder_twice_makes_thunder_card_execute_effect_twice() {
    // batch-027 pair 2（thunder-card-executes-twice）：treatment
    // [15, 407, 4000071] vs control [15, 0, 4000071]。witness：p1 卦象差分
    // +2、anima 差分 -1；p2.hp decrease 差分（双效+卦象加成）。
    let mut treatment = state(&[15, 407, 4_000_071], 3);
    treatment.test_execute_one_card(PlayerSide::P1); // 15 云剑•唤雨
    treatment.test_execute_one_card(PlayerSide::P1); // 407
    treatment.test_execute_one_card(PlayerSide::P1); // 4000071
    assert_eq!(treatment.p1.astrology.hexagram, 2); // 407 卦象+2
    assert_eq!(treatment.p1.core.anima, 0); // 15 +1、407 耗 1
    assert_eq!(treatment.p1.astrology.lei_shan_er_du, 0); // 已被雷卡消耗
                                                          // 15 2攻×3=6；4000071 梦•轰雷掣电 7+卦象2=9，LeiShanErDu 双效 → 18。
    assert_eq!(treatment.p2.core.hp, 30 - (6 + 18));

    let mut control = state(&[15, 0, 4_000_071], 3);
    control.test_execute_one_card(PlayerSide::P1); // 15
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 4000071
    assert_eq!(control.p1.astrology.hexagram, 0);
    assert_eq!(control.p1.core.anima, 1); // 15 +1，无 407 消耗
    assert_eq!(control.p1.astrology.lei_shan_er_du, 0);
    // 15 6 + 普通攻击 3 + 4000071 单效 7 = 16。
    assert_eq!(control.p2.core.hp, 30 - 16);
}

// ---- pair 3 — 422 触发点 1：CheckAnima 灵气不足时星力代替（CAB :5127）----

#[test]
fn star_power_pays_anima_shortage_for_card_72() {
    // batch-027 pair 3（star-power-pays-anima-shortage）：treatment [422, 72]
    // vs control [0, 72]。72 破空爪 anima=-1、13 攻。treatment：422 星力
    // 0→2（24705509 起 otherParams=[2]，24666769 为 [1]）、773+1；72 灵气
    // 不足 → 星力 2→1 代替 → 打出（星力流失同时触发 :8811-8816 加攻+1，
    // 72 攻击 13+1=14）。control：72 无 773 → AnimaShortage 退回，
    // p2 只吃普通攻击 3。
    let mut treatment = state(&[422, 72], 2);
    treatment.test_execute_one_card(PlayerSide::P1); // 422
    treatment.test_execute_one_card(PlayerSide::P1); // 72
    eprintln!("DBG p1 hp={} maxHp={} star={} ab={} anima={} neiShang={} sharp={} swordIntent={} p2hp={} p2def={}",
        treatment.p1.core.hp, treatment.p1.core.max_hp, treatment.p1.astrology.star_power,
        treatment.p1.core.attack_bonus, treatment.p1.core.anima, treatment.p1.status.internal_injury,
        treatment.p1.sword.sharpness, treatment.p1.sword.sword_intent, treatment.p2.core.hp, treatment.p2.core.defense);
    assert_eq!(treatment.p1.astrology.star_power, 1); // 2 - 1（代替灵气）
    assert_eq!(treatment.p1.core.attack_bonus, 1); // 星力流失 → 加攻
    assert_eq!(treatment.p1.astrology.zi_mang_xing_bao, 1);
    assert_eq!(treatment.p1.core.anima, 0);
    assert_eq!(treatment.p2.core.hp, 30 - 14);
    assert_eq!(treatment.p1.status.internal_injury, 1); // 72 自身+1内伤

    let mut control = state(&[0, 72], 2);
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 72 灵气不足且无 773 → 退回
    assert_eq!(control.p1.astrology.star_power, 0);
    assert_eq!(control.p1.core.attack_bonus, 0);
    // AnimaShortage 回退：+1 灵气（apply_anima_shortage_fallback），未出牌。
    assert_eq!(control.p1.core.anima, 1);
    assert_eq!(control.p2.core.hp, 30 - 3);
    // 回退的灵气让 72 下次可正常打出（witness 在退回时点采集，不参与差分）。
    control.test_execute_one_card(PlayerSide::P1);
    assert_eq!(control.p2.core.hp, 30 - 3 - 13);
}

// ---- pair 4 — 422 触发点 2：星力流失→加攻（BC :8811-8816）---------------

#[test]
fn star_power_loss_grants_attack_bonus_only_with_zi_mang_xing_bao() {
    // batch-027 pair 4（star-power-loss-grants-attack-bonus）：预置
    // ZiMangXingBao 1 层 + 星力 2 → 72 灵气不足星力代替 → 星力 2→1、
    // 加攻+1。control 无 773 → 72 打不出。加攻转换取实际（clamp 后）流失值
    // （BattleCharacter.cs:8615-8617 delta 同步为实际值后 :8811-8816 转换）。
    let mut treatment = state(&[72], 1);
    treatment.p1.astrology.zi_mang_xing_bao = 1;
    treatment.p1.astrology.star_power = 2;
    treatment.test_execute_one_card(PlayerSide::P1); // 72 打出
    assert_eq!(treatment.p1.astrology.star_power, 1);
    assert_eq!(treatment.p1.core.attack_bonus, 1);
    assert_eq!(treatment.p2.core.hp, 30 - 14); // 13 + 加攻 1

    let mut control = state(&[0], 1);
    control.p1.astrology.star_power = 2;
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    assert_eq!(control.p1.astrology.star_power, 2); // 无 773 星力不动
    assert_eq!(control.p1.core.attack_bonus, 0);
}

#[test]
fn star_power_loss_conversion_uses_clamped_actual_delta() {
    // 星力 2、773 1 层：-5 请求 → 实际 -2 → 加攻 +2（与既有
    // star_loss_uses_the_clamped_actual_delta_for_card_422 同语义，此处
    // 独立复述 batch-027 的星力流失→加攻 witness 锚点）。
    let mut treatment = state(&[], 1);
    treatment.p1.astrology.zi_mang_xing_bao = 1;
    treatment.p1.astrology.star_power = 2;
    assert_eq!(treatment.modify_star_power(PlayerSide::P1, -5), -2);
    assert_eq!(treatment.p1.astrology.star_power, 0);
    assert_eq!(treatment.p1.core.attack_bonus, 2);
}

// ---- pair 5 — 422 触发点 3a：卦象消耗先扣星力（BC :9332-9337）-----------

#[test]
fn hexagram_consumption_drains_star_power_first() {
    // batch-027 pair 5（hexagram-consumption-drains-star-power）：
    // 24589371/24610558 的 GetNextRandomValue（BC :9332-9337）中星力消耗
    // 与卦象消耗是两条独立 if（星力先扣、卦象照扣）。build 24646245 起
    // 星力分支改为 else-if 互斥（当前反编译证据），星力命中后不再扣卦象。
    // oracle 锚点：hf-32308000 pa_fe5e36fb cp9（五雷轰顶
    // 5 次随机持 773+星力 8 → 星力 8→2、卦象保持 1）。此处按当前 build
    // 改写：treatment [422, 4040073]：422 → 星力 2（24705509 起）、773+1；
    // 4040073 化神档 GetNextRandomValue（decisionTape [0]）→ 星力 2→1
    // （加攻+1）、卦象 2→2 不动。control 无 773 → 星力不动、卦象 2→1、无加攻。
    let mut treatment = ReplayState::test_from_fixture(&fixture(&[422, 4_040_073], 2, vec![0]));
    treatment.p1.astrology.hexagram = 2;
    treatment.p1.astrology.star_power = 1;
    treatment.test_execute_one_card(PlayerSide::P1); // 422
    treatment.test_execute_one_card(PlayerSide::P1); // 4040073
    assert_eq!(treatment.p1.astrology.star_power, 2); // 1+2-1
    assert_eq!(treatment.p1.core.attack_bonus, 1);
    // 星力分支命中后不再消耗卦象（24646245 else-if 链）。
    assert_eq!(treatment.p1.astrology.hexagram, 2);
    // 4040073 8攻+雷/卦牌数×2=10：主攻击 10 + （roll 后加攻 1）11 = 21。
    assert_eq!(treatment.p2.core.hp, 30 - 21);

    let mut control = ReplayState::test_from_fixture(&fixture(&[0, 4_040_073], 2, vec![0]));
    control.p1.astrology.hexagram = 2;
    control.p1.astrology.star_power = 1;
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 4040073
    assert_eq!(control.p1.astrology.star_power, 1); // 无 773 不动
    assert_eq!(control.p1.core.attack_bonus, 0);
    assert_eq!(control.p1.astrology.hexagram, 1);
    // 普通攻击 3 + 4040073 10+10 = 23。
    assert_eq!(control.p2.core.hp, 30 - 23);
}

// ---- pair 6 — 422 触发点 3b：卦象损失不重叠扣星力（capability 核心）-----

#[test]
fn hexagram_loss_does_not_overlap_star_power_spend() {
    // batch-027 pair 6（hexagram-loss-does-not-overlap-star-power）：预置
    // 卦象 2 + 星力 2；treatment 422 → 星力 3（24705509 起 422 星力+2）；
    // 4040073 roll → 星力 3→2、卦象 2→2（24646245 else-if 链：星力分支
    // 命中后卦象不动）、加攻+1。control（无 773）→ 星力不动、卦象 2→1。
    let mut treatment = ReplayState::test_from_fixture(&fixture(&[422, 4_040_073], 2, vec![0]));
    treatment.p1.astrology.hexagram = 2;
    treatment.p1.astrology.star_power = 2;
    treatment.test_execute_one_card(PlayerSide::P1); // 422
    treatment.test_execute_one_card(PlayerSide::P1); // 4040073
    assert_eq!(treatment.p1.astrology.star_power, 3); // 2+2-1
    assert_eq!(treatment.p1.core.attack_bonus, 1);
    assert_eq!(treatment.p1.astrology.hexagram, 2);

    let mut control = ReplayState::test_from_fixture(&fixture(&[0, 4_040_073], 2, vec![0]));
    control.p1.astrology.hexagram = 2;
    control.p1.astrology.star_power = 2;
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 4040073
    assert_eq!(control.p1.astrology.star_power, 2);
    assert_eq!(control.p1.core.attack_bonus, 0);
    assert_eq!(control.p1.astrology.hexagram, 1);
}

// ---- pair 7 — 7000108 极•金灵阵：激活金灵+1、金灵阵+1、再次行动；
//      使用「金灵」牌时锋锐+242 层 ----------------------------------------

#[test]
fn extreme_metal_formation_grants_sharpness_on_jin_ling_card() {
    // batch-027 pair 7（metal-formation-sharpness-on-jinling-card）：
    // treatment [7000108, 18] vs control [0, 18]。witness：锋锐差分 1 vs 0、
    // 激活金灵差分 1 vs 0、actionAgainPerRound 1 vs 0；18 金灵•刚劲
    // 5 攻 + 锋锐 1 = 6（control 仅 5）。
    let mut treatment = state(&[7_000_108, 18], 2);
    let action_again = treatment.test_execute_one_card(PlayerSide::P1); // 7000108
    assert!(action_again); // [再次行动]
    treatment.test_execute_one_card(PlayerSide::P1); // 18
    assert_eq!(treatment.p1.elements.activated_metal, 1); // 激活金灵 237
    assert_eq!(treatment.p1.elements.metal_formation, 1); // 金灵阵 242
    assert_eq!(treatment.p1.sword.sharpness, 1); // 使用金灵牌 → 锋锐+1
    assert_eq!(treatment.p1.turn.action_again_count, 1);
    assert_eq!(treatment.p2.core.hp, 30 - 6); // 5 + 锋锐 1

    let mut control = state(&[0, 18], 2);
    control.test_execute_one_card(PlayerSide::P1); // 普通攻击
    control.test_execute_one_card(PlayerSide::P1); // 18
    assert_eq!(control.p1.elements.activated_metal, 0);
    assert_eq!(control.p1.elements.metal_formation, 0);
    assert_eq!(control.p1.sword.sharpness, 0);
    assert_eq!(control.p2.core.hp, 30 - (3 + 5));
}
