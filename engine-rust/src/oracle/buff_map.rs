//! `battle-evaluator/scripts/lib/original-oracle-rust-buff-map.ts` 的 Rust 逐字移植。
//!
//! 映射表（battle-evaluator/generated/buff-id-rust-key-map.json）由
//! research/original-game/build_buff_id_rust_key_map.py 生成，每条目带完整锚定链：
//! BuffType ID ↔ archive 枚举名 ↔ 原文名（buff-category-archive.json）
//! ↔ Rust detail_entries key/label（snapshot.rs）↔ --events-extended 快照字段。
//!
//! 本模块是对照层访问映射的唯一入口：裸键兜底必须能解析到锚定，不能绕过。
//! 映射外 BuffType ID 只能走 [`unmapped_original_buff_entries`] 显式归类（引擎
//! 覆盖缺口，不是静默跳过）；映射表漂移（id/原文名与 archive 不符）由
//! [`validate_rust_buff_key_map`] 在加载时抛错。逐帧对照（[`compare_side`]
//! 四分类）也落在这里，供 bin 与单测共用。

use serde::de::{Deserializer, MapAccess, Visitor};
use serde::ser::SerializeMap;
use serde::{Deserialize, Serialize, Serializer};
use serde_json::{Map, Value};
use crate::{ReplayDetailEntry, ReplayDetailedSideState, ReplayPlayerSnapshot};
use crate::replay::wire::{event_snapshot, DisplayGapSnapshot, EventSnapshot};

/// 对照层错误：文案与 TS 抛错逐字一致。
pub type OracleResult<T> = Result<T, String>;

/// `--events-extended` 单侧快照的全部语义字段，顺序与
/// `RUST_EXTENDED_SNAPSHOT_FIELDS`（original-oracle-rust-buff-map.ts）逐字一致。
/// 前 29 个是 parity 协议字段；其余为扩展暴露缺口字段（wire.rs
/// `DisplayGapSnapshot` 一一对应）。
pub const RUST_EXTENDED_SNAPSHOT_FIELDS: [&str; 104] = [
    "hp",
    "max_hp",
    "defense",
    "anima",
    "guard",
    "physique",
    "sword_intent",
    "sword_energy",
    "sharpness",
    "cloud_chain",
    "cloud_sea",
    "momentum",
    "agility",
    "water_momentum",
    "activated_metal",
    "activated_water",
    "activated_wood",
    "activated_fire",
    "activated_earth",
    "hexagram",
    "star_power",
    "attack_bonus",
    "internal_injury",
    "weakness",
    "flaw",
    "attack_reduction",
    "entangle",
    "external_injury",
    "lost_mind",
    "action_again_count",
    "momentum_limit",
    "physique_limit",
    "quan_stance",
    "gun_stance",
    "metal_ring",
    "water_month_sword_formation",
    "water_formation",
    "metal_formation",
    "earth_formation",
    "fire_formation",
    "spring_flow",
    "water_stealth",
    "metal_iron_bone",
    "earth_eight_wastes",
    "wood_array",
    "wood_scattered_branches",
    "zhu_chen_bu",
    "xing_yi_lian",
    "turtle_formation",
    "shatter_formation",
    "thunder_formation",
    "evil_gu_formation",
    "spirit_gathering_formation",
    "heaven_cycle_sword_formation",
    "heaven_force_formation",
    "flower_maze_formation",
    "immovable_formation",
    "eight_gates_formation",
    "six_yao_formation",
    "beng_quan_cun_jin",
    "beng_quan_return_profound",
    "dream_beng_quan_chain",
    "immortal_binding_tune",
    "illusory_tune",
    "heartbreak_tune",
    "wild_dance_tune",
    "rejuvenation_tune",
    "xiaoyao_tune",
    "xiaoyao_guqin",
    "chaotic_mind_tune",
    "ling_gua_art",
    "star_moon_fan",
    "infinite_hexagram_plate",
    "all_goes_well",
    "ling_yuan_wai_fang",
    "recovery",
    "meditation",
    "blood_calamity",
    "lone_night_wolf",
    "leaf_blade_flower",
    "quiet_mindset",
    "reflect_mindset",
    "graft_flowers_to_tree",
    "tide",
    "dismantle_move",
    "all_things_inauspicious",
    "fate_cycle",
    "yellow_bird_behind",
    "exorcism",
    "ice_snow_lotus",
    "leaf_shield_flower",
    "paint_finishing_touch",
    "next_turn_defense",
    "ignore_defense_attacks",
    "next_attack_shatter_defense",
    "lose_hp_count",
    "lose_hp_times_count",
    "actual_damage_carry",
    "wounded_count_carry",
    "ji_lu_zong_ji_shang_zhi",
    "last_stand_intent",
    "last_stand_unyielding",
    // 原版 battleTempData.actionAgainPerRound（Rust 梦境再动上限）。
    "action_again_per_round",
    // 已解锁格数（原版 unlockGrids）。
    "active_slot_count",
];

/// 旁侧：与 TS `"p1" | "p2"` 同形。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Side {
    P1,
    P2,
}

