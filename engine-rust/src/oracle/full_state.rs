//! `original-oracle-full-state-rust.ts` 的 Rust 移植：用 canonical 引擎
//! （进程内 detailed 事件流）生成与冻结原版全状态基线同形的投影；Rust 快照只
//! 暴露 buff-id-rust-key-map 已映射的 BuffType，其余声明
//! （modeled-original-buffs.json）在 Rust 侧不可观测，比较前从基线里剔除并
//! 作为覆盖缺口显式列出，不当 0 比较。
//!
//! 同时移植 diff 依赖的 `original-oracle-full-state.ts` 两个纯函数：
//! [`project_original_oracle_observation`]（观察 → 全状态投影）与
//! [`compare_original_oracle_full_state`]（冻结基线 ↔ 投影逐键比较）。
//! `originalOracleContractBuffDeclarations` 的 semantic-contract 交叉核对
//! （missingSemanticIds / contractIds）依赖 TS 侧契约注册表，不参与任何比较
//! 判定，此处不移植；modeled 注册表自身的 schema/build/source 校验保留。

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeSet;
use std::path::Path;

use crate::BattleFixture;

use super::buff_map::{
    OracleResult, OrderedNumberRecord, RustBuffKeyMap, RustExtendedSnapshot, Side,
};
use super::diff::OriginalOracleObservation;
use super::events;

pub const ORIGINAL_ORACLE_FULL_STATE_SCHEMA_VERSION: u8 = 4;
/// 运行中的 runtime-trace 插件版本，唯一真相源是
/// `research/original-game/runtime-trace-plugin/src/YixianRuntimeTracePlugin.cs`。
pub const ORIGINAL_ORACLE_PLUGIN_VERSION: &str = "0.30.20";
pub const ORIGINAL_ORACLE_FULL_STATE_CHECKPOINT_BOUNDARY: &str =
    "cardExecuteCompletedBeforeAwaitContinuation";

/// `ORIGINAL_ORACLE_CORE_RESOURCE_FIELDS`（contracts/original-oracle-full-state.ts）。
pub const ORIGINAL_ORACLE_CORE_RESOURCE_FIELDS: [&str; 9] = [
    "hp",
    "maxHp",
    "defense",
    "anima",
    "momentum",
    "momentumLimit",
    "agility",
    "guard",
    "actionAgainPerRound",
];

/// `ORIGINAL_ORACLE_PROTOCOL_SNAPSHOT_FIELDS`：29 字段 TS/Rust 事件 parity 快照。
pub const ORIGINAL_ORACLE_PROTOCOL_SNAPSHOT_FIELDS: [&str; 29] = [
    "hp",
    "maxHp",
    "defense",
    "anima",
    "guard",
    "physique",
    "swordIntent",
    "sharpness",
    "cloudChain",
    "cloudSea",
    "momentum",
    "agility",
    "waterMomentum",
    "activatedMetal",
    "activatedWater",
    "activatedWood",
    "activatedFire",
    "activatedEarth",
    "hexagram",
    "starPower",
    "attackBonus",
    "internalInjury",
    "weakness",
    "flaw",
    "attackReduction",
    "entangle",
    "externalInjury",
    "lostMind",
    "actionAgainCount",
];

/// 契约字段 → extended 快照字段（original-oracle-full-state-rust.ts 的
/// PROTOCOL_TO_RUST）。
pub const PROTOCOL_TO_RUST: [(&str, &str); 29] = [
    ("hp", "hp"),
    ("maxHp", "max_hp"),
    ("defense", "defense"),
    ("anima", "anima"),
    ("guard", "guard"),
    ("physique", "physique"),
    ("swordIntent", "sword_intent"),
    ("sharpness", "sharpness"),
    ("cloudChain", "cloud_chain"),
    ("cloudSea", "cloud_sea"),
    ("momentum", "momentum"),
    ("agility", "agility"),
    ("waterMomentum", "water_momentum"),
    ("activatedMetal", "activated_metal"),
    ("activatedWater", "activated_water"),
    ("activatedWood", "activated_wood"),
    ("activatedFire", "activated_fire"),
    ("activatedEarth", "activated_earth"),
    ("hexagram", "hexagram"),
    ("starPower", "star_power"),
    ("attackBonus", "attack_bonus"),
    ("internalInjury", "internal_injury"),
    ("weakness", "weakness"),
    ("flaw", "flaw"),
    ("attackReduction", "attack_reduction"),
    ("entangle", "entangle"),
    ("externalInjury", "external_injury"),
    ("lostMind", "lost_mind"),
    ("actionAgainCount", "action_again_count"),
];

