//! 七星阁 star-slot family combinator.
//!
//! Reached from the QiXing sect chain (`card_routing`) and, for the legacy
//! three-digit id 53, from the shared primary archive.

use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
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
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        102,
                        other_param(card, 1).max(0),
                    );
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
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            4_000_009 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.gain_anima(actor_side, other_param(card, 0).max(0));
                }
                Some(attacked)
            }
            4_000_010 => {
                let primary_attacked = self.attack_by_config(actor_side, card, 0, slot);
                let mut extra = false;
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    let extra_attack = other_param(card, 0).max(0);
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
                let attack = card.attack.unwrap_or(0)
                    + if in_star_slot {
                        other_param(card, 0).max(0)
                    } else {
                        0
                    };
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
                    self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                }
                Some(attacked)
            }
            4_000_015 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                let mut star_slots = other_param(card, 0).max(0);
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
                let amount = other_param(card, 0).max(0);
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
