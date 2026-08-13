use super::original_config::{original_card_definition, original_card_realm_level};
use super::support::{
    has_cloud_chain, is_spirit_sword_for_actor, is_sword_formation_card, opponent_side,
    other_param, wu_xing_count_in_deck,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

#[cfg(all(test, feature = "private-fixtures"))]
pub(super) const SYNTHETIC_ORACLE_WAVE015_CARD_IDS: [i64; 25] = [
    334, 335, 340, 344, 345, 346, 348, 349, 350, 351, 369, 378, 1_000_069, 1_000_070, 1_000_074,
    1_000_075, 1_000_077, 1_000_078, 1_000_079, 1_000_080, 1_000_083, 1_000_084, 1_000_085,
    1_000_086, 1_000_087,
];

#[cfg(all(test, feature = "private-fixtures"))]
pub(super) const SYNTHETIC_ORACLE_WAVE016_CARD_IDS: [i64; 25] = [
    4_000_069, 4_000_070, 4_000_073, 4_000_075, 4_000_078, 4_000_079, 4_000_081, 4_000_082,
    4_000_084, 4_000_085, 4_000_087, 4_000_089, 7_000_074, 7_000_075, 7_000_076, 7_000_077,
    7_000_078, 7_000_079, 7_000_080, 7_000_081, 7_000_082, 7_000_083, 7_000_084, 7_000_086,
    7_000_087,
];

#[cfg(all(test, feature = "private-fixtures"))]
pub(super) const SYNTHETIC_ORACLE_WAVE017_CARD_IDS: [i64; 29] = [
    7_000_089, 7_000_090, 7_000_093, 10_000_069, 10_000_071, 10_000_076, 10_000_077, 10_000_078,
    10_000_080, 10_000_081, 10_000_082, 10_000_083, 10_000_084, 10_000_085, 10_000_087, 10_000_088,
    260, 261, 266, 269, 270, 271, 272, 273, 277, 395, 26, 79, 198,
];

/// Exact original BuffType-backed values owned by the shared waves015-017
/// runtime. Card bodies use this closed enum so an omitted persistent effect
/// cannot silently degrade to an approximate local field.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum DreamMirageValue {
    DreamUnmovingFormation,
    DreamDanceCountdown,
    DreamFlyingCloudPill,
    DreamGreatReturnPill,
    DreamTuneImmunity,
    DreamExtraActionLock,
    HalfAnimaGain,
    CannotGainDefense,
    CannotGainHp,
    CalamitySkipMask,
    TotalAnimaGained,
    CloudSwordUsedCount,
    SwordUsedCount,
    FormationUsedCount,
    AnimaGainDefense,
    SwordIntentGainDefense,
    TurnStartDefense,
    CloudSeaOnFormation,
    SwordEnergyOnSword,
    DoubleNextSwordIntentAndAttackBonus,
    HealingTurnEndFrenzy,
    RearMoveCardUsedCount,
    DreamReflection,
    DreamStarBoard,
    DreamStarBoardLowRealm,
    DreamStarBoardTriggered,
    SnakeShadow,
    SnakeCardUsedCount,
    WitheredTreeUsedCount,
    ActionAgainSharpness,
    TemporaryWaterDouble,
    TemporaryWaterLedger,
    TemporaryAnimaLedger,
    UnconditionalFiveElements,
    TotalActualDamage,
    AttackBonusToThorns,
    LostMaxHpEventCount,
    TotalSharpnessGained,
    TotalWaterMomentumGained,
    DreamCliff,
    FiveElementsMarrow,
    ConsumeNextCard,
    DreamFireFormation,
    UsedFiveElementsCount,
    DreamMysticFootwork,
    DreamMysticFootworkHigh,
    DreamMysticFootworkSuppressed,
    DreamMysticFootworkTriggerCount,
    DefenseLedger,
    TotalMomentumGained,
    FlatMomentumAttack,
    MomentumBeforeEveryAttack,
    NextHpCostRefund,
    NextHpGainDefense,
    HpGainDefense,
    NextBengQuanAdditionalAttack,
    TriggeredBengQuanAdditionalAttack,
    NextBengQuanPhysique,
    DreamForgeFist,
    DreamForgeFistConsumed,
    DefenseGainDamageLow,
    DreamDefenseGainDamage,
    DreamDefenseGainDamageGuard,
    FlowingMerciless,
    StarShift,
    StarShiftAttack,
    RepeatNextFireOrEarth,
    ExtraWaterMomentumTurnEnd,
    ReturnSharpness,
    ExcessPhysiqueHp,
    ExcessPhysiqueDamage,
    LastTurnStartHp,
    TurnHpGained,
    SpiritCatCloud,
    DragonExtraActionImmunity,
}

/*
Waves 015-017 card-body integration contract
=============================================

The 79 ids above and every immediate Card_*.cs body below were checked against
Steam build 24124964 and the TS aggregate in
synthetic-oracle-candidate-waves015-017.ts. This file must not be declared in
replay.rs until the integration layer supplies these mandatory interfaces:

  dream_mirage_value(side, DreamMirageValue) -> i64
  modify_dream_mirage_value(side, DreamMirageValue, delta)
  modify_actor_max_hp(side, delta)
  gain_dream_mirage_attack_bonus(side, amount)
  modify_sword_intent(side, delta)
  modify_star_power(side, delta)
  increase_dream_mirage_action_again_limit(side, amount, cap)
  execute_dream_mirage_temporary_card(actor, outer_slot, selected_id, virtual_slot)

The shared runtime must also wire, without optional fallbacks:

  resource mutation
    Unmoving loss clamp; half Anima; cannot-gain-defense; Anima/Sword-Intent
    defense conversion; attack-bonus-to-thorns; gain ledgers; max-HP-loss
    event count; excess-Physique HP/damage; reflection and Dream Cliff.
  attack/card lifecycle
    cloud/sword/formation/use counters; Point Star and adjacent hooks; Star
    Board; Star Shift; All-Purpose Sword classification; Beng Quan adjacency,
    appended attacks, Forge Fist conversion, flat Momentum attacks, persistent
    pre-attack Momentum, and the next Fire/Earth repeat.
  turn/action lifecycle
    Flying/Great Return pills; temporary Water/Anima restore; duration ticks;
    Wild Dance; Binding/Tune/Dragon action-again rules; Fire Formation;
    Spatial Spirit Field and Calamity skips; next-card exhaust; turn-end Water
    damage and Frenzy healing ledger.
  temporary transaction
    Cards 349, 395, 7_000_075 and 7_000_078 require the complete guarded
    temporary-card transaction, including printed fields, completion hooks,
    action-again projection, and restoration of the outer card.

7_000_093 and 198 remain in the frozen wave017 inventory, but deliberately
return None below. Dispatch must fall through to the more complete wave018 and
ronghui implementations respectively; whichever module is ordered first must
not shadow those authoritative bodies.
*/

