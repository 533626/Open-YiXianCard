//! 2026-10 平衡热更的既有卡/机制最小契约。

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

/// Typed rows for the data-driven rotation tables: every loop iteration of
/// the migrated tests below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `level_4`).
struct LevelDamageCase {
    level: i64,
    expected_damage: i64,
}

struct IdValueCase {
    id: i64,
    value: i64,
}

struct ResonanceCase {
    round: i64,
    resonance: Option<i64>,
    expected_hp: i64,
}

struct GuqinCase {
    guqin: i64,
    expected: i64,
}

struct LastElementCase {
    last: Element,
    expected: i64,
}

struct DreamAnimaCase {
    dream: bool,
    expected_anima: i64,
}

struct SwallowCase {
    id: i64,
}

struct TierDeltaCase {
    tier_id: i64,
    expected_delta: i64,
}

#[test]
fn xian_mo_damage_precedes_beng_mei_momentum_overflow() {
    // pa_ac08891a: 7037 consumes 断崖 with 1 damage;
    // 崩灭心法 then overflows 气势 and grants one defense.
    let mut strike = original(10_000_001);
    strike.hp_cost = Some(3);
    let battle = fixture(deck(strike.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.identity.xian_mo_strategies.push(7037);
    state.p1.core.defense = 2;
    state.p1.beng.momentum_limit = 5;
    state.p1.beng.momentum = 5;
    state.p1.beng.beng_mei_mindset = 1;
    state.p1.fate.sheng_qi_ling_ren = 2;
    state.p2.core.defense = 20;
    state.p2.elements.earth_cliff_counter = 1;
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &strike, 0, false);
    assert_eq!(state.p1.core.defense, 2);
    assert_eq!(state.p2.core.defense, 17);
    assert_eq!(state.p2.elements.earth_cliff_counter, 0);
}

#[test]
fn fu_hu_zero_defense_skips_damage_despite_iron_bone() {
    // Build 26477585 changed the rule pinned here through 26449625: the
    // OnAfterExecuted ChuFaFuHu branch now guards with `def > 0`
    // (CardActionBase.cs old :4706-4710 unconditional vs new :5141 guarded;
    // diff 26449625-to-26477585). Zero-def no longer reaches 铁骨's floor-to-1.
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.defense = 0;
    state.p1.beng.triggered_beng_quan_fu_hu = 1;
    state.p2.elements.metal_iron_bone = 1;
    let before = state.p2.core.hp;
    state.apply_beng_quan_fu_hu_after_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, before);
    assert_eq!(state.p1.beng.triggered_beng_quan_fu_hu, 0);
}

#[test]
fn attack_defense_reflection_precedes_fire_blade_max_hp_loss() {
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.identity.talents.extend([67, 68]);
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p2.core.hp = 30;
    state.p2.core.max_hp = 30;
    state.apply_attack(PlayerSide::P1, 2, 0);
    assert_eq!(state.p2.core.max_hp, 29);
    assert_eq!(state.p2.core.hp, 27);
}

#[test]
fn attack_damage_receipt_excludes_reentrant_target_healing() {
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 20;
    state.p1.chance.an_xing_bian_fu = 10;
    state.p2.identity.talents.push(177);
    state.p2.status.blood_calamity = 1;
    state.apply_attack(PlayerSide::P1, 4, 0);
    assert_eq!(state.p2.core.hp, 27); // 4 damage, then 1 healing
    assert_eq!(state.p1.turn.actual_damage_carry, 4);
    assert_eq!(state.p1.core.hp, 24); // 暗星蝙蝠 reads actual damage, not net HP change
}

#[test]
fn cloud_sword_healing_follows_on_play_reflection() {
    let cloud = original(1_000_040);
    let battle = fixture(deck(cloud.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.identity.xian_mo_strategies.push(13016);
    state
        .p1
        .identity
        .xian_mo_temp_datas
        .insert("13016".into(), 0);
    state.p1.core.hp = 28;
    state.p1.sword.sword_intent = 3;
    state.p1.sword.cloud_sword_soft_heart = 3;
    state.p2.core.defense = 2;
    state.p2.elements.earth_cliff_counter = 1;
    state.apply_before_execute_effect_hooks(PlayerSide::P1, &cloud, 0, false);
    assert_eq!(state.p1.core.hp, 29);
}

#[test]
fn parrot_copying_consumable_keeps_physical_parrot() {
    let parrot = original(99_000_216);
    let battle = fixture(deck(parrot.clone()), deck(original(282)));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &parrot, 0);
    assert!(!state.p1.deck.slots[0].skipped);
    assert_eq!(state.p2.status.internal_injury, 4);
}

#[test]
fn recovery_precedes_infinite_hexagram_plate_reflection() {
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.status.recovery = 2;
    state.p1.astrology.infinite_hexagram_plate = 1;
    state.p1.formations.six_yao_formation = 3;
    state.p2.core.defense = 3;
    state.p2.elements.earth_cliff_counter = 1;
    state.apply_turn_start_phases(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 27);
}

#[test]
fn swift_shadow_suppresses_sword_intent_for_enchantment_follow_up() {
    let card = original(1_000_094);
    let battle = fixture(deck(card), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.identity.xian_mo_strategies.push(13006);
    state
        .p1
        .identity
        .xian_mo_temp_datas
        .insert("13006".into(), 0);
    state.p1.sword.sword_intent = 17;
    state.p2.core.defense = 30;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.defense, 20);
    assert_eq!(state.p1.sword.sword_intent, 18);
}

#[test]
fn dream_thunder_counts_duplicate_card_in_other_grid() {
    let thunder = original(4_040_073);
    let mut cards = deck(thunder.clone());
    cards[1] = thunder.clone();
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.level = 3; // No random extra attack.
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &thunder, 0);
    assert_eq!(state.p2.core.hp, 18); // 8 + 2 * (self + other grid)
}

#[test]
fn spirit_drain_healing_follows_internal_injury() {
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.status.internal_injury = 2;
    state.p1.chance.shi_xu_ling_shou = 1;
    state.p1.fate.quiet_mindset = 5;
    state.p2.core.anima = 1;
    state.apply_turn_start_phases(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p2.core.anima, 0);
}

const YU_FENG_LEVEL_CASES: &[LevelDamageCase] = &[
    LevelDamageCase {
        level: 4,
        expected_damage: 4,
    },
    LevelDamageCase {
        level: 5,
        expected_damage: 8,
    },
    LevelDamageCase {
        level: 6,
        expected_damage: 8,
    },
];

