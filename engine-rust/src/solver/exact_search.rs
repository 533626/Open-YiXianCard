use super::value::ScoreProfile;
use super::variant::{
    capture_rule_impacts_for_fixture, create_fixture_variant, evaluate_fixture_across_battle_seeds,
    evaluate_fixture_deck_across_battle_seeds, fixture_deck_candidates, fixture_hand_candidates,
    SolverEvaluation,
};
use crate::fixture::BattleFixture;
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use crate::{EngineError, Result as EngineResult};
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};
use std::thread;
#[cfg(target_arch = "wasm32")]
use std::time::Duration;
#[cfg(not(target_arch = "wasm32"))]
use std::time::Instant as SolverInstant;

#[path = "candidate_decks.rs"]
mod candidate_decks;
#[path = "stratified.rs"]
mod stratified;
use candidate_decks::{
    bucket_cards, card_ids, compare_value_results, compare_win_first_hp_delta_results,
    complete_deck_ids, deck_key, deck_sort_key, enumerate_decks, evals_per_sec,
    evaluate_candidate_decks, seed_audit_fields, sort_results, summarize_evaluated_results,
    EvaluatedResultsSummary,
};
use stratified::{
    candidate_deck_from_parts, collect_stratified_candidate_decks, collect_stratified_shard,
    count_deck_sequences, plan_stratified_shards,
};

#[cfg(target_arch = "wasm32")]
struct SolverInstant;

#[cfg(target_arch = "wasm32")]
impl SolverInstant {
    fn now() -> Self {
        Self
    }

    fn elapsed(&self) -> Duration {
        Duration::ZERO
    }
}

