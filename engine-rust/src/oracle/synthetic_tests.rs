// synthetic oracle 对照的单测：装置与期望转译自
// battle-evaluator/__tests__/synthetic-oracle-rust-diff.test.ts（装置必读转
// Rust 单测）+ synthetic-oracle-witness/synthetic-oracle 的错误文案样例。
// 冻结基线直接读仓库内已准入 admission 报告与 data/synthetic-oracle-cases
// 的 JSON 源清单（数据单 owner），不依赖本地 trace。

use std::path::{Path, PathBuf};

use serde_json::Value;

use crate::oracle::diff::OriginalOracleObservation;
use crate::oracle::synthetic::{
    compare_synthetic_oracle_rust_baseline_case, compare_synthetic_oracle_rust_case,
    synthetic_oracle_input, SyntheticOracleCase, SyntheticOracleContext,
    SyntheticOracleOriginalBaselineCase,
};
use crate::oracle::buff_map::Side;

fn repo_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn context() -> SyntheticOracleContext {
    SyntheticOracleContext::load(&repo_root()).expect("synthetic oracle context loads")
}

fn data_root() -> PathBuf {
    repo_root().join("battle-evaluator")
}

fn load_json(path: &Path) -> Value {
    let text = std::fs::read_to_string(path).expect("test JSON reads");
    serde_json::from_str(&text).expect("test JSON parses")
}

/// 与 TS 测试同型的两条最新准入批（schema 5，v2/ts 引擎字段 + originalBaseline）。
const LATEST_BASELINE_SOURCES: [(&str, &str); 2] = [
    (
        "fate-128-water-spirit-sharpness-build-25093011",
        "fate-128-water-spirit-sharpness-build-25093011.json",
    ),
    (
        "fixture-categories-batch-029-sizhan-lethal-gate",
        "fixture-categories-batch-029-sizhan-lethal-gate.json",
    ),
];

fn source_manifest_path(batch_id: &str) -> PathBuf {
    data_root().join("data/synthetic-oracle-cases").join(format!("{batch_id}.json"))
}

fn admission_report_path(file: &str) -> PathBuf {
    data_root().join("generated/synthetic-oracle-admission").join(file)
}

fn raw_baseline_cases(report: &Value) -> Vec<Value> {
    report
        .get("originalBaseline")
        .and_then(|baseline| baseline.get("cases"))
        .and_then(Value::as_array)
        .expect("admission report carries an originalBaseline inventory")
        .clone()
}

fn baseline_cases(report: &Value) -> Vec<SyntheticOracleOriginalBaselineCase> {
    let mut parsed = Vec::new();
    for case in raw_baseline_cases(report) {
        parsed.push(
            serde_json::from_value(case)
                .expect("baseline case deserializes (camelCase wire contract)"),
        );
    }
    parsed
}

fn case_from_manifest(manifest: &Value, case_id: &str) -> SyntheticOracleCase {
    let cases = manifest.get("cases").and_then(Value::as_array).expect("manifest cases");
    let case = cases
        .iter()
        .find(|case| case.get("caseId").and_then(Value::as_str) == Some(case_id))
        .unwrap_or_else(|| panic!("manifest carries source case {case_id}"));
    SyntheticOracleCase::from_json(case).expect("source case deserializes")
}

/// TS "latest admitted original baselines are exact on the Rust canonical engine"。
#[test]
fn latest_admitted_original_baselines_are_exact_on_the_rust_canonical_engine() {
    let context = context();
    for (batch_id, report_file) in LATEST_BASELINE_SOURCES {
        let report = load_json(&admission_report_path(report_file));
        let baselines = baseline_cases(&report);
        assert!(
            !baselines.is_empty(),
            "{report_file} carries baseline cases"
        );
        let manifest = load_json(&source_manifest_path(batch_id));
        for baseline in &baselines {
            let synthetic_case = case_from_manifest(&manifest, &baseline.case_id);
            let diff = compare_synthetic_oracle_rust_baseline_case(&synthetic_case, baseline, &context);
            assert_eq!(diff.case_id, baseline.case_id);
            assert_eq!(diff.status, "exact", "{}: {:?}", baseline.case_id, diff.first_deviation);
            assert!(diff.first_deviation.is_none());
        }
    }
}

