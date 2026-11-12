//! Build 26477585 换代最小契约：崩拳•伏虎（337）及伏虎附加伤害新增 `def > 0` 守卫。
//!
//! 原版证据：diff 26449625-to-26477585 仅改 2 个反编译源文件，语义各 +1 条语句：
//! - `Card_337.OnExecuted`：`if (src.battleTempData.def > 0)` 包住防转伤害
//!   （new Card_337.cs :95-98；old :95 无条件 `ApplyDamage`）。
//! - `CardActionBase.OnAfterExecuted` 的 ChuFaFuHu（679）分支：同款守卫包住
//!   `ApplyDamage(def, skipWoundCheck: true)`（new :5141；old :4706-4710 无条件）。
//! 配置表 9 张 0 行变更；Buff 678 BengQuanFuHu（Neutral）/ 679 ChuFaFuHu（Hidden）不变。

use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::cards_dream_mirage::DreamMirageValue;
use crate::replay::cards_mirage_ronghui::MirageRonghuiValue;

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
fn card_337_normal_path_damages_and_gains_fu_hu() {
    // 337（攻 7 / 防 7）：攻击 7 + 防转伤 7 + 伏虎 +1；守卫通过分支保持旧行为。
    let card337 = original(337);
    let battle = fixture(deck(card337.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.defense = 0;
    state.p2.core.defense = 0;
    state.apply_card_effect_missing(PlayerSide::P1, &card337, 0, false, 337);
    assert_eq!(state.p2.core.hp, 30 - 7 - 7);
    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p1.beng.beng_quan_fu_hu, 1);
}

#[test]
fn card_337_zero_defense_arm_skips_def_damage_but_gains_fu_hu() {
    // 防御经公开变更钳在 >= 0，337 自带 +7 防使本分支实践不可达；
    // 直写负防御只为钉住与原版一致的守卫语义（跳过伤害，伏虎照加）。
    let card337 = original(337);
    let battle = fixture(deck(card337.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.defense = -7;
    state.p2.core.defense = 0;
    state.apply_card_effect_missing(PlayerSide::P1, &card337, 0, false, 337);
    assert_eq!(state.p2.core.hp, 30 - 7);
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.beng.beng_quan_fu_hu, 1);
}

#[test]
fn fu_hu_positive_defense_still_damages_through_hook() {
    // 守卫通过分支：触发伏虎 + 防 3 → 3 点伤害，触发计数清零。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.defense = 3;
    state.p1.beng.triggered_beng_quan_fu_hu = 1;
    state.p2.core.defense = 0;
    state.apply_beng_quan_fu_hu_after_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, 30 - 3);
    assert_eq!(state.p1.beng.triggered_beng_quan_fu_hu, 0);
}

#[test]
fn xianmo_7057_weakness_counts_for_13017_defense() {
    // XianMo OnPlayCard 执行序不是文件序：IL_0507 段的 7057（:1464）经
    // 13010 后的 goto IL_119a 先于 IL_119a 段的 13017（:1386）。13017 数
    // 负面须包含本张卦牌首用的 7057 虚弱。
    // 锚点：pa_60517796 [1]（乾卦：p1 内伤 1 + 虚弱 1 → 防+2；
    // 旧顺序先数后加得防+1）、同 match round-13 [3]（震卦：内伤 1 + 虚弱
    // 2 → 防+3；旧顺序得防+2）。
    let qian = original(4_000_026);
    let battle = fixture(deck(qian.clone()), deck(basic_attack()));
    let mut battle = battle;
    battle.players.p1.xian_mo_strategies = vec![7_057, 13_017];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &qian, 0, false);
    assert_eq!(state.p2.status.internal_injury, 1);
    assert_eq!(state.p2.status.weakness, 1);
    assert_eq!(state.p1.core.defense, 2);
}