fn run_yu_feng_level_case(case: &LevelDamageCase) {
    // Build 25981480 Card_383.cs:106-112. Odd agility at HuaShen must
    // produce 8, rather than 9; YuanYing produces 4 from the same 9 agility.
    let flying = original(383);
    let level = case.level;
    let expected_damage = case.expected_damage;
    let battle = fixture(deck(flying.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.identity.level = level;
    state.p1.turn.agility = 1;
    let before = state.p2.core.hp;
    state.test_apply_card_effect(PlayerSide::P1, &flying, 0);
    assert_eq!(state.p1.turn.agility, 9);
    assert_eq!(state.p2.core.hp, before - expected_damage);
}

crate::data_driven_cases! {
    table: YU_FENG_LEVEL_CASES,
    run: run_yu_feng_level_case,
    suite: "replay::tests_build_2026_10_rotation::yu_feng_fei_shan_divides_agility_before_huashen_multiplier",
    valid: yu_feng_fei_shan_divides_agility_case_table_is_valid,
    cases: [
        yu_feng_fei_shan_divides_agility_before_huashen_multiplier_level_4 => 0,
        yu_feng_fei_shan_divides_agility_before_huashen_multiplier_level_5 => 1,
        yu_feng_fei_shan_divides_agility_before_huashen_multiplier_level_6 => 2,
    ]
}

const ECHO_PATTERN_GAIN_CASES: &[IdValueCase] = &[
    IdValueCase {
        id: 8_000_012,
        value: 2,
    },
    IdValueCase {
        id: 8_010_012,
        value: 5,
    },
    IdValueCase {
        id: 8_020_012,
        value: 8,
    },
];

fn run_echo_pattern_gain_case(case: &IdValueCase) {
    // Build 25981480 Card_8000012.cs:156-157 precedes cardType guard.
    let id = case.id;
    let gain = case.value;
    let echo = original(id);
    assert_eq!(other_param(&echo, 1), gain);
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 20;
    state.test_apply_card_effect(PlayerSide::P1, &echo, 1);
    assert_eq!(
        (state.p1.core.hp, state.p1.core.max_hp),
        (20 + gain, 30 + gain)
    );
}

crate::data_driven_cases! {
    table: ECHO_PATTERN_GAIN_CASES,
    run: run_echo_pattern_gain_case,
    suite: "replay::tests_build_2026_10_rotation::echo_pattern_gains_health_even_without_a_first_slot_sustain",
    valid: echo_pattern_gains_health_case_table_is_valid,
    cases: [
        echo_pattern_gains_health_even_without_a_first_slot_sustain_card_8000012 => 0,
        echo_pattern_gains_health_even_without_a_first_slot_sustain_card_8010012 => 1,
        echo_pattern_gains_health_even_without_a_first_slot_sustain_card_8020012 => 2,
    ]
}

const RESONANCE_83_CASES: &[ResonanceCase] = &[
    ResonanceCase {
        round: 10,
        resonance: Some(83),
        expected_hp: 18,
    },
    ResonanceCase {
        round: 9,
        resonance: Some(83),
        expected_hp: 16,
    },
    ResonanceCase {
        round: 10,
        resonance: None,
        expected_hp: 16,
    },
];

fn run_resonance_83_case(case: &ResonanceCase) {
    let round = case.round;
    let resonance = case.resonance;
    let expected_hp = case.expected_hp;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.source = Some(crate::fixture::FixtureSource {
        round: Some(round),
        ..Default::default()
    });
    battle.players.p1.talent_resonance_id = resonance;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 20;
    state.p1.status.internal_injury = 4;
    state.test_play_actor_turn();
    assert_eq!(
        state.p1.core.hp, expected_hp,
        "round={round}, resonance={resonance:?}"
    );
}

crate::data_driven_cases! {
    table: RESONANCE_83_CASES,
    run: run_resonance_83_case,
    suite: "replay::tests_build_2026_10_rotation::resonance_83_heals_after_internal_injury_from_round_10",
    valid: resonance_83_heals_after_internal_injury_case_table_is_valid,
    cases: [
        resonance_83_heals_after_internal_injury_from_round_10_round_10_res_83 => 0,
        resonance_83_heals_after_internal_injury_from_round_10_round_9_res_83 => 1,
        resonance_83_heals_after_internal_injury_from_round_10_round_10_res_none => 2,
    ]
}

#[test]
fn xian_mo_13018_reduces_wood_healing_before_the_card_runs() {
    let dark_fragrance = original(7_010_061);
    let mut battle = fixture(deck(dark_fragrance), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![13_018];
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("13018".into(), 0);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 20;
    state.p1.core.attack_bonus = 5;
    state.p1.core.anima = 3;
    state.p1.elements.wood_healing_formation = 1;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 26, "3 healing after XianMo + 3 from card");
    assert_eq!(state.p1.core.attack_bonus, 3);
}

#[test]
fn card_175_doubles_health_after_turn_start_recovery() {
    // BattleCharacter.OnTurnStarted IL_139a heals HuiFu, then IL_144c applies
    // Card 175's NvWaShi current-HP doubling. 20 HP + 2 recovery must become 44.
    let five_color_stone = original(175);
    let battle = fixture(deck(five_color_stone.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &five_color_stone, 0);
    state.p1.core.hp = 20;
    state.p1.core.max_hp = 60;
    state.p1.status.recovery = 2;

    state.apply_turn_start_phases(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 44);
    assert_eq!(
        state.mirage_ronghui_value(
            PlayerSide::P1,
            super::cards_mirage_ronghui::MirageRonghuiValue::DoubleHpAtTurnStart,
        ),
        0
    );
}

const PHANTOM_PARROT_GUQIN_CASES: &[GuqinCase] = &[
    GuqinCase {
        guqin: 1,
        expected: 1,
    },
    GuqinCase {
        guqin: 0,
        expected: 0,
    },
];

fn run_phantom_parrot_guqin_case(case: &GuqinCase) {
    // Card_0.cs:78 writes cardConfig.actionAgain = HasBuff(XiaoYaoGuQin) on the
    // copied cardItem; Card_99000216.cs IL_03a8 then grants ExActionAgain+1
    // (HF pa_91eaa4cd).
    let parrot = original(99_000_216);
    let guqin = case.guqin;
    let expected = case.expected;
    let battle = fixture(deck(parrot.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.music.xiaoyao_guqin = guqin;
    state.test_apply_card_effect(PlayerSide::P1, &parrot, 0);
    assert_eq!(state.p1.turn.extra_actions, expected, "guqin={guqin}");
}

crate::data_driven_cases! {
    table: PHANTOM_PARROT_GUQIN_CASES,
    run: run_phantom_parrot_guqin_case,
    suite: "replay::tests_build_2026_10_rotation::phantom_parrot_copied_basic_attack_keeps_xiaoyao_guqin_action_again",
    valid: phantom_parrot_copied_basic_attack_case_table_is_valid,
    cases: [
        phantom_parrot_copied_basic_attack_keeps_xiaoyao_guqin_action_again_guqin_1 => 0,
        phantom_parrot_copied_basic_attack_keeps_xiaoyao_guqin_action_again_guqin_0 => 1,
    ]
}

#[test]
fn earth_fiend_defense_lands_before_reflect_mindset() {
    // BattleCharacter.ApplyDamage: TianJiDiSha ModifyDef (:11180) precedes
    // FanZhenXinFa (:11295), so the reflected 3 hits that defense first
    // (HF pa_aca1b4aa).
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 50;
    state.p1.ronghui.earth_fiend_defense = 1;
    state.p2.fate.reflect_mindset = 2;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!((state.p1.core.hp, state.p1.core.defense), (50, 1));
}

const TIAN_SUI_LAST_CASES: &[LastElementCase] = &[
    LastElementCase {
        last: Element::Wood,
        expected: 1,
    },
    LastElementCase {
        last: Element::Fire,
        expected: 0,
    },
];

fn run_tian_sui_last_case(case: &LastElementCase) {
    // CardActionBase.OnBeforeExecuted IL_14cc has no isTempCard guard: Card_7000111
    // temporarily executing 土灵阵 after a 木灵 card earns ExActionAgain
    // (HF pa_92ebc763).
    let earth_formation = original(7_000_023);
    let last = case.last;
    let expected = case.expected;
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.ronghui.tian_sui_ni_ke_jue = 1;
    state.p1.elements.last_element = Some(last);
    state.apply_temporary_card_effect(PlayerSide::P1, &earth_formation, 1);
    assert_eq!(state.p1.turn.extra_actions, expected, "last={last:?}");
}

crate::data_driven_cases! {
    table: TIAN_SUI_LAST_CASES,
    run: run_tian_sui_last_case,
    suite: "replay::tests_build_2026_10_rotation::tian_sui_ni_ke_jue_grants_action_again_to_temporary_element_cards",
    valid: tian_sui_ni_ke_jue_grants_action_again_case_table_is_valid,
    cases: [
        tian_sui_ni_ke_jue_grants_action_again_to_temporary_element_cards_last_wood => 0,
        tian_sui_ni_ke_jue_grants_action_again_to_temporary_element_cards_last_fire => 1,
    ]
}

#[test]
fn dream_ming_ying_bonus_skips_heaven_cycle_follow_up_attack() {
    // BattleCharacter.CalculateAttack:11740 requires !AfterCardAciton, so the
    // 周天剑阵 follow-up after the card does not read 梦•冥影身法's debuff bonus
    // (HF pa_02859c8e).
    let mut cards = vec![basic_attack(), original(10_030_077)];
    cards.resize(8, basic_attack());
    let battle = fixture(cards, deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.deck.active_slot_count = 8;
    state.p1.status.internal_injury = 4;
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 5;
    let hp_before = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(hp_before - state.p2.core.hp, (3 + 2) + 5);
}

const TALENT_106_DREAM_CASES: &[DreamAnimaCase] = &[
    DreamAnimaCase {
        dream: true,
        expected_anima: 1,
    },
    DreamAnimaCase {
        dream: false,
        expected_anima: 0,
    },
];

fn run_talent_106_dream_case(case: &DreamAnimaCase) {
    // BattleCharacter.OnBattleStarted marks 梦•斗转星移's neighbours before the
    // talent loop; Talent 106's AddXingWei(6) then hits an existing 星位 and
    // gains 1 anima (星位聚灵, :10656-10659; HF pa_57eb5dec).
    let dream = case.dream;
    let expected_anima = case.expected_anima;
    let mut cards = vec![basic_attack(); 8];
    if dream {
        cards[7] = original(4_040_079);
    }
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.talents = vec![106];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.anima, expected_anima, "dream={dream}");
    assert!(state.p1.astrology.star_slots.contains(&6));
}

crate::data_driven_cases! {
    table: TALENT_106_DREAM_CASES,
    run: run_talent_106_dream_case,
    suite: "replay::tests_build_2026_10_rotation::talent_106_gathers_anima_when_dream_star_shift_already_marked_grid_6",
    valid: talent_106_gathers_anima_case_table_is_valid,
    cases: [
        talent_106_gathers_anima_when_dream_star_shift_already_marked_grid_6_dream_true => 0,
        talent_106_gathers_anima_when_dream_star_shift_already_marked_grid_6_dream_false => 1,
    ]
}

#[test]
fn free_shadow_fist_counts_hidden_basic_attack_286() {
    // Card_339 counts IsPuTongGongJi (base 0 or 286; BattleCharacter.cs:12928)
    // (HF pa_c97d6c6b).
    let fist = original(339);
    let mut cards = vec![fist.clone(), original(286)];
    cards.resize(8, original(4_000_038));
    let battle = fixture(cards, deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    let hp_before = state.p2.core.hp;
    state.test_apply_card_effect(PlayerSide::P1, &fist, 0);
    let hits = fist.attack_count.unwrap_or(1) + 1;
    assert_eq!(
        hp_before - state.p2.core.hp,
        hits * fist.attack.unwrap_or(0)
    );
}

#[test]
fn dismantle_move_loses_one_stack_per_turn_start() {
    // BattleCharacter.cs:3958-3961 ModifyBuffValue(ChaiZhao, -1) at turn start
    // (HF pa_f67bc450: two stacks survive one turn start).
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.fate.dismantle_move = 2;
    state.apply_turn_start_phases(PlayerSide::P1);
    assert_eq!(state.p1.fate.dismantle_move, 1);
    state.apply_turn_start_phases(PlayerSide::P1);
    assert_eq!(state.p1.fate.dismantle_move, 0);
}

#[test]
fn xian_mo_5010_skips_last_grid_five_element_field_before_field_rule() {
    // BattleExecuter.cs:1960-1975: the 5010 loop precedes the 202 loop, so a
    // last-grid 五行灵田 is skipped and is not marked used.
    let mut cards = vec![basic_attack(); 5];
    cards.extend([original(4_000_038), original(4_000_038), original(10_202)]);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![5_010];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.deck.queue = vec![5, 6, 7, 0, 1, 2, 3, 4];
    let drawn = state.p1.draw_next_card(0).expect("draw after 5010 skips");
    assert_eq!(drawn.source_slot, 0);
    assert_eq!(drawn.skipped_slots, vec![5, 6, 7]);
    assert_eq!(state.p1.fate.skip_hp_deltas, Vec::<i64>::new());
    assert!(!state.p1.deck.slots[7].used);
}

#[test]
fn wan_xuan_counts_non_status_params_like_original_modify_buff_value() {
    // Card_82 calls ModifyBuffValue((BuffType)param, -1) for every non -1 param
    // and still counts it; an absent buff with a negative delta returns early
    // (BattleCharacter.ModifyBuffValue:8847; HF pa_834a2de3).
    let palm = original(82);
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.decision_tape = vec![7_000_029, 3];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.physique = 40 - palm.physique.unwrap_or(0);
    state.p1.astrology.star_power = 2;
    let bonus_before = state.p1.core.attack_bonus;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.attack_bonus - bonus_before, 2);
    assert_eq!(state.p1.astrology.star_power, 1);
    assert!(state.decision_tape.is_empty());
}

const XU_MI_PATTERN_CASES: &[IdValueCase] = &[
    IdValueCase {
        id: 8_000_014,
        value: 2,
    },
    IdValueCase {
        id: 8_010_014,
        value: 6,
    },
    IdValueCase {
        id: 8_020_014,
        value: 10,
    },
];

fn run_xu_mi_pattern_case(case: &IdValueCase) {
    // Build 25981480 Card_8000014.cs:62-64.
    let id = case.id;
    let defense = case.value;
    let pattern = original(id);
    assert_eq!(pattern.defense, Some(defense));
    let battle = fixture(deck(pattern.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &pattern, 0);
    assert_eq!(state.p1.core.defense, defense);
}

crate::data_driven_cases! {
    table: XU_MI_PATTERN_CASES,
    run: run_xu_mi_pattern_case,
    suite: "replay::tests_build_2026_10_rotation::xu_mi_pattern_gains_defense_before_skipping",
    valid: xu_mi_pattern_gains_defense_case_table_is_valid,
    cases: [
        xu_mi_pattern_gains_defense_before_skipping_card_8000014 => 0,
        xu_mi_pattern_gains_defense_before_skipping_card_8010014 => 1,
        xu_mi_pattern_gains_defense_before_skipping_card_8020014 => 2,
    ]
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
    assert_eq!(
        (state.p1.beng.quan_stance, state.p1.beng.gun_stance),
        (0, 0)
    );
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
    assert_eq!(
        state.spirit_formation_echo_card(PlayerSide::P1, &dream).id,
        7_010_074
    );
    let upgraded = original(10_208);
    assert_eq!(
        state
            .spirit_formation_echo_card(PlayerSide::P1, &upgraded)
            .id,
        208
    );
}

const FEI_YAN_SWALLOW_CASES: &[SwallowCase] = &[
    SwallowCase { id: 1_000_101 },
    SwallowCase { id: 1_010_101 },
    SwallowCase { id: 1_020_101 },
];

fn run_fei_yan_swallow_case(case: &SwallowCase) {
    // HF pa_57082518 cp0: p1 anima=1 before its first card.
    // BattleCharacter.TriggerOpening case 1000101 and Card_1000101.OnExecuted.
    let id = case.id;
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
    assert_eq!(
        state.p2.core.hp,
        before - swallow.attack.unwrap() * swallow.attack_count.unwrap()
    );
}

crate::data_driven_cases! {
    table: FEI_YAN_SWALLOW_CASES,
    run: run_fei_yan_swallow_case,
    suite: "replay::tests_build_2026_10_rotation::fei_yan_ling_jian_opening_only_supplies_missing_anima",
    valid: fei_yan_ling_jian_opening_only_case_table_is_valid,
    cases: [
        fei_yan_ling_jian_opening_only_supplies_missing_anima_card_1000101 => 0,
        fei_yan_ling_jian_opening_only_supplies_missing_anima_card_1010101 => 1,
        fei_yan_ling_jian_opening_only_supplies_missing_anima_card_1020101 => 2,
    ]
}

#[test]
fn ling_ping_activates_original_named_element_of_unimplemented_card() {
    // HF pa_90ce6396 cp0: 灵瓶 contains 木灵•散枝叶;
    // original ActiveWuXingInName activates wood, triggering 200/202.
    assert_eq!(
        super::support::element_from_card(&original(7_000_112)),
        Some(Element::Wood)
    );
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![199, 200, 202];
    battle
        .players
        .p1
        .talent_card_params
        .insert("199".into(), vec![7_000_112]);
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.max_hp, 32);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.anima, 1);
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Wood));
    // Only metadata is needed for a card stored in the bottle.
}

#[test]
fn baicao_activation_runs_after_first_use_xian_mo_healing() {
    // CardActionBase.OnBeforeExecuted runs XianMo.OnPlayCard before its talent
    // loop. A first-use 13002 heal is capped at max HP 78 before talent 200
    // activates wood and raises max HP to 80 (HF pa_64e3e620).
    let calamity = original(11_010_023);
    let mut battle = fixture(deck(calamity), deck(basic_attack()));
    battle.players.p1.talents = vec![200];
    battle.players.p1.xian_mo_strategies = vec![13_002];
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("13002".into(), 0);
    battle.decision_tape = vec![-1; 4];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 74;
    state.p1.core.max_hp = 78;

    state.apply_before_execute_effect_hooks(PlayerSide::P1, &original(11_010_023), 0, false);

    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (78, 80));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Wood));
}

