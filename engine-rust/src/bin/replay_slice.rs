use serde::{Deserialize, Serialize};
use std::env;
use std::fs;
use std::io::BufRead;
use std::path::{Path, PathBuf};
use std::thread;
use yixian_engine::{
    load_fixture_file, original_card_definition_by_id, run_replay_fixture,
    run_replay_fixture_with_detailed_events, run_replay_fixture_with_parity_events, BattleError,
    BattleFixture, EngineError, PlayerSide, ReplayDetailEntry, ReplayDetailedEvent,
    ReplayDetailedRun, ReplayDetailedSideState, ReplayEvent, ReplayEventKind,
    ReplayPlayerSnapshot, ReplayTerminationCause,
};
/// `--admission-stream` 输入行：fixture 路径加完整 payload。
#[derive(Deserialize)]
struct AdmissionJsonInput {
    fixture: String,
    payload: BattleFixture,
}

use yixian_engine::replay::wire::{
    event_snapshot, AdmissionDeviation, AdmissionFixtureResult, AdmissionMissingCard,
    AdmissionReport, DisplayGapSnapshot, EventRow, EventStream,
    ExtendedEventRow, ExtendedEventSnapshot, ExtendedEventStream, ExtendedTerminal,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OutputMode {
    Report,
    Admission,
    AdmissionStream,
    AdmissionBatchStream,
    Events,
    EventsExtended,
    EventsBatch,
}

#[derive(Debug, PartialEq, Eq)]
struct Cli {
    mode: OutputMode,
    strict: bool,
    failures_only: bool,
    help: bool,
    root: PathBuf,
    limit: Option<usize>,
    jobs: Option<usize>,
}

#[derive(Debug, Serialize)]
struct FixtureResult {
    fixture: String,
    exact: bool,
    actual_winner: String,
    expected_winner: String,
    actual_turn: i64,
    expected_turn: i64,
    actual_delta: i64,
    expected_delta: i64,
}

#[derive(Debug, Serialize)]
struct Report {
    root: String,
    total: usize,
    exact: usize,
    skipped: usize,
    limit: Option<usize>,
    results: Vec<FixtureResult>,
}

enum ScreenFixtureResult {
    Executed(FixtureResult),
    Skipped,
}


fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run() -> yixian_engine::Result<()> {
    let cli = parse_cli(env::args().skip(1))?;
    if cli.help {
        print_usage();
        return Ok(());
    }
    if cli.mode == OutputMode::Events {
        let fixture = load_fixture_file(&cli.root)?;
        let run = run_replay_fixture_with_parity_events(&fixture)?;
        let stream = EventStream {
            fixture: cli.root.to_string_lossy().to_string(),
            events: run
                .events
                .iter()
                .map(|event| {
                    event_row(event, run.termination_cause, run.completed_checkpoint_count)
                })
                .collect(),
        };
        println!("{}", serde_json::to_string_pretty(&stream)?);
        return Ok(());
    }

    if cli.mode == OutputMode::EventsExtended {
        let fixture = load_fixture_file(&cli.root)?;
        let run = run_replay_fixture_with_detailed_events(&fixture)?;
        let stream = extended_event_stream(cli.root.to_string_lossy().to_string(), run)?;
        println!("{}", serde_json::to_string_pretty(&stream)?);
        return Ok(());
    }

    // Keep one replay_slice process resident while an external converter
    // streams fixture paths. Each line is one strict admission result; the
    // winner/actorTurn/hpDelta triple remains enforced by admission_fixture_result.
    if cli.mode == OutputMode::AdmissionStream {
        for line in std::io::stdin().lock().lines() {
            let value =
                line.map_err(|error| cli_error(format!("admission stream read failed: {error}")))?;
            if value.trim().is_empty() {
                continue;
            }
            let path = PathBuf::from(value.trim());
            let root = path.parent().unwrap_or_else(|| Path::new("."));
            println!(
                "{}",
                serde_json::to_string(&admission_fixture_result(root, &path))?
            );
        }
        return Ok(());
    }

    // Process each bounded set of serialized fixtures in parallel. A blank
    // line marks the batch boundary, and results retain the original order.
    if cli.mode == OutputMode::AdmissionBatchStream {
        let stdin = std::io::stdin();
        let mut inputs = Vec::new();
        for line in stdin.lock().lines() {
            let value = line.map_err(|error| {
                cli_error(format!("admission batch stream read failed: {error}"))
            })?;
            if value.trim().is_empty() {
                write_admission_batch(&mut inputs, cli.jobs)?;
            } else {
                inputs.push(value);
            }
        }
        write_admission_batch(&mut inputs, cli.jobs)?;
        return Ok(());
    }

    let mut files = Vec::new();
    collect_json_inputs(&cli.root, &mut files)?;
    append_inflight_batch_fixtures(&cli.root, &mut files)?;
    files.sort();
    files.dedup();
    if let Some(limit) = cli.limit {
        files.truncate(limit);
    }
    if files.is_empty() && cli.mode != OutputMode::Report {
        return Err(cli_error(format!(
            "no fixture JSON files found under {}",
            cli.root.display()
        )));
    }

    if cli.mode == OutputMode::Admission {
        let jobs = replay_worker_count(cli.jobs, files.len());
        let results = ordered_parallel_map(&files, jobs, |path| {
            Ok(admission_fixture_result(&cli.root, path))
        })?;
        let report = AdmissionReport {
            root: cli.root.to_string_lossy().to_string(),
            total: results.len(),
            exact: results.iter().filter(|row| row.result == "exact").count(),
            blocked: results.iter().filter(|row| row.result == "blocked").count(),
            mismatch: results
                .iter()
                .filter(|row| row.result == "mismatch")
                .count(),
            error: results.iter().filter(|row| row.result == "error").count(),
            results,
        };
        println!("{}", serde_json::to_string(&report)?);
        return Ok(());
    }

    if cli.mode == OutputMode::EventsBatch {
        for path in files {
            let fixture = load_fixture_file(&path)?;
            let run = match run_replay_fixture_with_parity_events(&fixture) {
                Ok(run) => run,
                Err(err) if is_audit_only_unsupported_build(&err) => continue,
                Err(err) if is_missing_decision(&err) => continue,
                Err(err) => return Err(err),
            };
            let stream = EventStream {
                fixture: fixture_label(&cli.root, &path),
                events: run
                    .events
                    .iter()
                    .filter(|event| is_parity_protocol_event(event.kind))
                    .map(|event| {
                        event_row(event, run.termination_cause, run.completed_checkpoint_count)
                    })
                    .collect(),
            };
            println!("{}", serde_json::to_string(&stream)?);
        }
        return Ok(());
    }

    let jobs = replay_worker_count(cli.jobs, files.len());
    let screened = ordered_parallel_map(&files, jobs, |path| {
        let fixture = load_fixture_file(path)?;
        let summary = match run_replay_fixture(&fixture) {
            Ok(summary) => summary,
            Err(err) if is_audit_only_unsupported_build(&err) => {
                return Ok(ScreenFixtureResult::Skipped);
            }
            Err(err) if is_missing_decision(&err) => {
                return Ok(ScreenFixtureResult::Skipped);
            }
            Err(err) => return Err(err),
        };
        let exact = summary.matches_fixture(&fixture);
        Ok(ScreenFixtureResult::Executed(FixtureResult {
            fixture: fixture_label(&cli.root, path),
            exact,
            actual_winner: format!("{:?}", summary.winner_side).to_lowercase(),
            expected_winner: format!("{:?}", fixture.expected.winner_side).to_lowercase(),
            actual_turn: summary.actor_turn_count,
            expected_turn: fixture.expected.actor_turn_count,
            actual_delta: summary.hp_delta_p1_minus_p2,
            expected_delta: fixture.expected.hp_delta_p1_minus_p2,
        }))
    })?;
    let skipped = screened
        .iter()
        .filter(|result| matches!(result, ScreenFixtureResult::Skipped))
        .count();
    let mut results = screened
        .into_iter()
        .filter_map(|result| match result {
            ScreenFixtureResult::Executed(result) => Some(result),
            ScreenFixtureResult::Skipped => None,
        })
        .collect::<Vec<_>>();

    let total = results.len();
    let exact = results.iter().filter(|result| result.exact).count();
    // An intentionally empty certified corpus is a valid exact subset after a
    // formal withdrawal. Audit-only skipped fixtures are not: strict mode must
    // still fail when input exists but no executable result was produced.
    let all_exact = exact == total && skipped == 0;
    if cli.failures_only {
        results.retain(|result| !result.exact);
    }
    let report = Report {
        root: cli.root.to_string_lossy().to_string(),
        total,
        exact,
        skipped,
        limit: cli.limit,
        results,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    if cli.strict && !all_exact {
        std::process::exit(1);
    }
    Ok(())
}

/// 输出模式表：(flag, 目标模式)。`--help` 外所有模式互斥，默认 Report。
const MODE_FLAGS: &[(&str, OutputMode)] = &[
    ("--events", OutputMode::Events),
    ("--events-extended", OutputMode::EventsExtended),
    ("--admission", OutputMode::Admission),
    ("--admission-stream", OutputMode::AdmissionStream),
    ("--admission-batch-stream", OutputMode::AdmissionBatchStream),
    ("--events-batch", OutputMode::EventsBatch),
];

fn set_mode_once(mode: &mut OutputMode, next: OutputMode, flag: &str) -> yixian_engine::Result<()> {
    if *mode != OutputMode::Report {
        let kind = match next {
            OutputMode::Events | OutputMode::EventsExtended | OutputMode::EventsBatch => "event",
            _ => "output",
        };
        return Err(cli_error(format!(
            "{flag} conflicts with another {kind} mode"
        )));
    }
    *mode = next;
    Ok(())
}

fn set_bool_once(flag: &mut bool, name: &str) -> yixian_engine::Result<()> {
    if *flag {
        return Err(cli_error(format!("{name} may only be supplied once")));
    }
    *flag = true;
    Ok(())
}

fn parse_cli(args: impl IntoIterator<Item = String>) -> yixian_engine::Result<Cli> {
    let mut mode = OutputMode::Report;
    let mut strict = false;
    let mut failures_only = false;
    let mut help = false;
    let mut jobs = None;
    let mut positionals = Vec::new();

    let mut args = args.into_iter();
    while let Some(arg) = args.next() {
        if let Some((_, next)) = MODE_FLAGS.iter().find(|(flag, _)| *flag == arg.as_str()) {
            set_mode_once(&mut mode, *next, &arg)?;
            continue;
        }
        match arg.as_str() {
            "--strict" => set_bool_once(&mut strict, "--strict")?,
            "--failures-only" => set_bool_once(&mut failures_only, "--failures-only")?,
            "--help" | "-h" => help = true,
            "--jobs" => {
                if jobs.is_some() {
                    return Err(cli_error("--jobs may only be supplied once"));
                }
                let value = args
                    .next()
                    .ok_or_else(|| cli_error("--jobs requires a positive integer"))?;
                jobs = Some(parse_jobs(&value)?);
            }
            _ if arg.starts_with('-') => {
                if let Some(value) = arg.strip_prefix("--jobs=") {
                    if jobs.is_some() {
                        return Err(cli_error("--jobs may only be supplied once"));
                    }
                    jobs = Some(parse_jobs(value)?);
                } else {
                    return Err(cli_error(format!("unknown option: {arg}")));
                }
            }
            _ => positionals.push(arg),
        }
    }

    if help {
        return Ok(Cli {
            mode,
            strict,
            failures_only,
            help,
            root: PathBuf::new(),
            limit: None,
            jobs,
        });
    }
    for (enabled, name) in [(strict, "--strict"), (failures_only, "--failures-only")] {
        if enabled && mode != OutputMode::Report {
            return Err(cli_error(format!(
                "{name} cannot be combined with event modes"
            )));
        }
    }
    if jobs.is_some()
        && matches!(
            mode,
            OutputMode::Events
                | OutputMode::EventsExtended
                | OutputMode::EventsBatch
                | OutputMode::AdmissionStream
        )
    {
        return Err(cli_error(
            "--jobs is only supported for report or admission mode",
        ));
    }

    if matches!(
        mode,
        OutputMode::AdmissionStream | OutputMode::AdmissionBatchStream
    ) && !positionals.is_empty()
    {
        let stream_input = if mode == OutputMode::AdmissionStream {
            "fixture paths"
        } else {
            "fixture JSON lines"
        };
        let stream_flag = if mode == OutputMode::AdmissionStream {
            "--admission-stream"
        } else {
            "--admission-batch-stream"
        };
        return Err(cli_error(format!(
            "{stream_flag} reads {stream_input} from stdin and accepts no positional arguments"
        )));
    }
    let max_positionals = if mode == OutputMode::Events || mode == OutputMode::EventsExtended {
        1
    } else {
        2
    };
    if positionals.len() > max_positionals {
        return Err(cli_error("too many positional arguments"));
    }
    if mode != OutputMode::Report
        && !matches!(
            mode,
            OutputMode::AdmissionStream | OutputMode::AdmissionBatchStream
        )
        && positionals.is_empty()
    {
        return Err(cli_error(match mode {
            OutputMode::Events => "--events requires one fixture JSON path",
            OutputMode::EventsExtended => "--events-extended requires one fixture JSON path",
            OutputMode::EventsBatch => "--events-batch requires a fixture JSON or directory path",
            OutputMode::Report
            | OutputMode::Admission
            | OutputMode::AdmissionStream
            | OutputMode::AdmissionBatchStream => {
                unreachable!()
            }
        }));
    }

    let root = positionals
        .first()
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("battle-evaluator/fixtures/candidates"));
    let limit = positionals
        .get(1)
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| cli_error(format!("invalid limit: {value}")))
                .and_then(|limit| {
                    if limit == 0 {
                        Err(cli_error("limit must be greater than zero"))
                    } else {
                        Ok(limit)
                    }
                })
        })
        .transpose()?;

    Ok(Cli {
        mode,
        strict,
        failures_only,
        help,
        root,
        limit,
        jobs,
    })
}

