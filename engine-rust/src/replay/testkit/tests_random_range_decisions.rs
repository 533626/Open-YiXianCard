use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::{FixtureExpected, FixtureSource};
use crate::model::PlayerSide;
use std::path::{Path, PathBuf};

fn shared_random_range_fixture_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust has project root parent")
        .join("battle-evaluator/fixtures/contracts/decisions/random-range-seed17.json")
}

fn random_attack_card(attack_count: i64) -> CardDefinition {
    let mut card = test_card(1_000_001, 1_000_001, "random-range-probe");
    card.attack = Some(2);
    card.random_attack = Some(5);
    card.attack_count = Some(attack_count);
    card
}

fn random_attack_fixture(card: CardDefinition) -> BattleFixture {
    minimal_fixture(
        filler_cards(card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    )
}

#[test]
fn random_range_hash_matches_ts_contract() {
    let indexes = [1, 2, 17, 99].map(|seed| {
        decisions::seeded_decision_choice_index(
            seed,
            PlayerSide::P1,
            3,
            2_000_004,
            ReplayDecisionKind::RandomRange,
            1,
            1,
            0,
            8,
        )
    });
    assert_eq!(indexes, [5, 3, 4, 6]);
}

#[test]
fn seeded_random_ranges_use_kind_local_ordinals_without_consuming_tapes() {
    let card = random_attack_card(2);
    let mut fixture = random_attack_fixture(card.clone());
    fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![9_999];
    fixture.random_fallback_tape = vec![4];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.decision_tape, vec![9_999]);
    assert_eq!(state.random_fallback_tape, vec![4]);
    assert_eq!(state.decision_events.len(), 2);
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (
                event.provider,
                event.card_execution_occurrence,
                event.decision_occurrence,
                event.ordinal,
                event.selected_option,
            ))
            .collect::<Vec<_>>(),
        vec![
            (ReplayDecisionProvider::SeededSynthetic, 1, 1, 0, Some(2)),
            (ReplayDecisionProvider::SeededSynthetic, 1, 2, 1, Some(5)),
        ]
    );
    assert!(state.decision_events.iter().all(|event| {
        event.decision_kind == ReplayDecisionKind::RandomRange
            && event.legal_range()
                == Some(ReplayDecisionIntegerRange {
                    min_inclusive: 2,
                    max_inclusive: 5,
                })
    }));
}

#[test]
fn replay_random_range_logs_raw_missing_then_uses_fallback() {
    let card = random_attack_card(1);
    let mut fixture = random_attack_fixture(card.clone());
    fixture.decision_tape = vec![-1, 77];
    fixture.random_fallback_tape = vec![4];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p2.core.hp, 26);
    assert_eq!(state.decision_tape, vec![77]);
    assert!(state.random_fallback_tape.is_empty());
    assert_eq!(state.decision_events.len(), 1);
    assert_eq!(
        state.decision_events[0].provider,
        ReplayDecisionProvider::RandomFallbackTape
    );
    assert_eq!(state.decision_events[0].selected_option, Some(4));
}

#[test]
fn typed_fallback_consumes_random_range_tape_before_seed() {
    let card = random_attack_card(2);
    let mut fixture = random_attack_fixture(card.clone());
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(17),
        ..FixtureSource::default()
    });
    fixture.decision_tape = vec![3];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (event.provider, event.seed, event.selected_option))
            .collect::<Vec<_>>(),
        vec![
            (ReplayDecisionProvider::ReplayTape, None, Some(3)),
            (ReplayDecisionProvider::SeededSynthetic, Some(17), Some(5)),
        ]
    );
}

