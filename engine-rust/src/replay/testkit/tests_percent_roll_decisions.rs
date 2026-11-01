use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::{FixtureExpected, FixtureSource};
use crate::model::PlayerSide;
use std::path::{Path, PathBuf};

const PERCENT_CARD_ID: i64 = 4_000_057;

fn shared_percent_roll_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust has project root parent")
        .join("battle-evaluator/fixtures/contracts/decisions/percent-roll-seed17.json")
}

fn percent_fixture() -> BattleFixture {
    minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    )
}

fn draw_bow_shoot_tiger() -> CardDefinition {
    let mut card = test_card(4_000_097, 4_000_097, "弯弓射虎");
    card.attack = Some(8);
    card.other_params = vec![16, 4, 10].into();
    card
}

fn dragonfly_skims_water() -> CardDefinition {
    let mut card = test_card(4_000_035, 4_000_035, "蜻蜓点水");
    card.attack = Some(5);
    card.attack_count = Some(1);
    card.other_params = vec![10].into();
    card
}

fn g326_round_13_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust has project root parent")
        .join("battle-evaluator/fixtures/contracts/g326-round-13.json")
}

#[test]
fn percent_roll_hash_matches_ts_contract() {
    let indexes = [1, 2, 17, 99].map(|seed| {
        decisions::seeded_decision_choice_index(
            seed,
            PlayerSide::P1,
            3,
            4_000_046,
            ReplayDecisionKind::PercentRoll,
            1,
            1,
            0,
            100,
        )
    });

    assert_eq!(indexes, [73, 4, 39, 45]);
    assert_eq!(
        decisions::seeded_decision_choice_index(
            17,
            PlayerSide::P1,
            1,
            PERCENT_CARD_ID,
            ReplayDecisionKind::PercentRoll,
            1,
            1,
            0,
            100,
        ),
        0
    );
}

#[test]
fn seeded_percent_roll_records_compact_range_without_consuming_tapes() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![9_999];
    fixture.random_fallback_tape = vec![88];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.actor_turn = 1;
    let previous = state.begin_card_execution(PlayerSide::P1, PERCENT_CARD_ID);

    let roll = state.consume_percent_roll(PlayerSide::P1);
    state.finish_card_execution(previous);

    assert_eq!(roll, 0);
    assert_eq!(state.decision_tape, vec![9_999]);
    assert_eq!(state.random_fallback_tape, vec![88]);
    assert_eq!(state.decision_events.len(), 1);
    let event = &state.decision_events[0];
    assert_eq!(event.provider, ReplayDecisionProvider::SeededSynthetic);
    assert_eq!(event.seed, Some(17));
    assert_eq!(event.decision_kind, ReplayDecisionKind::PercentRoll);
    assert_eq!((event.decision_occurrence, event.ordinal), (1, 0));
    assert_eq!(event.selected_option, Some(0));
    assert_eq!(
        event.legal_range(),
        Some(ReplayDecisionIntegerRange {
            min_inclusive: 0,
            max_inclusive: 99,
        })
    );
    assert_eq!(event.legal_options(), None);
}

#[test]
fn five_seeded_percent_rolls_share_one_execution_and_match_ts_values() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    let mut state = ReplayState::test_from_fixture(&fixture);
    let previous = state.begin_card_execution(PlayerSide::P1, 4_000_046);

    let rolls = (0..5)
        .map(|_| state.consume_percent_roll(PlayerSide::P1))
        .collect::<Vec<_>>();
    state.finish_card_execution(previous);

    assert_eq!(rolls, vec![48, 57, 84, 88, 1]);
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (event.decision_occurrence, event.ordinal))
            .collect::<Vec<_>>(),
        vec![(1, 0), (2, 1), (3, 2), (4, 3), (5, 4)]
    );
    assert!(state
        .decision_events
        .iter()
        .all(|event| event.card_execution_occurrence == 1));
}

#[test]
fn typed_fallback_consumes_percent_replay_tape_before_seed() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(17),
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![42];
    let mut state = ReplayState::test_from_fixture(&fixture);
    let previous = state.begin_card_execution(PlayerSide::P1, 4_000_046);

    let rolls = (0..3)
        .map(|_| state.consume_percent_roll(PlayerSide::P1))
        .collect::<Vec<_>>();
    state.finish_card_execution(previous);

    assert_eq!(rolls, vec![42, 57, 84]);
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (event.provider, event.seed))
            .collect::<Vec<_>>(),
        vec![
            (ReplayDecisionProvider::ReplayTape, None),
            (ReplayDecisionProvider::SeededSynthetic, Some(17)),
            (ReplayDecisionProvider::SeededSynthetic, Some(17)),
        ]
    );
}

