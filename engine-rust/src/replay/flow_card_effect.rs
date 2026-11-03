use super::card_effect_catalog::{resolve_card_effect, CardEffectResolution};
#[cfg(test)]
use super::effect_invocation::EffectInvocationKind;
use super::effect_invocation::{EffectInvocationPhase, TemporaryInvocationSpec};
use super::support::{
    has_cloud_chain, is_cloud_sword, is_frenzy_sword_for_actor, is_spirit_sword_for_actor,
    is_sword_formation_card, normalized_base_id, opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

/// Non-negative card parameter shorthands: `paramN(card)` is exactly
/// `other_param(card, N).max(0)`, always inlined so generated code is
/// unchanged. Reads with other clamps, defaults, or unclamped values stay
/// explicit at their call sites.
#[inline(always)]
pub(super) fn param0(card: &CardDefinition) -> i64 {
    other_param(card, 0).max(0)
}

/// See [`param0`]: `other_param(card, 1)` clamped at zero.
#[inline(always)]
pub(super) fn param1(card: &CardDefinition) -> i64 {
    other_param(card, 1).max(0)
}

/// See [`param0`]: `other_param(card, 2)` clamped at zero.
#[inline(always)]
pub(super) fn param2(card: &CardDefinition) -> i64 {
    other_param(card, 2).max(0)
}

/// See [`param0`]: `other_param(card, 3)` clamped at zero.
#[inline(always)]
pub(super) fn param3(card: &CardDefinition) -> i64 {
    other_param(card, 3).max(0)
}

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    /// Resolve a card against the explicit catalog before any real execution
    /// state is opened. This is a pure capability lookup: it cannot mutate
    /// resources, invocation state, decision cursors, or RNG.
    pub(super) fn resolve_card_effect_before_execution(
        &self,
        _actor_side: PlayerSide,
        card: &CardDefinition,
        _slot: usize,
        _was_used_before_effect: bool,
    ) -> CardEffectResolution {
        let base_id = normalized_base_id(card);
        resolve_card_effect(card, base_id)
    }

    pub(super) fn require_card_effect_before_execution(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        allow_record_only: bool,
    ) -> Option<CardEffectResolution> {
        let base_id = normalized_base_id(card);
        let resolution = self.resolve_card_effect_before_execution(
            actor_side,
            card,
            slot,
            was_used_before_effect,
        );
        match resolution {
            CardEffectResolution::Missing => {
                self.missing_card_effect(card.id, base_id, "missing executable behavior");
                None
            }
            // 炼化/置换类（Refine/Change）牌被复制成临时效果时，原版走 FallbackCardAction，仅印面字段（本类无）
            // → 无战斗效果；oracle pa_4e088473 在飞枭灵芝（9020010）被复制后与 HF 终局一致。
            CardEffectResolution::RecordOnly
                if !allow_record_only
                    && !card.card_type.as_ref().is_some_and(|card_type| {
                        card_type.name.eq_ignore_ascii_case("refine")
                            || card_type.name.eq_ignore_ascii_case("change")
                    }) =>
            {
                self.missing_card_effect(
                    card.id,
                    base_id,
                    "record-only card cannot execute as a temporary effect",
                );
                None
            }
            _ => Some(resolution),
        }
    }

    pub(super) fn apply_before_execute_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        is_temporary: bool,
    ) {
        self.refresh_active_mechanisms();

        #[cfg(all(feature = "perf-ceiling", not(target_arch = "wasm32")))]
        let _timer = crate::performance::Timer::new(3);
        // OnBeforeExecuted first opens temporary sword-identity windows, then
        // runs its private adjacent-card helper. The counter-element branch
        // occurs later in that same method, after the shared selected hooks.
        if self.active_mechanisms.has(super::ActiveMechanisms::SWORD_SECT) {
            self.start_dream_mirage_selected_card(actor_side);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.apply_dream_mirage_before_card_hooks(actor_side, card, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY) {
            self.apply_qi_xing_lian_zhu(actor_side, card, slot);
        }
        // 伏魔 13017 数负面前于星弈连转内伤（CardActionBase
        // .OnBeforeExecuted :3300 先于 :3676），与 OnPlayCard 其余
        // 附魔的相对顺序见 apply_verified_xian_mo_on_play_card。
        if self.active_mechanisms.has(super::ActiveMechanisms::XIAN_MO)
            && !is_temporary
            && self
                .actor(actor_side)
                .deck
                .slots
                .get(slot)
                .is_some_and(|s| !s.used)
        {
            self.apply_fumo_13017_before_star_hooks(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::ASTROLOGY)
            || self.actor(actor_side).fate.xing_yi_lian > 0
        {
            self.apply_xing_yi_lian(actor_side, card, slot);
        }
        self.apply_selected_card_hooks_after_start(actor_side, card, slot, is_temporary);
        // FateStrategy 380 御云结阵 is an OnPlayCard hook: a sword-formation
        // card first adds one 云海 marker.  The shared post-body cloud-chain
        // path then consumes that marker and applies its +2 defense bonus.
        if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY)
            && (self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&380))
            && is_sword_formation_card(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.cloud_sea += 1;
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::XIAN_MO) {
            self.apply_verified_xian_mo_on_play_card(actor_side, card, slot, is_temporary);
        }
        // OnBeforeExecuted :3319 cloud-sword hooks follow XianMo.OnPlayCard.
        if self.active_mechanisms.has(super::ActiveMechanisms::SWORD_SECT)
            && is_cloud_sword(self.actor(actor_side), card)
        {
            let healing = self.actor(actor_side).sword.cloud_sword_soft_heart;
            if healing > 0 {
                self.modify_actor_hp(actor_side, healing, false, false);
            }
            if self.actor(actor_side).identity.talents.contains(&15) {
                self.gain_anima(actor_side, 1);
            }
            if self.actor(actor_side).sword.cloud_sword_heart > 0 {
                self.actor_mut(actor_side).sword.cloud_sword_heart -= 1;
                self.modify_extra_actions(actor_side, 1);
            }
        }
        // OnBeforeExecuted: XianMo.OnPlayCard (:3300) precedes 崩灭心法 (:3369).
        // Damage from the former can consume defense-loss hooks before QiShi overflows.
        if self.active_mechanisms.has(super::ActiveMechanisms::BENG_QUAN)
            && !is_temporary
            && card.hp_cost.unwrap_or(0) > 0
        {
            let momentum = self.actor(actor_side).beng.beng_mei_mindset;
            if momentum > 0 {
                self.modify_momentum(actor_side, momentum);
            }
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::FORMATION | super::ActiveMechanisms::ELEMENT_FLOW) {
            self.apply_muling_healing_formation_on_play_card(actor_side, card);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::RONGHUI) {
            self.apply_tian_sui_ni_ke_jue_before_card(actor_side, card);
        }
        self.apply_selected_card_talent_loop_hooks(actor_side, card, slot, is_temporary);
        self.apply_next_card_action_again_conversion(actor_side);
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE | super::ActiveMechanisms::RONGHUI) {
            self.apply_counter_element_before_card(actor_side, card, slot);
        }
        // 26125017 moved 惊雷 (KeYinJingLei) from OnBeforeExecuted to
        // BattleCharacter.CalculateAttack: it now triggers per attack segment
        // (see apply_jing_lei_on_attack_segment), not before the card.
        // 星缘 FateStrategy 402（CardActionBase.OnBeforeExecuted IL_245a）
        // 必须在牌体执行前检查星位；例如天元心法会在牌体内把全槽变星，
        // 但原版不会因此把该张牌自身追认为星位牌。
        if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY) {
            self.apply_xing_yuan_before_card_hook(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::ELEMENT_FLOW)
            && self.actor(actor_side).elements.wood_scattered_branches > 0
        {
            // CardActionBase.OnBeforeExecuted MuLingSanZhiYe (BuffType 777):
            // reads base Card_7000112.otherParams[1] (1), not played rarity.
            let amount = 1 + self.actor(actor_side).core.attack_bonus;
            self.modify_actor_hp(opponent_side(actor_side), -amount, false, false);
            self.modify_actor_hp(actor_side, amount, false, false);
            self.actor_mut(actor_side).elements.wood_scattered_branches -= 1;
        }
        self.clear_actual_damage_before_execute(actor_side);
    }

    /// CardActionBase.OnBeforeExecuted 末尾（:3221-3222，刻印出牌前
    /// 钩子之后、牌体之前）直接 RemoveBuff ActualDamage(302)/WoundedCount(303)：
    /// 回合末攻击或执行前钩子留下的残留不再被本张牌读到，也不转入 644
    /// （OnAfterExecuted 的 302→644 转移不变，见 `flush_actual_damage_carry`）。
    fn clear_actual_damage_before_execute(&mut self, actor_side: PlayerSide) {
        let damage_before = self.actor(actor_side).turn.actual_damage_carry;
        let wounded_before = self.actor(actor_side).turn.wounded_count_carry;
        {
            let turn = &mut self.actor_mut(actor_side).turn;
            turn.actual_damage_carry = 0;
            turn.wounded_count_carry = 0;
        }
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

    /// 惊雷破敌 FateStrategy 396（KeYinJingLei 574）：26125017 起原版移到
    /// BattleCharacter.CalculateAttack（BattleCharacter.cs 新增分支），不再经过
    /// KeYinCardFunctions.OnBeforeExecuted。每次攻击段入口检查当前执行牌名含
    /// 「雷」即触发（不再要求 attack>0/五雷轰顶——能进攻击段即等价），清空全部
    /// 层数，给本次牌体攻击碎防一段，并令目标 WaiShang(105) 增加捕获层数。
    pub(super) fn apply_jing_lei_on_attack_segment(&mut self, actor_side: PlayerSide) {
        let stacks = self.actor(actor_side).fate.ke_yin_jing_lei.max(0);
        let is_lei_card = self
            .effect_invocation_stack
            .last()
            .is_some_and(|frame| frame.effective.name.contains('雷'));
        if stacks <= 0 || !is_lei_card {
            return;
        }
        // The source calls RemoveBuff, so all accumulated layers are consumed
        // at once; WaiShang receives that captured stack count.
        self.actor_mut(actor_side).fate.ke_yin_jing_lei = 0;
        // 原版 SuiFang(340) 是整张卡行动期持续的碎防，仅在
        // AfterCardAction 入口移除（CardActionBase.cs:3738-3742）；一次性
        // 碎防是独立的 buff 341（XiaCiGongJiSuiFang，首段攻击后消耗）。
        // 因此发放到卡行动级碎防通道（effect invocation 帧），而不是按
        // 攻击段消耗的 status 层。oracle 锚点：mirror-32219000
        // pa_256e79d9（五雷轰顶 5 段全部要吃碎防）。
        self.gain_active_effect_shatter_defense(1);
        // 原版走 dst.ModifyBuffValue(WaiShang, ...)：会经过共享的负面状态
        // 增益管线，包括 星蚀（talent 103 系 BeiXingShi 317，
        // BattleCharacter.cs:8464-8467 —— 对方首次获得负面状态时额外加层）。
        // oracle 锚点：mirror-32299000 pa_320e5c54 cp5
        // （p2 外伤 3 = 惊雷 1 + 星蚀 2；引擎原 1，直接走 add_external_injury
        // 绕过了星蚀钩子）。
        self.add_actor_negative_status(opponent_side(actor_side), 105, stacks);
    }

    #[cfg(test)]
    pub(super) fn apply_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
    ) {
        if self
            .require_card_effect_before_execution(
                actor_side,
                card,
                slot,
                was_used_before_effect,
                true,
            )
            .is_none()
        {
            return;
        }
        // Unit tests exercise a card body directly, outside the full turn
        // transaction. Still give that execution the same invocation-local
        // semantics as production so current-effect resources cannot leak
        // back into player/turn state merely for test convenience.
        let opened_test_invocation = self.effect_invocation_stack.is_empty();
        if opened_test_invocation {
            let physical_card = self.actor(actor_side).deck.slots.get(slot).map_or_else(
                || card.clone(),
                |slot_state| slot_state.card.definition().clone(),
            );
            let is_beng_quan = self.is_dream_mirage_intrinsic_beng_quan(actor_side, card);
            self.begin_effect_invocation(
                actor_side,
                card,
                card,
                &physical_card,
                slot,
                slot,
                EffectInvocationKind::Played,
                is_beng_quan,
            );
        }
        self.set_active_effect_phase(EffectInvocationPhase::Body);
        self.apply_card_effect_body(actor_side, card, slot, was_used_before_effect);
        self.apply_regular_after_card_effect_hooks(actor_side, card, slot, false);
        self.apply_card_classification_completed_hooks(actor_side, card);
        self.set_active_effect_phase(EffectInvocationPhase::Settlement);
        self.settle_sword_intent_after_card_effect(actor_side);
        self.record_last_element(actor_side, card);
        self.set_active_effect_after_action(false);
        if opened_test_invocation {
            self.end_effect_invocation(actor_side, EffectInvocationKind::Played);
        }
    }

    pub(super) fn apply_card_effect_body(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
    ) {
        // CardItem.hadUsed belongs to this frame, not the shared temporary
        // counters cell: nested copied cards have their own use history.
        if let Some(frame) = self.effect_invocation_stack.last_mut() {
            frame.was_used_before_effect = was_used_before_effect;
        }
        let resolution =
            self.apply_card_effect_body_inner(actor_side, card, slot, was_used_before_effect, true);
        if resolution.executes_printed_follow_ups() {
            self.apply_regular_printed_card_effects(actor_side, card);
        }
    }

    pub(super) fn apply_temporary_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        self.apply_temporary_card_effect_with_spec(
            actor_side,
            card,
            TemporaryInvocationSpec::physical(slot),
        )
    }

    pub(super) fn apply_temporary_card_effect_with_spec(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        spec: TemporaryInvocationSpec,
    ) -> bool {
        let cloud_chain_before_effect = has_cloud_chain(self.actor(actor_side));
        self.with_temporary_effect_invocation(
            actor_side,
            card,
            spec,
            |state, was_used_before_effect| {
                state.apply_before_execute_effect_hooks(
                    actor_side,
                    card,
                    spec.invocation_slot,
                    true,
                );
                state.set_active_effect_phase(EffectInvocationPhase::Body);
                state.apply_dream_mirage_before_effect_hooks(actor_side, card);
                state.apply_card_effect_body(
                    actor_side,
                    card,
                    spec.invocation_slot,
                    was_used_before_effect,
                );
                // Dynamic action-again must be frozen after the card body but before
                // completed-card counters (notably QinShiPai) can satisfy the card's
                // own predicate.
                let action_again = card.action_again.unwrap_or(false)
                    || state.card_zero_xiaoyao_guqin_action_again(actor_side, card)
                    || state.resolve_card_action_again(
                        actor_side,
                        card,
                        spec.invocation_slot,
                        was_used_before_effect,
                        cloud_chain_before_effect,
                    );
                state.apply_regular_after_card_effect_hooks(
                    actor_side,
                    card,
                    spec.invocation_slot,
                    true,
                );
                state.set_active_effect_phase(EffectInvocationPhase::Settlement);
                state.apply_card_completed_hooks(actor_side, card, spec.invocation_slot);
                // CardActionBase.OnAfterExecuted's fate 387 branch is
                // unconditional and precedes the isTempCard-only branches.
                if card.hp_cost.unwrap_or(0) > 0 {
                    state.actor_mut(actor_side).turn.hp_cost_cards_used += 1;
                }
                state.settle_sword_intent_after_card_effect(actor_side);
                if let Some(slot_state) = state
                    .actor_mut(actor_side)
                    .deck
                    .slots
                    .get_mut(spec.physical_slot)
                {
                    slot_state.used = true;
                }
                state.record_last_element(actor_side, card);
                if card
                    .card_type
                    .as_ref()
                    .map_or(0, |card_type| card_type.value)
                    == super::CARD_TYPE_SUSTAIN
                {
                    state
                        .actor_mut(actor_side)
                        .formations
                        .array_echo_persistent_card += 1;
                }
                let used_before = state.actor(actor_side).turn.used_card_count;
                state.actor_mut(actor_side).turn.used_card_count += 1;
                let used_after = state.actor(actor_side).turn.used_card_count;
                state.record_counter_transition(
                    actor_side,
                    "回合",
                    "usedCardCount",
                    "已用牌数",
                    used_before,
                    used_after,
                );
                state.clear_rear_move_check(actor_side);
                action_again
            },
        )
        .unwrap_or(false)
    }

    fn apply_card_effect_body_inner(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        include_inherited_effects: bool,
    ) -> CardEffectResolution {
        let previous_card_execution = self.begin_card_execution(actor_side, card.id);
        // 迅身掌 FateStrategy 427（otherParams=[2]）：每次使用名字含「掌」的
        // 牌时加 2 身法。FateStrategyFunctions.OnPlayCard
        // (FateStrategyFunctions.cs:791-794)。oracle 锚点：mirror-32219000
        // pa_34a231f6 checkpoint[5] 万玄破魔掌后 p2 身法 6
        // （=14 身法 -10 再次行动消耗 +2），引擎原为 4；9874c3ab697ed8ec/
        // round-10 checkpoint[11] 迎风掌后同样 +2。
        if (self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&427))
            && card.name.contains('掌')
        {
            self.gain_agility(actor_side, 2);
        }
        let base_id = normalized_base_id(card);
        if include_inherited_effects && self.is_dream_mirage_beng_quan(actor_side, card, slot) {
            self.apply_beng_quan_fu_hu_before_attack(actor_side, card, false);
            let inherited_defense = self.actor(actor_side).beng.beng_quan_defense;
            if inherited_defense > 0 {
                self.gain_defense(actor_side, inherited_defense);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_defense = 0;
                }
            }
            let inherited_chan = self.actor(actor_side).beng.beng_quan_chan;
            if inherited_chan > 0 {
                self.add_actor_negative_status(opponent_side(actor_side), 105, inherited_chan);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_chan = 0;
                }
            }
            let inherited_meridian = self.actor(actor_side).beng.beng_quan_meridian;
            if inherited_meridian > 0 {
                self.transfer_selected_negative_statuses(actor_side, inherited_meridian);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_meridian = 0;
                }
            }
            let inherited_han = self.actor(actor_side).beng.beng_quan_han;
            if inherited_han > 0 {
                self.modify_momentum(actor_side, inherited_han);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_han = 0;
                }
            }
            let inherited_tu = self.actor(actor_side).beng.beng_quan_tu;
            if inherited_tu > 0 {
                self.gain_active_effect_shatter_defense(inherited_tu);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_tu = 0;
                }
            }
            let inherited_fei_zhou = self.actor(actor_side).beng.beng_quan_fei_zhou;
            if inherited_fei_zhou > 0 {
                self.add_actor_negative_status(opponent_side(actor_side), 102, inherited_fei_zhou);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_fei_zhou = 0;
                }
            }
            let inherited_flash = self.actor(actor_side).beng.beng_quan_flash_agility;
            if inherited_flash > 0 {
                self.gain_agility(actor_side, inherited_flash);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_flash_agility = 0;
                }
            }
            if self.actor(actor_side).beng.beng_quan_cun_jin > 0 {
                let actor = self.actor_mut(actor_side);
                if actor.beng.momentum_multiplier < 2 {
                    actor.beng.momentum_multiplier = 2;
                }
                if base_id != 10_000_035 {
                    actor.beng.beng_quan_cun_jin -= 1;
                }
            }
            let startled_touch = self.actor(actor_side).beng.beng_quan_startled_touch;
            if startled_touch > 0 {
                self.actor_mut(actor_side).beng.triggered_startled_touch += startled_touch;
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_startled_touch = 0;
                }
            }
            let dream_chain = self.actor(actor_side).beng.dream_beng_quan_chain;
            if dream_chain > 0 {
                self.actor_mut(actor_side)
                    .beng
                    .triggered_dream_beng_quan_chain += dream_chain;
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.dream_beng_quan_chain = 0;
                }
            }
        }

        let resolution = self
            .apply_typed_card_effect_body(actor_side, card, slot, was_used_before_effect, base_id)
            .map_or_else(
                || {
                    self.apply_card_effect_fallback(
                        actor_side,
                        card,
                        slot,
                        was_used_before_effect,
                        base_id,
                    )
                },
                |_| CardEffectResolution::Executable,
            );

        if base_id == 10_000_002 {
            self.actor_mut(actor_side).beng.beng_quan_defense += card.defense.unwrap_or(0).max(0);
        }
        self.finish_card_execution(previous_card_execution);
        resolution
    }

    #[cfg(test)]
    pub(super) fn probe_has_typed_card_effect(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        let mut probe = self.clone();
        // Test-only drift audit. Never share invocation-local Rc writes with
        // the source state, and never use this path for runtime resolution.
        probe.effect_invocation_stack.clear();
        probe.evaluation_error = None;
        let physical_card = probe.actor(actor_side).deck.slots.get(slot).map_or_else(
            || card.clone(),
            |slot_state| slot_state.card.definition().clone(),
        );
        let is_beng_quan = probe.is_dream_mirage_intrinsic_beng_quan(actor_side, card);
        probe.begin_effect_invocation(
            actor_side,
            card,
            card,
            &physical_card,
            slot,
            slot,
            EffectInvocationKind::Played,
            is_beng_quan,
        );
        probe.set_active_effect_phase(EffectInvocationPhase::Body);
        let previous_card_execution = probe.begin_card_execution(actor_side, card.id);
        let typed = probe
            .apply_typed_card_effect_body(actor_side, card, slot, false, normalized_base_id(card))
            .is_some();
        probe.finish_card_execution(previous_card_execution);
        typed
    }

    fn apply_regular_printed_card_effects(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let base_id = normalized_base_id(card);
        if let Some(sword_intent) = card.sword_intent.filter(|value| {
            *value > 0 && !matches!(base_id, 47 | 183 | 309 | 1_000_050 | 1_000_107)
        }) {
            self.modify_sword_intent(actor_side, sword_intent);
        }

        let Some(hexagram) = card.hexagram.filter(|value| *value > 0) else {
            return;
        };
        if !matches!(
            base_id,
            7 | 22
                | 51
                | 4_000_001
                | 4_000_002
                | 4_000_003
                | 4_000_015
                | 4_000_016
                | 4_000_025
                | 4_000_026
                | 4_000_034
                | 4_000_064
                | 4_000_086
                | 4_000_088
                | 4_000_099
                | 4_000_109
                | 307
                | 407 // 雷闪二度：卦象由 Card_407.cs 专属牌体发放
        ) {
            self.gain_hexagram(actor_side, hexagram);
        }
    }

    pub(super) fn apply_regular_after_card_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        is_temporary: bool,
    ) {
        self.refresh_active_mechanisms();

        #[cfg(all(feature = "perf-ceiling", not(target_arch = "wasm32")))]
        let _timer = crate::performance::Timer::new(4);
        self.set_active_effect_phase(EffectInvocationPhase::EnterAfter);
        self.apply_after_card_effect_early_cleanup(actor_side, card);
        // 万世如意转灵（387/7071 + id 19）：原版 OnAfterExecuted :4354-4365，
        // 紧随清理块、先于全部 after 钩子。梦•相邻被动等采样的是转后灵气。
        if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY | super::ActiveMechanisms::XIAN_MO) {
            self.convert_wan_shi_ru_yi_anima_to_sword_energy(actor_side, card);
        }
        self.set_active_effect_after_action(true);
        self.set_active_effect_phase(EffectInvocationPhase::AfterHooks);
        if !is_temporary {
            self.apply_ordinary_only_after_card_effect_hooks(actor_side, card, slot);
            if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE | super::ActiveMechanisms::RONGHUI) {
                self.complete_mirage_ronghui_anima_attack_card(actor_side);
            }
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::XIAN_MO) {
            self.apply_verified_xian_mo_after_card(actor_side, slot);
        }
        self.apply_common_after_card_effect_hooks(actor_side, slot);
        // Execute IL_1abe -> 1be0/common -> 213c/CuJuFeiXi -> adjacent.
        if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY) {
            self.apply_cu_ju_fei_xi_after_card_hook(actor_side, card, slot);
        }
        if !is_temporary && self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.apply_dream_mirage_adjacent_after_card_hooks(actor_side, slot);
        }
        self.clear_active_effect_attacks();
        self.actor_mut(actor_side).turn.dan_ka_gong_ji_ji_shu = 0;
    }

    /// CardActionBase.OnAfterExecuted's unconditional cleanup block. These
    /// windows belong only to the main card body and must be gone before any
    /// ordinary or common follow-up attack executes.
    pub(super) fn apply_after_card_effect_early_cleanup(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        self.set_active_effect_shatter_defense(0);
        let actor = self.actor_mut(actor_side);
        actor.status.mystic_soul = 0;
        actor.elements.no_sharpness_for_attack = 0;
        actor.turn.guaranteed_wound = 0;
        actor.beng.momentum_multiplier = 0;
        actor.turn.current_turn_ignore_defense = 0;
        actor.turn.ignore_weakness_attacks = 0;
        self.clear_consumed_beng_quan_chuo(actor_side, card);
        self.complete_beng_quan_star_seize_card(actor_side, card);
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.complete_dream_mirage_forge_fist_card(actor_side);
        }
    }

    fn apply_ordinary_only_after_card_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        // CardActionBase.OnAfterExecuted !isTempCard branch, in exact source
        // order. Keep each source dynamic: an earlier hook may change the
        // state read by a later one.
        if self.active_mechanisms.has(super::ActiveMechanisms::FORMATION | super::ActiveMechanisms::SWORD_SECT) {
            self.apply_secret_sword_formation_follow_up(actor_side, card, slot, false);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::RONGHUI | super::ActiveMechanisms::SWORD_SECT) {
            self.apply_ronghui_after_card_effect(actor_side, card, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::FORMATION)
            || self.actor(actor_side).formations.body_observation > 0
        {
            self.apply_after_attack_formation_hooks(actor_side, slot);
            self.apply_hundred_beast_spirit_sword_formation_after_card(actor_side, card);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::SWORD_SECT) {
            self.apply_sword_energy_after_card_hook(actor_side, card);
        }
        self.apply_physique_173_after_card_hook(actor_side, card, slot);
        if self.active_mechanisms.has(super::ActiveMechanisms::ASTROLOGY) {
            self.apply_after_card_fortune_hooks(actor_side, card);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::FORMATION) {
            self.apply_after_card_sustain_hooks(actor_side, card, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::ASTROLOGY | super::ActiveMechanisms::FATE_STRATEGY) {
            self.apply_yellow_bird_after_card_hook(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::RONGHUI) {
            self.apply_ronghui_spirit_sparrow_after_card(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.apply_dream_mirage_ordinary_after_effect_hooks(actor_side, card, slot);
        }
    }

    /// 星缘 FateStrategy 402（原版 BuffType XingYuan 765）：
    /// CardActionBase.OnBeforeExecuted IL_245a 的星位牌分支。只看本次
    /// 执行的当前槽位是否为星位，不按牌名或卡种猜测；实卡与 temporary
    /// invocation 共用此边界，层数耗尽后不再追加内伤。
    pub(super) fn apply_xing_yuan_before_card_hook(&mut self, actor_side: PlayerSide, slot: usize) {
        if self.actor(actor_side).fate.xing_yuan <= 0
            || !self.actor(actor_side).astrology.star_slots.contains(&slot)
        {
            return;
        }
        self.actor_mut(actor_side).fate.xing_yuan -= 1;
        self.add_actor_negative_status(opponent_side(actor_side), 100, 1);
    }

    /// 促局飞袭 FateStrategy 416（原版 BuffType CuJuFeiXi 768）：
    /// CardActionBase.OnAfterExecuted IL_1fc6（:3996-4002）——本张牌名
    /// 含「火灵」且 CuJuFeiXi > 0 时，消耗全部层数并对对方追加一次
    /// Attack(buffValue)（buff 值 = FateStrategyConfig 416 otherParams[0]=5，
    /// battle_start.rs 按 FateStrategyFunctions.cs:571-573 发放）。temp 执行
    /// （五行流转 7000067）同样触发——oracle 锚点：mirror-32219000-human-01
    /// pa_0a7d7655 t5u1（temp 火灵•烈燎原 后 768:5→消耗、
    /// 第 4 攻击段 8 = 5+加攻3，计入 323/493/644）、round-12 t7u1
    /// （temp 火灵•赤焰 第 4 段 8 = 5+加攻4-减攻1）。实卡段不出现的原因是
    /// 768 已被该玩家的首张「火灵」牌（temp 或实卡）消耗，而非路径差异。
    pub(super) fn apply_cu_ju_fei_xi_after_card_hook(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        let remaining = self.actor(actor_side).fate.cu_ju_fei_xi;
        if remaining <= 0 || !card.name.contains("火灵") {
            return;
        }
        self.actor_mut(actor_side).fate.cu_ju_fei_xi = 0;
        if remaining > 0 {
            self.apply_attack(actor_side, remaining, slot);
        }
    }

    pub(super) fn apply_common_after_card_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        if self.active_mechanisms.has(super::ActiveMechanisms::BENG_QUAN) {
            self.apply_startled_touch_common_after_card_hook(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.apply_dream_mirage_common_after_effect_hooks(actor_side, slot);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE | super::ActiveMechanisms::BENG_QUAN) {
            let dream_chain = self
                .actor(actor_side)
                .beng
                .triggered_dream_beng_quan_chain
                .max(0);
            for _ in 0..dream_chain {
                self.apply_attack_with_options(
                    actor_side,
                    2,
                    slot,
                    false,
                    false,
                    0,
                    Some("buff:dreamBengQuanChain"),
                );
            }
            if dream_chain > 0 {
                self.actor_mut(actor_side)
                    .beng
                    .triggered_dream_beng_quan_chain -= dream_chain;
            }
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::BENG_QUAN) {
            self.apply_beng_quan_fu_hu_after_card(actor_side);
        }
    }

    pub(super) fn apply_card_classification_completed_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        self.refresh_active_mechanisms();
        let base_id = normalized_base_id(card);
        if self.active_mechanisms.has(super::ActiveMechanisms::SWORD_SECT) {
            let resonance_10_spirit_sword = (self.actor(actor_side).identity.talent_resonance_id
                == Some(10))
                && is_spirit_sword_for_actor(self.actor(actor_side), card);
            if is_cloud_sword(self.actor(actor_side), card) {
                self.gain_cloud_chain(actor_side, 1);
                self.apply_secret_sword_cloud_step_after_cloud_sword(actor_side, card);
            } else if self.actor(actor_side).sword.cloud_sea > 0 {
                self.gain_cloud_chain(actor_side, 1);
                self.actor_mut(actor_side).sword.cloud_sea -= 1;
                if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY)
                    && self
                        .actor(actor_side)
                        .identity
                        .fate_strategies
                        .contains(&97)
                {
                    self.gain_anima(actor_side, 1);
                }
                // isStopLianYun:12707 仙魔 7010 在 FS97 之后、FS380 之前同样 +1 灵气。
                if self.active_mechanisms.has(super::ActiveMechanisms::XIAN_MO)
                    && self
                        .actor(actor_side)
                        .identity
                        .xian_mo_strategies
                        .contains(&7010)
                {
                    self.gain_anima(actor_side, 1);
                }
                // FateStrategy 380 御云结阵: when a non-cloud card consumes a
                // 云海 marker, the original isStopLianYun hook grants +2 defense
                // (BattleCharacter.cs:12331-12335).  This is the shared timing
                // path used by 御空剑阵 after 云剑•凌波; omitting it leaves each
                // such sword-formation checkpoint two defense short.
                if self.active_mechanisms.has(super::ActiveMechanisms::FATE_STRATEGY)
                    && self
                        .actor(actor_side)
                        .identity
                        .fate_strategies
                        .contains(&380)
                {
                    self.gain_defense(actor_side, 2);
                }
                if self.active_mechanisms.has(super::ActiveMechanisms::XIAN_MO)
                    && self
                        .actor(actor_side)
                        .identity
                        .xian_mo_strategies
                        .contains(&7132)
                {
                    self.gain_defense(actor_side, 2);
                }
            } else if !self.actor(actor_side).identity.talents.contains(&14)
                && !resonance_10_spirit_sword
            {
                let before = self.actor(actor_side).sword.cloud_chain;
                self.actor_mut(actor_side).sword.cloud_chain = 0;
                self.record_counter_transition(actor_side, "剑系", "cloudChain", "连云", before, 0);
            }
            if self.actor(actor_side).identity.talent_resonance_id == Some(10) {
                let flags = &mut self
                    .actor_mut(actor_side)
                    .identity
                    .talent_resonance_temp_flags;
                if resonance_10_spirit_sword {
                    if !flags.contains(&10) {
                        flags.push(10);
                    }
                } else {
                    flags.retain(|flag| *flag != 10);
                }
            }
            if is_frenzy_sword_for_actor(self.actor(actor_side), card) {
                self.modify_frenzy_sword(actor_side, 1);
            }
            if is_sword_formation_card(self.actor(actor_side), card) {
                self.modify_sword_formation_count(actor_side, 1);
            }
            if base_id == 19 && self.actor(actor_side).identity.talents.contains(&10_096) {
                self.actor_mut(actor_side).sword.next_cards_as_frenzy_sword += 1;
            }
        }
        // CardActionBase.OnAfterExecuted records the effective card config's
        // career after the card body and dynamic action-again snapshot. The
        // original QinShi Card_* bodies keep their separate +1.
        if self.active_mechanisms.has(super::ActiveMechanisms::MUSIC)
            && card.career_name.as_deref() == Some("QinShi")
        {
            self.actor_mut(actor_side).music.music_cards_played += 1;
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.complete_dream_mirage_card_classification(actor_side, card);
        }
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE | super::ActiveMechanisms::RONGHUI) {
            self.complete_mirage_ronghui_card_classification(actor_side, card);
        }
    }

    pub(super) fn settle_sword_intent_after_card_effect(&mut self, actor_side: PlayerSide) {
        if self.active_mechanisms.has(super::ActiveMechanisms::MIRAGE) {
            self.apply_mirage_sword_intent_refund_before_settlement(actor_side);
        }
        let pending_sword_intent = self.active_effect_pending_sword_intent();
        if pending_sword_intent > 0 {
            if !self.active_mechanisms.has(super::ActiveMechanisms::SWORD_SECT)
                || !self.preserve_secret_sword_intent_with_circulation(actor_side)
            {
                self.modify_sword_intent(actor_side, -pending_sword_intent);
            }
        }

        let deferred_restore = self.active_effect_deferred_sword_intent_restore().max(0);
        if deferred_restore > 0 {
            self.modify_sword_intent(actor_side, deferred_restore);
        }
        if let Some(frame) = self.effect_invocation_stack.last() {
            frame.local.borrow_mut().suppress_sword_intent = false;
        }
        self.clear_active_effect_settlement_local();
    }
}
