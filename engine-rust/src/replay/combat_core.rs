use super::support::div_ceil;
use super::support::opponent_side;
use super::{ReplayAttackSegment, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

/// Cached one-shot read of the per-attack replay trace toggle so the env
/// lookup does not repeat on every attack in solver/GA batches.
fn attack_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("YIXIAN_RUST_REPLAY_TRACE_ATTACK").is_some())
}

impl ReplayState {
    pub(super) fn attack_by_config(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        bonus: i64,
        slot: usize,
    ) -> bool {
        let xiaoyao_bonus = if card.id == super::BASIC_ATTACK_ID {
            self.actor(actor_side).music.xiaoyao_tune.max(0)
        } else {
            0
        };
        let attack = card.attack.unwrap_or(if card.id == super::BASIC_ATTACK_ID {
            super::BASIC_ATTACK_DAMAGE
        } else {
            0
        }) + bonus
            + xiaoyao_bonus;
        let attack_count = card
            .attack_count
            .unwrap_or(if attack > 0 { 1 } else { 0 })
            .max(0);
        for _ in 0..attack_count {
            if attack > 0 {
                // Per-hit sampling happens inside apply_attack_with_options.
                self.apply_attack(actor_side, attack, slot);
            }
        }
        attack_count > 0 && attack > 0
    }

    pub(super) fn apply_attack(
        &mut self,
        actor_side: PlayerSide,
        base_attack: i64,
        slot: usize,
    ) -> i64 {
        self.apply_attack_with_options(actor_side, base_attack, slot, false, false, 0, None)
    }