#[test]
fn fei_yan_ling_jian_receives_hundred_bird_spirit_sword_cost_reduction() {
    // HF pa_83159049 cp13: original spends zero anima.
    let swallow = original(1_000_101);
    let battle = fixture(deck(swallow.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.hundred_bird_spirit_sword_art = 1;
    assert_eq!(
        super::support::effective_anima_cost(&swallow, &state.p1, Some(0)),
        0
    );
}

#[test]
fn xian_mo_sword_bodies_preserve_cloud_count_and_defense_sampling() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
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
    // Build 25981480: base 9 + fire 2*2 + earth 9/3 + metal 4/2 = 18.
    assert_eq!(
        (
            state.p2.core.hp,
            state.p2.core.max_hp,
            state.p2.core.defense
        ),
        (82, 82, 99)
    );
}

#[test]
fn ju_ling_zhen_heals_before_internal_injury_at_the_hp_cap() {
    // HF pa_b0e272bd cp7: original p2 def4 versus old Rust3.
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(dragon.clone()), deck(basic_attack())));
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(card.clone()), deck(basic_attack())));
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
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
    // HF pa_e7b91805 cp12: 连环剑阵 must copy 千刃剑阵,
    // not skip it and copy the earlier 御空剑阵.
    let battle = fixture(deck(basic_attack()), deck(basic_attack()));
    let state = ReplayState::test_from_fixture(&battle);
    assert!(super::support::is_sword_formation_card(
        &state.p1,
        &original(1_000_108)
    ));
}

