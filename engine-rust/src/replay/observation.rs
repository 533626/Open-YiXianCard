use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDetailedRun {
    pub summary: ReplaySummary,
    pub events: Vec<ReplayDetailedEvent>,
    pub steps: Vec<ReplayDetailedStep>,
    pub decision_events: Vec<ReplayDecisionEvent>,
    pub attack_segments: Vec<ReplayAttackSegment>,
    pub turn_end_hooks: Vec<ReplayTurnEndHookReceipt>,
    pub mutation_receipts: Vec<ReplayMutationReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDetailedEvent {
    pub event: ReplayEvent,
    pub p1: Vec<ReplayDetailEntry>,
    pub p2: Vec<ReplayDetailEntry>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDetailEntry {
    pub group: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub value: i64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayHookCategory {
    BattleStart,
    TurnStart,
    SelectCost,
    TemporaryUpgrade,
    MainEffect,
    AfterCard,
    ActionAgain,
    TurnEnd,
    BattleEnd,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReplayDetailedStep {
    pub event_index: usize,
    pub category: ReplayHookCategory,
    pub turn: i64,
    pub actor: PlayerSide,
    pub slot: Option<usize>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    pub p1_snapshot: ReplayPlayerSnapshot,
    pub p2_snapshot: ReplayPlayerSnapshot,
    pub p1: Vec<ReplayDetailEntry>,
    pub p2: Vec<ReplayDetailEntry>,
    /// Number of `mutation_receipts` recorded before this step's sample, so
    /// the hook trace can slice the receipt log into per-step windows.
    pub mutation_receipt_offset: usize,
}

/// One hit of a multi-hit attack card, sampled for the hook trace.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayAttackSegment {
    /// The event index of the cardCompleted event this hit belongs to, for
    /// joining to the matching MainEffect hook step.
    pub event_index: usize,
    /// Which side was attacked (the target, not the actor).
    pub target: PlayerSide,
    /// 0-based hit index within this card's attack loop.
    pub hit_index: usize,
    pub actor_hp_before: i64,
    pub actor_hp_after: i64,
    pub hp_before: i64,
    pub hp_after: i64,
    pub def_before: i64,
    pub def_after: i64,
}

/// Which typed mutation family a trace receipt belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ReplayMutationKind {
    Hp,
    MaxHp,
    Defense,
    Momentum,
    NegativeStatus,
    Revive,
    Resource,
    Counter,
}

/// One typed mutation recorded for the observation trace (Detailed mode only).
///
/// The mutation kernel already returns a typed receipt (HpMutationReceipt,
/// MomentumMutationReceipt, NegativeStatusMutationReceipt,
/// DefenseMutationReceipt, MaxHpMutationReceipt, ReviveReceipt) at every atomic
/// mutation site; this log mirrors those receipts with the attribution context
/// active when the mutation ran (active effect invocation / phase block), so
/// the hook trace can attribute each mutation to the card or phase that caused
/// it instead of inferring attribution from state-sample windows.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayMutationReceipt {
    pub turn: i64,
    pub actor: PlayerSide,
    /// CardCompleted event index of the invocation the mutation ran under;
    /// for mutations outside any invocation this is the latest recorded event.
    pub event_index: usize,
    /// Effective card of the active invocation, if any.
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    /// Trace step category this mutation is attributed to.
    pub category: ReplayHookCategory,
    pub kind: ReplayMutationKind,
    pub group: &'static str,
    pub key: &'static str,
    pub label: &'static str,
    pub before: i64,
    pub after: i64,
    pub applied: i64,
}

impl ReplayState {
    /// Records a counter-field transition as a `Counter` receipt, skipping
    /// no-op transitions. This is the convenience wrapper over
    /// `record_mutation_receipt` for scalar counters; resource fields with
    /// their own kernel receipts (`Resource`, `Hp`, …) call
    /// `record_mutation_receipt` directly.
    pub(super) fn record_counter_transition(
        &mut self,
        actor_side: PlayerSide,
        group: &'static str,
        key: &'static str,
        label: &'static str,
        before: i64,
        after: i64,
    ) {
        if before == after {
            return;
        }
        self.record_mutation_receipt(
            actor_side,
            ReplayMutationKind::Counter,
            group,
            key,
            label,
            before,
            after,
            after - before,
        );
    }

    pub(super) fn record_event(
        &mut self,
        kind: ReplayEventKind,
        actor: PlayerSide,
        slot: Option<usize>,
        card: Option<&CardDefinition>,
    ) {
        if !self.observation.mode.emits_events() {
            return;
        }
        let event = ReplayEvent {
            turn: self.actor_turn,
            kind,
            actor,
            slot,
            card_id: card.map(|card| card.id),
            card_name: card.map(|card| card.name.clone()),
            p1: self.p1.snapshot(),
            p2: self.p2.snapshot(),
        };
        if self.observation.mode.is_ui() {
            self.observation.ui_events.push(ReplayUiEvent {
                turn: event.turn,
                kind: event.kind,
                actor: event.actor,
                slot: event.slot,
                card_id: event.card_id,
                card_name: event.card_name.clone(),
                p1: self.p1.ui_snapshot(),
                p2: self.p2.ui_snapshot(),
            });
        }
        if self.observation.mode.is_detailed() {
            self.observation.detailed_events.push(ReplayDetailedEvent {
                event: event.clone(),
                p1: self.p1.detail_entries(),
                p2: self.p2.detail_entries(),
            });
        }
        self.observation.events.push(event);
        // 与 events 严格同索引；漏推一次就会让归因错位到别的结算点。
        self.observation.prevention.push(ReplayPreventionPair {
            p1: self.p1.prevention,
            p2: self.p2.prevention,
        });
    }

    pub(super) fn record_detail_step(
        &mut self,
        event_index: usize,
        category: ReplayHookCategory,
        actor: PlayerSide,
        slot: Option<usize>,
        card: Option<&CardDefinition>,
    ) {
        if !self.observation.mode.is_detailed() {
            return;
        }
        let p1_snapshot = self.p1.snapshot();
        let p2_snapshot = self.p2.snapshot();
        let p1 = self.p1.detail_entries();
        let p2 = self.p2.detail_entries();
        self.observation.detailed_steps.push(ReplayDetailedStep {
            event_index,
            category,
            turn: self.actor_turn,
            actor,
            slot,
            card_id: card.map(|card| card.id),
            card_name: card.map(|card| card.name.clone()),
            p1_snapshot,
            p2_snapshot,
            p1: p1.clone(),
            p2: p2.clone(),
            mutation_receipt_offset: self.observation.mutation_receipts.len(),
        });
    }

    /// Mirrors one typed mutation receipt into the observation log (Detailed
    /// mode only), stamped with the attribution context active at the mutation
    /// site: the effective card of the active invocation, or the phase block
    /// when no invocation is active.
    ///
    /// Mutations outside both contexts (cost payment, card transforms such as
    /// 画龙点睛) are attributed to `SelectCost` — the explicit "no invocation,
    /// no phase block" bucket, not a fallback to hide unknown contexts. The
    /// hook trace folds `SelectCost` receipts into the step whose window
    /// contains them; any other category mismatch fails closed there.
    #[allow(clippy::too_many_arguments)]
    pub(super) fn record_mutation_receipt(
        &mut self,
        actor_side: PlayerSide,
        kind: ReplayMutationKind,
        group: &'static str,
        key: &'static str,
        label: &'static str,
        before: i64,
        after: i64,
        applied: i64,
    ) {
        if !self.observation.mode.is_detailed() {
            return;
        }
        let (card_id, card_name, category) = match self.effect_invocation_stack.last() {
            Some(frame) => {
                let category = match frame.phase {
                    effect_invocation::EffectInvocationPhase::EnterAfter
                    | effect_invocation::EffectInvocationPhase::AfterHooks
                    | effect_invocation::EffectInvocationPhase::Settlement => {
                        ReplayHookCategory::AfterCard
                    }
                    _ => ReplayHookCategory::MainEffect,
                };
                (
                    Some(frame.effective.card_id),
                    Some(frame.effective.name.clone()),
                    category,
                )
            }
            None => match self.attribution_block {
                Some(TraceAttributionBlock::BattleStart) => {
                    (None, None, ReplayHookCategory::BattleStart)
                }
                Some(TraceAttributionBlock::TurnStart) => {
                    (None, None, ReplayHookCategory::TurnStart)
                }
                Some(TraceAttributionBlock::ActionAgain) => {
                    (None, None, ReplayHookCategory::ActionAgain)
                }
                Some(TraceAttributionBlock::TurnEnd) => (None, None, ReplayHookCategory::TurnEnd),
                None => (None, None, ReplayHookCategory::SelectCost),
            },
        };
        let event_index = self
            .observation
            .current_card_event_index
            .unwrap_or_else(|| self.observation.events.len().saturating_sub(1));
        self.observation
            .mutation_receipts
            .push(ReplayMutationReceipt {
                turn: self.actor_turn,
                actor: actor_side,
                event_index,
                card_id,
                card_name,
                category,
                kind,
                group,
                key,
                label,
                before,
                after,
                applied,
            });
    }
}
