// ActualDamage(302) 持久计数语义契约测试。
//
// 原版语义（8aff06dd0e089b1c/round-11 玫刺回血 mismatch 根因，报告
// DIAG_20260809_8aff_meici_heal.md §2/§3）：
// - 302 是攻击者身上跨卡、跨回合持久累计的计数：凡走 ApplyDamage 的
//   Attack 型实际伤害都累加（BattleCharacter.cs:10858-10861），包括无
//   invocation 帧的回合末攻击（fate 137 凝水化刃，flow.rs 水势钩子）。
// - 只有该攻击者自己出牌完成时（OnAfterExecuted，CardActionBase.cs:
//   4743-4745）才把 302 转入 644(JiLuZongJiShangZhi) 并清零 302/303。
// - OnBeforeExecuted 末尾（CardActionBase.cs:3221-3222）先清零 302/303（不转 644），
//   回合末攻击等残留不会被下一张牌读到；玫刺(7000027) 等家族卡在自身攻击后读 302，
//   读到的是本卡（执行前钩子之后）累计的实际伤害。
// 引擎以 turn 级 actual_damage_carry / wounded_count_carry /
// ji_lu_zong_ji_shang_zhi 表达，每次 effect invocation 完成时 flush。
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck_with(cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    super::test_support::fill_deck(cards, basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 30, Some(0), 8, Some(6))
}

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
    super::test_support::make_fixture(player(p1_cards), player(p2_cards), PlayerSide::P1, 1, 0, 4)
}

fn activate(state: &mut ReplayState, element: Element) {
    state.p1.elements.activated_elements.push(element);
}


#[test]
fn meici_no_residue_heals_own_damage_only() {
    // 无残留回归：carry 起点 0 时玫刺回血 = 本卡实际伤害 / 3，行为与修复前
    // 一致；卡完成时 flush 只转移本卡伤害。
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![original_card(7_000_027)]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Wood);
    state.p1.core.hp = 20;
    let meici = original_card(7_000_027);
    state.test_apply_card_effect(PlayerSide::P1, &meici, 0);

    assert_eq!(state.p2.core.hp, 30 - 12); // 4×3 段 vs 防御 0 → 12 实际伤害
    assert_eq!(state.p1.core.hp, 24); // 回血 12/3 = 4（20 + 4，无残留行为不回归）
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 12);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
    assert_eq!(state.p1.turn.wounded_count_carry, 0);
}

#[test]
fn wounded_count_carry_accumulates_per_wounding_attack_and_flushes() {
    // 303 与 302 同生命周期：造成实际伤害的攻击段累加 wounded_count_carry，
    // 出牌完成时一并清零（原版 OnAfterExecuted 4745）。
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![basic_attack()]),
        deck_with(vec![basic_attack()]),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &basic_attack(), 0);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
    assert_eq!(state.p1.turn.wounded_count_carry, 0);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 3);

    // 无 invocation 帧的攻击（回合末凝水化刃同类路径）也累加 303/302。
    state.p1.elements.water_blade_seal = 1;
    state.apply_attack(PlayerSide::P1, 4, usize::MAX);
    state.p1.elements.water_blade_seal = 0;
    assert_eq!(state.p1.turn.actual_damage_carry, 6); // 4 × 1.5
    assert_eq!(state.p1.turn.wounded_count_carry, 1);
}

// ---- 家族护栏：读值取执行时 302 全量（直接预置 carry 验证读取口径） ----

#[test]
fn fen_hua_yin_413_reads_carry_with_residue() {
    // 原版 Card_413.cs:96 减对方生命上限 302×otherParams[0]（残留 + 本卡）。
    let card = original_card(413);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Fire);
    state.p1.turn.actual_damage_carry = 2; // 残留（如回合末攻击）
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 7 实际伤害 → 读值 9 → 削减 9×3 = 27（无残留对照为 7×3 = 21）。
    assert_eq!(state.p2.core.max_hp, 30 - 27);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 9);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
}

#[test]
fn mu_ling_zhan_373_reads_carry_with_residue() {
    // 原版 Card_373.cs:81-83 回血 302/otherParams[1]（残留 + 本卡）。
    let card = original_card(373);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Wood);
    state.p1.core.hp = 20;
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 10 → 读值 12 → 回血 12/3 = 4（无残留对照为 10/3 = 3）。
    assert_eq!(state.p1.core.hp, 24);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 12);
}

#[test]
fn tu_ling_zhan_375_reads_carry_with_residue() {
    // 原版 Card_375.cs:81-83 加防 302/otherParams[1]（残留 + 本卡）。
    let card = original_card(375);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Earth);
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 10 → 读值 12 → 防御 +12/3 = 4（无残留对照为 10/3 = 3）。
    assert_eq!(state.p1.core.defense, 4);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 12);
}

#[test]
fn jin_ling_zhan_376_reads_carry_with_residue() {
    // 原版 Card_376.cs:81-83 加锋锐 302/otherParams[1]（残留 + 本卡）。
    let card = original_card(376);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Metal);
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 10 → 读值 12 → 锋锐 +12/4 = 3（无残留对照为 10/4 = 2）。
    assert_eq!(state.p1.sword.sharpness, 3);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 12);
}

