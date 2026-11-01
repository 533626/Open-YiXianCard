//! `oracle_full_state_diff`：`battle-evaluator/scripts/dump-aligned-rust-full-state.ts`
//! 的 Rust 逐字移植。逐 checkpoint 对照原作客户端 full-state trace 与引擎
//! （进程内 detailed 事件流，不走 spawn），语义与 TS 对齐：
//!
//! - 对照字段集：hp / maxHp / def / anima（逐帧四元）+ buffs 字典逐键；
//! - buffs 逐键经 buff-id-rust-key-map 映射后四分类
//!   （only-original / only-rust / value-mismatch / match），
//!   未映射原版 buff 走 unmapped 显式归类（引擎覆盖缺口，只报告不判败）；
//! - winner / actorTurn / hpDelta 保持 exact；终局三元 + 终局全量状态与
//!   checkpoint 对称 fail-closed；
//! - `--json <path>` 写 CaseReport（schemaVersion 1，顶层键序与 TS 逐字一致，
//!   2 空格缩进 + 尾随换行，与 `JSON.stringify(report, null, 2)` 逐字节对齐）；
//! - 退出码：任一核心 / 映射 buff / 终局不一致 → 1；未映射原版 buff 只报告。
//!
//! 复用 `src/oracle/`（wave-2 移植）：[`buff_map`] 的映射加载/compare_side/
//! archive 锚定、[`diff`] 的 trace JSONL 解析、[`events`] 的进程内扩展流。
//!
//! 用法（与 TS 逐字一致）：
//! ```text
//! oracle_full_state_diff <caseId> [fromIdx] [toIdx] [--manifest <path>] \
//!   [--trace <path>] [--json <report.json>] [--batch <batchId>] \
//!   [--run <runId>] [--fixture <fixture.json>]
//! ```
//! 默认 trace 为 TraceLogs 下 mtime 最新的 `yixian-trace-*.jsonl`（只读）；
//! 默认 manifest 为 TraceLogs/original-oracle-cases/oracle-manifest.json；
//! 显式 `--fixture` 时不读 manifest，历史批次另用 `--batch/--run`。

use std::path::{Component, Path, PathBuf};

use serde_json::{Map, Value};

use yixian_engine::oracle::buff_map::{
    compare_side, load_rust_buff_key_map, original_buff_archive_anchor, BuffDiff, CoreDiff, Side,
    UnmappedBuffEntry,
};
use yixian_engine::oracle::diff::parse_original_oracle_run;
use yixian_engine::oracle::events::run_replay_event_stream_extended;

/// `$HOME` 之后的 traceRoot 段（TS gameRoot/BepInEx/TraceLogs）。
const TRACE_ROOT_PARTS: [&str; 7] = [
    ".steam",
    "debian-installation",
    "steamapps",
    "common",
    "YiXianPai",
    "BepInEx",
    "TraceLogs",
];