#[test]
fn beng_quan_fei_zhou_inherits_flaw_on_effective_beng_quan() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
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
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p2.astrology.star_erosion = 2;
    state.add_actor_negative_status(PlayerSide::P2, 100, 0);
    assert_eq!(state.p2.status.internal_injury, 0);
    assert_eq!(state.p2.astrology.star_erosion, 2);

    state.add_actor_negative_status(PlayerSide::P2, 100, 1);
    assert_eq!(state.p2.status.internal_injury, 3);
    assert_eq!(state.p2.astrology.star_erosion, 0);
}

#[test]
fn mirage_flow_activates_both_wrapped_neighbors_on_generation() {
    // Card_295.cs：相邻两名任一方向相生 → ActiveWuXingInName(prev) +
    // ActiveWuXingInName(next) + 再次行动；相邻按 unlockGrids 回绕。
    // HF pa_5be234ec cp2：0 槽 295 相邻 7 槽火（回绕）/1 槽木，
    // 木生火，原版火木双激活 + 防 +4（旧引擎只激活 next 的木）。
    let flow = original(295);
    let wood = card(7_010_061, 7_000_061, "木灵•暗香");
    let fire = card(7_000_113, 7_000_113, "火灵•燃烬灭");
    let filler = basic_attack();
    let mut battle = fixture(
        vec![
            flow.clone(),
            wood,
            filler.clone(),
            filler.clone(),
            filler.clone(),
            filler.clone(),
            filler.clone(),
            fire,
        ],
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &flow, 0);
    assert_eq!(state.p1.elements.activated_fire, 1);
    assert_eq!(state.p1.elements.activated_wood, 1);
    assert_eq!(state.p1.turn.extra_actions, 1);
    assert_eq!(
        state.p1.core.defense,
        2 * other_param(&flow, 0),
        "已激活数 2 × otherParams[0]"
    );

    // 无相生对照：相邻皆为普通攻击，不激活、不再动、不加防。
    let mut battle = fixture(deck(flow.clone()), deck(basic_attack()));
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &flow, 0);
    assert_eq!(state.p1.elements.activated_fire, 0);
    assert_eq!(state.p1.elements.activated_wood, 0);
    assert_eq!(state.p1.turn.extra_actions, 0);
    assert_eq!(state.p1.core.defense, 0);
}

