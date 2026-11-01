//! `battle-evaluator/scripts/lib/synthetic-oracle-rust-diff.ts`（239 行）的
//! Rust 逐字移植，并收敛其依赖的 `synthetic-oracle-witness.ts` 纯函数与
//! `synthetic-oracle.ts` 的输入 fixture 构造（syntheticOracleInput /
//! validateSyntheticCase）。语义逐字对齐：单个 synthetic case 的引擎判定
//! —— 终局六元 + 见证投影逐键对照（新批次 vs 原作观测；已准入回归 vs
//! 冻结基线），引擎拒绝记 error，见证 BuffType 无映射 fail-closed。
//!
//! 与 TS 的对应关系：
//! - [`compare_synthetic_oracle_rust_case`]：`compareSyntheticOracleRustCase`
//!   （新批次 vs 原作观测）；
//! - [`compare_synthetic_oracle_rust_baseline_case`]：
//!   `compareSyntheticOracleRustBaselineCase`（已准入回归 vs 冻结基线）；
//! - [`run_synthetic_oracle_rust_case`]：`runSyntheticOracleRustCase`
//!   （进程内调引擎，`run_replay_event_stream_extended`；TS 走 spawn
//!   `replay_slice --events-extended` + mkdtemp 临时文件，此处内存构造
//!   fixture 后走同一 parse→validate→run 路径，行为一致）；
//! - witness 投影/比较：`assert_current_checkpoint_boundary` /
//!   `select_witness_checkpoint` / `build_witness_projection` /
//!   `first_witness_deviation` / `first_json_deviation` / `format_value`
//!   （`synthetic-oracle-witness.ts`）；
//! - fixture 构造：`synthetic_oracle_input` / `rust_fixture`
//!   （`synthetic-oracle.ts` + `rustFixture`）。
//!
//! 对齐原则：camelCase 字段名、错误文案、判定语义与 TS 逐字一致。spawn
//! 错误包装（`Rust extended event replay failed (1): Error: {error}`）与
//! wave-2 `run_original_oracle_rust_engine` 同形。

use std::collections::BTreeMap;
use std::path::Path;

use serde::Deserialize;
use serde_json::{Map, Value};

use super::buff_map::{
    OracleResult, RustBuffKeyMap, RustBuffMapEntry, RustExtendedSnapshot, RustQueuedCard, Side,
    load_rust_buff_key_map,
};
use super::diff::{
    OriginalOracleCharacterState, OriginalOracleCheckpoint, OriginalOracleDeckSlot,
    OriginalOracleObservation, OriginalOracleTerminal,
};
use super::events::run_replay_event_stream_extended;
use crate::BattleFixture;

/// 当前 runner 唯一的 checkpoint 边界：卡牌执行完成、await 续延重新入队之前。
pub const CARD_EXECUTE_COMPLETED_BEFORE_AWAIT_CONTINUATION: &str =
    "cardExecuteCompletedBeforeAwaitContinuation";

const ENGINE_LABEL: &str = "Rust";

const TERMINAL_FIELDS: [&str; 6] = [
    "winnerSide",
    "actorTurn",
    "hpDelta",
    "p1Hp",
    "p2Hp",
    "remainingDecisionParams",
];

// ---- 输入数据（syntheticOracleInput 的两张查表） ----

/// synthetic oracle 对照的共享输入：buff 映射（witness fail-closed 校验）、
/// 原版卡表（exact 卡配置）、等级血量表。对应 TS 侧
/// `originalOracleRustBuffMap()` + `ORIGINAL_CARD_CONFIGS` + `level-config`。
pub struct SyntheticOracleContext {
    pub buff_map: RustBuffKeyMap,
    card_config_by_id: BTreeMap<i64, Value>,
    base_max_hp_by_level: BTreeMap<i64, i64>,
}

impl SyntheticOracleContext {
    /// 从仓库根加载（与 TS 模块相对路径一致）。
    pub fn load(repo_root: &Path) -> OracleResult<Self> {
        let buff_map = load_rust_buff_key_map(
            &repo_root.join("battle-evaluator/generated/buff-id-rust-key-map.json"),
            &repo_root.join("battle-evaluator/generated/buff-category-archive.json"),
        )?;
        let cards_text = std::fs::read_to_string(repo_root.join("shared/data/original-card-configs.json"))
            .map_err(|error| format!("original-card-configs read failed: {error}"))?;
        let cards: Value = serde_json::from_str(&cards_text)
            .map_err(|error| format!("original-card-configs JSON error: {error}"))?;
        let cards = cards
            .as_array()
            .ok_or_else(|| "original-card-configs must be an array".to_string())?;
        let mut card_config_by_id = BTreeMap::new();
        for config in cards {
            let id = config
                .get("id")
                .and_then(Value::as_i64)
                .ok_or_else(|| "original-card-configs entry has no integer id".to_string())?;
            card_config_by_id.insert(id, config.clone());
        }
        let levels_text = std::fs::read_to_string(repo_root.join("battle-evaluator/data/level-config.json"))
            .map_err(|error| format!("level-config read failed: {error}"))?;
        let levels: Value = serde_json::from_str(&levels_text)
            .map_err(|error| format!("level-config JSON error: {error}"))?;
        let records = levels
            .get("records")
            .and_then(Value::as_array)
            .ok_or_else(|| "level-config.records must be an array".to_string())?;
        let mut base_max_hp_by_level = BTreeMap::new();
        for record in records {
            let level = record
                .get("level")
                .and_then(|level| level.get("value"))
                .and_then(Value::as_i64)
                .ok_or_else(|| "level-config record has no level.value".to_string())?;
            let base_max_hp = record
                .get("baseMaxHp")
                .and_then(Value::as_i64)
                .ok_or_else(|| "level-config record has no baseMaxHp".to_string())?;
            base_max_hp_by_level.insert(level, base_max_hp);
        }
        Ok(Self {
            buff_map,
            card_config_by_id,
            base_max_hp_by_level,
        })
    }
}