struct Options {
    case_id: String,
    from: i64,
    to: i64,
    manifest_path: PathBuf,
    trace_path: Option<PathBuf>,
    json_path: Option<PathBuf>,
    batch_id: Option<String>,
    run_id: Option<String>,
    fixture_path: Option<PathBuf>,
}

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust lives inside the repo")
        .to_path_buf();
    let home = std::env::var("HOME").unwrap_or_else(|_| "/home/user".to_string());
    let trace_root = TRACE_ROOT_PARTS
        .iter()
        .collect::<PathBuf>()
        .to_str()
        .map(|relative| PathBuf::from(&home).join(relative))
        .expect("trace root parts are utf-8");
    let options = parse_args(&trace_root)?;
    let trace_path = match &options.trace_path {
        Some(path) => path.clone(),
        None => latest_trace_path(&trace_root)?,
    };
    let trace_bytes = std::fs::read(&trace_path)
        .map_err(|error| format!("read trace {}: {error}", trace_path.display()))?;
    let trace_sha256 = sha256_hex(&trace_bytes);
    let trace_text = String::from_utf8(trace_bytes)
        .map_err(|_| "trace bytes are not valid UTF-8".to_string())?;

    // manifest 每次 stage 只含最近一批（golden runner 结束后还会清掉整个目录）；
    // 显式 --fixture 时不再读 manifest；缺失时给可诊断错误而非裸 ENOENT。
    let fixture_path: PathBuf = match &options.fixture_path {
        Some(path) => path.clone(),
        None => {
            let runtime_case =
                resolve_runtime_case_from_manifest(&options.manifest_path, &options.case_id)?;
            match runtime_case {
                Some(fixture) => {
                    fixture_path_from_runtime_case(&repo_root, &options.case_id, &fixture)?
                }
                None => {
                    return Err(format!(
                        "case {} not in {} (pass --fixture <fixture.json> for a historical batch)",
                        options.case_id,
                        options.manifest_path.display()
                    ))
                }
            }
        }
    };

    let run = parse_original_oracle_run(
        &trace_text,
        options.batch_id.as_deref(),
        options.run_id.as_deref(),
    )?;
    let obs = run
        .observation(&options.case_id)
        .ok_or_else(|| format!("trace has no observation for {}", options.case_id))?;
    let map_path = repo_root.join("battle-evaluator/generated/buff-id-rust-key-map.json");
    let archive_path = repo_root.join("battle-evaluator/generated/buff-category-archive.json");
    let map = load_rust_buff_key_map(&map_path, &archive_path)?;
    let archive: Value = serde_json::from_str(
        &std::fs::read_to_string(&archive_path)
            .map_err(|error| format!("buff-category-archive read failed: {error}"))?,
    )
    .map_err(|error| format!("buff-category-archive JSON error: {error}"))?;

    let fixture = yixian_engine::load_fixture_file(&fixture_path)
        .map_err(|error| format!("Rust extended event replay failed (1): Error: {error}"))?;
    let stream = run_replay_event_stream_extended(&fixture, &fixture_path.to_string_lossy())
        .map_err(|error| format!("Rust extended event replay failed (1): Error: {error}"))?;
    let rust_checkpoints: Vec<&yixian_engine::oracle::buff_map::RustExtendedEvent> = stream
        .events
        .iter()
        .filter(|event| event.kind == "cardCompleted")
        .collect();

    println!(
        "case {}: original checkpoints={} rust cardCompleted={}",
        options.case_id,
        obs.checkpoints.len(),
        rust_checkpoints.len()
    );
    println!("trace | {}", trace_path.display());
    println!("rust  | fixture {}", fixture_path.display());

    let count = (obs.checkpoints.len().max(rust_checkpoints.len())) as i64;
    let mut core_diffs_all: Vec<CoreDiff> = Vec::new();
    let mut buff_diffs_all: Vec<BuffDiff> = Vec::new();
    let mut checkpoint_reports: Vec<Value> = Vec::new();
    let mut unmapped_ids: Vec<i64> = Vec::new();
    let mut compared_checkpoints = 0usize;
    let mut identity_mismatch_count = 0usize;
    let start = options.from.max(0);
    let end = options.to.min(count - 1);
    let mut i = start;
    while i <= end {
        let index = i as usize;
        let original = obs.checkpoints.get(index);
        let rust = rust_checkpoints.get(index).copied();
        let original_str = original
            .map(|o| {
                format!(
                    "turn={} side={} card={} p1Hp={} p2Hp={}",
                    o.actor_turn,
                    o.actor_side.as_str(),
                    o.card_id,
                    o.p1_hp,
                    o.p2_hp
                )
            })
            .unwrap_or_else(|| "(none)".to_string());
        let rust_str = rust
            .map(|r| {
                format!(
                    "turn={} actor={} card={} p1Hp={} p2Hp={}",
                    r.turn,
                    r.actor.as_str(),
                    r.card_id
                        .map(|card_id| card_id.to_string())
                        .unwrap_or_else(|| "?".to_string()),
                    r.p1.get("hp").unwrap_or(0),
                    r.p2.get("hp").unwrap_or(0)
                )
            })
            .unwrap_or_else(|| "(none)".to_string());
        let identity_diverged = match (original, rust) {
            (Some(o), Some(r)) => {
                o.actor_turn != r.turn
                    || o.actor_side != r.actor
                    // TS：o.cardId !== (r.cardId ?? "?") —— r 无 cardId 时必 diverge。
                    || r.card_id.map_or(true, |card_id| o.card_id != card_id)
            }
            _ => false,
        };
        if identity_diverged {
            identity_mismatch_count += 1;
        }
        let mut side_diffs: Vec<CoreDiff> = Vec::new();
        let mut buff_side_diffs: Vec<BuffDiff> = Vec::new();
        let mut unmapped_side: Vec<(Side, UnmappedBuffEntry)> = Vec::new();
        if let (Some(o), Some(r)) = (original, rust) {
            compared_checkpoints += 1;
            for (side, original_state, rust_state) in [
                (Side::P1, &o.p1_state, &r.p1),
                (Side::P2, &o.p2_state, &r.p2),
            ] {
                let original_snapshot =
                    original_side_snapshot(original_state);
                let compared = compare_side(side, &original_snapshot, rust_state, &map.entries);
                side_diffs.extend(compared.core_diffs);
                buff_side_diffs.extend(compared.buff_diffs);
                unmapped_side.extend(compared.unmapped.into_iter().map(|entry| (side, entry)));
            }
        }
        core_diffs_all.extend(side_diffs.iter().cloned());
        buff_diffs_all.extend(buff_side_diffs.iter().cloned());
        let mark = if identity_diverged || !side_diffs.is_empty() {
            " *"
        } else if !buff_side_diffs.is_empty() {
            " ~"
        } else if !unmapped_side.is_empty() {
            " +"
        } else {
            "  "
        };
        println!("[{i:>3}]{mark} orig {original_str}");
        println!("       rust {rust_str}");
        if identity_diverged {
            let o = original.expect("identity_diverged implies both sides");
            let r = rust.expect("identity_diverged implies both sides");
            println!(
                "       identity mismatch: turn {}->{} actor {}->{} card {}->{}",
                o.actor_turn,
                r.turn,
                o.actor_side.as_str(),
                r.actor.as_str(),
                o.card_id,
                r.card_id
                    .map(|card_id| card_id.to_string())
                    .unwrap_or_else(|| "?".to_string())
            );
        }
        for diff in &side_diffs {
            println!(
                "       drift {}.{} orig={} rust={}",
                diff.side.as_str(),
                diff.field.as_str(),
                diff.original,
                diff.rust
            );
        }
        for diff in &buff_side_diffs {
            println!("       drift {} {}", diff.side.as_str(), format_buff(diff));
        }
        for (side, unmapped) in &unmapped_side {
            let anchor = original_buff_archive_anchor(unmapped.buff_type_id, &archive)?;
            let anchor_text = match anchor {
                Some(anchor) => format!(
                    "{} {} ({})",
                    anchor.archive_enum,
                    anchor.original_name.as_deref().unwrap_or("?"),
                    anchor.category
                ),
                None => "not-in-archive".to_string(),
            };
            println!(
                "       unmapped {}.buff[{} {}] = {}",
                side.as_str(),
                unmapped.buff_type_id,
                anchor_text,
                unmapped.value
            );
            if !unmapped_ids.contains(&unmapped.buff_type_id) {
                unmapped_ids.push(unmapped.buff_type_id);
            }
        }
        checkpoint_reports.push(checkpoint_report_json(
            i,
            &side_diffs,
            &buff_side_diffs,
            &unmapped_side,
        ));
        i += 1;
    }

    // 终局三元 exact + 终局全量状态对照。
    let mut terminal_exact = false;
    let mut terminal_core_diffs: Vec<CoreDiff> = Vec::new();
    let mut terminal_buff_diffs: Vec<BuffDiff> = Vec::new();
    if let Some(terminal) = &obs.terminal {
        let rust_end = stream
            .events
            .iter()
            .rev()
            .find(|event| event.kind == "battleEnd");
        terminal_exact = match rust_end {
            Some(end) => {
                terminal.winner_side == end.actor
                    && terminal.actor_turn == end.turn
                    && terminal.hp_delta
                        == end.p1.get("hp").unwrap_or(0) - end.p2.get("hp").unwrap_or(0)
            }
            // TS：winner 对 undefined / hpDelta 对 NaN 的比较恒为 false。
            None => false,
        };
        if let Some(end) = rust_end {
            for (side, original_state, rust_state) in [
                (Side::P1, terminal.p1_state.as_ref(), &end.p1),
                (Side::P2, terminal.p2_state.as_ref(), &end.p2),
            ] {
                let Some(original_state) = original_state else {
                    continue;
                };
                let original_snapshot = original_side_snapshot(original_state);
                let compared = compare_side(side, &original_snapshot, rust_state, &map.entries);
                terminal_core_diffs.extend(compared.core_diffs);
                terminal_buff_diffs.extend(compared.buff_diffs);
            }
        }
        println!(
            "terminal | winner={} actorTurn={} hpDelta={} rustWinner={} rustTurn={} exact={}",
            terminal.winner_side.as_str(),
            terminal.actor_turn,
            terminal.hp_delta,
            rust_end
                .map(|end| end.actor.as_str())
                .unwrap_or("?"),
            rust_end
                .map(|end| end.turn.to_string())
                .unwrap_or_else(|| "?".to_string()),
            terminal_exact
        );
        for diff in &terminal_core_diffs {
            println!(
                "       terminal drift {}.{} orig={} rust={}",
                diff.side.as_str(),
                diff.field.as_str(),
                diff.original,
                diff.rust
            );
        }
        for diff in &terminal_buff_diffs {
            println!(
                "       terminal drift {} {}",
                diff.side.as_str(),
                format_buff(diff)
            );
        }
        if rust_end.is_some()
            && terminal
                .p1_state
                .as_ref()
                .map_or(true, |state| state.buffs.is_none())
        {
            println!(
                "       note: terminal buffs absent — 终局 buffs 无法对照，按 fail-closed 判定失败（采集需 CaptureBuffSnapshot=true）"
            );
        }
    }

    let mapped_compared = compared_checkpoints * 2 * map.entries.len();
    let only_original = buff_diffs_all
        .iter()
        .filter(|diff| diff.classification == "only-original")
        .count();
    let only_rust = buff_diffs_all
        .iter()
        .filter(|diff| diff.classification == "only-rust")
        .count();
    let value_mismatch = buff_diffs_all
        .iter()
        .filter(|diff| diff.classification == "value-mismatch")
        .count();
    println!(
        "summary | core mismatch={} mapped buffs onlyOriginal={} onlyRust={} valueMismatch={} unmappedIds={} terminalExact={}",
        core_diffs_all.len(),
        only_original,
        only_rust,
        value_mismatch,
        unmapped_ids.len(),
        terminal_exact
    );

    let failed = obs.checkpoints.len() != rust_checkpoints.len()
        || identity_mismatch_count > 0
        || !core_diffs_all.is_empty()
        || !terminal_core_diffs.is_empty()
        || !terminal_buff_diffs.is_empty()
        || only_original > 0
        || only_rust > 0
        || value_mismatch > 0
        || !terminal_exact;

    if let Some(json_path) = &options.json_path {
    let report = case_report_json(
        &options,
        &trace_path,
        &trace_sha256,
        &run,
        &fixture_path,
        obs,
        &archive,
        rust_checkpoints.len(),
            compared_checkpoints,
            &core_diffs_all,
            (mapped_compared, only_original, only_rust, value_mismatch),
            &unmapped_ids,
            terminal_exact,
            &checkpoint_reports,
            failed,
        )?;
        let text = serde_json::to_string_pretty(&report).expect("report serializes");
        std::fs::write(json_path, format!("{text}\n"))
            .map_err(|error| format!("write report {}: {error}", json_path.display()))?;
        println!("report | {}", json_path.display());
    }

    if failed {
        println!("FAILED: full-state comparison is not exact");
        std::process::exit(1);
    }
    println!("OK: full-state comparison exact (unmapped original buffs are engine coverage gaps)");
    Ok(())
}

