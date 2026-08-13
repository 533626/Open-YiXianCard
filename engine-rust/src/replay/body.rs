use super::support::{normalized_base_id, opponent_side, other_param};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

/// Base ids handled by `apply_body_card_effect` (kept in sync with the
/// match arms below). `card_routing` uses this to pin the sect-routing
/// invariant: the body kernel must never claim another sect's ids.
#[cfg(test)]
pub(super) const BODY_HANDLED_IDS: &[i64] = &[
    219, 222, 377, 381, 382, 383, 392, 417, 9_000_027, 99_000_215, 10_000_001, 10_000_002,
    10_000_003, 10_000_004, 10_000_005, 10_000_006, 10_000_007, 10_000_008, 10_000_009, 10_000_010,
    10_000_011, 10_000_012, 10_000_013, 10_000_014, 10_000_015, 10_000_016, 10_000_017, 10_000_018,
    10_000_021, 10_000_022, 10_000_024, 10_000_025, 10_000_026, 10_000_027, 10_000_028, 10_000_031,
    10_000_036, 10_000_037, 10_000_038, 10_000_039, 10_000_040, 10_000_042, 10_000_044, 10_000_045,
    10_000_046, 10_000_047, 10_000_048, 10_000_050, 10_000_056, 10_000_057, 10_000_068, 10_000_099,
    10_000_100, 10_000_101, 11_000_005, 11_000_007, 11_000_014,
];

