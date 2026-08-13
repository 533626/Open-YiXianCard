use super::original_config::original_card_definition;
use super::support::{card_rarity, normalized_base_id, opponent_side, other_param};
use super::{DrawnCard, Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

const FREE_AND_EASY_REPLICA: i64 = 322;
const DING_FENG_BO_UNSEALED: i64 = 372;

impl ReplayState {
    /// Source-backed current-build candidates exercised by the full-scope
    /// synthetic oracle. This dispatch deliberately remains separate from the
    /// verified-card modules until the original-client campaign is admitted.
    pub(super) fn apply_synthetic_full_scope_candidate_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        let target_side = opponent_side(actor_side);
        let attacked = match base_id {
            14 => {
                if self.actor(actor_side).elements.activated_elements.len() == 5 {
                    self.gain_attack_bonus(actor_side, other_param(card, 0));
                    self.gain_guard(actor_side, other_param(card, 1).max(0));
                }
                false
            }
            43 => {
                let bonus = if self.actor(actor_side).core.temp_life
                    < self.actor(target_side).core.temp_life
                {
                    other_param(card, 0)
                } else {
                    0
                };
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                if self.actor(actor_side).core.hp < self.actor(target_side).core.hp {
                    self.modify_actor_hp(actor_side, other_param(card, 1), false, false);
                }
                if self.actor(actor_side).identity.last_round_exp
                    < self.actor(target_side).identity.last_round_exp
                {
                    self.gain_agility(actor_side, other_param(card, 2));
                }
                attacked
            }
            44 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.gain_agility(actor_side, other_param(card, 0));
                attacked
            }
            258 => {
                self.apply_configured_anima(actor_side, card);
                let hp = other_param(card, 0);
                self.modify_actor_max_hp(actor_side, hp);
                self.modify_actor_hp(actor_side, hp, false, false);
                false
            }
            280 => {
                self.modify_star_chess_break(target_side, other_param(card, 0).max(0));
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                false
            }
            281 => {
                self.gain_guard(actor_side, other_param(card, 0).max(0));
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                false
            }
            282 => {
                self.add_actor_negative_status(target_side, 100, other_param(card, 0));
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                false
            }
            283 => {
                self.add_actor_negative_status(target_side, 101, other_param(card, 0));
                self.add_actor_negative_status(target_side, 102, other_param(card, 1));
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                false
            }
            284 | 285 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                attacked
            }
            FREE_AND_EASY_REPLICA => false,
            329 => {
                // Card_329.SetHp bypasses Defense and Guard.
                self.actor_mut(target_side).core.hp = 1;
                self.exhaust_synthetic_full_scope_card(actor_side, slot);
                false
            }
            DING_FENG_BO_UNSEALED => {
                let count = card.attack_count.unwrap_or(0).max(0)
                    + self.synthetic_full_scope_element_activation_count(actor_side);
                let attack = card.attack.unwrap_or(0).max(0);
                for _ in 0..count {
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                }
                count > 0 && attack > 0
            }
            373 => {
                let events = self.actor(actor_side).dream_mirage.hp_gain_event_count;
                let bonus = events * other_param(card, 0).max(0);
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // 原版 Card_373.cs:81-83 读持有者持久 ActualDamage(302)（残留 + 本卡），
                // 引擎以 turn.actual_damage_carry 表达（CardActionBase.cs:4743-4745
                // 出牌完成时清零）。
                if self.check_wu_xing(actor_side, Element::Wood) {
                    let divisor = other_param(card, 1).max(1);
                    let healing = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    self.modify_actor_hp(actor_side, healing, false, false);
                }
                attacked
            }
            374 => {
                let events = self.actor(target_side).dream_mirage.lost_max_hp_event_count;
                let bonus = events * other_param(card, 0).max(0);
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // 原版 Card_374.cs:81-83 读持有者持久 ActualDamage(302)。
                if self.check_wu_xing(actor_side, Element::Fire) {
                    let divisor = other_param(card, 1).max(1);
                    let amount = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    if amount > 0 {
                        self.modify_actor_max_hp(target_side, -amount);
                        self.modify_actor_hp(target_side, -amount, false, false);
                    }
                }
                attacked
            }
            375 => {
                let events = self.actor(actor_side).dream_mirage.defense_gain_event_count;
                let bonus = events * other_param(card, 0).max(0);
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // 原版 Card_375.cs:81-83 读持有者持久 ActualDamage(302)。
                if self.check_wu_xing(actor_side, Element::Earth) {
                    let divisor = other_param(card, 1).max(1);
                    let defense = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    self.gain_defense(actor_side, defense);
                }
                attacked
            }
            376 => {
                let events = self
                    .actor(actor_side)
                    .dream_mirage
                    .sharpness_gain_event_count;
                let bonus = events * other_param(card, 0).max(0);
                let attacked = self.attack_by_config(actor_side, card, bonus, slot);
                // 原版 Card_376.cs:81-83 读持有者持久 ActualDamage(302)。
                if self.check_wu_xing(actor_side, Element::Metal) {
                    let divisor = other_param(card, 1).max(1);
                    let sharpness = self.actor(actor_side).turn.actual_damage_carry / divisor;
                    self.gain_sharpness(actor_side, sharpness);
                }
                attacked
            }
            7_000_008 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                if self.check_wu_xing(actor_side, Element::Water) {
                    self.modify_actor_hp(actor_side, other_param(card, 0), false, false);
                }
                attacked
            }
            7_000_041 => {
                self.apply_configured_defense(actor_side, card);
                let damage = self.actor(actor_side).core.defense + other_param(card, 0);
                if damage > 0 {
                    self.apply_damage_to(actor_side, target_side, damage, false, false, false);
                    self.apply_damage_to(actor_side, actor_side, damage, false, false, false);
                }
                false
            }
            7_000_054 => {
                self.apply_configured_defense(actor_side, card);
                if self.actor(actor_side).elements.activated_elements.len() >= 2 {
                    let hp = other_param(card, 0);
                    self.modify_actor_max_hp(actor_side, hp);
                    self.modify_actor_hp(actor_side, hp, false, false);
                }
                false
            }
            9_000_026 => {
                if let Some(status) = self.consume_optional_negative_status_decision() {
                    self.remove_actor_negative_status(actor_side, status, other_param(card, 0));
                }
                false
            }
            _ => return None,
        };
        Some(attacked)
    }

    pub(super) fn synthetic_full_scope_candidate_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            258 => Some(
                !self
                    .actor(actor_side)
                    .elements
                    .activated_elements
                    .is_empty(),
            ),
            9_000_026 => {
                Some(card_rarity(card) >= 2 && self.negative_status_stack_count(actor_side) == 0)
            }
            _ => None,
        }
    }

    pub(super) fn synthetic_full_scope_candidate_has_opening_effect(base_id: i64) -> bool {
        base_id == DING_FENG_BO_UNSEALED
    }

    pub(super) fn apply_synthetic_full_scope_candidate_opening(
        &mut self,
        actor_side: PlayerSide,
        base_id: i64,
    ) {
        if base_id != DING_FENG_BO_UNSEALED {
            return;
        }
        self.actor_mut(actor_side)
            .elements
            .synthetic_ding_feng_bo_candidate = 1;
        for element in [
            Element::Wood,
            Element::Fire,
            Element::Earth,
            Element::Metal,
            Element::Water,
        ] {
            self.activate_element(actor_side, element);
        }
    }

    pub(super) fn apply_synthetic_full_scope_replica_transform(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: DrawnCard,
    ) -> DrawnCard {
        if normalized_base_id(&drawn.card) != FREE_AND_EASY_REPLICA {
            return drawn;
        }
        let target_side = opponent_side(actor_side);
        let Some(target_card) = self
            .actor(target_side)
            .deck
            .slots
            .get(drawn.source_slot)
            .map(|slot| slot.card.clone())
        else {
            return drawn;
        };
        let copied = if normalized_base_id(&target_card) == 0 {
            original_card_definition(286 + target_card.id).unwrap_or(target_card)
        } else {
            target_card
        };
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = copied.clone();
        }
        drawn.card = copied;
        drawn
    }

    pub(super) fn apply_synthetic_ding_feng_bo_activation_damage(
        &mut self,
        actor_side: PlayerSide,
    ) {
        if self
            .actor(actor_side)
            .elements
            .synthetic_ding_feng_bo_candidate
            <= 0
        {
            return;
        }
        let damage = self
            .actor(actor_side)
            .deck
            .slots
            .iter()
            .filter(|slot| normalized_base_id(&slot.card) == DING_FENG_BO_UNSEALED)
            .map(|slot| other_param(&slot.card, 0))
            .sum::<i64>();
        if damage > 0 {
            self.apply_damage(actor_side, damage, false, false, false);
        }
    }

    fn exhaust_synthetic_full_scope_card(&mut self, actor_side: PlayerSide, slot: usize) {
        if let Some(slot) = self.actor_mut(actor_side).deck.slots.get_mut(slot) {
            slot.skipped = true;
        }
    }

    fn synthetic_full_scope_element_activation_count(&self, actor_side: PlayerSide) -> i64 {
        // Card_372 shares BattleCharacter.GetWuXingActiveNumber with Card_371/295.
        self.wu_xing_active_number(actor_side)
    }
}