fn parse_args(trace_root: &Path) -> Result<Options, String> {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let mut positionals: Vec<String> = Vec::new();
    let mut manifest_path: Option<String> = None;
    let mut trace_path: Option<String> = None;
    let mut json_path: Option<String> = None;
    let mut batch_id: Option<String> = None;
    let mut run_id: Option<String> = None;
    let mut fixture_path: Option<String> = None;
    let mut index = 0usize;
    while index < argv.len() {
        let arg = argv[index].clone();
        match arg.as_str() {
            "--manifest" => {
                index += 1;
                if index < argv.len() {
                    manifest_path = Some(argv[index].clone());
                }
            }
            "--trace" => {
                index += 1;
                if index < argv.len() {
                    trace_path = Some(argv[index].clone());
                }
            }
            "--json" => {
                index += 1;
                if index < argv.len() {
                    json_path = Some(argv[index].clone());
                }
            }
            "--batch" => {
                index += 1;
                if index < argv.len() {
                    batch_id = Some(argv[index].clone());
                }
            }
            "--run" => {
                index += 1;
                if index < argv.len() {
                    run_id = Some(argv[index].clone());
                }
            }
            "--fixture" => {
                index += 1;
                if index < argv.len() {
                    fixture_path = Some(argv[index].clone());
                }
            }
            _ => positionals.push(arg),
        }
        index += 1;
    }
    let mut positionals = positionals.into_iter();
    let case_id = positionals.next();
    let Some(case_id) = case_id else {
        return Err(
            "usage: oracle_full_state_diff <caseId> [fromIdx] [toIdx] \
             [--manifest <path>] [--trace <path>] [--json <report.json>] \
             [--batch <batchId>] [--run <runId>] [--fixture <fixture.json>]"
                .to_string(),
        );
    };
    let from = parse_index_arg(positionals.next(), "fromIdx", 0)?;
    let to = parse_index_arg(positionals.next(), "toIdx", 9999)?;
    let default_manifest = trace_root.join("original-oracle-cases").join("oracle-manifest.json");
    Ok(Options {
        case_id,
        from,
        to,
        manifest_path: resolve_path(None, manifest_path.as_deref().unwrap_or(
            default_manifest.to_str().expect("default manifest is utf-8"),
        )),
        trace_path: trace_path.map(|path| resolve_path(None, &path)),
        json_path: json_path.map(|path| resolve_path(None, &path)),
        batch_id,
        run_id,
        fixture_path: fixture_path.map(|path| resolve_path(None, &path)),
    })
}

