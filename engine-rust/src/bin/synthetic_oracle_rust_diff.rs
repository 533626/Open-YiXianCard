//! `synthetic_oracle_rust_diff`：`battle-evaluator/scripts/lib/
//! synthetic-oracle-rust-diff.ts` 的 Rust bin 包装。对一份 synthetic oracle
//! 源清单（`battle-evaluator/data/synthetic-oracle-cases/<batchId>.json`，
//! ts-wave5 起的数据单 owner）逐 case 跑 canonical 引擎对照：
//!
//! - `--baseline-report <admission.json>`：已准入回归路径，与冻结基线
//!   （originalTerminalWitness v1）比较 —— `compareSyntheticOracleRustBaselineCase`；
//! - `--trace <trace.jsonl> [--batch <id>] [--run <id>]`：新批次路径，与原作
//!   客户端观测比较 —— `compareSyntheticOracleRustCase`；
//! - `--observations <file>`：同一新批次路径，但观测取自 JSON 文件
//!   （`{"observations": {caseId: {checkpoints, terminal, error?, timeout?}}}`），
//!   供无历史 TraceLogs 时的同输入对拍；
//! - `--case <caseId>` 可重复，过滤只跑指定 case；
//! - 输出 `{"batchId": ..., "results": [{caseId, status, firstDeviation?}, ...],
//!   "summary": {exact, mismatch, error}}`（2 空格缩进 + 尾随换行），
//!   mismatch/error 不改退出码（判定结果全在 JSON 里，逐字节对拍以内容为准）。
//!
//! 用法：
//! ```text
//! synthetic_oracle_rust_diff --source <manifest.json> --baseline-report <admission.json> [--case <caseId>]... [--json <out>]
//! synthetic_oracle_rust_diff --source <manifest.json> --trace <trace.jsonl> [--batch <id>] [--run <id>] [--case <caseId>]... [--json <out>]
//! synthetic_oracle_rust_diff --source <manifest.json> --observations <file> [--case <caseId>]... [--json <out>]
//! ```

use std::path::{Path, PathBuf};

use serde_json::{Map, Value};

use yixian_engine::oracle::diff::{parse_original_oracle_run, OriginalOracleObservation};
use yixian_engine::oracle::synthetic::{
    compare_synthetic_oracle_rust_baseline_case, compare_synthetic_oracle_rust_case,
    SyntheticOracleCase, SyntheticOracleContext, SyntheticOracleOriginalBaselineCase,
};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("..");
    if let Err(error) = run(&repo_root, &args) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

struct Options {
    source: PathBuf,
    baseline_report: Option<PathBuf>,
    trace: Option<PathBuf>,
    observations: Option<PathBuf>,
    batch: Option<String>,
    run: Option<String>,
    cases: Vec<String>,
    json: Option<PathBuf>,
}

fn parse_args(args: &[String]) -> Result<Options, String> {
    let mut options = Options {
        source: PathBuf::new(),
        baseline_report: None,
        trace: None,
        observations: None,
        batch: None,
        run: None,
        cases: Vec::new(),
        json: None,
    };
    let mut index = 0;
    let mut mode_count = 0usize;
    while index < args.len() {
        let flag = args[index].as_str();
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{flag} requires a value"))?;
        match flag {
            "--source" => options.source = PathBuf::from(value),
            "--baseline-report" => {
                options.baseline_report = Some(PathBuf::from(value));
                mode_count += 1;
            }
            "--trace" => {
                options.trace = Some(PathBuf::from(value));
                mode_count += 1;
            }
            "--observations" => {
                options.observations = Some(PathBuf::from(value));
                mode_count += 1;
            }
            "--batch" => options.batch = Some(value.clone()),
            "--run" => options.run = Some(value.clone()),
            "--case" => options.cases.push(value.clone()),
            "--json" => options.json = Some(PathBuf::from(value)),
            other => return Err(format!("unknown argument {other}")),
        }
        index += 2;
    }
    if options.source.as_os_str().is_empty() {
        return Err("--source is required".into());
    }
    if mode_count != 1 {
        return Err("exactly one of --baseline-report, --trace, or --observations is required".into());
    }
    if (options.batch.is_some() || options.run.is_some()) && options.trace.is_none() {
        return Err("--batch/--run require --trace".into());
    }
    Ok(options)
}

