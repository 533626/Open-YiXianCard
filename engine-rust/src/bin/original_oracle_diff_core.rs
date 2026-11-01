//! `original_oracle_diff_core`：oracle 引擎对照核心 diff 的 Rust 进程内编排。
//!
//! 输入 runtime manifest、oracle trace JSONL 与冻结全状态基线；对 manifest 的
//! 每个 case 进程内跑引擎（`run_replay_fixture_with_detailed_events`，缺规则/
//! 缺决策记 blocked、其余记 error），解析 trace 选取最后完成的 batch/run，逐
//! case 产出与 TS `original-oracle-diff.ts` 判定字段同形的比较报告：
//!
//! `{"batchId": ..., "runId": ..., "status": ..., "caseOrder": [...],
//!   "runtimeProvenance": {...}, "cases": [OriginalOracleCaseDiff...]}`
//!
//! golden 模式（缺省）做 `compare_original_oracle_run`：case 顺序等价检查 +
//! 冻结全状态基线逐 case 对照。`--diagnosis` 模式对应 TS diagnosis 分支：无
//! case 顺序检查、无冻结基线（`assess_full_state` 走 missing 义务分支），逐
//! case 状态重映射由编排层（Python 入口）完成，本 bin 只产 raw 比较。
//!
//! 用法：
//! ```text
//! original_oracle_diff_core --manifest <manifest.json> --trace <trace.jsonl> \
//!   [--baseline <full-state-baseline.json>] [--repo-root <dir>] \
//!   [--batch-id <id>] [--run-id <id>] [--diagnosis] [--declarations <path>] \
//!   [--buff-map <path>] [--buff-archive <path>]
//! ```

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde_json::Value;