/// TS `Number(arg ?? default)` + `Number.isInteger` + `>= 0` 的等价解析。
fn parse_index_arg(arg: Option<String>, label: &str, default: i64) -> Result<i64, String> {
    let Some(raw) = arg else {
        return Ok(default);
    };
    let invalid = || {
        format!(
            "invalid {label}: {} (must be a non-negative integer)",
            json_quote(&raw)
        )
    };
    let text = raw.trim();
    let value: f64 = if text.is_empty() {
        0.0
    } else {
        text.parse::<f64>().map_err(|_| invalid())?
    };
    if !value.is_finite() || value.fract() != 0.0 || value < 0.0 {
        return Err(invalid());
    }
    Ok(value as i64)
}

fn json_quote(text: &str) -> String {
    serde_json::to_string(text).expect("string serializes")
}

fn latest_trace_path(root: &Path) -> Result<PathBuf, String> {
    let entries = std::fs::read_dir(root)
        .map_err(|error| format!("readdir {}: {error}", root.display()))?;
    let mut latest: Option<(PathBuf, std::time::SystemTime)> = None;
    for entry in entries {
        let entry =
            entry.map_err(|error| format!("readdir {}: {error}", root.display()))?;
        let name = entry.file_name();
        let name = name.to_string_lossy();
        if !name.starts_with("yixian-trace-") || !name.ends_with(".jsonl") {
            continue;
        }
        let path = root.join(entry.file_name());
        let info = entry
            .metadata()
            .map_err(|error| format!("stat {}: {error}", path.display()))?;
        let modified = info
            .modified()
            .map_err(|error| format!("stat {}: {error}", path.display()))?;
        if latest.as_ref().map_or(true, |(_, time)| modified > *time) {
            latest = Some((path, modified));
        }
    }
    latest
        .map(|(path, _)| path)
        .ok_or_else(|| format!("no yixian-trace-*.jsonl under {}", root.display()))
}