#[test]
fn physique_overflow_counts_nested_talent_267_gain() {
    // BattleCharacter.ModifyTiPo 源序：163/7042 灵气（含嵌套的 267 体魄，
    // 同步完成）先于生命上限与溢出；外层溢出 num3 按嵌套完成后的当前体魄
    // 算（min(本层 delta, 当前体魄-上限)）。
    // 锚点：pa_80c00584 [2]（锻神开海体魄+4 嵌套 267 的 +1：
    // 62→66→67，上限 65，内层溢出 1 + 外层溢出 2 = 治疗 3；旧顺序外层
    // 先算得 1 + 1 = 2）、同 match round-17 [3]（+5 嵌套 +1：87→92→93，
    // 上限 90，治疗 1 + 3 = 4；旧顺序得 2 + 1 = 3）。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut battle = battle;
    battle.players.p1.talents = vec![267];
    battle.players.p1.xian_mo_strategies = vec![7_042];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.physique = 62;
    state.p1.core.physique_limit = 65;
    state.p1.core.hp = 100;
    state.p1.core.max_hp = 200;
    state.p1.turn.wind_spirit_body_forge_count = 1;
    state.apply_physique_amount(PlayerSide::P1, 4);
    assert_eq!(state.p1.core.physique, 67);
    assert_eq!(state.p1.core.max_hp, 205);
    assert_eq!(state.p1.core.hp, 103);
}

#[test]
fn fumo_13014_defense_uses_pre_707_physique() {
    // XianMo OnPlayCard（:3300，含 13014）先于 707 下张崩拳加体魄消费
    // （CardActionBase :3365），二者皆先于星弈连（:3676）。13014 按自身体魄
    // +2 后的旧体魄算防御（floor(体魄/6)），707 体魄后叠。
    // 锚点：pa_cf4ee85a [3]（崩拳•戳：体魄 32+2=34，防
    // floor(34/6)=5，再叠 707 的 +2 到 36；旧顺序 707 先叠得 6 防）。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut battle = battle;
    battle.players.p1.xian_mo_strategies = vec![13_014];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.physique = 32;
    state.p1.core.physique_limit = 45;
    state.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextBengQuanPhysique, 2);
    let card = state.p1.deck.slots[0].card.clone();
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &card, 0, false);
    assert_eq!(state.p1.core.defense, 5);
    assert_eq!(state.p1.core.physique, 36);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::NextBengQuanPhysique),
        0
    );
}

#[test]
fn xing_yi_tiao_marker_survives_7116_grid_skip() {
    // BattleExecuter 跳牌链：7116 跳过第 5 格（:1951）先于 XingYiTiao 判定
    // （:1991）。将被 7116 跳走的星弈牌不得清空跳牌标记，标记须留给
    // 7116 之后的牌。
    // 锚点：pa_a66ce89f [7]（星弈•跳标记 + 7116 跳第 5 格星弈•夹
    // 后，跳过第 6 格杯弓蛇影选中第 7 格星弈•飞；旧逻辑在第 5 格清空标记
    // 误选第 6 格）。
    let mut cards = vec![basic_attack(); 8];
    cards[4] = original(4_000_010);
    cards[5] = original(4_000_059);
    cards[6] = original(4_000_038);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![7_116];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.ronghui.star_chess_jump = 1;
    state.p1.deck.queue = vec![4, 5, 6, 7, 0, 1, 2, 3];
    let drawn = state
        .p1
        .draw_next_card(0)
        .expect("draw after 7116 + star jump");
    assert_eq!(drawn.source_slot, 6);
    assert_eq!(drawn.skipped_slots, vec![4, 5]);
    assert_eq!(state.p1.ronghui.star_chess_jump, 0);
}

#[test]
fn anima_hooks_run_two_polarity_before_ling_gua_art() {
    // BattleCharacter.ModifyAnima 源序：梦•两仪生命/防御（:9741-9750）先于
    // 灵卦术（:9759-9762）。两仪治疗先落（可能触顶），再转卦象触发两仪卦象
    // 钩；反过来会把大额治疗提前触顶，吞掉天赋 64（每次生命变化 +1 防，
    // 按 AfterHpModifyEffect 实际变化量触发）的后续触发。
    // 锚点：pa_6a7a1f70 [35]（转弦合调灵气+2，两仪命3/防2，
    // 灵卦术2，天赋64，生命89/97：治疗 +6 到 95、+12 到顶 97，64 触发两次
    // 共 +2 防，总防 +14；旧顺序得 +13）、同 match round-16 [13]
    // （灵元外放同理 26 非 25）。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut battle = battle;
    battle.players.p1.talents = vec![64];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 89;
    state.p1.core.max_hp = 97;
    state.p1.core.defense = 1;
    state.p1.elements.dream_two_polarity_hp = 3;
    state.p1.elements.dream_two_polarity_defense = 2;
    state.p1.astrology.ling_gua_art = 2;
    state.gain_anima(PlayerSide::P1, 2);
    assert_eq!(state.p1.core.hp, 97);
    assert_eq!(state.p1.astrology.hexagram, 4);
    assert_eq!(state.p1.core.defense, 15);
}