    #[allow(clippy::too_many_arguments)]
    pub(super) fn apply_attack_with_options(
        &mut self,
        actor_side: PlayerSide,
        base_attack: i64,
        slot: usize,
        ignore_sword_energy: bool,
        force_shatter_defense: bool,
        return_sharpness_percent: i64,
        segment_source: Option<&str>,
    ) -> i64 {
        let resolved_segment_source = match segment_source {
            Some(source) => Some(source.to_string()),
            None if !self.active_effect_segment_source().is_empty() => {
                Some(self.active_effect_segment_source().to_string())
            }
            None => None,
        };
        let segment_source = resolved_segment_source.as_deref();
        let target_side = opponent_side(actor_side);
        // Per-hit sampling: record the target's hp/def before this hit lands,
        // then again after the whole attack settles, so the browser can show
        // per-hit damage the way the original client's TmpFloatingText does.
        // Only Detailed mode and only inside a card effect (event_index known);
        // observation never changes the battle. This is the single choke point
        // covering every attack path: attack_by_config loops, 百杀 probability
        // segments, 五雷 repeat, 狂剑 follow-ups, etc.
        let segment_actor_hp_before = self.actor(actor_side).core.hp;
        let segment_hp_before = self.actor(target_side).core.hp;
        let segment_def_before = self.actor(target_side).core.defense;
        let reverse_gu_flat_bonus = self.apply_reverse_gu_before_attack(actor_side);
        let earth_fiend_active_before_attack =
            self.actor(actor_side).ronghui.earth_fiend_defense > 0;
        self.apply_delayed_attack_status_triggers(actor_side, target_side);
        let fire_blade_wound = self.actor(actor_side).identity.talents.contains(&67);
        if fire_blade_wound {
            self.modify_actor_max_hp(target_side, -1);
        }
        let actor = self.actor(actor_side);
        let sword_intent = if ignore_sword_energy {
            0
        } else {
            actor.sword.sword_intent
        };
        let attack_bonus = actor.core.attack_bonus;
        let ignore_defense =
            actor.turn.ignore_defense_attacks > 0 || actor.turn.current_turn_ignore_defense > 0;
        let in_star_slot = actor.astrology.star_slots.contains(&slot);
        let star_power = actor.astrology.star_power;
        let staff_stance_bonus =
            if actor.identity.talents.contains(&206) && actor.beng.gun_stance > 0 {
                1
            } else {
                0
            };
        let actor_sharpness = actor.sword.sharpness;
        let attack_reduction = if actor.status.mystic_soul > 0 {
            0
        } else {
            actor.status.attack_reduction
        };
        let shatter_bonus = if actor.formations.shatter_formation > 0 {
            actor.formations.shatter_formation_bonus
        } else {
            0
        };
        let forge_bone_bonus = if actor.formations.forge_bone_attacks > 0 {
            actor.formations.forge_bone_attack_bonus
        } else {
            0
        };
        let is_card_segment = segment_source.is_some_and(|source| source.starts_with("card:"));
        // 冥夜（卡 74 持续，BattleCharacter.cs:11383-11407）与崩拳戳
        // （11378-11381）都带 !HasBuff(AfterCardAciton) 门控：周天剑阵/
        // 察体等 OnAfterExecuted 追击（原版 AfterCardAciton 窗口内）不享受
        // 这两项加成，只吃加攻/气势等无门控平值。oracle 锚点：
        // hf-latest-32308000-16f9c778 d2ac90a8094fe7e4/round-09 cp7 p1.hp
        // 57（原版 11+6冥夜+5追击=22 两段；引擎 17+11=28：追击段重复吃
        // 冥夜 6）、dba3e283b85c91e7/round-08 cp8 p2.hp 45（原版 12+3加攻
        // +8追击=23；引擎 15+15=30：追击段重复吃降龙 +7 戳加成）。
        let min_night_bonus = if is_card_segment
            && actor.status.min_night > 0
            && self.active_effect_is_beng_quan()
            && !self.active_effect_after_action()
        {
            self.negative_status_stack_count(actor_side)
                .min(actor.status.min_night)
        } else {
            0
        };
        let (beng_quan_chuo_bonus, mark_consumed_beng_quan_chuo) = if is_card_segment
            && self.active_effect_is_beng_quan()
            && !self.active_effect_after_action()
        {
            let chuo = actor.beng.beng_quan_chuo.max(0);
            let mark = chuo > 0 && actor.beng.consumed_beng_quan_chuo != chuo;
            (chuo, mark)
        } else {
            (0, false)
        };
        // 孤夜狼（卡 99000209 持续）：BattleCharacter.cs:11492-11494 ——
        // HasBuff(GuYeLang) && hp×2 < maxHp 即加攻，无出牌/段源门控
        // （猛攻之姿 talent 173 追击等非牌体攻击同样享受）。oracle 锚点：
        // hf-latest-32308000-16f9c778 b0311c048df1dfae/round-13 cp7 p2.hp
        // 71（原版 14 = 3 猛攻 + 4 加攻 + 3 锻骨 + 4 孤夜狼；引擎 75 =
        // 漏孤夜狼 4）。
        let lone_night_wolf_bonus =
            if actor.status.lone_night_wolf > 0 && actor.core.hp * 2 < actor.core.max_hp {
                actor.status.lone_night_wolf
            } else {
                0
            };
        let spirit_claw_attack_bonus = if actor.identity.fate_strategies.contains(&152)
            && self.active_effect_has_anima_desc()
        {
            3
        } else {
            0
        };
        // 月魂爪 FateStrategy 424（otherParams=[3]，断玄宗 4000003）：
        // BattleCharacter.CalculateAttack 11413-11421 —— 使用卡组第 8 格
        // （gridNumber == 7，0-based）的牌攻击且非后招阶段时，
        // 自身每有 otherParams[0]=3 层负面状态就多 1 攻（每段攻击各自
        // 读取 GetDebuffCount()，整数除法；oracle 锚点：
        // mirror-32219000-human-01 2995be139404d0ed/round-10 cp14 迎风掌
        // 26 vs 22、fcb156fa3df7dbe1/round-13 cp13 百杀破境掌 32 vs 16）。
        let month_claw_attack_bonus = if is_card_segment
            && actor.identity.fate_strategies.contains(&424)
            && slot == 7
            && !self.active_effect_after_action()
        {
            self.negative_status_stack_count(actor_side) / 3
        } else {
            0
        };
        // BattleCharacter.CalculateAttack 11422-11427 (FateStrategy 432):
        // during the main effect of a 崩拳 card, every 2 current anima adds
        // one to the attack base, capped at 4.  This must be applied before
        // the existing percentage factor (for example momentum), and the
        // original explicitly excludes the AfterCardAciton window.
        let beng_quan_anima_attack_bonus = if is_card_segment
            && actor.identity.fate_strategies.contains(&432)
            && self.active_effect_is_beng_quan()
            && !self.active_effect_after_action()
        {
            (actor.core.anima.max(0) / 2).min(4)
        } else {
            0
        };
        let next_attack_bonus = actor.turn.next_attack_bonus.max(0);
        let had_current_effect_shatter_defense = self.active_effect_shatter_defense() > 0;
        // BattleCharacter.ApplyDamage routes persistent YeRenHua through the
        // same defense-piercing branch as SuiFang. SuiFang(340) itself is a
        // card-action-scoped charge (removed at AfterCardAction,
        // CardActionBase.cs:3738-3742), modeled by the effect-invocation
        // frame's shatter_defense instead of a per-segment status layer.
        let had_leaf_blade_flower = actor.status.leaf_blade_flower > 0;
        let had_shatter_formation = actor.formations.shatter_formation > 0;
        let next_attack_shatter_defense = actor.turn.next_attack_shatter_defense > 0;
        let shatter_defense = force_shatter_defense
            || had_current_effect_shatter_defense
            || had_leaf_blade_flower
            || had_shatter_formation
            || next_attack_shatter_defense;
        self.actor_mut(actor_side).turn.attack_segments_performed += 1;
        self.add_active_effect_attacks(1);
        self.actor_mut(actor_side).turn.dan_ka_gong_ji_ji_shu += 1;
        self.actor_mut(actor_side).turn.turn_attack_segments += 1;
        if mark_consumed_beng_quan_chuo {
            self.actor_mut(actor_side).beng.consumed_beng_quan_chuo = beng_quan_chuo_bonus;
        }

        // BattleCharacter.ApplyDamage gives the turn-wide source precedence;
        // persistent ignore-defense charges are consumed only by the else-if.
        if self.actor(actor_side).turn.current_turn_ignore_defense <= 0
            && self.actor(actor_side).turn.ignore_defense_attacks > 0
        {
            let before = self.actor(actor_side).turn.ignore_defense_attacks;
            self.actor_mut(actor_side).turn.ignore_defense_attacks -= 1;
            let after = self.actor(actor_side).turn.ignore_defense_attacks;
            self.record_counter_transition(
                actor_side,
                "回合",
                "ignoreDefenseAttacks",
                "无视防御攻击",
                before,
                after,
            );
        }
        if self.actor(actor_side).formations.forge_bone_attacks > 0 {
            self.actor_mut(actor_side).formations.forge_bone_attacks -= 1;
            self.apply_physique_amount(actor_side, 1);
        }
        let star_bonus = if in_star_slot { star_power } else { 0 };
        if self.actor(actor_side).beng.momentum_before_attack > 0 {
            self.actor_mut(actor_side).beng.momentum_before_attack -= 1;
            self.modify_momentum(actor_side, 1);
        }
        let dream_mirage_attack = self.prepare_dream_mirage_attack_segment(actor_side);
        let previous_card_debuff_bonus = if is_card_segment {
            self.dream_mirage_previous_card_debuff_attack_bonus(actor_side, slot)
        } else {
            0
        };
        let double_sword_intent_and_attack_bonus = self.dream_mirage_value(
            actor_side,
            super::cards_dream_mirage::DreamMirageValue::DoubleNextSwordIntentAndAttackBonus,
        ) > 0;
        let mut amount = base_attack
            + attack_bonus
            + if double_sword_intent_and_attack_bonus {
                attack_bonus
            } else {
                0
            }
            + staff_stance_bonus
            + sword_intent
            + if double_sword_intent_and_attack_bonus {
                sword_intent
            } else {
                0
            }
            + star_bonus
            + shatter_bonus
            + forge_bone_bonus
            + spirit_claw_attack_bonus
            + month_claw_attack_bonus
            + beng_quan_anima_attack_bonus
            + min_night_bonus
            + beng_quan_chuo_bonus
            + lone_night_wolf_bonus
            + next_attack_bonus
            + reverse_gu_flat_bonus
            + dream_mirage_attack.flat_bonus
            + previous_card_debuff_bonus
            + self.actor(actor_side).music.wild_dance_tune.max(0);
        if next_attack_bonus > 0 {
            self.actor_mut(actor_side).turn.next_attack_bonus = 0;
        }
        if attack_trace_enabled() {
            eprintln!(
                "attack turn={} actor={:?} base={} attackBonus={} staffBonus={} gun={} talents206={} starBonus={} shatterBonus={} forgeBoneBonus={} swordIntent={} attackReduction={} amountBeforeReduce={}",
                self.actor_turn,
                actor_side,
                base_attack,
                attack_bonus,
                staff_stance_bonus,
                self.actor(actor_side).beng.gun_stance,
                self.actor(actor_side).identity.talents.contains(&206),
                star_bonus,
                shatter_bonus,
                forge_bone_bonus,
                sword_intent,
                attack_reduction,
                amount,
            );
        }
        self.update_active_effect_pending_sword_intent(sword_intent);
        if amount > 0
            && self.actor(target_side).identity.talents.contains(&206)
            && self.actor(target_side).beng.quan_stance > 0
        {
            amount = (amount - 1).max(1);
        }
        let amount_before_reduction = amount;
        let amount_without_sharpness = if amount_before_reduction > 0 {
            (amount_before_reduction - attack_reduction).max(1)
        } else {
            0
        };
        let amount_without_sharpness = if amount_without_sharpness > 0
            && self.actor(target_side).elements.metal_iron_bone > 0
        {
            (amount_without_sharpness - 5).max(1)
        } else {
            amount_without_sharpness
        };
        let amount_without_sharpness = if amount_without_sharpness > 0
            && self.actor(target_side).fate.all_things_inauspicious > 0
        {
            self.actor_mut(target_side).fate.all_things_inauspicious -= 1;
            (amount_without_sharpness - 6).max(1)
        } else {
            amount_without_sharpness
        };
        let no_sharpness = self.actor(actor_side).elements.no_sharpness_for_attack > 0;
        let actor = self.actor(actor_side);
        let target = self.actor(target_side);
        let mut factor_percent = 100;
        if target.status.flaw > 0 {
            factor_percent += if target.status.yin_fu > 0 {
                target.status.yin_fu
            } else {
                40
            };
        }
        if actor.status.weakness > 0
            && actor.status.mystic_soul <= 0
            && actor.turn.ignore_weakness_attacks <= 0
        {
            factor_percent -= if actor.status.yin_fu > 0 {
                actor.status.yin_fu
            } else {
                40
            };
        }
        factor_percent +=
            super::cards_synthetic_oracle_verified_secret_misc::thunder_mindset_attack_bonus_percent(
                actor,
                self.active_effect_name(),
                self.active_effect_after_action(),
            );
        if actor.status.mystic_soul > 0 && !dream_mirage_attack.replaces_percent_momentum {
            let debuff_count = self.negative_status_stack_count(actor_side);
            if debuff_count > 0 {
                // 原版 CalculateAttack 玄心斩魄分支（BattleCharacter.cs:11701-11706）：
                // 负面状态按 气势 计层，百分比倍率须乘 QiShiBeiLv（气势倍率）。
                // 卡组含 威震四方（base 10000084）时改走 11583-11584 平值分支，
                // 平值加攻已在 prepare_dream_mirage_attack_segment 处理，此处跳过。
                // QiShiBeiLv 由崩拳寸劲被下一张牌消耗时置 2
                // （CardActionBase.cs:3059-3065，崩拳寸劲标记不计入连崩），
                // AfterCardAction 清除（CardActionBase.cs:3743-3745）。
                // oracle 锚点：28b5e2bbb01dabca/round-14 cp13 p1.hp 86 vs 88
                // （(8+2)*1.4=14，减防 7 + 外伤 2 = 9）、
                // d18a47dfdf4cff04/round-14 cp13 p2.hp 70 vs 82
                // （(8+2)*3.4=34，负面 12 层 ×10% ×2 倍率）。
                let multiplier = actor.beng.momentum_multiplier.max(1);
                factor_percent += debuff_count * 10 * multiplier;
            }
        }
        if target.fate.dismantle_move > 0 && target.beng.quan_stance > 0 {
            factor_percent -= 50;
        }
        if self.actor(target_side).elements.water_stealth > 0 {
            factor_percent -= 40;
        }
        factor_percent += self.qi_sinks_attack_factor_percent(actor_side);
        let dream_mirage_reflection = self.dream_mirage_reflection_active(target_side);
        if dream_mirage_reflection {
            factor_percent -= 25;
        }
        let momentum = self.actor(actor_side).beng.momentum;
        if momentum > 0 && !dream_mirage_attack.replaces_percent_momentum {
            let momentum_multiplier = self.actor(actor_side).beng.momentum_multiplier.max(1);
            factor_percent += momentum * 10 * momentum_multiplier;
            let retain_momentum = self.actor(actor_side).identity.fate_strategies.contains(&428)
                && self.active_effect_name().contains('掌');
            if !retain_momentum {
                self.modify_momentum(actor_side, -1);
            }
        }
        // 梦•枯木逢春家族（base 4000087）：原版 CalculateAttack 在百分比
        // 倍率之前对「当前累计攻击值」整体翻倍（含星力/加攻等平值，
        // BattleCharacter.cs:11627-11641）。因百分比为乘法，等价于在
        // 平值削减（减攻/铁骨/注视不移）之后、factor 之前乘倍率。
        let amount_without_sharpness = if self.active_effect_attack_multiplier() > 1 {
            amount_without_sharpness.saturating_mul(self.active_effect_attack_multiplier())
        } else {
            amount_without_sharpness
        };
        let mut final_without_sharpness = if amount_without_sharpness > 0 {
            ((amount_without_sharpness * factor_percent.max(0)) / 100).max(1)
        } else {
            0
        };
        if final_without_sharpness > 0
            && self.actor(actor_side).astrology.all_goes_well > 0
            && final_without_sharpness < 6
        {
            final_without_sharpness = 6;
        }
        // BattleCharacter.CalculateAttack applies percentage multipliers before
        // 万事如意. ApplyDamage then lets only that base value cross defense;
        // Sharpness and 水刃 are post-defense wound effects.
        let actor = self.actor(actor_side);
        let force_wound = fire_blade_wound
            || actor.turn.next_attack_wound_bonus > 0
            || actor.turn.guaranteed_wound > 0
            || actor.elements.long_ma_spirit > 0;
        if self.actor(actor_side).identity.talents.contains(&68) {
            let defense_gain = if self.actor(actor_side).core.defense == 0 {
                2
            } else {
                1
            };
            self.gain_defense(actor_side, defense_gain);
        }
        // 原版每段攻击管道顺序（BattleCharacter.cs:11757-11775，
        // decompiled build-24646245）：万魔噬心曲（幽绪乱心曲 5010013）
        // 自伤先于木刺/吸血/木灵回血 —— 先自伤让出 maxHp 余量，木刺等量
        // 回血才不会被上限截断。oracle 锚点：mirror-32299000
        // e55579d1d2fd6ce1/round-15 cp4（p1.hp 91 → 87：乱心曲 -5×2 +
        // 木刺 +1×2 + 木灵 +2×2；引擎原 91 → 86，首段木刺回血被
        // maxHp 91 截断）。
        let chaotic_mind = self.actor(actor_side).music.chaotic_mind_tune.max(0);
        if chaotic_mind > 0 {
            self.modify_actor_hp(actor_side, -chaotic_mind, false, false);
        }
        self.apply_wood_thorn_before_attack_segment(actor_side);
        // 原版攻击管道（BattleCharacter.Attack:11757-11775 → ApplyDamage:10787）：
        // 万魔噬心曲自伤 → 木刺/吸血等段前钩子 → ApplyDamage 内 wound 判定
        // （含锋锐加成）才读取目标护体。木刺可能先吃掉护体：护体 1 被木刺
        // 消耗后，本段攻击应照常触发 wound 并附加锋锐。引擎原先在木刺之前
        // 用旧护体状态判 would_wound，首段攻击丢失锋锐（oracle 锚点：
        // hf-latest-32308000-16f9c778 3e38402e6f96ebd6/round-12 cp8：
        // 原版 44+30+木刺 1=75、锋锐 36→22→14；引擎 8+44+1=53、锋锐 36→22）。
        let target_guard = self.actor(target_side).core.guard;
        let would_wound_without_force = if final_without_sharpness <= 0
            || target_guard > 0
            || self.actor(target_side).fate.graft_flowers_to_tree > 0
        {
            false
        } else if ignore_defense {
            true
        } else if shatter_defense {
            let target_defense = self.actor(target_side).core.defense;
            let defense_piercing_attack = if final_without_sharpness * 2 >= target_defense {
                final_without_sharpness + div_ceil(target_defense, 2)
            } else {
                final_without_sharpness * 2
            };
            defense_piercing_attack > target_defense
        } else {
            final_without_sharpness > self.actor(target_side).core.defense
        };
        // Forced-wound effects enter the wound branch even through Guard or
        // 移花接木. Those defenses still settle the resulting HP request later.
        let would_wound = if force_wound {
            true
        } else {
            would_wound_without_force
        };
        let sharpness = if would_wound && !no_sharpness {
            actor_sharpness
        } else {
            0
        };
        if sharpness > 0 {
            self.actor_mut(actor_side).sword.sharpness -= sharpness;
            if return_sharpness_percent > 0 {
                let returned = div_ceil(sharpness * return_sharpness_percent, 100);
                if returned > 0 {
                    self.gain_sharpness(actor_side, returned);
                }
            }
            self.consume_dream_mirage_return_sharpness(actor_side, sharpness);
            // 水灵•劲浪（卡 423）水灵激活：BattleCharacter.cs:10819-10824 —
            // 当前牌 base 423、水灵已激活（CheckWuXing JiHuoShuiLing）、且非
            // 后招阶段（AfterCardAction）时，每消耗 1 锋锐 → 水势+1 且生命及
            // 上限+1。锋锐消耗量取 7000099 回锋返还之前的 buffValue2
            // （原版 10802 先读 FengRui 总值，10808 RemoveBuff 后 10810/10819
            // 两分支共用该值）。
            if self.active_effect_base_id() == 423
                && self.check_wu_xing(actor_side, super::Element::Water)
                && !self.active_effect_after_action()
            {
                self.gain_water_momentum(actor_side, sharpness);
                self.modify_actor_max_hp(actor_side, sharpness);
                self.modify_actor_hp(actor_side, sharpness, false, false);
            }
        }
        let water_blade_seal = self.actor(actor_side).elements.water_blade_seal > 0;
        amount = final_without_sharpness + sharpness;
        if water_blade_seal && would_wound && amount > 0 {
            amount = amount * 3 / 2;
        }

        if double_sword_intent_and_attack_bonus {
            self.modify_dream_mirage_value(
                actor_side,
                super::cards_dream_mirage::DreamMirageValue::DoubleNextSwordIntentAndAttackBonus,
                -1,
            );
        }
        let target_hp_before = self.actor(target_side).core.hp;
        let target_defense_before = self.actor(target_side).core.defense;
        self.apply_attack_damage(
            actor_side,
            final_without_sharpness,
            sharpness,
            if water_blade_seal { 150 } else { 100 },
            ignore_defense,
            shatter_defense,
        );
        if dream_mirage_reflection {
            self.apply_dream_mirage_reflected_life_loss(actor_side, amount_without_sharpness);
        }
        if attack_trace_enabled() {
            eprintln!(
                "damage turn={} actor={:?} final={} target={:?} defenseBefore={} defenseAfter={} hpBefore={} hpAfter={} ignoreDefense={} shatterDefense={} factor={}",
                self.actor_turn,
                actor_side,
                amount,
                target_side,
                target_defense_before,
                self.actor(target_side).core.defense,
                target_hp_before,
                self.actor(target_side).core.hp,
                ignore_defense,
                shatter_defense,
                factor_percent,
            );
        }
        let hp_lost = (target_hp_before - self.actor(target_side).core.hp).max(0);
        // BattleCharacter.cs:10787-10855：四个「必定击伤」旁路（talent 67 /
        // 下次攻击后减生命 536 / 必定使击伤 693 / 龙马精神 708）直接进入击伤
        // 分支，IL_034a-03a4 只有默认分支（num>0 且非 skipWoundCheck）才检查
        // 目标护体与移花接木；旁路分支两者都不检查。因此被护体全挡的伤害
        // 在旁路下仍计 WoundedCount。oracle 锚点：ed3ca57eb75f856f/round-11
        // cp[8]（形意剑 1010011 邻接梦•灵气灌注 1040067 → 必定使击伤，
        // 护体 1 全挡 12 伤仍计击伤 → 剑意 1-1+4=4，原版 4 引擎 0）。
        let forced_wounded_count = force_wound;
        if hp_lost > 0 || fire_blade_wound || forced_wounded_count {
            self.add_active_effect_wounded_count(1);
            // WoundedCount(303) 与 ActualDamage(302) 同生命周期：跨卡持久
            // 累计（BattleCharacter.cs:10854 +1），仅在该攻击者自己出牌完成
            // 时（OnAfterExecuted，CardActionBase.cs:4745）一并清零。
            let before = self.actor(actor_side).turn.wounded_count_carry;
            self.actor_mut(actor_side).turn.wounded_count_carry += 1;
            let after = self.actor(actor_side).turn.wounded_count_carry;
            self.record_counter_transition(
                actor_side,
                "回合",
                "woundedCountCarry",
                "击伤计数",
                before,
                after,
            );
        }
        if hp_lost > 0 {
            self.add_active_effect_actual_damage(hp_lost);
            // ActualDamage(302) 原版是攻击者身上跨卡持久计数
            // （BattleCharacter.cs:10858-10861）：凡走 ApplyDamage 的
            // Attack 型实际伤害都累加，不区分出牌攻击或回合末被动攻击
            // （fate 137 凝水化刃等无 invocation 帧的路径），仅在该攻击者
            // 自己出牌完成时（CardActionBase.cs:4743-4745）转入 644 并清零。
            // 玫刺(7000027) 等卡在自身攻击后读到的值 = 残留 + 本卡。
            let before = self.actor(actor_side).turn.actual_damage_carry;
            self.actor_mut(actor_side).turn.actual_damage_carry += hp_lost;
            let after = self.actor(actor_side).turn.actual_damage_carry;
            self.record_counter_transition(
                actor_side,
                "回合",
                "actualDamageCarry",
                "实际伤害",
                before,
                after,
            );
        }
        self.apply_beng_quan_star_seize_after_attack(actor_side, hp_lost);
        self.apply_dream_mirage_forge_fist_damage_to_physique(actor_side, slot, hp_lost);
        let bat_threshold = self.actor(actor_side).chance.an_xing_bian_fu.max(0);
        if hp_lost > 0 && bat_threshold > 0 && hp_lost <= bat_threshold {
            self.modify_actor_hp(actor_side, hp_lost, false, false);
        }
        if self.actor(actor_side).elements.spring_flow > 0 {
            self.actor_mut(actor_side).elements.spring_flow -= 1;
            if hp_lost > 0 {
                let momentum_gain = hp_lost / 5;
                if momentum_gain > 0 {
                    self.gain_water_momentum(actor_side, momentum_gain);
                }
            }
        }
        let dismantle_reflect = if self.actor(target_side).fate.dismantle_move > 0
            && self.actor(target_side).beng.gun_stance > 0
        {
            self.actor(target_side).fate.dismantle_move_reflect
        } else {
            0
        };
        if dismantle_reflect > 0 {
            self.apply_damage(target_side, dismantle_reflect, false, false, false);
        }
        if had_shatter_formation {
            self.actor_mut(actor_side).formations.shatter_formation -= 1;
        }
        if next_attack_shatter_defense
            && !had_current_effect_shatter_defense
            && !had_leaf_blade_flower
            && !had_shatter_formation
        {
            self.modify_next_attack_shatter_defense(actor_side, -1);
        }
        // 截拳式（Fate 430）在原版 ApplyDamage 中位于梦断拳/猫系天赋共振
        // （:10960-10979）之后、反震心法（:10998-11005）之前；引擎侧反震
        // 在 apply_post_attack_buff_hooks 末尾结算，故此处先于该钩子链触发。
        self.apply_jie_quan_shi_after_attack(actor_side, target_side, hp_lost);
        self.apply_post_attack_buff_hooks(actor_side, target_side, hp_lost);
        self.apply_ronghui_post_attack(actor_side, hp_lost, earth_fiend_active_before_attack);
        if self.observation.mode.is_detailed() {
            if let Some(event_index) = self.observation.current_card_event_index {
                let hit_index = self.observation.current_attack_segment_index;
                self.observation.current_attack_segment_index += 1;
                self.observation.attack_segments.push(ReplayAttackSegment {
                    event_index,
                    target: target_side,
                    hit_index,
                    actor_hp_before: segment_actor_hp_before,
                    actor_hp_after: self.actor(actor_side).core.hp,
                    hp_before: segment_hp_before,
                    hp_after: self.actor(target_side).core.hp,
                    def_before: segment_def_before,
                    def_after: self.actor(target_side).core.defense,
                });
            }
        }
        hp_lost
    }

