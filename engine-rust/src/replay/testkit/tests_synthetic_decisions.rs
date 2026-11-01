use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::{FixtureExpected, FixtureSource};
use crate::model::PlayerSide;
use std::path::{Path, PathBuf};

fn shared_decision_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust has project root parent")
        .join("battle-evaluator/fixtures/contracts/decisions/exorcism-pill-seed17.json")
}

#[test]
fn synthetic_battle_choice_hash_matches_ts_contract() {
    let indexes = [1, 2, 17, 99].map(|seed| {
        decisions::seeded_decision_choice_index(
            seed,
            PlayerSide::P1,
            3,
            2_000_004,
            ReplayDecisionKind::NegativeStatus,
            1,
            1,
            0,
            8,
        )
    });
    assert_eq!(indexes, [2, 4, 2, 3]);
    assert_eq!(
        decisions::seeded_decision_choice_index(
            17,
            PlayerSide::P1,
            3,
            2_000_004,
            ReplayDecisionKind::NegativeStatus,
            2,
            2,
            1,
            7,
        ),
        5
    );
}

#[test]
fn synthetic_exorcism_pill_logs_exact_live_choice_without_consuming_replay_tape() {
    let mut pill = test_card(2_000_004, 2_000_004, "驱邪丹");
    pill.defense = Some(8);
    pill.other_params = vec![1, 3].into();
    let mut fixture = minimal_fixture(
        filler_cards(pill.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![0, 9_999];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.add_actor_negative_status(PlayerSide::P1, 100, 4);
    state.add_actor_negative_status(PlayerSide::P1, 101, 4);

    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);

    assert_eq!(state.decision_tape, vec![0, 9_999]);
    assert_eq!(state.p1.status.internal_injury, 4);
    assert_eq!(state.p1.status.weakness, 1);
    assert_eq!(
        state.decision_events,
        vec![ReplayDecisionEvent {
            provider: ReplayDecisionProvider::SeededSynthetic,
            seed: Some(17),
            side: PlayerSide::P1,
            actor_turn: 0,
            card_id: 2_000_004,
            decision_kind: ReplayDecisionKind::NegativeStatus,
            card_execution_occurrence: 1,
            decision_occurrence: 1,
            ordinal: 0,
            domain: ReplayDecisionDomain::Discrete {
                legal_options: vec![100, 101],
            },
            selected_option: Some(101),
        }]
    );
}

#[test]
fn replay_exorcism_pill_consumes_tape_through_typed_resolver() {
    let mut pill = test_card(2_000_004, 2_000_004, "驱邪丹");
    pill.other_params = vec![1, 3].into();
    let mut fixture = minimal_fixture(
        filler_cards(pill.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.decision_tape = vec![101, 9_999];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.add_actor_negative_status(PlayerSide::P1, 100, 4);
    state.add_actor_negative_status(PlayerSide::P1, 101, 4);

    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);

    assert_eq!(state.decision_tape, vec![9_999]);
    assert_eq!(state.p1.status.internal_injury, 4);
    assert_eq!(state.p1.status.weakness, 1);
    assert_eq!(state.decision_events.len(), 1);
    let event = &state.decision_events[0];
    assert_eq!(event.provider, ReplayDecisionProvider::ReplayTape);
    assert_eq!(event.seed, None);
    assert_eq!(event.legal_options(), Some([100, 101].as_slice()));
    assert_eq!(event.selected_option, Some(101));
}

#[test]
fn typed_fallback_selects_negative_status_only_after_tape_exhaustion() {
    let mut pill = test_card(2_000_004, 2_000_004, "驱邪丹");
    pill.other_params = vec![1, 3].into();
    let mut fixture = minimal_fixture(
        filler_cards(pill.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(17),
        ..FixtureSource::default()
    });
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.add_actor_negative_status(PlayerSide::P1, 100, 4);
    state.add_actor_negative_status(PlayerSide::P1, 101, 4);

    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);

    assert_eq!(state.decision_events.len(), 1);
    assert_eq!(
        state.decision_events[0].provider,
        ReplayDecisionProvider::SeededSynthetic
    );
    assert_eq!(state.decision_events[0].seed, Some(17));
    assert_eq!(state.decision_events[0].selected_option, Some(101));
}

#[test]
fn replay_exorcism_pill_consumes_and_rejects_a_non_status_value() {
    let mut pill = test_card(2_000_004, 2_000_004, "驱邪丹");
    pill.other_params = vec![1, 3].into();
    let mut fixture = minimal_fixture(
        filler_cards(pill.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.decision_tape = vec![9_999, 101];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.add_actor_negative_status(PlayerSide::P1, 100, 4);
    state.add_actor_negative_status(PlayerSide::P1, 101, 4);

    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);

    assert_eq!(state.decision_tape, vec![101]);
    assert_eq!(state.p1.status.internal_injury, 4);
    assert_eq!(state.p1.status.weakness, 4);
    assert_eq!(state.decision_events[0].selected_option, Some(9_999));
    assert!(matches!(
        state.evaluation_error.as_ref(),
        Some(BattleError::InvalidDecision {
            card_id: 2_000_004,
            kind: "negative-status",
            selected: 9_999,
            turn: 0,
        })
    ));
    assert_eq!(
        state
            .evaluation_error
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("invalid decision: card:2000004:negative-status=9999; turn=0"),
    );
}

#[test]
fn repeated_same_card_effects_receive_distinct_execution_and_decision_occurrences() {
    let mut pill = test_card(2_000_004, 2_000_004, "驱邪丹");
    pill.other_params = vec![1, 1].into();
    let mut fixture = minimal_fixture(
        filler_cards(pill.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.add_actor_negative_status(PlayerSide::P1, 100, 4);
    state.add_actor_negative_status(PlayerSide::P1, 101, 4);

    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);
    state.test_apply_card_effect(PlayerSide::P1, &pill, 0);

    assert_eq!(state.decision_events.len(), 2);
    assert_eq!(state.decision_events[0].card_execution_occurrence, 1);
    assert_eq!(state.decision_events[0].decision_occurrence, 1);
    assert_eq!(state.decision_events[1].card_execution_occurrence, 2);
    assert_eq!(state.decision_events[1].decision_occurrence, 2);
    assert_eq!(state.current_card_execution, None);
    assert_ne!(
        (
            state.decision_events[0].card_execution_occurrence,
            state.decision_events[0].decision_occurrence,
        ),
        (
            state.decision_events[1].card_execution_occurrence,
            state.decision_events[1].decision_occurrence,
        )
    );
}

#[test]
fn shared_seed17_fixture_locks_decision_and_terminal_state() {
    let path = shared_decision_fixture_path();
    let contract_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("shared decision fixture reads"),
    )
    .expect("shared decision fixture JSON parses");
    let fixture = load_fixture_file(path).expect("shared decision fixture loads");
    let mut state = ReplayState::test_from_fixture(&fixture);

    let summary = state.test_run();

    assert!(summary.matches_fixture(&fixture));
    assert_eq!(
        serde_json::to_value(&state.decision_events).expect("decision events serialize"),
        contract_json["decisionContract"]["events"]
    );
    let final_hp = fixture
        .expected
        .final_hp
        .as_ref()
        .expect("fixture has finalHp");
    assert_eq!(state.test_final_hp(), (final_hp.p1, final_hp.p2));
    assert_eq!(
        state.p1.status.internal_injury,
        contract_json["decisionContract"]["finalNegativeStatusLayers"]["100"]
            .as_i64()
            .expect("fixture has status 100")
    );
    assert_eq!(
        state.p1.status.weakness,
        contract_json["decisionContract"]["finalNegativeStatusLayers"]["101"]
            .as_i64()
            .expect("fixture has status 101")
    );
    assert_eq!(
        serde_json::to_value(&state.decision_tape).expect("remaining tape serializes"),
        contract_json["decisionContract"]["remainingDecisionTape"]
    );

    assert_eq!(
        state.decision_events[0].provider,
        ReplayDecisionProvider::SeededSynthetic
    );
    assert_eq!(
        state.decision_events[0].decision_kind,
        ReplayDecisionKind::NegativeStatus
    );
    assert_eq!(state.decision_events[0].selected_option, Some(101));
}

#[test]
fn battle_start_opening_gets_its_own_execution_before_seeded_decision() {
    let mut fixture =
        load_fixture_file(shared_decision_fixture_path()).expect("shared decision fixture loads");
    fixture.players.p2.cards[7] = test_card(55, 55, "五行开局牌");
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_run();

    assert_eq!(state.decision_events.len(), 1);
    let event = &state.decision_events[0];
    assert_eq!(event.card_execution_occurrence, 4);
    assert_eq!(event.decision_occurrence, 1);
    assert_eq!(event.legal_options(), Some([100, 101].as_slice()));
    assert_eq!(event.selected_option, Some(100));
    assert_eq!(state.decision_tape, vec![9_999]);
}
