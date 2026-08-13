use super::advisor::{advise_remaining_order, advisor_candidate_run, AdvisorOptions};
use crate::replay::run_replay_fixture_with_events;
use crate::{engine_contract_fixture, BattleFixture, PlayerSide, ReplayEventKind, DECK_SIZE};

/// The advisor contract must not depend on mutable admission/corpus state.
fn advisor_fixture() -> BattleFixture {
    engine_contract_fixture().expect("engine contract fixture builds")
}

/// 光标定位：目标方第 `nth`（1-based）次 CardCompleted 的事件下标。
fn nth_card_completed_index(events: &[crate::ReplayEvent], side: PlayerSide, nth: usize) -> usize {
    events
        .iter()
        .enumerate()
        .filter(|(_, event)| event.kind == ReplayEventKind::CardCompleted && event.actor == side)
        .nth(nth - 1)
        .map(|(index, _)| index)
        .expect("enough card completions")
}

#[test]
fn advisor_baseline_is_consistent_and_slots_partition() {
    let fixture = advisor_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    let cursor = nth_card_completed_index(&run.events, PlayerSide::P1, 2);
    let report = advise_remaining_order(
        &fixture,
        &run.events,
        cursor,
        PlayerSide::P1,
        &AdvisorOptions {
            max_evaluations: 64,
            top: 5,
        },
    );

    let baseline = report.baseline.as_ref().expect("baseline evaluated");
    assert!(baseline.is_baseline);
    // 基线就是真实卡组：结局必须与真实回放一致。
    let real_delta = match report.side {
        PlayerSide::P1 => run.summary.hp_delta_p1_minus_p2,
        PlayerSide::P2 => -run.summary.hp_delta_p1_minus_p2,
    };
    assert_eq!(baseline.hp_delta_for_side, real_delta);
    assert_eq!(
        baseline.win_for_side,
        run.summary.winner_side == report.side
    );

    // 锁定/开放槽位互斥且覆盖整副牌。
    let mut all: Vec<usize> = report
        .locked_slots
        .iter()
        .chain(report.open_slots.iter())
        .copied()
        .collect();
    all.sort_unstable();
    assert_eq!(all, (0..DECK_SIZE).collect::<Vec<_>>());
    assert_eq!(report.locked_slots.len(), 2);

    // 预算生效，且报告内候选全部通过前缀一致性（不一致的已被丢弃计数）。
    assert!(report.evaluated <= 64);
    assert!(report.truncated);
    assert!(!report.top.is_empty());
    assert!(report.top.len() <= 5);
}

#[test]
fn advisor_is_deterministic() {
    let fixture = advisor_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    let cursor = nth_card_completed_index(&run.events, PlayerSide::P2, 1);
    let options = AdvisorOptions {
        max_evaluations: 48,
        top: 3,
    };
    let first = advise_remaining_order(&fixture, &run.events, cursor, PlayerSide::P2, &options);
    let second = advise_remaining_order(&fixture, &run.events, cursor, PlayerSide::P2, &options);

    assert_eq!(first.evaluated, second.evaluated);
    assert_eq!(first.inconsistent, second.inconsistent);
    let ids = |report: &super::advisor::AdvisorReport| {
        report
            .top
            .iter()
            .map(|candidate| {
                (
                    candidate.win_for_side,
                    candidate.hp_delta_for_side,
                    candidate
                        .open_cards
                        .iter()
                        .map(|card| card.id)
                        .collect::<Vec<_>>(),
                )
            })
            .collect::<Vec<_>>()
    };
    assert_eq!(ids(&first), ids(&second));
}

#[test]
fn advisor_ranks_wins_before_hp_delta() {
    let fixture = advisor_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    let cursor = nth_card_completed_index(&run.events, PlayerSide::P1, 1);
    let report = advise_remaining_order(
        &fixture,
        &run.events,
        cursor,
        PlayerSide::P1,
        &AdvisorOptions {
            max_evaluations: 256,
            top: 8,
        },
    );

    let wins: Vec<_> = report
        .top
        .iter()
        .take_while(|candidate| candidate.win_for_side)
        .collect();
    let rest: Vec<_> = report
        .top
        .iter()
        .skip_while(|candidate| candidate.win_for_side)
        .collect();
    assert!(
        rest.iter().all(|candidate| !candidate.win_for_side),
        "win candidates must rank before losses"
    );
    for block in [wins, rest] {
        for pair in block.windows(2) {
            assert!(pair[0].hp_delta_for_side >= pair[1].hp_delta_for_side);
        }
    }
}

#[test]
fn advisor_reports_no_candidates_when_all_slots_played() {
    let fixture = advisor_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    // 光标推进到战斗结束：若该方 8 个槽位都出现过，开放槽为空。
    let last = run.events.len() - 1;
    let report = advise_remaining_order(
        &fixture,
        &run.events,
        last,
        PlayerSide::P1,
        &AdvisorOptions::default(),
    );
    if report.open_slots.len() < 2 {
        assert_eq!(report.evaluated, 0);
        assert!(report.baseline.is_none());
        assert!(report.top.is_empty());
    } else {
        // 部分对局结束时仍有未打出的槽（提前分出胜负）：所有候选前缀=全程事件，
        // 结局与基线一致。
        let baseline = report.baseline.expect("baseline evaluated");
        for candidate in &report.top {
            assert_eq!(candidate.win_for_side, baseline.win_for_side);
            assert_eq!(candidate.hp_delta_for_side, baseline.hp_delta_for_side);
        }
    }
}

#[test]
fn advisor_candidate_run_baseline_matches_real_replay_summary() {
    let fixture = advisor_fixture();
    let run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    let cursor = nth_card_completed_index(&run.events, PlayerSide::P1, 2);
    let report = advise_remaining_order(
        &fixture,
        &run.events,
        cursor,
        PlayerSide::P1,
        &AdvisorOptions {
            max_evaluations: 16,
            top: 5,
        },
    );

    let baseline = report.baseline.as_ref().expect("baseline evaluated");
    let full_deck = report.full_deck_for_candidate(&fixture, baseline);
    let candidate_run =
        advisor_candidate_run(&fixture, report.side, &full_deck).expect("baseline replay succeeds");

    assert_eq!(candidate_run.summary.winner_side, run.summary.winner_side);
    assert_eq!(
        candidate_run.summary.actor_turn_count,
        run.summary.actor_turn_count
    );
    assert_eq!(
        candidate_run.summary.hp_delta_p1_minus_p2,
        run.summary.hp_delta_p1_minus_p2
    );
}