    fn apply_delayed_attack_status_triggers(
        &mut self,
        actor_side: PlayerSide,
        target_side: PlayerSide,
    ) {
        if self.actor(target_side).status.back_solitude > 0 {
            self.actor_mut(target_side).status.back_solitude -= 1;
            self.add_actor_negative_status(actor_side, 101, 1);
        }
        if self.actor(actor_side).status.strike_void > 0 {
            self.actor_mut(actor_side).status.strike_void -= 1;
            self.add_actor_negative_status(target_side, 102, 1);
        }
    }

    /// 截拳式（Fate 430）：BattleCharacter.cs:10981-10990 — ApplyDamage 攻击
    /// 伤害段中，实际伤害（减防/护体后 HP 损失）> 0 且持有 JieQuanShi(770)
    /// 时消耗 1 层；自身为拳架势则对目标 +1 减攻（JianGong 103），否则
    /// +1 虚弱（XuRuo 101）。每场战斗开局仅 1 层（battle_start.rs），
    /// 即「首次击伤」触发一次。
    fn apply_jie_quan_shi_after_attack(
        &mut self,
        actor_side: PlayerSide,
        target_side: PlayerSide,
        hp_lost: i64,
    ) {
        if hp_lost <= 0 || self.actor(actor_side).fate.jie_quan_shi <= 0 {
            return;
        }
        self.actor_mut(actor_side).fate.jie_quan_shi -= 1;
        if self.actor(actor_side).beng.quan_stance > 0 {
            self.add_actor_negative_status(target_side, 103, 1);
        } else {
            self.add_actor_negative_status(target_side, 101, 1);
        }
    }

