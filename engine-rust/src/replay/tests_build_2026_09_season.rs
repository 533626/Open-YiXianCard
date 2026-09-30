//! build 25621897（仙魔赛季）换代中既有卡的最小契约：只覆盖本次源码 / 配置改动点。

use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::MirageRonghuiValue;
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

#[test]
fn star_shift_269_marks_persistent_effect_once_regardless_of_immediate_slots() {
    // Card_269.cs：立即成为星位 otherParams[0] 格，持续标记 HuanDouZhuanJiaXingWei 固定 +1。
    let mut star_shift = card(20_269, 269, "幻•斗转星移");
    star_shift.rarity = Some(2);
    star_shift.other_params = vec![3, 6, 2].into();
    let battle = fixture(deck(star_shift), deck(basic_attack()));

    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::StarShift), 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::StarShiftAttack),
        6
    );
}

#[test]
fn steadfast_291_raises_momentum_limit_before_momentum() {
    // Card_291.cs：QiShiShangXian += otherParams[0] 先于 QiShi += otherParams[0]。
    let mut steadfast = card(291, 291, "幻•岿然不动");
    steadfast.other_params = vec![3, 1, 1].into();
    let battle = fixture(deck(steadfast), deck(basic_attack()));

    let mut state = ReplayState::test_from_fixture(&battle);
    let limit_before = state.p1.beng.momentum_limit;
    state.test_configure_p1(|player| player.beng.momentum = limit_before);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.beng.momentum_limit, limit_before + 3);
    assert_eq!(state.p1.beng.momentum, limit_before + 3);
}

#[test]
fn earth_formation_317_level_three_removes_turn_end_defense_cap() {
    // Card_317.cs：rarity 2 写 HuanTuLingZhenYiChuShangXian；OnTurnEnded 持有时水势加防不截断。
    for (rarity, expected_defense) in [(1, 10), (2, 25)] {
        let mut formation = card(10_317 + (rarity - 1) * 10_000, 317, "幻•土灵阵");
        formation.rarity = Some(rarity);
        formation.other_params = vec![1, 10].into();
        let battle = fixture(deck(formation), deck(basic_attack()));

        let mut state = ReplayState::test_from_fixture(&battle);
        state.test_execute_one_card(PlayerSide::P1);
        assert_eq!(
            state.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::MirageWaterDefenseUncapped),
            i64::from(rarity == 2)
        );
        state.test_configure_p1(|player| {
            player.elements.water_momentum = 25;
            player.core.defense = 0;
        });
        state.apply_mirage_ronghui_turn_end(PlayerSide::P1);
        assert_eq!(state.p1.core.defense, expected_defense, "rarity {rarity}");
    }
}

#[test]
fn battle_relevant_xian_mo_strategy_fails_closed_until_implemented() {
    // 1001 天灵之音：XianMoStrategyFunctions.OnBattleStart 双方加灵气，未实现 → MissingRule。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p2.xian_mo_strategies = vec![1001];
    let error = ReplayState::from_fixture(&battle, true)
        .expect_err("strict construction rejects unimplemented xianMo strategy");
    assert!(error.to_string().contains("xianMo strategy 1001"), "{error}");

    // 12003 AddCardPool：战斗外（卡池），不影响战斗 → 放行。
    battle.players.p2.xian_mo_strategies = vec![12003];
    assert!(ReplayState::from_fixture(&battle, true).is_ok());
}
