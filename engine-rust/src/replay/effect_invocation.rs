//! Runtime context for the original game's `ExecuteEffect` boundary.
//!
//! One physical card item can successively execute a transformed card, a
//! repeated effect, or recursively selected temporary cards. Keep those
//! identities separate:
//! - `physical` is the stable card item and grid slot selected by the outer
//!   transaction;
//! - `origin` is the effect that opened the current nested invocation;
//! - `effective` is the card definition whose hooks and body are running now.
//!
//! Temporary card implementations must enter through
//! `with_temporary_effect_invocation`; the stack, physical-card morph, and
//! rear-move window then unwind as one operation instead of field-by-field
//! restoration in every card implementation.

use super::support::{is_frenzy_sword_for_actor, normalized_base_id};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};
use std::cell::RefCell;
use std::rc::Rc;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectInvocationKind {
    Played,
    Repeated,
    Temporary,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum EffectInvocationPhase {
    BeforeBody,
    Body,
    EnterAfter,
    AfterHooks,
    Settlement,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemporaryHadUsedSource {
    PhysicalAtEntry,
    Explicit(bool),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TemporaryDeckIdentityMode {
    ReplaceWithEffective,
    PreservePhysical,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct TemporaryInvocationSpec {
    pub(super) physical_slot: usize,
    pub(super) invocation_slot: usize,
    pub(super) had_used_source: TemporaryHadUsedSource,
    pub(super) deck_identity_mode: TemporaryDeckIdentityMode,
    pub(super) inherit_parent_beng_quan: bool,
}

impl TemporaryInvocationSpec {
    pub(super) fn physical(slot: usize) -> Self {
        Self {
            physical_slot: slot,
            invocation_slot: slot,
            had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
            deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
            inherit_parent_beng_quan: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EffectCardIdentity {
    pub(super) card_id: i64,
    pub(super) base_id: i64,
    pub(super) name: String,
    pub(super) segment_source: String,
    pub(super) has_anima_desc: bool,
    pub(super) is_beng_quan: bool,
    pub(super) is_frenzy_sword: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct PhysicalCardIdentity {
    pub(super) actor_side: PlayerSide,
    pub(super) slot: usize,
    pub(super) card: EffectCardIdentity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EffectInvocationLocal {
    pub(super) action_again: bool,
    pub(super) after_action: bool,
    pub(super) wood_spirit_patrol_active_before_card: bool,
    /// Original `DanKaGongJiJiShu`: nested temporary effects share this cell,
    /// and every `OnAfterExecuted` clears it instead of restoring a snapshot.
    pub(super) attacks: i64,
    pub(super) shatter_defense: i64,
    /// 原版 CalculateAttack 中在百分比倍率之前对当前攻击值整体翻倍的
    /// 卡牌（base 4000087 梦•枯木逢春家族）使用的 invocation 级倍率：
    /// 覆盖星力/加攻等已在翻倍点之前累加的平值（BattleCharacter.cs
    /// 11627-11641，decompiled build-24646245）。默认 1（不翻倍）。
    pub(super) attack_multiplier: i64,
    pub(super) pending_sword_intent: i64,
    pub(super) deferred_sword_intent_restore: i64,
    pub(super) actual_damage: i64,
    pub(super) wounded_count: i64,
}

impl Default for EffectInvocationLocal {
    fn default() -> Self {
        Self {
            action_again: false,
            after_action: false,
            wood_spirit_patrol_active_before_card: false,
            attacks: 0,
            shatter_defense: 0,
            attack_multiplier: 1,
            pending_sword_intent: 0,
            deferred_sword_intent_restore: 0,
            actual_damage: 0,
            wounded_count: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EffectInvocationWindowSnapshot {
    action_again: bool,
    after_action: bool,
    wood_spirit_patrol_active_before_card: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) struct EffectInvocationFrame {
    pub(super) origin: EffectCardIdentity,
    pub(super) effective: EffectCardIdentity,
    effective_definition: CardDefinition,
    pub(super) physical: PhysicalCardIdentity,
    pub(super) invocation_slot: usize,
    pub(super) kind: EffectInvocationKind,
    pub(super) phase: EffectInvocationPhase,
    pub(super) local: Rc<RefCell<EffectInvocationLocal>>,
}

impl ReplayState {
    fn effect_card_identity(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        is_beng_quan: bool,
    ) -> EffectCardIdentity {
        let base_id = normalized_base_id(card);
        EffectCardIdentity {
            card_id: card.id,
            base_id,
            name: card.name.clone(),
            segment_source: format!("card:{base_id}"),
            has_anima_desc: super::original_config::original_card_desc_contains_anima(card),
            is_beng_quan,
            is_frenzy_sword: is_frenzy_sword_for_actor(self.actor(actor_side), card),
        }
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn begin_effect_invocation(
        &mut self,
        actor_side: PlayerSide,
        origin_card: &CardDefinition,
        effective_card: &CardDefinition,
        physical_card: &CardDefinition,
        physical_slot: usize,
        invocation_slot: usize,
        kind: EffectInvocationKind,
        effective_is_beng_quan: bool,
    ) {
        let origin_is_beng_quan = if origin_card.id == effective_card.id {
            effective_is_beng_quan
        } else {
            self.is_dream_mirage_intrinsic_beng_quan(actor_side, origin_card)
        };
        let physical_is_beng_quan = if physical_card.id == effective_card.id {
            effective_is_beng_quan
        } else {
            self.is_dream_mirage_intrinsic_beng_quan(actor_side, physical_card)
        };
        let frame = EffectInvocationFrame {
            origin: self.effect_card_identity(actor_side, origin_card, origin_is_beng_quan),
            effective: self.effect_card_identity(
                actor_side,
                effective_card,
                effective_is_beng_quan,
            ),
            effective_definition: effective_card.clone(),
            physical: PhysicalCardIdentity {
                actor_side,
                slot: physical_slot,
                card: self.effect_card_identity(actor_side, physical_card, physical_is_beng_quan),
            },
            invocation_slot,
            kind,
            phase: EffectInvocationPhase::BeforeBody,
            local: Rc::new(RefCell::new(EffectInvocationLocal::default())),
        };
        self.effect_invocation_stack.push(frame);
    }

    fn begin_temporary_effect_invocation(
        &mut self,
        actor_side: PlayerSide,
        selected: &CardDefinition,
        spec: TemporaryInvocationSpec,
    ) -> Option<(CardDefinition, bool, Option<EffectInvocationWindowSnapshot>)> {
        let physical_slot = spec.physical_slot;
        let invocation_slot = spec.invocation_slot;
        let physical_card = self
            .actor(actor_side)
            .deck
            .slots
            .get(physical_slot)
            .map(|slot| slot.card.clone())?;
        let parent_frame = self.effect_invocation_stack.last();
        let restore_card = parent_frame
            .filter(|frame| {
                frame.physical.actor_side == actor_side && frame.physical.slot == physical_slot
            })
            .map(|frame| frame.effective_definition.clone())
            .unwrap_or_else(|| physical_card.clone());
        let parent_window = parent_frame.map(|frame| {
            let local = frame.local.borrow();
            EffectInvocationWindowSnapshot {
                action_again: local.action_again,
                after_action: local.after_action,
                wood_spirit_patrol_active_before_card: local.wood_spirit_patrol_active_before_card,
            }
        });
        let local = parent_frame
            .map(|frame| Rc::clone(&frame.local))
            .unwrap_or_else(|| Rc::new(RefCell::new(EffectInvocationLocal::default())));
        let origin = parent_frame
            .map(|frame| frame.effective.clone())
            .unwrap_or_else(|| {
                let intrinsic =
                    self.is_dream_mirage_intrinsic_beng_quan(actor_side, &physical_card);
                self.effect_card_identity(actor_side, &physical_card, intrinsic)
            });
        let physical = parent_frame
            .filter(|frame| {
                frame.physical.actor_side == actor_side && frame.physical.slot == physical_slot
            })
            .map(|frame| frame.physical.clone())
            .unwrap_or_else(|| PhysicalCardIdentity {
                actor_side,
                slot: physical_slot,
                card: self.effect_card_identity(
                    actor_side,
                    &physical_card,
                    self.is_dream_mirage_intrinsic_beng_quan(actor_side, &physical_card),
                ),
            });
        let parent_is_beng_quan = self.active_effect_is_beng_quan();
        let effective_is_beng_quan = (spec.inherit_parent_beng_quan && parent_is_beng_quan)
            || self.is_dream_mirage_intrinsic_beng_quan(actor_side, selected);
        let original_rear_move = self.actor(actor_side).fate.rear_move_succeeded;

        if spec.deck_identity_mode == TemporaryDeckIdentityMode::ReplaceWithEffective {
            self.actor_mut(actor_side).deck.slots[physical_slot].card = selected.clone();
        }
        self.actor_mut(actor_side).fate.rear_move_succeeded = false;

        let frame = EffectInvocationFrame {
            origin,
            effective: self.effect_card_identity(actor_side, selected, effective_is_beng_quan),
            effective_definition: selected.clone(),
            physical,
            invocation_slot,
            kind: EffectInvocationKind::Temporary,
            phase: EffectInvocationPhase::BeforeBody,
            local: Rc::clone(&local),
        };
        self.effect_invocation_stack.push(frame);
        if parent_window.is_some() {
            let mut local = local.borrow_mut();
            local.action_again = false;
            local.after_action = false;
            local.wood_spirit_patrol_active_before_card = false;
        }
        Some((restore_card, original_rear_move, parent_window))
    }

    pub(super) fn with_temporary_effect_invocation<R>(
        &mut self,
        actor_side: PlayerSide,
        selected: &CardDefinition,
        spec: TemporaryInvocationSpec,
        run: impl FnOnce(&mut Self, bool) -> R,
    ) -> Option<R> {
        let physical_was_used = self
            .actor(actor_side)
            .deck
            .slots
            .get(spec.physical_slot)
            .is_some_and(|slot_state| slot_state.used);
        let was_used_before_effect = match spec.had_used_source {
            TemporaryHadUsedSource::PhysicalAtEntry => physical_was_used,
            TemporaryHadUsedSource::Explicit(value) => value,
        };
        self.require_card_effect_before_execution(
            actor_side,
            selected,
            spec.invocation_slot,
            was_used_before_effect,
            false,
        )?;
        if let TemporaryHadUsedSource::Explicit(value) = spec.had_used_source {
            if let Some(slot_state) = self
                .actor_mut(actor_side)
                .deck
                .slots
                .get_mut(spec.physical_slot)
            {
                slot_state.used = value;
            }
        }
        let original_depth = self.effect_invocation_stack.len();
        let (physical_card, original_rear_move, parent_window) =
            self.begin_temporary_effect_invocation(actor_side, selected, spec)?;
        let result = run(self, was_used_before_effect);
        debug_assert_eq!(self.effect_invocation_stack.len(), original_depth + 1);
        let completed = self
            .effect_invocation_stack
            .pop()
            .expect("temporary effect invocation frame must remain active");
        debug_assert_eq!(completed.kind, EffectInvocationKind::Temporary);
        debug_assert_eq!(completed.physical.actor_side, actor_side);
        debug_assert_eq!(completed.physical.slot, spec.physical_slot);
        debug_assert!(!completed.physical.card.segment_source.is_empty());
        debug_assert_eq!(completed.invocation_slot, spec.invocation_slot);

        self.actor_mut(actor_side).deck.slots[spec.physical_slot].card = physical_card;
        self.actor_mut(actor_side).fate.rear_move_succeeded = original_rear_move;
        if let Some(snapshot) = parent_window {
            let mut local = completed.local.borrow_mut();
            local.action_again = snapshot.action_again;
            local.after_action = snapshot.after_action;
            local.wood_spirit_patrol_active_before_card =
                snapshot.wood_spirit_patrol_active_before_card;
        }
        // 移花接木/后招等临时 invocation 在引擎里也代表一次完整出牌流程，
        // 出牌完成时同样 flush 302→644（原版 OnAfterExecuted 4743-4745）。
        self.flush_actual_damage_carry(actor_side);
        Some(result)
    }

    pub(super) fn end_effect_invocation(
        &mut self,
        actor_side: PlayerSide,
        expected_kind: EffectInvocationKind,
    ) {
        let completed = self
            .effect_invocation_stack
            .pop()
            .expect("effect invocation frame must remain active");
        debug_assert_eq!(completed.kind, expected_kind);
        debug_assert_eq!(completed.physical.actor_side, actor_side);
        debug_assert!(!completed.origin.segment_source.is_empty());
        // 原版 OnAfterExecuted（CardActionBase.cs:4743-4745）：每次出牌执行
        // 完成时把攻击者身上持久累计的 ActualDamage(302) 转入
        // JiLuZongJiShangZhi(644)，并清空 302 与 WoundedCount(303)。引擎的
        // 每次 effect invocation（主牌、重复源、测试直调）都对应一次完整
        // 出牌流程，故在此 flush。
        self.flush_actual_damage_carry(actor_side);
    }

    /// 302 → 644 转移并清零 302/303（CardActionBase.cs:4743-4745）。
    pub(super) fn flush_actual_damage_carry(&mut self, actor_side: PlayerSide) {
        let total_before = self.actor(actor_side).turn.ji_lu_zong_ji_shang_zhi;
        let damage_before = self.actor(actor_side).turn.actual_damage_carry;
        let wounded_before = self.actor(actor_side).turn.wounded_count_carry;
        {
            let turn = &mut self.actor_mut(actor_side).turn;
            turn.ji_lu_zong_ji_shang_zhi += turn.actual_damage_carry;
            turn.actual_damage_carry = 0;
            turn.wounded_count_carry = 0;
        }
        self.record_counter_transition(
            actor_side,
            "回合",
            "jiLuZongJiShangZhi",
            "累计总击伤",
            total_before,
            self.actor(actor_side).turn.ji_lu_zong_ji_shang_zhi,
        );
        self.record_counter_transition(
            actor_side,
            "回合",
            "actualDamageCarry",
            "实际伤害",
            damage_before,
            0,
        );
        self.record_counter_transition(
            actor_side,
            "回合",
            "woundedCountCarry",
            "击伤计数",
            wounded_before,
            0,
        );
    }

    pub(super) fn active_effect_name(&self) -> &str {
        self.effect_invocation_stack
            .last()
            .map_or("", |frame| frame.effective.name.as_str())
    }

    pub(super) fn active_effect_segment_source(&self) -> &str {
        self.effect_invocation_stack
            .last()
            .map_or("", |frame| frame.effective.segment_source.as_str())
    }

    pub(super) fn active_effect_has_anima_desc(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.effective.has_anima_desc)
    }

    pub(super) fn active_effect_after_action(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.local.borrow().after_action)
    }

    pub(super) fn set_active_effect_after_action(&mut self, active: bool) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().after_action = active;
        }
    }

    pub(super) fn active_effect_action_again(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.local.borrow().action_again)
    }

    pub(super) fn set_active_effect_action_again(&mut self, active: bool) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().action_again = active;
        }
    }

    pub(super) fn active_effect_base_id(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.effective.base_id)
    }

    pub(super) fn active_effect_is_beng_quan(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.effective.is_beng_quan)
    }

    pub(super) fn active_effect_is_frenzy_sword(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.effective.is_frenzy_sword)
    }

    pub(super) fn active_effect_wood_spirit_patrol_before_card(&self) -> bool {
        self.effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.local.borrow().wood_spirit_patrol_active_before_card)
    }

    pub(super) fn set_active_effect_wood_spirit_patrol_before_card(&mut self, active: bool) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame
                .local
                .borrow_mut()
                .wood_spirit_patrol_active_before_card = active;
        }
    }

    pub(super) fn active_effect_attacks(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.local.borrow().attacks)
    }

    pub(super) fn add_active_effect_attacks(&mut self, count: i64) {
        if count <= 0 {
            return;
        }
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().attacks += count;
        }
    }

    pub(super) fn clear_active_effect_attacks(&mut self) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().attacks = 0;
        }
    }

    pub(super) fn active_effect_shatter_defense(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.local.borrow().shatter_defense)
    }

    pub(super) fn set_active_effect_shatter_defense(&mut self, amount: i64) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().shatter_defense = amount.max(0);
        }
    }

    pub(super) fn gain_active_effect_shatter_defense(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().shatter_defense += amount;
        }
    }

    pub(super) fn active_effect_attack_multiplier(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(1, |frame| frame.local.borrow().attack_multiplier)
    }

    pub(super) fn set_active_effect_attack_multiplier(&mut self, amount: i64) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().attack_multiplier = amount.max(1);
        }
    }

    pub(super) fn active_effect_pending_sword_intent(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.local.borrow().pending_sword_intent)
    }

    pub(super) fn set_active_effect_pending_sword_intent(&mut self, amount: i64) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().pending_sword_intent = amount.max(0);
        }
    }

    pub(super) fn update_active_effect_pending_sword_intent(&mut self, amount: i64) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            let mut local = frame.local.borrow_mut();
            local.pending_sword_intent = local.pending_sword_intent.max(amount.max(0));
        }
    }

    pub(super) fn active_effect_deferred_sword_intent_restore(&self) -> i64 {
        self.effect_invocation_stack.last().map_or(0, |frame| {
            frame.local.borrow().deferred_sword_intent_restore
        })
    }

    pub(super) fn add_active_effect_deferred_sword_intent_restore(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().deferred_sword_intent_restore += amount;
        }
    }

    pub(super) fn active_effect_actual_damage(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.local.borrow().actual_damage)
    }

    pub(super) fn add_active_effect_actual_damage(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().actual_damage += amount;
        }
    }

    pub(super) fn active_effect_wounded_count(&self) -> i64 {
        self.effect_invocation_stack
            .last()
            .map_or(0, |frame| frame.local.borrow().wounded_count)
    }

    pub(super) fn add_active_effect_wounded_count(&mut self, amount: i64) {
        if amount <= 0 {
            return;
        }
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.local.borrow_mut().wounded_count += amount;
        }
    }

    pub(super) fn clear_active_effect_settlement_local(&mut self) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            let mut local = frame.local.borrow_mut();
            local.shatter_defense = 0;
            local.attack_multiplier = 1;
            local.pending_sword_intent = 0;
            local.deferred_sword_intent_restore = 0;
            local.actual_damage = 0;
            local.wounded_count = 0;
        }
    }

    pub(super) fn set_active_effect_phase(&mut self, phase: EffectInvocationPhase) {
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.phase = phase;
        }
    }

    #[cfg(test)]
    pub(super) fn active_effect_frame(&self) -> Option<&EffectInvocationFrame> {
        self.effect_invocation_stack.last()
    }
}
