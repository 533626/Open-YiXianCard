// ActualDamage(302) 持久计数语义契约测试。
//
// 原版语义（8aff06dd0e089b1c/round-11 玫刺回血 mismatch 根因，报告
// DIAG_20260809_8aff_meici_heal.md §2/§3）：
// - 302 是攻击者身上跨卡、跨回合持久累计的计数：凡走 ApplyDamage 的
//   Attack 型实际伤害都累加（BattleCharacter.cs:10858-10861），包括无
//   invocation 帧的回合末攻击（fate 137 凝水化刃，flow.rs 水势钩子）。
// - 只有该攻击者自己出牌完成时（OnAfterExecuted，CardActionBase.cs:
//   4743-4745）才把 302 转入 644(JiLuZongJiShangZhi) 并清零 302/303。
// - 玫刺(7000027) 等家族卡在自身攻击后读 302：读到「残留 + 本卡」。
// 引擎以 turn 级 actual_damage_carry / wounded_count_carry /
// ji_lu_zong_ji_shang_zhi 表达，每次 effect invocation 完成时 flush。
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing card {id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(0)
}

fn deck_with(cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    let mut deck = cards;
    while deck.len() < DECK_SIZE {
        deck.push(basic_attack());
    }
    deck
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 30,
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

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
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
        max_actor_turns: Some(4),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers {
            p1: player(p1_cards),
            p2: player(p2_cards),
        },
    }
}

fn activate(state: &mut ReplayState, element: Element) {
    state.p1.elements.activated_elements.push(element);
}

// ---- 语义级：fate 137 回合末攻击残留 → 玫刺回血 ----

#[test]
fn meici_heal_reads_persistent_actual_damage_carry_with_fate_137_residue() {
    // 8aff06dd0e089b1c/round-11 根因复现（语义级）：p1 带 fate 137 凝水化刃，
    // 回合末自动攻击 水势 7 × 1.5 = 10 实际伤害 → 计入 p1 的 302 持久计数
    //（无 invocation 帧路径，引擎此前静默丢弃）。玫刺在自身攻击后读到的 =
    // 残留 10 + 本卡 3 = 13 → 回血 13/3 = 4；卡完成时 flush：carry=0、
    // ji_lu_zong_ji_shang_zhi += 13。
    let mut battle = fixture(
        deck_with(vec![basic_attack()]),
        deck_with(vec![basic_attack()]),
    );
    battle.players.p1.fate_strategies = vec![137];
    battle.players.p2.initial_defense = 0;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.elements.water_momentum = 7; // (7+0)×1.5 = 10（floor）

    // p1 回合：普通攻击 3（完成时 flush → carry 0）→ 回合末凝水化刃
    // 水势 7 × 1.5 = 10（floor）→ carry = 10，p2 hp 30 → 17。
    state.test_play_actor_turn();
    assert_eq!(state.p1.turn.actual_damage_carry, 10);
    assert_eq!(state.p2.core.hp, 17);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 3);

    // p2 回合（普通攻击 3 → p1 27）：不触碰 p1 的 carry，残留跨回合存活。
    state.test_advance_actor();
    state.test_play_actor_turn();
    assert_eq!(state.p1.turn.actual_damage_carry, 10);
    assert_eq!(state.p1.core.hp, 27);

    // p1 打玫刺：木灵已激活，本卡 4×3 段 vs p2 防御 9（防御逐段消耗）→
    // 实际伤害 3；读值 = 残留 10 + 本卡 3 = 13 → 回血 13/3 = 4（41→45 语义）。
    state.test_advance_actor();
    state.p1.core.hp = 20; // 回血断言留出上限余量（maxHp 30）
    state.p2.core.defense = 9;
    activate(&mut state, Element::Wood);
    let meici = original_card(7_000_027);
    state.test_apply_card_effect(PlayerSide::P1, &meici, 0);

    assert_eq!(state.p2.core.hp, 17 - 3);
    assert_eq!(state.p1.core.hp, 24); // 20 + 4
                                      // 卡完成时 flush：302 → 644（13）、303 清零、carry 清零。
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 3 + 13);
    assert_eq!(state.p1.turn.actual_damage_carry, 0);
    assert_eq!(state.p1.turn.wounded_count_carry, 0);
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

// ---- 家族护栏：残留 > 0 时读值 = 残留 + 本卡 ----

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

    // 本卡 6 实际伤害 → 读值 8 → 削减 8×3 = 24（无残留对照为 6×3 = 18）。
    assert_eq!(state.p2.core.max_hp, 30 - 24);
    assert_eq!(state.p1.turn.ji_lu_zong_ji_shang_zhi, 8);
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
