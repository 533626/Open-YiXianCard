use serde::Serialize;
use std::env;
use std::path::PathBuf;
use yixian_engine::model::PlayerSide;
use yixian_engine::solver::{solve_deck, ScoreProfile, SolveDeckOptions, VisitOrder};
use yixian_engine::{engine_identity, load_fixture_file, EngineIdentity, Result};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    engine_identity: EngineIdentity,
    fixture: String,
    #[serde(flatten)]
    result: yixian_engine::solver::SolveDeckResult,
}

fn main() -> Result<()> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.iter().any(|arg| arg == "--help" || arg == "-h") {
        print_help();
        return Ok(());
    }
    let cli = CliOptions::parse(args)?;
    let fixture = load_fixture_file(&cli.fixture)?;
    let result = solve_deck(
        &fixture,
        SolveDeckOptions {
            side: cli.side,
            mode: cli.mode,
            visit_order: cli.visit_order,
            visit_seed: cli.visit_seed,
            top: cli.top,
            max_evaluations: cli.max_evals,
            score_profile: cli.score_profile,
            exact_deck_ids: cli.deck,
            battle_seeds: cli.battle_seeds,
        },
    )?;
    let report = Report {
        engine_identity: engine_identity(),
        fixture: cli.fixture.to_string_lossy().to_string(),
        result,
    };
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

struct CliOptions {
    fixture: PathBuf,
    side: PlayerSide,
    mode: yixian_engine::solver::SolverMode,
    visit_order: VisitOrder,
    visit_seed: u64,
    score_profile: ScoreProfile,
    top: usize,
    max_evals: usize,
    deck: Option<Vec<i64>>,
    battle_seeds: Option<Vec<u32>>,
}

impl CliOptions {
    fn parse(args: Vec<String>) -> Result<Self> {
        let mut fixture = None;
        let mut side = PlayerSide::P1;
        let mut mode = yixian_engine::solver::SolverMode::Order;
        let mut visit_order = VisitOrder::Canonical;
        let mut visit_seed = 0_u64;
        let mut score_profile = ScoreProfile::HpDelta;
        let mut top = 20_usize;
        let mut max_evals = 200_000_usize;
        let mut deck = None;
        let mut battle_seeds = None;
        let mut index = 0;
        while index < args.len() {
            let arg = &args[index];
            match arg.as_str() {
                "--fixture" => {
                    index += 1;
                    fixture = args.get(index).map(PathBuf::from);
                }
                "--side" => {
                    index += 1;
                    side = parse_side(args.get(index).map(String::as_str).unwrap_or("p1"))?;
                }
                "--mode" => {
                    index += 1;
                    mode = parse_mode(args.get(index).map(String::as_str).unwrap_or("order"))?;
                }
                "--score-profile" => {
                    index += 1;
                    score_profile = parse_score_profile(
                        args.get(index).map(String::as_str).unwrap_or("hpDelta"),
                    )?;
                }
                "--visit-order" => {
                    index += 1;
                    visit_order =
                        parse_visit_order(args.get(index).map(String::as_str).unwrap_or(""))?;
                }
                "--seed" => {
                    index += 1;
                    visit_seed = parse_u64(args.get(index), "--seed")?;
                }
                "--top" => {
                    index += 1;
                    top = parse_usize(args.get(index), "--top")?;
                }
                "--max-evals" => {
                    index += 1;
                    max_evals = parse_usize(args.get(index), "--max-evals")?;
                }
                "--deck" => {
                    index += 1;
                    deck = Some(parse_deck(
                        args.get(index).map(String::as_str).unwrap_or(""),
                    )?);
                }
                "--battle-seeds" => {
                    index += 1;
                    battle_seeds = Some(parse_battle_seeds(
                        args.get(index).map(String::as_str).unwrap_or(""),
                    )?);
                }
                value if value.starts_with("--") => {
                    return Err(yixian_engine::EngineError::InvalidFixture(format!(
                        "unknown solve_deck option {value}"
                    )));
                }
                value => {
                    fixture = Some(PathBuf::from(value));
                }
            }
            index += 1;
        }
        let fixture = match fixture {
            Some(f) => f,
            None => default_fixture_path()?,
        };
        Ok(Self {
            fixture,
            side,
            mode,
            visit_order,
            visit_seed,
            score_profile,
            top,
            max_evals,
            deck,
            battle_seeds,
        })
    }
}

