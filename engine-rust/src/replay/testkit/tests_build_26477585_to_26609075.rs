//! Build 26477585 → 26609075 换代最小契约。
//!
//! 原版证据（`research/original-game/BUILD_26477585-to-26609075_RULE_DELTA.md`）：
//! - 仙魔 3001 开局分支新增 `IsSwitchActive(src, 3001)` 门
//!   （new `XianMoStrategyFunctions.cs:1078`；配置同步 `category → Switch`）。
//!   缺 `tempDatas[3001]` 视为 0（开启），与旧行为一致。
//! - 幻•锻骨（277）三档 `otherParams[2]` 0/5/10 → 5/10/15：引擎本就动态读参，
//!   此处钉住新配置值真实流入治疗结算（防配置/读参口径漂移）。
//! 其余本轮 live 项（11 卡数值、ShenFa→810 / DiXuanGui→806 Hidden 追踪写）走
//! 配置轮换或 exact-neutral，见 RULE_DELTA §2/§4；死内容（31 Card_*、7 死策略、
//! 15 新 buff、274/275）不实现。

use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

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

fn battle_3001(switch: Option<i64>) -> BattleFixture {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![3001];
    if let Some(value) = switch {
        battle
            .players
            .p1
            .xian_mo_temp_datas
            .insert("3001".to_string(), value);
    }
    battle
}

#[test]
fn xian_mo_3001_missing_switch_defaults_active() {
    // 缺 tempDatas 即 IsSwitchActive（== 0）→ 对手 JianGong(103) +1，旧行为不变。
    let state = ReplayState::test_from_fixture(&battle_3001(None));
    assert_eq!(state.p2.status.attack_reduction, 1);
}

#[test]
fn xian_mo_3001_switch_zero_is_active() {
    let state = ReplayState::test_from_fixture(&battle_3001(Some(0)));
    assert_eq!(state.p2.status.attack_reduction, 1);
}

#[test]
fn xian_mo_3001_switch_nonzero_skips_opponent_jian_gong() {
    // 26609075 新语义：开关关闭（!= 0）时跳过 JianGong 授予。
    let state = ReplayState::test_from_fixture(&battle_3001(Some(1)));
    assert_eq!(state.p2.status.attack_reduction, 0);
}

#[test]
fn card_277_new_configured_heal_flows_into_settlement() {
    // 三档 otherParams[2] 新值 5/10/15（旧 0/5/10）必须真实流入治疗。
    for (id, expected) in [(277, 5), (10277, 10), (20277, 15)] {
        let card = original(id);
        assert_eq!(
            card.other_params.get(2).copied().unwrap_or(-1),
            expected,
            "card {id} otherParams[2]"
        );
        let battle = fixture(deck(card.clone()), deck(basic_attack()));
        let mut state = ReplayState::test_from_fixture(&battle);
        state.p1.core.hp = 10;
        state.apply_synthetic_oracle_dream_mirage_card_effect(
            PlayerSide::P1,
            &card,
            0,
            false,
            277,
        );
        assert_eq!(state.p1.core.hp, 10 + expected, "card {id} heal");
    }
}

#[test]
fn xian_mo_5009_repeats_base_rank_definition_for_rarity_card() {
    // 26609075：5009 复用格重复 1 级效果（CardActionBase.cs:1406-1432，
    // rarity>0 时 cardInfo.id→GetBaseCardId；XianMoStrategyDesc_5009 同步）。
    // 1010002（1 级云剑•飞刺，攻 6/op[5]）：primary 打 6 并建立连云，
    // 重复按 1000002 定义打 5+连云 3=8 → 30-6-8=16。旧规则重复 6+5=11 → 13。
    let card = original(1010002);
    let mut twin = fixture(deck(card.clone()), deck(basic_attack()));
    let mut baseline_state = ReplayState::test_from_fixture(&twin);
    baseline_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(baseline_state.p2.core.hp, 30 - 6);
    let played_slot = baseline_state
        .test_events()
        .iter()
        .find(|event| {
            event.kind == super::ReplayEventKind::CardCompleted && event.actor == PlayerSide::P1
        })
        .and_then(|event| event.slot)
        .expect("baseline must complete one selected card");

    twin.players.p1.xian_mo_strategies = vec![5009];
    twin.players
        .p1
        .xian_mo_temp_datas
        .insert("5009".into(), played_slot as i64);
    let mut twin_state = ReplayState::test_from_fixture(&twin);
    twin_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(twin_state.p2.core.hp, 16);
    // 主牌仍为原级定义（还原语义：替换只活在重复执行内）。
    assert_eq!(twin_state.test_actor_card(PlayerSide::P1, 0).id, card.id);
}
