use crate::fixture::{load_fixture_file, BattleFixture};
use crate::model::{CardDefinition, PlayerSide};
use crate::{EngineError, Result};
use std::collections::BTreeMap;
use std::path::Path;

pub use engine_contract::engine_contract_fixture;
pub use hook_trace::{
    trace_replay_fixture_hooks, ReplayHookTrace, ReplayHookTraceChange, ReplayHookTraceStep,
};
pub use turn_end_trace::{
    ReplayTurnEndHookPair, ReplayTurnEndHookReceipt, ReplayTurnEndHookSnapshot,
};

mod action_again;
mod battle_start;
mod body;
mod card_effect_catalog;
mod card_routing;
mod cards_dream_direct;
mod cards_dream_fate;
mod cards_dream_mirage;
mod cards_dream_mirage_direct;
mod cards_mirage_ronghui;
mod cards_missing;
mod cards_qixing;
mod cards_ronghui;
mod cards_ronghui_early;
mod cards_synthetic_full_scope_candidates;
mod cards_synthetic_oracle_dream_mirage_pilot;
mod cards_synthetic_oracle_verified;
mod cards_synthetic_oracle_verified_secret_extreme_remaining;
mod cards_synthetic_oracle_verified_secret_misc;
mod cards_synthetic_oracle_verified_secret_sword;
mod chance_cards;
mod combat;
mod combat_core;
mod combat_core_outcome;
mod combat_core_status;
mod decisions;
mod deck_start;
mod dream_mirage_hooks;
mod effect_invocation;
mod elements;
mod elements_late;
mod engine_contract;
mod error;
mod observation;
mod original_build_profile;
mod original_config;
pub use observation::{
    ReplayAttackSegment, ReplayDetailEntry, ReplayDetailedEvent, ReplayDetailedRun,
    ReplayDetailedStep, ReplayHookCategory, ReplayMutationKind, ReplayMutationReceipt,
};

mod fate_strategy;
mod fate_strategy_elements;
mod flow;
mod flow_card_effect;
mod flow_card_effect_fallback;
mod flow_card_effect_primary;
mod flow_initialization;
mod flow_support;
mod formations;
mod hexagram_cards;
mod hook_trace;
mod mechanic_cards_extra;
mod player;
mod resources;
mod snapshot;
mod support;
mod swords;
#[cfg(test)]
mod test_support;
#[cfg(test)]
mod tests;
#[cfg(test)]
mod tests_action_again;
#[cfg(test)]
mod tests_actual_damage_carry;
#[cfg(all(test, feature = "private-fixtures"))]
mod tests_body;
#[cfg(test)]
mod tests_build_2026_07;
#[cfg(test)]
mod tests_build_2026_08;
#[cfg(test)]
#[cfg(all(test, feature = "private-fixtures"))]
mod tests_build_2026_08_rotation_fixtures;
#[cfg(test)]
mod tests_card19_fate387;
#[cfg(test)]
mod tests_card_execution_lifecycle;
#[cfg(test)]
mod tests_chance;
#[cfg(test)]
mod tests_damage;
#[cfg(test)]
mod tests_dream_five_elements_spike;
#[cfg(test)]
mod tests_fate_strategy;
#[cfg(test)]
mod tests_fate_strategy_batch_009;
#[cfg(test)]
mod tests_hold;
#[cfg(test)]
mod tests_original_grants;
#[cfg(all(test, feature = "private-fixtures"))]
mod tests_percent_roll_decisions;
#[cfg(test)]
mod tests_player;
#[cfg(all(test, feature = "private-fixtures"))]
mod tests_random_range_decisions;
#[cfg(test)]
mod tests_resource_mutation_contract;
#[cfg(test)]
mod tests_star_cards;
#[cfg(test)]
mod tests_strict_battle_start;
#[cfg(all(test, feature = "private-fixtures"))]
mod tests_synthetic_decisions;
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
#[cfg(test)]
mod tests_talent_batch_010;
#[cfg(test)]
mod tests_talent_resource;
#[cfg(test)]
mod tests_turn_action_lifecycle;
mod turn_end_trace;

pub use decisions::{
    ReplayDecisionDomain, ReplayDecisionEvent, ReplayDecisionIntegerRange, ReplayDecisionKind,
    ReplayDecisionProvider,
};
pub use error::BattleError;