#[test]
fn random_range_hexagram_keeps_seeded_and_replay_priorities_distinct() {
    let card = random_attack_card(1);

    let mut seeded_fixture = random_attack_fixture(card.clone());
    seeded_fixture.source = Some(FixtureSource {
        synthetic_decision_seed: Some(17),
        synthetic_decision_sides: vec![PlayerSide::P1],
        ..FixtureSource::default()
    });
    seeded_fixture.decision_tape = vec![3, 77];
    seeded_fixture.random_fallback_tape = vec![4];
    let mut seeded = ReplayState::test_from_fixture(&seeded_fixture);
    seeded.p1.astrology.hexagram = 1;
    seeded.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(seeded.p2.core.hp, 25);
    assert_eq!(seeded.p1.astrology.hexagram, 0);
    assert_eq!(seeded.p1.astrology.hexagram_effective_count, 1);
    assert_eq!(seeded.decision_events.len(), 1);
    assert_eq!(
        seeded.decision_events[0].provider,
        ReplayDecisionProvider::Hexagram
    );
    assert_eq!(seeded.decision_events[0].selected_option, Some(5));
    assert_eq!(seeded.decision_tape, vec![3, 77]);
    assert_eq!(seeded.random_fallback_tape, vec![4]);

    let mut replay_fixture = random_attack_fixture(card.clone());
    replay_fixture.decision_tape = vec![3, 77];
    replay_fixture.random_fallback_tape = vec![4];
    let mut replay = ReplayState::test_from_fixture(&replay_fixture);
    replay.p1.astrology.hexagram = 1;
    replay.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(replay.p2.core.hp, 27);
    assert_eq!(replay.decision_events[0].selected_option, Some(3));
    assert_eq!(replay.decision_tape, vec![77]);
    assert_eq!(replay.random_fallback_tape, vec![4]);

    let mut missing_fixture = random_attack_fixture(card.clone());
    missing_fixture.decision_tape = vec![-1, 77];
    missing_fixture.random_fallback_tape = vec![4];
    let mut missing = ReplayState::test_from_fixture(&missing_fixture);
    missing.p1.astrology.hexagram = 1;
    missing.test_apply_card_effect(PlayerSide::P1, &card, 0);
    // 真实 replay 中卦象只执行 GetNextRandomValue 副作用，不伪造区间上限
    // （BUILD_24666769-to-24705509_RULE_DELTA.md「2026-08-15 复核与修复收口」第 4 条）；
    // tape 缺值时落到 random_fallback_tape。
    assert_eq!(missing.p2.core.hp, 26);
    assert_eq!(missing.p1.astrology.hexagram, 0);
    assert_eq!(
        missing.decision_events[0].provider,
        ReplayDecisionProvider::RandomFallbackTape
    );
    assert_eq!(missing.decision_events[0].selected_option, Some(4));
    assert_eq!(missing.decision_tape, vec![77]);
    assert!(missing.random_fallback_tape.is_empty());
}

#[test]
fn card_422_pre_random_star_loss_does_not_overlap_hexagram_loss() {
    let card = random_attack_card(1);
    let mut fixture = random_attack_fixture(card);
    fixture.players.p1.cards[1] = test_card(422, 422, "星卦重叠探针");
    fixture.players.p1.active_slot_count = 2;
    fixture.decision_tape = vec![5];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 3;
    state.p1.astrology.star_power = 3;
    state.p1.astrology.dream_thunder_hexagram = 1;
    // 24589371/24610558 起（BUILD_24589371_RULE_DELTA §2，synthetic
    // batch-027 转正）：卦象消耗先扣星力的触发条件从 HasCardInDeck(422)
    // 改写为 HasBuff(ZiMangXingBao)——卡组含 422 本身不再生效。
    state.p1.astrology.zi_mang_xing_bao = 1;

    // build 24646245 起 GetNextRandomValue 的星力/卦象/共鸣71 改为互斥的
    // else-if 链（当前反编译 BattleCharacter.cs:9331-9345）：星力分支命中
    // 后不再消耗卦象，随机值走回放带、不再按卦象取 max；梦御雷不触发、
    // 卦象账本不动。oracle 锚点：hf-32308000 pa_fe5e36fb
    // cp9（五雷轰顶 5 次随机持 773+星力 8 → 星力 8→2、卦象保持 1）。
    assert_eq!(state.consume_random_range(PlayerSide::P1, 1, 9), 5);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p1.core.attack_bonus, 1);
    assert_eq!(state.p1.astrology.hexagram, 3);
    assert_eq!(state.p1.astrology.hexagram_effective_count, 0);
    assert_eq!(state.p1.astrology.dream_thunder_round_limit, 0);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 0);
}

#[test]
fn set_hexagram_bypasses_modify_loss_hooks() {
    let card = random_attack_card(1);
    let mut fixture = random_attack_fixture(card);
    fixture.players.p1.cards[1] = test_card(422, 422, "星卦重叠探针");
    fixture.players.p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 3;
    state.p1.astrology.star_power = 2;
    state.p1.astrology.dream_thunder_hexagram = 1;
    // 新语义下即使持有 buff 773，SetBuffValue 仍是纯替换：不得触发
    // 星力消耗、加攻转换或 梦御雷 回合标记。
    state.p1.astrology.zi_mang_xing_bao = 1;

    // SetBuffValue is replacement only: unlike ModifyBuffValue(-3), it must
    // not overlap Star Power, grant Attack Bonus, or arm 梦御雷's turn marker.
    assert_eq!(state.set_hexagram(PlayerSide::P1, 0), -3);
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p1.core.attack_bonus, 0);
    assert_eq!(state.p1.astrology.dream_thunder_round_limit, 0);
}