/// TS "legacy-boundary baselines are refused instead of approximated"。
#[test]
fn legacy_boundary_baselines_are_refused_instead_of_approximated() {
    let context = context();
    let (batch_id, report_file) = LATEST_BASELINE_SOURCES[1];
    let report = load_json(&admission_report_path(report_file));
    let baseline = &baseline_cases(&report)[0];
    let manifest = load_json(&source_manifest_path(batch_id));
    let synthetic_case = case_from_manifest(&manifest, &baseline.case_id);

    // 丢弃 checkpointBoundary，回到 legacy 边界 → error 而非近似换算。
    let mut legacy_value = raw_baseline_cases(&report).remove(0);
    if let Value::Object(object) = &mut legacy_value {
        object.remove("checkpointBoundary");
    }
    let legacy: SyntheticOracleOriginalBaselineCase =
        serde_json::from_value(legacy_value).expect("legacy baseline deserializes");

    let diff = compare_synthetic_oracle_rust_baseline_case(&synthetic_case, &legacy, &context);

    assert_eq!(diff.status, "error");
    let deviation = diff.first_deviation.expect("legacy refusal carries a reason");
    assert!(
        deviation.contains("recollect with the current runner"),
        "unexpected deviation: {deviation}"
    );
}

/// 见证 BuffType 无映射 fail-closed（runSyntheticOracleRustCase 的
/// witnessBuffEntries 前置校验，先于引擎运行）。
#[test]
fn witness_buff_without_rust_mapping_fails_closed() {
    let context = context();
    let manifest = load_json(&source_manifest_path(LATEST_BASELINE_SOURCES[1].0));
    let baseline = {
        let report = load_json(&admission_report_path(LATEST_BASELINE_SOURCES[1].1));
        baseline_cases(&report).remove(0)
    };
    // 用原始 JSON 改写 witness.buffIds（SyntheticOracleCase 只承载读取路径）。
    let mut case_value = manifest
        .get("cases")
        .and_then(Value::as_array)
        .and_then(|cases| {
            cases
                .iter()
                .find(|case| {
                    case.get("caseId").and_then(Value::as_str) == Some(baseline.case_id.as_str())
                })
                .cloned()
        })
        .expect("source case JSON");
    case_value["witness"]["buffIds"] = Value::Array(vec![Value::from(987_654_321)]);
    let unmapped = SyntheticOracleCase::from_json(&case_value).expect("mutated case deserializes");

    let diff = compare_synthetic_oracle_rust_baseline_case(&unmapped, &baseline, &context);
    assert_eq!(diff.status, "error");
    assert_eq!(
        diff.first_deviation.as_deref(),
        Some(
            "synthetic/card/385-lethal-gate-sizhan-converts-control witness BuffType 987654321 has no Rust mapping (buff-id-rust-key-map.json)"
        )
    );
}

/// 终局六元任一不一致即 mismatch（对照文案格式）。
#[test]
fn terminal_hp_drift_is_a_mismatch_with_typed_message() {
    let context = context();
    let (batch_id, report_file) = LATEST_BASELINE_SOURCES[1];
    let report = load_json(&admission_report_path(report_file));
    let mut baselines = baseline_cases(&report);
    let baseline = &mut baselines[0];
    let manifest = load_json(&source_manifest_path(batch_id));
    let synthetic_case = case_from_manifest(&manifest, &baseline.case_id);

    let original_p1_hp = baseline.terminal.p1_hp;
    baseline.terminal.p1_hp += 1;

    let diff = compare_synthetic_oracle_rust_baseline_case(&synthetic_case, baseline, &context);
    assert_eq!(diff.status, "mismatch");
    let expected = format!(
        "terminal.p1Hp: original={} Rust={original_p1_hp}",
        original_p1_hp + 1
    );
    assert_eq!(diff.first_deviation.as_deref(), Some(expected.as_str()));
}

