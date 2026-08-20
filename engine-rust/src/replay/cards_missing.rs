use super::cards_dream_mirage::DreamMirageValue;
use super::original_config::original_config_rarity;
use super::support::{
    card_rarity, div_ceil, element_from_card, is_element_generated_by,
    neighbor_card as support_neighbor_card, normalized_base_id, opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

const RANDOM_NEGATIVE_STATUSES: [i64; 8] = [100, 101, 102, 103, 104, 105, 367, 393];

impl ReplayState {
    pub(super) fn apply_card_effect_missing(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            126 => {
                // Card_126.cs in Steam build 24610558 delegates all six
                // talent branches to TalentConfig.otherParams (the same
                // values used by Card_19): 20094=1 attack bonus,
                // 30094=4 sword intent, 10095=2 anima, 20095=3 defense per
                // anima, and 30095=4 damage across two attack segments.
                self.apply_configured_defense(actor_side, card);
                if self.actor(actor_side).identity.talents.contains(&10_094) {
                    self.apply_attack(actor_side, 6, slot);
                }
                if self.actor(actor_side).identity.talents.contains(&20_094) {
                    self.gain_attack_bonus(actor_side, 1);
                }
                if self.actor(actor_side).identity.talents.contains(&30_094) {
                    self.modify_sword_intent(actor_side, 4);
                }
                if self.actor(actor_side).identity.talents.contains(&10_095) {
                    self.gain_anima(actor_side, 2);
                }
                if self.actor(actor_side).identity.talents.contains(&20_095) {
                    let anima = self.actor(actor_side).core.anima.max(0);
                    self.gain_defense(actor_side, anima * 3);
                }
                if self.actor(actor_side).identity.talents.contains(&30_095)
                    && self.actor(actor_side).core.anima > 0
                {
                    self.spend_anima_unchecked(actor_side, 1);
                    self.apply_attack(actor_side, 4, slot);
                    self.apply_attack(actor_side, 4, slot);
                }
                Some(self.actor(actor_side).turn.attack_segments_performed > 0)
            }
            23 => {
                if self.actor(actor_side).core.anima >= 1 {
                    self.modify_star_power(actor_side, other_param(card, 0).max(0));
                }
                self.actor_mut(actor_side).astrology.star_moon_fan += 1;
                Some(false)
            }
            36 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if super::support::has_cloud_chain(self.actor(actor_side)) {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        102,
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            52 => {
                // 摘花飞叶
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).status.leaf_pluck_flying_leaf += 1;
                Some(false)
            }
            56 => {
                // 熔岩之印
                let divisor = other_param(card, 1).max(1);
                let amount = other_param(card, 0).max(0)
                    + self.actor(actor_side).core.defense.max(0) / divisor;
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                Some(false)
            }
            45 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                Some(attacked)
            }
            70 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).fate.sheng_qi_ling_ren += other_param(card, 1).max(0);
                Some(false)
            }
            71 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.add_actor_negative_status(actor_side, 105, other_param(card, 0).max(0));
                Some(attacked)
            }
            72 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.add_actor_negative_status(actor_side, 100, other_param(card, 0).max(0));
                Some(attacked)
            }
            132 => {
                self.apply_configured_anima(actor_side, card);
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.modify_five_elements_gourd(actor_side, other_param(card, 1).max(0));
                Some(false)
            }
            140 => {
                // 崩拳•返玄
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.add_actor_negative_status(actor_side, 100, other_param(card, 1).max(0));
                let recovery_before = self.actor(actor_side).status.recovery;
                self.actor_mut(actor_side).status.recovery += other_param(card, 1).max(0);
                self.record_counter_transition(
                    actor_side,
                    "状态",
                    "recovery",
                    "恢复",
                    recovery_before,
                    self.actor(actor_side).status.recovery,
                );
                self.actor_mut(actor_side).beng.beng_quan_return_profound += 1;
                Some(false)
            }
            205 => {
                // 灵气锻身
                self.apply_configured_anima(actor_side, card);
                let anima_per_physique = other_param(card, 0).max(1);
                let anima_physique = (self.actor(actor_side).core.anima / anima_per_physique)
                    .min(other_param(card, 1).max(0))
                    .max(0);
                self.apply_configured_physique(actor_side, card);
                self.apply_physique_amount(actor_side, anima_physique);
                Some(false)
            }
            262 => {
                // Card_262.cs: 幻·引气剑把决策牌作为 isTempCard 完整执行；
                // 随后读取临时牌结算后的灵气，按所选牌境界追加自身攻击。
                let selected_id = self.consume_optional_decision();
                if selected_id < 0 {
                    return Some(false);
                }
                let Some(selected_card) =
                    super::original_config::original_card_definition(selected_id)
                else {
                    self.missing_decision("card:262 temporary card definition");
                    return Some(false);
                };
                if self.apply_temporary_card_effect(actor_side, &selected_card, slot) {
                    self.modify_extra_actions(actor_side, 1);
                }

                let realm =
                    super::original_config::original_card_realm_level(selected_id).unwrap_or(1);
                let mut attacked = false;
                if realm <= 4 {
                    let divisor = other_param(card, 1).max(1);
                    let attack = card.attack.unwrap_or(0).max(0)
                        + self.actor(actor_side).core.anima.max(0) / divisor;
                    let attack_count = card
                        .attack_count
                        .unwrap_or(if attack > 0 { 1 } else { 0 })
                        .max(0);
                    for _ in 0..attack_count {
                        if attack > 0 {
                            self.apply_attack(actor_side, attack, slot);
                            attacked = true;
                        }
                    }
                }
                Some(attacked)
            }
            268 => {
                // 幻•星罗棋布
                self.apply_configured_anima(actor_side, card);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).astrology.anima_to_star_power +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            275 => {
                // 幻•血气方刚
                let attack = card.attack.unwrap_or(0).max(0);
                let count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0)
                    + self.actor(actor_side).add_hp_count() / other_param(card, 0).max(1);
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                Some(attack > 0 && count > 0)
            }
            264 => {
                // Card_264.cs: 幻•云剑探云。牌体追加参数减一层连云，并
                // 安装一层云剑之心；通用云剑完成钩随后再追加一层连云。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_cloud_chain(actor_side, other_param(card, 0).max(1) - 1);
                self.actor_mut(actor_side).sword.cloud_sword_heart += 1;
                Some(attacked)
            }
            291 => {
                // 幻•岿然不动
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                self.apply_configured_defense(actor_side, card);
                let defense = self.actor(actor_side).beng.momentum * other_param(card, 1).max(0);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
                let agility = self.actor(actor_side).turn.agility.max(0);
                let healing = agility * other_param(card, 2).max(0);
                if healing > 0 {
                    self.modify_actor_hp(actor_side, healing, false, false);
                }
                if self.actor(actor_side).turn.agility > 5 {
                    self.set_agility_from_original_card_291(actor_side, 5);
                }
                Some(false)
            }
            294 => {
                // 幻•飞星刺
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                Some(attacked)
            }
            295 => {
                // 幻•五行流转
                self.apply_configured_anima(actor_side, card);
                let previous = support_neighbor_card(self.actor(actor_side), slot, -1).clone();
                let next = support_neighbor_card(self.actor(actor_side), slot, 1).clone();
                if let (Some(previous_element), Some(next_element)) =
                    (element_from_card(&previous), element_from_card(&next))
                {
                    let fire_generates_all = self.actor(actor_side).identity.talents.contains(&137);
                    if is_element_generated_by(previous_element, next_element, fire_generates_all)
                        || is_element_generated_by(
                            next_element,
                            previous_element,
                            fire_generates_all,
                        )
                    {
                        self.activate_element(actor_side, next_element);
                        self.modify_extra_actions(actor_side, 1);
                    }
                }
                // Card_295.cs: GetWuXingActiveNumber() * otherParams[0] defense.
                let activated_number = self.wu_xing_active_number(actor_side);
                let defense = activated_number * other_param(card, 0).max(0);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
                Some(false)
            }
            296 => {
                // 幻•双鬼拍门：原版依次给双方施加内伤、外伤、虚弱。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(actor_side, 100, other_param(card, 0).max(0));
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                self.add_actor_negative_status(actor_side, 105, other_param(card, 1).max(0));
                self.add_actor_negative_status(target_side, 105, other_param(card, 1).max(0));
                self.add_actor_negative_status(actor_side, 101, other_param(card, 2).max(0));
                self.add_actor_negative_status(target_side, 101, other_param(card, 2).max(0));
                Some(attacked)
            }
            308 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let divisor = other_param(card, 0).max(1);
                let gain = self.actor(actor_side).core.physique / divisor;
                self.actor_mut(actor_side).beng.beng_quan_chuo += gain;
                Some(attacked)
            }
            310 => {
                // 幻•浩然正气
                // Card_310.cs loops exactly otherParams[0] times and calls
                // GetNextParam() every iteration (removing the buff only when the
                // pick != -1). A -1 pick reduces nothing but is still consumed, so
                // the loop must not break early: breaking on the first -1 would
                // leave a trailing "no status" marker in the tape and desync every
                // later decision (e.g. a following 无尽崩绝 reading one card short).
                self.apply_configured_anima(actor_side, card);
                let mut removed = 0;
                for _ in 0..other_param(card, 0).max(0) {
                    let status = if self.decision_tape.is_empty() {
                        self.negative_status_types_present(actor_side)
                            .first()
                            .copied()
                    } else {
                        self.consume_optional_negative_status_decision()
                    };
                    if let Some(status) = status {
                        if self
                            .remove_actor_negative_status(actor_side, status, 1)
                            .applied
                            != 0
                        {
                            removed += 1;
                        }
                    }
                }
                if removed > 0 {
                    self.gain_agility(actor_side, removed * other_param(card, 1).max(0));
                    self.modify_momentum(actor_side, removed);
                }
                self.gain_agility(actor_side, other_param(card, 2).max(0));
                Some(false)
            }
            312 => {
                // 幻•狂剑盘龙
                self.apply_configured_defense(actor_side, card);
                let next_defense = self.actor(actor_side).sword.sword_formation_count
                    * other_param(card, 0).max(0);
                if next_defense > 0 {
                    self.actor_mut(actor_side).turn.next_turn_defense += next_defense;
                }
                Some(false)
            }
            319 => {
                // Card_319.cs: 幻•崩拳缠先施加外伤，再攻击，最后把牌面攻击
                // 登记为下一张崩拳结算后的两段追加攻击。
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 105, other_param(card, 0).max(0));
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::NextBengQuanAdditionalAttack,
                    card.attack.unwrap_or(0).max(0),
                );
                Some(attacked)
            }
            1_000_076 => {
                // 梦•狂剑零式
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                let frenzy_sword = self.actor(actor_side).sword.frenzy_sword.max(0);
                if frenzy_sword > 0 {
                    self.modify_actor_hp(
                        actor_side,
                        frenzy_sword * other_param(card, 1).max(0),
                        false,
                        false,
                    );
                    if self.actor(actor_side).identity.level >= 4 {
                        self.modify_sword_intent(actor_side, frenzy_sword);
                    }
                }
                Some(false)
            }
            1_000_067 => {
                self.apply_configured_anima(actor_side, card);
                let attack_bonus = other_param(card, 0).max(0);
                if super::original_config::original_card_realm_level(card.id).unwrap_or(1) <= 3 {
                    self.actor_mut(actor_side).turn.next_attack_bonus += attack_bonus;
                } else {
                    self.gain_attack_bonus(actor_side, attack_bonus);
                }
                Some(false)
            }
            4_000_072 => {
                // 梦•两仪阵
                self.apply_configured_defense(actor_side, card);
                let max_hp = other_param(card, 2).max(0);
                if max_hp > 0 {
                    self.modify_actor_max_hp(actor_side, max_hp);
                }
                self.actor_mut(actor_side)
                    .elements
                    .dream_two_polarity_defense += other_param(card, 0).max(0);
                if super::original_config::original_card_realm_level(card.id).unwrap_or(1) >= 3 {
                    self.actor_mut(actor_side).elements.dream_two_polarity_hp +=
                        other_param(card, 1).max(0);
                }
                Some(false)
            }
            4_000_076 => {
                // 梦•海底捞月
                if !self.check_rear_move(actor_side, was_used_before_effect) {
                    return Some(false);
                }
                let rear_slot_count = other_param(card, 2).max(0);
                let rear_slot_bonus = if card_rarity(card) + 1 > 3
                    && rear_slot_count > 0
                    && slot as i64 > 7 - rear_slot_count
                {
                    other_param(card, 3).max(0)
                } else {
                    0
                };
                let attack = other_param(card, 0).max(0) + rear_slot_bonus;
                let count = other_param(card, 1).max(0);
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                Some(attack > 0 && count > 0)
            }
            9_000_017 => {
                // 邪灵葵：原版 Card_9000017.cs 先按「扣减前」的灵气计算伤害
                // 值，再 ModifyAnima 扣灵气（触发御灵心法等「每失去灵气加防」
                // 的持续效果），最后才 ApplyDamage —— 因此扣灵带来的防御在
                // 本次伤害结算前生效并抵扣（oracle 锚点：
                // 9905fbac33978a8e/round-09 cp4 p1.hp 75 vs 73、
                // d5bf6497da48c668/round-08 cp16 p2.hp 19 vs 17、
                // efa7a146be9540c8/round-11 cp5 p1.hp 59 vs 57）。
                let target_side = opponent_side(actor_side);
                let drain = other_param(card, 0).max(0);
                let damage_per_missing = other_param(card, 1).max(0);
                let target_anima = self.actor(target_side).core.anima.max(0);
                let damage = if damage_per_missing > 0 && target_anima < drain {
                    (drain - target_anima) * damage_per_missing
                } else {
                    0
                };
                self.reduce_anima_unchecked(target_side, drain);
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                }
                Some(false)
            }
            11_000_001 => {
                // 卜命：出牌主体与开局分支分别读取 otherParams[0]/[1]。
                self.modify_target_hp(actor_side, -other_param(card, 0).max(0));
                Some(false)
            }
            99_000_110 => {
                // 金元三尖枪
                let target_side = opponent_side(actor_side);
                let attack = card.attack.unwrap_or(0).max(0);
                let count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0);
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                    self.add_actor_negative_status(target_side, 102, 1);
                }
                Some(attack > 0 && count > 0)
            }
            324 => {
                let value = other_param(card, 0).max(0);
                if value > 0 {
                    self.modify_actor_max_hp(actor_side, value);
                    self.modify_actor_hp(actor_side, value, false, false);
                }
                self.actor_mut(actor_side).fate.mirage_vitality_bloom += 1;
                self.actor_mut(actor_side).fate.mirage_vitality_bloom_heal =
                    other_param(card, 1).max(0);
                Some(false)
            }
            330 => {
                let target_side = opponent_side(actor_side);
                let injury = other_param(card, 1).max(0);
                self.add_actor_negative_status(target_side, 100, injury);
                self.add_actor_negative_status(target_side, 105, injury);
                self.apply_damage(actor_side, other_param(card, 0).max(0), false, false, false);
                Some(false)
            }
            332 => {
                // 玲珑剑阵先获得牌面灵气，再以结算后的灵气同时计算伤害
                // 与下回合防御（Card_332.OnExecuted 的原版顺序）。
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.apply_damage(
                        actor_side,
                        anima * other_param(card, 0).max(0),
                        false,
                        false,
                        false,
                    );
                    self.actor_mut(actor_side).turn.next_turn_defense +=
                        anima * other_param(card, 1).max(0);
                    if self.actor(actor_side).identity.talents.contains(&222) {
                        self.modify_actor_max_hp(actor_side, anima);
                        self.modify_actor_hp(actor_side, anima, false, false);
                    }
                }
                Some(false)
            }
            337 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                let defense = self.actor(actor_side).core.defense;
                if defense > 0 {
                    self.apply_damage(actor_side, defense, false, false, false);
                }
                self.actor_mut(actor_side).beng.beng_quan_fu_hu += 1;
                Some(attacked)
            }
            341 => {
                // 梦·神来之笔
                self.apply_configured_defense(actor_side, card);
                let selected_id = self.consume_optional_decision();
                if selected_id < 0 {
                    return Some(false);
                }
                let selected_card = super::original_config::original_card_definition(selected_id)?;
                if self.apply_temporary_card_effect(actor_side, &selected_card, slot) {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(false)
            }
            387 => {
                // 梦·弱体符；再次行动与消耗由配置和通用事务处理。
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    101,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            388 => {
                // 百鸟曳影诀
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .sword
                    .hundred_bird_trailing_shadow_art += 1;
                Some(false)
            }
            339 => {
                // 逍遥无影拳
                let basic_attacks = self
                    .actor(actor_side)
                    .deck
                    .slots
                    .iter()
                    .filter(|slot| normalized_base_id(&slot.card) == 0)
                    .count() as i64;
                let attack = card.attack.unwrap_or(0).max(0);
                let count = card.attack_count.unwrap_or(1).max(0) + basic_attacks;
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                    self.apply_physique_amount(actor_side, other_param(card, 0).max(0));
                    self.gain_agility(actor_side, other_param(card, 1).max(0));
                }
                Some(attack > 0 && count > 0)
            }
            2_000_002 => {
                self.apply_elixir_base(actor_side, card);
                Some(false)
            }
            2_000_005 => {
                self.apply_elixir_base(actor_side, card);
                self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            3_000_002 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                Some(false)
            }
            3_000_014 => {
                let target_side = opponent_side(actor_side);
                let target = self.actor(target_side);
                let percent = other_param(card, 0).max(0);
                let hp_loss = div_ceil(target.core.hp * percent, 100);
                let max_hp_loss = div_ceil(target.core.max_hp * percent, 100);
                if hp_loss > 0 {
                    self.modify_actor_hp(target_side, -hp_loss, false, false);
                }
                if max_hp_loss > 0 {
                    self.modify_actor_max_hp(target_side, -max_hp_loss);
                }
                let anima_loss = other_param(card, 1).max(0);
                if anima_loss > 0 {
                    self.reduce_anima_unchecked(target_side, anima_loss);
                }
                Some(false)
            }
            5_000_003 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.xiaoyao_tune += value;
                self.actor_mut(opponent_side(actor_side)).music.xiaoyao_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_012 => {
                let target_side = opponent_side(actor_side);
                let statuses = [
                    (100, self.actor(actor_side).status.internal_injury),
                    (101, self.actor(actor_side).status.weakness),
                    (102, self.actor(actor_side).status.flaw),
                    (103, self.actor(actor_side).status.attack_reduction),
                    (104, self.actor(actor_side).status.entangle),
                    (105, self.actor(actor_side).status.external_injury),
                    (367, self.actor(actor_side).status.meditation),
                ];
                for (status, amount) in statuses {
                    if amount > 0 {
                        self.add_actor_negative_status(target_side, status, amount);
                    }
                }
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_013 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.chaotic_mind_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .chaotic_mind_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            6_000_005 => {
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0));
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            6_000_007 => {
                if self.decision_tape.first().is_some_and(|value| *value < 0) {
                    self.decision_tape.remove(0);
                    return Some(false);
                }
                let selected = self
                    .consume_required_negative_status_decision()
                    .or_else(|| {
                        let index = self.consume_random_range_or_default_plain(
                            actor_side,
                            0,
                            RANDOM_NEGATIVE_STATUSES.len() as i64 - 1,
                            0,
                        );
                        RANDOM_NEGATIVE_STATUSES.get(index as usize).copied()
                    });
                if let Some(selected) = selected {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        selected,
                        other_param(card, 0).max(0),
                    );
                }
                Some(false)
            }
            6_000_008 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .turn
                    .next_card_anima_cost_reduction += other_param(card, 0).max(0);
                Some(false)
            }
            6_000_011 => {
                self.apply_configured_defense(actor_side, card);
                let selected_id = self.consume_required_decision();
                let Some(selected_card) =
                    super::original_config::original_card_definition(selected_id)
                else {
                    self.missing_decision("card:6000011 temporary card definition");
                    return Some(false);
                };
                if self.apply_temporary_card_effect(actor_side, &selected_card, slot) {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(false)
            }
            6_000_014 => {
                if self.decision_tape.first().is_some_and(|value| *value < 0) {
                    self.decision_tape.remove(0);
                    return Some(false);
                }
                // Card_6000014.cs:80 用 GetNextParam()（原版 BattleCharacter
                // GetNextParam:9314-9320），不消耗卦象；随机值来自服务端
                // battleParams 队列（replay 的 decisionTape）。
                if self.decision_tape.is_empty() {
                    // 原版 GetNextParam 对空队列 Dequeue 抛异常 → catch 返回
                    // -1 → switch 无匹配分支 → 整张牌无效果（不回退默认 0）。
                    // oracle 锚点：mirror-32299000 98e5d481d179d416/round-19
                    // cp76（队列 5 个参数在 turn34 耗尽，原版 turn40 落纸云烟
                    // p1.def=0，引擎默认 0 → +26 防）。
                    return Some(false);
                }
                match self.consume_random_range_or_default_plain(actor_side, 0, 2, 0) {
                    0 => {
                        self.gain_defense(actor_side, other_param(card, 0).max(0));
                    }
                    1 => {
                        self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                    }
                    2 => {
                        self.gain_guard(actor_side, other_param(card, 2).max(0));
                    }
                    _ => {}
                }
                Some(false)
            }
            7_000_014 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .elements
                    .next_card_activate_element += other_param(card, 0).max(0);
                Some(attacked)
            }
            8_000_006 => {
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    self.modify_actor_max_hp(actor_side, other_param(card, 1).max(0));
                }
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                Some(false)
            }
            8_000_009 => {
                self.actor_mut(actor_side).fate.exorcism += other_param(card, 0).max(0);
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    self.gain_defense(actor_side, other_param(card, 1).max(0));
                }
                Some(false)
            }
            9_000_005 => {
                let bonus = if self.actor(actor_side).core.anima > 0
                    || self.actor(opponent_side(actor_side)).core.anima > 0
                {
                    other_param(card, 0).max(0)
                } else {
                    0
                };
                self.gain_anima(actor_side, card.anima.unwrap_or(0).max(0) + bonus);
                Some(false)
            }
            9_000_009 => {
                self.apply_configured_defense(actor_side, card);
                let value = other_param(card, 0).max(0);
                if value > 0 {
                    self.actor_mut(actor_side).fate.leaf_shield_flower = value;
                }
                Some(false)
            }
            9_000_013 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                let lotus = other_param(card, 1).max(0);
                if lotus > 0 {
                    self.actor_mut(actor_side).fate.ice_snow_lotus += lotus;
                }
                Some(false)
            }
            9_000_018 => {
                // Card_9000018.OnExecuted: the current attack resolves before
                // YeRenHua is installed for later attacks.
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let leaf_blade_flower = other_param(card, 0).max(0);
                if leaf_blade_flower > 0 {
                    self.actor_mut(actor_side).status.leaf_blade_flower = leaf_blade_flower;
                }
                Some(attacked)
            }
            9_000_020 => {
                let damage = other_param(card, 0).max(0);
                let guard_gain = other_param(card, 1).max(0);
                for _ in 0..guard_gain.max(0) {
                    let actor = self.actor(actor_side);
                    let mut loss = damage.min((actor.core.hp - 1).max(0));
                    if actor.core.guard > 0 && loss <= 0 {
                        loss = 1;
                    }
                    if loss > 0 {
                        self.modify_actor_hp(actor_side, -loss, false, false);
                    }
                }
                self.gain_guard(actor_side, guard_gain);
                Some(false)
            }
            10_000_023 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).fate.exorcism += other_param(card, 0).max(0);
                Some(false)
            }
            10_000_032 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                Some(false)
            }
            10_000_043 => {
                let momentum = self.actor(actor_side).beng.momentum.max(0);
                if momentum > 0 {
                    self.modify_momentum(actor_side, -momentum);
                    self.gain_defense(actor_side, momentum * other_param(card, 0).max(0));
                    let damage = momentum * other_param(card, 1).max(0);
                    if damage > 0 {
                        self.apply_damage(actor_side, damage, false, false, false);
                    }
                }
                self.gain_agility(actor_side, other_param(card, 2).max(0));
                Some(false)
            }
            10_000_060 => {
                // 崩拳•双影
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_double_shadow += 1;
                Some(attacked)
            }
            11_000_008 => {
                self.actor_mut(actor_side).fate.fortune_seek_auspicious +=
                    other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .fate
                    .fortune_seek_auspicious_damage = other_param(card, 1).max(0);
                Some(false)
            }
            11_000_011 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_star_chess_break(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            11_000_012 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.actor_mut(actor_side).turn.jump_to_previous_card +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            11_000_023 => {
                self.modify_blood_calamity(opponent_side(actor_side), other_param(card, 0).max(0));
                Some(false)
            }
            10_000_055 => {
                // 百毒不侵
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.actor_mut(actor_side).status.poison_immunity += other_param(card, 1).max(0);
                Some(false)
            }
            1_000_018 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            1_000_023 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.reduce_anima_unchecked(
                        opponent_side(actor_side),
                        other_param(card, 0).max(0),
                    );
                }
                Some(attacked)
            }
            99_000_109 => {
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                let recovery_before = self.actor(actor_side).status.recovery;
                self.actor_mut(actor_side).status.recovery += other_param(card, 1).max(0);
                self.record_counter_transition(
                    actor_side,
                    "状态",
                    "recovery",
                    "恢复",
                    recovery_before,
                    self.actor(actor_side).status.recovery,
                );
                Some(false)
            }
            99_000_207 => {
                self.actor_mut(actor_side).fate.fire_phoenix_revive_hp +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            99_000_209 => {
                self.actor_mut(actor_side).status.lone_night_wolf += other_param(card, 0).max(0);
                Some(false)
            }
            99_000_214 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn card_has_opening_effect(base_id: i64) -> bool {
        Self::dream_mirage_card_has_opening_effect(base_id)
            || Self::mirage_ronghui_card_has_opening_effect(base_id)
            || Self::ronghui_card_has_opening_effect(base_id)
            || Self::synthetic_full_scope_candidate_has_opening_effect(base_id)
            || matches!(
                base_id,
                55 | 56
                    | 57
                    | 58
                    | 11_000_001
                    | 11_000_005
                    | 11_000_009
                    | 11_000_013
                    | 11_000_014
                    | 11_000_018
                    | 11_000_022
                    | 11_000_023
                    | 11_000_024
            )
    }

    pub(super) fn apply_opening_effect_for_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot_index: usize,
    ) {
        let previous_card_execution = self.begin_card_execution(actor_side, card.id);
        self.apply_opening_effect_for_card_inner(actor_side, card, slot_index, slot_index);
        self.finish_card_execution(previous_card_execution);
    }

    /// 带 triggerGrid 的开局触发：原版 `TriggerOpening(grid, triggerGrid)`
    /// 中 triggerGrid 决定「同格」类效果的目标格（BattleCharacter.cs:11056-11059
    /// 默认取 grid 自身；天星•牵引/天星•反击 Card_11000025/11000026.cs
    /// 传的是天星牌自己的格位）。
    pub(super) fn apply_opening_effect_for_card_with_trigger_grid(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot_index: usize,
        trigger_grid: usize,
    ) {
        let previous_card_execution = self.begin_card_execution(actor_side, card.id);
        self.apply_opening_effect_for_card_inner(actor_side, card, slot_index, trigger_grid);
        self.finish_card_execution(previous_card_execution);
    }

    fn apply_opening_effect_for_card_inner(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot_index: usize,
        trigger_grid: usize,
    ) {
        let base_id = normalized_base_id(card);
        self.apply_dream_mirage_battle_start_opening_with_trigger_grid(
            actor_side,
            card,
            slot_index,
            base_id,
            trigger_grid,
        );
        self.apply_mirage_ronghui_battle_start_opening(actor_side, base_id);
        self.apply_ronghui_battle_start_opening(actor_side, card, slot_index);
        self.apply_synthetic_full_scope_candidate_opening(actor_side, base_id);
        match base_id {
            55 => {
                self.activate_element(actor_side, super::Element::Water);
                self.activate_element(actor_side, super::Element::Wood);
            }
            56 => {
                self.activate_element(actor_side, super::Element::Fire);
                self.activate_element(actor_side, super::Element::Earth);
            }
            58 => self.apply_wave_cutting_seal_opening(actor_side),
            11_000_001 => {
                let damage = other_param(card, 1);
                if damage > 0 {
                    self.modify_target_hp(actor_side, -damage);
                }
            }
            11_000_005 => {
                let value = other_param(card, 1).max(0);
                if value > 0 {
                    self.modify_actor_max_hp(actor_side, value);
                }
                self.modify_actor_hp(actor_side, value, false, false);
            }
            11_000_009 => {
                self.gain_anima(actor_side, other_param(card, 0).max(0));
            }
            11_000_013 => {
                self.actor_mut(actor_side).fate.exorcism += other_param(card, 2).max(0);
            }
            11_000_014 => {
                self.gain_defense(actor_side, other_param(card, 2).max(0));
            }
            11_000_018 => {
                // 同格目标 = triggerGrid（天星•牵引等触发者传其自身格位；
                // 开局/命运轮回跳过路径默认同格自身，BattleCharacter.cs:11132-11148）。
                let target_side = opponent_side(actor_side);
                if let Some(target_slot) = self.actor(target_side).deck.slots.get(trigger_grid) {
                    let target_card = target_slot.card.clone();
                    // 原版 TriggerOpening case 11000018（BattleCharacter.cs:
                    // 11132-11147）读 cardItem2.cardConfig.rarity（配置值，
                    // 无 rarity 字段 = 0），不是 id 档位推断值。梦牌/隐藏牌
                    // 配置 rarity=0（如 1040081 梦•巨鹏灵剑），原版不降级而
                    // 是造成 otherParams[1] ReflectDamage。oracle 锚点：
                    // hf-latest-32308000-16f9c778 985f1b6f8b6b9fdf/round-13
                    // cp（命运轮回 skip-opening 对 1040081：原版 6 伤-1 防=5，
                    // 引擎 id 档位误判 rarity=4 走降级漏伤；且降级后 1030081
                    // 在 T11 被误打出）。
                    if original_config_rarity(target_card.id) >= 1 && target_card.id != 19 {
                        let lower_id = target_card.id - 10_000;
                        if let Some(lowered) =
                            super::original_config::original_card_definition(lower_id)
                        {
                            self.actor_mut(target_side).deck.slots[trigger_grid].card = lowered;
                        }
                    } else {
                        let damage = other_param(card, 1).max(0);
                        if damage > 0 {
                            self.apply_damage(actor_side, damage, false, false, false);
                        }
                    }
                }
            }
            11_000_022 => {
                // 察体 [开局]：下 otherParams[1] 次攻击附加[碎防]。
                // BattleCharacter.TriggerOpening case 11000022 与开局通用路径
                // 同款（ModifyBuffValue(XiaCiGongJiSuiFang, otherParams[1])）。
                self.modify_next_attack_shatter_defense(actor_side, other_param(card, 1).max(0));
            }
            11_000_023 => {
                let loss = other_param(card, 1).max(0);
                if loss > 0 {
                    self.modify_actor_hp(opponent_side(actor_side), -loss, false, false);
                    self.modify_actor_hp(actor_side, -loss, false, false);
                }
            }
            11_000_024 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 1).max(0),
                );
            }
            _ => {}
        }
    }

    pub(super) fn apply_beng_quan_fu_hu_before_attack(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        persist: bool,
    ) {
        if !super::support::applies_beng_quan_inherited_effects(self.actor(actor_side), card) {
            return;
        }
        let base_id = super::support::normalized_base_id(card);
        let fu_hu = self.actor(actor_side).beng.beng_quan_fu_hu.max(0);
        if fu_hu <= 0 {
            return;
        }
        self.actor_mut(actor_side).beng.triggered_beng_quan_fu_hu += fu_hu;
        if !persist && base_id != 10_000_035 {
            self.actor_mut(actor_side).beng.beng_quan_fu_hu -= fu_hu;
        }
    }

    pub(super) fn apply_beng_quan_fu_hu_after_card(&mut self, actor_side: PlayerSide) {
        let triggered = self.actor(actor_side).beng.triggered_beng_quan_fu_hu.max(0);
        if triggered <= 0 {
            return;
        }
        let defense = self.actor(actor_side).core.defense;
        if defense > 0 {
            self.apply_damage(actor_side, defense * triggered, false, false, false);
        }
        self.actor_mut(actor_side).beng.triggered_beng_quan_fu_hu = 0;
    }
}
