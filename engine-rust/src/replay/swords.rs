use super::effect_invocation::TemporaryInvocationSpec;
use super::support::{
    has_cloud_chain, is_sword_formation_card, normalized_base_id, opponent_side, other_param,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};
use std::collections::HashSet;

/// Base ids handled by `apply_sword_card_effect` (kept in sync with the
/// match arms below). `card_routing` uses this to pin the sect-routing
/// invariant: the sword kernel must never claim another sect's ids.
#[cfg(test)]
pub(super) const SWORD_HANDLED_IDS: &[i64] = &[
    331, 401, 424, 213, 263, 1_000_002, 1_000_003, 1_000_004, 1_000_006, 1_000_008, 1_000_009,
    1_000_011, 1_000_014, 1_000_015, 1_000_016, 1_000_020, 1_000_021, 1_000_022, 1_000_024,
    1_000_025, 1_000_026, 1_000_027, 1_000_028, 1_000_029, 1_000_030, 1_000_033, 1_000_034,
    1_000_035, 1_000_036, 1_000_038, 1_000_039, 1_000_040, 1_000_041, 1_000_042, 1_000_043,
    1_000_044, 1_000_045, 1_000_046, 1_000_048, 1_000_052, 1_000_055, 1_000_059, 1_000_060,
    1_000_062, 1_000_063, 1_000_064, 1_000_065, 1_000_096, 1_000_099, 1_000_100,
];