impl ReplayState {
    pub(super) fn execute_dream_mirage_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        outer_slot: usize,
        selected_id: i64,
        virtual_slot: usize,
    ) {
        self.execute_mirage_ronghui_temporary_card(
            actor_side,
            outer_slot,
            selected_id,
            virtual_slot,
        );
    }

    pub(super) fn dream_mirage_card_has_opening_effect(base_id: i64) -> bool {
        matches!(base_id, 348 | 369 | 378 | 1_000_086 | 4_000_079 | 7_000_079)
    }

    pub(super) fn apply_dream_mirage_battle_start_opening(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) {
        self.apply_dream_mirage_battle_start_opening_with_trigger_grid(
            actor_side, card, slot, base_id, slot,
        );
    }

    pub(super) fn apply_dream_mirage_battle_start_opening_with_trigger_grid(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
        trigger_grid: usize,
    ) {
        let target_side = opponent_side(actor_side);
        match base_id {
            348 => {
                let duration = other_param(card, 0).max(0);
                for side in [actor_side, target_side] {
                    self.modify_dream_mirage_value(
                        side,
                        DreamMirageValue::DreamExtraActionLock,
                        duration,
                    );
                }
            }
            369 => {
                let target_card_id = self
                    .actor(target_side)
                    .deck
                    .slots
                    .get(trigger_grid)
                    .map(|slot_state| slot_state.card.id);
                let Some(target_card_id) = target_card_id else {
                    self.missing_decision("card:369:opponent same-grid card");
                    return;
                };
                // 原版 TriggerOpening case 369（BattleCharacter.cs:11204-11219）
                // 读 cardItem.cardConfig.rarity（配置值，无 rarity 字段 = 0），
                // 不是 id 档位；梦牌/隐藏牌配置 rarity=0 时不降级，改为
                // AddMengEJieSkipPos（跳过该格）。
                let rarity = super::original_config::original_config_rarity(target_card_id);
                if rarity >= 1 && target_card_id != 19 {
                    if let Some(lower) = original_card_definition(target_card_id - 10_000) {
                        if let Some(target_slot) =
                            self.actor_mut(target_side).deck.slots.get_mut(trigger_grid)
                        {
                            target_slot.card = lower;
                        }
                    } else {
                        self.missing_decision("card:369:lower-rarity card config");
                    }
                } else {
                    self.modify_dream_mirage_value(
                        target_side,
                        DreamMirageValue::CalamitySkipMask,
                        1_i64 << trigger_grid,
                    );
                }
            }
            378 => {
                let duration = other_param(card, 0).max(0);
                for side in [actor_side, target_side] {
                    self.modify_dream_mirage_value(side, DreamMirageValue::HalfAnimaGain, duration);
                }
            }
            1_000_086 => {
                if dream_mirage_realm(card) >= 4 {
                    self.actor_mut(actor_side).sword.cloud_sea += other_param(card, 1).max(0);
                }
            }
            4_000_079 => {
                let previous = self.dream_mirage_previous_grid(actor_side, slot);
                let next = self.dream_mirage_next_grid(actor_side, slot);
                self.ensure_dream_mirage_star_slot(actor_side, previous);
                self.ensure_dream_mirage_star_slot(actor_side, next);
            }
            7_000_079 => self.gain_anima(actor_side, other_param(card, 1).max(0)),
            _ => {}
        }
    }

    pub(super) fn resolve_synthetic_oracle_dream_mirage_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        base_id: i64,
    ) -> Option<bool> {
        let actor = self.actor(actor_side);
        let target = self.actor(opponent_side(actor_side));
        let realm = dream_mirage_realm(card);
        match base_id {
            335 => Some(target.turn.used_card_count > 0),
            346 => Some(actor.turn.used_card_count > 0),
            1_000_080 => Some(
                realm >= 4
                    && actor
                        .deck
                        .slots
                        .iter()
                        .filter(|slot_state| is_sword_formation_card(actor, &slot_state.card))
                        .count() as i64
                        >= other_param(card, 2),
            ),
            1_000_085 => Some(
                realm >= 4
                    && (actor.sword.sword_intent >= other_param(card, 0)
                        || actor.core.attack_bonus >= other_param(card, 0)),
            ),
            4_000_081 => Some(realm >= 4 && actor.astrology.hexagram > other_param(card, 0)),
            // CardConfig: 7020077 (金丹) has no actionAgain; 7030077/7040077
            // (元婴/化神) carry the explicit [再次行动] flag.
            7_000_077 => Some(realm >= 4),
            7_000_080 => Some(
                realm > 3 && (actor.elements.water_momentum > 0 || actor.core.attack_bonus > 0),
            ),
            7_000_083 => Some(
                realm >= 4
                    && self.dream_mirage_value(actor_side, DreamMirageValue::DefenseLedger) > 0,
            ),
            260 => Some(
                actor
                    .deck
                    .slots
                    .iter()
                    .filter(|slot_state| is_spirit_sword_for_actor(actor, &slot_state.card))
                    .count() as i64
                    >= other_param(card, 1),
            ),
            _ => None,
        }
    }

    #[allow(clippy::too_many_lines)]
    pub(super) fn apply_synthetic_oracle_dream_mirage_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        let realm = dream_mirage_realm(card);
        match base_id {
            334 => {
                let amount = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                let duration = if self
                    .dream_mirage_value(actor_side, DreamMirageValue::DreamUnmovingFormation)
                    > 0
                {
                    1
                } else {
                    2
                };
                self.actor_mut(actor_side).status.cannot_act += 1;
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamUnmovingFormation,
                    duration,
                );
                Some(false)
            }
            335 => {
                self.modify_actor_hp(actor_side, -other_param(card, 0).max(0), false, false);
                if !was_used_before_effect && self.actor(target_side).deck.queue.len() > 1 {
                    // Card_335.cs:86 dst.RightMoveCardItems(1, CareSkip: true)
                    // （BattleCharacter.cs:8268-8290）：把对方卡组末尾的牌移到
                    // 队首；末尾牌已跳过时该次移动不计入 R，继续移动新末尾牌
                    // （本轮已用/消耗的牌在快照里 skip=true）。
                    // oracle 锚点：mirror-32299000 22cbe42588093881/round-14
                    // t5 p2 抽到 213 曳影剑阵（队列 [213, s0, s1, s2, ...]）。
                    self.actor_mut(target_side).right_move_card_queue(1);
                }
                Some(false)
            }
            340 => {
                let amount = other_param(card, 0).max(0);
                for side in [actor_side, target_side] {
                    self.modify_dream_mirage_value(
                        side,
                        DreamMirageValue::DreamDanceCountdown,
                        amount,
                    );
                }
                Some(false)
            }
            344 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamFlyingCloudPill,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            345 => {
                let amount = other_param(card, 1).max(0);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamGreatReturnPill,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            346 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamTuneImmunity, 1);
                Some(false)
            }
            348 => {
                self.modify_actor_hp(actor_side, -other_param(card, 1).max(0), false, false);
                Some(false)
            }
            349 => {
                let amount = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                self.execute_dream_mirage_selected_temporary_card(actor_side, slot);
                Some(false)
            }
            350 => Some(false),
            351 => {
                for _ in 0..other_param(card, 1).max(0) {
                    self.modify_actor_hp(target_side, -other_param(card, 0).max(0), false, false);
                }
                let duration = other_param(card, 2).max(0);
                self.modify_dream_mirage_value(
                    target_side,
                    DreamMirageValue::CannotGainHp,
                    duration,
                );
                self.modify_dream_mirage_value(
                    target_side,
                    DreamMirageValue::CannotGainDefense,
                    duration,
                );
                Some(false)
            }
            369 => {
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                Some(false)
            }
            378 => {
                self.spend_anima_up_to(actor_side, other_param(card, 1).max(0));
                self.spend_anima_up_to(target_side, other_param(card, 1).max(0));
                Some(false)
            }
            1_000_069 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            1_000_070 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).sword.cloud_sea += other_param(card, 0).max(0);
                // Card_1000070.cs:99-131（梦•云剑极意）：[连云] 剑意增益整段
                // 受 HasBuff(LianYun) 门控（化神档除外）。档位分支：筑基及
                // 以下 +op[1] 平值；金丹~元婴 +op[1]×连云层数；化神
                // +op[1] + op[1]×用过云剑次数（不受连云门控）。引擎此前
                // 无连云也发 op[1]。oracle 锚点：hf-32308000
                // 6b1f17c3eac57536/round-07 cp16（p1 无连云 0→0 不发，
                // 引擎 +1）、7a3090163702dc0d/round-06 cp10（p2 无连云，
                // 攻击消耗 3 后不发 → 0，引擎剩 1）。
                let mut intent = 0;
                if realm >= 5 {
                    intent += other_param(card, 1).max(0)
                        + self
                            .dream_mirage_value(actor_side, DreamMirageValue::CloudSwordUsedCount)
                            .max(0)
                            * other_param(card, 1).max(0);
                } else if has_cloud_chain(self.actor(actor_side)) {
                    intent = if realm > 2 {
                        other_param(card, 1).max(0)
                            * self.actor(actor_side).sword.cloud_chain.max(0)
                    } else {
                        other_param(card, 1).max(0)
                    };
                }
                self.modify_sword_intent(actor_side, intent);
                Some(attacked)
            }
            1_000_074 => {
                self.apply_configured_anima(actor_side, card);
                let divisor = other_param(card, 0);
                let amount = if realm <= 3 {
                    divisor.max(0)
                } else if divisor > 0 {
                    self.dream_mirage_value(actor_side, DreamMirageValue::TotalAnimaGained)
                        / divisor
                } else {
                    0
                };
                self.actor_mut(actor_side).sword.sword_energy += amount.max(0);
                Some(false)
            }
            1_000_075 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).sword.sword_energy += other_param(card, 0).max(0);
                if realm > 3 {
                    let anima = self.actor(actor_side).core.anima.max(0);
                    if anima > 0 {
                        self.spend_anima_unchecked(actor_side, anima);
                        self.actor_mut(actor_side).sword.sword_energy += anima;
                    }
                }
                Some(attacked)
            }
            1_000_077 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::AnimaGainDefense,
                    other_param(card, 0).max(0),
                );
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::SwordIntentGainDefense,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            1_000_078 => {
                self.apply_configured_defense(actor_side, card);
                if realm >= 5 {
                    let defense = self
                        .dream_mirage_value(actor_side, DreamMirageValue::CloudSwordUsedCount)
                        .max(0)
                        * other_param(card, 0).max(0);
                    self.gain_defense(actor_side, defense.max(0));
                }
                self.actor_mut(actor_side).sword.cloud_sea += other_param(card, 2).max(0);
                if realm >= 4 {
                    self.actor_mut(actor_side).sword.water_month_sword_formation +=
                        other_param(card, 1).max(0);
                }
                // Card_1000078.cs IL_012e：境界 ≤ 元婴 走 [连云] 分支——先读
                // 当前 LianYun 计数（本卡完成时 classification hook 才 +1，
                // 与 CardActionBase.cs:4602-4606 的补计数时序一致），再
                // ModifyDef(LianYun × otherParams[0])；境界 = 化神 走
                // YongGuoYunJianJiShu（720 用过云剑计数）分支。原版两分支
                // 互斥，引擎此前只实现了化神分支，元婴档（1030078/1020078）
                // 少了连云加防。oracle 锚点：mirror-32299000
                // 7d2572044fb994f0/round-10 cp11（原版 p1.def=15 = 9+2×3，
                // rust=9）、7f45e2b2095d598c/round-07 cp10（17 = 9+4×2）、
                // 9bd061474bea1143/round-10 cp2（12 = 9+1×3）、
                // a48308baca307537/round-09 cp15（24 = 9+5×3）、
                // ff968b01d5c365e1/round-10 cp6（12 = 9+1×3）。
                if realm <= 4 {
                    let lian_yun = self.actor(actor_side).sword.cloud_chain.max(0);
                    if lian_yun > 0 {
                        self.gain_defense(actor_side, lian_yun * other_param(card, 0).max(0));
                    }
                }
                Some(false)
            }
            1_000_079 => {
                let scaled = if realm >= 3 {
                    self.dream_mirage_value(actor_side, DreamMirageValue::SwordUsedCount)
                        * other_param(card, 1).max(0)
                } else {
                    0
                };
                let attacked = self.dream_mirage_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0) + scaled,
                    card.attack_count.unwrap_or(1),
                    slot,
                );
                self.gain_defense(actor_side, card.defense.unwrap_or(0).max(0) + scaled);
                Some(attacked)
            }
            1_000_080 => {
                // Card_1000080.cs（梦•御空剑阵）：伤害 = otherParams[0] +
                // 剑阵数加成。金丹及以下（realm ≤ 3）：遍历
                // GetBattleDeckIdList 数剑阵牌（BattleCharacter.cs:12444
                // IsJianZhen）；当前牌结算时仍在 m_BattleDeck 中、计入自身，
                // 每张 +otherParams[1]。元婴及以上（realm ≥ 4）：改加
                // JianZhenCount(580)（已结算剑阵数，不含自身）×otherParams[1]
                // —— 即 FormationUsedCount（complete_dream_mirage_card_classification
                // 在牌结算后 +1）；actionAgain 条件由
                // resolve_synthetic_oracle_dream_mirage_action_again 按卡组剑阵
                // 总数（含自身）判定。
                // oracle 锚点：hf-latest-32308000-16f9c778
                // 401536d03d54eff4/round-06 cp14 p2.hp 12 vs 15（卡组 2 张剑阵
                // 含自身：6+3×2=12 → 防御 1 → 11；引擎 FormationUsedCount 只算
                // 已用 1 张 → 6+3=9 → 8）。
                let mut amount = other_param(card, 0).max(0);
                if realm <= 3 {
                    let formation_count = self
                        .actor(actor_side)
                        .deck
                        .slots
                        .iter()
                        .filter(|candidate| {
                            is_sword_formation_card(self.actor(actor_side), &candidate.card)
                        })
                        .count() as i64;
                    amount += formation_count * other_param(card, 1).max(0);
                } else {
                    amount += self
                        .dream_mirage_value(actor_side, DreamMirageValue::FormationUsedCount)
                        * other_param(card, 1).max(0);
                }
                self.dream_mirage_direct_damage(actor_side, target_side, amount);
                Some(false)
            }
            1_000_083 => {
                self.apply_configured_defense(actor_side, card);
                if realm >= 5 {
                    self.actor_mut(actor_side).sword.water_month_sword_formation +=
                        other_param(card, 1).max(0);
                }
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::TurnStartDefense,
                    other_param(card, 0).max(0),
                );
                if realm >= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::CloudSeaOnFormation,
                        1,
                    );
                }
                Some(false)
            }
            1_000_084 => {
                self.apply_configured_anima(actor_side, card);
                if realm <= 3 {
                    self.actor_mut(actor_side).sword.sword_energy += other_param(card, 0).max(0);
                } else {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::SwordEnergyOnSword,
                        other_param(card, 0).max(0),
                    );
                }
                Some(false)
            }
            1_000_085 => {
                if realm >= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::DoubleNextSwordIntentAndAttackBonus,
                        1,
                    );
                }
                // Printed Sword Intent is applied by the central printed-field path.
                Some(false)
            }
            1_000_086 => {
                let divisor = if has_cloud_chain(self.actor(actor_side)) {
                    other_param(card, 2)
                } else {
                    other_param(card, 0)
                };
                let extra = if realm <= 3 {
                    if self.actor(actor_side).core.anima > 0 {
                        1
                    } else {
                        0
                    }
                } else if divisor > 0 {
                    self.actor(actor_side).core.anima / divisor
                } else {
                    0
                };
                Some(self.dream_mirage_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1).max(0) + extra.max(0),
                    slot,
                ))
            }
            1_000_087 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if other_param(card, 0) > 0 {
                    self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                }
                // Card_1000087.OnExecuted: at or below JinDan the extra 狂剑 use is
                // folded directly into the KuangJian buff (sword.frenzy_sword);
                // above JinDan it installs the MengKuangEr sustain that later
                // converts turn-end healing into KuangJian.
                if realm <= 3 {
                    self.actor_mut(actor_side).sword.frenzy_sword += other_param(card, 1).max(0);
                } else {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::HealingTurnEndFrenzy,
                        other_param(card, 1).max(0),
                    );
                }
                Some(attacked)
            }

            4_000_069 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.gain_defense(
                        actor_side,
                        self.actor(actor_side).astrology.star_power.max(0)
                            * other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            4_000_070 => {
                self.apply_configured_anima(actor_side, card);
                if realm < 4 {
                    let hp = other_param(card, 0).max(0);
                    self.modify_actor_max_hp(actor_side, hp);
                    self.modify_actor_hp(actor_side, hp, false, false);
                } else {
                    self.gain_hexagram(actor_side, other_param(card, 0).max(0));
                }
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                // Card_4000070.OnExecuted calls ModifyBuffValue(GuaXiang,
                // -current), so the loss ledger and 梦御雷 refund remain
                // visible.
                self.modify_hexagram(actor_side, -hexagram);
                self.modify_star_power(actor_side, hexagram);
                if realm >= 4 {
                    let mut grid = slot;
                    for _ in 0..hexagram.min(self.dream_mirage_active_slot_count(actor_side) as i64)
                    {
                        grid = self.dream_mirage_next_grid(actor_side, grid);
                        self.add_dream_mirage_star_slot(actor_side, grid);
                    }
                }
                Some(false)
            }
            4_000_073 => {
                let names = 1 + self
                    .actor(actor_side)
                    .deck
                    .slots
                    .iter()
                    .filter(|slot_state| {
                        slot_state.card.id != card.id
                            && (slot_state.card.name.contains('雷')
                                || slot_state.card.name.contains('卦'))
                    })
                    .count() as i64;
                let value = card.attack.unwrap_or(0) + names * other_param(card, 0).max(0);
                let mut attacked = self.dream_mirage_attack_with_value(
                    actor_side,
                    value,
                    card.attack_count.unwrap_or(1),
                    slot,
                );
                if realm > 3 && self.consume_percent_roll(actor_side) < 10 {
                    attacked |= self.dream_mirage_attack_with_value(
                        actor_side,
                        value,
                        card.attack_count.unwrap_or(1),
                        slot,
                    );
                }
                Some(attacked)
            }
            4_000_075 => {
                let mut attacked = self.attack_by_config(actor_side, card, 0, slot);
                if realm >= 5 || (realm <= 4 && !was_used_before_effect) {
                    let reversed = self.actor(actor_side).fate.reverse_card_direction;
                    self.actor_mut(actor_side).fate.reverse_card_direction =
                        if reversed > 0 { 0 } else { 1 };
                    self.reverse_queue(actor_side);
                }
                let repeats = self
                    .dream_mirage_value(actor_side, DreamMirageValue::RearMoveCardUsedCount)
                    .max(0);
                if realm <= 3 && repeats > 0 {
                    attacked |= self.dream_mirage_attack_with_value(
                        actor_side,
                        other_param(card, 0).max(0),
                        card.attack_count.unwrap_or(1),
                        slot,
                    );
                } else if repeats > 0 {
                    attacked |= self.dream_mirage_attack_with_value(
                        actor_side,
                        other_param(card, 0).max(0),
                        repeats,
                        slot,
                    );
                }
                Some(attacked)
            }
            4_000_078 => {
                let hp = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamReflection,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            4_000_079 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            4_000_081 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                // Printed Hexagram is applied by the central printed-field path.
                Some(attacked)
            }
            4_000_082 => {
                let hp = other_param(card, 1).max(0);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                // Card_4000082.cs 是三个独立 if（非 else-if）：
                //   level <= JinDan → dst 内伤(100)
                //   level == JinDan → dst 虚弱(101)
                //   level >  JinDan → dst 蛇影 + 用过蛇卡时 src 身法 10
                // 金丹同时吃内伤+虚弱。oracle 锚点：hf-32308000
                // fbde66539ca72b2d/round-06 cp[16] p2.internalInjury 1（引擎
                // 0）、3f8fce148a56cd82/round-07 cp[6] p2 内伤 2 虚弱 1
                //（引擎 0/2）。
                if realm <= 3 {
                    self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                }
                if realm == 3 {
                    self.add_actor_negative_status(target_side, 101, other_param(card, 0).max(0));
                }
                if realm > 3 {
                    self.modify_dream_mirage_value(
                        target_side,
                        DreamMirageValue::SnakeShadow,
                        other_param(card, 0).max(0),
                    );
                    if self.dream_mirage_value(actor_side, DreamMirageValue::SnakeCardUsedCount) > 0
                    {
                        self.gain_agility(actor_side, 10);
                    }
                }
                Some(false)
            }
            4_000_084 => {
                // Card_4000084 uses two distinct original buffs. 炼气 through
                // 元婴 consume the printed number of uses; 化神 stacks a
                // permanent once-per-turn reward instead.
                if original_card_realm_level(card.id).unwrap_or(0) <= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::DreamStarBoardLowRealm,
                        other_param(card, 0).max(0),
                    );
                } else {
                    self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamStarBoard, 1);
                }
                Some(false)
            }
            4_000_085 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let rear_move = self.check_rear_move(actor_side, was_used_before_effect);
                let repeats = other_param(card, 0).max(0)
                    + if rear_move {
                        other_param(card, 1).max(0)
                    } else {
                        0
                    };
                for _ in 0..repeats {
                    if let Some(status) = self.consume_optional_negative_status_decision() {
                        self.add_actor_negative_status(target_side, status, 1);
                    }
                }
                Some(attacked)
            }
            4_000_087 => {
                // 梦•枯木逢春（Card_4000087.cs + BattleCharacter.CalculateAttack
                // 11627-11641，decompiled build-24646245）：
                //   4040087：每用过一次本家族牌，攻就翻倍一次 → ×2^count；
                //   其余变体（4020087 等）：只要用过就翻倍一次 → count>0 时 ×2。
                // 翻倍点位于 CalculateAttack 百分比倍率之前，覆盖星力/加攻
                // 等已累加平值（见 active_effect_attack_multiplier）。
                // oracle 锚点：97977de0a7697428/round-13 cp13 p2.hp 47 vs 49
                // （梅开二度双执行：第二段 (12+星力2)×2）、
                // fb3c4ca4a7a4e78e/round-06 cp37/cp48/cp59 p1.hp
                // （4020087 只翻倍一次，非指数）。
                let times = self
                    .dream_mirage_value(actor_side, DreamMirageValue::WitheredTreeUsedCount)
                    .max(0);
                let multiplier = if card.id == 4_040_087 {
                    if times <= 0 {
                        1
                    } else {
                        2_i64.saturating_pow(times.min(62) as u32)
                    }
                } else if times > 0 {
                    2
                } else {
                    1
                };
                let previous_multiplier = self.active_effect_attack_multiplier();
                self.set_active_effect_attack_multiplier(
                    previous_multiplier.saturating_mul(multiplier.max(1)),
                );
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.set_active_effect_attack_multiplier(previous_multiplier);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::WitheredTreeUsedCount,
                    1,
                );
                Some(attacked)
            }
            4_000_089 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if realm > 3 {
                    self.modify_star_power(actor_side, other_param(card, 1).max(0));
                } else if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                }
                Some(attacked)
            }
            7_000_074 => {
                self.activate_element(actor_side, Element::Metal);
                self.gain_sharpness(actor_side, other_param(card, 0).max(0));
                if realm > 3 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::ActionAgainSharpness,
                        other_param(card, 1).max(0),
                    );
                }
                Some(false)
            }
            7_000_075 => {
                self.apply_configured_anima(actor_side, card);
                if realm >= 5 {
                    self.execute_dream_mirage_selected_temporary_card(actor_side, slot);
                } else {
                    self.dream_mirage_direct_damage(
                        actor_side,
                        target_side,
                        other_param(card, 0).max(0),
                    );
                }
                Some(false)
            }
            7_000_076 => {
                self.apply_configured_anima(actor_side, card);
                let hp = other_param(card, 1).max(0);
                if hp > 0 {
                    self.modify_actor_max_hp(actor_side, hp);
                    self.modify_actor_hp(actor_side, hp, false, false);
                }
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                if realm >= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::TemporaryWaterDouble,
                        1,
                    );
                }
                Some(false)
            }
            7_000_077 => {
                if realm <= 3 {
                    // Card_7000077.cs:112-140：金丹及以下先攻击，再按
                    // GetWuXingCountInDeck() × otherParams[0] 加防。
                    // 高境界分支读取 JiLuWuXingPaiShiYongCiShu，不能与低境界
                    // 的卡组五行种类计数混用。
                    let attacked = self.attack_by_config(actor_side, card, 0, slot);
                    let defense =
                        wu_xing_count_in_deck(self.actor(actor_side)) * other_param(card, 0).max(0);
                    self.gain_defense(actor_side, defense.max(0));
                    Some(attacked)
                } else {
                    let defense = self
                        .dream_mirage_value(actor_side, DreamMirageValue::UsedFiveElementsCount)
                        * other_param(card, 0).max(0);
                    self.gain_defense(actor_side, defense.max(0));
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::UnconditionalFiveElements,
                        1,
                    );
                    Some(false)
                }
            }
            7_000_078 => {
                if realm > 3 {
                    let next = self.dream_mirage_next_grid(actor_side, slot);
                    if let Some(name) = self
                        .actor(actor_side)
                        .deck
                        .slots
                        .get(next)
                        .map(|slot_state| slot_state.card.name.clone())
                    {
                        self.activate_dream_mirage_elements_from_name(actor_side, &name);
                    }
                }
                self.execute_dream_mirage_selected_temporary_card(actor_side, slot);
                Some(false)
            }
            7_000_079 => {
                self.apply_configured_anima(actor_side, card);
                let divisor = other_param(card, 0);
                if divisor > 0 {
                    self.gain_water_momentum(
                        actor_side,
                        self.dream_mirage_value(actor_side, DreamMirageValue::TotalActualDamage)
                            / divisor,
                    );
                }
                Some(false)
            }
            7_000_080 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                if realm <= 3 {
                    let water_momentum = self.actor(actor_side).elements.water_momentum.max(0);
                    self.actor_mut(actor_side).turn.next_attack_bonus += water_momentum;
                } else {
                    let divisor = other_param(card, 1);
                    let bonus = if divisor > 0 {
                        (self.actor(actor_side).elements.water_momentum / divisor)
                            .min(other_param(card, 2).max(0))
                    } else {
                        0
                    };
                    self.gain_dream_mirage_attack_bonus(actor_side, bonus.max(0));
                }
                Some(false)
            }
            7_000_081 => {
                self.activate_element(actor_side, Element::Wood);
                self.apply_configured_anima(actor_side, card);
                // 原版 Card_7000081.cs:97-108 按境界分段发放：
                //   level <= 元婴: GongJiXiQuShengMing(668) += otherParams[0]
                //     （攻击吸取生命 charge，每段攻击消耗 1 层：目标 -1 血、
                //     自身 +1 血，BattleCharacter.cs:11766-11769）；
                //   level >= 元婴: MuCi(645) += otherParams[1]（木刺，持久层数
                //     伤害+等量回血，BattleCharacter.cs:11761-11764）；
                //   level == 化神: maxHp += otherParams[2]，
                //     JiaGongZhuanMuCi += otherParams[0]（加攻转木刺）。
                // oracle 锚点：mirror-32299000 01426afd87ec8719/round-10 cp6
                // p2.hp 62 vs 63（混元碎击缺 668 的 1 伤 1 回）、
                // 0e4dd80777aec990/round-08 cp5 p1.hp 63 vs 65。
                if realm <= 4 {
                    self.actor_mut(actor_side).elements.attack_life_drain +=
                        other_param(card, 0).max(0);
                }
                if realm >= 4 {
                    self.actor_mut(actor_side).elements.wood_thorn += other_param(card, 1).max(0);
                }
                if realm >= 5 {
                    self.modify_actor_max_hp(actor_side, other_param(card, 2).max(0));
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::AttackBonusToThorns,
                        other_param(card, 0).max(0),
                    );
                }
                Some(false)
            }
            7_000_082 => {
                let attack = card.attack.unwrap_or(0)
                    + self.dream_mirage_value(target_side, DreamMirageValue::LostMaxHpEventCount)
                        * other_param(card, 0).max(0);
                Some(self.dream_mirage_attack_with_value(
                    actor_side,
                    attack,
                    card.attack_count.unwrap_or(1),
                    slot,
                ))
            }
            7_000_083 => {
                let amount = other_param(card, 0).max(0)
                    + self.actor(actor_side).core.defense / other_param(card, 1).max(1);
                self.modify_actor_hp(target_side, -amount, false, false);
                self.modify_actor_max_hp(target_side, -amount);
                Some(false)
            }
            7_000_084 => {
                let defense = card.defense.unwrap_or(0).max(0)
                    + self.dream_mirage_value(actor_side, DreamMirageValue::TotalSharpnessGained)
                        / other_param(card, 0).max(1)
                    + self
                        .dream_mirage_value(actor_side, DreamMirageValue::TotalWaterMomentumGained)
                        / other_param(card, 1).max(1);
                self.gain_defense(actor_side, defense);
                if realm >= 4 {
                    self.actor_mut(actor_side).turn.next_turn_defense += defense;
                }
                Some(false)
            }
            7_000_086 => {
                self.apply_configured_defense(actor_side, card);
                if realm <= 3 {
                    self.actor_mut(actor_side).turn.next_turn_defense +=
                        other_param(card, 0).max(0);
                } else {
                    self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamCliff, 1);
                }
                Some(false)
            }
            7_000_087 => {
                // Card_7000087.cs: OnExecuted stores otherParams[1] in
                // MengTianSuiDiJingJie (the low-to-YuanYing finite-use buff).
                // The HuaShen variant uses MengTianSui and is handled as an
                // unbounded marker by the shared hook below.
                if original_card_realm_level(card.id).unwrap_or(0) <= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::FiveElementsMarrow,
                        other_param(card, 1).max(0),
                    );
                } else {
                    self.actor_mut(actor_side)
                        .dream_mirage
                        .five_elements_marrow_infinite += 1;
                }
                Some(false)
            }

            7_000_089 => {
                self.apply_configured_anima(actor_side, card);
                let amount = other_param(card, 0).max(0)
                    + self.actor(actor_side).core.anima * other_param(card, 1).max(0);
                self.modify_actor_hp(target_side, -amount, false, false);
                self.modify_actor_max_hp(target_side, -amount);
                if !was_used_before_effect {
                    for side in [actor_side, target_side] {
                        self.modify_dream_mirage_value(side, DreamMirageValue::ConsumeNextCard, 1);
                    }
                }
                Some(false)
            }
            7_000_090 => {
                self.activate_element(actor_side, Element::Fire);
                self.apply_configured_defense(actor_side, card);
                let amount = other_param(card, 0).max(0);
                self.modify_actor_hp(target_side, -amount, false, false);
                self.modify_actor_max_hp(target_side, -amount);
                if realm >= 4 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::DreamFireFormation,
                        other_param(card, 1).max(0),
                    );
                }
                Some(false)
            }
            7_000_093 => None,
            10_000_069 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamMysticFootwork,
                    if realm <= 3 { 1 } else { 2 },
                );
                if realm >= 5 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::DreamMysticFootworkHigh,
                        1,
                    );
                }
                Some(false)
            }
            10_000_071 => {
                self.apply_configured_defense(actor_side, card);
                let divisor = other_param(card, 0);
                let amount = if realm < 3 {
                    divisor.max(0)
                } else if divisor > 0 {
                    self.dream_mirage_value(actor_side, DreamMirageValue::DefenseLedger) / divisor
                } else {
                    0
                };
                self.actor_mut(actor_side).beng.beng_quan_chuo += amount.max(0);
                Some(false)
            }
            10_000_076 => {
                self.apply_configured_defense(actor_side, card);
                let (key, amount) = if realm <= 4 {
                    (
                        DreamMirageValue::NextHpGainDefense,
                        other_param(card, 0).max(0),
                    )
                } else {
                    (DreamMirageValue::HpGainDefense, 1)
                };
                self.modify_dream_mirage_value(actor_side, key, amount);
                Some(false)
            }
            10_000_077 => {
                // 梦•冥影身法（Card_10000077.cs）：金丹及以下时，若自身有
                // 负面状态，攻击 +1。原版脚本硬编码 +1（不是 otherParams[0]：
                // 10020077 的 otherParams[0] = 0，但持有 冥(367)/破绽/外伤
                // 等负面状态时每段仍多 1 攻）。oracle 锚点：
                // aa1dd3dad8d82ea6/round-07 cp3 p1.hp 48 vs 50（冥 367 计为
                // 负面）、b897036ba60289fc/round-07 cp2 p2.hp 49 vs 51（破绽）、
                // dbe54b8d15e15d01/round-06 cp8 p2.hp 4 vs 6（外伤）。
                let extra = if realm <= 3 && self.negative_status_stack_count(actor_side) > 0 {
                    1
                } else {
                    0
                };
                let attacked = self.dream_mirage_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0) + extra,
                    card.attack_count.unwrap_or(1),
                    slot,
                );
                if realm >= 4 {
                    self.gain_agility(actor_side, other_param(card, 1).max(0));
                }
                Some(attacked)
            }
            10_000_078 => {
                if realm >= 4 && self.negative_status_stack_count(actor_side) > 0 {
                    self.modify_momentum(actor_side, other_param(card, 0).max(0));
                }
                Some(
                    self.dream_mirage_attack_with_value(
                        actor_side,
                        card.attack.unwrap_or(0)
                            + self.dream_mirage_value(
                                actor_side,
                                DreamMirageValue::TotalMomentumGained,
                            ),
                        card.attack_count.unwrap_or(1),
                        slot,
                    ),
                )
            }
            10_000_080 => {
                self.apply_configured_defense(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            10_000_081 => {
                let extra = if realm >= 5 {
                    self.actor(actor_side).core.defense / other_param(card, 0).max(1)
                } else {
                    0
                };
                let mut attacked = self.dream_mirage_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1).max(0) + extra.max(0),
                    slot,
                );
                // Card_10000081.cs:67-115：炼气至元婴先完成主攻击，再读取
                // BuffType.JiLuJiaFang；曾经有过正向加防时追加
                // Attack(otherParams[0], otherParams[1])。Rust 的
                // DefenseLedger 是 BattleCharacter.ModifyDef 在
                // resources.rs:811-831 写入的共享 JiLuJiaFang 等价账本。
                if realm <= 4
                    && self.dream_mirage_value(actor_side, DreamMirageValue::DefenseLedger) > 0
                {
                    attacked |= self.dream_mirage_attack_with_value(
                        actor_side,
                        other_param(card, 0).max(0),
                        other_param(card, 1).max(0),
                        slot,
                    );
                }
                Some(attacked)
            }
            10_000_082 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if realm <= 3 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::NextBengQuanAdditionalAttack,
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            10_000_083 => {
                let defense = card.defense.unwrap_or(0).max(0)
                    + self.negative_status_stack_count(actor_side) * other_param(card, 0).max(0);
                self.gain_defense(actor_side, defense);
                Some(false)
            }
            10_000_084 => {
                if realm >= 4 {
                    self.modify_momentum_limit(actor_side, other_param(card, 1).max(0));
                } else {
                    self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                }
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                if realm >= 5 {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::FlatMomentumAttack,
                        1,
                    );
                }
                Some(false)
            }
            10_000_085 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_momentum(actor_side, other_param(card, 1).max(0));
                if realm <= 4 {
                    self.actor_mut(actor_side).beng.momentum_before_attack +=
                        other_param(card, 0).max(0);
                } else {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::MomentumBeforeEveryAttack,
                        1,
                    );
                }
                Some(false)
            }
            10_000_087 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if realm <= 3 {
                    let physique = card.physique.unwrap_or(0).max(0);
                    self.apply_physique_amount(actor_side, physique);
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::NextBengQuanPhysique,
                        physique,
                    );
                } else {
                    self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamForgeFist, 1);
                }
                Some(attacked)
            }
            10_000_088 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_dream_mirage_value(
                    actor_side,
                    if realm <= 3 {
                        DreamMirageValue::DefenseGainDamageLow
                    } else {
                        DreamMirageValue::DreamDefenseGainDamage
                    },
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            260 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_defense(
                    actor_side,
                    (self.actor(actor_side).core.anima * other_param(card, 0).max(0))
                        .min(other_param(card, 2).max(0)),
                );
                Some(attacked)
            }
            261 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).sword.all_purpose_sword += 1;
                Some(attacked)
            }
            266 => {
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                self.modify_dream_mirage_value(
                    target_side,
                    DreamMirageValue::FlowingMerciless,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            269 => {
                let mut grid = slot;
                for _ in 0..other_param(card, 0).max(0) {
                    grid = self.dream_mirage_next_grid(actor_side, grid);
                    self.add_dream_mirage_star_slot(actor_side, grid);
                }
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::StarShift,
                    other_param(card, 0).max(0),
                );
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::StarShiftAttack,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            270 => {
                self.apply_configured_anima(actor_side, card);
                let amount = other_param(card, 1).max(0);
                self.modify_actor_hp(target_side, -amount, false, false);
                self.modify_actor_max_hp(target_side, -amount);
                let cost = other_param(card, 0).max(0);
                if self.is_element_activated(actor_side, Element::Fire)
                    && self.actor(actor_side).core.anima >= cost
                {
                    self.spend_anima_unchecked(actor_side, cost);
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::RepeatNextFireOrEarth,
                        1,
                    );
                }
                Some(false)
            }
            271 => {
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::ExtraWaterMomentumTurnEnd,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            272 => {
                self.apply_configured_anima(actor_side, card);
                let heal = (self.dream_mirage_value(actor_side, DreamMirageValue::LastTurnStartHp)
                    - self.actor(actor_side).core.hp)
                    .max(0)
                    .min(other_param(card, 1).max(0));
                self.modify_actor_hp(actor_side, heal, false, false);
                if self.is_element_activated(actor_side, Element::Wood)
                    && self.actor(actor_side).add_hp_count() > 0
                {
                    self.gain_dream_mirage_attack_bonus(actor_side, other_param(card, 0).max(0));
                }
                Some(false)
            }
            273 => {
                self.actor_mut(actor_side).turn.current_turn_ignore_defense += 1;
                let attacked = self.dream_mirage_attack_with_value(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.attack_count.unwrap_or(1),
                    slot,
                );
                self.gain_sharpness(actor_side, other_param(card, 0).max(0));
                if self.is_element_activated(actor_side, Element::Metal) {
                    self.modify_dream_mirage_value(
                        actor_side,
                        DreamMirageValue::ReturnSharpness,
                        1,
                    );
                }
                Some(attacked)
            }
            277 => {
                if other_param(card, 2) > 0 {
                    self.modify_actor_hp(actor_side, other_param(card, 2), false, false);
                }
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::ExcessPhysiqueHp,
                    other_param(card, 0).max(0),
                );
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::ExcessPhysiqueDamage,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            395 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.gain_agility(actor_side, other_param(card, 1).max(0));
                self.execute_dream_mirage_selected_temporary_card(actor_side, slot);
                Some(false)
            }
            26 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_dream_mirage_value(actor_side, DreamMirageValue::SpiritCatCloud, 1);
                Some(false)
            }
            79 => {
                self.gain_dream_mirage_attack_bonus(actor_side, other_param(card, 0).max(0));
                self.gain_guard(actor_side, other_param(card, 1).max(0));
                let duration = other_param(card, 2).max(0);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DragonExtraActionImmunity,
                    duration,
                );
                self.increase_dream_mirage_action_again_limit(actor_side, duration, 8);
                Some(false)
            }
            198 => None,
            _ => None,
        }
    }

    fn dream_mirage_attack_with_value(
        &mut self,
        actor_side: PlayerSide,
        attack: i64,
        count: i64,
        slot: usize,
    ) -> bool {
        if attack <= 0 || count <= 0 {
            return false;
        }
        for _ in 0..count {
            self.apply_attack(actor_side, attack, slot);
        }
        true
    }

    fn dream_mirage_direct_damage(
        &mut self,
        actor_side: PlayerSide,
        target_side: PlayerSide,
        amount: i64,
    ) {
        if amount > 0 {
            self.apply_damage_to(actor_side, target_side, amount, false, false, false);
        }
    }

    fn dream_mirage_active_slot_count(&self, actor_side: PlayerSide) -> usize {
        self.actor(actor_side)
            .deck
            .active_slot_count
            .min(self.actor(actor_side).deck.slots.len())
            .max(1)
    }

    fn dream_mirage_next_grid(&self, actor_side: PlayerSide, grid: usize) -> usize {
        let count = self.dream_mirage_active_slot_count(actor_side);
        let step = if self.actor(actor_side).fate.reverse_card_direction > 0 {
            -1
        } else {
            1
        };
        (grid as i64 + step).rem_euclid(count as i64) as usize
    }

    fn dream_mirage_previous_grid(&self, actor_side: PlayerSide, grid: usize) -> usize {
        let count = self.dream_mirage_active_slot_count(actor_side);
        let step = if self.actor(actor_side).fate.reverse_card_direction > 0 {
            1
        } else {
            -1
        };
        (grid as i64 + step).rem_euclid(count as i64) as usize
    }

    fn add_dream_mirage_star_slot(&mut self, actor_side: PlayerSide, grid: usize) {
        if self.actor(actor_side).astrology.star_slots.contains(&grid) {
            self.gain_anima(actor_side, 1);
        } else {
            self.actor_mut(actor_side).astrology.star_slots.push(grid);
        }
    }

    fn ensure_dream_mirage_star_slot(&mut self, actor_side: PlayerSide, grid: usize) {
        if !self.actor(actor_side).astrology.star_slots.contains(&grid) {
            self.actor_mut(actor_side).astrology.star_slots.push(grid);
        }
    }

    fn activate_dream_mirage_elements_from_name(&mut self, actor_side: PlayerSide, name: &str) {
        for (token, element) in [
            ("金灵", Element::Metal),
            ("木灵", Element::Wood),
            ("水灵", Element::Water),
            ("火灵", Element::Fire),
            ("土灵", Element::Earth),
        ] {
            if name.contains(token) {
                self.activate_element(actor_side, element);
            }
        }
    }

    fn execute_dream_mirage_selected_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        outer_slot: usize,
    ) {
        if self.decision_tape.is_empty() {
            return;
        }
        let selected_id = self.consume_optional_decision();
        if selected_id < 0 {
            return;
        }
        if original_card_definition(selected_id).is_none() {
            self.missing_decision("dream-mirage selected temporary card config");
            return;
        }
        self.execute_dream_mirage_temporary_card(actor_side, outer_slot, selected_id, outer_slot);
    }
}

fn dream_mirage_realm(card: &CardDefinition) -> i64 {
    original_card_realm_level(card.id).unwrap_or(0)
}
