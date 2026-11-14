use super::effect_invocation::{
    TemporaryDeckIdentityMode, TemporaryHadUsedSource, TemporaryInvocationSpec,
};
use super::flow_card_effect::{param0, param1, param2};
use super::original_config::{original_card_definition, original_card_realm_level};
use super::support::{
    card_rarity, div_ceil_signed, has_cloud_chain, normalized_base_id, opponent_side,
    other_param, other_param_or, wu_xing_count_in_deck,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};
use std::collections::BTreeSet;

#[cfg(test)]
pub(super) const SYNTHETIC_ORACLE_WAVE018_CARD_IDS: [i64; 25] = [
    278, 289, 292, 293, 315, 316, 317, 318, 323, 122, 123, 124, 125, 127, 128, 129, 130, 131, 133,
    135, 137, 138, 139, 141, 142,
];

#[cfg(test)]
pub(super) const SYNTHETIC_ORACLE_WAVE019_CARD_IDS: [i64; 27] = [
    144, 151, 152, 153, 154, 156, 157, 158, 159, 160, 161, 162, 163, 164, 165, 166, 167, 168, 170,
    171, 172, 173, 174, 175, 176, 39, 7_000_093,
];

/// Exact state keys shared by the 52 wave018/019 card bodies and their
/// lifecycle hooks.  `ReplayState::mirage_ronghui_value` and
/// `ReplayState::modify_mirage_ronghui_value` are deliberately owned by the
/// shared-runtime integration: a card arm in this module must not silently
/// omit one of these mutations merely because the state does not belong to the
/// immediate card transaction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum MirageRonghuiValue {
    MirageAnimaAttackCards,
    MirageInternalInjuryAmplifierTurns,
    MirageSwordIntentRefund,
    MirageSharpnessConversionTurns,
    MirageHealingConversionTurns,
    MirageWaterDefenseCap,
    MirageWaterDefenseUncapped,
    InternalInjuryExtraTriggers,
    OrdinarySwordActionAgainCards,
    InfinityPlate,
    SixYaoFanDamage,
    CounterElementAnima,
    CounterElementDefense,
    MoltenRing,
    FirstHpLossReward,
    FirstHpLossRewardTriggered,
    CrashFistStarSeize,
    CrashFistStarSeizeConsumed,
    BilateralTurnEndGrowth,
    BilateralTurnEndLoss,
    HpLossAttackBonusCharges,
    LastTurnStartHp,
    ThisTurnStartHp,
    NineHeavensRevive,
    DoubleHpAtTurnStart,
    TemporaryCopyDepth,
    ActionAgainIgnoresBinding,
    FiveElementsCardsUsed,
    CannotGainHp,
}

