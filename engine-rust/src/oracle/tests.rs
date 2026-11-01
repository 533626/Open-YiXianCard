//! `original-oracle-rust-buff-map.test.ts` / `original-oracle-diff.test.ts`
//! 关键期望的 Rust 单测移植：映射重锚定漂移、extended 流解析 fail-closed、
//! compareSide 四分类、未映射零值过滤、trace 解析身份/漂移拒绝、逐 case
//! 比较（exact / blocked / 首分歧）、真实 fixture 的进程内引擎对照。

use super::buff_map::{
    compare_side, core_snapshot_value, parse_rust_extended_replay_event_stream,
    rust_snapshot_value, unmapped_original_buff_entries, validate_rust_buff_key_map, CoreField,
    OriginalSideSnapshot, RustBuffMapEntry, RustExtendedSnapshot, Side,
};
use super::diff::{
    compare_original_oracle_case, expected_original_oracle_runtime_provenance,
    parse_original_oracle_run, run_original_oracle_rust_engine,
    validate_original_oracle_runtime_provenance, OriginalOracleEngineCheckpoint,
    OriginalOracleEngineRun, OriginalOracleRuntimeCase,
};
use super::full_state::{
    project_original_oracle_observation, validate_original_oracle_contract_buff_declarations,
    FrozenFullStateCase, FrozenFullStateSource,
};
use serde_json::{json, Value};

fn archive() -> Value {
    json!({
    "schemaVersion": 1,
    "buffs": {
        "JiaGong": { "id": 2, "category": "Positive", "name": "加攻" },
        "JianQi": { "id": 625, "category": "Positive", "name": "剑气" },
        "TiPo": { "id": 10023, "category": "Permanent", "name": "体魄" },
    },
})
}

fn entry_with(overrides: Value) -> Value {
    let mut entry = json!({
        "buffTypeId": 2,
        "archiveEnum": "JiaGong",
        "originalName": "加攻",
        "category": "Positive",
        "rustKey": "attack_bonus",
        "rustLabel": "加攻",
    });
    let object = entry.as_object_mut().expect("entry object");
    for (key, value) in overrides.as_object().expect("overrides object") {
        object.insert(key.clone(), value.clone());
    }
    entry
}

fn map_with(entries: &[Value]) -> Value {
    json!({ "schemaVersion": 1, "count": entries.len(), "entries": entries })
}

fn zero_snapshot() -> RustExtendedSnapshot {
    RustExtendedSnapshot::default()
}

fn repo_root() -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn evidence_manifest() -> Value {
    serde_json::from_str(
        &std::fs::read_to_string(
            repo_root().join("research/original-game/EVIDENCE_MANIFEST.json"),
        )
        .expect("EVIDENCE_MANIFEST exists"),
    )
    .expect("EVIDENCE_MANIFEST parses")
}

fn oracle_contract_declarations() -> Vec<super::full_state::OriginalOracleContractBuffDeclaration> {
    validate_original_oracle_contract_buff_declarations(
        &serde_json::from_str(
            &std::fs::read_to_string(
                repo_root().join("battle-evaluator/data/modeled-original-buffs.json"),
            )
            .expect("modeled-original-buffs exists"),
        )
        .expect("modeled-original-buffs parses"),
        &serde_json::from_str(
            &std::fs::read_to_string(
                repo_root().join("battle-evaluator/data/original-build-profiles.json"),
            )
            .expect("original-build-profiles exists"),
        )
        .expect("original-build-profiles parses"),
        &evidence_manifest(),
    )
    .expect("modeled declarations validate")
}

// ---- validateRustBuffKeyMap 重锚定（裸键兜底 fail-closed） ----

#[test]
fn valid_map_passes_and_freezes_fields() {
    let map = validate_rust_buff_key_map(&map_with(&[entry_with(json!({}))]), &archive())
        .expect("valid map");
    assert_eq!(map.count, 1);
    assert_eq!(map.entries.len(), 1);
    assert_eq!(map.entries[0].buff_type_id, 2);
    assert_eq!(map.entries[0].archive_enum, "JiaGong");
    assert_eq!(map.entries[0].original_name.as_deref(), Some("加攻"));
    assert_eq!(map.entries[0].category, "Positive");
    assert_eq!(map.entries[0].rust_key, "attack_bonus");
    assert_eq!(map.entries[0].rust_label, "加攻");
}

