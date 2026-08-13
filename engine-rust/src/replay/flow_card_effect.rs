use super::card_effect_catalog::{resolve_card_effect, CardEffectResolution};
#[cfg(test)]
use super::effect_invocation::EffectInvocationKind;
use super::effect_invocation::{EffectInvocationPhase, TemporaryInvocationSpec};
use super::support::{
    has_cloud_chain, is_cloud_sword, is_frenzy_sword_for_actor, is_spirit_sword_for_actor,
    is_sword_formation_card, normalized_base_id, opponent_side,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
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
            CardEffectResolution::RecordOnly if !allow_record_only => {
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
        // OnBeforeExecuted first opens temporary sword-identity windows, then
        // runs its private adjacent-card helper. The counter-element branch
        // occurs later in that same method, after the shared selected hooks.
        self.start_dream_mirage_selected_card(actor_side);
        self.apply_dream_mirage_before_card_hooks(actor_side, card, slot);
        self.apply_qi_xing_lian_zhu(actor_side, card, slot);
        self.apply_selected_card_hooks_after_start(actor_side, card, slot, is_temporary);
        // FateStrategy 380 御云结阵 is an OnPlayCard hook: a sword-formation
        // card first adds one 云海 marker.  The shared post-body cloud-chain
        // path then consumes that marker and applies its +2 defense bonus.
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&380)
            && is_sword_formation_card(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.cloud_sea += 1;
        }
        self.apply_counter_element_before_card(actor_side, card, slot);
        self.apply_jing_lei_before_card_hook(actor_side, card);
        // 星缘 FateStrategy 402（CardActionBase.OnBeforeExecuted IL_245a）
        // 必须在牌体执行前检查星位；例如天元心法会在牌体内把全槽变星，
        // 但原版不会因此把该张牌自身追认为星位牌。
        self.apply_xing_yuan_before_card_hook(actor_side, slot);
    }

    /// 惊雷破敌 FateStrategy 396（KeYinJingLei 574）：原版
    /// CardActionBase/KeYinCardFunctions 的共享出牌前钩子只对当前原版
    /// 配置归类为「雷」的牌生效，不按 Rust handler 猜测。命中时清空全部
    /// 层数，给本次牌体攻击碎防一段，并令目标 WaiShang(105) 增加捕获层数。
    fn apply_jing_lei_before_card_hook(&mut self, actor_side: PlayerSide, card: &CardDefinition) {
        let stacks = self.actor(actor_side).fate.ke_yin_jing_lei.max(0);
        // KeYinCardFunctions.cs:996: name contains 雷 and either a positive
        // attack value or the special multi-segment 五雷轰顶 card.
        let qualifies =
            card.name.contains('雷') && (card.attack.unwrap_or(0) > 0 || card.name == "五雷轰顶");
        if stacks <= 0 || !qualifies {
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
        // bd14d54dc1d0cde3/round-15（五雷轰顶 5 段全部要吃碎防）。
        self.gain_active_effect_shatter_defense(1);
        // 原版走 dst.ModifyBuffValue(WaiShang, ...)：会经过共享的负面状态
        // 增益管线，包括 星蚀（talent 103 系 BeiXingShi 317，
        // BattleCharacter.cs:8464-8467 —— 对方首次获得负面状态时额外加层）。
        // oracle 锚点：mirror-32299000 9c3ca847ba7679a5/round-08 cp5
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
            let physical_card = self
                .actor(actor_side)
                .deck
                .slots
                .get(slot)
                .map_or_else(|| card.clone(), |slot_state| slot_state.card.clone());
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
        // CardActionBase.OnAfterExecuted: fate 387 moves the remaining anima
        // into JianQi for CardConfig id 19. Fate 101's JianQi grant is applied
        // by the classification hook above, so settle both together here.
        self.set_active_effect_phase(EffectInvocationPhase::Settlement);
        self.settle_wan_shi_ru_yi_card_19(actor_side, card);
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
                state.settle_wan_shi_ru_yi_card_19(actor_side, card);
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
        // d5884a1c411a0681/round-18 checkpoint[5] 万玄破魔掌后 p2 身法 6
        // （=14 身法 -10 再次行动消耗 +2），引擎原为 4；9874c3ab697ed8ec/
        // round-10 checkpoint[11] 迎风掌后同样 +2。
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&427)
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
        let physical_card = probe
            .actor(actor_side)
            .deck
            .slots
            .get(slot)
            .map_or_else(|| card.clone(), |slot_state| slot_state.card.clone());
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
        if let Some(sword_intent) = card
            .sword_intent
            .filter(|value| *value > 0 && !matches!(base_id, 47 | 183 | 309 | 1_000_050))
        {
            self.modify_sword_intent(actor_side, sword_intent);
        }

        let Some(hexagram) = card.hexagram.filter(|value| *value > 0) else {
            return;
        };
        if !matches!(
            base_id,
            7 | 51
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
        self.set_active_effect_phase(EffectInvocationPhase::EnterAfter);
        self.apply_after_card_effect_early_cleanup(actor_side, card);
        self.set_active_effect_after_action(true);
        self.set_active_effect_phase(EffectInvocationPhase::AfterHooks);
        if !is_temporary {
            self.apply_ordinary_only_after_card_effect_hooks(actor_side, card, slot);
            self.complete_mirage_ronghui_anima_attack_card(actor_side);
        }
        self.apply_cu_ju_fei_xi_after_card_hook(actor_side, card, slot);
        self.apply_common_after_card_effect_hooks(actor_side, slot);
        if !is_temporary {
            self.apply_dream_mirage_adjacent_after_card_hooks(actor_side, slot);
        }
        self.clear_active_effect_attacks();
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
        self.complete_dream_mirage_forge_fist_card(actor_side);
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
        self.apply_secret_sword_formation_follow_up(actor_side, card, slot, false);
        self.apply_ronghui_after_card_effect(actor_side, card, slot);
        self.apply_after_attack_formation_hooks(actor_side, slot);
        self.apply_hundred_beast_spirit_sword_formation_after_card(actor_side, card);
        self.apply_sword_energy_after_card_hook(actor_side, card);
        self.apply_physique_173_after_card_hook(actor_side, card, slot);
        self.apply_after_card_fortune_hooks(actor_side, card);
        self.apply_after_card_sustain_hooks(actor_side, card, slot);
        self.apply_yellow_bird_after_card_hook(actor_side, slot);
        self.apply_ronghui_spirit_sparrow_after_card(actor_side, slot);
        self.apply_dream_mirage_ordinary_after_effect_hooks(actor_side, card, slot);
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
    /// cae463212f8c4c43/round-15 t5u1（temp 火灵•烈燎原 后 768:5→消耗、
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
        self.apply_startled_touch_common_after_card_hook(actor_side, slot);
        self.apply_dream_mirage_common_after_effect_hooks(actor_side, slot);
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
        self.apply_beng_quan_fu_hu_after_card(actor_side);
    }

    pub(super) fn apply_card_classification_completed_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let base_id = normalized_base_id(card);
        let resonance_10_spirit_sword = self.actor(actor_side).identity.talent_resonance_id
            == Some(10)
            && is_spirit_sword_for_actor(self.actor(actor_side), card);
        if is_cloud_sword(self.actor(actor_side), card) {
            self.gain_cloud_chain(actor_side, 1);
            self.apply_secret_sword_cloud_step_after_cloud_sword(actor_side, card);
        } else if self.actor(actor_side).sword.cloud_sea > 0 {
            self.gain_cloud_chain(actor_side, 1);
            self.actor_mut(actor_side).sword.cloud_sea -= 1;
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&97)
            {
                self.gain_anima(actor_side, 1);
            }
            // FateStrategy 380 御云结阵: when a non-cloud card consumes a
            // 云海 marker, the original isStopLianYun hook grants +2 defense
            // (BattleCharacter.cs:12331-12335).  This is the shared timing
            // path used by 御空剑阵 after 云剑•凌波; omitting it leaves each
            // such sword-formation checkpoint two defense short.
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&380)
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
        // CardActionBase.OnAfterExecuted records the effective card config's
        // career after the card body and dynamic action-again snapshot. The
        // original QinShi Card_* bodies keep their separate +1.
        if card.career_name.as_deref() == Some("QinShi") {
            self.actor_mut(actor_side).music.music_cards_played += 1;
        }
        self.complete_dream_mirage_card_classification(actor_side, card);
        self.complete_mirage_ronghui_card_classification(actor_side, card);
    }

    pub(super) fn settle_sword_intent_after_card_effect(&mut self, actor_side: PlayerSide) {
        self.apply_mirage_sword_intent_refund_before_settlement(actor_side);
        let pending_sword_intent = self.active_effect_pending_sword_intent();
        if pending_sword_intent > 0
            && !self.preserve_secret_sword_intent_with_circulation(actor_side)
        {
            self.modify_sword_intent(actor_side, -pending_sword_intent);
        }

        let deferred_restore = self.active_effect_deferred_sword_intent_restore().max(0);
        if deferred_restore > 0 {
            self.modify_sword_intent(actor_side, deferred_restore);
        }
        self.clear_active_effect_settlement_local();
    }
}