/// 新批次路径（compareSyntheticOracleRustCase）：观测缺失 / 边界不符的
/// error 分类与 TS 文案逐字一致；exact 路径由观测取自当前引擎的构造 trace
/// 覆盖（历史 TraceLogs 已不在仓库，真实 trace 覆盖记录在回执）。
#[test]
fn original_observation_error_classes_match_ts_messages() {
    let context = context();
    let manifest = load_json(&source_manifest_path(LATEST_BASELINE_SOURCES[1].0));
    let synthetic_case = case_from_manifest(
        &manifest,
        "synthetic/card/385-lethal-gate-sizhan-converts-control",
    );

    let missing = compare_synthetic_oracle_rust_case(&synthetic_case, None, &context);
    assert_eq!(missing.status, "error");
    assert_eq!(missing.first_deviation.as_deref(), Some("original observation is missing"));

    let empty = || OriginalOracleObservation {
        case_id: synthetic_case.case_id.clone(),
        input_sha256: String::new(),
        checkpoints: Vec::new(),
        terminal: None,
        error: None,
        timeout: false,
    };

    // original error 分类。
    let mut with_error = empty();
    with_error.error = Some("trace record truncated".into());
    let error_diff = compare_synthetic_oracle_rust_case(&synthetic_case, Some(&with_error), &context);
    assert_eq!(error_diff.status, "error");
    assert_eq!(
        error_diff.first_deviation.as_deref(),
        Some("original error: trace record truncated")
    );

    // timeout 优先于 terminal 缺失。
    let mut timed_out = empty();
    timed_out.timeout = true;
    let timeout_diff = compare_synthetic_oracle_rust_case(&synthetic_case, Some(&timed_out), &context);
    assert_eq!(timeout_diff.status, "error");
    assert_eq!(
        timeout_diff.first_deviation.as_deref(),
        Some("original observation timed out")
    );

    // terminal 缺失。
    let no_terminal_diff = compare_synthetic_oracle_rust_case(&synthetic_case, Some(&empty()), &context);
    assert_eq!(no_terminal_diff.status, "error");
    assert_eq!(
        no_terminal_diff.first_deviation.as_deref(),
        Some("original terminal is missing")
    );
}

/// 新批次路径（compareSyntheticOracleRustCase）的 legacy 边界拒绝与 exact
/// 判定：观测取自当前引擎的构造 observation（历史 TraceLogs 已不在仓库，
/// 真实 trace 覆盖记录在回执）。
#[test]
fn original_observation_boundary_gate_and_exact_roundtrip() {
    let context = context();
    let manifest = load_json(&source_manifest_path(LATEST_BASELINE_SOURCES[1].0));
    let case_id = "synthetic/card/385-lethal-gate-sizhan-converts-control";
    let synthetic_case = case_from_manifest(&manifest, case_id);

    // 用当前引擎跑出的 checkpoint/terminal 构造原作观测形状。
    let run = crate::oracle::synthetic::run_synthetic_oracle_rust_case(&synthetic_case, &context)
        .expect("engine runs the case");
    let observation = observation_from_engine(case_id, &run);
    let legacy_observation = {
        let mut legacy = observation_from_engine(case_id, &run);
        for checkpoint in &mut legacy.checkpoints {
            checkpoint.checkpoint_boundary = Some("legacyCardCompleted".into());
        }
        legacy
    };

    // legacy 边界观测先被拒绝（不做近似换算）。
    let legacy_diff = compare_synthetic_oracle_rust_case(
        &synthetic_case,
        Some(&legacy_observation),
        &context,
    );
    assert_eq!(legacy_diff.status, "error");
    assert!(
        legacy_diff
            .first_deviation
            .as_deref()
            .unwrap_or_default()
            .contains("recollect with the current runner"),
        "unexpected deviation: {:?}",
        legacy_diff.first_deviation
    );

    // 当前边界观测 → exact（观测即引擎输出，逐键应全等）。
    let diff = compare_synthetic_oracle_rust_case(&synthetic_case, Some(&observation), &context);
    assert_eq!(diff.status, "exact", "{:?}", diff.first_deviation);

    // 终局漂移 → mismatch（新批次路径的对照文案）。
    let drifted_terminal = {
        let mut terminal = run.terminal.clone();
        terminal.hp_delta += 1;
        terminal
    };
    let drifted_observation = {
        let mut drifted = observation_from_engine(case_id, &run);
        drifted.terminal = Some(drifted_terminal);
        drifted
    };
    let drifted_diff =
        compare_synthetic_oracle_rust_case(&synthetic_case, Some(&drifted_observation), &context);
    assert_eq!(drifted_diff.status, "mismatch");
    let expected_drift = format!(
        "terminal.hpDelta: original={} Rust={}",
        run.terminal.hp_delta + 1,
        run.terminal.hp_delta
    );
    assert_eq!(drifted_diff.first_deviation.as_deref(), Some(expected_drift.as_str()));
}