#[test]
fn seeded_hexagram_percent_roll_uses_zero_and_leaves_both_tapes_untouched() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![37];
    fixture.random_fallback_tape = vec![88];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 1;
    let previous = state.begin_card_execution(PlayerSide::P1, PERCENT_CARD_ID);

    let roll = state.consume_percent_roll(PlayerSide::P1);
    state.finish_card_execution(previous);

    assert_eq!(roll, 0);
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.p1.astrology.hexagram_effective_count, 1);
    assert_eq!(state.decision_tape, vec![37]);
    assert_eq!(state.random_fallback_tape, vec![88]);
    assert_eq!(state.decision_events.len(), 1);
    assert_eq!(
        (
            state.decision_events[0].provider,
            state.decision_events[0].seed,
            state.decision_events[0].selected_option,
        ),
        (ReplayDecisionProvider::Hexagram, None, Some(0))
    );
}

#[test]
fn replay_percent_roll_obeys_raw_hexagram_then_fallback_priority() {
    let mut fixture = percent_fixture();
    fixture.decision_tape = vec![7, -1, -1];
    fixture.random_fallback_tape = vec![42];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 2;
    let previous = state.begin_card_execution(PlayerSide::P1, PERCENT_CARD_ID);

    let raw = state.consume_percent_roll(PlayerSide::P1);
    let hexagram = state.consume_percent_roll(PlayerSide::P1);
    let fallback = state.consume_percent_roll(PlayerSide::P1);
    state.finish_card_execution(previous);

    assert_eq!((raw, hexagram, fallback), (7, 0, 42));
    assert!(state.decision_tape.is_empty());
    assert!(state.random_fallback_tape.is_empty());
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (
                event.provider,
                event.decision_occurrence,
                event.ordinal,
                event.selected_option,
            ))
            .collect::<Vec<_>>(),
        vec![
            (ReplayDecisionProvider::ReplayTape, 1, 0, Some(7)),
            (ReplayDecisionProvider::Hexagram, 2, 1, Some(0)),
            (ReplayDecisionProvider::RandomFallbackTape, 3, 2, Some(42),),
        ]
    );
}

#[test]
fn replay_percent_roll_accepts_out_of_99_values_verbatim() {
    // GetNextRandomValue -> GetNextParam 无上限：d8cd6e93/round-13 turn16
    // 后招判定读到 3000010，调用方 `< 1` 比较失败——回放带百分骰读到
    // 什么返回什么，不报 out of range。
    let mut fixture = percent_fixture();
    fixture.decision_tape = vec![3000010];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;
    let previous = state.begin_card_execution(PlayerSide::P1, PERCENT_CARD_ID);

    assert_eq!(state.consume_percent_roll(PlayerSide::P1), 3000010);
    state.finish_card_execution(previous);

    assert_eq!(state.decision_occurrence, 1);
    assert_eq!(state.evaluation_error, None);
}

/// Typed rows for the data-driven percent-roll table: every loop iteration of
/// the migrated test below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `fallback_out_of_range`).
struct PercentRollCase {
    decision_tape: &'static [i64],
    random_fallback_tape: &'static [i64],
    reason: &'static str,
}

const INVALID_PERCENT_ROLL_CASES: &[PercentRollCase] = &[
    PercentRollCase {
        decision_tape: &[-1],
        random_fallback_tape: &[100],
        reason: "percent roll fallback out of range",
    },
    PercentRollCase {
        decision_tape: &[],
        random_fallback_tape: &[],
        reason: "percent roll",
    },
];

fn run_invalid_percent_roll_case(case: &PercentRollCase) {
    let decision_tape = case.decision_tape.to_vec();
    let random_fallback_tape = case.random_fallback_tape.to_vec();
    let reason = case.reason;
    let mut fixture = percent_fixture();
    fixture.decision_tape = decision_tape;
    fixture.random_fallback_tape = random_fallback_tape;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;
    let previous = state.begin_card_execution(PlayerSide::P1, PERCENT_CARD_ID);

    assert_eq!(state.consume_percent_roll(PlayerSide::P1), 100);
    state.decision_tape.push(7);
    assert_eq!(state.consume_percent_roll(PlayerSide::P1), 7);
    state.finish_card_execution(previous);

    assert_eq!(state.decision_occurrence, 2);
    assert_eq!(state.decision_events.len(), 1);
    assert_eq!(
        (
            state.decision_events[0].decision_occurrence,
            state.decision_events[0].ordinal,
            state.decision_events[0].selected_option,
        ),
        (2, 1, Some(7))
    );
    assert_eq!(
        state
            .evaluation_error
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some(format!("missing original decision: {reason}; turn=0").as_str())
    );
}