impl Side {
    pub fn as_str(self) -> &'static str {
        match self {
            Side::P1 => "p1",
            Side::P2 => "p2",
        }
    }

    pub fn parse(value: &str) -> Option<Side> {
        match value {
            "p1" => Some(Side::P1),
            "p2" => Some(Side::P2),
            _ => None,
        }
    }
}

impl Serialize for Side {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for Side {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = String::deserialize(deserializer)?;
        Side::parse(&value)
            .ok_or_else(|| serde::de::Error::custom(format!("side must be p1/p2, got {value}")))
    }
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RustBuffMapEntry {
    pub buff_type_id: i64,
    pub archive_enum: String,
    pub original_name: Option<String>,
    pub category: String,
    pub rust_key: String,
    pub rust_label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustBuffKeyMap {
    pub schema_version: u8,
    pub count: usize,
    pub entries: Vec<RustBuffMapEntry>,
}

impl RustBuffMapEntry {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("buffTypeId".into(), json_i64(self.buff_type_id));
        map.insert("archiveEnum".into(), Value::String(self.archive_enum.clone()));
        map.insert(
            "originalName".into(),
            match &self.original_name {
                Some(name) => Value::String(name.clone()),
                None => Value::Null,
            },
        );
        map.insert("category".into(), Value::String(self.category.clone()));
        map.insert("rustKey".into(), Value::String(self.rust_key.clone()));
        map.insert("rustLabel".into(), Value::String(self.rust_label.clone()));
        Value::Object(map)
    }
}

impl RustBuffKeyMap {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("schemaVersion".into(), Value::from(self.schema_version));
        map.insert("count".into(), Value::from(self.count));
        map.insert(
            "entries".into(),
            Value::Array(self.entries.iter().map(RustBuffMapEntry::to_json).collect()),
        );
        Value::Object(map)
    }

    /// BuffType ID → rustKey 索引（full-state 投影用）。
    pub fn rust_key_by_id(&self, buff_type_id: i64) -> Option<&str> {
        self.entries
            .iter()
            .find(|entry| entry.buff_type_id == buff_type_id)
            .map(|entry| entry.rust_key.as_str())
    }
}

/// 加载并重新锚定映射表；任何漂移都 fail closed。
pub fn load_rust_buff_key_map(map_path: &std::path::Path, archive_path: &std::path::Path) -> OracleResult<RustBuffKeyMap> {
    let map_text = std::fs::read_to_string(map_path)
        .map_err(|error| format!("buff-id-rust-key-map read failed: {error}"))?;
    let archive_text = std::fs::read_to_string(archive_path)
        .map_err(|error| format!("buff-category-archive read failed: {error}"))?;
    let map: Value = serde_json::from_str(&map_text)
        .map_err(|error| format!("buff-id-rust-key-map JSON error: {error}"))?;
    let archive: Value = serde_json::from_str(&archive_text)
        .map_err(|error| format!("buff-category-archive JSON error: {error}"))?;
    validate_rust_buff_key_map(&map, &archive)
}