// ---- synthetic case 类型（SyntheticOracleCase，camelCase 反序列化） ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticOracleCase {
    pub schema_version: i64,
    pub case_id: String,
    pub evidence_kind: String,
    pub legality: String,
    pub target: SyntheticTarget,
    pub pair: SyntheticPair,
    pub witness: SyntheticWitness,
    pub context: SyntheticContext,
    pub players: SyntheticPlayers,
    pub decision_tape: Vec<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticTarget {
    pub kind: String,
    pub id: i64,
    pub category: String,
    pub source_evidence: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPair {
    pub pair_id: String,
    pub role: String,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticWitness {
    pub actor_side: Side,
    pub actor_card_ordinal: i64,
    pub sides: Vec<Side>,
    pub state_fields: Vec<String>,
    pub buff_ids: Vec<i64>,
    #[serde(default)]
    pub buff_ids_by_side: Option<BTreeMap<String, Vec<i64>>>,
    pub deck: bool,
    pub require_pair_difference: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticContext {
    pub steam_build: String,
    pub round: i64,
    pub game_mode: i64,
    pub season_mechanism: i64,
    pub first_side: Side,
    #[serde(default)]
    pub max_actor_turns: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SyntheticPlayers {
    pub p1: SyntheticPlayer,
    pub p2: SyntheticPlayer,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SyntheticPlayer {
    pub level: i64,
    pub extra_max_hp: i64,
    pub character_id: i64,
    #[serde(default)]
    pub career_id: Option<i64>,
    pub cards: Vec<i64>,
    pub active_slot_count: i64,
    #[serde(default)]
    pub talents: Option<Vec<i64>>,
    #[serde(default)]
    pub talent_card_params: Option<BTreeMap<String, Vec<i64>>>,
    #[serde(default)]
    pub talent_temp_datas: Option<BTreeMap<String, i64>>,
    #[serde(default)]
    pub permanent_buff_temp_datas: Option<BTreeMap<String, i64>>,
    #[serde(default)]
    pub fate_strategies: Option<Vec<i64>>,
    #[serde(default)]
    pub fate_strategy_selections: Option<Vec<FateStrategySelection>>,
    #[serde(default)]
    pub fate_strategy_counters: Option<BTreeMap<String, i64>>,
    #[serde(default)]
    pub fate_strategy_temp_datas: Option<BTreeMap<String, i64>>,
    #[serde(default)]
    pub hand_cards: Option<Vec<i64>>,
    #[serde(default)]
    pub used_ke_yin_cards: Option<Vec<i64>>,
    #[serde(default)]
    pub last_round_life: Option<i64>,
    #[serde(default)]
    pub last_round_exp: Option<i64>,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct FateStrategySelection {
    pub id: i64,
    pub selected: i64,
    pub pendings: Vec<i64>,
}

impl SyntheticOracleCase {
    pub fn from_json(value: &Value) -> OracleResult<Self> {
        serde_json::from_value(value.clone())
            .map_err(|error| format!("synthetic oracle case JSON error: {error}"))
    }

    /// TS `witness.buffIdsBySide?.[side] ?? witness.buffIds`。
    pub fn witness_buff_ids_for_side(&self, side: Side) -> Vec<i64> {
        let by_side = self
            .witness
            .buff_ids_by_side
            .as_ref()
            .and_then(|map| map.get(side.as_str()));
        by_side.cloned().unwrap_or_else(|| self.witness.buff_ids.clone())
    }
}

// ---- 冻结基线（SyntheticOracleOriginalBaselineCase） ----

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SyntheticOracleOriginalBaselineCase {
    pub case_id: String,
    #[serde(default)]
    pub input_sha256: String,
    #[serde(default)]
    pub checkpoint_boundary: Option<String>,
    pub terminal: OriginalOracleTerminal,
    pub witness: Value,
}

// ---- 引擎侧运行结果（SyntheticRustRunResult） ----

/// 单侧可投影状态；buffs 以原版 BuffType ID 为键（ProjectableState）。
#[derive(Clone)]
pub struct ProjectableState {
    pub hp: i64,
    pub max_hp: Option<i64>,
    pub def: Option<i64>,
    pub anima: Option<i64>,
    pub momentum: Option<i64>,
    pub momentum_limit: Option<i64>,
    pub agility: Option<i64>,
    pub guard: Option<i64>,
    pub action_again_per_round: Option<i64>,
    pub buffs: Option<Map<String, Value>>,
    pub deck: Option<Value>,
}

pub struct SyntheticRustCheckpoint {
    pub actor_turn: i64,
    pub actor_side: Side,
    pub card_id: i64,
    pub p1_state: ProjectableState,
    pub p2_state: ProjectableState,
}

pub struct SyntheticRustRunResult {
    pub case_id: String,
    pub checkpoints: Vec<SyntheticRustCheckpoint>,
    pub terminal: OriginalOracleTerminal,
}

/// SyntheticOracleEngineDiff（camelCase 输出，键序 caseId/status/firstDeviation）。
#[derive(Debug, Clone, PartialEq)]
pub struct SyntheticOracleEngineDiff {
    pub case_id: String,
    pub status: &'static str,
    pub first_deviation: Option<String>,
}

impl SyntheticOracleEngineDiff {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("caseId".into(), Value::String(self.case_id.clone()));
        map.insert("status".into(), Value::String(self.status.into()));
        if let Some(deviation) = &self.first_deviation {
            map.insert("firstDeviation".into(), Value::String(deviation.clone()));
        }
        Value::Object(map)
    }
}

fn failure(case_id: &str, status: &'static str, first_deviation: String) -> SyntheticOracleEngineDiff {
    SyntheticOracleEngineDiff {
        case_id: case_id.to_string(),
        status,
        first_deviation: Some(first_deviation),
    }
}

// ---- 在 Rust canonical 引擎上跑一个合成用例 ----

/// TS `runSyntheticOracleRustCase`：buff 映射 fail-closed → 内存构造 fixture
/// （TS 写 mkdtemp 临时文件后 spawn replay_slice --events-extended；此处
/// `serde_json::to_string` + `load` 走同一 parse→validate→run 路径）→
/// cardCompleted checkpoint 上的可投影状态 + 终局。
pub fn run_synthetic_oracle_rust_case(
    synthetic_case: &SyntheticOracleCase,
    context: &SyntheticOracleContext,
) -> Result<SyntheticRustRunResult, String> {
    let entries = witness_buff_entries(synthetic_case, &context.buff_map)?;
    let fixture_value = rust_fixture(synthetic_case, context)?;
    let fixture_text = serde_json::to_string(&fixture_value)
        .map_err(|error| format!("Rust extended event replay failed (1): Error: {error}"))?;
    // 与 replay_slice --events-extended 的 parse→validate 相同。
    let fixture: BattleFixture = serde_json::from_str(&fixture_text).map_err(|error| {
        format!("Rust extended event replay failed (1): Error: {error}")
    })?;
    let stream = run_replay_event_stream_extended(&fixture, "case.json").map_err(|error| {
        format!("Rust extended event replay failed (1): Error: {error}")
    })?;
    Ok(SyntheticRustRunResult {
        case_id: synthetic_case.case_id.clone(),
        checkpoints: stream
            .events
            .iter()
            .filter(|event| event.kind == "cardCompleted" && event.card_id.is_some())
            .map(|event| SyntheticRustCheckpoint {
                actor_turn: event.turn,
                actor_side: event.actor,
                card_id: event.card_id.expect("filtered above"),
                p1_state: projectable_rust_state(&event.p1, &event.p1_card_queue, &entries),
                p2_state: projectable_rust_state(&event.p2, &event.p2_card_queue, &entries),
            })
            .collect(),
        terminal: OriginalOracleTerminal {
            winner_side: stream.terminal.winner_side,
            actor_turn: stream.terminal.actor_turn,
            hp_delta: stream.terminal.hp_delta,
            p1_hp: stream.terminal.p1_hp,
            p2_hp: stream.terminal.p2_hp,
            remaining_decision_params: stream.terminal.remaining_decision_params,
            p1_state: None,
            p2_state: None,
        },
    })
}

/// 见证 BuffType 必须全部有 Rust 映射；缺映射即引擎覆盖缺口，fail closed。
fn witness_buff_entries(
    synthetic_case: &SyntheticOracleCase,
    entries: &RustBuffKeyMap,
) -> Result<Vec<RustBuffMapEntry>, String> {
    let mut ids: Vec<i64> = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    let mut push = |id: i64| {
        if seen.insert(id) {
            ids.push(id);
        }
    };
    for id in &synthetic_case.witness.buff_ids {
        push(*id);
    }
    if let Some(by_side) = &synthetic_case.witness.buff_ids_by_side {
        for side_ids in by_side.values() {
            for id in side_ids {
                push(*id);
            }
        }
    }
    ids.iter()
        .map(|id| {
            entries
                .entries
                .iter()
                .find(|entry| entry.buff_type_id == *id)
                .cloned()
                .ok_or_else(|| {
                    format!(
                        "{} witness BuffType {} has no Rust mapping (buff-id-rust-key-map.json)",
                        synthetic_case.case_id, id
                    )
                })
        })
        .collect()
}

fn projectable_rust_state(
    snapshot: &RustExtendedSnapshot,
    queue: &[RustQueuedCard],
    entries: &[RustBuffMapEntry],
) -> ProjectableState {
    let mut buffs = Map::new();
    for entry in entries {
        buffs.insert(
            entry.buff_type_id.to_string(),
            Value::from(snapshot.get(&entry.rust_key).unwrap_or(0)),
        );
    }
    ProjectableState {
        hp: snapshot.get("hp").unwrap_or(0),
        max_hp: snapshot.get("max_hp"),
        def: snapshot.get("defense"),
        anima: snapshot.get("anima"),
        momentum: snapshot.get("momentum"),
        momentum_limit: snapshot.get("momentum_limit"),
        agility: snapshot.get("agility"),
        guard: snapshot.get("guard"),
        action_again_per_round: snapshot.get("action_again_per_round"),
        buffs: Some(buffs),
        deck: Some(Value::Array(
            queue
                .iter()
                .map(|slot| {
                    let mut map = Map::new();
                    map.insert("id".into(), Value::from(slot.id));
                    map.insert("grid".into(), Value::from(slot.grid));
                    map.insert("skip".into(), Value::from(slot.skip));
                    map.insert("hadUsed".into(), Value::from(slot.had_used));
                    Value::Object(map)
                })
                .collect(),
        )),
    }
}

// ---- 与一次原版客户端观测比较（新批次 diff 用） ----

pub fn compare_synthetic_oracle_rust_case(
    synthetic_case: &SyntheticOracleCase,
    original: Option<&OriginalOracleObservation>,
    context: &SyntheticOracleContext,
) -> SyntheticOracleEngineDiff {
    let case_id = synthetic_case.case_id.as_str();
    let Some(original) = original else {
        return failure(case_id, "error", "original observation is missing".into());
    };
    if let Some(error) = &original.error {
        return failure(case_id, "error", format!("original error: {error}"));
    }
    if original.timeout || original.terminal.is_none() {
        return failure(
            case_id,
            "error",
            if original.timeout {
                "original observation timed out".into()
            } else {
                "original terminal is missing".into()
            },
        );
    }
    if let Err(error) = assert_current_checkpoint_boundary(case_id, &original.checkpoints) {
        return failure(case_id, "error", error);
    }
    let projection = match original_witness_projection(synthetic_case, original) {
        Ok(value) => value,
        Err(error) => return failure(case_id, "error", error),
    };
    compare_against_rust(
        synthetic_case,
        original.terminal.as_ref().expect("checked above"),
        &projection,
        context,
    )
}

// ---- 与冻结的原版基线比较（已准入用例回归用；不需要本地 trace） ----

pub fn compare_synthetic_oracle_rust_baseline_case(
    synthetic_case: &SyntheticOracleCase,
    baseline: &SyntheticOracleOriginalBaselineCase,
    context: &SyntheticOracleContext,
) -> SyntheticOracleEngineDiff {
    let case_id = synthetic_case.case_id.as_str();
    if baseline.case_id != case_id {
        return failure(
            case_id,
            "error",
            format!(
                "baseline case id drift: expected={} actual={}",
                case_id, baseline.case_id
            ),
        );
    }
    if baseline.checkpoint_boundary.as_deref()
        != Some(CARD_EXECUTE_COMPLETED_BEFORE_AWAIT_CONTINUATION)
    {
        return failure(
            case_id,
            "error",
            format!(
                "baseline checkpoint boundary {} is not comparable — recollect with the current runner",
                baseline
                    .checkpoint_boundary
                    .as_deref()
                    .unwrap_or("legacyCardCompleted")
            ),
        );
    }
    compare_against_rust(synthetic_case, &baseline.terminal, &baseline.witness, context)
}

fn compare_against_rust(
    synthetic_case: &SyntheticOracleCase,
    terminal: &OriginalOracleTerminal,
    witness: &Value,
    context: &SyntheticOracleContext,
) -> SyntheticOracleEngineDiff {
    let case_id = synthetic_case.case_id.as_str();
    let run = match run_synthetic_oracle_rust_case(synthetic_case, context) {
        Ok(run) => run,
        Err(error) => return failure(case_id, "error", error),
    };
    for field in TERMINAL_FIELDS {
        let original = terminal_field(terminal, field);
        let engine = engine_terminal_field(&run.terminal, field);
        if !object_is(&original, &engine) {
            return failure(
                case_id,
                "mismatch",
                format!(
                    "terminal.{field}: original={} {ENGINE_LABEL}={}",
                    format_value(&original),
                    format_value(&engine)
                ),
            );
        }
    }
    let checkpoint = match select_engine_witness_checkpoint(synthetic_case, &run.checkpoints) {
        Ok(checkpoint) => checkpoint,
        Err(error) => return failure(case_id, "error", error),
    };
    let projection = match build_witness_projection(synthetic_case, |side| {
        Ok(checkpoint_state(checkpoint, side).clone())
    }) {
        Ok(projection) => projection,
        Err(error) => return failure(case_id, "error", error),
    };
    match first_json_deviation("witness", Some(witness), Some(&projection), ENGINE_LABEL) {
        Some(deviation) => failure(case_id, "mismatch", deviation),
        None => SyntheticOracleEngineDiff {
            case_id: case_id.to_string(),
            status: "exact",
            first_deviation: None,
        },
    }
}

/// TS `selectWitnessCheckpoint` 失败返回 undefined 后走 error 分支；
/// 此处等价地把缺失转成 error 文案。
fn select_engine_witness_checkpoint<'a>(
    synthetic_case: &SyntheticOracleCase,
    checkpoints: &'a [SyntheticRustCheckpoint],
) -> Result<&'a SyntheticRustCheckpoint, String> {
    let ordinal = synthetic_case.witness.actor_card_ordinal;
    if ordinal < 1 {
        return Err(format!(
            "{} {ENGINE_LABEL} witness checkpoint is missing",
            synthetic_case.case_id
        ));
    }
    checkpoints
        .iter()
        .filter(|checkpoint| checkpoint.actor_side == synthetic_case.witness.actor_side)
        .nth(ordinal as usize - 1)
        .ok_or_else(|| {
            format!(
                "{} {ENGINE_LABEL} witness checkpoint is missing",
                synthetic_case.case_id
            )
        })
}

fn checkpoint_state(
    checkpoint: &SyntheticRustCheckpoint,
    side: Side,
) -> &ProjectableState {
    match side {
        Side::P1 => &checkpoint.p1_state,
        Side::P2 => &checkpoint.p2_state,
    }
}

// ---- witness 投影（synthetic-oracle-witness.ts） ----

/// 检查原版 checkpoint 边界：只接受当前 runner 的稳定边界；旧 runner trace
/// 须重采，不做近似换算。
pub fn assert_current_checkpoint_boundary(
    case_id: &str,
    checkpoints: &[OriginalOracleCheckpoint],
) -> OracleResult<()> {
    let mut declared: Vec<&str> = checkpoints
        .iter()
        .map(|checkpoint| {
            checkpoint
                .checkpoint_boundary
                .as_deref()
                .unwrap_or("legacyCardCompleted")
        })
        .collect();
    if declared.is_empty() {
        return Ok(());
    }
    declared.sort_unstable();
    declared.dedup();
    if declared.len() != 1 || declared[0] != CARD_EXECUTE_COMPLETED_BEFORE_AWAIT_CONTINUATION {
        return Err(format!(
            "{} original checkpoints use boundary {}; only {} is comparable — recollect with the current runner",
            case_id,
            declared.join(","),
            CARD_EXECUTE_COMPLETED_BEFORE_AWAIT_CONTINUATION
        ));
    }
    Ok(())
}

/// 行动方第 witness.actorCardOrdinal 张完成的牌（1 起）。
pub fn select_witness_checkpoint<'a, T>(
    synthetic_case: &SyntheticOracleCase,
    checkpoints: &'a [T],
    actor_side: impl Fn(&T) -> Side,
) -> Option<&'a T> {
    let ordinal = synthetic_case.witness.actor_card_ordinal;
    if ordinal < 1 {
        return None;
    }
    checkpoints
        .iter()
        .filter(|checkpoint| actor_side(checkpoint) == synthetic_case.witness.actor_side)
        .nth(ordinal as usize - 1)
}

fn original_witness_projection(
    synthetic_case: &SyntheticOracleCase,
    observation: &OriginalOracleObservation,
) -> OracleResult<Value> {
    let checkpoint = select_witness_checkpoint(synthetic_case, &observation.checkpoints, |checkpoint| {
        checkpoint.actor_side
    })
    .ok_or_else(|| {
        format!(
            "{} original witness checkpoint is missing",
            synthetic_case.case_id
        )
    })?;
    build_witness_projection(synthetic_case, |side| {
        let state = match side {
            Side::P1 => &checkpoint.p1_state,
            Side::P2 => &checkpoint.p2_state,
        };
        Ok(original_projectable_state(state))
    })
}

/// full-state v4 trace 用 cardQueue；旧 synthetic 见证形状用 deck。
fn original_projectable_state(state: &OriginalOracleCharacterState) -> ProjectableState {
    let deck = state
        .deck
        .as_ref()
        .or(state.card_queue.as_ref())
        .map(|slots| deck_slots_to_json(slots));
    ProjectableState {
        hp: state.hp,
        max_hp: state.max_hp,
        def: state.def,
        anima: state.anima,
        momentum: None,
        momentum_limit: None,
        agility: None,
        guard: None,
        action_again_per_round: state.action_again_per_round,
        buffs: state.buffs.clone(),
        deck,
    }
}

fn deck_slots_to_json(slots: &[OriginalOracleDeckSlot]) -> Value {
    Value::Array(
        slots
            .iter()
            .map(|slot| {
                let mut map = Map::new();
                map.insert("id".into(), Value::from(slot.id));
                map.insert("grid".into(), Value::from(slot.grid));
                map.insert("skip".into(), Value::from(slot.skip));
                map.insert("hadUsed".into(), Value::from(slot.had_used));
                Value::Object(map)
            })
            .collect(),
    )
}

pub fn build_witness_projection(
    synthetic_case: &SyntheticOracleCase,
    state: impl Fn(Side) -> OracleResult<ProjectableState>,
) -> OracleResult<Value> {
    let mut sides = Map::new();
    for side in &synthetic_case.witness.sides {
        let current = state(*side)?;
        let mut fields = Map::new();
        for field in &synthetic_case.witness.state_fields {
            fields.insert(field.clone(), Value::from(state_field(&current, field)?));
        }
        let mut buffs = Map::new();
        for id in synthetic_case.witness_buff_ids_for_side(*side) {
            let key = id.to_string();
            let value = current
                .buffs
                .as_ref()
                .and_then(|buffs| buffs.get(&key))
                .cloned()
                .unwrap_or_else(|| Value::from(0));
            buffs.insert(key, value);
        }
        let mut projection = Map::new();
        projection.insert("fields".into(), Value::Object(fields));
        projection.insert("buffs".into(), Value::Object(buffs));
        if synthetic_case.witness.deck {
            projection.insert(
                "deck".into(),
                current.deck.clone().unwrap_or_else(|| Value::Array(Vec::new())),
            );
        }
        sides.insert(side.as_str().to_string(), Value::Object(projection));
    }
    let mut result = Map::new();
    result.insert("sides".into(), Value::Object(sides));
    Ok(Value::Object(result))
}

fn state_field(state: &ProjectableState, field: &str) -> OracleResult<i64> {
    match field {
        "hp" => Ok(state.hp),
        "maxHp" => state
            .max_hp
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "def" => state
            .def
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "anima" => state
            .anima
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "momentum" => state
            .momentum
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "momentumLimit" => state
            .momentum_limit
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "agility" => state
            .agility
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "guard" => state
            .guard
            .ok_or_else(|| format!("witness state field {field} is missing")),
        "actionAgainPerRound" => state
            .action_again_per_round
            .ok_or_else(|| format!("witness state field {field} is missing")),
        _ => Err(format!("witness state field {field} is missing")),
    }
}

/// 逐键 JSON 比较，返回第一处偏差；`engine_label` 为右侧名称。
pub fn first_witness_deviation(
    original: &Value,
    engine: &Value,
    engine_label: &str,
) -> Option<String> {
    first_json_deviation("witness", Some(original), Some(engine), engine_label)
}

/// TS `firstJsonDeviation`：Object.is 相等短路；数组先比长度；对象取键并集
/// 排序逐键下钻；其余（含 undefined → "missing"）用 formatValue 文案。
pub fn first_json_deviation(
    path: &str,
    left: Option<&Value>,
    right: Option<&Value>,
    right_label: &str,
) -> Option<String> {
    match (left, right) {
        (None, None) => None,
        (Some(left), Some(right)) => {
            if json_object_is(left, right) {
                return None;
            }
            match (left, right) {
                (Value::Array(left), Value::Array(right)) => {
                    if left.len() != right.len() {
                        return Some(format!(
                            "{path}.length: original={} {right_label}={}",
                            left.len(),
                            right.len()
                        ));
                    }
                    for (index, (l, r)) in left.iter().zip(right.iter()).enumerate() {
                        if let Some(deviation) =
                            first_json_deviation(&format!("{path}[{index}]"), Some(l), Some(r), right_label)
                        {
                            return Some(deviation);
                        }
                    }
                    None
                }
                (Value::Object(left), Value::Object(right)) => {
                    let mut keys: Vec<String> = left
                        .keys()
                        .chain(right.keys())
                        .cloned()
                        .collect();
                    keys.sort();
                    keys.dedup();
                    for key in keys {
                        if let Some(deviation) = first_json_deviation(
                            &format!("{path}.{key}"),
                            left.get(&key),
                            right.get(&key),
                            right_label,
                        ) {
                            return Some(deviation);
                        }
                    }
                    None
                }
                _ => Some(format!(
                    "{path}: original={} {right_label}={}",
                    format_value(left),
                    format_value(right)
                )),
            }
        }
        (left, right) => Some(format!(
            "{path}: original={} {right_label}={}",
            format_value_opt(left),
            format_value_opt(right)
        )),
    }
}

/// TS `Object.is` 语义（NaN 同 NaN、±0 区分）下的 JSON 相等。
fn json_object_is(left: &Value, right: &Value) -> bool {
    match (left, right) {
        (Value::Number(left), Value::Number(right)) => number_object_is(left, right),
        (Value::String(left), Value::String(right)) => left == right,
        (Value::Bool(left), Value::Bool(right)) => left == right,
        (Value::Null, Value::Null) => true,
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .zip(right.iter())
                    .all(|(l, r)| json_object_is(l, r))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left
                    .iter()
                    .all(|(key, value)| right.get(key).is_some_and(|other| json_object_is(value, other)))
        }
        _ => false,
    }
}