crate::data_driven_cases! {
    table: INVALID_PERCENT_ROLL_CASES,
    run: run_invalid_percent_roll_case,
    suite: "replay::tests_percent_roll_decisions::invalid_or_missing_percent_rolls_advance_identity_without_success_events",
    valid: invalid_or_missing_percent_rolls_case_table_is_valid,
    cases: [
        invalid_or_missing_percent_rolls_advance_identity_without_success_events_fallback_out_of_range => 0,
        invalid_or_missing_percent_rolls_advance_identity_without_success_events_missing => 1,
    ]
}

#[test]
fn replay_random_range_hexagram_side_effect_does_not_invent_a_missing_value() {
    let fixture = percent_fixture();
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;
    state.p1.astrology.hexagram = 1;
    let previous = state.begin_card_execution(PlayerSide::P1, 4_000_068);

    assert_eq!(state.consume_random_range(PlayerSide::P1, 1, 11), 1);
    state.finish_card_execution(previous);

    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.p1.astrology.hexagram_effective_count, 1);
    assert!(state.decision_events.is_empty());
    assert_eq!(
        state
            .evaluation_error
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("missing original decision: random range; turn=0")
    );
}

#[test]
fn draw_bow_empty_queue_optional_roll_succeeds_without_error_or_event() {
    // 原版 GetNextParam 空队列返回 -1，-1 < 阈值即判定成功（再次行动）；
    // oracle 锚点 pa_23160a71 cp[44]。
    let card = draw_bow_shoot_tiger();
    let mut fixture = percent_fixture();
    fixture.players.p1.cards = filler_cards(card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.turn.extra_actions, 1);
    assert_eq!(state.decision_occurrence, 1);
    assert!(state.decision_events.is_empty());
    assert_eq!(state.evaluation_error, None);
}

#[test]
fn talent_108_and_draw_bow_share_execution_with_percent_ordinals_zero_and_one() {
    let card = draw_bow_shoot_tiger();
    let mut fixture = percent_fixture();
    fixture.players.p1.cards = filler_cards(card.clone());
    fixture.players.p1.talents = vec![108];
    fixture.decision_tape = vec![50, 9];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.turn.extra_actions, 1);
    assert_eq!(state.evaluation_error, None);
    assert_eq!(state.decision_events.len(), 2);
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (
                event.card_id,
                event.card_execution_occurrence,
                event.decision_occurrence,
                event.ordinal,
                event.selected_option,
            ))
            .collect::<Vec<_>>(),
        vec![
            (4_000_097, 1, 1, 0, Some(50)),
            (4_000_097, 1, 2, 1, Some(9)),
        ]
    );
}

#[test]
fn dream_white_snake_runs_rear_move_roll_before_status_params() {
    let card = original_card_definition_by_id(4_030_085).expect("missing 梦•白蛇吐信");
    let mut fixture = percent_fixture();
    fixture.players.p1.cards = filler_cards(card.clone());
    fixture.players.p1.talents = vec![108];
    fixture.decision_tape = vec![8, 105, 105];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p2.status.external_injury, 2);
    assert!(state.decision_tape.is_empty());
    assert_eq!(state.evaluation_error, None);
    assert_eq!(state.decision_events.len(), 1);
    assert_eq!(state.decision_events[0].selected_option, Some(8));
}

#[test]
fn nested_scopes_keep_percent_and_random_range_ordinals_independent() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    let mut state = ReplayState::test_from_fixture(&fixture);
    let outer = state.begin_card_execution(PlayerSide::P1, 98_000_001);
    state.consume_percent_roll(PlayerSide::P1);
    state.consume_random_range(PlayerSide::P1, 2, 5);

    let inner = state.begin_card_execution(PlayerSide::P1, 98_000_002);
    state.consume_percent_roll(PlayerSide::P1);
    state.consume_random_range(PlayerSide::P1, 2, 5);
    state.finish_card_execution(inner);

    state.consume_percent_roll(PlayerSide::P1);
    state.consume_random_range(PlayerSide::P1, 2, 5);
    state.finish_card_execution(outer);

    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (
                event.card_id,
                event.decision_kind,
                event.decision_occurrence,
                event.ordinal,
            ))
            .collect::<Vec<_>>(),
        vec![
            (98_000_001, ReplayDecisionKind::PercentRoll, 1, 0),
            (98_000_001, ReplayDecisionKind::RandomRange, 2, 0),
            (98_000_002, ReplayDecisionKind::PercentRoll, 3, 0),
            (98_000_002, ReplayDecisionKind::RandomRange, 4, 0),
            (98_000_001, ReplayDecisionKind::PercentRoll, 5, 1),
            (98_000_001, ReplayDecisionKind::RandomRange, 6, 1),
        ]
    );
}

