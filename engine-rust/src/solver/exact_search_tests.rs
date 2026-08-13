use super::stratified::{
    collect_stratified_candidate_decks_parallel, collect_stratified_candidate_decks_single_thread,
};
use super::*;

const TEST_POPULATION_LIMIT: usize = STRATIFIED_SHARD_TARGET * 2 + 1;
const TEST_MAX_EVALUATIONS: usize = 512;

#[test]
fn public_solver_normalizes_battle_seeds_and_rejects_empty_work() {
    let mut options = SolveDeckOptions {
        max_evaluations: 0,
        ..SolveDeckOptions::default()
    };
    assert_eq!(
        normalize_solve_deck_options(options.clone())
            .expect_err("zero max evaluations must fail")
            .to_string(),
        "invalid fixture: maxEvaluations must be greater than zero"
    );

    options.max_evaluations = 1;
    options.battle_seeds = Some(Vec::new());
    assert_eq!(
        normalize_solve_deck_options(options.clone())
            .expect_err("empty battle seeds must fail")
            .to_string(),
        "invalid fixture: battleSeeds must not be empty"
    );

    options.battle_seeds = Some(vec![3, 1, 3, 2]);
    assert_eq!(
        normalize_solve_deck_options(options)
            .expect("valid options normalize")
            .battle_seeds,
        Some(vec![1, 2, 3])
    );
}

#[test]
fn stratified_parallel_order_matches_single_thread_reference_for_two_seeds() {
    let buckets = test_buckets(&[
        (10_001, 1),
        (10_002, 1),
        (10_003, 1),
        (10_004, 1),
        (10_005, 1),
        (10_006, 1),
        (10_007, 1),
        (10_008, 1),
    ]);
    assert_parallel_matches_single_thread_reference(&buckets, SolverMode::Order);
}

#[test]
fn stratified_parallel_hand_matches_single_thread_reference_for_two_seeds() {
    let buckets = test_buckets(&[
        (20_001, 1),
        (20_002, 1),
        (20_003, 1),
        (20_004, 1),
        (20_005, 1),
        (20_006, 1),
        (20_007, 1),
        (20_008, 1),
        (20_009, 1),
    ]);
    assert_parallel_matches_single_thread_reference(&buckets, SolverMode::Hand);
}

fn assert_parallel_matches_single_thread_reference(buckets: &[CandidateBucket], mode: SolverMode) {
    for seed in [0, 0x9e37_79b9_7f4a_7c15] {
        let expected = collect_stratified_candidate_decks_single_thread(
            buckets,
            mode,
            seed,
            TEST_MAX_EVALUATIONS,
            TEST_POPULATION_LIMIT,
        );
        let actual = collect_stratified_candidate_decks_parallel(
            buckets,
            mode,
            seed,
            TEST_MAX_EVALUATIONS,
            TEST_POPULATION_LIMIT,
        );
        assert_eq!(
            actual.skipped_duplicate_count,
            expected.skipped_duplicate_count
        );
        assert_eq!(actual.truncated, expected.truncated);
        assert_eq!(candidate_sequence(&actual), candidate_sequence(&expected));
    }
}

fn candidate_sequence(collection: &CandidateDeckCollection) -> Vec<(String, Vec<i64>)> {
    collection
        .decks
        .iter()
        .map(|deck| (deck.key.clone(), deck.leftover_hand_card_ids.clone()))
        .collect()
}

fn test_buckets(cards: &[(i64, usize)]) -> Vec<CandidateBucket> {
    cards
        .iter()
        .map(|(id, count)| CandidateBucket {
            card: CardDefinition {
                id: *id,
                base_id: None,
                name: format!("test-card-{id}"),
                card_type: None,
                attack: Some((*id % 13) + 1),
                random_attack: None,
                random_defense: None,
                attack_count: None,
                defense: None,
                damage: None,
                anima: None,
                hp_cost: None,
                action_again: None,
                physique: None,
                sword_intent: None,
                hexagram: None,
                rarity: None,
                career_name: None,
                other_params: Vec::new(),
            },
            count: *count,
        })
        .collect()
}
