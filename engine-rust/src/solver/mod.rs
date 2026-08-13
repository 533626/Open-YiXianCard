mod advisor;
mod counterfactual;
mod exact_search;
mod rule_impact;
#[cfg(test)]
mod tests_advisor;
#[cfg(test)]
mod tests_value;
mod value;
mod value_weights;
mod variant;

pub use advisor::{
    advise_remaining_order, advisor_candidate_run, AdvisorCandidate, AdvisorOptions, AdvisorReport,
    DEFAULT_ADVISOR_MAX_EVALUATIONS, DEFAULT_ADVISOR_TOP,
};
pub use counterfactual::{
    explain_fixture_counterfactuals, CounterfactualDivergenceReason, CounterfactualElement,
    CounterfactualElementResult, CounterfactualReport, COUNTERFACTUAL_SCHEMA_VERSION,
};
pub use exact_search::{
    solve_deck, SolveDeckOptions, SolveDeckResult, SolverDeckResult, SolverMode, VisitOrder,
};
pub use rule_impact::{
    SolverRuleImpactCard, SolverRuleImpactCheckpoint, SolverRuleImpactContribution,
    SolverRuleImpactReport,
};
pub use value::{ScoreProfile, SolverValueMetrics};
pub use variant::{
    evaluate_fixture, evaluate_fixture_deck, explain_fixture_rule_impact, SolverEvaluation,
};

pub const FAILED_SCORE: i64 = -1_000_000_000;