/// 原版 protocol BuffType ID（original-oracle-full-state.ts 的
/// ORIGINAL_PROTOCOL_BUFF_IDS）。
const ORIGINAL_PROTOCOL_BUFF_IDS: [(&str, i64); 25] = [
    ("guard", 5),
    ("physique", 10_023),
    ("swordIntent", 6),
    ("sharpness", 8),
    ("cloudChain", 304),
    ("cloudSea", 624),
    ("momentum", 16),
    ("agility", 15),
    ("waterMomentum", 10),
    ("activatedMetal", 237),
    ("activatedWater", 239),
    ("activatedWood", 238),
    ("activatedFire", 240),
    ("activatedEarth", 241),
    ("hexagram", 1),
    ("starPower", 3),
    ("attackBonus", 2),
    ("internalInjury", 100),
    ("weakness", 101),
    ("flaw", 102),
    ("attackReduction", 103),
    ("entangle", 104),
    ("externalInjury", 105),
    ("lostMind", 393),
    ("actionAgainCount", 308),
];

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalOracleContractBuffDeclaration {
    #[serde(rename = "originalId")]
    pub original_id: i64,
    #[serde(rename = "originalName")]
    pub original_name: String,
    #[serde(rename = "tsKey")]
    pub ts_key: String,
    pub storage: String,
}

/// modeled-original-buffs.json 的声明装载与 schema/build/source 校验
/// （originalOracleContractBuffDeclarations 的数据部分）。
pub fn load_original_oracle_contract_buff_declarations(
    modeled_path: &Path,
    build_profiles_path: &Path,
    evidence_manifest_path: &Path,
) -> OracleResult<Vec<OriginalOracleContractBuffDeclaration>> {
    let modeled: Value = serde_json::from_str(
        &std::fs::read_to_string(modeled_path)
            .map_err(|error| format!("modeled-original-buffs read failed: {error}"))?,
    )
    .map_err(|error| format!("modeled-original-buffs JSON error: {error}"))?;
    let build_profiles: Value = serde_json::from_str(
        &std::fs::read_to_string(build_profiles_path)
            .map_err(|error| format!("original-build-profiles read failed: {error}"))?,
    )
    .map_err(|error| format!("original-build-profiles JSON error: {error}"))?;
    let evidence: Value = serde_json::from_str(
        &std::fs::read_to_string(evidence_manifest_path)
            .map_err(|error| format!("EVIDENCE_MANIFEST read failed: {error}"))?,
    )
    .map_err(|error| format!("EVIDENCE_MANIFEST JSON error: {error}"))?;
    validate_original_oracle_contract_buff_declarations(&modeled, &build_profiles, &evidence)
}

