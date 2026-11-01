//! bin：original-oracle trace 切片（TS `original-oracle-trace-slice.ts` 的命令行面）。
//!
//! 语义核心在 `yixian_engine::oracle_trace`（逐字移植说明见该模块头注释）；
//! 本 bin 只做参数解析与 stdout 输出：canonical JSON（camelCase，与 TS 内存对象
//! 同形）单行打印，错误走 stderr 并退出 1，用法错误退出 2。
//!
//! 用法：
//! - 单切片：`oracle_trace_slice <trace.jsonl> --batch <batchId> --run <runId>`
//!   输出 `{ evidence: {batchId, runId, startSeq, endSeq, recordCount, sha256}, jsonl }`
//! - forward/reverse 报告：`oracle_trace_slice <trace.jsonl> --batch <batchId> --forward <runId> --reverse <runId>`
//!   输出 `{ format: "canonical-jsonl-seq-v1", combinedSha256, forward, reverse }`

use yixian_engine::oracle_trace::{
    build_original_oracle_trace_slice_report, slice_original_oracle_run_trace, stable_json,
};

fn usage() -> ! {
    eprintln!(
        "usage: oracle_trace_slice <trace.jsonl> --batch <batchId> (--run <runId> | --forward <runId> --reverse <runId>)"
    );
    std::process::exit(2);
}

struct Args {
    trace_path: String,
    batch_id: String,
    run: Option<String>,
    forward: Option<String>,
    reverse: Option<String>,
}

fn parse_args(argv: &[String]) -> Args {
    let mut trace_path = String::new();
    let mut batch_id = String::new();
    let mut run = None;
    let mut forward = None;
    let mut reverse = None;
    let mut i = 0;
    while i < argv.len() {
        let arg = argv[i].as_str();
        i += 1;
        let mut value = || {
            let v = argv.get(i).cloned();
            i += 1;
            v.unwrap_or_default()
        };
        match arg {
            "--batch" => batch_id = value(),
            "--run" => run = Some(value()),
            "--forward" => forward = Some(value()),
            "--reverse" => reverse = Some(value()),
            other => {
                if !other.starts_with("--") && trace_path.is_empty() {
                    trace_path = other.to_string();
                }
            }
        }
    }
    if trace_path.is_empty() || batch_id.is_empty() {
        usage();
    }
    Args {
        trace_path,
        batch_id,
        run,
        forward,
        reverse,
    }
}

fn fatal(message: &str) -> ! {
    eprintln!("{message}");
    std::process::exit(1);
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&argv);
    let jsonl = std::fs::read_to_string(&args.trace_path)
        .unwrap_or_else(|e| fatal(&format!("trace read failed: {}: {e}", args.trace_path)));

    match (&args.run, (&args.forward, &args.reverse)) {
        (Some(run_id), _) => {
            let slice = slice_original_oracle_run_trace(&jsonl, &args.batch_id, run_id)
                .unwrap_or_else(|e| fatal(&e.message));
            println!("{}", stable_json(&slice.to_json()));
        }
        (None, (Some(forward_id), Some(reverse_id))) => {
            let forward = slice_original_oracle_run_trace(&jsonl, &args.batch_id, forward_id)
                .unwrap_or_else(|e| fatal(&e.message));
            let reverse = slice_original_oracle_run_trace(&jsonl, &args.batch_id, reverse_id)
                .unwrap_or_else(|e| fatal(&e.message));
            let report = build_original_oracle_trace_slice_report(&forward.evidence, &reverse.evidence)
                .unwrap_or_else(|e| fatal(&e.message));
            println!("{}", stable_json(&report.to_json()));
        }
        (None, _) => usage(),
    }
}
