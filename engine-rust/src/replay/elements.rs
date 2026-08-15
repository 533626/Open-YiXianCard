use super::support::{
    div_ceil, element_from_card, is_cloud_sword, is_element_generated_by,
    is_frenzy_sword_for_actor, is_sword_formation_card, normalized_base_id, opponent_side,
    other_param,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

fn regenerative_body_from_talents(talents: &[i64]) -> (i64, i64) {
    talents
        .iter()
        .fold((0, 0), |(physique, hp), talent| match *talent {
            149 => (physique + 1, hp + 1),    // 再生之躯
            10_149 => (physique + 1, hp + 2), // 再生之躯
            20_149 => (physique + 2, hp + 4), // 再生之躯
            30_149 => (physique + 2, hp + 6), // 再生之躯
            _ => (physique, hp),
        })
}

fn triggers_five_element_formation(
    actor: &super::ReplayPlayer,
    card_element: Element,
    formation_element: Element,
    generated_element: Element,
) -> bool {
    card_element == formation_element
        || ((actor.identity.talents.contains(&130)
            || actor.identity.fate_strategies.contains(&133))
            && card_element == generated_element)
}

fn first_slot_max_hp_gain_from_talents(talents: &[i64]) -> i64 {
    [(60, 10), (10_060, 10), (20_060, 15), (30_060, 20)]
        .iter()
        .filter(|(talent, _)| talents.contains(talent))
        .map(|(_, gain)| *gain)
        .sum()
}

impl ReplayState {
    /// CardActionBase.OnAfterExecuted: record the just-completed card's
    /// element as `lastElement` (five-elements generating-chain anchor), and
    /// when it is water, accumulate the persistent Steam build 24217566
    /// YiYongGuoShuiLingPai counter alongside it. Must run for every real
    /// card completion (primary plays and temporary/echoed re-executions
    /// alike) — the three call sites (production primary path, temporary
    /// effect path, and the `#[cfg(test)]` direct-body harness) share this
    /// helper so none of them can silently drop the water-spirit branch.
    pub(super) fn record_last_element(&mut self, actor_side: PlayerSide, card: &CardDefinition) {
        let last_element = element_from_card(card);
        self.actor_mut(actor_side).elements.last_element = last_element;
        if last_element == Some(Element::Water) {
            self.actor_mut(actor_side).elements.used_water_spirit_card += 1;
        }
    }

    /// CardActionBase.cs:2383-2386（OnBeforeExecuted 天赋参数 switch）：
    /// `num3 = current % 10000` 逐天赋命中 case 69，每个 69 系天赋档位
    /// （69/10069/20069/30069）独立执行一次 `ModifyDef(otherParams[0])`。
    /// 引擎不能汇总成一次加防：NiShi（逆施）与 HeBaHuang（合八荒）都挂在
    /// ModifyDef 上按次结算，汇总会改变反弹值（floor(7×50/100)=3 对
    /// 1+1+1）与八荒退款顺序。oracle 锚点：mirror-32299000
    /// 2ae5ddcc93eaebab/round-15 cp12（游龙后 p2.def 16→4 = 15 伤 - 3
    /// 退款；引擎汇总 +7 时 16→10）。
    fn cloud_way_defense_gains(actor: &super::ReplayPlayer) -> [i64; 4] {
        let mut gains = [0_i64; 4];
        if actor.identity.talents.contains(&69) {
            gains[0] = 1;
        }
        if actor.identity.talents.contains(&10_069) {
            gains[1] = 2;
        }
        if actor.identity.talents.contains(&20_069) {
            gains[2] = 2;
        }
        if actor.identity.talents.contains(&30_069) {
            gains[3] = 3;
        }
        gains
    }

    fn sword_formation_guard_defense_gain(actor: &super::ReplayPlayer) -> i64 {
        (if actor.identity.talents.contains(&57) {
            1
        } else {
            0
        }) + (if actor.identity.talents.contains(&10_057) {
            3
        } else {
            0
        }) + (if actor.identity.talents.contains(&20_057) {
            6
        } else {
            0
        }) + (if actor.identity.talents.contains(&30_057) {
            9
        } else {
            0
        })
    }

    fn sword_formation_guard_trigger_count(actor: &super::ReplayPlayer) -> i64 {
        [57, 10_057, 20_057, 30_057]
            .iter()
            .filter(|talent| actor.identity.talents.contains(talent))
            .count() as i64
    }

    #[cfg(test)]
    pub(super) fn apply_selected_card_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        self.start_dream_mirage_selected_card(actor_side);
        self.apply_qi_xing_lian_zhu(actor_side, card, slot);
        self.apply_selected_card_hooks_after_start(actor_side, card, slot, false);
    }

    pub(super) fn apply_selected_card_hooks_after_start(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        temporary_replay: bool,
    ) {
        let base_id = normalized_base_id(card);
        self.apply_ordinary_sword_action_again_before_card(actor_side, card);
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&337)
            && base_id == 381
        {
            self.gain_anima(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&33)
            && base_id == 0
            && self
                .actor(actor_side)
                .deck
                .slots
                .get(slot)
                .is_some_and(|slot_state| !slot_state.used)
        {
            self.gain_guard(actor_side, 2);
        }
        if !temporary_replay && slot == 0 {
            let defense_gain = Self::sword_formation_guard_defense_gain(self.actor(actor_side));
            if defense_gain > 0 {
                self.gain_defense(actor_side, defense_gain);
                let triggers = Self::sword_formation_guard_trigger_count(self.actor(actor_side));
                self.actor_mut(actor_side).sword.water_month_sword_formation += triggers;
            }
        }
        if is_cloud_sword(self.actor(actor_side), card) {
            for defense_gain in Self::cloud_way_defense_gains(self.actor(actor_side)) {
                if defense_gain > 0 {
                    self.gain_defense(actor_side, defense_gain);
                }
            }
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&97)
            && is_cloud_sword(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.cloud_sea += 1;
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&325)
            && is_frenzy_sword_for_actor(self.actor(actor_side), card)
        {
            self.gain_anima(actor_side, 1);
        }
        if card.id == 19 {
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&100)
            {
                self.gain_anima(actor_side, 1);
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&101)
            {
                self.actor_mut(actor_side).sword.sword_energy += 2;
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&102)
            {
                self.reduce_all_actor_negative_statuses(actor_side, 2);
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&103)
            {
                self.actor_mut(actor_side).sword.water_month_sword_formation += 2;
            }
            if self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&324)
            {
                self.actor_mut(actor_side).sword.cloud_sea += 5;
                if self.original_build_has_capability(
                    super::original_build_profile::OriginalBuildCapability::Fate324GrantsCloudChain,
                ) {
                    self.gain_cloud_chain(actor_side, 1);
                }
            }
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&121)
            && slot == 7
        {
            self.add_actor_negative_status(opponent_side(actor_side), 100, 2);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&153)
        {
            let damage = card.hp_cost.unwrap_or(0).max(0) * 40 / 100;
            if damage > 0 {
                self.apply_damage(actor_side, damage, false, false, false);
            }
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&32)
            && base_id == 0
        {
            let target_side = opponent_side(actor_side);
            let attack_bonus = self.actor(target_side).core.attack_bonus;
            self.actor_mut(target_side).core.attack_bonus = (attack_bonus - 3).max(0);
            self.modify_guard(target_side, -3);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&36)
            && base_id == 0
        {
            self.gain_attack_bonus(actor_side, 2);
        }
        if self.actor(actor_side).fate.sword_formation_guard > 0
            && is_sword_formation_card(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).fate.sword_formation_guard -= 1;
            self.gain_defense(actor_side, 5);
            self.gain_guard(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&128)
            && card.name.contains("金灵")
        {
            self.gain_anima(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&320)
            && is_cloud_sword(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.water_month_sword_formation += 1;
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&330)
            && base_id == 22
        {
            self.gain_temporary_guard(actor_side, 3);
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&345)
            && base_id == 7_000_069
        {
            let damage = self.actor(actor_side).core.anima.max(0);
            if damage > 0 {
                self.modify_target_hp(actor_side, -damage);
                self.modify_target_max_hp(actor_side, -damage);
            }
        }
        if !temporary_replay && slot == 0 {
            let (physique, hp) =
                regenerative_body_from_talents(&self.actor(actor_side).identity.talents);
            if physique > 0 || hp > 0 {
                self.apply_physique_amount(actor_side, physique);
                self.modify_actor_hp(actor_side, hp, false, false);
            }
        }
        if slot == 0 {
            let max_hp_gain =
                first_slot_max_hp_gain_from_talents(&self.actor(actor_side).identity.talents);
            if max_hp_gain > 0 {
                self.modify_actor_max_hp(actor_side, max_hp_gain);
            }
        }
        if !temporary_replay
            && self.actor(actor_side).elements.swift_burn_seal > 0
            && card.name.contains('印')
        {
            self.modify_extra_actions(actor_side, 1);
            self.actor_mut(actor_side).elements.swift_burn_seal -= 1;
            if let Some(slot_state) = self.actor_mut(actor_side).deck.slots.get_mut(slot) {
                slot_state.skipped = true;
            }
        }
        if self.actor(actor_side).turn.next_card_action_again > 0 {
            self.actor_mut(actor_side).turn.next_card_action_again -= 1;
            self.modify_extra_actions(actor_side, 1);
        }
        if is_cloud_sword(self.actor(actor_side), card) {
            if self.actor(actor_side).identity.talents.contains(&15) {
                self.gain_anima(actor_side, 1);
            }
            let healing = self.actor(actor_side).sword.cloud_sword_soft_heart;
            if healing > 0 {
                self.modify_actor_hp(actor_side, healing, false, false);
            }
            if self.actor(actor_side).sword.cloud_sword_heart > 0 {
                self.actor_mut(actor_side).sword.cloud_sword_heart -= 1;
                self.modify_extra_actions(actor_side, 1);
            }
        }
        // CardActionBase.OnBeforeExecuted reads the effective post-transform
        // card once per ExecuteEffect. Unlike the Cloud Sword hooks above,
        // 崩灭心法 explicitly excludes isTempCard executions.
        if !temporary_replay
            && card.hp_cost.unwrap_or(0) > 0
            && self.actor(actor_side).beng.beng_mei_mindset > 0
        {
            self.modify_momentum(actor_side, self.actor(actor_side).beng.beng_mei_mindset);
        }
        if self.actor(actor_side).identity.talents.contains(&62) && card.name.contains("星弈") {
            self.gain_hexagram(actor_side, 1);
        }
        if element_from_card(card).is_none()
            && self.actor(actor_side).identity.talents.contains(&200)
        {
            self.activate_element(actor_side, Element::Wood);
        }
        let Some(element) = element_from_card(card) else {
            return;
        };
        if self.actor(actor_side).elements.next_card_activate_element > 0 {
            self.activate_element(actor_side, element);
            self.actor_mut(actor_side)
                .elements
                .next_card_activate_element -= 1;
        }
        if triggers_five_element_formation(
            self.actor(actor_side),
            element,
            Element::Water,
            Element::Wood,
        ) {
            let water_formation = self.actor(actor_side).elements.water_formation;
            if water_formation > 0 {
                self.gain_anima(actor_side, water_formation);
            }
        }
        if triggers_five_element_formation(
            self.actor(actor_side),
            element,
            Element::Earth,
            Element::Metal,
        ) {
            let earth_formation = self.actor(actor_side).elements.earth_formation;
            if earth_formation > 0 {
                self.gain_defense(actor_side, earth_formation);
            }
        }
        if triggers_five_element_formation(
            self.actor(actor_side),
            element,
            Element::Fire,
            Element::Earth,
        ) {
            let fire_formation = self.actor(actor_side).elements.fire_formation;
            if fire_formation > 0 {
                self.modify_target_hp(actor_side, -fire_formation);
                self.modify_target_max_hp(actor_side, -fire_formation);
            }
        }
        if triggers_five_element_formation(
            self.actor(actor_side),
            element,
            Element::Metal,
            Element::Water,
        ) {
            let metal_formation = self.actor(actor_side).elements.metal_formation;
            if metal_formation > 0 {
                self.gain_sharpness(actor_side, metal_formation);
            }
        }
        if triggers_five_element_formation(
            self.actor(actor_side),
            element,
            Element::Wood,
            Element::Fire,
        ) {
            let wood_array = self.actor(actor_side).elements.wood_array;
            if wood_array > 0 {
                self.modify_actor_hp(actor_side, wood_array, false, false);
            }
        }
        if (element == Element::Wood
            || (element == Element::Fire && self.actor(actor_side).identity.talents.contains(&130)))
            && self.actor(actor_side).elements.wood_healing_formation > 0
        {
            // CardActionBase3180: 木灵疗愈阵 heals attack-bonus times its layers.
            // Talent_130 extends only the fire-card predicate; Card_201 remains out of scope.
            let healing = self.actor(actor_side).core.attack_bonus
                * self.actor(actor_side).elements.wood_healing_formation;
            if healing > 0 {
                self.modify_actor_hp(actor_side, healing, false, false);
            }
        }

        let generated = self
            .actor(actor_side)
            .elements
            .last_element
            .is_some_and(|last_element| {
                is_element_generated_by(
                    last_element,
                    element,
                    self.actor(actor_side).identity.talents.contains(&137),
                )
            });
        if generated {
            if self.actor(actor_side).identity.talents.contains(&101) {
                self.gain_anima(actor_side, 1);
                self.modify_actor_max_hp(actor_side, 1);
                self.modify_actor_hp(actor_side, 1, false, false);
            }
            if self.actor(actor_side).identity.talents.contains(&102) {
                self.gain_defense(actor_side, 3);
            }
            if self.actor(actor_side).identity.talents.contains(&10_102) {
                self.gain_anima(actor_side, 1);
            }
            if self.actor(actor_side).identity.talents.contains(&20_102) {
                self.modify_actor_max_hp(actor_side, 4);
                self.modify_actor_hp(actor_side, 4, false, false);
            }
            if self.actor(actor_side).identity.talents.contains(&30_102) {
                self.gain_sharpness(actor_side, 4);
            }
        }

        // CardActionBase.OnBeforeExecuted checks this after the generating-chain
        // rewards, but independently of whether that chain generated. Use the
        // full original base id so seasonal 7_010_020 cannot alias card 20.
        if self.actor(actor_side).identity.talents.contains(&101)
            && normalized_base_id(card) == 20
            && self.actor(actor_side).elements.used_water_spirit_card > 0
        {
            self.modify_extra_actions(actor_side, 1);
        }
    }
}

