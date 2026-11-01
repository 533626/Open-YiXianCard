//! 七星阁 star-slot family combinator.
//!
//! Reached from the QiXing sect chain (`card_routing`) and, for the legacy
//! three-digit id 53, from the shared primary archive.

use super::flow_card_effect::{param0, param1};
use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    /// 七星阁 star-slot family combinator.
    pub(super) fn apply_qi_xing_star_slot_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            4_000_102 => {
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                self.apply_attack(actor_side, attack, slot);
                if self.consume_percent_roll(actor_side) < 1 {
                    self.apply_attack(actor_side, other_param(card, 0), slot);
                }
                self.actor_mut(actor_side).fate.exorcism += param1(card);
                Some(true)
            }
            4_000_104 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).fate.xing_yi_lian += param0(card);
                Some(attacked)
            }
            4_000_106 => {
                // Card_4000106: LingYuanWaiFang duration, then ModifyAnima,
                // then ModifyHp(otherParams[0] + anima). Overflow reflect is
                // BattleCharacter.ModifyHp:10110 (shared hook).
                self.actor_mut(actor_side).astrology.ling_yuan_wai_fang += param1(card);
                self.apply_configured_anima(actor_side, card);
                let amount = other_param(card, 0) + self.actor(actor_side).core.anima;
                self.modify_actor_hp(actor_side, amount, false, false);
                Some(false)
            }
            4_000_108 => {
                self.apply_configured_anima(actor_side, card);
                let defense = self.consume_random_range(
                    actor_side,
                    card.defense.unwrap_or(0),
                    card.random_defense.unwrap_or(card.defense.unwrap_or(0)),
                );
                let adjacent_thunder =
                    super::support::active_neighbor_card(self.actor(actor_side), slot, -1)
                        .is_some_and(|c| c.name.contains('雷'))
                        || super::support::active_neighbor_card(self.actor(actor_side), slot, 1)
                            .is_some_and(|c| c.name.contains('雷'));
                if adjacent_thunder {
                    self.apply_attack(actor_side, defense, slot);
                } else {
                    self.gain_defense(actor_side, defense);
                }
                if self.consume_percent_roll(actor_side) < 10 {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(adjacent_thunder)
            }
            4_000_105 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0),
                );
                self.modify_dream_mirage_value(
                    opponent_side(actor_side),
                    super::cards_dream_mirage::DreamMirageValue::SnakeShadow,
                    other_param(card, 1),
                );
                self.modify_extra_actions(actor_side, 1);
                Some(false)
            }
            4_000_107 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if slot.is_multiple_of(2) {
                    self.modify_star_power(actor_side, other_param(card, 0));
                } else {
                    let amount = self
                        .actor(actor_side)
                        .astrology
                        .star_power
                        .min(other_param(card, 1));
                    self.modify_star_power(actor_side, -amount);
                    self.modify_guard(actor_side, amount);
                }
                Some(attacked)
            }
            4_000_109 => {
                let bonus = if self.known_negative_status_count(opponent_side(actor_side)) > 0 {
                    other_param(card, 0)
                } else {
                    0
                };
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0) + bonus);
                if slot.is_multiple_of(2) {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        100,
                        other_param(card, 1),
                    );
                } else {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        105,
                        other_param(card, 2),
                    );
                }
                Some(false)
            }
            4_000_103 => {
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                if !was_used_before_effect {
                    self.actor_mut(actor_side).turn.jump_to_previous_card += 1;
                }
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    let defense = self.actor(actor_side).dream_mirage.turn_hp_gained
                        / other_param(card, 1).max(1);
                    self.gain_defense(actor_side, defense);
                }
                Some(false)
            }
            4_000_002 => {
                self.apply_configured_defense(actor_side, card);
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                if self.yi_gua_self_resolution(actor_side) {
                    self.add_actor_negative_status(opponent_side(actor_side), 101, 2);
                }
                Some(false)
            }
            4_000_004 | 4_000_030 => {
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                self.apply_attack(actor_side, attack, slot);
                Some(true)
            }
            4_000_031 => {
                if self.consume_percent_roll(actor_side) < other_param(card, 0) {
                    self.add_actor_negative_status(opponent_side(actor_side), 102, param1(card));
                }
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                Some(attacked)
            }
            4_000_035 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.consume_percent_roll(actor_side) < other_param(card, 0) {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(attacked)
            }
            4_000_005 => {
                // The random attack resolves before the printed anima gain.
                // Otherwise 灵卦术-created hexagram from this card's own anima
                // is immediately consumed by the same random roll.
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                self.apply_attack(actor_side, attack, slot);
                self.apply_configured_anima(actor_side, card);
                Some(attack > 0)
            }
            4_000_006 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let defense = self.consume_random_range(
                    actor_side,
                    card.defense.unwrap_or(0),
                    card.random_defense.unwrap_or(card.defense.unwrap_or(0)),
                );
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
                Some(attacked)
            }
            4_000_008 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.modify_star_power(actor_side, param0(card));
                Some(false)
            }
            4_000_009 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.gain_anima(actor_side, param0(card));
                }
                Some(attacked)
            }
            4_000_010 => {
                let primary_attacked = self.attack_by_config(actor_side, card, 0, slot);
                let mut extra = false;
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    let extra_attack = param0(card);
                    if extra_attack > 0 {
                        self.apply_attack(actor_side, extra_attack, slot);
                        extra = true;
                    }
                }
                Some(primary_attacked || extra)
            }
            4_000_011 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.gain_defense(actor_side, other_param(card, 0));
                }
                Some(attacked)
            }
            53 => {
                // 星弈·双飞燕：星位加攻；后招成功时攻击次数 +1。
                let in_star_slot = self.actor(actor_side).astrology.star_slots.contains(&slot);
                let attack = card.attack.unwrap_or(0) + if in_star_slot { param0(card) } else { 0 };
                let rear_move_bonus_count =
                    if self.check_rear_move(actor_side, was_used_before_effect) {
                        1
                    } else {
                        0
                    };
                let attack_count = card.attack_count.unwrap_or(if attack > 0 { 1 } else { 0 })
                    + rear_move_bonus_count;
                for _ in 0..attack_count.max(0) {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                Some(attack > 0 && attack_count > 0)
            }
            4_000_013 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.modify_actor_hp(actor_side, param0(card), false, false);
                }
                Some(attacked)
            }
            4_000_015 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                let mut star_slots = param0(card);
                if self.yi_gua_self_resolution(actor_side) {
                    star_slots += 4;
                }
                self.add_following_star_slots(actor_side, slot, star_slots);
                Some(false)
            }
            4_000_016 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                let amount = param0(card);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                if self.yi_gua_self_resolution(actor_side) {
                    self.modify_actor_max_hp(actor_side, 10);
                    self.modify_actor_hp(actor_side, 10, false, false);
                }
                Some(false)
            }
            _ => None,
        }
    }
}