/// 重新锚定映射表：每个 entry 的枚举名必须在 archive 中存在，且 id/原文名
/// 逐字一致、rustKey 必须是 extended 快照字段、buffTypeId 不得重复。
/// 任何漂移都 fail closed（返回 Err）。导出供单测注入漂移用例。
pub fn validate_rust_buff_key_map(value: &Value, archive_value: &Value) -> OracleResult<RustBuffKeyMap> {
    let map = assert_object(value, "buff-id-rust-key-map")?;
    if map.get("schemaVersion").and_then(Value::as_i64) != Some(1) {
        return Err("buff-id-rust-key-map has an unsupported schema".into());
    }
    let archive = assert_object(archive_value, "buff-category-archive")?;
    let archive_buffs = assert_object(
        archive
            .get("buffs")
            .ok_or_else(|| "buff-category-archive.buffs must be an object".to_string())?,
        "buff-category-archive.buffs",
    )?;
    let entries = map
        .get("entries")
        .and_then(Value::as_array)
        .ok_or_else(|| "buff-id-rust-key-map.entries must be an array".to_string())?;
    let count_ok = map
        .get("count")
        .and_then(Value::as_u64)
        .map(|count| count as usize == entries.len())
        .unwrap_or(false);
    if !count_ok {
        return Err("buff-id-rust-key-map.count is inconsistent".into());
    }
    let mut seen: Vec<i64> = Vec::new();
    let mut parsed = Vec::with_capacity(entries.len());
    for (index, raw) in entries.iter().enumerate() {
        let entry = assert_object(
            raw,
            &format!("buff-id-rust-key-map.entries[{index}]"),
        )?;
        let buff_type_id = json_number(
            entry.get("buffTypeId").unwrap_or(&Value::Null),
            &format!("entries[{index}].buffTypeId"),
        )?;
        let archive_enum = assert_string(
            entry.get("archiveEnum").unwrap_or(&Value::Null),
            &format!("entries[{index}].archiveEnum"),
        )?;
        let rust_key = assert_string(
            entry.get("rustKey").unwrap_or(&Value::Null),
            &format!("entries[{index}].rustKey"),
        )?;
        if seen.contains(&buff_type_id) {
            return Err(format!("entries[{index}]: duplicate buffTypeId {buff_type_id}"));
        }
        seen.push(buff_type_id);
        // 裸键兜底必须能解析到锚定：枚举名在 archive 里存在，且 id/原文名逐字一致。
        let anchored = assert_object(
            archive_buffs
                .get(&archive_enum)
                .unwrap_or(&Value::Null),
            &format!("archive[{archive_enum}]"),
        )?;
        let anchored_id = anchored.get("id").unwrap_or(&Value::Null);
        if anchored_id.as_i64() != Some(buff_type_id) {
            return Err(format!(
                "entries[{index}]: {archive_enum} archive id={} map id={buff_type_id}",
                js_string(anchored_id),
            ));
        }
        let anchored_name = anchored.get("name").unwrap_or(&Value::Null);
        let original_name = match entry.get("originalName") {
            None | Some(Value::Null) => None,
            Some(Value::String(name)) => Some(name.clone()),
            Some(_) => return Err("originalName must be a string".into()),
        };
        let names_equal = match (&original_name, anchored_name) {
            (None, Value::Null) => true,
            (Some(left), Value::String(right)) => left == right,
            _ => false,
        };
        if !names_equal {
            return Err(format!(
                "entries[{index}]: {archive_enum} archive name={} map name={}",
                js_string(anchored_name),
                js_string(
                    &original_name
                        .clone()
                        .map(Value::String)
                        .unwrap_or(Value::Null)
                ),
            ));
        }
        if !RUST_EXTENDED_SNAPSHOT_FIELDS.contains(&rust_key.as_str()) {
            return Err(format!(
                "entries[{index}]: rustKey {rust_key} is not an extended snapshot field"
            ));
        }
        let rust_label = entry.get("rustLabel").unwrap_or(&Value::Null);
        let category = entry.get("category").unwrap_or(&Value::Null);
        if !rust_label.is_string() || !category.is_string() {
            return Err(format!("entries[{index}]: rustLabel/category must be strings"));
        }
        parsed.push(RustBuffMapEntry {
            buff_type_id,
            archive_enum,
            original_name,
            category: category.as_str().unwrap().to_string(),
            rust_key,
            rust_label: rust_label.as_str().unwrap().to_string(),
        });
    }
    Ok(RustBuffKeyMap {
        schema_version: 1,
        count: parsed.len(),
        entries: parsed,
    })
}

/// --events-extended 流的单侧快照：104 个语义字段全部必填，
/// 字段序与 [`RUST_EXTENDED_SNAPSHOT_FIELDS`] 一致。
#[derive(Debug, Clone, PartialEq)]
pub struct RustExtendedSnapshot {
    values: Vec<i64>,
}

impl Default for RustExtendedSnapshot {
    fn default() -> Self {
        Self {
            values: vec![0; RUST_EXTENDED_SNAPSHOT_FIELDS.len()],
        }
    }
}

impl RustExtendedSnapshot {
    pub fn field_index(field: &str) -> Option<usize> {
        RUST_EXTENDED_SNAPSHOT_FIELDS.iter().position(|name| *name == field)
    }

    pub fn get(&self, field: &str) -> Option<i64> {
        Self::field_index(field).map(|index| self.values[index])
    }

    /// 供测试/驱动构造：按字段名设置值；未知字段 panic（常量表漂移即缺陷）。
    pub fn with(field: &str, value: i64) -> Self {
        let mut snapshot = Self::default();
        let index = Self::field_index(field)
            .unwrap_or_else(|| panic!("unknown extended snapshot field {field}"));
        snapshot.values[index] = value;
        snapshot
    }

    pub fn set(&mut self, field: &str, value: i64) {
        let index = Self::field_index(field)
            .unwrap_or_else(|| panic!("unknown extended snapshot field {field}"));
        self.values[index] = value;
    }

    /// 从引擎 Detailed 观测构造（与 replay_slice `extended_event_snapshot` 同语义）。
    pub fn from_engine(
        parity: &ReplayPlayerSnapshot,
        detail_entries: &[ReplayDetailEntry],
        side_state: &ReplayDetailedSideState,
    ) -> Self {
        let es = event_snapshot(parity);
        let mut gap = DisplayGapSnapshot::from_parity(parity);
        // detail_entries 暴露的 9 个持久计数（原版 BuffType
        // 298/10024/351/409/302/303/644/17/18）。
        for (field, key) in [
            (&mut gap.momentum_limit, "momentumLimit"),
            (&mut gap.physique_limit, "physiqueLimit"),
            (&mut gap.lose_hp_count, "loseHpCount"),
            (&mut gap.lose_hp_times_count, "loseHpTimesCount"),
            (&mut gap.actual_damage_carry, "actualDamageCarry"),
            (&mut gap.wounded_count_carry, "woundedCountCarry"),
            (
                &mut gap.ji_lu_zong_ji_shang_zhi,
                "jiLuZongJiShangZhi",
            ),
            (&mut gap.last_stand_intent, "lastStandIntent"),
            (&mut gap.last_stand_unyielding, "lastStandUnyielding"),
        ] {
            *field = detail_entry_value(detail_entries, key);
        }
        gap.action_again_per_round = side_state.action_again_per_round;
        gap.active_slot_count = side_state.active_slot_count as i64;
        let mut snapshot = Self::default();
        for (index, field) in RUST_EXTENDED_SNAPSHOT_FIELDS.iter().enumerate() {
            snapshot.values[index] = snapshot_field_value(field, &es, &gap);
        }
        snapshot
    }