/// Base ids handled by `apply_element_card_effect` (kept in sync with the
/// match arms below). `card_routing` uses this to pin the sect-routing
/// invariant: the element kernel must never claim another sect's ids.
#[cfg(test)]
pub(super) const ELEMENT_HANDLED_IDS: &[i64] = &[
    13, 134, 413, 7_000_001, 7_000_002, 7_000_003, 7_000_004, 7_000_006, 7_000_007, 7_000_009,
    7_000_010, 7_000_011, 7_000_012, 7_000_013, 7_000_017, 7_000_018, 7_000_019, 7_000_022,
    7_000_023, 7_000_024, 7_000_027, 7_000_028, 7_000_029, 7_000_030, 7_000_031, 7_000_037,
    7_000_059, 7_000_060, 7_000_068, 7_000_104,
];

impl ReplayState {
    pub(super) fn apply_element_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> Option<bool> {
        let mut attacked = false;
        match normalized_base_id(card) {
            13 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
            }
            7_000_001 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.activate_element(actor_side, Element::Metal);
            }
            7_000_002 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Metal) {
                    self.gain_sharpness(actor_side, other_param(card, 0));
                }
            }
            134 => {
                let value = other_param(card, 0).max(0);
                if value > 0 {
                    self.modify_actor_max_hp(actor_side, value);
                    self.modify_actor_hp(actor_side, value, false, false);
                }
                self.actor_mut(actor_side).elements.wood_spirit_all_growth += 1;
                self.actor_mut(actor_side)
                    .elements
                    .wood_spirit_all_growth_attack = other_param(card, 1).max(0);
            }
            7_000_003 => {
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                self.activate_element(actor_side, Element::Wood);
            }
            7_000_004 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Wood) {
                    self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                }
            }
            7_000_068 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                let hp_gain = other_param(card, 1).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                self.activate_element(actor_side, Element::Wood);
            }
            7_000_018 => {
                self.apply_configured_anima(actor_side, card);
                if self.check_wu_xing(actor_side, Element::Wood) {
                    let heal = other_param(card, 0)
                        + self.actor(actor_side).core.attack_bonus * other_param(card, 1);
                    self.modify_actor_hp(actor_side, heal, false, false);
                }
            }
            7_000_017 => {
                // Card_7000017.cs:80-82（木灵•疏影）[木灵] 分支走完整
                // CheckWuXing(src, JiHuoMuLing)（激活 / 上次使用五行及相生链 /
                // 龙马精神 / 卡组含 7030077|7040077 五行刺恒真），非仅木灵激活。
                // oracle 锚点：95fa8fdf614cb5d4/round-10 cp[8]（卡组含
                // 7030077、lastElement=火灵，加攻 +1 原版触发引擎漏判）。
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Wood) {
                    self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                }
            }
            7_000_027 => {
                // 玫刺 Card_7000027.cs:80-82：CheckWuXing(src, JiHuoMuLing)
                // （完整 CheckWuXing，含卡组 7030077|7040077 五行刺恒真，非仅
                // 木灵激活）且持有 ActualDamage(302) 时回血 302/otherParams[0]。
                // 302 是攻击者身上跨卡持久计数（BattleCharacter.cs:10858-10861
                // 累加，CardActionBase.cs:4743-4745 出牌完成才清零），自身攻击
                // 循环之后读到的值 = 残留 + 本卡，故读 turn.actual_damage_carry。
                // oracle 锚点：95fa8fdf614cb5d4/round-10 cp[10]（卡组含
                // 7030077，实际伤害 32/3=10 回血原版触发引擎漏判）。
                let attack = card.attack.unwrap_or(0);
                let attack_count = card
                    .attack_count
                    .unwrap_or(if attack > 0 { 1 } else { 0 })
                    .max(0);
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                attacked = attack_count > 0 && attack > 0;
                if self.check_wu_xing(actor_side, Element::Wood) {
                    let divisor = other_param(card, 0).max(1);
                    let heal = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    if heal > 0 {
                        self.modify_actor_hp(actor_side, heal, false, false);
                    }
                }
            }
            7_000_028 => {
                let previous_shatter_defense = self.active_effect_shatter_defense();
                self.set_active_effect_shatter_defense(previous_shatter_defense + 1);
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.set_active_effect_shatter_defense(previous_shatter_defense);
            }
            7_000_019 => {
                let momentum_gain = other_param(card, 0).max(0);
                if momentum_gain > 0 {
                    self.gain_water_momentum(actor_side, momentum_gain);
                }
            }
            7_000_029 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).elements.water_formation += other_param(card, 0).max(0);
            }
            7_000_104 => {
                // 极•水灵阵
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).elements.water_formation += other_param(card, 0).max(0);
            }
            7_000_030 => {
                self.apply_configured_anima(actor_side, card);
                let hp_gain = other_param(card, 1).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                // Card_7000030.cs 读 CardActionBase.CheckWuXing(src,
                // JiHuoShuiLing)（激活 / 龙马精神 / UsedWuXing 相生链 /
                // 卡组含 7030077|7040077 五行刺恒真），不是仅看水灵激活。
                // oracle 锚点：hf-32308000 d7d6da3ccefac976/round-13 cp[6]
                // p2.waterMomentum 9（引擎 4：卡组含 7040077 时原版恒真，
                // 引擎 last_element=木 漏判）。
                if self.check_wu_xing(actor_side, Element::Water) {
                    let divisor = other_param(card, 0).max(1);
                    let water_momentum = self.actor(actor_side).core.anima / divisor;
                    if water_momentum > 0 {
                        self.gain_water_momentum(actor_side, water_momentum);
                    }
                }
            }
            7_000_006 => {
                self.apply_configured_anima(actor_side, card);
                self.activate_element(actor_side, Element::Water);
            }
            7_000_007 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Water) {
                    let momentum_gain = other_param(card, 0).max(0);
                    if momentum_gain > 0 {
                        self.gain_water_momentum(actor_side, momentum_gain);
                    }
                }
            }
            7_000_037 => {
                let bonus = if self.check_wu_xing(actor_side, Element::Water) {
                    self.actor(actor_side).elements.water_momentum.max(0)
                } else {
                    0
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            7_000_059 => {
                self.apply_configured_anima(actor_side, card);
                // Card_7000059.cs:93 CheckWuXing(JiHuoShuiLing)（卡组含
                // 7030077|7040077 五行刺恒真），不是仅看水灵激活。
                if self.check_wu_xing(actor_side, Element::Water) {
                    self.actor_mut(actor_side).elements.spring_flow += other_param(card, 0).max(0);
                }
            }
            7_000_009 => {
                self.apply_configured_anima(actor_side, card);
                let amount = other_param(card, 0);
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                self.activate_element(actor_side, Element::Fire);
            }
            7_000_010 | 7_000_022 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Fire) {
                    self.apply_attack(actor_side, other_param(card, 0), slot);
                    attacked = true;
                }
            }
            413 => {
                // 火灵•焚花印 Card_413.cs: ModifyBuffValue(JiHuoHuoLing, 1) →
                // Attack(attack + anima/2, attackCount) → HasBuff(ActualDamage)
                // 时 dst.ModifyMaxHpWithFx(-GetBuffValue(ActualDamage) *
                // otherParams[0])。ActualDamage(302) 是攻击者身上跨卡持久计数
                // （BattleCharacter.cs:10860 攻击后累加，CardActionBase.cs:
                // 4743-4744 OnAfterExecuted 先记入 JiLuZongJiShangZhi 再
                // RemoveBuff 清零），读值发生在自身攻击之后（含本牌攻击造成
                // 的实际伤害），故读 turn.actual_damage_carry（残留 + 本卡，
                // 与 374 火灵斩 / 1_000_030 地煞剑同构）。
                self.activate_element(actor_side, Element::Fire);
                let anima_bonus = self.actor(actor_side).core.anima.max(0) / 2;
                attacked |= self.attack_by_config(actor_side, card, anima_bonus, slot);
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry;
                if actual_damage > 0 {
                    let reduction = actual_damage * other_param(card, 0).max(0);
                    self.modify_target_max_hp(actor_side, -reduction);
                }
            }
            7_000_031 => {
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                self.actor_mut(actor_side).elements.fire_formation += other_param(card, 1).max(0);
            }
            7_000_011 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                self.activate_element(actor_side, Element::Earth);
            }
            7_000_012 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                // Card_7000012.cs 读 CardActionBase.CheckWuXing(src,
                // JiHuoTuLing)（激活 / 龙马精神 / UsedWuXing 相生链 /
                // 卡组含 7030077|7040077 五行刺恒真），不是仅看土灵激活。
                // oracle 锚点：hf-32308000 551810ea5efcf995/round-16 cp[8]
                // p2.defense 15（引擎 0）、965c5de8251a0f66/round-16 cp[9]
                // p2.defense 7（引擎 0）：两局卡组均含 7040077。
                if self.check_wu_xing(actor_side, Element::Earth) {
                    let divisor = other_param(card, 0).max(1);
                    let target_side = opponent_side(actor_side);
                    let defense = (self.actor(actor_side).core.max_hp
                        - self.actor(target_side).core.max_hp)
                        .abs()
                        / divisor;
                    if defense > 0 {
                        self.gain_defense(actor_side, defense);
                        self.actor_mut(actor_side).turn.next_turn_defense += defense;
                    }
                }
            }
            7_000_023 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).elements.earth_formation += other_param(card, 0).max(0);
            }
            7_000_013 => {
                let bonus = if self.check_wu_xing(actor_side, Element::Earth)
                    && (self.actor(actor_side).core.defense > 0
                        || self.actor(opponent_side(actor_side)).core.defense > 0)
                {
                    other_param(card, 0)
                } else {
                    0
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            7_000_024 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                if self.check_wu_xing(actor_side, Element::Earth) {
                    self.actor_mut(actor_side).turn.next_turn_defense +=
                        other_param(card, 0).max(0);
                }
            }
            7_000_060 => {
                let bonus = if self
                    .actor(actor_side)
                    .elements
                    .activated_elements
                    .is_empty()
                {
                    0
                } else {
                    other_param(card, 0)
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            21 => {
                let lost_defense = if self.check_wu_xing(actor_side, Element::Earth) {
                    div_ceil(self.actor(actor_side).core.defense, 2)
                } else {
                    0
                };
                if lost_defense > 0 {
                    self.lose_defense(actor_side, lost_defense);
                }
                let bonus = lost_defense * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            base_id => return self.apply_element_card_effect_late(actor_side, card, slot, base_id),
        }
        Some(attacked)
    }
}
