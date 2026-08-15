use super::support::{
    active_neighbor_card, active_neighbor_slot_index, card_rarity, div_ceil, element_from_card,
    is_five_element_card, normalized_base_id, opponent_side, other_param, wu_xing_count_in_deck,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_qi_xing_lian_zhu(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        if self.actor(actor_side).fate.qi_xing_lian_zhu <= 0 || !card.name.contains("星弈") {
            return;
        }
        self.actor_mut(actor_side).fate.qi_xing_lian_zhu -= 1;
        let Some(next) = active_neighbor_slot_index(self.actor(actor_side), slot, 1) else {
            return;
        };
        if self.actor(actor_side).astrology.star_slots.contains(&next) {
            self.gain_anima(actor_side, 1);
        } else {
            self.actor_mut(actor_side).astrology.star_slots.push(next);
        }
    }

    pub(super) fn apply_card_completed_hooks(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        _slot: usize,
    ) {
        self.apply_card_classification_completed_hooks(actor_side, card);
        if self.actor(actor_side).beng.triggered_startled_touch > 0 {
            self.actor_mut(actor_side).beng.triggered_startled_touch = 0;
        }
    }

    pub(super) fn apply_element_card_effect_late(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        let mut attacked = false;
        match base_id {
            7_000_065 => {
                // Card_7000065.OnExecuted: ExecuteTemporaryCard is deliberately
                // routed through the five existing 五灵印 handlers.  Besides
                // preserving their element-specific side effects, this keeps
                // temporary-card lifecycle semantics (and invocation order)
                // identical to the original client.
                let rarity = card_rarity(card);
                let temporary_base_ids = [7_000_003, 7_000_009, 7_000_011, 7_000_001, 7_000_006];
                let temporary_cards = temporary_base_ids
                    .into_iter()
                    .map(|temporary_base_id| {
                        super::original_config::original_card_definition(
                            temporary_base_id + rarity * 10_000,
                        )
                    })
                    .collect::<Option<Vec<_>>>();
                let Some(temporary_cards) = temporary_cards else {
                    self.missing_decision("card:7000065 temporary card definition");
                    return Some(false);
                };
                for temporary_card in temporary_cards {
                    if self.apply_temporary_card_effect(actor_side, &temporary_card, slot) {
                        self.modify_extra_actions(actor_side, 1);
                    }
                }
            }
            18 => {
                let bonus = if self.check_wu_xing(actor_side, Element::Metal) {
                    self.actor_mut(actor_side).elements.no_sharpness_for_attack += 1;
                    self.actor(actor_side).sword.sharpness.max(0)
                } else {
                    0
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            7_000_015 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Metal) {
                    self.actor_mut(actor_side).turn.ignore_defense_attacks +=
                        other_param(card, 0).max(0);
                }
            }
            7_000_016 => {
                self.gain_sharpness(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).elements.metal_formation += other_param(card, 1).max(0);
            }
            7_000_020 => {
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                // Card_7000020.cs:97 CheckWuXing(JiHuoShuiLing)（卡组含
                // 7030077|7040077 五行刺恒真），不是仅看水灵激活。
                if self.check_wu_xing(actor_side, Element::Water) {
                    self.actor_mut(actor_side).elements.water_stealth +=
                        other_param(card, 1).max(0);
                }
            }
            7_000_021 => {
                self.apply_configured_anima(actor_side, card);
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let amount = other_param(card, 0).max(0);
                    if amount > 0 {
                        self.modify_target_hp(actor_side, -amount);
                        self.modify_target_max_hp(actor_side, -amount);
                    }
                }
            }
            7_000_025 => {
                self.apply_configured_defense(actor_side, card);
                let bonus = if self.check_wu_xing(actor_side, Element::Metal) {
                    let divisor = other_param(card, 1).max(1);
                    self.actor(actor_side).core.defense / divisor
                } else {
                    0
                };
                self.gain_sharpness(actor_side, other_param(card, 0).max(0) + bonus);
            }
            7_000_026 => {
                // 原版 Card_7000026.cs:62-64 金灵激活时读持有者持久
                // ActualDamage(302)（残留 + 本卡）→ 锋锐 +302/2。
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry;
                if self.check_wu_xing(actor_side, Element::Metal) && actual_damage > 0 {
                    self.gain_sharpness(actor_side, actual_damage / 2);
                }
            }
            7_000_032 => {
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let amount =
                        other_param(card, 1).max(0) + self.actor(actor_side).core.attack_bonus;
                    if amount > 0 {
                        self.modify_target_hp(actor_side, -amount);
                        self.modify_target_max_hp(actor_side, -amount);
                    }
                }
            }
            7_000_033 => {
                // Card_7000033.cs:46-50（土灵•流沙）：[土灵] 分支走完整
                // CardActionBase.CheckWuXing（激活 / 龙马精神 / UsedWuXing
                // 相生链 / 卡组含 7030077|7040077 五行刺恒真），不是仅看
                // 土灵激活。oracle 锚点：hf-latest-32308000-16f9c778
                // 62935cbae85c5db2/round-14 cp13 p2.hp 84（原版 21 = 9 +
                // 72 失去防御/6：卡组含 7040077 恒真；引擎 96 = 无加成 9）、
                // 98c1b2dae5fcfb69/round-14 cp15（同构 -3）。
                let bonus = if self.check_wu_xing(actor_side, Element::Earth) {
                    let divisor = other_param(card, 0).max(1);
                    self.actor(actor_side).turn.lost_defense_count / divisor
                } else {
                    0
                };
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
            }
            7_000_034 => {
                self.actor_mut(actor_side).elements.no_sharpness_for_attack += 1;
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
            }
            7_000_035 => {
                // 金灵•铁骨 Card_7000035.cs:80-82 —— CheckWuXing(src,
                // JiHuoJinLing) 完整语义（激活 / 上次使用五行及相生链 /
                // 龙马精神 / 卡组含 7030077|7040077 五行刺时恒真），不是
                // 仅看金灵激活。oracle 锚点：hf-latest-32308000-16f9c778
                // 032a9c253d375d03/round-12 cp41（卡组含 7040077 梦•五行刺
                // → 原版铁骨 0→2；引擎 is_element_activated 假 → 0）。
                if self.check_wu_xing(actor_side, Element::Metal) {
                    self.actor_mut(actor_side).elements.metal_iron_bone +=
                        other_param(card, 1).max(0);
                }
            }
            7_000_050 => {
                if self.check_wu_xing(actor_side, Element::Metal) {
                    self.gain_sharpness(actor_side, other_param(card, 0).max(0));
                }
                let sharpness = self.actor(actor_side).sword.sharpness.max(0);
                if self.check_wu_xing(actor_side, Element::Water) && sharpness > 0 {
                    let converted = div_ceil(sharpness, 2);
                    self.actor_mut(actor_side).sword.sharpness =
                        (self.actor(actor_side).sword.sharpness - converted).max(0);
                    self.gain_water_momentum(actor_side, converted);
                }
            }
            7_000_036 => {
                let max_hp_gain = other_param(card, 0).max(0);
                if max_hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, max_hp_gain);
                }
                self.actor_mut(actor_side).elements.wood_array += other_param(card, 1).max(0);
            }
            7_000_038 => {
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    self.modify_target_max_hp(actor_side, -amount);
                }
            }
            7_000_039 => {
                // 原版 Card_7000039.cs:95-108 火灵激活且持 ActualDamage(302)
                // 时减对方生命上限 302×otherParams[0]——与 413 焚花印同构，
                // 读持有者持久 302（残留 + 本卡）。
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                let actual_damage = self.actor(actor_side).turn.actual_damage_carry;
                if self.check_wu_xing(actor_side, Element::Fire) && actual_damage > 0 {
                    self.modify_target_max_hp(
                        actor_side,
                        -actual_damage * other_param(card, 0).max(0),
                    );
                }
            }
            7_000_040 => {
                // Card_7000040.cs:81（土灵•绝壁）[土灵] 分支走完整
                // CheckWuXing（含卡组 7040077 五行刺恒真），非仅激活判断。
                let bonus = if self.check_wu_xing(actor_side, Element::Earth) {
                    let divisor = other_param(card, 0).max(1);
                    self.actor(actor_side).turn.lost_defense_count / divisor
                } else {
                    0
                };
                let total = card.defense.unwrap_or(0).max(0) + bonus;
                if total > 0 {
                    self.gain_defense(actor_side, total);
                }
            }
            7_000_042 => {
                let active = self.check_wu_xing(actor_side, Element::Metal);
                if active {
                    self.actor_mut(actor_side).elements.metal_cauldron_drop += 1;
                }
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                if active {
                    self.actor_mut(actor_side).elements.metal_cauldron_drop = 0;
                }
            }
            7_000_043 => {
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                self.modify_actor_hp(actor_side, other_param(card, 1), false, false);
            }
            7_000_047 => {
                // 土灵·韵金：土灵分支先加防，再把当前防御清空并重新获得，
                // 因而新旧防御都会计入本回合失防；金灵分支随后读取该累计值。
                if self.check_wu_xing(actor_side, Element::Earth) {
                    self.gain_defense(actor_side, other_param(card, 0).max(0));
                    let defense = self.actor(actor_side).core.defense.max(0);
                    if defense > 0 {
                        self.lose_defense(actor_side, defense);
                        self.gain_defense(actor_side, defense);
                    }
                }
                if self.check_wu_xing(actor_side, Element::Metal) {
                    let divisor = other_param(card, 1).max(1);
                    let sharpness = self.actor(actor_side).turn.lost_defense_count / divisor;
                    if sharpness > 0 {
                        self.gain_sharpness(actor_side, sharpness);
                    }
                }
            }
            7_000_044 => {
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                if self.check_wu_xing(actor_side, Element::Water) {
                    let target_side = opponent_side(actor_side);
                    self.actor_mut(target_side).status.cannot_act += 1;
                }
            }
            7_000_053 => {
                self.apply_configured_defense(actor_side, card);
                // Card_7000053.cs（土灵•合八荒）[土灵] 分支走完整
                // CheckWuXing（含卡组 7040077 五行刺恒真），非仅激活判断。
                if self.check_wu_xing(actor_side, Element::Earth) {
                    self.actor_mut(actor_side).elements.earth_eight_wastes +=
                        other_param(card, 0).max(0);
                }
            }
            7_000_056 => {
                attacked |= self.attack_by_config(actor_side, card, 0, slot);
                // Card_7000056（火灵•烈燎原）[火灵] 分支走完整 CheckWuXing
                // （与 Card_7000053/7000096 同款），包含卡组含 7040077
                // 五行刺时恒真。oracle 锚点：6687c7e1ce03cb49/round-12
                // cp[13]（p1 maxHp 97→44；引擎原先 is_element_activated
                // 漏判导致 maxHp 不降）。
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let target_side = opponent_side(actor_side);
                    let delta = self.actor(target_side).core.max_hp
                        - self.actor(target_side).core.hp
                        + other_param(card, 0).max(0);
                    if delta > 0 {
                        self.modify_target_max_hp(actor_side, -delta);
                    }
                }
            }
            7_000_057 => {
                let momentum_gain = other_param(card, 0).max(0);
                if momentum_gain > 0 {
                    self.gain_water_momentum(actor_side, momentum_gain);
                }
                // Card_7000057.cs 读 CardActionBase.CheckWuXing(src,
                // JiHuoShuiLing)（激活 / 龙马精神 / UsedWuXing 相生链 /
                // 卡组含 7030077|7040077 五行刺恒真），不是仅看水灵激活。
                // oracle 锚点：hf-32308000 d7d6da3ccefac976/round-13 cp[9]
                // p2.maxHp 125（引擎 107：卡组含 7040077 时原版恒真，
                // 引擎 last_element 非水漏判 +18/+18）。
                if self.check_wu_xing(actor_side, Element::Water) {
                    let amount = other_param(card, 1).max(0)
                        + self.actor(actor_side).elements.water_momentum;
                    if amount > 0 {
                        self.modify_actor_max_hp(actor_side, amount);
                        self.modify_actor_hp(actor_side, amount, false, false);
                    }
                }
            }
            7_000_061 => {
                // Card_7000061.cs（木灵•暗香）:CheckWuXing(src, JiHuoMuLing)
                // —— 完整 CheckWuXing（激活 / 上次使用五行及相生链 / 龙马精神 /
                // 卡组含 7030077|7040077 五行刺恒真），不是仅看木灵激活。
                // oracle 锚点：95fa8fdf614cb5d4/round-10 cp[4]（p1 卡组含
                // 7030077、未激活木灵，anima 7/3=2 加攻原版 2 引擎 0）。
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                if self.check_wu_xing(actor_side, Element::Wood) {
                    let divisor = other_param(card, 1).max(1);
                    let attack_bonus = self.actor(actor_side).core.anima / divisor;
                    self.gain_attack_bonus(actor_side, attack_bonus);
                }
            }
            7_000_062 => {
                self.apply_configured_defense(actor_side, card);
                // Card_7000062.cs:97 CheckWuXing(JiHuoTuLing)（卡组含
                // 7030077|7040077 五行刺恒真），不是仅看土灵激活。
                if self.check_wu_xing(actor_side, Element::Earth) {
                    self.actor_mut(actor_side).elements.earth_cliff_counter +=
                        other_param(card, 0).max(0);
                }
            }
            7_000_066 => {
                let bonus =
                    wu_xing_count_in_deck(self.actor(actor_side)) * other_param(card, 0).max(0);
                let previous_shatter_defense = self.active_effect_shatter_defense();
                self.set_active_effect_shatter_defense(previous_shatter_defense + 1);
                attacked |= self.attack_by_config(actor_side, card, bonus, slot);
                self.set_active_effect_shatter_defense(previous_shatter_defense);
            }
            7_000_067 => {
                attacked |= self.apply_five_elements_cycle(actor_side, card, slot);
            }
            7_000_069 => {
                let amount = other_param(card, 0).max(0);
                if amount > 0 {
                    self.modify_target_hp(actor_side, -amount);
                    self.modify_target_max_hp(actor_side, -amount);
                }
                self.activate_element(actor_side, Element::Fire);
            }
            7_000_070 => {
                self.apply_configured_defense(actor_side, card);
                self.actor_mut(actor_side).turn.next_turn_defense += other_param(card, 0).max(0);
                self.activate_element(actor_side, Element::Earth);
            }
            7_000_071 => {
                self.activate_element(actor_side, Element::Metal);
                self.gain_sharpness(actor_side, other_param(card, 0).max(0));
            }
            7_000_072 => {
                let momentum_gain = other_param(card, 0).max(0);
                if momentum_gain > 0 {
                    self.gain_water_momentum(actor_side, momentum_gain);
                }
                self.activate_element(actor_side, Element::Water);
            }
            371 => {
                // Card_371.cs: Attack(attack, attackCount + src.GetWuXingActiveNumber()).
                let attack = card.attack.unwrap_or(0).max(0);
                let attack_count =
                    card.attack_count.unwrap_or(1).max(0) + self.wu_xing_active_number(actor_side);
                for _ in 0..attack_count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
            }
            _ => return None,
        }
        Some(attacked)
    }

    fn apply_five_elements_cycle(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        // 已证伪的假设（2026-08-08，DIAG_20260808_hf_32219000 采集 B A-2）：
        // cae463212f8c4c43/round-15 cp[5] 五行流转 temp 执行 烈燎原 原版承伤
        // 44 vs 引擎 36，曾假设「temp 执行在激活火灵分支后追加一段基础攻击
        // （attack，不加加攻）」。全批次验证：该追加在 mirror-32219000 批次
        // 回归 165 例（含全部 五行流转→烈燎原 temp 场景），仅 r15 1 例吻合，
        // 假设不成立、已回退。r15 的 +8 真实机制未明（烈燎原 temp 下
        // 激活火灵 delta 分支与 hp 钳制顺序的候选解释均被 checkpoint 数值
        // 排除），待事件级 oracle 证据再立项。
        let fire_generates_all = self.actor(actor_side).identity.talents.contains(&137);
        let Some(previous_card) = active_neighbor_card(self.actor(actor_side), slot, -1).cloned()
        else {
            return false;
        };
        let Some(next_card) = active_neighbor_card(self.actor(actor_side), slot, 1).cloned() else {
            return false;
        };
        if !is_five_element_generation(&previous_card, &next_card, fire_generates_all) {
            return false;
        }
        self.activate_element_by_card(actor_side, &next_card);
        let temporary_card = self.temporary_cycle_card(actor_side, card, slot, 1);
        let Some(temporary_card) = temporary_card else {
            return false;
        };
        let action_again = self.apply_temporary_card_effect(actor_side, &temporary_card, slot);
        // Card_7000067 reads the CardConfig.actionAgain value written by the
        // selected temporary card's complete body.
        if action_again {
            self.modify_extra_actions(actor_side, 1);
        }
        false
    }

    fn temporary_cycle_card(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        direction: i64,
    ) -> Option<CardDefinition> {
        let next_card = active_neighbor_card(self.actor(actor_side), slot, direction)?.clone();
        // 原版 Card_7000067.cs:60-66 的钳制比较读 CardConfig.rarity
        // （nextCardConfig.rarity vs card_.cardConfig.rarity），无 rarity
        // 字段的牌按 0 处理——不能沿用 id 档位推断（card_rarity）：梦•
        // 火灵聚炎（7020089/7040089 等，配置无 rarity）不会被钳制，按整卡
        // 执行（anima 3/4）。oracle 锚点：7a0da91b97b4d1e9/round-07
        // cp[0] p2.anima 3=0+3、t17u1 p1 15→5（2+8×1）；ac9dacde7087f49d/
        // round-13 cp[8] p1.anima 18=14+4、p2 81→43（2+18×2）、maxHp-38。
        // 普通牌 7020022（配置 rarity 2）仍钳制到 7000022（锚点：
        // cae463212f8c4c43/round-12 t7u1 承伤 29）。
        let max_rarity = super::original_config::original_config_rarity(card.id);
        let next_rarity = super::original_config::original_config_rarity(next_card.id);
        if next_rarity <= max_rarity {
            return Some(next_card);
        }
        let base_id = normalized_base_id(&next_card);
        let target_id = base_id + max_rarity * 10_000;
        self.actor(actor_side)
            .deck
            .slots
            .iter()
            .find(|slot| slot.card.id == target_id)
            .map(|slot| slot.card.clone())
            .or_else(|| super::original_config::original_card_definition(target_id))
    }
}

fn is_five_element_generation(
    previous: &CardDefinition,
    next: &CardDefinition,
    fire_generates_all: bool,
) -> bool {
    let previous_element = element_from_card(previous);
    let next_element = element_from_card(next);
    matches!(
        (previous_element, next_element),
        (Some(Element::Metal), Some(Element::Water))
            | (Some(Element::Water), Some(Element::Wood))
            | (Some(Element::Wood), Some(Element::Fire))
            | (Some(Element::Fire), Some(Element::Earth))
            | (Some(Element::Earth), Some(Element::Metal))
    ) || (fire_generates_all
        && previous_element != next_element
        && ((previous_element == Some(Element::Fire) && is_five_element_card(next))
            || (is_five_element_card(previous) && next_element == Some(Element::Fire))))
}
