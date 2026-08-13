use super::support::{
    is_spirit_sword_for_actor, is_sword_formation_card, opponent_side, other_param,
};
use super::{DrawnCard, Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_dream_fate_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            49 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .sword
                    .hundred_beast_spirit_sword_formation += 1;
                Some(false)
            }
            54 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                self.set_active_effect_action_again(
                    self.negative_status_stack_count(target_side) > 0,
                );
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                }
                Some(attacked)
            }
            58 => {
                self.apply_configured_anima(actor_side, card);
                let sharpness =
                    self.actor(actor_side).core.anima.max(0) * other_param(card, 0).max(0);
                self.gain_sharpness(actor_side, sharpness);
                Some(false)
            }
            333 => {
                self.apply_configured_defense(actor_side, card);
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                if card.id == 10_333 && !was_used_before_effect {
                    self.gain_guard(actor_side, 1);
                }
                Some(false)
            }
            338 => {
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).fate.instant_shadow_strike +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            342 => {
                self.apply_configured_anima(actor_side, card);
                let statuses = self.negative_status_types_present(actor_side);
                for status in statuses {
                    self.remove_actor_negative_status(
                        actor_side,
                        status,
                        other_param(card, 0).max(0),
                    );
                }
                self.actor_mut(actor_side).fate.exorcism += other_param(card, 1).max(0);
                Some(false)
            }
            379 => {
                let target_side = opponent_side(actor_side);
                if self.actor(target_side).core.hp <= other_param(card, 1) {
                    self.actor_mut(target_side).core.hp = -100;
                } else {
                    self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                }
                Some(false)
            }
            4_000_080 => {
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                let hp_loss = other_param(card, 0).max(0) + hexagram * other_param(card, 1).max(0);
                // Card_4000080.cs:82-85 uses ModifyBuffValue, so losing all
                // current Hexagram must still update the loss ledger and run
                // 梦御雷's refund hook.
                self.modify_hexagram(actor_side, -hexagram);
                self.modify_target_hp(actor_side, -hp_loss);
                Some(false)
            }
            10_000_072 => {
                let anima_bonus = if card.id >= 10_030_072 {
                    self.actor(actor_side).core.anima.max(0)
                } else {
                    0
                };
                let attacked = self.attack_by_config(actor_side, card, anima_bonus, slot);
                self.gain_defense(actor_side, card.defense.unwrap_or(0).max(0) + anima_bonus);
                self.apply_physique_amount(
                    actor_side,
                    card.physique.unwrap_or(0).max(0) + anima_bonus,
                );
                Some(attacked)
            }
            10_000_075 => {
                self.apply_configured_physique(actor_side, card);
                let defense = if card.id >= 10_030_075 {
                    self.actor(actor_side).core.max_hp.max(0) * other_param(card, 0).max(0) / 100
                } else {
                    card.defense.unwrap_or(0).max(0)
                };
                self.gain_defense(actor_side, defense);
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn apply_hundred_beast_spirit_sword_formation_after_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let formation = self
            .actor(actor_side)
            .sword
            .hundred_beast_spirit_sword_formation
            .max(0);
        if formation <= 0
            || self
                .actor(actor_side)
                .sword
                .hundred_beast_spirit_sword_formation_triggered
            || (!is_sword_formation_card(self.actor(actor_side), card)
                && !is_spirit_sword_for_actor(self.actor(actor_side), card))
        {
            return;
        }
        let damage = self.actor(actor_side).core.anima.max(0) * formation;
        if damage > 0 {
            self.apply_damage(actor_side, damage, false, false, false);
        }
        self.actor_mut(actor_side)
            .sword
            .hundred_beast_spirit_sword_formation_triggered = true;
    }

    pub(super) fn reset_hundred_beast_spirit_sword_formation_marker(
        &mut self,
        actor_side: PlayerSide,
    ) {
        self.actor_mut(actor_side)
            .sword
            .hundred_beast_spirit_sword_formation_triggered = false;
    }

    pub(super) fn apply_wave_cutting_seal_opening(&mut self, actor_side: PlayerSide) {
        self.activate_element(actor_side, Element::Metal);
        self.activate_element(actor_side, Element::Water);
    }

    pub(super) fn should_skip_card_with_instant_shadow_strike(
        &self,
        actor_side: PlayerSide,
        drawn: &DrawnCard,
    ) -> bool {
        self.actor(actor_side).fate.instant_shadow_strike > 0 && drawn.card.hp_cost.unwrap_or(0) > 0
    }

    pub(super) fn apply_instant_shadow_strike_skip(
        &mut self,
        actor_side: PlayerSide,
        drawn: &DrawnCard,
    ) {
        let printed_hp_cost = drawn.card.hp_cost.unwrap_or(0).max(0);
        assert!(
            printed_hp_cost > 0 && self.actor(actor_side).fate.instant_shadow_strike > 0,
            "instant shadow strike received an ineligible card"
        );
        self.modify_actor_hp(actor_side, -printed_hp_cost, true, true);
        self.apply_damage(actor_side, printed_hp_cost * 2, false, false, false);
        self.actor_mut(actor_side)
            .return_card_to_tail(drawn.source_slot);
        self.actor_mut(actor_side).fate.instant_shadow_strike -= 1;
    }
}
