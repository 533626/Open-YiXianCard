mod advisor;
mod counterfactual;
mod exact_search;
#[cfg(test)]
mod tests_advisor;
#[cfg(test)]
mod tests_solver;
mod variant;

pub use advisor::{
    advise_remaining_order, advisor_candidate_run, AdvisorCandidate, AdvisorOptions, AdvisorReport,
    DEFAULT_ADVISOR_MAX_EVALUATIONS, DEFAULT_ADVISOR_TOP,
};
pub use counterfactual::{
    explain_fixture_counterfactuals, CounterfactualDivergenceReason, CounterfactualElement,
    CounterfactualElementResult, CounterfactualReport, COUNTERFACTUAL_SCHEMA_VERSION,
};
#[cfg(all(feature = "perf-ceiling", not(target_arch = "wasm32")))]
pub use exact_search::prepare_exact_fixture;
pub use exact_search::{
    evaluate_exact_deck, evaluate_exact_deck_with_capture, evaluate_exact_deck_with_players,
    evaluate_exact_deck_with_players_and_capture, solve_deck, SolveDeckOptions, SolveDeckResult,
    SolverDeckResult, SolverMode, VisitOrder,
};
pub use variant::{evaluate_fixture, evaluate_fixture_deck, ScoreProfile, SolverEvaluation};

pub const FAILED_SCORE: i64 = -1_000_000_000;
