use super::support::{
    active_neighbor_card, elements_in_card_name, other_param, wu_xing_count_in_deck,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_fate_strategy_element_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        _was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let mut attacked = false;
        match base_id {
            7_000_095 => {
                let attack = card.attack.unwrap_or(0) + self.actor(actor_side).core.anima;
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            7_000_096 => {
                self.apply_configured_defense(actor_side, card);
            }
            7_000_097 => {
                if self.is_element_activated(actor_side, Element::Wood) {
                    self.actor_mut(actor_side).turn.wood_spring_turns +=
                        other_param(card, 1).max(0);
                }
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
            }
            7_000_098 => {
                self.apply_configured_anima(actor_side, card);
                let missing_attack_bonus =
                    (other_param(card, 0) - self.actor(actor_side).core.attack_bonus).max(0);
                self.gain_attack_bonus(actor_side, missing_attack_bonus);
                let max_hp_loss = missing_attack_bonus * other_param(card, 1).max(0);
                if max_hp_loss > 0 {
                    self.modify_actor_max_hp(actor_side, -max_hp_loss);
                }
                if self.is_element_activated(actor_side, Element::Fire) {
                    let loss =
                        self.actor(actor_side).core.attack_bonus * other_param(card, 2).max(0);
                    if loss > 0 {
                        self.modify_target_hp(actor_side, -loss);
                        self.modify_target_max_hp(actor_side, -loss);
                    }
                }
            }
            7_000_099 => {
                let attack = card.attack.unwrap_or(0);
                let attack_count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0);
                let return_percent = if self.is_element_activated(actor_side, Element::Metal) {
                    other_param(card, 0).max(0)
                } else {
                    0
                };
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack_with_options(
                            actor_side,
                            attack,
                            slot,
                            false,
                            false,
                            return_percent,
                            None,
                        );
                        attacked = true;
                    }
                }
            }
            7_000_100 => {
                let momentum_gain = other_param(card, 0).max(0);
                if momentum_gain > 0 {
                    self.gain_water_momentum(actor_side, momentum_gain);
                }
                let hp_gain = other_param(card, 1).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                if self.is_element_activated(actor_side, Element::Water) {
                    let agility_cap = other_param(card, 2).max(0);
                    let agility_gain = ((self.actor(actor_side).elements.water_momentum
                        + self.actor(actor_side).core.anima)
                        * 2)
                    .min(agility_cap);
                    self.gain_agility(actor_side, agility_gain);
                }
            }
            7_000_101 => {
                // Card_7000101.cs:100-114 混元化灵:
                //   ModifyAnima(cardConfig.anima + GetWuXingCountInDeck())
                //   ActiveWuXingInName(prev.name); ActiveWuXingInName(next.name)
                // ActiveWuXingInName 按名字激活其中出现的全部五行
                // （BattleCharacter.cs:9216-9237），非单元素映射。
                let deck_bonus = self.element_count_in_deck(actor_side);
                let anima_gain = card.anima.filter(|value| *value > 0).unwrap_or(0) + deck_bonus;
                if anima_gain > 0 {
                    self.gain_anima(actor_side, anima_gain);
                }
                let previous = active_neighbor_card(self.actor(actor_side), slot, -1).cloned();
                let next = active_neighbor_card(self.actor(actor_side), slot, 1).cloned();
                if let Some(prev_card) = previous {
                    for element in elements_in_card_name(&prev_card) {
                        self.activate_element(actor_side, element);
                    }
                }
                if let Some(next_card) = next {
                    for element in elements_in_card_name(&next_card) {
                        self.activate_element(actor_side, element);
                    }
                }
            }
            _ => return None,
        }
        Some(attacked)
    }

    pub(super) fn element_count_in_deck(&self, actor_side: PlayerSide) -> i64 {
        wu_xing_count_in_deck(self.actor(actor_side))
    }
}
