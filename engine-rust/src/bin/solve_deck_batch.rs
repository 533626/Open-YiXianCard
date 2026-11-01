use rayon::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::hash_map::DefaultHasher;
use std::collections::HashMap;
use std::hash::Hasher;
use std::io::{self, BufRead, Read, Write};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::{Mutex, OnceLock};
use yixian_engine::fixture::FixturePlayer;
use yixian_engine::model::PlayerSide;
use yixian_engine::solver::{
    evaluate_exact_deck_with_capture, evaluate_exact_deck_with_players_and_capture, ScoreProfile,
    SolverEvaluation,
};
use yixian_engine::{
    engine_identity, BattleFixture, EngineError, EngineIdentity, Result, DECK_SIZE,
};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchInput {
    /// 常驻模式（`--serve`）里用于把响应和请求对齐；一次性模式可省略。
    #[serde(default)]
    batch_id: Option<String>,
    /// 共享模式：请求不带整份 fixture，而是引用 `template` + `sides` 里的玩家侧（见 BatchRequest）。
    /// 分布布阵一批几千个请求只有几十个不同的玩家侧，整份 fixture 逐个序列化 / 解析是主要开销。
    #[serde(default)]
    template: Option<BattleFixture>,
    #[serde(default)]
    sides: Vec<FixturePlayer>,
    /// 精简回复：每个结果只回 winner / hpDeltaForSide / physiqueForSide / failed（字段名与完整评估相同）。
    /// 分布布阵、机制探索一批上万场只读这几项，完整评估约 400 字节 / 场，单线程序列化与解析占了引擎一半墙钟。
    #[serde(default)]
    compact: bool,
    /// 行式回复（优先于 compact）：`{"rows":[[胜方, 血差, 行动数, p1血, p2血, 体魄|null, 失败], ...]}`，
    /// 与请求同序、不带 id；胜方 1 = p1、2 = p2，失败 0 / 1。数值与 compact 同一份 f64。
    /// 驱动侧逐场只读这几项，免得每场解析出一个 7 键对象。
    #[serde(default)]
    rows: bool,
    requests: Vec<BatchRequest>,
}

