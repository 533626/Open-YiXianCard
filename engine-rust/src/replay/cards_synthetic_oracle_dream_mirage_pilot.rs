use super::support::{has_cloud_chain, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// Audited Dream/Mirage pilot bodies: all seven cards are admitted.
    pub(super) fn apply_synthetic_oracle_dream_mirage_pilot_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            1_000_071 => {
                // 梦•飞牙剑：先登记本牌待消耗剑意，统一结算消费/保留后再返还。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let pending = self.active_effect_pending_sword_intent().max(0);
                if pending > 0 {
                    self.add_active_effect_deferred_sword_intent_restore(pending);
                }
                Some(attacked)
            }
            4_000_071 => {
                // 梦•轰雷掣电：整张牌冻结出牌前卦象，每段使用同一份加攻。
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                Some(self.attack_by_config(actor_side, card, hexagram, slot))
            }
            4_000_086 => {
                // 梦•乾卦：专用牌体负责卦象；通用 printed-field 阶段已排除本牌以防重复。
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0).max(0));
                Some(false)
            }
            7_000_088 => {
                // 梦·木灵芽：低阶只判断是否加过生命，高阶按累计加生命量整除。
                let add_hp_count = self.actor(actor_side).add_hp_count();
                let low_realm = super::original_config::original_card_realm_level(card.id)
                    .map_or(card.other_params.is_empty(), |realm| realm <= 2);
                let bonus = if add_hp_count <= 0 {
                    0
                } else if low_realm {
                    1
                } else {
                    let divisor = other_param(card, 0);
                    if divisor > 0 {
                        add_hp_count / divisor
                    } else {
                        0
                    }
                };
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            288 => {
                // 幻•云剑无锋：未连云时整张牌不攻击。
                // BattleCharacter.cs:8306-8320: talent 14、resonance 10 + temp flag 10、
                // LongMaJingShen 均让 HasBuff(LianYun) 返回 true。
                if !has_cloud_chain(self.actor(actor_side)) {
                    return Some(false);
                }
                let prior_segments = self.actor(actor_side).turn.attack_segments_performed.max(0);
                let bonus = other_param(card, 0).max(0) * prior_segments;
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            309 => {
                // 幻•云舞诀：只读取此前完成的牌数；本牌在通用 completed 阶段才入账。
                let used_before = self.actor(actor_side).turn.used_card_count.max(0);
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.modify_sword_intent(
                    actor_side,
                    card.sword_intent.unwrap_or(0).max(0) + used_before,
                );
                Some(false)
            }
            325 => {
                // 幻•灵犀剑阵：先加牌面防，再以结算后的总防换取灵气。
                self.apply_configured_defense(actor_side, card);
                let divisor = other_param(card, 0);
                if divisor > 0 {
                    let anima = self.actor(actor_side).core.defense.max(0) / divisor;
                    if anima > 0 {
                        self.gain_anima(actor_side, anima);
                    }
                }
                Some(false)
            }
            _ => None,
        }
    }
}