fn number_object_is(left: &serde_json::Number, right: &serde_json::Number) -> bool {
    if let (Some(left), Some(right)) = (left.as_i64(), right.as_i64()) {
        return left == right;
    }
    if let (Some(left), Some(right)) = (left.as_f64(), right.as_f64()) {
        return object_is_f64(left, right);
    }
    false
}

fn object_is_f64(left: f64, right: f64) -> bool {
    if left.is_nan() && right.is_nan() {
        return true;
    }
    left == right && (left != 0.0 || left.is_sign_negative() == right.is_sign_negative())
}

/// TS `Object.is` 下的字段值比较（winnerSide 为字符串，其余为整数）。
fn object_is(left: &Value, right: &Value) -> bool {
    json_object_is(left, right)
}

fn terminal_field(terminal: &OriginalOracleTerminal, field: &str) -> Value {
    match field {
        "winnerSide" => serde_json::to_value(terminal.winner_side).expect("side serializes"),
        "actorTurn" => Value::from(terminal.actor_turn),
        "hpDelta" => Value::from(terminal.hp_delta),
        "p1Hp" => Value::from(terminal.p1_hp),
        "p2Hp" => Value::from(terminal.p2_hp),
        "remainingDecisionParams" => Value::from(terminal.remaining_decision_params),
        other => panic!("unknown terminal field {other}"),
    }
}