#[test]
fn outside_execution_scope_keeps_legacy_percent_behavior_without_audit_identity() {
    let mut fixture = percent_fixture();
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![7];
    fixture.random_fallback_tape = vec![-1];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 1;

    assert_eq!(state.consume_percent_roll(PlayerSide::P1), 7);
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.random_fallback_tape, vec![-1]);
    assert_eq!(state.decision_occurrence, 0);
    assert!(state.decision_events.is_empty());

    let mut fallback = ReplayState::test_from_fixture(&percent_fixture());
    fallback.random_fallback_tape = vec![-1];
    assert_eq!(fallback.consume_percent_roll(PlayerSide::P1), -1);
    assert_eq!(fallback.decision_occurrence, 0);
    assert!(fallback.decision_events.is_empty());
}

#[test]
fn shared_percent_roll_fixture_locks_decision_and_terminal_state() {
    let path = shared_percent_roll_fixture_path();
    let contract_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("shared percent-roll fixture reads"),
    )
    .expect("shared percent-roll fixture JSON parses");
    let fixture = load_fixture_file(path).expect("shared percent-roll fixture loads");
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
        serde_json::to_value(&state.decision_tape).expect("remaining tape serializes"),
        contract_json["decisionContract"]["remainingDecisionTape"]
    );
    assert_eq!(
        serde_json::to_value(&state.random_fallback_tape)
            .expect("remaining fallback tape serializes"),
        contract_json["decisionContract"]["remainingRandomFallbackTape"]
    );
}

#[test]
fn card_4000035_dragonfly_skims_water_percent_roll_contract() {
    let card = dragonfly_skims_water();

    // 1. Success case: roll < 10 grants ExActionAgain (extra action)
    {
        let mut fixture = percent_fixture();
        fixture.players.p1.cards = filler_cards(card.clone());
        fixture.decision_tape = vec![5];
        let mut state = ReplayState::test_from_fixture(&fixture);
        let previous = state.begin_card_execution(PlayerSide::P1, card.id);
        state.test_apply_card_effect(PlayerSide::P1, &card, 0);
        state.finish_card_execution(previous);

        assert_eq!(state.p1.turn.extra_actions, 1);
        assert_eq!(state.decision_occurrence, 1);
        assert!(state.decision_tape.is_empty());
    }

    // 2. Failure case: roll >= 10 does NOT grant ExActionAgain (e.g. roll 23 like in g326 R13)
    {
        let mut fixture = percent_fixture();
        fixture.players.p1.cards = filler_cards(card.clone());
        fixture.decision_tape = vec![23];
        let mut state = ReplayState::test_from_fixture(&fixture);
        let previous = state.begin_card_execution(PlayerSide::P1, card.id);
        state.test_apply_card_effect(PlayerSide::P1, &card, 0);
        state.finish_card_execution(previous);

        assert_eq!(state.p1.turn.extra_actions, 0);
        assert_eq!(state.decision_occurrence, 1);
        assert!(state.decision_tape.is_empty());
    }

    // 3. Hexagram (卦象) case: consumes hexagram, rolls 0 (< 10), and guarantees ExActionAgain under synthetic seed
    {
        let mut fixture = percent_fixture();
        fixture.source = Some(FixtureSource {
            synthetic_decision_seed: Some(17),
            synthetic_decision_sides: vec![PlayerSide::P1],
            ..FixtureSource::default()
        });
        fixture.players.p1.cards = filler_cards(card.clone());
        let mut state = ReplayState::test_from_fixture(&fixture);
        state.p1.astrology.hexagram = 2;
        let previous = state.begin_card_execution(PlayerSide::P1, card.id);
        state.test_apply_card_effect(PlayerSide::P1, &card, 0);
        state.finish_card_execution(previous);

        assert_eq!(state.p1.turn.extra_actions, 1);
        assert_eq!(state.p1.astrology.hexagram, 1);
        assert_eq!(state.decision_occurrence, 1);
        assert_eq!(state.decision_events.len(), 1);
        assert_eq!(
            state.decision_events[0].provider,
            ReplayDecisionProvider::Hexagram
        );
        assert_eq!(state.decision_events[0].selected_option, Some(0));
    }
}

#[test]
fn replay_g326_round_13_exact_contract() {
    let path = g326_round_13_fixture_path();
    let fixture = load_fixture_file(&path).expect("g326 round 13 fixture loads");
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();

    assert!(summary.matches_fixture(&fixture));
    assert_eq!(summary.winner_side, PlayerSide::P2);
    assert_eq!(summary.actor_turn_count, 12);
    assert_eq!(summary.hp_delta_p1_minus_p2, -114);
}
