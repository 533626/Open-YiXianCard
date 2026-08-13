use super::support::{
    card_rarity, is_five_element_control, neighbor_card, opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

impl ReplayState {
    pub(super) fn apply_hexagram_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            4_000_001 => {
                self.apply_configured_anima(actor_side, card);
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                if self.yi_gua_self_resolution(actor_side) {
                    self.gain_anima(actor_side, 4);
                }
                Some(false)
            }
            4_000_003 => {
                // 震卦：攻击并加卦象；易卦自解触发时对目标施加 4 层破绽。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let hexagram = card.hexagram.filter(|value| *value > 0).unwrap_or(1);
                self.gain_hexagram(actor_side, hexagram);
                if self.yi_gua_self_resolution(actor_side) {
                    self.add_actor_negative_status(opponent_side(actor_side), 102, 4);
                }
                Some(attacked)
            }
            4_000_034 => {
                let hexagram_gain = card
                    .hexagram
                    .filter(|value| *value > 0)
                    .unwrap_or(card_rarity(card) + 3);
                self.gain_hexagram(actor_side, hexagram_gain);
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    let target_side = opponent_side(actor_side);
                    self.modify_actor_hp(target_side, -amount, false, false);
                    self.modify_actor_max_hp(target_side, -amount);
                }
                if self.yi_gua_self_resolution(actor_side) {
                    self.gain_hexagram(actor_side, 1);
                }
                Some(false)
            }
            4_000_017 => {
                let min_heal = other_param(card, 0).max(0);
                let heal = self.consume_random_range(
                    actor_side,
                    min_heal,
                    other_param(card, 1).max(min_heal),
                );
                if min_heal > 0 {
                    self.modify_actor_max_hp(actor_side, min_heal);
                }
                self.modify_actor_hp(actor_side, heal.max(0), false, false);
                Some(false)
            }
            4_000_018 => {
                self.apply_configured_anima(actor_side, card);
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                }
                Some(false)
            }
            4_000_019 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let amount = self.consume_random_range(
                    actor_side,
                    other_param(card, 0),
                    other_param(card, 1),
                );
                if amount > 0 {
                    self.modify_target_max_hp(actor_side, -amount);
                }
                Some(attacked)
            }
            4_000_025 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                let anima = other_param(card, 0);
                self.gain_anima(actor_side, anima);
                self.gain_anima(opponent_side(actor_side), anima);
                if self.yi_gua_self_resolution(actor_side) {
                    self.add_actor_negative_status(opponent_side(actor_side), 100, 3);
                }
                Some(false)
            }
            4_000_036 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.gain_guard(actor_side, other_param(card, 1).max(0));
                }
                Some(false)
            }
            4_000_041 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.actor_mut(actor_side).fate.plum_blossom_twice += 1;
                Some(false)
            }
            4_000_045 => {
                let star_gain = other_param(card, 0).max(0) + if slot == 0 { 1 } else { 0 };
                self.modify_star_power(actor_side, star_gain);
                self.actor_mut(actor_side).astrology.star_slots = (0..DECK_SIZE).collect();
                Some(false)
            }
            4_000_046 => {
                let mut count = 0;
                for _ in 0..5 {
                    if self.consume_percent_roll(actor_side) < other_param(card, 0) {
                        count += 1;
                    }
                }
                let damage = other_param(card, 1).max(0);
                for _ in 0..count {
                    if damage > 0 {
                        self.apply_attack(actor_side, damage, slot);
                    }
                }
                Some(count > 0 && damage > 0)
            }
            4_000_063 => {
                let attack_count = card.attack_count.unwrap_or(1).max(0);
                let mut attacked = false;
                for _ in 0..attack_count {
                    let had_hexagram = self.actor(actor_side).astrology.hexagram > 0;
                    let attack = self.consume_random_range(
                        actor_side,
                        card.attack.unwrap_or(0),
                        card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                    );
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                    if had_hexagram {
                        self.gain_anima(actor_side, 1);
                    }
                }
                Some(attacked)
            }
            4_000_066 => {
                if self.actor(actor_side).fate.reverse_card_direction > 0 {
                    self.actor_mut(actor_side).fate.reverse_card_direction = 0;
                } else {
                    self.actor_mut(actor_side).fate.reverse_card_direction = 1;
                }
                self.reverse_queue(actor_side);
                self.actor_mut(actor_side).fate.yellow_bird_behind += other_param(card, 0).max(0);
                Some(false)
            }
            4_000_067 => {
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima;
                let hexagram = self.actor(actor_side).astrology.hexagram;
                let star_power = self.actor(actor_side).astrology.star_power;
                let gain = other_param(card, 0).max(0);
                if anima >= hexagram && anima >= star_power {
                    self.gain_anima(actor_side, gain);
                } else if hexagram >= anima && hexagram >= star_power {
                    self.gain_hexagram(actor_side, gain);
                } else if star_power >= hexagram && star_power >= anima {
                    self.modify_star_power(actor_side, gain);
                }
                Some(false)
            }
            4_000_088 => {
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                }
                let realm = super::original_config::original_card_realm_level(card.id).unwrap_or(1);
                if realm <= 4 {
                    // Card_4000088.cs:86-92 reads hidden buff 700 without
                    // clearing it. Build 24180265 restores the accumulated
                    // positive Hexagram-loss ledger.
                    let lost_hexagram = self.original_lost_hexagram_ledger(actor_side);
                    if lost_hexagram > 0 {
                        self.modify_hexagram(actor_side, lost_hexagram);
                    }
                } else {
                    // Card_4000088.cs:94-97 installs the persistent 梦御雷 hook.
                    self.actor_mut(actor_side).astrology.dream_thunder_hexagram += 1;
                }
                Some(attack > 0)
            }
            32 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                let previous = neighbor_card(self.actor(actor_side), slot, -1).clone();
                let next = neighbor_card(self.actor(actor_side), slot, 1).clone();
                if is_five_element_control(&previous, &next) {
                    self.activate_element_by_card(actor_side, &next);
                }
                Some(false)
            }
            4_000_100 => {
                // 极•六爻绝阵 Card_4000100.cs: Cast → ModifyBuffValue(
                // LiuYaoShaZhen, otherParams[0])。与卡 4000014 六爻绝阵同一 buff
                // （formations.six_yao_formation，mechanic_cards_extra.rs
                // 4_000_014 同款）；持续效果「每加 1 卦象 → 对方伤害」由共享
                // gain_hexagram hook 承担（combat_core_status.rs gain_hexagram，
                // 对应 BattleCharacter.cs:8761-8766）。4010100/4020100 数值由
                // 配置档位提供；再次行动由配置 actionAgain 统一判定。
                self.actor_mut(actor_side).formations.six_yao_formation +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            5 => {
                let bonus = self.known_negative_status_count(opponent_side(actor_side))
                    * other_param(card, 0).max(0);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            51 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                self.actor_mut(actor_side).astrology.ling_gua_art += 1;
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn yi_gua_self_resolution(&self, actor_side: PlayerSide) -> bool {
        if !self.actor(actor_side).identity.talents.contains(&197) {
            return false;
        }
        const QUALIFYING: [i64; 8] = [
            4_000_001, 4_000_002, 4_000_003, 4_000_015, 4_000_016, 4_000_025, 4_000_034, 4_000_026,
        ];
        let count = self
            .actor(actor_side)
            .turn
            .last_round_used_card_base_ids
            .iter()
            .filter(|base_id| QUALIFYING.contains(base_id))
            .count();
        count == 2
    }

    pub(super) fn reverse_queue(&mut self, actor_side: PlayerSide) {
        self.actor_mut(actor_side).deck.queue.reverse();
    }

    pub(super) fn clear_rear_move_check(&mut self, actor_side: PlayerSide) {
        self.actor_mut(actor_side).fate.used_rear_move_check = 0;
    }

    pub(super) fn apply_physique_173_after_card_hook(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        if card.physique.unwrap_or(0) > 0 && self.actor(actor_side).identity.talents.contains(&173)
        {
            self.apply_attack_with_options(
                actor_side,
                3,
                slot,
                false,
                false,
                0,
                Some("talent:173"),
            );
        }
    }

    pub(super) fn apply_yellow_bird_after_card_hook(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        if self.actor(actor_side).fate.used_rear_move_check <= 0 {
            return;
        }
        let damage = self.actor(actor_side).fate.yellow_bird_behind.max(0);
        if damage > 0 {
            self.apply_attack_with_options(
                actor_side,
                damage,
                slot,
                false,
                false,
                0,
                Some("buff:yellowBirdBehind"),
            );
        }
    }

    pub(super) fn apply_startled_touch_common_after_card_hook(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        let triggered_startled_touch = self.actor(actor_side).beng.triggered_startled_touch;
        if triggered_startled_touch > 0 {
            let attack = triggered_startled_touch + self.actor(actor_side).turn.lose_hp_count / 5;
            if attack > 0 {
                self.apply_attack_with_options(
                    actor_side,
                    attack,
                    slot,
                    false,
                    false,
                    0,
                    Some("buff:triggeredStartledTouch"),
                );
            }
        }
    }

    pub(super) fn apply_spirit_formation_echo_setup(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        if self.actor(actor_side).formations.spirit_formation_echo <= 0
            || !card.name.contains("灵阵")
        {
            return false;
        }
        self.actor_mut(actor_side).formations.spirit_formation_echo -= 1;
        // Original only replays the base-card effect and consumes the echo.
        // It does not add cannot_act here.
        self.actor_mut(actor_side)
            .formations
            .spirit_formation_echo_triggered = true;
        true
    }

    pub(super) fn clear_spirit_formation_echo_triggered(&mut self, actor_side: PlayerSide) {
        self.actor_mut(actor_side)
            .formations
            .spirit_formation_echo_triggered = false;
    }
}