#[test]
fn illusory_metal_seal_decay_follows_all_turn_end_healing() {
    // 幻•金灵印（JinLingYinPlus/577）：原版 OnTurnEnded :6804-6806 才 -1，晚于全部
    // 回合末治疗（回春曲 :6545-6546、百毒不侵 :5964 等）；最后一层在治疗结算时仍
    // 按 :9966-9970 把加生命转为锋锐。
    // 锚点：pa_3844ef5a cp[17]（t12 p1 回合末回春曲 +6 上限：原版
    // 73 血不动、锋锐 14→20、577 消退；旧顺序先衰减，+6 变真治疗到 79、锋锐不动）。
    let battle = fixture(deck(original(5_010_010)), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 20;
    state.p1.core.max_hp = 30;
    state.p1.music.rejuvenation_tune = 6;
    state.modify_mirage_ronghui_value(
        PlayerSide::P1,
        MirageRonghuiValue::MirageHealingConversionTurns,
        1,
    );
    state.apply_turn_end_phases(PlayerSide::P1);
    assert_eq!(state.p1.core.max_hp, 36);
    assert_eq!(state.p1.core.hp, 20);
    assert_eq!(state.p1.sword.sharpness, 6);
    assert_eq!(
        state.mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::MirageHealingConversionTurns
        ),
        0
    );
}

#[test]
fn dream_water_doubling_heals_before_internal_injury_tick() {
    // 梦•水灵波澜临时加倍（OnTurnStarted :4493-4504，ModifyAnima 连带静气心法治疗
    // :9729-9731）先于内伤 tick（:4273-4290 取值，:4545-4558 循环结算）：治疗先落
    // （可能触顶），再吃内伤。
    // 锚点：pa_1bbf4b84 cp[22]（t24 p2 回合开始：水势/灵气临时加倍，
    // +15 灵气 → 静气心法回 60 触顶 124，再 -30 内伤到 94；旧顺序先 -30 再回满 124）。
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 122;
    state.p1.core.max_hp = 124;
    state.p1.core.anima = 15;
    state.p1.elements.water_momentum = 9;
    state.p1.fate.quiet_mindset = 4;
    state.p1.status.internal_injury = 30;
    state.modify_dream_mirage_value(
        PlayerSide::P1,
        DreamMirageValue::TemporaryWaterDouble,
        1,
    );
    state.apply_turn_start_phases(PlayerSide::P1);
    assert_eq!(state.p1.core.anima, 30);
    assert_eq!(state.p1.elements.water_momentum, 18);
    assert_eq!(state.p1.core.hp, 94);
    assert_eq!(state.p1.status.internal_injury, 30);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TemporaryWaterDouble),
        0
    );
}

