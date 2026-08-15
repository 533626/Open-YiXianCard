use super::cards_dream_mirage::DreamMirageValue;
use super::original_config::original_card_desc_contains_rear_move;
use super::support::{
    has_base_card_in_deck, is_cloud_sword, is_effective_beng_quan_card, is_sword_card,
    is_sword_formation_card, normalized_base_id, opponent_side,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

const LOW_DREAM_BENG_TIAN_IDS: [i64; 3] = [10_000_080, 10_010_080, 10_020_080];
const HIGH_DREAM_BENG_TIAN_IDS: [i64; 2] = [10_030_080, 10_040_080];
const HIGH_DREAM_STAR_SHIFT_IDS: [i64; 2] = [4_030_079, 4_040_079];
const HIGH_DREAM_STAR_POINT_IDS: [i64; 2] = [4_030_089, 4_040_089];
const DREAM_GUARANTEED_WOUND_ADJACENT_IDS: [i64; 3] = [1_020_067, 1_030_067, 1_040_067];
const HIGH_DREAM_BEARING_WEIGHT_IDS: [i64; 2] = [10_030_083, 10_040_083];
const HIGH_DREAM_FORGE_FIST_IDS: [i64; 2] = [10_030_087, 10_040_087];
// CardActionBase.CheckAdjacentEffectsBeforeCard (CardActionBase.cs:2019-2027):
// 相邻格为任意档梦•浑天印（7000075-7040075）时，先按本牌名字激活其所属
// 五行（ActiveWuXingInName）。oracle 锚点：hf-32308000
// 8a5572baa414efc1/round-12 cp8（土灵•扬尘邻 7040075，原版先激活土灵 →
// 镇印心法 3 伤被护体 2→0 全挡；引擎未激活 → 护体剩 1）。
const DREAM_HUN_TIAN_YIN_IDS: [i64; 5] = [7_000_075, 7_010_075, 7_020_075, 7_030_075, 7_040_075];

pub(super) struct DreamMirageAttackPreparation {
    pub(super) flat_bonus: i64,
    pub(super) replaces_percent_momentum: bool,
}

impl ReplayState {
    /// BattleCharacter.Attack:11300-11471. Run exactly once before every
    /// attack segment and before the ordinary percentage-Momentum branch.
    pub(super) fn prepare_dream_mirage_attack_segment(
        &mut self,
        actor_side: PlayerSide,
    ) -> DreamMirageAttackPreparation {
        if self.dream_mirage_value(actor_side, DreamMirageValue::MomentumBeforeEveryAttack) > 0 {
            self.modify_momentum(actor_side, 1);
        }
        // BattleCharacter.CalculateAttack checks HasCardInDeck(10000084), so
        // 梦·威震四方 changes Momentum into flat attack even before its slot is
        // used. This applies to every rarity, not only the top-rarity sustain
        // marker set by the card handler.
        let replaces = has_base_card_in_deck(self.actor(actor_side), 10_000_084)
            || self.dream_mirage_value(actor_side, DreamMirageValue::FlatMomentumAttack) > 0;
        if !replaces {
            return DreamMirageAttackPreparation {
                flat_bonus: 0,
                replaces_percent_momentum: false,
            };
        }
        let multiplier = self.actor(actor_side).beng.momentum_multiplier.max(1);
        let mut flat_bonus = self.actor(actor_side).beng.momentum.max(0) * multiplier;
        if self.actor(actor_side).beng.momentum > 0 {
            self.modify_momentum(actor_side, -1);
        }
        // BattleCharacter.CalculateAttack HasCardInDeck(10000084) 平值分支
        // （BattleCharacter.cs:11583-11584）：玄心斩魄（buff XuanXinZhanPo →
        // mystic_soul）把自身负面状态总层数作为平值加攻
        // （num += GetDebuffCount() * QiShiBeiLv），与气势同走平值，不走
        // 11701-11706 的百分比分支（combat_core 里当 replaces 生效时跳过）。
        // oracle 锚点：hf-latest-32308000-16f9c778 05875ccb9da9a58e/round-15
        // cp9（卡组含梦•威震四方 10040084 → GetCardBaseId=10000084：
        // 8+8 气势平值+10 负面层数 = 26 伤；原引擎 16×200% = 32 多 6）。
        if self.actor(actor_side).status.mystic_soul > 0 {
            flat_bonus += self.negative_status_stack_count(actor_side) * multiplier;
        }
        DreamMirageAttackPreparation {
            flat_bonus,
            replaces_percent_momentum: true,
        }
    }

    pub(super) fn dream_mirage_previous_card_debuff_attack_bonus(
        &self,
        actor_side: PlayerSide,
        slot: usize,
    ) -> i64 {
        let next = self.dream_mirage_runtime_next_grid(actor_side, slot);
        self.actor(actor_side)
            .deck
            .slots
            .get(next)
            .filter(|entry| matches!(entry.card.id, 10_030_077 | 10_040_077))
            .map_or(0, |_| self.negative_status_stack_count(actor_side) / 2)
    }

    pub(super) fn dream_mirage_reflection_active(&self, actor_side: PlayerSide) -> bool {
        self.dream_mirage_value(actor_side, DreamMirageValue::DreamReflection) > 0
    }

    pub(super) fn apply_dream_mirage_reflected_life_loss(
        &mut self,
        actor_side: PlayerSide,
        incoming: i64,
    ) {
        let reflected = (incoming.max(0) + 3) / 4;
        if reflected > 0 {
            self.modify_actor_hp(actor_side, -reflected, false, false);
        }
    }

    pub(super) fn consume_dream_mirage_return_sharpness(
        &mut self,
        actor_side: PlayerSide,
        sharpness: i64,
    ) {
        if sharpness <= 0
            || self.dream_mirage_value(actor_side, DreamMirageValue::ReturnSharpness) <= 0
        {
            return;
        }
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::ReturnSharpness, -1);
        self.gain_sharpness(actor_side, sharpness);
    }

    /// BattleCharacter.ApplyDamage:10859-10870. This uses actual HP lost by
    /// the current attack and is disabled during after-action hooks.
    pub(super) fn apply_dream_mirage_forge_fist_damage_to_physique(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
        hp_lost: i64,
    ) {
        let sustained = self.dream_mirage_value(actor_side, DreamMirageValue::DreamForgeFist) > 0;
        let high_current = self
            .actor(actor_side)
            .deck
            .slots
            .get(slot)
            .is_some_and(|entry| HIGH_DREAM_FORGE_FIST_IDS.contains(&entry.card.id));
        if !sustained && !high_current {
            return;
        }
        // The original BuffType.MengDuanQuan is consumed by the next card
        // with an attack even when every segment is absorbed by defense. Its
        // physique amount still uses actual HP lost, so keep the two decisions
        // separate: arm consumption on any sustained attack, then stop before
        // the gain calculation when there is no HP loss or this is an after-action attack.
        if sustained && !self.active_effect_after_action() {
            let current =
                self.dream_mirage_value(actor_side, DreamMirageValue::DreamForgeFistConsumed);
            if current <= 0 {
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::DreamForgeFistConsumed,
                    1,
                );
            }
        }
        if hp_lost <= 0 || self.active_effect_after_action() {
            return;
        }
        let physique = hp_lost * 40 / 100;
        if physique > 0 {
            self.apply_physique_amount(actor_side, physique);
        }
    }

    /// CardActionBase.OnAfterExecuted:3774-3777. The consumed persistent
    /// marker settles before any ordinary/common follow-up attack.
    pub(super) fn complete_dream_mirage_forge_fist_card(&mut self, actor_side: PlayerSide) {
        let consumed = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamForgeFistConsumed)
            .max(0);
        if consumed <= 0 {
            return;
        }
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamForgeFist, -consumed);
        self.modify_dream_mirage_value(
            actor_side,
            DreamMirageValue::DreamForgeFistConsumed,
            -consumed,
        );
    }

    /// CardActionBase.CheckAdjacentEffectsBeforeCard:2000-2054,2949-2958.
    /// Despite the similar name, the original calls this private helper from
    /// OnBeforeExecuted, once per successful ExecuteEffect repetition.
    pub(super) fn apply_dream_mirage_before_card_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        for adjacent in self.dream_mirage_adjacent_cards(actor_side, slot) {
            // 梦•浑天印被动在相邻牌使用时最先结算（原版
            // CheckAdjacentEffectsBeforeCard 的 foreach 内第一分支）。
            if DREAM_HUN_TIAN_YIN_IDS.contains(&adjacent.id) {
                self.activate_element_by_card(actor_side, card);
            }
            if HIGH_DREAM_STAR_SHIFT_IDS.contains(&adjacent.id) {
                self.modify_star_power(actor_side, 1);
                if adjacent.id == 4_040_079 && card.name.contains("星弈") {
                    self.modify_star_power(actor_side, 1);
                }
            }
            if DREAM_GUARANTEED_WOUND_ADJACENT_IDS.contains(&adjacent.id) {
                self.actor_mut(actor_side).turn.guaranteed_wound += 1;
            }
        }

        let finite_marrow =
            self.dream_mirage_value(actor_side, DreamMirageValue::FiveElementsMarrow) > 0;
        let infinite_marrow = self
            .actor(actor_side)
            .dream_mirage
            .five_elements_marrow_infinite
            > 0;
        if (finite_marrow || infinite_marrow)
            && (card.name.contains("灵印") || card.name.contains("灵阵"))
        {
            self.gain_agility(actor_side, 10);
            // CardActionBase.OnBeforeExecuted:2956-2962 consumes
            // MengTianSuiDiJingJie for every non-HuaShen variant.  The
            // HuaShen's MengTianSui is unbounded and is tracked separately.
            if finite_marrow {
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::FiveElementsMarrow,
                    -1,
                );
            }
        }

        // CardActionBase.OnBeforeExecuted:2859-2862 consumes the
        // XiaZhangBengQuanJiaTiPo (707, 梦·锻拳) marker for the next card
        // unconditionally; it is not guarded by IsBengQuan.  Keep this
        // before the card body so the configured physique and its HP/max-HP
        // hooks observe the same order as the original client.
        let pending_physique = self
            .dream_mirage_value(actor_side, DreamMirageValue::NextBengQuanPhysique)
            .max(0);
        if pending_physique > 0 {
            self.apply_physique_amount(actor_side, pending_physique);
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::NextBengQuanPhysique,
                -pending_physique,
            );
        }

        let pending = self
            .dream_mirage_value(actor_side, DreamMirageValue::NextBengQuanAdditionalAttack)
            .max(0);
        if pending > 0 && self.active_effect_is_beng_quan() {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::TriggeredBengQuanAdditionalAttack,
                pending,
            );
            // CardActionBase.cs:3083-3090：ChanPlus→ChanPlusChuFa 转移时，
            // isLianBeng（BattleCharacter.cs:12243-12252：base 10000035
            // 与 10030082/10040082）不 RemoveBuff(ChanPlus)——连崩触发的
            // 追加攻击保留给下一张崩拳。oracle 锚点：mirror-32299000
            // b18d6644c37af418/round-13 t8 降龙仍继承 319 的 3×2 追加攻击
            // （气势 2→0，共 3 攻击段）。
            if !matches!(card.id, 10_030_082 | 10_040_082) && normalized_base_id(card) != 10_000_035
            {
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::NextBengQuanAdditionalAttack,
                    -pending,
                );
            }
        }
    }

    /// CardActionBase.OnBeforeExecuted:2281-2284. This runs for normal and
    /// guarded temporary cards so 万能剑 classification observes the same
    /// effective-count window as the original.
    pub(super) fn start_dream_mirage_selected_card(&mut self, actor_side: PlayerSide) {
        if self.actor(actor_side).sword.all_purpose_sword > 0 {
            self.actor_mut(actor_side)
                .sword
                .all_purpose_sword_effective_count += 1;
        }
        if self.actor(actor_side).sword.next_cards_as_frenzy_sword > 0 {
            self.actor_mut(actor_side)
                .sword
                .next_cards_as_frenzy_sword_effective_count += 1;
        }
    }

    /// CardActionBase.Execute repetition hook. 聚焰 adds one complete
    /// ExecuteEffect lifecycle; it is consumed while the outer repetition
    /// count is frozen, before any repetition begins.
    pub(super) fn consume_dream_mirage_repeat_fire_or_earth(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        let repeat = self
            .dream_mirage_value(actor_side, DreamMirageValue::RepeatNextFireOrEarth)
            .max(0);
        if repeat <= 0 || (!card.name.contains("火灵") && !card.name.contains("土灵")) {
            return false;
        }
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::RepeatNextFireOrEarth, -1);
        true
    }

    /// CardActionBase.ExecuteEffect pre-body hooks. The outer transaction
    /// deliberately does not call these for a selected temporary card.
    pub(super) fn apply_dream_mirage_before_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        let dance = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamDanceCountdown)
            .max(0);
        if dance > 0
            && self.actor(actor_side).turn.extra_actions <= 0
            && self.actor(actor_side).turn.action_again_count
                < self.dream_mirage_action_again_limit(actor_side)
        {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamDanceCountdown, -1);
            self.modify_extra_actions(actor_side, 1);
        }

        let cloud_sea = self
            .dream_mirage_value(actor_side, DreamMirageValue::CloudSeaOnFormation)
            .max(0);
        if cloud_sea > 0 && is_sword_formation_card(self.actor(actor_side), card) {
            self.actor_mut(actor_side).sword.cloud_sea += cloud_sea;
        }
        let sword_energy = self
            .dream_mirage_value(actor_side, DreamMirageValue::SwordEnergyOnSword)
            .max(0);
        if sword_energy > 0 && is_sword_card(self.actor(actor_side), card) {
            self.actor_mut(actor_side).sword.sword_energy += sword_energy;
        }
        let spirit_cat = self
            .dream_mirage_value(actor_side, DreamMirageValue::SpiritCatCloud)
            .max(0);
        if spirit_cat > 0 && is_cloud_sword(self.actor(actor_side), card) {
            self.gain_anima(actor_side, spirit_cat);
        }
    }

    /// CardActionBase.OnAfterExecuted ordinary-only hooks before IL_19cf.
    pub(super) fn apply_dream_mirage_ordinary_after_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        // Source order is 幻星移 before 梦星罗棋布.
        if self.dream_mirage_value(actor_side, DreamMirageValue::StarShift) > 0
            && card.name.contains("星弈")
        {
            let attack = self
                .dream_mirage_value(actor_side, DreamMirageValue::StarShiftAttack)
                .max(0);
            if attack > 0 {
                self.apply_attack(actor_side, attack, slot);
            }
            let next = self.dream_mirage_runtime_next_grid(actor_side, slot);
            self.add_dream_mirage_runtime_star_slot(actor_side, next);
        }

        let star_board = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamStarBoard)
            .max(0);
        let is_non_star = !self.actor(actor_side).astrology.star_slots.contains(&slot);
        if star_board > 0
            && self.dream_mirage_value(actor_side, DreamMirageValue::DreamStarBoardTriggered) <= 0
            && is_non_star
        {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::DreamStarBoardTriggered,
                1,
            );
            self.gain_anima(actor_side, star_board);
            self.modify_star_power(actor_side, star_board);
        }
        let low_realm = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamStarBoardLowRealm)
            .max(0);
        if low_realm > 0 && is_non_star {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::DreamStarBoardLowRealm,
                -1,
            );
            self.gain_anima(actor_side, 1);
            self.modify_star_power(actor_side, 1);
        }
    }

    /// CardActionBase.OnAfterExecuted common tail at and after IL_19cf.
    pub(super) fn apply_dream_mirage_common_after_effect_hooks(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        let appended = self
            .dream_mirage_value(
                actor_side,
                DreamMirageValue::TriggeredBengQuanAdditionalAttack,
            )
            .max(0);
        if appended > 0 {
            // CardActionBase.OnAfterExecuted: the two 幻•崩拳缠 segments use
            // their Buff source, so they do not inherit the current card's
            // 崩拳-specific flat additions such as 崩拳绰.
            self.apply_attack_with_options(
                actor_side,
                appended,
                slot,
                false,
                false,
                0,
                Some("buff:bengQuanEntanglePlus"),
            );
            self.apply_attack_with_options(
                actor_side,
                appended,
                slot,
                false,
                false,
                0,
                Some("buff:bengQuanEntanglePlus"),
            );
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::TriggeredBengQuanAdditionalAttack,
                -appended,
            );
        }
    }

    /// CardActionBase.CheckAdjacentEffectsAfterCard is ordinary-only. The
    /// lifecycle boundary prevents temporary cards from entering this method.
    pub(super) fn apply_dream_mirage_adjacent_after_card_hooks(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        let target_side = opponent_side(actor_side);
        for adjacent in self.dream_mirage_adjacent_cards(actor_side, slot) {
            if normalized_base_id(&adjacent) != 1_000_069 {
                continue;
            }
            let value = adjacent.other_params.first().copied().unwrap_or(0).max(0);
            if super::original_config::original_card_realm_level(adjacent.id).unwrap_or(0) >= 4 {
                self.apply_attack(
                    actor_side,
                    value + self.actor(actor_side).core.anima.max(0),
                    slot,
                );
            } else if value > 0 {
                self.apply_damage_to(actor_side, target_side, value, false, false, false);
            }
        }

        if self.actor(actor_side).astrology.star_slots.contains(&slot)
            && self
                .actor(actor_side)
                .deck
                .slots
                .iter()
                .any(|entry| HIGH_DREAM_STAR_POINT_IDS.contains(&entry.card.id))
        {
            self.add_actor_negative_status(target_side, 100, 1);
        }
    }

    /// Shared card-completion counters. 连云 gains update the original
    /// `YongGuoYunJianJiShu` ledger at their mutation sites.
    pub(super) fn complete_dream_mirage_card_classification(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if is_sword_card(self.actor(actor_side), card) {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::SwordUsedCount, 1);
        }
        if is_sword_formation_card(self.actor(actor_side), card) {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::FormationUsedCount, 1);
        }
        if card.name.contains('蛇') {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::SnakeCardUsedCount, 1);
        }
        if original_card_desc_contains_rear_move(card) {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::RearMoveCardUsedCount, 1);
        }
        if ["金灵", "水灵", "木灵", "火灵", "土灵"]
            .iter()
            .any(|token| card.name.contains(token))
        {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::UsedFiveElementsCount, 1);
        }
        let actual_damage = self.active_effect_actual_damage().max(0);
        if actual_damage > 0 {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::TotalActualDamage,
                actual_damage,
            );
        }

        let effective = self
            .actor(actor_side)
            .sword
            .all_purpose_sword_effective_count;
        let all_purpose = self.actor(actor_side).sword.all_purpose_sword;
        if effective > 0 && all_purpose > 0 {
            if effective <= all_purpose {
                self.actor_mut(actor_side).sword.all_purpose_sword -= 1;
            }
            self.actor_mut(actor_side)
                .sword
                .all_purpose_sword_effective_count -= 1;
        }

        let frenzy_effective = self
            .actor(actor_side)
            .sword
            .next_cards_as_frenzy_sword_effective_count;
        let frenzy_cards = self.actor(actor_side).sword.next_cards_as_frenzy_sword;
        if frenzy_effective > 0 && frenzy_cards > 0 {
            if frenzy_effective <= frenzy_cards {
                self.actor_mut(actor_side).sword.next_cards_as_frenzy_sword -= 1;
            }
            self.actor_mut(actor_side)
                .sword
                .next_cards_as_frenzy_sword_effective_count -= 1;
        }
    }

    pub(super) fn consume_dream_mirage_next_card_exhaust(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
        is_temporary: bool,
    ) -> bool {
        let pending = self
            .dream_mirage_value(actor_side, DreamMirageValue::ConsumeNextCard)
            .max(0);
        if is_temporary || pending <= 0 {
            return false;
        }
        self.modify_dream_mirage_value(actor_side, DreamMirageValue::ConsumeNextCard, -1);
        if let Some(slot_state) = self.actor_mut(actor_side).deck.slots.get_mut(slot) {
            slot_state.skipped = true;
        }
        true
    }

    /// BattleCharacter.OnTurnStarted duration phase, before defense decay.
    pub(super) fn apply_dream_mirage_turn_start_duration_ticks(&mut self, actor_side: PlayerSide) {
        for value in [
            DreamMirageValue::DreamReflection,
            DreamMirageValue::DreamCliff,
        ] {
            if self.dream_mirage_value(actor_side, value) > 0 {
                self.modify_dream_mirage_value(actor_side, value, -1);
            }
        }
        let opponent = opponent_side(actor_side);
        if self.dream_mirage_value(opponent, DreamMirageValue::DreamMysticFootwork) > 0 {
            let blocked =
                self.dream_mirage_value(opponent, DreamMirageValue::DreamMysticFootworkSuppressed);
            self.modify_dream_mirage_value(
                opponent,
                DreamMirageValue::DreamMysticFootworkSuppressed,
                1 - blocked,
            );
        }
    }

    /// BattleCharacter.OnTurnStarted late phase, after normal turn-start
    /// formations and ledgers have settled.
    pub(super) fn apply_dream_mirage_turn_start_late(&mut self, actor_side: PlayerSide) {
        let target_side = opponent_side(actor_side);
        let flying = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamFlyingCloudPill)
            .max(0);
        if flying > 0 {
            if self.actor(target_side).core.defense > 0 {
                self.actor_mut(actor_side).turn.ignore_defense_attacks += flying;
            } else {
                self.gain_anima(actor_side, flying);
            }
        }

        let turn_defense = self
            .dream_mirage_value(actor_side, DreamMirageValue::TurnStartDefense)
            .max(0);
        if turn_defense > 0 {
            self.gain_defense(actor_side, turn_defense);
        }

        let doubles = self
            .dream_mirage_value(actor_side, DreamMirageValue::TemporaryWaterDouble)
            .max(0);
        if doubles > 0 {
            let water = self.actor(actor_side).elements.water_momentum.max(0) * doubles;
            let anima = self.actor(actor_side).core.anima.max(0) * doubles;
            if water > 0 {
                self.gain_water_momentum(actor_side, water);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::TemporaryWaterLedger,
                    water,
                );
            }
            if anima > 0 {
                self.gain_anima(actor_side, anima);
                self.modify_dream_mirage_value(
                    actor_side,
                    DreamMirageValue::TemporaryAnimaLedger,
                    anima,
                );
            }
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::TemporaryWaterDouble,
                -doubles,
            );
        }
    }

    /// BattleCharacter.OnTurnStarted IL_1448-15a3: 梦•大还丹在回合开始
    /// 常规治疗后、内伤结算前比较双方当前生命与上限。
    pub(super) fn apply_dream_great_return_pill_at_turn_start(&mut self, actor_side: PlayerSide) {
        let target_side = opponent_side(actor_side);
        let great_return = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamGreatReturnPill)
            .max(0);
        if great_return <= 0 {
            return;
        }
        if self.actor(actor_side).core.max_hp < self.actor(target_side).core.max_hp {
            self.modify_actor_max_hp(actor_side, great_return);
        }
        if self.actor(actor_side).core.hp < self.actor(target_side).core.hp {
            self.modify_actor_hp(actor_side, great_return, false, false);
        }
    }

    pub(super) fn apply_dream_mirage_defense_decay_healing(
        &mut self,
        actor_side: PlayerSide,
        defense_lost: i64,
    ) {
        if defense_lost > 0
            && self
                .actor(actor_side)
                .deck
                .slots
                .iter()
                .any(|slot| HIGH_DREAM_BEARING_WEIGHT_IDS.contains(&slot.card.id))
        {
            self.modify_actor_hp(actor_side, defense_lost, false, false);
        }
    }

    /// OnTurnEnded phase before ordinary Water Momentum damage.
    pub(super) fn apply_dream_mirage_turn_end_before_water(&mut self, actor_side: PlayerSide) {
        let frenzy = self
            .dream_mirage_value(actor_side, DreamMirageValue::HealingTurnEndFrenzy)
            .max(0);
        if frenzy > 0 && self.dream_mirage_value(actor_side, DreamMirageValue::TurnHpGained) >= 3 {
            self.modify_actor_hp(actor_side, -3 * frenzy, false, false);
            // BattleCharacter.OnTurnEnded (IL_18c6): 梦•狂剑二式 pays otherParams[2]
            // life per MengKuangEr stack and folds those stacks straight into the
            // KuangJian buff, i.e. 狂剑 use count. That is the same counter
            // 狂剑•二式 (Card_1000035) reads via GetBuffValue(KuangJian), so the
            // gain must land on sword.frenzy_sword, not a detached used-count.
            self.actor_mut(actor_side).sword.frenzy_sword += frenzy;
        }

        let extra_water = self
            .dream_mirage_value(actor_side, DreamMirageValue::ExtraWaterMomentumTurnEnd)
            .max(0);
        let water = self.actor(actor_side).elements.water_momentum.max(0);
        if extra_water > 0 && water > 0 {
            // 波澜额外触发段与主水势伤害同路径（BattleCharacter.cs
            // OnTurnEnded IL_1cd0 循环体）：KeYinShuiRen/FS137 时 Attack(水势)
            // ×1.5，否则 ApplyDamage(水势)。oracle 锚点见 flow.rs
            // apply_turn_end_water_momentum_damage（56f1c06b0530592f cp7、
            // 8f95021d967dff1e cp17、e1eb5c51c3f179d9 cp7 均差在额外段）。
            self.apply_turn_end_water_momentum_damage(actor_side, water);
        }
        if extra_water > 0 {
            self.modify_dream_mirage_value(
                actor_side,
                DreamMirageValue::ExtraWaterMomentumTurnEnd,
                -1,
            );
        }

        for value in [
            DreamMirageValue::CannotGainDefense,
            DreamMirageValue::DreamExtraActionLock,
            DreamMirageValue::HalfAnimaGain,
            DreamMirageValue::DreamUnmovingFormation,
        ] {
            if self.dream_mirage_value(actor_side, value) > 0 {
                self.modify_dream_mirage_value(actor_side, value, -1);
            }
        }
        self.dream_mirage_clear_value(actor_side, DreamMirageValue::DreamStarBoardTriggered);
        for side in [actor_side, opponent_side(actor_side)] {
            self.dream_mirage_clear_value(side, DreamMirageValue::DreamMysticFootworkSuppressed);
            self.dream_mirage_clear_value(side, DreamMirageValue::DreamMysticFootworkTriggerCount);
        }
    }

    /// OnTurnEnded phase after ordinary Water Momentum damage.
    pub(super) fn restore_dream_mirage_temporary_turn_resources(&mut self, actor_side: PlayerSide) {
        let water = self
            .dream_mirage_value(actor_side, DreamMirageValue::TemporaryWaterLedger)
            .max(0);
        let anima = self
            .dream_mirage_value(actor_side, DreamMirageValue::TemporaryAnimaLedger)
            .max(0);
        if water > 0 {
            self.actor_mut(actor_side).elements.water_momentum =
                (self.actor(actor_side).elements.water_momentum - water).max(0);
        }
        if anima > 0 {
            self.spend_anima_up_to(actor_side, anima);
        }
        self.dream_mirage_clear_value(actor_side, DreamMirageValue::TemporaryWaterLedger);
        self.dream_mirage_clear_value(actor_side, DreamMirageValue::TemporaryAnimaLedger);
    }

    pub(super) fn dream_mirage_action_again_prevention_bypass(
        &self,
        actor_side: PlayerSide,
    ) -> bool {
        self.dream_mirage_value(actor_side, DreamMirageValue::DreamTuneImmunity) > 0
            || self.dream_mirage_value(actor_side, DreamMirageValue::DragonExtraActionImmunity) > 0
    }

    pub(super) fn dream_mirage_extra_action_locked(&self, actor_side: PlayerSide) -> bool {
        self.dream_mirage_value(actor_side, DreamMirageValue::DreamExtraActionLock) > 0
    }

    pub(super) fn apply_dream_mirage_successful_action_again_hooks(
        &mut self,
        actor_side: PlayerSide,
    ) {
        if self.dream_mirage_value(actor_side, DreamMirageValue::DreamTuneImmunity) > 0 {
            self.modify_dream_mirage_value(actor_side, DreamMirageValue::DreamTuneImmunity, -1);
        }
        let sharpness = self
            .dream_mirage_value(actor_side, DreamMirageValue::ActionAgainSharpness)
            .max(0);
        if sharpness > 0 {
            self.gain_sharpness(actor_side, sharpness);
        }
        let fire = self
            .dream_mirage_value(actor_side, DreamMirageValue::DreamFireFormation)
            .max(0);
        if fire > 0 {
            let target_side = opponent_side(actor_side);
            self.modify_actor_hp(target_side, -fire, false, false);
            self.modify_actor_max_hp(target_side, -fire);
            self.gain_defense(actor_side, fire);
        }
    }

    pub(super) fn is_dream_mirage_beng_quan(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        self.is_dream_mirage_intrinsic_beng_quan(actor_side, card)
            || self.dream_mirage_public_adjacent_beng_quan(actor_side, slot)
    }

    pub(super) fn is_dream_mirage_intrinsic_beng_quan(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        is_effective_beng_quan_card(self.actor(actor_side), card)
            || matches!(normalized_base_id(card), 10_000_080 | 10_000_087)
    }

    /// Public CardActionBase.CheckAdjacentEffects:1934-1967. BattleExecuter
    /// snapshots this once before CheckCardCost; the marker is cleared by the
    /// first non-temporary ExecuteEffect completion.
    pub(super) fn dream_mirage_public_adjacent_beng_quan(
        &self,
        actor_side: PlayerSide,
        slot: usize,
    ) -> bool {
        let previous = self.dream_mirage_runtime_previous_grid(actor_side, slot);
        let next = self.dream_mirage_runtime_next_grid(actor_side, slot);
        let previous_id = self
            .actor(actor_side)
            .deck
            .slots
            .get(previous)
            .map(|entry| entry.card.id);
        let next_id = self
            .actor(actor_side)
            .deck
            .slots
            .get(next)
            .map(|entry| entry.card.id);
        previous_id.is_some_and(|id| {
            LOW_DREAM_BENG_TIAN_IDS.contains(&id) || HIGH_DREAM_BENG_TIAN_IDS.contains(&id)
        }) || next_id.is_some_and(|id| HIGH_DREAM_BENG_TIAN_IDS.contains(&id))
    }

    fn dream_mirage_adjacent_cards(
        &self,
        actor_side: PlayerSide,
        slot: usize,
    ) -> Vec<CardDefinition> {
        [
            self.dream_mirage_runtime_previous_grid(actor_side, slot),
            self.dream_mirage_runtime_next_grid(actor_side, slot),
        ]
        .into_iter()
        .filter_map(|index| {
            self.actor(actor_side)
                .deck
                .slots
                .get(index)
                .map(|entry| entry.card.clone())
        })
        .collect()
    }

    fn dream_mirage_runtime_active_slot_count(&self, actor_side: PlayerSide) -> usize {
        self.actor(actor_side)
            .deck
            .active_slot_count
            .min(self.actor(actor_side).deck.slots.len())
            .max(1)
    }

    fn dream_mirage_runtime_next_grid(&self, actor_side: PlayerSide, grid: usize) -> usize {
        let count = self.dream_mirage_runtime_active_slot_count(actor_side);
        let step = if self.actor(actor_side).fate.reverse_card_direction > 0 {
            -1
        } else {
            1
        };
        (grid as i64 + step).rem_euclid(count as i64) as usize
    }

    fn dream_mirage_runtime_previous_grid(&self, actor_side: PlayerSide, grid: usize) -> usize {
        let count = self.dream_mirage_runtime_active_slot_count(actor_side);
        let step = if self.actor(actor_side).fate.reverse_card_direction > 0 {
            1
        } else {
            -1
        };
        (grid as i64 + step).rem_euclid(count as i64) as usize
    }

    fn add_dream_mirage_runtime_star_slot(&mut self, actor_side: PlayerSide, grid: usize) {
        if self.actor(actor_side).astrology.star_slots.contains(&grid) {
            self.gain_anima(actor_side, 1);
        } else {
            self.actor_mut(actor_side).astrology.star_slots.push(grid);
        }
    }

    fn dream_mirage_clear_value(&mut self, actor_side: PlayerSide, value: DreamMirageValue) {
        let current = self.dream_mirage_value(actor_side, value);
        if current != 0 {
            self.modify_dream_mirage_value(actor_side, value, -current);
        }
    }

    pub(super) fn clear_turn_hp_gained_ledgers(&mut self, actor_side: PlayerSide) {
        // Original `HuiHeJiaShengMing` is turn-local. `AddHpCount` is a
        // separate battle-lifetime ledger and must survive this reset.
        let before = self.actor(actor_side).dream_mirage.turn_hp_gained;
        self.dream_mirage_clear_value(actor_side, DreamMirageValue::TurnHpGained);
        self.record_counter_transition(
            actor_side,
            "回合",
            "hpGained",
            "本回合获得生命",
            before,
            self.actor(actor_side).dream_mirage.turn_hp_gained,
        );
    }
}
