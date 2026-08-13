use super::effect_invocation::{
    TemporaryDeckIdentityMode, TemporaryHadUsedSource, TemporaryInvocationSpec,
};
use super::original_config::{can_upgrade_original_battle_deck_card, original_card_definition};
use super::support::{card_rarity, has_card_trait, normalized_base_id, opponent_side, other_param};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
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
                self.modify_actor_max_hp(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).ronghui.five_emperors_upgrade +=
                    other_param(card, 1).max(0);
                Some(false)
            }
            179 => {
                if self.actor(actor_side).ronghui.dong_huang_copy_guard > 0 {
                    return Some(false);
                }
                self.actor_mut(actor_side).ronghui.dong_huang_copy_guard += 1;
                let prior_cards: Vec<(usize, i64, bool)> = (0..slot)
                    .filter_map(|grid| {
                        self.actor(actor_side)
                            .deck
                            .slots
                            .get(grid)
                            .filter(|slot_state| slot_state.used)
                            .map(|slot_state| (grid, slot_state.card.id, slot_state.used))
                    })
                    .collect();
                for (grid, selected_id, had_used) in prior_cards {
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
                self.actor_mut(target_side).ronghui.alchemy_pot += other_param(card, 0).max(0);
                Some(false)
            }
            185 => {
                self.actor_mut(actor_side)
                    .turn
                    .spirit_control_anima_gain_defense += other_param(card, 0).max(0);
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            186 => {
                self.apply_configured_defense(actor_side, card);
                self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                for _ in 0..other_param(card, 0).max(0) {
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
                self.actor_mut(actor_side).sword.cloud_sword_heaven_cycle +=
                    other_param(card, 0).max(0);
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
                self.gain_anima(actor_side, spirit_swords.min(other_param(card, 0).max(0)));
                Some(false)
            }
            189 => {
                let uses = other_param(card, 0).max(0);
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).ronghui.earth_fiend_defense += uses;
                self.modify_next_attack_shatter_defense(actor_side, uses);
                Some(false)
            }
            190 => {
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).ronghui.spirit_sparrow_behind +=
                    other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .ronghui
                    .yellow_bird_cost_reduction += other_param(card, 1).max(0);
                Some(false)
            }
            191 => {
                self.add_actor_negative_status(target_side, 100, other_param(card, 0).max(0));
                self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                // Card_191.cs:84-87 consumes all Hexagram via
                // ModifyBuffValue before applying the captured amount.
                self.modify_hexagram(actor_side, -hexagram);
                self.add_actor_negative_status(target_side, 100, hexagram);
                self.add_actor_negative_status(target_side, 101, hexagram);
                Some(false)
            }
            192 => {
                let hp = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                let damage = other_param(card, 1).max(0);
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
                self.actor_mut(actor_side).ronghui.two_polarity_vajra +=
                    other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .ronghui
                    .two_polarity_anima_multiplier = other_param(card, 1).max(0);
                self.actor_mut(actor_side)
                    .ronghui
                    .two_polarity_hexagram_multiplier = other_param(card, 2).max(0);
                Some(false)
            }
            195 => {
                if !was_used_before_effect {
                    self.add_actor_negative_status(target_side, 104, other_param(card, 0).max(0));
                }
                self.add_actor_negative_status(target_side, 105, other_param(card, 1).max(0));
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
                    for _ in 0..other_param(card, 0).max(0) {
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
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let layers = other_param(card, 1).max(0);
                    self.reduce_all_actor_negative_statuses(actor_side, layers);
                    self.reduce_all_actor_negative_statuses(target_side, layers);
                }
                Some(false)
            }
            199 => {
                self.apply_configured_anima(actor_side, card);
                let attack = other_param(card, 0).max(0);
                for side in [actor_side, target_side] {
                    self.gain_attack_bonus(side, attack);
                    self.actor_mut(side).ronghui.all_cards_action_again = 1;
                }
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            201 => {
                self.gain_attack_bonus(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).elements.wood_healing_formation += 1;
                Some(false)
            }
            202 => Some(false),
            203 => {
                self.gain_water_momentum(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            206 => {
                self.gain_agility(actor_side, other_param(card, 0).max(0));
                self.actor_mut(actor_side).ronghui.free_and_easy_tune += 1;
                self.actor_mut(target_side).ronghui.free_and_easy_tune += 1;
                self.actor_mut(actor_side).music.music_cards_played += 1;
                Some(false)
            }
            207 => {
                self.apply_configured_physique(actor_side, card);
                self.actor_mut(actor_side).core.physique_limit += other_param(card, 0).max(0);
                self.gain_attack_bonus(actor_side, other_param(card, 1).max(0));
                Some(false)
            }
            208 => {
                self.modify_momentum_limit(actor_side, other_param(card, 2).max(0));
                self.actor_mut(actor_side).ronghui.momentum_formation +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            209 => {
                self.modify_actor_hp(actor_side, other_param(card, 1).max(0), false, false);
                self.actor_mut(actor_side).ronghui.snow_lotus_mirror += other_param(card, 0).max(0);
                Some(false)
            }
            210 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).ronghui.reverse_gu_attack += other_param(card, 0).max(0);
                Some(attacked)
            }
            389 => {
                self.apply_configured_anima(actor_side, card);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.add_actor_negative_status(target_side, 101, other_param(card, 0).max(0));
                    self.add_actor_negative_status(target_side, 102, other_param(card, 1).max(0));
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
                let hp = other_param(card, 0).max(0);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                let selected_id = self.actor(actor_side).ronghui.five_elements_bottle_card_id;
                // Card_390.cs 读 talentDatas[199].commonParams[0]；玉瓶首格为空
                // （cardId=0）时原版仍对 cardId 0 执行 ExecuteEffect ——
                // cardId 0 即普通攻击（3攻），攻击照样结算。oracle 锚点：
                // mirror-32299000 43876aabed9dde9d/round-11 cp6 p1.hp 67 vs 70
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
            _ => None,
        }
    }

    pub(super) fn apply_ronghui_five_emperors_upgrade_transform(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).ronghui.five_emperors_upgrade > 0
            && can_upgrade_original_battle_deck_card(drawn.card.id)
        {
            if let Some(upgraded) = original_card_definition(drawn.card.id + 10_000) {
                self.actor_mut(actor_side).ronghui.five_emperors_upgrade -= 1;
                self.replace_ronghui_drawn_card(actor_side, &mut drawn, upgraded);
            } else {
                self.missing_decision("card:178:upgrade definition");
            }
        }
        drawn
    }

    pub(super) fn apply_ronghui_alchemy_pot_transform(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).ronghui.alchemy_pot > 0 {
            self.actor_mut(actor_side).ronghui.alchemy_pot -= 1;
            // CardActionBase reads the config field, never an id-derived
            // upgrade rank. A missing rarity is CardConfig's default zero.
            if drawn.card.rarity.unwrap_or(0) >= 1 && drawn.card.id != 19 {
                if let Some(lowered) = original_card_definition(drawn.card.id - 10_000) {
                    self.replace_ronghui_drawn_card(actor_side, &mut drawn, lowered);
                } else {
                    self.missing_decision("card:181:downgrade definition");
                }
            } else {
                let drain = original_card_definition(181)
                    .map(|card| other_param(&card, 1).max(0))
                    .unwrap_or(0);
                let target_side = opponent_side(actor_side);
                self.modify_actor_hp(actor_side, -drain, false, false);
                self.modify_actor_max_hp(actor_side, -drain);
                self.modify_actor_max_hp(target_side, drain);
                self.modify_actor_hp(target_side, drain, false, false);
            }
        }
        drawn
    }

    pub(super) fn apply_ronghui_free_and_easy_tune_transform(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: super::DrawnCard,
    ) -> super::DrawnCard {
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
                    self.replace_ronghui_drawn_card(actor_side, &mut drawn, replacement);
                } else {
                    self.missing_decision("card:206:replacement definition");
                }
            } else {
                self.missing_decision("card:206:previous active grid");
            }
        }
        drawn
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
            .map(|card| other_param(&card, 1).max(0))
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
                self.gain_anima(actor_side, other_param(card, 1).max(0));
                self.gain_water_momentum(actor_side, other_param(card, 2).max(0));
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
                    self.actor_mut(actor_side).deck.slots[grid].card = replacement;
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
            slot_state.card = replacement.clone();
        }
        drawn.card = replacement;
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
        let Some(selected) = original_card_definition(selected_id) else {
            self.missing_decision("ronghui temporary card definition");
            return;
        };
        if self.apply_temporary_card_effect_with_spec(actor_side, &selected, spec) {
            self.modify_extra_actions(actor_side, 1);
        }
    }
}
