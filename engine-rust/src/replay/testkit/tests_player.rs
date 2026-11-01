use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::CardDefinition;

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck() -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![basic_attack()], basic_attack())
}

fn player() -> FixturePlayer {
    super::test_support::make_player(deck(), 5, 50, Some(0), 8, Some(6))
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
}

#[test]
fn missing_permanent_physique_limit_is_zero_and_gains_overflow() {
    // BattleCharacter.cs:149-153 只在永久值 > 0 时写入 TiPoShangXian；缺失即 0，
    // :10256 起的溢出判定对首点体魄就生效（golden pa_72349f76/round-01 等 p 侧 10024=0）。
    let mut p1 = player();
    p1.permanent_buff_temp_datas.remove("10024");
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player()));
    assert_eq!(state.p1.core.physique_limit, 0);
    let hp_before = state.p1.core.hp;

    state.apply_physique_amount(PlayerSide::P1, 2);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.max_hp, 52);
    assert_eq!(state.p1.core.hp, hp_before + 2);
}

#[test]
fn talent_183_start_physique_counts_as_battle_physique_gain() {
    let mut p1 = player();
    p1.talents = vec![183, 184];
    p1.fate_strategies = vec![166];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 4);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.physique, 6);
    assert_eq!(state.p1.core.hp, 51);
    assert_eq!(state.p1.core.defense, 2);
    assert_eq!(state.p1.turn.battle_physique_gain_count, 2);
}

#[test]
fn talent_183_opening_heals_only_new_physique_overflow() {
    let mut p1 = player();
    p1.talents = vec![183];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 5);
    p1.permanent_buff_temp_datas.insert("10024".to_string(), 5);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.physique, 6);
    assert_eq!(state.p1.core.max_hp, 56);
    assert_eq!(state.p1.core.hp, 51);
}

#[test]
fn robust_bones_variants_stack_start_hp_healing() {
    let mut p1 = player();
    p1.talents = vec![10_176, 20_176];
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 35);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.core.max_hp, 85);
    assert_eq!(state.p1.core.hp, 58);
}

#[test]
fn permanent_power_loss_grass_initializes_attack_reduction() {
    let mut p1 = player();
    p1.permanent_buff_temp_datas.insert("10018".to_string(), 2);

    let state = ReplayState::test_from_fixture(&fixture(p1, player()));

    assert_eq!(state.p1.status.attack_reduction, 0);
    assert_eq!(state.p2.status.attack_reduction, 2);
}

#[test]
fn power_loss_grass_from_first_actor_ignores_not_yet_granted_gun_stance() {
    // BattleCharacter.OnBattleStarted 顺序（:897-907 发架势、:2502-2506
    // 失力草对对手施加减攻；BattleExecuter 先 await 首位再启动次位）：
    // 首位施加时次位尚未获得棍架势，206 不能抵扣；次位反施时首位已有架势，
    // 206 正常抵扣 1 层。HF 锚点：pa_769a28d0、
    // pa_48411345 的 cp0 减攻 1 vs 引擎 0。
    let mut owner = player();
    owner.permanent_buff_temp_datas.insert("10018".to_string(), 1);
    let mut recipient = player();
    recipient.character_id = Some(4_000_005);
    recipient.xian_mo_strategies = vec![7111];
    recipient.talents = vec![206];

    let state = ReplayState::test_from_fixture(&fixture(owner, recipient));

    assert_eq!(state.p2.beng.gun_stance, 1);
    assert_eq!(state.p2.status.attack_reduction, 1);
}

#[test]
fn power_loss_grass_from_second_actor_is_reduced_by_granted_gun_stance() {
    let mut recipient = player();
    recipient.character_id = Some(4_000_005);
    recipient.xian_mo_strategies = vec![7111];
    recipient.talents = vec![206];
    let mut owner = player();
    owner.permanent_buff_temp_datas.insert("10018".to_string(), 1);

    let state = ReplayState::test_from_fixture(&fixture(recipient, owner));

    assert_eq!(state.p1.beng.gun_stance, 1);
    assert_eq!(state.p1.status.attack_reduction, 0);
}
