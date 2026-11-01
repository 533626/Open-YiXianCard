//! `battle-evaluator/scripts/lib/original-oracle-diff.ts` 的 Rust 逐字移植：
//! trace JSONL → OriginalOracleRun 解析（batch/run/status/runtimeProvenance/
//! observations）、Rust 引擎对照（blocked/error 分类）、golden/diagnosis 逐
//! case 比较（originalMatchesFixture / engineMatchesOriginal / checkpointsMatch
//! / baseline）。错误文案与 TS 抛错逐字一致。

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::buff_map::{optional_number, OracleResult, Side};
use super::events;
use super::full_state::{
    compare_original_oracle_full_state, project_original_oracle_observation,
    FrozenFullStateCase, OriginalOracleContractBuffDeclaration,
    ORIGINAL_ORACLE_FULL_STATE_CHECKPOINT_BOUNDARY, ORIGINAL_ORACLE_PLUGIN_VERSION,
};

pub type OriginalOracleDiffStatus = &'static str;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleExpected {
    pub winner_side: Side,
    pub actor_turn_count: i64,
    pub hp_delta_p1_minus_p2: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleRuntimeCase {
    pub case_id: String,
    pub fixture: String,
    #[serde(rename = "fixtureFingerprint", default)]
    pub fixture_fingerprint: String,
    pub expected: OriginalOracleExpected,
    #[serde(rename = "battleResultFile", default)]
    pub battle_result_file: String,
    #[serde(rename = "battleResultFingerprint")]
    pub battle_result_fingerprint: String,
    #[serde(rename = "evidenceKind", default)]
    pub evidence_kind: String,
    #[serde(default)]
    pub legality: String,
}