    /// 解析单侧快照 JSON（104 字段全部必填且为有限数）。
    pub fn from_json(value: &Value, label: &str) -> OracleResult<Self> {
        let row = assert_object(value, label)?;
        let mut snapshot = Self::default();
        for (index, field) in RUST_EXTENDED_SNAPSHOT_FIELDS.iter().enumerate() {
            snapshot.values[index] = json_number(
                row.get(*field).unwrap_or(&Value::Null),
                &format!("{label}.{field}"),
            )?;
        }
        Ok(snapshot)
    }

    /// 序列化：字段按 `RUST_EXTENDED_SNAPSHOT_FIELDS` 常量序输出（与 TS 对象一致）。
    pub fn to_json(&self) -> Value {
        let mut map = Map::with_capacity(self.values.len());
        for (field, value) in RUST_EXTENDED_SNAPSHOT_FIELDS.iter().zip(&self.values) {
            map.insert((*field).to_string(), json_i64(*value));
        }
        Value::Object(map)
    }
}

macro_rules! snapshot_parity_arms {
    ($name:expr, $es:expr; $($field:ident),* $(,)?) => {
        $(if $name == stringify!($field) { return $es.$field; })*
    };
}

macro_rules! snapshot_gap_arms {
    ($name:expr, $gap:expr; $($field:ident),* $(,)?) => {
        $(if $name == stringify!($field) { return $gap.$field; })*
    };
}

fn snapshot_field_value(name: &str, es: &EventSnapshot, gap: &DisplayGapSnapshot) -> i64 {
    snapshot_parity_arms!(name, es;
        hp, max_hp, defense, anima, guard, physique, sword_intent, sharpness,
        cloud_chain, cloud_sea, momentum, agility, water_momentum, activated_metal,
        activated_water, activated_wood, activated_fire, activated_earth, hexagram,
        star_power, attack_bonus, internal_injury, weakness, flaw, attack_reduction,
        entangle, external_injury, lost_mind, action_again_count
    );
    snapshot_gap_arms!(name, gap;
        sword_energy, quan_stance, gun_stance, metal_ring,
        water_month_sword_formation, water_formation, metal_formation,
        earth_formation, fire_formation, spring_flow, water_stealth,
        metal_iron_bone, earth_eight_wastes, wood_array, wood_scattered_branches,
        zhu_chen_bu, xing_yi_lian, turtle_formation, shatter_formation,
        thunder_formation, evil_gu_formation, spirit_gathering_formation,
        heaven_cycle_sword_formation, heaven_force_formation, flower_maze_formation,
        immovable_formation, eight_gates_formation, six_yao_formation,
        beng_quan_cun_jin, beng_quan_return_profound, dream_beng_quan_chain,
        immortal_binding_tune, illusory_tune, heartbreak_tune, wild_dance_tune,
        rejuvenation_tune, xiaoyao_tune, xiaoyao_guqin, chaotic_mind_tune,
        ling_gua_art, star_moon_fan, infinite_hexagram_plate, all_goes_well,
        ling_yuan_wai_fang, recovery, meditation, blood_calamity, lone_night_wolf,
        leaf_blade_flower, quiet_mindset, reflect_mindset, graft_flowers_to_tree,
        tide, dismantle_move, all_things_inauspicious, fate_cycle,
        yellow_bird_behind, exorcism, ice_snow_lotus, leaf_shield_flower,
        paint_finishing_touch, next_turn_defense, ignore_defense_attacks,
        next_attack_shatter_defense, momentum_limit, physique_limit, lose_hp_count,
        lose_hp_times_count, actual_damage_carry, wounded_count_carry,
        ji_lu_zong_ji_shang_zhi, last_stand_intent, last_stand_unyielding,
        action_again_per_round, active_slot_count
    );
    unreachable!("unknown extended snapshot field {name}")
}

fn detail_entry_value(entries: &[ReplayDetailEntry], key: &str) -> i64 {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value)
        .unwrap_or(0)
}

/// 剩余出牌队列项，与原版 oracle trace 的 cardQueue 同形。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustQueuedCard {
    pub id: i64,
    pub grid: i64,
    pub skip: bool,
    pub had_used: bool,
}

