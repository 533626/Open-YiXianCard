//! `replay_slice` 对外 JSON 的线协议类型：事件流、扩展事件流与准入报告。
//!
//! 这些结构体是三条线合同的唯一事实来源；`export_contracts` bin 从它们
//! 生成 JSON Schema（入库 `battle-evaluator/generated/contracts/`），
//! Python/JS 消费方只按 Schema 校验，不重声明字段清单。
//! 字段名与可省略性由 serde 属性决定，Schema 随 serde 同源生成。

use super::{
    ReplayAttackSegment, ReplayDecisionEvent, ReplayPlayerSnapshot, ReplayQueuedCard,
    ReplayTerminationCause, ReplayTurnEndHookReceipt,
};
use serde::Serialize;

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionReport {
    pub root: String,
    pub total: usize,
    pub exact: usize,
    pub blocked: usize,
    pub mismatch: usize,
    pub error: usize,
    pub results: Vec<AdmissionFixtureResult>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionFixtureResult {
    pub fixture: String,
    pub result: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_winner: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_turn: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual_delta: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub first_deviation: Option<AdmissionDeviation>,
    pub warnings: Vec<AdmissionWarning>,
    pub missing_cards: Vec<AdmissionMissingCard>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AdmissionWarning {
    pub side: String,
    pub kind: String,
    pub id: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<i64>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct AdmissionMissingCard {
    pub id: i64,
    pub base_id: i64,
    pub name: String,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct AdmissionDeviation {
    pub kind: &'static str,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actual: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub expected: Option<String>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EventStream {
    pub fixture: String,
    pub events: Vec<EventRow>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct EventRow {
    pub kind: String,
    pub turn: i64,
    pub actor: String,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub termination_cause: Option<ReplayTerminationCause>,
    pub completed_checkpoint_count: Option<usize>,
    pub p1: EventSnapshot,
    pub p2: EventSnapshot,
}

/// `--events-extended` 快照：29 字段 parity 协议原样保留，另附
/// detail_entries 里的四个 BuffType 值（298 QiShiShangXian、
/// 10024 TiPoShangXian、351 LoseHpCount、409 LoseHpTimesCount）、
/// parity 快照上的锻玄架势（443/444）与
/// 全量暴露缺口字段（档 1a/1b，见 build_display_gap_report.py 锚定表；
/// 625 JianQi 剑气、264 HuangQueZaiHou 黄雀在后等原版 RefreshBuff 会显示、
/// 左侧状态条此前缺失的 BuffType）。
/// parity 协议本身（EventSnapshot 29 字段）不动，扩展只发生在 CLI 序列化层。
#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExtendedEventSnapshot {
    #[serde(flatten)]
    pub parity: EventSnapshot,
    #[serde(flatten)]
    pub display_gap: DisplayGapSnapshot,
}

/// 显示缺口字段：61 个直接来自 `ReplayPlayerSnapshot`（锻玄架势 + 剑气 +
/// 全量暴露缺口），9 个来自 detail_entries（momentum_limit / physique_limit /
/// lose_hp_count / lose_hp_times_count / actual_damage_carry / wounded_count_carry /
/// ji_lu_zong_ji_shang_zhi / last_stand_intent / last_stand_unyielding，
/// 原版 BuffType 298/10024/351/409/302/303/644/17/18）。
/// 字段名与 `RUST_EXTENDED_SNAPSHOT_FIELDS`
/// （battle-evaluator/scripts/lib/original-oracle-rust-buff-map.ts）一一对应。
/// `define_gap_snapshot!` 一次声明结构体与 `from_parity` 拷贝，
/// 用 `#[serde(flatten)]` 平铺进 `ExtendedEventSnapshot`，避免逐字段手抄。
macro_rules! define_gap_snapshot {
    ([$($field:ident),* $(,)?]) => {
        #[derive(Debug, Default, Serialize, schemars::JsonSchema)]
        pub struct DisplayGapSnapshot {
            pub momentum_limit: i64,
            pub physique_limit: i64,
            pub lose_hp_count: i64,
            pub lose_hp_times_count: i64,
            pub actual_damage_carry: i64,
            pub wounded_count_carry: i64,
            pub ji_lu_zong_ji_shang_zhi: i64,
            pub last_stand_intent: i64,
            pub last_stand_unyielding: i64,
            /// 原版 battleTempData.actionAgainPerRound（Rust 梦境再动上限）。
            pub action_again_per_round: i64,
            /// 已解锁格数（原版 unlockGrids）。
            pub active_slot_count: i64,
            $(pub $field: i64),*
        }
        impl DisplayGapSnapshot {
            /// 从 `ReplayPlayerSnapshot` 取显示缺口字段；
            /// `momentum_limit` / `physique_limit` 等 9 个来自 detail_entries，由调用方填充。
            pub fn from_parity(parity: &ReplayPlayerSnapshot) -> Self {
                let mut out = Self::default();
                $(out.$field = parity.$field;)*
                out
            }
        }
    };
}

define_gap_snapshot!([
    sword_energy,
    quan_stance,
    gun_stance,
    metal_ring,
    water_month_sword_formation,
    water_formation,
    metal_formation,
    earth_formation,
    fire_formation,
    spring_flow,
    water_stealth,
    metal_iron_bone,
    earth_eight_wastes,
    wood_array,
    wood_scattered_branches,
    zhu_chen_bu,
    xing_yi_lian,
    turtle_formation,
    shatter_formation,
    thunder_formation,
    evil_gu_formation,
    spirit_gathering_formation,
    heaven_cycle_sword_formation,
    heaven_force_formation,
    flower_maze_formation,
    immovable_formation,
    eight_gates_formation,
    six_yao_formation,
    beng_quan_cun_jin,
    beng_quan_return_profound,
    dream_beng_quan_chain,
    immortal_binding_tune,
    illusory_tune,
    heartbreak_tune,
    wild_dance_tune,
    rejuvenation_tune,
    xiaoyao_tune,
    xiaoyao_guqin,
    chaotic_mind_tune,
    ling_gua_art,
    star_moon_fan,
    infinite_hexagram_plate,
    all_goes_well,
    ling_yuan_wai_fang,
    recovery,
    meditation,
    blood_calamity,
    lone_night_wolf,
    leaf_blade_flower,
    quiet_mindset,
    reflect_mindset,
    graft_flowers_to_tree,
    tide,
    dismantle_move,
    all_things_inauspicious,
    fate_cycle,
    yellow_bird_behind,
    exorcism,
    ice_snow_lotus,
    leaf_shield_flower,
    paint_finishing_touch,
    next_turn_defense,
    ignore_defense_attacks,
    next_attack_shatter_defense,
]);

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExtendedEventRow {
    pub kind: String,
    pub turn: i64,
    pub actor: String,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1: ExtendedEventSnapshot,
    pub p2: ExtendedEventSnapshot,
    /// 剩余出牌队列（原版 oracle trace 的 cardQueue）。
    pub p1_card_queue: Vec<ReplayQueuedCard>,
    pub p2_card_queue: Vec<ReplayQueuedCard>,
    /// 已解锁格位（原版 oracle trace 的 activeDeck）。
    pub p1_physical_deck: Vec<ReplayQueuedCard>,
    pub p2_physical_deck: Vec<ReplayQueuedCard>,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtendedTerminal {
    pub winner_side: &'static str,
    pub actor_turn: i64,
    pub hp_delta: i64,
    pub p1_hp: i64,
    pub p2_hp: i64,
    pub remaining_decision_params: usize,
}

#[derive(Debug, Serialize, schemars::JsonSchema)]
pub struct ExtendedEventStream {
    pub fixture: String,
    pub terminal: ExtendedTerminal,
    pub events: Vec<ExtendedEventRow>,
    #[serde(rename = "decisionEvents")]
    pub decision_events: Vec<ReplayDecisionEvent>,
    #[serde(rename = "attackSegments")]
    pub attack_segments: Vec<ReplayAttackSegment>,
    #[serde(rename = "turnEndHooks")]
    pub turn_end_hooks: Vec<ReplayTurnEndHookReceipt>,
}

/// CLI 序列化层快照：字段名与 `ReplayPlayerSnapshot` 一一对应，
/// 由 `define_snapshot!` 一次声明结构体与拷贝函数，避免手抄漂移。
macro_rules! define_snapshot {
    ($name:ident, $from_fn:ident, [$($field:ident),* $(,)?]) => {
        #[derive(Debug, Serialize, schemars::JsonSchema)]
        pub struct $name {
            $(pub $field: i64),*
        }
        pub fn $from_fn(snapshot: &ReplayPlayerSnapshot) -> $name {
            $name {
                $($field: snapshot.$field),*
            }
        }
    };
}

define_snapshot!(
    EventSnapshot,
    event_snapshot,
    [
        hp,
        max_hp,
        defense,
        anima,
        guard,
        physique,
        sword_intent,
        sharpness,
        cloud_chain,
        cloud_sea,
        momentum,
        agility,
        water_momentum,
        activated_metal,
        activated_water,
        activated_wood,
        activated_fire,
        activated_earth,
        hexagram,
        star_power,
        attack_bonus,
        internal_injury,
        weakness,
        flaw,
        attack_reduction,
        entangle,
        external_injury,
        lost_mind,
        action_again_count,
    ]
);