const DEFAULT_TOP_N: usize = 20;
const DEFAULT_MAX_EVALUATIONS: usize = 200_000;
const STRATIFIED_POPULATION_LIMIT: usize = 1_000_000;
const STRATIFIED_SHARD_TARGET: usize = 4_096;
const STRATIFIED_SORT_CHUNK_TARGET: usize = 4_096;
const FNV_OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
const FNV_PRIME: u64 = 0x0000_0100_0000_01b3;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum SolverMode {
    Order,
    Hand,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum VisitOrder {
    Canonical,
    Stratified,
}

#[derive(Debug, Clone)]
pub struct SolveDeckOptions {
    pub side: PlayerSide,
    pub mode: SolverMode,
    pub visit_order: VisitOrder,
    pub visit_seed: u64,
    pub top: usize,
    pub max_evaluations: usize,
    pub score_profile: ScoreProfile,
    pub exact_deck_ids: Option<Vec<i64>>,
    pub battle_seeds: Option<Vec<u32>>,
    pub capture_rule_impact: bool,
}

impl Default for SolveDeckOptions {
    fn default() -> Self {
        Self {
            side: PlayerSide::P1,
            mode: SolverMode::Order,
            visit_order: VisitOrder::Canonical,
            visit_seed: 0,
            top: DEFAULT_TOP_N,
            max_evaluations: DEFAULT_MAX_EVALUATIONS,
            score_profile: ScoreProfile::HpDelta,
            exact_deck_ids: None,
            battle_seeds: None,
            capture_rule_impact: false,
        }
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolveDeckResult {
    pub mode: SolverMode,
    pub visit_order: VisitOrder,
    pub visit_seed: u64,
    pub side: PlayerSide,
    pub confidence: String,
    pub evaluated_count: usize,
    pub deck_count: usize,
    pub win_deck_count: usize,
    pub first_win_rank: Option<usize>,
    pub first_win_deck_key: Option<String>,
    pub best_hp_delta_for_side: f64,
    pub best_hp_delta_deck_key: String,
    pub value_top_deck_key: String,
    pub skipped_duplicate_count: usize,
    pub candidate_card_count: usize,
    pub truncated: bool,
    pub elapsed_ms: u128,
    pub evals_per_sec: f64,
    pub baseline: SolverEvaluation,
    pub baseline_deck: Vec<i64>,
    pub results: Vec<SolverDeckResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub seeds_used: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub synthetic_decision_seeds_used: Option<Vec<u32>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub used_synthetic_decisions: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SolverDeckResult {
    pub rank: usize,
    pub score: f64,
    pub deck: Vec<i64>,
    pub leftover_hand_card_ids: Vec<i64>,
    pub evaluation: SolverEvaluation,
    pub deck_key: String,
    #[serde(skip)]
    sort_key: String,
}

#[derive(Clone)]
struct CandidateBucket {
    card: CardDefinition,
    count: usize,
}

#[derive(Clone)]
struct CandidateDeck {
    key: String,
    sort_key: String,
    deck: Vec<CardDefinition>,
    leftover_hand_card_ids: Vec<i64>,
}

struct StratifiedPopulation {
    decks: Vec<CandidateDeck>,
    skipped_duplicate_count: usize,
    population_truncated: bool,
}

#[derive(Clone)]
struct StratifiedShard {
    prefix: Vec<CardDefinition>,
    buckets: Vec<CandidateBucket>,
    limit: usize,
}

struct StratifiedSortItem {
    hash: u64,
    deck: CandidateDeck,
}

struct CandidateEvaluationOptions<'a> {
    fixture: &'a BattleFixture,
    side: PlayerSide,
    mode: SolverMode,
    score_profile: ScoreProfile,
    battle_seeds: Option<&'a [u32]>,
}

pub fn solve_deck(
    fixture: &BattleFixture,
    options: SolveDeckOptions,
) -> EngineResult<SolveDeckResult> {
    let options = normalize_solve_deck_options(options)?;
    Ok(solve_deck_normalized(fixture, options))
}

fn normalize_solve_deck_options(mut options: SolveDeckOptions) -> EngineResult<SolveDeckOptions> {
    if options.max_evaluations == 0 {
        return Err(EngineError::InvalidFixture(
            "maxEvaluations must be greater than zero".to_string(),
        ));
    }
    if let Some(seeds) = options.battle_seeds.as_mut() {
        if seeds.is_empty() {
            return Err(EngineError::InvalidFixture(
                "battleSeeds must not be empty".to_string(),
            ));
        }
        seeds.sort_unstable();
        seeds.dedup();
    }
    Ok(options)
}

fn solve_deck_normalized(fixture: &BattleFixture, options: SolveDeckOptions) -> SolveDeckResult {
    let started = SolverInstant::now();
    let top_n = options.top.max(1);
    let baseline_deck = fixture_deck_candidates(fixture, options.side);
    let mut baseline = evaluate_fixture_across_battle_seeds(
        fixture,
        options.side,
        options.score_profile,
        options.battle_seeds.as_deref(),
    );
    let baseline_ids = card_ids(&baseline_deck);

    if let Some(deck_ids) = options.exact_deck_ids {
        let deck = complete_deck_ids(&baseline_deck, &deck_ids);
        let evaluation = evaluate_fixture_deck_across_battle_seeds(
            fixture,
            options.side,
            &deck,
            None,
            options.score_profile,
            options.battle_seeds.as_deref(),
        );
        let mut result = SolverDeckResult {
            rank: 1,
            score: evaluation.score,
            deck: card_ids(&deck),
            leftover_hand_card_ids: Vec::new(),
            deck_key: deck_key(&deck),
            sort_key: deck_sort_key(&deck),
            evaluation,
        };
        if options.capture_rule_impact {
            attach_rule_impacts(
                fixture,
                &baseline_deck,
                options.side,
                options.mode,
                options.battle_seeds.as_deref(),
                &mut baseline,
                std::slice::from_mut(&mut result),
            );
        }
        let summary = summarize_evaluated_results(std::slice::from_ref(&result));
        let seed_audit = seed_audit_fields(
            std::iter::once(&baseline).chain(std::iter::once(&result.evaluation)),
        );
        let elapsed_ms = started.elapsed().as_millis();
        return SolveDeckResult {
            mode: options.mode,
            visit_order: options.visit_order,
            visit_seed: options.visit_seed,
            side: options.side,
            confidence: "exact".to_string(),
            evaluated_count: 1,
            deck_count: summary.deck_count,
            win_deck_count: summary.win_deck_count,
            first_win_rank: summary.first_win_rank,
            first_win_deck_key: summary.first_win_deck_key,
            best_hp_delta_for_side: summary.best_hp_delta_for_side,
            best_hp_delta_deck_key: summary.best_hp_delta_deck_key,
            value_top_deck_key: summary.value_top_deck_key,
            skipped_duplicate_count: 0,
            candidate_card_count: deck.len(),
            truncated: false,
            elapsed_ms,
            evals_per_sec: evals_per_sec(1, started.elapsed().as_secs_f64()),
            baseline,
            baseline_deck: baseline_ids,
            results: vec![result],
            seeds_used: seed_audit.0,
            synthetic_decision_seeds_used: seed_audit.1,
            used_synthetic_decisions: seed_audit.2,
        };
    }

    let hand_candidates = if options.mode == SolverMode::Hand {
        fixture_hand_candidates(fixture, options.side)
    } else {
        Vec::new()
    };
    let mut candidates = baseline_deck.clone();
    candidates.extend(hand_candidates);
    let buckets = bucket_cards(candidates, options.mode);
    let candidate_card_count = buckets.iter().map(|bucket| bucket.count).sum::<usize>();
    let evaluation_options = CandidateEvaluationOptions {
        fixture,
        side: options.side,
        mode: options.mode,
        score_profile: options.score_profile,
        battle_seeds: options.battle_seeds.as_deref(),
    };

    if options.visit_order == VisitOrder::Canonical {
        let CandidateDeckStreamResult {
            evaluated_count,
            skipped_duplicate_count,
            truncated,
            summary,
            mut results,
        } = evaluate_canonical_candidate_decks_streaming(
            &evaluation_options,
            &buckets,
            options.max_evaluations,
            top_n,
        );
        for (index, result) in results.iter_mut().enumerate() {
            result.rank = index + 1;
        }
        if options.capture_rule_impact {
            attach_rule_impacts(
                fixture,
                &baseline_deck,
                options.side,
                options.mode,
                options.battle_seeds.as_deref(),
                &mut baseline,
                &mut results,
            );
        }
        let seed_audit = seed_audit_fields(
            std::iter::once(&baseline).chain(results.iter().map(|item| &item.evaluation)),
        );
        let elapsed = started.elapsed();
        return SolveDeckResult {
            mode: options.mode,
            visit_order: options.visit_order,
            visit_seed: options.visit_seed,
            side: options.side,
            confidence: if truncated { "truncated" } else { "exact" }.to_string(),
            evaluated_count,
            deck_count: summary.deck_count,
            win_deck_count: summary.win_deck_count,
            first_win_rank: summary.first_win_rank,
            first_win_deck_key: summary.first_win_deck_key,
            best_hp_delta_for_side: summary.best_hp_delta_for_side,
            best_hp_delta_deck_key: summary.best_hp_delta_deck_key,
            value_top_deck_key: summary.value_top_deck_key,
            skipped_duplicate_count,
            candidate_card_count,
            truncated,
            elapsed_ms: elapsed.as_millis(),
            evals_per_sec: evals_per_sec(evaluated_count, elapsed.as_secs_f64()),
            baseline,
            baseline_deck: baseline_ids,
            results,
            seeds_used: seed_audit.0,
            synthetic_decision_seeds_used: seed_audit.1,
            used_synthetic_decisions: seed_audit.2,
        };
    }

    let CandidateDeckCollection {
        decks,
        skipped_duplicate_count,
        truncated,
    } = collect_candidate_decks(
        &buckets,
        options.mode,
        options.visit_order,
        options.visit_seed,
        options.max_evaluations,
    );

    let mut results = evaluate_candidate_decks(&evaluation_options, &decks);
    let summary = summarize_evaluated_results(&results);
    sort_results(&mut results);
    results.truncate(top_n);
    for (index, result) in results.iter_mut().enumerate() {
        result.rank = index + 1;
    }
    if options.capture_rule_impact {
        attach_rule_impacts(
            fixture,
            &baseline_deck,
            options.side,
            options.mode,
            options.battle_seeds.as_deref(),
            &mut baseline,
            &mut results,
        );
    }
    let seed_audit = seed_audit_fields(
        std::iter::once(&baseline).chain(results.iter().map(|item| &item.evaluation)),
    );

    let elapsed = started.elapsed();
    SolveDeckResult {
        mode: options.mode,
        visit_order: options.visit_order,
        visit_seed: options.visit_seed,
        side: options.side,
        confidence: if truncated { "truncated" } else { "exact" }.to_string(),
        evaluated_count: decks.len(),
        deck_count: summary.deck_count,
        win_deck_count: summary.win_deck_count,
        first_win_rank: summary.first_win_rank,
        first_win_deck_key: summary.first_win_deck_key,
        best_hp_delta_for_side: summary.best_hp_delta_for_side,
        best_hp_delta_deck_key: summary.best_hp_delta_deck_key,
        value_top_deck_key: summary.value_top_deck_key,
        skipped_duplicate_count,
        candidate_card_count,
        truncated,
        elapsed_ms: elapsed.as_millis(),
        evals_per_sec: evals_per_sec(decks.len(), elapsed.as_secs_f64()),
        baseline,
        baseline_deck: baseline_ids,
        results,
        seeds_used: seed_audit.0,
        synthetic_decision_seeds_used: seed_audit.1,
        used_synthetic_decisions: seed_audit.2,
    }
}

fn attach_rule_impacts(
    fixture: &BattleFixture,
    baseline_deck: &[CardDefinition],
    side: PlayerSide,
    mode: SolverMode,
    battle_seeds: Option<&[u32]>,
    baseline: &mut SolverEvaluation,
    results: &mut [SolverDeckResult],
) {
    set_rule_impacts(
        baseline,
        capture_rule_impacts_for_fixture(fixture, side, battle_seeds),
    );
    for result in results {
        let deck = complete_deck_ids(baseline_deck, &result.deck);
        let hand_card_ids =
            (mode == SolverMode::Hand).then(|| result.leftover_hand_card_ids.clone());
        let variant = create_fixture_variant(fixture, side, &deck, hand_card_ids);
        set_rule_impacts(
            &mut result.evaluation,
            capture_rule_impacts_for_fixture(&variant, side, battle_seeds),
        );
    }
}

fn set_rule_impacts(
    evaluation: &mut SolverEvaluation,
    impacts: Result<Vec<super::SolverRuleImpactReport>, String>,
) {
    match impacts {
        Ok(impacts) => evaluation.rule_impacts = impacts,
        Err(error) => evaluation
            .warnings
            .push(format!("rule-impact:error:{error}")),
    }
}

struct CandidateDeckCollection {
    decks: Vec<CandidateDeck>,
    skipped_duplicate_count: usize,
    truncated: bool,
}

struct CandidateDeckStreamResult {
    evaluated_count: usize,
    skipped_duplicate_count: usize,
    truncated: bool,
    summary: EvaluatedResultsSummary,
    results: Vec<SolverDeckResult>,
}

#[derive(Default)]
struct StreamingEvaluationSummary {
    deck_count: usize,
    win_deck_count: usize,
    first_win_rank: Option<usize>,
    first_win_deck_key: Option<String>,
    best_hp_delta: Option<SolverDeckResult>,
    value_top: Option<SolverDeckResult>,
}

impl StreamingEvaluationSummary {
    fn observe(&mut self, result: &SolverDeckResult, canonical_rank: usize) {
        self.deck_count += 1;
        if result.evaluation.win_for_side {
            self.win_deck_count += 1;
            if self.first_win_rank.is_none() {
                self.first_win_rank = Some(canonical_rank);
                self.first_win_deck_key = Some(result.deck_key.clone());
            }
        }
        if self.best_hp_delta.as_ref().is_none_or(|current| {
            compare_win_first_hp_delta_results(result, current) < Ordering::Equal
        }) {
            self.best_hp_delta = Some(result.clone());
        }
        if self
            .value_top
            .as_ref()
            .is_none_or(|current| compare_value_results(result, current) < Ordering::Equal)
        {
            self.value_top = Some(result.clone());
        }
    }

    fn finish(self) -> Option<EvaluatedResultsSummary> {
        let best_hp_delta = self.best_hp_delta?;
        let value_top = self.value_top?;
        Some(EvaluatedResultsSummary {
            deck_count: self.deck_count,
            win_deck_count: self.win_deck_count,
            first_win_rank: self.first_win_rank,
            first_win_deck_key: self.first_win_deck_key,
            best_hp_delta_for_side: best_hp_delta.evaluation.hp_delta_for_side,
            best_hp_delta_deck_key: best_hp_delta.deck_key,
            value_top_deck_key: value_top.deck_key,
        })
    }
}

fn evaluate_canonical_candidate_decks_streaming(
    options: &CandidateEvaluationOptions<'_>,
    buckets: &[CandidateBucket],
    max_evaluations: usize,
    top_n: usize,
) -> CandidateDeckStreamResult {
    let total_count = count_deck_sequences(buckets, DECK_SIZE, max_evaluations.saturating_add(1));
    let target_count = total_count.min(max_evaluations);
    let truncated = total_count > max_evaluations;
    let shards = plan_stratified_shards(buckets, target_count);
    let mut summary = StreamingEvaluationSummary::default();
    let mut top_results = Vec::new();
    let mut evaluated_count = 0_usize;

    for shard in shards {
        let decks = collect_stratified_shard(&shard, options.mode);
        let mut shard_results = evaluate_candidate_decks(options, &decks);
        for (index, result) in shard_results.iter().enumerate() {
            summary.observe(result, evaluated_count + index + 1);
        }
        evaluated_count += shard_results.len();
        merge_top_results(&mut top_results, &mut shard_results, top_n);
    }

    CandidateDeckStreamResult {
        evaluated_count,
        skipped_duplicate_count: 0,
        truncated,
        summary: summary
            .finish()
            .expect("solver evaluated at least one canonical deck"),
        results: top_results,
    }
}

fn merge_top_results(
    top_results: &mut Vec<SolverDeckResult>,
    shard_results: &mut Vec<SolverDeckResult>,
    top_n: usize,
) {
    top_results.append(shard_results);
    sort_results(top_results);
    top_results.truncate(top_n);
}

fn collect_candidate_decks(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    visit_order: VisitOrder,
    visit_seed: u64,
    max_evaluations: usize,
) -> CandidateDeckCollection {
    match visit_order {
        VisitOrder::Canonical => collect_canonical_candidate_decks(buckets, mode, max_evaluations),
        VisitOrder::Stratified => {
            collect_stratified_candidate_decks(buckets, mode, visit_seed, max_evaluations)
        }
    }
}

fn collect_canonical_candidate_decks(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    max_evaluations: usize,
) -> CandidateDeckCollection {
    let mut seen_deck_keys = BTreeSet::new();
    let mut skipped_duplicate_count = 0_usize;
    let mut truncated = false;
    let mut decks = Vec::new();

    enumerate_decks(buckets, DECK_SIZE, &mut |deck, leftovers| {
        let key = deck_key(deck);
        if !seen_deck_keys.insert(key.clone()) {
            skipped_duplicate_count += 1;
            return true;
        }
        if decks.len() >= max_evaluations {
            truncated = true;
            return false;
        }
        decks.push(candidate_deck_from_parts(key, deck, leftovers, mode));
        true
    });

    CandidateDeckCollection {
        decks,
        skipped_duplicate_count,
        truncated,
    }
}

#[cfg(not(target_arch = "wasm32"))]
#[cfg(test)]
#[path = "exact_search_tests.rs"]
mod tests;