struct SharedFixtures {
    template: Option<BattleFixture>,
    sides: Vec<FixturePlayer>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct BatchRequest {
    id: String,
    /// 完整 fixture；缺省时用 `players` 引用批次的 `sides`，其余字段取批次 `template`。
    #[serde(default)]
    fixture: Option<BattleFixture>,
    /// `[p1 在 sides 中的下标, p2 在 sides 中的下标]`
    #[serde(default)]
    players: Option<[usize; 2]>,
    /// 共享模式下覆盖 template 的先手方
    #[serde(default)]
    first_player_side: Option<PlayerSide>,
    side: PlayerSide,
    score_profile: ScoreProfile,
    deck: Vec<i64>,
    #[serde(default)]
    capture_decision_events: bool,
    #[serde(default)]
    battle_seeds: Option<Vec<u32>>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    engine_identity: EngineIdentity,
    results: Vec<BatchResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct ServeErrorReport {
    batch_id: Option<String>,
    error: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct BatchResult {
    id: String,
    evaluation: SolverEvaluation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactReport {
    #[serde(skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    results: Vec<CompactResult>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactResult {
    id: String,
    evaluation: CompactEvaluation,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct CompactEvaluation {
    winner: PlayerSide,
    hp_delta_for_side: f64,
    /// 动数钟读数（驱动侧速度识别）：本场行动回合数与双方终局血量；只是读数，不参与判定。
    actor_turn: f64,
    p1_hp: f64,
    p2_hp: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    physique_for_side: Option<f64>,
    failed: bool,
}

/// 一行：(胜方 1 = p1 / 2 = p2, 血差, 行动数, p1 血, p2 血, 体魄, 失败 0 / 1)
type ResultRow = (u8, f64, f64, f64, f64, Option<f64>, u8);

#[derive(Debug, Serialize)]
struct RowsReport {
    #[serde(rename = "batchId", skip_serializing_if = "Option::is_none")]
    batch_id: Option<String>,
    rows: Vec<ResultRow>,
}

fn rows_report(batch_id: Option<String>, results: Vec<BatchResult>) -> RowsReport {
    RowsReport {
        batch_id,
        rows: results
            .into_iter()
            .map(|r| {
                let e = r.evaluation;
                let winner = match e.winner {
                    PlayerSide::P1 => 1,
                    PlayerSide::P2 => 2,
                };
                (
                    winner,
                    e.hp_delta_for_side,
                    e.actor_turn,
                    e.p1_hp,
                    e.p2_hp,
                    e.physique_for_side,
                    u8::from(e.failed),
                )
            })
            .collect(),
    }
}

fn compact_report(batch_id: Option<String>, results: Vec<BatchResult>) -> CompactReport {
    CompactReport {
        batch_id,
        results: results
            .into_iter()
            .map(|r| CompactResult {
                id: r.id,
                evaluation: CompactEvaluation {
                    winner: r.evaluation.winner,
                    hp_delta_for_side: r.evaluation.hp_delta_for_side,
                    actor_turn: r.evaluation.actor_turn,
                    p1_hp: r.evaluation.p1_hp,
                    p2_hp: r.evaluation.p2_hp,
                    physique_for_side: r.evaluation.physique_for_side,
                    failed: r.evaluation.failed,
                },
            })
            .collect(),
    }
}

fn main() -> Result<()> {
    let serve = std::env::args().any(|arg| arg == "--serve")
        || std::env::var("SOLVE_DECK_BATCH_SERVE").ok().as_deref() == Some("1");
    if serve {
        return serve_loop();
    }

    let mut input = String::new();
    io::stdin().read_to_string(&mut input)?;
    let batch: BatchInput = serde_json::from_str(&input)?;
    let shared = SharedFixtures {
        template: batch.template,
        sides: batch.sides,
    };
    let results = solve_batch(batch.requests, &shared)?;

    println!(
        "{}",
        serde_json::to_string(&BatchReport {
            batch_id: batch.batch_id,
            engine_identity: engine_identity(),
            results,
        })?
    );
    Ok(())
}

/// 常驻模式：stdin/stdout 各一行 compact JSON（NDJSON）。
/// 避免 GA 每批 spawn 一次二进制——启动/退出抖动是整机平均只吃一半核的主因。
/// 算法与一次性模式共用 `solve_batch`，结果逐字节对齐（同一请求集）。
fn serve_loop() -> Result<()> {
    let stdin = io::stdin();
    let mut stdout = io::stdout();
    let mut reader = stdin.lock();
    let mut line = String::new();
    loop {
        line.clear();
        let n = reader.read_line(&mut line).map_err(|error| {
            EngineError::InvalidFixture(format!("solve_deck_batch --serve read: {error}"))
        })?;
        if n == 0 {
            break;
        }
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed == "{\"cmd\":\"quit\"}" || trimmed == "quit" {
            break;
        }
        let response = match serde_json::from_str::<BatchInput>(trimmed) {
            Ok(batch) => match solve_batch(
                batch.requests,
                &SharedFixtures {
                    template: batch.template,
                    sides: batch.sides,
                },
            ) {
                Ok(results) if batch.rows => serde_json::to_string(&rows_report(
                    batch.batch_id,
                    results,
                ))
                .map_err(|error| {
                    EngineError::InvalidFixture(format!("solve_deck_batch serialize: {error}"))
                })?,
                Ok(results) if batch.compact => serde_json::to_string(&compact_report(
                    batch.batch_id,
                    results,
                ))
                .map_err(|error| {
                    EngineError::InvalidFixture(format!("solve_deck_batch serialize: {error}"))
                })?,
                Ok(results) => serde_json::to_string(&BatchReport {
                    batch_id: batch.batch_id,
                    engine_identity: engine_identity(),
                    results,
                })
                .map_err(|error| {
                    EngineError::InvalidFixture(format!("solve_deck_batch serialize: {error}"))
                })?,
                Err(error) => serde_json::to_string(&ServeErrorReport {
                    batch_id: batch.batch_id,
                    error: error.to_string(),
                })
                .map_err(|ser| {
                    EngineError::InvalidFixture(format!("solve_deck_batch error serialize: {ser}"))
                })?,
            },
            Err(error) => serde_json::to_string(&ServeErrorReport {
                batch_id: None,
                error: format!("invalid batch json: {error}"),
            })
            .map_err(|ser| {
                EngineError::InvalidFixture(format!("solve_deck_batch error serialize: {ser}"))
            })?,
        };
        writeln!(stdout, "{response}").map_err(|error| {
            EngineError::InvalidFixture(format!("solve_deck_batch --serve write: {error}"))
        })?;
        stdout.flush().map_err(|error| {
            EngineError::InvalidFixture(format!("solve_deck_batch --serve flush: {error}"))
        })?;
    }
    report_matchup_cache();
    Ok(())
}

/// Solves every request across worker threads while preserving input order in
/// the output. On error the failure with the lowest request index wins, so
/// error selection matches the sequential loop this replaced.
///
/// 线程数 1（或一批只有一个请求）时在当前线程内联执行；否则交给进程内常驻的 rayon 线程池。
/// 原写法每批现起 / 回收一组线程：常驻模式一局上千批、每批几十场，起线程开销与线程本地缓存
/// （段来源驻留、效果调用单元池）每批归零都落在热路径上。
fn solve_batch(requests: Vec<BatchRequest>, shared: &SharedFixtures) -> Result<Vec<BatchResult>> {
    let digests = matchup_cache().map(|_| BatchDigests::new(shared));
    let digests = digests.as_ref();
    if batch_threads() == 1 || requests.len() <= 1 {
        return requests
            .iter()
            .map(|request| solve_request_cached(request, shared, digests))
            .collect();
    }
    // 有序收集后按下标顺序取首个错误，与顺序循环的报错选择一致
    batch_pool()
        .install(|| {
            requests
                .par_iter()
                .map(|request| solve_request_cached(request, shared, digests))
                .collect::<Vec<_>>()
        })
        .into_iter()
        .collect()
}

/// 对决置换表：进程内跨批记忆「同一对决 → 评估结果」。键是对决全部输入（模板 / 双方侧 / 整份夹具按
/// 解析后的规范序列化取 128 位摘要，加先手、出阵方、评分口径、卡组、战斗种子）的摘要；只收不要决策
/// 事件 / rule impact 的请求，报错不收。`SOLVE_DECK_BATCH_CACHE=<条数>` 开启（默认关：整局仿真主线
/// 驱动侧已有记忆，引擎侧实测命中仅 ~0.1%，开了只有内存成本），满了整表清空；退出时 stderr 报命中率。
struct MatchupCache {
    capacity: usize,
    table: Mutex<HashMap<u128, SolverEvaluation>>,
    hits: AtomicU64,
    misses: AtomicU64,
}

fn matchup_cache() -> Option<&'static MatchupCache> {
    static CACHE: OnceLock<Option<MatchupCache>> = OnceLock::new();
    CACHE
        .get_or_init(|| {
            let capacity = std::env::var("SOLVE_DECK_BATCH_CACHE")
                .ok()
                .and_then(|value| value.parse::<usize>().ok())
                .filter(|&value| value > 0)?;
            Some(MatchupCache {
                capacity,
                table: Mutex::new(HashMap::new()),
                hits: AtomicU64::new(0),
                misses: AtomicU64::new(0),
            })
        })
        .as_ref()
}

/// 128 位内容摘要：两路不同前缀的 SipHash（标准库固定键，跨进程稳定）。
fn digest(bytes: &[u8]) -> u128 {
    let half = |salt: u8| {
        let mut hasher = DefaultHasher::new();
        hasher.write_u8(salt);
        hasher.write(bytes);
        hasher.finish()
    };
    (u128::from(half(0x5a)) << 64) | u128::from(half(0xa5))
}

fn digest_of<T: Serialize>(value: &T) -> u128 {
    digest(&serde_json::to_vec(value).expect("fixture serializes"))
}

/// 一批共享部分的摘要（只在开启置换表时算）：模板（不含双方侧与先手）与各侧。
struct BatchDigests {
    template: Option<u128>,
    sides: Vec<u128>,
}

impl BatchDigests {
    fn new(shared: &SharedFixtures) -> Self {
        // 模板里的双方侧与先手由请求覆盖，不进摘要
        let template = shared.template.as_ref().map(|template| {
            let mut bare = serde_json::to_value(template).expect("template serializes");
            if let Some(object) = bare.as_object_mut() {
                object.remove("players");
                object.remove("firstPlayerSide");
            }
            digest_of(&bare)
        });
        Self {
            template,
            sides: shared.sides.iter().map(digest_of).collect(),
        }
    }

    fn key(&self, request: &BatchRequest) -> Option<u128> {
        if request.capture_decision_events {
            return None;
        }
        let mut parts: Vec<u8> = Vec::with_capacity(128);
        if let Some(fixture) = &request.fixture {
            parts.push(0);
            parts.extend_from_slice(&digest_of(fixture).to_le_bytes());
        } else {
            let [p1, p2] = request.players?;
            parts.push(1);
            parts.extend_from_slice(&self.template?.to_le_bytes());
            parts.extend_from_slice(&self.sides.get(p1)?.to_le_bytes());
            parts.extend_from_slice(&self.sides.get(p2)?.to_le_bytes());
            parts.push(match request.first_player_side {
                None => 0,
                Some(PlayerSide::P1) => 1,
                Some(PlayerSide::P2) => 2,
            });
        }
        parts.push(request.side as u8);
        parts.extend_from_slice(&serde_json::to_vec(&request.score_profile).ok()?);
        for card in &request.deck {
            parts.extend_from_slice(&card.to_le_bytes());
        }
        parts.push(0xff);
        for seed in request.battle_seeds.iter().flatten() {
            parts.extend_from_slice(&seed.to_le_bytes());
        }
        Some(digest(&parts))
    }
}

fn solve_request_cached(
    request: &BatchRequest,
    shared: &SharedFixtures,
    digests: Option<&BatchDigests>,
) -> Result<BatchResult> {
    let (Some(cache), Some(key)) = (matchup_cache(), digests.and_then(|d| d.key(request))) else {
        return solve_request(request, shared);
    };
    if let Some(evaluation) = cache
        .table
        .lock()
        .expect("matchup cache")
        .get(&key)
        .cloned()
    {
        cache.hits.fetch_add(1, Ordering::Relaxed);
        return Ok(BatchResult {
            id: request.id.clone(),
            evaluation,
        });
    }
    cache.misses.fetch_add(1, Ordering::Relaxed);
    let result = solve_request(request, shared)?;
    let mut table = cache.table.lock().expect("matchup cache");
    if table.len() >= cache.capacity {
        table.clear();
    }
    table.insert(key, result.evaluation.clone());
    Ok(result)
}

fn report_matchup_cache() {
    if let Some(cache) = matchup_cache() {
        let hits = cache.hits.load(Ordering::Relaxed);
        let misses = cache.misses.load(Ordering::Relaxed);
        eprintln!(
            "solve_deck_batch matchup cache: {hits} hits / {} lookups ({:.2}%)",
            hits + misses,
            100.0 * hits as f64 / (hits + misses).max(1) as f64
        );
    }
}

fn batch_threads() -> usize {
    static THREADS: OnceLock<usize> = OnceLock::new();
    *THREADS.get_or_init(|| {
        std::env::var("SOLVE_DECK_BATCH_THREADS")
            .ok()
            .and_then(|value| value.parse::<usize>().ok())
            .filter(|&value| value > 0)
            .unwrap_or_else(|| {
                std::thread::available_parallelism()
                    .map(|value| value.get())
                    .unwrap_or(1)
            })
    })
}

fn batch_pool() -> &'static rayon::ThreadPool {
    static POOL: OnceLock<rayon::ThreadPool> = OnceLock::new();
    POOL.get_or_init(|| {
        rayon::ThreadPoolBuilder::new()
            .num_threads(batch_threads())
            .thread_name(|index| format!("solve-batch-{index}"))
            .build()
            .expect("solve_deck_batch thread pool")
    })
}

fn resolve_fixture<'a>(
    request: &'a BatchRequest,
    shared: &SharedFixtures,
    owned: &'a mut Option<BattleFixture>,
) -> Result<&'a BattleFixture> {
    if let Some(fixture) = &request.fixture {
        return Ok(fixture);
    }
    let missing = |what: &str| {
        EngineError::InvalidFixture(format!(
            "batch request {} has no fixture and {what}",
            request.id
        ))
    };
    let [p1, p2] = request
        .players
        .ok_or_else(|| missing("no players reference"))?;
    let template = shared
        .template
        .as_ref()
        .ok_or_else(|| missing("the batch has no template"))?;
    let side_at = |index: usize| {
        shared
            .sides
            .get(index)
            .cloned()
            .ok_or_else(|| missing(&format!("side index {index} is out of range")))
    };
    let mut fixture = template.clone();
    fixture.players.p1 = side_at(p1)?;
    fixture.players.p2 = side_at(p2)?;
    if let Some(first) = request.first_player_side {
        fixture.first_player_side = first;
    }
    Ok(owned.insert(fixture))
}

fn solve_request(request: &BatchRequest, shared: &SharedFixtures) -> Result<BatchResult> {
    if request.fixture.is_none() {
        let missing = |what: &str| {
            EngineError::InvalidFixture(format!(
                "batch request {} has no fixture and {what}",
                request.id
            ))
        };
        let [p1, p2] = request
            .players
            .ok_or_else(|| missing("no players reference"))?;
        let template = shared
            .template
            .as_ref()
            .ok_or_else(|| missing("the batch has no template"))?;
        let side_at = |index: usize| {
            shared
                .sides
                .get(index)
                .ok_or_else(|| missing(&format!("side index {index} is out of range")))
        };
        let p1 = side_at(p1)?;
        let p2 = side_at(p2)?;
        template.validate_with_players(p1, p2)?;
        if request.deck.len() != DECK_SIZE {
            return Err(EngineError::InvalidFixture(format!(
                "batch request {} deck has {} cards, expected {DECK_SIZE}",
                request.id,
                request.deck.len()
            )));
        }
        let evaluation = evaluate_exact_deck_with_players_and_capture(
            template,
            p1,
            p2,
            request
                .first_player_side
                .unwrap_or(template.first_player_side),
            request.side,
            &request.deck,
            request.score_profile,
            request.battle_seeds.clone(),
            request.capture_decision_events,
        )?;
        return Ok(BatchResult {
            id: request.id.clone(),
            evaluation,
        });
    }
    let mut owned = None;
    let fixture = resolve_fixture(request, shared, &mut owned)?;
    fixture.validate()?;
    if request.deck.len() != DECK_SIZE {
        return Err(EngineError::InvalidFixture(format!(
            "batch request {} deck has {} cards, expected {DECK_SIZE}",
            request.id,
            request.deck.len(),
        )));
    }
    // 只打指定阵型这一场（solve_deck 的 exact 路径会多打一场 baseline）
    let evaluation = evaluate_exact_deck_with_capture(
        fixture,
        request.side,
        &request.deck,
        request.score_profile,
        request.battle_seeds.clone(),
        request.capture_decision_events,
    )?;
    Ok(BatchResult {
        id: request.id.clone(),
        evaluation,
    })
}

#[cfg(all(test, feature = "private-fixtures"))]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capture_decision_events_defaults_false_and_reads_camel_case() {
        let fixture_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("engine-rust has project root parent")
            .join("battle-evaluator/fixtures/contracts/decisions/exorcism-pill-seed17.json");
        let fixture: serde_json::Value = serde_json::from_str(
            &std::fs::read_to_string(fixture_path).expect("shared decision fixture reads"),
        )
        .expect("shared decision fixture parses");
        let request = |capture_decisions: Option<bool>| {
            let mut value = json!({
                "id": "decision-contract",
                "fixture": fixture.clone(),
                "side": "p1",
                "scoreProfile": "hpDelta",
                "deck": [16, 2_000_004, 0, 0, 0, 0, 0, 0]
            });
            if let Some(capture) = capture_decisions {
                value["captureDecisionEvents"] = json!(capture);
            }
            value
        };

        let without_capture: BatchRequest =
            serde_json::from_value(request(None)).expect("default request parses");
        let with_capture: BatchRequest =
            serde_json::from_value(request(Some(true))).expect("capture request parses");

        assert!(!without_capture.capture_decision_events);
        assert!(with_capture.capture_decision_events);
    }
}
