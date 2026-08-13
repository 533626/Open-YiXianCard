use crate::solver::{
    explain_fixture_counterfactuals, explain_fixture_rule_impact, solve_deck,
    CounterfactualElement, ScoreProfile, SolveDeckOptions, SolverMode, VisitOrder,
};
use crate::PlayerSide;
use crate::{run_replay_fixture_with_ui_events, trace_replay_fixture_hooks, BattleFixture};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(untagged)]
enum WasmResponse<T: Serialize> {
    Success { ok: bool, run: T },
    Failure { ok: bool, error: String },
}

#[no_mangle]
pub extern "C" fn yixian_alloc(len: usize) -> *mut u8 {
    let mut bytes = Vec::<u8>::with_capacity(len);
    let pointer = bytes.as_mut_ptr();
    std::mem::forget(bytes);
    pointer
}

#[no_mangle]
pub unsafe extern "C" fn yixian_dealloc(pointer: *mut u8, len: usize) {
    if !pointer.is_null() && len > 0 {
        drop(Vec::from_raw_parts(pointer, 0, len));
    }
}

/// Accepts one canonical BattleFixture JSON document and returns a packed
/// `(length << 32) | pointer` for a UTF-8 JSON response. The caller owns the
/// returned buffer and releases it through `yixian_dealloc`.
#[no_mangle]
pub unsafe extern "C" fn yixian_simulate_json(pointer: *const u8, len: usize) -> u64 {
    let input = std::slice::from_raw_parts(pointer, len);
    let response = serde_json::from_slice::<BattleFixture>(input)
        .map_err(crate::EngineError::from)
        .and_then(|fixture| {
            fixture.validate()?;
            run_replay_fixture_with_ui_events(&fixture)
        });
    let output = match response {
        Ok(run) => serde_json::to_vec(&WasmResponse::Success { ok: true, run }),
        Err(error) => serde_json::to_vec(&WasmResponse::<()>::Failure {
            ok: false,
            error: error.to_string(),
        }),
    }
    .expect("WASM response serialization must not fail");
    pack_output(output)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmSolveRequest {
    fixture: BattleFixture,
    side: PlayerSide,
    mode: SolverMode,
    visit_order: VisitOrder,
    visit_seed: u64,
    score_profile: ScoreProfile,
    top_n: usize,
    max_evaluations: usize,
    #[serde(default)]
    battle_seeds: Option<Vec<u32>>,
}

#[no_mangle]
pub unsafe extern "C" fn yixian_solve_json(pointer: *const u8, len: usize) -> u64 {
    let input = std::slice::from_raw_parts(pointer, len);
    let response = serde_json::from_slice::<WasmSolveRequest>(input)
        .map_err(crate::EngineError::from)
        .and_then(|request| {
            request.fixture.validate()?;
            solve_deck(
                &request.fixture,
                SolveDeckOptions {
                    side: request.side,
                    mode: request.mode,
                    visit_order: request.visit_order,
                    visit_seed: request.visit_seed,
                    top: request.top_n,
                    max_evaluations: request.max_evaluations,
                    score_profile: request.score_profile,
                    exact_deck_ids: None,
                    battle_seeds: request.battle_seeds,
                    capture_rule_impact: false,
                },
            )
        });
    let output = match response {
        Ok(run) => serde_json::to_vec(&WasmResponse::Success { ok: true, run }),
        Err(error) => serde_json::to_vec(&WasmResponse::<()>::Failure {
            ok: false,
            error: error.to_string(),
        }),
    }
    .expect("WASM solver response serialization must not fail");
    pack_output(output)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmExplainRequest {
    fixture: BattleFixture,
    side: PlayerSide,
}

/// Returns `canonical-rule-impact-v1` for one fixture so the browser can explain
/// a finished battle from the same attribution the analysis pipeline consumes,
/// instead of re-deriving value channels on the consumer side.
#[no_mangle]
pub unsafe extern "C" fn yixian_explain_json(pointer: *const u8, len: usize) -> u64 {
    let input = std::slice::from_raw_parts(pointer, len);
    let response = serde_json::from_slice::<WasmExplainRequest>(input)
        .map_err(crate::EngineError::from)
        .and_then(|request| {
            request.fixture.validate()?;
            explain_fixture_rule_impact(&request.fixture, request.side).map_err(|error| {
                crate::EngineError::Battle(crate::BattleError::Invariant { message: error })
            })
        });
    let output = match response {
        Ok(run) => serde_json::to_vec(&WasmResponse::Success { ok: true, run }),
        Err(error) => serde_json::to_vec(&WasmResponse::<()>::Failure {
            ok: false,
            error: error.to_string(),
        }),
    }
    .expect("WASM explain response serialization must not fail");
    pack_output(output)
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WasmCounterfactualRequest {
    fixture: BattleFixture,
    side: PlayerSide,
    elements: Vec<CounterfactualElement>,
}

/// Removes each requested opening-state element independently, replays with the
/// same decision/random tapes, and reports both the clean-prefix and terminal HP
/// gap changes. This is an explanation API only; exact replay assertions continue
/// to run through the unperturbed parity surface.
#[no_mangle]
pub unsafe extern "C" fn yixian_counterfactual_json(pointer: *const u8, len: usize) -> u64 {
    let input = std::slice::from_raw_parts(pointer, len);
    let response = serde_json::from_slice::<WasmCounterfactualRequest>(input)
        .map_err(crate::EngineError::from)
        .and_then(|request| {
            request.fixture.validate()?;
            explain_fixture_counterfactuals(&request.fixture, request.side, &request.elements)
                .map_err(|error| {
                    crate::EngineError::Battle(crate::BattleError::Invariant { message: error })
                })
        });
    let output = match response {
        Ok(run) => serde_json::to_vec(&WasmResponse::Success { ok: true, run }),
        Err(error) => serde_json::to_vec(&WasmResponse::<()>::Failure {
            ok: false,
            error: error.to_string(),
        }),
    }
    .expect("WASM counterfactual response serialization must not fail");
    pack_output(output)
}

/// Returns the canonical hook chain for one fixture: every hook invocation and
/// the fields it changed. The battle view joins it onto the parity event stream
/// by `eventIndex`, so the browser reads the engine's own chain instead of
/// reconstructing a plausible one from state snapshots.
#[no_mangle]
pub unsafe extern "C" fn yixian_trace_json(pointer: *const u8, len: usize) -> u64 {
    let input = std::slice::from_raw_parts(pointer, len);
    let response = serde_json::from_slice::<BattleFixture>(input)
        .map_err(crate::EngineError::from)
        .and_then(|fixture| {
            fixture.validate()?;
            trace_replay_fixture_hooks(&fixture)
        });
    let output = match response {
        Ok(run) => serde_json::to_vec(&WasmResponse::Success { ok: true, run }),
        Err(error) => serde_json::to_vec(&WasmResponse::<()>::Failure {
            ok: false,
            error: error.to_string(),
        }),
    }
    .expect("WASM hook trace response serialization must not fail");
    pack_output(output)
}

fn pack_output(output: Vec<u8>) -> u64 {
    let bytes = output.into_boxed_slice();
    let len = bytes.len();
    let pointer = Box::into_raw(bytes) as *mut u8;
    ((len as u64) << 32) | pointer as u64
}