/// manifest 缺失时给可诊断错误（与 TS ENOENT 分支文案逐字一致）。
fn resolve_runtime_case_from_manifest(
    manifest_path: &Path,
    case_id: &str,
) -> Result<Option<String>, String> {
    let text = std::fs::read_to_string(manifest_path).map_err(|error| {
        if error.kind() == std::io::ErrorKind::NotFound {
            format!(
                "oracle manifest 不存在：{}（stage 目录可能已被 golden runner 清理）— \
                 请用 --fixture <fixture.json> 显式指定 fixture；历史批次另用 --batch/--run 选定 trace 内批次",
                manifest_path.display()
            )
        } else {
            format!("read manifest {}: {error}", manifest_path.display())
        }
    })?;
    let manifest: Value = serde_json::from_str(&text)
        .map_err(|error| format!("manifest JSON error: {error}"))?;
    let cases = manifest
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "manifest cases must be an array".to_string())?;
    Ok(cases
        .iter()
        .find(|case| case.get("caseId").and_then(Value::as_str) == Some(case_id))
        .and_then(|case| case.get("fixture").and_then(Value::as_str))
        .map(str::to_string))
}

/// `fixturePathFromRuntimeCase`：manifest 内 fixture 必须是 repo 相对路径
/// 且不得逃出 repo 根（词法判定，与 TS resolve/relative 一致）。
fn fixture_path_from_runtime_case(
    repo_root: &Path,
    case_id: &str,
    fixture: &str,
) -> Result<PathBuf, String> {
    if Path::new(fixture).is_absolute() {
        return Err(format!("{case_id} runtime fixture path must be repo-relative"));
    }
    let path = resolve_path(Some(repo_root), fixture);
    let within = path == repo_root || path.starts_with(repo_root);
    if !within {
        return Err(format!(
            "{case_id} runtime fixture escapes its fixture root: {}",
            path.display()
        ));
    }
    Ok(path)
}

