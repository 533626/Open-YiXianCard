use super::support::{div_ceil, opponent_side, other_param, other_param_or};
use super::{Element, ReplayPlayer, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// Card bodies backed by the batch-006 synthetic original-client oracle.
    ///
    /// Persistent hooks live beside these bodies as small helpers below so the
    /// shared combat/resource paths can call the same rule without duplicating
    /// its boundary conditions.
    pub(super) fn apply_synthetic_oracle_verified_secret_misc_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            4_000_048 => {
                // 生机绽放：牌面加上限、回血均先于持续标记安装。
                let amount = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                self.actor_mut(actor_side).fate.vitality_bloom = 1;
                Some(false)
            }
            4_000_049 => {
                // 先发制人：本次攻击不能享受随后才安装的首次后招通行证。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).fate.first_strike = 1;
                Some(attacked)
            }
            4_000_057 => {
                // 铁树开花：原作百分骰仅在结果为 0 时令两段收益同时翻倍。
                let multiplier = if self.consume_percent_roll(actor_side) < 1 {
                    2
                } else {
                    1
                };
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0) * multiplier);
                self.modify_actor_hp(
                    actor_side,
                    other_param(card, 1).max(0) * multiplier,
                    false,
                    false,
                );
                Some(false)
            }
            4_000_065 => {
                // 流星天陨先清空全部星力，再按原星力逐次造成伤害。
                let star_power = self.actor(actor_side).astrology.star_power.max(0);
                self.remove_star_power_from_original_card_4000065(actor_side);
                for _ in 0..star_power {
                    self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                }
                Some(false)
            }
            4_000_064 => {
                // 雷霆心法：先结算卦象，再叠加仅作用于牌体阶段雷牌攻击的百分比。
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0).max(0));
                self.actor_mut(actor_side).astrology.thunder_mindset += other_param(card, 0).max(0);
                Some(false)
            }
            7_000_045 => {
                // 火灵·焚天焱：累计失去上限先整除，再把结果加到每一段攻击。
                let step = other_param_or(card, 0, 1);
                let bonus = if self.check_wu_xing(actor_side, Element::Fire) {
                    self.actor(opponent_side(actor_side))
                        .core
                        .lost_max_hp_count
                        .max(0)
                        / step
                } else {
                    0
                };
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            7_000_049 => {
                // 火灵·窑土：火分支先减生命及上限，土分支读取更新后的累计值。
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let amount = other_param(card, 0).max(0);
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                if self.check_wu_xing(actor_side, Element::Earth) {
                    let lost_max_hp = self
                        .actor(opponent_side(actor_side))
                        .core
                        .lost_max_hp_count
                        .max(0);
                    self.gain_defense(actor_side, other_param(card, 1).max(0) + lost_max_hp / 2);
                }
                Some(false)
            }
            7_000_051 => {
                // 木灵·燃火：木分支先回血，火分支再向上取整扣除当前生命的一半。
                if self.check_wu_xing(actor_side, Element::Wood) {
                    self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                }
                if !self.check_wu_xing(actor_side, Element::Fire) {
                    return Some(false);
                }
                let hp_spent = div_ceil(self.actor(actor_side).core.hp.max(0), 2);
                self.modify_actor_hp(actor_side, -hp_spent, false, false);
                let attack = other_param(card, 1).max(0) + hp_spent;
                self.apply_attack(actor_side, attack, slot);
                Some(attack > 0)
            }
            7_000_064 => {
                // 木灵·藤蔓：Card_7000064 reads battle-lifetime AddHpCount,
                // divides it, caps the added segment count, then attacks.
                let extra = if self.check_wu_xing(actor_side, Element::Wood) {
                    (self.actor(actor_side).add_hp_count() / other_param(card, 0).max(1))
                        .min(other_param(card, 1).max(0))
                } else {
                    0
                };
                let attack = card.attack.unwrap_or(0).max(0);
                let attack_count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0)
                    + extra;
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                Some(attack > 0 && attack_count > 0)
            }
            10_000_053 => {
                // 气势不绝：空池先按本张牌面回填，持续值随后照常叠加。
                let amount = other_param(card, 0).max(0);
                if self.actor(actor_side).beng.momentum == 0 {
                    self.modify_momentum(actor_side, amount);
                }
                self.actor_mut(actor_side).beng.unceasing_momentum += amount;
                Some(false)
            }
            _ => None,
        }
    }
}

pub(super) fn vitality_bloom_overflow_max_hp_gain(player: &ReplayPlayer, healing: i64) -> i64 {
    if healing <= 0 || player.fate.vitality_bloom <= 0 {
        return 0;
    }
    (player.core.hp + healing - player.core.max_hp).max(0)
}

pub(super) fn first_strike_enables_rear_move(player: &ReplayPlayer) -> bool {
    player.fate.first_strike > 0
}

pub(super) fn thunder_mindset_attack_bonus_percent(
    player: &ReplayPlayer,
    current_card_name: &str,
    after_card_action: bool,
) -> i64 {
    if after_card_action || !current_card_name.contains('雷') {
        return 0;
    }
    player.astrology.thunder_mindset.max(0)
}

pub(super) fn lost_max_hp_layers_from_delta(actual_max_hp_delta: i64) -> i64 {
    (-actual_max_hp_delta).max(0)
}

pub(super) fn unceasing_momentum_refill(player: &ReplayPlayer) -> i64 {
    if player.beng.momentum != 0 {
        return 0;
    }
    player.beng.unceasing_momentum.max(0)
}