fn parse_jobs(value: &str) -> yixian_engine::Result<usize> {
    value
        .parse::<usize>()
        .map_err(|_| cli_error(format!("invalid jobs: {value}")))
        .and_then(|jobs| {
            if jobs == 0 {
                Err(cli_error("jobs must be greater than zero"))
            } else {
                Ok(jobs)
            }
        })
}

fn cli_error(message: impl Into<String>) -> EngineError {
    EngineError::InvalidFixture(format!("replay_slice CLI: {}", message.into()))
}

fn print_usage() {
    println!(
        "Usage:\n\
  replay_slice [--strict] [--failures-only] [--jobs N] [fixture-json-or-directory] [limit]\n\
  replay_slice --admission [--jobs N] <fixture-json-or-directory> [limit]\n\
  replay_slice --admission-stream < fixture-paths.txt\n\
  replay_slice --admission-batch-stream [--jobs N] < bounded-jsonl-batches.txt\n\
  replay_slice --events <fixture-json>\n\
  replay_slice --events-extended <fixture-json>\n\
  replay_slice --events-batch <fixture-json-or-directory> [limit]\n\
\n\
The report mode defaults to battle-evaluator/fixtures/candidates.\n\
--failures-only keeps the report totals but omits exact fixture rows.\n\
--admission emits one compact, fail-closed JSON batch for replay admission.\n\
--events emits one pretty-printed full event stream.\n\
--events-extended emits the same stream plus momentumLimit/physiqueLimit/LoseHpCount/LoseHpTimesCount from detail_entries, per-hit attackSegments, stable turnEndHooks receipts, and the full exposure-gap fields on the parity snapshot (JianQi 625, HuangQueZaiHou 264, stances 443/444, etc.; see build_display_gap_report.py); the 29-field parity protocol itself is unchanged.\n\
--events-batch recursively sorts inputs and emits compact JSONL with four completed-state checkpoints plus one final battleEnd marker."
    );
}

