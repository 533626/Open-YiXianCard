use super::support::{
    active_neighbor_slot_index, div_ceil, has_base_card_in_deck, is_element_generated_by,
    opponent_side,
};
use super::{NegativeStatusMutationReceipt, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// Detail-entry identity for a negative-status stack, matching
    /// snapshot.rs's detail_entries keys; unknown statuses stay unrecorded and
    /// fall back to the sampling diff.
    fn negative_status_detail_key(
        status: i64,
    ) -> Option<(&'static str, &'static str, &'static str)> {
        Some(match status {
            100 => ("状态", "internalInjury", "内伤"),
            101 => ("状态", "weakness", "虚弱"),
            102 => ("状态", "flaw", "破绽"),
            103 => ("状态", "attackReduction", "减攻"),
            104 => ("状态", "entangle", "困缚"),
            105 => ("状态", "externalInjury", "外伤"),
            367 => ("状态", "meditation", "冥"),
            393 => ("状态", "lostMind", "食滞"),
            379 => ("状态", "bloodCalamity", "血光之灾"),
            _ => return None,
        })
    }

    pub(super) fn reduce_all_actor_negative_statuses(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
    ) {
        if amount <= 0 {
            return;
        }
        for status in [100, 101, 102, 103, 104, 105, 367, 393] {
            self.remove_actor_negative_status(actor_side, status, amount);
        }
    }

    pub(super) fn transfer_selected_negative_statuses(
        &mut self,
        actor_side: PlayerSide,
        count: i64,
    ) {
        for _ in 0..count.max(0) {
            let Some(status) = self.consume_optional_negative_status_decision() else {
                continue;
            };
            if self
                .remove_actor_negative_status(actor_side, status, 1)
                .applied
                != 0
            {
                self.add_actor_negative_status(opponent_side(actor_side), status, 1);
            }
        }
    }

    pub(super) fn remove_actor_negative_status(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        amount: i64,
    ) -> NegativeStatusMutationReceipt {
        let receipt = self.remove_actor_negative_status_inner(actor_side, status, amount);
        if let Some((group, key, label)) = Self::negative_status_detail_key(status) {
            self.record_mutation_receipt(
                actor_side,
                super::ReplayMutationKind::NegativeStatus,
                group,
                key,
                label,
                receipt.before,
                receipt.after,
                receipt.applied,
            );
        }
        receipt
    }

    fn remove_actor_negative_status_inner(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        amount: i64,
    ) -> NegativeStatusMutationReceipt {
        let before = self.negative_status_stack_value(actor_side, status);
        // 疯魔架势（卡 415）被动覆盖「失去负面状态」方向：
        // 每个实际移除的层数 +1 体魄（原版 ModifyBuffValue delta<0 分支，
        // BattleCharacter.cs:8711-8713）。
        let removed = match status {
            100 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.internal_injury;
                actor.status.internal_injury = (actor.status.internal_injury - amount).max(0);
                before - actor.status.internal_injury
            }
            101 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.weakness;
                actor.status.weakness = (actor.status.weakness - amount).max(0);
                before - actor.status.weakness
            }
            103 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.attack_reduction;
                actor.status.attack_reduction = (actor.status.attack_reduction - amount).max(0);
                before - actor.status.attack_reduction
            }
            102 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.flaw;
                actor.status.flaw = (actor.status.flaw - amount).max(0);
                let removed = before - actor.status.flaw;
                actor.apply_flaw_loss_hooks(removed);
                removed
            }
            104 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.entangle;
                actor.status.entangle = (actor.status.entangle - amount).max(0);
                before - actor.status.entangle
            }
            105 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.external_injury;
                actor.status.external_injury = (actor.status.external_injury - amount).max(0);
                before - actor.status.external_injury
            }
            367 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.meditation;
                actor.status.meditation = (actor.status.meditation - amount).max(0);
                let removed = before - actor.status.meditation;
                if removed > 0 {
                    self.apply_meditation_hp_delta(actor_side, -removed);
                }
                removed
            }
            393 => {
                let actor = self.actor_mut(actor_side);
                let before = actor.status.lost_mind;
                actor.status.lost_mind = (actor.status.lost_mind - amount).max(0);
                before - actor.status.lost_mind
            }
            _ => 0,
        };
        if removed > 0 {
            self.apply_feng_mo_stance_physique(actor_side, removed);
        }
        let after = self.negative_status_stack_value(actor_side, status);
        NegativeStatusMutationReceipt {
            status,
            requested: -amount,
            applied: -removed,
            before,
            after,
        }
    }

    pub(super) fn add_actor_negative_status(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        amount: i64,
    ) -> NegativeStatusMutationReceipt {
        let receipt = self.add_actor_negative_status_inner(actor_side, status, amount);
        if let Some((group, key, label)) = Self::negative_status_detail_key(status) {
            self.record_mutation_receipt(
                actor_side,
                super::ReplayMutationKind::NegativeStatus,
                group,
                key,
                label,
                receipt.before,
                receipt.after,
                receipt.applied,
            );
        }
        receipt
    }

    fn add_actor_negative_status_inner(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        mut amount: i64,
    ) -> NegativeStatusMutationReceipt {
        let requested = amount;
        let before = self.negative_status_stack_value(actor_side, status);
        if amount > 0 && matches!(status, 100 | 105) {
            amount += self
                .dream_mirage_value(
                    actor_side,
                    super::cards_dream_mirage::DreamMirageValue::SnakeShadow,
                )
                .max(0);
        }
        if status != 367 {
            let star_erosion = self.actor(actor_side).astrology.star_erosion.max(0);
            if star_erosion > 0 {
                amount += star_erosion;
                self.actor_mut(actor_side).astrology.star_erosion = 0;
            }
        }
        if status == 100 && self.actor(actor_side).status.flame_heart_urging > 0 {
            self.actor_mut(actor_side).status.flame_heart_urging -= 1;
            amount += 1;
        }
        if status == 100
            && amount > 0
            && self
                .actor(actor_side)
                .mirage_ronghui
                .mirage_internal_injury_amplifier_turns
                > 0
        {
            amount += 2;
        }
        // BattleCharacter.ModifyBuffValue（build 24666769:8558-8566）：
        // 206 的拳/棍架势先把外伤/减攻 delta 减 1，随后 :8582-8588
        // 辟邪才按剩余 delta 消耗。若架势已把 1 层抵成 0，辟邪不能白耗。
        if amount > 0
            && self.actor(actor_side).identity.talents.contains(&206)
            && ((status == 103 && self.actor(actor_side).beng.gun_stance > 0)
                || (status == 105 && self.actor(actor_side).beng.quan_stance > 0))
        {
            amount -= 1;
        }
        let exorcism = self.actor(actor_side).fate.exorcism.max(0);
        if exorcism > 0 && amount > 0 {
            let prevented = amount.min(exorcism);
            amount -= prevented;
            self.actor_mut(actor_side).fate.exorcism -= prevented;
        }
        if amount <= 0 {
            return NegativeStatusMutationReceipt {
                status,
                requested,
                applied: 0,
                before,
                after: before,
            };
        }
        let actual = match status {
            100 => {
                self.actor_mut(actor_side).status.internal_injury += amount;
                amount
            }
            101 => {
                self.actor_mut(actor_side).status.weakness += amount;
                amount
            }
            103 => {
                self.actor_mut(actor_side).status.attack_reduction += amount;
                amount
            }
            102 => {
                self.actor_mut(actor_side).status.flaw += amount;
                amount
            }
            104 => {
                self.actor_mut(actor_side).status.entangle += amount;
                amount
            }
            105 => {
                let actual = self.add_external_injury(actor_side, amount);
                self.apply_feng_mo_stance_physique(actor_side, actual);
                self.apply_dream_mirage_negative_status_gain_hooks(actor_side, status, actual);
                self.apply_ronghui_negative_status_mirror(actor_side, status, actual);
                self.apply_yin_fu_jue_zhen_reflect(actor_side, status, actual);
                return NegativeStatusMutationReceipt {
                    status,
                    requested,
                    applied: actual,
                    before,
                    after: self.negative_status_stack_value(actor_side, status),
                };
            }
            367 => {
                // 原版 ModifyBuffValue 对 Negative 类 delta 的钩子顺序
                // （BattleCharacter.cs:8634-8729）：talent 177 回血 →
                // YinFuJueZhen 反伤（豁免 Min）→ 415 疯魔架势
                // ModifyTiPo(abs(delta)) → Min 分支 ModifyHp(abs(delta)*3)。
                // 415 先涨体魄/生命上限，Min 回血按更高上限截断
                // （oracle 锚点：a94331ac41eb745f/round-09 cp2 p1.hp 80：
                // maxHp 77→80 后回 9 截断到 80；引擎原 77 = 先回 9 被 77
                // 截断再涨上限）。整段自带尾钩并 return，避免共享尾部
                // 重复触发 415。
                self.apply_dream_mirage_negative_status_gain_hooks(actor_side, status, amount);
                self.apply_negative_status_gain_hooks(actor_side, amount);
                self.apply_feng_mo_stance_physique(actor_side, amount);
                self.actor_mut(actor_side).status.meditation += amount;
                self.apply_meditation_hp_delta(actor_side, amount);
                self.apply_ronghui_negative_status_mirror(actor_side, status, amount);
                self.apply_yin_fu_jue_zhen_reflect(actor_side, status, amount);
                return NegativeStatusMutationReceipt {
                    status,
                    requested,
                    applied: amount,
                    before,
                    after: self.negative_status_stack_value(actor_side, status),
                };
            }
            393 => {
                self.actor_mut(actor_side).status.lost_mind += amount;
                amount
            }
            _ => 0,
        };
        // BattleCharacter.ModifyBuffValue (build-24610558, lines 8822-8825):
        // 辟邪之类 paths can grant anima to the opposing character when this
        // side gains 虚弱.  FateStrategy 405 is the observed HF witness used
        // by 阴符绝阵 (card 429); apply it after the weakness actually lands.
        if status == 101
            && actual > 0
            && self
                .actor(opponent_side(actor_side))
                .identity
                .fate_strategies
                .contains(&405)
        {
            self.gain_anima(opponent_side(actor_side), 1);
        }
        self.apply_dream_mirage_negative_status_gain_hooks(actor_side, status, actual);
        self.apply_negative_status_gain_hooks(actor_side, actual);
        self.apply_feng_mo_stance_physique(actor_side, actual);
        self.apply_ronghui_negative_status_mirror(actor_side, status, actual);
        self.apply_yin_fu_jue_zhen_reflect(actor_side, status, actual);
        NegativeStatusMutationReceipt {
            status,
            requested,
            applied: actual,
            before,
            after: self.negative_status_stack_value(actor_side, status),
        }
    }

    /// 阴符绝阵（卡 429）持续反伤：BattleCharacter.cs:8644-8648 —
    /// ModifyBuffValue 对 Negative 类 buff 的 delta > 0 时（豁免
    /// BuffType.Min「冥」367），造成
    /// delta × YinFuJueZhen 层数的 ReflectDamage（skipWoundCheck）。原版虽
    /// 在 `defaultOpponentTarget` 接收调用，但显式 `dst` 是 `this`，所以
    /// 受击方仍是刚获得负面状态、同时持有阴符的一方。
    /// delta 取实际生效值（扣除 206 架势减免 / 辟邪后），与同处
    /// apply_negative_status_gain_hooks 的 talent 177 语义一致；
    /// 位置在原版 talent 177 回血（8715-8718）之后、加防 hook 之前。
    /// 注：引擎侧旧 build 无困龙绝阵（KunLongJueZhen）实现——
    /// 24589371 起 BuffConfig 759 已更名 YinFuJueZhen，本实现只按
    /// 新 build 语义落地。
    fn apply_yin_fu_jue_zhen_reflect(&mut self, actor_side: PlayerSide, status: i64, actual: i64) {
        if actual <= 0 || status == 367 {
            return;
        }
        let stacks = self.actor(actor_side).status.yin_fu_jue_zhen.max(0);
        if stacks <= 0 {
            return;
        }
        let damage = actual * stacks;
        if damage > 0 {
            // BattleCharacter.cs:8646-8647 calls
            // `defaultOpponentTarget.ApplyDamage(this, ...)`: the opponent
            // object applies damage to `this`, i.e. the character that just
            // gained the negative status and holds YinFuJueZhen.  Do not use
            // the ordinary source->opponent helper here; that would assign
            // the reflect to the wrong side.
            self.apply_damage(opponent_side(actor_side), damage, false, false, false);
        }
    }

    /// 疯魔架势（卡 415）被动：BattleCharacter.cs:8711-8713 —
    /// ModifyBuffValue 对 Negative 类 buff 的 delta != 0 时，若牌组含 415，
    /// 则 ModifyTiPo(abs(delta))。覆盖获得与失去两个方向；amount 取实际
    /// 生效值（扣除 206 架势减免 / 辟邪后），与 8711 处 delta 语义一致。
    pub(super) fn apply_feng_mo_stance_physique(&mut self, actor_side: PlayerSide, amount: i64) {
        if amount <= 0 || !has_base_card_in_deck(self.actor(actor_side), 415) {
            return;
        }
        self.apply_physique_amount(actor_side, amount);
    }

    fn apply_negative_status_gain_hooks(&mut self, actor_side: PlayerSide, actual: i64) {
        if actual > 0 && self.actor(actor_side).identity.talents.contains(&177) {
            self.modify_actor_hp(actor_side, actual, false, false);
        }
    }

    pub(super) fn apply_meditation_hp_delta(&mut self, actor_side: PlayerSide, delta: i64) {
        let amount = delta.abs() * 3;
        if amount <= 0 {
            return;
        }
        let hp_delta = if self.actor(actor_side).identity.character_id == Some(4_000_003) {
            amount
        } else {
            -amount
        };
        self.modify_actor_hp(actor_side, hp_delta, false, false);
    }

    pub(super) fn modify_actor_negative_status(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        delta: i64,
    ) -> NegativeStatusMutationReceipt {
        if delta > 0 {
            return self.add_actor_negative_status(actor_side, status, delta);
        }
        if delta < 0 {
            return self.remove_actor_negative_status(actor_side, status, -delta);
        }
        let before = self.negative_status_stack_value(actor_side, status);
        NegativeStatusMutationReceipt {
            status,
            requested: 0,
            applied: 0,
            before,
            after: before,
        }
    }

    pub(super) fn negative_status_stack_value(&self, actor_side: PlayerSide, status: i64) -> i64 {
        let actor = self.actor(actor_side);
        match status {
            100 => actor.status.internal_injury.max(0),
            101 => actor.status.weakness.max(0),
            102 => actor.status.flaw.max(0),
            103 => actor.status.attack_reduction.max(0),
            104 => actor.status.entangle.max(0),
            105 => actor.status.external_injury.max(0),
            367 => actor.status.meditation.max(0),
            393 => actor.status.lost_mind.max(0),
            379 => actor.status.blood_calamity.max(0),
            _ => 0,
        }
    }

    /// 凶象/血光之灾 marker（buff 379，Neutral）专用入口：只做计数写入，
    /// 不走 Negative 类的共享钩子（talent 177/415、阴符绝阵等）。
    pub(super) fn modify_blood_calamity(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
    ) -> NegativeStatusMutationReceipt {
        let receipt = self.modify_blood_calamity_inner(actor_side, delta);
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::NegativeStatus,
            "状态",
            "bloodCalamity",
            "血光之灾",
            receipt.before,
            receipt.after,
            receipt.applied,
        );
        receipt
    }

    fn modify_blood_calamity_inner(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
    ) -> NegativeStatusMutationReceipt {
        let before = self.negative_status_stack_value(actor_side, 379);
        let after = (before + delta).max(0);
        self.actor_mut(actor_side).status.blood_calamity = after;
        NegativeStatusMutationReceipt {
            status: 379,
            requested: delta,
            applied: after - before,
            before,
            after,
        }
    }

    pub(super) fn known_negative_status_count(&self, actor_side: PlayerSide) -> i64 {
        self.negative_status_stack_count(actor_side)
    }

    pub(super) fn negative_status_stack_count(&self, actor_side: PlayerSide) -> i64 {
        let actor = self.actor(actor_side);
        actor.status.internal_injury.max(0)
            + actor.status.weakness.max(0)
            + actor.status.attack_reduction.max(0)
            + actor.status.flaw.max(0)
            + actor.status.entangle.max(0)
            + actor.status.external_injury.max(0)
            + actor.status.meditation.max(0)
            + actor.status.lost_mind.max(0)
    }

    pub(super) fn copy_negative_statuses_to_target(
        &mut self,
        actor_side: PlayerSide,
        amount_per_type: i64,
    ) {
        if amount_per_type <= 0 {
            return;
        }
        let statuses = self.negative_status_types_present(actor_side);
        let target_side = opponent_side(actor_side);
        for status in statuses {
            self.add_actor_negative_status(target_side, status, amount_per_type);
        }
    }

    pub(super) fn negative_status_types_present(&self, actor_side: PlayerSide) -> Vec<i64> {
        let actor = self.actor(actor_side);
        let mut statuses = Vec::new();
        if actor.status.internal_injury > 0 {
            statuses.push(100);
        }
        if actor.status.weakness > 0 {
            statuses.push(101);
        }
        if actor.status.flaw > 0 {
            statuses.push(102);
        }
        if actor.status.attack_reduction > 0 {
            statuses.push(103);
        }
        if actor.status.entangle > 0 {
            statuses.push(104);
        }
        if actor.status.external_injury > 0 {
            statuses.push(105);
        }
        if actor.status.meditation > 0 {
            statuses.push(367);
        }
        if actor.status.lost_mind > 0 {
            statuses.push(393);
        }
        statuses
    }

    pub(super) fn add_external_injury(&mut self, actor_side: PlayerSide, amount: i64) -> i64 {
        if amount <= 0 {
            return 0;
        }
        // 206 架势减免已在 add_actor_negative_status 中按原版顺序先于
        // 辟邪完成；这里只负责实际写入与共享增益钩子。
        let actual = amount;
        if actual > 0 {
            self.actor_mut(actor_side).status.external_injury += actual;
            self.apply_negative_status_gain_hooks(actor_side, actual);
        }
        actual
    }

    pub(super) fn activate_element(&mut self, actor_side: PlayerSide, element: super::Element) {
        let first_activation = !self
            .actor(actor_side)
            .elements
            .activated_elements
            .contains(&element);
        if first_activation {
            self.actor_mut(actor_side)
                .elements
                .activated_elements
                .push(element);
            self.trigger_five_elements_primordial_spirit(actor_side, element);
            if self.actor(actor_side).identity.talents.contains(&202) {
                self.gain_anima(actor_side, 1);
            }
        }
        self.increment_element_activation_count(actor_side, element);
        self.apply_talent_79_element_activation_reward(actor_side);
        let activation_count = match element {
            super::Element::Metal => self.actor(actor_side).elements.activated_metal,
            super::Element::Water => self.actor(actor_side).elements.activated_water,
            super::Element::Wood => self.actor(actor_side).elements.activated_wood,
            super::Element::Fire => self.actor(actor_side).elements.activated_fire,
            super::Element::Earth => self.actor(actor_side).elements.activated_earth,
        };
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&310)
            && self
                .actor(actor_side)
                .fate
                .five_elements_gathering_triggered
                <= 0
            && activation_count > 1
        {
            self.actor_mut(actor_side)
                .fate
                .five_elements_gathering_triggered = 1;
            self.gain_anima(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .elements
            .primordial_infinity_formation
            > 0
            && self.actor(actor_side).turn.extra_actions <= 0
            && self.actor(actor_side).turn.action_again_count < 1
        {
            self.actor_mut(actor_side)
                .elements
                .primordial_infinity_formation -= 1;
            self.modify_extra_actions(actor_side, 1);
        }
        // 原版 ModifyBuffValue 激活五行时序（BattleCharacter.cs:8835+）：
        // talent 138 的 ModifyHp/ModifyMaxHp 在镇印 ApplyDamage 之前。
        self.apply_talent_138_element_activation(actor_side);
        if self.actor(actor_side).elements.seal_suppressing_mindset > 0 {
            self.trigger_seal_suppressing_mindset(actor_side);
        }
        if element == super::Element::Wood && self.actor(actor_side).identity.talents.contains(&200)
        {
            self.modify_actor_max_hp(actor_side, 2);
        }
        // 花沁蕊 fate 411（BattleCharacter.cs:8987-8997，ModifyBuffValue 内
        // 的五行钩子）：激活水灵时同步激活土灵；激活土灵时 ModifyDef
        // (FateStrategyConfig(411).otherParams[0] = 2)。原版位置在
        // talent 138/202 之后、ModifyBuffValue 尾部，重入的土激活走完整
        // 语义路径（talent 79/112 等钩子照常触发）。oracle 锚点：
        // mirror-32299000 dcc66e8dfc226124/round-10 cp0（原版 p2 buffs
        // 241 JiHuoTuLing=1 且 def=1 = (0+2) 经 turn1 防御减半）。
        self.apply_fate_strategy_411_element_hook(actor_side, element);
        self.apply_synthetic_ding_feng_bo_activation_damage(actor_side);
    }

    /// BattleCharacter.cs:8987-8997 fate 411 hook（见 activate_element 注释）。
    fn apply_fate_strategy_411_element_hook(
        &mut self,
        actor_side: PlayerSide,
        element: super::Element,
    ) {
        if !self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&411)
        {
            return;
        }
        match element {
            super::Element::Water => {
                // 重入走完整语义路径，原版 ModifyBuffValue(JiHuoTuLing, 1)
                // 同样跑全部激活钩子（包括本函数的土分支 → ModifyDef(2)）。
                self.activate_element(actor_side, super::Element::Earth);
            }
            super::Element::Earth => {
                self.gain_defense(actor_side, 2);
            }
            _ => {}
        }
    }

    fn apply_talent_79_element_activation_reward(&mut self, actor_side: PlayerSide) {
        let talents = &self.actor(actor_side).identity.talents;
        let defense = if talents.contains(&79) { 3 } else { 0 };
        let hp = if talents.contains(&10_079) { 3 } else { 0 };
        let sharpness = if talents.contains(&20_079) { 4 } else { 0 };
        let anima = if talents.contains(&30_079) { 2 } else { 0 };

        if defense > 0 {
            self.gain_defense(actor_side, defense);
        }
        if hp > 0 {
            self.modify_actor_hp(actor_side, hp, false, false);
        }
        if sharpness > 0 {
            self.gain_sharpness(actor_side, sharpness);
        }
        if anima > 0 {
            self.gain_anima(actor_side, anima);
        }
    }

    pub(super) fn increment_element_activation_count(
        &mut self,
        actor_side: PlayerSide,
        element: super::Element,
    ) {
        let (key, label, before) = match element {
            super::Element::Metal => (
                "activatedMetal",
                "金灵激活",
                self.actor(actor_side).elements.activated_metal,
            ),
            super::Element::Water => (
                "activatedWater",
                "水灵激活",
                self.actor(actor_side).elements.activated_water,
            ),
            super::Element::Wood => (
                "activatedWood",
                "木灵激活",
                self.actor(actor_side).elements.activated_wood,
            ),
            super::Element::Fire => (
                "activatedFire",
                "火灵激活",
                self.actor(actor_side).elements.activated_fire,
            ),
            super::Element::Earth => (
                "activatedEarth",
                "土灵激活",
                self.actor(actor_side).elements.activated_earth,
            ),
        };
        let after = before + 1;
        match element {
            super::Element::Metal => self.actor_mut(actor_side).elements.activated_metal = after,
            super::Element::Water => self.actor_mut(actor_side).elements.activated_water = after,
            super::Element::Wood => self.actor_mut(actor_side).elements.activated_wood = after,
            super::Element::Fire => self.actor_mut(actor_side).elements.activated_fire = after,
            super::Element::Earth => self.actor_mut(actor_side).elements.activated_earth = after,
        }
        self.record_counter_transition(actor_side, "五行", key, label, before, after);
    }

    /// Direct `ModifyBuffValue(JiHuo*)` writes still make the element active
    /// for later `CheckWuXing`/card predicates, but do not run the semantic
    /// `ActiveWuXing` callbacks (for example Talent 138's HP loss).
    pub(super) fn add_direct_element_activation(
        &mut self,
        actor_side: PlayerSide,
        element: super::Element,
    ) {
        if !self
            .actor(actor_side)
            .elements
            .activated_elements
            .contains(&element)
        {
            self.actor_mut(actor_side)
                .elements
                .activated_elements
                .push(element);
        }
        self.increment_element_activation_count(actor_side, element);
        // Card_136.cs writes JiHuoShuiLing/JiHuoHuoLing directly.  The
        // ModifyBuffValue path still invokes Talent 138, but deliberately
        // skips ActiveWuXing's generated-element and other callbacks.
        self.apply_talent_138_element_activation(actor_side);
        // fate 411 钩子在原版位于 ModifyBuffValue 内，直接写入同样触发
        // （与 6_000_... 直接写 JiHuo* 的路径一致）；重入的土激活走完整
        // 语义路径（原版重入 ModifyBuffValue(JiHuoTuLing, 1) 同样完整）。
        self.apply_fate_strategy_411_element_hook(actor_side, element);
    }

    pub(super) fn apply_talent_138_element_activation(&mut self, actor_side: PlayerSide) {
        // Talent 138's verified build-24610558 contract is -3 HP and -3 max
        // HP per JiHuo* buff gain (TalentConfig otherParams[1]=3).
        if self.actor(actor_side).identity.talents.contains(&138) {
            let amount = 3;
            self.modify_target_hp(actor_side, -amount);
            self.modify_target_max_hp(actor_side, -amount);
        }
    }

    fn trigger_five_elements_primordial_spirit(
        &mut self,
        actor_side: PlayerSide,
        element: super::Element,
    ) {
        if !self.actor(actor_side).identity.talents.contains(&112) {
            return;
        }
        match element {
            super::Element::Metal => {
                self.gain_sharpness(actor_side, 4);
            }
            super::Element::Water => {
                self.gain_water_momentum(actor_side, 2);
            }
            super::Element::Wood => {
                self.gain_attack_bonus(actor_side, 1);
            }
            super::Element::Fire => {
                self.modify_target_hp(actor_side, -7);
                self.modify_target_max_hp(actor_side, -7);
            }
            super::Element::Earth => {
                self.gain_defense(actor_side, 12);
            }
        }
    }

    pub(super) fn trigger_seal_suppressing_mindset(&mut self, actor_side: PlayerSide) {
        let amount = self.actor(actor_side).elements.seal_suppressing_mindset;
        if amount <= 0 {
            return;
        }
        self.apply_damage(actor_side, amount, false, false, false);
    }

    pub(super) fn is_element_activated(
        &self,
        actor_side: PlayerSide,
        required: super::Element,
    ) -> bool {
        let actor = self.actor(actor_side);
        actor.elements.activated_elements.contains(&required)
            || actor.elements.last_element == Some(required)
            || actor.elements.last_element.is_some_and(|last| {
                is_element_generated_by(last, required, actor.identity.talents.contains(&137))
            })
    }

    pub(super) fn check_wu_xing(&self, actor_side: PlayerSide, required: super::Element) -> bool {
        let actor = self.actor(actor_side);
        let activation_count = match required {
            super::Element::Metal => actor.elements.activated_metal,
            super::Element::Water => actor.elements.activated_water,
            super::Element::Wood => actor.elements.activated_wood,
            super::Element::Fire => actor.elements.activated_fire,
            super::Element::Earth => actor.elements.activated_earth,
        };
        self.is_element_activated(actor_side, required)
            || activation_count > 0
            || actor.elements.long_ma_spirit > 0
            || super::support::active_deck_cards(actor)
                .any(|card| matches!(card.id, 7_030_077 | 7_040_077))
    }

    /// BattleCharacter.GetWuXingActiveNumber: sum of activated-element buff stacks.
    /// Mirrors engine-ts `getWuXingActiveNumber` — not `activated_elements.len()`.
    pub(super) fn wu_xing_active_number(&self, actor_side: PlayerSide) -> i64 {
        let elements = &self.actor(actor_side).elements;
        elements.activated_metal
            + elements.activated_water
            + elements.activated_wood
            + elements.activated_fire
            + elements.activated_earth
    }

    pub(super) fn add_following_star_slots(
        &mut self,
        actor_side: PlayerSide,
        current_slot: usize,
        count: i64,
    ) {
        let mut slot = current_slot;
        for _ in 0..count.max(0) {
            let Some(next_slot) = active_neighbor_slot_index(self.actor(actor_side), slot, 1)
            else {
                break;
            };
            slot = next_slot;
            if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                self.gain_anima(actor_side, 1);
            } else {
                self.actor_mut(actor_side).astrology.star_slots.push(slot);
            }
        }
    }

    pub(super) fn check_rear_move(&mut self, actor_side: PlayerSide, was_slot_used: bool) -> bool {
        self.actor_mut(actor_side).fate.used_rear_move_check += 1;
        if !was_slot_used {
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&332)
            {
                self.add_actor_negative_status(opponent_side(actor_side), 101, 1);
            }
            // CardActionBase.CheckHouZhao scans all four 后发制人 talent
            // ranks independently. Keep every ModifyDef/ModifyMaxHp/ModifyHp
            // transaction separate so HP-change hooks fire once per rank.
            let rear_move_talent_amounts = [(71, 2), (10_071, 3), (20_071, 4), (30_071, 5)]
                .into_iter()
                .filter_map(|(talent, amount)| {
                    self.actor(actor_side)
                        .identity
                        .talents
                        .contains(&talent)
                        .then_some(amount)
                })
                .collect::<Vec<_>>();
            for amount in rear_move_talent_amounts {
                self.gain_defense(actor_side, amount);
                self.modify_actor_max_hp(actor_side, amount);
                self.modify_actor_hp(actor_side, amount, false, false);
            }
        }
        let mut succeeded = was_slot_used
            || super::cards_synthetic_oracle_verified_secret_misc::first_strike_enables_rear_move(
                self.actor(actor_side),
            );
        if !succeeded && self.actor(actor_side).fate.next_rear_move_bypass > 0 {
            self.actor_mut(actor_side).fate.next_rear_move_bypass -= 1;
            succeeded = true;
        }
        if !succeeded && self.actor(actor_side).identity.talents.contains(&108) {
            succeeded = self.consume_percent_roll(actor_side) < 1;
        }
        self.actor_mut(actor_side).fate.rear_move_succeeded |= succeeded;
        succeeded
    }

    pub(super) fn activate_element_by_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if let Some(element) = super::support::element_from_card(card) {
            self.activate_element(actor_side, element);
        }
    }

    pub(super) fn apply_damage(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
        apply_wound_bonus: bool,
        ignore_defense: bool,
        shatter_defense: bool,
    ) {
        let target = opponent_side(actor_side);
        self.apply_damage_to_inner(
            actor_side,
            target,
            amount,
            apply_wound_bonus,
            ignore_defense,
            shatter_defense,
            0,
            100,
        );
    }

    pub(super) fn apply_attack_damage(
        &mut self,
        actor_side: PlayerSide,
        amount: i64,
        post_defense_wound_bonus: i64,
        post_wound_multiplier_percent: i64,
        ignore_defense: bool,
        shatter_defense: bool,
    ) {
        self.apply_damage_to_inner(
            actor_side,
            opponent_side(actor_side),
            amount,
            true,
            ignore_defense,
            shatter_defense,
            post_defense_wound_bonus,
            post_wound_multiplier_percent,
        );
    }

    pub(super) fn apply_damage_to(
        &mut self,
        actor_side: PlayerSide,
        target: PlayerSide,
        amount: i64,
        apply_wound_bonus: bool,
        ignore_defense: bool,
        shatter_defense: bool,
    ) {
        self.apply_damage_to_inner(
            actor_side,
            target,
            amount,
            apply_wound_bonus,
            ignore_defense,
            shatter_defense,
            0,
            100,
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn apply_damage_to_inner(
        &mut self,
        actor_side: PlayerSide,
        target: PlayerSide,
        amount: i64,
        apply_wound_bonus: bool,
        ignore_defense: bool,
        shatter_defense: bool,
        post_defense_wound_bonus: i64,
        post_wound_multiplier_percent: i64,
    ) {
        let actor = self.actor(actor_side);
        let metal_cauldron_drop = actor.elements.metal_cauldron_drop;
        let shatter_defense =
            shatter_defense || (apply_wound_bonus && actor.status.leaf_blade_flower > 0);
        let force_wound = apply_wound_bonus
            && (actor.identity.talents.contains(&67)
                || actor.turn.next_attack_wound_bonus > 0
                || actor.turn.guaranteed_wound > 0
                || actor.elements.long_ma_spirit > 0);
        let mut remaining = amount.max(0);
        let dream_mirage_reflected_incoming =
            if !apply_wound_bonus && remaining > 0 && self.dream_mirage_reflection_active(target) {
                let incoming = remaining;
                remaining = (remaining * 75 / 100).max(1);
                incoming
            } else {
                0
            };
        if !apply_wound_bonus && remaining > 0 && self.actor(target).elements.metal_iron_bone > 0 {
            remaining = (remaining - 5).max(1);
        }
        if !apply_wound_bonus
            && remaining > 0
            && self.actor(target).fate.dismantle_move > 0
            && self.actor(target).beng.quan_stance > 0
        {
            remaining = (remaining / 2).max(1);
        }
        let mut defense_absorbed = 0;
        if !ignore_defense {
            let target_defense = self.actor(target).core.defense;
            if shatter_defense && target_defense > 0 && remaining > 0 {
                remaining = if remaining * 2 >= target_defense {
                    remaining + div_ceil(target_defense, 2)
                } else {
                    remaining * 2
                };
            }
            let absorbed = target_defense.min(remaining);
            if absorbed > 0 {
                self.lose_defense(target, absorbed);
                self.actor_mut(target)
                    .prevention
                    .hp_loss_prevented_by_defense += absorbed;
            }
            defense_absorbed = absorbed;
            remaining -= absorbed;
        }
        if remaining <= 0 && !force_wound {
            if dream_mirage_reflected_incoming > 0 {
                self.apply_dream_mirage_reflected_life_loss(
                    actor_side,
                    dream_mirage_reflected_incoming,
                );
            }
            return;
        }
        let wound_triggered = apply_wound_bonus
            && (force_wound
                || (remaining > 0
                    && self.actor(target).core.guard <= 0
                    && self.actor(target).fate.graft_flowers_to_tree <= 0));
        let wound = if wound_triggered {
            self.actor(target).status.external_injury.max(0)
        } else {
            0
        };
        let next_attack_wound_bonus = if wound_triggered {
            self.actor(actor_side).turn.next_attack_wound_bonus.max(0)
        } else {
            0
        };
        if next_attack_wound_bonus > 0 {
            self.actor_mut(actor_side).turn.next_attack_wound_bonus = 0;
        }
        remaining += wound
            + if wound_triggered {
                post_defense_wound_bonus.max(0) + next_attack_wound_bonus
            } else {
                0
            };
        if wound_triggered && metal_cauldron_drop > 0 {
            remaining *= 2;
        }
        if wound_triggered && post_wound_multiplier_percent != 100 {
            remaining = remaining * post_wound_multiplier_percent.max(0) / 100;
        }
        let hp_receipt = self.mutate_actor_hp(target, -remaining, false, false);
        if hp_receipt.prevention == Some(super::HpMutationPrevention::Guard) && defense_absorbed > 0
        {
            // Defense is still spent before guard in the original pipeline, but it
            // has no marginal HP value on this hit: without that defense, the same
            // guard layer would cancel the entire remaining loss. Keep the battle
            // mutation untouched and only retract the derived prevention telemetry.
            self.actor_mut(target)
                .prevention
                .hp_loss_prevented_by_defense -= defense_absorbed;
        }
        if dream_mirage_reflected_incoming > 0 {
            self.apply_dream_mirage_reflected_life_loss(
                actor_side,
                dream_mirage_reflected_incoming,
            );
        }
    }
}
