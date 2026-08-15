use super::cards_dream_mirage::DreamMirageValue;
use super::decisions::{PercentRollDecisionResolution, RandomRangeDecisionResolution};
use super::support::{normalized_base_id, opponent_side};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// BattleCharacter.ModifyBuffValue:8620-8640,8830-8865.
    /// Call once with the post-mitigation positive delta for every core
    /// negative status, including External Injury's specialized path.
    pub(super) fn apply_dream_mirage_negative_status_gain_hooks(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        actual_delta: i64,
    ) {
        if actual_delta <= 0 || !matches!(status, 100 | 101 | 102 | 103 | 104 | 105 | 367 | 393) {
            return;
        }
        if status != 100 && status != 367 {
            let flowing = self
                .dream_mirage_value(actor_side, DreamMirageValue::FlowingMerciless)
                .max(0);
            if flowing > 0 {
                self.add_actor_negative_status(actor_side, 100, actual_delta * flowing);
            }
        }

        let footwork = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamMysticFootwork)
            .max(0);
        let suppressed =
            self.dream_mirage_value(actor_side, DreamMirageValue::DreamMysticFootworkSuppressed);
        let triggered = self.dream_mirage_value(
            actor_side,
            DreamMirageValue::DreamMysticFootworkTriggerCount,
        );
        if footwork <= 0 || suppressed == 1 || triggered >= footwork {
            return;
        }
        self.modify_dream_mirage_value(
            actor_side,
            DreamMirageValue::DreamMysticFootworkTriggerCount,
            1,
        );
        let reflected =
            if self.dream_mirage_value(actor_side, DreamMirageValue::DreamMysticFootworkHigh) > 0 {
                actual_delta.min(2)
            } else {
                1
            };
        self.add_actor_negative_status(opponent_side(actor_side), status, reflected);
    }

    pub(super) fn apply_configured_anima(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
    ) {
        if let Some(anima) = card.anima.filter(|value| *value > 0) {
            self.gain_anima(actor_side, anima);
        }
    }

    pub(super) fn apply_after_hp_cost_hooks(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
        printed_hp_cost: i64,
        is_beng_quan: bool,
    ) {
        let base_id = normalized_base_id(card);
        if printed_hp_cost > 0 {
            // CardActionBase.CheckCardCost post-payment source order is
            // observable: 血影 -> 崩拳弹 -> 崩拳返玄 -> the one-shot
            // printed-cost refund -> persistent/one-shot defense -> card
            // damage -> next-Beng damage -> 热血化气.
            if self.actor(actor_side).turn.blood_shadow > 0 {
                self.actor_mut(actor_side).turn.blood_shadow -= 1;
                self.gain_agility(actor_side, printed_hp_cost);
            }
            if is_beng_quan && self.actor(actor_side).beng.beng_quan_bounce > 0 {
                self.modify_actor_hp(actor_side, printed_hp_cost, false, false);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side).beng.beng_quan_bounce -= 1;
                }
            }
            if self.actor(actor_side).beng.beng_quan_return_profound > 0 && is_beng_quan {
                self.modify_actor_hp(actor_side, printed_hp_cost, false, false);
            }
            if self.dream_mirage_value(actor_side, DreamMirageValue::NextHpCostRefund) > 0 {
                self.modify_dream_mirage_value(actor_side, DreamMirageValue::NextHpCostRefund, -1);
                self.modify_actor_hp(actor_side, printed_hp_cost, false, false);
            }
            let persistent = self
                .dream_mirage_value(actor_side, DreamMirageValue::HpGainDefense)
                .max(0);
            if persistent > 0 {
                // HaoShengMingJiaFang is a presence gate, not a per-layer
                // multiplier: any positive value grants printed hpCost once.
                self.gain_defense(actor_side, printed_hp_cost);
            }
            if self.dream_mirage_value(actor_side, DreamMirageValue::NextHpGainDefense) > 0 {
                self.modify_dream_mirage_value(actor_side, DreamMirageValue::NextHpGainDefense, -1);
                self.gain_defense(actor_side, printed_hp_cost);
            }
            if base_id == 10_000_093 {
                self.apply_damage(actor_side, printed_hp_cost, false, false, false);
            }
            if self.actor(actor_side).beng.next_beng_quan_hp_cost_damage > 0 && is_beng_quan {
                self.apply_damage(actor_side, printed_hp_cost, false, false, false);
                if base_id != 10_000_035 {
                    self.actor_mut(actor_side)
                        .beng
                        .next_beng_quan_hp_cost_damage -= 1;
                }
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&347)
                && self.actor(actor_side).fate.hot_blood_to_qi_triggered <= 0
            {
                self.actor_mut(actor_side).fate.hot_blood_to_qi_triggered = 1;
                self.gain_anima(actor_side, 1);
            }
        }
    }

    pub(super) fn apply_after_card_fortune_hooks(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
    ) {
        // 天运•避凶：只在非开局效果牌后触发，探灵等开局牌不消耗层数。
        if ReplayState::card_has_opening_effect(normalized_base_id(card)) {
            return;
        }
        let count = self.actor(actor_side).formations.fortune_avoid_misfortune;
        if count <= 0 {
            return;
        }
        let defense = self
            .actor(actor_side)
            .formations
            .fortune_avoid_misfortune_defense;
        let healing = self
            .actor(actor_side)
            .formations
            .fortune_avoid_misfortune_healing;
        if defense > 0 {
            self.gain_defense(actor_side, defense);
        }
        if healing > 0 {
            self.modify_actor_hp(actor_side, healing, false, false);
        }
        self.actor_mut(actor_side)
            .formations
            .fortune_avoid_misfortune -= 1;
    }

    pub(super) fn clear_consumed_beng_quan_chuo(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
    ) {
        let consumed = self.actor(actor_side).beng.consumed_beng_quan_chuo;
        if consumed <= 0 {
            return;
        }
        let base_id = normalized_base_id(card);
        let actor = self.actor_mut(actor_side);
        if base_id != 10_000_035 {
            actor.beng.beng_quan_chuo = (actor.beng.beng_quan_chuo - consumed).max(0);
        }
        actor.beng.consumed_beng_quan_chuo = 0;
    }

    pub(super) fn apply_anima_shortage_fallback(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
    ) {
        if self.actor(actor_side).identity.talents.contains(&142) {
            self.gain_anima(actor_side, 3);
            self.gain_hexagram(actor_side, 3);
        } else {
            self.gain_anima(actor_side, 1);
        }
        let charge_qi = super::original_config::original_card_charge_qi(card.id);
        if charge_qi > 0 {
            self.gain_anima(actor_side, charge_qi);
        }
    }

    pub(super) fn gain_hexagram(&mut self, actor_side: crate::model::PlayerSide, amount: i64) {
        if amount <= 0 {
            return;
        }
        let mut amount = amount;
        if self.actor(actor_side).fate.wu_jing_gua_yan > 0 {
            // BattleCharacter.cs:8553-8557：每次正向 ModifyBuffValue(GuaXiang)
            // 只加一次，不按 delta 点数重复，并立即消耗一层。
            amount += 1;
            self.actor_mut(actor_side).fate.wu_jing_gua_yan -= 1;
        }
        let six_yao = self.actor(actor_side).formations.six_yao_formation;
        if six_yao > 0 {
            let damage = amount * six_yao;
            if damage > 0 {
                // 原版 BattleCharacter.cs:8761-8769 走
                // ApplyDamage(DamageType.Damage, skipWoundCheck: true, hitDef: false)，
                // 即完整伤害管线：目标铁骨 −5 最低 1（:10715-10720）+ 防御吸收
                // + 护体。引擎原先用 apply_defense_first_hp_loss 裸扣，漏了
                // 铁骨。apply_damage 的 apply_wound_bonus=false 对应非 Attack
                // 路径（含铁骨），ignore_defense=false 对应仍吃防御。
                // oracle 锚点：0b54851d5a95ad84/round-13（p1 金灵•铁骨 生效期
                // 内 p2 灵卦自衍 卦象+1 → 六爻杀阵 3 → 原版 1 / 引擎 3）。
                self.apply_damage(actor_side, damage, false, false, false);
            }
        }
        let should_gain_star = self.actor(actor_side).identity.talents.contains(&61)
            && self.actor(actor_side).astrology.star_power == 0;
        let before = self.actor(actor_side).astrology.hexagram;
        self.actor_mut(actor_side).astrology.hexagram += amount;
        let after = self.actor(actor_side).astrology.hexagram;
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            "七星",
            "hexagram",
            "卦象",
            before,
            after,
            after - before,
        );
        self.apply_six_yao_buff_gain_damage(actor_side, amount);
        if self.actor(actor_side).identity.talents.contains(&36) {
            // Talent_36 rewards each positive AddGuaXiang call, not each point gained.
            self.gain_anima(actor_side, 2);
        }
        let two_polarity_hp = self.actor(actor_side).elements.dream_two_polarity_hp.max(0);
        if two_polarity_hp > 0 {
            self.modify_actor_hp(actor_side, amount * two_polarity_hp, false, false);
        }
        let two_polarity_defense = self
            .actor(actor_side)
            .elements
            .dream_two_polarity_defense
            .max(0);
        if two_polarity_defense > 0 {
            self.gain_defense(actor_side, amount * two_polarity_defense);
        }
        if should_gain_star {
            self.modify_star_power(actor_side, 1);
        }
    }

    /// Original SetBuffValue semantics: exact replacement, lower-clamped, and
    /// deliberately without ModifyBuffValue gain/loss hooks.
    #[allow(dead_code)]
    pub(super) fn set_hexagram(&mut self, actor_side: PlayerSide, value: i64) -> i64 {
        let before = self.actor(actor_side).astrology.hexagram;
        let after = value.max(0);
        self.actor_mut(actor_side).astrology.hexagram = after;
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            "七星",
            "hexagram",
            "卦象",
            before,
            after,
            after - before,
        );
        after - before
    }

    /// BattleCharacter.ModifyBuffValue, Steam build 24180265. Positive gains
    /// retain the existing AddGuaXiang hooks; losses commit/clamp first, then
    /// accumulate the actual loss and run 梦御雷 in source order.
    pub(super) fn modify_hexagram(&mut self, actor_side: PlayerSide, delta: i64) -> i64 {
        if delta == 0 {
            return 0;
        }
        if delta > 0 {
            self.gain_hexagram(actor_side, delta);
            return delta;
        }

        let before = self.actor(actor_side).astrology.hexagram.max(0);
        let actual_loss = before.min(delta.saturating_neg());
        if actual_loss <= 0 {
            return 0;
        }
        let after = before - actual_loss;
        self.actor_mut(actor_side).astrology.hexagram = after;
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Resource,
            "七星",
            "hexagram",
            "卦象",
            before,
            after,
            after - before,
        );
        self.apply_original_hexagram_loss_hooks(actor_side, actual_loss);
        -actual_loss
    }

    fn apply_original_hexagram_loss_hooks(&mut self, actor_side: PlayerSide, actual_loss: i64) {
        if actual_loss <= 0 {
            return;
        }

        // Build 24180265 records the lower-clamped committed loss as a positive
        // delta in hidden buff 700. Card 422 no longer overlaps this path.
        let lost_hexagram = self.actor(actor_side).astrology.lost_hexagram;
        self.actor_mut(actor_side).astrology.lost_hexagram =
            lost_hexagram.saturating_add(actual_loss);
        if self.actor(actor_side).astrology.dream_thunder_hexagram > 0
            && self.actor(actor_side).astrology.dream_thunder_round_limit == 0
        {
            self.actor_mut(actor_side)
                .astrology
                .dream_thunder_round_limit = 1;
            self.modify_hexagram(actor_side, actual_loss);
        }
    }

    /// BattleCharacter.GetNextRandomValue:9332-9337 (build 24646245).
    /// Card 422 紫芒星爆 (buff 773 ZiMangXingBao) 的星力、卦象与共鸣 71
    /// 的灵气是互斥的 if-else-if 分支，不是顺序消耗：命中星力分支后
    /// 不再扣卦象。oracle 锚点：hf-32308000 371a9271ba43ad84/round-14
    /// cp9（五雷轰顶 5 次随机：原版持紫芒星爆+星力 8 → 星力 8→2、卦象
    /// 保持 1；引擎顺序消耗把卦象 1→0）。
    pub(super) fn consume_original_random_hexagram_side_effects(
        &mut self,
        actor_side: PlayerSide,
    ) -> bool {
        if self.actor(actor_side).astrology.zi_mang_xing_bao > 0
            && self.actor(actor_side).astrology.star_power > 0
        {
            self.modify_star_power(actor_side, -1);
            return false;
        }
        let used_hexagram = self.modify_hexagram(actor_side, -1) < 0;
        if used_hexagram {
            self.actor_mut(actor_side)
                .astrology
                .hexagram_effective_count += 1;
            return true;
        }
        // 原版第三分支：共鸣 71 生效且临时标记已置位时消耗 1 灵气
        // （GetNextRandomValue:9335-9337）。
        if self.actor(actor_side).identity.talent_resonance_id == Some(71)
            && self
                .actor(actor_side)
                .identity
                .talent_resonance_temp_flags
                .contains(&71)
            && self.actor(actor_side).core.anima > 0
        {
            self.reduce_anima_unchecked(actor_side, 1);
        }
        false
    }

    /// BattleCharacter.OnTurnEnded:5771-5774 uses RemoveBuff, not Modify.
    pub(super) fn clear_dream_thunder_round_limit(&mut self, actor_side: PlayerSide) {
        self.actor_mut(actor_side)
            .astrology
            .dream_thunder_round_limit = 0;
    }

    /// Card_4000088.cs:86-92 reads hidden buff 700 without clearing it. Steam
    /// build 24180265 accumulates each actual Hexagram loss in this ledger.
    pub(super) fn original_lost_hexagram_ledger(&self, actor_side: PlayerSide) -> i64 {
        self.actor(actor_side).astrology.lost_hexagram.max(0)
    }

    pub(super) fn try_cost_anima(
        &mut self,
        actor_side: crate::model::PlayerSide,
        cost: i64,
    ) -> bool {
        let spent = self.spend_anima_up_to(actor_side, cost);
        spent >= cost.max(0)
    }

    pub(super) fn spend_anima_up_to(
        &mut self,
        actor_side: crate::model::PlayerSide,
        cost: i64,
    ) -> i64 {
        if cost <= 0 {
            return 0;
        }
        let available = self.actor(actor_side).core.anima;
        let spent = cost.min(available);
        if spent > 0 {
            self.spend_anima_unchecked(actor_side, spent);
        }
        spent
    }

    pub(super) fn try_pay_ling_yong_bu_jue_anima_shortage(
        &mut self,
        actor_side: crate::model::PlayerSide,
        deficit: i64,
    ) -> bool {
        if deficit <= 0 || !self.actor(actor_side).identity.talents.contains(&153) {
            return false;
        }
        let hp_cost = deficit * 3;
        let physique_cost = deficit;
        if hp_cost >= self.actor(actor_side).core.hp
            || physique_cost > self.actor(actor_side).core.physique
        {
            return false;
        }
        self.modify_actor_hp(actor_side, -hp_cost, true, true);
        self.modify_physique_amount(actor_side, -physique_cost);
        true
    }

    pub(super) fn try_pay_meditation_anima_shortage(
        &mut self,
        actor_side: crate::model::PlayerSide,
        deficit: i64,
    ) -> bool {
        let actor = self.actor(actor_side);
        let enabled = actor.identity.fate_strategies.contains(&160)
            || actor.identity.talent_resonance_id == Some(57);
        if deficit <= 0 || !enabled || deficit > actor.status.meditation {
            return false;
        }
        self.modify_actor_negative_status(actor_side, 367, -deficit);
        true
    }

    pub(super) fn spend_anima_unchecked(
        &mut self,
        actor_side: crate::model::PlayerSide,
        amount: i64,
    ) {
        self.reduce_anima_unchecked(actor_side, amount);
    }

    pub(super) fn consume_random_range(
        &mut self,
        actor_side: crate::model::PlayerSide,
        min: i64,
        max: i64,
    ) -> i64 {
        if min > max {
            self.missing_decision("random range invalid range");
            return min;
        }
        let used_hexagram = self.consume_original_random_hexagram_side_effects(actor_side);
        match self.resolve_random_range_decision(actor_side, min, max, used_hexagram, None) {
            RandomRangeDecisionResolution::Selected(value) => return value,
            RandomRangeDecisionResolution::Missing => return min,
            RandomRangeDecisionResolution::Unscoped => {
                if !self.decision_tape.is_empty() {
                    let value = self.decision_tape.remove(0);
                    if value >= 0 {
                        return value;
                    }
                }
            }
        }
        if !self.random_fallback_tape.is_empty() {
            let fallback = self.random_fallback_tape.remove(0);
            if fallback < min || fallback > max {
                self.missing_decision("random range fallback out of range");
            }
            return fallback;
        }
        self.missing_decision("random range");
        min
    }

    #[cfg(all(test, feature = "private-fixtures"))]
    pub(super) fn consume_random_range_or_default(
        &mut self,
        actor_side: crate::model::PlayerSide,
        min: i64,
        max: i64,
        default_value: i64,
    ) -> i64 {
        self.consume_random_range_or_default_inner(actor_side, min, max, default_value, true)
    }

    /// Same resolution as `consume_random_range_or_default`, but without the
    /// original `GetNextRandomValue` side effects. 原版这些卡走
    /// `GetNextParam()`（Card_6000007.cs:82、Card_6000014.cs:80），只取
    /// 服务端 battleParams 队列值，不消耗卦象/星力/灵气（BattleCharacter.cs:
    /// 9314-9338 只有 GetNextRandomValue 才走 GuaXiang 消耗分支）。
    pub(super) fn consume_random_range_or_default_plain(
        &mut self,
        actor_side: crate::model::PlayerSide,
        min: i64,
        max: i64,
        default_value: i64,
    ) -> i64 {
        self.consume_random_range_or_default_inner(actor_side, min, max, default_value, false)
    }

    fn consume_random_range_or_default_inner(
        &mut self,
        actor_side: crate::model::PlayerSide,
        min: i64,
        max: i64,
        default_value: i64,
        consume_hexagram_side_effects: bool,
    ) -> i64 {
        if min > max {
            self.missing_decision("random range default invalid range");
            return default_value;
        }
        if default_value < min || default_value > max {
            self.missing_decision("random range default out of range");
            return min;
        }
        let used_hexagram = if consume_hexagram_side_effects {
            self.consume_original_random_hexagram_side_effects(actor_side)
        } else {
            false
        };
        match self.resolve_random_range_decision(
            actor_side,
            min,
            max,
            used_hexagram,
            Some(default_value),
        ) {
            RandomRangeDecisionResolution::Selected(value) => return value,
            RandomRangeDecisionResolution::Missing => return default_value,
            RandomRangeDecisionResolution::Unscoped => {
                if !self.decision_tape.is_empty() {
                    let value = self.decision_tape.remove(0);
                    if value >= 0 {
                        return value;
                    }
                }
            }
        }
        if !self.random_fallback_tape.is_empty() {
            let fallback = self.random_fallback_tape.remove(0);
            if fallback < min || fallback > max {
                self.missing_decision("random range fallback out of range");
            }
            return fallback;
        }
        default_value
    }

    pub(super) fn consume_percent_roll(&mut self, actor_side: crate::model::PlayerSide) -> i64 {
        self.consume_percent_roll_with_missing_policy(actor_side, false)
    }

    pub(super) fn consume_optional_percent_roll_fail_closed(
        &mut self,
        actor_side: crate::model::PlayerSide,
    ) -> i64 {
        self.consume_percent_roll_with_missing_policy(actor_side, true)
    }

    fn consume_percent_roll_with_missing_policy(
        &mut self,
        actor_side: crate::model::PlayerSide,
        suppress_missing_error: bool,
    ) -> i64 {
        let used_hexagram = self.consume_original_random_hexagram_side_effects(actor_side);
        let resolved =
            self.resolve_percent_roll_decision(actor_side, used_hexagram, suppress_missing_error);
        match resolved {
            PercentRollDecisionResolution::Selected(value) => return value,
            // BattleCharacter.GetNextRandomValue -> GetNextParam
            // （BattleCharacter.cs:9315-9327）：battleParamsQueue 耗尽时
            // Dequeue 抛异常被捕获并返回 -1 —— 对任意非负阈值都判定成功，
            // 不是失败。oracle 锚点：e0242566c4335718/round-16 cp[44]
            // （t38 弯弓射虎第 7 次取随机，队列已空 → -1 < 10 → 获得
            // ExActionAgain 并再次行动；引擎原先 fail-closed 返回 100
            // 导致 actorTurn 偏差）。仅对 optional（fail-closed 命名
            // 沿用历史）路径生效；strict 路径保持 100 并照常上报缺失。
            PercentRollDecisionResolution::Missing if suppress_missing_error => return -1,
            PercentRollDecisionResolution::Missing => return 100,
            PercentRollDecisionResolution::Unscoped => {}
        }

        // Calls outside a card execution intentionally retain the original
        // untyped helper contract. Typed validation and audit identity only
        // apply while an execution scope is active.
        let value = if self.decision_tape.is_empty() {
            -1
        } else {
            self.decision_tape.remove(0)
        };
        if value < 0 && used_hexagram {
            return 0;
        }
        if value < 0 && !self.random_fallback_tape.is_empty() {
            let fallback = self.random_fallback_tape.remove(0);
            if fallback >= 100 {
                self.missing_decision("percent fallback out of range");
            }
            return fallback;
        }
        if value < 0 {
            if !suppress_missing_error {
                self.missing_decision("percent roll");
            }
            return 100;
        }
        if value >= 100 {
            self.missing_decision("percent roll replay out of range");
            return 100;
        }
        value
    }

    pub(super) fn consume_optional_decision(&mut self) -> i64 {
        if self.decision_tape.is_empty() {
            if self.fail_on_missing_decision {
                return -1;
            }
            0
        } else if self.fail_on_missing_decision {
            self.decision_tape.remove(0)
        } else {
            self.decision_tape.remove(0).max(0)
        }
    }

    pub(super) fn consume_required_decision(&mut self) -> i64 {
        if self.decision_tape.is_empty() {
            self.missing_decision("required decision");
            0
        } else {
            self.decision_tape.remove(0)
        }
    }

    pub(super) fn consume_required_negative_status_decision(&mut self) -> Option<i64> {
        let next = *self.decision_tape.first()?;
        match next {
            100 | 101 | 102 | 103 | 104 | 105 | 367 | 393 => {
                self.decision_tape.remove(0);
                Some(next)
            }
            _ => None,
        }
    }

    pub(super) fn clear_next_card_anima_cost_reduction(
        &mut self,
        actor_side: crate::model::PlayerSide,
    ) {
        let reduction = self
            .actor(actor_side)
            .turn
            .next_card_anima_cost_reduction
            .max(0);
        if reduction > 0 {
            self.actor_mut(actor_side)
                .turn
                .next_card_anima_cost_reduction = 0;
        }
    }

    pub(super) fn apply_after_card_sustain_hooks(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &CardDefinition,
        slot_index: usize,
    ) {
        let base_id = normalized_base_id(card);
        if self.actor(actor_side).fate.fortune_seek_auspicious > 0
            && ReplayState::card_has_opening_effect(base_id)
        {
            let card = card.clone();
            self.apply_opening_effect_for_card(actor_side, &card, slot_index);
            let damage = self
                .actor(actor_side)
                .fate
                .fortune_seek_auspicious_damage
                .max(0);
            if damage > 0 {
                self.apply_damage(actor_side, damage, false, false, false);
            }
            self.actor_mut(actor_side).fate.fortune_seek_auspicious -= 1;
        }
    }

    pub(super) fn consume_optional_negative_status_decision(&mut self) -> Option<i64> {
        let next = *self.decision_tape.first()?;
        match next {
            100 | 101 | 102 | 103 | 104 | 105 | 367 | 393 | -1 => {
                self.decision_tape.remove(0);
                if next < 0 {
                    None
                } else {
                    Some(next)
                }
            }
            _ => None,
        }
    }
}
