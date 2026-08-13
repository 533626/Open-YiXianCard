use super::support::{
    active_neighbor_card, elements_in_card_name, has_cloud_chain, opponent_side, other_param,
    wu_xing_count_in_deck,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_ronghui_early_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            136 => {
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                // Card_136.cs:88-89 directly calls ModifyBuffValue on the
                // two JiHuo* counters.  This is deliberately not the
                // semantic ActiveWuXing/activate_element path: generated
                // element/full semantic callbacks do not run, while direct
                // JiHuo gain hooks such as Talent 138 still do.
                self.add_direct_element_activation(actor_side, Element::Water);
                self.add_direct_element_activation(actor_side, Element::Fire);
                Some(false)
            }
            143 => {
                let divisor = other_param(card, 0);
                let bonus = if divisor > 0 {
                    self.actor(actor_side).turn.lost_defense_count.max(0) / divisor
                } else {
                    0
                };
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            169 => {
                let bonus =
                    wu_xing_count_in_deck(self.actor(actor_side)) * other_param(card, 0).max(0);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            180 => {
                self.apply_configured_anima(actor_side, card);
                let hp_gain = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, hp_gain);
                self.modify_actor_hp(actor_side, hp_gain, false, false);
                Some(false)
            }
            183 => {
                self.apply_elixir_base(actor_side, card);
                self.modify_sword_intent(actor_side, card.sword_intent.unwrap_or(0).max(0));
                self.modify_actor_max_hp(actor_side, other_param(card, 1).max(0));
                let healing =
                    self.actor(actor_side).sword.sword_intent.max(0) * other_param(card, 0).max(0);
                if healing > 0 {
                    self.modify_actor_hp(actor_side, healing, false, false);
                }
                Some(false)
            }
            184 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        101,
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            197 => {
                self.apply_configured_defense(actor_side, card);
                let next_elements = active_neighbor_card(self.actor(actor_side), slot, 1)
                    .map(elements_in_card_name)
                    .unwrap_or_default();
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                if !was_used_before_effect {
                    self.gain_guard(actor_side, 1);
                }
                if let Some(element) = next_elements.first().copied() {
                    self.activate_element(actor_side, element);
                }
                Some(false)
            }
            200 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                let next_elements = active_neighbor_card(self.actor(actor_side), slot, 1)
                    .map(elements_in_card_name)
                    .unwrap_or_default();
                if !next_elements.is_empty() {
                    for element in next_elements {
                        self.activate_element(actor_side, element);
                    }
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(false)
            }
            204 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                    other_param(card, 0).max(0);
                self.gain_agility(actor_side, other_param(card, 1).max(0));
                Some(false)
            }
            391 => {
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            _ => None,
        }
    }
}