impl ReplayState {
    pub(super) fn apply_sword_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> Option<bool> {
        let mut attacked = false;
        match normalized_base_id(card) {
            331 => {
                // Card_331.OnExecuted attacks before installing the temporary
                // all-sword identity. The current card therefore opens no
                // effective-count window; the granted stacks belong to the
                // following cards.
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let all_purpose_sword = if has_cloud_chain(self.actor(actor_side)) {
                    other_param(card, 1)
                } else {
                    other_param(card, 0)
                }
                .max(0);
                self.actor_mut(actor_side).sword.all_purpose_sword += all_purpose_sword;
                if self.actor(actor_side).identity.talents.contains(&222)
                    && self.actor(actor_side).sword.frenzy_sword > 0
                {
                    self.modify_extra_actions(actor_side, 1);
                }
            }
            2 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        105,
                        other_param(card, 0).max(0),
                    );
                }
            }
            401 => {
                // 云剑•狂炎 Card_401.cs: Attack(attack, attackCount)，随后
                // HasBuff(WoundedCount) 成立时给对方 WaiShang+otherParams[0]
                // 并给自己 ExActionAgain+1。WoundedCount 是当前牌执行期计数
                // （BattleCharacter.cs:10854 攻击后 +1、CardActionBase.cs:4745
                // 牌结算完移除），复用引擎既有的 invocation-local
                // active_effect_wounded_count（与卡 2 云剑•游龙同构）。
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    let injury = other_param(card, 0).max(0);
                    if injury > 0 {
                        self.add_actor_negative_status(opponent_side(actor_side), 105, injury);
                    }
                    self.modify_extra_actions(actor_side, 1);
                }
            }
            9 => {
                let count = self.consume_optional_decision();
                let max_segments =
                    card.attack_count.unwrap_or(0).max(0) + other_param(card, 0).max(0);
                let capped = if max_segments > 0 {
                    count.min(max_segments)
                } else {
                    count
                };
                let attack = card.attack.unwrap_or(0).max(0);
                for _ in 0..capped.max(0) {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
            }
            50 => {
                self.actor_mut(actor_side)
                    .sword
                    .frenzy_dragon_swallows_cloud += 1;
                self.modify_frenzy_sword(actor_side, other_param(card, 0).max(0));
            }
            8 => {
                let previous = self.actor(actor_side).turn.current_turn_ignore_defense;
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous + 1;
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous;
                return Some(attacked);
            }
            424 => {
                // 极•云剑猫爪 Card_424.cs: ModifyBuffValue(BenLunWuShiFangYu, 1)
                // → Attack(attack, attackCount) → ModifyBuffValue(ShenFa,
                // otherParams[0])。BenLunWuShiFangYu 是「本轮无视防御」非消耗
                // buff（ApplyDamage BattleCharacter.cs:10747 只查不扣；
                // OnAfterExecuted 回合末统一移除），引擎用
                // turn.current_turn_ignore_defense 表达，作用域模式同卡 8
                // 云剑•猫爪（10424/20424 攻击段数由配置档位提供）。
                let previous = self.actor(actor_side).turn.current_turn_ignore_defense;
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous + 1;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).turn.current_turn_ignore_defense = previous;
                self.gain_agility(actor_side, other_param(card, 0).max(0));
            }
            3 => {
                let bonus = self.actor(actor_side).sword.cloud_chain * other_param(card, 0).max(0);
                return Some(self.attack_by_config(actor_side, card, bonus, slot));
            }
            1_000_006 => {
                self.apply_configured_anima(actor_side, card);
                let before = self.actor(actor_side).turn.ignore_defense_attacks;
                self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                    other_param(card, 0).max(0);
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
            1_000_002 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    let follow_up = other_param(card, 0).max(0);
                    if follow_up > 0 {
                        self.apply_attack(actor_side, follow_up, slot);
                        attacked = true;
                    }
                }
            }
            1_000_003 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.gain_defense(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_004 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
            }
            1_000_009 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let follow_up = other_param(card, 0).max(0);
                if follow_up > 0 && self.active_effect_wounded_count() > 0 {
                    self.apply_attack(actor_side, follow_up, slot);
                    attacked = true;
                }
            }
            1_000_011 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.modify_sword_intent(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_014 => {
                if has_cloud_chain(self.actor(actor_side)) {
                    let attack = other_param(card, 0).max(0);
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
            }
            1_000_015 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.modify_sword_intent(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_016 => {
                self.apply_configured_defense(actor_side, card);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.modify_actor_hp(actor_side, other_param(card, 0).max(0), false, false);
                }
            }
            1_000_030 => {
                let attack = card.attack.unwrap_or(0).max(0);
                let attack_count = card.attack_count.unwrap_or(if attack > 0 { 1 } else { 0 });
                for _ in 0..attack_count.max(0) {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
                // Card_1000030.cs: HasBuff(WoundedCount) && GetBuffValue
                // (ActualDamage) > 0 时 ModifyDef(302)。302 是攻击者身上跨卡
                // 持久计数（残留 + 本卡），故直接读 turn.actual_damage_carry；
                // WoundedCount 判定保持 invocation-local（本卡是否击伤）。
                // 相邻 HP 变更（如 fate 67 炽火炎刃的 attack 前 maxHp 削减）
                // 不计入 302，与 apply_attack_damage 的累加点天然一致。
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry;
                if self.active_effect_wounded_count() > 0 && actual_damage > 0 {
                    self.gain_defense(actor_side, actual_damage);
                }
            }
            1_000_021 => {
                let pending_before = self.active_effect_pending_sword_intent();
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let pending_after = self.active_effect_pending_sword_intent();
                if self.active_effect_wounded_count() > 0 && pending_after > pending_before {
                    // Card_1000021 records the newly pending spend for a
                    // deferred restore. After-card attacks can still observe
                    // and extend the pending ledger before common settlement
                    // consumes then restores Sword Intent.
                    self.add_active_effect_deferred_sword_intent_restore(
                        pending_after - pending_before,
                    );
                }
            }
            263 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let pending = self.active_effect_pending_sword_intent().max(0);
                if pending > 0 {
                    self.add_active_effect_deferred_sword_intent_restore(
                        pending + other_param(card, 0).max(0),
                    );
                }
            }
            1_000_022 => {
                let bonus =
                    self.actor(actor_side).sword.frenzy_sword.max(0) * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            1_000_099 => {
                // 极•狂剑一式 Card_1000099.cs: num2 = GetBuffValue(KuangJian) *
                // otherParams[0]; Attack(attack + num2, attackCount)。KuangJian
                // buff 即「用过狂剑次数」计数（每张狂剑牌 OnAfterExecuted +1，
                // CardActionBase.cs:4616-4618；本牌 body 先读、计数后加），与
                // 1_000_022 狂剑•一式同构。1010099/1020099 数值由配置档位提供。
                let bonus =
                    self.actor(actor_side).sword.frenzy_sword.max(0) * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            1_000_100 => {
                // 极•灵犀剑阵 Card_1000100.cs: def>0 → ModifyDef(def)；剑意>0 时
                // num3 = min(剑意, otherParams[0])，先 ModifyBuffValue(JianYi,
                // -num3) 再 ModifyAnima(num3)（原版顺序）。与 1_000_033 灵犀剑阵
                // 同族；1010100/1020100 数值由配置档位提供。
                self.apply_configured_defense(actor_side, card);
                let limit = other_param(card, 0).max(0);
                let sword_intent = self.actor(actor_side).sword.sword_intent.max(0);
                let convert = sword_intent.min(limit);
                if convert > 0 {
                    self.modify_sword_intent(actor_side, -convert);
                    self.gain_anima(actor_side, convert);
                }
            }
            1_000_024 => {
                let card_anima = card.anima.unwrap_or(0).max(0);
                self.apply_configured_anima(actor_side, card);
                if self.actor(actor_side).core.anima > card_anima {
                    let attack = other_param(card, 0).max(0);
                    let count = card.other_params.get(1).copied().unwrap_or(1).max(0);
                    for _ in 0..count {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
            }
            1_000_026 => {
                self.actor_mut(actor_side).sword.cloud_sword_soft_heart +=
                    other_param(card, 0).max(0);
            }
            1_000_096 => {
                // 极•云剑柔心
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).sword.cloud_sword_soft_heart +=
                    other_param(card, 1).max(0);
            }
            1_000_027 => {
                self.apply_configured_anima(actor_side, card);
                let attack = self.actor(actor_side).core.anima.max(0);
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            1_000_028 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                        other_param(card, 0).max(0);
                }
            }
            1_000_029 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            1_000_038 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let limit = if self.active_effect_wounded_count() > 0 {
                    other_param(card, 1).max(0)
                } else {
                    other_param(card, 0).max(0)
                };
                let gain = self.actor(actor_side).core.anima.max(0).min(limit);
                if gain > 0 {
                    self.modify_sword_intent(actor_side, gain);
                }
            }
            1_000_008 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    let before = self.actor(actor_side).turn.next_turn_defense;
                    self.actor_mut(actor_side).turn.next_turn_defense +=
                        other_param(card, 0).max(0);
                    let after = self.actor(actor_side).turn.next_turn_defense;
                    self.record_counter_transition(
                        actor_side,
                        "回合",
                        "nextTurnDefense",
                        "下回合加防",
                        before,
                        after,
                    );
                }
            }
            1_000_043 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.active_effect_wounded_count() > 0 {
                    self.gain_anima(actor_side, self.active_effect_wounded_count());
                }
            }
            1_000_045 => {
                let anima = self.actor(actor_side).core.anima.max(0);
                if anima > 0 {
                    self.spend_anima_unchecked(actor_side, anima);
                }
                let bonus = anima * other_param(card, 0).max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            1_000_052 => {
                // 残云封天剑的 GetNextParam 是回放带中的云剑手牌加攻。
                let bonus = self.consume_required_decision().max(0);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            1_000_055 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
            }
            1_000_060 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                // Card_1000060.cs settles printed defense and the 连云 bonus
                // through two independent ModifyDef calls. Keep that boundary
                // because FateStrategy 382 augments every positive call.
                self.apply_configured_defense(actor_side, card);
                if has_cloud_chain(self.actor(actor_side)) {
                    let bonus = self.actor(actor_side).sword.cloud_chain.max(0)
                        * other_param(card, 0).max(0);
                    self.gain_defense(actor_side, bonus);
                }
            }
            1_000_034 => {
                self.apply_configured_anima(actor_side, card);
                let mindset_gain = other_param(card, 0).max(0);
                self.actor_mut(actor_side).fate.spirit_gathering_mindset += mindset_gain;
                if mindset_gain == 1 {
                    self.actor_mut(actor_side).fate.half_anima += 1;
                }
            }
            1_000_036 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
            }
            1_000_040 => {
                self.apply_configured_defense(actor_side, card);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_042 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if has_cloud_chain(self.actor(actor_side)) {
                    self.gain_defense(actor_side, other_param(card, 0).max(0));
                }
            }
            1_000_063 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let gain = self.actor(actor_side).sword.cloud_chain.max(0);
                if gain > 0 {
                    self.gain_anima(actor_side, gain);
                }
            }
            1_000_035 => {
                let count = 1 + self.actor(actor_side).sword.frenzy_sword.max(0);
                let attack = card.attack.unwrap_or(0).max(0);
                for _ in 0..count {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            1_000_039 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            1_000_046 => {
                self.apply_configured_defense(actor_side, card);
                let attack = self.actor(actor_side).core.defense.max(0);
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            1_000_025 => {
                self.apply_configured_defense(actor_side, card);
            }
            1_000_041 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).sword.water_month_sword_formation +=
                    other_param(card, 0).max(0);
            }
            1_000_048 => {
                self.apply_configured_defense(actor_side, card);
                if has_cloud_chain(self.actor(actor_side)) {
                    let attack = other_param(card, 0).max(0);
                    let count = other_param(card, 1).max(0);
                    for _ in 0..count {
                        if attack > 0 {
                            self.apply_attack(actor_side, attack, slot);
                            attacked = true;
                        }
                    }
                }
            }
            1_000_020 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let defense_gain =
                    self.actor(actor_side).core.anima.max(0) * other_param(card, 0).max(0);
                if defense_gain > 0 {
                    self.gain_defense(actor_side, defense_gain);
                }
            }
            1_000_033 => {
                self.apply_configured_defense(actor_side, card);
                let sword_intent = self.actor(actor_side).sword.sword_intent.max(0);
                if sword_intent > 0 {
                    self.gain_anima(actor_side, sword_intent);
                    self.modify_sword_intent(actor_side, -sword_intent);
                }
            }
            1_000_044 => {
                let current = self.actor(actor_side).sword.sword_intent.max(0);
                let gain = current * other_param(card, 0).max(0) / 100;
                self.modify_sword_intent(actor_side, gain);
            }
            1_000_059 => {
                let bonus = self.actor(actor_side).core.anima.max(0) * other_param(card, 0).max(0);
                let attack = card.attack.unwrap_or(0).max(0) + bonus;
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                    attacked = true;
                }
            }
            1_000_064 => {
                self.apply_configured_defense(actor_side, card);
                self.apply_chain_sword_formation(actor_side, slot);
            }
            1_000_062 => {
                let sword_formation_count = self
                    .actor(actor_side)
                    .deck
                    .slots
                    .iter()
                    .enumerate()
                    .filter(|(index, candidate)| {
                        *index != slot
                            && is_sword_formation_card(self.actor(actor_side), &candidate.card)
                    })
                    .count() as i64;
                let attack = card.attack.unwrap_or(0).max(0)
                    + sword_formation_count * other_param(card, 0).max(0);
                let attack_count = card.attack_count.unwrap_or(if attack > 0 { 1 } else { 0 });
                for _ in 0..attack_count.max(0) {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
                let defense = card.defense.unwrap_or(0).max(0)
                    + sword_formation_count * other_param(card, 1).max(0);
                if defense > 0 {
                    self.gain_defense(actor_side, defense);
                }
            }
            1_000_065 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side)
                    .sword
                    .hundred_bird_spirit_sword_art += other_param(card, 0).max(0);
            }
            213 => {
                let sword_formation_count =
                    self.actor(actor_side)
                        .deck
                        .slots
                        .iter()
                        .filter(|candidate| {
                            is_sword_formation_card(self.actor(actor_side), &candidate.card)
                        })
                        .count()
                        .min(other_param(card, 0).max(0) as usize) as i64;
                let attack = card.attack.unwrap_or(0).max(0);
                let attack_count = card.attack_count.unwrap_or(1).max(0) + sword_formation_count;
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
                if self.active_effect_wounded_count() > 0 {
                    let defense_gain =
                        self.active_effect_wounded_count().max(0) * other_param(card, 1).max(0);
                    if defense_gain > 0 {
                        self.gain_defense(actor_side, defense_gain);
                    }
                }
            }
            _ => return None,
        }
        Some(attacked)
    }

    fn apply_chain_sword_formation(&mut self, actor_side: PlayerSide, slot: usize) {
        let mut cursor = self
            .actor(actor_side)
            .sword
            .chain_sword_temporary_cursor
            .unwrap_or(slot);
        let mut visited = HashSet::new();
        while cursor > 0 {
            let previous_index = cursor - 1;
            if visited.contains(&previous_index) {
                break;
            }
            let previous_card = self.actor(actor_side).deck.slots[previous_index]
                .card
                .clone();
            if !is_sword_formation_card(self.actor(actor_side), &previous_card) {
                cursor = previous_index;
                continue;
            }
            visited.insert(previous_index);
            // Original Card_1000064 keeps using the outer CardItem: only its
            // config/skin are replaced, while gridNumber remains the chain's
            // slot. LianHuanTempPos separately carries the recursive search
            // cursor when the copied card is another 连环剑阵.
            self.actor_mut(actor_side)
                .sword
                .chain_sword_temporary_cursor = Some(previous_index);
            self.execute_chain_sword_temporary_card(actor_side, slot, &previous_card);
            self.actor_mut(actor_side)
                .sword
                .chain_sword_temporary_cursor = None;
            break;
        }
    }

    fn execute_chain_sword_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
        selected: &CardDefinition,
    ) {
        let mut spec = TemporaryInvocationSpec::physical(slot);
        spec.inherit_parent_beng_quan = true;
        if self.apply_temporary_card_effect_with_spec(actor_side, selected, spec) {
            self.modify_extra_actions(actor_side, 1);
        }
    }
}