fn replay_worker_count(requested: Option<usize>, task_count: usize) -> usize {
    let available = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1);
    requested.unwrap_or(available).min(task_count.max(1))
}

fn write_admission_batch(
    inputs: &mut Vec<String>,
    requested_jobs: Option<usize>,
) -> yixian_engine::Result<()> {
    if inputs.is_empty() {
        return Ok(());
    }
    let jobs = replay_worker_count(requested_jobs, inputs.len());
    let results = ordered_parallel_map(inputs, jobs, |line| {
        let input = match serde_json::from_str::<AdmissionJsonInput>(line) {
            Ok(input) => input,
            Err(error) => {
                return Ok(admission_error(
                    "stream/input".to_string(),
                    EngineError::Json(error),
                ));
            }
        };
        if let Err(error) = input.payload.validate() {
            return Ok(admission_error(input.fixture, error));
        }
        Ok(admission_fixture_result_from_fixture(
            input.fixture,
            input.payload,
        ))
    })?;
    let lines = results
        .iter()
        .map(serde_json::to_string)
        .collect::<std::result::Result<Vec<_>, _>>()?;
    println!("{}", lines.join("\n"));
    inputs.clear();
    Ok(())
}

fn ordered_parallel_map<T, U, F>(
    items: &[T],
    jobs: usize,
    operation: F,
) -> yixian_engine::Result<Vec<U>>
where
    T: Sync,
    U: Send,
    F: Fn(&T) -> yixian_engine::Result<U> + Sync,
{
    if items.is_empty() {
        return Ok(Vec::new());
    }
    let chunk_size = items.len().div_ceil(jobs.max(1));
    thread::scope(|scope| {
        let handles = items
            .chunks(chunk_size)
            .map(|chunk| {
                let operation = &operation;
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(operation)
                        .collect::<yixian_engine::Result<Vec<_>>>()
                })
            })
            .collect::<Vec<_>>();
        let mut ordered = Vec::with_capacity(items.len());
        for handle in handles {
            let chunk = handle
                .join()
                .map_err(|_| cli_error("replay worker panicked"))??;
            ordered.extend(chunk);
        }
        Ok(ordered)
    })
}

