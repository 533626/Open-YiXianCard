//! 对局内“剩余槽位重排”建议器。
//!
//! 语义：固定光标前的可观测事件前缀（含快照逐字段相等），只重排目标方尚未打出
//! （前缀内没有 CardCompleted）的槽位，对每个变体整场重放。变体事件前缀与真实
//! 回放不一致（开局跳过效果、读槽类联动、随机决策流错位都会体现为快照差异）
//! 即丢弃，保证给出的每条建议都是“历史完全不变、只有未来不同”的反事实。

use std::collections::BTreeSet;

use crate::fixture::BattleFixture;
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::{
    run_replay_fixture_with_events_fallible, ReplayEvent, ReplayEventKind, ReplayRun,
};

use super::variant::{create_fixture_variant, fixture_deck_candidates};

pub const DEFAULT_ADVISOR_MAX_EVALUATIONS: usize = 720;
pub const DEFAULT_ADVISOR_TOP: usize = 5;

#[derive(Debug, Clone)]
pub struct AdvisorOptions {
    pub max_evaluations: usize,
    pub top: usize,
}

impl Default for AdvisorOptions {
    fn default() -> Self {
        Self {
            max_evaluations: DEFAULT_ADVISOR_MAX_EVALUATIONS,
            top: DEFAULT_ADVISOR_TOP,
        }
    }
}

#[derive(Debug, Clone)]
pub struct AdvisorCandidate {
    /// 开放槽位上的卡排列，按槽位序；`open_slots[i]` 槽放 `open_cards[i]`。
    pub open_cards: Vec<CardDefinition>,
    pub win_for_side: bool,
    pub hp_delta_for_side: i64,
    pub actor_turn_count: i64,
    pub is_baseline: bool,
}

#[derive(Debug, Clone)]
pub struct AdvisorReport {
    pub side: PlayerSide,
    /// 保持不变的事件前缀长度（光标事件含在内）。
    pub prefix_len: usize,
    pub locked_slots: Vec<usize>,
    pub open_slots: Vec<usize>,
    /// 开放槽多重集的全部去重排列数。
    pub arrangement_count: usize,
    pub evaluated: usize,
    pub inconsistent: usize,
    pub failed: usize,
    pub truncated: bool,
    pub baseline: Option<AdvisorCandidate>,
    pub top: Vec<AdvisorCandidate>,
}

impl AdvisorReport {
    pub fn full_deck_for_candidate(
        &self,
        fixture: &BattleFixture,
        candidate: &AdvisorCandidate,
    ) -> Vec<CardDefinition> {
        let mut full_deck = fixture_deck_candidates(fixture, self.side);
        for (position, slot) in self.open_slots.iter().enumerate() {
            if let Some(card) = candidate.open_cards.get(position) {
                if *slot < full_deck.len() {
                    full_deck[*slot] = card.clone();
                }
            }
        }
        full_deck
    }
}

pub fn advisor_candidate_run(
    fixture: &BattleFixture,
    side: PlayerSide,
    full_deck_cards: &[CardDefinition],
) -> Result<ReplayRun, String> {
    let variant = create_fixture_variant(fixture, side, full_deck_cards, None);
    run_replay_fixture_with_events_fallible(&variant)
}