/// TS `String.prototype.padStart` 无关；`formatBuff` 的逐字文本。
fn format_buff(diff: &BuffDiff) -> String {
    let entry = &diff.entry;
    format!(
        "{} {} {}({}) orig={} rust={} [{}]",
        entry.buff_type_id,
        entry.archive_enum,
        entry.original_name.as_deref().unwrap_or("?"),
        entry.rust_label,
        diff.original,
        diff.rust,
        diff.classification
    )
}

fn original_side_snapshot(
    state: &yixian_engine::oracle::diff::OriginalOracleCharacterState,
) -> yixian_engine::oracle::buff_map::OriginalSideSnapshot {
    yixian_engine::oracle::buff_map::OriginalSideSnapshot {
        hp: state.hp,
        max_hp: state.max_hp,
        def: state.def,
        anima: state.anima,
        buffs: state.buffs.clone(),
    }
}

fn checkpoint_report_json(
    index: i64,
    side_diffs: &[CoreDiff],
    buff_side_diffs: &[BuffDiff],
    unmapped_side: &[(Side, UnmappedBuffEntry)],
) -> Value {
    let mut map = Map::new();
    map.insert("index".into(), Value::from(index));
    map.insert(
        "coreDiffs".into(),
        Value::Array(side_diffs.iter().map(core_diff_json).collect()),
    );
    map.insert(
        "buffDiffs".into(),
        Value::Array(buff_side_diffs.iter().map(buff_diff_json).collect()),
    );
    map.insert(
        "unmappedOriginal".into(),
        Value::Array(
            unmapped_side
                .iter()
                .map(|(side, entry)| {
                    let mut map = Map::new();
                    map.insert("buffTypeId".into(), Value::from(entry.buff_type_id));
                    map.insert("value".into(), Value::from(entry.value));
                    map.insert("side".into(), Value::String(side.as_str().into()));
                    Value::Object(map)
                })
                .collect(),
        ),
    );
    Value::Object(map)
}

fn core_diff_json(diff: &CoreDiff) -> Value {
    let mut map = Map::new();
    map.insert("side".into(), Value::String(diff.side.as_str().into()));
    map.insert("field".into(), Value::String(diff.field.as_str().into()));
    map.insert("original".into(), Value::from(diff.original));
    map.insert("rust".into(), Value::from(diff.rust));
    Value::Object(map)
}