impl RustQueuedCard {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("id".into(), json_i64(self.id));
        map.insert("grid".into(), json_i64(self.grid));
        map.insert("skip".into(), Value::Bool(self.skip));
        map.insert("hadUsed".into(), Value::Bool(self.had_used));
        Value::Object(map)
    }

    fn from_json(value: &Value, label: &str) -> OracleResult<Self> {
        let row = assert_object(value, label)?;
        let skip = row.get("skip").unwrap_or(&Value::Null);
        let had_used = row.get("hadUsed").unwrap_or(&Value::Null);
        if !skip.is_boolean() || !had_used.is_boolean() {
            return Err(format!("{label}.skip/hadUsed must be booleans"));
        }
        Ok(RustQueuedCard {
            id: json_number(row.get("id").unwrap_or(&Value::Null), &format!("{label}.id"))?,
            grid: json_number(row.get("grid").unwrap_or(&Value::Null), &format!("{label}.grid"))?,
            skip: skip.as_bool().unwrap(),
            had_used: had_used.as_bool().unwrap(),
        })
    }
}

fn parse_card_queue(value: &Value, label: &str) -> OracleResult<Vec<RustQueuedCard>> {
    let entries = value
        .as_array()
        .ok_or_else(|| format!("{label} must be an array"))?;
    entries
        .iter()
        .enumerate()
        .map(|(index, entry)| RustQueuedCard::from_json(entry, &format!("{label}[{index}]")))
        .collect()
}

/// `--events-extended` 流的单个事件（TS `RustExtendedEvent` 同形）。
#[derive(Debug, Clone, PartialEq)]
pub struct RustExtendedEvent {
    pub kind: String,
    pub turn: i64,
    pub actor: Side,
    pub slot: Option<i64>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1: RustExtendedSnapshot,
    pub p2: RustExtendedSnapshot,
    pub p1_card_queue: Vec<RustQueuedCard>,
    pub p2_card_queue: Vec<RustQueuedCard>,
    pub p1_physical_deck: Vec<RustQueuedCard>,
    pub p2_physical_deck: Vec<RustQueuedCard>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustExtendedTerminal {
    pub winner_side: Side,
    pub actor_turn: i64,
    pub hp_delta: i64,
    pub p1_hp: i64,
    pub p2_hp: i64,
    pub remaining_decision_params: i64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustExtendedEventStream {
    pub fixture: String,
    pub terminal: RustExtendedTerminal,
    pub events: Vec<RustExtendedEvent>,
}

impl RustExtendedTerminal {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("winnerSide".into(), Value::String(self.winner_side.as_str().into()));
        map.insert("actorTurn".into(), json_i64(self.actor_turn));
        map.insert("hpDelta".into(), json_i64(self.hp_delta));
        map.insert("p1Hp".into(), json_i64(self.p1_hp));
        map.insert("p2Hp".into(), json_i64(self.p2_hp));
        map.insert(
            "remainingDecisionParams".into(),
            json_i64(self.remaining_decision_params),
        );
        Value::Object(map)
    }
}

impl RustExtendedEvent {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("kind".into(), Value::String(self.kind.clone()));
        map.insert("turn".into(), json_i64(self.turn));
        map.insert("actor".into(), Value::String(self.actor.as_str().into()));
        if let Some(slot) = self.slot {
            map.insert("slot".into(), json_i64(slot));
        }
        if let Some(card_id) = self.card_id {
            map.insert("cardId".into(), json_i64(card_id));
        }
        if let Some(card_name) = &self.card_name {
            map.insert("cardName".into(), Value::String(card_name.clone()));
        }
        map.insert("p1".into(), self.p1.to_json());
        map.insert("p2".into(), self.p2.to_json());
        map.insert(
            "p1CardQueue".into(),
            Value::Array(self.p1_card_queue.iter().map(RustQueuedCard::to_json).collect()),
        );
        map.insert(
            "p2CardQueue".into(),
            Value::Array(self.p2_card_queue.iter().map(RustQueuedCard::to_json).collect()),
        );
        map.insert(
            "p1PhysicalDeck".into(),
            Value::Array(self.p1_physical_deck.iter().map(RustQueuedCard::to_json).collect()),
        );
        map.insert(
            "p2PhysicalDeck".into(),
            Value::Array(self.p2_physical_deck.iter().map(RustQueuedCard::to_json).collect()),
        );
        Value::Object(map)
    }
}

impl RustExtendedEventStream {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("fixture".into(), Value::String(self.fixture.clone()));
        map.insert("terminal".into(), self.terminal.to_json());
        map.insert(
            "events".into(),
            Value::Array(self.events.iter().map(RustExtendedEvent::to_json).collect()),
        );
        Value::Object(map)
    }
}

/// 解析 `replay_slice --events-extended` 的 JSON 输出。
pub fn parse_rust_extended_replay_event_stream(value: &Value) -> OracleResult<RustExtendedEventStream> {
    let root = assert_object(value, "Rust extended replay event stream")?;
    let fixture = assert_string(root.get("fixture").unwrap_or(&Value::Null), "stream.fixture")?;
    let events = root
        .get("events")
        .and_then(Value::as_array)
        .ok_or_else(|| "stream.events must be an array".to_string())?;
    let terminal = parse_extended_terminal(
        root.get("terminal").unwrap_or(&Value::Null),
    )?;
    Ok(RustExtendedEventStream {
        fixture,
        terminal,
        events: events
            .iter()
            .enumerate()
            .map(|(index, entry)| parse_extended_event(entry, index))
            .collect::<OracleResult<Vec<_>>>()?,
    })
}

