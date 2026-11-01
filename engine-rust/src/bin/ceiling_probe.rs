//! Diagnostic harness using the production batch adapter and battle kernel.
//! Input is JSONL {"input": <original batch JSON>, "rows": <original replies>}.
//! Every row must match, including IEEE bits and failure results.
#[allow(dead_code)]
mod production {
    include!("solve_deck_batch.rs");

    fn exact_value(actual: &serde_json::Value, expected: &serde_json::Value) -> bool {
        match (actual, expected) {
            (serde_json::Value::Number(a), serde_json::Value::Number(b)) => {
                a.as_f64().map(f64::to_bits) == b.as_f64().map(f64::to_bits)
            }
            (serde_json::Value::Array(a), serde_json::Value::Array(b)) => {
                a.len() == b.len() && a.iter().zip(b).all(|(a, b)| exact_value(a, b))
            }
            _ => actual == expected,
        }
    }

    pub(super) fn probe(path: &str, output: &str, mode: &str) -> Result<()> {
        use std::time::Instant;
        use yixian_engine::performance::{self, CATEGORIES, CATEGORY_COUNT};
        let file = std::fs::File::open(path)?;
        let mut reports = Vec::new();
        let mut total_ns = 0u64;
        let mut total_requests = 0usize;
        let mut clock_rates = Vec::new();
        for _ in 0..5 {
            let before = Instant::now();
            let (ticks, cpu) = performance::ticks();
            while before.elapsed().as_millis() < 20 {
                std::hint::spin_loop();
            }
            let (end, final_cpu) = performance::ticks();
            let seconds = before.elapsed().as_secs_f64();
            if cpu != final_cpu || end < ticks {
                return Err(EngineError::InvalidFixture(
                    "TSC migrated or regressed".into(),
                ));
            }
            clock_rates.push((end - ticks) as f64 / seconds);
        }
        let minimum_rate = clock_rates.iter().copied().reduce(f64::min).unwrap();
        let maximum_rate = clock_rates.iter().copied().reduce(f64::max).unwrap();
        if maximum_rate / minimum_rate > 1.001 {
            return Err(EngineError::InvalidFixture(
                "unstable calibrated TSC rate".into(),
            ));
        }
        let rate = clock_rates.iter().sum::<f64>() / clock_rates.len() as f64;
        performance::begin();
        let calibration_start = Instant::now();
        for _ in 0..100_000 {
            drop(performance::Timer::new(2));
        }
        let calibration_wall = calibration_start.elapsed().as_nanos();
        let calibration = performance::finish();
        for (batch_index, line) in std::io::BufReader::new(file).lines().enumerate() {
            let record: serde_json::Value = serde_json::from_str(&line?)?;
            let batch: BatchInput =
                serde_json::from_str(record["input"].as_str().ok_or_else(|| {
                    EngineError::InvalidFixture("missing captured input".into())
                })?)?;
            let shared = SharedFixtures {
                template: batch.template,
                sides: batch.sides,
            };
            let mut ready = Vec::new();
            let prep_start = Instant::now();
            if mode == "prepared" {
                for request in &batch.requests {
                    let mut owned = None;
                    let fixture = resolve_fixture(request, &shared, &mut owned)?;
                    fixture.validate()?;
                    ready.push(yixian_engine::solver::prepare_exact_fixture(
                        fixture,
                        request.side,
                        &request.deck,
                        request.battle_seeds.as_deref(),
                    )?);
                }
            }
            let prep_ns = prep_start.elapsed().as_nanos() as u64;
            let mut results = Vec::with_capacity(batch.requests.len());
            let mut durations = Vec::with_capacity(batch.requests.len());
            let mut phases = [0u64; CATEGORY_COUNT];
            let mut phase_calls = [0u64; CATEGORY_COUNT];
            let mut sampled = 0usize;
            let mut preview_paths = [0u64; 3];
            for (index, request) in batch.requests.iter().enumerate() {
                let sample = index % 64 == 0;
                if sample {
                    performance::begin();
                }
                let (start, cpu) = performance::ticks();
                let result = if mode == "prepared" {
                    let mut evaluation = yixian_engine::solver::evaluate_fixture(
                        &ready[index],
                        request.side,
                        request.score_profile,
                    );
                    if !request.capture_decision_events {
                        evaluation.decision_events.clear();
                    }
                    BatchResult {
                        id: request.id.clone(),
                        evaluation,
                    }
                } else {
                    solve_request(request, &shared)?
                };
                let (end, final_cpu) = performance::ticks();
                if cpu != final_cpu || end < start {
                    return Err(EngineError::InvalidFixture(
                        "request TSC migrated or regressed".into(),
                    ));
                }
                durations.push(((end - start) as f64 / rate * 1e9) as u64);
                if sample {
                    let record = performance::finish();
                    if record.migrations > 0 {
                        return Err(EngineError::InvalidFixture(
                            "phase TSC migrated or regressed".into(),
                        ));
                    }
                    for i in 0..CATEGORY_COUNT {
                        phases[i] += (record.ticks[i] as f64 / rate * 1e9) as u64;
                        phase_calls[i] += record.calls[i];
                    }
                    for i in 0..3 {
                        preview_paths[i] += record.preview_paths[i];
                    }
                    sampled += 1;
                }
                results.push(result);
            }
            let observed = serde_json::to_value(rows_report(batch.batch_id, results))?;
            if !exact_value(&observed["rows"], &record["rows"]) {
                return Err(EngineError::InvalidFixture(format!(
                    "row mismatch in batch {batch_index}"
                )));
            }
            let work_ns: u64 = durations.iter().sum();
            let maximum_ns = durations.iter().copied().max().unwrap_or(0);
            let mut ideal = Vec::new();
            for cores in [1, 2, 4, 8] {
                let mut workers = vec![0u64; cores];
                let mut sorted = durations.clone();
                sorted.sort_unstable_by(|a, b| b.cmp(a));
                for duration in sorted {
                    let worker = workers
                        .iter()
                        .enumerate()
                        .min_by_key(|(_, v)| *v)
                        .unwrap()
                        .0;
                    workers[worker] += duration;
                }
                ideal.push(serde_json::json!({"cores":cores,"lower_ns":(work_ns+cores as u64-1)/cores as u64,
                    "longest_request_ns":maximum_ns,"greedy_ns":workers.into_iter().max().unwrap_or(0)}));
            }
            reports.push(serde_json::json!({"batch":batch_index,"requests":durations.len(),
                "prepare_ns":prep_ns,"work_ns":work_ns,"maximum_ns":maximum_ns,
                "sampled":sampled,"sample_ns":phases,"sample_calls":phase_calls,"preview_paths":preview_paths,
                "ideal_scheduling":ideal}));
            total_ns += work_ns;
            total_requests += durations.len();
        }
        let report = serde_json::json!({"mode":mode,"all_rows_exact":true,"categories":CATEGORIES,
            "requests":total_requests,"work_ns":total_ns,"batches":reports,
            "calibration":calibration,"calibration_wall_ns":calibration_wall,
            "clock_rates_hz":clock_rates,"tsc_hz":rate,
            "engine_identity":engine_identity()});
        std::fs::write(output, serde_json::to_vec_pretty(&report)?)?;
        println!(
            "{mode}: {total_requests} requests, all rows exact, {:.3}s timed work",
            total_ns as f64 / 1e9
        );
        Ok(())
    }
}

fn main() -> yixian_engine::Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 4 || !["service", "prepared"].contains(&args[3].as_str()) {
        return Err(yixian_engine::EngineError::InvalidFixture(
            "usage: ceiling_probe captured.jsonl report.json service|prepared".into(),
        ));
    }
    production::probe(&args[1], &args[2], &args[3])
}