fn buff_diff_json(diff: &BuffDiff) -> Value {
    let mut map = Map::new();
    map.insert("side".into(), Value::String(diff.side.as_str().into()));
    map.insert("entry".into(), diff.entry.to_json());
    map.insert("original".into(), Value::from(diff.original));
    map.insert("rust".into(), Value::from(diff.rust));
    map.insert(
        "classification".into(),
        Value::String(diff.classification.into()),
    );
    Value::Object(map)
}

/// CaseReport（TS `JSON.stringify(report, null, 2)` 的同形 JSON，键序逐字一致）。
#[allow(clippy::too_many_arguments)]
fn case_report_json(
    options: &Options,
    trace_path: &Path,
    trace_sha256: &str,
    run: &yixian_engine::oracle::diff::OriginalOracleRun,
    fixture_path: &Path,
    obs: &yixian_engine::oracle::diff::OriginalOracleObservation,
    archive: &Value,
    rust_checkpoint_count: usize,
    compared_checkpoints: usize,
    core_diffs_all: &[CoreDiff],
    buff_summary: (usize, usize, usize, usize),
    unmapped_ids: &[i64],
    terminal_exact: bool,
    checkpoint_reports: &[Value],
    failed: bool,
) -> Result<Value, String> {
    let (mapped_compared, only_original, only_rust, value_mismatch) = buff_summary;
    let mut report = Map::new();
    report.insert("schemaVersion".into(), Value::from(1));
    report.insert("caseId".into(), Value::String(options.case_id.clone()));
    let mut source = Map::new();
    let trace_file = trace_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_default();
    source.insert("traceFile".into(), Value::String(trace_file));
    source.insert("traceSha256".into(), Value::String(trace_sha256.to_string()));
    report.insert("source".into(), Value::Object(source));
    let mut run_json = Map::new();
    run_json.insert("batchId".into(), Value::String(run.batch_id.clone()));
    if let Some(run_id) = &run.run_id {
        run_json.insert("runId".into(), Value::String(run_id.clone()));
    }
    run_json.insert(
        "fixture".into(),
        Value::String(fixture_path.to_string_lossy().to_string()),
    );
    report.insert("run".into(), Value::Object(run_json));
    let mut checkpoint_count = Map::new();
    checkpoint_count.insert("original".into(), Value::from(obs.checkpoints.len() as u64));
    checkpoint_count.insert("rust".into(), Value::from(rust_checkpoint_count as u64));
    report.insert("checkpointCount".into(), Value::Object(checkpoint_count));
    let mut core = Map::new();
    core.insert("compared".into(), Value::from((compared_checkpoints * 2 * 4) as u64));
    core.insert("mismatch".into(), Value::from(core_diffs_all.len() as u64));
    report.insert("core".into(), Value::Object(core));
    let mut mapped_buffs = Map::new();
    mapped_buffs.insert("compared".into(), Value::from(mapped_compared as u64));
    mapped_buffs.insert(
        "match".into(),
        Value::from(
            (mapped_compared - only_original - only_rust - value_mismatch) as u64,
        ),
    );
    mapped_buffs.insert("onlyOriginal".into(), Value::from(only_original as u64));
    mapped_buffs.insert("onlyRust".into(), Value::from(only_rust as u64));
    mapped_buffs.insert("valueMismatch".into(), Value::from(value_mismatch as u64));
    report.insert("mappedBuffs".into(), Value::Object(mapped_buffs));
    let mut unmapped_original: Vec<Value> = Vec::new();
    for buff_type_id in unmapped_ids {
        let mut entry = Map::new();
        entry.insert("buffTypeId".into(), Value::from(*buff_type_id));
        if let Some(anchor) = original_buff_archive_anchor(*buff_type_id, archive)? {
            entry.insert(
                "archiveEnum".into(),
                Value::String(anchor.archive_enum.clone()),
            );
            entry.insert(
                "originalName".into(),
                match &anchor.original_name {
                    Some(name) => Value::String(name.clone()),
                    None => Value::Null,
                },
            );
            entry.insert("category".into(), Value::String(anchor.category.clone()));
        }
        unmapped_original.push(Value::Object(entry));
    }
    report.insert("unmappedOriginal".into(), Value::Array(unmapped_original));
    let mut terminal = Map::new();
    match obs.terminal.as_ref() {
        Some(observed) => {
            terminal.insert(
                "winnerSide".into(),
                Value::String(observed.winner_side.as_str().into()),
            );
            terminal.insert("actorTurn".into(), Value::from(observed.actor_turn));
            terminal.insert("hpDelta".into(), Value::from(observed.hp_delta));
            terminal.insert("exact".into(), Value::Bool(terminal_exact));
        }
        None => {
            terminal.insert("winnerSide".into(), Value::String("p1".into()));
            terminal.insert("actorTurn".into(), Value::from(-1));
            terminal.insert("hpDelta".into(), Value::from(0));
            terminal.insert("exact".into(), Value::Bool(false));
        }
    }
    report.insert("terminal".into(), Value::Object(terminal));
    report.insert(
        "checkpoints".into(),
        Value::Array(checkpoint_reports.to_vec()),
    );
    report.insert("failed".into(), Value::Bool(failed));
    Ok(Value::Object(report))
}