#[test]
fn shui_ling_zhan_377_reads_carry_with_residue() {
    // 原版 Card_377.cs:81-83 加水势 302/otherParams[1]（残留 + 本卡）。
    let card = original_card(377);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Water);
    state.p1.turn.actual_damage_carry = 5;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 10 → 读值 15 → 水势 +15/5 = 3（无残留对照为 10/5 = 2）。
    assert_eq!(state.p1.elements.water_momentum, 3);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 15);
}

#[test]
fn jin_ling_feng_mang_7000026_reads_carry_with_residue() {
    // 原版 Card_7000026.cs:62-64 锋锐 +302/2（残留 + 本卡）。
    let card = original_card(7_000_026);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Metal);
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 6 → 读值 8 → 锋锐 +8/2 = 4（无残留对照为 6/2 = 3）。
    assert_eq!(state.p1.sword.sharpness, 4);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 8);
}

#[test]
fn huo_ling_zhuo_xin_7000039_reads_carry_with_residue() {
    // 原版 Card_7000039.cs:95-108 减对方生命上限 302×otherParams[0]
    //（残留 + 本卡），与 413 焚花印同构。
    let card = original_card(7_000_039);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    activate(&mut state, Element::Fire);
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 4×2 段 = 8 → 读值 10 → 削减 10×2 = 20（无残留对照为 8×2 = 16）。
    assert_eq!(state.p2.core.max_hp, 30 - 20);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 10);
}

#[test]
fn dream_jin_ling_feng_mang_7030085_reads_carry_with_residue() {
    // 原版 Card_7000085.cs:63-71 水势 +302/otherParams[0]；7030085 为
    // 返虚/元婴 realm 4 版本，走 302 读取分支（realm > 3）。
    let card = original_card(7_030_085);
    assert_eq!(card.other_params, vec![3]);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    state.p1.turn.actual_damage_carry = 4;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 6 → 读值 10 → 水势 +10/3 = 3（无残留对照为 6/3 = 2）。
    assert_eq!(state.p1.elements.water_momentum, 3);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 10);
}

#[test]
fn dream_shui_ling_xiong_yong_7030103_reads_carry_with_residue() {
    // 原版 Card_7000103.cs:84-92 生命及上限 +302/otherParams[0]；7030103
    // 为 realm 4 版本，走 302 读取分支（realm >= 4）。
    let card = original_card(7_030_103);
    assert_eq!(card.other_params, vec![3]);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    state.p1.core.hp = 20;
    state.p1.turn.actual_damage_carry = 4;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 5 → 读值 9 → 生命及上限 +9/3 = 3（无残留对照为 5/3 = 1）。
    assert_eq!(state.p1.core.max_hp, 33);
    assert_eq!(state.p1.core.hp, 23);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 9);
}

#[test]
fn di_sha_jian_1000030_reads_carry_with_residue() {
    // 原版 Card_1000030.cs 击伤且 302 > 0 时防御 + 完整 302（残留 + 本卡），
    // 不再用 invocation-local 差值 hack。
    let card = original_card(1_000_030);
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with(vec![card.clone()]),
        deck_with(vec![basic_attack()]),
    ));
    state.p1.turn.actual_damage_carry = 2;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 本卡 8（击伤）→ 读值 10 → 防御 +10（无残留对照为 +8）。
    assert_eq!(state.p1.core.defense, 10);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 10);
}

// ---- OnBeforeExecuted 末尾清零 302/303（不转 644） ----

fn meici_state() -> ReplayState {
    ReplayState::test_from_fixture(&fixture(
        deck_with(vec![original_card(7_000_027)]),
        deck_with(vec![basic_attack()]),
    ))
}

#[test]
fn before_execute_clears_residue_without_ledger() {
    let mut state = meici_state();
    state.p1.turn.actual_damage_carry = 10;
    state.p1.turn.wounded_count_carry = 2;
    state.p1.turn.ji_lu_zong_ji_shang_zhi = 3;
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &basic_attack(), 0, false);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
    assert_eq!(state.p1.turn.wounded_count_carry, 0);
    // RemoveBuff 直接丢弃，不走 OnAfterExecuted 的 302→644 转移。
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 3);
}

#[test]
fn meici_heals_own_damage_only_when_residue_is_cleared() {
    // 残留 10（如 fate 137 回合末攻击）在牌体前被清掉，玫刺只读本卡实际伤害 3 →
    // 回血 3/3 = 1；完成时 644 只加本卡 3。
    let mut state = meici_state();
    state.p1.turn.actual_damage_carry = 10;
    state.p1.core.hp = 20;
    state.p2.core.defense = 9;
    activate(&mut state, Element::Wood);
    let meici = original_card(7_000_027);
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &meici, 0, false);
    state.test_apply_card_effect(PlayerSide::P1, &meici, 0);
    assert_eq!(state.p2.core.hp, 30 - 3);
    assert_eq!(state.p1.core.hp, 21);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 3);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
}
