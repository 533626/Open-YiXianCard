use super::support::{div_ceil, has_cloud_chain, opponent_side, other_param};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};
use std::collections::BTreeSet;

impl ReplayState {
    pub(super) fn apply_chance_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            1_000_047 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let actual_damage = self.active_effect_actual_damage();
                if self.active_effect_wounded_count() > 0 && actual_damage > 0 {
                    self.gain_anima(actor_side, actual_damage);
                }
                Some(attacked)
            }
            1_000_049 => {
                let multiplier = 1 + self.actor(actor_side).sword.frenzy_sword.max(0);
                self.gain_defense(actor_side, card.defense.unwrap_or(0).max(0) * multiplier);
                Some(false)
            }
            1_000_054 => {
                let bonus =
                    self.actor(actor_side).identity.last_round_exp / other_param(card, 0).max(1);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            1_000_058 => {
                let attack = card.attack.unwrap_or(0).max(0)
                    * if has_cloud_chain(self.actor(actor_side)) {
                        2
                    } else {
                        1
                    };
                let attack_count = 1 + self.actor(actor_side).core.anima.max(0);
                for _ in 0..attack_count {
                    self.apply_attack(actor_side, attack, slot);
                }
                Some(attack > 0 && attack_count > 0)
            }
            4_000_047 => {
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                let anima = self.actor(actor_side).core.anima.max(0);
                self.modify_hexagram(actor_side, -hexagram);
                self.spend_anima_unchecked(actor_side, anima);
                self.modify_actor_hp(
                    actor_side,
                    (hexagram + anima) * other_param(card, 0).max(0),
                    false,
                    false,
                );
                Some(false)
            }
            4_000_050 => {
                let actor_max_hp = self.actor(actor_side).core.max_hp;
                let target_max_hp = self.actor(opponent_side(actor_side)).core.max_hp;
                let bonus = (actor_max_hp - target_max_hp).max(0) / other_param(card, 1).max(1);
                self.apply_damage(
                    actor_side,
                    other_param(card, 0).max(0) + bonus,
                    false,
                    false,
                    false,
                );
                Some(false)
            }
            4_000_051 => {
                let star_bonus = if self.actor(actor_side).astrology.star_slots.contains(&slot)
                    && self.actor(actor_side).astrology.star_power > 0
                {
                    self.actor(actor_side).astrology.star_power * (other_param(card, 0).max(1) - 1)
                } else {
                    0
                };
                Some(self.attack_by_config(actor_side, card, star_bonus, slot))
            }
            4_000_053 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let weakness =
                    self.consume_random_range(actor_side, 0, other_param(card, 0).max(0));
                let flaw = self.consume_random_range(actor_side, 0, other_param(card, 1).max(0));
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 101, weakness);
                self.add_actor_negative_status(target_side, 102, flaw);
                Some(attacked)
            }
            4_000_054 => {
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                self.check_rear_move(actor_side, was_used_before_effect);
                Some(false)
            }
            7_000_046 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                if self.is_element_activated(actor_side, Element::Earth) {
                    let anima_loss = div_ceil(self.actor(target_side).core.anima.max(0), 2);
                    self.reduce_anima_unchecked(target_side, anima_loss);
                }
                self.apply_configured_defense(actor_side, card);
                let defense_loss = div_ceil(self.actor(target_side).core.defense.max(0), 2);
                self.lose_defense(target_side, defense_loss);
                Some(attacked)
            }
            7_000_048 => {
                if self.is_element_activated(actor_side, Element::Water) {
                    self.gain_anima(actor_side, other_param(card, 0).max(0));
                }
                if self.is_element_activated(actor_side, Element::Wood) {
                    let healing =
                        self.actor(actor_side).core.anima.max(0) * other_param(card, 1).max(0);
                    self.modify_actor_hp(actor_side, healing, false, false);
                }
                Some(false)
            }
            7_000_052 => {
                let mut attacked = false;
                for element in [
                    Element::Wood,
                    Element::Water,
                    Element::Fire,
                    Element::Metal,
                    Element::Earth,
                ] {
                    if self.is_element_activated(actor_side, element) {
                        attacked |= self.attack_by_config(actor_side, card, 0, slot);
                    }
                }
                Some(attacked)
            }
            7_000_063 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.is_element_activated(actor_side, Element::Metal) {
                    self.actor_mut(opponent_side(actor_side))
                        .mirage_ronghui
                        .cannot_gain_hp += other_param(card, 0).max(0);
                }
                Some(attacked)
            }
            7_000_065 => Some(false),
            10_000_054 => {
                let amount = other_param(card, 0).max(0);
                // Card_10000054.OnExecuted first calls src.ModifyTiPo(physique),
                // then (after the await) lowers both max HP values. The order
                // matters when max-HP loss clamps HP above the new cap: the
                // physique gain must be visible before that clamp.
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_max_hp(actor_side, -amount);
                self.modify_actor_max_hp(opponent_side(actor_side), -amount);
                Some(false)
            }
            10_000_061 => {
                let amount = other_param(card, 1).max(0);
                self.modify_momentum(actor_side, amount);
                self.gain_guard(actor_side, amount);
                self.gain_attack_bonus(actor_side, amount);
                let max_hp_loss = div_ceil(
                    self.actor(actor_side).core.max_hp.max(0) * other_param(card, 0).max(0),
                    100,
                );
                self.modify_actor_max_hp(actor_side, -max_hp_loss);
                Some(false)
            }
            10_000_064 => {
                let amount = other_param(card, 0).max(0);
                for side in [actor_side, opponent_side(actor_side)] {
                    for status in [100, 102, 101, 104, 105] {
                        self.add_actor_negative_status(side, status, amount);
                    }
                }
                Some(false)
            }
            10_000_067 => {
                let divisor = other_param(card, 0).max(1);
                let max_hp_bonus = self.actor(actor_side).core.max_hp / divisor;
                Some(self.attack_by_config(actor_side, card, max_hp_bonus, slot))
            }
            99_000_101 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.modify_temp_life(actor_side, other_param(card, 1).max(0));
                Some(false)
            }
            99_000_100 => {
                self.apply_configured_defense(actor_side, card);
                let current_defense = self.actor(actor_side).core.defense.max(0);
                self.gain_defense(actor_side, current_defense);
                Some(false)
            }
            99_000_102 => {
                let target_side = opponent_side(actor_side);
                let amount = other_param(card, 0).max(0);
                self.modify_actor_hp(target_side, -amount, false, false);
                self.modify_actor_max_hp(target_side, -amount);
                self.add_actor_negative_status(target_side, 100, other_param(card, 1).max(0));
                Some(false)
            }
            99_000_103 => {
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    101,
                    other_param(card, 1).max(0),
                );
                Some(false)
            }
            99_000_104 => {
                let target_side = opponent_side(actor_side);
                self.set_guard(target_side, 0);
                self.actor_mut(target_side).core.temporary_guard = 0;
                let defense = self.actor(target_side).core.defense;
                self.lose_defense(target_side, defense);
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                Some(false)
            }
            99_000_105 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                self.modify_temp_life(target_side, -other_param(card, 0).max(0));
                if self.actor(target_side).core.temp_life <= 0 {
                    // 原版 Card_99000105.cs：命元（tempLife）归零时仅在
                    // 当前 hp > 0 时把 hp 置 0，否则保留原值 —— 攻击已把
                    // hp 打成负数时不再归零。oracle 锚点：mirror-32299000
                    // f8d669b4ef449c86/round-15 cp23（p1.hp 38 → -6，
                    // 引擎原 0：第二次裂虚时 tempLife 7-4-4 = -1 触发
                    // 无条件的 hp=0，覆盖了 -6）。
                    if self.actor(target_side).core.hp > 0 {
                        self.actor_mut(target_side).core.hp = 0;
                    }
                    self.actor_mut(target_side).chance.cannot_revive += 1;
                }
                Some(attacked)
            }
            99_000_107 => {
                let target_defense = self.actor(opponent_side(actor_side)).core.defense.max(0);
                let bonus = target_defense / other_param(card, 0).max(1);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            99_000_111 => {
                self.actor_mut(opponent_side(actor_side)).status.cannot_act +=
                    other_param(card, 1).max(0);
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                Some(false)
            }
            99_000_112 => {
                self.gain_attack_bonus(actor_side, other_param(card, 1).max(0));
                let percent = other_param(card, 0).max(0);
                let hp_loss =
                    super::support::div_ceil(self.actor(actor_side).core.hp.max(0) * percent, 100);
                self.modify_actor_hp(actor_side, -hp_loss, false, false);
                Some(false)
            }
            99_000_113 => {
                let value = other_param(card, 0).max(0);
                let target_side = opponent_side(actor_side);
                self.actor_mut(actor_side).music.xiaoyao_tune += value;
                self.actor_mut(target_side).music.xiaoyao_tune += value;
                self.actor_mut(actor_side).music.xiaoyao_guqin += 1;
                self.actor_mut(target_side).music.xiaoyao_guqin += 1;
                Some(false)
            }
            99_000_114 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        104,
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            99_000_213 => {
                let attacks = card.attack_count.unwrap_or(1).max(0)
                    + self.distinct_deck_realms_except_slot(actor_side, slot)
                    + 1;
                for _ in 0..attacks {
                    self.apply_attack(actor_side, card.attack.unwrap_or(0), slot);
                }
                Some(attacks > 0 && card.attack.unwrap_or(0) > 0)
            }
            99_000_200 => {
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                self.actor_mut(actor_side).chance.po_kong_diao += other_param(card, 1).max(0);
                Some(false)
            }
            99_000_201 => {
                let value = other_param(card, 0).max(0);
                let current = self.actor(actor_side).chance.an_xing_bian_fu;
                self.actor_mut(actor_side).chance.an_xing_bian_fu = current.max(value);
                Some(false)
            }
            99_000_202 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).chance.di_xuan_gui += other_param(card, 0).max(0);
                Some(false)
            }
            99_000_203 => {
                self.modify_actor_hp(actor_side, -other_param(card, 1).max(0), false, false);
                self.actor_mut(actor_side).chance.jin_mao_shu += other_param(card, 0).max(0);
                Some(false)
            }
            99_000_204 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).chance.qi_cai_ling_he = other_param(card, 0).max(0);
                Some(false)
            }
            99_000_205 => {
                self.actor_mut(actor_side).chance.tun_tian_chi_yan_shou +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            99_000_206 => {
                self.actor_mut(actor_side).chance.shi_xu_ling_shou += other_param(card, 0).max(0);
                Some(false)
            }
            99_000_208 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.actor_mut(actor_side).chance.pang_xian_li += other_param(card, 1).max(0);
                Some(false)
            }
            99_000_210 => {
                self.actor_mut(actor_side).chance.ying_xiao_tu = other_param(card, 0).max(0);
                Some(false)
            }
            99_000_211 => {
                let target_side = opponent_side(actor_side);
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                self.actor_mut(target_side).chance.you_ming_xu_hun_quan += 1;
                Some(false)
            }
            99_000_212 => {
                self.actor_mut(actor_side).chance.san_wei_huan += other_param(card, 1).max(0);
                Some(false)
            }
            99_000_216 => {
                self.apply_phantom_feather_parrot(actor_side, card, slot);
                Some(false)
            }
            _ => None,
        }
    }

    fn distinct_deck_realms_except_slot(&self, actor_side: PlayerSide, slot: usize) -> i64 {
        let mut realms = BTreeSet::new();
        for (index, card_slot) in self.actor(actor_side).deck.slots.iter().enumerate() {
            if index == slot {
                continue;
            }
            let Some(level) = super::original_config::original_card_realm_level(card_slot.card.id)
            else {
                continue;
            };
            if (1..=5).contains(&level) {
                realms.insert(level);
            }
        }
        realms.len() as i64
    }

    fn apply_phantom_feather_parrot(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        if self.actor(actor_side).chance.huan_yu_ying_copy_guard > 0 {
            return;
        }
        self.actor_mut(actor_side).chance.huan_yu_ying_copy_guard += 1;
        self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
        let target_side = opponent_side(actor_side);
        let copied = self
            .actor(target_side)
            .deck
            .slots
            .get(slot)
            .map(|slot| slot.card.clone());
        if let Some(copied) = copied {
            // Card_99000216.cs: `cardItem.InitData(cardId, ...)` 只取对方
            // 牌组该格**卡牌 ID**，从全局 CardConfig 重新加载基础配置再执行；
            // 持有者专属改版（如 澄心剑胚 19 随持有者天赋 92/20093/30096
            // 改 attack/def，adapt_fixture_card_for_replay）不随复制带出。
            // oracle 锚点：hf-32308000 24f83df2a5db19b4/round-13 cp[7]
            // p1.defense 0 / p2.defense 31（引擎复制改版实例得 10/20）。
            let copied =
                super::original_config::original_card_definition(copied.id).unwrap_or(copied);
            if self.apply_temporary_card_effect(actor_side, &copied, slot) {
                self.modify_extra_actions(actor_side, 1);
            }
        }
        self.actor_mut(actor_side).chance.huan_yu_ying_copy_guard =
            (self.actor(actor_side).chance.huan_yu_ying_copy_guard - 1).max(0);
    }
}
