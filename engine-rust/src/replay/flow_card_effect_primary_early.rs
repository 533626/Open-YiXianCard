use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};
use crate::replay::{
    support::{card_rarity, opponent_side, other_param},
    Element, BASIC_ATTACK_DAMAGE, BASIC_ATTACK_ID,
};

impl ReplayState {
    pub(super) fn apply_card_effect_primary_early(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            BASIC_ATTACK_ID => Some(self.apply_basic_attack_effect(actor_side, card, slot)),
            11 => {
                let damage = other_param(card, 0).max(0);
                if damage > 0 {
                    self.modify_target_hp(actor_side, -damage);
                }
                self.actor_mut(opponent_side(actor_side)).status.cannot_act += 1;
                Some(damage > 0)
            }
            12 => {
                self.apply_configured_anima(actor_side, card);
                if card_rarity(card) == 0 {
                    let _ = self.check_rear_move(actor_side, was_used_before_effect);
                } else {
                    // Card_12: upgraded ranks pay HP before CheckHouZhao.
                    // The check can grant max HP and arm 雁栖 healing, so moving
                    // it ahead of the loss wastes that recovery at full HP.
                    self.modify_actor_hp(actor_side, -other_param(card, 1).max(0), false, false);
                    if self.check_rear_move(actor_side, was_used_before_effect) {
                        self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                    }
                }
                Some(false)
            }
            17 => {
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.actor_mut(actor_side).fate.tide += other_param(card, 1).max(0);
                Some(false)
            }
            428 => {
                // 极•水灵春雨 Card_428.cs: ModifyBuffValue(ShuiShi, otherParams[0])
                // → ModifyBuffValue(HaiChao, otherParams[1])。
                // 24589371 起行为类型变更（BUILD_24589371_RULE_DELTA §3-a）：
                // 旧「生命及上限+otherParams[0]」（与卡 17 同构）→ 新「水势+
                // otherParams[0]、海潮+otherParams[1]」。卡 17 保持旧行为
                // （Card_17.cs: ModifyMaxHp/ModifyHp + HaiChao）。10428/20428
                // 数值由配置档位提供（[4,1]/[6,1]）。
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).fate.tide += other_param(card, 1).max(0);
                Some(false)
            }
            10 => {
                let bonus = if self.actor(actor_side).astrology.star_power > 0 {
                    self.modify_star_power(actor_side, -1);
                    other_param(card, 0).max(0)
                } else {
                    0
                };
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            // Early five-elements cards used heavily by replay slice candidates.
            19 => {
                let wuji_bonus = if self.actor(actor_side).identity.talents.contains(&30_096) {
                    1
                } else {
                    0
                };
                let frenzy_heart_bonus =
                    if self.actor(actor_side).identity.talents.contains(&10_096) {
                        3
                    } else {
                        0
                    };
                let mut attacked =
                    self.attack_by_config(actor_side, card, frenzy_heart_bonus, slot);
                self.apply_configured_defense(actor_side, card);
                if self.actor(actor_side).identity.talents.contains(&10_094) {
                    // TalentConfig 10094 otherParams[0]; Steam build 24217566: 5 -> 6.
                    self.apply_attack(actor_side, 6 + wuji_bonus, slot);
                    attacked = true;
                }
                if self.actor(actor_side).identity.talents.contains(&20_094) {
                    self.gain_attack_bonus(actor_side, 1 + wuji_bonus);
                }
                if self.actor(actor_side).identity.talents.contains(&30_094) {
                    self.modify_sword_intent(actor_side, 4 + wuji_bonus);
                }
                if self.actor(actor_side).identity.talents.contains(&10_095) {
                    self.gain_anima(actor_side, 2 + wuji_bonus);
                }
                if self.actor(actor_side).identity.talents.contains(&20_095)
                    && self.actor(actor_side).core.anima > 0
                {
                    let gain = self.actor(actor_side).core.anima * (3 + wuji_bonus);
                    if gain > 0 {
                        self.gain_defense(actor_side, gain);
                    }
                }
                let lingwei_cost = 1 + wuji_bonus;
                if self.actor(actor_side).identity.talents.contains(&30_095)
                    && self.actor(actor_side).core.anima >= lingwei_cost
                {
                    self.spend_anima_unchecked(actor_side, lingwei_cost);
                    for _ in 0..(2 + wuji_bonus) {
                        self.apply_attack(actor_side, 4 + wuji_bonus, slot);
                        attacked = true;
                    }
                }
                if self.actor(actor_side).identity.talents.contains(&20_096) {
                    self.actor_mut(actor_side).sword.cloud_sword_heart += 1;
                }
                Some(attacked)
            }
            2_000_007 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_physique(actor_side, card);
                let heal = other_param(card, 0).max(0)
                    + self.actor(actor_side).core.anima.max(0) * other_param(card, 1).max(0);
                if heal > 0 {
                    self.modify_actor_hp(actor_side, heal, false, false);
                }
                Some(false)
            }
            2_000_001 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                Some(false)
            }
            2_000_004 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                let count = other_param(card, 0).max(0);
                let amount = other_param(card, 1).max(0);
                for ordinal in 0..count {
                    let options = self.negative_status_types_present(actor_side);
                    let status = self
                        .resolve_negative_status_decision(actor_side, card.id, ordinal, &options);
                    if let Some(status) = status {
                        if amount > 0 {
                            self.modify_actor_negative_status(actor_side, status, -amount);
                        }
                    }
                }
                Some(false)
            }
            2_000_008 => {
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            11_000_021 => {
                let hp_gain = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, hp_gain);
                if hp_gain > 0 {
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                let actor = self.actor_mut(actor_side);
                // Card_11000021.cs:82 用 ModifyBuffValue(DongZhuJiXian, ...)
                // 累加次数：重复打出/多张命运轮回叠加跳过次数，不是覆盖。
                actor.fate.fate_cycle += other_param(card, 1).max(0);
                actor.fate.fate_cycle_slots[0] = other_param(card, 2).max(0);
                actor.fate.fate_cycle_slots[1] = other_param(card, 3).max(0);
                Some(false)
            }
            22 => {
                self.apply_configured_anima(actor_side, card);
                self.add_following_star_slots(actor_side, slot, 1);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                let hp_gain = other_param(card, 1).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                Some(false)
            }
            4_000_022 => {
                let mut attacked = false;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let target_side = opponent_side(actor_side);
                if self.known_negative_status_count(target_side) > 0 {
                    self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                }
                Some(attacked)
            }
            4_000_040 => {
                self.apply_configured_anima(actor_side, card);
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                if hexagram > 0 {
                    self.gain_defense(actor_side, other_param(card, 0).max(0) * hexagram);
                    self.modify_actor_hp(
                        actor_side,
                        other_param(card, 1).max(0) * hexagram,
                        false,
                        false,
                    );
                }
                Some(false)
            }
            4_000_043 => {
                let mut attacked = false;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.apply_attack(actor_side, other_param(card, 1).max(0), slot);
                    attacked = true;
                }
                Some(attacked)
            }
            4_000_101 => {
                // 极•螳螂捕蝉 Card_4000101.cs: Attack(attack, attackCount) →
                // ModifyBuffValue(JiaGong, otherParams[0]) → CheckHouZhao 成功时
                // 再 Attack(attack, attackCount)（后招与主攻击同数值同段数，
                // 独立于主攻击是否命中）。4010101/4020101 数值由配置档位提供。
                let mut attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    attacked |= self.attack_by_config(actor_side, card, 0, slot);
                }
                Some(attacked)
            }
            214 => {
                self.apply_configured_anima(actor_side, card);
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 101, other_param(card, 0).max(0));
                self.add_actor_negative_status(target_side, 102, other_param(card, 1).max(0));
                let target_yin_fu = other_param(card, 2).max(0);
                self.actor_mut(target_side).status.yin_fu = target_yin_fu;
                Some(false)
            }
            38 | 426 => {
                // 锟铻金环 Card_38.cs 与极•锟铻金环 Card_426.cs 同构：
                // JiHuoTuLing+1、JiHuoJinLing+1、KunWuJinHuan+=otherParams[0]。
                // 426 的持续效果（每次加防/加锋锐时多加层数）由共享结算中的
                // metal_ring hook 承担（resources.rs gain_defense /
                // gain_sharpness，对应 BattleCharacter.cs:8572-8574 / 10088-10090）。
                self.activate_element(actor_side, Element::Earth);
                self.activate_element(actor_side, Element::Metal);
                self.actor_mut(actor_side).sword.metal_ring += other_param(card, 0).max(0);
                Some(false)
            }
            415 => {
                // 疯魔架势 Card_415.cs: ModifyBuffValue(Min, otherParams[0])。
                // 被动（每获得/失去 1 层负面状态加 1 体魄）在共享
                // add/remove_actor_negative_status hook 中承担，见 combat.rs
                // apply_feng_mo_stance_physique。Min 的扣血副作用由
                // apply_meditation_hp_delta 承担（BattleCharacter.cs:8715-8730）。
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    self.add_actor_negative_status(actor_side, 367, amount);
                }
                Some(false)
            }
            425 => {
                // 极•飞鸿踏雪 Card_425.cs: ModifyAnima(anima)、
                // ModifyBuffValue(ShenFa, otherParams[0])、后招成功时
                // ModifyHp(otherParams[1])。与卡 12 不同：无 rarity 分支，
                // 后招固定回血 otherParams[1]（10425/20425 数值由配置档位提供）。
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                }
                Some(false)
            }
            418 => {
                // 极•盛气凌人 Card_418.cs: ModifyBuffValue(ShenFa, otherParams[0])
                // → ModifyBuffValue(ShengQiLingRen, otherParams[1])（气血消耗
                // 2 由 hpCost 配置走通用费用结算，flow.rs pay_card_hp_cost）。
                // 持续效果「每超过上限 1 气势或失去 1 气势 → 对方
                // otherParams[1] 伤害」由既有 momentum hook 承担：
                // resources.rs modify_momentum_inner（失去）与
                // modify_momentum_limit（超限）调用 apply_sheng_qi_ling_ren_damage
                // （对应 BattleCharacter.cs:8818-8820 / 9079-9081）。与卡 70
                // 气势如虹同 buff（fate.sheng_qi_ling_ren）。
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).fate.sheng_qi_ling_ren += other_param(card, 1).max(0);
                Some(false)
            }
            423 => {
                // 水灵•劲浪：无 Card_423 专属类（§3-b），仅打印字段
                // 攻击 9 / 灵气+1（10423/20423 数值由配置档位提供）。
                // 水灵激活（攻击时每耗 1 锋锐 → 水势+1 且生命及上限+1）由
                // 共享攻击结算 combat_core.rs 承担（BattleCharacter.cs:10819）。
                self.apply_configured_anima(actor_side, card);
                Some(self.attack_by_config(actor_side, card, 0, slot))
            }
            429 => {
                // 阴符绝阵 Card_429.cs: ModifyBuffValue(XuRuo, otherParams[0])
                // → ModifyBuffValue(YinFuJueZhen, otherParams[1])（施法动画）。
                // 顺序关键：虚弱先于阴符绝阵 buff，首轮 2 层虚弱不触发反伤
                // （反伤 hook 要求 HasBuff(YinFuJueZhen) 成立）。持续反伤
                // （对方每获得 1 层负面状态 → 2×层数反伤，豁免「冥」）由共享
                // 结算 combat.rs add_actor_negative_status 承担
                // （BattleCharacter.cs:8644-8648，BuffConfig 759 YinFuJueZhen）。
                let target_side = opponent_side(actor_side);
                self.add_actor_negative_status(target_side, 101, other_param(card, 0).max(0));
                self.actor_mut(target_side).status.yin_fu_jue_zhen += other_param(card, 1).max(0);
                Some(false)
            }
            7_000_107 => {
                // 极•五行灵击 Card_7000107.cs:
                // Attack(dst, attack + anima * otherParams[0], attackCount)。
                // 灵气减免（卡组每有 1 种不同五行少耗 1 灵气）由统一费用结算
                // support.rs effective_anima_cost 承担（CardActionBase.cs:5026，
                // 与 7000095 同分支）。7010107/7020107 数值由配置档位提供。
                let bonus = self.actor(actor_side).core.anima.max(0) * other_param(card, 0).max(0);
                Some(self.attack_by_config(actor_side, card, bonus, slot))
            }
            403 => {
                // 云剑•猫影 Card_403.cs（build 24610558，synthetic batch-027
                // pair 1）：otherParams[0]>0 时生命及上限 + otherParams[0]
                // （先上限后生命），随后 YunJianMaoYing(757) += otherParams[1]
                // （持续：每次使用云剑后追加该层数攻击，见 cards_ronghui.rs
                // apply_ronghui_after_card_effect）。10403/20403 数值由配置
                // 档位提供（[6,4]/[10,5]）。
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.actor_mut(actor_side).sword.yun_jian_mao_ying += other_param(card, 1).max(0);
                Some(false)
            }
            407 => {
                // 雷闪二度 Card_407.cs（build 24610558，synthetic batch-027
                // pair 2）：卦象 + cardConfig.guaXiang、LeiShanErDu(763) + 1。
                // 灵气 -1 由 anima 配置走统一费用结算（flow.rs 出牌事务）；
                // 「下一张名字含『雷』的牌连续生效 2 次」由重复源
                // RepeatSource::LeiShanErDu 承担（flow.rs）。
                // 10407/20407 卦象由配置档位提供（4/6）。
                self.gain_hexagram(actor_side, card.hexagram.unwrap_or(0).max(0));
                self.actor_mut(actor_side).astrology.lei_shan_er_du += 1;
                Some(false)
            }
            422 => {
                // 紫芒星爆 Card_422.cs（build 24610558，synthetic batch-027
                // pairs 3-6）：星力 + otherParams[0]、ZiMangXingBao(773) + 1
                // （无攻击）。持续（共享结算）：耗灵气或卦象时优先用星力
                // 代替，失去星力时获得等量加攻——flow.rs CheckAnima 星力代替
                // 分支、resources.rs modify_star_power_inner 加攻转换、
                // combat_core_status.rs 卦象消耗先扣星力三处触发点均以
                // buff 773 判定。10422/20422 星力由配置档位提供（2/3）。
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).astrology.zi_mang_xing_bao += 1;
                Some(false)
            }
            7_000_108 => {
                // 极•金灵阵 Card_7000108.cs（build 24610558，synthetic
                // batch-027 pair 7）：JiHuoJinLing(237) + 1、JinLingZhen(242)
                // += otherParams[0]；[再次行动] 由 actionAgain 配置走统一
                // action-again 结算（action_again.rs）。持续「每次使用
                // 『金灵』牌时加 otherParams[0] 锋锐」由既有五行相生共享
                // hook 承担（elements.rs apply_selected_card_hooks_after_start
                // 金灵阵分支，BattleCharacter.cs:2741-2743 OnBeforeExecuted）。
                // 7010108/7020108 数值由配置档位提供（2/3）。
                self.activate_element(actor_side, Element::Metal);
                self.actor_mut(actor_side).elements.metal_formation += other_param(card, 0).max(0);
                Some(false)
            }
            221 => {
                self.apply_configured_defense(actor_side, card);
                let before = self.actor(actor_side).fate.dismantle_move;
                self.actor_mut(actor_side).fate.dismantle_move += 1;
                let after = self.actor(actor_side).fate.dismantle_move;
                self.record_counter_transition(
                    actor_side,
                    "仙命",
                    "dismantleMove",
                    "拆招",
                    before,
                    after,
                );
                self.actor_mut(actor_side).fate.dismantle_move_reflect =
                    other_param(card, 1).max(0);
                Some(false)
            }
            // Early talisman cards.
            3_000_001 => {
                let damage = self.consume_random_range(
                    actor_side,
                    other_param(card, 0),
                    other_param(card, 1),
                );
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                }
                Some(false)
            }
            3_000_003 => {
                let damage = other_param(card, 0);
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                }
                Some(false)
            }
            3_000_004 => {
                self.apply_configured_anima(actor_side, card);
                let damage = other_param(card, 0);
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                }
                Some(false)
            }
            3_000_005 => {
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                Some(false)
            }
            3_000_006 => {
                self.apply_configured_anima(actor_side, card);
                let amount = other_param(card, 0).max(0);
                self.reduce_all_actor_negative_statuses(actor_side, amount);
                Some(false)
            }
            3_000_007 => {
                self.apply_configured_anima(actor_side, card);
                self.reduce_anima_unchecked(opponent_side(actor_side), other_param(card, 0).max(0));
                Some(false)
            }
            3_000_008 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            3_000_009 => {
                for _ in 0..other_param(card, 0).max(0) {
                    self.modify_actor_hp(
                        opponent_side(actor_side),
                        -other_param(card, 1).max(0),
                        false,
                        false,
                    );
                }
                Some(false)
            }
            3_000_012 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    101,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            5_000_001 => {
                let bonus = if self.actor(actor_side).music.music_cards_played > 0 {
                    1
                } else {
                    0
                };
                self.actor_mut(actor_side).turn.ignore_defense_attacks += 1;
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(attacked)
            }
            5_000_002 => {
                let defense = other_param(card, 0).max(0);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                    self.gain_defense(opponent_side(actor_side), defense);
                }
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_006 => {
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_015 => {
                let value = other_param(card, 0).max(1);
                self.actor_mut(actor_side).music.immortal_binding_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .immortal_binding_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            6_000_003 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_anima(actor_side, other_param(card, 0).max(0));
                Some(attacked)
            }
            6_000_004 => {
                let defense = self.consume_random_range(
                    actor_side,
                    card.defense.unwrap_or(0),
                    card.random_defense.unwrap_or(card.defense.unwrap_or(0)),
                );
                self.gain_defense(actor_side, defense);
                Some(false)
            }
            6 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let injury = self.consume_random_range(
                    actor_side,
                    other_param(card, 0),
                    other_param(card, 1),
                );
                if injury > 0 {
                    self.add_actor_negative_status(opponent_side(actor_side), 100, injury);
                }
                Some(attacked)
            }
            7 => {
                if let Some(hexagram) = card.hexagram.filter(|value| *value > 0) {
                    self.gain_hexagram(actor_side, hexagram);
                }
                let target_side = opponent_side(actor_side);
                let damage = other_param(card, 0).max(0);
                if damage > 0 {
                    self.modify_actor_hp(target_side, -damage, false, false);
                }
                self.reduce_anima_unchecked(target_side, other_param(card, 1).max(0));
                Some(false)
            }
            3_000_011 => {
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    102,
                    other_param(card, 0).max(0),
                );
                Some(false)
            }
            5_000_004 => {
                let amount = other_param(card, 0).max(0);
                self.gain_anima(actor_side, amount);
                self.gain_anima(opponent_side(actor_side), amount);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_005 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.illusory_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .illusory_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_007 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.heartbreak_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .heartbreak_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_008 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.wild_dance_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .wild_dance_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_010 => {
                let value = other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.rejuvenation_tune += value;
                self.actor_mut(opponent_side(actor_side))
                    .music
                    .rejuvenation_tune += value;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_011 => {
                let amount = other_param(card, 0).max(0);
                self.spend_anima_unchecked(actor_side, amount);
                self.spend_anima_unchecked(opponent_side(actor_side), amount);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            5_000_014 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            57 => {
                let gain = card.defense.unwrap_or(0).max(0)
                    + self.actor(actor_side).turn.used_card_count * other_param(card, 0).max(0);
                self.gain_defense(actor_side, gain);
                Some(false)
            }
            _ => None,
        }
    }

    fn apply_basic_attack_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        let xiaoyao_bonus = self.actor(actor_side).music.xiaoyao_tune.max(0);
        let return_to_simplicity = self.apply_return_to_simplicity_basic_attack(actor_side);
        let attack =
            card.attack.unwrap_or(BASIC_ATTACK_DAMAGE) + xiaoyao_bonus + return_to_simplicity;
        let drunken_leisure = self.actor(actor_side).status.drunken_leisure.max(0);
        if drunken_leisure > 0 {
            self.actor_mut(actor_side).status.drunken_leisure = 0;
        }
        let attack_count = 1 + drunken_leisure;
        for _ in 0..attack_count {
            if attack > 0 {
                self.apply_attack(actor_side, attack, slot);
            }
        }
        attack > 0 && attack_count > 0
    }
}
