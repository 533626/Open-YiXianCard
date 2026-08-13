use super::{explain_fixture_counterfactuals, CounterfactualElement};
use super::{solve_deck, ScoreProfile, SolveDeckOptions, SolverEvaluation, SolverMode, VisitOrder};
use crate::{
    engine_contract_fixture, original_card_definition_by_id, run_replay_fixture,
    run_replay_fixture_with_events, run_replay_fixture_with_parity_events, BattleFixture,
    PlayerSide, SolverStartingPerturbation,
};

/// Use a code-owned contract fixture without depending on replay admission state.
fn generated_value_fixture() -> BattleFixture {
    engine_contract_fixture().expect("engine contract fixture builds")
}

fn evaluate_value_deck_for_fixture(fixture: &BattleFixture) -> SolverEvaluation {
    let deck = fixture
        .players
        .p1
        .cards
        .iter()
        .map(|card| card.id)
        .collect();
    solve_deck(
        fixture,
        SolveDeckOptions {
            side: PlayerSide::P1,
            mode: SolverMode::Order,
            visit_order: VisitOrder::Canonical,
            visit_seed: 0,
            top: 1,
            max_evaluations: 1,
            score_profile: ScoreProfile::ValueV0,
            exact_deck_ids: Some(deck),
            battle_seeds: None,
            capture_rule_impact: false,
        },
    )
    .expect("valid exact-deck solve options")
    .results
    .into_iter()
    .next()
    .expect("one exact result")
    .evaluation
}

#[test]
fn value_v0_is_deterministic_on_machine_selected_current_build_fixture() {
    let fixture = generated_value_fixture();
    let first = evaluate_value_deck_for_fixture(&fixture);
    let second = evaluate_value_deck_for_fixture(&fixture);

    assert_eq!(first.score_profile, ScoreProfile::ValueV0);
    assert_eq!(first.score, second.score);
    assert_eq!(first.win_for_side, second.win_for_side);
    assert_eq!(first.hp_delta_for_side, second.hp_delta_for_side);
    assert!(first.rule_impacts.is_empty());

    let metrics = first.value_metrics.expect("value metrics");
    assert_eq!(
        first.score,
        rounded(metrics.terminal_value_for_side + metrics.area_score_for_side)
    );
    assert!(metrics.area_sample_count > 0.0);
}

#[test]
fn rule_impact_replays_only_returned_candidates_and_reconstructs_checkpoint_delta() {
    let fixture = generated_value_fixture();
    let deck = fixture
        .players
        .p1
        .cards
        .iter()
        .map(|card| card.id)
        .collect();
    let result = solve_deck(
        &fixture,
        SolveDeckOptions {
            side: PlayerSide::P1,
            mode: SolverMode::Order,
            visit_order: VisitOrder::Canonical,
            visit_seed: 0,
            top: 1,
            max_evaluations: 1,
            score_profile: ScoreProfile::ValueV0,
            exact_deck_ids: Some(deck),
            battle_seeds: None,
            capture_rule_impact: true,
        },
    )
    .expect("valid exact-deck solve options");

    let impact = result
        .results
        .first()
        .expect("one exact result")
        .evaluation
        .rule_impacts
        .first()
        .expect("canonical rule impact");
    assert_eq!(impact.schema_version, "canonical-rule-impact-v1");
    assert_eq!(impact.source, "rust-canonical-replay-checkpoints");
    assert_eq!(impact.value_profile, "value-v0-terminal");
    assert!(impact
        .checkpoints
        .iter()
        .any(|checkpoint| checkpoint.kind == crate::ReplayEventKind::CardCompleted));
    assert!(!impact.cards.is_empty());
    assert!(impact.feature_sample_count > 0);
    assert!(impact.features.contains_key("terminal.hp"));
    assert_close(impact.audit_delta_for_side, 0.0);
    assert_close(
        impact.terminal_delta_for_side,
        impact.terminal_value_for_side - impact.start_value_for_side,
    );
    let published_total = impact
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.contribution.total)
        .sum::<f64>();
    assert_close(impact.attributed_delta_for_side, published_total);

    // The audit must be falsifiable. It compares the terminal delta against the
    // contributions this report published, so losing one published checkpoint has
    // to show up. A private running total of the same consecutive deltas would
    // telescope to terminal - start regardless and keep reporting zero.
    let dropped = impact
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.contribution.total)
        .find(|total| *total != 0.0)
        .expect("at least one checkpoint moves value, otherwise the audit proves nothing");
    assert!(
        rounded(impact.terminal_delta_for_side - (published_total - dropped)).abs() > 1e-9,
        "dropping a published checkpoint must break the audit",
    );
}