#[test]
fn dark_owl_sword_defense_cap_is_lifted_for_rarity_two() {
    // Card_260.cs：防御 += 灵气×otherParams[0]；rarity=2（如 20260）不钳制，
    // 否则钳制 otherParams[2]。HF pa_96c867da cp12：20260 在
    // 8 灵气时原版 +16 防，旧引擎按缺省 otherParams[2]=0 钳成 +0。
    let upgraded = original(20260);
    assert_eq!(other_param(&upgraded, 0), 2);
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(upgraded.clone()), deck(basic_attack())));
    state.p1.core.anima = 8;
    state.test_apply_card_effect(PlayerSide::P1, &upgraded, 0);
    assert_eq!(state.p1.core.defense, 16);

    // 基卡 260（rarity 0）：8×2=16 钳制到 otherParams[2]=6。
    let base = original(260);
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(base.clone()), deck(basic_attack())));
    state.p1.core.anima = 8;
    state.test_apply_card_effect(PlayerSide::P1, &base, 0);
    assert_eq!(state.p1.core.defense, 6);

    // 0 灵气：原版不调用 ModifyDef。
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(base.clone()), deck(basic_attack())));
    state.test_apply_card_effect(PlayerSide::P1, &base, 0);
    assert_eq!(state.p1.core.defense, 0);
}

#[test]
fn first_use_upgrade_runs_before_paint_finishing_touch() {
    // CardActionBase.cs Execute：13003 检查（IL_1816）在前，画龙检查（IL_1a01）
    // 在后。HF pa_86273610：t2 时 13003 先把 0 槽 4010027 升到
    // 4020027（noUpgrade），画龙检查失效、画龙层数保留；旧顺序误耗画龙，
    // t4 打不出 4010033。
    let calm = original(4010027);
    let moon = original(4000033);
    let filler = basic_attack();
    let mut battle = fixture(
        vec![
            calm,
            moon,
            filler.clone(),
            filler.clone(),
            filler.clone(),
            filler.clone(),
            filler.clone(),
            filler,
        ],
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 8;
    battle.players.p1.xian_mo_strategies = vec![13003];
    battle.players.p2.xian_mo_strategies = vec![1007];
    let mut state = ReplayState::test_from_fixture(&battle);
    // 开局双方画龙 =1（p2 的 1007 授予；缺 tempData 视为 0 即开启）。
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    assert_eq!(state.p2.fate.paint_finishing_touch, 1);
    // t2：0 槽升阶到 4020027，画龙保留。
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 4020027);
    assert_eq!(state.p1.fate.paint_finishing_touch, 1);
    // t4：1 槽非伏魔格，13003 跳过，画龙消耗并升阶到 4010033。
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 1).id, 4010033);
    assert_eq!(state.p1.fate.paint_finishing_touch, 0);
}

#[test]
fn ling_xuan_bu_xi_grants_configured_agility_and_sustain_stack() {
    // Card_416.cs OnExecuted: anima, ShenFa otherParams[1], LingXuanBuXi otherParams[0].
    // HF pa_a32d3278 cp3: original p1 ShenFa=4 while the engine had 0.
    let sustain = original(416);
    assert_eq!(sustain.anima, Some(2));
    assert_eq!((other_param(&sustain, 0), other_param(&sustain, 1)), (2, 4));
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(sustain.clone()), deck(basic_attack())));
    state.test_apply_card_effect(PlayerSide::P1, &sustain, 0);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.turn.agility, 4);
    assert_eq!(state.p1.ronghui.ling_xuan_bu_xi, 2);
}

const LING_XUAN_APPEND_CASES: &[TierDeltaCase] = &[
    TierDeltaCase {
        tier_id: 5010015,
        expected_delta: 6,
    },
    TierDeltaCase {
        tier_id: 5020015,
        expected_delta: 3,
    },
];

fn run_ling_xuan_append_case(case: &TierDeltaCase) {
    // BattleCharacter.CalculateAttack:11903-11910（152/7101）：当前档位
    // desc 含「灵气」或 anima<0 才 +3 攻。天音困仙曲 5010015 的 anima=-1
    // 可吃，5020015 无 anima 且 desc 无灵气不可吃；经 base_id 归并会让
    // 5020015 的灵玄不息追击误加 3（HF pa_ad0cfab1 cp2）。
    let tier_id = case.tier_id;
    let expected_delta = case.expected_delta;
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![7101];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_configure_p1(|p| p.ronghui.ling_xuan_bu_xi = 3);
    let hp_before = state.p2.core.hp;
    state.test_apply_card_effect(PlayerSide::P1, &original(tier_id), 0);
    assert_eq!(
        hp_before - state.p2.core.hp,
        expected_delta,
        "tier {tier_id}"
    );
}

crate::data_driven_cases! {
    table: LING_XUAN_APPEND_CASES,
    run: run_ling_xuan_append_case,
    suite: "replay::tests_build_2026_10_rotation::ling_xuan_append_anima_bonus_uses_exact_tier_config",
    valid: ling_xuan_append_anima_bonus_case_table_is_valid,
    cases: [
        ling_xuan_append_anima_bonus_uses_exact_tier_config_tier_5010015 => 0,
        ling_xuan_append_anima_bonus_uses_exact_tier_config_tier_5020015 => 1,
    ]
}

#[test]
fn talent_57_series_grants_defense_per_talent_call() {
    // CardActionBase.cs:2875 case 57: per 57-series talent with gridNumber==0 and
    // not temp: ModifyDef(otherParams[0]) + ShuiYueJianZhen+1. ModifyDef hooks
    // (BattleCharacter.cs:10349: 7133 +1 per call) settle per call, so talents
    // 10057+20057 with effective 7133 give (3+1)+(6+1)=11, not 9+1=10.
    // Oracle anchor r3check-7 b83d/round-14 cp0: original p1.def=11, engine had 10.
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.source = Some(crate::fixture::FixtureSource {
        round: Some(14),
        ..Default::default()
    });
    battle.players.p1.talents = vec![10_057, 20_057];
    battle.players.p1.xian_mo_strategies = vec![7133];
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("7133".into(), 0);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.defense, 11);
    assert_eq!(state.p1.sword.water_month_sword_formation, 2);
}

#[test]
fn xianshou_10060_life_applies_both_hands_unconditionally() {
    // XianMoStrategyFunctions.cs:248 writes 10060 (XianShouHuoTongXiuJiaShengMing)
    // unconditionally per character: no first-hand gate. The earlier first-hand-only
    // gate matched some server reports but contradicted the client; a same-battle
    // player video (pa_d40b4116 = pa_d7b90378, second-hand p2
    // 10060=10) shows the client applying it (p2 survives at 7 and counter-kills).
    // Server/client settlement divergences keep the server expected (see receipt).
    let plain =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.first_player_side = PlayerSide::P1;
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10060".into(), 7);
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10060".into(), 10);
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.max_hp, plain.p1.core.max_hp + 7);
    assert_eq!(state.p1.core.hp, plain.p1.core.hp + 7);
    assert_eq!(state.p2.core.max_hp, plain.p2.core.max_hp + 10);
    assert_eq!(state.p2.core.hp, plain.p2.core.hp + 10);
}

