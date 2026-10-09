//! build 25796222（平衡热更）换代中既有卡/机制的最小契约：只覆盖本次源码 / 配置改动点。

use super::support::other_param;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    super::test_support::test_card(id, base_id, name)
}

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::resize_deck(vec![active], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 1, 30, None, 1, None)
}

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
    super::test_support::default_fixture(player(p1_cards), player(p2_cards))
}

fn original(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

#[test]
fn zhuan_shi_222_grants_base_agility_without_stance() {
    // Card_222.cs：先无条件 ShenFa += otherParams[0]；无拳/棍架势时不攻击、不切架势。
    let zhuan_shi = original(222);
    let battle = fixture(deck(zhuan_shi.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.beng.quan_stance = 0;
    state.p1.beng.gun_stance = 0;
    let p2_hp = state.p2.core.hp;

    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0);

    assert_eq!(state.p1.turn.agility, other_param(&zhuan_shi, 0));
    assert_eq!(state.p2.core.hp, p2_hp);
    assert_eq!((state.p1.beng.quan_stance, state.p1.beng.gun_stance), (0, 0));
}

#[test]
fn kuang_lei_29_gains_configured_anima_before_flaw() {
    // Card_29.cs：ModifyAnima(anima) 后给对方破绽 otherParams[0]。
    let thunder = original(29);
    let battle = fixture(deck(thunder.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    let anima = state.p1.core.anima;

    state.test_apply_card_effect(PlayerSide::P1, &thunder, 0);

    assert_eq!(state.p1.core.anima, anima + thunder.anima.unwrap());
    assert_eq!(state.p2.status.flaw, other_param(&thunder, 0));
}

#[test]
fn yin_fu_214_raises_max_hp_and_hp_first() {
    // Card_214.cs：ModifyMaxHp / ModifyHp(otherParams[3]) → 虚弱 → 破绽 → 阴符玉简。
    let jade = original(214);
    let battle = fixture(deck(jade.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    let (hp, max_hp) = (state.p1.core.hp, state.p1.core.max_hp);

    state.test_apply_card_effect(PlayerSide::P1, &jade, 0);

    let gain = other_param(&jade, 3);
    assert_eq!(gain, 3);
    assert_eq!(state.p1.core.max_hp, max_hp + gain);
    assert_eq!(state.p1.core.hp, hp + gain);
    assert_eq!(state.p2.status.weakness, other_param(&jade, 0));
    assert_eq!(state.p2.status.flaw, other_param(&jade, 1));
}

#[test]
fn fate_36_counts_hidden_basic_attack_286() {
    // FateStrategyFunctions.OnPlayCard：32/33/36 改用 IsPuTongGongJi（基础 id 0 或 286）。
    let hidden_attack = card(286, 286, "隐藏普通攻击");
    let mut battle = fixture(deck(hidden_attack.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![36];
    let mut state = ReplayState::test_from_fixture(&battle);

    state.apply_selected_card_hooks(PlayerSide::P1, &hidden_attack, 0);

    assert_eq!(state.p1.core.attack_bonus, 2);
}

#[test]
fn spirit_formation_echo_keeps_rarity_zero_realm_variant() {
    // CardActionBase.Execute 灵阵回响：仅 cardConfig.rarity > 0 时回响基础牌；
    // 梦•金灵阵 7010074 配置无 rarity（0），按自身 id 回响。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.formations.spirit_formation_echo_triggered = true;

    let dream = original(7_010_074);
    assert_eq!(state.spirit_formation_echo_card(PlayerSide::P1, &dream).id, 7_010_074);
    let upgraded = original(10_208);
    assert_eq!(state.spirit_formation_echo_card(PlayerSide::P1, &upgraded).id, 208);
}

#[test]
fn fei_yan_ling_jian_opening_only_supplies_missing_anima() {
    // HF 00e5d014aeecaad1/round-08 cp0: p1 anima=1 before its first card.
    // BattleCharacter.TriggerOpening case 1000101 and Card_1000101.OnExecuted.
    for id in [1_000_101, 1_010_101, 1_020_101] {
        let swallow = original(id);
        let battle = fixture(deck(swallow.clone()), deck(basic_attack()));
        let mut state = ReplayState::test_from_fixture(&battle);
        state.p1.core.anima = 0;
        state.apply_opening_effect_for_card(PlayerSide::P1, &swallow, 0);
        assert_eq!(state.p1.core.anima, other_param(&swallow, 0));
        state.p1.core.anima = 2;
        state.apply_opening_effect_for_card(PlayerSide::P1, &swallow, 0);
        assert_eq!(state.p1.core.anima, 2);
        let before = state.p2.core.hp;
        state.test_apply_card_effect(PlayerSide::P1, &swallow, 0);
        assert_eq!(state.p2.core.hp, before - swallow.attack.unwrap() * swallow.attack_count.unwrap());
    }
}

#[test]
fn ling_ping_activates_original_named_element_of_unimplemented_card() {
    // HF 31940270d3d4425e/round-17 cp0: 灵瓶 contains 木灵•散枝叶;
    // original ActiveWuXingInName activates wood, triggering 200/202.
    assert_eq!(super::support::element_from_card(&original(7_000_112)), Some(Element::Wood));
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![199, 200, 202];
    battle.players.p1.talent_card_params.insert("199".into(), vec![7_000_112]);
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.max_hp, 32);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.anima, 1);
    assert!(state.p1.elements.activated_elements.contains(&Element::Wood));
    // Only metadata is needed for a card stored in the bottle.
}

#[test]
fn fei_yan_ling_jian_receives_hundred_bird_spirit_sword_cost_reduction() {
    // HF e64e011e62f10f87/round-08 cp13: original spends zero anima.
    let swallow = original(1_000_101);
    let battle = fixture(deck(swallow.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.hundred_bird_spirit_sword_art = 1;
    assert_eq!(super::support::effective_anima_cost(&swallow, &state.p1, Some(0)), 0);
}

#[test]
fn xian_mo_sword_bodies_preserve_cloud_count_and_defense_sampling() {
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    let sea = original(1_000_104);
    state.p1.sword.cloud_chain = 2;
    state.test_apply_card_effect(PlayerSide::P1, &sea, 0);
    assert_eq!(state.p1.sword.cloud_chain, 4);
    assert_eq!(state.p1.sword.cloud_sea, 2);
    assert_eq!(state.p1.core.anima, 3);
    let glow = original(1_000_107);
    state.p1.core.anima = 10;
    state.p1.sword.sword_intent = 2;
    state.test_apply_card_effect(PlayerSide::P1, &glow, 0);
    assert_eq!(state.p1.sword.sword_intent, 3);
    assert_eq!(state.p1.core.defense, 8);
    let blades = original(1_000_108);
    state.p1.core.defense = 15;
    state.p2.core.hp = 100;
    state.p2.core.max_hp = 100;
    state.p1.sword.sword_intent = 0;
    state.test_apply_card_effect(PlayerSide::P1, &blades, 0);
    assert_eq!(state.p2.core.hp, 84);
}

#[test]
fn san_zhi_ye_arms_only_in_wood_and_drains_before_the_next_body() {
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    let branches = original(7_000_112);
    state.test_apply_card_effect(PlayerSide::P1, &branches, 0);
    assert_eq!(state.p1.elements.wood_scattered_branches, 0);
    state.activate_element(PlayerSide::P1, Element::Wood);
    state.test_apply_card_effect(PlayerSide::P1, &branches, 0);
    assert_eq!(state.p1.elements.wood_scattered_branches, 3);
    state.p1.core.attack_bonus = 2;
    state.p1.core.hp = 20;
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &basic_attack(), 0, false);
    assert_eq!((state.p1.core.hp, state.p2.core.hp), (23, 27));
    assert_eq!(state.p1.elements.wood_scattered_branches, 2);
}

#[test]
fn ran_jin_mie_uses_fire_earth_metal_bonuses_without_attack() {
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    for element in [Element::Fire, Element::Earth, Element::Metal] {
        state.activate_element(PlayerSide::P1, element);
    }
    state.p1.core.attack_bonus = 2;
    state.p1.core.defense = 9;
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 100;
    state.p2.core.max_hp = 100;
    state.p2.core.defense = 99;
    state.test_apply_card_effect(PlayerSide::P1, &original(7_000_113), 0);
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp, state.p2.core.defense), (79, 79, 99));
}

#[test]
fn ju_ling_zhen_heals_before_internal_injury_at_the_hp_cap() {
    // HF 32f8ca23ebece021/round-12 cp7: original p2 def4 versus old Rust3.
    // OnTurnStarted JuLinZhen IL_0e56, NeiShang IL_1a4c;
    // ModifyAnima -> JingQiXinFa -> ModifyHp -> Talent64Defense.
    let mut battle = fixture(deck(original(12)), deck(basic_attack()));
    battle.players.p1.talents = vec![64];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 28;
    state.p1.fate.quiet_mindset = 2;
    state.p1.formations.spirit_gathering_formation = 1;
    state.p1.formations.spirit_gathering_formation_value = 2;
    state.p1.status.internal_injury = 2;
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.defense, 3);
}

#[test]
fn shen_long_bai_wei_heals_then_uses_first_strike_jump_or_turn_healing_defense() {
    let dragon = original(4_000_103);
    let mut state = ReplayState::test_from_fixture(&fixture(deck(dragon.clone()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.test_apply_card_effect(PlayerSide::P1, &dragon, 0);
    assert_eq!(state.p1.core.hp, 17);
    assert_eq!(state.p1.turn.jump_to_previous_card, 1);
    state.p1.core.hp = 10;
    state.p1.fate.next_rear_move_bypass = 1;
    state.p1.dream_mirage.turn_hp_gained = 2;
    state.test_apply_card_effect(PlayerSide::P1, &dragon, 0);
    assert_eq!(state.p1.core.defense, 4); // (2 + 7) / 2, after the heal.
}

#[test]
fn ling_yuan_wai_fang_overflow_heal_damages_opponent() {
    // Card_4000106 then BattleCharacter.ModifyHp:10110 ReflectDamage.
    let card = original(4_000_106);
    let mut state = ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
    state.p1.core.hp = 30;
    state.p1.core.max_hp = 30;
    state.p1.core.anima = 0;
    state.p2.core.hp = 80;
    state.p2.core.max_hp = 80;
    state.p2.core.defense = 0;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    let heal = other_param(&card, 0) + card.anima.unwrap_or(0);
    assert_eq!(state.p1.astrology.ling_yuan_wai_fang, other_param(&card, 1));
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.anima, card.anima.unwrap_or(0));
    assert_eq!(state.p2.core.hp, 80 - heal);
}

#[test]
fn xing_yi_lian_marks_current_star_grid_or_gains_star_power() {
    // CardActionBase.OnBeforeExecuted:3618-3629 and Card_4000104.
    let card = original(4_000_104);
    let follow = original(4_000_028); // 星弈•点
    let mut state = ReplayState::test_from_fixture(&fixture(
        super::test_support::resize_deck(vec![card.clone(), follow.clone()], basic_attack()),
        deck(basic_attack()),
    ));
    state.p1.core.hp = 20;
    state.p1.core.max_hp = 40;
    state.p1.astrology.star_slots.clear();
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.fate.xing_yi_lian, other_param(&card, 0));
    let heal = other_param(&original(4_000_104), 1);
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &follow, 1, true);
    assert_eq!(state.p1.astrology.star_slots, vec![1]);
    assert_eq!(state.p1.core.hp, 20 + heal);
    assert_eq!(state.p1.astrology.star_power, 0);
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &follow, 1, true);
    assert_eq!(state.p1.astrology.star_power, 1);
    assert_eq!(state.p1.fate.xing_yi_lian, other_param(&card, 0) - 2);
}

#[test]
fn zhu_chen_bu_grants_defense_to_the_attacker() {
    // BattleCharacter.ApplyDamage:11202-11209: attacker's ZhuChenBu, num/3 after def.
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.elements.zhu_chen_bu = 1;
    state.p1.core.defense = 0;
    state.p2.core.hp = 100;
    state.p2.core.max_hp = 100;
    state.p2.core.defense = 0;
    state.p2.status.external_injury = 0;
    state.apply_damage(PlayerSide::P1, 9, true, false, false);
    assert_eq!(state.p1.elements.zhu_chen_bu, 0);
    assert_eq!(state.p1.core.defense, 3);
    assert_eq!(state.p2.core.hp, 91);

    state.p1.elements.zhu_chen_bu = 1;
    state.p1.core.defense = 0;
    state.p2.core.defense = 20;
    state.apply_damage(PlayerSide::P1, 9, true, false, false);
    assert_eq!(state.p1.elements.zhu_chen_bu, 0);
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p2.core.defense, 11);

    // Guard (HuTi) absorbs all HP damage: ZhuChenBu consumes, but grants 0 defense.
    state.p1.elements.zhu_chen_bu = 1;
    state.p1.core.defense = 0;
    state.p2.core.hp = 100;
    state.p2.core.defense = 0;
    state.p2.core.guard = 1;
    state.apply_damage(PlayerSide::P1, 9, true, false, false);
    assert_eq!(state.p1.elements.zhu_chen_bu, 0);
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p2.core.hp, 100);

    // Damage multiplier (metal_cauldron_drop / 巨鼎落) doubles post-defense wound:
    // 9 atk against 7 def => 2 post-defense, doubled to 4 damage => 4 / 3 = 1 defense.
    state.p1.elements.zhu_chen_bu = 1;
    state.p1.elements.metal_cauldron_drop = 1;
    state.p1.core.defense = 0;
    state.p2.core.defense = 7;
    state.apply_damage(PlayerSide::P1, 9, true, false, false);
    assert_eq!(state.p1.elements.zhu_chen_bu, 0);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.hp, 96);
}