#[test]
fn solver_starting_perturbations_apply_before_the_first_canonical_checkpoint() {
    let baseline = generated_value_fixture();
    let baseline_run = run_replay_fixture_with_events(&baseline).expect("baseline replay");
    let mut perturbed = baseline;
    perturbed
        .source
        .get_or_insert_default()
        .solver_starting_perturbations
        .push(SolverStartingPerturbation {
            side: PlayerSide::P1,
            field: "guard".to_string(),
            amount: 3,
        });
    let perturbed_run =
        run_replay_fixture_with_events(&perturbed).expect("perturbed canonical replay");

    assert_eq!(
        perturbed_run.events[0].p1.guard,
        baseline_run.events[0].p1.guard + 3
    );
}

#[test]
fn solver_starting_perturbations_cannot_run_under_exact_comparison_observation() {
    let mut fixture = generated_value_fixture();
    fixture
        .source
        .get_or_insert_default()
        .solver_starting_perturbations
        .push(SolverStartingPerturbation {
            side: PlayerSide::P1,
            field: "guard".to_string(),
            amount: 3,
        });

    // Parity feeds the golden winner / actorTurn / hpDelta comparison and the plain
    // summary feeds replay admission. Neither may silently replay a perturbed battle.
    for message in [
        run_replay_fixture_with_parity_events(&fixture)
            .expect_err("parity observation rejects perturbations")
            .to_string(),
        run_replay_fixture(&fixture)
            .expect_err("summary replay rejects perturbations")
            .to_string(),
    ] {
        assert!(
            message.contains("analysis-only"),
            "unexpected rejection message: {message}"
        );
    }
}

#[test]
fn prevention_telemetry_records_absorbed_damage_without_moving_the_score() {
    let fixture = generated_value_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("replay");
    assert_eq!(run.prevention.len(), run.events.len());

    let terminal = run.prevention.last().copied().expect("terminal prevention");
    let absorbed = terminal.p1.hp_loss_prevented_by_guard
        + terminal.p1.hp_loss_prevented_by_defense
        + terminal.p2.hp_loss_prevented_by_guard
        + terminal.p2.hp_loss_prevented_by_defense;
    assert!(
        absorbed > 0,
        "contract fixture must exercise at least one absorption, otherwise this proves nothing"
    );
    // 累计量只增不减：任何一步减少都说明记的是瞬时值而不是累计值。
    let mut previous = run.prevention[0];
    for pair in &run.prevention {
        assert!(pair.p1.hp_loss_prevented_by_guard >= previous.p1.hp_loss_prevented_by_guard);
        assert!(pair.p1.hp_loss_prevented_by_defense >= previous.p1.hp_loss_prevented_by_defense);
        assert!(pair.p2.hp_loss_prevented_by_guard >= previous.p2.hp_loss_prevented_by_guard);
        assert!(pair.p2.hp_loss_prevented_by_defense >= previous.p2.hp_loss_prevented_by_defense);
        previous = *pair;
    }

    // 分数不能因为这项遥测而移动：挡掉的伤害已经体现在双方实际生命里，
    // 折进 total 就是双算，也会让已认证的 120 例 order 基线失效。
    let evaluation = evaluate_value_deck_for_fixture(&fixture);
    let metrics = evaluation.value_metrics.expect("value metrics");
    assert_eq!(
        evaluation.score,
        rounded(metrics.terminal_value_for_side + metrics.area_score_for_side)
    );
}