fn engine_terminal_field(terminal: &OriginalOracleTerminal, field: &str) -> Value {
    terminal_field(terminal, field)
}

/// TS `formatValue`：undefined → "missing"，否则 JSON.stringify。
pub fn format_value(value: &Value) -> String {
    serde_json::to_string(value).expect("JSON value serializes")
}

fn format_value_opt(value: Option<&Value>) -> String {
    match value {
        Some(value) => format_value(value),
        None => "missing".into(),
    }
}

// ---- 输入 fixture 构造（synthetic-oracle.ts + rustFixture） ----

/// TS `syntheticOracleInput`：校验 + 输入 fixture（键序与 TS 对象一致）。
pub fn synthetic_oracle_input(
    synthetic_case: &SyntheticOracleCase,
    context: &SyntheticOracleContext,
) -> OracleResult<Value> {
    validate_synthetic_case(synthetic_case)?;
    let mut input = Map::new();
    input.insert(
        "firstPlayerSide".into(),
        Value::String(synthetic_case.context.first_side.as_str().into()),
    );
    input.insert(
        "decisionTape".into(),
        Value::Array(
            synthetic_case
                .decision_tape
                .iter()
                .map(|value| Value::from(*value))
                .collect(),
        ),
    );
    if let Some(max_actor_turns) = synthetic_case.context.max_actor_turns {
        input.insert("maxActorTurns".into(), Value::from(max_actor_turns));
    }
    let mut source = Map::new();
    source.insert("round".into(), Value::from(synthetic_case.context.round));
    source.insert(
        "steamBuild".into(),
        Value::String(synthetic_case.context.steam_build.clone()),
    );
    source.insert(
        "gameMode".into(),
        Value::from(synthetic_case.context.game_mode),
    );
    source.insert(
        "seasonMechanism".into(),
        Value::from(synthetic_case.context.season_mechanism),
    );
    input.insert("source".into(), Value::Object(source));
    let mut players = Map::new();
    players.insert(
        "p1".into(),
        synthetic_player(&synthetic_case.players.p1, Side::P1, context)?,
    );
    players.insert(
        "p2".into(),
        synthetic_player(&synthetic_case.players.p2, Side::P2, context)?,
    );
    input.insert("players".into(), Value::Object(players));
    Ok(Value::Object(input))
}