/// 同名 TS 函数的纯校验部分（注入 Value，供单测）。
pub fn validate_original_oracle_contract_buff_declarations(
    modeled: &Value,
    build_profiles: &Value,
    evidence: &Value,
) -> OracleResult<Vec<OriginalOracleContractBuffDeclaration>> {
    let expected_source_build = build_profiles
        .get("projectTargetSteamBuild")
        .and_then(Value::as_str)
        .ok_or_else(|| "original-build-profiles.json has no authoritative Steam build".to_string())?;
    let expected_source = format!(
        "research/original-game/extracted/build-{expected_source_build}/BuffConfig.json"
    );
    let buff_config_evidence = evidence
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "EVIDENCE_MANIFEST has invalid BuffConfig.json identity".to_string())?
        .iter()
        .find(|file| file.get("path").and_then(Value::as_str) == Some("BuffConfig.json"))
        .ok_or_else(|| "EVIDENCE_MANIFEST has invalid BuffConfig.json identity".to_string())?;
    let declared_ok = modeled.get("schemaVersion").and_then(Value::as_i64) == Some(1)
        && modeled.get("steamBuild").and_then(Value::as_str) == Some(expected_source_build)
        && modeled.get("source").and_then(Value::as_str) == Some(expected_source.as_str())
        && modeled.get("sourceSha256").and_then(Value::as_str)
            == buff_config_evidence.get("sha256").and_then(Value::as_str);
    if !declared_ok {
        return Err("modeled original Buff declaration has an unsupported schema/build/source".into());
    }
    let declarations = modeled
        .get("declarations")
        .and_then(Value::as_array)
        .ok_or_else(|| "modeled original Buff declarations must be an array".to_string())?;
    declarations
        .iter()
        .map(|declaration| {
            serde_json::from_value::<OriginalOracleContractBuffDeclaration>(declaration.clone())
                .map_err(|error| format!("modeled original Buff declaration error: {error}"))
        })
        .collect()
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OracleDeckSlot {
    pub id: i64,
    pub grid: i64,
    pub skip: bool,
    #[serde(rename = "hadUsed")]
    pub had_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CoreResources {
    pub hp: i64,
    pub max_hp: i64,
    pub defense: i64,
    pub anima: i64,
    pub momentum: i64,
    pub momentum_limit: i64,
    pub agility: i64,
    pub guard: i64,
    pub action_again_per_round: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct ProtocolSnapshot {
    pub hp: i64,
    #[serde(rename = "maxHp")]
    pub max_hp: i64,
    pub defense: i64,
    pub anima: i64,
    pub guard: i64,
    pub physique: i64,
    #[serde(rename = "swordIntent")]
    pub sword_intent: i64,
    pub sharpness: i64,
    #[serde(rename = "cloudChain")]
    pub cloud_chain: i64,
    #[serde(rename = "cloudSea")]
    pub cloud_sea: i64,
    pub momentum: i64,
    pub agility: i64,
    #[serde(rename = "waterMomentum")]
    pub water_momentum: i64,
    #[serde(rename = "activatedMetal")]
    pub activated_metal: i64,
    #[serde(rename = "activatedWater")]
    pub activated_water: i64,
    #[serde(rename = "activatedWood")]
    pub activated_wood: i64,
    #[serde(rename = "activatedFire")]
    pub activated_fire: i64,
    #[serde(rename = "activatedEarth")]
    pub activated_earth: i64,
    pub hexagram: i64,
    #[serde(rename = "starPower")]
    pub star_power: i64,
    #[serde(rename = "attackBonus")]
    pub attack_bonus: i64,
    #[serde(rename = "internalInjury")]
    pub internal_injury: i64,
    pub weakness: i64,
    pub flaw: i64,
    #[serde(rename = "attackReduction")]
    pub attack_reduction: i64,
    pub entangle: i64,
    #[serde(rename = "externalInjury")]
    pub external_injury: i64,
    #[serde(rename = "lostMind")]
    pub lost_mind: i64,
    #[serde(rename = "actionAgainCount")]
    pub action_again_count: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullStateSideProjection {
    pub core: CoreResources,
    pub protocol: ProtocolSnapshot,
    /// All declared TS-modeled BuffType values, keyed by original numeric id.
    pub modeled_buffs: OrderedNumberRecord,
    /// Non-zero original BuffType values not yet declared by the TS model.
    pub unmapped_buffs: OrderedNumberRecord,
    pub active_slot_count: i64,
    /// Physical active grid (`m_BattleDeck` / `player.slots`).
    pub physical_deck: Vec<OracleDeckSlot>,
    /// Remaining execution queue (`m_TempBattleCardItems` / `player.cardQueue`).
    pub card_queue: Vec<OracleDeckSlot>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Sides {
    pub p1: FullStateSideProjection,
    pub p2: FullStateSideProjection,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullStateCheckpoint {
    pub actor_turn: i64,
    pub actor_side: Side,
    pub card_id: i64,
    pub sides: Sides,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullStateTerminal {
    pub winner_side: Side,
    pub actor_turn: i64,
    pub sides: Sides,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FullStateCaseProjection {
    pub case_id: String,
    pub checkpoint_count: usize,
    pub checkpoints: Vec<FullStateCheckpoint>,
    pub terminal: FullStateTerminal,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenFullStateSource {
    pub kind: String,
    #[serde(rename = "traceSha256")]
    pub trace_sha256: String,
    #[serde(rename = "runId", skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FrozenFullStateCase {
    #[serde(flatten)]
    pub projection: FullStateCaseProjection,
    #[serde(rename = "inputSha256")]
    pub input_sha256: String,
    pub source: FrozenFullStateSource,
}

/// `runOriginalOracleRustFullState` 返回形状。
#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleRustFullState {
    pub projection: FullStateCaseProjection,
    /// 已声明但 Rust 快照不可观测的原版 BuffType（升序）。
    pub unobserved_buff_ids: Vec<i64>,
}

impl OriginalOracleRustFullState {
    pub fn to_json(&self) -> Value {
        serde_json::to_value(self).expect("full-state result serializes")
    }
}

/// 用 Rust canonical 引擎生成与冻结原版全状态基线同形的投影（进程内事件流）。
pub fn run_original_oracle_rust_full_state(
    case_id: &str,
    fixture: &BattleFixture,
    fixture_label: &str,
    declarations: &[OriginalOracleContractBuffDeclaration],
    map: &RustBuffKeyMap,
) -> OracleResult<OriginalOracleRustFullState> {
    let observable: Vec<&OriginalOracleContractBuffDeclaration> = declarations
        .iter()
        .filter(|declaration| map.rust_key_by_id(declaration.original_id).is_some())
        .collect();
    let mut unobserved_buff_ids: Vec<i64> = declarations
        .iter()
        .filter(|declaration| map.rust_key_by_id(declaration.original_id).is_none())
        .map(|declaration| declaration.original_id)
        .collect();
    unobserved_buff_ids.sort_unstable();
    let stream = events::run_replay_event_stream_extended(fixture, fixture_label)
        .map_err(|error| error.to_string())?;
    let sides_of = |event: &super::buff_map::RustExtendedEvent| Sides {
        p1: side_projection(
            &event.p1,
            &event.p1_card_queue,
            &event.p1_physical_deck,
            &observable,
            map,
        ),
        p2: side_projection(
            &event.p2,
            &event.p2_card_queue,
            &event.p2_physical_deck,
            &observable,
            map,
        ),
    };
    let checkpoints: Vec<FullStateCheckpoint> = stream
        .events
        .iter()
        .filter(|event| event.kind == "cardCompleted" && event.card_id.is_some())
        .map(|event| FullStateCheckpoint {
            actor_turn: event.turn,
            actor_side: event.actor,
            card_id: event.card_id.expect("filtered above"),
            sides: sides_of(event),
        })
        .collect();
    let last = stream
        .events
        .last()
        .ok_or_else(|| format!("{case_id}: Rust replay produced no events"))?;
    Ok(OriginalOracleRustFullState {
        projection: FullStateCaseProjection {
            case_id: case_id.to_string(),
            checkpoint_count: checkpoints.len(),
            checkpoints,
            terminal: FullStateTerminal {
                winner_side: stream.terminal.winner_side,
                actor_turn: stream.terminal.actor_turn,
                sides: sides_of(last),
            },
        },
        unobserved_buff_ids,
    })
}

/// 把冻结基线限制到 Rust 可观测的声明 BuffType，便于与 Rust 投影逐键比较。
pub fn restrict_original_oracle_full_state_to_ids(
    projection: &FullStateCaseProjection,
    unobserved_buff_ids: &[i64],
) -> FullStateCaseProjection {
    let hidden: BTreeSet<String> = unobserved_buff_ids
        .iter()
        .map(|id| id.to_string())
        .collect();
    let restrict_side = |side: &FullStateSideProjection| FullStateSideProjection {
        core: side.core.clone(),
        protocol: side.protocol.clone(),
        modeled_buffs: OrderedNumberRecord(
            side.modeled_buffs
                .0
                .iter()
                .filter(|(id, _)| !hidden.contains(id))
                .cloned()
                .collect(),
        ),
        unmapped_buffs: side.unmapped_buffs.clone(),
        active_slot_count: side.active_slot_count,
        physical_deck: side.physical_deck.clone(),
        card_queue: side.card_queue.clone(),
    };
    let restrict_sides = |sides: &Sides| Sides {
        p1: restrict_side(&sides.p1),
        p2: restrict_side(&sides.p2),
    };
    FullStateCaseProjection {
        case_id: projection.case_id.clone(),
        checkpoint_count: projection.checkpoint_count,
        checkpoints: projection
            .checkpoints
            .iter()
            .map(|checkpoint| FullStateCheckpoint {
                actor_turn: checkpoint.actor_turn,
                actor_side: checkpoint.actor_side,
                card_id: checkpoint.card_id,
                sides: restrict_sides(&checkpoint.sides),
            })
            .collect(),
        terminal: FullStateTerminal {
            winner_side: projection.terminal.winner_side,
            actor_turn: projection.terminal.actor_turn,
            sides: restrict_sides(&projection.terminal.sides),
        },
    }
}

fn side_projection(
    snapshot: &RustExtendedSnapshot,
    card_queue: &[super::buff_map::RustQueuedCard],
    physical_deck: &[super::buff_map::RustQueuedCard],
    observable: &[&OriginalOracleContractBuffDeclaration],
    map: &RustBuffKeyMap,
) -> FullStateSideProjection {
    let snapshot_value =
        |field: &str| snapshot.get(field).expect("extended snapshot fields are const");
    let core = CoreResources {
        hp: snapshot_value("hp"),
        max_hp: snapshot_value("max_hp"),
        defense: snapshot_value("defense"),
        anima: snapshot_value("anima"),
        momentum: snapshot_value("momentum"),
        momentum_limit: snapshot_value("momentum_limit"),
        agility: snapshot_value("agility"),
        guard: snapshot_value("guard"),
        action_again_per_round: snapshot_value("action_again_per_round"),
    };
    let protocol_values: Vec<(&str, i64)> = PROTOCOL_TO_RUST
        .iter()
        .map(|(field, rust_key)| (*field, snapshot_value(rust_key)))
        .collect();
    let protocol = serde_json::from_value::<ProtocolSnapshot>(build_protocol_json(&protocol_values))
        .expect("protocol projection builds from the const field list");
    let modeled_buffs = OrderedNumberRecord(
        observable
            .iter()
            .map(|declaration| {
                (
                    declaration.original_id.to_string(),
                    map.rust_key_by_id(declaration.original_id)
                        .map(|rust_key| {
                            snapshot
                                .get(rust_key)
                                .expect("observable declarations are in-map")
                        })
                        .unwrap_or(0),
                )
            })
            .collect(),
    );
    FullStateSideProjection {
        core,
        protocol,
        modeled_buffs,
        unmapped_buffs: OrderedNumberRecord(Vec::new()),
        active_slot_count: snapshot_value("active_slot_count"),
        physical_deck: physical_deck.iter().map(deck_slot).collect(),
        card_queue: card_queue.iter().map(deck_slot).collect(),
    }
}

fn deck_slot(slot: &super::buff_map::RustQueuedCard) -> OracleDeckSlot {
    OracleDeckSlot {
        id: slot.id,
        grid: slot.grid,
        skip: slot.skip,
        had_used: slot.had_used,
    }
}

fn build_protocol_json(values: &[(&str, i64)]) -> Value {
    let mut map = serde_json::Map::new();
    for (field, value) in values {
        map.insert((*field).to_string(), Value::from(*value));
    }
    Value::Object(map)
}

// ---- original-oracle-full-state.ts 的两个 diff 依赖纯函数 ----

#[derive(Debug, Clone, PartialEq)]
pub struct OriginalOracleProjectionResult {
    pub value: Option<FullStateCaseProjection>,
    pub obligation: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OriginalOracleUnmappedBuff {
    pub state: String,
    #[serde(rename = "originalId")]
    pub original_id: i64,
    pub value: i64,
}

/// 把原版 oracle 观察投影成与冻结基线同形的 full-state 投影。
pub fn project_original_oracle_observation(
    observation: &OriginalOracleObservation,
    declarations: &[OriginalOracleContractBuffDeclaration],
) -> OriginalOracleProjectionResult {
    match project_inner(observation, declarations) {
        Ok(value) => OriginalOracleProjectionResult {
            value: Some(value),
            obligation: None,
        },
        Err(error) => OriginalOracleProjectionResult {
            value: None,
            obligation: Some(format!(
                "{}: original full-state baseline obligation: {error}",
                observation.case_id
            )),
        },
    }
}

fn project_inner(
    observation: &OriginalOracleObservation,
    declarations: &[OriginalOracleContractBuffDeclaration],
) -> OracleResult<FullStateCaseProjection> {
    let terminal = observation
        .terminal
        .as_ref()
        .ok_or_else(|| "terminal output is missing".to_string())?;
    let p1_state = terminal
        .p1_state
        .as_ref()
        .ok_or_else(|| "terminal full state is missing".to_string())?;
    let p2_state = terminal
        .p2_state
        .as_ref()
        .ok_or_else(|| "terminal full state is missing".to_string())?;
    let checkpoints = observation
        .checkpoints
        .iter()
        .enumerate()
        .map(|(index, checkpoint)| {
            Ok(FullStateCheckpoint {
                actor_turn: checkpoint.actor_turn,
                actor_side: checkpoint.actor_side,
                card_id: checkpoint.card_id,
                sides: Sides {
                    p1: project_original_side(
                        &checkpoint.p1_state,
                        declarations,
                        &format!("checkpoint[{index}].p1"),
                    )?,
                    p2: project_original_side(
                        &checkpoint.p2_state,
                        declarations,
                        &format!("checkpoint[{index}].p2"),
                    )?,
                },
            })
        })
        .collect::<OracleResult<Vec<_>>>()?;
    Ok(FullStateCaseProjection {
        case_id: observation.case_id.clone(),
        checkpoint_count: checkpoints.len(),
        checkpoints,
        terminal: FullStateTerminal {
            winner_side: terminal.winner_side,
            actor_turn: terminal.actor_turn,
            sides: Sides {
                p1: project_original_side(p1_state, declarations, "terminal.p1")?,
                p2: project_original_side(p2_state, declarations, "terminal.p2")?,
            },
        },
    })
}

fn project_original_side(
    state: &super::diff::OriginalOracleCharacterState,
    declarations: &[OriginalOracleContractBuffDeclaration],
    label: &str,
) -> OracleResult<FullStateSideProjection> {
    let max_hp = required(state.max_hp, &format!("{label}.maxHp"))?;
    let defense = required(state.def, &format!("{label}.def"))?;
    let anima = required(state.anima, &format!("{label}.anima"))?;
    let action_again_per_round = required(
        state.action_again_per_round,
        &format!("{label}.actionAgainPerRound"),
    )?;
    let buffs = state
        .buffs
        .as_ref()
        .ok_or_else(|| format!("{label}.buffs is missing"))?;
    let active_slot_count = state
        .active_slot_count
        .ok_or_else(|| format!("{label}.activeSlotCount is missing"))?;
    let active_deck = state
        .active_deck
        .as_ref()
        .ok_or_else(|| format!("{label}.activeDeck is missing"))?;
    let card_queue = state
        .card_queue
        .as_ref()
        .ok_or_else(|| format!("{label}.cardQueue is missing"))?;
    let buff = |id: i64| -> i64 {
        buffs
            .get(&id.to_string())
            .and_then(Value::as_i64)
            .unwrap_or(0)
    };
    let core = CoreResources {
        hp: state.hp,
        max_hp: max_hp,
        defense,
        anima,
        momentum: buff(16),
        momentum_limit: buff(298),
        agility: buff(15),
        guard: buff(5),
        action_again_per_round,
    };
    let protocol_values: Vec<(&str, i64)> = ORIGINAL_PROTOCOL_BUFF_IDS
        .iter()
        .map(|(field, id)| (*field, buff(*id)))
        .chain([
            ("hp", state.hp),
            ("maxHp", max_hp),
            ("defense", defense),
            ("anima", anima),
        ])
        .collect();
    let protocol = serde_json::from_value::<ProtocolSnapshot>(build_protocol_json(&protocol_values))
        .expect("protocol projection covers every const field");
    let declared_ids: BTreeSet<String> = declarations
        .iter()
        .map(|declaration| declaration.original_id.to_string())
        .collect();
    let mut unmapped: Vec<(i64, i64)> = buffs
        .iter()
        .filter_map(|(id, value)| {
            let id = id.parse::<i64>().ok()?;
            let value = value.as_i64()?;
            (value != 0 && !declared_ids.contains(&id.to_string())).then_some((id, value))
        })
        .collect();
    unmapped.sort_by_key(|(id, _)| *id);
    Ok(FullStateSideProjection {
        core,
        protocol,
        modeled_buffs: OrderedNumberRecord(
            declarations
                .iter()
                .map(|declaration| (declaration.original_id.to_string(), buff(declaration.original_id)))
                .collect(),
        ),
        unmapped_buffs: OrderedNumberRecord(
            unmapped
                .into_iter()
                .map(|(id, value)| (id.to_string(), value))
                .collect(),
        ),
        active_slot_count,
        physical_deck: active_deck
            .iter()
            .map(|slot| OracleDeckSlot {
                id: slot.id,
                grid: slot.grid,
                skip: slot.skip,
                had_used: slot.had_used,
            })
            .collect(),
        card_queue: card_queue
            .iter()
            .map(|slot| OracleDeckSlot {
                id: slot.id,
                grid: slot.grid,
                skip: slot.skip,
                had_used: slot.had_used,
            })
            .collect(),
    })
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct OriginalOracleFullStateComparison {
    pub exact: bool,
    #[serde(rename = "firstDeviation", skip_serializing_if = "Option::is_none")]
    pub first_deviation: Option<String>,
}

/// 冻结基线 ↔ 投影 逐键比较（compareOriginalOracleFullState）。
pub fn compare_original_oracle_full_state(
    expected: &FullStateCaseProjection,
    actual: &FullStateCaseProjection,
) -> OriginalOracleFullStateComparison {
    let deviation = |first_deviation: String| OriginalOracleFullStateComparison {
        exact: false,
        first_deviation: Some(first_deviation),
    };
    if expected.case_id != actual.case_id {
        return deviation(format!(
            "caseId: expected={} actual={}",
            expected.case_id, actual.case_id
        ));
    }
    if expected.checkpoint_count != expected.checkpoints.len() {
        return deviation(format!(
            "frozen checkpointCount is inconsistent: count={} checkpoints={}",
            expected.checkpoint_count,
            expected.checkpoints.len()
        ));
    }
    if actual.checkpoint_count != actual.checkpoints.len() {
        return deviation(format!(
            "actual checkpointCount is inconsistent: count={} checkpoints={}",
            actual.checkpoint_count,
            actual.checkpoints.len()
        ));
    }
    if expected.checkpoint_count != actual.checkpoint_count {
        return deviation(format!(
            "checkpointCount: expected={} actual={}",
            expected.checkpoint_count, actual.checkpoint_count
        ));
    }
    for (index, (left, right)) in expected.checkpoints.iter().zip(&actual.checkpoints).enumerate() {
        if left.actor_turn != right.actor_turn {
            return deviation(format!(
                "checkpoint[{index}].actorTurn: expected={} actual={}",
                left.actor_turn, right.actor_turn
            ));
        }
        if left.actor_side != right.actor_side {
            return deviation(format!(
                "checkpoint[{index}].actorSide: expected={} actual={}",
                left.actor_side.as_str(),
                right.actor_side.as_str()
            ));
        }
        if left.card_id != right.card_id {
            return deviation(format!(
                "checkpoint[{index}].cardId: expected={} actual={}",
                left.card_id, right.card_id
            ));
        }
        if let Some(state_deviation) = compare_sides(
            &format!("checkpoint[{index}]"),
            &left.sides,
            &right.sides,
        ) {
            return deviation(state_deviation);
        }
    }
    if expected.terminal.winner_side != actual.terminal.winner_side {
        return deviation(format!(
            "terminal.winnerSide: expected={} actual={}",
            expected.terminal.winner_side.as_str(),
            actual.terminal.winner_side.as_str()
        ));
    }
    if expected.terminal.actor_turn != actual.terminal.actor_turn {
        return deviation(format!(
            "terminal.actorTurn: expected={} actual={}",
            expected.terminal.actor_turn, actual.terminal.actor_turn
        ));
    }
    match compare_sides("terminal", &expected.terminal.sides, &actual.terminal.sides) {
        Some(terminal_deviation) => deviation(terminal_deviation),
        None => OriginalOracleFullStateComparison {
            exact: true,
            first_deviation: None,
        },
    }
}

fn compare_sides(prefix: &str, expected: &Sides, actual: &Sides) -> Option<String> {
    for (side, left, right) in [
        ("p1", &expected.p1, &actual.p1),
        ("p2", &expected.p2, &actual.p2),
    ] {
        let left_core = serde_json::to_value(&left.core).expect("core serializes");
        let right_core = serde_json::to_value(&right.core).expect("core serializes");
        for field in ORIGINAL_ORACLE_CORE_RESOURCE_FIELDS {
            let left_value = left_core.get(field).and_then(Value::as_i64).unwrap_or(0);
            let right_value = right_core.get(field).and_then(Value::as_i64).unwrap_or(0);
            if left_value != right_value {
                return Some(format!(
                    "{prefix}.{side}.core.{field}: expected={left_value} actual={right_value}"
                ));
            }
        }
        let left_protocol = serde_json::to_value(&left.protocol).expect("protocol serializes");
        let right_protocol = serde_json::to_value(&right.protocol).expect("protocol serializes");
        for field in ORIGINAL_ORACLE_PROTOCOL_SNAPSHOT_FIELDS {
            let left_value = left_protocol.get(field).and_then(Value::as_i64).unwrap_or(0);
            let right_value = right_protocol.get(field).and_then(Value::as_i64).unwrap_or(0);
            if left_value != right_value {
                return Some(format!(
                    "{prefix}.{side}.protocol.{field}: expected={left_value} actual={right_value}"
                ));
            }
        }
        let mut expected_buff_keys: Vec<String> =
            left.modeled_buffs.0.iter().map(|(key, _)| key.clone()).collect();
        expected_buff_keys.sort();
        let mut actual_buff_keys: Vec<String> =
            right.modeled_buffs.0.iter().map(|(key, _)| key.clone()).collect();
        actual_buff_keys.sort();
        if expected_buff_keys != actual_buff_keys {
            return Some(format!(
                "{prefix}.{side}.modeledBuffs keys: expected={} actual={}",
                expected_buff_keys.join(","),
                actual_buff_keys.join(",")
            ));
        }
        for (key, left_value) in &left.modeled_buffs.0 {
            if right.modeled_buffs.get(key) != Some(*left_value) {
                return Some(format!(
                    "{prefix}.{side}.modeledBuffs.{key}: expected={left_value} actual={}",
                    right.modeled_buffs.get(key).unwrap_or(0)
                ));
            }
        }
        if let Some(unmapped_deviation) =
            compare_number_records(&format!("{prefix}.{side}.unmappedBuffs"), &left.unmapped_buffs, &right.unmapped_buffs)
        {
            return Some(unmapped_deviation);
        }
        if left.active_slot_count != right.active_slot_count {
            return Some(format!(
                "{prefix}.{side}.activeSlotCount: expected={} actual={}",
                left.active_slot_count, right.active_slot_count
            ));
        }
        if let Some(physical_deck_deviation) = compare_deck_slots(
            &format!("{prefix}.{side}.physicalDeck"),
            &left.physical_deck,
            &right.physical_deck,
        ) {
            return Some(physical_deck_deviation);
        }
        if let Some(card_queue_deviation) = compare_deck_slots(
            &format!("{prefix}.{side}.cardQueue"),
            &left.card_queue,
            &right.card_queue,
        ) {
            return Some(card_queue_deviation);
        }
    }
    None
}

fn compare_number_records(prefix: &str, expected: &OrderedNumberRecord, actual: &OrderedNumberRecord) -> Option<String> {
    let mut expected_keys: Vec<String> = expected.0.iter().map(|(key, _)| key.clone()).collect();
    expected_keys.sort();
    let mut actual_keys: Vec<String> = actual.0.iter().map(|(key, _)| key.clone()).collect();
    actual_keys.sort();
    if expected_keys != actual_keys {
        return Some(format!(
            "{prefix} keys: expected={} actual={}",
            expected_keys.join(","),
            actual_keys.join(",")
        ));
    }
    for (key, expected_value) in &expected.0 {
        if actual.get(key) != Some(*expected_value) {
            return Some(format!(
                "{prefix}.{key}: expected={expected_value} actual={}",
                actual.get(key).unwrap_or(0)
            ));
        }
    }
    None
}

fn compare_deck_slots(prefix: &str, expected: &[OracleDeckSlot], actual: &[OracleDeckSlot]) -> Option<String> {
    if expected.len() != actual.len() {
        return Some(format!(
            "{prefix}.length: expected={} actual={}",
            expected.len(),
            actual.len()
        ));
    }
    for (index, (expected_slot, actual_slot)) in expected.iter().zip(actual).enumerate() {
        for (field, expected_value, actual_value) in [
            ("id", expected_slot.id.to_string(), actual_slot.id.to_string()),
            ("grid", expected_slot.grid.to_string(), actual_slot.grid.to_string()),
            ("skip", expected_slot.skip.to_string(), actual_slot.skip.to_string()),
            ("hadUsed", expected_slot.had_used.to_string(), actual_slot.had_used.to_string()),
        ] {
            if expected_value != actual_value {
                return Some(format!(
                    "{prefix}[{index}].{field}: expected={expected_value} actual={actual_value}"
                ));
            }
        }
    }
    None
}

fn required(value: Option<i64>, label: &str) -> OracleResult<i64> {
    value.ok_or_else(|| format!("{label} is missing"))
}
