use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_mechanic_card_effect_extra(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        let mut attacked = false;
        match base_id {
            4_000_037 => {
                let hp_gain = other_param(card, 1).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.actor_mut(actor_side).fate.reflect_mindset += other_param(card, 0).max(0);
            }
            4_000_052 => {
                self.actor_mut(actor_side).fate.graft_flowers_to_tree +=
                    other_param(card, 0).max(0);
            }
            9_000_011 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 104, other_param(card, 0).max(0));
                self.actor_mut(actor_side).music.immortal_binding_vine +=
                    other_param(card, 1).max(0);
            }
            9_000_019 => {
                let target_side = opponent_side(actor_side);
                let hp_drain = other_param(card, 0).max(0);
                if hp_drain > 0 {
                    self.modify_actor_hp(target_side, -hp_drain, false, false);
                    self.modify_actor_hp(actor_side, hp_drain, false, false);
                }
                self.actor_mut(actor_side).music.devouring_ancient_vine +=
                    other_param(card, 1).max(0);
            }
            1_000_066 => {
                self.actor_mut(actor_side).sword.frenzy_sword_zero += other_param(card, 0).max(0);
            }
            5_000_009 => {
                self.actor_mut(actor_side).turn.current_turn_ignore_defense += 1;
                let bonus = if self.actor(actor_side).music.music_cards_played > 0 {
                    1
                } else {
                    0
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
                self.actor_mut(actor_side).music.music_cards_played += 1;
            }
            4_000_014 => {
                let reduction = if self.actor(actor_side).formations.six_yao_formation > 0 {
                    other_param(card, 1).max(0)
                } else {
                    0
                };
                self.actor_mut(actor_side).formations.six_yao_formation +=
                    other_param(card, 0).max(0) - reduction;
            }
            4_000_042 => {
                let divisor = other_param(card, 0).max(1);
                let bonus = self.actor(actor_side).add_hp_count() / divisor;
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            6_000_013 => {
                self.modify_paint_finishing_touch(actor_side, other_param(card, 0).max(0));
            }
            10_000_051 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_momentum_limit(actor_side, other_param(card, 0).max(0));
                self.modify_momentum(actor_side, other_param(card, 1).max(0));
            }
            220 => {
                if self.actor(actor_side).beng.quan_stance > 0 {
                    self.modify_momentum(actor_side, other_param(card, 0).max(0));
                    let divisor = other_param(card, 1).max(1);
                    let heal = self.actor(actor_side).core.physique / divisor;
                    if heal > 0 {
                        self.modify_actor_hp(actor_side, heal, false, false);
                    }
                } else {
                    let divisor = other_param(card, 2).max(1);
                    let bonus = self.actor(actor_side).core.physique / divisor;
                    attacked |= self.attack_by_config(actor_side, card, bonus, slot);
                }
                if self.has_locked_li_stance(actor_side) {
                    // 335/349 锁定架势：不切换，只结算命运策略效果。
                } else if self.actor(actor_side).beng.quan_stance > 0 {
                    self.actor_mut(actor_side).beng.quan_stance -= 1;
                    self.actor_mut(actor_side).beng.gun_stance += 1;
                } else {
                    self.actor_mut(actor_side).beng.gun_stance =
                        (self.actor(actor_side).beng.gun_stance - 1).max(0);
                    self.actor_mut(actor_side).beng.quan_stance += 1;
                }
                // 429 强攻架势按切换后的最终架势发奖（拳→+1 气势，棍→+1 加攻）。
                self.apply_fate_strategy_stance_switch(actor_side);
            }
            _ => return None,
        }
        Some(attacked)
    }
}
