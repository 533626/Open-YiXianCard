use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_dream_direct_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        let realm = super::original_config::original_card_realm_level(card.id).unwrap_or(0);
        match base_id {
            1_000_081 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                if realm >= 4 && self.actor(actor_side).core.anima > card.anima.unwrap_or(0).max(0)
                {
                    self.gain_agility(actor_side, 10);
                }
                Some(attacked)
            }
            1_000_082 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let actual_damage = self.active_effect_actual_damage().max(0);
                let wounded_count = self.active_effect_wounded_count().max(0);
                let divisor = other_param(card, 1);
                if wounded_count > 0 && actual_damage > 0 && divisor > 0 {
                    let mut internal_injury = actual_damage / divisor;
                    if realm <= 4 {
                        internal_injury = internal_injury.min(other_param(card, 0).max(0));
                    }
                    if internal_injury > 0 {
                        self.add_actor_negative_status(target_side, 100, internal_injury);
                    }
                }
                Some(attacked)
            }
            4_000_074 => {
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                if realm >= 4 {
                    self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                }
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    if realm <= 3 {
                        self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                    } else {
                        let divisor = other_param(card, 2);
                        if divisor > 0 {
                            let internal_injury = self.actor(actor_side).add_hp_count() / divisor;
                            if internal_injury > 0 {
                                self.add_actor_negative_status(target_side, 100, internal_injury);
                            }
                        }
                    }
                }
                Some(false)
            }
            7_000_085 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                // 原版 Card_7000085.cs:63-71 读持有者持久 ActualDamage(302)
                //（残留 + 本卡）→ 水势 +302/otherParams[0]。
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry.max(0);
                if actual_damage > 0 {
                    let parameter = other_param(card, 0);
                    let water_momentum = if realm <= 3 {
                        parameter.max(0)
                    } else if parameter > 0 {
                        actual_damage / parameter
                    } else {
                        0
                    };
                    self.gain_water_momentum(actor_side, water_momentum);
                }
                Some(attacked)
            }
            7_000_103 => {
                let bonus = self.actor(actor_side).core.anima.max(0)
                    + self.actor(actor_side).elements.water_momentum.max(0);
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // 原版 Card_7000103.cs:84-92 读持有者持久 ActualDamage(302)
                //（残留 + 本卡）→ 生命及上限 +302/otherParams[0]。
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry.max(0);
                let divisor = other_param(card, 0);
                if realm >= 4 && actual_damage > 0 && divisor > 0 {
                    let hp_gain = actual_damage / divisor;
                    if hp_gain > 0 {
                        self.modify_actor_max_hp(actor_side, hp_gain);
                        self.modify_actor_hp(actor_side, hp_gain, false, false);
                    }
                }
                Some(attacked)
            }
            10_000_070 => {
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                let internal_injury = if realm <= 3 {
                    other_param(card, 1).max(0)
                } else {
                    let divisor = other_param(card, 1);
                    if divisor > 0 {
                        self.actor(actor_side).core.physique.max(0) / divisor
                    } else {
                        0
                    }
                };
                if internal_injury > 0 {
                    self.add_actor_negative_status(actor_side, 100, internal_injury);
                    self.add_actor_negative_status(target_side, 100, internal_injury);
                }
                Some(false)
            }
            10_000_073 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                let agility = other_param(card, 1).max(0)
                    + if realm >= 4 {
                        self.actor(actor_side).core.anima.max(0)
                    } else {
                        0
                    };
                self.gain_agility(actor_side, agility);
                Some(false)
            }
            10_000_074 => {
                let agility = self.actor(actor_side).turn.agility.max(0);
                let momentum = self.actor(actor_side).beng.momentum.max(0);
                let extra_attack = if realm <= 4 {
                    if agility > 0 || momentum > 0 {
                        other_param(card, 0).max(0)
                    } else {
                        0
                    }
                } else if agility > 0 {
                    let divisor = other_param(card, 0);
                    if divisor > 0 {
                        agility / divisor
                    } else {
                        0
                    }
                } else {
                    0
                };
                let attacked = self.attack_by_config(actor_side, card, extra_attack, slot);
                self.actor_mut(actor_side).beng.beng_quan_chuo += extra_attack;
                Some(attacked)
            }
            10_000_079 => {
                let attack_divisor = other_param(card, 1);
                let attack_bonus = if attack_divisor > 0 {
                    self.actor(actor_side)
                        .turn
                        .battle_physique_gain_count
                        .max(0)
                        / attack_divisor
                } else {
                    0
                };
                if realm >= 4 {
                    let momentum_divisor = other_param(card, 0);
                    if momentum_divisor > 0 {
                        let momentum =
                            self.actor(actor_side).core.physique.max(0) / momentum_divisor;
                        self.modify_momentum(actor_side, momentum);
                    }
                }
                Some(self.attack_by_config(actor_side, card, attack_bonus, slot))
            }
            10_000_086 => {
                let captured_momentum = self.actor(actor_side).beng.momentum.max(0);
                let captured_anima = self.actor(actor_side).core.anima.max(0);
                if realm <= 3 {
                    if captured_momentum > 0 || captured_anima > 0 {
                        self.apply_physique_amount(actor_side, other_param(card, 0).max(0));
                        self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                    }
                } else {
                    self.gain_agility(actor_side, other_param(card, 3).max(0));
                    if captured_momentum > 0 {
                        self.modify_momentum(actor_side, -captured_momentum);
                        self.apply_physique_amount(
                            actor_side,
                            captured_momentum * other_param(card, 1).max(0),
                        );
                        self.modify_actor_hp(
                            actor_side,
                            captured_momentum * other_param(card, 2).max(0),
                            false,
                            false,
                        );
                    }
                }
                Some(false)
            }
            _ => None,
        }
    }
}