/// TS `resolve` 的词法等价：相对路径按 cwd（或显式 base）拼接后规范化。
fn resolve_path(base: Option<&Path>, path: &str) -> PathBuf {
    let candidate = Path::new(path);
    let joined = if candidate.is_absolute() {
        candidate.to_path_buf()
    } else {
        let root = match base {
            Some(base) => base.to_path_buf(),
            None => std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")),
        };
        root.join(candidate)
    };
    normalize_lexical(&joined)
}

fn normalize_lexical(path: &Path) -> PathBuf {
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::ParentDir => {
                out.pop();
            }
            Component::CurDir => {}
            other => out.push(other.as_os_str()),
        }
    }
    out
}

// ---- SHA-256（trace 指纹；dump bin 自包含，不新增依赖） ----

fn sha256_hex(data: &[u8]) -> String {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4,
        0xab1c5ed5, 0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe,
        0x9bdc06a7, 0xc19bf174, 0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f,
        0x4a7484aa, 0x5cb0a9dc, 0x76f988da, 0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7,
        0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967, 0x27b70a85, 0x2e1b2138, 0x4d2c6dfc,
        0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85, 0xa2bfe8a1, 0xa81a664b,
        0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070, 0x19a4c116,
        0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7,
        0xc67178f2,
    ];
    let mut h: [u32; 8] = [
        0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab,
        0x5be0cd19,
    ];
    let bit_len = (data.len() as u64).wrapping_mul(8);
    let mut message = data.to_vec();
    message.push(0x80);
    while message.len() % 64 != 56 {
        message.push(0);
    }
    message.extend_from_slice(&bit_len.to_be_bytes());
    for chunk in message.chunks_exact(64) {
        let mut w = [0u32; 64];
        for (index, word) in chunk.chunks_exact(4).enumerate() {
            w[index] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
        }
        for index in 16..64 {
            let s0 = w[index - 15].rotate_right(7) ^ w[index - 15].rotate_right(18)
                ^ (w[index - 15] >> 3);
            let s1 = w[index - 2].rotate_right(17) ^ w[index - 2].rotate_right(19)
                ^ (w[index - 2] >> 10);
            w[index] = w[index - 16]
                .wrapping_add(s0.wrapping_add(w[index - 7].wrapping_add(s1)));
        }
        let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
            (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
        for index in 0..64 {
            let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
            let ch = (e & f) ^ ((!e) & g);
            let temp1 = hh.wrapping_add(s1.wrapping_add(ch.wrapping_add(K[index].wrapping_add(w[index]))));
            let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
            let maj = (a & b) ^ (a & c) ^ (b & c);
            let temp2 = s0.wrapping_add(maj);
            hh = g;
            g = f;
            f = e;
            e = d.wrapping_add(temp1);
            d = c;
            c = b;
            b = a;
            a = temp1.wrapping_add(temp2);
        }
        h[0] = h[0].wrapping_add(a);
        h[1] = h[1].wrapping_add(b);
        h[2] = h[2].wrapping_add(c);
        h[3] = h[3].wrapping_add(d);
        h[4] = h[4].wrapping_add(e);
        h[5] = h[5].wrapping_add(f);
        h[6] = h[6].wrapping_add(g);
        h[7] = h[7].wrapping_add(hh);
    }
    h.iter().map(|value| format!("{value:08x}")).collect()
}
