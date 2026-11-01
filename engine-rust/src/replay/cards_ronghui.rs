use super::effect_invocation::{
    TemporaryDeckIdentityMode, TemporaryHadUsedSource, TemporaryInvocationSpec,
};
use super::flow_card_effect::{param0, param1, param2};
use super::original_config::{can_upgrade_original_battle_deck_card, original_card_definition};
use super::support::{card_rarity, has_card_trait, normalized_base_id, opponent_side, other_param};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn apply_synthetic_oracle_ronghui_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        match base_id {
            177 => {
                if self.actor(actor_side).ronghui.fu_xi_copy_guard > 0 {
                    return Some(false);
                }
                self.actor_mut(actor_side).ronghui.fu_xi_copy_guard += 1;
                // Card_177：dst.ShiftCard() 把对方队首的 skip 牌依次转到队尾（最多 8 次，满 8 次则当前牌重置为普攻），
                // UnshiftCard 放回后 GetCurrentCardInDeck 才是复制对象；旋转对对方队列持续生效。
                {
                    let deck = &mut self.actor_mut(target_side).deck;
                    let mut rotations = 0;
                    while rotations < 8 {
                        let Some(&head) = deck.queue.first() else {
                            break;
                        };
                        if !deck.slots.get(head).is_some_and(|slot| slot.skipped) {
                            break;
                        }
                        deck.queue.rotate_left(1);
                        rotations += 1;
                    }
                    if rotations == 8 {
                        if let Some(&head) = deck.queue.first() {
                            if let Some(slot) = deck.slots.get_mut(head) {
                                slot.card = super::support::basic_attack_card().into();
                                slot.skipped = false;
                            }
                        }
                    }
                }
                let selected_id = self
                    .actor(target_side)
                    .deck
                    .queue
                    .first()
                    .and_then(|index| self.actor(target_side).deck.slots.get(*index))
                    .map(|slot_state| slot_state.card.id);
                if let Some(selected_id) = selected_id {
                    self.execute_ronghui_temporary_card(
                        actor_side,
                        selected_id,
                        TemporaryInvocationSpec {
                            physical_slot: slot,
                            invocation_slot: slot,
                            had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
                            deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
                            inherit_parent_beng_quan: true,
                        },
                    );
                    self.modify_star_chess_break(target_side, 1);
                } else {
                    self.missing_decision("card:177:opponent next card");
                }
                self.actor_mut(actor_side).ronghui.fu_xi_copy_guard -= 1;
                // Card_177 records its successful outer body separately from
                // the copied card and CardActionBase's career-wide counter.
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            178 => {
                self.modify_actor_max_hp(actor_side, param0(card));
                self.actor_mut(actor_side).ronghui.five_emperors_upgrade += param1(card);
                Some(false)
            }
            179 => {
                if self.actor(actor_side).ronghui.dong_huang_copy_guard > 0 {
                    return Some(false);
                }
                self.actor_mut(actor_side).ronghui.dong_huang_copy_guard += 1;
                // Card_179.cs rereads hadUsed and cardConfig.id from the live
                // deck for each grid after the preceding temporary ExecuteEffect.
                // Do not snapshot the prefix: nested effects can mutate a later
                // slot before its iteration is reached.
                for grid in 0..slot {
                    let Some((selected_id, had_used)) = self
                        .actor(actor_side)
                        .deck
                        .slots
                        .get(grid)
                        .filter(|slot_state| slot_state.used)
                        .map(|slot_state| (slot_state.card.id, slot_state.used))
                    else {
                        continue;
                    };
                    self.execute_ronghui_temporary_card(
                        actor_side,
                        selected_id,
                        TemporaryInvocationSpec {
                            physical_slot: slot,
                            invocation_slot: grid,
                            had_used_source: TemporaryHadUsedSource::Explicit(had_used),
                            // 东皇钟 changes the outer CardItem's CardConfig and
                            // gridNumber, but writes its own id back before the
                            // nested ExecuteEffect. Deck-wide censuses therefore
                            // continue to see the physical 东皇钟 identity.
                            deck_identity_mode: TemporaryDeckIdentityMode::PreservePhysical,
                            inherit_parent_beng_quan: true,
                        },
                    );
                }
                self.actor_mut(actor_side).ronghui.dong_huang_copy_guard -= 1;
                Some(false)
            }
            181 => {
                self.actor_mut(target_side).ronghui.alchemy_pot += param0(card);
                Some(false)
            }
            185 => {
                self.actor_mut(actor_side)
                    .turn
                    .spirit_control_anima_gain_defense += param0(card);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            186 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_actor_hp(actor_side, param1(card), false, false);
                for _ in 0..param0(card) {
                    let selected_id = self.consume_ronghui_optional_decision();
                    if selected_id >= 0 {
                        self.execute_ronghui_temporary_card(
                            actor_side,
                            selected_id,
                            TemporaryInvocationSpec {
                                physical_slot: slot,
                                invocation_slot: slot,
                                had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
                                deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
                                inherit_parent_beng_quan: true,
                            },
                        );
                    }
                }
                Some(false)
            }
            187 => {
                self.actor_mut(actor_side).sword.cloud_sword_heaven_cycle += param0(card);
                Some(false)
            }
            188 => {
                self.apply_configured_anima(actor_side, card);
                let spirit_swords = self
                    .actor(actor_side)
                    .deck
                    .slots
                    .iter()
                    .filter(|slot_state| slot_state.card.name.contains("灵剑"))
                    .count() as i64;
                self.gain_anima(actor_side, spirit_swords.min(param0(card)));
                Some(false)
            }
            189 => {
                let uses = param0(card);
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).ronghui.earth_fiend_defense += uses;
                self.modify_next_attack_shatter_defense(actor_side, uses);
                Some(false)
            }
            190 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).ronghui.spirit_sparrow_behind += param0(card);
                self.actor_mut(actor_side)
                    .ronghui
                    .yellow_bird_cost_reduction += param1(card);
                Some(false)
            }
            191 => {
                self.add_actor_negative_status(target_side, 100, param0(card));
                self.modify_actor_hp(actor_side, param1(card), false, false);
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                // Card_191.cs:84-87 consumes all Hexagram via
                // ModifyBuffValue before applying the captured amount.
                self.modify_hexagram(actor_side, -hexagram);
                self.add_actor_negative_status(target_side, 100, hexagram);
                self.add_actor_negative_status(target_side, 101, hexagram);
                Some(false)
            }
            192 => {
                let hp = param0(card);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                let damage = param1(card);
                self.actor_mut(actor_side).ronghui.thunder_tune += damage;
                self.actor_mut(target_side).ronghui.thunder_tune += damage;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            193 => {
                let count = self.consume_ronghui_optional_original_random(actor_side);
                for _ in 0..count.max(0) {
                    let status = self.consume_ronghui_optional_decision();
                    if status < 0 {
                        continue;
                    }
                    if !matches!(status, 100 | 101 | 102 | 103 | 104 | 105 | 367 | 393) {
                        self.missing_decision("card:193:negative status");
                        continue;
                    }
                    self.add_actor_negative_status(target_side, status, 1);
                }
                Some(false)
            }
            194 => {
                self.actor_mut(actor_side).ronghui.two_polarity_vajra += param0(card);
                self.actor_mut(actor_side)
                    .ronghui
                    .two_polarity_anima_multiplier = param1(card);
                self.actor_mut(actor_side)
                    .ronghui
                    .two_polarity_hexagram_multiplier = param2(card);
                Some(false)
            }
            195 => {
                if !was_used_before_effect {
                    self.add_actor_negative_status(target_side, 104, param0(card));
                }
                self.add_actor_negative_status(target_side, 105, param1(card));
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    return Some(self.attack_by_config(actor_side, card, 0, slot));
                }
                Some(false)
            }
            196 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    let count = self.actor(actor_side).deck.active_slot_count.max(1);
                    let step: i64 = if self.actor(actor_side).fate.reverse_card_direction > 0 {
                        -1
                    } else {
                        1
                    };
                    let mut grid = slot;
                    for _ in 0..param0(card) {
                        grid = (grid as i64 + step).rem_euclid(count as i64) as usize;
                        if self
                            .actor(actor_side)
                            .deck
                            .slots
                            .get(grid)
                            .is_some_and(|slot_state| slot_state.card.name.contains("星弈"))
                        {
                            self.actor_mut(actor_side).ronghui.star_chess_jump += 1;
                            break;
                        }
                    }
                }
                Some(attacked)
            }
            198 => {
                self.gain_attack_bonus(actor_side, param0(card));
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let layers = param1(card);
                    self.reduce_all_actor_negative_statuses(actor_side, layers);
                    self.reduce_all_actor_negative_statuses(target_side, layers);
                }
                Some(false)
            }
            199 => {
                self.apply_configured_anima(actor_side, card);
                let attack = param0(card);
                for side in [actor_side, target_side] {
                    self.gain_attack_bonus(side, attack);
                    self.actor_mut(side).ronghui.all_cards_action_again = 1;
                }
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            201 => {
                self.gain_attack_bonus(actor_side, param0(card));
                self.actor_mut(actor_side).elements.wood_healing_formation += 1;
                Some(false)
            }
            202 => Some(false),
            203 => {
                self.gain_water_momentum(actor_side, param0(card));
                Some(false)
            }
            206 => {
                self.gain_agility(actor_side, param0(card));
                self.actor_mut(actor_side).ronghui.free_and_easy_tune += 1;
                self.actor_mut(target_side).ronghui.free_and_easy_tune += 1;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            207 => {
                self.apply_configured_physique(actor_side, card);
                self.actor_mut(actor_side).core.physique_limit += param0(card);
                self.gain_attack_bonus(actor_side, param1(card));
                Some(false)
            }
            208 => {
                self.modify_momentum_limit(actor_side, param2(card));
                self.actor_mut(actor_side).ronghui.momentum_formation += param0(card);
                Some(false)
            }
            209 => {
                self.modify_actor_hp(actor_side, param1(card), false, false);
                self.actor_mut(actor_side).ronghui.snow_lotus_mirror += param0(card);
                Some(false)
            }
            210 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).ronghui.reverse_gu_attack += param0(card);
                Some(attacked)
            }
            389 => {
                self.apply_configured_anima(actor_side, card);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.add_actor_negative_status(target_side, 101, param0(card));
                    self.add_actor_negative_status(target_side, 102, param1(card));
                }
                Some(false)
            }
            390 => {
                if self
                    .actor(actor_side)
                    .ronghui
                    .five_elements_dream_copy_guard
                    > 0
                {
                    return Some(false);
                }
                self.actor_mut(actor_side)
                    .ronghui
                    .five_elements_dream_copy_guard += 1;
                let hp = param0(card);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                let selected_id = self.actor(actor_side).ronghui.five_elements_bottle_card_id;
                // Card_390.cs 读 talentDatas[199].commonParams[0]；玉瓶首格为空
                // （cardId=0）时原版仍对 cardId 0 执行 ExecuteEffect ——
                // cardId 0 即普通攻击（3攻），攻击照样结算。oracle 锚点：
                // mirror-32299000 pa_78ab0a36 cp6 p1.hp 67 vs 70
                // （忘忧梦复制的空玉瓶打出 3 攻）。因此空玉瓶不再跳过，而是
                // 以普通攻击定义执行临时卡。
                if selected_id > 0 {
                    self.execute_ronghui_temporary_card(
                        actor_side,
                        selected_id,
                        TemporaryInvocationSpec {
                            physical_slot: slot,
                            invocation_slot: slot,
                            had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
                            deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
                            inherit_parent_beng_quan: true,
                        },
                    );
                } else if let Some(basic) = super::original_config::original_card_definition(0) {
                    self.execute_ronghui_temporary_card(
                        actor_side,
                        basic.id,
                        TemporaryInvocationSpec {
                            physical_slot: slot,
                            invocation_slot: slot,
                            had_used_source: TemporaryHadUsedSource::PhysicalAtEntry,
                            deck_identity_mode: TemporaryDeckIdentityMode::ReplaceWithEffective,
                            inherit_parent_beng_quan: true,
                        },
                    );
                }
                self.actor_mut(actor_side)
                    .ronghui
                    .five_elements_dream_copy_guard -= 1;
                Some(false)
            }
            // —— build 25981480 实测出现的融会/极牌（Card_*.cs 逐张取证）——
            402 => {
                // Card_402：灵气 → 云海 +otherParams[0] → 奇数格（gridNumber+1 为奇）FuYunLianHuanBu +1，偶数格身法 +otherParams[1]。
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).sword.cloud_sea += param0(card);
                if (slot + 1) % 2 == 1 {
                    self.actor_mut(actor_side).ronghui.fu_yun_lian_huan_bu += 1;
                } else {
                    self.gain_agility(actor_side, param1(card));
                }
                Some(false)
            }
            405 => {
                // Card_405：灵气 → 生命上限 +otherParams[0] → 回复 星力+当前灵气 → actionAgain = 后招 || 星位。
                self.apply_configured_anima(actor_side, card);
                self.modify_actor_max_hp(actor_side, param0(card));
                let heal =
                    self.actor(actor_side).astrology.star_power + self.actor(actor_side).core.anima;
                if heal > 0 {
                    self.modify_actor_hp(actor_side, heal, false, false);
                }
                let _ = self.check_rear_move(actor_side, was_used_before_effect);
                Some(false)
            }
            416 => {
                // Card_416：灵气 → 身法 +otherParams[1] → 灵玄不息 +otherParams[0]。
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, param1(card));
                self.actor_mut(actor_side).ronghui.ling_xuan_bu_xi += param0(card);
                Some(false)
            }
            419 => {
                // Card_419：双方各加内伤/虚弱 → 每层自身负面状态按架势回血（拳）或造成伤害（棍）→ 切换架势。
                let internal = other_param(card, 0);
                let weak = other_param(card, 1);
                if internal > 0 {
                    self.add_actor_negative_status(actor_side, 100, internal);
                    self.add_actor_negative_status(target_side, 100, internal);
                }
                if weak > 0 {
                    self.add_actor_negative_status(actor_side, 101, weak);
                    self.add_actor_negative_status(target_side, 101, weak);
                }
                let amount = other_param(card, 2) * self.negative_status_stack_count(actor_side);
                if amount > 0 {
                    if self.actor(actor_side).beng.quan_stance > 0 {
                        self.modify_actor_hp(actor_side, amount, false, false);
                    }
                    if self.actor(actor_side).beng.gun_stance > 0 {
                        self.apply_damage(actor_side, amount, false, false, false);
                    }
                }
                if self.has_locked_li_stance(actor_side) {
                } else if self.actor(actor_side).beng.quan_stance > 0 {
                    self.actor_mut(actor_side).beng.quan_stance -= 1;
                    self.actor_mut(actor_side).beng.gun_stance += 1;
                } else if self.actor(actor_side).beng.gun_stance > 0 {
                    self.actor_mut(actor_side).beng.gun_stance -= 1;
                    self.actor_mut(actor_side).beng.quan_stance += 1;
                }
                self.apply_fate_strategy_stance_switch(actor_side);
                Some(false)
            }
            436 => {
                // Card_436：攻击后重复 otherParams[0] 次 GetNextRandomValue 或 Random.Range(otherParams[1], otherParams[2]+1) 层内伤。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                for _ in 0..param0(card) {
                    let layers = self.consume_random_range(
                        actor_side,
                        other_param(card, 1),
                        other_param(card, 2),
                    );
                    self.add_actor_negative_status(target_side, 100, layers);
                }
                Some(attacked)
            }
            437 => {
                // Card_437：攻击后体魄 +physique + 累计失去生命 × otherParams[0] / 100。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                let delta = card.physique.unwrap_or(0)
                    + self.actor(actor_side).turn.lose_hp_count * other_param(card, 0) / 100;
                self.apply_physique_amount(actor_side, delta);
                Some(attacked)
            }
            440 => {
                // Card_440：攻击 → 灵气 → 体魄 → 身法 +otherParams[0]。
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_anima(actor_side, card);
                self.apply_physique_amount(actor_side, card.physique.unwrap_or(0));
                self.gain_agility(actor_side, param0(card));
                Some(attacked)
            }
            441 => {
                // Card_441：防 → 直写 JiHuoTuLing；被动（每回合末按防/otherParams[0] 造成伤害）由 turn-end 阶段读卡组结算。
                self.apply_configured_defense(actor_side, card);
                self.add_direct_element_activation(actor_side, Element::Earth);
                Some(false)
            }
            455 => {
                // Card_455：灵气 → 依卡组顺序逐个五行激活 → 卡组恰含两种五行时 TianSuiNiKeJue +1。
                self.apply_configured_anima(actor_side, card);
                let mut found: Vec<Element> = Vec::new();
                let actor = self.actor(actor_side);
                for deck_card in super::support::active_deck_cards(actor) {
                    for (token, element) in [
                        ("金灵", Element::Metal),
                        ("水灵", Element::Water),
                        ("木灵", Element::Wood),
                        ("火灵", Element::Fire),
                        ("土灵", Element::Earth),
                    ] {
                        if deck_card.name.contains(token) && !found.contains(&element) {
                            found.push(element);
                        }
                    }
                }
                let two_only = super::support::wu_xing_count_in_deck(self.actor(actor_side)) == 2
                    && found.len() == 2;
                for element in found {
                    self.activate_element(actor_side, element);
                }
                if two_only {
                    self.actor_mut(actor_side).ronghui.tian_sui_ni_ke_jue += 1;
                }
                Some(false)
            }
            456 => {
                // Card_456：对方有负面状态则多攻击 1 次；星位 → 困缚 +otherParams[0]。
                let extra = i64::from(self.negative_status_stack_count(target_side) > 0);
                let attack = card.attack.unwrap_or(0);
                let count = card.attack_count.unwrap_or(1).max(0) + extra;
                let mut attacked = false;
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                        attacked = true;
                    }
                }
                if self.actor(actor_side).astrology.star_slots.contains(&slot)
                    || self.actor(actor_side).elements.long_ma_spirit > 0
                {
                    self.add_actor_negative_status(target_side, 104, param0(card));
                }
                Some(attacked)
            }
            7_000_106 => {
                self.actor_mut(actor_side).elements.seal_suppressing_mindset += param0(card);
                Some(false)
            }
            7_000_117 => {
                // Card_7000117：灵气 → 加攻 +otherParams[0] → 火灵：对方失去（otherParams[1] + 加攻层数）生命及上限。
                self.apply_configured_anima(actor_side, card);
                self.gain_attack_bonus(actor_side, param0(card));
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let loss = other_param(card, 1) + self.actor(actor_side).core.attack_bonus;
                    self.modify_actor_hp(target_side, -loss, false, false);
                    self.modify_actor_max_hp(target_side, -loss);
                }
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn apply_ronghui_five_emperors_upgrade_transform(
        &mut self,
        actor_side: PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        if self.actor(actor_side).ronghui.five_emperors_upgrade > 0
            && can_upgrade_original_battle_deck_card(drawn.card.id)
        {
            if let Some(upgraded) = original_card_definition(drawn.card.id + 10_000) {
                self.actor_mut(actor_side).ronghui.five_emperors_upgrade -= 1;
                self.replace_ronghui_drawn_card(actor_side, drawn, upgraded);
            } else {
                self.missing_decision("card:178:upgrade definition");
            }
        }
    }

    pub(super) fn apply_ronghui_alchemy_pot_transform(
        &mut self,
        actor_side: PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        if self.actor(actor_side).ronghui.alchemy_pot > 0 {
            self.actor_mut(actor_side).ronghui.alchemy_pot -= 1;
            // CardActionBase reads the config field, never an id-derived
            // upgrade rank. A missing rarity is CardConfig's default zero.
            if drawn.card.rarity.unwrap_or(0) >= 1 && drawn.card.id != 19 {
                if let Some(lowered) = original_card_definition(drawn.card.id - 10_000) {
                    self.replace_ronghui_drawn_card(actor_side, drawn, lowered);
                } else {
                    self.missing_decision("card:181:downgrade definition");
                }
            } else {
                let drain = original_card_definition(181)
                    .map(|card| param1(&card))
                    .unwrap_or(0);
                let target_side = opponent_side(actor_side);
                self.modify_actor_hp(actor_side, -drain, false, false);
                self.modify_actor_max_hp(actor_side, -drain);
                self.modify_actor_max_hp(target_side, drain);
                self.modify_actor_hp(target_side, drain, false, false);
            }
        }
    }

    pub(super) fn apply_ronghui_free_and_easy_tune_transform(
        &mut self,
        actor_side: PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        if self.actor(actor_side).ronghui.free_and_easy_tune > 0
            && normalized_base_id(&drawn.card) == 0
        {
            self.actor_mut(actor_side).ronghui.free_and_easy_tune -= 1;
            let count = self.actor(actor_side).deck.active_slot_count.max(1);
            let direction: i64 = if self.actor(actor_side).fate.reverse_card_direction > 0 {
                1
            } else {
                -1
            };
            let previous = (drawn.source_slot as i64 + direction).rem_euclid(count as i64) as usize;
            let previous_card = self
                .actor(actor_side)
                .deck
                .slots
                .get(previous)
                .map(|slot_state| slot_state.card.clone());
            if let Some(previous_card) = previous_card {
                let replacement_id = if can_upgrade_original_battle_deck_card(previous_card.id) {
                    previous_card.id + 10_000
                } else {
                    previous_card.id
                };
                if let Some(replacement) = original_card_definition(replacement_id) {
                    self.replace_ronghui_drawn_card(actor_side, drawn, replacement);
                } else {
                    self.missing_decision("card:206:replacement definition");
                }
            } else {
                self.missing_decision("card:206:previous active grid");
            }
        }
    }

    pub(super) fn reduce_ronghui_rear_move_anima_cost(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        cost: i64,
    ) -> i64 {
        if cost <= 0 || !has_card_trait(card, "rearMove") {
            return cost;
        }
        (cost
            - self
                .actor(actor_side)
                .ronghui
                .yellow_bird_cost_reduction
                .max(0))
        .max(0)
    }

    pub(super) fn apply_ronghui_after_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        let cloud_attack = self.actor(actor_side).sword.cloud_sword_heaven_cycle.max(0);
        if cloud_attack > 0 && card.career_name.as_deref() == Some("ZhenFaShi") {
            self.apply_attack_with_options(
                actor_side,
                cloud_attack,
                slot,
                false,
                false,
                0,
                Some("buff:cloudSwordHeavenCycle"),
            );
        }
        // 云剑•猫影（卡 403，buff 757 YunJianMaoYing）OnAfterExecuted 追加
        // 攻击（CardActionBase.cs:4413-4427，synthetic batch-027 pair 1）：
        // 每次使用云剑后按 buff 层数追加攻击，紧跟云剑周天（IL_0415→
        // IL_0513 原版顺序）。
        let mao_ying = self.actor(actor_side).sword.yun_jian_mao_ying.max(0);
        if mao_ying > 0 && super::support::is_cloud_sword(self.actor(actor_side), card) {
            self.apply_attack_with_options(
                actor_side,
                mao_ying,
                slot,
                false,
                false,
                0,
                Some("buff:yunJianMaoYing"),
            );
        }
        // CardActionBase.OnAfterExecuted IL_0f82：灵玄不息（LingXuanBuXi）在云剑猫影与五行共鸣 132 追加攻击之后，
        // 追加 buff 层数 + 自身负面状态层数/2 点攻击。
        let ling_xuan = self.actor(actor_side).ronghui.ling_xuan_bu_xi.max(0);
        if ling_xuan > 0 {
            let amount = ling_xuan + self.negative_status_stack_count(actor_side) / 2;
            self.apply_attack_with_options(
                actor_side,
                amount,
                slot,
                false,
                false,
                0,
                Some("buff:lingXuanBuXi"),
            );
        }
    }

    /// CardActionBase.OnBeforeExecuted IL_14cc（:3589，无 isTempCard 守卫）天髓逆克诀：
    /// 五行牌与上一张五行牌存在克制（IsCunZaiWuXingKeZhi）时 ExActionAgain+1。
    /// 临时执行同样触发（五行阵旗临时执行土灵阵，oracle pa_92ebc763）。
    pub(super) fn apply_tian_sui_ni_ke_jue_before_card(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) {
        if self.actor(actor_side).ronghui.tian_sui_ni_ke_jue > 0
            && !super::support::elements_in_card_name(card).is_empty()
            && self.actor(actor_side).turn.extra_actions <= 0
            && self.actor(actor_side).turn.action_again_count
                < self.dream_mirage_action_again_limit(actor_side)
        {
            let last = self.actor(actor_side).elements.last_element;
            let elements = super::support::elements_in_card_name(card);
            let ke_zhi = match last {
                Some(Element::Metal) => elements
                    .iter()
                    .any(|e| matches!(e, Element::Wood | Element::Fire)),
                Some(Element::Wood) => elements
                    .iter()
                    .any(|e| matches!(e, Element::Earth | Element::Metal)),
                Some(Element::Earth) => elements
                    .iter()
                    .any(|e| matches!(e, Element::Water | Element::Wood)),
                Some(Element::Water) => elements
                    .iter()
                    .any(|e| matches!(e, Element::Fire | Element::Earth)),
                Some(Element::Fire) => elements
                    .iter()
                    .any(|e| matches!(e, Element::Metal | Element::Water)),
                None => false,
            };
            if ke_zhi {
                self.modify_extra_actions(actor_side, 1);
            }
        }
    }

    pub(super) fn apply_ronghui_spirit_sparrow_after_card(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        let rear_attack = self.actor(actor_side).ronghui.spirit_sparrow_behind.max(0);
        if rear_attack > 0 && self.actor(actor_side).fate.used_rear_move_check > 0 {
            self.apply_attack_with_options(
                actor_side,
                rear_attack,
                slot,
                false,
                false,
                0,
                Some("buff:spiritSparrowBehind"),
            );
        }
    }

    pub(super) fn apply_ronghui_turn_start(&mut self, actor_side: PlayerSide) {
        if self.actor(actor_side).ronghui.momentum_formation <= 0 {
            return;
        }
        let gain = original_card_definition(208)
            .map(|card| param1(&card))
            .unwrap_or(0);
        self.gain_anima(actor_side, gain);
        self.modify_momentum(actor_side, gain);
        self.actor_mut(actor_side).ronghui.momentum_formation -= 1;
    }

    pub(super) fn apply_ronghui_turn_end(&mut self, actor_side: PlayerSide) {
        let charges = self.actor(actor_side).ronghui.two_polarity_vajra.max(0);
        let anima = self.actor(actor_side).core.anima.max(0);
        let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
        if charges > 0 && (anima > 0 || hexagram > 0) {
            let amount = anima
                * self
                    .actor(actor_side)
                    .ronghui
                    .two_polarity_anima_multiplier
                    .max(0)
                + hexagram
                    * self
                        .actor(actor_side)
                        .ronghui
                        .two_polarity_hexagram_multiplier
                        .max(0);
            self.gain_defense(actor_side, amount);
            self.modify_actor_hp(actor_side, amount, false, false);
            self.actor_mut(actor_side).ronghui.two_polarity_vajra -= 1;
        }

        let thunder = self.actor(actor_side).ronghui.thunder_tune.max(0);
        if thunder > 0 {
            let roll = self.consume_ronghui_optional_original_random(actor_side);
            if roll < 10 {
                self.apply_damage(actor_side, thunder, false, false, false);
            }
        }
    }

    pub(super) fn apply_ronghui_post_attack(
        &mut self,
        actor_side: PlayerSide,
        hp_lost: i64,
        earth_fiend_active_before_attack: bool,
    ) {
        if !earth_fiend_active_before_attack {
            return;
        }
        self.actor_mut(actor_side).ronghui.earth_fiend_defense -= 1;
        if hp_lost > 0 {
            self.gain_defense(actor_side, hp_lost);
        }
    }

    pub(super) fn apply_reverse_gu_before_attack(&mut self, actor_side: PlayerSide) -> i64 {
        if self.actor(actor_side).ronghui.reverse_gu_attack <= 0 {
            return 0;
        }
        self.actor_mut(actor_side).ronghui.reverse_gu_attack -= 1;
        let target_side = opponent_side(actor_side);
        let snapshot: Vec<(i64, i64)> = self
            .negative_status_types_present(target_side)
            .into_iter()
            .map(|status| (status, self.negative_status_amount(target_side, status)))
            .collect();
        let removed: i64 = snapshot.iter().map(|(_, amount)| *amount).sum();
        for (status, amount) in snapshot {
            self.add_actor_negative_status(actor_side, status, amount);
            self.remove_actor_negative_status(target_side, status, amount);
        }
        removed
    }

    pub(super) fn apply_ronghui_negative_status_mirror(
        &mut self,
        actor_side: PlayerSide,
        status: i64,
        actual_delta: i64,
    ) {
        if actual_delta <= 0 || self.actor(actor_side).ronghui.snow_lotus_mirror <= 0 {
            return;
        }
        self.actor_mut(actor_side).ronghui.snow_lotus_mirror -= 1;
        self.add_actor_negative_status(opponent_side(actor_side), status, actual_delta);
    }

    pub(super) fn apply_ronghui_battle_start_opening(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        match normalized_base_id(card) {
            181 => {
                self.actor_mut(opponent_side(actor_side))
                    .ronghui
                    .alchemy_pot += 2;
            }
            203 => {
                self.gain_anima(actor_side, param1(card));
                self.gain_water_momentum(actor_side, param2(card));
            }
            389 => self.apply_star_chess_opening(actor_side, card, slot),
            _ => {}
        }
    }

    pub(super) fn ronghui_card_has_opening_effect(base_id: i64) -> bool {
        matches!(base_id, 181 | 203 | 389)
    }

    fn apply_star_chess_opening(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        let count = self.actor(actor_side).deck.active_slot_count.max(1);
        let reverse = self.actor(actor_side).fate.reverse_card_direction > 0;
        let offsets: [i64; 2] = if reverse { [1, -1] } else { [-1, 1] };
        for offset in offsets {
            let grid = (slot as i64 + offset).rem_euclid(count as i64) as usize;
            if self.actor(actor_side).astrology.star_slots.contains(&grid) {
                self.gain_anima(actor_side, 1);
            } else {
                self.actor_mut(actor_side).astrology.star_slots.push(grid);
                self.actor_mut(actor_side)
                    .astrology
                    .star_slots
                    .sort_unstable();
            }
            let is_basic = self
                .actor(actor_side)
                .deck
                .slots
                .get(grid)
                .is_some_and(|slot_state| normalized_base_id(&slot_state.card) == 0);
            if is_basic {
                let replacement_id = 4_000_038 + card_rarity(card) * 10_000;
                if let Some(replacement) = original_card_definition(replacement_id) {
                    self.actor_mut(actor_side).deck.slots[grid].card = replacement.into();
                } else {
                    self.missing_decision("card:389:opening replacement definition");
                }
            }
        }
    }

    fn replace_ronghui_drawn_card(
        &mut self,
        actor_side: PlayerSide,
        drawn: &mut super::DrawnCard,
        replacement: CardDefinition,
    ) {
        if let Some(slot_state) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot_state.card = replacement.clone().into();
        }
        drawn.card = replacement.into();
    }

    fn consume_ronghui_optional_decision(&mut self) -> i64 {
        if self.decision_tape.is_empty() {
            -1
        } else {
            self.decision_tape.remove(0)
        }
    }

    fn consume_ronghui_optional_original_random(&mut self, actor_side: PlayerSide) -> i64 {
        self.consume_original_random_hexagram_side_effects(actor_side);
        self.consume_ronghui_optional_decision()
    }

    fn negative_status_amount(&self, actor_side: PlayerSide, status: i64) -> i64 {
        let actor = self.actor(actor_side);
        match status {
            100 => actor.status.internal_injury,
            101 => actor.status.weakness,
            102 => actor.status.flaw,
            103 => actor.status.attack_reduction,
            104 => actor.status.entangle,
            105 => actor.status.external_injury,
            367 => actor.status.meditation,
            393 => actor.status.lost_mind,
            _ => 0,
        }
        .max(0)
    }

    fn execute_ronghui_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        selected_id: i64,
        spec: TemporaryInvocationSpec,
    ) {
        let Some(mut selected) = original_card_definition(selected_id) else {
            self.missing_decision("ronghui temporary card definition");
            return;
        };
        // Card_179.cs calls InitData(id) then RefreshSpecialCard(publicData)
        // before ExecuteEffect. CardItem.RefreshSpecialCard rebuilds card 19
        // from the copier's current talents and talentTempDatas (Card_19.cs),
        // so its attack/defense must not use the catalog's unrefreshed values.
        if normalized_base_id(&selected) == 19 {
            let actor = self.actor(actor_side);
            selected = super::support::adapt_cheng_xin_card(
                selected,
                &actor.identity.talents,
                actor.identity.sword_forging_attack,
            );
        }
        if self.apply_temporary_card_effect_with_spec(actor_side, &selected, spec) {
            self.modify_extra_actions(actor_side, 1);
        }
    }
}