impl ReplayState {
    pub(super) fn apply_body_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> Option<bool> {
        let mut attacked = false;
        match normalized_base_id(card) {
            219 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let locked_stance = self.has_locked_li_stance(actor_side);
                if self.actor(actor_side).beng.quan_stance > 0 {
                    let divisor = other_param(card, 0);
                    if divisor > 0 {
                        let defense = self.actor(actor_side).core.physique / divisor;
                        self.gain_defense(actor_side, defense);
                    }
                    if !locked_stance {
                        self.actor_mut(actor_side).beng.quan_stance -= 1;
                        self.actor_mut(actor_side).beng.gun_stance += 1;
                    }
                } else if self.actor(actor_side).beng.gun_stance > 0 {
                    let divisor = other_param(card, 1);
                    let extra_attack = if divisor > 0 {
                        1 + self.actor(actor_side).core.physique / divisor
                            - card.attack.unwrap_or(0)
                    } else {
                        0
                    };
                    self.attack_by_config(actor_side, card, extra_attack, slot);
                    attacked = true;
                    if !locked_stance {
                        self.actor_mut(actor_side).beng.gun_stance -= 1;
                        self.actor_mut(actor_side).beng.quan_stance += 1;
                    }
                }
                // BattleCharacter.SwitchJiaShi (CardActionBase.cs:5570) runs
                // 335/349/429 fate bonuses on every stance-switch card,
                // locked or not; 429 (强攻架势) needs the post-switch stance.
                self.apply_fate_strategy_stance_switch(actor_side);
            }
            222 => {
                let agility = other_param(card, 0).max(0)
                    + if self.actor(actor_side).beng.quan_stance > 0 {
                        other_param(card, 1).max(0)
                    } else {
                        0
                    };
                self.gain_agility(actor_side, agility);
                if self.actor(actor_side).beng.gun_stance > 0 {
                    attacked |= self.attack_by_config(actor_side, card, 0, slot);
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
            73 => {
                let meditation_gain = other_param(card, 0).max(0);
                if meditation_gain > 0 {
                    self.add_actor_negative_status(actor_side, 367, meditation_gain);
                }
                for _ in 0..other_param(card, 1).max(0) {
                    if let Some(status) = self.consume_optional_negative_status_decision() {
                        self.add_actor_negative_status(actor_side, status, 1);
                    }
                }
                let cap = other_param(card, 2);
                let physique_gain = (card.physique.unwrap_or(0).max(0)
                    + self.known_negative_status_count(actor_side))
                .min(if cap > 0 { cap } else { i64::MAX });
                if physique_gain > 0 {
                    self.apply_physique_amount(actor_side, physique_gain);
                }
            }
            381 => {
                // 灵羽 Card_381.cs：先 Attack(dst, attack, attackCount)，
                // 攻击完成后才 ModifyAnima(anima) → ModifyTiPo(physique) →
                // ShenFa+otherParams[0]。顺序影响 F432 等按攻击时刻灵气
                // 读数的命运策略（oracle 锚点：mirror-32219000-human-01
                // 6f0470d9de1927d7/round-11）。
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_physique(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
            }
            382 => {
                let divisor = other_param(card, 0);
                if divisor > 0 {
                    let gain = self.actor(actor_side).core.physique / divisor;
                    self.gain_anima(actor_side, gain);
                }
                let bonus = self.actor(actor_side).core.anima;
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            417 => {
                // 万玄锻灵 Card_417.cs: ModifyAnima(anima) → 每有
                // otherParams[0] 体魄消耗 1 个战斗参数（GetNextParam，回放带
                // 中的负面状态 id）对其 -1 层并计 2 灵气（参数取尽返回 -1 时
                // 提前结束，num2 计已消耗参数数）→ ModifyTiPo(当前灵气)。
                // 引擎以 resolve_negative_status_decision 消费回放带
                // （与 2_000_004 同款）；10417/20417 数值由配置档位提供。
                self.apply_configured_anima(actor_side, card);
                let divisor = other_param(card, 0).max(1);
                let quota = self.actor(actor_side).core.physique.max(0) / divisor;
                let mut converted = 0;
                for ordinal in 0..quota {
                    let options = self.negative_status_types_present(actor_side);
                    let Some(status) = self
                        .resolve_negative_status_decision(actor_side, card.id, ordinal, &options)
                    else {
                        break;
                    };
                    converted += 1;
                    self.modify_actor_negative_status(actor_side, status, -1);
                }
                if converted > 0 {
                    self.gain_anima(actor_side, converted * 2);
                }
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.apply_physique_amount(actor_side, anima);
                }
            }
            9_000_027 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).formations.hard_branch_bamboo +=
                    other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .formations
                    .hard_branch_bamboo_defense_per_damage = other_param(card, 1).max(0);
            }
            377 => {
                // 原版 Card_377.cs:81-83 水灵激活时读持有者持久
                // ActualDamage(302)（残留 + 本卡）转水势。
                let bonus = (self.actor(actor_side).turn.anima_gain_count
                    + self.actor(actor_side).elements.water_momentum_gain_count)
                    * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
                if self.is_element_activated(actor_side, Element::Water) {
                    let divisor = other_param(card, 1).max(1);
                    let gain = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    if gain > 0 {
                        self.gain_water_momentum(actor_side, gain);
                    }
                }
            }
            75 => {
                self.apply_configured_physique(actor_side, card);
                let meditation_gain = other_param(card, 0).max(0);
                if meditation_gain > 0 {
                    self.add_actor_negative_status(actor_side, 367, meditation_gain);
                }
                self.gain_agility(actor_side, other_param(card, 1).max(0));
            }
            81 => {
                self.apply_configured_physique(actor_side, card);
                self.actor_mut(actor_side).fate.mystic_heart_enter_profound +=
                    other_param(card, 1).max(0);
            }
            82 => {
                // 万玄破魔掌 Card_82.cs: ModifyTiPo(physique) → 每有
                // otherParams[0] 体魄读 1 个战斗参数（GetNextParam，回放带
                // 中的负面状态 id），对其 -1 层并计 1 加攻；参数取尽返回
                // -1 时提前结束（num2 只计已消耗参数数，即使该状态当前为
                // 0 层也照计）。参数必须来自回放带，不带启发式兜底。
                // oracle 锚点：hf-32308000 a2ace584d6664933/round-16 cp25
                // （turn14 万玄破魔掌已耗尽带内 [100,103,100,103]，turn26
                // 再出时原版参数队列空 → 0 转化、加攻保持 12；引擎兜底
                // 按在场负面状态自选 +4 → 16）。
                self.apply_configured_physique(actor_side, card);
                let step = other_param(card, 0).max(1);
                let tries = self.actor(actor_side).core.physique / step;
                let mut removed = 0;
                for ordinal in 0..tries {
                    let options = self.negative_status_types_present(actor_side);
                    let Some(status) = self
                        .resolve_negative_status_decision(actor_side, card.id, ordinal, &options)
                    else {
                        break;
                    };
                    removed += 1;
                    self.modify_actor_negative_status(actor_side, status, -1);
                }
                if removed > 0 {
                    self.gain_attack_bonus(actor_side, removed);
                }
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            383 => {
                // Steam build 24217566: Card_383.OnExecuted gains configured
                // physique (ModifyTiPo) before the agility gain.
                self.apply_configured_physique(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                let damage = self.actor(actor_side).turn.agility.max(0);
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                    attacked = true;
                }
            }
            392 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.gain_agility(actor_side, other_param(card, 1).max(0));
                // Card_392.cs:80-82 adds PingXuYuFeng after this card's own
                // ShenFa gain.  Keep every resolved Sustain instance: the
                // shared ShenFa hook multiplies future gains by the stacked
                // BuffType.PingXuYuFeng value (BattleCharacter.cs:8659-8662).
                self.actor_mut(actor_side).turn.agility_gain_damage += 1;
            }
            74 => {
                let meditation_gain = other_param(card, 0).max(0);
                if meditation_gain > 0 {
                    self.add_actor_negative_status(actor_side, 367, meditation_gain);
                }
                self.actor_mut(actor_side).status.min_night += other_param(card, 1).max(0);
            }
            10_000_028 => {
                let physique_step = other_param(card, 0).max(1);
                let bonus_limit = other_param(card, 1).max(0);
                let bonus = (self.actor(actor_side).core.physique / physique_step).min(bonus_limit);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            10_000_099 => {
                self.apply_configured_physique(actor_side, card);
                let bonus = self.actor(actor_side).core.physique / other_param(card, 0).max(1);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            10_000_040 => {
                let base = other_param(card, 0).max(0);
                let amount = (base + self.negative_status_stack_count(actor_side))
                    .min(base * 2)
                    .max(0);
                if amount > 0 {
                    let target_side = opponent_side(actor_side);
                    self.modify_actor_hp(target_side, -amount, false, false);
                    self.modify_actor_hp(actor_side, amount, false, false);
                }
            }
            10_000_044 => {
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).beng.beng_mei_mindset += other_param(card, 1).max(0);
            }
            10_000_047 => {
                let divisor = other_param(card, 0).max(1);
                let segment_divisor = other_param(card, 1).max(1);
                let attack_bonus = self.actor(actor_side).turn.battle_physique_gain_count / divisor;
                let extra_count = self.actor(actor_side).core.physique / segment_divisor;
                let segment_count = card.attack_count.unwrap_or(1).max(0) + extra_count;
                let base_attack = card.attack.unwrap_or(0).max(0) + attack_bonus;
                for _ in 0..segment_count {
                    // Per-hit sampling happens inside apply_attack_with_options.
                    self.apply_attack(actor_side, base_attack, slot);
                    attacked = true;
                }
            }
            10_000_048 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
                if self.actor(actor_side).core.physique >= other_param(card, 0).max(0) {
                    self.gain_attack_bonus(actor_side, other_param(card, 1).max(0));
                }
                if self.actor(actor_side).core.physique >= other_param(card, 2).max(0) {
                    self.gain_agility(actor_side, other_param(card, 3).max(0));
                }
            }
            10_000_057 => {
                let amount = self.actor(actor_side).core.physique / other_param(card, 0).max(1);
                if amount > 0 {
                    self.gain_attack_bonus(actor_side, amount);
                    self.add_actor_negative_status(actor_side, 100, amount);
                }
            }
            10_000_050 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).turn.spirit_turtle_footwork +=
                    other_param(card, 1).max(0);
            }
            10_000_068 => {
                self.copy_negative_statuses_to_target(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).status.mystic_soul = 1;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            10_000_001 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_chuo += other_param(card, 0).max(0);
            }
            10_000_002 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
            }
            10_000_005 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            10_000_003 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_bounce += other_param(card, 0).max(0);
            }
            10_000_004 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
            }
            10_000_006 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
            }
            10_000_007 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.add_actor_negative_status(actor_side, 100, other_param(card, 0).max(0));
            }
            10_000_008 => {
                self.apply_configured_anima(actor_side, card);
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                // Card_10000008.cs applies WaiShang through ModifyBuffValue;
                // retain generic negative-status prevention such as BiXie.
                self.add_actor_negative_status(actor_side, 105, other_param(card, 0).max(0));
            }
            10_000_009 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if let Some(status) = self.consume_optional_negative_status_decision() {
                    self.modify_actor_negative_status(
                        actor_side,
                        status,
                        -other_param(card, 0).max(0),
                    );
                }
            }
            10_000_010 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
            }
            10_000_011 => {
                let bonus = self.actor(actor_side).beng.momentum * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
                if self.try_cost_anima(actor_side, 1) {
                    self.modify_momentum(actor_side, other_param(card, 1).max(0));
                }
            }
            10_000_012 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.try_cost_anima(actor_side, 1) {
                    self.apply_attack(actor_side, other_param(card, 0), slot);
                    attacked = true;
                }
            }
            10_000_014 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_chan += other_param(card, 0).max(0);
            }
            10_000_015 => {
                self.gain_active_effect_shatter_defense(1);
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_tu += other_param(card, 0).max(0);
            }
            10_000_016 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_physique(actor_side, card);
                if self.actor(actor_side).core.physique >= other_param(card, 0).max(0) {
                    self.gain_defense(actor_side, other_param(card, 1).max(0));
                }
            }
            10_000_017 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let step = other_param(card, 0).max(1);
                let cap = other_param(card, 1).max(0);
                self.gain_defense(
                    actor_side,
                    (self.actor(actor_side).core.physique / step).min(cap),
                );
            }
            10_000_018 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.add_actor_negative_status(actor_side, 102, other_param(card, 0).max(0));
            }
            10_000_021 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_momentum(actor_side, other_param(card, 0).max(0));
                if self.try_cost_anima(actor_side, 1) {
                    self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                }
            }
            10_000_022 => {
                let mut bonus = 0;
                if self.try_cost_anima(actor_side, 1) {
                    bonus = other_param(card, 0).max(0);
                    self.gain_active_effect_shatter_defense(1);
                }
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            10_000_013 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_han += other_param(card, 0).max(0);
            }
            10_000_045 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let agility = other_param(card, 0).max(0);
                self.gain_agility(actor_side, agility);
                self.actor_mut(actor_side).beng.beng_quan_flash_agility += agility;
            }
            10_000_046 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_startled_touch +=
                    other_param(card, 0).max(0);
            }
            10_000_024 => {
                self.transfer_selected_negative_statuses(actor_side, other_param(card, 0).max(0));
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).beng.beng_quan_meridian += other_param(card, 0).max(0);
            }
            10_000_025 => {
                let actor = self.actor_mut(actor_side);
                if actor.beng.momentum_multiplier < 2 {
                    actor.beng.momentum_multiplier = 2;
                }
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let actor = self.actor_mut(actor_side);
                actor.beng.momentum_multiplier = 0;
                actor.beng.beng_quan_cun_jin += other_param(card, 0).max(0);
            }
            10_000_026 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).beng.beng_tian_step += 1;
            }
            10_000_027 => {
                self.apply_configured_physique(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                self.actor_mut(actor_side).formations.forge_bone_attacks +=
                    other_param(card, 1).max(0);
                self.actor_mut(actor_side)
                    .formations
                    .forge_bone_attack_bonus = other_param(card, 2).max(0);
            }
            10_000_100 => {
                // 极•锻骨 Card_10000100.cs: ModifyTiPo(physique) →
                // ModifyBuffValue(DuanGu, otherParams[0]) → ModifyBuffValue(
                // ShenFa, otherParams[2])。DuanGu 消耗在共享攻击结算
                // （BattleCharacter.cs:11499-11503：每次攻击消耗 1 层、加
                // cardConfigDict[10000027].otherParams[2] 攻并 +1 体魄），引擎由
                // formations.forge_bone_attacks / forge_bone_attack_bonus 承担
                // （10_000_027 同款）。原版消耗固定读 10000027 的配置而非发牌方，
                // 此处按同一来源补齐 attack bonus，避免只打 10000100（未打
                // 10000027）时攻击加成为 0。10010100/10020100 数值由配置档位提供。
                self.apply_configured_physique(actor_side, card);
                self.actor_mut(actor_side).formations.forge_bone_attacks +=
                    other_param(card, 0).max(0);
                if let Some(forge_base) = super::original_config::original_card_definition(10000027)
                {
                    self.actor_mut(actor_side)
                        .formations
                        .forge_bone_attack_bonus =
                        forge_base.other_params.get(2).copied().unwrap_or(0).max(0);
                }
                self.gain_agility(actor_side, other_param(card, 2).max(0));
            }
            10_000_101 => {
                // 极•夜鬼啸 Card_10000101.cs: ModifyBuffValue(
                // BenLunWuShiFangYu, 1) → Attack(attack, attackCount) → 自身与
                // 对方 ModifyBuffValue(XuRuo, otherParams[0])（攻击之后施加）。
                // BenLunWuShiFangYu 是「本轮无视防御」非消耗 buff（ApplyDamage
                // BattleCharacter.cs:10747 只查不扣；OnAfterExecuted 统一移除），
                // 引擎用 turn.current_turn_ignore_defense 表达，作用域模式同
                // swords.rs 卡 8 云剑•猫爪。10010101/10020101 数值由配置档位提供。
                let previous = self.actor(actor_side).turn.current_turn_ignore_defense;
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous + 1;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous;
                let weakness = other_param(card, 0).max(0);
                if weakness > 0 {
                    self.add_actor_negative_status(actor_side, 101, weakness);
                    self.add_actor_negative_status(opponent_side(actor_side), 101, weakness);
                }
            }
            10_000_031 => {
                self.apply_configured_physique(actor_side, card);
                self.add_actor_negative_status(actor_side, 104, other_param(card, 0).max(0));
                let defense = card.defense.unwrap_or(0).max(0)
                    + self.known_negative_status_count(actor_side) * other_param(card, 1).max(0);
                self.gain_defense(actor_side, defense);
            }
            10_000_036 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let inherited = other_param(card, 0).max(0);
                self.actor_mut(actor_side).beng.beng_quan_chuo += inherited;
                self.actor_mut(actor_side).beng.beng_quan_defense += inherited;
            }
            10_000_037 => {
                self.apply_configured_physique(actor_side, card);
                let heal = self.actor(actor_side).core.max_hp * other_param(card, 0).max(0) / 100;
                self.modify_actor_hp(actor_side, heal, false, false);
            }
            10_000_038 => {
                self.modify_actor_hp(actor_side, other_param(card, 2).max(0), false, false);
                let divisor = other_param(card, 0).max(1);
                let cap = other_param(card, 1).max(0);
                let gain = (self.actor(actor_side).core.physique / divisor).min(cap);
                self.gain_agility(actor_side, gain);
            }
            10_000_039 => {
                self.add_actor_negative_status(actor_side, 103, other_param(card, 0).max(0));
                self.gain_agility(actor_side, other_param(card, 1).max(0));
            }
            10_000_056 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).status.drunken_leisure += other_param(card, 1).max(0);
            }
            10_000_042 => {
                self.apply_configured_anima(actor_side, card);
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.modify_momentum(actor_side, anima);
                }
                self.gain_agility(actor_side, other_param(card, 0).max(0));
            }
            11_000_005 => {
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
            }
            11_000_007 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.fortune_avoid_misfortune += other_param(card, 0).max(0);
                actor.formations.fortune_avoid_misfortune_defense = other_param(card, 1).max(0);
                actor.formations.fortune_avoid_misfortune_healing = other_param(card, 2).max(0);
            }
            11_000_014 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).fate.all_things_inauspicious +=
                    other_param(card, 0).max(0);
            }
            99_000_215 => {
                self.add_actor_negative_status(actor_side, 100, 1);
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    let amount = other_param(card, 0).max(0)
                        + self.actor(actor_side).status.internal_injury.max(0);
                    self.add_actor_negative_status(opponent_side(actor_side), 100, amount);
                }
            }
            _ => return None,
        }
        Some(attacked)
    }

    pub(super) fn has_locked_li_stance(&self, actor_side: PlayerSide) -> bool {
        let fate_strategies = &self.actor(actor_side).identity.fate_strategies;
        fate_strategies.contains(&335)
    }

    pub(super) fn apply_fate_strategy_stance_switch(&mut self, actor_side: PlayerSide) -> bool {
        let mut handled = false;
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&335)
        {
            self.modify_momentum_limit(actor_side, 1);
            if self.original_build_has_capability(
                super::original_build_profile::OriginalBuildCapability::Fate335GrantsMomentum,
            ) {
                self.modify_momentum(actor_side, 1);
            }
            self.gain_defense(actor_side, 3);
            handled = true;
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&349)
        {
            self.apply_damage(actor_side, 3, false, false, false);
            handled = true;
        }
        // 强攻架势 FateStrategy 429（otherParams=[1,1]）：切换架势后按最终架势
        // 发奖——变为拳 → +1 气势，变为棍 → +1 加攻。BattleCharacter.SwitchJiaShi
        // (CardActionBase.cs:5604-5614) 在切换完成后按 QuanJiaShi buff 判断。
        // 349 在原版只追加伤害，架势仍由各卡牌正常切换；只有 335
        // 将切换改写为棍架势。429 的最终架势判断因此读取实际切换结果。
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&429)
        {
            if self.actor(actor_side).beng.quan_stance > 0 {
                self.modify_momentum(actor_side, 1);
            } else {
                self.gain_attack_bonus(actor_side, 1);
            }
            handled = true;
        }
        handled
    }
}