#[test]
fn xianshou_10060_taptap_freeze_matches_client_video() {
    // 冻结回归：pa_d7b90378（HF 34096731_p3 = pa_d40b4116
    // 同场）。服务器战报判 p1 胜（Δ=11），玩家录像显示客户端 p2 胜：
    // p1=-76、p2=7、14 回合。引擎必须复现客户端行为；expected 保留服务
    // 器原值，此 mismatch 是有录像支持的原版客户端/服务器分歧。
    // 回执：logs/practice/hf-screen-20261027-r3/diag-10060-taptap-receipt.json。
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data-driven")
        .join("pa_3994735b.json");
    let fixture = crate::fixture::load_fixture_file(&path)
        .unwrap_or_else(|error| panic!("taptap freeze fixture loads: {error}"));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert_eq!(summary.winner_side, PlayerSide::P2);
    assert_eq!(summary.actor_turn_count, 14);
    assert_eq!(summary.hp_delta_p1_minus_p2, -83);
    assert_eq!(state.test_final_hp(), (-76, 7));
    assert!(
        !summary.matches_fixture(&fixture),
        "server expected (p1 win, delta 11) must keep diverging from client result"
    );
}

#[test]
fn replica_322_chengxin_readapts_to_executing_owner() {
    // CardItem.RefreshSpecialCard (CardItem.cs:1666) reloads base-19 via
    // Card_19.UpdateCardInfo for the executing owner after transforms. A
    // 322-replica cloned from the opponent must NOT keep the opponent's
    // 19-adaptation. Oracle pa_44f2d4ca 2104e5fc/round-13 t1:
    // p2 (talents 92+20093, t92=12) replica of p1's 19 (no 20093) must be
    // 17攻+9防 (7+12-2, stable edge), not p1's 19攻+0防.
    let mut battle = fixture(deck(original(19)), deck(original(322)));
    battle.first_player_side = PlayerSide::P2;
    battle.players.p1.talents = vec![92];
    battle.players.p1.talent_temp_datas.insert("92".into(), 12);
    battle.players.p2.talents = vec![92, 20_093];
    battle.players.p2.talent_temp_datas.insert("92".into(), 12);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P2);
    // Re-adapted to p2: attack 7+12-2=17, defense 9 (stable edge 20093).
    assert_eq!(state.p2.core.defense, 9);
    assert_eq!(state.p1.core.hp, 30 - 17);
}

#[test]
fn fumo_13017_counts_debuffs_before_xing_yi_lian_converts() {
    // CardActionBase.OnBeforeExecuted：XianMo OnPlayCard（:3300，先
    // 13017 加 1 内伤再数负面加防）先于星弈连（:3676-3687，星位
    // +1 星力经 23069 转内伤）。对方 6 层内伤时，13017 数到 7
    // 加 7 防，之后星弈连再 +1 内伤到 8；反过来会数到 8。
    // 锚点：pa_4e9c2355 t9（R4 p1.def 7/8 首差）。
    let tiger = original(4_000_039);
    let mut battle = fixture(deck(tiger), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies = vec![13_017, 23_069];
    battle
        .players
        .p1
        .xian_mo_temp_datas
        .insert("13017".into(), 0);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.fate.xing_yi_lian = 1;
    state.p1.astrology.star_slots.push(0);
    state.p2.status.internal_injury = 6;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p2.status.internal_injury, 8);
}

#[test]
fn chengxin_30096_adds_attack_unconditionally() {
    // Card_19.UpdateCardInfo：30096（无极）恒 +1 攻，不以前置
    // 92/20093 为条件。7 攻底、30094（-2）、30096（+1）、无 92
    // 时应为 6 攻；旧适配只在锻剑/稳固分支加无极 bonus。
    // 锚点：pa_b9546f0b t1（R4 19 攻 5/6 首差）。
    let sword = original(19);
    let battle = fixture(deck(sword.clone()), deck(basic_attack()));
    let adapted = super::support::adapt_cheng_xin_card(sword, &[18, 66, 30_094, 7, 30_096], 0);
    assert_eq!(adapted.attack, Some(6));
    let _ = battle;
}

#[test]
fn xianmo_1003_samples_hp_after_talent_loop_drain() {
    // 原版单 actor 开局：天赋循环 → FS → XM。199 玉瓶（case 199 →
    // ActiveWuXingInName）的五行激活经 talent 138 先扣对手 3 血/上限，
    // XM-1003 采样看到的是扣后 HP。
    // 锚点：pa_722e26ca pa_29e61b8c
    // （p1 80-3=77 → 77*15/100=11，原版 88/88；旧顺序提前采样得 12 → 89/89）。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.base_max_hp = 80;
    battle.players.p2.talents = vec![199, 138];
    battle
        .players
        .p2
        .talent_card_params
        .insert("199".to_string(), vec![7_000_018]);
    battle.players.p2.xian_mo_strategies = vec![1003];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (88, 88));
}

#[test]
fn xianmo_1003_samples_before_later_7098_hp_and_max_hp_loss() {
    // XianMoStrategyFunctions.OnBattleStart 中 1003 (IL_02ff) 先于 7098
    // (IL_114e)：双方先从 100 HP 得到 +15，再由 p1 的 7098 扣 p2 4 HP/maxHP。
    // 延迟到所有仙魔策略之后才采样会错误地产生 p2 110/110。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.base_max_hp = 100;
    battle.players.p2.base_max_hp = 100;
    battle.players.p1.xian_mo_strategies = vec![1003, 7098];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (115, 115));
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (111, 111));
}

#[test]
fn battle_start_183_7042_anima_guarded_by_trigger_marker() {
    // 原版 ModifyTiPo:10516-10525：163/7042 缠心聚灵以 !HasBuff(YiChuFa)
    // 守卫，整场首次体魄增加才 +1 灵气。30013 开局 +3 灵气若先经 267
    // 风灵锻体和 183 都会增加体魄，但标记只允许触发一次。
    // 锚点：pa_602015b1（同 build 7 例 t1 灵气 o=4 r=5）。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![183, 267, 30013];
    battle.players.p1.xian_mo_strategies = vec![7042];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.anima, 4);
}

#[test]
fn opening_physique_follows_actor_and_talent_order() {
    // pa_4df18826: first actor's 火梭兰 clamps the
    // opponent's HP before the opponent's talent 183 raises max HP.
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.first_player_side = PlayerSide::P1;
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10017".into(), 4);
    battle.players.p2.base_max_hp = 45;
    battle.players.p2.extra_max_hp = Some(8);
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10023".into(), 3);
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10024".into(), 24);
    battle.players.p2.talents = vec![183];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!((state.p2.core.hp, state.p2.core.max_hp), (52, 53));

    // pa_d0cf13cd: 183 before 筋骨健壮 crosses
    // the 84/7 boundary. The reversed talent order samples 83/7.
    let mut ordered = fixture(deck(basic_attack()), deck(basic_attack()));
    ordered.players.p2.base_max_hp = 110;
    ordered
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10023".into(), 83);
    ordered
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10024".into(), 93);
    ordered.players.p2.talents = vec![183, 20_176];
    let first = ReplayState::test_from_fixture(&ordered);
    ordered.players.p2.talents = vec![20_176, 183];
    let second = ReplayState::test_from_fixture(&ordered);
    assert_eq!((first.p2.core.hp, second.p2.core.hp), (122, 121));
}