fn admission_fixture_result(root: &Path, path: &Path) -> AdmissionFixtureResult {
    let fixture_label = fixture_label(root, path);
    let fixture = match load_fixture_file(path) {
        Ok(fixture) => fixture,
        Err(error) => return admission_error(fixture_label, error),
    };
    admission_fixture_result_from_fixture(fixture_label, fixture)
}

fn admission_fixture_result_from_fixture(
    fixture_label: String,
    fixture: BattleFixture,
) -> AdmissionFixtureResult {
    if fixture
        .source
        .as_ref()
        .and_then(|source| source.eligible_for_base_rule_replay)
        != Some(true)
    {
        return AdmissionFixtureResult {
            fixture: fixture_label,
            result: "blocked",
            actual_winner: None,
            actual_turn: None,
            actual_delta: None,
            first_deviation: Some(AdmissionDeviation {
                kind: "blocked",
                message: "scope=invalid".to_string(),
                actual: None,
                expected: None,
            }),
            warnings: Vec::new(),
            missing_cards: Vec::new(),
        };
    }
    let summary = match run_replay_fixture(&fixture) {
        Ok(summary) => summary,
        Err(error) => return admission_error(fixture_label, error),
    };
    let actual_winner = player_side(summary.winner_side).to_string();
    let expected_winner = player_side(fixture.expected.winner_side).to_string();
    let first_deviation = if actual_winner != expected_winner {
        Some(AdmissionDeviation {
            kind: "winner",
            message: format!("winner mismatch: expected={expected_winner} actual={actual_winner}"),
            actual: Some(actual_winner.clone()),
            expected: Some(expected_winner),
        })
    } else if summary.actor_turn_count != fixture.expected.actor_turn_count {
        Some(AdmissionDeviation {
            kind: "actorTurn",
            message: format!(
                "actorTurn mismatch: expected={} actual={}",
                fixture.expected.actor_turn_count, summary.actor_turn_count
            ),
            actual: Some(summary.actor_turn_count.to_string()),
            expected: Some(fixture.expected.actor_turn_count.to_string()),
        })
    } else if summary.hp_delta_p1_minus_p2 != fixture.expected.hp_delta_p1_minus_p2 {
        Some(AdmissionDeviation {
            kind: "hpDelta",
            message: format!(
                "hpDelta mismatch: expected={} actual={}",
                fixture.expected.hp_delta_p1_minus_p2, summary.hp_delta_p1_minus_p2
            ),
            actual: Some(summary.hp_delta_p1_minus_p2.to_string()),
            expected: Some(fixture.expected.hp_delta_p1_minus_p2.to_string()),
        })
    } else {
        None
    };
    AdmissionFixtureResult {
        fixture: fixture_label,
        result: if first_deviation.is_none() {
            "exact"
        } else {
            "mismatch"
        },
        actual_winner: Some(actual_winner),
        actual_turn: Some(summary.actor_turn_count),
        actual_delta: Some(summary.hp_delta_p1_minus_p2),
        first_deviation,
        warnings: Vec::new(),
        missing_cards: Vec::new(),
    }
}