// The public build ships no bundled replay corpus. The default fixture was a
// private corpus manifest lookup that does not exist in a fresh public checkout
// and is therefore fail-closed: without an explicit --fixture the solver
// refuses to run instead of silently guessing. The restored private companion
// (see PRIVATE_ENGINEERING_EXTRACTION.json) keeps the manifest at
// battle-evaluator/generated/replay-corpus-manifest.json, so this default also
// works unchanged once the companion mappings are restored.
fn default_fixture_path() -> Result<PathBuf> {
    Err(yixian_engine::EngineError::InvalidFixture(
        "solve_deck requires an explicit --fixture PATH: the public build has no bundled replay corpus manifest (restore the private companion for the corpus-based default)"
            .to_string(),
    ))
}

fn print_help() {
    println!(
        "Rust canonical deck solver\n\n\
usage: cargo run --release --bin solve_deck -- [fixture.json] [options]\n\n\
options:\n  --fixture PATH\n  --side p1|p2\n  --mode order|hand\n  \
--score-profile hpDelta\n  --top N\n  --max-evals N\n  \
--visit-order canonical|stratified\n  --seed N\n  --battle-seeds 1,2,3\n\n\
--fixture is required: the public build ships no replay corpus manifest, so the default fixture lookup is fail-closed (restore the private companion for the corpus-based default)."    );
}

fn parse_battle_seeds(value: &str) -> Result<Vec<u32>> {
    value
        .split(',')
        .map(|part| part.trim().parse::<u32>().ok())
        .collect::<Option<Vec<_>>>()
        .filter(|items| !items.is_empty())
        .ok_or_else(|| {
            yixian_engine::EngineError::InvalidFixture(
                "invalid --battle-seeds (expected comma-separated uint32 values)".to_string(),
            )
        })
}

fn parse_visit_order(value: &str) -> Result<VisitOrder> {
    match value {
        "canonical" => Ok(VisitOrder::Canonical),
        "stratified" => Ok(VisitOrder::Stratified),
        _ => Err(yixian_engine::EngineError::InvalidFixture(format!(
            "invalid visit order {value}"
        ))),
    }
}

fn parse_side(value: &str) -> Result<PlayerSide> {
    match value {
        "p1" | "P1" => Ok(PlayerSide::P1),
        "p2" | "P2" => Ok(PlayerSide::P2),
        _ => Err(yixian_engine::EngineError::InvalidFixture(format!(
            "invalid side {value}"
        ))),
    }
}

fn parse_mode(value: &str) -> Result<yixian_engine::solver::SolverMode> {
    match value {
        "order" => Ok(yixian_engine::solver::SolverMode::Order),
        "hand" => Ok(yixian_engine::solver::SolverMode::Hand),
        _ => Err(yixian_engine::EngineError::InvalidFixture(format!(
            "invalid mode {value}"
        ))),
    }
}

fn parse_score_profile(value: &str) -> Result<ScoreProfile> {
    match value {
        "hpDelta" | "hpdelta" | "hp-delta" => Ok(ScoreProfile::HpDelta),
        _ => Err(yixian_engine::EngineError::InvalidFixture(format!(
            "invalid score profile {value}"
        ))),
    }
}

fn parse_usize(value: Option<&String>, name: &str) -> Result<usize> {
    value
        .and_then(|value| value.parse::<usize>().ok())
        .filter(|value| *value > 0)
        .ok_or_else(|| yixian_engine::EngineError::InvalidFixture(format!("invalid {name}")))
}

fn parse_u64(value: Option<&String>, name: &str) -> Result<u64> {
    value
        .and_then(|value| value.parse::<u64>().ok())
        .ok_or_else(|| yixian_engine::EngineError::InvalidFixture(format!("invalid {name}")))
}

fn parse_deck(value: &str) -> Result<Vec<i64>> {
    let ids = value
        .split(',')
        .filter(|part| !part.trim().is_empty())
        .map(|part| {
            part.trim().parse::<i64>().map_err(|_| {
                yixian_engine::EngineError::InvalidFixture(format!("invalid deck id {part}"))
            })
        })
        .collect::<Result<Vec<_>>>()?;
    if ids.len() != yixian_engine::DECK_SIZE {
        return Err(yixian_engine::EngineError::InvalidFixture(format!(
            "--deck requires {} ids",
            yixian_engine::DECK_SIZE
        )));
    }
    Ok(ids)
}