#[test]
fn prevention_contributions_sum_to_the_terminal_absorbed_total() {
    let fixture = generated_value_fixture();
    let deck = fixture
        .players
        .p1
        .cards
        .iter()
        .map(|card| card.id)
        .collect();
    let result = solve_deck(
        &fixture,
        SolveDeckOptions {
            side: PlayerSide::P1,
            mode: SolverMode::Order,
            visit_order: VisitOrder::Canonical,
            visit_seed: 0,
            top: 1,
            max_evaluations: 1,
            score_profile: ScoreProfile::ValueV0,
            exact_deck_ids: Some(deck),
            battle_seeds: None,
            capture_rule_impact: true,
        },
    )
    .expect("valid exact-deck solve options");
    let impact = result
        .results
        .first()
        .expect("one exact result")
        .evaluation
        .rule_impacts
        .first()
        .expect("canonical rule impact");
    let run = run_replay_fixture_with_events(&fixture).expect("replay");
    let terminal = run.prevention.last().copied().expect("terminal prevention");

    let published_guard = impact
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.contribution.hp_loss_prevented_by_guard)
        .sum::<f64>();
    let published_defense = impact
        .checkpoints
        .iter()
        .map(|checkpoint| checkpoint.contribution.hp_loss_prevented_by_defense)
        .sum::<f64>();

    // 逐段增量必须重建出终局累计量，否则某一步的吸收被算到了别的结算点上。
    assert_close(
        published_guard,
        terminal.p1.hp_loss_prevented_by_guard as f64,
    );
    assert_close(
        published_defense,
        terminal.p1.hp_loss_prevented_by_defense as f64,
    );
    // 吸收量不进 total。
    for checkpoint in &impact.checkpoints {
        let channels = checkpoint.contribution.hp
            + checkpoint.contribution.defense
            + checkpoint.contribution.guard
            + checkpoint.contribution.resource
            + checkpoint.contribution.debuff
            + checkpoint.contribution.tempo;
        assert_close(rounded(channels), checkpoint.contribution.total);
    }
}

#[test]
fn counterfactuals_separate_clean_prefix_from_terminal_and_expose_zero_margin_defense() {
    let mut fixture = generated_value_fixture();
    let basic = original_card_definition_by_id(0).expect("basic attack is registered");
    fixture.players.p1.cards = vec![basic.clone(); 8];
    fixture.players.p2.cards = vec![basic; 8];
    fixture.players.p1.initial_defense = 2;
    fixture.players.p1.initial_guard = 1;
    fixture.max_actor_turns = Some(2);

    let report = explain_fixture_counterfactuals(
        &fixture,
        PlayerSide::P1,
        &[
            CounterfactualElement {
                id: "opening-defense".to_string(),
                label: "开局防御 2".to_string(),
                side: PlayerSide::P1,
                field: "defense".to_string(),
                amount: 2,
            },
            CounterfactualElement {
                id: "opening-guard".to_string(),
                label: "开局护体 1 层".to_string(),
                side: PlayerSide::P1,
                field: "guard".to_string(),
                amount: 1,
            },
        ],
    )
    .expect("counterfactual report");

    assert_eq!(report.schema_version, "canonical-counterfactual-v1");
    assert_eq!(report.elements.len(), 2);
    let defense = &report.elements[0];
    assert_eq!(defense.terminal_hp_delta_change_for_side, 0);
    assert_eq!(defense.pre_divergence_hp_delta_change_for_side, 0);
    assert_eq!(defense.first_divergence_actor_turn, None);
    assert!(!defense.winner_changed);

    let guard = &report.elements[1];
    assert_eq!(guard.terminal_hp_delta_change_for_side, -2);
    assert_eq!(guard.pre_divergence_hp_delta_change_for_side, -2);
    assert_eq!(guard.first_divergence_actor_turn, None);
}

#[test]
fn counterfactual_rejects_removing_more_than_the_opening_state_contains() {
    let fixture = generated_value_fixture();
    let error = explain_fixture_counterfactuals(
        &fixture,
        PlayerSide::P1,
        &[CounterfactualElement {
            id: "impossible-guard".to_string(),
            label: "不存在的护体".to_string(),
            side: PlayerSide::P1,
            field: "guard".to_string(),
            amount: 999,
        }],
    )
    .expect_err("over-removal must fail closed");

    assert!(
        error.contains("battleStart only has"),
        "unexpected error: {error}"
    );
}

fn rounded(value: f64) -> f64 {
    (value * 10.0).round() / 10.0
}

fn assert_close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() < 1e-9,
        "expected {expected}, got {actual}"
    );
}