/*
Wave 018-019 card-body integration contract
============================================

Every match arm below contains the complete card-local effect.  State that
outlives the immediate transaction goes through the intentionally explicit
`mirage_ronghui_value` / `modify_mirage_ronghui_value` interface.  Therefore this
module must not be declared or promoted until the shared runtime supplies the
following exact interface and hooks:

  Required ReplayState interface
    mirage_ronghui_value(side, MirageRonghuiValue) -> i64
    modify_mirage_ronghui_value(side, MirageRonghuiValue, delta)
    mirage_ronghui_last_round_exp(side) -> i64
    modify_star_power(side, delta)
    modify_actor_max_hp(side, delta)
    apply_mirage_ronghui_damage(source, target, amount, ignore_defense)
    execute_mirage_ronghui_temporary_card(actor, outer_slot, selected_id, virtual_slot)

  ReplayMirageRonghuiState fields
    mirage_anima_attack_cards
    mirage_internal_injury_amplifier_turns
    mirage_sword_intent_refund
    mirage_sharpness_conversion_turns
    mirage_healing_conversion_turns
    mirage_water_defense_cap
    mirage_water_defense_uncapped
    internal_injury_extra_triggers
    ordinary_sword_action_again_cards
    infinity_plate
    six_yao_fan_damage
    counter_element_anima / counter_element_defense
    molten_ring
    first_hp_loss_reward / first_hp_loss_reward_triggered
    crash_fist_star_seize / crash_fist_star_seize_consumed
    bilateral_turn_end_growth / bilateral_turn_end_loss
    hp_loss_attack_bonus_charges
    last_turn_start_hp / this_turn_start_hp
    nine_heavens_revive
    double_hp_at_turn_start
    temporary_copy_depth / action_again_ignores_binding
    five_elements_cards_used
    cannot_gain_hp

  Other state
    last_round_exp (Card 323)

  Shared hooks (all are required; none is an optional approximation)
    battle opening: Card 315 element activation
    attack pre/post: 278, 144, 154 and 160
    resource mutation: 289, 315-317, 131 and 135
    HP mutation: 139 and 160
    before-card/completed-card: 278, 323, 133, 144 and 7_000_093
    sword-intent settlement: 293
    turn start/end: 289, 315-318, 128, 139, 158, 162 and 175
    resurrection settlement: 172
    guarded full temporary-card transaction: 173
    dynamic action-again: 122, 127, 152 and 165

The immediate bodies intentionally call the missing shared-state interface, so
linking this module before that integration fails loudly instead of producing a
plausible but incomplete replay.
*/

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn mirage_ronghui_value(
        &self,
        actor_side: PlayerSide,
        value: MirageRonghuiValue,
    ) -> i64 {
        let state = &self.actor(actor_side).mirage_ronghui;
        match value {
            MirageRonghuiValue::MirageAnimaAttackCards => state.mirage_anima_attack_cards,
            MirageRonghuiValue::MirageInternalInjuryAmplifierTurns => {
                state.mirage_internal_injury_amplifier_turns
            }
            MirageRonghuiValue::MirageSwordIntentRefund => state.mirage_sword_intent_refund,
            MirageRonghuiValue::MirageSharpnessConversionTurns => {
                state.mirage_sharpness_conversion_turns
            }
            MirageRonghuiValue::MirageHealingConversionTurns => {
                state.mirage_healing_conversion_turns
            }
            MirageRonghuiValue::MirageWaterDefenseCap => state.mirage_water_defense_cap,
            MirageRonghuiValue::MirageWaterDefenseUncapped => state.mirage_water_defense_uncapped,
            MirageRonghuiValue::InternalInjuryExtraTriggers => state.internal_injury_extra_triggers,
            MirageRonghuiValue::OrdinarySwordActionAgainCards => {
                state.ordinary_sword_action_again_cards
            }
            MirageRonghuiValue::InfinityPlate => state.infinity_plate,
            MirageRonghuiValue::SixYaoFanDamage => state.six_yao_fan_damage,
            MirageRonghuiValue::CounterElementAnima => state.counter_element_anima,
            MirageRonghuiValue::CounterElementDefense => state.counter_element_defense,
            MirageRonghuiValue::MoltenRing => state.molten_ring,
            MirageRonghuiValue::FirstHpLossReward => state.first_hp_loss_reward,
            MirageRonghuiValue::FirstHpLossRewardTriggered => state.first_hp_loss_reward_triggered,
            MirageRonghuiValue::CrashFistStarSeize => state.crash_fist_star_seize,
            MirageRonghuiValue::CrashFistStarSeizeConsumed => state.crash_fist_star_seize_consumed,
            MirageRonghuiValue::BilateralTurnEndGrowth => state.bilateral_turn_end_growth,
            MirageRonghuiValue::BilateralTurnEndLoss => state.bilateral_turn_end_loss,
            MirageRonghuiValue::HpLossAttackBonusCharges => state.hp_loss_attack_bonus_charges,
            MirageRonghuiValue::LastTurnStartHp => state.last_turn_start_hp,
            MirageRonghuiValue::ThisTurnStartHp => state.this_turn_start_hp,
            MirageRonghuiValue::NineHeavensRevive => state.nine_heavens_revive,
            MirageRonghuiValue::DoubleHpAtTurnStart => state.double_hp_at_turn_start,
            MirageRonghuiValue::TemporaryCopyDepth => state.temporary_copy_depth,
            MirageRonghuiValue::ActionAgainIgnoresBinding => state.action_again_ignores_binding,
            MirageRonghuiValue::FiveElementsCardsUsed => state.five_elements_cards_used,
            MirageRonghuiValue::CannotGainHp => state.cannot_gain_hp,
        }
    }

    pub(super) fn modify_mirage_ronghui_value(
        &mut self,
        actor_side: PlayerSide,
        value: MirageRonghuiValue,
        delta: i64,
    ) -> i64 {
        let state = &mut self.actor_mut(actor_side).mirage_ronghui;
        let field = match value {
            MirageRonghuiValue::MirageAnimaAttackCards => &mut state.mirage_anima_attack_cards,
            MirageRonghuiValue::MirageInternalInjuryAmplifierTurns => {
                &mut state.mirage_internal_injury_amplifier_turns
            }
            MirageRonghuiValue::MirageSwordIntentRefund => &mut state.mirage_sword_intent_refund,
            MirageRonghuiValue::MirageSharpnessConversionTurns => {
                &mut state.mirage_sharpness_conversion_turns
            }
            MirageRonghuiValue::MirageHealingConversionTurns => {
                &mut state.mirage_healing_conversion_turns
            }
            MirageRonghuiValue::MirageWaterDefenseCap => &mut state.mirage_water_defense_cap,
            MirageRonghuiValue::MirageWaterDefenseUncapped => {
                &mut state.mirage_water_defense_uncapped
            }
            MirageRonghuiValue::InternalInjuryExtraTriggers => {
                &mut state.internal_injury_extra_triggers
            }
            MirageRonghuiValue::OrdinarySwordActionAgainCards => {
                &mut state.ordinary_sword_action_again_cards
            }
            MirageRonghuiValue::InfinityPlate => &mut state.infinity_plate,
            MirageRonghuiValue::SixYaoFanDamage => &mut state.six_yao_fan_damage,
            MirageRonghuiValue::CounterElementAnima => &mut state.counter_element_anima,
            MirageRonghuiValue::CounterElementDefense => &mut state.counter_element_defense,
            MirageRonghuiValue::MoltenRing => &mut state.molten_ring,
            MirageRonghuiValue::FirstHpLossReward => &mut state.first_hp_loss_reward,
            MirageRonghuiValue::FirstHpLossRewardTriggered => {
                &mut state.first_hp_loss_reward_triggered
            }
            MirageRonghuiValue::CrashFistStarSeize => &mut state.crash_fist_star_seize,
            MirageRonghuiValue::CrashFistStarSeizeConsumed => {
                &mut state.crash_fist_star_seize_consumed
            }
            MirageRonghuiValue::BilateralTurnEndGrowth => &mut state.bilateral_turn_end_growth,
            MirageRonghuiValue::BilateralTurnEndLoss => &mut state.bilateral_turn_end_loss,
            MirageRonghuiValue::HpLossAttackBonusCharges => &mut state.hp_loss_attack_bonus_charges,
            MirageRonghuiValue::LastTurnStartHp => &mut state.last_turn_start_hp,
            MirageRonghuiValue::ThisTurnStartHp => &mut state.this_turn_start_hp,
            MirageRonghuiValue::NineHeavensRevive => &mut state.nine_heavens_revive,
            MirageRonghuiValue::DoubleHpAtTurnStart => &mut state.double_hp_at_turn_start,
            MirageRonghuiValue::TemporaryCopyDepth => &mut state.temporary_copy_depth,
            MirageRonghuiValue::ActionAgainIgnoresBinding => {
                &mut state.action_again_ignores_binding
            }
            MirageRonghuiValue::FiveElementsCardsUsed => &mut state.five_elements_cards_used,
            MirageRonghuiValue::CannotGainHp => &mut state.cannot_gain_hp,
        };
        let before = *field;
        *field = (*field + delta).max(0);
        *field - before
    }

    pub(super) fn mirage_ronghui_last_round_exp(&self, actor_side: PlayerSide) -> i64 {
        self.actor(actor_side).identity.last_round_exp
    }

    pub(super) fn apply_mirage_ronghui_damage(
        &mut self,
        source_side: PlayerSide,
        target_side: PlayerSide,
        amount: i64,
        ignore_defense: bool,
    ) {
        self.apply_damage_to(
            source_side,
            target_side,
            amount,
            false,
            ignore_defense,
            false,
        );
    }

    pub(super) fn apply_six_yao_buff_gain_damage(&mut self, actor_side: PlayerSide, units: i64) {
        let multiplier = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::SixYaoFanDamage);
        let damage = units.max(0) * multiplier.max(0);
        if damage > 0 {
            // BattleCharacter.ModifyBuffValue creates ordinary Damage with
            // `hitDef: false`. That flag records whether defense was hit; it
            // does not bypass the defense-first branch in ApplyDamage.
            self.apply_mirage_ronghui_damage(actor_side, opponent_side(actor_side), damage, false);
        }
    }

    pub(super) fn apply_six_yao_anima_gain_damage(
        &mut self,
        actor_side: PlayerSide,
        anima_gain: i64,
    ) {
        // Steam build 24217566: BattleCharacter.ModifyAnima dropped the `* 2 / 3`
        // on the XingYueQianKunShan anima branch.
        let multiplier = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::SixYaoFanDamage);
        let damage = anima_gain.max(0) * multiplier.max(0);
        if damage > 0 {
            self.apply_mirage_ronghui_damage(actor_side, opponent_side(actor_side), damage, false);
        }
    }

    pub(super) fn initialize_last_turn_start_hp(&mut self, actor_side: PlayerSide) {
        let hp = self.actor(actor_side).core.hp;
        let current = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::LastTurnStartHp);
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::LastTurnStartHp,
            hp - current,
        );
    }

    pub(super) fn apply_counter_element_before_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        let anima = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::CounterElementAnima);
        if anima <= 0 {
            return false;
        }
        let Some(next_grid) = self.mirage_ronghui_next_grid(actor_side, slot) else {
            return false;
        };
        let Some(next_name) = self
            .actor(actor_side)
            .deck
            .slots
            .get(next_grid)
            .map(|slot_state| slot_state.card.name)
        else {
            return false;
        };
        if !mirage_ronghui_name_counters(&card.name, &next_name) {
            return false;
        }
        self.activate_mirage_ronghui_elements_from_name(actor_side, &next_name);
        self.gain_anima(actor_side, anima);
        let defense =
            self.mirage_ronghui_value(actor_side, MirageRonghuiValue::CounterElementDefense);
        self.gain_defense(actor_side, defense);
        true
    }

    pub(super) fn apply_ordinary_sword_action_again_before_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        // Build 26315169 (CardActionBase.cs:3618):
        // if (src.HasBuff(BuffType.HuanQinZhuoJianZaiCiXingDong) && cardActionBase.cardConfig.name.Contains("剑")
        //     && (cardActionBase.cardConfig.id != 19 || !src.HasTalent(30096))
        //     && src.GetBuffValue(BuffType.ExActionAgain) <= 0
        //     && src.GetBuffValue(BuffType.ZaiCiXingDong) < src.battleTempData.actionAgainPerRound)
        if self.mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::OrdinarySwordActionAgainCards,
        ) <= 0
            || !card.name.contains('剑')
            || (card.id == 19 && self.actor(actor_side).identity.talents.contains(&30_096))
            || self.actor(actor_side).turn.extra_actions > 0
            || self.actor(actor_side).turn.action_again_count
                >= self.dream_mirage_action_again_limit(actor_side)
        {
            return false;
        }
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::OrdinarySwordActionAgainCards,
            -1,
        );
        self.modify_extra_actions(actor_side, 1);
        true
    }

    pub(super) fn qi_sinks_attack_factor_percent(&self, actor_side: PlayerSide) -> i64 {
        if self.mirage_ronghui_value(actor_side, MirageRonghuiValue::MirageAnimaAttackCards) > 0 {
            self.actor(actor_side).core.anima.max(0) * 10
        } else {
            0
        }
    }

    pub(super) fn apply_beng_quan_star_seize_after_attack(
        &mut self,
        actor_side: PlayerSide,
        hp_lost: i64,
    ) {
        if self.active_effect_after_action() {
            return;
        }
        let marker = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::CrashFistStarSeize);
        let eligible = self.active_effect_base_id() == 144
            || (marker > 0 && self.active_effect_is_beng_quan());
        if !eligible {
            return;
        }
        let healing = hp_lost.max(0) * 3 / 5;
        if healing > 0 {
            self.modify_actor_hp(actor_side, healing, false, false);
        }
        if marker > 0 {
            let consumed = self
                .mirage_ronghui_value(actor_side, MirageRonghuiValue::CrashFistStarSeizeConsumed);
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::CrashFistStarSeizeConsumed,
                1 - consumed,
            );
        }
    }

    /// QiChenDanTianPlus remains active through the card's ordinary/common
    /// follow-up attacks and is consumed only at the late attack-card tail.
    pub(super) fn complete_mirage_ronghui_anima_attack_card(&mut self, actor_side: PlayerSide) {
        if self.active_effect_attacks() > 0
            && self.mirage_ronghui_value(actor_side, MirageRonghuiValue::MirageAnimaAttackCards) > 0
        {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::MirageAnimaAttackCards,
                -1,
            );
        }
    }

    /// CrashFistStarSeizeConsumed settles in OnAfterExecuted's early block,
    /// before any ordinary/common follow-up attack.
    pub(super) fn complete_beng_quan_star_seize_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let consumed =
            self.mirage_ronghui_value(actor_side, MirageRonghuiValue::CrashFistStarSeizeConsumed);
        if consumed <= 0 {
            return;
        }
        // CardActionBase.OnAfterExecuted：XiaoHaoBengQuanDuoXing 在
        // IsLianBeng 时不扣 BengQuanDuoXing。IsLianBeng 是基础 id
        // 10000035，以及 id 恰好为 10030082 / 10040082（二者归一化后是
        // 10000082）。连崩打出时仍然按 60% 回血，标记留给下一张崩拳。
        // oracle 锚点：hf-latest-33912000 pa_ec03597e
        // 崩拳•突 p1.hp 原版 29、引擎 20；pa_bce1434a
        // 崩拳•寸劲 p1.hp 原版 20、引擎 10。两轮出牌前 buff 420 仍在。
        if !matches!(normalized_base_id(card), 10_000_035 | 10_000_082) {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::CrashFistStarSeize,
                -consumed,
            );
        }
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::CrashFistStarSeizeConsumed,
            -consumed,
        );
    }

    pub(super) fn apply_mirage_sword_intent_refund_before_settlement(
        &mut self,
        actor_side: PlayerSide,
    ) -> i64 {
        let pending = self.active_effect_pending_sword_intent().max(0);
        if pending <= 0
            || self.mirage_ronghui_value(actor_side, MirageRonghuiValue::MirageSwordIntentRefund)
                <= 0
            || self.actor(actor_side).sword.sword_intent_circulation > 0
        {
            return 0;
        }
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::MirageSwordIntentRefund,
            -1,
        );
        self.gain_anima(actor_side, pending);
        pending
    }

    pub(super) fn consume_internal_injury_trigger_count(&mut self, actor_side: PlayerSide) -> i64 {
        let extra =
            self.mirage_ronghui_value(actor_side, MirageRonghuiValue::InternalInjuryExtraTriggers);
        if extra > 0 {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::InternalInjuryExtraTriggers,
                -extra,
            );
        }
        1 + extra
    }

    pub(super) fn apply_mirage_ronghui_turn_start(&mut self, actor_side: PlayerSide) {
        let hp = self.actor(actor_side).core.hp;
        let previous = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::ThisTurnStartHp);
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::ThisTurnStartHp,
            hp - previous,
        );
        let plate = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::InfinityPlate);
        if plate > 0 {
            self.gain_hexagram(actor_side, plate);
            self.modify_star_power(actor_side, plate);
        }
    }

    pub(super) fn apply_mirage_ronghui_double_hp_at_turn_start(&mut self, actor_side: PlayerSide) {
        if self.mirage_ronghui_value(actor_side, MirageRonghuiValue::DoubleHpAtTurnStart) > 0 {
            let current_hp = self.actor(actor_side).core.hp;
            self.modify_actor_hp(actor_side, current_hp, false, false);
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::DoubleHpAtTurnStart,
                -1,
            );
        }
    }

    pub(super) fn apply_mirage_ronghui_turn_end(&mut self, actor_side: PlayerSide) {
        let growth =
            self.mirage_ronghui_value(actor_side, MirageRonghuiValue::BilateralTurnEndGrowth);
        if growth > 0 {
            self.modify_actor_max_hp(actor_side, growth);
            self.modify_actor_hp(actor_side, growth, false, false);
        }
        let loss = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::BilateralTurnEndLoss);
        if loss > 0 {
            self.modify_actor_hp(actor_side, -loss, false, false);
        }
        // MirageHealingConversionTurns（幻•金灵印/JinLingYinPlus/577）不在此处衰减：
        // 原版 OnTurnEnded :6804-6806 才 -1，晚于全部回合末治疗；提前衰减会在最后一层
        // 把"加生命转锋锐"变成真治疗。见 flow.rs TurnEndPhase::PoisonImmunity 臂末。
        for value in [
            MirageRonghuiValue::MirageInternalInjuryAmplifierTurns,
            MirageRonghuiValue::MirageSharpnessConversionTurns,
        ] {
            if self.mirage_ronghui_value(actor_side, value) > 0 {
                self.modify_mirage_ronghui_value(actor_side, value, -1);
            }
        }
        let bypass =
            self.mirage_ronghui_value(actor_side, MirageRonghuiValue::ActionAgainIgnoresBinding);
        if bypass > 0 {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::ActionAgainIgnoresBinding,
                -bypass,
            );
        }
        let start = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::ThisTurnStartHp);
        let previous = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::LastTurnStartHp);
        self.modify_mirage_ronghui_value(
            actor_side,
            MirageRonghuiValue::LastTurnStartHp,
            start - previous,
        );
    }

    /// 幻•金灵印（JinLingYinPlus/577）回合末衰减。调用点必须在全部回合末治疗之后
    /// （flow.rs TurnEndPhase::PoisonImmunity 臂）：原版 OnTurnEnded :6804-6806 才 -1，
    /// 晚于回春曲（:6545-6546）、百毒不侵（:5964）等治疗；ModifyHp 的加生命转锋锐
    /// （:9966-9970）在此之前仍能看到最后一层。
    /// oracle 锚点：pa_3844ef5a cp[17]（t12 回春曲 +6 上限：原版 73 血
    /// 不动、锋锐 +6；旧顺序回 6 血、锋锐不动，终局 hpDelta -114 非 -120）。
    pub(super) fn decay_mirage_healing_conversion_turns(&mut self, actor_side: PlayerSide) {
        if self
            .mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::MirageHealingConversionTurns,
            )
            > 0
        {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::MirageHealingConversionTurns,
                -1,
            );
        }
    }

    pub(super) fn apply_mirage_water_defense_after_turn_end_water_damage(
        &mut self,
        actor_side: PlayerSide,
    ) {
        let cap = self.mirage_ronghui_value(actor_side, MirageRonghuiValue::MirageWaterDefenseCap);
        let water = self.actor(actor_side).elements.water_momentum.max(0);
        if cap <= 0 || water <= 0 {
            return;
        }
        // Card_317's OnTurnEnded branch follows the ShuiShi damage loop in
        // BattleCharacter.cs (water damage at IL_1bf8; this ModifyDef branch
        // at IL_1e18). Keep ModifyDef's callbacks after every water hit: they
        // can reflect damage and consume the opponent's remaining defense.
        let uncapped = self
            .mirage_ronghui_value(actor_side, MirageRonghuiValue::MirageWaterDefenseUncapped)
            > 0;
        self.gain_defense(actor_side, if uncapped { water } else { water.min(cap) });
    }

    pub(super) fn reset_mirage_ronghui_first_hp_loss_rewards(&mut self) {
        // 原版 BattleCharacter.cs:5753-5759 在 OnTurnEnded 的末尾清除自身及对手的
        // MingYeMiZongBuShengXiao；必须晚于该回合所有 late HP 变更。
        for side in [PlayerSide::P1, PlayerSide::P2] {
            let triggered =
                self.mirage_ronghui_value(side, MirageRonghuiValue::FirstHpLossRewardTriggered);
            if triggered > 0 {
                self.modify_mirage_ronghui_value(
                    side,
                    MirageRonghuiValue::FirstHpLossRewardTriggered,
                    -triggered,
                );
            }
        }
    }

    pub(super) fn has_binding_bypass(&self, actor_side: PlayerSide) -> bool {
        self.mirage_ronghui_value(actor_side, MirageRonghuiValue::ActionAgainIgnoresBinding) > 0
            || self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&28001)
    }

    pub(super) fn complete_mirage_ronghui_card_classification(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if ["金灵", "水灵", "木灵", "火灵", "土灵"]
            .iter()
            .any(|token| card.name.contains(token))
        {
            self.modify_mirage_ronghui_value(
                actor_side,
                MirageRonghuiValue::FiveElementsCardsUsed,
                1,
            );
        }
    }

    /// Card 315 is the only wave018/019 body with a battle-opening effect.
    /// The battle-start dispatcher must call this for active and skipped
    /// opening slots exactly as it does for the other opening registries.
    pub(super) fn mirage_ronghui_card_has_opening_effect(base_id: i64) -> bool {
        base_id == 315
    }

    pub(super) fn apply_mirage_ronghui_battle_start_opening(
        &mut self,
        actor_side: PlayerSide,
        base_id: i64,
    ) {
        if base_id != 315 {
            return;
        }
        self.activate_element(actor_side, Element::Fire);
        self.activate_element(actor_side, Element::Metal);
    }

    /// Dynamic action-again predicates are evaluated after the card-local
    /// effect and before classification/completion hooks mutate their inputs.
    /// Returning `None` keeps the central resolver's existing fallthrough.
    pub(super) fn resolve_synthetic_oracle_mirage_ronghui_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        base_id: i64,
    ) -> Option<bool> {
        let actor = self.actor(actor_side);
        match base_id {
            122 => Some(has_cloud_chain(actor)),
            127 => Some(card_rarity(card) == 0 && actor.fate.rear_move_succeeded),
            152 => Some(actor.add_hp_count() > 0),
            165 => Some(wu_xing_count_in_deck(actor) <= 2),
            _ => None,
        }
    }

    pub(super) fn execute_mirage_ronghui_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        outer_slot: usize,
        selected_id: i64,
        virtual_slot: usize,
    ) {
        let Some(mut selected) = original_card_definition(selected_id) else {
            self.missing_decision("mirage-ronghui temporary card definition");
            return;
        };
        // Card_173.cs calls InitData(id) then RefreshSpecialCard(publicData)
        // before ExecuteEffect. CardItem.RefreshSpecialCard rebuilds card 19
        // from the copier's current talents and talentTempDatas (Card_19.cs),
        // so its attack/defense must not use the catalog's unrefreshed values.
        if normalized_base_id(&selected) == 19 {
            let actor = self.actor(actor_side);
            selected = super::support::adapt_cheng_xin_card(
                selected,
                &actor.identity.talents,
                actor.identity.sword_forging_attack,
            );
        }
        let action_again = self.apply_temporary_card_effect_with_spec(
            actor_side,
            &selected,
            TemporaryInvocationSpec {
                physical_slot: outer_slot,
                invocation_slot: virtual_slot,
                had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
                deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
                inherit_parent_beng_quan: true,
            },
        );
        if action_again {
            self.modify_extra_actions(actor_side, 1);
        }
    }

    pub(super) fn apply_synthetic_oracle_mirage_ronghui_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        match base_id {
            278 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, param0(card), false, false);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MirageAnimaAttackCards,
                    param1(card),
                );
                Some(false)
            }
            289 => {
                self.apply_configured_anima(actor_side, card);
                self.add_actor_negative_status(target_side, 100, param0(card));
                self.modify_mirage_ronghui_value(
                    target_side,
                    MirageRonghuiValue::MirageInternalInjuryAmplifierTurns,
                    param1(card),
                );
                Some(false)
            }
            292 => {
                self.apply_configured_anima(actor_side, card);
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let mut grid = slot;
                for _ in 0..param0(card) {
                    let Some(next) = self.mirage_ronghui_next_grid(actor_side, grid) else {
                        self.missing_decision("card:292:next active grid");
                        break;
                    };
                    grid = next;
                    let Some(name) = self
                        .actor(actor_side)
                        .deck
                        .slots
                        .get(grid)
                        .map(|slot_state| slot_state.card.name)
                    else {
                        self.missing_decision("card:292:next card");
                        break;
                    };
                    self.activate_mirage_ronghui_elements_from_name(actor_side, &name);
                }
                Some(attacked)
            }
            293 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MirageSwordIntentRefund,
                    param0(card),
                );
                Some(false)
            }
            315 => {
                self.apply_configured_anima(actor_side, card);
                let loss = param1(card);
                self.modify_actor_hp(target_side, -loss, false, false);
                self.modify_actor_max_hp(target_side, -loss);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MirageSharpnessConversionTurns,
                    param0(card),
                );
                Some(false)
            }
            316 => {
                self.gain_sharpness(actor_side, param1(card));
                self.activate_element(actor_side, Element::Metal);
                self.activate_element(actor_side, Element::Wood);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MirageHealingConversionTurns,
                    param0(card),
                );
                Some(false)
            }
            317 => {
                self.gain_water_momentum(actor_side, param0(card));
                self.activate_element(actor_side, Element::Earth);
                self.activate_element(actor_side, Element::Water);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MirageWaterDefenseCap,
                    param1(card),
                );
                // Card_317.cs (build 25621897)：3 级（rarity 2）写入 HuanTuLingZhenYiChuShangXian，取消回合末加防上限。
                if card_rarity(card) == 2 {
                    self.modify_mirage_ronghui_value(
                        actor_side,
                        MirageRonghuiValue::MirageWaterDefenseUncapped,
                        1,
                    );
                }
                Some(false)
            }
            318 => {
                self.apply_configured_physique(actor_side, card);
                let injury = param0(card);
                self.add_actor_negative_status(actor_side, 100, injury);
                self.add_actor_negative_status(target_side, 100, injury);
                let divisor = other_param(card, 1);
                let triggers = if divisor > 0 {
                    self.actor(actor_side).core.physique.max(0) / divisor
                } else {
                    0
                };
                if triggers > 0 {
                    self.modify_mirage_ronghui_value(
                        actor_side,
                        MirageRonghuiValue::InternalInjuryExtraTriggers,
                        triggers,
                    );
                    self.modify_mirage_ronghui_value(
                        target_side,
                        MirageRonghuiValue::InternalInjuryExtraTriggers,
                        triggers,
                    );
                }
                Some(false)
            }
            323 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let count =
                    if self.mirage_ronghui_last_round_exp(actor_side) >= other_param(card, 1) {
                        other_param(card, 2)
                    } else {
                        other_param(card, 0)
                    };
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::OrdinarySwordActionAgainCards,
                    count.max(0),
                );
                Some(attacked)
            }
            122 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.modify_paint_finishing_touch(actor_side, param0(card));
                }
                // Dynamic action-again is frozen by the shared action-again
                // resolver immediately after this effect.
                Some(attacked)
            }
            123 => {
                let cloud_chain = self.actor(actor_side).sword.cloud_chain.max(0);
                let attacked = self.mirage_ronghui_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1).max(0) + cloud_chain,
                    slot,
                );
                if cloud_chain > 0 {
                    self.gain_anima(actor_side, cloud_chain);
                }
                Some(attacked)
            }
            124 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_guard(actor_side, param1(card));
                self.actor_mut(actor_side)
                    .sword
                    .hundred_bird_spirit_sword_art += param0(card);
                Some(false)
            }
            125 => {
                self.actor_mut(actor_side).turn.current_turn_ignore_defense += 1;
                let frenzy = self.actor(actor_side).sword.frenzy_sword.max(0);
                let attacked = self.mirage_ronghui_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1).max(0) + frenzy,
                    slot,
                );
                // Generic classification completion contributes the second
                // Frenzy Sword layer after this body.
                self.actor_mut(actor_side).sword.frenzy_sword += 1;
                Some(attacked)
            }
            127 => {
                let divisor = other_param(card, 0);
                let bonus = if divisor > 0 {
                    self.actor(actor_side).add_hp_count() / divisor
                } else {
                    0
                };
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                self.apply_configured_anima(actor_side, card);
                let rear_move = self.check_rear_move(actor_side, was_used_before_effect);
                if card_rarity(card) > 0 && rear_move {
                    self.modify_actor_hp(actor_side, param1(card), false, false);
                }
                // Rarity-zero action-again is resolved from the rear-move flag
                // immediately after this effect.
                Some(attacked)
            }
            128 => {
                let mut grid = slot;
                for _ in 0..param0(card) {
                    let Some(next) = self.mirage_ronghui_next_grid(actor_side, grid) else {
                        self.missing_decision("card:128:next active grid");
                        break;
                    };
                    grid = next;
                    self.add_mirage_ronghui_star_slot(actor_side, grid);
                }
                self.modify_mirage_ronghui_value(actor_side, MirageRonghuiValue::InfinityPlate, 1);
                Some(false)
            }
            129 => {
                self.add_actor_negative_status(target_side, 100, param0(card));
                for _ in 0..param1(card) {
                    let roll = self.consume_mirage_ronghui_optional_original_random(actor_side);
                    let injury = self.actor(target_side).status.internal_injury.max(0);
                    if roll < other_param(card, 2) && injury > 0 {
                        self.modify_actor_hp(target_side, -injury, false, false);
                    }
                }
                Some(false)
            }
            130 => {
                let star_power = self.actor(actor_side).astrology.star_power.max(0);
                let attacked =
                    self.attack_by_config(actor_side, card, star_power * param0(card), slot);
                self.modify_star_power(actor_side, -((star_power + 1) / 2));
                Some(attacked)
            }
            131 => {
                self.modify_star_power(actor_side, param1(card));
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::SixYaoFanDamage,
                    param0(card),
                );
                Some(false)
            }
            133 => {
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::CounterElementAnima,
                    param0(card),
                );
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::CounterElementDefense,
                    param1(card),
                );
                Some(false)
            }
            135 => {
                self.activate_element(actor_side, Element::Fire);
                self.activate_element(actor_side, Element::Earth);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::MoltenRing,
                    param0(card),
                );
                Some(false)
            }
            137 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, param0(card), false, false);
                self.modify_momentum(actor_side, param1(card));
                self.gain_agility(actor_side, param2(card));
                Some(attacked)
            }
            138 => {
                let status_types = self.negative_status_types_present(actor_side).len() as i64;
                self.modify_momentum(actor_side, status_types * param0(card));
                let bonus = self.actor(actor_side).beng.momentum.max(0) * param1(card);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            139 => {
                self.gain_agility(actor_side, param1(card));
                self.add_actor_negative_status(actor_side, 367, param0(card));
                // 原版 Card_139.cs:123 写入一层 BuffType.MingYeMiZongBu，
                // BattleCharacter.cs:9815-9819 在首次掉血时按该 Buff 的当前值结算。
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::FirstHpLossReward,
                    1,
                );
                Some(false)
            }
            141 => {
                // Card_141.cs:115 先于 :99-100：先加 cardConfig.def，再加连云
                // anima×op0。顺序决定天机•逆施两次反射的先后（4→2 伤先、
                // 3→1 伤后），后者被雪羽清风 64 的 +1 防全吸收则不再触发 64。
                // oracle 锚点：pa_b90a1351 卡#4（p1 防 0 非 1）。
                self.apply_configured_defense(actor_side, card);
                if has_cloud_chain(self.actor(actor_side)) && self.actor(actor_side).core.anima > 0
                {
                    self.gain_defense(actor_side, self.actor(actor_side).core.anima * param0(card));
                }
                Some(false)
            }
            142 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let mut grid = slot;
                for _ in 0..param0(card) {
                    let Some(next) = self.mirage_ronghui_next_grid(actor_side, grid) else {
                        self.missing_decision("card:142:next active grid");
                        break;
                    };
                    grid = next;
                    self.add_mirage_ronghui_star_slot(actor_side, grid);
                }
                self.apply_configured_anima(actor_side, card);
                Some(attacked)
            }
            144 => {
                // Per-segment 60% healing is part of the shared post-attack
                // hook for this card and any later eligible Beng Quan.  It
                // must run inside attack settlement, not be reconstructed
                // from the final card ledger here.
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::CrashFistStarSeize,
                    1,
                );
                Some(attacked)
            }
            151 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let frenzy = self.actor(actor_side).sword.frenzy_sword.max(0);
                if frenzy > 0 {
                    self.gain_anima(actor_side, frenzy);
                    self.gain_defense(actor_side, frenzy * param0(card));
                    self.modify_sword_intent(actor_side, frenzy);
                }
                Some(attacked)
            }
            152 => {
                let anima = self.actor(actor_side).core.anima.max(0);
                let star_power = self.actor(actor_side).astrology.star_power.max(0);
                self.spend_anima_unchecked(actor_side, anima);
                self.modify_star_power(actor_side, -star_power);
                let hp_gain = anima * param0(card) + star_power * param1(card);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.gain_anima(actor_side, param2(card));
                }
                // Dynamic action-again reads the post-effect HpGained ledger.
                Some(false)
            }
            153 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let mut grid = slot;
                let mut primary_kinds = BTreeSet::new();
                for _ in 0..param0(card) {
                    let Some(next) = self.mirage_ronghui_next_grid(actor_side, grid) else {
                        self.missing_decision("card:153:next active grid");
                        break;
                    };
                    grid = next;
                    let Some(name) = self
                        .actor(actor_side)
                        .deck
                        .slots
                        .get(grid)
                        .map(|slot_state| slot_state.card.name)
                    else {
                        self.missing_decision("card:153:next card");
                        break;
                    };
                    if let Some(primary) = self
                        .activate_mirage_ronghui_elements_from_name(actor_side, &name)
                        .last()
                        .copied()
                    {
                        primary_kinds.insert(primary);
                    }
                }
                if primary_kinds.len() == 1 {
                    self.gain_anima(actor_side, 1);
                }
                Some(attacked)
            }
            154 => {
                self.apply_configured_physique(actor_side, card);
                self.modify_momentum(actor_side, param0(card));
                let divisor = other_param(card, 1);
                let bonus = if divisor > 0 {
                    (self.actor(actor_side).core.physique.max(0) / divisor).min(param2(card))
                } else {
                    0
                };
                self.actor_mut(actor_side).turn.next_attack_bonus += bonus;
                Some(false)
            }
            156 => {
                let printed = param0(card);
                self.modify_actor_max_hp(actor_side, printed);
                self.modify_actor_hp(actor_side, printed, false, false);
                let divisor = other_param(card, 1);
                let ledger = if divisor > 0 {
                    self.actor(actor_side).turn.lose_hp_count.max(0) / divisor
                } else {
                    0
                };
                if ledger > 0 {
                    self.modify_actor_max_hp(actor_side, ledger);
                    self.modify_actor_hp(actor_side, ledger, false, false);
                }
                Some(false)
            }
            157 => {
                let hp = self.actor(actor_side).core.hp.max(0);
                let self_loss = (hp * param0(card) + 99) / 100;
                self.modify_actor_hp(actor_side, -self_loss, false, false);
                let target_loss = self_loss + param1(card);
                self.modify_actor_hp(target_side, -target_loss, false, false);
                self.modify_actor_max_hp(target_side, -target_loss);
                Some(false)
            }
            158 => {
                let growth = param0(card);
                let loss = param1(card);
                for side in [actor_side, target_side] {
                    self.modify_mirage_ronghui_value(
                        side,
                        MirageRonghuiValue::BilateralTurnEndGrowth,
                        growth,
                    );
                    self.modify_mirage_ronghui_value(
                        side,
                        MirageRonghuiValue::BilateralTurnEndLoss,
                        loss,
                    );
                }
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            159 => {
                for _ in 0..param0(card) {
                    match self.consume_mirage_ronghui_optional_decision() {
                        0 => {
                            self.modify_actor_hp(target_side, -6, false, false);
                            self.modify_actor_hp(actor_side, 6, false, false);
                        }
                        1 => {
                            self.add_actor_negative_status(target_side, 101, 2);
                        }
                        2 => {
                            self.gain_guard(actor_side, 1);
                        }
                        -1 => {}
                        _ => self.missing_decision("card:159:branch must be 0, 1, or 2"),
                    }
                }
                Some(false)
            }
            160 => {
                self.gain_attack_bonus(actor_side, param0(card));
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::HpLossAttackBonusCharges,
                    param1(card),
                );
                Some(false)
            }
            161 => {
                self.modify_actor_hp(actor_side, param0(card), false, false);
                self.apply_configured_defense(actor_side, card);
                let divisor = other_param(card, 1);
                if divisor > 0 {
                    self.actor_mut(actor_side).turn.next_turn_defense +=
                        self.actor(actor_side).turn.lose_hp_count.max(0) / divisor;
                }
                Some(false)
            }
            162 => {
                let heal = self
                    .mirage_ronghui_value(actor_side, MirageRonghuiValue::LastTurnStartHp)
                    - self.actor(actor_side).core.hp
                    + other_param(card, 0);
                if heal > 0 {
                    self.modify_actor_hp(actor_side, heal, false, false);
                }
                self.actor_mut(actor_side).turn.jump_to_previous_card += 1;
                Some(false)
            }
            163 => {
                // Generic printed-field completion adds the printed Sword
                // Intent; this body contributes only the lost-HP bonus.
                let divisor = other_param(card, 0);
                if divisor > 0 {
                    self.modify_sword_intent(
                        actor_side,
                        self.actor(actor_side).turn.lose_hp_count.max(0) / divisor,
                    );
                }
                Some(false)
            }
            164 => {
                for _ in 0..param0(card) {
                    let roll = self.consume_mirage_ronghui_optional_original_random(actor_side);
                    if roll >= 10 {
                        self.apply_mirage_ronghui_damage(
                            actor_side,
                            actor_side,
                            param1(card),
                            true,
                        );
                    }
                    self.apply_mirage_ronghui_damage(actor_side, target_side, param2(card), true);
                }
                Some(false)
            }
            165 => {
                let divisor = other_param(card, 0);
                let bonus = if divisor > 0 {
                    self.actor(actor_side).turn.lose_hp_count.max(0) / divisor
                } else {
                    0
                };
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // Dynamic action-again is resolved from the complete deck
                // element count immediately after this effect.
                Some(attacked)
            }
            166 => {
                let divisor = other_param(card, 0);
                let extra = if divisor > 0 {
                    self.actor(actor_side).turn.lose_hp_count.max(0) / divisor
                } else {
                    0
                };
                Some(self.mirage_ronghui_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1).max(0) + extra,
                    slot,
                ))
            }
            167 => {
                let extra = self.actor(actor_side).sword.sword_intent.max(0)
                    * (other_param_or(card, 0, 1) - 1)
                    + self.actor(actor_side).core.attack_bonus.max(0)
                        * (other_param_or(card, 1, 1) - 1);
                Some(self.mirage_ronghui_attack_with_shatter(
                    actor_side,
                    card.attack.unwrap_or(0) + extra,
                    card.attack_count.unwrap_or(1).max(0),
                    slot,
                ))
            }
            168 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let reverse = self.actor(actor_side).fate.reverse_card_direction;
                self.actor_mut(actor_side).fate.reverse_card_direction =
                    if reverse > 0 { 0 } else { 1 };
                self.reverse_queue(actor_side);
                if self.consume_mirage_ronghui_optional_original_random(actor_side) < 10 {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(attacked)
            }
            170 => {
                let momentum = self.actor(actor_side).beng.momentum.max(0);
                let anima = self.actor(actor_side).core.anima.max(0);
                let agility = self.actor(actor_side).turn.agility.max(0);
                let attack = card.attack.unwrap_or(0)
                    + momentum * param0(card)
                    + anima * param1(card)
                    + agility * param2(card);
                self.modify_momentum(actor_side, -momentum);
                self.spend_anima_unchecked(actor_side, anima);
                self.modify_agility(actor_side, -agility);
                Some(self.mirage_ronghui_attack_with_value(
                    actor_side,
                    attack,
                    card.attack_count.unwrap_or(1).max(0),
                    slot,
                ))
            }
            171 => Some(self.attack_ignoring_defense_and_guard(
                actor_side,
                card.attack.unwrap_or(0),
                card.attack_count.unwrap_or(1).max(0),
                slot,
            )),
            172 => {
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::NineHeavensRevive,
                    1,
                );
                Some(false)
            }
            173 => {
                if self.mirage_ronghui_value(actor_side, MirageRonghuiValue::TemporaryCopyDepth) > 0
                {
                    return Some(false);
                }
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::TemporaryCopyDepth,
                    1,
                );
                for element in [
                    Element::Wood,
                    Element::Fire,
                    Element::Earth,
                    Element::Metal,
                    Element::Water,
                ] {
                    self.activate_element(actor_side, element);
                }
                let selected_id = self
                    .mirage_ronghui_next_grid(actor_side, slot)
                    .and_then(|grid| self.actor(actor_side).deck.slots.get(grid))
                    .map(|slot_state| slot_state.card.id);
                if let Some(selected_id) = selected_id {
                    self.execute_mirage_ronghui_temporary_card(actor_side, slot, selected_id, slot);
                } else {
                    self.missing_decision("card:173:next active card");
                }
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::ActionAgainIgnoresBinding,
                    1,
                );
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::TemporaryCopyDepth,
                    -1,
                );
                Some(false)
            }
            174 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_mirage_ronghui_value(
                    target_side,
                    MirageRonghuiValue::CannotGainHp,
                    param1(card),
                );
                Some(attacked)
            }
            175 => {
                let hp = param0(card);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                self.modify_mirage_ronghui_value(
                    actor_side,
                    MirageRonghuiValue::DoubleHpAtTurnStart,
                    param1(card),
                );
                Some(false)
            }
            176 => {
                // Card_176.cs OnExecuted: ModifyAnima/ModifyHp/ModifyMaxHp take
                // -Mathf.CeilToInt(value / 2). At negative hp the ceiling is
                // negative, i.e. a heal (same as Card_3000014). Clamping to
                // zero would swallow that branch.
                // oracle: pa_bbf13c88 [20] (p2 -7 -> -4).
                let anima_loss =
                    div_ceil_signed(self.actor(target_side).core.anima, 2);
                self.spend_anima_unchecked(target_side, anima_loss);
                let hp_loss = div_ceil_signed(self.actor(target_side).core.hp, 2);
                self.modify_actor_hp(target_side, -hp_loss, false, false);
                let max_hp_loss =
                    div_ceil_signed(self.actor(target_side).core.max_hp, 2);
                self.modify_actor_max_hp(target_side, -max_hp_loss);
                self.add_actor_negative_status(target_side, 104, param0(card));
                Some(false)
            }
            39 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.add_actor_negative_status(target_side, 100, param0(card));
                Some(attacked)
            }
            7_000_093 => {
                self.apply_configured_anima(actor_side, card);
                let element_count = wu_xing_count_in_deck(self.actor(actor_side)).max(0);
                let realm = original_card_realm_level(card.id).unwrap_or(0);
                if realm <= 4 {
                    let attacked = self.attack_by_config(actor_side, card, 0, slot);
                    let followups = element_count.min(param1(card));
                    let followup_attacked = self.mirage_ronghui_attack_with_value(
                        actor_side,
                        param0(card),
                        followups,
                        slot,
                    );
                    return Some(attacked || followup_attacked);
                }
                let five_elements_cards_used = self
                    .mirage_ronghui_value(actor_side, MirageRonghuiValue::FiveElementsCardsUsed);
                Some(self.mirage_ronghui_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0) + five_elements_cards_used * param0(card),
                    card.attack_count.unwrap_or(1).max(0) + element_count,
                    slot,
                ))
            }
            _ => None,
        }
    }

    fn mirage_ronghui_attack_with_value(
        &mut self,
        actor_side: PlayerSide,
        attack: i64,
        attack_count: i64,
        slot: usize,
    ) -> bool {
        if attack <= 0 || attack_count <= 0 {
            return false;
        }
        for _ in 0..attack_count {
            self.apply_attack(actor_side, attack, slot);
        }
        true
    }

    fn mirage_ronghui_attack_with_shatter(
        &mut self,
        actor_side: PlayerSide,
        attack: i64,
        attack_count: i64,
        slot: usize,
    ) -> bool {
        if attack <= 0 || attack_count <= 0 {
            return false;
        }
        for _ in 0..attack_count {
            self.apply_attack_with_options(actor_side, attack, slot, false, true, 0, None);
        }
        true
    }

    fn attack_ignoring_defense_and_guard(
        &mut self,
        actor_side: PlayerSide,
        attack: i64,
        attack_count: i64,
        slot: usize,
    ) -> bool {
        if attack <= 0 || attack_count <= 0 {
            return false;
        }
        for _ in 0..attack_count {
            // ignore_defense_attacks is segment-counted, so adding one before
            // each segment preserves any pre-existing charges exactly.
            self.actor_mut(actor_side).turn.ignore_defense_attacks += 1;
            // 无视护体同理用段计次标透传：目标护体全程保留（原版 WuShiHuTi
            // 只旁路 :9961 格挡，:11075 击伤/锋锐门仍看到护体）。
            self.actor_mut(actor_side).turn.ignore_guard_attacks += 1;
            self.apply_attack(actor_side, attack, slot);
        }
        true
    }

    fn mirage_ronghui_next_grid(
        &self,
        actor_side: PlayerSide,
        current_grid: usize,
    ) -> Option<usize> {
        let actor = self.actor(actor_side);
        let active_count = actor
            .deck
            .active_slot_count
            .min(actor.deck.slots.len())
            .max(1);
        if actor.deck.slots.is_empty() {
            return None;
        }
        let step = if actor.fate.reverse_card_direction > 0 {
            -1
        } else {
            1
        };
        Some((current_grid as i64 + step).rem_euclid(active_count as i64) as usize)
    }

    fn add_mirage_ronghui_star_slot(&mut self, actor_side: PlayerSide, grid: usize) {
        if self.actor(actor_side).astrology.star_slots.contains(&grid) {
            self.gain_anima(actor_side, 1);
        } else {
            self.actor_mut(actor_side).astrology.star_slots.push(grid);
        }
    }

    fn activate_mirage_ronghui_elements_from_name(
        &mut self,
        actor_side: PlayerSide,
        name: &str,
    ) -> Vec<Element> {
        const TOKENS: [(&str, Element); 5] = [
            ("金灵", Element::Metal),
            ("木灵", Element::Wood),
            ("水灵", Element::Water),
            ("火灵", Element::Fire),
            ("土灵", Element::Earth),
        ];
        let mut activated = Vec::new();
        for (token, element) in TOKENS {
            if name.contains(token) {
                self.activate_element(actor_side, element);
                activated.push(element);
            }
        }
        activated
    }

    fn consume_mirage_ronghui_optional_original_random(&mut self, actor_side: PlayerSide) -> i64 {
        // Every original branch here calls BattleCharacter.GetNextRandomValue,
        // whose Card 422 pre-loss and subsequent Hexagram-loss ordering is
        // centralized by this helper.
        self.consume_original_random_hexagram_side_effects(actor_side);
        self.consume_mirage_ronghui_optional_decision()
    }

    fn consume_mirage_ronghui_optional_decision(&mut self) -> i64 {
        if self.decision_tape.is_empty() {
            -1
        } else {
            self.decision_tape.remove(0)
        }
    }
}

fn mirage_ronghui_name_counters(current: &str, next: &str) -> bool {
    (current.contains("金灵") && next.contains("木灵"))
        || (current.contains("木灵") && next.contains("土灵"))
        || (current.contains("土灵") && next.contains("水灵"))
        || (current.contains("水灵") && next.contains("火灵"))
        || (current.contains("火灵") && next.contains("金灵"))
}