#[test]
fn cloud_sword_starry_sky_grants_base_def_before_lianyun_def() {
    // Card_141.cs:115（cardConfig.def）先于 :99-100（连云 anima×op0）。
    // 顺序决定天机•逆施两次反射的先后：4 防→2 伤先（雪羽清风 64 +1 防），
    // 3 防→1 伤后（被该 1 防全吸收，不再触发 64）。反序则终局多 1 防。
    // 锚点：pa_b90a1351 卡#4（p1 hp-2、防 0）。
    let mut battle = fixture(deck(basic_attack()), deck(original(141)));
    battle.players.p1.talents = vec![64];
    battle.players.p2.talents = vec![14];
    let mut state = ReplayState::test_from_fixture(&battle);
    let p1_hp = state.p1.core.hp;
    state.p2.core.anima = 3;
    state.p2.fate.heavenly_secret_reverse = 1;
    state.p2.deck.queue = vec![0];
    state.current_actor = PlayerSide::P2;
    let _ = state.test_execute_one_card(PlayerSide::P2);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.core.hp, p1_hp - 2);
    assert_eq!(state.p1.core.defense, 0);
}

#[test]
fn ignore_guard_attack_keeps_target_guard_for_wound_gate() {
    // 原版 WuShiHuTi（171 首山定岳剑 Card_171.cs:46）只旁路护体格挡
    // （BattleCharacter.cs:9961 检查攻击者标），目标护体保留且击伤/锋锐
    // 门（:11075）照常看到护体：有护体时锋锐不触发、不消耗。
    // 锚点：pa_8a46963b（护体 3 保留、锋锐 6 不消耗、120 平伤）。
    let battle = fixture(deck(original(171)), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    let p2_hp = state.p2.core.hp;
    state.p1.core.anima = 12;
    state.p1.sword.sharpness = 6;
    state.p2.core.guard = 3;
    state.p1.deck.queue = vec![0];
    state.current_actor = PlayerSide::P1;
    let _ = state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, p2_hp - 120);
    assert_eq!(state.p2.core.guard, 3);
    assert_eq!(state.p1.sword.sharpness, 6);
}

#[test]
fn luo_sha_pu_attacks_before_granting_anima() {
    // Card_10000008.cs:63 攻击 → :80 加灵 → :93 自身外伤。攻击采样加前灵气：
    // 气沉（灵气×10% 攻）下，灵气 1 → factor 110；若先加灵再攻则采 2 →
    // factor 120，多伤。锚点：pa_8a0e52f4 卡#16。
    let battle = fixture(deck(original(10_000_008)), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    let p2_hp = state.p2.core.hp;
    state.p1.core.anima = 1;
    state.modify_mirage_ronghui_value(
        PlayerSide::P1,
        super::cards_mirage_ronghui::MirageRonghuiValue::MirageAnimaAttackCards,
        1,
    );
    state.p1.deck.queue = vec![0];
    state.current_actor = PlayerSide::P1;
    let _ = state.test_execute_one_card(PlayerSide::P1);
    // 8 攻 ×110% = 8（取整），无防：p2 -8；灵气 1+1=2。
    // 若先加灵再攻则采 2 → ×120% = 9，多 1 伤。
    assert_eq!(state.p2.core.hp, p2_hp - 8);
    assert_eq!(state.p1.core.anima, 2);
}

#[test]
fn xianmo_7116_yields_fate_cycle_on_grid5_overlap() {
    // 反编译三版 BattleExecuter 都是 7116（:1951）在洞烛机先（:2013）之前，
    // 但 live oracle 行为相反：洞烛格含第 5 格（gridNumber 4）时，洞烛先跳
    // 且触发开局，7116 不吞该格。pa_f7c24edd 卡#7（t5）：
    // 洞烛 4→2（slot3 探灵 + slot4 血光之灾双跳），血光开局内伤 +1 经蛇影
    // 放大 +1 = +2（3→5）；旧顺序 7116 静默吞 slot4，fate 只 4→3，内伤 +0，
    // 导致 #9 闪身侧袭内伤结算错 2 血。同模式另见同 match round-15 #8、
    // pa_57ee18a6 #6/#15、pa_6ba96fbf #10、
    // pa_3a055ce7 #9（含 blocked-1 的解堵）。
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data-driven")
        .join("pa_13f7c765.json");
    let fixture = crate::fixture::load_fixture_file(&path)
        .unwrap_or_else(|error| panic!("7116 overlap fixture loads: {error}"));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert_eq!(summary.winner_side, PlayerSide::P2);
    assert_eq!(summary.actor_turn_count, 12);
    assert_eq!(summary.hp_delta_p1_minus_p2, -25);
    assert_eq!(state.test_final_hp(), (-3, 22));
    assert!(
        summary.matches_fixture(&fixture),
        "7116/fate-cycle overlap must replay exact"
    );
}

#[test]
fn turn_end_formation_order_bloom_before_maze() {
    // OnTurnEnded 阵法三相位的源码顺序是生机绽放（states 21-23）→
    // 不动金刚阵（state 28）→ 万花迷魂阵（state 34），反编译行号误导了旧顺序。
    // pa_ee3e7add（及镜像 round-16）卡#6 窗口：生机先抬上限
    // +15/治疗+15/反噬-15，万花后吸（减攻 1→2，按减攻 2 吸血+2）；若万花先行，
    // +2 被旧上限 149 截断、天赋 64 少触发一次防，飞牙剑（4+剑意6-减攻2=8）
    // 对 3 防打出 5（149→144），原版对 4 防打出 4（151→147）。
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("data-driven")
        .join("pa_31843a8a.json");
    let fixture = crate::fixture::load_fixture_file(&path)
        .unwrap_or_else(|error| panic!("formation order fixture loads: {error}"));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert_eq!(summary.winner_side, PlayerSide::P1);
    assert_eq!(summary.actor_turn_count, 32);
    assert_eq!(summary.hp_delta_p1_minus_p2, 185);
    assert!(
        summary.matches_fixture(&fixture),
        "bloom-before-maze order must replay exact"
    );
}

#[test]
fn xing_yi_duan_can_cycle_past_used_sustain_cards() {
    // pa_3a4d3f38：须弥后的断累计 6 层，牌队列里已有离场牌。
    // BattleCharacter.ShiftCard 只在连续 8 张标记 skip 时替换为普通攻击；
    // BattleExecuter 的断每消耗一层后会再次 ShiftCard，可以绕回可用牌。
    let mut state = ReplayState::test_from_fixture(&fixture(
        vec![original(1_020_010); 8],
        deck(basic_attack()),
    ));
    state.p1.deck.queue = (0..8).collect();
    state.p1.deck.slots[0].skipped = true;
    state.p1.deck.slots[1].skipped = true;
    state.p1.astrology.star_chess_break = 6;

    let drawn = state.p1.draw_next_card(0).expect("available card");
    assert_eq!(drawn.source_slot, 2);
    assert_eq!(drawn.card.id, 1_020_010);
    assert!(!drawn.fallback_basic_attack);
    assert_eq!(state.p1.astrology.star_chess_break, 0);
}

#[test]
fn next_card_anima_reduction_survives_shortage_until_success() {
    // CardActionBase.CheckAnima:5740-5749：不足时返回，成功后移除 buff 319。
    // pa_a55ebbbc：减耗 2、灵气 0 时无法出巨鲲；灵气补到 1 后可出。
    let mut battle = fixture(deck(original(1_000_036)), deck(basic_attack()));
    battle.players.p1.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.anima = 0;
    state.p1.turn.next_card_anima_cost_reduction = 2;
    state.p1.deck.queue = vec![0];
    state.current_actor = PlayerSide::P1;

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.turn.next_card_anima_cost_reduction, 2);
    assert_eq!(state.p1.deck.queue[0], 0);

    state.p1.core.anima = 1;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.turn.next_card_anima_cost_reduction, 0);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.defense, 10);
}