fn admission_error(fixture: String, error: EngineError) -> AdmissionFixtureResult {
    let (result, kind, missing_cards) = match &error {
        EngineError::InvalidFixture(_) => ("blocked", "invalid-fixture", Vec::new()),
        EngineError::Battle(BattleError::MissingRule {
            card_id, base_id, ..
        }) => {
            let name = original_card_definition_by_id(*card_id)
                .or_else(|| original_card_definition_by_id(*base_id))
                .map(|card| card.name.to_string())
                .unwrap_or_else(|| format!("unknown-card-{base_id}"));
            (
                "blocked",
                "missing-rule",
                vec![AdmissionMissingCard {
                    id: *card_id,
                    base_id: *base_id,
                    name,
                }],
            )
        }
        EngineError::Battle(BattleError::MissingDecision { .. }) => {
            ("blocked", "missing-decision", Vec::new())
        }
        EngineError::Battle(BattleError::InvalidDecision { .. }) => {
            ("blocked", "invalid-decision", Vec::new())
        }
        EngineError::Battle(BattleError::Invariant { .. }) => ("blocked", "invariant", Vec::new()),
        EngineError::Io(_) | EngineError::Json(_) => ("error", "error", Vec::new()),
    };
    AdmissionFixtureResult {
        fixture,
        result,
        actual_winner: None,
        actual_turn: None,
        actual_delta: None,
        first_deviation: Some(AdmissionDeviation {
            kind,
            message: error.to_string(),
            actual: None,
            expected: None,
        }),
        warnings: Vec::new(),
        missing_cards,
    }
}

fn event_row(
    event: &ReplayEvent,
    termination_cause: ReplayTerminationCause,
    completed_checkpoint_count: usize,
) -> EventRow {
    EventRow {
        kind: event_kind(event.kind).to_string(),
        turn: event.turn,
        actor: player_side(event.actor).to_string(),
        slot: event.slot,
        card_id: event.card_id,
        card_name: event.card_name.clone(),
        termination_cause: (event.kind == ReplayEventKind::BattleEnd).then_some(termination_cause),
        completed_checkpoint_count: (event.kind == ReplayEventKind::BattleEnd)
            .then_some(completed_checkpoint_count),
        p1: event_snapshot(&event.p1),
        p2: event_snapshot(&event.p2),
    }
}

fn event_kind(kind: ReplayEventKind) -> &'static str {
    match kind {
        ReplayEventKind::BattleStart => "battleStart",
        ReplayEventKind::TurnStart => "turnStart",
        ReplayEventKind::CardCompleted => "cardCompleted",
        ReplayEventKind::TurnEnd => "turnEnd",
        ReplayEventKind::BattleEnd => "battleEnd",
    }
}

fn extended_event_stream(
    fixture: String,
    run: ReplayDetailedRun,
) -> yixian_engine::Result<ExtendedEventStream> {
    let last = run
        .events
        .last()
        .ok_or_else(|| cli_error("detailed replay produced no events"))?;
    let terminal = ExtendedTerminal {
        winner_side: player_side(run.summary.winner_side),
        actor_turn: run.summary.actor_turn_count,
        hp_delta: run.summary.hp_delta_p1_minus_p2,
        p1_hp: last.event.p1.hp,
        p2_hp: last.event.p2.hp,
        remaining_decision_params: run.remaining_decision_params,
    };
    Ok(ExtendedEventStream {
        fixture,
        terminal,
        events: run.events.iter().map(extended_event_row).collect(),
        decision_events: run.decision_events,
        attack_segments: run.attack_segments,
        turn_end_hooks: run.turn_end_hooks,
    })
}

fn extended_event_row(detailed: &ReplayDetailedEvent) -> ExtendedEventRow {
    ExtendedEventRow {
        kind: event_kind(detailed.event.kind).to_string(),
        turn: detailed.event.turn,
        actor: player_side(detailed.event.actor).to_string(),
        slot: detailed.event.slot,
        card_id: detailed.event.card_id,
        card_name: detailed.event.card_name.clone(),
        p1: extended_event_snapshot(&detailed.event.p1, &detailed.p1, &detailed.p1_state),
        p2: extended_event_snapshot(&detailed.event.p2, &detailed.p2, &detailed.p2_state),
        p1_card_queue: detailed.p1_state.card_queue.clone(),
        p2_card_queue: detailed.p2_state.card_queue.clone(),
        p1_physical_deck: detailed.p1_state.physical_deck.clone(),
        p2_physical_deck: detailed.p2_state.physical_deck.clone(),
    }
}

fn extended_event_snapshot(
    parity: &ReplayPlayerSnapshot,
    entries: &[ReplayDetailEntry],
    side_state: &ReplayDetailedSideState,
) -> ExtendedEventSnapshot {
    let mut display_gap = DisplayGapSnapshot::from_parity(parity);
    display_gap.action_again_per_round = side_state.action_again_per_round;
    display_gap.active_slot_count = side_state.active_slot_count as i64;
    // detail_entries 暴露的 9 个持久计数（原版 BuffType
    // 298/10024/351/409/302/303/644/17/18）：(快照字段, detail key)。
    for (field, key) in [
        (&mut display_gap.momentum_limit, "momentumLimit"),
        (&mut display_gap.physique_limit, "physiqueLimit"),
        (&mut display_gap.lose_hp_count, "loseHpCount"),
        (&mut display_gap.lose_hp_times_count, "loseHpTimesCount"),
        (&mut display_gap.actual_damage_carry, "actualDamageCarry"),
        (&mut display_gap.wounded_count_carry, "woundedCountCarry"),
        (
            &mut display_gap.ji_lu_zong_ji_shang_zhi,
            "jiLuZongJiShangZhi",
        ),
        (&mut display_gap.last_stand_intent, "lastStandIntent"),
        (
            &mut display_gap.last_stand_unyielding,
            "lastStandUnyielding",
        ),
    ] {
        *field = detail_entry_value(entries, key);
    }
    ExtendedEventSnapshot {
        parity: event_snapshot(parity),
        display_gap,
    }
}

