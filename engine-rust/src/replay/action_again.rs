use super::support::{
    active_neighbor_card, card_rarity, element_from_card, is_five_element_control,
    is_sword_formation_card, neighbor_card, normalized_base_id, opponent_side, other_param,
};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ActionAgainSource {
    Card,
    XiaoYaoGuQin,
    ShadowOwlRabbit,
    ExtraAction,
    FiveElementsMarrow,
    FiveElementsGourd,
    Agility,
}

impl ReplayState {
    pub(super) fn resolve_card_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        _was_slot_used: bool,
        cloud_chain_before_card: bool,
    ) -> bool {
        let actor = self.actor(actor_side);
        if actor.ronghui.all_cards_action_again > 0 {
            return true;
        }
        let base_id = normalized_base_id(card);
        if let Some(result) =
            self.resolve_synthetic_oracle_dream_mirage_action_again(actor_side, card, base_id)
        {
            return result;
        }
        if let Some(result) =
            self.resolve_synthetic_oracle_mirage_ronghui_action_again(actor_side, card, base_id)
        {
            return result;
        }
        if let Some(result) =
            self.synthetic_full_scope_candidate_action_again(actor_side, card, base_id)
        {
            return result;
        }
        match base_id {
            126 => {
                actor
                    .deck
                    .slots
                    .iter()
                    .take(actor.deck.active_slot_count)
                    .filter(|slot| is_sword_formation_card(actor, &slot.card))
                    .count() as i64
                    >= other_param(card, 0)
            }
            2 => actor.sword.frenzy_sword > 0,
            54 => self.active_effect_action_again(),
            29 => actor.deck.queue.iter().any(|&index| {
                index != slot
                    && actor
                        .deck
                        .slots
                        .get(index)
                        .is_some_and(|slot_state| slot_state.card.name.contains('雷'))
            }),
            23 => actor.astrology.star_power >= other_param(card, 1),
            32 => is_five_element_control(
                neighbor_card(actor, slot, -1),
                neighbor_card(actor, slot, 1),
            ),
            57 => true,
            193 => {
                self.negative_status_stack_count(opponent_side(actor_side)) >= other_param(card, 2)
            }
            12 => card_rarity(card) == 0 && self.actor(actor_side).fate.rear_move_succeeded,
            13 => !self
                .actor(actor_side)
                .elements
                .activated_elements
                .is_empty(),
            134 => self.is_element_activated(actor_side, Element::Wood),
            143 => self.check_wu_xing(actor_side, Element::Metal),
            214 => card_rarity(card) == 0 && actor.fate.rear_move_succeeded,
            294 => !actor.astrology.star_slots.contains(&slot),
            4_000_033 => actor.fate.rear_move_succeeded,
            4_000_038 => actor.astrology.star_slots.contains(&slot),
            4_000_054 => actor.fate.rear_move_succeeded,
            1_000_029 => actor.core.anima > other_param(card, 0),
            1_000_039 => cloud_chain_before_card,
            1_000_042 => self.active_effect_wounded_count() > 0,
            1_000_094 => actor.sword.sword_intent > 0,
            7_000_055 => actor.elements.activated_elements.len() >= 2,
            7_000_065 => true,
            4_000_026 => {
                actor.astrology.hexagram >= other_param(card, 0)
                    || self.yi_gua_self_resolution(actor_side)
            }
            4_000_094 => {
                actor.astrology.star_slots.contains(&slot)
                    && actor.astrology.star_power >= other_param(card, 1)
            }
            4_000_095 => actor.core.defense >= other_param(card, 2),
            5_000_014 => {
                actor.music.music_cards_played > 1
                    || self
                        .next_active_slot_card(actor_side, slot)
                        .is_some_and(|card| is_music_card(&card))
            }
            7_000_028 => {
                self.is_element_activated(actor_side, Element::Wood)
                    && actor.add_hp_count() >= other_param(card, 0)
            }
            7_000_105 => {
                self.active_effect_wood_spirit_patrol_before_card()
                    && actor.add_hp_count() >= other_param(card, 0)
            }
            7_000_034 => self.is_element_activated(actor_side, Element::Metal),
            7_000_038 => self.is_element_activated(actor_side, Element::Fire),
            7_000_043 => self.is_element_activated(actor_side, Element::Wood),
            // Card_7000096.cs（土灵•遁地）: cardConfig.actionAgain =
            // CheckWuXing(src, JiHuoTuLing) —— 完整 CheckWuXing 语义（激活 /
            // 上次使用的五行及相生链 / 龙马精神 / 卡组含 7030077|7040077
            // 五行刺时恒真），不是仅看土灵激活。oracle 锚点：
            // 6687c7e1ce03cb49/round-12 cp[2]（卡组含 7040077，土灵未激活
            // 仍再次行动）。
            7_000_096 => self.check_wu_xing(actor_side, Element::Earth),
            8_000_014 => actor.formations.array_echo_persistent_card > 0,
            _ => false,
        }
    }

    pub(super) fn consume_action_again(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        _slot: usize,
        _was_slot_used: bool,
        _cloud_chain_before_card: bool,
        repetition_card_action_again: bool,
    ) -> bool {
        if self.actor(actor_side).turn.action_again_count
            >= self.dream_mirage_action_again_limit(actor_side)
        {
            if self.actor(actor_side).turn.extra_actions > 0 {
                let amount = self.actor(actor_side).turn.extra_actions;
                self.modify_extra_actions(actor_side, -amount);
            }
            return false;
        }

        let had_extra_action = self.actor(actor_side).turn.extra_actions > 0;
        let source =
            self.resolve_action_again_source(actor_side, card, repetition_card_action_again);
        let Some(source) = source else {
            return false;
        };

        let binding_bypass = self.has_binding_bypass(actor_side)
            || self.dream_mirage_action_again_prevention_bypass(actor_side);
        if self.actor(actor_side).music.immortal_binding_tune > 0 && !binding_bypass {
            if self.actor(actor_side).turn.extra_actions > 0 {
                let amount = self.actor(actor_side).turn.extra_actions;
                self.modify_extra_actions(actor_side, -amount);
            }
            return false;
        }

        match source {
            ActionAgainSource::ExtraAction => {}
            ActionAgainSource::FiveElementsMarrow => {
                self.modify_five_elements_marrow_art(actor_side, -1);
            }
            ActionAgainSource::FiveElementsGourd => {
                self.modify_five_elements_gourd(actor_side, -1);
            }
            ActionAgainSource::Agility => {
                let cost = self.agility_action_again_cost(actor_side);
                self.modify_agility(actor_side, -cost);
            }
            ActionAgainSource::Card => {}
            ActionAgainSource::XiaoYaoGuQin => {}
            ActionAgainSource::ShadowOwlRabbit => {}
        }
        // BattleExecuter removes ExActionAgain after any successful action-again,
        // even when an earlier source (for example the card itself) won priority.
        if had_extra_action {
            let amount = self.actor(actor_side).turn.extra_actions;
            self.modify_extra_actions(actor_side, -amount);
        }

        let action_again_before = self.actor(actor_side).turn.action_again_count;
        self.actor_mut(actor_side).turn.action_again_count += 1;
        let action_again_after = self.actor(actor_side).turn.action_again_count;
        self.record_counter_transition(
            actor_side,
            "回合",
            "actionAgainCount",
            "再次行动次数",
            action_again_before,
            action_again_after,
        );

        if self.actor(actor_side).status.entangle > 0 && !binding_bypass {
            // BattleExecuter 消耗困缚走 ModifyBuffValue(KunFu, -1)
            // （BattleCharacter.cs:2051），因此会触发卡 415 疯魔架势被动；
            // 走共享移除 hook 保持同一语义。
            self.remove_actor_negative_status(actor_side, 104, 1);
            let vine = self
                .actor(opponent_side(actor_side))
                .music
                .immortal_binding_vine;
            if vine > 0 {
                self.add_actor_negative_status(actor_side, 105, vine);
            }
            return false;
        }

        if self.dream_mirage_extra_action_locked(actor_side) && !binding_bypass {
            return false;
        }

        // BattleExecuter resolves KunFu/MengKunXian blocking before the
        // successful-action rewards and YingXiaoTu life payment.
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&340)
        {
            self.modify_actor_hp(actor_side, 2, false, false);
        }
        let shadow_owl_rabbit = self.actor(actor_side).chance.ying_xiao_tu.max(0);
        if shadow_owl_rabbit > 0 {
            self.modify_actor_hp(actor_side, -shadow_owl_rabbit, false, false);
        }

        // 原版 BattleExecuter.cs:2089-2094：八门金锁阵的反击伤害在
        // KunFu/MengKunXian 阻挡检查（:2050-2068）与再次行动奖励
        // （FateStrategy340 / YingXiaoTu）之后、ShiXianGuTeng 之前结算；
        // 被困缚挡下的再次行动不会触发八门金锁阵。
        // oracle 锚点：hf-latest-32308000-16f9c778 97000b0bc234817a/round-12
        // （T17 冥月蟾光 身法 10 → 再次行动被荷重前行自缚的困缚挡下：
        // 原版 256 八门层数保持 2、T18 星弈虎 3×4 打在防御 5 上净 7 伤
        // （0+3+4 两段）；引擎先前先结算八门 → 防御 5 被提前打穿、
        // 星弈虎净 12 伤）。
        self.apply_eight_gates_action_again_damage(actor_side);

        let devouring_vine = self
            .actor(opponent_side(actor_side))
            .music
            .devouring_ancient_vine
            .max(0);
        if devouring_vine > 0 {
            let target_side = opponent_side(actor_side);
            self.modify_actor_hp(actor_side, -devouring_vine, false, false);
            self.modify_actor_hp(target_side, devouring_vine, false, false);
        }

        if self.actor(actor_side).has_ling_qi_ben_yong()
            && self.actor(actor_side).beng.gun_stance > 0
        {
            self.gain_anima(actor_side, 1);
            self.modify_actor_hp(actor_side, 2, false, false);
        }

        self.apply_dream_mirage_successful_action_again_hooks(actor_side);

        true
    }

    fn resolve_action_again_source(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        repetition_card_action_again: bool,
    ) -> Option<ActionAgainSource> {
        if card.action_again.unwrap_or(false) || repetition_card_action_again {
            return Some(ActionAgainSource::Card);
        }
        if normalized_base_id(card) == 0 && self.actor(actor_side).music.xiaoyao_guqin > 0 {
            return Some(ActionAgainSource::XiaoYaoGuQin);
        }
        if self.actor(actor_side).chance.ying_xiao_tu > 0 {
            return Some(ActionAgainSource::ShadowOwlRabbit);
        }
        if self.actor(actor_side).turn.extra_actions > 0 {
            return Some(ActionAgainSource::ExtraAction);
        }
        if self.would_five_elements_marrow_action_again(actor_side, card) {
            return Some(ActionAgainSource::FiveElementsMarrow);
        }
        if self.would_five_elements_gourd_action_again(actor_side, card) {
            return Some(ActionAgainSource::FiveElementsGourd);
        }
        if self.actor(actor_side).turn.agility >= self.agility_action_again_cost(actor_side) {
            return Some(ActionAgainSource::Agility);
        }
        None
    }

    fn agility_action_again_cost(&self, actor_side: PlayerSide) -> i64 {
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&348)
        {
            9
        } else {
            10
        }
    }

    fn would_five_elements_gourd_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        if self.actor(actor_side).elements.five_elements_gourd <= 0 {
            return false;
        }
        let Some(element) = element_from_card(card) else {
            return false;
        };
        self.actor(actor_side)
            .elements
            .activated_elements
            .contains(&element)
    }

    fn would_five_elements_marrow_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        self.actor(actor_side).elements.five_elements_marrow_art > 0
            && self.actor(actor_side).turn.turn_attack_segments == 0
            && element_from_card(card).is_some()
    }

    fn next_active_slot_card(&self, actor_side: PlayerSide, slot: usize) -> Option<CardDefinition> {
        active_neighbor_card(self.actor(actor_side), slot, 1).cloned()
    }
}

fn is_music_card(card: &CardDefinition) -> bool {
    card.career_name.as_deref() == Some("QinShi")
}