#[derive(Debug, Clone, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleRuntimeManifest {
    #[serde(default)]
    pub schema_version: i64,
    pub batch_id: String,
    #[serde(rename = "runId")]
    pub run_id: Option<String>,
    #[serde(default)]
    pub generated_at: String,
    #[serde(default)]
    pub engine_revision: String,
    #[serde(default)]
    pub evidence_kind: String,
    #[serde(default)]
    pub fixture_source: Value,
    pub runtime_provenance: RuntimeProvenance,
    #[serde(default)]
    pub cases: Vec<OriginalOracleRuntimeCase>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeProvenance {
    pub plugin_version: String,
    pub oracle_manifest_schema_version: i64,
    pub checkpoint_boundary: String,
    pub steam_app_id: i64,
    pub steam_build: String,
    pub hot_update: HotUpdateProvenance,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HotUpdateProvenance {
    pub path: String,
    pub bytes: i64,
    pub sha256: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct OriginalOracleDeckSlot {
    pub id: i64,
    pub grid: i64,
    pub skip: bool,
    #[serde(rename = "hadUsed")]
    pub had_used: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleCharacterState {
    pub hp: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub def: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub anima: Option<i64>,
    #[serde(rename = "actionAgainPerRound", skip_serializing_if = "Option::is_none")]
    pub action_again_per_round: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub buffs: Option<Map<String, Value>>,
    #[serde(rename = "activeSlotCount", skip_serializing_if = "Option::is_none")]
    pub active_slot_count: Option<i64>,
    #[serde(rename = "activeDeck", skip_serializing_if = "Option::is_none")]
    pub active_deck: Option<Vec<OriginalOracleDeckSlot>>,
    #[serde(rename = "cardQueue", skip_serializing_if = "Option::is_none")]
    pub card_queue: Option<Vec<OriginalOracleDeckSlot>>,
    /// Legacy synthetic-oracle witness shape; full-state v4 does not consume it.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deck: Option<Vec<OriginalOracleDeckSlot>>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleCheckpoint {
    #[serde(rename = "traceSequence")]
    pub trace_sequence: i64,
    #[serde(rename = "checkpointBoundary", skip_serializing_if = "Option::is_none")]
    pub checkpoint_boundary: Option<String>,
    #[serde(rename = "actorTurn")]
    pub actor_turn: i64,
    #[serde(rename = "actorSide")]
    pub actor_side: Side,
    #[serde(rename = "cardId")]
    pub card_id: i64,
    #[serde(rename = "p1Hp")]
    pub p1_hp: i64,
    #[serde(rename = "p2Hp")]
    pub p2_hp: i64,
    #[serde(rename = "p1State")]
    pub p1_state: OriginalOracleCharacterState,
    #[serde(rename = "p2State")]
    pub p2_state: OriginalOracleCharacterState,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleTerminal {
    pub winner_side: Side,
    pub actor_turn: i64,
    pub hp_delta: i64,
    pub p1_hp: i64,
    pub p2_hp: i64,
    pub remaining_decision_params: i64,
    #[serde(rename = "p1State", skip_serializing_if = "Option::is_none")]
    pub p1_state: Option<OriginalOracleCharacterState>,
    #[serde(rename = "p2State", skip_serializing_if = "Option::is_none")]
    pub p2_state: Option<OriginalOracleCharacterState>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleObservation {
    pub case_id: String,
    #[serde(rename = "inputSha256", default, skip_serializing_if = "String::is_empty")]
    pub input_sha256: String,
    pub checkpoints: Vec<OriginalOracleCheckpoint>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminal: Option<OriginalOracleTerminal>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub timeout: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OriginalOracleRun {
    pub batch_id: String,
    pub run_id: Option<String>,
    pub status: String,
    pub runtime_provenance: RuntimeProvenance,
    pub case_order: Vec<String>,
    pub observations: Vec<(String, OriginalOracleObservation)>,
}

impl OriginalOracleRun {
    pub fn observation(&self, case_id: &str) -> Option<&OriginalOracleObservation> {
        self.observations
            .iter()
            .find(|(candidate, _)| candidate == case_id)
            .map(|(_, observation)| observation)
    }

    /// TS 驱动侧的 JSON 形状（observations 为对象，键序 = caseOrder）。
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("batchId".into(), Value::String(self.batch_id.clone()));
        if let Some(run_id) = &self.run_id {
            map.insert("runId".into(), Value::String(run_id.clone()));
        }
        map.insert("status".into(), Value::String(self.status.clone()));
        map.insert(
            "runtimeProvenance".into(),
            serde_json::to_value(&self.runtime_provenance).expect("provenance serializes"),
        );
        map.insert(
            "caseOrder".into(),
            Value::Array(
                self.case_order
                    .iter()
                    .map(|case_id| Value::String(case_id.clone()))
                    .collect(),
            ),
        );
        map.insert(
            "observations".into(),
            Value::Object(self.observations_to_map()),
        );
        Value::Object(map)
    }

    pub fn observations_to_map(&self) -> Map<String, Value> {
        let mut map = Map::new();
        for case_id in &self.case_order {
            if let Some((_, observation)) = self
                .observations
                .iter()
                .find(|(candidate, _)| candidate == case_id)
            {
                map.insert(
                    case_id.clone(),
                    serde_json::to_value(observation).expect("observation serializes"),
                );
            }
        }
        map
    }
}

/// Rust canonical 引擎对同一 fixture 的终局与逐牌 checkpoint（golden/diagnosis
/// diff 的引擎侧）。
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleEngineCheckpoint {
    pub actor_turn: i64,
    pub actor_side: Side,
    pub card_id: i64,
    pub p1_hp: i64,
    pub p2_hp: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleEngineRun {
    pub result: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner: Option<Side>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub actor_turn: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hp_delta: Option<i64>,
    pub checkpoints: Vec<OriginalOracleEngineCheckpoint>,
}

/// 跑引擎 detailed 事件流（进程内）；缺规则/缺决策记 blocked，其余失败记 error。
/// 错误分类与 TS 的 `/missing (rule|original decision)|invalid decision/` 正则
/// 对错误文本的匹配逐字一致。
pub fn run_original_oracle_rust_engine(fixture_path: &std::path::Path) -> OriginalOracleEngineRun {
    let outcome = crate::load_fixture_file(fixture_path)
        .and_then(|fixture| crate::run_replay_fixture_with_detailed_events(&fixture));
    match outcome {
        Ok(run) => {
            let stream = events::extended_oracle_stream("", &run);
            OriginalOracleEngineRun {
                result: "ran".to_string(),
                message: None,
                winner: Some(stream.terminal.winner_side),
                actor_turn: Some(stream.terminal.actor_turn),
                hp_delta: Some(stream.terminal.hp_delta),
                checkpoints: stream
                    .events
                    .iter()
                    .filter(|event| event.kind == "cardCompleted" && event.card_id.is_some())
                    .map(|event| OriginalOracleEngineCheckpoint {
                        actor_turn: event.turn,
                        actor_side: event.actor,
                        card_id: event.card_id.expect("filtered above"),
                        p1_hp: event.p1.get("hp").unwrap_or(0),
                        p2_hp: event.p2.get("hp").unwrap_or(0),
                    })
                    .collect(),
            }
        }
        Err(error) => {
            // 与 TS spawn 路径同形：replay_slice 失败时 stderr 以
            // "Error: {error}" 退出（exit code 1）。
            let message =
                format!("Rust extended event replay failed (1): Error: {error}");
            let result = if message.contains("missing rule")
                || message.contains("missing original decision")
                || message.contains("invalid decision")
            {
                "blocked"
            } else {
                "error"
            };
            OriginalOracleEngineRun {
                result: result.to_string(),
                message: Some(message),
                winner: None,
                actor_turn: None,
                hp_delta: None,
                checkpoints: Vec::new(),
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OriginalOracleCaseDiff {
    pub case_id: String,
    pub status: OriginalOracleDiffStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub observation: Option<OriginalOracleObservation>,
    pub engine: OriginalOracleEngineRun,
    pub fixture_expected: OriginalOracleExpected,
    pub original_matches_fixture: bool,
    pub engine_matches_original: bool,
    pub checkpoints_match: bool,
    pub baseline_ready: bool,
    pub original_matches_baseline: bool,
    #[serde(rename = "fullStateObligation", skip_serializing_if = "Option::is_none")]
    pub full_state_obligation: Option<String>,
    #[serde(rename = "firstDeviation", skip_serializing_if = "Option::is_none")]
    pub first_deviation: Option<String>,
}

/// 解析 oracle trace JSONL，选取最后一个完成的 original-oracle batch/run。
pub fn parse_original_oracle_run(
    jsonl: &str,
    requested_batch_id: Option<&str>,
    requested_run_id: Option<&str>,
) -> OracleResult<OriginalOracleRun> {
    let records = parse_jsonl(jsonl)?;
    let mut completed_index: Option<usize> = None;
    for index in (0..records.len()).rev() {
        let record = &records[index];
        if record.get("kind").and_then(Value::as_str) == Some("originalOracleBatchCompleted")
            && requested_batch_id
                .map(|requested| record.get("batchId").and_then(Value::as_str) == Some(requested))
                .unwrap_or(true)
            && requested_run_id
                .map(|requested| record.get("runId").and_then(Value::as_str) == Some(requested))
                .unwrap_or(true)
        {
            completed_index = Some(index);
            break;
        }
    }
    let completed_index = completed_index.ok_or_else(|| {
        let requested = [requested_batch_id, requested_run_id]
            .into_iter()
            .flatten()
            .collect::<Vec<_>>()
            .join("/");
        if requested.is_empty() {
            "trace has no completed original-oracle batch".to_string()
        } else {
            format!("trace has no completed original-oracle batch/run {requested}")
        }
    })?;
    let completed = &records[completed_index];
    let batch_id = required_string(completed, "batchId")?;
    let run_id = optional_string(completed.get("runId"));
    let mut started_indexes = Vec::new();
    for index in 0..=completed_index {
        let record = &records[index];
        if record.get("kind").and_then(Value::as_str) == Some("originalOracleBatchStarted")
            && record.get("batchId").and_then(Value::as_str) == Some(batch_id.as_str())
            && same_run(record, run_id.as_deref())
        {
            started_indexes.push(index);
        }
    }
    if started_indexes.len() != 1 {
        return Err(format!(
            "trace must contain exactly one original-oracle batch start for {batch_id}{}; found {}",
            run_id
                .as_deref()
                .map(|run_id| format!("/{run_id}"))
                .unwrap_or_default(),
            started_indexes.len()
        ));
    }
    let started_index = started_indexes[0];
    let selected_records = &records[started_index..=completed_index];
    let runtime_provenance = runtime_provenance_from_record(&selected_records[0])?;
    let completion_provenance = runtime_provenance_from_record(completed)?;
    let completion_provenance_errors = validate_original_oracle_runtime_provenance(
        &completion_provenance,
        &runtime_provenance,
        "original-oracle batch completion",
    );
    if !completion_provenance_errors.is_empty() {
        return Err(completion_provenance_errors.join("\n"));
    }

    let mut observations: Vec<(String, MutableObservation)> = Vec::new();
    let mut case_order: Vec<String> = Vec::new();
    let mut completed_cases: Vec<String> = Vec::new();
    let mut active_case_id: Option<String> = None;
    for record in &selected_records[1..selected_records.len() - 1] {
        let kind = optional_string(record.get("kind")).unwrap_or_default();
        if kind == "originalOracleBatchStarted" || kind == "originalOracleBatchCompleted" {
            return Err(format!(
                "trace contains a nested original-oracle batch marker inside {batch_id}{}",
                run_id
                    .as_deref()
                    .map(|run_id| format!("/{run_id}"))
                    .unwrap_or_default()
            ));
        }
        if kind.starts_with("originalOracle") && record.get("batchId").is_some() {
            require_run_identity(record, &batch_id, run_id.as_deref(), &kind)?;
        }
        if kind == "originalOracleCaseStarted" {
            if let Some(active) = &active_case_id {
                return Err(format!(
                    "original-oracle case {} started while {active} is still active",
                    required_string(record, "caseId")?
                ));
            }
            let case_id = required_string(record, "caseId")?;
            if observations.iter().any(|(candidate, _)| candidate == &case_id)
                || completed_cases.contains(&case_id)
            {
                return Err(format!("original-oracle case started more than once: {case_id}"));
            }
            let input_sha256 = required_string(record, "inputSha256")?;
            observations.push((
                case_id.clone(),
                MutableObservation {
                    case_id: case_id.clone(),
                    input_sha256,
                    checkpoints: Vec::new(),
                    terminal: None,
                    error: None,
                    timeout: false,
                },
            ));
            case_order.push(case_id.clone());
            active_case_id = Some(case_id);
            continue;
        }
        if kind == "cardCompleted" && record.get("oracleCaseId").is_some() {
            let active = active_case_id
                .as_deref()
                .ok_or_else(|| "original-oracle card checkpoint appeared outside an active case".to_string())?;
            require_run_identity(record, &batch_id, run_id.as_deref(), &kind)?;
            let observation =
                require_active_observation(&mut observations, active, &kind)?;
            require_case_identity(record, active, &observation.input_sha256, "oracleCaseId")?;
            observation
                .checkpoints
                .push(checkpoint_from_record(record)?);
            continue;
        }
        if kind == "originalOracleCaseTimeout" {
            let active = active_case_id
                .as_ref()
                .ok_or_else(|| format!("{kind} appeared outside an active original-oracle case"))?;
            let observation =
                require_active_observation(&mut observations, active, &kind)?;
            require_case_identity(record, &observation.case_id, &observation.input_sha256, "caseId")?;
            observation.timeout = true;
            continue;
        }
        if kind == "originalOracleCaseCompleted" {
            let active = active_case_id
                .as_ref()
                .ok_or_else(|| format!("{kind} appeared outside an active original-oracle case"))?;
            let observation =
                require_active_observation(&mut observations, active, &kind)?;
            require_case_identity(record, &observation.case_id, &observation.input_sha256, "caseId")?;
            observation.terminal = Some(terminal_from_record(record)?);
            completed_cases.push(observation.case_id.clone());
            active_case_id = None;
            continue;
        }
        if kind == "originalOracleCaseError" {
            let active = active_case_id
                .as_ref()
                .ok_or_else(|| format!("{kind} appeared outside an active original-oracle case"))?;
            let observation =
                require_active_observation(&mut observations, active, &kind)?;
            require_case_identity(record, &observation.case_id, &observation.input_sha256, "caseId")?;
            observation.error = Some(
                optional_string(record.get("error"))
                    .unwrap_or_else(|| "unknown original-oracle error".to_string()),
            );
            completed_cases.push(observation.case_id.clone());
            active_case_id = None;
        }
    }
    if let Some(active_case_id) = active_case_id {
        return Err(format!(
            "original-oracle case did not terminate before batch completion: {active_case_id}"
        ));
    }

    Ok(OriginalOracleRun {
        batch_id,
        run_id,
        status: optional_string(completed.get("status")).unwrap_or_else(|| "unknown".to_string()),
        runtime_provenance,
        case_order,
        observations: observations
            .into_iter()
            .map(|(case_id, observation)| (case_id, freeze_observation(observation)))
            .collect(),
    })
}

/// expectedOriginalOracleRuntimeProvenance：EVIDENCE_MANIFEST.json 的
/// DarkSun.HotUpdate.dll 指纹 + 常量 plugin/schema/boundary。
pub fn expected_original_oracle_runtime_provenance(
    evidence_manifest: &Value,
) -> OracleResult<RuntimeProvenance> {
    let hot_update = evidence_manifest
        .get("files")
        .and_then(Value::as_array)
        .ok_or_else(|| "EVIDENCE_MANIFEST is missing DarkSun.HotUpdate.dll".to_string())?
        .iter()
        .find(|file| file.get("path").and_then(Value::as_str) == Some("DarkSun.HotUpdate.dll"))
        .ok_or_else(|| "EVIDENCE_MANIFEST is missing DarkSun.HotUpdate.dll".to_string())?;
    let bytes = hot_update.get("bytes").and_then(Value::as_i64);
    let sha256 = hot_update.get("sha256").and_then(Value::as_str);
    let bytes_ok = bytes.map(|bytes| bytes > 0).unwrap_or(false);
    let sha_ok = sha256.map(is_sha256_text).unwrap_or(false);
    if !bytes_ok || !sha_ok {
        return Err("EVIDENCE_MANIFEST has invalid DarkSun.HotUpdate.dll identity".into());
    }
    Ok(RuntimeProvenance {
        plugin_version: ORIGINAL_ORACLE_PLUGIN_VERSION.to_string(),
        oracle_manifest_schema_version: 2,
        checkpoint_boundary: ORIGINAL_ORACLE_FULL_STATE_CHECKPOINT_BOUNDARY.to_string(),
        steam_app_id: evidence_manifest
            .get("steamAppId")
            .and_then(Value::as_i64)
            .ok_or_else(|| "EVIDENCE_MANIFEST has invalid DarkSun.HotUpdate.dll identity".to_string())?,
        steam_build: evidence_manifest
            .get("steamBuild")
            .and_then(Value::as_str)
            .ok_or_else(|| "EVIDENCE_MANIFEST has invalid DarkSun.HotUpdate.dll identity".to_string())?
            .to_string(),
        hot_update: HotUpdateProvenance {
            path: "DarkSun.HotUpdate.dll".to_string(),
            bytes: bytes.unwrap_or(0),
            sha256: sha256.unwrap_or_default().to_string(),
        },
    })
}

/// runtimeProvenanceFromRecord：从 trace 记录解析 provenance，常量字段
/// （HotUpdate 路径 / plugin 版本 / schema / checkpoint 边界）与指纹格式 fail-closed。
pub fn runtime_provenance_from_record(record: &Value) -> OracleResult<RuntimeProvenance> {
    let provenance = Value::Object(required_object(record, "runtimeProvenance")?.clone());
    let hot_update = Value::Object(required_object(&provenance, "hotUpdate")?.clone());
    let path = required_string(&hot_update, "path")?;
    if path != "DarkSun.HotUpdate.dll" {
        return Err(format!(
            "trace runtime provenance HotUpdate path is not DarkSun.HotUpdate.dll: {path}"
        ));
    }
    let steam_app_id = required_number(&provenance, "steamAppId")?;
    let plugin_version = required_string(&provenance, "pluginVersion")?;
    let oracle_manifest_schema_version =
        required_number(&provenance, "oracleManifestSchemaVersion")?;
    let checkpoint_boundary = required_string(&provenance, "checkpointBoundary")?;
    if plugin_version != ORIGINAL_ORACLE_PLUGIN_VERSION {
        return Err(format!(
            "trace runtime provenance plugin version is unsupported: {plugin_version}"
        ));
    }
    if oracle_manifest_schema_version != 2 {
        return Err(format!(
            "trace runtime provenance oracle schema is unsupported: {oracle_manifest_schema_version}"
        ));
    }
    if checkpoint_boundary != ORIGINAL_ORACLE_FULL_STATE_CHECKPOINT_BOUNDARY {
        return Err(format!(
            "trace runtime provenance checkpoint boundary is unsupported: {checkpoint_boundary}"
        ));
    }
    let bytes = required_number(&hot_update, "bytes")?;
    let sha256 = required_string(&hot_update, "sha256")?;
    if steam_app_id <= 0 {
        return Err("trace runtime provenance steamAppId is not a positive integer".into());
    }
    if bytes <= 0 {
        return Err("trace runtime provenance HotUpdate bytes is not a positive integer".into());
    }
    if !is_sha256_text(&sha256) {
        return Err("trace runtime provenance HotUpdate SHA-256 is invalid".into());
    }
    Ok(RuntimeProvenance {
        plugin_version,
        oracle_manifest_schema_version,
        checkpoint_boundary,
        steam_app_id,
        steam_build: required_string(&provenance, "steamBuild")?,
        hot_update: HotUpdateProvenance {
            path,
            bytes,
            sha256,
        },
    })
}

pub fn is_sha256_text(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

/// validateOriginalOracleRuntimeProvenance：返回错误列表（空列表 = 通过）。
pub fn validate_original_oracle_runtime_provenance(
    actual: &RuntimeProvenance,
    expected: &RuntimeProvenance,
    label: &str,
) -> Vec<String> {
    let mut errors = Vec::new();
    if actual.plugin_version != expected.plugin_version {
        errors.push(format!(
            "{label} plugin version mismatch: actual={} expected={}",
            actual.plugin_version, expected.plugin_version
        ));
    }
    if actual.oracle_manifest_schema_version != expected.oracle_manifest_schema_version {
        errors.push(format!(
            "{label} oracle schema mismatch: actual={} expected={}",
            actual.oracle_manifest_schema_version, expected.oracle_manifest_schema_version
        ));
    }
    if actual.checkpoint_boundary != expected.checkpoint_boundary {
        errors.push(format!(
            "{label} checkpoint boundary mismatch: actual={} expected={}",
            actual.checkpoint_boundary, expected.checkpoint_boundary
        ));
    }
    if actual.steam_app_id != expected.steam_app_id {
        errors.push(format!(
            "{label} Steam app mismatch: actual={} expected={}",
            actual.steam_app_id, expected.steam_app_id
        ));
    }
    if actual.steam_build != expected.steam_build {
        errors.push(format!(
            "{label} Steam build mismatch: actual={} expected={}",
            actual.steam_build, expected.steam_build
        ));
    }
    if actual.hot_update.path != expected.hot_update.path {
        errors.push(format!(
            "{label} HotUpdate path mismatch: actual={} expected={}",
            actual.hot_update.path, expected.hot_update.path
        ));
    }
    if actual.hot_update.bytes != expected.hot_update.bytes {
        errors.push(format!(
            "{label} HotUpdate bytes mismatch: actual={} expected={}",
            actual.hot_update.bytes, expected.hot_update.bytes
        ));
    }
    if actual.hot_update.sha256 != expected.hot_update.sha256 {
        errors.push(format!(
            "{label} HotUpdate SHA-256 mismatch: actual={} expected={}",
            actual.hot_update.sha256, expected.hot_update.sha256
        ));
    }
    errors
}

pub fn compare_original_oracle_case(
    runtime_case: &OriginalOracleRuntimeCase,
    observation: Option<&OriginalOracleObservation>,
    engine: &OriginalOracleEngineRun,
    frozen_baseline: Option<&FrozenFullStateCase>,
    declarations: &[OriginalOracleContractBuffDeclaration],
) -> OriginalOracleCaseDiff {
    let Some(observation) = observation else {
        return case_diff(
            runtime_case,
            engine,
            "missing",
            false,
            false,
            false,
            None,
            None,
            Some("original oracle did not emit this case".to_string()),
        );
    };
    if observation.input_sha256 != runtime_case.battle_result_fingerprint {
        return case_diff(
            runtime_case,
            engine,
            "error",
            false,
            false,
            false,
            Some(observation),
            None,
            Some(format!(
                "oracle input fingerprint mismatch: trace={} manifest={}",
                observation.input_sha256, runtime_case.battle_result_fingerprint
            )),
        );
    }
    if let Some(error) = &observation.error {
        return case_diff(
            runtime_case,
            engine,
            "error",
            false,
            false,
            false,
            Some(observation),
            None,
            Some(format!("original oracle error: {error}")),
        );
    }
    let Some(terminal) = &observation.terminal else {
        return case_diff(
            runtime_case,
            engine,
            "missing",
            false,
            false,
            false,
            Some(observation),
            None,
            Some(
                if observation.timeout {
                    "original oracle timed out without terminal output".to_string()
                } else {
                    "original oracle terminal output is missing".to_string()
                },
            ),
        );
    };
    let full_state = assess_full_state(runtime_case, observation, frozen_baseline, declarations);
    if engine.result != "ran"
        || engine.winner.is_none()
        || engine.actor_turn.is_none()
        || engine.hp_delta.is_none()
    {
        return case_diff(
            runtime_case,
            engine,
            if engine.result == "blocked" { "blocked" } else { "error" },
            terminal_matches_expected(terminal, &runtime_case.expected),
            false,
            false,
            Some(observation),
            Some(full_state),
            Some(format!(
                "Rust {}: {}",
                engine.result,
                engine.message.clone().unwrap_or_else(|| "no terminal".to_string())
            )),
        );
    }

    let original_matches_fixture = terminal_matches_expected(terminal, &runtime_case.expected);
    let engine_terminal = OriginalOracleEngineTerminal {
        winner_side: engine.winner.expect("checked above").as_str(),
        actor_turn: engine.actor_turn.expect("checked above"),
        hp_delta: engine.hp_delta.expect("checked above"),
    };
    let engine_matches_original = engine_terminal.winner_side == terminal.winner_side.as_str()
        && engine_terminal.actor_turn == terminal.actor_turn
        && engine_terminal.hp_delta == terminal.hp_delta;
    let checkpoint_deviation =
        first_checkpoint_deviation(&observation.checkpoints, &engine.checkpoints);
    let checkpoints_match = checkpoint_deviation.is_none();
    let terminal_exact = original_matches_fixture && engine_matches_original && checkpoints_match;
    let first_deviation = checkpoint_deviation
        .or_else(|| {
            terminal_deviation(
                "original vs fixture",
                &OriginalOracleEngineTerminal {
                    winner_side: terminal.winner_side.as_str(),
                    actor_turn: terminal.actor_turn,
                    hp_delta: terminal.hp_delta,
                },
                &EngineShapedTerminal {
                    winner_side: runtime_case.expected.winner_side.as_str(),
                    actor_turn: runtime_case.expected.actor_turn_count,
                    hp_delta: runtime_case.expected.hp_delta_p1_minus_p2,
                },
            )
        })
        .or_else(|| {
            terminal_deviation("Rust vs original", &engine_terminal, &EngineShapedTerminal {
                winner_side: terminal.winner_side.as_str(),
                actor_turn: terminal.actor_turn,
                hp_delta: terminal.hp_delta,
            })
        })
        .or_else(|| full_state.first_deviation.clone());
    let status = if terminal_exact && full_state.baseline_ready && full_state.original_matches_baseline {
        "exact"
    } else if !terminal_exact {
        "mismatch"
    } else if full_state.full_state_obligation.is_some() {
        "blocked"
    } else {
        "mismatch"
    };
    case_diff(
        runtime_case,
        engine,
        status,
        original_matches_fixture,
        engine_matches_original,
        checkpoints_match,
        Some(observation),
        Some(full_state),
        first_deviation,
    )
}

pub fn compare_original_oracle_run(
    manifest: &OriginalOracleRuntimeManifest,
    run: &OriginalOracleRun,
    engines: &std::collections::BTreeMap<String, OriginalOracleEngineRun>,
    frozen_baselines: &std::collections::BTreeMap<String, FrozenFullStateCase>,
    declarations: &[OriginalOracleContractBuffDeclaration],
) -> OracleResult<Vec<OriginalOracleCaseDiff>> {
    let expected_order: Vec<&str> = manifest.cases.iter().map(|c| c.case_id.as_str()).collect();
    if run.case_order.iter().map(String::as_str).collect::<Vec<_>>() != expected_order {
        return Err(format!(
            "original-oracle case order mismatch: trace={} manifest={}",
            run.case_order.join(","),
            expected_order.join(",")
        ));
    }
    manifest
        .cases
        .iter()
        .map(|runtime_case| {
            let engine = engines
                .get(&runtime_case.case_id)
                .ok_or_else(|| format!("missing Rust engine run for {}", runtime_case.case_id))?;
            Ok(compare_original_oracle_case(
                runtime_case,
                run.observation(&runtime_case.case_id),
                engine,
                frozen_baselines.get(&runtime_case.case_id),
                declarations,
            ))
        })
        .collect()
}

// ---- 内部结构 ----

#[derive(Default)]
struct MutableObservation {
    case_id: String,
    input_sha256: String,
    checkpoints: Vec<OriginalOracleCheckpoint>,
    terminal: Option<OriginalOracleTerminal>,
    error: Option<String>,
    timeout: bool,
}

#[derive(Clone, Copy)]
struct OriginalOracleEngineTerminal<'a> {
    winner_side: &'a str,
    actor_turn: i64,
    hp_delta: i64,
}

/// terminalDeviation 的期望侧：既接受 fixture expected（actorTurnCount /
/// hpDeltaP1MinusP2），也接受 observation terminal（actorTurn / hpDelta）。
struct EngineShapedTerminal<'a> {
    winner_side: &'a str,
    actor_turn: i64,
    hp_delta: i64,
}

// 全状态义务只比较原版 trace 与冻结基线；Rust 逐帧全状态对照见
// dump-aligned-rust-full-state.ts。
#[derive(Debug, Clone, Default)]
struct FullStateAssessment {
    baseline_ready: bool,
    original_matches_baseline: bool,
    full_state_obligation: Option<String>,
    first_deviation: Option<String>,
}

fn assess_full_state(
    runtime_case: &OriginalOracleRuntimeCase,
    observation: &OriginalOracleObservation,
    frozen_baseline: Option<&FrozenFullStateCase>,
    declarations: &[OriginalOracleContractBuffDeclaration],
) -> FullStateAssessment {
    let no_full_state = FullStateAssessment::default();
    let Some(frozen_baseline) = frozen_baseline else {
        let obligation = format!(
            "{}: frozen original-client full-state baseline is missing",
            runtime_case.case_id
        );
        return FullStateAssessment {
            full_state_obligation: Some(obligation.clone()),
            first_deviation: Some(obligation),
            ..no_full_state
        };
    };
    if frozen_baseline.input_sha256 != runtime_case.battle_result_fingerprint {
        let obligation = format!(
            "{}: frozen full-state input fingerprint drift: baseline={} manifest={}",
            runtime_case.case_id, frozen_baseline.input_sha256, runtime_case.battle_result_fingerprint
        );
        return FullStateAssessment {
            full_state_obligation: Some(obligation.clone()),
            first_deviation: Some(obligation),
            ..no_full_state
        };
    }
    let original_projection = project_original_oracle_observation(observation, declarations);
    let Some(original) = original_projection.value else {
        let obligation = original_projection.obligation.unwrap_or_else(|| {
            format!(
                "{}: original full-state projection is unavailable",
                runtime_case.case_id
            )
        });
        return FullStateAssessment {
            full_state_obligation: Some(obligation.clone()),
            first_deviation: Some(obligation),
            ..no_full_state
        };
    };
    let comparison = compare_original_oracle_full_state(&frozen_baseline.projection, &original);
    FullStateAssessment {
        baseline_ready: true,
        original_matches_baseline: comparison.exact,
        full_state_obligation: None,
        first_deviation: if comparison.exact {
            None
        } else {
            Some(format!(
                "original/baseline full state {}",
                comparison.first_deviation.unwrap_or_default()
            ))
        },
    }
}

fn parse_jsonl(jsonl: &str) -> OracleResult<Vec<Value>> {
    let mut records = Vec::new();
    for (index, line) in jsonl.split('\n').map(str::trim_end).enumerate() {
        // TS split(/\r?\n/) 后逐行 trim 判空；这里先去掉行尾 \r。
        let line = line.trim_end_matches('\r');
        if line.trim().is_empty() {
            continue;
        }
        let parsed: Value = serde_json::from_str(line).map_err(|error| {
            format!("invalid trace JSONL line {}: {error}", index + 1)
        })?;
        if !parsed.is_object() {
            return Err(format!(
                "invalid trace JSONL line {}: record is not an object",
                index + 1
            ));
        }
        records.push(parsed);
    }
    Ok(records)
}

fn same_run(record: &Value, run_id: Option<&str>) -> bool {
    match run_id {
        None => record.get("runId").is_none(),
        Some(run_id) => record.get("runId").and_then(Value::as_str) == Some(run_id),
    }
}

fn require_run_identity(
    record: &Value,
    batch_id: &str,
    run_id: Option<&str>,
    kind: &str,
) -> OracleResult<()> {
    if record.get("batchId").and_then(Value::as_str) != Some(batch_id)
        || !same_run(record, run_id)
    {
        return Err(format!(
            "{kind} belongs to a foreign original-oracle run: batch={} run={}",
            js_value_string(record.get("batchId")),
            js_value_string(record.get("runId")),
        ));
    }
    Ok(())
}

fn require_active_observation<'a>(
    observations: &'a mut [(String, MutableObservation)],
    active_case_id: &str,
    kind: &str,
) -> OracleResult<&'a mut MutableObservation> {
    let (_, observation) = observations
        .iter_mut()
        .find(|(case_id, _)| case_id == active_case_id)
        .ok_or_else(|| format!("{kind} refers to unknown active case {active_case_id}"))?;
    Ok(observation)
}

fn require_case_identity(
    record: &Value,
    case_id: &str,
    input_sha256: &str,
    case_field: &str,
) -> OracleResult<()> {
    if required_string(record, case_field)? != case_id {
        return Err(format!(
            "original-oracle case identity mismatch: trace={} active={case_id}",
            js_value_string(record.get(case_field)),
        ));
    }
    let observed_input = optional_string(record.get("inputSha256"));
    let Some(observed_input) = observed_input else {
        return Err(format!("{case_id}: trace field inputSha256 is not a string"));
    };
    if observed_input != input_sha256 {
        return Err(format!(
            "{case_id}: input fingerprint changed inside original-oracle case: start={input_sha256} event={observed_input}"
        ));
    }
    Ok(())
}

fn checkpoint_from_record(record: &Value) -> OracleResult<OriginalOracleCheckpoint> {
    let card = Value::Object(required_object(record, "card")?.clone());
    let src = Value::Object(required_object(record, "src")?.clone());
    let dst = Value::Object(required_object(record, "dst")?.clone());
    let src_side = required_side(&src, "side")?;
    let dst_side = required_side(&dst, "side")?;
    if src_side == dst_side {
        return Err("original cardCompleted src/dst have the same side".into());
    }
    let p1_state = character_state_from_record(if src_side == Side::P1 { &src } else { &dst })?;
    let p2_state = character_state_from_record(if src_side == Side::P2 { &src } else { &dst })?;
    let checkpoint_boundary = optional_string(record.get("checkpointBoundary"));
    Ok(OriginalOracleCheckpoint {
        trace_sequence: required_number(record, "seq")?,
        checkpoint_boundary,
        actor_turn: required_number(record, "actorTurn")?,
        actor_side: src_side,
        card_id: required_number(&card, "id")?,
        p1_hp: p1_state.hp,
        p2_hp: p2_state.hp,
        p1_state,
        p2_state,
    })
}

fn character_state_from_record(record: &Value) -> OracleResult<OriginalOracleCharacterState> {
    let max_hp = optional_number(record.get("maxHp"));
    let def = optional_number(record.get("def"));
    let anima = optional_number(record.get("anima"));
    let action_again_per_round = optional_number(record.get("actionAgainPerRound"));
    let buffs = optional_number_record(record.get("buffs"))?;
    let active_slot_count = optional_number(record.get("activeSlotCount"));
    let active_deck = optional_deck(record.get("activeDeck"), "activeDeck")?;
    let card_queue = optional_deck(record.get("cardQueue"), "cardQueue")?;
    // The runtime plugin renamed the m_TempBattleCardItems snapshot from `deck`
    // to `cardQueue` (`activeDeck` is the separate m_BattleDeck grid); legacy
    // traces only carry `deck`, current traces only carry `cardQueue`.
    let deck = match optional_deck(record.get("deck"), "deck") {
        Ok(deck) => deck,
        Err(error) => return Err(error),
    }
    .or_else(|| card_queue.clone());
    Ok(OriginalOracleCharacterState {
        hp: required_number(record, "hp")?,
        max_hp,
        def,
        anima,
        action_again_per_round,
        buffs,
        active_slot_count,
        active_deck,
        card_queue,
        deck,
    })
}

fn terminal_from_record(record: &Value) -> OracleResult<OriginalOracleTerminal> {
    let states = terminal_states(record.get("state"))?;
    Ok(OriginalOracleTerminal {
        winner_side: required_side(record, "winnerSide")?,
        actor_turn: required_number(record, "actorTurn")?,
        hp_delta: required_number(record, "hpDelta")?,
        p1_hp: required_number(record, "p1Hp")?,
        p2_hp: required_number(record, "p2Hp")?,
        remaining_decision_params: required_number(record, "remainingDecisionParams")?,
        p1_state: states.0,
        p2_state: states.1,
    })
}

fn terminal_states(
    value: Option<&Value>,
) -> OracleResult<(Option<OriginalOracleCharacterState>, Option<OriginalOracleCharacterState>)> {
    let Some(value) = value else {
        return Ok((None, None));
    };
    let left = required_object(value, "left")?;
    let right = required_object(value, "right")?;
    let left_side = required_side(&Value::Object(left.clone()), "side")?;
    let right_side = required_side(&Value::Object(right.clone()), "side")?;
    if left_side == right_side {
        return Err("trace terminal states have the same side".into());
    }
    let left_value = Value::Object(left.clone());
    let right_value = Value::Object(right.clone());
    Ok((
        Some(character_state_from_record(if left_side == Side::P1 {
            &left_value
        } else {
            &right_value
        })?),
        Some(character_state_from_record(if left_side == Side::P2 {
            &left_value
        } else {
            &right_value
        })?),
    ))
}

fn freeze_observation(observation: MutableObservation) -> OriginalOracleObservation {
    OriginalOracleObservation {
        case_id: observation.case_id,
        input_sha256: observation.input_sha256,
        checkpoints: observation.checkpoints,
        terminal: observation.terminal,
        error: observation.error.filter(|error| !error.is_empty()),
        timeout: observation.timeout,
    }
}

fn terminal_matches_expected(
    terminal: &OriginalOracleTerminal,
    expected: &OriginalOracleExpected,
) -> bool {
    terminal.winner_side == expected.winner_side
        && terminal.actor_turn == expected.actor_turn_count
        && terminal.hp_delta == expected.hp_delta_p1_minus_p2
}

fn terminal_deviation(
    label: &str,
    actual: &OriginalOracleEngineTerminal<'_>,
    expected: &EngineShapedTerminal<'_>,
) -> Option<String> {
    if actual.winner_side != expected.winner_side {
        return Some(format!(
            "{label} winner: actual={} expected={}",
            actual.winner_side, expected.winner_side
        ));
    }
    if actual.actor_turn != expected.actor_turn {
        return Some(format!(
            "{label} actorTurn: actual={} expected={}",
            actual.actor_turn, expected.actor_turn
        ));
    }
    if actual.hp_delta != expected.hp_delta {
        return Some(format!(
            "{label} hpDelta: actual={} expected={}",
            actual.hp_delta, expected.hp_delta
        ));
    }
    None
}

fn first_checkpoint_deviation(
    original: &[OriginalOracleCheckpoint],
    engine: &[OriginalOracleEngineCheckpoint],
) -> Option<String> {
    let count = original.len().min(engine.len());
    for index in 0..count {
        let left = &original[index];
        let right = &engine[index];
        let values: [(&str, String, String); 5] = [
            ("actorTurn", left.actor_turn.to_string(), right.actor_turn.to_string()),
            ("actorSide", left.actor_side.as_str().to_string(), right.actor_side.as_str().to_string()),
            ("cardId", left.card_id.to_string(), right.card_id.to_string()),
            ("p1Hp", left.p1_hp.to_string(), right.p1_hp.to_string()),
            ("p2Hp", left.p2_hp.to_string(), right.p2_hp.to_string()),
        ];
        for (field, original_value, engine_value) in values {
            if original_value != engine_value {
                return Some(format!(
                    "cardCompleted[{index}] {field}: original={original_value} Rust={engine_value}"
                ));
            }
        }
    }
    if original.len() != engine.len() {
        return Some(format!(
            "cardCompleted count: original={} Rust={}",
            original.len(),
            engine.len()
        ));
    }
    None
}

#[allow(clippy::too_many_arguments)]
fn case_diff(
    runtime_case: &OriginalOracleRuntimeCase,
    engine: &OriginalOracleEngineRun,
    status: OriginalOracleDiffStatus,
    original_matches_fixture: bool,
    engine_matches_original: bool,
    checkpoints_match: bool,
    observation: Option<&OriginalOracleObservation>,
    full_state: Option<FullStateAssessment>,
    first_deviation: Option<String>,
) -> OriginalOracleCaseDiff {
    let full_state = full_state.unwrap_or_default();
    OriginalOracleCaseDiff {
        case_id: runtime_case.case_id.clone(),
        status,
        observation: observation.cloned(),
        engine: engine.clone(),
        fixture_expected: runtime_case.expected.clone(),
        original_matches_fixture,
        engine_matches_original,
        checkpoints_match,
        baseline_ready: full_state.baseline_ready,
        original_matches_baseline: full_state.original_matches_baseline,
        full_state_obligation: full_state.full_state_obligation,
        first_deviation: first_deviation.filter(|deviation| !deviation.is_empty()),
    }
}

// ---- JSONL/JSON 解析助手（错误文案与 TS 逐字一致） ----

fn required_object<'a>(record: &'a Value, key: &str) -> OracleResult<&'a Map<String, Value>> {
    record
        .get(key)
        .and_then(Value::as_object)
        .ok_or_else(|| format!("trace field {key} is not an object"))
}

fn required_string(record: &Value, key: &str) -> OracleResult<String> {
    match optional_string(record.get(key)) {
        Some(value) => Ok(value),
        None => Err(format!("trace field {key} is not a string")),
    }
}

fn optional_string(value: Option<&Value>) -> Option<String> {
    value.and_then(Value::as_str).map(str::to_string)
}

fn optional_number_record(value: Option<&Value>) -> OracleResult<Option<Map<String, Value>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(record) = value.as_object() else {
        return Err("trace character buffs is not an object".into());
    };
    for (key, entry) in record {
        if entry.as_i64().is_none() {
            return Err(format!(
                "trace character buff {key} is not a finite number"
            ));
        }
    }
    Ok(Some(record.clone()))
}

fn optional_deck(value: Option<&Value>, label: &str) -> OracleResult<Option<Vec<OriginalOracleDeckSlot>>> {
    let Some(value) = value else {
        return Ok(None);
    };
    let Some(entries) = value.as_array() else {
        return Err(format!("trace character {label} is not an array"));
    };
    let mut slots = Vec::with_capacity(entries.len());
    for (index, entry) in entries.iter().enumerate() {
        if entry.as_object().is_none() {
            return Err(format!("trace character {label}[{index}] is not an object"));
        }
        let skip = entry.get("skip").unwrap_or(&Value::Null);
        let had_used = entry.get("hadUsed").unwrap_or(&Value::Null);
        if !skip.is_boolean() || !had_used.is_boolean() {
            return Err(format!(
                "trace character {label}[{index}] flags are not boolean"
            ));
        }
        slots.push(OriginalOracleDeckSlot {
            id: required_number(entry, "id")?,
            grid: required_number(entry, "grid")?,
            skip: skip.as_bool().expect("checked above"),
            had_used: had_used.as_bool().expect("checked above"),
        });
    }
    Ok(Some(slots))
}

fn required_number(record: &Value, key: &str) -> OracleResult<i64> {
    record
        .get(key)
        .and_then(Value::as_i64)
        .ok_or_else(|| format!("trace field {key} is not a finite number"))
}

fn required_side(record: &Value, key: &str) -> OracleResult<Side> {
    match record.get(key).and_then(Value::as_str) {
        Some("p1") => Ok(Side::P1),
        Some("p2") => Ok(Side::P2),
        _ => Err(format!("trace field {key} is not p1/p2")),
    }
}

fn js_value_string(value: Option<&Value>) -> String {
    match value {
        None => "undefined".into(),
        Some(Value::Null) => "null".into(),
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
    }
}
