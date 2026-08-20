use super::support::{div_ceil, opponent_side, other_param};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_dream_mirage_direct_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        match base_id {
            265 => {
                // Card_265 reads both resources after the printed Anima cost has
                // already been paid by the transaction.
                let target_missing_hp =
                    (self.actor(target_side).core.max_hp - self.actor(target_side).core.hp).max(0);
                let anima = self.actor(actor_side).core.anima.max(0);
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                let max_hp_loss = target_missing_hp
                    + other_param(card, 0).max(0)
                    + anima * other_param(card, 1).max(0)
                    + hexagram * other_param(card, 2).max(0);
                self.reduce_anima_unchecked(actor_side, anima);
                // Card_265.cs:83-88 uses ModifyBuffValue, not SetBuffValue.
                self.modify_hexagram(actor_side, -hexagram);
                self.modify_actor_max_hp(target_side, -max_hp_loss);
                Some(false)
            }
            274 => {
                let lost_defense = self.apply_half_defense_damage(actor_side, card);
                if self.check_wu_xing(actor_side, Element::Earth) {
                    self.actor_mut(actor_side).turn.next_turn_defense += lost_defense;
                }
                Some(false)
            }
            297 => {
                self.apply_configured_anima(actor_side, card);
                let debuff_count = self.negative_status_stack_count(target_side);
                self.reduce_all_actor_negative_statuses(target_side, i64::MAX);
                self.add_actor_negative_status(target_side, 105, debuff_count);
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                    other_param(card, 0).max(0);
                Some(attacked)
            }
            298 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let recovery_before = self.actor(actor_side).status.recovery;
                self.actor_mut(actor_side).status.recovery += other_param(card, 0).max(0);
                self.record_counter_transition(
                    actor_side,
                    "状态",
                    "recovery",
                    "恢复",
                    recovery_before,
                    self.actor(actor_side).status.recovery,
                );
                let hp_gain =
                    other_param(card, 1).max(0) + self.actor(actor_side).status.recovery.max(0);
                self.modify_actor_max_hp(actor_side, hp_gain);
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                Some(attacked)
            }
            307 => {
                self.apply_configured_defense(actor_side, card);
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0).max(0));
                let divisor = other_param(card, 0);
                let extra = if divisor > 0 {
                    self.actor(actor_side).astrology.hexagram.max(0) / divisor
                } else {
                    0
                };
                self.modify_paint_finishing_touch(actor_side, 1 + extra);
                Some(false)
            }
            320 => {
                self.apply_configured_anima(actor_side, card);
                let current_anima = self.actor(actor_side).core.anima.max(0);
                let current_hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                let gain = (current_anima + current_hexagram) * other_param(card, 0).max(0);
                self.gain_defense(actor_side, gain);
                self.modify_actor_hp(actor_side, gain, false, false);

                if current_anima > 0 {
                    self.reduce_anima_unchecked(actor_side, current_anima);
                    self.gain_hexagram(actor_side, current_anima);
                }
                if current_hexagram > 0 {
                    // Card_320.cs:165-173 converts in this exact order and
                    // loses the captured amount through ModifyBuffValue.
                    self.modify_hexagram(actor_side, -current_hexagram);
                    self.gain_anima(actor_side, current_hexagram);
                }
                Some(false)
            }
            343 => {
                self.add_actor_negative_status(target_side, 105, other_param(card, 0).max(0));
                self.modify_next_attack_shatter_defense(actor_side, other_param(card, 1).max(0));
                if !was_used_before_effect {
                    self.add_actor_negative_status(target_side, 104, other_param(card, 2).max(0));
                }
                Some(false)
            }
            1_000_072 => {
                self.apply_configured_anima(actor_side, card);
                // The common printed-field stage adds Sword Intent after this
                // body. Card_1000072 nevertheless calculates Frenzy from the
                // post-gain value, so include the pending printed gain here.
                let sword_intent = self.actor(actor_side).sword.sword_intent.max(0)
                    + card.sword_intent.unwrap_or(0).max(0);
                let realm = super::original_config::original_card_realm_level(card.id).unwrap_or(0);
                if realm >= 4 {
                    let divisor = other_param(card, 0);
                    let scaled = if divisor > 0 {
                        sword_intent / divisor
                    } else {
                        0
                    };
                    self.actor_mut(actor_side).sword.frenzy_sword += scaled + i64::from(realm == 5);
                }
                Some(false)
            }
            1_000_073 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let drain =
                    self.actor(actor_side).sword.frenzy_sword.max(0) * other_param(card, 1).max(0);
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0));
                if drain > 0 {
                    self.modify_actor_hp(target_side, -drain, false, false);
                    self.modify_actor_hp(actor_side, drain, false, false);
                }
                Some(attacked)
            }
            7_000_091 => {
                let lost_defense = self.apply_half_defense_damage(actor_side, card);
                let realm = super::original_config::original_card_realm_level(card.id).unwrap_or(0);
                if realm >= 4 {
                    self.modify_actor_max_hp(actor_side, lost_defense);
                    self.modify_actor_hp(actor_side, lost_defense, false, false);
                }
                Some(false)
            }
            _ => None,
        }
    }

    fn apply_half_defense_damage(&mut self, actor_side: PlayerSide, card: &CardDefinition) -> i64 {
        self.apply_configured_defense(actor_side, card);
        let lost_defense = div_ceil(self.actor(actor_side).core.defense, 2);
        self.lose_defense(actor_side, lost_defense);
        self.apply_damage(actor_side, lost_defense, false, false, false);
        lost_defense
    }
}
