use super::{
    evaluate_exact_deck, solve_deck, ScoreProfile, SolveDeckOptions, SolverMode, VisitOrder,
};
use super::{explain_fixture_counterfactuals, CounterfactualElement};
use crate::{
    engine_contract_fixture, original_card_definition_by_id, run_replay_fixture,
    run_replay_fixture_with_events, run_replay_fixture_with_parity_events, BattleFixture,
    PlayerSide, SolverStartingPerturbation,
};

/// Use a code-owned contract fixture without depending on replay admission state.
fn generated_value_fixture() -> BattleFixture {
    engine_contract_fixture().expect("engine contract fixture builds")
}

#[test]
fn evaluate_exact_deck_matches_solve_deck_exact_path() {
    // solve_deck_batch 改走 evaluate_exact_deck（省掉 baseline 那一场）：结果必须与原路径逐字段相同
    let fixture = generated_value_fixture();
    let mut deck: Vec<i64> = fixture
        .players
        .p1
        .cards
        .iter()
        .map(|card| card.id)
        .collect();
    deck.reverse();
    let profile = ScoreProfile::HpDelta;
    for seeds in [None, Some(vec![42_u32, 7, 42])] {
        let via_solve = solve_deck(
            &fixture,
            SolveDeckOptions {
                side: PlayerSide::P1,
                mode: SolverMode::Order,
                visit_order: VisitOrder::Canonical,
                visit_seed: 0,
                top: 1,
                max_evaluations: 1,
                score_profile: profile,
                exact_deck_ids: Some(deck.clone()),
                battle_seeds: seeds.clone(),
            },
        )
        .expect("valid exact-deck solve options")
        .results
        .remove(0)
        .evaluation;
        let direct = evaluate_exact_deck(&fixture, PlayerSide::P1, &deck, profile, seeds.clone())
            .expect("valid exact deck");
        assert_eq!(
            serde_json::to_value(&via_solve).unwrap(),
            serde_json::to_value(&direct).unwrap(),
            "profile {profile:?} seeds {seeds:?}"
        );
    }
    assert!(evaluate_exact_deck(
        &fixture,
        PlayerSide::P1,
        &deck,
        ScoreProfile::HpDelta,
        Some(vec![])
    )
    .is_err());
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
fn prevention_telemetry_accumulates_absorbed_damage_monotonically() {
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

#[test]
fn borrowed_shared_sides_match_owned_variant_with_history_and_seed_overlays() {
    use crate::fixture::{HistoricalCardOverride, HistoricalCardPatch};
    let fixture = generated_value_fixture();
    let mut template = fixture.clone();
    // The template's embedded sides are deliberately different. The shared
    // sides must govern every initialization/opening predicate, in both orders.
    template.players.p1.base_max_hp += 500;
    template.players.p2.initial_defense += 500;
    for side in [PlayerSide::P1, PlayerSide::P2] {
        template
            .historical_card_overrides
            .push(HistoricalCardOverride {
                side,
                slot_index: 0,
                patch: HistoricalCardPatch {
                    attack: Some(1),
                    ..Default::default()
                },
                reason: "borrowed view contract".into(),
                evidence: "code-owned fixture".into(),
            });
    }
    for first in [PlayerSide::P1, PlayerSide::P2] {
        for side in [PlayerSide::P1, PlayerSide::P2] {
            let player = match side {
                PlayerSide::P1 => &fixture.players.p1,
                PlayerSide::P2 => &fixture.players.p2,
            };
            let mut cards = player.cards.clone();
            cards.reverse();
            let deck: Vec<_> = cards.iter().map(|card| card.id).collect();
            let mut owned = template.clone();
            owned.players = fixture.players.clone();
            owned.first_player_side = first;
            let baseline = super::variant::fixture_deck_candidates(&owned, side);
            let deck_cards = deck
                .iter()
                .map(|id| baseline.iter().find(|card| card.id == *id).unwrap().clone())
                .collect::<Vec<_>>();
            let owned = super::variant::create_fixture_variant(&owned, side, &deck_cards, None);
            let profile = ScoreProfile::HpDelta;
            for seeds in [None, Some(vec![7, 42])] {
                let expected = super::variant::evaluate_fixture_across_battle_seeds(
                    &owned,
                    side,
                    profile,
                    seeds.as_deref(),
                );
                let actual = super::evaluate_exact_deck_with_players(
                    &template,
                    &fixture.players.p1,
                    &fixture.players.p2,
                    first,
                    side,
                    &deck,
                    profile,
                    seeds.clone(),
                )
                .unwrap();
                assert_eq!(
                    serde_json::to_value(actual).unwrap(),
                    serde_json::to_value(expected).unwrap(),
                    "first {first:?} side {side:?} profile {profile:?} seeds {seeds:?}"
                );
            }
        }
    }
}

#[test]
fn uncaptured_decisions_preserve_full_evaluation_and_synthetic_seed_metadata() {
    use crate::fixture::FixtureSource;
    let mut fixture = generated_value_fixture();
    let random = original_card_definition_by_id(4_000_006).unwrap();
    fixture.players.p1.cards = vec![random.clone(); 8];
    fixture.players.p2.cards = vec![random; 8];
    fixture.players.p1.active_slot_count = 1;
    fixture.players.p2.active_slot_count = 1;
    fixture.players.p1.base_max_hp = 2_000;
    fixture.players.p2.base_max_hp = 2_000;
    fixture.max_actor_turns = Some(6);
    fixture.decision_tape.clear();
    fixture.random_fallback_tape.clear();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        synthetic_decision_fallback_seed: Some(31),
        ..Default::default()
    });
    let deck = [4_000_006; 8];
    for robust in [false, true] {
        fixture.source.as_mut().unwrap().robust_decisions = robust;
        for side in [PlayerSide::P1, PlayerSide::P2] {
            let profile = ScoreProfile::HpDelta;
            for seeds in [None, Some(vec![42]), Some(vec![7, 42, 7])] {
                let mut expected =
                    evaluate_exact_deck(&fixture, side, &deck, profile, seeds.clone()).unwrap();
                if let (false, Some(selected_seeds)) = (robust, seeds.as_ref()) {
                    assert!(!expected.failed);
                    assert_eq!(
                        expected
                            .seed_aggregate
                            .as_ref()
                            .unwrap()
                            .synthetic_decision_seeds_used,
                        if selected_seeds.len() == 1 {
                            vec![17, 42]
                        } else {
                            vec![7, 17, 42]
                        }
                    );
                }
                expected.decision_events.clear();
                let direct = super::evaluate_exact_deck_with_capture(
                    &fixture,
                    side,
                    &deck,
                    profile,
                    seeds.clone(),
                    false,
                )
                .unwrap();
                let shared = super::evaluate_exact_deck_with_players_and_capture(
                    &fixture,
                    &fixture.players.p1,
                    &fixture.players.p2,
                    fixture.first_player_side,
                    side,
                    &deck,
                    profile,
                    seeds.clone(),
                    false,
                )
                .unwrap();
                let expected = serde_json::to_value(expected).unwrap();
                assert_eq!(serde_json::to_value(direct).unwrap(), expected);
                assert_eq!(serde_json::to_value(shared).unwrap(), expected);
            }
        }
    }
    fixture.source = None;
    // Missing and invalid replay choices retain their failure diagnostics.
    for tape in [vec![], vec![999]] {
        fixture.decision_tape = tape;
        let mut expected =
            evaluate_exact_deck(&fixture, PlayerSide::P1, &deck, ScoreProfile::HpDelta, None)
                .unwrap();
        let actual = super::evaluate_exact_deck_with_capture(
            &fixture,
            PlayerSide::P1,
            &deck,
            ScoreProfile::HpDelta,
            None,
            false,
        )
        .unwrap();
        expected.decision_events.clear();
        assert_eq!(
            serde_json::to_value(actual).unwrap(),
            serde_json::to_value(expected).unwrap()
        );
    }
}