fn parse_extended_terminal(value: &Value) -> OracleResult<RustExtendedTerminal> {
    let row = assert_object(value, "stream.terminal")?;
    let winner_side = assert_string(
        row.get("winnerSide").unwrap_or(&Value::Null),
        "stream.terminal.winnerSide",
    )?;
    let winner_side = Side::parse(&winner_side)
        .ok_or_else(|| "stream.terminal.winnerSide must be p1 or p2".to_string())?;
    Ok(RustExtendedTerminal {
        winner_side,
        actor_turn: json_number(row.get("actorTurn").unwrap_or(&Value::Null), "stream.terminal.actorTurn")?,
        hp_delta: json_number(row.get("hpDelta").unwrap_or(&Value::Null), "stream.terminal.hpDelta")?,
        p1_hp: json_number(row.get("p1Hp").unwrap_or(&Value::Null), "stream.terminal.p1Hp")?,
        p2_hp: json_number(row.get("p2Hp").unwrap_or(&Value::Null), "stream.terminal.p2Hp")?,
        remaining_decision_params: json_number(
            row.get("remainingDecisionParams").unwrap_or(&Value::Null),
            "stream.terminal.remainingDecisionParams",
        )?,
    })
}

fn parse_extended_event(value: &Value, index: usize) -> OracleResult<RustExtendedEvent> {
    let row = assert_object(value, &format!("stream.events[{index}]"))?;
    let actor = assert_string(
        row.get("actor").unwrap_or(&Value::Null),
        &format!("stream.events[{index}].actor"),
    )?;
    let actor = Side::parse(&actor)
        .ok_or_else(|| format!("stream.events[{index}].actor must be p1 or p2"))?;
    Ok(RustExtendedEvent {
        kind: assert_string(
            row.get("kind").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].kind"),
        )?,
        turn: json_number(row.get("turn").unwrap_or(&Value::Null), &format!("stream.events[{index}].turn"))?,
        actor,
        slot: optional_number(row.get("slot")),
        card_id: optional_number(row.get("card_id")),
        card_name: row.get("card_name").and_then(Value::as_str).map(str::to_string),
        p1: RustExtendedSnapshot::from_json(
            row.get("p1").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p1"),
        )?,
        p2: RustExtendedSnapshot::from_json(
            row.get("p2").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p2"),
        )?,
        p1_card_queue: parse_card_queue(
            row.get("p1_card_queue").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p1_card_queue"),
        )?,
        p2_card_queue: parse_card_queue(
            row.get("p2_card_queue").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p2_card_queue"),
        )?,
        p1_physical_deck: parse_card_queue(
            row.get("p1_physical_deck").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p1_physical_deck"),
        )?,
        p2_physical_deck: parse_card_queue(
            row.get("p2_physical_deck").unwrap_or(&Value::Null),
            &format!("stream.events[{index}].p2_physical_deck"),
        )?,
    })
}

/// 核心四元字段：与 TS `"hp" | "maxHp" | "def" | "anima"` 同形。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CoreField {
    Hp,
    MaxHp,
    Def,
    Anima,
}

impl CoreField {
    pub const ALL: [CoreField; 4] = [CoreField::Hp, CoreField::MaxHp, CoreField::Def, CoreField::Anima];

    pub fn as_str(self) -> &'static str {
        match self {
            CoreField::Hp => "hp",
            CoreField::MaxHp => "maxHp",
            CoreField::Def => "def",
            CoreField::Anima => "anima",
        }
    }
}

/// 原作侧单帧快照（结构类型：oracle trace 的 OriginalOracleCharacterState 可赋值）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OriginalSideSnapshot {
    pub hp: i64,
    pub max_hp: Option<i64>,
    pub def: Option<i64>,
    pub anima: Option<i64>,
    pub buffs: Option<Map<String, Value>>,
}