fn synthetic_player(
    player: &SyntheticPlayer,
    side: Side,
    context: &SyntheticOracleContext,
) -> OracleResult<Value> {
    let side_label = side.as_str();
    let base_max_hp = context
        .base_max_hp_by_level
        .get(&player.level)
        .copied()
        .ok_or_else(|| format!("{side_label} has unsupported level {}", player.level))?;
    let mut object = Map::new();
    object.insert("level".into(), Value::from(player.level));
    object.insert("baseMaxHp".into(), Value::from(base_max_hp));
    object.insert("extraMaxHp".into(), Value::from(player.extra_max_hp));
    object.insert("characterId".into(), Value::from(player.character_id));
    if let Some(career_id) = player.career_id {
        object.insert("careerId".into(), Value::from(career_id));
    }
    object.insert(
        "talents".into(),
        Value::Array(
            player
                .talents
                .as_deref()
                .unwrap_or(&[])
                .iter()
                .map(|value| Value::from(*value))
                .collect(),
        ),
    );
    object.insert(
        "activeSlotCount".into(),
        Value::from(player.active_slot_count),
    );
    object.insert(
        "cards".into(),
        Value::Array(
            player
                .cards
                .iter()
                .map(|id| exact_card_config(*id, side_label, context))
                .collect::<OracleResult<Vec<Value>>>()?,
        ),
    );
    object.insert(
        "permanentBuffTempDatas".into(),
        string_i64_record(player.permanent_buff_temp_datas.as_ref()),
    );
    if let Some(talent_card_params) = &player.talent_card_params {
        let mut record = Map::new();
        for (key, values) in talent_card_params {
            record.insert(
                key.clone(),
                Value::Array(values.iter().map(|value| Value::from(*value)).collect()),
            );
        }
        object.insert("talentCardParams".into(), Value::Object(record));
    }
    if let Some(talent_temp_datas) = &player.talent_temp_datas {
        object.insert(
            "talentTempDatas".into(),
            Value::Object(string_i64_map(talent_temp_datas)),
        );
    }
    if let Some(fate_strategies) = &player.fate_strategies {
        object.insert(
            "fateStrategies".into(),
            Value::Array(fate_strategies.iter().map(|value| Value::from(*value)).collect()),
        );
    }
    if let Some(selections) = &player.fate_strategy_selections {
        object.insert(
            "fateStrategySelections".into(),
            Value::Array(
                selections
                    .iter()
                    .map(|selection| {
                        let mut selection_object = Map::new();
                        selection_object.insert("id".into(), Value::from(selection.id));
                        selection_object.insert("selected".into(), Value::from(selection.selected));
                        selection_object.insert(
                            "pendings".into(),
                            Value::Array(
                                selection
                                    .pendings
                                    .iter()
                                    .map(|value| Value::from(*value))
                                    .collect(),
                            ),
                        );
                        Value::Object(selection_object)
                    })
                    .collect(),
            ),
        );
    }
    if let Some(counters) = &player.fate_strategy_counters {
        object.insert(
            "fateStrategyCounters".into(),
            Value::Object(string_i64_map(counters)),
        );
    }
    if let Some(temp_datas) = &player.fate_strategy_temp_datas {
        object.insert(
            "fateStrategyTempDatas".into(),
            Value::Object(string_i64_map(temp_datas)),
        );
    }
    if let Some(hand_cards) = &player.hand_cards {
        object.insert(
            "handCards".into(),
            Value::Array(hand_cards.iter().map(|value| Value::from(*value)).collect()),
        );
    }
    if let Some(used_ke_yin_cards) = &player.used_ke_yin_cards {
        object.insert(
            "usedKeYinCards".into(),
            Value::Array(
                used_ke_yin_cards
                    .iter()
                    .map(|value| Value::from(*value))
                    .collect(),
            ),
        );
    }
    if let Some(last_round_life) = player.last_round_life {
        object.insert("lastRoundLife".into(), Value::from(last_round_life));
    }
    if let Some(last_round_exp) = player.last_round_exp {
        object.insert("lastRoundExp".into(), Value::from(last_round_exp));
    }
    Ok(Value::Object(object))
}