fn run(repo_root: &Path, args: &[String]) -> Result<(), String> {
    let options = parse_args(args)?;
    let context = SyntheticOracleContext::load(repo_root)?;

    let source_text = std::fs::read_to_string(&options.source)
        .map_err(|error| format!("cannot read source manifest {}: {error}", options.source.display()))?;
    let source: Value = serde_json::from_str(&source_text)
        .map_err(|error| format!("source manifest JSON error: {error}"))?;
    let batch_id = source
        .get("batchId")
        .and_then(Value::as_str)
        .ok_or_else(|| "source manifest has no batchId".to_string())?
        .to_string();
    let mut source_cases = Vec::new();
    for case in source
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "source manifest has no cases".to_string())?
    {
        source_cases.push(SyntheticOracleCase::from_json(case)?);
    }

    let mut results = Vec::new();
    let mode = if options.baseline_report.is_some() {
        "baseline"
    } else if options.trace.is_some() {
        "trace"
    } else {
        "observations"
    };
    match (&options.baseline_report, &options.trace, &options.observations) {
        (Some(report_path), _, _) => {
            let report_text = std::fs::read_to_string(report_path)
                .map_err(|error| format!("cannot read admission report {}: {error}", report_path.display()))?;
            let report: Value = serde_json::from_str(&report_text)
                .map_err(|error| format!("admission report JSON error: {error}"))?;
            let baselines = report
                .get("originalBaseline")
                .and_then(|baseline| baseline.get("cases"))
                .and_then(Value::as_array)
                .ok_or_else(|| "admission report has no originalBaseline.cases".to_string())?
                .clone();
            for baseline_value in &baselines {
                let baseline: SyntheticOracleOriginalBaselineCase =
                    serde_json::from_value(baseline_value.clone())
                        .map_err(|error| format!("baseline case JSON error: {error}"))?;
                if !options.cases.is_empty() && !options.cases.contains(&baseline.case_id) {
                    continue;
                }
                let synthetic_case = source_cases
                    .iter()
                    .find(|case| case.case_id == baseline.case_id)
                    .ok_or_else(|| {
                        format!(
                            "source manifest {} has no case {}",
                            batch_id, baseline.case_id
                        )
                    })?;
                results.push(compare_synthetic_oracle_rust_baseline_case(
                    synthetic_case,
                    &baseline,
                    &context,
                ));
            }
        }
        (_, Some(trace_path), _) => {
            let trace_text = std::fs::read_to_string(trace_path)
                .map_err(|error| format!("cannot read trace {}: {error}", trace_path.display()))?;
            let run = parse_original_oracle_run(
                &trace_text,
                options.batch.as_deref(),
                options.run.as_deref(),
            )?;
            for synthetic_case in &source_cases {
                if !options.cases.is_empty() && !options.cases.contains(&synthetic_case.case_id) {
                    continue;
                }
                results.push(compare_synthetic_oracle_rust_case(
                    synthetic_case,
                    run.observation(&synthetic_case.case_id),
                    &context,
                ));
            }
        }
        (_, _, Some(observations_path)) => {
            let observations_text = std::fs::read_to_string(observations_path).map_err(|error| {
                format!(
                    "cannot read observations {}: {error}",
                    observations_path.display()
                )
            })?;
            let observations: Value = serde_json::from_str(&observations_text)
                .map_err(|error| format!("observations JSON error: {error}"))?;
            let observations = observations
                .get("observations")
                .and_then(Value::as_object)
                .ok_or_else(|| "observations file must carry an observations object".to_string())?;
            for synthetic_case in &source_cases {
                if !options.cases.is_empty() && !options.cases.contains(&synthetic_case.case_id) {
                    continue;
                }
                let observation = observations
                    .get(&synthetic_case.case_id)
                    .map(|value| {
                        serde_json::from_value::<OriginalOracleObservation>(value.clone())
                            .map_err(|error| {
                                format!(
                                    "observation for {} JSON error: {error}",
                                    synthetic_case.case_id
                                )
                            })
                    })
                    .transpose()?;
                results.push(compare_synthetic_oracle_rust_case(
                    synthetic_case,
                    observation.as_ref(),
                    &context,
                ));
            }
        }
        _ => unreachable!("parse_args enforces exactly one mode"),
    }

    let mut exact = 0usize;
    let mut mismatch = 0usize;
    let mut error = 0usize;
    let mut result_values = Vec::new();
    for result in &results {
        match result.status {
            "exact" => exact += 1,
            "mismatch" => mismatch += 1,
            _ => error += 1,
        }
        result_values.push(result.to_json());
    }
    let mut summary = Map::new();
    summary.insert("exact".into(), Value::from(exact));
    summary.insert("mismatch".into(), Value::from(mismatch));
    summary.insert("error".into(), Value::from(error));
    let mut output = Map::new();
    output.insert("batchId".into(), Value::String(batch_id));
    output.insert("mode".into(), Value::String(mode.to_string()));
    output.insert("results".into(), Value::Array(result_values));
    output.insert("summary".into(), Value::Object(summary));
    let rendered = format!("{}\n", serde_json::to_string_pretty(&Value::Object(output)).expect("report serializes"));
    match &options.json {
        Some(path) => {
            std::fs::write(path, rendered)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        }
        None => print!("{rendered}"),
    }
    Ok(())
}