    fn apply_post_attack_buff_hooks(
        &mut self,
        actor_side: PlayerSide,
        target_side: PlayerSide,
        hp_lost: i64,
    ) {
        if self.actor(actor_side).turn.wood_spring_turns > 0 {
            self.modify_actor_max_hp(actor_side, 2);
            self.modify_actor_hp(actor_side, 2, false, false);
        }
        if self
            .actor(actor_side)
            .turn
            .attack_applies_internal_injury_turns
            > 0
        {
            self.add_actor_negative_status(target_side, 100, 1);
        }
        // 万魔噬心曲自伤已移至每段攻击起点（先于木刺/吸血/木灵回血，
        // 见 apply_attack_with_options 内 segment 前置钩子），原版顺序
        // BattleCharacter.cs:11757-11759 在 CalculateAttack 阶段、ApplyDamage
        // 之前。此处不再重复结算。build 24610558 与旧 build 24466094
        // 反编译顺序一致（历史行为）。反震（FanZhenXinFa，:10998-11005）
        // 仍在吸血之后、本钩子链末尾。
        if hp_lost > 0
            && self.active_effect_is_frenzy_sword()
            && self.actor(actor_side).sword.frenzy_sword_zero > 0
            && !self.active_effect_after_action()
        {
            let heal = hp_lost * self.actor(actor_side).sword.frenzy_sword_zero / 100;
            if heal > 0 {
                self.modify_actor_hp(actor_side, heal, false, false);
            }
        }
        let reflect = self.actor(target_side).fate.reflect_mindset.max(0);
        if reflect > 0 {
            self.apply_damage(target_side, reflect, false, false, false);
        }
        // BattleCharacter.ApplyDamage（build 24666769:10882-11033）：
        // 狂剑零式吸血、反震先结算，摘花飞叶与伤魂咒阵随后才施加内伤。
        // 该顺序在接近生命上限及阴符绝阵反伤时会改变实际生命。
        let actor_leaf = self.actor(actor_side).status.leaf_pluck_flying_leaf.max(0);
        if actor_leaf > 0 {
            self.add_actor_negative_status(target_side, 100, actor_leaf);
        }
        let target_leaf = self.actor(target_side).status.leaf_pluck_flying_leaf.max(0);
        if target_leaf > 0 {
            self.add_actor_negative_status(actor_side, 100, target_leaf);
        }
        if self
            .actor(actor_side)
            .formations
            .soul_injury_curse_formation
            > 0
        {
            self.actor_mut(actor_side)
                .formations
                .soul_injury_curse_formation -= 1;
            self.add_actor_negative_status(target_side, 100, 1);
        }
        if self
            .actor(target_side)
            .formations
            .soul_injury_curse_formation
            > 0
        {
            self.actor_mut(target_side)
                .formations
                .soul_injury_curse_formation -= 1;
            self.add_actor_negative_status(actor_side, 100, 1);
        }
    }
}

#[cfg(test)]
#[path = "tests_fate_strategy_432.rs"]
mod fate_432_tests;