impl OriginalSideSnapshot {
    pub fn from_json(value: &Value) -> OracleResult<Self> {
        let row = assert_object(value, "original side snapshot")?;
        Ok(OriginalSideSnapshot {
            hp: json_number(row.get("hp").unwrap_or(&Value::Null), "hp")?,
            max_hp: optional_number(row.get("maxHp")),
            def: optional_number(row.get("def")),
            anima: optional_number(row.get("anima")),
            buffs: row.get("buffs").and_then(Value::as_object).cloned(),
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CoreDiff {
    pub side: Side,
    pub field: CoreField,
    pub original: i64,
    pub rust: i64,
}

impl CoreDiff {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("side".into(), Value::String(self.side.as_str().into()));
        map.insert("field".into(), Value::String(self.field.as_str().into()));
        map.insert("original".into(), json_i64(self.original));
        map.insert("rust".into(), json_i64(self.rust));
        Value::Object(map)
    }
}

pub type BuffDiffClass = &'static str;

#[derive(Debug, Clone, PartialEq)]
pub struct BuffDiff {
    pub side: Side,
    pub entry: RustBuffMapEntry,
    pub original: i64,
    pub rust: i64,
    pub classification: BuffDiffClass,
}

impl BuffDiff {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("side".into(), Value::String(self.side.as_str().into()));
        map.insert("entry".into(), self.entry.to_json());
        map.insert("original".into(), json_i64(self.original));
        map.insert("rust".into(), json_i64(self.rust));
        map.insert("classification".into(), Value::String(self.classification.into()));
        Value::Object(map)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct UnmappedBuffEntry {
    pub buff_type_id: i64,
    pub value: i64,
}

impl UnmappedBuffEntry {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("buffTypeId".into(), json_i64(self.buff_type_id));
        map.insert("value".into(), json_i64(self.value));
        Value::Object(map)
    }
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct SideComparison {
    pub core_diffs: Vec<CoreDiff>,
    pub buff_diffs: Vec<BuffDiff>,
    pub unmapped: Vec<UnmappedBuffEntry>,
}

impl SideComparison {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert(
            "coreDiffs".into(),
            Value::Array(self.core_diffs.iter().map(CoreDiff::to_json).collect()),
        );
        map.insert(
            "buffDiffs".into(),
            Value::Array(self.buff_diffs.iter().map(BuffDiff::to_json).collect()),
        );
        map.insert(
            "unmapped".into(),
            Value::Array(self.unmapped.iter().map(UnmappedBuffEntry::to_json).collect()),
        );
        Value::Object(map)
    }
}

/// 原作侧核心四元取值；字段缺失即抛错（不静默当 0）。
pub fn core_snapshot_value(state: &OriginalSideSnapshot, field: CoreField) -> OracleResult<i64> {
    match field {
        CoreField::Hp => Ok(state.hp),
        CoreField::MaxHp => state
            .max_hp
            .ok_or_else(|| "original snapshot state lacks maxHp".to_string()),
        CoreField::Def => state
            .def
            .ok_or_else(|| "original snapshot state lacks def".to_string()),
        CoreField::Anima => state
            .anima
            .ok_or_else(|| "original snapshot state lacks anima".to_string()),
    }
}

/// Rust 侧核心四元取值（--events-extended 快照字段名映射）。
pub fn rust_snapshot_value(snapshot: &RustExtendedSnapshot, field: CoreField) -> i64 {
    snapshot
        .get(match field {
            CoreField::Hp => "hp",
            CoreField::MaxHp => "max_hp",
            CoreField::Def => "defense",
            CoreField::Anima => "anima",
        })
        .expect("core snapshot fields are part of the extended const list")
}

/// 映射内 BuffType ID 的 Rust 语义值。入口只接收已通过
/// `validate_rust_buff_key_map` 重锚定的 in-map entry；`expect` 对应 TS 的
/// `?? 0` 类型收窄（parse 已强制 104 个字段全部为有限数，不可达）。
pub fn original_oracle_rust_buff_value(entry: &RustBuffMapEntry, snapshot: &RustExtendedSnapshot) -> i64 {
    snapshot
        .get(&entry.rust_key)
        .unwrap_or_else(|| panic!("rustKey {} is not an extended snapshot field", entry.rust_key))
}

/// 未映射的原版 buff 显式归类入口（引擎覆盖缺口）：值为 0 的键不报告。
pub fn unmapped_original_buff_entries(
    buffs: &Map<String, Value>,
    entries: &[RustBuffMapEntry],
) -> Vec<UnmappedBuffEntry> {
    let mut unmapped: Vec<UnmappedBuffEntry> = buffs
        .iter()
        .filter_map(|(key, value)| {
            let buff_type_id = key.parse::<i64>().ok()?;
            let value = value.as_i64()?;
            (value != 0
                && !entries.iter().any(|entry| entry.buff_type_id == buff_type_id))
            .then_some(UnmappedBuffEntry { buff_type_id, value })
        })
        .collect();
    unmapped.sort_by_key(|entry| entry.buff_type_id);
    unmapped
}

/// 单侧逐帧对照：核心四元 + 映射 buff 逐键四分类（match / only-original /
/// only-rust / value-mismatch）+ 未映射原版 buff 显式归类。零值键不产生差异。
pub fn compare_side(
    side: Side,
    original: &OriginalSideSnapshot,
    rust: &RustExtendedSnapshot,
    entries: &[RustBuffMapEntry],
) -> SideComparison {
    let mut core_diffs = Vec::new();
    for field in CoreField::ALL {
        let original_value = core_snapshot_value(original, field)
            .unwrap_or_else(|error| panic!("{error}"));
        let rust_value = rust_snapshot_value(rust, field);
        if original_value != rust_value {
            core_diffs.push(CoreDiff {
                side,
                field,
                original: original_value,
                rust: rust_value,
            });
        }
    }
    let mut buff_diffs = Vec::new();
    let empty = Map::new();
    let original_buffs = original.buffs.as_ref().unwrap_or(&empty);
    for entry in entries {
        let original_value = original_buffs
            .get(&entry.buff_type_id.to_string())
            .and_then(Value::as_i64)
            .unwrap_or(0);
        let rust_value = original_oracle_rust_buff_value(entry, rust);
        let classification: BuffDiffClass = if original_value == rust_value {
            continue;
        } else if original_value != 0 && rust_value == 0 {
            "only-original"
        } else if original_value == 0 && rust_value != 0 {
            "only-rust"
        } else {
            "value-mismatch"
        };
        buff_diffs.push(BuffDiff {
            side,
            entry: entry.clone(),
            original: original_value,
            rust: rust_value,
            classification,
        });
    }
    SideComparison {
        core_diffs,
        buff_diffs,
        unmapped: unmapped_original_buff_entries(original_buffs, entries),
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct OriginalBuffAnchor {
    pub archive_enum: String,
    pub original_name: Option<String>,
    pub category: String,
}

/// 按 BuffType ID 查 archive 锚定（枚举名/原文名/分类）；未收录返回 None。
/// TS 版按 id 缓存；Rust 侧由调用方持有 archive Value（进程内一次解析）。
pub fn original_buff_archive_anchor(
    buff_type_id: i64,
    archive: &Value,
) -> OracleResult<Option<OriginalBuffAnchor>> {
    let archive = assert_object(archive, "buff-category-archive")?;
    let buffs = assert_object(
        archive
            .get("buffs")
            .ok_or_else(|| "buff-category-archive.buffs must be an object".to_string())?,
        "buff-category-archive.buffs",
    )?;
    for (archive_enum, raw) in buffs {
        let record = assert_object(raw, &format!("archive[{archive_enum}]"))?;
        if record.get("id").and_then(Value::as_i64) == Some(buff_type_id) {
            return Ok(Some(OriginalBuffAnchor {
                archive_enum: archive_enum.clone(),
                original_name: record.get("name").and_then(Value::as_str).map(str::to_string),
                category: record
                    .get("category")
                    .map(js_string)
                    .unwrap_or_else(|| "undefined".into()),
            }));
        }
    }
    Ok(None)
}

// ---- 共享解析助手（错误文案与 TS _shared.ts / 局部助手逐字一致） ----

pub(crate) fn assert_object<'a>(value: &'a Value, label: &str) -> OracleResult<&'a Map<String, Value>> {
    value
        .as_object()
        .ok_or_else(|| format!("{label} must be an object"))
}

pub(crate) fn assert_string(value: &Value, label: &str) -> OracleResult<String> {
    value
        .as_str()
        .map(str::to_string)
        .ok_or_else(|| format!("{label} must be a string"))
}

pub(crate) fn json_number(value: &Value, label: &str) -> OracleResult<i64> {
    value
        .as_i64()
        .ok_or_else(|| format!("{label} must be a finite number"))
}

pub(crate) fn optional_number(value: Option<&Value>) -> Option<i64> {
    value.and_then(Value::as_i64)
}

/// TS `String(value)` 的窄化近似：只覆盖错误文案里出现的原始 JSON 形态。
pub(crate) fn js_string(value: &Value) -> String {
    match value {
        Value::Null => "null".into(),
        Value::Bool(inner) => inner.to_string(),
        Value::Number(inner) => inner.to_string(),
        Value::String(inner) => inner.clone(),
        Value::Array(_) => "[object Array]".into(),
        Value::Object(_) => "[object Object]".into(),
    }
}

pub(crate) fn json_i64(value: i64) -> Value {
    Value::from(value)
}

/// 有序 `Record<string, number>`：键序即插入序（与 TS 对象一致）。
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedNumberRecord(pub Vec<(String, i64)>);

impl OrderedNumberRecord {
    pub fn to_json(&self) -> Value {
        let mut map = Map::with_capacity(self.0.len());
        for (key, value) in &self.0 {
            map.insert(key.clone(), json_i64(*value));
        }
        Value::Object(map)
    }

    pub fn get(&self, key: &str) -> Option<i64> {
        self.0
            .iter()
            .find(|(candidate, _)| candidate == key)
            .map(|(_, value)| *value)
    }
}

impl Serialize for OrderedNumberRecord {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (key, value) in &self.0 {
            map.serialize_entry(key, value)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderedNumberRecord {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        struct RecordVisitor;
        impl<'de> Visitor<'de> for RecordVisitor {
            type Value = OrderedNumberRecord;

            fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
                formatter.write_str("a record of finite numbers")
            }

            fn visit_map<A: MapAccess<'de>>(
                self,
                mut access: A,
            ) -> Result<Self::Value, A::Error> {
                let mut entries = Vec::new();
                while let Some((key, value)) = access.next_entry::<String, i64>()? {
                    entries.push((key, value));
                }
                Ok(OrderedNumberRecord(entries))
            }
        }
        deserializer.deserialize_map(RecordVisitor)
    }
}