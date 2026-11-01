use super::cards_dream_mirage::DreamMirageValue;
use super::flow_card_effect::{param0, param1, param2, param3};
use super::support::{
    active_neighbor_card, card_rarity, div_ceil, element_from_card, elements_in_card_name,
    is_element_generated_by, neighbor_card as support_neighbor_card, normalized_base_id,
    opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

const RANDOM_NEGATIVE_STATUSES: [i64; 8] = [100, 101, 102, 103, 104, 105, 367, 393];

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn apply_card_effect_missing(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            311 => {
                // 云剑·月旋 Card_311 converts all current 剑意 before attack.
                let sword = self.actor(actor_side).sword.sword_intent;
                if sword > 0 {
                    self.modify_sword_intent(actor_side, -sword);
                    self.gain_attack_bonus(actor_side, sword);
                }
                Some(self.attack_by_config(actor_side, card, 0, slot))
            }
            400 => {
                let amount = param0(card);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
                self.actor_mut(actor_side)
                    .sword
                    .frenzy_sword_cloud_gathering += 1;
                Some(false)
            }
            430 => {
                // 獾之力 Card_430: Damage, not Attack. Exchange gain is outside battle.
                self.apply_damage(actor_side, param0(card), false, false, false);
                Some(false)
            }
            431 => {
                // 龟之力 Card_431: defense then max HP; no healing. Permanent
                // player-data write is outside this battle's kernel.
                self.apply_configured_defense(actor_side, card);
                self.modify_actor_max_hp(actor_side, param0(card));
                Some(false)
            }
            433 | 434 => {
                // 狸之力 / 鹦之力: Card_433 / Card_434 heal. 命元 / copy /
                // cultivation are outside battle; mirror skips the life write.
                self.modify_actor_hp(actor_side, param0(card), false, false);
                Some(false)
            }
            439 => {
                // 极·冥心入玄: Card_439 order is physique, 冥心入玄, agility.
                self.apply_configured_physique(actor_side, card);
                self.actor_mut(actor_side).fate.mystic_heart_enter_profound += param0(card);
                self.gain_agility(actor_side, param1(card));
                Some(false)
            }
            438 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.gain_agility(actor_side, param0(card));
                let previous = support_neighbor_card(self.actor(actor_side), slot, -1);
                let next = support_neighbor_card(self.actor(actor_side), slot, 1);
                if super::support::is_five_element_control(previous, next) {
                    // ActiveWuXingInName selects the first element in original
                    // name order, identical to the ordinary 五行流转 kernel.
                    let element = element_from_card(next);
                    if let Some(element) = element {
                        self.activate_element(actor_side, element);
                    }
                }
                Some(false)
            }
            448 => {
                // 极·画龙点睛 Card_448; actionAgain comes from current config.
                self.modify_paint_finishing_touch(actor_side, param0(card));
                Some(false)
            }
            1_000_093 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_attack_bonus(actor_side, param0(card));
                if self.actor(actor_side).turn.successful_action_again_total > 0 {
                    self.modify_extra_actions(actor_side, 1);
                }
                Some(attacked)
            }
            10_000_110 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_physique(actor_side, card);
                self.gain_attack_bonus(actor_side, param0(card));
                Some(false)
            }
            435 => {
                // Current-build Card_435.cs:45-55: Attack(config.attack,
                // config.attackCount), then actionAgainPerRound++ and
                // KeYinShanFeng(572) += 1. The engraving is removed at turn end
                // and subtracts exactly its stacks from the per-round limit.
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.modify_action_again_per_round(actor_side, 1);
                self.modify_keyin_shan_feng(actor_side, 1);
                Some(attacked)
            }
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
                    self.modify_star_power(actor_side, param0(card));
                }
                self.actor_mut(actor_side).astrology.star_moon_fan += 1;
                Some(false)
            }
            36 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if super::support::has_cloud_chain(self.actor(actor_side)) {
                    self.add_actor_negative_status(opponent_side(actor_side), 102, param0(card));
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
                let amount = param0(card) + self.actor(actor_side).core.defense.max(0) / divisor;
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                Some(false)
            }
            45 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, param0(card), false, false);
                Some(attacked)
            }
            70 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_momentum(actor_side, param0(card));
                self.actor_mut(actor_side).fate.sheng_qi_ling_ren += param1(card);
                Some(false)
            }
            71 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.add_actor_negative_status(actor_side, 105, param0(card));
                Some(attacked)
            }
            72 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.add_actor_negative_status(actor_side, 100, param0(card));
                Some(attacked)
            }
            132 => {
                self.apply_configured_anima(actor_side, card);
                let hp_gain = param0(card);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.modify_five_elements_gourd(actor_side, param1(card));
                Some(false)
            }
            140 => {
                // 崩拳•返玄
                self.modify_actor_hp(actor_side, param0(card), false, false);
                self.add_actor_negative_status(actor_side, 100, param1(card));
                let recovery_before = self.actor(actor_side).status.recovery;
                self.actor_mut(actor_side).status.recovery += param1(card);
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
                    .min(param1(card))
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
                self.modify_star_power(actor_side, param0(card));
                self.actor_mut(actor_side).astrology.anima_to_star_power += param1(card);
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
                // 幻•岿然不动（Card_291.cs build 25621897：先加气势上限再加气势）
                self.modify_momentum_limit(actor_side, param0(card));
                self.modify_momentum(actor_side, param0(card));
                self.apply_configured_defense(actor_side, card);
                let defense = self.actor(actor_side).beng.momentum * param1(card);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
                let agility = self.actor(actor_side).turn.agility.max(0);
                let healing = agility * param2(card);
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
                self.modify_star_power(actor_side, param0(card));
                Some(attacked)
            }
            295 => {
                // 幻•五行流转（Card_295.cs）：灵气 → 相邻两名任一方向相生则按名字激活其中
                // 全部五行（ActiveWuXingInName 先 prev 后 next）+ 再次行动 → 防御 +=
                // 已激活数 × otherParams[0]。相邻按 GetPreviousGrid/GetNextGrid 在
                // unlockGrids 内回绕（含 FanZhuanChuPai 反向），即 active_neighbor。
                self.apply_configured_anima(actor_side, card);
                let previous =
                    active_neighbor_card(self.actor(actor_side), slot, -1).cloned();
                let next = active_neighbor_card(self.actor(actor_side), slot, 1).cloned();
                if let (Some(previous), Some(next)) = (previous.as_ref(), next.as_ref()) {
                    let fire_generates_all =
                        self.actor(actor_side).identity.talents.contains(&137);
                    let previous_elements = elements_in_card_name(previous);
                    let next_elements = elements_in_card_name(next);
                    let generated = previous_elements.iter().any(|a| {
                        next_elements.iter().any(|b| {
                            is_element_generated_by(*a, *b, fire_generates_all)
                                || is_element_generated_by(*b, *a, fire_generates_all)
                        })
                    });
                    if generated {
                        for element in previous_elements
                            .into_iter()
                            .chain(next_elements)
                        {
                            self.activate_element(actor_side, element);
                        }
                        self.modify_extra_actions(actor_side, 1);
                    }
                }
                // Card_295.cs: GetWuXingActiveNumber() * otherParams[0] defense.
                let activated_number = self.wu_xing_active_number(actor_side);
                let defense = activated_number * param0(card);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
                Some(false)
            }
            296 => {
                // 幻•双鬼拍门：原版依次给双方施加内伤、外伤、虚弱。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(actor_side, 100, param0(card));
                self.add_actor_negative_status(target_side, 100, param0(card));
                self.add_actor_negative_status(actor_side, 105, param1(card));
                self.add_actor_negative_status(target_side, 105, param1(card));
                self.add_actor_negative_status(actor_side, 101, param2(card));
                self.add_actor_negative_status(target_side, 101, param2(card));
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
                for _ in 0..param0(card) {
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
                    self.gain_agility(actor_side, removed * param1(card));
                    self.modify_momentum(actor_side, removed);
                }
                self.gain_agility(actor_side, param2(card));
                Some(false)
            }
            312 => {
                // 幻•狂剑盘龙
                self.apply_configured_defense(actor_side, card);
                let next_defense =
                    self.actor(actor_side).sword.sword_formation_count * param0(card);
                if next_defense > 0 {
                    self.actor_mut(actor_side).turn.next_turn_defense += next_defense;
                }
                Some(false)
            }
            319 => {
                // Card_319.cs: 幻•崩拳缠先施加外伤，再攻击，最后把牌面攻击
                // 登记为下一张崩拳结算后的两段追加攻击。
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 105, param0(card));
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
                self.modify_actor_hp(actor_side, param0(card), false, false);
                let frenzy_sword = self.actor(actor_side).sword.frenzy_sword.max(0);
                if frenzy_sword > 0 {
                    self.modify_actor_hp(actor_side, frenzy_sword * param1(card), false, false);
                    if self.actor(actor_side).identity.level >= 4 {
                        self.modify_sword_intent(actor_side, frenzy_sword);
                    }
                }
                Some(false)
            }
            1_000_067 => {
                self.apply_configured_anima(actor_side, card);
                let attack_bonus = param0(card);
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
                let max_hp = param2(card);
                if max_hp > 0 {
                    self.modify_actor_max_hp(actor_side, max_hp);
                }
                self.actor_mut(actor_side)
                    .elements
                    .dream_two_polarity_defense += param0(card);
                if super::original_config::original_card_realm_level(card.id).unwrap_or(1) >= 3 {
                    self.actor_mut(actor_side).elements.dream_two_polarity_hp += param1(card);
                }
                Some(false)
            }
            4_000_076 => {
                // 梦•海底捞月
                if !self.check_rear_move(actor_side, was_used_before_effect) {
                    return Some(false);
                }
                let rear_slot_count = param2(card);
                let rear_slot_bonus = if card_rarity(card) + 1 > 3
                    && rear_slot_count > 0
                    && slot as i64 > 7 - rear_slot_count
                {
                    param3(card)
                } else {
                    0
                };
                let attack = param0(card) + rear_slot_bonus;
                let count = param1(card);
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
                // pa_2cf0ca52 cp4 p1.hp 75 vs 73、
                // pa_70986d31 cp16 p2.hp 19 vs 17、
                // pa_4bc9c1ae cp5 p1.hp 59 vs 57）。
                let target_side = opponent_side(actor_side);
                let drain = param0(card);
                let damage_per_missing = param1(card);
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
                self.modify_target_hp(actor_side, -param0(card));
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
                // Card_324.cs:80-83：先 ModifyAnima(anima)，再
                // ModifyMaxHp/ModifyHp(otherParams[0])，最后
                // ModifyBuffValue(HuanShengJiZhanFang, otherParams[1])。
                // buff 值是累加，不是覆盖。回响阵纹把同一张幻•生机绽放再执行
                // 一次时，回合末治疗量必须是各次 otherParams[1] 之和。
                // oracle 锚点：hf-latest-33912000 pa_47b5f0ae
                // 回响后的回合交界 p1.maxHp 原版 176、引擎 160（少累加 16）。
                // oracle 锚点 2：hf-34040000 pa_f9cb62aa cp[1]
                // p1.anima 原版 1、引擎 0（漏首行动画灵气）。
                self.apply_configured_anima(actor_side, card);
                let value = param0(card);
                if value > 0 {
                    self.modify_actor_max_hp(actor_side, value);
                    self.modify_actor_hp(actor_side, value, false, false);
                }
                self.actor_mut(actor_side).fate.mirage_vitality_bloom += 1;
                self.actor_mut(actor_side).fate.mirage_vitality_bloom_heal += param1(card);
                Some(false)
            }
            330 => {
                let target_side = opponent_side(actor_side);
                let injury = param1(card);
                self.add_actor_negative_status(target_side, 100, injury);
                self.add_actor_negative_status(target_side, 105, injury);
                self.apply_damage(actor_side, param0(card), false, false, false);
                Some(false)
            }
            332 => {
                // 玲珑剑阵先获得牌面灵气，再以结算后的灵气同时计算伤害
                // 与下回合防御（Card_332.OnExecuted 的原版顺序）。
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.apply_damage(actor_side, anima * param0(card), false, false, false);
                    self.actor_mut(actor_side).turn.next_turn_defense += anima * param1(card);
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
                self.apply_damage(actor_side, defense, false, false, false);
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
                self.add_actor_negative_status(opponent_side(actor_side), 101, param0(card));
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
                // 逍遥无影拳：Card_339 按 IsPuTongGongJi（基础 id 0 或 286）计数
                // （BattleCharacter.cs:12928；oracle pa_c97d6c6b）。
                let basic_attacks = self
                    .actor(actor_side)
                    .deck
                    .slots
                    .iter()
                    .filter(|slot| {
                        super::support::is_pu_tong_gong_ji_base_id(normalized_base_id(&slot.card))
                    })
                    .count() as i64;
                let attack = card.attack.unwrap_or(0).max(0);
                let count = card.attack_count.unwrap_or(1).max(0) + basic_attacks;
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                    self.apply_physique_amount(actor_side, param0(card));
                    self.gain_agility(actor_side, param1(card));
                }
                Some(attack > 0 && count > 0)
            }
            2_000_002 => {
                self.apply_elixir_base(actor_side, card);
                Some(false)
            }
            2_000_005 => {
                self.apply_elixir_base(actor_side, card);
                self.actor_mut(actor_side).turn.ignore_defense_attacks += param0(card);
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
                let percent = param0(card);
                let hp_loss = div_ceil(target.core.hp * percent, 100);
                let max_hp_loss = div_ceil(target.core.max_hp * percent, 100);
                if hp_loss > 0 {
                    self.modify_actor_hp(target_side, -hp_loss, false, false);
                }
                if max_hp_loss > 0 {
                    self.modify_actor_max_hp(target_side, -max_hp_loss);
                }
                let anima_loss = param1(card);
                if anima_loss > 0 {
                    self.reduce_anima_unchecked(target_side, anima_loss);
                }
                Some(false)
            }
            5_000_003 => {
                let value = param0(card);
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
                let value = param0(card);
                self.actor_mut(actor_side).music.chaotic_mind_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .chaotic_mind_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            6_000_005 => {
                self.modify_actor_max_hp(actor_side, param0(card));
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            6_000_007 => {
                if self.decision_tape.first().is_some_and(|value| *value < 0) {
                    self.decision_tape.remove(0);
                    return Some(false);
                }
                // Card_6000007.cs:82 同落纸云烟用 GetNextParam()：空队列 catch 返回
                // -1 → `nextParam != -1` 不成立，整张牌无效果（不回退默认状态）。
                if self.decision_tape.is_empty() {
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
                        param0(card),
                    );
                }
                Some(false)
            }
            6_000_008 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .turn
                    .next_card_anima_cost_reduction += param0(card);
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
                    // oracle 锚点：mirror-32299000 pa_e48d2948
                    // cp76（队列 5 个参数在 turn34 耗尽，原版 turn40 落纸云烟
                    // p1.def=0，引擎默认 0 → +26 防）。
                    return Some(false);
                }
                match self.consume_random_range_or_default_plain(actor_side, 0, 2, 0) {
                    0 => {
                        self.gain_defense(actor_side, param0(card));
                    }
                    1 => {
                        self.modify_actor_hp(actor_side, param1(card), false, false);
                    }
                    2 => {
                        self.gain_guard(actor_side, param2(card));
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
                    .next_card_activate_element += param0(card);
                Some(attacked)
            }
            8_000_006 => {
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    self.modify_actor_max_hp(actor_side, param1(card));
                }
                self.modify_actor_hp(actor_side, param0(card), false, false);
                Some(false)
            }
            8_000_009 => {
                self.actor_mut(actor_side).fate.exorcism += param0(card);
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    self.gain_defense(actor_side, param1(card));
                }
                Some(false)
            }
            9_000_005 => {
                let bonus = if self.actor(actor_side).core.anima > 0
                    || self.actor(opponent_side(actor_side)).core.anima > 0
                {
                    param0(card)
                } else {
                    0
                };
                self.gain_anima(actor_side, card.anima.unwrap_or(0).max(0) + bonus);
                Some(false)
            }
            9_000_009 => {
                self.apply_configured_defense(actor_side, card);
                let value = param0(card);
                if value > 0 {
                    self.actor_mut(actor_side).fate.leaf_shield_flower = value;
                }
                Some(false)
            }
            9_000_013 => {
                self.modify_actor_hp(actor_side, param0(card), false, false);
                let lotus = param1(card);
                if lotus > 0 {
                    self.actor_mut(actor_side).fate.ice_snow_lotus += lotus;
                }
                Some(false)
            }
            9_000_018 => {
                // Card_9000018.OnExecuted: the current attack resolves before
                // YeRenHua is installed for later attacks.
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let leaf_blade_flower = param0(card);
                if leaf_blade_flower > 0 {
                    self.actor_mut(actor_side).status.leaf_blade_flower = leaf_blade_flower;
                }
                Some(attacked)
            }
            9_000_020 => {
                let damage = param0(card);
                let guard_gain = param1(card);
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
                self.actor_mut(actor_side).fate.exorcism += param0(card);
                Some(false)
            }
            10_000_032 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, param0(card), false, false);
                Some(false)
            }
            10_000_043 => {
                let momentum = self.actor(actor_side).beng.momentum.max(0);
                if momentum > 0 {
                    self.modify_momentum(actor_side, -momentum);
                    self.gain_defense(actor_side, momentum * param0(card));
                    let damage = momentum * param1(card);
                    if damage > 0 {
                        self.apply_damage(actor_side, damage, false, false, false);
                    }
                }
                self.gain_agility(actor_side, param2(card));
                Some(false)
            }
            10_000_060 => {
                // 崩拳•双影
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_double_shadow += 1;
                Some(attacked)
            }
            11_000_008 => {
                self.actor_mut(actor_side).fate.fortune_seek_auspicious += param0(card);
                self.actor_mut(actor_side)
                    .fate
                    .fortune_seek_auspicious_damage = param1(card);
                Some(false)
            }
            11_000_011 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_star_chess_break(actor_side, param0(card));
                Some(false)
            }
            11_000_012 => {
                self.modify_actor_hp(actor_side, param0(card), false, false);
                self.actor_mut(actor_side).turn.jump_to_previous_card += param1(card);
                Some(false)
            }
            11_000_023 => {
                self.modify_blood_calamity(opponent_side(actor_side), param0(card));
                Some(false)
            }
            10_000_055 => {
                // 百毒不侵
                let hp_gain = param0(card);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.actor_mut(actor_side).status.poison_immunity += param1(card);
                Some(false)
            }
            1_000_018 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            1_000_023 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.reduce_anima_unchecked(opponent_side(actor_side), param0(card));
                }
                Some(attacked)
            }
            99_000_109 => {
                let hp_gain = param0(card);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                let recovery_before = self.actor(actor_side).status.recovery;
                self.actor_mut(actor_side).status.recovery += param1(card);
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
                self.actor_mut(actor_side).fate.fire_phoenix_revive_hp += param0(card);
                Some(false)
            }
            99_000_209 => {
                self.actor_mut(actor_side).status.lone_night_wolf += param0(card);
                Some(false)
            }
            99_000_214 => {
                self.apply_configured_anima(actor_side, card);
                Some(false)
            }
            _ => None,
        }
    }

    /// 原版 BattleCharacter.TriggerOpening 的 switch（跳牌循环、天星牵引/反击、天运避凶判定都走它）：
    /// 梦牌 348 / 378 / 4000079 在当前 build 是开局时扫描卡组的被动（BattleCharacter.cs:884、2511–2518），
    /// 不在 switch 里，被跳过/牵引时不会再次触发。
    pub(super) fn card_triggers_opening(base_id: i64) -> bool {
        Self::card_has_opening_effect(base_id) && !matches!(base_id, 348 | 378 | 4_000_079)
    }

    pub(super) fn card_triggers_opening_on_skip(base_id: i64) -> bool {
        Self::card_triggers_opening(base_id) && base_id != 56
    }

    pub(super) fn card_has_opening_effect(base_id: i64) -> bool {
        super::card_routing::CardLifecycle::for_base_id::<OBSERVE>(base_id)
            .contains(super::card_routing::CardLifecycle::BATTLE_START)
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
        self.dispatch_opening_effect(actor_side, card, slot_index, trigger_grid, &[]);
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
        // Original invokes ApplyDamage even at zero defense; 铁骨 can floor it to 1.
        self.apply_damage(actor_side, defense * triggered, false, false, false);
        self.actor_mut(actor_side).beng.triggered_beng_quan_fu_hu = 0;
    }
}