use yixian_engine::oracle::buff_map::{load_rust_buff_key_map, OracleResult};
use yixian_engine::oracle::diff::{
    compare_original_oracle_case, compare_original_oracle_run, parse_original_oracle_run,
    run_original_oracle_rust_engine, OriginalOracleRun,
};
use yixian_engine::oracle::full_state::{
    load_original_oracle_contract_buff_declarations, FrozenFullStateCase,
    OriginalOracleContractBuffDeclaration,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> OracleResult<()> {
    let mut manifest_path: Option<PathBuf> = None;
    let mut trace_path: Option<PathBuf> = None;
    let mut baseline_path: Option<PathBuf> = None;
    let mut repo_root = PathBuf::from(".");
    let mut batch_id: Option<String> = None;
    let mut run_id: Option<String> = None;
    let mut diagnosis = false;
    let mut declarations: Option<PathBuf> = None;
    let mut buff_map: Option<PathBuf> = None;
    let mut buff_archive: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} requires a value"));
        match arg.as_str() {
            "--manifest" => manifest_path = Some(PathBuf::from(value()?)),
            "--trace" => trace_path = Some(PathBuf::from(value()?)),
            "--baseline" => baseline_path = Some(PathBuf::from(value()?)),
            "--repo-root" => repo_root = PathBuf::from(value()?),
            "--batch-id" => batch_id = Some(value()?),
            "--run-id" => run_id = Some(value()?),
            "--diagnosis" => diagnosis = true,
            "--declarations" => declarations = Some(PathBuf::from(value()?)),
            "--buff-map" => buff_map = Some(PathBuf::from(value()?)),
            "--buff-archive" => buff_archive = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    let manifest_path = manifest_path.ok_or("--manifest is required")?;
    let trace_path = trace_path.ok_or("--trace is required")?;
    let baseline_path = if diagnosis {
        None
    } else {
        Some(baseline_path.ok_or("--baseline is required")?)
    };

    let manifest: Value = serde_json::from_str(
        &std::fs::read_to_string(&manifest_path)
            .map_err(|error| format!("manifest read failed: {error}"))?,
    )
    .map_err(|error| format!("manifest JSON error: {error}"))?;
    let manifest: yixian_engine::oracle::diff::OriginalOracleRuntimeManifest =
        serde_json::from_value(manifest).map_err(|error| format!("manifest is invalid: {error}"))?;

    let trace = std::fs::read_to_string(&trace_path)
        .map_err(|error| format!("trace read failed: {error}"))?;
    let run: OriginalOracleRun =
        parse_original_oracle_run(&trace, batch_id.as_deref(), run_id.as_deref())?;

    let frozen_baselines: BTreeMap<String, FrozenFullStateCase> = match &baseline_path {
        Some(path) => {
            let baseline: Value = serde_json::from_str(
                &std::fs::read_to_string(path)
                    .map_err(|error| format!("baseline read failed: {error}"))?,
            )
            .map_err(|error| format!("baseline JSON error: {error}"))?;
            baseline
                .get("cases")
                .and_then(Value::as_array)
                .ok_or_else(|| "baseline.cases must be an array".to_string())?
                .iter()
                .map(|case| {
                    let frozen: FrozenFullStateCase = serde_json::from_value(case.clone())
                        .map_err(|error| format!("frozen full-state case is invalid: {error}"))?;
                    Ok((frozen.projection.case_id.clone(), frozen))
                })
                .collect::<OracleResult<_>>()?
        }
        None => BTreeMap::new(),
    };

    // 全状态投影只在有冻结基线时消费声明与 buff 映射；diagnosis 无基线直接跳过。
    let declarations: Vec<OriginalOracleContractBuffDeclaration> = if diagnosis {
        Vec::new()
    } else {
        let declarations_path = declarations.unwrap_or_else(|| {
            repo_root.join("battle-evaluator/data/modeled-original-buffs.json")
        });
        load_original_oracle_contract_buff_declarations(
            &declarations_path,
            &repo_root.join("battle-evaluator/data/original-build-profiles.json"),
            &repo_root.join("research/original-game/EVIDENCE_MANIFEST.json"),
        )?
    };

    if !diagnosis {
        let map_path = buff_map.unwrap_or_else(|| {
            repo_root.join("battle-evaluator/generated/buff-id-rust-key-map.json")
        });
        let archive_path = buff_archive.unwrap_or_else(|| {
            repo_root.join("battle-evaluator/generated/buff-category-archive.json")
        });
        let _map = load_rust_buff_key_map(&map_path, &archive_path)?;
    }

    // fixture 路径与 TS fixturePathFromRuntimeCase 同规则：必须仓库相对且不出根。
    let mut engines = BTreeMap::new();
    for runtime_case in &manifest.cases {
        if runtime_case.fixture.starts_with('/') || runtime_case.fixture.contains("..") {
            return Err(format!(
                "{} runtime fixture path must be repo-relative",
                runtime_case.case_id
            ));
        }
        let fixture_path = repo_root.join(&runtime_case.fixture);
        engines.insert(
            runtime_case.case_id.clone(),
            run_original_oracle_rust_engine(&fixture_path),
        );
    }

    // diagnosis 模式对齐 TS diagnosis 分支：不做 case 顺序检查，逐 case 用
    // manifest 顺序比较（无冻结基线，全状态走 missing 义务分支）。
    let diffs = if diagnosis {
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
                    None,
                    &declarations,
                ))
            })
            .collect::<OracleResult<Vec<_>>>()?
    } else {
        compare_original_oracle_run(&manifest, &run, &engines, &frozen_baselines, &declarations)?
    };

    // 报告顶层形状：batch 概览 + 逐 case 比较数组。
    let mut report = serde_json::Map::new();
    report.insert("batchId".into(), Value::String(run.batch_id.clone()));
    if let Some(run_id) = &run.run_id {
        report.insert("runId".into(), Value::String(run_id.clone()));
    }
    report.insert("status".into(), Value::String(run.status.clone()));
    report.insert(
        "runtimeProvenance".into(),
        serde_json::to_value(&run.runtime_provenance).map_err(|error| error.to_string())?,
    );
    report.insert(
        "caseOrder".into(),
        Value::Array(
            run.case_order
                .iter()
                .map(|case_id| Value::String(case_id.clone()))
                .collect(),
        ),
    );
    report.insert(
        "cases".into(),
        serde_json::to_value(&diffs).map_err(|error| error.to_string())?,
    );
    println!("{}", Value::Object(report));
    Ok(())
}

fn print_usage() {
    println!(
        "Usage:\n  \
original_oracle_diff_core --manifest <manifest.json> --trace <trace.jsonl> \
[--baseline <full-state-baseline.json>] [--repo-root <dir>] [--batch-id <id>] [--run-id <id>] \
[--diagnosis] [--declarations <path>] [--buff-map <path>] [--buff-archive <path>]\n\n\
Emits one JSON report with per-case diffs (caseId/status/originalMatchesFixture/\
engineMatchesOriginal/checkpointsMatch/baselineReady/originalMatchesBaseline/\
fullStateObligation/firstDeviation). --diagnosis compares without the frozen \
baseline and without the case-order check, matching the TS diagnosis branch."
    );
}