pub fn advise_remaining_order(
    fixture: &BattleFixture,
    real_events: &[ReplayEvent],
    cursor_event_index: usize,
    side: PlayerSide,
    options: &AdvisorOptions,
) -> AdvisorReport {
    let deck = fixture_deck_candidates(fixture, side);
    let prefix_len = cursor_event_index.saturating_add(1).min(real_events.len());
    let prefix = &real_events[..prefix_len];

    let locked: BTreeSet<usize> = prefix
        .iter()
        .filter(|event| event.kind == ReplayEventKind::CardCompleted && event.actor == side)
        .filter_map(|event| event.slot)
        .filter(|slot| *slot < deck.len())
        .collect();
    let open_slots: Vec<usize> = (0..deck.len())
        .filter(|slot| !locked.contains(slot))
        .collect();
    let open_cards: Vec<CardDefinition> =
        open_slots.iter().map(|slot| deck[*slot].clone()).collect();

    let mut report = AdvisorReport {
        side,
        prefix_len,
        locked_slots: locked.into_iter().collect(),
        open_slots: open_slots.clone(),
        arrangement_count: multiset_arrangement_count(&open_cards),
        evaluated: 0,
        inconsistent: 0,
        failed: 0,
        truncated: false,
        baseline: None,
        top: Vec::new(),
    };
    if open_slots.len() < 2 {
        return report;
    }

    let evaluate =
        |arrangement: &[CardDefinition], is_baseline: bool, report: &mut AdvisorReport| {
            report.evaluated += 1;
            let mut full_deck = deck.clone();
            for (position, slot) in open_slots.iter().enumerate() {
                full_deck[*slot] = arrangement[position].clone();
            }
            let variant = create_fixture_variant(fixture, side, &full_deck, None);
            let run = match run_replay_fixture_with_events_fallible(&variant) {
                Ok(run) => run,
                Err(_) => {
                    report.failed += 1;
                    return;
                }
            };
            if run.events.len() < prefix.len() || run.events[..prefix.len()] != *prefix {
                report.inconsistent += 1;
                return;
            }
            let hp_delta_for_side = match side {
                PlayerSide::P1 => run.summary.hp_delta_p1_minus_p2,
                PlayerSide::P2 => -run.summary.hp_delta_p1_minus_p2,
            };
            let candidate = AdvisorCandidate {
                open_cards: arrangement.to_vec(),
                win_for_side: run.summary.winner_side == side,
                hp_delta_for_side,
                actor_turn_count: run.summary.actor_turn_count,
                is_baseline,
            };
            if is_baseline {
                report.baseline = Some(candidate.clone());
            }
            report.top.push(candidate);
        };

    evaluate(&open_cards, true, &mut report);
    let budget = options.max_evaluations.max(1);
    let mut pool = group_cards(&open_cards);
    let mut arrangement: Vec<CardDefinition> = Vec::with_capacity(open_cards.len());
    enumerate_arrangements(
        &mut pool,
        &mut arrangement,
        open_cards.len(),
        &mut |candidate| {
            if candidate == open_cards.as_slice() {
                return true;
            }
            if report.evaluated >= budget {
                report.truncated = true;
                return false;
            }
            evaluate(candidate, false, &mut report);
            true
        },
    );

    report.top.sort_by(|left, right| {
        right
            .win_for_side
            .cmp(&left.win_for_side)
            .then(right.hp_delta_for_side.cmp(&left.hp_delta_for_side))
            .then_with(|| {
                card_id_sequence(&left.open_cards).cmp(&card_id_sequence(&right.open_cards))
            })
    });
    report.top.truncate(options.top.max(1));
    report
}

/// 分组保持“先按卡 id、再按名字、再按首次出现槽序”的确定性顺序；
/// 只有完全相等的卡（含历史补丁后的数值）才会合并成同一组。
fn group_cards(cards: &[CardDefinition]) -> Vec<(CardDefinition, usize)> {
    let mut groups: Vec<(CardDefinition, usize)> = Vec::new();
    for card in cards {
        if let Some(group) = groups.iter_mut().find(|(existing, _)| existing == card) {
            group.1 += 1;
        } else {
            groups.push((card.clone(), 1));
        }
    }
    groups.sort_by(|(left, _), (right, _)| {
        left.id
            .cmp(&right.id)
            .then_with(|| left.name.cmp(&right.name))
    });
    groups
}

fn enumerate_arrangements(
    pool: &mut Vec<(CardDefinition, usize)>,
    arrangement: &mut Vec<CardDefinition>,
    target_len: usize,
    visit: &mut impl FnMut(&[CardDefinition]) -> bool,
) -> bool {
    if arrangement.len() == target_len {
        return visit(arrangement);
    }
    for index in 0..pool.len() {
        if pool[index].1 == 0 {
            continue;
        }
        pool[index].1 -= 1;
        arrangement.push(pool[index].0.clone());
        let keep_going = enumerate_arrangements(pool, arrangement, target_len, visit);
        arrangement.pop();
        pool[index].1 += 1;
        if !keep_going {
            return false;
        }
    }
    true
}

fn multiset_arrangement_count(cards: &[CardDefinition]) -> usize {
    let groups = group_cards(cards);
    let mut count: u128 = (1..=cards.len() as u128).product();
    for (_, group_size) in groups {
        let divisor: u128 = (1..=group_size as u128).product();
        count /= divisor;
    }
    count.min(usize::MAX as u128) as usize
}

fn card_id_sequence(cards: &[CardDefinition]) -> Vec<i64> {
    cards.iter().map(|card| card.id).collect()
}
