use super::super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::support::{card_rarity, div_ceil, opponent_side, other_param};

impl ReplayState {
    pub(super) fn apply_card_effect_misc(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            20 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.activate_element(actor_side, Element::Wood);
                Some(attacked)
            }
            3_000_010 => {
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                let defense_gain =
                    (anima * other_param(card, 0).max(0)).min(other_param(card, 1).max(0));
                if defense_gain > 0 {
                    self.gain_defense(actor_side, defense_gain);
                }
                Some(false)
            }
            6_000_012 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).turn.next_card_action_again += 1;
                Some(false)
            }
            3_000_013 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            3_000_015 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                self.add_actor_negative_status(target_side, 102, other_param(card, 1).max(0));
                self.add_actor_negative_status(target_side, 101, other_param(card, 2).max(0));
                Some(false)
            }
            4_000_026 => {
                self.apply_configured_anima(actor_side, card);
                let hexagram_gain = card
                    .hexagram
                    .filter(|value| *value > 0)
                    .unwrap_or(card_rarity(card) + 1);
                self.gain_hexagram(actor_side, hexagram_gain);
                Some(false)
            }
            4_000_044 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.modify_star_chess_break(
                        opponent_side(actor_side),
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            4_000_059 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                let drain = self.actor(target_side).status.internal_injury.max(0);
                if drain > 0 {
                    self.modify_actor_hp(target_side, -drain, false, false);
                }
                Some(false)
            }
            99_000_108 => {
                let threshold = other_param(card, 0).max(0);
                let attack = card.attack.unwrap_or(0)
                    * if self.actor(actor_side).core.hp < threshold {
                        2
                    } else {
                        1
                    };
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    Some(true)
                } else {
                    Some(false)
                }
            }
            216 => {
                self.actor_mut(actor_side).status.back_solitude += 1;
                Some(false)
            }
            217 => {
                self.actor_mut(actor_side).status.strike_void += 1;
                Some(false)
            }
            215 => {
                self.apply_configured_anima(actor_side, card);
                match card_rarity(card) {
                    0 => {
                        self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                    }
                    1 => {
                        self.apply_configured_defense(actor_side, card);
                    }
                    2 => {
                        let anima = self.actor(actor_side).core.anima.max(0);
                        self.add_actor_negative_status(opponent_side(actor_side), 100, anima);
                        self.spend_anima_unchecked(actor_side, div_ceil(anima, 2));
                    }
                    _ => {}
                }
                Some(false)
            }
            _ => None,
        }
    }
}