#[test]
fn ronghui_optional_random_uses_shared_hexagram_side_effect_kernel() {
    let optional_random = test_card(193, 193, "负面随机探针");
    let mut fixture = random_attack_fixture(optional_random.clone());
    fixture.players.p1.cards[1] = test_card(422, 422, "星卦重叠探针");
    fixture.players.p1.active_slot_count = 2;
    fixture.decision_tape = vec![0];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 3;
    state.p1.astrology.star_power = 3;
    // 同 card_422_pre_random_star_loss：触发条件为 buff 773 而非卡组含 422。
    state.p1.astrology.zi_mang_xing_bao = 1;

    assert_eq!(
        state.apply_synthetic_oracle_ronghui_card_effect(
            PlayerSide::P1,
            &optional_random,
            0,
            false,
            193,
        ),
        Some(false),
    );

    // build 24646245 else-if 链：星力分支命中后不再消耗卦象。
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p1.core.attack_bonus, 1);
    assert_eq!(state.p1.astrology.hexagram, 3);
    assert_eq!(state.p1.astrology.hexagram_effective_count, 0);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 0);
}

#[test]
fn dream_thunder_repeats_only_first_loss_and_turn_end_reopens_the_gate() {
    let fixture = random_attack_fixture(random_attack_card(1));
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 5;
    state.p1.astrology.dream_thunder_hexagram = 1;

    assert_eq!(state.modify_hexagram(PlayerSide::P1, -1), -1);
    assert_eq!(state.p1.astrology.hexagram, 5);
    assert_eq!(state.p1.astrology.dream_thunder_round_limit, 1);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 1);

    assert_eq!(state.modify_hexagram(PlayerSide::P1, -1), -1);
    assert_eq!(state.p1.astrology.hexagram, 4);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 2);

    state.test_play_actor_turn();
    assert_eq!(state.p1.astrology.dream_thunder_round_limit, 0);

    assert_eq!(state.modify_hexagram(PlayerSide::P1, -1), -1);
    // The played random card consumed one unrefunded Hexagram before turn end;
    // clearing the marker then lets this post-turn loss refund back to three.
    assert_eq!(state.p1.astrology.hexagram, 3);
    assert_eq!(state.p1.astrology.dream_thunder_round_limit, 1);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 4);
}

#[test]
fn hexagram_loss_accumulates_in_a_preexisting_hidden_ledger() {
    let fixture = random_attack_fixture(random_attack_card(1));
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.astrology.hexagram = 2;
    state.p1.astrology.lost_hexagram = 3;

    assert_eq!(state.modify_hexagram(PlayerSide::P1, -1), -1);
    assert_eq!(state.p1.astrology.hexagram, 1);
    assert_eq!(state.original_lost_hexagram_ledger(PlayerSide::P1), 4);
}

#[test]
fn random_range_missing_marks_strict_evaluation_and_returns_minimum() {
    let card = random_attack_card(1);
    let fixture = random_attack_fixture(card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p2.core.hp, 28);
    assert!(state.decision_events.is_empty());
    assert_eq!(state.decision_occurrence, 1);
    assert_eq!(
        state
            .evaluation_error
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("missing original decision: random range; turn=0"),
    );
}

#[test]
fn invalid_random_range_fallback_is_not_logged_as_a_successful_decision() {
    let card = random_attack_card(1);
    let mut fixture = random_attack_fixture(card.clone());
    fixture.random_fallback_tape = vec![99];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.fail_on_missing_decision = true;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p2.core.hp, 28);
    assert!(state.decision_events.is_empty());
    assert_eq!(state.decision_occurrence, 1);
    assert_eq!(
        state
            .evaluation_error
            .as_ref()
            .map(ToString::to_string)
            .as_deref(),
        Some("missing original decision: random range fallback out of range; turn=0"),
    );
}

#[test]
fn random_range_or_default_records_truthful_fallback_and_default_providers() {
    let fixture = random_attack_fixture(random_attack_card(1));
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.decision_tape = vec![-1, -1];
    state.random_fallback_tape = vec![2];
    let previous = state.begin_card_execution(PlayerSide::P1, 98_000_001);

    let fallback = state.consume_random_range_or_default(PlayerSide::P1, 0, 2, 1);
    let default = state.consume_random_range_or_default(PlayerSide::P1, 0, 2, 1);
    state.finish_card_execution(previous);

    assert_eq!((fallback, default), (2, 1));
    assert_eq!(
        state
            .decision_events
            .iter()
            .map(|event| (event.provider, event.ordinal, event.selected_option))
            .collect::<Vec<_>>(),
        vec![
            (ReplayDecisionProvider::RandomFallbackTape, 0, Some(2)),
            (ReplayDecisionProvider::DefaultValue, 1, Some(1)),
        ]
    );
    assert!(state.decision_tape.is_empty());
    assert!(state.random_fallback_tape.is_empty());
}

#[test]
fn shared_random_range_fixture_locks_decision_and_terminal_state() {
    let path = shared_random_range_fixture_path();
    let contract_json: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(&path).expect("shared random-range fixture reads"),
    )
    .expect("shared random-range fixture JSON parses");
    let fixture = load_fixture_file(path).expect("shared random-range fixture loads");
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
}
