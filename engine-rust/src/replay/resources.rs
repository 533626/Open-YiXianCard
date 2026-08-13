use super::cards_dream_mirage::DreamMirageValue;
use super::support::{div_ceil, has_active_base_card_in_deck, opponent_side};
use super::{DefenseMutationReceipt, MaxHpMutationReceipt, MomentumMutationReceipt, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum CombatResource {
    StarPower,
    SwordIntent,
    Agility,
    Momentum,
}

impl ReplayState {
    /// Single clamped counter mutation with receipt recording. The named
    /// `modify_*` wrappers below are the documented entry points; this generic
    /// holds the before/after/clamp/record template they used to duplicate.
    pub(super) fn modify_counter(
        &mut self,
        actor_side: PlayerSide,
        group: &'static str,
        key: &'static str,
        label: &'static str,
        accessor: impl FnOnce(&mut super::ReplayPlayer) -> &mut i64,
        delta: i64,
    ) {
        let field = accessor(self.actor_mut(actor_side));
        let before = *field;
        let after = (before + delta).max(0);
        *field = after;
        self.record_counter_transition(actor_side, group, key, label, before, after);
    }

    pub(super) fn modify_extra_actions(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "回合",
            "extraActions",
            "再次行动",
            |player| &mut player.turn.extra_actions,
            delta,
        );
    }

    pub(super) fn modify_next_attack_shatter_defense(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
    ) {
        self.modify_counter(
            actor_side,
            "回合",
            "nextAttackShatterDefense",
            "下次攻击碎防",
            |player| &mut player.turn.next_attack_shatter_defense,
            delta,
        );
    }

    pub(super) fn modify_five_elements_marrow_art(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "五行",
            "fiveElementsMarrowArt",
            "五行髓",
            |player| &mut player.elements.five_elements_marrow_art,
            delta,
        );
    }

    pub(super) fn modify_five_elements_gourd(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "五行",
            "fiveElementsGourd",
            "五行玉瓶",
            |player| &mut player.elements.five_elements_gourd,
            delta,
        );
    }

    pub(super) fn modify_paint_finishing_touch(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "命运",
            "paintFinishingTouch",
            "画龙点睛",
            |player| &mut player.fate.paint_finishing_touch,
            delta,
        );
    }

    pub(super) fn modify_star_chess_break(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "七星",
            "starChessBreak",
            "星弈断",
            |player| &mut player.astrology.star_chess_break,
            delta,
        );
    }

    pub(super) fn modify_frenzy_sword(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "剑系",
            "frenzySword",
            "狂剑计数",
            |player| &mut player.sword.frenzy_sword,
            delta,
        );
    }

    pub(super) fn modify_sword_formation_count(&mut self, actor_side: PlayerSide, delta: i64) {
        self.modify_counter(
            actor_side,
            "剑系",
            "swordFormationCount",
            "剑阵计数",
            |player| &mut player.sword.sword_formation_count,
            delta,
        );
    }

    pub(super) fn gain_cloud_chain(&mut self, actor_side: PlayerSide, amount: i64) {
        let amount = amount.max(0);
        if amount == 0 {
            return;
        }
        let before = self.actor(actor_side).sword.cloud_chain;
        self.actor_mut(actor_side).sword.cloud_chain += amount;
        let after = self.actor(actor_side).sword.cloud_chain;
        self.record_counter_transition(actor_side, "剑系", "cloudChain", "连云", before, after);
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::CloudSwordUsedCount, amount);
    }

    /// BattleCharacter.ModifyHp/ModifyDef dispatch NiShi through ApplyDamage as
    /// ReflectDamage. The non-attack damage path owns defense, guard, TieGu,
    /// ChaiZhao + QuanJiaShi, and YiHuaJieMu semantics.
    pub(super) fn apply_heavenly_secret_reverse_from_gain(
        &mut self,
        actor_side: PlayerSide,
        positive_delta: i64,
    ) {
        if positive_delta <= 0 || self.actor(actor_side).fate.heavenly_secret_reverse <= 0 {
            return;
        }
        let damage = positive_delta / 2;
        if damage > 0 {
            self.apply_damage(actor_side, damage, false, false, false);
        }
    }

    pub(super) fn dream_mirage_value(
        &self,
        actor_side: PlayerSide,
        value: DreamMirageValue,
    ) -> i64 {
        let actor = self.actor(actor_side);
        let state = &actor.dream_mirage;
        match value {
            DreamMirageValue::DreamUnmovingFormation => state.dream_unmoving_formation,
            DreamMirageValue::DreamDanceCountdown => state.dream_dance_countdown,
            DreamMirageValue::DreamFlyingCloudPill => state.dream_flying_cloud_pill,
            DreamMirageValue::DreamGreatReturnPill => state.dream_great_return_pill,
            DreamMirageValue::DreamTuneImmunity => state.dream_tune_immunity,
            DreamMirageValue::DreamExtraActionLock => state.dream_extra_action_lock,
            DreamMirageValue::HalfAnimaGain => state.half_anima_gain,
            DreamMirageValue::CannotGainDefense => state.cannot_gain_defense,
            DreamMirageValue::CannotGainHp => actor.mirage_ronghui.cannot_gain_hp,
            DreamMirageValue::CalamitySkipMask => state.calamity_skip_mask,
            DreamMirageValue::TotalAnimaGained => state.total_anima_gained,
            DreamMirageValue::CloudSwordUsedCount => state.cloud_sword_used_count,
            DreamMirageValue::SwordUsedCount => state.sword_used_count,
            DreamMirageValue::FormationUsedCount => state.formation_used_count,
            DreamMirageValue::AnimaGainDefense => state.anima_gain_defense,
            DreamMirageValue::SwordIntentGainDefense => state.sword_intent_gain_defense,
            DreamMirageValue::TurnStartDefense => state.turn_start_defense,
            DreamMirageValue::CloudSeaOnFormation => state.cloud_sea_on_formation,
            DreamMirageValue::SwordEnergyOnSword => state.sword_energy_on_sword,
            DreamMirageValue::DoubleNextSwordIntentAndAttackBonus => {
                state.double_next_sword_intent_and_attack_bonus
            }
            DreamMirageValue::HealingTurnEndFrenzy => state.healing_turn_end_frenzy,
            DreamMirageValue::RearMoveCardUsedCount => state.rear_move_card_used_count,
            DreamMirageValue::DreamReflection => state.dream_reflection,
            DreamMirageValue::DreamStarBoard => state.dream_star_board,
            DreamMirageValue::DreamStarBoardLowRealm => state.dream_star_board_low_realm,
            DreamMirageValue::DreamStarBoardTriggered => state.dream_star_board_triggered,
            DreamMirageValue::SnakeShadow => state.snake_shadow,
            DreamMirageValue::SnakeCardUsedCount => state.snake_card_used_count,
            DreamMirageValue::WitheredTreeUsedCount => state.withered_tree_used_count,
            DreamMirageValue::ActionAgainSharpness => state.action_again_sharpness,
            DreamMirageValue::TemporaryWaterDouble => state.temporary_water_double,
            DreamMirageValue::TemporaryWaterLedger => state.temporary_water_ledger,
            DreamMirageValue::TemporaryAnimaLedger => state.temporary_anima_ledger,
            DreamMirageValue::UnconditionalFiveElements => state.unconditional_five_elements,
            DreamMirageValue::TotalActualDamage => state.total_actual_damage,
            DreamMirageValue::AttackBonusToThorns => state.attack_bonus_to_thorns,
            DreamMirageValue::LostMaxHpEventCount => state.lost_max_hp_event_count,
            DreamMirageValue::TotalSharpnessGained => state.total_sharpness_gained,
            DreamMirageValue::TotalWaterMomentumGained => state.total_water_momentum_gained,
            DreamMirageValue::DreamCliff => state.dream_cliff,
            DreamMirageValue::FiveElementsMarrow => state.five_elements_marrow,
            DreamMirageValue::ConsumeNextCard => state.consume_next_card,
            DreamMirageValue::DreamFireFormation => state.dream_fire_formation,
            DreamMirageValue::UsedFiveElementsCount => state.used_five_elements_count,
            DreamMirageValue::DreamMysticFootwork => state.dream_mystic_footwork,
            DreamMirageValue::DreamMysticFootworkHigh => state.dream_mystic_footwork_high,
            DreamMirageValue::DreamMysticFootworkSuppressed => state.dream_mystic_footwork_blocked,
            DreamMirageValue::DreamMysticFootworkTriggerCount => {
                state.dream_mystic_footwork_triggered
            }
            DreamMirageValue::DefenseLedger => state.defense_ledger,
            DreamMirageValue::TotalMomentumGained => state.total_momentum_gained,
            DreamMirageValue::FlatMomentumAttack => state.flat_momentum_attack,
            DreamMirageValue::MomentumBeforeEveryAttack => state.momentum_before_every_attack,
            DreamMirageValue::NextHpCostRefund => state.next_hp_cost_refund,
            DreamMirageValue::NextHpGainDefense => state.next_hp_gain_defense,
            DreamMirageValue::HpGainDefense => state.hp_gain_defense,
            DreamMirageValue::NextBengQuanAdditionalAttack => {
                state.next_beng_quan_additional_attack
            }
            DreamMirageValue::TriggeredBengQuanAdditionalAttack => {
                state.next_beng_quan_additional_attack_triggered
            }
            DreamMirageValue::NextBengQuanPhysique => state.next_beng_quan_physique,
            DreamMirageValue::DreamForgeFist => state.dream_forge_fist,
            DreamMirageValue::DreamForgeFistConsumed => state.dream_forge_fist_consumed,
            DreamMirageValue::DefenseGainDamageLow => state.defense_gain_damage_low,
            DreamMirageValue::DreamDefenseGainDamage => state.dream_defense_gain_damage,
            DreamMirageValue::DreamDefenseGainDamageGuard => state.dream_defense_gain_damage_guard,
            DreamMirageValue::FlowingMerciless => state.flowing_merciless,
            DreamMirageValue::StarShift => state.star_shift,
            DreamMirageValue::StarShiftAttack => state.star_shift_attack,
            DreamMirageValue::RepeatNextFireOrEarth => state.repeat_next_fire_or_earth,
            DreamMirageValue::ExtraWaterMomentumTurnEnd => state.extra_water_momentum_turn_end,
            DreamMirageValue::ReturnSharpness => state.return_sharpness,
            DreamMirageValue::ExcessPhysiqueHp => state.excess_physique_hp,
            DreamMirageValue::ExcessPhysiqueDamage => state.excess_physique_damage,
            DreamMirageValue::LastTurnStartHp => actor.mirage_ronghui.last_turn_start_hp,
            DreamMirageValue::TurnHpGained => state.turn_hp_gained,
            DreamMirageValue::SpiritCatCloud => state.spirit_cat_cloud,
            DreamMirageValue::DragonExtraActionImmunity => state.dragon_extra_action_immunity,
        }
    }

    pub(super) fn modify_dream_mirage_value(
        &mut self,
        actor_side: PlayerSide,
        value: DreamMirageValue,
        delta: i64,
    ) -> i64 {
        let actor = self.actor_mut(actor_side);
        let field = match value {
            DreamMirageValue::DreamUnmovingFormation => {
                &mut actor.dream_mirage.dream_unmoving_formation
            }
            DreamMirageValue::DreamDanceCountdown => &mut actor.dream_mirage.dream_dance_countdown,
            DreamMirageValue::DreamFlyingCloudPill => {
                &mut actor.dream_mirage.dream_flying_cloud_pill
            }
            DreamMirageValue::DreamGreatReturnPill => {
                &mut actor.dream_mirage.dream_great_return_pill
            }
            DreamMirageValue::DreamTuneImmunity => &mut actor.dream_mirage.dream_tune_immunity,
            DreamMirageValue::DreamExtraActionLock => {
                &mut actor.dream_mirage.dream_extra_action_lock
            }
            DreamMirageValue::HalfAnimaGain => &mut actor.dream_mirage.half_anima_gain,
            DreamMirageValue::CannotGainDefense => &mut actor.dream_mirage.cannot_gain_defense,
            DreamMirageValue::CannotGainHp => &mut actor.mirage_ronghui.cannot_gain_hp,
            DreamMirageValue::CalamitySkipMask => &mut actor.dream_mirage.calamity_skip_mask,
            DreamMirageValue::TotalAnimaGained => &mut actor.dream_mirage.total_anima_gained,
            DreamMirageValue::CloudSwordUsedCount => &mut actor.dream_mirage.cloud_sword_used_count,
            DreamMirageValue::SwordUsedCount => &mut actor.dream_mirage.sword_used_count,
            DreamMirageValue::FormationUsedCount => &mut actor.dream_mirage.formation_used_count,
            DreamMirageValue::AnimaGainDefense => &mut actor.dream_mirage.anima_gain_defense,
            DreamMirageValue::SwordIntentGainDefense => {
                &mut actor.dream_mirage.sword_intent_gain_defense
            }
            DreamMirageValue::TurnStartDefense => &mut actor.dream_mirage.turn_start_defense,
            DreamMirageValue::CloudSeaOnFormation => &mut actor.dream_mirage.cloud_sea_on_formation,
            DreamMirageValue::SwordEnergyOnSword => &mut actor.dream_mirage.sword_energy_on_sword,
            DreamMirageValue::DoubleNextSwordIntentAndAttackBonus => {
                &mut actor.dream_mirage.double_next_sword_intent_and_attack_bonus
            }
            DreamMirageValue::HealingTurnEndFrenzy => {
                &mut actor.dream_mirage.healing_turn_end_frenzy
            }
            DreamMirageValue::RearMoveCardUsedCount => {
                &mut actor.dream_mirage.rear_move_card_used_count
            }
            DreamMirageValue::DreamReflection => &mut actor.dream_mirage.dream_reflection,
            DreamMirageValue::DreamStarBoard => &mut actor.dream_mirage.dream_star_board,
            DreamMirageValue::DreamStarBoardLowRealm => {
                &mut actor.dream_mirage.dream_star_board_low_realm
            }
            DreamMirageValue::DreamStarBoardTriggered => {
                &mut actor.dream_mirage.dream_star_board_triggered
            }
            DreamMirageValue::SnakeShadow => &mut actor.dream_mirage.snake_shadow,
            DreamMirageValue::SnakeCardUsedCount => &mut actor.dream_mirage.snake_card_used_count,
            DreamMirageValue::WitheredTreeUsedCount => {
                &mut actor.dream_mirage.withered_tree_used_count
            }
            DreamMirageValue::ActionAgainSharpness => {
                &mut actor.dream_mirage.action_again_sharpness
            }
            DreamMirageValue::TemporaryWaterDouble => {
                &mut actor.dream_mirage.temporary_water_double
            }
            DreamMirageValue::TemporaryWaterLedger => {
                &mut actor.dream_mirage.temporary_water_ledger
            }
            DreamMirageValue::TemporaryAnimaLedger => {
                &mut actor.dream_mirage.temporary_anima_ledger
            }
            DreamMirageValue::UnconditionalFiveElements => {
                &mut actor.dream_mirage.unconditional_five_elements
            }
            DreamMirageValue::TotalActualDamage => &mut actor.dream_mirage.total_actual_damage,
            DreamMirageValue::AttackBonusToThorns => &mut actor.dream_mirage.attack_bonus_to_thorns,
            DreamMirageValue::LostMaxHpEventCount => {
                &mut actor.dream_mirage.lost_max_hp_event_count
            }
            DreamMirageValue::TotalSharpnessGained => {
                &mut actor.dream_mirage.total_sharpness_gained
            }
            DreamMirageValue::TotalWaterMomentumGained => {
                &mut actor.dream_mirage.total_water_momentum_gained
            }
            DreamMirageValue::DreamCliff => &mut actor.dream_mirage.dream_cliff,
            DreamMirageValue::FiveElementsMarrow => &mut actor.dream_mirage.five_elements_marrow,
            DreamMirageValue::ConsumeNextCard => &mut actor.dream_mirage.consume_next_card,
            DreamMirageValue::DreamFireFormation => &mut actor.dream_mirage.dream_fire_formation,
            DreamMirageValue::UsedFiveElementsCount => {
                &mut actor.dream_mirage.used_five_elements_count
            }
            DreamMirageValue::DreamMysticFootwork => &mut actor.dream_mirage.dream_mystic_footwork,
            DreamMirageValue::DreamMysticFootworkHigh => {
                &mut actor.dream_mirage.dream_mystic_footwork_high
            }
            DreamMirageValue::DreamMysticFootworkSuppressed => {
                &mut actor.dream_mirage.dream_mystic_footwork_blocked
            }
            DreamMirageValue::DreamMysticFootworkTriggerCount => {
                &mut actor.dream_mirage.dream_mystic_footwork_triggered
            }
            DreamMirageValue::DefenseLedger => &mut actor.dream_mirage.defense_ledger,
            DreamMirageValue::TotalMomentumGained => &mut actor.dream_mirage.total_momentum_gained,
            DreamMirageValue::FlatMomentumAttack => &mut actor.dream_mirage.flat_momentum_attack,
            DreamMirageValue::MomentumBeforeEveryAttack => {
                &mut actor.dream_mirage.momentum_before_every_attack
            }
            DreamMirageValue::NextHpCostRefund => &mut actor.dream_mirage.next_hp_cost_refund,
            DreamMirageValue::NextHpGainDefense => &mut actor.dream_mirage.next_hp_gain_defense,
            DreamMirageValue::HpGainDefense => &mut actor.dream_mirage.hp_gain_defense,
            DreamMirageValue::NextBengQuanAdditionalAttack => {
                &mut actor.dream_mirage.next_beng_quan_additional_attack
            }
            DreamMirageValue::TriggeredBengQuanAdditionalAttack => {
                &mut actor
                    .dream_mirage
                    .next_beng_quan_additional_attack_triggered
            }
            DreamMirageValue::NextBengQuanPhysique => {
                &mut actor.dream_mirage.next_beng_quan_physique
            }
            DreamMirageValue::DreamForgeFist => &mut actor.dream_mirage.dream_forge_fist,
            DreamMirageValue::DreamForgeFistConsumed => {
                &mut actor.dream_mirage.dream_forge_fist_consumed
            }
            DreamMirageValue::DefenseGainDamageLow => {
                &mut actor.dream_mirage.defense_gain_damage_low
            }
            DreamMirageValue::DreamDefenseGainDamage => {
                &mut actor.dream_mirage.dream_defense_gain_damage
            }
            DreamMirageValue::DreamDefenseGainDamageGuard => {
                &mut actor.dream_mirage.dream_defense_gain_damage_guard
            }
            DreamMirageValue::FlowingMerciless => &mut actor.dream_mirage.flowing_merciless,
            DreamMirageValue::StarShift => &mut actor.dream_mirage.star_shift,
            DreamMirageValue::StarShiftAttack => &mut actor.dream_mirage.star_shift_attack,
            DreamMirageValue::RepeatNextFireOrEarth => {
                &mut actor.dream_mirage.repeat_next_fire_or_earth
            }
            DreamMirageValue::ExtraWaterMomentumTurnEnd => {
                &mut actor.dream_mirage.extra_water_momentum_turn_end
            }
            DreamMirageValue::ReturnSharpness => &mut actor.dream_mirage.return_sharpness,
            DreamMirageValue::ExcessPhysiqueHp => &mut actor.dream_mirage.excess_physique_hp,
            DreamMirageValue::ExcessPhysiqueDamage => {
                &mut actor.dream_mirage.excess_physique_damage
            }
            DreamMirageValue::LastTurnStartHp => &mut actor.mirage_ronghui.last_turn_start_hp,
            DreamMirageValue::TurnHpGained => &mut actor.dream_mirage.turn_hp_gained,
            DreamMirageValue::SpiritCatCloud => &mut actor.dream_mirage.spirit_cat_cloud,
            DreamMirageValue::DragonExtraActionImmunity => {
                &mut actor.dream_mirage.dragon_extra_action_immunity
            }
        };
        let before = *field;
        *field = (*field + delta).max(0);
        *field - before
    }

    pub(super) fn modify_actor_max_hp(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
    ) -> MaxHpMutationReceipt {
        let receipt = self.modify_actor_max_hp_inner(actor_side, delta);
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::MaxHp,
            "核心",
            "maxHp",
            "生命上限",
            receipt.before,
            receipt.after,
            receipt.applied,
        );
        receipt
    }

    fn modify_actor_max_hp_inner(
        &mut self,
        actor_side: PlayerSide,
        mut delta: i64,
    ) -> MaxHpMutationReceipt {
        let requested = delta;
        let before = self.actor(actor_side).core.max_hp;
        if delta == 0 {
            return MaxHpMutationReceipt {
                requested: 0,
                resolved: 0,
                applied: 0,
                before,
                after: before,
            };
        }
        // 雪羽清风 FateStrategy 394（BattleCharacter.ModifyMaxHp 正向分支）：
        // 每次正向生命上限请求额外 +1。tempData 缺失/0 表示开启，非零关闭；
        // 只在共享上限入口加成，避免由此触发的回血再次被当作上限变化。
        if delta > 0
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&394)
            && self
                .actor(actor_side)
                .identity
                .fate_strategy_temp_datas
                .get("394")
                .copied()
                .unwrap_or(0)
                == 0
        {
            delta += 1;
        }
        // BattleCharacter.ModifyMaxHp:9840-9882. This wrapper owns the full
        // ordered modifier/hook path; ReplayPlayer only applies the final
        // clamped vitals delta.
        delta = self.actor(actor_side).apply_adaptation_boost(delta);
        let molten = if delta < 0 {
            self.actor(opponent_side(actor_side))
                .mirage_ronghui
                .molten_ring
                .max(0)
        } else {
            0
        };
        let mut adjusted = delta - molten;
        if adjusted < 0
            && self.dream_mirage_value(actor_side, DreamMirageValue::DreamUnmovingFormation) > 0
        {
            adjusted = -1;
        }
        if adjusted < 0 {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::LostMaxHpEventCount, 1);
            self.actor_mut(actor_side).core.lost_max_hp_count +=
                super::cards_synthetic_oracle_verified_secret_misc::lost_max_hp_layers_from_delta(
                    adjusted,
                );
        }
        let actual_delta = self.actor_mut(actor_side).apply_max_hp_delta_raw(adjusted);
        if actual_delta > 0 {
            self.apply_yan_qi_healing(actor_side);
        }
        MaxHpMutationReceipt {
            requested,
            resolved: adjusted,
            applied: actual_delta,
            before,
            after: self.actor(actor_side).core.max_hp,
        }
    }

    pub(super) fn gain_dream_mirage_attack_bonus(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) -> i64 {
        self.gain_attack_bonus(actor_side, amount)
    }

    pub(super) fn gain_attack_bonus(&mut self, actor_side: PlayerSide, amount: i64) -> i64 {
        self.actor_mut(actor_side).gain_attack_bonus_local(amount)
    }

    fn combat_resource_value(&self, actor_side: PlayerSide, resource: CombatResource) -> i64 {
        let actor = self.actor(actor_side);
        match resource {
            CombatResource::StarPower => actor.astrology.star_power,
            CombatResource::SwordIntent => actor.sword.sword_intent,
            CombatResource::Agility => actor.turn.agility,
            CombatResource::Momentum => actor.beng.momentum,
        }
    }

    /// BattleCharacter.ModifyBuffValue:8537-8553. This is the only raw write
    /// primitive for the four combat resources represented by CombatResource.
    /// It clamps at zero and returns the actual committed delta.
    fn commit_combat_resource_value(
        &mut self,
        actor_side: PlayerSide,
        resource: CombatResource,
        value: i64,
    ) -> i64 {
        let before = self.combat_resource_value(actor_side, resource);
        let after = value.max(0);
        let actor = self.actor_mut(actor_side);
        match resource {
            CombatResource::StarPower => actor.astrology.star_power = after,
            CombatResource::SwordIntent => actor.sword.sword_intent = after,
            CombatResource::Agility => actor.turn.agility = after,
            CombatResource::Momentum => actor.beng.momentum = after,
        }
        let (group, key, label) = match resource {
            CombatResource::StarPower => ("七星", "starPower", "星力"),
            CombatResource::SwordIntent => ("剑系", "swordIntent", "剑意"),
            CombatResource::Agility => ("回合", "agility", "身法"),
            CombatResource::Momentum => return after - before,
        };
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            group,
            key,
            label,
            before,
            after,
            after - before,
        );
        after - before
    }

    fn modify_combat_resource(
        &mut self,
        actor_side: PlayerSide,
        resource: CombatResource,
        delta: i64,
    ) -> i64 {
        if delta == 0 {
            return 0;
        }
        match resource {
            CombatResource::StarPower => self.modify_star_power_inner(actor_side, delta),
            CombatResource::SwordIntent => self.modify_sword_intent_inner(actor_side, delta),
            CombatResource::Agility => self.modify_agility_inner(actor_side, delta),
            CombatResource::Momentum => self.modify_momentum_inner(actor_side, delta).hook_delta,
        }
    }

    pub(super) fn modify_star_power(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        self.modify_combat_resource(actor_side, CombatResource::StarPower, delta)
    }

    fn modify_star_power_inner(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        if delta > 0 && super::support::has_ke_yin_type(self.actor(actor_side), 29) {
            // BattleCharacter.ModifyBuffValue:8532-8536. 刻印 29 redirects
            // the requested Star Power into the opponent's Internal Injury
            // before the resource is committed, so no Star Power post-hook
            // (including Xing Yue Qian Kun Shan) may observe this request.
            self.add_actor_negative_status(opponent_side(actor_side), 100, delta);
            return 0;
        }
        if delta > 0
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&399)
            && self
                .actor(actor_side)
                .identity
                .fate_strategy_temp_datas
                .get("399")
                .copied()
                .unwrap_or(0)
                == 0
        {
            // BattleCharacter.ModifyBuffValue:8598-8601. FateStrategy 399
            // redirects positive XingLi to the opponent's NeiShang before
            // Star Power is committed; IsSwitchActive treats missing/zero
            // tempData as active and any non-zero value as disabled.
            self.add_actor_negative_status(opponent_side(actor_side), 100, delta);
            return 0;
        }
        let before = self.actor(actor_side).astrology.star_power;
        let actual_delta = self.commit_combat_resource_value(
            actor_side,
            CombatResource::StarPower,
            before + delta,
        );
        if actual_delta > 0 {
            // BattleCharacter.ModifyBuffValue:8721-8724. The fan reacts after
            // the value is committed and reads the lower-clamped actual delta.
            self.apply_six_yao_buff_gain_damage(actor_side, actual_delta);
        } else if actual_delta < 0 && self.actor(actor_side).astrology.zi_mang_xing_bao > 0 {
            // BattleCharacter.ModifyBuffValue:8811-8816. Card 422 紫芒星爆
            // (buff 773 ZiMangXingBao) converts each actually lost Star Power
            // into Attack Bonus. 24589371/24610558 起触发条件从
            // HasCardInDeck(422) 改写为 HasBuff(ZiMangXingBao)
            // （BUILD_24589371_RULE_DELTA §2，synthetic batch-027 转正）。
            self.gain_attack_bonus(actor_side, -actual_delta);
        }
        actual_delta
    }

    pub(super) fn modify_sword_intent(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        self.modify_combat_resource(actor_side, CombatResource::SwordIntent, delta)
    }

    fn modify_sword_intent_inner(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        let before = self.actor(actor_side).sword.sword_intent;
        let actual_delta = self.commit_combat_resource_value(
            actor_side,
            CombatResource::SwordIntent,
            before + delta,
        );
        // BattleCharacter.ModifyBuffValue:8766-8769. Dream Lingxi Formation
        // reacts only after a positive Sword Intent delta has been committed.
        let defense = actual_delta.max(0)
            * self
                .dream_mirage_value(actor_side, DreamMirageValue::SwordIntentGainDefense)
                .max(0);
        if defense > 0 {
            self.gain_defense(actor_side, defense);
        }
        actual_delta
    }

    pub(super) fn modify_agility(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        self.modify_combat_resource(actor_side, CombatResource::Agility, delta)
    }

    fn modify_agility_inner(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        let before = self.actor(actor_side).turn.agility;
        let actual_delta =
            self.commit_combat_resource_value(actor_side, CombatResource::Agility, before + delta);
        if actual_delta > 0 {
            // BattleCharacter.ModifyBuffValue:8589-8597. Card 391 defense is
            // resolved before PingXuYuFeng's post-gain damage.
            if has_active_base_card_in_deck(self.actor(actor_side), 391) {
                self.gain_defense(actor_side, actual_delta);
            }
            let multiplier = self.actor(actor_side).turn.agility_gain_damage.max(0);
            if multiplier > 0 {
                self.apply_damage(actor_side, actual_delta * multiplier, false, false, false);
            }
            if self.actor(actor_side).fate.tian_yan_feng_ling_duan_qu > 0 {
                // 风灵锻躯（Fate 431）：BattleCharacter.cs:8733-8737 —
                // ModifyBuffValue(ShenFa, delta>0) 时消耗 1 层
                // TianYanFengLingDuanQu(771) 换 1 体魄。位于 391 加防与
                // 凭虚御风自伤之后（原版对应 8589/8610 段早于 8733）。
                self.actor_mut(actor_side).fate.tian_yan_feng_ling_duan_qu -= 1;
                self.apply_physique_amount(actor_side, 1);
            }
        }
        actual_delta
    }

    /// Card_291.cs:95-102 calls SetBuffValue(ShenFa, 5), deliberately
    /// bypassing ModifyBuffValue gain/loss hooks.
    pub(super) fn set_agility_from_original_card_291(
        &mut self,
        actor_side: PlayerSide,
        value: i64,
    ) -> i64 {
        self.commit_combat_resource_value(actor_side, CombatResource::Agility, value)
    }

    /// Card_4000065.cs:84-91 calls RemoveBuff(XingLi), deliberately bypassing
    /// ModifyBuffValue's Star Power loss conversion.
    pub(super) fn remove_star_power_from_original_card_4000065(
        &mut self,
        actor_side: PlayerSide,
    ) -> i64 {
        self.commit_combat_resource_value(actor_side, CombatResource::StarPower, 0)
    }

    /// 七星借命 FateStrategy 436 的清空星力（BattleExecuter.
    /// CharacterResurrectionCheckAsync 的 RemoveBuff(XingLi)）：同为原始
    /// 移除，绕过 ModifyBuffValue 的星力流失转换钩子（如 422 紫芒星爆）。
    pub(super) fn remove_all_star_power_for_qi_xing_jie_ming(
        &mut self,
        actor_side: PlayerSide,
    ) -> i64 {
        self.commit_combat_resource_value(actor_side, CombatResource::StarPower, 0)
    }

    pub(super) fn increase_dream_mirage_action_again_limit(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
        cap: i64,
    ) -> i64 {
        if amount <= 0 {
            return 0;
        }
        let before = self.actor(actor_side).dream_mirage.action_again_limit;
        let after = (before + amount).min(cap.max(0));
        self.actor_mut(actor_side).dream_mirage.action_again_limit = after;
        after - before
    }

    pub(super) fn dream_mirage_action_again_limit(&self, actor_side: PlayerSide) -> i64 {
        self.actor(actor_side).dream_mirage.action_again_limit
    }

    /// BattleCharacter.ModifyHp/ModifyDef:9758-9767,10060-10088.
    /// `resolved_delta` is the post-modifier positive request, not the
    /// capped actual gain; overhealing therefore still reflects the full
    /// original amount.
    pub(super) fn apply_dream_mirage_positive_resource_gain_damage(
        &mut self,
        actor_side: PlayerSide,
        resolved_delta: i64,
        include_flat_defense_damage: bool,
    ) -> i64 {
        if resolved_delta <= 0 {
            return 0;
        }
        let flat = if include_flat_defense_damage {
            self.dream_mirage_value(actor_side, DreamMirageValue::DefenseGainDamageLow)
                .max(0)
        } else {
            0
        };
        if flat > 0 {
            self.apply_damage(actor_side, flat, false, false, false);
        }
        let percent = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamDefenseGainDamage)
            .max(0);
        if percent <= 0
            || self.dream_mirage_value(actor_side, DreamMirageValue::DreamDefenseGainDamageGuard)
                > 0
        {
            return flat;
        }
        let damage = resolved_delta * percent / 100;
        if damage <= 0 {
            return flat;
        }
        self.modify_dream_mirage_value(
            actor_side,
            DreamMirageValue::DreamDefenseGainDamageGuard,
            1,
        );
        self.apply_damage(actor_side, damage, false, false, false);
        self.modify_dream_mirage_value(
            actor_side,
            DreamMirageValue::DreamDefenseGainDamageGuard,
            -1,
        );
        flat + damage
    }

    pub(super) fn apply_dream_mirage_hp_loss_modifier(
        &self,
        actor_side: PlayerSide,
        delta: i64,
    ) -> i64 {
        if delta < 0
            && self.dream_mirage_value(actor_side, DreamMirageValue::DreamUnmovingFormation) > 0
        {
            -1
        } else {
            delta
        }
    }

    pub(super) fn apply_dream_mirage_hp_loss_hooks(
        &mut self,
        actor_side: PlayerSide,
        actual_delta: i64,
    ) {
        if actual_delta < 0 && self.dream_mirage_value(actor_side, DreamMirageValue::DreamCliff) > 0
        {
            let before = self.actor(actor_side).turn.next_turn_defense;
            self.actor_mut(actor_side).turn.next_turn_defense += -actual_delta;
            let after = self.actor(actor_side).turn.next_turn_defense;
            self.record_counter_transition(
                actor_side,
                "回合",
                "nextTurnDefense",
                "下回合加防",
                before,
                after,
            );
        }
    }

    pub(super) fn gain_agility(&mut self, actor_side: PlayerSide, amount: i64) {
        self.modify_agility(actor_side, amount);
    }

    pub(super) fn gain_water_momentum(&mut self, actor_side: PlayerSide, amount: i64) {
        if amount <= 0 {
            return;
        }
        self.actor_mut(actor_side).elements.water_momentum += amount;
        self.actor_mut(actor_side)
            .elements
            .water_momentum_gain_count += 1;
        self.modify_dream_mirage_value(
            actor_side,
            DreamMirageValue::TotalWaterMomentumGained,
            amount,
        );
    }

    pub(super) fn gain_anima(&mut self, actor_side: PlayerSide, amount: i64) {
        let mut amount = amount;
        if amount <= 0 {
            return;
        }
        let multiplier = self.actor(actor_side).chance.qi_cai_ling_he.max(0);
        if multiplier > 0 {
            amount *= multiplier;
        }
        if self.dream_mirage_value(actor_side, DreamMirageValue::HalfAnimaGain) > 0 {
            amount /= 2;
        }
        if amount <= 0 {
            return;
        }
        // BattleCharacter.ModifyAnima：凡躯(204) 把加灵转成体魄；但得炁(208) 后
        // 入战按灵炁奔涌走棍/拳分支，共鸣·得炁(137) 也会关闭此转换。镜像
        // engine-ts resources.ts modifyAnima 的 `!includes(208) && resonance!=137`
        // 排除，否则李㵘棍系(再次行动加灵)在 rust 后端会被凡躯把灵气清成 0 → 卡灵。
        let identity = &self.actor(actor_side).identity;
        if identity.talents.contains(&204)
            && !identity.talents.contains(&208)
            && identity.talent_resonance_id != Some(137)
        {
            self.apply_physique_amount(actor_side, amount);
            return;
        }
        if self.actor(actor_side).fate.wu_you_ling_niang > 0 {
            // 天衍-无忧灵酿（FateStrategy 407 / BuffType 767）：原版
            // 在所有加灵倍率、减半和凡躯转换完成后，确认实际增加了正向灵气
            // 才消费一层；随后按 otherParams[0]=4 依次进入 ModifyMaxHp、
            // ModifyHp。先扣层再进入 HP 管线，避免其递归副作用重复消费同一层。
            self.actor_mut(actor_side).fate.wu_you_ling_niang -= 1;
            self.modify_actor_max_hp(actor_side, 4);
            self.modify_actor_hp(actor_side, 4, false, false);
        }
        let anima_before = self.actor(actor_side).core.anima;
        self.actor_mut(actor_side).core.anima += amount;
        let anima_after = self.actor(actor_side).core.anima;
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            "核心",
            "anima",
            "灵气",
            anima_before,
            anima_after,
            anima_after - anima_before,
        );
        self.actor_mut(actor_side).turn.anima_gain_count += 1;
        self.apply_six_yao_anima_gain_damage(actor_side, amount);
        let defense = self
            .actor(actor_side)
            .turn
            .spirit_control_anima_gain_defense
            * amount;
        if defense > 0 {
            self.gain_defense(actor_side, defense);
        }
        if self.actor(actor_side).identity.talents.contains(&267) {
            // TalentConfig 267 otherParams[0]; Steam build 24217566 lowered 5 -> 3.
            let remaining = (3 - self.actor(actor_side).turn.wind_spirit_body_forge_count).max(0);
            let physique_gain = amount.min(remaining);
            if physique_gain > 0 {
                self.actor_mut(actor_side).turn.wind_spirit_body_forge_count += physique_gain;
                self.apply_physique_amount(actor_side, physique_gain);
            }
        }
        let quiet_healing = self.actor(actor_side).fate.quiet_mindset * amount;
        if quiet_healing > 0 {
            self.modify_actor_hp(actor_side, quiet_healing, false, false);
        }
        let ling_gua_art = self.actor(actor_side).astrology.ling_gua_art.max(0);
        if ling_gua_art > 0 {
            self.gain_hexagram(actor_side, amount * ling_gua_art);
        }
        let anima_to_star_power = self.actor(actor_side).astrology.anima_to_star_power.max(0);
        if anima_to_star_power > 0 {
            let converted = amount.min(anima_to_star_power);
            if converted > 0 {
                self.actor_mut(actor_side).astrology.anima_to_star_power -= converted;
                self.modify_star_power(actor_side, converted);
            }
        }
        let two_polarity_hp = self.actor(actor_side).elements.dream_two_polarity_hp.max(0);
        if two_polarity_hp > 0 {
            self.modify_actor_hp(actor_side, amount * two_polarity_hp, false, false);
        }
        let two_polarity_defense = self
            .actor(actor_side)
            .elements
            .dream_two_polarity_defense
            .max(0);
        if two_polarity_defense > 0 {
            self.gain_defense(actor_side, amount * two_polarity_defense);
        }
        let dream_defense = amount
            * self
                .dream_mirage_value(actor_side, DreamMirageValue::AnimaGainDefense)
                .max(0);
        if dream_defense > 0 {
            self.gain_defense(actor_side, dream_defense);
        }
        if self.actor(actor_side).fate.wild_ferry_seal > 0 {
            self.actor_mut(actor_side).fate.wild_ferry_seal -= 1;
            self.modify_extra_actions(actor_side, 1);
        }
        let feng_ling_zhan_yi = self.actor(actor_side).fate.feng_ling_zhan_yi.max(0);
        if feng_ling_zhan_yi > 0 {
            self.actor_mut(actor_side).fate.feng_ling_zhan_yi = 0;
            self.gain_agility(actor_side, feng_ling_zhan_yi);
        }
        if self.actor(actor_side).identity.talents.contains(&195)
            && !self.actor(actor_side).astrology.pending_anima_hexagram
        {
            self.actor_mut(actor_side).astrology.pending_anima_hexagram = true;
        }
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::TotalAnimaGained, amount);
    }

    pub(super) fn apply_configured_defense(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if let Some(defense) = card.defense.filter(|value| *value > 0) {
            self.gain_defense(actor_side, defense);
        }
    }

    pub(super) fn reduce_anima_unchecked(&mut self, actor_side: PlayerSide, amount: i64) {
        if amount <= 0 {
            return;
        }
        let before = self.actor(actor_side).core.anima;
        self.actor_mut(actor_side).core.anima = (before - amount).max(0);
        let after = self.actor(actor_side).core.anima;
        let actual_loss = before - after;
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            "核心",
            "anima",
            "灵气",
            before,
            after,
            after - before,
        );
        let defense = self
            .actor(actor_side)
            .turn
            .spirit_control_anima_loss_defense
            * actual_loss;
        if defense > 0 {
            self.gain_defense(actor_side, defense);
        }
        let trailing_shadow = self
            .actor(actor_side)
            .sword
            .hundred_bird_trailing_shadow_art
            .max(0);
        if actual_loss > 0 && trailing_shadow > 0 {
            self.actor_mut(actor_side).sword.sword_energy += actual_loss * trailing_shadow;
        }
    }

    pub(super) fn decay_actor_defense(&mut self, actor_side: PlayerSide) {
        let defense = self.actor(actor_side).core.defense;
        let loss = div_ceil(defense, 2);
        if loss > 0 {
            self.lose_defense(actor_side, loss);
        }
    }

    pub(super) fn decay_actor_defense_percent(&mut self, actor_side: PlayerSide, percent: i64) {
        let defense = self.actor(actor_side).core.defense;
        let loss = div_ceil(defense * percent.max(0), 100);
        if loss > 0 {
            self.lose_defense(actor_side, loss);
        }
    }

    pub(super) fn modify_temp_life(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        let before = self.actor(actor_side).core.temp_life;
        self.actor_mut(actor_side).core.temp_life = (before + delta).max(0);
        self.actor(actor_side).core.temp_life - before
    }

    pub(super) fn modify_guard(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        let before = self.actor(actor_side).core.guard;
        self.actor_mut(actor_side).core.guard = (before + delta).max(0);
        self.actor(actor_side).core.guard - before
    }

    pub(super) fn gain_guard(&mut self, actor_side: PlayerSide, amount: i64) -> i64 {
        if amount <= 0 {
            return 0;
        }
        self.modify_guard(actor_side, amount)
    }

    pub(super) fn gain_temporary_guard(&mut self, actor_side: PlayerSide, amount: i64) -> i64 {
        let actual_gain = self.gain_guard(actor_side, amount);
        if actual_gain > 0 {
            self.actor_mut(actor_side).core.temporary_guard += actual_gain;
        }
        actual_gain
    }

    pub(super) fn set_guard(&mut self, actor_side: PlayerSide, value: i64) -> i64 {
        let before = self.actor(actor_side).core.guard;
        self.actor_mut(actor_side).core.guard = value.max(0);
        self.actor(actor_side).core.guard - before
    }

    pub(super) fn gain_defense(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) -> DefenseMutationReceipt {
        let receipt = self.gain_defense_inner(actor_side, amount);
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Defense,
            "核心",
            "defense",
            "防",
            receipt.before,
            receipt.after,
            receipt.visible_delta,
        );
        receipt
    }

    fn gain_defense_inner(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) -> DefenseMutationReceipt {
        let before = self.actor(actor_side).core.defense;
        if amount <= 0 {
            return DefenseMutationReceipt {
                requested: amount,
                applied: 0,
                visible_delta: 0,
                before,
                after: before,
            };
        }
        if self.dream_mirage_value(actor_side, DreamMirageValue::CannotGainDefense) > 0 {
            return DefenseMutationReceipt {
                requested: amount,
                applied: 0,
                visible_delta: 0,
                before,
                after: before,
            };
        }
        let mut delta = amount;
        // BattleCharacter.ModifyDef (BattleCharacter.cs:10075): every positive
        // defense gain gets +1 while fate strategy 382 (霜华守护) is active.
        // oracle 锚点: mirror-32219000 3cba3714dc255204/round-12 checkpoint[2]
        // 云剑•点星 p2.defense 3 (def 0+2+1；引擎缺 +1 为 2)。
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&382)
        {
            delta += 1;
        }
        let metal_ring = self.actor(actor_side).sword.metal_ring.max(0);
        if metal_ring > 0 {
            delta += metal_ring;
        }
        if self.actor(actor_side).turn.adaptation > 0 {
            delta += div_ceil(delta * 40, 100);
        }
        delta += self.actor(actor_side).mirage_ronghui.molten_ring.max(0);
        self.actor_mut(actor_side).core.defense += delta;
        self.actor_mut(actor_side)
            .dream_mirage
            .defense_gain_event_count += 1;
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::DefenseLedger, delta);
        self.apply_heavenly_secret_reverse_from_gain(actor_side, delta);
        self.apply_dream_mirage_positive_resource_gain_damage(actor_side, delta, true);
        let after = self.actor(actor_side).core.defense;
        DefenseMutationReceipt {
            requested: amount,
            applied: delta,
            visible_delta: after - before,
            before,
            after,
        }
    }

    pub(super) fn lose_defense(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) -> DefenseMutationReceipt {
        let receipt = self.lose_defense_inner(actor_side, amount);
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Defense,
            "核心",
            "defense",
            "防",
            receipt.before,
            receipt.after,
            receipt.visible_delta,
        );
        receipt
    }

    fn lose_defense_inner(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) -> DefenseMutationReceipt {
        let before = self.actor(actor_side).core.defense;
        if amount <= 0 {
            return DefenseMutationReceipt {
                requested: amount,
                applied: 0,
                visible_delta: 0,
                before,
                after: before,
            };
        }
        self.actor_mut(actor_side).core.defense = (before - amount).max(0);
        let actual_loss = before - self.actor(actor_side).core.defense;
        if actual_loss <= 0 {
            return DefenseMutationReceipt {
                requested: amount,
                applied: 0,
                visible_delta: 0,
                before,
                after: before,
            };
        }
        let lost_before = self.actor(actor_side).turn.lost_defense_count;
        self.actor_mut(actor_side).turn.lost_defense_count += actual_loss;
        let lost_after = self.actor(actor_side).turn.lost_defense_count;
        self.record_counter_transition(
            actor_side,
            "回合",
            "lostDefenseCount",
            "失防次数",
            lost_before,
            lost_after,
        );
        if self.actor(actor_side).elements.earth_cliff_counter > 0 {
            self.actor_mut(actor_side).elements.earth_cliff_counter -= 1;
            self.apply_damage(actor_side, actual_loss, false, false, false);
        }
        if self.dream_mirage_value(actor_side, DreamMirageValue::DreamCliff) > 0 {
            let before = self.actor(actor_side).turn.next_turn_defense;
            self.actor_mut(actor_side).turn.next_turn_defense += actual_loss;
            let after = self.actor(actor_side).turn.next_turn_defense;
            self.record_counter_transition(
                actor_side,
                "回合",
                "nextTurnDefense",
                "下回合加防",
                before,
                after,
            );
        }
        if self.actor(actor_side).elements.earth_eight_wastes > 0 {
            self.actor_mut(actor_side).elements.earth_eight_wastes -= 1;
            self.gain_defense(actor_side, actual_loss);
        }
        let after = self.actor(actor_side).core.defense;
        DefenseMutationReceipt {
            requested: amount,
            applied: actual_loss,
            visible_delta: after - before,
            before,
            after,
        }
    }

    pub(super) fn gain_sharpness(&mut self, actor_side: PlayerSide, amount: i64) {
        if amount <= 0 {
            return;
        }
        let metal_ring = self.actor(actor_side).sword.metal_ring.max(0);
        let delta = amount + metal_ring;
        if self
            .actor(actor_side)
            .mirage_ronghui
            .mirage_sharpness_conversion_turns
            > 0
        {
            let target_side = opponent_side(actor_side);
            let loss = delta * 2;
            self.modify_actor_hp(target_side, -loss, false, false);
            self.modify_actor_max_hp(target_side, -loss);
            return;
        }
        self.actor_mut(actor_side).sword.sharpness += delta;
        self.actor_mut(actor_side)
            .dream_mirage
            .sharpness_gain_event_count += 1;
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::TotalSharpnessGained, delta);
    }

    pub(super) fn apply_configured_physique(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if let Some(physique) = card.physique.filter(|value| *value > 0) {
            self.apply_physique_amount(actor_side, physique);
        }
    }

    pub(super) fn apply_elixir_base(&mut self, actor_side: PlayerSide, card: &CardDefinition) {
        self.apply_configured_anima(actor_side, card);
        self.apply_configured_defense(actor_side, card);
        self.apply_configured_physique(actor_side, card);
    }

    pub(super) fn apply_physique_amount(&mut self, actor_side: PlayerSide, amount: i64) {
        if amount <= 0 {
            return;
        }
        let before = self.actor(actor_side).core.physique;
        let limit = self.actor(actor_side).core.physique_limit;
        self.actor_mut(actor_side).core.physique += amount;
        self.modify_actor_max_hp(actor_side, amount);
        let after = self.actor(actor_side).core.physique;
        let actual_delta = after - before;
        let excess = if actual_delta > 0 {
            actual_delta.min((after - limit).max(0))
        } else {
            0
        };
        if excess > 0 {
            let extra_healing = excess
                * self
                    .dream_mirage_value(actor_side, DreamMirageValue::ExcessPhysiqueHp)
                    .max(0);
            if extra_healing > 0 {
                self.modify_actor_hp(actor_side, extra_healing, false, false);
            }
            let extra_damage = excess
                * self
                    .dream_mirage_value(actor_side, DreamMirageValue::ExcessPhysiqueDamage)
                    .max(0);
            if extra_damage > 0 {
                self.apply_damage(actor_side, extra_damage, false, false, false);
            }
        }
        let healing = (after - limit).max(0) - (before - limit).max(0);
        if healing > 0 {
            self.modify_actor_hp(actor_side, healing, false, false);
        }
        if actual_delta > 0 {
            self.actor_mut(actor_side).turn.battle_physique_gain_count += actual_delta;
            if self.actor(actor_side).identity.talents.contains(&184) {
                self.gain_defense(actor_side, actual_delta.min(5));
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&163)
                && self.actor(actor_side).fate.chan_xin_ju_ling_triggered <= 0
            {
                self.actor_mut(actor_side).fate.chan_xin_ju_ling_triggered = 1;
                self.gain_anima(actor_side, 1);
            }
        }
    }

    pub(super) fn modify_physique_amount(&mut self, actor_side: PlayerSide, amount: i64) -> i64 {
        if amount == 0 {
            return 0;
        }
        if amount > 0 {
            self.apply_physique_amount(actor_side, amount);
            return amount;
        }
        let before = self.actor(actor_side).core.physique;
        let after = (before + amount).max(0);
        let actual_delta = after - before;
        if actual_delta < 0 {
            self.actor_mut(actor_side).core.physique = after;
            self.modify_actor_max_hp(actor_side, actual_delta);
        }
        actual_delta
    }

    pub(super) fn modify_target_hp(&mut self, actor_side: PlayerSide, delta: i64) {
        let target = opponent_side(actor_side);
        self.modify_actor_hp(target, delta, false, false);
    }

    pub(super) fn modify_target_max_hp(&mut self, actor_side: PlayerSide, delta: i64) {
        let target = opponent_side(actor_side);
        self.modify_actor_max_hp(target, delta);
    }

    pub(super) fn modify_momentum_limit(&mut self, actor_side: PlayerSide, delta: i64) {
        if delta == 0 {
            return;
        }
        let (limit, overflow) = {
            let actor = self.actor_mut(actor_side);
            actor.beng.momentum_limit = (actor.beng.momentum_limit + delta).max(0);
            let limit = actor.beng.momentum_limit;
            (limit, (actor.beng.momentum - limit).max(0))
        };
        if overflow > 0 {
            // BattleCharacter caps QiShi with SetBuffValue: this raw clamp
            // intentionally bypasses ordinary momentum gain/loss hooks.
            self.commit_combat_resource_value(actor_side, CombatResource::Momentum, limit);
            self.gain_defense(actor_side, overflow);
            self.apply_sheng_qi_ling_ren_damage(actor_side, overflow);
        }
    }

    pub(super) fn modify_momentum(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
    ) -> MomentumMutationReceipt {
        let receipt = self.modify_momentum_inner(actor_side, delta);
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Momentum,
            "锻玄",
            "momentum",
            "气势",
            receipt.before,
            receipt.after,
            receipt.visible_delta,
        );
        receipt
    }

    fn modify_momentum_inner(
        &mut self,
        actor_side: PlayerSide,
        requested_delta: i64,
    ) -> MomentumMutationReceipt {
        let mut delta = requested_delta;
        if delta > 0 {
            let pending = self.actor(actor_side).beng.pending_momentum_bonus.max(0);
            if pending > 0 {
                // Original ModifyBuffValue(QiShi) adds Buff 769 before the
                // shared resource write and removes it immediately. Keep the
                // adjusted delta on the same path so all existing positive
                // hooks, upper-limit overflow and receipts observe it.
                delta += pending;
                self.actor_mut(actor_side).beng.pending_momentum_bonus = 0;
            }
        }
        let before = self.actor(actor_side).beng.momentum;
        let limit = self.actor(actor_side).beng.momentum_limit.max(0);
        // BattleCharacter.ModifyBuffValue first clamps only at zero. QiShi's
        // upper limit is a later post-hook, so positive hooks still observe
        // gains that ultimately overflow into Defense.
        let hook_delta =
            self.commit_combat_resource_value(actor_side, CombatResource::Momentum, before + delta);
        if hook_delta < 0 {
            let refill =
                super::cards_synthetic_oracle_verified_secret_misc::unceasing_momentum_refill(
                    self.actor(actor_side),
                );
            if refill > 0 {
                self.modify_momentum(actor_side, refill);
            }
        }
        if hook_delta < 0 {
            self.apply_sheng_qi_ling_ren_damage(actor_side, -hook_delta);
        }
        if hook_delta > 0
            && self.actor(actor_side).has_ling_qi_ben_yong()
            && self.actor(actor_side).beng.quan_stance > 0
            && self.actor(actor_side).beng.momentum_gain_agility_triggered <= 0
        {
            self.actor_mut(actor_side)
                .beng
                .momentum_gain_agility_triggered = 1;
            self.modify_agility(actor_side, 2);
        }
        if hook_delta > 0 {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::TotalMomentumGained,
                hook_delta,
            );
        }
        // BattleCharacter.ModifyBuffValue:8979-8987. Both hooks observe the
        // lower-clamped actual delta, and run before the upper-limit overflow
        // is converted into Defense.
        if hook_delta != 0 {
            let ke_yin_109_multiplier = self
                .actor(actor_side)
                .identity
                .ke_yin_card_ids
                .iter()
                .filter(|card_id| card_id.rem_euclid(10_000) == 109)
                .count() as i64;
            if ke_yin_109_multiplier > 0 {
                self.gain_defense(actor_side, hook_delta.abs() * ke_yin_109_multiplier);
            }
        }
        let overflow = (self.actor(actor_side).beng.momentum - limit).max(0);
        if overflow > 0 {
            self.commit_combat_resource_value(actor_side, CombatResource::Momentum, limit);
            self.gain_defense(actor_side, overflow);
            self.apply_sheng_qi_ling_ren_damage(actor_side, overflow);
        }
        let after = self.actor(actor_side).beng.momentum;
        MomentumMutationReceipt {
            requested_delta,
            hook_delta,
            visible_delta: after - before,
            overflow_delta: overflow,
            before,
            after,
        }
    }

    pub(super) fn apply_sheng_qi_ling_ren_damage(&mut self, actor_side: PlayerSide, units: i64) {
        let damage_per_unit = self.actor(actor_side).fate.sheng_qi_ling_ren.max(0);
        let amount = units.max(0) * damage_per_unit;
        if amount <= 0 {
            return;
        }
        let target_side = opponent_side(actor_side);
        // BattleCharacter.ModifyBuffValue(QiShi) constructs DamageInfo.Damage
        // with skipWoundCheck=true. The ordinary damage kernel still owns all
        // non-wound mitigation, defense, guard and HP hooks.
        self.apply_damage_to(actor_side, target_side, amount, false, false, false);
    }
}