fn detail_entry_value(entries: &[ReplayDetailEntry], key: &str) -> i64 {
    entries
        .iter()
        .find(|entry| entry.key == key)
        .map(|entry| entry.value)
        .unwrap_or(0)
}

fn is_parity_protocol_event(kind: ReplayEventKind) -> bool {
    matches!(
        kind,
        ReplayEventKind::BattleStart
            | ReplayEventKind::TurnStart
            | ReplayEventKind::CardCompleted
            | ReplayEventKind::TurnEnd
            | ReplayEventKind::BattleEnd
    )
}

fn player_side(side: PlayerSide) -> &'static str {
    match side {
        PlayerSide::P1 => "p1",
        PlayerSide::P2 => "p2",
    }
}

fn collect_json_inputs(root: &Path, files: &mut Vec<PathBuf>) -> yixian_engine::Result<()> {
    if root.is_file() {
        if root.extension().is_some_and(|ext| ext == "json") {
            files.push(root.to_path_buf());
        }
        return Ok(());
    }
    collect_json_files(root, files)
}

/// freeze:replay --full --inflight-batch <id> 期间，将未准入的 incoming 批 fixture
/// 并入扫描范围（镜像 TS candidateFixturePath 的 inflight 回退）。仅当
/// REPLAY_ADMISSION_INFLIGHT_BATCH_ID 置位且 root 为目录时，追加
/// root.parent()/incoming/<batch>；无 env 或目录不存在则原样返回。
fn append_inflight_batch_fixtures(
    root: &Path,
    files: &mut Vec<PathBuf>,
) -> yixian_engine::Result<()> {
    let Some(batch_id) = env::var("REPLAY_ADMISSION_INFLIGHT_BATCH_ID")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
    else {
        return Ok(());
    };
    if !root.is_dir() {
        return Ok(());
    }
    let Some(parent) = root.parent() else {
        return Ok(());
    };
    let inflight_root = parent.join("incoming").join(&batch_id);
    if inflight_root.is_dir() {
        collect_json_inputs(&inflight_root, files)?;
    }
    Ok(())
}

/// audit-only（非 runtime-supported）build 的 fixture 由 TS 审计、Rust 不运行；
/// run_replay_fixture 对这类 fixture 返回的 "unsupported original Steam build
/// ... runtime-supported builds: ..." 错误应跳过而非 --strict 失败。其余
/// InvalidFixture（如 missing required runtime capability）仍向上抛出。
fn is_audit_only_unsupported_build(err: &EngineError) -> bool {
    matches!(
        err,
        EngineError::InvalidFixture(msg)
            if msg.starts_with("unsupported original Steam build")
                && msg.contains("runtime-supported builds")
    )
}

/// A missing-decision error means the engine cannot replay a particular fixture
/// because the decision tape lacks a primitive the current rules require (e.g.
/// a random-range pick the original client made but the fixture did not record).
/// This is a per-fixture gap, not a screening infrastructure failure: skip the
/// fixture and continue screening the rest of the batch.
fn is_missing_decision(err: &EngineError) -> bool {
    matches!(
        err,
        EngineError::Battle(yixian_engine::BattleError::MissingDecision { .. })
    )
}

fn collect_json_files(dir: &Path, files: &mut Vec<PathBuf>) -> yixian_engine::Result<()> {
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        if path.is_dir() {
            collect_json_files(&path, files)?;
        } else if path.extension().is_some_and(|ext| ext == "json") {
            files.push(path);
        }
    }
    Ok(())
}