fn string_i64_record(record: Option<&BTreeMap<String, i64>>) -> Value {
    Value::Object(string_i64_map(record.unwrap_or(&BTreeMap::new())))
}

fn string_i64_map(record: &BTreeMap<String, i64>) -> Map<String, Value> {
    let mut map = Map::new();
    for (key, value) in record {
        map.insert(key.clone(), Value::from(*value));
    }
    map
}

fn exact_card_config(
    id: i64,
    side: &str,
    context: &SyntheticOracleContext,
) -> OracleResult<Value> {
    context
        .card_config_by_id
        .get(&id)
        .cloned()
        .ok_or_else(|| format!("{side} synthetic deck contains unknown exact card id {id}"))
}

/// TS `validateSyntheticCase`：文案逐字一致。
pub fn validate_synthetic_case(synthetic_case: &SyntheticOracleCase) -> OracleResult<()> {
    if synthetic_case.schema_version != 1
        || synthetic_case.evidence_kind != "synthetic-original-oracle"
        || synthetic_case.legality != "client-executable"
    {
        return Err("synthetic oracle case has an invalid evidence contract".into());
    }
    if !synthetic_case_id_valid(&synthetic_case.case_id) {
        return Err(format!(
            "invalid synthetic oracle case id: {}",
            synthetic_case.case_id
        ));
    }
    if synthetic_case.target.source_evidence.is_empty() {
        return Err(format!("{} has no source evidence", synthetic_case.case_id));
    }
    let buff_ids_by_side_empty = synthetic_case
        .witness
        .buff_ids_by_side
        .as_ref()
        .map(|map| map.values().all(|ids| ids.is_empty()))
        .unwrap_or(true);
    if synthetic_case.witness.actor_card_ordinal < 1
        || synthetic_case.witness.sides.is_empty()
        || (synthetic_case.witness.state_fields.is_empty()
            && synthetic_case.witness.buff_ids.is_empty()
            && buff_ids_by_side_empty
            && !synthetic_case.witness.deck)
        || !synthetic_case.witness.require_pair_difference
    {
        return Err(format!(
            "{} has an invalid witness contract",
            synthetic_case.case_id
        ));
    }
    for (side, player) in [("p1", &synthetic_case.players.p1), ("p2", &synthetic_case.players.p2)] {
        if player.cards.len() != 8 {
            return Err(format!("{side} synthetic deck must contain 8 cards"));
        }
        if !(1..=8).contains(&player.active_slot_count) {
            return Err(format!("{side} activeSlotCount must be in [1, 8]"));
        }
    }
    Ok(())
}

