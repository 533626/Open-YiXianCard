use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// Card bodies verified against synthetic, client-executable original-game oracle cases.
    /// This evidence is intentionally distinct from server-recorded replay fixtures.
    pub(super) fn apply_synthetic_oracle_verified_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            47 => {
                // 云剑·晚霞：每次牌体执行均先加牌面剑意，再按结算前连云追加。
                self.modify_sword_intent(actor_side, card.sword_intent.unwrap_or(0).max(0));
                let cloud_chain = self.actor(actor_side).sword.cloud_chain.max(0);
                self.modify_sword_intent(actor_side, cloud_chain * other_param(card, 0).max(0));
                Some(false)
            }
            48 => {
                // 青龙剑阵：先加牌面防，再按结算后的总防获得剑意。
                self.apply_configured_defense(actor_side, card);
                let sword_intent =
                    self.actor(actor_side).core.defense / other_param(card, 0).max(1);
                self.modify_sword_intent(actor_side, sword_intent.max(0));
                Some(false)
            }
            336 => {
                // 玄灵附身：新施加的减攻也计入随后读取的负面状态总层数。
                self.apply_configured_anima(actor_side, card);
                let attack_modifier = other_param(card, 0).max(0);
                self.gain_attack_bonus(actor_side, attack_modifier);
                self.add_actor_negative_status(actor_side, 103, attack_modifier);
                let agility = self.negative_status_stack_count(actor_side);
                self.gain_agility(actor_side, agility);
                Some(false)
            }
            1_000_097 => {
                // 极•暗鸦灵剑：攻击后加灵气，防御读取加灵气后的总量。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.gain_defense(actor_side, anima * other_param(card, 0).max(0));
                }
                Some(attacked)
            }
            4_000_058 => {
                // 天命劫：仅恰好 8 层卦象时直接设置对方生命，绕过护体。
                if self.actor(actor_side).astrology.hexagram == 8 {
                    let target_side = opponent_side(actor_side);
                    let hp = other_param(card, 0).min(self.actor(target_side).core.max_hp);
                    self.actor_mut(target_side).core.hp = hp;
                }
                Some(false)
            }
            4_000_083 => {
                // 梦•气疗术：元婴以上按加灵气后的总灵气与当前星力回血。
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0));
                let scales_with_resources =
                    super::original_config::original_card_realm_level(card.id)
                        .is_some_and(|level| level > 3)
                        || card.other_params.len() >= 3;
                let healing = if scales_with_resources {
                    other_param(card, 1).max(0) * self.actor(actor_side).core.anima.max(0)
                        + other_param(card, 2).max(0)
                            * self.actor(actor_side).astrology.star_power.max(0)
                } else {
                    other_param(card, 1).max(0)
                };
                self.modify_actor_hp(actor_side, healing, false, false);
                Some(false)
            }
            4_000_099 => {
                // 极•两仪阵：先加灵气与卦象，再按结算后的总卦象加防、回血。
                self.apply_configured_anima(actor_side, card);
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0).max(0));
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                if hexagram > 0 {
                    self.gain_defense(actor_side, hexagram * other_param(card, 0).max(0));
                    self.modify_actor_hp(
                        actor_side,
                        hexagram * other_param(card, 1).max(0),
                        false,
                        false,
                    );
                }
                Some(false)
            }
            10_000_062 => {
                // 破茧化蝶：移除的负面状态层数，而非种类数，决定体魄与回血。
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                let removed_layers = self.negative_status_stack_count(actor_side);
                self.reduce_all_actor_negative_statuses(actor_side, i64::MAX);
                if removed_layers > 0 {
                    let gain = removed_layers * other_param(card, 1).max(0);
                    self.apply_physique_amount(actor_side, gain);
                    self.modify_actor_hp(actor_side, gain, false, false);
                }
                Some(false)
            }
            _ => None,
        }
    }
}