fn fixture_label(root: &Path, path: &Path) -> String {
    if root.is_file() {
        let stem = path
            .file_stem()
            .map(|value| value.to_string_lossy())
            .unwrap_or_default();
        if let Some(parent) = path.parent().and_then(|parent| parent.file_name()) {
            return format!("{}/{}", parent.to_string_lossy(), stem);
        }
        return stem.to_string();
    }
    if let Ok(stripped) = path.strip_prefix(root) {
        return stripped
            .with_extension("")
            .to_string_lossy()
            .trim_start_matches('/')
            .to_string();
    }
    // inflight 批 fixture 位于 root 之外（incoming/<batch>/），退回
    // <match>/<round> 形式，与 candidates 下的 label 及 TS event sweep 对齐。
    let stem = path
        .file_stem()
        .map(|value| value.to_string_lossy().to_string())
        .unwrap_or_default();
    if let Some(parent) = path.parent().and_then(|parent| parent.file_name()) {
        return format!("{}/{}", parent.to_string_lossy(), stem);
    }
    stem
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(values: &[&str]) -> Vec<String> {
        values.iter().map(|value| (*value).to_string()).collect()
    }

    /// CLI 表：(参数, 期望模式) 与必错参数；覆盖原 7 个单测的全部断言。
    #[test]
    fn cli_modes_and_limits() {
        for (argv, expected) in [
            (
                &["--events-batch", "fixtures", "2"][..],
                OutputMode::EventsBatch,
            ),
            (&["--admission-stream"][..], OutputMode::AdmissionStream),
            (
                &["--admission", "--jobs=3", "fixtures"][..],
                OutputMode::Admission,
            ),
            (
                &["--admission-batch-stream", "--jobs=3"][..],
                OutputMode::AdmissionBatchStream,
            ),
        ] {
            let cli = parse_cli(args(argv)).unwrap();
            assert_eq!(cli.mode, expected, "argv: {argv:?}");
        }
        let admission = parse_cli(args(&["--admission", "--jobs=3", "fixtures"])).unwrap();
        assert_eq!(admission.jobs, Some(3));
        let batch = parse_cli(args(&["--events-batch", "fixtures", "2"])).unwrap();
        assert_eq!(batch.root, PathBuf::from("fixtures"));
        assert_eq!(batch.limit, Some(2));
        assert!(!batch.strict && !batch.failures_only);
        let stream = parse_cli(args(&["--admission-stream"])).unwrap();
        assert_eq!(
            stream.root,
            PathBuf::from("battle-evaluator/fixtures/candidates")
        );
        let batch_stream = parse_cli(args(&["--admission-batch-stream", "--jobs=3"])).unwrap();
        assert_eq!(batch_stream.mode, OutputMode::AdmissionBatchStream);
        assert_eq!(batch_stream.jobs, Some(3));
        let strict = parse_cli(args(&[
            "--strict",
            "--failures-only",
            "--jobs",
            "4",
            "fixtures",
        ]))
        .unwrap();
        assert_eq!(strict.mode, OutputMode::Report);
        assert!(strict.strict && strict.failures_only);
        assert_eq!(strict.jobs, Some(4));
        for argv in [
            &["--admission-stream", "fixture.json"][..],
            &["--admission-batch-stream", "fixture.json"][..],
            &["--admission-stream", "--jobs=2"][..],
            &["--events", "--events-batch", "fixture.json"][..],
            &["--events-batch"][..],
            &["fixtures", "invalid"][..],
            &["fixtures", "0"][..],
            &["--jobs", "0", "fixtures"][..],
            &["--jobs=invalid", "fixtures"][..],
            &["--jobs"][..],
            &["--events-batch", "--failures-only", "fixtures"][..],
        ] {
            assert!(parse_cli(args(argv)).is_err(), "argv: {argv:?}");
        }
    }
    #[test]
    fn ordered_parallel_map_preserves_input_order() {
        let paths = ["3", "1", "2"]
            .into_iter()
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        let actual =
            ordered_parallel_map(&paths, 3, |path| Ok(path.to_string_lossy().to_string())).unwrap();
        assert_eq!(actual, ["3", "1", "2"]);
    }

    #[test]
    fn rejects_failures_only_event_mode() {
        assert!(parse_cli(args(&["--events-batch", "--failures-only", "fixtures",])).is_err());
    }

    #[test]
    fn admission_errors_preserve_battle_error_categories_and_missing_card_payload() {
        let missing_rule = admission_error(
            "match/round-01".to_string(),
            EngineError::Battle(BattleError::MissingRule {
                card_id: 135,
                base_id: 135,
                reason: "missing executable effect".to_string(),
                turn: 7,
            }),
        );
        assert_eq!(missing_rule.result, "blocked");
        assert_eq!(
            missing_rule
                .first_deviation
                .as_ref()
                .expect("deviation")
                .kind,
            "missing-rule"
        );
        assert_eq!(missing_rule.missing_cards.len(), 1);
        assert_eq!(missing_rule.missing_cards[0].id, 135);
        assert_eq!(missing_rule.missing_cards[0].base_id, 135);
        assert_eq!(missing_rule.missing_cards[0].name, "锟铻熔火环");
        assert!(missing_rule.warnings.is_empty());

        let categories = [
            (
                EngineError::Battle(BattleError::MissingDecision {
                    reason: "choice absent".to_string(),
                    turn: 2,
                }),
                "missing-decision",
            ),
            (
                EngineError::Battle(BattleError::InvalidDecision {
                    card_id: 135,
                    kind: "card",
                    selected: 0,
                    turn: 2,
                }),
                "invalid-decision",
            ),
            (
                EngineError::Battle(BattleError::Invariant {
                    message: "state corrupt".to_string(),
                }),
                "invariant",
            ),
            (
                EngineError::InvalidFixture("bad input".to_string()),
                "invalid-fixture",
            ),
        ];
        for (error, expected_kind) in categories {
            let result = admission_error("fixture".to_string(), error);
            assert_eq!(result.result, "blocked");
            assert_eq!(
                result.first_deviation.expect("deviation").kind,
                expected_kind
            );
            assert!(result.missing_cards.is_empty());
        }
    }

    #[test]
    fn admission_missing_card_payload_uses_camel_case_wire_fields() {
        let result = admission_error(
            "fixture".to_string(),
            EngineError::Battle(BattleError::MissingRule {
                card_id: 135,
                base_id: 135,
                reason: "missing executable effect".to_string(),
                turn: 1,
            }),
        );
        let json = serde_json::to_value(result).expect("admission result serializes");
        assert_eq!(json["firstDeviation"]["kind"], "missing-rule");
        assert_eq!(json["missingCards"][0]["id"], 135);
        assert_eq!(json["missingCards"][0]["baseId"], 135);
        assert_eq!(json["missingCards"][0]["name"], "锟铻熔火环");
        assert_eq!(json["warnings"], serde_json::json!([]));
    }

    #[test]
    fn parity_batch_keeps_four_completed_checkpoints_and_terminal() {
        assert!(is_parity_protocol_event(ReplayEventKind::BattleStart));
        assert!(is_parity_protocol_event(ReplayEventKind::TurnStart));
        assert!(is_parity_protocol_event(ReplayEventKind::CardCompleted));
        assert!(is_parity_protocol_event(ReplayEventKind::TurnEnd));
        assert!(is_parity_protocol_event(ReplayEventKind::BattleEnd));
    }

    #[test]
    fn termination_cause_wire_values_are_camel_case() {
        let cases = [
            (ReplayTerminationCause::TurnStartLethal, "turnStartLethal"),
            (ReplayTerminationCause::CardLethal, "cardLethal"),
            (
                ReplayTerminationCause::ActionAgainLethal,
                "actionAgainLethal",
            ),
            (ReplayTerminationCause::TurnEndLethal, "turnEndLethal"),
            (ReplayTerminationCause::MaxTurn, "maxTurn"),
        ];

        for (cause, expected) in cases {
            assert_eq!(
                serde_json::to_value(cause).expect("termination cause serializes"),
                serde_json::Value::String(expected.to_string())
            );
        }
    }

    #[test]
    fn event_rows_expose_termination_cause_only_on_battle_end() {
        let fixture =
            yixian_engine::engine_contract_fixture().expect("engine contract fixture builds");
        let run =
            run_replay_fixture_with_parity_events(&fixture).expect("reference fixture replays");
        let rows = run
            .events
            .iter()
            .map(|event| event_row(event, run.termination_cause, run.completed_checkpoint_count))
            .collect::<Vec<_>>();

        assert!(rows.len() > 1);
        assert!(rows[..rows.len() - 1].iter().all(|row| {
            row.termination_cause.is_none() && row.completed_checkpoint_count.is_none()
        }));
        assert_eq!(
            rows.last().expect("terminal row").termination_cause,
            Some(run.termination_cause)
        );
        assert_eq!(rows.last().expect("terminal row").kind, "battleEnd");
        assert_eq!(
            rows.last()
                .expect("terminal row")
                .completed_checkpoint_count,
            Some(run.completed_checkpoint_count)
        );

        let json_rows = rows
            .iter()
            .map(|row| serde_json::to_value(row).expect("event row serializes"))
            .collect::<Vec<_>>();
        assert!(json_rows[..json_rows.len() - 1].iter().all(|row| {
            row.get("termination_cause") == Some(&serde_json::Value::Null)
                && row.get("completed_checkpoint_count") == Some(&serde_json::Value::Null)
        }));
        assert_eq!(
            json_rows
                .last()
                .expect("terminal JSON row")
                .get("termination_cause"),
            Some(&serde_json::to_value(run.termination_cause).expect("cause serializes"))
        );
        assert_eq!(
            json_rows
                .last()
                .expect("terminal JSON row")
                .get("completed_checkpoint_count"),
            Some(&serde_json::json!(run.completed_checkpoint_count))
        );
    }

    #[test]
    fn extended_events_expose_loss_ledgers_and_turn_end_hook_receipts() {
        let fixture =
            yixian_engine::engine_contract_fixture().expect("engine contract fixture builds");
        let run = run_replay_fixture_with_detailed_events(&fixture).expect("detailed replay");
        let first = extended_event_row(run.events.first().expect("detailed event"));
        let snapshot = serde_json::to_value(&first.p1).expect("extended snapshot serializes");
        assert_eq!(snapshot.get("lose_hp_count"), Some(&serde_json::json!(0)));
        assert_eq!(
            snapshot.get("lose_hp_times_count"),
            Some(&serde_json::json!(0))
        );

        let summary = run.summary.clone();
        let stream =
            extended_event_stream("contract".to_string(), run).expect("extended stream builds");
        assert_eq!(stream.terminal.actor_turn, summary.actor_turn_count);
        assert_eq!(
            stream.terminal.p1_hp - stream.terminal.p2_hp,
            summary.hp_delta_p1_minus_p2
        );
        assert!(!stream.events[0].p1_card_queue.is_empty());
        let value = serde_json::to_value(stream).expect("extended stream serializes");
        assert!(value.get("decisionEvents").is_some());
        let receipts = value
            .get("turnEndHooks")
            .and_then(serde_json::Value::as_array)
            .expect("turnEndHooks array");
        assert!(!receipts.is_empty());
        assert_eq!(receipts[0].get("hook"), Some(&serde_json::json!("ronghui")));
        assert!(receipts[0]["before"]["p1"].get("loseHpCount").is_some());
        let segments = value
            .get("attackSegments")
            .and_then(serde_json::Value::as_array)
            .expect("attackSegments array");
        assert!(!segments.is_empty());
        assert!(segments[0].get("actorHpBefore").is_some());
        assert!(segments[0].get("actorHpAfter").is_some());
    }
}