#[test]
fn qian_ren_jian_zhen_is_a_sword_formation_for_shared_callbacks() {
    // HF 0221752fc2c38bde/round-15 cp12: 连环剑阵 must copy 千刃剑阵,
    // not skip it and copy the earlier 御空剑阵.
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let state = ReplayState::test_from_fixture(&battle);
    assert!(super::support::is_sword_formation_card(&state.p1, &original(1_000_108)));
}

#[test]
fn beng_quan_fei_zhou_inherits_flaw_on_effective_beng_quan() {
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.beng.beng_tian_step = 1;
    state.p1.beng.beng_quan_fei_zhou = 2;
    state.execute_card_transaction(PlayerSide::P1);
    assert_eq!(state.p2.status.flaw, 2);
    assert_eq!(state.p1.beng.beng_quan_fei_zhou, 0);
}

#[test]
fn zero_negative_status_does_not_trigger_star_erosion() {
    // BattleCharacter.ModifyBuffValue:8681 delta==0 return.
    // 拨草寻蛇(4000105) otherParams[0]=0 internal injury must not trigger or consume BeiXingShi/StarErosion.
    let mut state = ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p2.astrology.star_erosion = 2;
    state.add_actor_negative_status(PlayerSide::P2, 100, 0);
    assert_eq!(state.p2.status.internal_injury, 0);
    assert_eq!(state.p2.astrology.star_erosion, 2);

    state.add_actor_negative_status(PlayerSide::P2, 100, 1);
    assert_eq!(state.p2.status.internal_injury, 3);
    assert_eq!(state.p2.astrology.star_erosion, 0);
}