fn observation_from_engine(
    case_id: &str,
    run: &crate::oracle::synthetic::SyntheticRustRunResult,
) -> crate::oracle::diff::OriginalOracleObservation {
    let boundary = Some(
        crate::oracle::synthetic::CARD_EXECUTE_COMPLETED_BEFORE_AWAIT_CONTINUATION.to_string(),
    );
    crate::oracle::diff::OriginalOracleObservation {
        case_id: case_id.to_string(),
        input_sha256: String::new(),
        // 全部 cardCompleted checkpoint 都带上，使 actorCardOrdinal 索引与
        // 引擎一致（selectWitnessCheckpoint 按行动方过滤后取第 N 张）。
        checkpoints: run
            .checkpoints
            .iter()
            .enumerate()
            .map(|(index, checkpoint)| crate::oracle::diff::OriginalOracleCheckpoint {
                trace_sequence: index as i64 + 1,
                checkpoint_boundary: boundary.clone(),
                actor_turn: checkpoint.actor_turn,
                actor_side: checkpoint.actor_side,
                card_id: checkpoint.card_id,
                p1_hp: checkpoint.p1_state.hp,
                p2_hp: checkpoint.p2_state.hp,
                p1_state: character_state_json_roundtrip(&checkpoint.p1_state),
                p2_state: character_state_json_roundtrip(&checkpoint.p2_state),
            })
            .collect(),
        terminal: Some(run.terminal.clone()),
        error: None,
        timeout: false,
    }
}

fn character_state_json_roundtrip(
    state: &crate::oracle::synthetic::ProjectableState,
) -> crate::oracle::diff::OriginalOracleCharacterState {
    let mut object = serde_json::Map::new();
    object.insert("hp".into(), Value::from(state.hp));
    if let Some(max_hp) = state.max_hp {
        object.insert("maxHp".into(), Value::from(max_hp));
    }
    if let Some(def) = state.def {
        object.insert("def".into(), Value::from(def));
    }
    if let Some(anima) = state.anima {
        object.insert("anima".into(), Value::from(anima));
    }
    if let Some(action_again) = state.action_again_per_round {
        object.insert("actionAgainPerRound".into(), Value::from(action_again));
    }
    if let Some(buffs) = &state.buffs {
        object.insert("buffs".into(), Value::Object(buffs.clone()));
    }
    if let Some(deck) = &state.deck {
        object.insert("cardQueue".into(), deck.clone());
    }
    serde_json::from_value(Value::Object(object)).expect("character state round-trips")
}

/// validateSyntheticCase 的 fail-closed 文案。
#[test]
fn synthetic_case_validation_matches_ts_messages() {
    let context = context();
    let manifest = load_json(&source_manifest_path(LATEST_BASELINE_SOURCES[1].0));
    let case_value = manifest
        .get("cases")
        .and_then(Value::as_array)
        .expect("cases")
        [0]
        .clone();

    // invalid evidence contract
    let mut mutated = case_value.clone();
    mutated["legality"] = Value::String("approx".into());
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "synthetic oracle case has an invalid evidence contract"
    );

    // invalid case id
    let mut mutated = case_value.clone();
    mutated["caseId"] = Value::String("not-synthetic".into());
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "invalid synthetic oracle case id: not-synthetic"
    );

    // 8 张牌约束
    let mut mutated = case_value.clone();
    mutated["players"]["p1"]["cards"] =
        Value::Array((0..7).map(|index| Value::from(1000 + index)).collect());
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "p1 synthetic deck must contain 8 cards"
    );

    // activeSlotCount 边界
    let mut mutated = case_value.clone();
    mutated["players"]["p1"]["activeSlotCount"] = Value::from(0);
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "p1 activeSlotCount must be in [1, 8]"
    );

    // 未知卡 id
    let mut mutated = case_value.clone();
    mutated["players"]["p1"]["cards"] = Value::Array(
        (0..8).map(|index| Value::from(if index == 0 { 42_000_001 } else { 1000 + index })).collect(),
    );
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "p1 synthetic deck contains unknown exact card id 42000001"
    );

    // 不支持的等级
    let mut mutated = case_value.clone();
    mutated["players"]["p1"]["level"] = Value::from(99);
    let case = SyntheticOracleCase::from_json(&mutated).expect("case parses");
    assert_eq!(
        synthetic_oracle_input(&case, &context).unwrap_err(),
        "p1 has unsupported level 99"
    );

    // 有效 case 通过校验并产出 fixture
    let case = SyntheticOracleCase::from_json(&case_value).expect("case parses");
    let input = synthetic_oracle_input(&case, &context).expect("valid case builds");
    assert_eq!(
        input.get("firstPlayerSide").and_then(Value::as_str),
        Some(Side::P1.as_str())
    );
}