#[test]
fn xian_mo_strategy_7138_turn_end_water_defense() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.xian_mo_strategies.push(7138);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.elements.water_momentum = 7;
    state.apply_turn_end_phases(PlayerSide::P1);
    // water_momentum 7 / 2 = 3 defense gained.
    assert_eq!(state.p1.core.defense, 3);
}

#[test]
fn xian_mo_strategy_1010_turn_3_reduces_attack_and_damage() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p2.xian_mo_strategies.push(1010);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p2.core.max_hp = 100;
    state.p2.core.hp = 100;
    // Turn 2: own_turn_count == 2, no reduction (100 - 10 = 90)
    state.p1.turn.own_turn_count = 2;
    state.apply_attack(PlayerSide::P1, 10, 0);
    assert_eq!(state.p2.core.hp, 90);

    // Turn 3: own_turn_count == 3, factor_percent -= 40 -> 10 * 60% = 6 (90 - 6 = 84)
    state.p1.turn.own_turn_count = 3;
    state.apply_attack(PlayerSide::P1, 10, 0);
    assert_eq!(state.p2.core.hp, 84);
}

#[test]
fn xian_mo_strategy_7097_round_gate_and_sharpness() {
    // 7097 takes effect from Round 13: each Earth Spirit card played grants 2 sharpness.
    let mut battle = fixture(deck(original(7_010_062)), deck(basic_attack())); // 土灵•断崖
    battle.players.p1.xian_mo_strategies.push(7097);
    battle.source = Some(crate::FixtureSource {
        round: Some(11),
        ..crate::FixtureSource::default()
    });
    let mut state = ReplayState::test_from_fixture(&battle);
    // Round 11 (< 13): playing Earth Spirit card grants no sharpness from 7097
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.sharpness, 0);

    let mut battle13 = fixture(deck(original(7_010_062)), deck(basic_attack()));
    battle13.players.p1.xian_mo_strategies.push(7097);
    battle13.source = Some(crate::FixtureSource {
        round: Some(13),
        ..crate::FixtureSource::default()
    });
    let mut state13 = ReplayState::test_from_fixture(&battle13);
    // Round 13: playing Earth Spirit card grants 2 sharpness
    state13.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state13.p1.sword.sharpness, 2);
}

#[test]
fn xian_mo_strategy_7100_round_gate() {
    // 7100 takes effect from Round 13: upgrades 1 Fire Spirit card in battle deck.
    let mut battle12 = fixture(deck(original(7_000_009)), deck(basic_attack())); // 火灵印
    battle12.players.p1.xian_mo_strategies.push(7100);
    battle12.source = Some(crate::FixtureSource {
        round: Some(12),
        ..crate::FixtureSource::default()
    });
    let state12 = ReplayState::test_from_fixture(&battle12);
    assert_eq!(state12.p1.deck.slots[0].card.id, 7_000_009);

    let mut battle13 = fixture(deck(original(7_000_009)), deck(basic_attack()));
    battle13.players.p1.xian_mo_strategies.push(7100);
    battle13.source = Some(crate::FixtureSource {
        round: Some(13),
        ..crate::FixtureSource::default()
    });
    let state13 = ReplayState::test_from_fixture(&battle13);
    assert_eq!(state13.p1.deck.slots[0].card.id, 7_010_009); // upgraded
}

#[test]
fn xian_mo_strategy_7080_triggers_on_internal_injury_and_flaw() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p2.xian_mo_strategies.push(7080);
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.core.anima, 0);

    // NeiShang (100) on P1 gives P2 1 anima
    state.add_actor_negative_status(PlayerSide::P1, 100, 1);
    assert_eq!(state.p2.core.anima, 1);

    // PoZhan (102) on P1 gives P2 1 anima
    state.add_actor_negative_status(PlayerSide::P1, 102, 1);
    assert_eq!(state.p2.core.anima, 2);

    // XuRuo (101) on P1 gives P2 1 anima
    state.add_actor_negative_status(PlayerSide::P1, 101, 1);
    assert_eq!(state.p2.core.anima, 3);
}

#[test]
fn xian_mo_strategy_7026_grants_two_sharpness_for_water_spirit() {
    let mut battle = fixture(deck(original(7_000_019)), deck(basic_attack())); // 水灵•波澜
    battle.players.p1.xian_mo_strategies.push(7026);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.anima = 2;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.sharpness, 2);
}

#[test]
fn xian_mo_strategy_7113_deals_five_damage_on_stance_switch() {
    let mut battle = fixture(deck(original(222)), deck(basic_attack())); // 转势
    battle.players.p1.character_id = Some(4000005);
    battle.players.p1.xian_mo_strategies.push(7113);
    let mut state = ReplayState::test_from_fixture(&battle);
    let opponent_hp_before = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, opponent_hp_before - 5);
}

#[test]
fn xian_mo_strategy_7111_modifies_physique_and_defense_on_stance_switch() {
    let mut battle = fixture(deck(original(222)), deck(basic_attack())); // 转势
    battle.players.p1.character_id = Some(4000005);
    battle.players.p1.xian_mo_strategies.push(7111);
    let mut state = ReplayState::test_from_fixture(&battle);
    let def_before = state.p1.core.defense;
    let physique_before = state.p1.core.physique;
    let momentum_before = state.p1.beng.momentum;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.defense, def_before + 3);
    assert_eq!(state.p1.core.physique, physique_before + 3);
    assert_eq!(state.p1.beng.momentum, momentum_before);
}

#[test]
fn opening_herbs_resolve_before_opening_cards_triggering_star_erosion_on_herb_debuff() {
    // 灵植草药（失力草 10018 减攻 1）先于开局卡牌（血光之灾 11010024 内伤 1）结算：
    // p2 星蚀（10103，首个负面状态 +1）增幅失力草使其减攻变为 2，
    // 随后的血光之灾内伤保持 1。
    // 锚点：hf-26315169-unverified pa_13fa2682
    let mut battle = fixture(deck(original(11_010_024)), deck(basic_attack()));
    battle.players.p1.permanent_buff_temp_datas.insert("10018".to_string(), 1);
    battle.players.p1.talents.push(10103);
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.status.attack_reduction, 2);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn multiple_regenerative_body_talents_apply_physique_sequentially_reflecting_under_iron_bone() {
    // 多个再生之躯（149/10149/20149）首格出牌前逐个结算，分别触发 幻•锻骨 (277) 溢出反弹伤害；
    // 目标有金灵•铁骨（TieGu，-5 最低减至 1）时，每次独立反弹各自保底 1 伤（1+1+1=3 伤）。
    // 锚点：hf-26315169-unverified pa_e0345f4f
    let mut battle = fixture(deck(original(277)), deck(basic_attack()));
    battle.players.p1.talents = vec![149, 10_149, 20_149];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);
    state.p1.core.physique = 50;
    state.p1.core.physique_limit = 50;
    state.p2.elements.metal_iron_bone = 1;
    let p2_hp_before = state.p2.core.hp;
    let card_def = basic_attack();
    state.apply_selected_card_talent_loop_hooks(PlayerSide::P1, &card_def, 0, false);
    assert_eq!(state.p2.core.hp, p2_hp_before - 3);
}