#[test]
fn buff_type_id_drift_fails_closed() {
    let error = validate_rust_buff_key_map(
        &map_with(&[entry_with(json!({ "buffTypeId": 3 }))]),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(error.contains("archive id=2 map id=3"), "{error}");
}

#[test]
fn original_name_drift_fails_closed() {
    let error = validate_rust_buff_key_map(
        &map_with(&[entry_with(json!({ "originalName": "加功" }))]),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(error.contains("archive name=加攻 map name=加功"), "{error}");
}

#[test]
fn unknown_archive_enum_fails_closed() {
    let error = validate_rust_buff_key_map(
        &map_with(&[entry_with(json!({ "archiveEnum": "JiaGongX" }))]),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(error.contains("must be an object"), "{error}");
}

#[test]
fn unknown_rust_key_fails_closed() {
    let error = validate_rust_buff_key_map(
        &map_with(&[entry_with(json!({ "rustKey": "not_a_field" }))]),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(
        error.contains("rustKey not_a_field is not an extended snapshot field"),
        "{error}"
    );
}

#[test]
fn duplicate_buff_type_id_fails_closed() {
    let error = validate_rust_buff_key_map(
        &map_with(&[
            entry_with(json!({})),
            entry_with(json!({
                "archiveEnum": "JianQi",
                "originalName": "剑气",
                "rustKey": "sword_energy",
                "rustLabel": "剑气",
            })),
        ]),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(error.contains("duplicate buffTypeId 2"), "{error}");
}

#[test]
fn inconsistent_count_fails_closed() {
    let error = validate_rust_buff_key_map(
        &json!({ "schemaVersion": 1, "count": 2, "entries": [entry_with(json!({}))] }),
        &archive(),
    )
    .expect_err("drift must fail");
    assert!(error.contains("count is inconsistent"), "{error}");
}

// ---- parseRustExtendedReplayEventStream 缺字段/非法值 fail-closed ----

fn terminal() -> Value {
    json!({
    "winnerSide": "p1",
    "actorTurn": 12,
    "hpDelta": 5,
    "p1Hp": 5,
    "p2Hp": 0,
    "remainingDecisionParams": 0,
})
}

fn snapshot_json(overrides: Value) -> Value {
    let mut snapshot = serde_json::Map::new();
    for field in super::buff_map::RUST_EXTENDED_SNAPSHOT_FIELDS {
        snapshot.insert(field.to_string(), json!(0));
    }
    if let Value::Object(overrides) = overrides {
        for (key, value) in overrides {
            snapshot.insert(key, value);
        }
    }
    Value::Object(snapshot)
}

fn valid_stream_event(overrides: Value) -> Value {
    let mut event = json!({
        "kind": "cardCompleted",
        "turn": 1,
        "actor": "p1",
        "p1": snapshot_json(json!({ "hp": 80, "anima": 3 })),
        "p2": snapshot_json(json!({ "hp": 80 })),
        "p1_card_queue": [{ "id": 1000019, "grid": 0, "skip": false, "hadUsed": true }],
        "p2_card_queue": [],
        "p1_physical_deck": [{ "id": 1000019, "grid": 0, "skip": false, "hadUsed": true }],
        "p2_physical_deck": [],
    });
    let object = event.as_object_mut().expect("event object");
    for (key, value) in overrides.as_object().expect("overrides object") {
        object.insert(key.clone(), value.clone());
    }
    event
}

fn stream_with(events: Value) -> Value {
    json!({
        "fixture": "fixtures/x.json",
        "terminal": terminal(),
        "events": events,
    })
}

#[test]
fn valid_extended_stream_parses_with_all_fields() {
    let stream = parse_rust_extended_replay_event_stream(&stream_with(json!([
        valid_stream_event(json!({
            "card_id": 1000019,
            "card_name": "轻剑",
        }))
    ])))
    .expect("valid stream");
    assert_eq!(stream.events.len(), 1);
    assert_eq!(stream.events[0].p1.get("hp"), Some(80));
    assert_eq!(stream.events[0].p1.get("sword_energy"), Some(0));
    assert_eq!(stream.events[0].actor, Side::P1);
    assert_eq!(stream.events[0].card_id, Some(1000019));
    assert_eq!(
        stream.events[0].p1_card_queue,
        vec![super::buff_map::RustQueuedCard {
            id: 1000019,
            grid: 0,
            skip: false,
            had_used: true,
        }]
    );
    assert_eq!(stream.terminal.remaining_decision_params, 0);
}

#[test]
fn missing_terminal_fails_closed() {
    let value = json!({ "fixture": "fixtures/x.json", "events": [] });
    let error = parse_rust_extended_replay_event_stream(&value).expect_err("must fail");
    assert!(error.contains("stream.terminal"), "{error}");
}

#[test]
fn missing_snapshot_field_fails_closed() {
    let bad = stream_with(json!([{
        "kind": "cardCompleted",
        "turn": 1,
        "actor": "p1",
        "p1": { "hp": 80 },
        "p2": snapshot_json(json!({})),
    }]));
    let error = parse_rust_extended_replay_event_stream(&bad).expect_err("must fail");
    assert!(error.contains("must be a finite number"), "{error}");
}

#[test]
fn non_finite_snapshot_value_fails_closed() {
    let bad = stream_with(json!([{
        "kind": "cardCompleted",
        "turn": 1,
        "actor": "p1",
        "p1": snapshot_json(json!({ "hp": "NaN" })),
        "p2": snapshot_json(json!({})),
    }]));
    let error = parse_rust_extended_replay_event_stream(&bad).expect_err("must fail");
    assert!(error.contains("must be a finite number"), "{error}");
}

#[test]
fn invalid_actor_fails_closed() {
    let bad = stream_with(json!([{
        "kind": "cardCompleted",
        "turn": 1,
        "actor": "p3",
        "p1": snapshot_json(json!({})),
        "p2": snapshot_json(json!({})),
    }]));
    let error = parse_rust_extended_replay_event_stream(&bad).expect_err("must fail");
    assert!(error.contains("must be p1 or p2"), "{error}");
}

// ---- compareSide 四分类 ----

fn compare_entries() -> Vec<RustBuffMapEntry> {
    vec![
        RustBuffMapEntry {
            buff_type_id: 2,
            archive_enum: "JiaGong".into(),
            original_name: Some("加攻".into()),
            category: "Positive".into(),
            rust_key: "attack_bonus".into(),
            rust_label: "加攻".into(),
        },
        RustBuffMapEntry {
            buff_type_id: 625,
            archive_enum: "JianQi".into(),
            original_name: Some("剑气".into()),
            category: "Positive".into(),
            rust_key: "sword_energy".into(),
            rust_label: "剑气".into(),
        },
    ]
}

fn original_side_with(buffs: Value) -> OriginalSideSnapshot {
    OriginalSideSnapshot {
        hp: 80,
        max_hp: Some(80),
        def: Some(0),
        anima: Some(2),
        buffs: Some(buffs.as_object().expect("buffs object").clone()),
    }
}

#[test]
fn compare_side_match_produces_no_diffs() {
    let mut rust = zero_snapshot();
    rust.set("hp", 80);
    rust.set("max_hp", 80);
    rust.set("anima", 2);
    rust.set("attack_bonus", 3);
    rust.set("sword_energy", 4);
    let compared = compare_side(
        Side::P1,
        &original_side_with(json!({ "2": 3, "625": 4 })),
        &rust,
        &compare_entries(),
    );
    assert!(compared.core_diffs.is_empty());
    assert!(compared.buff_diffs.is_empty());
    assert!(compared.unmapped.is_empty());
}

#[test]
fn compare_side_only_original() {
    let compared = compare_side(
        Side::P1,
        &original_side_with(json!({ "2": 3 })),
        &zero_snapshot(),
        &compare_entries(),
    );
    assert_eq!(compared.buff_diffs.len(), 1);
    assert_eq!(compared.buff_diffs[0].original, 3);
    assert_eq!(compared.buff_diffs[0].rust, 0);
    assert_eq!(compared.buff_diffs[0].classification, "only-original");
}

#[test]
fn compare_side_only_rust() {
    let mut rust = zero_snapshot();
    rust.set("attack_bonus", 5);
    let compared = compare_side(
        Side::P2,
        &original_side_with(json!({})),
        &rust,
        &compare_entries(),
    );
    assert_eq!(compared.buff_diffs.len(), 1);
    assert_eq!(compared.buff_diffs[0].side, Side::P2);
    assert_eq!(compared.buff_diffs[0].original, 0);
    assert_eq!(compared.buff_diffs[0].rust, 5);
    assert_eq!(compared.buff_diffs[0].classification, "only-rust");
}

#[test]
fn compare_side_value_mismatch() {
    let mut rust = zero_snapshot();
    rust.set("attack_bonus", 2);
    let compared = compare_side(
        Side::P1,
        &original_side_with(json!({ "2": 7 })),
        &rust,
        &compare_entries(),
    );
    assert_eq!(compared.buff_diffs.len(), 1);
    assert_eq!(compared.buff_diffs[0].original, 7);
    assert_eq!(compared.buff_diffs[0].rust, 2);
    assert_eq!(compared.buff_diffs[0].classification, "value-mismatch");
}

#[test]
fn compare_side_core_diffs_and_missing_max_hp() {
    let mut rust = zero_snapshot();
    rust.set("hp", 80);
    rust.set("max_hp", 80);
    rust.set("anima", 2);
    let original = OriginalSideSnapshot {
        hp: 79,
        max_hp: Some(81),
        def: Some(1),
        anima: Some(3),
        buffs: None,
    };
    let compared = compare_side(Side::P1, &original, &rust, &compare_entries());
    let rendered: Vec<(Side, CoreField, i64, i64)> = compared
        .core_diffs
        .iter()
        .map(|diff| (diff.side, diff.field, diff.original, diff.rust))
        .collect();
    assert_eq!(
        rendered,
        vec![
            (Side::P1, CoreField::Hp, 79, 80),
            (Side::P1, CoreField::MaxHp, 81, 80),
            (Side::P1, CoreField::Def, 1, 0),
            (Side::P1, CoreField::Anima, 3, 2),
        ]
    );
    let result = std::panic::catch_unwind(|| {
        compare_side(
            Side::P1,
            &OriginalSideSnapshot {
                hp: 80,
                ..Default::default()
            },
            &zero_snapshot(),
            &compare_entries(),
        )
    });
    assert!(result.is_err(), "missing maxHp must throw");
}

// ---- unmappedOriginalBuffEntries 零值过滤与映射内排除 ----

#[test]
fn unmapped_filters_zero_and_mapped_keys() {
    let unmapped = unmapped_original_buff_entries(
        &json!({ "2": 0, "625": 3, "999": 0, "998": 2, "10023": 1 })
            .as_object()
            .expect("buffs object")
            .clone(),
        &compare_entries(),
    );
    assert_eq!(
        unmapped,
        vec![
            super::buff_map::UnmappedBuffEntry { buff_type_id: 998, value: 2 },
            super::buff_map::UnmappedBuffEntry { buff_type_id: 10023, value: 1 },
        ]
    );
}

#[test]
fn unmapped_all_zero_is_empty() {
    let unmapped = unmapped_original_buff_entries(
        &json!({ "999": 0, "998": 0 }).as_object().expect("buffs object").clone(),
        &compare_entries()[..1],
    );
    assert!(unmapped.is_empty());
}

// ---- 核心四元取值辅助 ----

#[test]
fn core_snapshot_value_missing_field_throws() {
    let state = OriginalSideSnapshot {
        hp: 80,
        ..Default::default()
    };
    assert_eq!(core_snapshot_value(&state, CoreField::Hp).unwrap(), 80);
    let state = OriginalSideSnapshot {
        hp: 80,
        max_hp: Some(90),
        ..Default::default()
    };
    assert_eq!(core_snapshot_value(&state, CoreField::MaxHp).unwrap(), 90);
    let state = OriginalSideSnapshot {
        hp: 80,
        ..Default::default()
    };
    assert!(core_snapshot_value(&state, CoreField::MaxHp)
        .unwrap_err()
        .contains("lacks maxHp"));
}

#[test]
fn rust_snapshot_value_maps_field_names() {
    let mut rust = zero_snapshot();
    rust.set("hp", 1);
    rust.set("max_hp", 2);
    rust.set("defense", 3);
    rust.set("anima", 4);
    assert_eq!(rust_snapshot_value(&rust, CoreField::Hp), 1);
    assert_eq!(rust_snapshot_value(&rust, CoreField::MaxHp), 2);
    assert_eq!(rust_snapshot_value(&rust, CoreField::Def), 3);
    assert_eq!(rust_snapshot_value(&rust, CoreField::Anima), 4);
}

// ---- original oracle diff control plane ----

fn runtime_case() -> OriginalOracleRuntimeCase {
    serde_json::from_value(json!({
        "caseId": "example/round-01",
        "fixture": "battle-evaluator/fixtures/candidates/example/round-01.json",
        "fixtureFingerprint": "fixture-sha",
        "expected": { "winnerSide": "p1", "actorTurnCount": 3, "hpDeltaP1MinusP2": 7 },
        "battleResultFile": "example__round-01.battle-result.json",
        "battleResultFingerprint": "input-sha",
        "evidenceKind": "real-replay-golden",
        "legality": "server-recorded",
    }))
    .expect("runtime case")
}

fn engine_run() -> OriginalOracleEngineRun {
    OriginalOracleEngineRun {
        result: "ran".to_string(),
        message: None,
        winner: Some(Side::P1),
        actor_turn: Some(3),
        hp_delta: Some(7),
        checkpoints: vec![OriginalOracleEngineCheckpoint {
            actor_turn: 1,
            actor_side: Side::P1,
            card_id: 123,
            p1_hp: 27,
            p2_hp: 20,
        }],
    }
}

fn sequence_events(lines: &mut Vec<String>, sequence: &mut i64, kind: &str, fields: Value) {
    *sequence += 1;
    let mut record = json!({ "seq": sequence, "kind": kind });
    let object = record.as_object_mut().expect("record object");
    for (key, value) in fields.as_object().expect("fields object") {
        object.insert(key.clone(), value.clone());
    }
    lines.push(record.to_string());
}

fn provenance() -> Value {
    serde_json::to_value(expected_original_oracle_runtime_provenance(&evidence_manifest()).expect("provenance"))
        .expect("provenance serializes")
}

fn batch_started(lines: &mut Vec<String>, sequence: &mut i64, batch_id: &str, run_id: Option<&str>) {
    let mut fields = json!({ "batchId": batch_id, "runtimeProvenance": provenance() });
    if let Some(run_id) = run_id {
        fields["runId"] = json!(run_id);
    }
    sequence_events(lines, sequence, "originalOracleBatchStarted", fields);
}

fn batch_completed(lines: &mut Vec<String>, sequence: &mut i64, batch_id: &str, run_id: Option<&str>) {
    let mut fields = json!({ "batchId": batch_id, "status": "completed", "runtimeProvenance": provenance() });
    if let Some(run_id) = run_id {
        fields["runId"] = json!(run_id);
    }
    sequence_events(lines, sequence, "originalOracleBatchCompleted", fields);
}

fn oracle_state(side: &str, hp: i64) -> Value {
    json!({
        "side": side,
        "hp": hp,
        "maxHp": 30,
        "def": if side == "p1" { 4 } else { 0 },
        "anima": if side == "p1" { 2 } else { 0 },
        "actionAgainPerRound": 1,
        "buffs": { "298": 6, "308": 2 },
        "activeSlotCount": 1,
        "activeDeck": [{ "id": 123, "grid": 0, "skip": false, "hadUsed": true }],
        "cardQueue": [{ "id": 123, "grid": 0, "skip": false, "hadUsed": true }],
    })
}

fn oracle_terminal_state(p1_hp: i64, p2_hp: i64) -> Value {
    json!({ "left": oracle_state("p1", p1_hp), "right": oracle_state("p2", p2_hp) })
}

#[test]
fn requires_a_unique_measured_runtime_provenance_at_batch_start() {
    let error = parse_original_oracle_run(
        &[
            json!({ "seq": 1, "kind": "originalOracleBatchStarted", "batchId": "batch" }),
            json!({ "seq": 2, "kind": "originalOracleBatchCompleted", "batchId": "batch", "status": "completed" }),
        ]
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join("\n"),
        None,
        None,
    )
    .expect_err("must fail");
    assert!(error.contains("trace field runtimeProvenance is not an object"), "{error}");
}

#[test]
fn rejects_runtime_provenance_drift_at_batch_completion() {
    let mut drifted = provenance();
    drifted["hotUpdate"]["sha256"] = json!("f".repeat(64));
    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleBatchCompleted",
        json!({
            "batchId": "batch",
            "status": "completed",
            "runtimeProvenance": drifted,
        }),
    );
    let error = parse_original_oracle_run(&lines.join("\n"), None, None).expect_err("must fail");
    assert!(
        error.contains("original-oracle batch completion HotUpdate SHA-256 mismatch"),
        "{error}"
    );
}

#[test]
fn rejects_duplicate_case_lifecycles_and_checkpoint_input_drift() {
    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    batch_completed(&mut lines, &mut sequence, "batch", None);
    let error = parse_original_oracle_run(&lines.join("\n"), None, None).expect_err("must fail");
    assert!(
        error.contains("started while example/round-01 is still active"),
        "{error}"
    );

    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "cardCompleted",
        json!({
            "batchId": "batch",
            "oracleCaseId": "example/round-01",
            "inputSha256": "other-input",
            "actorTurn": 1,
            "card": { "id": 123 },
            "src": oracle_state("p1", 27),
            "dst": oracle_state("p2", 20),
        }),
    );
    batch_completed(&mut lines, &mut sequence, "batch", None);
    let error = parse_original_oracle_run(&lines.join("\n"), None, None).expect_err("must fail");
    assert!(
        error.contains("input fingerprint changed inside original-oracle case"),
        "{error}"
    );

    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseTimeout",
        json!({ "batchId": "batch", "caseId": "example/round-01" }),
    );
    batch_completed(&mut lines, &mut sequence, "batch", None);
    let error = parse_original_oracle_run(&lines.join("\n"), None, None).expect_err("must fail");
    assert!(
        error.contains("example/round-01: trace field inputSha256 is not a string"),
        "{error}"
    );
}

#[test]
fn selects_latest_completed_batch_and_associates_card_checkpoints() {
    let mut lines = Vec::new();
    let mut sequence = 0;
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleBatchCompleted",
        json!({ "batchId": "older", "status": "completed" }),
    );
    batch_started(&mut lines, &mut sequence, "latest", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "latest", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "cardCompleted",
        json!({
            "actorTurn": 99,
            "card": { "id": 999 },
            "src": { "side": "p2", "hp": 1 },
            "dst": { "side": "p1", "hp": 1 },
        }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "cardCompleted",
        json!({
            "batchId": "latest",
            "oracleCaseId": "example/round-01",
            "inputSha256": "input-sha",
            "checkpointBoundary": "cardExecuteCompletedBeforeAwaitContinuation",
            "actorTurn": 1,
            "card": { "id": 123 },
            "src": {
                "side": "p1",
                "hp": 27,
                "maxHp": 30,
                "def": 4,
                "anima": 2,
                "actionAgainPerRound": 1,
                "buffs": { "3": 2 },
                "activeSlotCount": 1,
                "activeDeck": [{ "id": 123, "grid": 0, "skip": false, "hadUsed": true }],
                "cardQueue": [{ "id": 123, "grid": 0, "skip": false, "hadUsed": true }],
            },
            "dst": { "side": "p2", "hp": 20, "maxHp": 30, "def": 0, "anima": 0, "buffs": {} },
        }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseCompleted",
        json!({
            "batchId": "latest",
            "caseId": "example/round-01",
            "inputSha256": "input-sha",
            "winnerSide": "p1",
            "actorTurn": 3,
            "hpDelta": 7,
            "p1Hp": 27,
            "p2Hp": 20,
            "remainingDecisionParams": 0,
        }),
    );
    batch_completed(&mut lines, &mut sequence, "latest", None);
    let run = parse_original_oracle_run(&lines.join("\n"), None, None).expect("run parses");
    assert_eq!(run.batch_id, "latest");
    let observation = run.observation("example/round-01").expect("observation");
    assert_eq!(observation.checkpoints.len(), 1);
    let checkpoint = &observation.checkpoints[0];
    assert_eq!(checkpoint.trace_sequence, 5);
    assert_eq!(
        checkpoint.checkpoint_boundary.as_deref(),
        Some("cardExecuteCompletedBeforeAwaitContinuation")
    );
    assert_eq!(checkpoint.actor_turn, 1);
    assert_eq!(checkpoint.actor_side, Side::P1);
    assert_eq!(checkpoint.card_id, 123);
    assert_eq!(checkpoint.p1_hp, 27);
    assert_eq!(checkpoint.p2_hp, 20);
    assert_eq!(checkpoint.p1_state.hp, 27);
    assert_eq!(checkpoint.p1_state.max_hp, Some(30));
    assert_eq!(checkpoint.p1_state.action_again_per_round, Some(1));
    assert_eq!(checkpoint.p1_state.deck, checkpoint.p1_state.card_queue);
    assert_eq!(checkpoint.p2_state.max_hp, Some(30));
    assert_eq!(checkpoint.p2_state.def, Some(0));
    assert!(checkpoint.p2_state.card_queue.is_none());
    // 派生字段照 TS 断言：JSON 序列化形状里的键序（caseId 在前，timeout 在后）。
    let observation_json = serde_json::to_value(observation).expect("observation serializes");
    let keys: Vec<&str> = observation_json
        .as_object()
        .expect("object")
        .keys()
        .map(String::as_str)
        .collect();
    assert_eq!(
        keys,
        vec!["caseId", "inputSha256", "checkpoints", "terminal", "timeout"]
    );
}

#[test]
fn isolates_repeated_runs_that_share_one_stable_batch_id() {
    let mut lines = Vec::new();
    let mut sequence = 0;
    for (index, run_id) in ["run-a", "run-b"].iter().enumerate() {
        batch_started(&mut lines, &mut sequence, "stable-batch", Some(run_id));
        sequence_events(
            &mut lines,
            &mut sequence,
            "originalOracleCaseStarted",
            json!({
                "batchId": "stable-batch",
                "runId": run_id,
                "caseId": "example/round-01",
                "inputSha256": "input-sha",
            }),
        );
        sequence_events(
            &mut lines,
            &mut sequence,
            "originalOracleCaseCompleted",
            json!({
                "batchId": "stable-batch",
                "runId": run_id,
                "caseId": "example/round-01",
                "inputSha256": "input-sha",
                "winnerSide": if index == 0 { "p1" } else { "p2" },
                "actorTurn": 3,
                "hpDelta": if index == 0 { 7 } else { -7 },
                "p1Hp": if index == 0 { 27 } else { 20 },
                "p2Hp": if index == 0 { 20 } else { 27 },
                "remainingDecisionParams": 0,
            }),
        );
        batch_completed(&mut lines, &mut sequence, "stable-batch", Some(run_id));
    }
    let run =
        parse_original_oracle_run(&lines.join("\n"), Some("stable-batch"), Some("run-a"))
            .expect("run parses");
    assert_eq!(run.run_id.as_deref(), Some("run-a"));
    let observation = run.observation("example/round-01").expect("observation");
    assert_eq!(
        observation.terminal.as_ref().expect("terminal").winner_side,
        Side::P1
    );
}

fn frozen_baseline(
    observation: &super::diff::OriginalOracleObservation,
    trace_sha: &str,
) -> FrozenFullStateCase {
    let projection = project_original_oracle_observation(observation, &oracle_contract_declarations())
        .value
        .expect("projection");
    FrozenFullStateCase {
        projection,
        input_sha256: "input-sha".to_string(),
        source: FrozenFullStateSource {
            kind: "original-client-trace".to_string(),
            trace_sha256: trace_sha.to_string(),
            run_id: None,
        },
    }
}

#[test]
fn requires_input_identity_terminal_parity_and_checkpoint_parity() {
    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "cardCompleted",
        json!({
            "batchId": "batch",
            "oracleCaseId": "example/round-01",
            "inputSha256": "input-sha",
            "actorTurn": 1,
            "card": { "id": 123 },
            "src": oracle_state("p1", 27),
            "dst": oracle_state("p2", 20),
        }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseCompleted",
        json!({
            "batchId": "batch",
            "caseId": "example/round-01",
            "inputSha256": "input-sha",
            "winnerSide": "p1",
            "actorTurn": 3,
            "hpDelta": 7,
            "p1Hp": 27,
            "p2Hp": 20,
            "remainingDecisionParams": 0,
            "state": oracle_terminal_state(27, 20),
        }),
    );
    batch_completed(&mut lines, &mut sequence, "batch", None);
    let run = parse_original_oracle_run(&lines.join("\n"), None, None).expect("run parses");
    let observation = run.observation("example/round-01").expect("observation");
    let diff = compare_original_oracle_case(
        &runtime_case(),
        Some(observation),
        &engine_run(),
        Some(&frozen_baseline(observation, &"a".repeat(64))),
        &oracle_contract_declarations(),
    );
    assert_eq!(diff.status, "exact");
    assert!(diff.original_matches_fixture);
    assert!(diff.engine_matches_original);
    assert!(diff.checkpoints_match);
    assert!(diff.baseline_ready);
    assert!(diff.original_matches_baseline);

    let without_baseline = compare_original_oracle_case(
        &runtime_case(),
        Some(observation),
        &engine_run(),
        None,
        &oracle_contract_declarations(),
    );
    assert_eq!(without_baseline.status, "blocked");
    assert_eq!(
        without_baseline.full_state_obligation.as_deref(),
        Some("example/round-01: frozen original-client full-state baseline is missing")
    );
}

#[test]
fn reports_the_first_per_card_divergence_without_hiding_a_terminal_match() {
    let mut lines = Vec::new();
    let mut sequence = 0;
    batch_started(&mut lines, &mut sequence, "batch", None);
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseStarted",
        json!({ "batchId": "batch", "caseId": "example/round-01", "inputSha256": "input-sha" }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "cardCompleted",
        json!({
            "batchId": "batch",
            "oracleCaseId": "example/round-01",
            "inputSha256": "input-sha",
            "actorTurn": 1,
            "card": { "id": 123 },
            "src": oracle_state("p1", 26),
            "dst": oracle_state("p2", 20),
        }),
    );
    sequence_events(
        &mut lines,
        &mut sequence,
        "originalOracleCaseCompleted",
        json!({
            "batchId": "batch",
            "caseId": "example/round-01",
            "inputSha256": "input-sha",
            "winnerSide": "p1",
            "actorTurn": 3,
            "hpDelta": 7,
            "p1Hp": 27,
            "p2Hp": 20,
            "remainingDecisionParams": 0,
            "state": oracle_terminal_state(27, 20),
        }),
    );
    batch_completed(&mut lines, &mut sequence, "batch", None);
    let run = parse_original_oracle_run(&lines.join("\n"), None, None).expect("run parses");
    let observation = run.observation("example/round-01").expect("observation");
    let diff = compare_original_oracle_case(
        &runtime_case(),
        Some(observation),
        &engine_run(),
        Some(&frozen_baseline(observation, &"b".repeat(64))),
        &oracle_contract_declarations(),
    );
    assert_eq!(diff.status, "mismatch");
    assert!(diff.engine_matches_original);
    assert_eq!(
        diff.first_deviation.as_deref(),
        Some("cardCompleted[0] p1Hp: original=26 Rust=27")
    );
}

#[test]
fn provenance_validation_reports_each_drift_field() {
    let expected = expected_original_oracle_runtime_provenance(&evidence_manifest()).expect("provenance");
    let mut actual = expected.clone();
    actual.hot_update.sha256 = "f".repeat(64);
    actual.steam_build = "999".into();
    let errors = validate_original_oracle_runtime_provenance(&actual, &expected, "label");
    assert_eq!(errors.len(), 2);
    assert!(errors[0].contains("Steam build mismatch"), "{errors:?}");
    assert!(errors[1].contains("HotUpdate SHA-256 mismatch"), "{errors:?}");
    let identical = validate_original_oracle_runtime_provenance(&expected, &expected, "label");
    assert!(identical.is_empty());
}

// ---- original oracle Rust engine side（进程内） ----

#[test]
fn replays_an_admitted_fixture_through_the_extended_stream() {
    let fixture_path = repo_root()
        .join("engine-rust/tests/data-driven/eswmbqw_round-19.json");
    let run = run_original_oracle_rust_engine(&fixture_path);
    assert_eq!(run.result, "ran");
    assert_eq!(run.winner, Some(Side::P2));
    assert_eq!(run.actor_turn, Some(14));
    assert_eq!(run.hp_delta, Some(-100));
    assert!(!run.checkpoints.is_empty());
}

#[test]
fn reports_an_unreadable_fixture_as_an_engine_error_not_a_verdict() {
    let run = run_original_oracle_rust_engine(std::path::Path::new("/nonexistent/fixture.json"));
    assert_eq!(run.result, "error");
    assert!(run.checkpoints.is_empty());
}