/// `/^synthetic(?:\/[a-z0-9._-]+){2,}$/i` 的无依赖等价实现。
fn synthetic_case_id_valid(case_id: &str) -> bool {
    let Some(rest) = strip_case_insensitive_prefix(case_id, "synthetic/") else {
        return false;
    };
    let segments: Vec<&str> = rest.split('/').collect();
    segments.len() >= 2
        && segments
            .iter()
            .all(|segment| !segment.is_empty() && segment.chars().all(|c| {
                c.is_ascii_lowercase() || c.is_ascii_digit() || c == '.' || c == '_' || c == '-'
            }))
}

fn strip_case_insensitive_prefix<'a>(value: &'a str, prefix: &str) -> Option<&'a str> {
    if value.len() < prefix.len() {
        return None;
    }
    let (head, rest) = value.split_at(prefix.len());
    head.eq_ignore_ascii_case(prefix).then_some(rest)
}

/// TS `rustFixture`：`{schemaVersion: 1, ...input, randomFallbackTape: [],
/// expected: {...}}`（--events-extended 不比对 expected；占位只为满足
/// fixture schema）。
pub fn rust_fixture(
    synthetic_case: &SyntheticOracleCase,
    context: &SyntheticOracleContext,
) -> OracleResult<Value> {
    let input = synthetic_oracle_input(synthetic_case, context)?;
    let mut fixture = Map::new();
    fixture.insert("schemaVersion".into(), Value::from(1));
    if let Value::Object(input) = input {
        for (key, value) in input {
            fixture.insert(key, value);
        }
    } else {
        unreachable!("syntheticOracleInput returns an object");
    }
    fixture.insert("randomFallbackTape".into(), Value::Array(Vec::new()));
    let mut final_hp = Map::new();
    final_hp.insert("p1".into(), Value::from(0));
    final_hp.insert("p2".into(), Value::from(0));
    let mut expected = Map::new();
    expected.insert("winnerSide".into(), Value::String("p1".into()));
    expected.insert("actorTurnCount".into(), Value::from(0));
    expected.insert("hpDeltaP1MinusP2".into(), Value::from(0));
    expected.insert("finalHp".into(), Value::Object(final_hp));
    fixture.insert("expected".into(), Value::Object(expected));
    Ok(Value::Object(fixture))
}

#[cfg(test)]
#[path = "synthetic_tests.rs"]
mod tests;