const DEFAULT_MAX_ACTOR_TURNS: i64 = 64;
const BASIC_ATTACK_ID: i64 = 0;
const BASIC_ATTACK_DAMAGE: i64 = 3;
const CARD_TYPE_CONSUME: i64 = 1;
const CARD_TYPE_SUSTAIN: i64 = 3;
const PERMANENT_PHYSIQUE_KEY: &str = "10023";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Element {
    Metal,
    Water,
    Wood,
    Fire,
    Earth,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplaySummary {
    pub winner_side: PlayerSide,
    pub actor_turn_count: i64,
    pub hp_delta_p1_minus_p2: i64,
}

impl ReplaySummary {
    pub fn matches_fixture(&self, fixture: &BattleFixture) -> bool {
        self.winner_side == fixture.expected.winner_side
            && self.actor_turn_count == fixture.expected.actor_turn_count
            && self.hp_delta_p1_minus_p2 == fixture.expected.hp_delta_p1_minus_p2
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayRun {
    pub summary: ReplaySummary,
    pub termination_cause: ReplayTerminationCause,
    pub completed_checkpoint_count: usize,
    pub events: Vec<ReplayEvent>,
    /// One entry per `events` element, same index. Kept beside the events instead of
    /// inside their snapshots so the oracle parity protocol stays exactly 29 fields.
    pub prevention: Vec<ReplayPreventionPair>,
    pub decision_events: Vec<ReplayDecisionEvent>,
}

/// Browser-facing replay stream with live state that intentionally stays outside
/// the exact 29-field original-oracle snapshot.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayUiRun {
    pub summary: ReplaySummary,
    pub termination_cause: ReplayTerminationCause,
    pub completed_checkpoint_count: usize,
    pub events: Vec<ReplayUiEvent>,
    pub decision_events: Vec<ReplayDecisionEvent>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayTerminationCause {
    TurnStartLethal,
    CardLethal,
    ActionAgainLethal,
    TurnEndLethal,
    MaxTurn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayEvaluationRun {
    pub summary: ReplaySummary,
    pub p1: ReplayPlayerSnapshot,
    pub p2: ReplayPlayerSnapshot,
    pub decision_events: Vec<ReplayDecisionEvent>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayEvent {
    pub turn: i64,
    pub kind: ReplayEventKind,
    pub actor: PlayerSide,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1: ReplayPlayerSnapshot,
    pub p2: ReplayPlayerSnapshot,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayUiEvent {
    pub turn: i64,
    pub kind: ReplayEventKind,
    pub actor: PlayerSide,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1: ReplayUiPlayerSnapshot,
    pub p2: ReplayUiPlayerSnapshot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayEventKind {
    BattleStart,
    TurnStart,
    CardCompleted,
    TurnEnd,
    BattleEnd,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPlayerSnapshot {
    pub hp: i64,
    pub max_hp: i64,
    pub defense: i64,
    pub anima: i64,
    pub guard: i64,
    pub physique: i64,
    pub sword_intent: i64,
    pub sharpness: i64,
    pub cloud_chain: i64,
    pub cloud_sea: i64,
    pub momentum: i64,
    pub agility: i64,
    pub water_momentum: i64,
    pub activated_metal: i64,
    pub activated_water: i64,
    pub activated_wood: i64,
    pub activated_fire: i64,
    pub activated_earth: i64,
    pub hexagram: i64,
    pub star_power: i64,
    pub attack_bonus: i64,
    pub internal_injury: i64,
    pub weakness: i64,
    pub flaw: i64,
    pub attack_reduction: i64,
    pub entangle: i64,
    pub external_injury: i64,
    pub lost_mind: i64,
    pub action_again_count: i64,
    /// 李㵘锻玄架势：互斥的拳/棍模式标记。detail_entries 一直有这两项，
    /// snapshot 必须同口径暴露，否则右侧钩子链与左侧状态条对同一字段
    /// 一边显示一边缺失。
    pub quan_stance: i64,
    pub gun_stance: i64,
    /// 全量暴露缺口（档 1a/1b）：原版 RefreshBuff 会显示、左侧状态条此前缺失的
    /// 字段，逐一与 detail_entries 同口径（见 build_display_gap_report.py 锚定表）。
    /// 每项锚定 archive 枚举名（BuffConfig 分类显示类）与 BuffType ID：
    /// 剑系
    pub metal_ring: i64, // KunWuJinHuan(273) 锟铻金环 Neutral
    pub sword_energy: i64,                // JianQi(625) 剑气 Positive
    pub water_month_sword_formation: i64, // ShuiYueJianZhen(202) 水月剑阵 Neutral
    /// 五行
    pub water_formation: i64, // ShuiLingZhen(244) 水灵阵 Neutral
    pub metal_formation: i64,             // JinLingZhen(242) 金灵阵 Neutral
    pub earth_formation: i64,             // TuLingZhen(246) 土灵阵 Neutral
    pub fire_formation: i64,              // HuoLingZhen(245) 火灵阵 Neutral
    pub spring_flow: i64,                 // QuanYong(270) 泉涌 Neutral
    pub water_stealth: i64,               // QianDun(12) 潜遁 Positive
    pub metal_iron_bone: i64,             // TieGu(9) 铁骨 Positive
    pub earth_eight_wastes: i64,          // HeBaHuang(11) 合八荒 Positive
    pub wood_array: i64,                  // MuLingZhen(243) 木灵阵 Neutral
    /// 阵法
    pub turtle_formation: i64, // GuiJiaZhen(252) 龟甲阵 Neutral
    pub shatter_formation: i64,           // SuiShaZhen(251) 碎杀阵 Neutral
    pub thunder_formation: i64,           // YinLeiZhen(250) 引雷阵 Neutral
    pub evil_gu_formation: i64,           // XieGuZhen(253) 邪蛊阵 Neutral
    pub spirit_gathering_formation: i64,  // JuLinZhen(254) 聚灵阵 Neutral
    pub heaven_cycle_sword_formation: i64, // ZhouTianJianZhen(255) 周天剑阵 Neutral
    pub heaven_force_formation: i64,      // TianGangJuLiZhen(257) 天罡聚力阵 Neutral
    pub flower_maze_formation: i64,       // WanHuaMiHunZhen(258) 万花迷魂阵 Neutral
    pub immovable_formation: i64,         // BuDongJinGangZhen(271) 不动金刚阵 Neutral
    pub eight_gates_formation: i64,       // BaMenJinSuoZhen(256) 八门金锁阵 Neutral
    pub six_yao_formation: i64,           // LiuYaoShaZhen(204) 六爻煞阵 Neutral
    /// 锻玄
    pub beng_quan_cun_jin: i64, // BengQuanCunJin(290) 崩拳寸劲 Neutral
    pub beng_quan_return_profound: i64,   // BengQuanFanXuan(418) 崩拳返玄 Neutral
    pub dream_beng_quan_chain: i64,       // MengLianBeng(725) 梦崩拳连崩 Neutral
    /// 琴曲
    pub immortal_binding_tune: i64, // TianYinKunXianQu(215) 天音困仙曲 Neutral
    pub illusory_tune: i64,               // HuanYinQu(209) 幻音曲 Neutral
    pub heartbreak_tune: i64,             // DuanChangQu(211) 断肠曲 Neutral
    pub wild_dance_tune: i64,             // KuangWuQu(212) 狂舞曲 Neutral
    pub rejuvenation_tune: i64,           // HuiChunQu(213) 回春曲 Neutral
    pub xiaoyao_tune: i64,                // XiaoYaoQu(208) 逍遥曲 Neutral
    pub xiaoyao_guqin: i64,               // XiaoYaoGuQin(274) 逍遥古琴 Neutral
    pub chaotic_mind_tune: i64,           // WanMoShiXinQu(214) 万魔蚀心曲 Neutral
    /// 卦星
    pub ling_gua_art: i64,    // LingGuaShu(358) 灵卦术 Neutral
    pub star_moon_fan: i64,               // XingYueYuShan(260) 星月折扇 Neutral
    pub infinite_hexagram_plate: i64,     // WuJiGuaPan(266) 无极卦盘 Neutral
    pub all_goes_well: i64,               // WanShiRuYi(387) 万事如意 Neutral
    /// 状态
    pub recovery: i64,        // HuiFu(248) 恢复 Positive
    pub meditation: i64, // Min(367) 冥 Negative（detail_entries 同口径 label「冥」）
    pub blood_calamity: i64, // XueGuangZhiZai(379) 血光之灾 Neutral
    pub lone_night_wolf: i64, // GuYeLang(234) 孤夜狼 Neutral
    pub leaf_blade_flower: i64, // YeRenHua(278) 叶刃花 Neutral
    /// 仙命
    pub quiet_mindset: i64, // JingQiXinFa(203) 静气心法 Neutral
    pub reflect_mindset: i64, // FanZhenXinFa(217) 反震心法 Neutral
    pub graft_flowers_to_tree: i64, // YiHuaJieMu(216) 移花接木 Neutral
    pub tide: i64,       // HaiChao(247) 海潮 Neutral
    pub dismantle_move: i64, // ChaiZhao(459) 拆招 Neutral
    pub all_things_inauspicious: i64, // ZhuShiBuYi(381) 诸事不宜 Neutral
    pub fate_cycle: i64, // DongZhuJiXian(390) 命运轮回 Neutral
    pub yellow_bird_behind: i64, // HuangQueZaiHou(264) 黄雀在后 Neutral
    pub exorcism: i64,   // BiXie(13) 辟邪 Positive
    pub ice_snow_lotus: i64, // BingFengXueLian(14) 冰封雪莲 Positive
    pub leaf_shield_flower: i64, // YeDunHua(276) 叶盾花 Neutral
    pub paint_finishing_touch: i64, // HuaLongDianJing(281) 画龙点睛 Neutral
    /// 回合
    pub next_turn_defense: i64, // XiaHuiHeJiaFang(7) 下回合加防 Positive
    pub ignore_defense_attacks: i64, // WuShiFangYu(4) 无视防御 Positive
    pub next_attack_shatter_defense: i64, // XiaCiGongJiSuiFang(383) 下次攻击碎防 Neutral
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayUiPlayerSnapshot {
    #[serde(flatten)]
    pub parity: ReplayPlayerSnapshot,
    pub momentum_limit: i64,
    pub last_element: Option<&'static str>,
    pub card_queue: Vec<usize>,
    pub slots: Vec<ReplayUiCardSlotSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayUiCardSlotSnapshot {
    pub index: usize,
    pub card_id: i64,
    pub base_id: i64,
    pub name: String,
    pub skipped: bool,
    pub had_used: bool,
}

#[derive(Debug, Clone)]
struct ReplayCardSlot {
    card: CardDefinition,
    skipped: bool,
    used: bool,
}

#[derive(Debug, Clone, Default)]
struct ReplayPlayer {
    identity: ReplayPlayerIdentity,
    core: ReplayCoreVitals,
    sword: ReplaySwordState,
    elements: ReplayElementState,
    formations: ReplayFormationState,
    beng: ReplayBengQuanState,
    music: ReplayMusicState,
    astrology: ReplayAstrologyState,
    status: ReplayStatusState,
    hp_mutation: ReplayHpMutationState,
    fate: ReplayFateTalentState,
    chance: ReplayChanceState,
    dream_mirage: ReplayDreamMirageState,
    mirage_ronghui: ReplayMirageRonghuiState,
    ronghui: ReplayRonghuiState,
    turn: ReplayTurnState,
    deck: ReplayDeckState,
    prevention: ReplayPreventionState,
}

/// Cumulative HP loss this player never took because guard or defense absorbed it.
///
/// This is derived telemetry, not battle state: the original client does not report
/// it, so it must stay out of the 29-field oracle parity snapshot
/// (`ORIGINAL_ORACLE_PROTOCOL_SNAPSHOT_FIELDS`) and travels beside the event stream
/// instead. Nothing in the rules reads it back; removing it cannot change an outcome.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPreventionState {
    /// Requested HP loss cancelled by a guard stack.
    pub hp_loss_prevented_by_guard: i64,
    /// Incoming damage absorbed by defense before it could become HP loss.
    pub hp_loss_prevented_by_defense: i64,
}

impl ReplayPreventionState {
    /// 两个累计量之差，也就是这一段里新增的吸收量。
    pub fn saturating_sub_state(self, earlier: Self) -> Self {
        Self {
            hp_loss_prevented_by_guard: (self.hp_loss_prevented_by_guard
                - earlier.hp_loss_prevented_by_guard)
                .max(0),
            hp_loss_prevented_by_defense: (self.hp_loss_prevented_by_defense
                - earlier.hp_loss_prevented_by_defense)
                .max(0),
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayPreventionPair {
    pub p1: ReplayPreventionState,
    pub p2: ReplayPreventionState,
}

#[derive(Debug, Clone, Default)]
struct ReplayPlayerIdentity {
    level: i64,
    character_id: Option<i64>,
    fate_strategies: Vec<i64>,
    /// FateStrategyFunctions.IsSwitchActive reads this fixture-owned map:
    /// missing/zero means active, non-zero means disabled.
    fate_strategy_temp_datas: BTreeMap<String, i64>,
    talents: Vec<i64>,
    talent_resonance_id: Option<i64>,
    talent_resonance_temp_flags: Vec<i64>,
    ke_yin_card_ids: Vec<i64>,
    last_round_exp: i64,
    /// Talent 199 (五行灵田系) card params — BattleCharacter.GetWuXingCountInDeck
    /// scans their names for 五行 tokens when fate strategy 417 is present.
    talent_199_card_ids: Vec<i64>,
}

#[derive(Debug, Clone, Default)]
struct ReplayCoreVitals {
    hp: i64,
    max_hp: i64,
    temp_life: i64,
    defense: i64,
    anima: i64,
    guard: i64,
    temporary_guard: i64,
    attack_bonus: i64,
    physique: i64,
    physique_limit: i64,
    lost_max_hp_count: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplaySwordState {
    sword_intent: i64,
    sword_energy: i64,
    sharpness: i64,
    metal_ring: i64,
    cloud_chain: i64,
    cloud_sea: i64,
    cloud_sword_soft_heart: i64,
    cloud_sword_heart: i64,
    frenzy_dragon_swallows_cloud: i64,
    frenzy_sword: i64,
    frenzy_sword_zero: i64,
    /// BuffType.ShengJiXiaCiKuangJian = 671（升级下次狂剑）。
    upgrade_next_frenzy_sword: i64,
    sword_formation_count: i64,
    hundred_bird_spirit_sword_art: i64,
    hundred_bird_trailing_shadow_art: i64,
    hundred_beast_spirit_sword_formation: i64,
    hundred_beast_spirit_sword_formation_triggered: bool,
    water_month_sword_formation: i64,
    sword_intent_circulation: i64,
    dark_void_sword_formation_art: i64,
    spirit_sword_mindset: i64,
    cloud_step: i64,
    frenzy_sword_double_effect: i64,
    ling_wu_card_base_ids: Vec<i64>,
    cloud_sword_heaven_cycle: i64,
    all_cards_as_cloud_sword: i64,
    frenzy_sword_cloud_gathering: i64,
    all_purpose_sword: i64,
    all_purpose_sword_effective_count: i64,
    next_card_as_cloud_sword: i64,
    next_cards_as_frenzy_sword: i64,
    next_cards_as_frenzy_sword_effective_count: i64,
    /// 云剑•猫影（卡 403，BuffConfig 757 YunJianMaoYing Hidden）：
    /// 每次使用云剑后追加此层数攻击（CardActionBase.cs:4413-4427）。
    /// 内部状态，不进 parity snapshot。
    yun_jian_mao_ying: i64,
    chain_sword_temporary_cursor: Option<usize>,
}

#[derive(Debug, Clone, Default)]
struct ReplayElementState {
    water_momentum: i64,
    water_formation: i64,
    metal_formation: i64,
    earth_formation: i64,
    fire_formation: i64,
    spring_flow: i64,
    water_stealth: i64,
    metal_iron_bone: i64,
    swift_burn_seal: i64,
    metal_cauldron_drop: i64,
    no_sharpness_for_attack: i64,
    water_blade_seal: i64,
    earth_cliff_counter: i64,
    earth_eight_wastes: i64,
    dream_two_polarity_defense: i64,
    dream_two_polarity_hp: i64,
    primordial_infinity_formation: i64,
    five_elements_marrow_art: i64,
    wood_array: i64,
    wood_healing_formation: i64,
    wood_spirit_all_growth: i64,
    wood_spirit_all_growth_attack: i64,
    wood_thorn: i64,
    /// GongJiXiQuShengMing(668) 攻击吸取生命：梦•木灵阵（700081，境界<=元婴）
    /// 发放的逐段攻击吸血 charge。原版每段攻击前置钩子
    /// （BattleCharacter.cs:11766-11769）消耗 1 层：目标 -1 血、自身 +1 血。
    attack_life_drain: i64,
    activated_metal: i64,
    activated_water: i64,
    activated_wood: i64,
    activated_fire: i64,
    activated_earth: i64,
    next_card_activate_element: i64,
    seal_suppressing_mindset: i64,
    water_momentum_gain_count: i64,
    five_elements_gourd: i64,
    activated_elements: Vec<Element>,
    last_element: Option<Element>,
    // Steam build 24217566: YiYongGuoShuiLingPai — water-spirit cards resolved this battle.
    used_water_spirit_card: i64,
    long_ma_spirit: i64,
    synthetic_ding_feng_bo_candidate: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayFormationState {
    turtle_formation: i64,
    turtle_formation_defense: i64,
    hard_branch_bamboo: i64,
    hard_branch_bamboo_defense_per_damage: i64,
    forge_bone_attacks: i64,
    forge_bone_attack_bonus: i64,
    fortune_avoid_misfortune: i64,
    fortune_avoid_misfortune_defense: i64,
    fortune_avoid_misfortune_healing: i64,
    shatter_formation: i64,
    shatter_formation_bonus: i64,
    thunder_formation: i64,
    thunder_formation_damage: i64,
    evil_gu_formation: i64,
    evil_gu_formation_value: i64,
    spirit_gathering_formation: i64,
    spirit_gathering_formation_value: i64,
    heaven_cycle_sword_formation: i64,
    heaven_cycle_sword_formation_damage: i64,
    heaven_force_formation: i64,
    flower_maze_formation: i64,
    flower_maze_drain: i64,
    immovable_formation: i64,
    immovable_formation_value: i64,
    eight_gates_formation: i64,
    eight_gates_formation_damage: i64,
    array_echo_persistent_card: i64,
    body_observation: i64,
    soul_injury_curse_formation: i64,
    six_yao_formation: i64,
    spirit_formation_echo: i64,
    spirit_formation_echo_triggered: bool,
}

#[derive(Debug, Clone, Default)]
struct ReplayBengQuanState {
    next_beng_quan_hp_cost_damage: i64,
    beng_quan_double_shadow: i64,
    momentum: i64,
    momentum_limit: i64,
    /// Fate 423 XiaCiQiShiDuoJia (Buff 769): consumed by the next positive
    /// QiShi mutation before the ordinary momentum hooks and cap/overflow.
    pending_momentum_bonus: i64,
    momentum_multiplier: i64,
    momentum_gain_agility_triggered: i64,
    beng_quan_cun_jin: i64,
    beng_quan_bounce: i64,
    beng_quan_chan: i64,
    beng_quan_han: i64,
    beng_quan_flash_agility: i64,
    beng_quan_chuo: i64,
    consumed_beng_quan_chuo: i64,
    beng_quan_defense: i64,
    beng_quan_tu: i64,
    beng_quan_meridian: i64,
    beng_quan_startled_touch: i64,
    triggered_startled_touch: i64,
    beng_quan_return_profound: i64,
    beng_tian_step: i64,
    beng_mei_mindset: i64,
    momentum_before_attack: i64,
    unceasing_momentum: i64,
    return_to_simplicity: i64,
    dream_beng_quan_chain: i64,
    triggered_dream_beng_quan_chain: i64,
    beng_quan_fu_hu: i64,
    triggered_beng_quan_fu_hu: i64,
    quan_stance: i64,
    gun_stance: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayMusicState {
    music_cards_played: i64,
    immortal_binding_tune: i64,
    immortal_binding_vine: i64,
    devouring_ancient_vine: i64,
    illusory_tune: i64,
    heartbreak_tune: i64,
    wild_dance_tune: i64,
    rejuvenation_tune: i64,
    xiaoyao_tune: i64,
    xiaoyao_guqin: i64,
    chaotic_mind_tune: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayAstrologyState {
    hexagram: i64,
    lost_hexagram: i64,
    hexagram_effective_count: i64,
    ling_gua_art: i64,
    star_power: i64,
    star_moon_fan: i64,
    anima_to_star_power: i64,
    pending_anima_hexagram: bool,
    star_chess_break: i64,
    infinite_hexagram_plate: i64,
    all_goes_well: i64,
    star_erosion: i64,
    thunder_mindset: i64,
    dream_thunder_hexagram: i64,
    dream_thunder_round_limit: i64,
    /// 紫芒星爆（卡 422，BuffConfig 773 ZiMangXingBao Hidden）：
    /// 持有期间星力代替灵气/卦象消耗，且星力流失转等量加攻
    /// （BattleCharacter.cs:8811-8816 / :9333-9337、CardActionBase.cs:5127）。
    /// 内部状态，不进 parity snapshot。
    zi_mang_xing_bao: i64,
    /// 雷闪二度（卡 407，BuffConfig 763 LeiShanErDu Hidden）：
    /// 下一张名字含「雷」的牌连续生效 2 次（CardActionBase.cs:1506-1519）。
    /// 内部状态，不进 parity snapshot。
    lei_shan_er_du: i64,
    star_slots: Vec<usize>,
}

#[derive(Debug, Clone, Default)]
struct ReplayStatusState {
    external_injury: i64,
    internal_injury: i64,
    transient_internal_injury: i64,
    flame_heart_urging: i64,
    recovery: i64,
    meditation: i64,
    min_night: i64,
    mystic_soul: i64,
    weakness: i64,
    attack_reduction: i64,
    cannot_act: i64,
    flaw: i64,
    drunken_fist_stance: i64,
    drunken_leisure: i64,
    entangle: i64,
    yin_fu: i64,
    /// 阴符绝阵（卡 429，BuffConfig 759 YinFuJueZhen Hidden）：
    /// 对方每获得 1 层负面状态对其造成 层数×此值 反伤（豁免「冥」367）。
    /// 内部状态，不进 parity snapshot。
    yin_fu_jue_zhen: i64,
    back_solitude: i64,
    strike_void: i64,
    blood_calamity: i64,
    lone_night_wolf: i64,
    leaf_pluck_flying_leaf: i64,
    leaf_blade_flower: i64,
    poison_immunity: i64,
    lost_mind: i64,
}

/// Outcome-relevant BuffType state owned by BattleCharacter.ModifyHp.
///
/// Intentionally absent: `JiLuZhanDouZuiGaoShengMing` is a write-only
/// diagnostic high-water mark in the current build, while the `fengRui`
/// ModifyHp argument controls floating text/FX only.
#[derive(Debug, Clone, Default)]
struct ReplayHpMutationState {
    /// Original `AddHpCount`: post-modifier, pre-max-HP-clamp positive delta
    /// accumulated for the whole battle. This is distinct from turn-local
    /// `HuiHeJiaShengMing`.
    add_hp_count: i64,
    no_hp_loss_before_next_turn: i64,
    appetite: i64,
    red_date_zongzi: i64,
    egg_yolk_zongzi: i64,
    immortal_egg_yolk_zongzi: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayFateTalentState {
    quiet_mindset: i64,
    reflect_mindset: i64,
    graft_flowers_to_tree: i64,
    paint_finishing_touch: i64,
    generating_interaction_upgrade: i64,
    tide: i64,
    spirit_gathering_mindset: i64,
    five_elements_gathering_triggered: i64,
    half_anima: i64,
    mystic_heart_enter_profound: i64,
    dismantle_move: i64,
    dismantle_move_reflect: i64,
    all_things_inauspicious: i64,
    last_stand_intent: i64,
    last_stand_unyielding: i64,
    flame_soul_return: i64,
    fire_phoenix_revive_hp: i64,
    /// 七星借命 FateStrategy 436 标记（原版 BuffType QiXingJieMing 772）：
    /// 战斗中首次生命 ≤ 0 时，失去所有卦象与星力，每失去 1 点加
    /// otherParams[0]=3 生命及上限，随后若生命 > 0 则继续战斗。
    qi_xing_jie_ming: i64,
    /// 截拳式 FateStrategy 430 标记（原版 BuffType JieQuanShi 770）：
    /// 开局 1 层；每次造成实际攻击伤害时消耗 1 层，拳架势给目标
    /// +1 减攻，否则 +1 虚弱（BattleCharacter.cs:10981-10990）。
    jie_quan_shi: i64,
    /// 风灵锻躯 FateStrategy 431 计数（原版 BuffType
    /// TianYanFengLingDuanQu 771）：开局 5 层；每次加身法时消耗
    /// 1 层换 1 体魄（BattleCharacter.cs:8733-8737）。
    tian_yan_feng_ling_duan_qu: i64,
    /// 促局飞袭 FateStrategy 416 标记（原版 BuffType CuJuFeiXi 768）：
    /// 开局 otherParams[0]=5 层（FateStrategyFunctions.cs:571-573，
    /// OnBattleStart 内）；每张名字含「火灵」的牌（含 temp 执行）在
    /// OnAfterExecuted 时若仍有层数则全部消耗并对对方追加一次
    /// Attack(buffValue)（CardActionBase.cs:3996-4002）。
    cu_ju_fei_xi: i64,
    /// 天衍-无忧灵酿 FateStrategy 407 标记（原版 BuffType WuYouLingNiang 767）：
    /// 开局 3 层；每次正向 ModifyAnima 消耗 1 层，并按 otherParams[0]=4
    /// 增加生命上限及生命（FateStrategyFunctions.cs:557-560、
    /// BattleCharacter.cs:9516-9521）。
    wu_you_ling_niang: i64,
    /// 无尽卦衍 FateStrategy 395 标记（原版 BuffType WuJingGuaYan 764）：
    /// 开局 3 层；每次正向增加卦象时额外增加 1 点并消耗 1 层。
    wu_jing_gua_yan: i64,
    /// 星缘 FateStrategy 402 标记（原版 BuffType XingYuan 765）：开局
    /// otherParams[0]=3；每次在星位执行牌时消耗 1 层，并使对手获得 1 层内伤。
    xing_yuan: i64,
    fate_cycle: i64,
    fate_cycle_slots: [i64; 2],
    plum_blossom_twice: i64,
    qi_xing_lian_zhu: i64,
    yellow_bird_behind: i64,
    reverse_card_direction: i64,
    sword_formation_guard: i64,
    next_rear_move_bypass: i64,
    used_rear_move_check: i64,
    rear_move_succeeded: bool,
    exorcism: i64,
    heavenly_secret_reverse: i64,
    vitality_bloom: i64,
    first_strike: i64,
    mirage_vitality_bloom: i64,
    mirage_vitality_bloom_heal: i64,
    fortune_seek_auspicious: i64,
    fortune_seek_auspicious_damage: i64,
    ice_snow_lotus: i64,
    leaf_shield_flower: i64,
    sheng_qi_ling_ren: i64,
    wild_ferry_seal: i64,
    yan_qi: i64,
    feng_ling_zhan_yi: i64,
    chan_xin_ju_ling_triggered: i64,
    hot_blood_to_qi_triggered: i64,
    vermilion_bird_tear: i64,
    resonance_mystic_heart_enter_profound: i64,
    instant_shadow_strike: i64,
    /// 惊雷破敌 FateStrategy 396：开局一层 KeYinJingLei(574)；每张原版
    /// 雷牌出牌前消耗一层，给本次攻击碎防并给对手等量外伤。
    ke_yin_jing_lei: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayChanceState {
    cannot_revive: i64,
    po_kong_diao: i64,
    an_xing_bian_fu: i64,
    di_xuan_gui: i64,
    jin_mao_shu: i64,
    qi_cai_ling_he: i64,
    tun_tian_chi_yan_shou: i64,
    shi_xu_ling_shou: i64,
    pang_xian_li: i64,
    ying_xiao_tu: i64,
    you_ming_xu_hun_quan: i64,
    san_wei_huan: i64,
    huan_yu_ying_copy_guard: i64,
}

/// Exact BuffType-backed storage for the dream and mirage runtime.
///
/// `LastTurnStartHp` and `CannotGainHp` deliberately remain in
/// `ReplayMirageRonghuiState`: both waves use the same original BuffType, so the
/// resource and turn pipelines must observe one canonical value rather than
/// duplicate ledgers that can drift or trigger twice.
#[derive(Debug, Clone, Default)]
struct ReplayDreamMirageState {
    dream_unmoving_formation: i64,
    dream_dance_countdown: i64,
    dream_flying_cloud_pill: i64,
    dream_great_return_pill: i64,
    dream_tune_immunity: i64,
    dream_extra_action_lock: i64,
    half_anima_gain: i64,
    cannot_gain_defense: i64,
    calamity_skip_mask: i64,
    total_anima_gained: i64,
    cloud_sword_used_count: i64,
    sword_used_count: i64,
    formation_used_count: i64,
    anima_gain_defense: i64,
    sword_intent_gain_defense: i64,
    turn_start_defense: i64,
    cloud_sea_on_formation: i64,
    sword_energy_on_sword: i64,
    double_next_sword_intent_and_attack_bonus: i64,
    healing_turn_end_frenzy: i64,
    rear_move_card_used_count: i64,
    dream_reflection: i64,
    dream_star_board: i64,
    dream_star_board_low_realm: i64,
    snake_shadow: i64,
    snake_card_used_count: i64,
    withered_tree_used_count: i64,
    action_again_sharpness: i64,
    temporary_water_double: i64,
    unconditional_five_elements: i64,
    total_actual_damage: i64,
    attack_bonus_to_thorns: i64,
    lost_max_hp_event_count: i64,
    total_sharpness_gained: i64,
    total_water_momentum_gained: i64,
    dream_cliff: i64,
    five_elements_marrow: i64,
    five_elements_marrow_infinite: i64,
    consume_next_card: i64,
    dream_fire_formation: i64,
    used_five_elements_count: i64,
    dream_mystic_footwork: i64,
    dream_mystic_footwork_high: i64,
    defense_ledger: i64,
    total_momentum_gained: i64,
    flat_momentum_attack: i64,
    momentum_before_every_attack: i64,
    next_hp_cost_refund: i64,
    next_hp_gain_defense: i64,
    hp_gain_defense: i64,
    next_beng_quan_additional_attack: i64,
    next_beng_quan_physique: i64,
    dream_forge_fist: i64,
    defense_gain_damage_low: i64,
    dream_defense_gain_damage: i64,
    flowing_merciless: i64,
    star_shift: i64,
    star_shift_attack: i64,
    repeat_next_fire_or_earth: i64,
    extra_water_momentum_turn_end: i64,
    return_sharpness: i64,
    excess_physique_hp: i64,
    excess_physique_damage: i64,
    spirit_cat_cloud: i64,
    dragon_extra_action_immunity: i64,

    // Runtime ledgers for the current-build full-scope candidates. They stay
    // separate from verified card state until original-oracle admission.
    hp_gain_event_count: i64,
    defense_gain_event_count: i64,
    sharpness_gain_event_count: i64,

    // Runtime-only ledgers/recursion guards from BattleCharacter hooks.
    dream_star_board_triggered: i64,
    temporary_water_ledger: i64,
    temporary_anima_ledger: i64,
    dream_forge_fist_consumed: i64,
    dream_defense_gain_damage_guard: i64,
    turn_hp_gained: i64,
    dream_mystic_footwork_blocked: i64,
    dream_mystic_footwork_triggered: i64,
    next_beng_quan_additional_attack_triggered: i64,
    action_again_limit: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayMirageRonghuiState {
    mirage_anima_attack_cards: i64,
    mirage_internal_injury_amplifier_turns: i64,
    mirage_sword_intent_refund: i64,
    mirage_sharpness_conversion_turns: i64,
    mirage_healing_conversion_turns: i64,
    mirage_water_defense_cap: i64,
    internal_injury_extra_triggers: i64,
    ordinary_sword_action_again_cards: i64,
    infinity_plate: i64,
    six_yao_fan_damage: i64,
    counter_element_anima: i64,
    counter_element_defense: i64,
    molten_ring: i64,
    first_hp_loss_reward: i64,
    first_hp_loss_reward_triggered: i64,
    crash_fist_star_seize: i64,
    crash_fist_star_seize_consumed: i64,
    bilateral_turn_end_growth: i64,
    bilateral_turn_end_loss: i64,
    hp_loss_attack_bonus_charges: i64,
    last_turn_start_hp: i64,
    this_turn_start_hp: i64,
    nine_heavens_revive: i64,
    double_hp_at_turn_start: i64,
    temporary_copy_depth: i64,
    action_again_ignores_binding: i64,
    five_elements_cards_used: i64,
    cannot_gain_hp: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayRonghuiState {
    five_emperors_upgrade: i64,
    alchemy_pot: i64,
    earth_fiend_defense: i64,
    spirit_sparrow_behind: i64,
    yellow_bird_cost_reduction: i64,
    thunder_tune: i64,
    two_polarity_vajra: i64,
    two_polarity_anima_multiplier: i64,
    two_polarity_hexagram_multiplier: i64,
    star_chess_jump: i64,
    all_cards_action_again: i64,
    momentum_formation: i64,
    free_and_easy_tune: i64,
    reverse_gu_attack: i64,
    snow_lotus_mirror: i64,
    fu_xi_copy_guard: i64,
    dong_huang_copy_guard: i64,
    five_elements_dream_copy_guard: i64,
    five_elements_bottle_card_id: i64,
}

#[derive(Debug, Clone, Default)]
struct ReplayTurnState {
    lost_defense_count: i64,
    turn_attack_segments: i64,
    next_turn_defense: i64,
    spirit_control_anima_gain_defense: i64,
    spirit_control_anima_loss_defense: i64,
    attack_applies_internal_injury_turns: i64,
    wood_spring_turns: i64,
    blood_shadow: i64,
    hp_cost_cards_used: i64,
    used_card_count: i64,
    adaptation: i64,
    ignore_defense_attacks: i64,
    ignore_weakness_attacks: i64,
    next_attack_shatter_defense: i64,
    attack_segments_performed: i64,
    extra_actions: i64,
    agility: i64,
    action_again_count: i64,
    wind_spirit_body_forge_count: i64,
    battle_physique_gain_count: i64,
    lose_hp_count: i64,
    lose_hp_times_count: i64,
    // ActualDamage(302)/WoundedCount(303) 持久计数：原版在攻击者身上跨卡、
    // 跨回合累计（BattleCharacter.cs:10858-10861 累加；CardActionBase.cs:
    // 4743-4745 该攻击者自己出牌完成时转入 JiLuZongJiShangZhi(644) 并清零
    // 302/303）。回合末攻击（fate 137 凝水化刃）等无 invocation 帧的路径
    // 也计入，玫刺(7000027) 等卡在自身攻击后读到的值 = 残留 + 本卡。
    actual_damage_carry: i64,
    wounded_count_carry: i64,
    dan_ka_gong_ji_ji_shu: i64,
    ji_lu_zong_ji_shang_zhi: i64,
    spirit_turtle_footwork: i64,
    spirit_turtle_footwork_triggered: i64,
    next_card_action_again: i64,
    current_turn_ignore_defense: i64,
    agility_gain_damage: i64,
    anima_gain_count: i64,
    next_attack_bonus: i64,
    next_attack_wound_bonus: i64,
    guaranteed_wound: i64,
    next_card_anima_cost_reduction: i64,
    jump_to_previous_card: i64,
    last_round_used_card_base_ids: Vec<i64>,
}

#[derive(Debug, Clone, Default)]
struct ReplayDeckState {
    slots: Vec<ReplayCardSlot>,
    queue: Vec<usize>,
    active_slot_count: usize,
}

#[derive(Debug, Clone)]
struct DrawnCard {
    source_slot: usize,
    card: CardDefinition,
    fallback_basic_attack: bool,
    skipped_slots: Vec<usize>,
    skipped_opening_slots: Vec<usize>,
    /// BattleExecuter.cs:1857-1864：FS398（老鼠偷油/星弈跳牌）的 while 循环
    /// 跳过第 5 格时先按 otherParams[0] 加血。只有该循环自己跳过第 5 格才
    /// 加血——若第 5 格是被更靠前的星弈断等机制跳掉的，FS398 循环根本不会
    /// 触发（原版 while 链按 星弈断→梦EJie→FS398→… 顺序重入）。
    fate_398_skipped_fifth_grid: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReplayCardExecution {
    occurrence: u64,
    side: PlayerSide,
    card_id: i64,
    percent_roll_ordinal: i64,
    random_range_ordinal: i64,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
enum ReplayObservationMode {
    #[default]
    None,
    Events,
    Parity,
    Ui,
    Detailed,
}

impl ReplayObservationMode {
    fn emits_events(self) -> bool {
        self != Self::None
    }

    fn is_parity(self) -> bool {
        matches!(self, Self::Parity | Self::Ui)
    }

    fn is_ui(self) -> bool {
        self == Self::Ui
    }

    fn is_detailed(self) -> bool {
        self == Self::Detailed
    }
}

#[derive(Debug, Clone, Default)]
struct ReplayObservationRuntime {
    prevention: Vec<ReplayPreventionPair>,
    mode: ReplayObservationMode,
    events: Vec<ReplayEvent>,
    ui_events: Vec<ReplayUiEvent>,
    detailed_events: Vec<ReplayDetailedEvent>,
    detailed_steps: Vec<ReplayDetailedStep>,
    /// Per-hit attack segments sampled inside `attack_by_config`'s loop in
    /// Detailed mode only. Each segment records the target's hp/def before and
    /// after one hit, so the browser can show per-hit damage the way the
    /// original client's `TmpFloatingText` does (百杀 4 段 8 攻 → 4 个独立数字)
    /// instead of a single net diff. Observation-only: sampling never changes
    /// the battle, so `winner / actorTurn / hpDelta` are unaffected.
    attack_segments: Vec<ReplayAttackSegment>,
    /// Detailed-only, observation-only receipts for stable OnTurnEnded phases.
    turn_end_hooks: Vec<ReplayTurnEndHookReceipt>,
    /// Detailed-only, observation-only mirror of the typed mutation receipts,
    /// stamped with the attribution context active at the mutation site.
    mutation_receipts: Vec<ReplayMutationReceipt>,
    /// The event index of the card currently being executed, set before the
    /// card effect body runs and cleared after. Attack segments use this to
    /// join onto the matching MainEffect hook step by event_index rather than
    /// by turn (a turn can have multiple cards via 再动).
    current_card_event_index: Option<usize>,
    /// 0-based counter for attack segments within the current card effect.
    /// Reset when the effect body starts; every sampled hit takes the current
    /// value and increments, so a card that attacks several times (循环多段、
    /// 追加攻击、百杀概率段) gets one continuous numbering (第 1 段…第 N 段).
    current_attack_segment_index: usize,
}

#[derive(Debug)]
struct ExecutedReplay {
    state: ReplayState,
    summary: ReplaySummary,
}

/// Keeps analysis-only starting perturbations out of every exact-comparison path.
///
/// `Events` is the observation mode the solver rule-impact / lambda pipeline uses,
/// and it is the only caller allowed to perturb the opening state. Parity feeds the
/// golden `winner / actorTurn / hpDelta` comparison, `Ui` feeds the browser's
/// parity-shaped timeline, `None` feeds the summary comparison and `Detailed`
/// feeds the inspector/TUI surfaces: a perturbed run reaching any of them would
/// silently rewrite the battle it claims to reproduce.
fn reject_perturbations_outside_analysis(
    fixture: &BattleFixture,
    observation_mode: ReplayObservationMode,
) -> std::result::Result<(), BattleError> {
    if observation_mode == ReplayObservationMode::Events {
        return Ok(());
    }
    let Some(source) = &fixture.source else {
        return Ok(());
    };
    if let Some(perturbation) = source.solver_starting_perturbations.first() {
        return Err(BattleError::Invariant {
            message: format!(
                "solver starting perturbations are analysis-only and cannot run under {observation_mode:?} observation: {} {}",
                format!("{:?}", perturbation.side).to_lowercase(),
                perturbation.field
            ),
        });
    }
    Ok(())
}

fn execute_replay_fixture(
    fixture: &BattleFixture,
    observation_mode: ReplayObservationMode,
) -> std::result::Result<ExecutedReplay, BattleError> {
    reject_perturbations_outside_analysis(fixture, observation_mode)?;
    // Do not expose a partially initialized state: an opening error must be
    // returned before a BattleStart event or the first actor turn can run.
    // `from_fixture_with_mode` opens Detailed observation before the opening
    // (receipt contract) and defers every other mode's observation to here.
    let mut state = ReplayState::from_fixture_with_mode(fixture, true, observation_mode)?;
    state.observation.mode = observation_mode;
    state.record_event(
        ReplayEventKind::BattleStart,
        fixture.first_player_side,
        None,
        None,
    );
    state.record_detail_step(
        state.observation.events.len().saturating_sub(1),
        ReplayHookCategory::BattleStart,
        fixture.first_player_side,
        None,
        None,
    );
    let summary = state.run();
    if let Some(error) = state.evaluation_error.take() {
        return Err(error);
    }
    state.record_event(ReplayEventKind::BattleEnd, summary.winner_side, None, None);
    state.record_detail_step(
        state.observation.events.len().saturating_sub(1),
        ReplayHookCategory::BattleEnd,
        summary.winner_side,
        None,
        None,
    );
    Ok(ExecutedReplay { state, summary })
}

pub fn run_replay_fixture_file(path: impl AsRef<Path>) -> Result<ReplaySummary> {
    let fixture = load_fixture_file(path)?;
    run_replay_fixture(&fixture)
}

pub fn run_replay_fixture(fixture: &BattleFixture) -> Result<ReplaySummary> {
    execute_replay_fixture(fixture, ReplayObservationMode::None)
        .map(|execution| execution.summary)
        .map_err(EngineError::Battle)
}

pub fn run_replay_fixture_with_events(fixture: &BattleFixture) -> Result<ReplayRun> {
    run_replay_fixture_with_observation(fixture, ReplayObservationMode::Events)
}

pub fn run_replay_fixture_with_parity_events(fixture: &BattleFixture) -> Result<ReplayRun> {
    run_replay_fixture_with_observation(fixture, ReplayObservationMode::Parity)
}

pub fn run_replay_fixture_with_ui_events(fixture: &BattleFixture) -> Result<ReplayUiRun> {
    let execution =
        execute_replay_fixture(fixture, ReplayObservationMode::Ui).map_err(EngineError::Battle)?;
    let summary = execution.summary;
    let state = execution.state;
    let termination_cause = state.termination_cause.ok_or_else(|| {
        EngineError::Battle(BattleError::Invariant {
            message: "replay has no termination cause".into(),
        })
    })?;
    if state.observation.ui_events.len() != state.observation.events.len() {
        return Err(EngineError::Battle(BattleError::Invariant {
            message: format!(
                "UI telemetry is misaligned: {} entries for {} events",
                state.observation.ui_events.len(),
                state.observation.events.len()
            ),
        }));
    }
    Ok(ReplayUiRun {
        summary,
        termination_cause,
        completed_checkpoint_count: state.completed_checkpoint_count,
        events: state.observation.ui_events,
        decision_events: state.decision_events,
    })
}

fn run_replay_fixture_with_observation(
    fixture: &BattleFixture,
    observation_mode: ReplayObservationMode,
) -> Result<ReplayRun> {
    let execution =
        execute_replay_fixture(fixture, observation_mode).map_err(EngineError::Battle)?;
    let summary = execution.summary;
    let state = execution.state;
    let termination_cause = state.termination_cause.ok_or_else(|| {
        EngineError::Battle(BattleError::Invariant {
            message: format!(
                "replay has no termination cause: actor_turn={} max_actor_turns={} p1_hp={} p2_hp={} winner={:?}",
                state.actor_turn,
                state.max_actor_turns,
                state.p1.core.hp,
                state.p2.core.hp,
                summary.winner_side,
            ),
        })
    })?;
    let events = state.observation.events;
    let prevention = state.observation.prevention;
    if prevention.len() != events.len() {
        return Err(EngineError::Battle(BattleError::Invariant {
            message: format!(
                "prevention telemetry is misaligned: {} entries for {} events",
                prevention.len(),
                events.len()
            ),
        }));
    }
    Ok(ReplayRun {
        summary,
        termination_cause,
        completed_checkpoint_count: state.completed_checkpoint_count,
        events,
        prevention,
        decision_events: state.decision_events,
    })
}

pub fn run_replay_fixture_with_detailed_events(
    fixture: &BattleFixture,
) -> Result<ReplayDetailedRun> {
    let execution = execute_replay_fixture(fixture, ReplayObservationMode::Detailed)
        .map_err(EngineError::Battle)?;
    let summary = execution.summary;
    let state = execution.state;
    Ok(ReplayDetailedRun {
        summary,
        events: state.observation.detailed_events,
        steps: state.observation.detailed_steps,
        decision_events: state.decision_events,
        attack_segments: state.observation.attack_segments,
        turn_end_hooks: state.observation.turn_end_hooks,
        mutation_receipts: state.observation.mutation_receipts,
    })
}

pub fn evaluate_replay_fixture_fallible(
    fixture: &BattleFixture,
) -> std::result::Result<ReplayEvaluationRun, String> {
    let execution = execute_replay_fixture(fixture, ReplayObservationMode::None)
        .map_err(|error| error.to_string())?;
    let summary = execution.summary;
    let state = execution.state;
    Ok(ReplayEvaluationRun {
        summary,
        p1: state.p1.snapshot(),
        p2: state.p2.snapshot(),
        decision_events: state.decision_events,
    })
}

pub fn run_replay_fixture_with_events_fallible(
    fixture: &BattleFixture,
) -> std::result::Result<ReplayRun, String> {
    run_replay_fixture_with_events(fixture).map_err(|error| error.to_string())
}

pub fn original_card_definition_by_id(card_id: i64) -> Option<CardDefinition> {
    original_config::original_card_definition(card_id)
}

/// Semantic result of one atomic HP mutation.
///
/// `requested` is the caller's input, `resolved` is the accepted delta after
/// rule rewrites but before the HP cap, and `applied` is the visible HP
/// change. `ledger` is the signed amount projected into HP-gain/loss ledgers:
/// accepted healing records its resolved amount (including overheal), while
/// HP loss records only the amount actually applied.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct HpMutationReceipt {
    requested: i64,
    resolved: i64,
    applied: i64,
    ledger: i64,
    prevention: Option<HpMutationPrevention>,
}

impl HpMutationReceipt {
    fn prevented(requested: i64) -> Self {
        Self {
            requested,
            ..Self::default()
        }
    }

    fn prevented_by(requested: i64, prevention: HpMutationPrevention) -> Self {
        Self {
            requested,
            prevention: Some(prevention),
            ..Self::default()
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum HpMutationPrevention {
    Guard,
}

/// Semantic result of one original ModifyBuffValue(QiShi) call.
///
/// `hook_delta` is the lower-clamped delta observed by ordinary momentum
/// hooks. `visible_delta` is the final panel change after the later
/// upper-limit Set and can therefore differ from `hook_delta`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MomentumMutationReceipt {
    requested_delta: i64,
    hook_delta: i64,
    visible_delta: i64,
    overflow_delta: i64,
    before: i64,
    after: i64,
}

/// Semantic result of one atomic negative-status mutation (original
/// ModifyBuffValue on a Negative-class buff). `requested` is the signed
/// caller delta, `applied` the signed stacks actually changed after
/// mitigation (206 stance, 辟邪), `before`/`after` the stack counts.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct NegativeStatusMutationReceipt {
    status: i64,
    requested: i64,
    applied: i64,
    before: i64,
    after: i64,
}

/// Semantic result of one defense (ModifyDef) mutation. `applied` is the
/// amount this mutation added/removed; `visible_delta` is the final panel
/// change, which can differ when gain-damage or 八荒 earth re-gain runs.
/// Unlike the signed deltas of the other receipt families, `requested` and
/// `applied` are positive magnitudes — the direction lives in
/// `visible_delta`/`before`/`after`.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct DefenseMutationReceipt {
    requested: i64,
    applied: i64,
    visible_delta: i64,
    before: i64,
    after: i64,
}

/// Semantic result of one atomic max-HP (ModifyMaxHp) mutation.
/// `resolved` is the post-modifier delta before the zero clamp, `applied`
/// the accepted max-HP change.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct MaxHpMutationReceipt {
    requested: i64,
    resolved: i64,
    applied: i64,
    before: i64,
    after: i64,
}

/// Which original revive path produced a `ReviveReceipt`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ReviveKind {
    FlameSoulReturn,
    FirePhoenix,
    NineHeavens,
    QiXingJieMing,
}

/// Semantic result of one completed revive body: the vitals the character
/// ends with after the revive's own max-HP/HP replacement or healing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ReviveReceipt {
    kind: ReviveKind,
    hp_after: i64,
    max_hp_after: i64,
}

#[derive(Debug, Clone)]
struct ReplayState {
    p1: ReplayPlayer,
    p2: ReplayPlayer,
    first_player: PlayerSide,
    current_actor: PlayerSide,
    original_build_profile: original_build_profile::OriginalBuildProfile,
    actor_turn: i64,
    max_actor_turns: i64,
    decision_tape: Vec<i64>,
    random_fallback_tape: Vec<i64>,
    synthetic_decision_seed: Option<u32>,
    synthetic_decision_sides: Vec<PlayerSide>,
    synthetic_decision_fallback_seed: Option<u32>,
    decision_occurrence: u64,
    card_execution_occurrence: u64,
    current_card_execution: Option<ReplayCardExecution>,
    decision_events: Vec<ReplayDecisionEvent>,
    effect_invocation_stack: Vec<effect_invocation::EffectInvocationFrame>,
    /// Which explicit phase table is currently running, when no effect
    /// invocation is active. Receipts recorded inside a phase block are
    /// attributed to the matching phase step (BattleStart/TurnStart/TurnEnd).
    attribution_block: Option<TraceAttributionBlock>,
    fail_on_missing_decision: bool,
    evaluation_error: Option<BattleError>,
    observation: ReplayObservationRuntime,
    termination_cause: Option<ReplayTerminationCause>,
    completed_checkpoint_count: usize,
}

/// One of the explicit phase tables from the PR-1 pipeline, used to attribute
/// mutations that run outside any card effect invocation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum TraceAttributionBlock {
    BattleStart,
    TurnStart,
    ActionAgain,
    TurnEnd,
}

impl ReplayState {
    fn missing_card_effect(&mut self, card_id: i64, base_id: i64, reason: &str) {
        if self.evaluation_error.is_none() {
            self.evaluation_error = Some(BattleError::MissingRule {
                card_id,
                base_id,
                reason: reason.into(),
                turn: self.actor_turn,
            });
        }
    }

    fn missing_decision(&mut self, reason: &str) {
        if self.fail_on_missing_decision && self.evaluation_error.is_none() {
            self.evaluation_error = Some(BattleError::MissingDecision {
                reason: reason.into(),
                turn: self.actor_turn,
            });
        }
    }

    fn original_build_has_capability(
        &self,
        capability: original_build_profile::OriginalBuildCapability,
    ) -> bool {
        self.original_build_profile.has(capability)
    }
}