#[test]
fn dream_mystic_footwork_suppressed_during_opponent_turn_end_water() {
    // 梦•灵玄迷踪步 (10030069, MengLingXuan 647): 自身回合内前 N 次获得负面状态
    // 才镜像给对方; 对方回合开始时己方已置 MengLingXuanBuChuFa (654)=1
    // (BattleCharacter.OnTurnStarted:3923-3925), 回合末清除晚于水势伤害与状态衰减
    // (OnTurnEnded:6720-6734, 水势 IL_1bc1 在前)。
    // 锚点: hf-auto-9c1ca5855c8d-0000 pa_400a8635 p2 T3 末水势 3 →
    // p1 掉 3 血触发血光之灾外伤 1, 不应镜像 (p2 外伤 0); 旧顺序在水前清除抑制,
    // 误镜像致 p2 外伤 1, 耗掉 T4 配额, T4 减攻未镜像, 终局 11 vs 13。
    let p1dream = original(10_030_069);
    let battle = fixture(deck(p1dream), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 30;
    state.p1.core.max_hp = 30;
    state.p2.core.hp = 30;
    state.p2.core.max_hp = 30;
    state.p2.elements.water_momentum = 3;
    state.p1.status.blood_calamity = 3;
    state.modify_dream_mirage_value(
        PlayerSide::P1,
        DreamMirageValue::DreamMysticFootwork,
        2,
    );
    state.modify_dream_mirage_value(
        PlayerSide::P1,
        DreamMirageValue::DreamMysticFootworkSuppressed,
        1,
    );
    state.apply_turn_end_phases(PlayerSide::P2);
    assert_eq!(state.p1.core.hp, 27);
    assert_eq!(state.p1.status.external_injury, 1);
    assert_eq!(state.p2.status.external_injury, 0);
    assert_eq!(state.p1.status.blood_calamity, 2);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::DreamMysticFootwork),
        2
    );
    assert_eq!(
        state.dream_mirage_value(
            PlayerSide::P1,
            DreamMirageValue::DreamMysticFootworkSuppressed
        ),
        0
    );
    assert_eq!(
        state.dream_mirage_value(
            PlayerSide::P1,
            DreamMirageValue::DreamMysticFootworkTriggerCount
        ),
        0
    );
}

#[test]
fn fate_7116_isolated_overlap_prefers_7116_without_opening() {
    // BattleExecuter 外层 while(flag) + 内层 while：7116(:1951)在洞烛(:2013)之前。
    // 孤立重叠格（当前即第 5 格，非连续洞烛链）时 7116 先跳，不耗洞烛、不触发开局。
    // 连续洞烛格（grid3→grid4）仍由洞烛内层 while 批量跳过（fc621a 4→2），不重入 7116。
    // 锚点：pa_6cabbdea [7]（洞烛 4 不动、探灵开局未触发、灵气 1；
    // 旧 7116-after-fate 误耗 1 次洞烛并 +1 灵气，终局 p2 胜 vs p1 胜）。
    let mut cards = vec![basic_attack(); 8];
    cards[4] = original(11_010_009);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![7_116];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.fate.fate_cycle = 4;
    state.p1.fate.fate_cycle_slots = [4, 5];
    state.p1.deck.queue = vec![4, 5, 6, 7, 0, 1, 2, 3];
    let drawn = state
        .p1
        .draw_next_card(0)
        .expect("draw isolated fate/7116 overlap");
    assert_eq!(drawn.source_slot, 5);
    assert_eq!(drawn.skipped_slots, vec![4]);
    assert!(drawn.skipped_opening_slots.is_empty());
    assert_eq!(state.p1.fate.fate_cycle, 4);
    assert!(state.p1.fate.skip_hp_deltas.is_empty());
    assert!(state.p1.fate.skip_anima_deltas.is_empty());
}

#[test]
fn xing_yi_tiao_landing_clears_before_7116_grid_skip() {
    // BattleExecuter 外层循环：跳牌落到星弈牌后当轮清空标记(:2011-2013)，
    // 下一外层迭代 7116 再跳第 5 格；起始即第 5 格星弈时 7116 在同轮先跳，
    // 标记保留（pa_a66ce89f [7] 仍通过）。
    // 锚点：pa_45faec9d [4]（跳过 slot3 厄劫缠身落到 slot4 星弈•打，
    // 清空后 7116 跳 slot4，选中 slot5 众星拱月；旧保留标记误跳 slot5 选 slot6）。
    let mut cards = vec![basic_attack(); 8];
    cards[3] = original(11_000_018);
    cards[4] = original(4_000_029);
    cards[5] = original(4_010_032);
    cards[6] = original(4_000_107);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![7_116];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.ronghui.star_chess_jump = 1;
    state.p1.deck.queue = vec![3, 4, 5, 6, 7, 0, 1, 2];
    let drawn = state
        .p1
        .draw_next_card(0)
        .expect("draw jump landing + 7116");
    assert_eq!(drawn.source_slot, 5);
    assert_eq!(drawn.skipped_slots, vec![3, 4]);
    assert_eq!(drawn.skipped_opening_slots, vec![3]);
    assert_eq!(state.p1.ronghui.star_chess_jump, 0);
}
