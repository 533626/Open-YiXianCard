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
            final_life: None,
            life_damage: None,
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
            final_life: None,
            life_damage: None,
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
            final_life: None,
            life_damage: None,
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
            final_life: None,
            life_damage: None,
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
            final_life: None,
            life_damage: None,
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

// Merged from tests_robust_decisions.rs (2026-11-14).

fn fixture_with(p1: CardDefinition, p2: CardDefinition) -> BattleFixture {
    let mut fixture = minimal_fixture(
        filler_cards(p1),
        filler_cards(p2),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
            final_life: None,
            life_damage: None,
        },
    );
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(17),
        robust_decisions: true,
        ..FixtureSource::default()
    });
    fixture
}

fn random_attack_card() -> CardDefinition {
    let mut card = test_card(1_000_001, 1_000_001, "random-range-probe");
    card.attack = Some(2);
    card.random_attack = Some(5);
    card.attack_count = Some(2);
    card
}

fn dragonfly_skims_water() -> CardDefinition {
    let mut card = test_card(4_000_035, 4_000_035, "蜻蜓点水");
    card.attack = Some(5);
    card.attack_count = Some(1);
    card.other_params = vec![10].into();
    card
}

#[test]
fn robust_random_range_takes_floor_mean_for_both_sides() {
    let card = random_attack_card();
    let fixture = fixture_with(card.clone(), card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    state.test_apply_card_effect(PlayerSide::P2, &card, 0);
    let picks: Vec<_> = state
        .decision_events
        .iter()
        .map(|e| (e.side, e.provider, e.selected_option))
        .collect();
    assert_eq!(
        picks,
        vec![
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(3)),
        ]
    );
}

#[test]
fn robust_percent_roll_is_pessimistic_for_p1() {
    let card = dragonfly_skims_water();
    let fixture = fixture_with(card.clone(), card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    state.test_apply_card_effect(PlayerSide::P2, &card, 0);
    let rolls: Vec<_> = state
        .decision_events
        .iter()
        .filter(|e| e.decision_kind == ReplayDecisionKind::PercentRoll)
        .map(|e| (e.side, e.provider, e.selected_option))
        .collect();
    assert_eq!(
        rolls,
        vec![
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(99)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(0)),
        ]
    );
}

#[test]
fn robust_mode_off_keeps_seeded_synthetic_choices() {
    let card = random_attack_card();
    let mut fixture = fixture_with(card.clone(), basic_attack_test_card());
    fixture.source.as_mut().expect("source").robust_decisions = false;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert!(state
        .decision_events
        .iter()
        .all(|e| e.provider == ReplayDecisionProvider::SeededSynthetic));
}

#[test]
fn xianmo_13002_fails_closed_without_tape_or_synthetic_seed() {
    let mut fixture = fixture_with(basic_attack_test_card(), basic_attack_test_card());
    fixture.source = None;
    fixture.players.p1.xian_mo_strategies = vec![13002];
    fixture
        .players
        .p1
        .xian_mo_temp_datas
        .insert("13002".into(), 0);
    let err = crate::replay::run_replay_fixture(&fixture).unwrap_err();
    assert!(
        err.to_string().contains("card parameter"),
        "real replay without tape or synthetic seed must fail closed: {err}"
    );
}

#[test]
fn xianmo_13002_synthetic_seed_clears_debuffs_and_heals_remainder() {
    let mut fixture = fixture_with(basic_attack_test_card(), basic_attack_test_card());
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(42),
        ..FixtureSource::default()
    });
    fixture.players.p1.base_max_hp = 50;
    fixture.players.p1.battle_start_hp = Some(30);
    fixture.players.p1.xian_mo_strategies = vec![13002];
    fixture
        .players
        .p1
        .xian_mo_temp_datas
        .insert("13002".into(), 0);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 30;
    state.add_actor_negative_status(PlayerSide::P1, 100, 1);
    state.add_actor_negative_status(PlayerSide::P1, 101, 1);
    let card = basic_attack_test_card();
    state.apply_verified_xian_mo_on_play_card(PlayerSide::P1, &card, 0, false);
    assert!(
        state.evaluation_error.is_none(),
        "synthetic 13002 must not fail: {:?}",
        state.evaluation_error
    );
    assert_eq!(state.p1.status.internal_injury, 0);
    assert_eq!(state.p1.status.weakness, 0);
    // 2 debuffs cleared, 2 remaining slots -> +4 HP (30 -> 34).
    assert_eq!(state.p1.core.hp, 34);
    assert!(
        crate::replay::run_replay_fixture(&fixture).is_ok(),
        "full synthetic replay with 13002 must succeed"
    );
}

#[test]
fn card_7000111_synthetic_seed_uses_hand_formation_and_seal_while_real_replay_fails_closed() {
    let flag = crate::replay::original_config::original_card_definition(7_000_111)
        .expect("7000111 definition");
    let mut real_fixture = fixture_with(flag.clone(), basic_attack_test_card());
    real_fixture.source = None;
    let err = crate::replay::run_replay_fixture(&real_fixture).unwrap_err();
    assert!(
        err.to_string().contains("card parameter"),
        "real replay 7000111 without tape must fail closed: {err}"
    );

    let mut synth = fixture_with(flag, basic_attack_test_card());
    synth.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(77),
        ..FixtureSource::default()
    });
    synth.players.p1.hand_cards = vec![7_010_016, 7_000_001];
    assert!(
        crate::replay::run_replay_fixture(&synth).is_ok(),
        "synthetic 7000111 must resolve hand formation/seal without error"
    );
}
