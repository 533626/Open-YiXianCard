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
