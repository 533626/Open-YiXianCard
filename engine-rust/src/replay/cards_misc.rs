use super::super::flow_card_effect::{param0, param1, param2};
use super::super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::support::{card_rarity, div_ceil, opponent_side, other_param};

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn apply_card_effect_misc(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            20 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.activate_element(actor_side, Element::Wood);
                Some(attacked)
            }
            3_000_010 => {
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                let defense_gain = (anima * param0(card)).min(param1(card));
                if defense_gain > 0 {
                    self.gain_defense(actor_side, defense_gain);
                }
                Some(false)
            }
            6_000_012 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).turn.next_card_action_again += 1;
                Some(false)
            }
            3_000_013 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            3_000_015 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 100, param0(card));
                self.add_actor_negative_status(target_side, 102, param1(card));
                self.add_actor_negative_status(target_side, 101, param2(card));
                Some(false)
            }
            4_000_026 => {
                self.apply_configured_anima(actor_side, card);
                let hexagram_gain = card
                    .hexagram
                    .filter(|value| *value > 0)
                    .unwrap_or(card_rarity(card) + 1);
                self.gain_hexagram(actor_side, hexagram_gain);
                Some(false)
            }
            4_000_044 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.modify_star_chess_break(opponent_side(actor_side), param0(card));
                }
                Some(attacked)
            }
            4_000_059 => {
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 100, param0(card));
                let drain = self.actor(target_side).status.internal_injury.max(0);
                if drain > 0 {
                    self.modify_actor_hp(target_side, -drain, false, false);
                }
                Some(false)
            }
            99_000_108 => {
                let threshold = param0(card);
                let attack = card.attack.unwrap_or(0)
                    * if self.actor(actor_side).core.hp < threshold {
                        2
                    } else {
                        1
                    };
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    Some(true)
                } else {
                    Some(false)
                }
            }
            216 => {
                self.actor_mut(actor_side).status.back_solitude += 1;
                Some(false)
            }
            217 => {
                self.actor_mut(actor_side).status.strike_void += 1;
                Some(false)
            }
            215 => {
                self.apply_configured_anima(actor_side, card);
                match card_rarity(card) {
                    0 => {
                        self.modify_actor_hp(actor_side, param0(card), false, false);
                    }
                    1 => {
                        self.apply_configured_defense(actor_side, card);
                    }
                    2 => {
                        let anima = self.actor(actor_side).core.anima.max(0);
                        self.add_actor_negative_status(opponent_side(actor_side), 100, anima);
                        let current_anima = self.actor(actor_side).core.anima.max(0);
                        self.spend_anima_unchecked(actor_side, div_ceil(current_anima, 2));
                    }
                    _ => {}
                }
                Some(false)
            }
            // 活动甜粽牌 Card_94 鲜果宝粽：ModifyHp(p0) 后
            // ModifyBuffValue(XianGuoBaoZong=399, p1)（Card_94.cs:80-81）。
            // Buff 399 分类 Hidden，加层无钩子，直接计数。
            94 => {
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                self.actor_mut(actor_side).fate.xian_guo_bao_zong += param1(card);
                Some(false)
            }
            // 活动咸粽牌 Card_100 鲜肉粽：ModifyHp(p0) 后取一个战斗参数，
            // 非 -1 则该 BuffType -p1 层（Card_100.cs:99-101）。参数域与
            // 仙魔 13002 同口径（100..=105/367/393），域外 fail closed。
            100 => {
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                let selected = self.consume_card_param(actor_side);
                if selected != -1 {
                    if !matches!(selected, 100..=105 | 367 | 393) {
                        self.missing_card_effect(
                            card.id,
                            100,
                            "card 100 selected an unmodelled buff type",
                        );
                        return Some(false);
                    }
                    let amount = param1(card);
                    if amount > 0 {
                        self.modify_actor_negative_status(actor_side, selected, -amount);
                    }
                }
                Some(false)
            }
            // 活动咸粽牌 Card_102 麻辣粽：内伤+p0（走完整负面管线，206/
            // 鲜果宝粽/辟邪照常生效），两次独立 ModifyHp(p1)，再
            // TryCostAnima(1)>0 时 ModifyHp(p2)（Card_102.cs:142-168）。
            // 两次回血必须分开调用：每次各自触发治疗钩子。
            102 => {
                self.add_actor_negative_status(actor_side, 100, param0(card));
                self.modify_actor_hp(actor_side, other_param(card, 1), false, false);
                self.modify_actor_hp(actor_side, other_param(card, 1), false, false);
                let paid = self.try_cost_anima(actor_side, 1)
                    || (self.actor(actor_side).identity.talent_resonance_id == Some(50)
                        && self.try_pay_ling_yong_bu_jue_anima_shortage(actor_side, 1));
                if paid {
                    self.modify_actor_hp(actor_side, other_param(card, 2), false, false);
                }
                Some(false)
            }
            // 活动咸粽牌 Card_103 腊肉粽：印面灵气+anima（Fallback 字段），
            // ModifyHp(p0)，ModifyBuffValue(BiXie=13, p1)（Card_103.cs:111/137）。
            103 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                self.actor_mut(actor_side).fate.exorcism += param1(card);
                Some(false)
            }
            _ => None,
        }
    }
}
