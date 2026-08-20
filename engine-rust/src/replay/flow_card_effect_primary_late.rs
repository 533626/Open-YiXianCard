use super::super::ReplayState;
use crate::replay::original_config::{
    original_card_definition, original_card_echo_upgrade_id, original_card_realm_level,
};
use crate::replay::support::{has_cloud_chain, normalized_base_id, opponent_side, other_param};

use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_card_effect_primary_late(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            2_000_003 | 2_000_010 => {
                self.apply_elixir_base(actor_side, card);
                Some(false)
            }
            2_000_011 => {
                self.apply_elixir_base(actor_side, card);
                let gain = other_param(card, 0).max(0);
                if gain > 0 {
                    self.modify_actor_max_hp(actor_side, gain);
                    self.modify_actor_hp(actor_side, gain, false, false);
                }
                Some(false)
            }
            2_000_013 => {
                self.apply_elixir_base(actor_side, card);
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            99_000_106 => {
                self.apply_configured_defense(actor_side, card);
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            28 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                } else {
                    let hexagram = other_param(card, 0).max(0);
                    if hexagram > 0 {
                        self.gain_hexagram(actor_side, hexagram);
                    }
                }
                self.actor_mut(actor_side).astrology.infinite_hexagram_plate += 1;
                Some(false)
            }
            29 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    102,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            37 => {
                let sword_intent = self.actor(actor_side).sword.sword_intent.max(0);
                let defense =
                    card.defense.unwrap_or(0).max(0) + sword_intent * other_param(card, 0).max(0);
                self.gain_defense(actor_side, defense);
                Some(false)
            }
            4_000_056 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0).max(0),
                );
                let debuff_count = self.known_negative_status_count(opponent_side(actor_side));
                let mut attacked = false;
                for _ in 0..debuff_count.max(0) {
                    self.apply_attack(actor_side, 1, slot);
                    attacked = true;
                }
                Some(attacked)
            }
            4_000_060 => {
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                }
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                }
                Some(false)
            }
            6_000_006 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_defense(opponent_side(actor_side), other_param(card, 0).max(0));
                Some(attacked)
            }
            8_000_012 => {
                let first_card = self.actor(actor_side).deck.slots.first()?.card.clone();
                let is_sustain = first_card
                    .card_type
                    .as_ref()
                    .is_some_and(|card_type| card_type.value == 3);
                if !is_sustain {
                    return Some(false);
                }
                let upgrade_times = other_param(card, 0).max(0);
                // Card_8000012.cs walks the first battle-deck card id +10000 per
                // level for otherParams[0] steps (noUpgrade-gated), then executes
                // that id as a temporary card. See engine-ts echoPattern /
                // upgradeEchoedSustainCardId (commit f508dba0).
                let upgraded_id = original_card_echo_upgrade_id(first_card.id, upgrade_times);
                let echoed = original_card_definition(upgraded_id).unwrap_or(first_card);
                if self.apply_temporary_card_effect(actor_side, &echoed, slot) {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(false)
            }
            10_000_019 => {
                self.actor_mut(actor_side).turn.current_turn_ignore_defense += 1;
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.add_actor_negative_status(actor_side, 101, other_param(card, 0).max(0));
                Some(attacked)
            }
            10_000_020 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).beng.momentum_before_attack +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            10_000_030 => {
                let internal = other_param(card, 0).max(0);
                let external = other_param(card, 1).max(0);
                for side in [actor_side, opponent_side(actor_side)] {
                    self.add_actor_negative_status(side, 100, internal);
                    self.add_actor_negative_status(side, 105, external);
                }
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                Some(attacked)
            }
            10_000_033 => {
                let cost = other_param(card, 0).max(0);
                let spent = self.spend_anima_up_to(actor_side, cost);
                let momentum_gain = spent * other_param(card, 1).max(0);
                let attack_bonus = spent * other_param(card, 2).max(0);
                self.modify_momentum(actor_side, momentum_gain);
                let attacked = self.attack_by_config(actor_side, card, attack_bonus, slot);
                Some(attacked)
            }
            10_000_034 => {
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                self.gain_agility(actor_side, other_param(card, 1).max(0));
                Some(false)
            }
            10_000_041 => {
                let amount = other_param(card, 0).max(0);
                self.add_actor_negative_status(actor_side, 100, amount);
                self.add_actor_negative_status(opponent_side(actor_side), 100, amount);
                let gain =
                    self.known_negative_status_count(actor_side) / other_param(card, 1).max(1);
                if gain > 0 {
                    self.gain_anima(actor_side, gain);
                    self.modify_actor_hp(actor_side, gain, false, false);
                }
                Some(false)
            }
            10_000_052 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                let agility_gain = other_param(card, 1).max(0)
                    + self.actor(actor_side).beng.momentum.max(0) * other_param(card, 2).max(0);
                self.gain_agility(actor_side, agility_gain);
                Some(attacked)
            }
            10_000_089 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                // Card_10000089 only calls GetNextParam above JinDan. Lower
                // realms must leave that replay decision for the next effect.
                let reads_hand_count = original_card_realm_level(card.id).unwrap_or(0) > 3;
                let queued_extra = if reads_hand_count {
                    self.consume_optional_decision()
                } else {
                    -1
                };
                let fallback_extra = if queued_extra >= 0 {
                    queued_extra
                } else if reads_hand_count {
                    let cap = other_param(card, 1).max(0);
                    if cap > 0 {
                        self.actor(actor_side)
                            .deck
                            .slots
                            .iter()
                            .filter(|slot| {
                                normalized_base_id(&slot.card) >= 10_000_001
                                    && normalized_base_id(&slot.card) <= 10_000_047
                            })
                            .count()
                            .min(cap as usize) as i64
                    } else {
                        0
                    }
                } else {
                    0
                };
                self.actor_mut(actor_side).beng.dream_beng_quan_chain +=
                    other_param(card, 0).max(0) + fallback_extra;
                Some(attacked)
            }
            11_000_013 => {
                self.actor_mut(actor_side).astrology.all_goes_well += other_param(card, 1).max(0);
                Some(false)
            }
            11_000_018 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            11_000_019 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).turn.adaptation += other_param(card, 1).max(0);
                Some(false)
            }
            11_000_024 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            11_000_026 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 2).max(0), false, false);
                self.trigger_nearby_openings(
                    actor_side,
                    slot,
                    -1,
                    other_param(card, 0).max(0),
                    other_param(card, 1).max(1),
                );
                Some(false)
            }
            11_000_020 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).fate.heavenly_secret_reverse +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            11_000_025 => {
                self.apply_configured_anima(actor_side, card);
                self.trigger_nearby_openings(
                    actor_side,
                    slot,
                    1,
                    other_param(card, 0).max(0),
                    other_param(card, 1).max(1),
                );
                Some(false)
            }
            15 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
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
                let convert_limit = other_param(card, 1).max(0);
                if has_cloud_chain(self.actor(actor_side))
                    && !self.actor(actor_side).identity.talents.contains(&222)
                {
                    let converted =
                        convert_limit.min(self.actor(actor_side).status.recovery.max(0));
                    if converted > 0 {
                        let recovery_before = self.actor(actor_side).status.recovery;
                        self.actor_mut(actor_side).status.recovery =
                            (self.actor(actor_side).status.recovery - converted).max(0);
                        self.record_counter_transition(
                            actor_side,
                            "状态",
                            "recovery",
                            "恢复",
                            recovery_before,
                            self.actor(actor_side).status.recovery,
                        );
                        self.gain_attack_bonus(actor_side, converted);
                    }
                }
                Some(attacked)
            }
            55 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).fate.wild_ferry_seal += 1;
                Some(false)
            }
            16 => {
                self.apply_configured_defense(actor_side, card);
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            218 => {
                let hp_gain = other_param(card, 0).max(0)
                    + self.actor(actor_side).elements.activated_wood.max(0) * 2;
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                let guard_gain = other_param(card, 1).max(0);
                self.gain_guard(actor_side, guard_gain);
                self.gain_guard(opponent_side(actor_side), guard_gain);
                let clear = other_param(card, 2).max(0);
                self.reduce_all_actor_negative_statuses(actor_side, clear);
                self.reduce_all_actor_negative_statuses(opponent_side(actor_side), clear);
                self.actor_mut(actor_side).status.cannot_act += 1;
                self.actor_mut(opponent_side(actor_side)).status.cannot_act += 1;
                Some(false)
            }
            _ => None,
        }
    }

    fn trigger_nearby_openings(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
        direction: i64,
        count: i64,
        repeat: i64,
    ) {
        let mut triggered = 0_i64;
        let mut index = slot as i64 + direction;
        while index >= 0
            && (index as usize) < self.actor(actor_side).deck.slots.len()
            && triggered < count
        {
            let card = self.actor(actor_side).deck.slots[index as usize]
                .card
                .clone();
            let base_id = normalized_base_id(&card);
            if !ReplayState::card_has_opening_effect(base_id) {
                index += direction;
                continue;
            }
            for _ in 0..repeat.max(1) {
                // 天星•牵引/天星•反击原版传自身格位作 triggerGrid
                // （Card_11000025.cs:166、Card_11000026.cs:164），决定
                // 厄劫缠身/梦•厄劫缠身等「同格」开局的目标格。
                self.apply_opening_effect_for_card_with_trigger_grid(
                    actor_side,
                    &card,
                    index as usize,
                    slot,
                );
            }
            triggered += 1;
            index += direction;
        }
    }
}
