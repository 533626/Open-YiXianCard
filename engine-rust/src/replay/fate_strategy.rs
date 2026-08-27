use super::support::{
    card_rarity, has_base_card_in_deck, is_cloud_sword, is_fate_strategy_card, is_sword_card,
    opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_fate_strategy_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        if !is_fate_strategy_card(base_id) {
            return None;
        }
        if (7_000_095..=7_000_101).contains(&base_id) {
            return self.apply_fate_strategy_element_card_effect(
                actor_side,
                card,
                slot,
                was_used_before_effect,
                base_id,
            );
        }
        Some(self.apply_fate_strategy_main_card_effect(
            actor_side,
            card,
            slot,
            was_used_before_effect,
            base_id,
        ))
    }

    pub(super) fn apply_sword_energy_after_card_hook(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let sword_energy = self.actor(actor_side).sword.sword_energy;
        if sword_energy <= 0 {
            return;
        }
        let is_spirit_sword =
            super::support::is_spirit_sword_for_actor(self.actor(actor_side), card);
        let is_sword_energy_granted_card = has_base_card_in_deck(self.actor(actor_side), 1_000_075)
            && is_sword_card(self.actor(actor_side), card);
        let is_fate_379_cloud_sword = self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&379)
            && is_cloud_sword(self.actor(actor_side), card);
        if !(is_spirit_sword || is_sword_energy_granted_card || is_fate_379_cloud_sword) {
            return;
        }
        self.apply_damage(actor_side, sword_energy, false, false, false);
    }

    pub(super) fn settle_wan_shi_ru_yi_card_19(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if super::support::normalized_base_id(card) != 19
            || !self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&387)
        {
            return;
        }
        let anima = self.actor(actor_side).core.anima.max(0);
        if anima > 0 {
            self.spend_anima_unchecked(actor_side, anima);
            self.actor_mut(actor_side).sword.sword_energy += anima;
        }
        let sword_energy = self.actor(actor_side).sword.sword_energy.max(0);
        if sword_energy > 0 {
            self.apply_damage(actor_side, sword_energy, false, false, false);
        }
    }

    fn apply_fate_strategy_main_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> bool {
        let mut attacked = false;
        match base_id {
            1_000_088 => {
                self.gain_defense(actor_side, card.defense.unwrap_or(0).max(0));
                self.actor_mut(actor_side)
                    .turn
                    .spirit_control_anima_gain_defense += other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .turn
                    .spirit_control_anima_loss_defense += other_param(card, 1).max(0);
            }
            1_000_089 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.try_cost_anima(actor_side, 1) {
                    self.apply_attack(actor_side, other_param(card, 0).max(0), slot);
                    attacked = true;
                }
            }
            1_000_090 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).sword.sword_energy += other_param(card, 0).max(0);
                let agility_cap = other_param(card, 2).max(0);
                let agility_gain = ((self.actor(actor_side).core.anima
                    + self.actor(actor_side).sword.sword_energy)
                    * other_param(card, 1).max(0))
                .min(agility_cap);
                self.gain_agility(actor_side, agility_gain);
            }
            1_000_091 => {
                let has_cloud_chain = super::support::has_cloud_chain(self.actor(actor_side));
                if has_cloud_chain {
                    self.gain_active_effect_shatter_defense(1);
                }
                let cloud_chain_bonus = if has_cloud_chain {
                    other_param(card, 0).max(0)
                } else {
                    0
                };
                let explicit_double_bonus = self.actor(actor_side).sword.sword_intent
                    + self.actor(actor_side).core.attack_bonus;
                let attack = card.attack.unwrap_or(0) + cloud_chain_bonus + explicit_double_bonus;
                let attack_count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0);
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack_with_options(
                            actor_side,
                            attack,
                            slot,
                            false,
                            has_cloud_chain,
                            0,
                            None,
                        );
                        attacked = true;
                    }
                }
            }
            1_000_092 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                let anima_divisor = other_param(card, 0).max(1);
                let defense_divisor = other_param(card, 1).max(1);
                let anima_attack =
                    card.attack.unwrap_or(0) + self.actor(actor_side).core.anima / anima_divisor;
                if anima_attack > 0 {
                    self.apply_attack_with_options(
                        actor_side,
                        anima_attack,
                        slot,
                        false,
                        false,
                        0,
                        None,
                    );
                    attacked = true;
                }
                let defense_attack = card.attack.unwrap_or(0)
                    + self.actor(actor_side).core.defense / defense_divisor;
                if defense_attack > 0 {
                    self.apply_attack_with_options(
                        actor_side,
                        defense_attack,
                        slot,
                        false,
                        false,
                        0,
                        None,
                    );
                    attacked = true;
                }
            }
            1_000_094 => {
                let attack = card.attack.unwrap_or(0);
                if attack > 0 {
                    self.apply_attack_with_options(actor_side, attack, slot, true, false, 0, None);
                    attacked = true;
                }
                if !was_used_before_effect {
                    self.modify_sword_intent(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_095 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                let segments = self.actor(actor_side).turn.attack_segments_performed;
                self.gain_defense(actor_side, segments * other_param(card, 0).max(0));
            }
            1_000_098 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).sword.cloud_sea += other_param(card, 0).max(0);
                self.gain_cloud_chain(actor_side, other_param(card, 1).max(0).saturating_sub(1));
            }
            4_000_090 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.add_following_star_slots(actor_side, slot, other_param(card, 0));
            }
            4_000_091 => {
                let mut return_hexagram = 0;
                if self.actor(actor_side).astrology.hexagram > 0 {
                    return_hexagram += 1;
                }
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
                if self.actor(actor_side).astrology.hexagram > 0 {
                    return_hexagram += 1;
                }
                let hp_gain = self.consume_random_range(
                    actor_side,
                    other_param(card, 0),
                    other_param(card, 1).max(other_param(card, 0)),
                );
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                if return_hexagram > 0 {
                    self.gain_hexagram(actor_side, return_hexagram);
                }
            }
            4_000_092 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .formations
                    .soul_injury_curse_formation += other_param(card, 0).max(0);
            }
            4_000_093 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    let gain = other_param(card, 1).max(0)
                        + self.actor(actor_side).astrology.star_power * other_param(card, 2).max(0);
                    if gain > 0 {
                        self.modify_actor_max_hp(actor_side, gain);
                        self.modify_actor_hp(actor_side, gain, false, false);
                    }
                }
            }
            4_000_094 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                let target_side = opponent_side(actor_side);
                if self.actor(actor_side).core.anima > self.actor(target_side).core.anima {
                    self.add_actor_negative_status(target_side, 102, other_param(card, 0).max(0));
                }
            }
            4_000_095 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                let defense_gain = card.defense.unwrap_or(0).max(0)
                    + self.known_negative_status_count(target_side) * other_param(card, 1).max(0);
                let cap = other_param(card, 3);
                let capped = if cap > 0 {
                    defense_gain.min(cap)
                } else {
                    defense_gain
                };
                self.gain_defense(actor_side, capped);
            }
            4_000_096 => {
                self.actor_mut(actor_side)
                    .turn
                    .attack_applies_internal_injury_turns += other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.consume_percent_roll(actor_side) < 10 {
                    for _ in 0..2 {
                        self.apply_attack(actor_side, other_param(card, 1).max(0), slot);
                        attacked = true;
                    }
                }
                if self.consume_percent_roll(actor_side) < 10 {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        103,
                        other_param(card, 2).max(0),
                    );
                }
            }
            4_000_097 => {
                if !was_used_before_effect {
                    let gain = other_param(card, 0).max(0);
                    if gain > 0 {
                        self.modify_actor_max_hp(actor_side, gain);
                        self.modify_actor_hp(actor_side, gain, false, false);
                    }
                }
                if self.actor(actor_side).add_hp_count() > 0
                    && self.check_rear_move(actor_side, was_used_before_effect)
                {
                    let follow_up = card.attack.unwrap_or(0)
                        + self.actor(actor_side).add_hp_count() / other_param(card, 1).max(1);
                    if follow_up > 0 {
                        self.apply_attack(actor_side, follow_up, slot);
                        attacked = true;
                    }
                }
                if self.consume_optional_percent_roll_fail_closed(actor_side) < other_param(card, 2)
                {
                    self.modify_extra_actions(actor_side, 1);
                }
            }
            7_000_094 => {
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                self.actor_mut(actor_side).elements.seal_suppressing_mindset +=
                    other_param(card, 1).max(0);
            }
            10_000_090 => {
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.add_actor_negative_status(actor_side, 102, other_param(card, 1).max(0));
                if card_rarity(card) >= 2 || self.actor(actor_side).status.drunken_fist_stance == 0
                {
                    // 原版通过累加写入醉拳架势层数，证据：Card_10000090.cs:145-148。
                    // 失去破绽时再由 BattleCharacter.cs:8684-8691 按当前层数计算一次加攻。
                    self.actor_mut(actor_side).status.drunken_fist_stance +=
                        other_param(card, 2).max(0);
                }
            }
            10_000_091 => {
                self.apply_configured_anima(actor_side, card);
                self.add_actor_negative_status(actor_side, 100, other_param(card, 0).max(0));
                let gain = (self.actor(actor_side).core.anima
                    + self.known_negative_status_count(actor_side))
                    * other_param(card, 1).max(0);
                if gain > 0 {
                    self.modify_actor_max_hp(actor_side, gain);
                    self.modify_actor_hp(actor_side, gain, false, false);
                }
            }
            10_000_092 => {
                let attack = card.attack.unwrap_or(0)
                    + self.actor(actor_side).core.anima / 2;
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
                let spent = self.spend_anima_up_to(actor_side, other_param(card, 1).max(0));
                self.gain_agility(actor_side, spent * other_param(card, 2).max(0));
                let mut remaining = spent * other_param(card, 3).max(0);
                while remaining > 0 {
                    remaining -= 1;
                    let Some(status) = self.consume_optional_negative_status_decision() else {
                        continue;
                    };
                    self.modify_actor_negative_status(actor_side, status, -1);
                }
            }
            10_000_093 => {
                let attack = card.attack.unwrap_or(0);
                let attack_count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0);
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack_with_options(
                            actor_side, attack, slot, false, true, 0, None,
                        );
                        attacked = true;
                    }
                }
                self.actor_mut(actor_side)
                    .beng
                    .next_beng_quan_hp_cost_damage += 1;
            }
            10_000_094 => {
                let momentum_gain =
                    self.actor(actor_side).turn.hp_cost_cards_used * other_param(card, 0).max(0);
                self.modify_momentum(actor_side, momentum_gain);
                let attack = card.attack.unwrap_or(0) + self.actor(actor_side).beng.momentum;
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            10_000_095 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.gain_agility(actor_side, other_param(card, 1).max(0));
                self.actor_mut(actor_side).turn.blood_shadow += 1;
            }
            10_000_096 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                let physique_gain = card.physique.unwrap_or(0).max(0)
                    + self.actor(actor_side).core.anima / other_param(card, 0).max(1);
                self.apply_physique_amount(actor_side, physique_gain);
            }
            10_000_097 => {
                let spent = self.spend_anima_up_to(actor_side, other_param(card, 0).max(0));
                let attack = card.attack.unwrap_or(0) + spent * other_param(card, 1).max(0);
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
                let defense_gain =
                    card.defense.unwrap_or(0).max(0) + spent * other_param(card, 2).max(0);
                self.gain_defense(actor_side, defense_gain);
            }
            _ => {}
        }
        attacked
    }
}
