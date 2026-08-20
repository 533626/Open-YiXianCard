//! Reduces one Detailed replay run to the hook chain and what each hook changed.
//!
//! `ReplayDetailedStep` already carries a full state sample per hook invocation,
//! but a consumer that renders those samples verbatim shows the whole state at
//! every step and hides the one thing worth reading: which hook moved which
//! field. This module validates consecutive samples against semantic mutation
//! receipts and publishes only receipt-attributed changes.
//!
//! Observation mode must not change the battle, so the trace carries the same
//! `ReplaySummary` any other mode produces; `hook_trace_matches_other_observation_modes`
//! pins that.

use super::{
    run_replay_fixture_with_detailed_events, ReplayAttackSegment, ReplayDetailEntry,
    ReplayDetailedStep, ReplayHookCategory, ReplayMutationReceipt, ReplaySummary,
};
use crate::fixture::BattleFixture;
use crate::model::PlayerSide;
use crate::replay::ReplayState;
use crate::Result;

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayHookTrace {
    pub summary: ReplaySummary,
    pub steps: Vec<ReplayHookTraceStep>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayHookTraceStep {
    /// Index of the event this hook ran under.
    ///
    /// The parity stream a consumer renders drops the TurnEnd boundary of a turn
    /// that never completed, which only ever happens on the final (lethal) turn,
    /// so the two streams agree on every index a hook step references — except the
    /// `BattleEnd` step, which can sit one past the parity stream. Consumers must
    /// therefore ignore out-of-range indices instead of shifting them.
    /// `hook_trace_event_indices_join_onto_the_parity_stream` pins this.
    pub event_index: usize,
    pub category: ReplayHookCategory,
    pub turn: i64,
    pub actor: PlayerSide,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1_changes: Vec<ReplayHookTraceChange>,
    pub p2_changes: Vec<ReplayHookTraceChange>,
    /// Per-hit attack segments for this step's turn, only populated for
    /// MainEffect steps of multi-hit attack cards. Empty for all other steps.
    /// The original client shows each hit as a separate floating damage number
    /// (百杀 4 段 8 攻 → 4 个 -8); without this the hook trace would only show
    /// the net hp/def diff folded across all hits.
    pub attack_segments: Vec<ReplayAttackSegment>,
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayHookTraceChange {
    pub group: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub before: i64,
    pub after: i64,
    /// Which card's effect produced this change. Receipt-attributed changes
    /// carry the effective card of the invocation the mutation ran under
    /// (temporary/nested invocations keep their own identity).
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
}

pub fn trace_replay_fixture_hooks(fixture: &BattleFixture) -> Result<ReplayHookTrace> {
    let run = run_replay_fixture_with_detailed_events(fixture)?;
    // The BattleStart step is recorded after the battle-start phase has already
    // mutated state, so without a baseline it would fold every "战斗开始时加 X"
    // effect (卜卦加卦象、筋骨健壮加上限、冥心烙印加内伤/恢复 …) into an empty
    // diff and hide them. Rebuild the pre-opening state and snapshot its
    // detail entries as the baseline the BattleStart step diffs against — this
    // matches the original game's battle-replay view, which shows the opening
    // state changes as part of the battle-start frame rather than as already-set
    // state with no visible cause.
    let pre_opening = ReplayState::from_fixture_pre_opening(fixture, true).ok();
    let pre_opening_p1 = pre_opening.as_ref().map(|state| state.p1.detail_entries());
    let pre_opening_p2 = pre_opening.as_ref().map(|state| state.p2.detail_entries());
    let mut previous: Option<&ReplayDetailedStep> = None;
    // After-card-hook receipts recorded inside a MainEffect window are deferred
    // here and claimed by the matching AfterCard step.
    let mut deferred: Vec<ReplayMutationReceipt> = Vec::new();
    let mut steps = Vec::with_capacity(run.steps.len());
    for step in &run.steps {
        let (earlier_p1, earlier_p2) = if step.category == ReplayHookCategory::BattleStart {
            (pre_opening_p1.as_deref(), pre_opening_p2.as_deref())
        } else {
            (
                previous.map(|earlier| earlier.p1.as_slice()),
                previous.map(|earlier| earlier.p2.as_slice()),
            )
        };
        let window_start = previous.map_or(0, |earlier| earlier.mutation_receipt_offset);
        let window = &run.mutation_receipts[window_start..step.mutation_receipt_offset];
        // Per-hit attack segments only belong on MainEffect steps: they sample
        // inside the card effect's attack loop, which runs before this step is
        // recorded. Group by event_index so segments attach to the exact card
        // that fired them, not to other cards on the same turn (再动 can put
        // multiple cards on one turn).
        let attack_segments = if step.category == ReplayHookCategory::MainEffect {
            run.attack_segments
                .iter()
                .filter(|segment| segment.event_index == step.event_index)
                .cloned()
                .collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let (p1_changes, p2_changes) = (
            attributed_changes(
                step,
                earlier_p1,
                &step.p1,
                window,
                &mut deferred,
                PlayerSide::P1,
            )?,
            attributed_changes(
                step,
                earlier_p2,
                &step.p2,
                window,
                &mut deferred,
                PlayerSide::P2,
            )?,
        );
        steps.push(ReplayHookTraceStep {
            event_index: step.event_index,
            category: step.category,
            turn: step.turn,
            actor: step.actor,
            slot: step.slot,
            card_id: step.card_id,
            card_name: step.card_name.clone(),
            p1_changes,
            p2_changes,
            attack_segments,
        });
        previous = Some(step);
    }
    ensure_no_deferred_receipts(&deferred)?;
    Ok(ReplayHookTrace {
        summary: run.summary,
        steps,
    })
}

fn ensure_no_deferred_receipts(deferred: &[ReplayMutationReceipt]) -> Result<()> {
    if deferred.is_empty() {
        return Ok(());
    }
    Err(crate::EngineError::Battle(super::BattleError::Invariant {
        message: format!("unclaimed deferred mutation receipts: {deferred:?}"),
    }))
}

fn is_card_step(category: ReplayHookCategory) -> bool {
    matches!(
        category,
        ReplayHookCategory::MainEffect | ReplayHookCategory::AfterCard
    )
}

/// Fold a receipt slice for one key into the net (before, after) pair the
/// trace publishes, mirroring what a sample diff over the same mutations would
/// produce.
fn fold_receipts(receipts: &[&ReplayMutationReceipt]) -> Result<Option<(i64, i64)>> {
    let Some(first) = receipts.first() else {
        return Ok(None);
    };
    let last = receipts.last().expect("non-empty receipt slice");
    if let Some(pair) = receipts
        .windows(2)
        .find(|pair| pair[0].after != pair[1].before)
    {
        return Err(crate::EngineError::Battle(super::BattleError::Invariant {
            message: format!(
                "discontinuous mutation receipt chain for key {}: {}->{}, then {}->{}",
                first.key, pair[0].before, pair[0].after, pair[1].before, pair[1].after
            ),
        }));
    }
    Ok(Some((first.before, last.after)))
}

/// Builds one step's change list exclusively from semantic mutation receipts.
/// Samples are an audit boundary, never an attribution source. After-card
/// receipts recorded before the MainEffect sample are deferred to the
/// matching AfterCard step.
fn attributed_changes(
    step: &ReplayDetailedStep,
    earlier: Option<&[ReplayDetailEntry]>,
    later: &[ReplayDetailEntry],
    window: &[ReplayMutationReceipt],
    deferred: &mut Vec<ReplayMutationReceipt>,
    side: PlayerSide,
) -> Result<Vec<ReplayHookTraceChange>> {
    // Claim deferred after-card receipts whose event joins onto this step.
    let mut claimed = Vec::new();
    let mut unclaimed = Vec::new();
    for receipt in deferred.drain(..) {
        if receipt.actor == side
            && receipt.category == step.category
            && (!is_card_step(step.category) || receipt.event_index == step.event_index)
        {
            claimed.push(receipt);
        } else {
            unclaimed.push(receipt);
        }
    }
    *deferred = unclaimed;

    let window_side: Vec<&ReplayMutationReceipt> = window
        .iter()
        .filter(|receipt| receipt.actor == side)
        .collect();
    let mut changes = Vec::new();
    // Deferred after-card receipts claimed by this step were recorded across
    // distinct invocations of the transaction; each keeps its own change, in
    // receipt order. Folding them would merge separate attacks into one
    // (before, after) pair that contradicts the neighboring samples, and
    // would keep only the last receipt's card identity. Emitted before the
    // window pass: they predate this step's sample.
    for receipt in &claimed {
        if receipt.before != receipt.after {
            changes.push(ReplayHookTraceChange {
                group: receipt.group,
                key: receipt.key,
                label: receipt.label,
                before: receipt.before,
                after: receipt.after,
                card_id: receipt.card_id,
                card_name: receipt.card_name.clone(),
            });
        }
    }
    // Keys in display order: later entries first, then earlier-only entries
    // (fields that dropped back to zero stay visible).
    let mut keys: Vec<&str> = later.iter().map(|entry| entry.key).collect();
    for entry in earlier.into_iter().flatten() {
        if !keys.contains(&entry.key) {
            keys.push(entry.key);
        }
    }
    for key in keys {
        let sample_before = earlier
            .into_iter()
            .flatten()
            .find(|entry| entry.key == key)
            .map_or(0, |entry| entry.value);
        let sample_after = later
            .iter()
            .find(|entry| entry.key == key)
            .map_or(0, |entry| entry.value);
        let window_receipts: Vec<&ReplayMutationReceipt> = window_side
            .iter()
            .filter(|receipt| receipt.key == key)
            .copied()
            .collect();
        let complete = match fold_receipts(&window_receipts)? {
            Some((before, after)) => before == sample_before && after == sample_after,
            None => sample_before == sample_after,
        };
        if !complete {
            return Err(crate::EngineError::Battle(super::BattleError::Invariant {
                message: format!(
                    "mutation receipt coverage gap at {:?} turn {} side {:?} key {}: sample {}->{}, receipts {:?}",
                    step.category,
                    step.turn,
                    side,
                    key,
                    sample_before,
                    sample_after,
                    window_receipts,
                ),
            }));
        }
        let mut attributed: Vec<&ReplayMutationReceipt> = Vec::new();
        for receipt in window_receipts {
            if receipt.category == step.category
                && (!is_card_step(step.category) || receipt.event_index == step.event_index)
            {
                attributed.push(receipt);
            } else if receipt.category == ReplayHookCategory::AfterCard {
                // Recorded inside this window but belongs to the card's
                // AfterCard step, which is recorded later.
                deferred.push(receipt.clone());
            } else if receipt.category == ReplayHookCategory::SelectCost {
                // SelectCost is the attribution bucket for mutations that run
                // outside any invocation or phase block (cost payment, card
                // transforms). They belong to the step whose window contains
                // them; folding is intentional, not a silent fallback.
                attributed.push(receipt);
            } else {
                return Err(crate::EngineError::Battle(super::BattleError::Invariant {
                    message: format!(
                        "receipt category {:?} mismatches step {:?} at turn {} side {:?} key {}",
                        receipt.category, step.category, step.turn, side, receipt.key
                    ),
                }));
            }
        }
        if step.category == ReplayHookCategory::AfterCard {
            for receipt in attributed {
                if receipt.before != receipt.after {
                    changes.push(ReplayHookTraceChange {
                        group: receipt.group,
                        key: receipt.key,
                        label: receipt.label,
                        before: receipt.before,
                        after: receipt.after,
                        card_id: receipt.card_id,
                        card_name: receipt.card_name.clone(),
                    });
                }
            }
            continue;
        }
        if let Some((before, after)) = fold_receipts(&attributed)? {
            if before != after {
                let last = attributed.last().copied();
                let (group, label) =
                    last.map_or(("", ""), |receipt| (receipt.group, receipt.label));
                changes.push(ReplayHookTraceChange {
                    group,
                    key,
                    label,
                    before,
                    after,
                    card_id: last.and_then(|receipt| receipt.card_id),
                    card_name: last.and_then(|receipt| receipt.card_name.clone()),
                });
            }
        }
    }
    Ok(changes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
    use crate::model::{CardDefinition, DECK_SIZE};
    use crate::{
        engine_contract_fixture, evaluate_replay_fixture_fallible,
        run_replay_fixture_with_parity_events,
    };

    fn contract_fixture() -> BattleFixture {
        engine_contract_fixture().expect("engine contract fixture builds")
    }

    fn mutation_receipt(before: i64, after: i64) -> ReplayMutationReceipt {
        ReplayMutationReceipt {
            turn: 1,
            actor: PlayerSide::P1,
            event_index: 1,
            card_id: Some(0),
            card_name: Some("普通攻击".into()),
            category: ReplayHookCategory::MainEffect,
            kind: super::super::ReplayMutationKind::Counter,
            group: "测试",
            key: "counter",
            label: "计数",
            before,
            after,
            applied: after - before,
        }
    }

    #[test]
    fn receipt_fold_rejects_an_internally_discontinuous_chain() {
        let first = mutation_receipt(0, 5);
        let second = mutation_receipt(3, 10);
        let error = fold_receipts(&[&first, &second]).expect_err("chain must fail closed");
        assert!(error
            .to_string()
            .contains("discontinuous mutation receipt chain"));
    }

    #[test]
    fn unclaimed_deferred_receipts_fail_closed() {
        let receipt = mutation_receipt(0, 1);
        let error = ensure_no_deferred_receipts(&[receipt]).expect_err("tail must fail closed");
        assert!(error
            .to_string()
            .contains("unclaimed deferred mutation receipts"));
    }

    #[test]
    fn extra_action_source_is_receipted_in_the_action_again_step() {
        // 云剑•狂炎 (401) wounds, grants ExActionAgain, then consumes it
        // when the completed card resolves its single action-again transaction.
        let fixture = invocation_fixture(&[401], &[0], Vec::new());
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let main = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.card_name.as_deref() == Some("云剑•狂炎")
            })
            .expect("云剑•狂炎 MainEffect step");
        let granted = main
            .p1_changes
            .iter()
            .find(|change| change.key == "extraActions")
            .expect("MainEffect extra-action grant");
        assert_eq!((granted.before, granted.after), (0, 1));
        assert_eq!(granted.card_name.as_deref(), Some("云剑•狂炎"));

        let action_again = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::ActionAgain
                    && step.card_name.as_deref() == Some("云剑•狂炎")
            })
            .expect("云剑•狂炎 ActionAgain step");
        let consumed = action_again
            .p1_changes
            .iter()
            .find(|change| change.key == "extraActions")
            .expect("ActionAgain extra-action consumption");
        assert_eq!((consumed.before, consumed.after), (1, 0));
    }

    #[test]
    fn physique_mutation_is_receipted_in_main_effect() {
        let mut fixture = invocation_fixture(&[205], &[0], Vec::new());
        fixture.players.p1.cards[0].anima = Some(0);
        fixture.players.p1.cards[0].physique = Some(2);
        fixture.players.p1.cards[0].other_params = vec![100, 0];

        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let main = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect && step.card_id == Some(205)
            })
            .expect("physique card MainEffect step");
        let physique = main
            .p1_changes
            .iter()
            .find(|change| change.key == "physique")
            .expect("MainEffect physique mutation");
        assert_eq!((physique.before, physique.after), (0, 2));
        assert_eq!(physique.card_id, Some(205));
    }

    #[test]
    fn gourd_source_and_entangle_block_are_receipted_in_action_again() {
        // 天髓葫芦 (132) grants two gourd uses. 土灵印 (7_000_011)
        // activates Earth before action-again resolution, making
        // the gourd eligible. 困缚 then blocks the continuation, but both
        // the gourd charge and the negative-status layer are still consumed in
        // the explicit ActionAgain attribution scope.
        // P2's 梦•缚仙古藤 (343) supplies the one 困缚 layer on turn 2.
        let mut fixture = invocation_fixture(&[132, 7_000_011], &[343], Vec::new());
        fixture.max_actor_turns = Some(3);
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let action_again = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::ActionAgain
                    && step.card_name.as_deref() == Some("土灵印")
            })
            .expect("土灵印 ActionAgain step");
        let gourd = action_again
            .p1_changes
            .iter()
            .find(|change| change.key == "fiveElementsGourd")
            .expect("ActionAgain gourd consumption");
        assert_eq!((gourd.before, gourd.after), (2, 1));
        let entangle = action_again
            .p1_changes
            .iter()
            .find(|change| change.key == "entangle")
            .expect("ActionAgain entangle consumption");
        assert_eq!((entangle.before, entangle.after), (1, 0));
    }

    /// Minimal two-player fixture for the temporary/nested invocation
    /// contract tests. `initial_anima` is high so cost payment never blocks
    /// the copied/replayed cards.
    fn invocation_fixture(p1_ids: &[i64], p2_ids: &[i64], decisions: Vec<i64>) -> BattleFixture {
        use crate::replay::original_config;
        let cards = |ids: &[i64]| -> Vec<CardDefinition> {
            let mut cards: Vec<_> = ids
                .iter()
                .map(|id| {
                    original_config::original_card_definition(*id)
                        .expect("card must be in original config")
                })
                .collect();
            cards.resize_with(DECK_SIZE, || {
                original_config::original_card_definition(0).expect("basic attack")
            });
            cards
        };
        let player = |cards: Vec<CardDefinition>| FixturePlayer {
            level: 5,
            base_max_hp: 500,
            extra_max_hp: None,
            battle_start_hp: None,
            character_id: None,
            talents: Vec::new(),
            talent_resonance_id: None,
            fate_strategies: Vec::new(),
            fate_strategy_temp_datas: Default::default(),
            active_slot_count: DECK_SIZE,
            initial_defense: 0,
            initial_anima: 10,
            initial_guard: 0,
            initial_momentum: 0,
            initial_momentum_limit: None,
            initial_agility: 0,
            initial_battle_buffs: Default::default(),
            permanent_buff_temp_datas: Default::default(),
            talent_temp_datas: Default::default(),
            talent_card_params: Default::default(),
            last_round_used_card_base_ids: Vec::new(),
            last_round_life: None,
            last_round_exp: 0,
            hand_cards: Vec::new(),
            used_ke_yin_cards: Vec::new(),
            cards,
        };
        BattleFixture {
            schema_version: 1,
            source: None,
            first_player_side: PlayerSide::P1,
            decision_tape: decisions,
            random_fallback_tape: Vec::new(),
            expected: FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
            max_actor_turns: Some(1),
            historical_card_overrides: Vec::new(),
            catalog_cards: Vec::new(),
            players: FixturePlayers {
                p1: player(cards(p1_ids)),
                p2: player(cards(p2_ids)),
            },
        }
    }

    /// 战斗开始效果必须落在 BattleStart 步骤里，而不是被折叠进空基线。
    /// 原版战斗播放界面在开局帧显示“战斗开始时加 X”的状态变化，hook trace
    /// 要复现这一点：卜卦仙命（30）加 1 卦象应作为 BattleStart 的 p1 变更出现。
    #[test]
    fn battle_start_effects_report_as_battle_start_changes() {
        let mut fixture = contract_fixture();
        fixture.players.p1.talents = vec![30];
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");

        assert_eq!(trace.steps[0].category, ReplayHookCategory::BattleStart);
        let hexagram = trace.steps[0]
            .p1_changes
            .iter()
            .find(|change| change.key == "hexagram")
            .expect("battle-start divination gain must show in the BattleStart step");
        assert_eq!((hexagram.before, hexagram.after), (0, 1));
    }

    /// 没有战斗开始效果的 fixture 保持原契约：BattleStart 步骤仍是空基线。
    #[test]
    fn battle_start_step_stays_empty_without_opening_effects() {
        let fixture = contract_fixture();
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        assert!(trace.steps[0].p1_changes.is_empty());
        assert!(trace.steps[0].p2_changes.is_empty());
    }

    #[test]
    fn hook_trace_matches_other_observation_modes() {
        let fixture = contract_fixture();
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let parity = run_replay_fixture_with_parity_events(&fixture).expect("parity replay");
        let plain = evaluate_replay_fixture_fallible(&fixture).expect("summary replay");

        // Observing the battle must not change it.
        assert_eq!(trace.summary, parity.summary);
        assert_eq!(trace.summary, plain.summary);
    }

    #[test]
    fn detailed_turn_end_receipts_keep_stable_phase_order_without_changing_summary() {
        let fixture = contract_fixture();
        let detailed =
            crate::run_replay_fixture_with_detailed_events(&fixture).expect("detailed replay");
        let parity = run_replay_fixture_with_parity_events(&fixture).expect("parity replay");
        assert_eq!(detailed.summary, parity.summary);

        // 原版 OnTurnEnded：formations 先于 梦狂耳，再先于 waterMomentum
        // （BattleCharacter.cs IL_0b61 → IL_18c6 → IL_1cd0）。
        let expected = [
            "ronghui",
            "mirageRonghui",
            "formations",
            "dreamBeforeWater",
            "waterMomentum",
            "temporaryResources",
            "hardBranchBamboo",
            "sanWeiHuan",
            "poisonImmunity",
            "pendingHexagram",
            "statusDecay",
            "fengMoPhysique",
            "ledgerReset",
        ];
        assert!(!detailed.turn_end_hooks.is_empty());
        for turn in detailed.turn_end_hooks.chunks_exact(expected.len()) {
            assert_eq!(
                turn.iter().map(|receipt| receipt.hook).collect::<Vec<_>>(),
                expected
            );
            for adjacent in turn.windows(2) {
                assert_eq!(adjacent[0].after, adjacent[1].before);
            }
        }
        assert!(detailed
            .turn_end_hooks
            .chunks_exact(expected.len())
            .remainder()
            .is_empty());
    }

    /// A step whose `event_index` landed on the wrong event would attribute one
    /// hook's effect to a different card, so the join has to be checked by event
    /// kind and turn rather than by a bound check that any index would pass.
    #[test]
    fn hook_trace_event_indices_join_onto_the_parity_stream() {
        let fixture = contract_fixture();
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let parity = run_replay_fixture_with_parity_events(&fixture).expect("parity replay");

        for step in &trace.steps {
            // The parity stream omits an aborted turn's TurnEnd, which pushes the
            // BattleEnd step one past its end; nothing else may fall out of range.
            if step.category == ReplayHookCategory::BattleEnd {
                continue;
            }
            let event = parity
                .events
                .get(step.event_index)
                .unwrap_or_else(|| panic!("step {step:?} has no parity event"));
            let expected = match step.category {
                ReplayHookCategory::BattleStart => crate::ReplayEventKind::BattleStart,
                ReplayHookCategory::TurnStart => crate::ReplayEventKind::TurnStart,
                ReplayHookCategory::TurnEnd => crate::ReplayEventKind::TurnEnd,
                _ => crate::ReplayEventKind::CardCompleted,
            };
            assert_eq!(
                event.kind, expected,
                "{:?} step on turn {} joined onto {:?}",
                step.category, step.turn, event.kind,
            );
            if step.category != ReplayHookCategory::BattleStart {
                assert_eq!(event.turn, step.turn);
            }
        }
    }

    #[test]
    fn hook_trace_reports_only_what_each_hook_changed() {
        let fixture = contract_fixture();
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let detailed =
            crate::run_replay_fixture_with_detailed_events(&fixture).expect("detailed replay");
        assert_eq!(trace.steps.len(), detailed.steps.len());

        // The opening baseline has nothing to diff against.
        assert!(trace.steps[0].p1_changes.is_empty());
        assert!(trace.steps[0].p2_changes.is_empty());

        // Reducing to changes has to actually reduce: a trace where every step
        // republishes the whole state would pass the joins above and still be
        // unreadable.
        let changed_fields = trace
            .steps
            .iter()
            .map(|step| step.p1_changes.len() + step.p2_changes.len())
            .sum::<usize>();
        let sampled_fields = detailed
            .steps
            .iter()
            .map(|step| step.p1.len() + step.p2.len())
            .sum::<usize>();
        assert!(changed_fields > 0, "no hook changed anything");
        assert!(
            changed_fields * 4 < sampled_fields,
            "hook trace kept {changed_fields} of {sampled_fields} sampled fields",
        );

        // Each published change must be a real diff against the previous sample.
        for (index, step) in trace.steps.iter().enumerate().skip(1) {
            let earlier = &detailed.steps[index - 1];
            let later = &detailed.steps[index];
            for (changes, before, after) in [
                (&step.p1_changes, &earlier.p1, &later.p1),
                (&step.p2_changes, &earlier.p2, &later.p2),
            ] {
                for change in changes {
                    assert_ne!(change.before, change.after);
                    assert_eq!(
                        change.before,
                        value_of(before, change.key),
                        "step {index} {:?} key {}",
                        step.category,
                        change.key,
                    );
                    assert_eq!(
                        change.after,
                        value_of(after, change.key),
                        "step {index} {:?} key {}",
                        step.category,
                        change.key,
                    );
                }
            }
        }
    }

    #[test]
    fn hook_trace_covers_the_turn_and_card_hook_categories() {
        let trace = trace_replay_fixture_hooks(&contract_fixture()).expect("hook trace");
        for category in [
            ReplayHookCategory::BattleStart,
            ReplayHookCategory::TurnStart,
            ReplayHookCategory::MainEffect,
            ReplayHookCategory::AfterCard,
            ReplayHookCategory::TurnEnd,
            ReplayHookCategory::BattleEnd,
        ] {
            assert!(
                trace.steps.iter().any(|step| step.category == category),
                "hook category {category:?} never appears in the trace",
            );
        }
    }

    fn value_of(entries: &[ReplayDetailEntry], key: &str) -> i64 {
        entries
            .iter()
            .find(|entry| entry.key == key)
            .map_or(0, |entry| entry.value)
    }

    /// Per-hit attack segments must be sampled inside the `attack_by_config`
    /// loop and attached to the MainEffect step of the turn that fired them.
    /// Each segment carries the target's hp/def before and after one hit, so
    /// the browser can show per-hit damage the way the original client does.
    #[test]
    fn attack_segments_sample_per_hit_on_main_effect() {
        let fixture = contract_fixture();
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let main_effect_steps: Vec<_> = trace
            .steps
            .iter()
            .filter(|step| step.category == ReplayHookCategory::MainEffect)
            .collect();
        assert!(
            !main_effect_steps.is_empty(),
            "no MainEffect step in contract fixture",
        );
        // Every attack segment must be attached to a MainEffect step and carry
        // a hit_index, target side, and before/after hp+def.
        for step in &trace.steps {
            for segment in &step.attack_segments {
                assert_eq!(
                    step.category,
                    ReplayHookCategory::MainEffect,
                    "attack segment attached to non-MainEffect step {:?}",
                    step.category,
                );
                assert_eq!(segment.event_index, step.event_index);
                assert!(
                    segment.hp_before >= segment.hp_after,
                    "hp before {} should be >= after {} for hit {}",
                    segment.hp_before,
                    segment.hp_after,
                    segment.hit_index,
                );
                assert!(
                    segment.actor_hp_before != 0 || segment.actor_hp_after != 0,
                    "actor HP sampling is missing for hit {}",
                    segment.hit_index,
                );
            }
        }
        // Non-MainEffect steps must have zero attack segments.
        for step in &trace.steps {
            if step.category != ReplayHookCategory::MainEffect {
                assert!(
                    step.attack_segments.is_empty(),
                    "{:?} step should not carry attack segments",
                    step.category,
                );
            }
        }
    }

    /// Custom attack loops that do not go through `attack_by_config` must be
    /// covered by the per-hit sampling that lives in
    /// `apply_attack_with_options`. 五雷轰顶 (4000046) repeats 5 times with a
    /// 30% chance per hit (otherParams=[30,8]); with the decision tape forcing
    /// all five rolls to hit, the MainEffect step must carry 5 segments with
    /// continuous hit_index 0..5.
    #[test]
    fn attack_segments_cover_custom_attack_loops_like_wulei() {
        use crate::fixture::FixturePlayer;
        use crate::replay::original_config;

        let mut fixture = contract_fixture();
        let wulei = original_config::original_card_definition(4_000_046)
            .expect("五雷轰顶 must be in original config");
        let basic = original_config::original_card_definition(0)
            .expect("普通攻击 must be in original config");
        // 五雷轰顶 anima=-1 表示 1 灵气费用（原版编码），p1 初始给足灵气。
        fixture.players.p1 = FixturePlayer {
            cards: std::iter::once(wulei)
                .chain(std::iter::repeat_n(basic, 7))
                .collect(),
            initial_anima: 2,
            ..fixture.players.p1
        };
        // 5 次 30% 判定全部命中（roll 值 0 < 30）。
        fixture.decision_tape = vec![0, 0, 0, 0, 0];
        fixture.max_actor_turns = Some(1);

        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let wulei_steps: Vec<_> = trace
            .steps
            .iter()
            .filter(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.card_name.as_deref() == Some("五雷轰顶")
            })
            .collect();
        assert_eq!(wulei_steps.len(), 1, "五雷轰顶 mainEffect step missing");
        let segments = &wulei_steps[0].attack_segments;
        assert_eq!(
            segments.len(),
            5,
            "五雷轰顶 5 次全中应产生 5 个攻击段，实际 {}",
            segments.len(),
        );
        for (index, segment) in segments.iter().enumerate() {
            assert_eq!(segment.hit_index, index, "段号必须从 0 连续递增");
            assert_eq!(segment.target, PlayerSide::P2);
            assert!(segment.hp_before >= segment.hp_after);
        }
    }

    /// Shared OnAfterExecuted attacks still belong to the card that opened the
    /// execution. Closing the observation window immediately after the printed
    /// body would hide this second segment (and any HP hooks it triggers).
    #[test]
    fn attack_segments_include_after_card_formation_follow_ups() {
        use crate::fixture::FixturePlayer;
        use crate::replay::original_config;

        let mut fixture = contract_fixture();
        let formation = original_config::original_card_definition(8_020_008)
            .expect("周天剑阵 must be in original config");
        let basic = original_config::original_card_definition(0)
            .expect("普通攻击 must be in original config");
        fixture.players.p1 = FixturePlayer {
            cards: std::iter::once(formation)
                .chain(std::iter::repeat_n(basic, 7))
                .collect(),
            ..fixture.players.p1
        };
        // p1 establishes the formation on turn 1 and attacks on turn 3.
        fixture.max_actor_turns = Some(3);

        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let attack = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.actor == PlayerSide::P1
                    && step.turn == 3
            })
            .expect("p1 turn-3 attack step");
        assert_eq!(attack.attack_segments.len(), 2);
        assert_eq!(attack.attack_segments[0].hit_index, 0);
        assert_eq!(attack.attack_segments[1].hit_index, 1);
    }

    /// A card that attacks several times through `attack_by_config` (attackCount
    /// `attackCount > 1`, e.g. 云剑•猫爪 4攻×2, must also number its segments continuously
    /// from 0, sharing the same choke point as custom loops.
    #[test]
    fn attack_segments_number_continuously_across_attack_count() {
        use crate::fixture::FixturePlayer;
        use crate::replay::original_config;

        let mut fixture = contract_fixture();
        let multi_hit = original_config::original_card_definition(8)
            .expect("云剑•猫爪 must be in original config");
        assert!(
            multi_hit.attack_count.unwrap_or(1) > 1,
            "test card must be multi-hit",
        );
        let basic = original_config::original_card_definition(0)
            .expect("普通攻击 must be in original config");
        fixture.players.p1 = FixturePlayer {
            cards: std::iter::once(multi_hit)
                .chain(std::iter::repeat_n(basic, 7))
                .collect(),
            ..fixture.players.p1
        };
        fixture.max_actor_turns = Some(1);

        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        for step in &trace.steps {
            if step.category == ReplayHookCategory::MainEffect {
                for (index, segment) in step.attack_segments.iter().enumerate() {
                    assert_eq!(
                        segment.hit_index, index,
                        "段号必须从 0 连续递增，第 {index} 段拿到 {}",
                        segment.hit_index,
                    );
                }
            }
        }
    }

    /// AfterCard misattribution class: after-card hooks (e.g. 周天剑阵's
    /// follow-up attack) run in the AfterCardHooks phase, whose mutations the
    /// MainEffect sample window straddles. The follow-up damage must be
    /// attributed to the AfterCard step of the card that opened the
    /// execution, not folded into that card's MainEffect step.
    #[test]
    fn after_card_hook_mutations_join_the_after_card_step() {
        use crate::fixture::FixturePlayer;
        use crate::replay::original_config;

        let mut fixture = contract_fixture();
        let formation = original_config::original_card_definition(8_020_008)
            .expect("周天剑阵 must be in original config");
        let basic = original_config::original_card_definition(0)
            .expect("普通攻击 must be in original config");
        fixture.players.p1 = FixturePlayer {
            cards: std::iter::once(formation)
                .chain(std::iter::repeat_n(basic, 7))
                .collect(),
            ..fixture.players.p1
        };
        // p1 establishes the formation on turn 1 and attacks on turn 3.
        fixture.max_actor_turns = Some(3);

        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let main_effect = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.actor == PlayerSide::P1
                    && step.turn == 3
            })
            .expect("p1 turn-3 attack step");
        let after_card = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::AfterCard
                    && step.actor == PlayerSide::P1
                    && step.turn == 3
            })
            .expect("p1 turn-3 after-card step");
        assert_eq!(main_effect.attack_segments.len(), 2);
        let printed = &main_effect.attack_segments[0];
        let follow_up = &main_effect.attack_segments[1];

        // The printed attack's damage stays on the MainEffect step…
        let main_hp = main_effect
            .p2_changes
            .iter()
            .find(|change| change.key == "hp")
            .expect("MainEffect hp change");
        assert_eq!(
            main_hp.before - main_hp.after,
            printed.hp_before - printed.hp_after,
            "MainEffect step must carry only the printed attack damage",
        );
        assert_eq!(main_hp.card_name.as_deref(), Some("普通攻击"));
        // …and the follow-up attack, which runs in the after-card hook phase,
        // must join the AfterCard step instead of being folded into MainEffect.
        let after_hp = after_card
            .p2_changes
            .iter()
            .find(|change| change.key == "hp")
            .expect("AfterCard hp change");
        assert_eq!(
            after_hp.before - after_hp.after,
            follow_up.hp_before - follow_up.hp_after,
            "AfterCard step must carry the follow-up attack damage",
        );
        assert_eq!(after_hp.card_name.as_deref(), Some("普通攻击"));
    }

    /// Multi-invocation transactions (梅开二度 repeat + a persistent
    /// after-card follow-up) defer several same-key receipts into one AfterCard
    /// step; each follow-up must keep its own (before, after) instead of being
    /// merged into a single change spanning all of them.
    #[test]
    fn after_card_step_emits_one_change_per_deferred_follow_up() {
        use crate::replay::original_config;
        // 溟空剑阵诀 (1_000_051) grants a persistent follow-up attack value;
        // 梅开二度 (4_000_041) repeats the next card play, so the 周天剑阵
        // (8_020_008) transaction on turn 5 runs two invocations (formation
        // counter 2). Each invocation fires the dark-void follow-up, whose
        // attack then satisfies 周天剑阵's own heaven-cycle follow-up — four
        // after-card attacks in one transaction.
        let art_card = original_config::original_card_definition(1_000_051)
            .expect("溟空剑阵诀 must be in original config");
        let art = art_card.other_params.first().copied().unwrap_or(0);
        let formation_card = original_config::original_card_definition(8_020_008)
            .expect("周天剑阵 must be in original config");
        let heaven_damage = formation_card.other_params.get(1).copied().unwrap_or(0);
        assert!(
            art > 0 && heaven_damage > 0,
            "follow-up values must be configured"
        );
        let mut fixture = invocation_fixture(&[1_000_051, 4_000_041, 8_020_008], &[0], Vec::new());
        fixture.max_actor_turns = Some(5);
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let step = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::AfterCard
                    && step.turn == 5
                    && step.card_name.as_deref() == Some("周天剑阵")
            })
            .expect("turn-5 周天剑阵 AfterCard step");
        let hp_changes: Vec<_> = step
            .p2_changes
            .iter()
            .filter(|change| change.key == "hp")
            .collect();
        assert_eq!(
            hp_changes.len(),
            4,
            "each after-card follow-up must keep its own change: {:#?}",
            step.p2_changes,
        );
        // Turn 1's own 溟空剑阵诀 follow-up (500 → 500-art) landed on turn 1's
        // AfterCard step; the turn-5 chain starts from 500-art and alternates
        // dark-void (art) and heaven-cycle (heaven_damage) follow-ups.
        assert_eq!(hp_changes[0].before, 500 - art);
        let expected_deltas = [art, heaven_damage, art, heaven_damage];
        for (index, change) in hp_changes.iter().enumerate() {
            assert_eq!(
                change.before - change.after,
                expected_deltas[index],
                "follow-up {index} must carry its own delta",
            );
            assert_eq!(change.card_name.as_deref(), Some("周天剑阵"));
            if index + 1 < hp_changes.len() {
                assert_eq!(
                    change.after,
                    hp_changes[index + 1].before,
                    "follow-up changes must be contiguous",
                );
            }
        }
    }

    /// A temporary invocation's after-card follow-up is deferred from the outer
    /// card's MainEffect window and claimed by the outer card's AfterCard step;
    /// the claimed change must keep the copied (inner) card's identity instead
    /// of falling back to the AfterCard step's own card.
    #[test]
    fn deferred_after_card_follow_up_keeps_the_copied_card_identity() {
        // 灵韵御心琴 (177) copies the opponent's next card as a temporary
        // invocation. FateStrategy 416 (促局飞袭) fires on temp executions too
        // (flow_card_effect.rs hook, oracle-anchored): 火灵•赤焰's after-card
        // phase consumes all 5 layers and attacks once more. That follow-up
        // runs inside the copied invocation, so it is claimed by 灵韵御心琴's
        // AfterCard step — and must keep 火灵•赤焰's identity.
        let mut fixture = invocation_fixture(&[177], &[7_000_022], Vec::new());
        fixture.players.p1.fate_strategies = vec![416];
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let step = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::AfterCard
                    && step.card_name.as_deref() == Some("灵韵御心琴")
            })
            .expect("灵韵御心琴 AfterCard step");
        let follow_up = step
            .p2_changes
            .iter()
            .find(|change| change.key == "hp" && change.card_id == Some(7_000_022))
            .expect("copied card's after-card follow-up change");
        assert_eq!(follow_up.card_name.as_deref(), Some("火灵•赤焰"));
        assert_eq!(follow_up.before - follow_up.after, 5);
        // The AfterCard step belongs to 灵韵御心琴; no claimed change may
        // masquerade as the outer card.
        assert!(step
            .p2_changes
            .iter()
            .filter(|change| change.key == "hp")
            .all(|change| change.card_name.as_deref() != Some("灵韵御心琴")));
    }

    /// 临时牌 misattribution class: a card copied as a temporary invocation
    /// (灵韵御心琴 copying the opponent's next card) must keep the temporary
    /// card's own identity on the mutations it produces, not the origin card's.
    #[test]
    fn temporary_card_mutations_keep_the_temporary_card_identity() {
        let fixture = invocation_fixture(&[177], &[0], Vec::new());
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let step = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.card_name.as_deref() == Some("灵韵御心琴")
            })
            .expect("灵韵御心琴 MainEffect step");
        let hp = step
            .p2_changes
            .iter()
            .find(|change| change.key == "hp")
            .expect("copied attack hp change");
        assert_eq!((hp.before, hp.after), (500, 497));
        // The copy runs the opponent's 普通攻击 as a temporary invocation; the
        // damage belongs to the temporary card, not to 灵韵御心琴.
        assert_eq!(hp.card_id, Some(0));
        assert_eq!(hp.card_name.as_deref(), Some("普通攻击"));
    }

    /// 嵌套效果 misattribution class: mutations produced by a three-deep
    /// invocation chain (灵韵御心琴 → copied 狂剑•降神 → replayed 狂剑•炎舞)
    /// must each carry the identity of the card whose effect produced them,
    /// even though all of them run under the outer card's event index.
    #[test]
    fn nested_effect_mutations_keep_the_innermost_card_identity() {
        // 灵韵御心琴 copies the opponent's next card (20_186 狂剑•降神:
        // defense + def, then replays one selected 狂剑 — the tape supplies
        // 20_002 狂剑•炎舞: attack 2 + 外伤). All mutations run inside 灵韵
        // 御心琴's MainEffect window; each must keep its own card identity.
        let fixture = invocation_fixture(&[177], &[20_186], vec![20_002]);
        let trace = trace_replay_fixture_hooks(&fixture).expect("hook trace");
        let step = trace
            .steps
            .iter()
            .find(|step| {
                step.category == ReplayHookCategory::MainEffect
                    && step.card_name.as_deref() == Some("灵韵御心琴")
            })
            .expect("灵韵御心琴 MainEffect step");
        let defense = step
            .p1_changes
            .iter()
            .find(|change| change.key == "defense")
            .expect("狂剑•降神 defense change");
        assert_eq!(defense.card_name.as_deref(), Some("狂剑•降神"));
        let p2_hp = step
            .p2_changes
            .iter()
            .find(|change| change.key == "hp")
            .expect("innermost 狂剑 attack change");
        assert_eq!(p2_hp.card_name.as_deref(), Some("狂剑•炎舞"));
        assert!(p2_hp.after < p2_hp.before);
        let wound = step
            .p2_changes
            .iter()
            .find(|change| change.key == "externalInjury")
            .expect("innermost 狂剑 wound change");
        assert_eq!(wound.card_name.as_deref(), Some("狂剑•炎舞"));
    }
}
