use super::support::{has_ke_yin_type, opponent_side, other_param, wu_xing_count_in_deck};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

/// BattleCharacter.OnTurnEnded formations block, in current-build source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum TurnEndFormationPhase {
    Thunder,
    Turtle,
    RejuvenationTune,
    FlowerMaze,
    Immovable,
    MirageVitalityBloom,
}

pub(super) const TURN_END_FORMATION_PHASES: [TurnEndFormationPhase; 6] = [
    TurnEndFormationPhase::Thunder,
    TurnEndFormationPhase::Turtle,
    TurnEndFormationPhase::RejuvenationTune,
    TurnEndFormationPhase::FlowerMaze,
    TurnEndFormationPhase::Immovable,
    TurnEndFormationPhase::MirageVitalityBloom,
];

/// Base ids handled by `apply_formation_card_effect` (kept in sync with
/// the match arms below). `card_routing` uses this to pin the sect-routing
/// invariant: the formation kernel is reachable from the shared chain and
/// the WuXing chain (7_000_058 / 7_000_073), never from the other sects.
#[cfg(test)]
pub(super) const FORMATION_HANDLED_IDS: &[i64] = &[
    7_000_058, 7_000_073, 8_000_001, 8_000_002, 8_000_003, 8_000_004, 8_000_005, 8_000_006,
    8_000_007, 8_000_008, 8_000_009, 8_000_010, 8_000_011, 8_000_013, 8_000_014, 8_000_016,
    11_000_003, 11_000_004, 11_000_022,
];

impl ReplayState {
    pub(super) fn apply_formation_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            8_000_001 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.thunder_formation += other_param(card, 0).max(0);
                actor.formations.thunder_formation_damage = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_002 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.shatter_formation += other_param(card, 0).max(0);
                actor.formations.shatter_formation_bonus = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_003 => {
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    let attack = card.attack.unwrap_or(0).max(0);
                    let bonus = other_param(card, 0).max(0);
                    if attack > 0 {
                        self.apply_attack(actor_side, attack, slot);
                    }
                    if bonus > 0 {
                        self.apply_attack(actor_side, bonus, slot);
                    }
                    Some(attack > 0 || bonus > 0)
                } else {
                    Some(self.attack_by_config(actor_side, card, 0, slot))
                }
            }
            8_000_004 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.turtle_formation += other_param(card, 0).max(0);
                actor.formations.turtle_formation_defense = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_005 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.evil_gu_formation += other_param(card, 0).max(0);
                actor.formations.evil_gu_formation_value = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_006 => {
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    let max_hp_gain = other_param(card, 1).max(0);
                    if max_hp_gain > 0 {
                        self.modify_actor_max_hp(actor_side, max_hp_gain);
                    }
                }
                let hp_gain = other_param(card, 0).max(0);
                if hp_gain > 0 {
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
                Some(false)
            }
            8_000_009 => {
                self.actor_mut(actor_side).fate.exorcism += other_param(card, 0).max(0);
                if self.actor(actor_side).formations.array_echo_persistent_card > 0 {
                    self.gain_defense(actor_side, other_param(card, 1).max(0));
                }
                Some(false)
            }
            8_000_007 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.spirit_gathering_formation += other_param(card, 0).max(0);
                actor.formations.spirit_gathering_formation_value = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_008 => {
                let before = self
                    .actor(actor_side)
                    .formations
                    .heaven_cycle_sword_formation;
                self.actor_mut(actor_side)
                    .formations
                    .heaven_cycle_sword_formation += other_param(card, 0).max(0);
                self.actor_mut(actor_side)
                    .formations
                    .heaven_cycle_sword_formation_damage = other_param(card, 1).max(0);
                let after = self
                    .actor(actor_side)
                    .formations
                    .heaven_cycle_sword_formation;
                self.record_counter_transition(
                    actor_side,
                    "阵法",
                    "heavenCycleSwordFormation",
                    "周天剑阵",
                    before,
                    after,
                );
                Some(false)
            }
            8_000_010 => {
                let damage = other_param(card, 2).max(0);
                if damage > 0 {
                    self.apply_damage(actor_side, damage, false, false, false);
                }
                let actor = self.actor_mut(actor_side);
                actor.formations.eight_gates_formation += other_param(card, 0).max(0);
                actor.formations.eight_gates_formation_damage = other_param(card, 1).max(0);
                Some(false)
            }
            8_000_011 => {
                self.actor_mut(actor_side).formations.heaven_force_formation +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            8_000_013 => {
                self.actor_mut(actor_side).formations.flower_maze_formation +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            8_000_014 => {
                self.modify_star_chess_break(actor_side, other_param(card, 0).max(0));
                Some(false)
            }
            8_000_016 => {
                let actor = self.actor_mut(actor_side);
                actor.formations.immovable_formation += other_param(card, 0).max(0);
                actor.formations.immovable_formation_value = other_param(card, 1).max(0);
                Some(false)
            }
            11_000_022 => {
                self.actor_mut(actor_side).formations.body_observation +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            11_000_003 | 11_000_004 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.apply_configured_defense(actor_side, card);
                Some(attacked)
            }
            7_000_058 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                // Card_7000058.cs:115 充能 = GetWuXingCountInDeck + otherParams[0]
                // （含 417/talent199/292/刻印76/147 的全口径）。
                self.actor_mut(actor_side)
                    .elements
                    .primordial_infinity_formation +=
                    wu_xing_count_in_deck(self.actor(actor_side)) + other_param(card, 0).max(0);
                Some(false)
            }
            7_000_073 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                // Card_7000073.cs（五行天髓诀）: WuXingTianSuiJue 的授予条件
                // 是 GetWuXingCountInDeck()（BattleCharacter.cs:12118 全口径，
                // 含 292 / 7000101 / 刻印 76 / fate 147 火灵修正 / fate 417 +
                // talent199），不是简单的元素种类数 == 1：
                //   KeYinType(104) 且 count <= 2 → otherParams[0] * 2
                //   count == 1 或（fate 408 且 count == 2）→ otherParams[0]
                // oracle 锚点：f5d6ca391895cdaa/round-13 cp[1] p2 272=2
                // （土+金=2 且 fate 408 → 授予）；b3135e34464fb802/round-15
                // cp[2] p1 无 272（全火 deck + fate 147 → count=2，无 408
                // → 不授予，引擎原先误授导致火灵阵后错误再次行动）。
                let count = wu_xing_count_in_deck(self.actor(actor_side));
                let grant = if has_ke_yin_type(self.actor(actor_side), 104) && count <= 2 {
                    other_param(card, 0).max(0) * 2
                } else if count == 1
                    || (self
                        .actor(actor_side)
                        .identity
                        .fate_strategies
                        .contains(&408)
                        && count == 2)
                {
                    other_param(card, 0).max(0)
                } else {
                    0
                };
                if grant > 0 {
                    self.modify_five_elements_marrow_art(actor_side, grant);
                }
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn apply_after_attack_formation_hooks(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
    ) {
        if self.active_effect_attacks() <= 0
            && self.actor(actor_side).turn.dan_ka_gong_ji_ji_shu <= 0
        {
            return;
        }
        if self
            .actor(actor_side)
            .formations
            .heaven_cycle_sword_formation
            > 0
        {
            let damage = self
                .actor(actor_side)
                .formations
                .heaven_cycle_sword_formation_damage;
            if damage > 0 {
                // 周天剑阵 OnAfterExecuted 追加攻击（CardActionBase.cs:
                // 4473-4513）在 KaPaiBuChuFaJianYi（迅影飞剑
                // Card_1000094.cs:63 设置、CardActionBase.cs:4734 才移除）
                // 仍生效期间执行——追击同样不触发剑意，与溟空剑阵诀/察体
                // 追击同构。oracle 锚点：hf-32308000
                // 2b3d798972192efc/round-16 cp4（迅影飞剑剑意 2→3 后
                // 周天剑阵追击不消耗 → 3，原引擎 0）、
                // 7f2ce1909636f3af/round-13 cp19（剑意 0→1 保留；流云乱剑
                // 无 729 时追击正常消耗 12 → 0）。
                let suppress_sword_intent = self.active_effect_base_id() == 1_000_094;
                self.apply_attack_with_options(
                    actor_side,
                    damage,
                    slot,
                    suppress_sword_intent,
                    false,
                    0,
                    None,
                );
            }
            let before = self
                .actor(actor_side)
                .formations
                .heaven_cycle_sword_formation;
            self.actor_mut(actor_side)
                .formations
                .heaven_cycle_sword_formation -= 1;
            let after = self
                .actor(actor_side)
                .formations
                .heaven_cycle_sword_formation;
            self.record_counter_transition(
                actor_side,
                "阵法",
                "heavenCycleSwordFormation",
                "周天剑阵",
                before,
                after,
            );
        }
        let body_observation = self.actor(actor_side).formations.body_observation;
        if body_observation > 0 {
            // 察体（卡 11020022，buff ChaTi）OnAfterExecuted 追加攻击
            // （CardActionBase.cs:4516-4520）。该追击在 KaPaiBuChuFaJianYi
            // （迅影飞剑 Card_1000094.cs:63 设置、CardActionBase.cs:4734 才
            // 移除）仍生效期间执行——追击同样不触发剑意，与溟空剑阵诀追击
            // 同构（oracle 锚点：hf-latest-32308000-16f9c778
            // d2a4070fa6f2934f/round-17 cp13：原版 11 攻无剑意 → 破防后
            // 11-7=4 伤、剑意 9→10；引擎 11+10 剑意=21 → 14 伤、剑意被清 0，
            // 连锁导致澄心剑胚 51 伤 vs 原版 101 伤、胜负翻转）。
            let suppress_sword_intent = self.active_effect_base_id() == 1_000_094;
            self.apply_attack_with_options(
                actor_side,
                body_observation,
                slot,
                suppress_sword_intent,
                false,
                0,
                Some("buff:bodyObservation"),
            );
            self.actor_mut(actor_side).formations.body_observation = 0;
        }
    }

    pub(super) fn apply_turn_start_buff_decrements(&mut self, actor_side: PlayerSide) {
        let actor = self.actor_mut(actor_side);
        // BattleCharacter.cs:3799-3801. Buff 598 is duration state: HP-loss
        // requests leave it intact and its owner-turn-start phase decrements it.
        if actor.hp_mutation.no_hp_loss_before_next_turn > 0 {
            actor.hp_mutation.no_hp_loss_before_next_turn -= 1;
        }
        if actor.turn.attack_applies_internal_injury_turns > 0 {
            actor.turn.attack_applies_internal_injury_turns -= 1;
        }
        if actor.turn.wood_spring_turns > 0 {
            actor.turn.wood_spring_turns -= 1;
        }
        if actor.turn.adaptation > 0 {
            actor.turn.adaptation -= 1;
        }
        if actor.fate.heavenly_secret_reverse > 0 {
            // BattleCharacter.cs:3874-3877：NiShi 与 ShunYing/TieGu 同块逐回合 -1。
            actor.fate.heavenly_secret_reverse -= 1;
        }
        if actor.elements.metal_iron_bone > 0 {
            actor.elements.metal_iron_bone -= 1;
        }
        if actor.elements.water_stealth > 0 {
            actor.elements.water_stealth -= 1;
        }
        if actor.sword.water_month_sword_formation > 0 {
            actor.sword.water_month_sword_formation -= 1;
        }
        if actor.astrology.all_goes_well > 0 {
            actor.astrology.all_goes_well -= 1;
        }
    }

    pub(super) fn apply_turn_start_wood_spirit_all_growth(&mut self, actor_side: PlayerSide) {
        // BattleCharacter.OnTurnStarted (BattleCharacter.cs:4482-4484):
        // JiaGong += GetBuffValue(WanWuShengZhang) — the STACKED buff value
        // (each 万物生 adds otherParams[1] to the buff), not a flat one-shot.
        // oracle 锚点: mirror-32219000 04bcf167c7fe26d7/round-13 checkpoint[3]
        // 木灵•暗香 p1.attackBonus 3 (t1 五行流转链 万物生×2 → 万物生长 2 →
        // turn-3 开始 +2，引擎按 op[1] 只 +1); round-16 checkpoint[4]
        // 千里神行符 attackBonus 2 (引擎 1)。
        if self.actor(actor_side).elements.wood_spirit_all_growth > 0 {
            let attack_gain = self
                .actor(actor_side)
                .elements
                .wood_spirit_all_growth
                .max(0)
                .saturating_mul(
                    self.actor(actor_side)
                        .elements
                        .wood_spirit_all_growth_attack
                        .max(0),
                );
            if attack_gain > 0 {
                self.gain_attack_bonus(actor_side, attack_gain);
            }
        }
    }

    pub(super) fn apply_turn_start_tune_effects(&mut self, actor_side: PlayerSide) {
        let illusory_tune = self.actor(actor_side).music.illusory_tune.max(0);
        if illusory_tune > 0 {
            self.modify_actor_hp(actor_side, -illusory_tune, false, false);
            self.gain_defense(actor_side, illusory_tune);
        }
        let heartbreak_tune = self.actor(actor_side).music.heartbreak_tune.max(0);
        if heartbreak_tune > 0 {
            self.add_actor_negative_status(actor_side, 100, heartbreak_tune);
        }
    }

    pub(super) fn apply_turn_start_post_injury_formations(&mut self, actor_side: PlayerSide) {
        if self.actor(actor_side).formations.evil_gu_formation > 0 {
            let amount = self.actor(actor_side).formations.evil_gu_formation_value;
            if amount > 0 {
                self.add_actor_negative_status(opponent_side(actor_side), 100, amount);
            }
            self.actor_mut(actor_side).formations.evil_gu_formation -= 1;
        }
        if self.actor(actor_side).formations.spirit_gathering_formation > 0 {
            let amount = self
                .actor(actor_side)
                .formations
                .spirit_gathering_formation_value;
            if amount > 0 {
                self.gain_anima(actor_side, amount);
            }
            self.actor_mut(actor_side)
                .formations
                .spirit_gathering_formation -= 1;
        }
        if self.actor(actor_side).formations.heaven_force_formation > 0 {
            self.gain_attack_bonus(actor_side, 1);
            self.actor_mut(actor_side).formations.heaven_force_formation -= 1;
        }
    }

    pub(super) fn trigger_turn_end_formations(&mut self, actor_side: PlayerSide) {
        for phase in TURN_END_FORMATION_PHASES {
            match phase {
                TurnEndFormationPhase::Thunder => {
                    if self.actor(actor_side).formations.thunder_formation > 0 {
                        let damage = self.actor(actor_side).formations.thunder_formation_damage;
                        if damage > 0 {
                            self.apply_damage(actor_side, damage, false, false, false);
                        }
                        self.actor_mut(actor_side).formations.thunder_formation -= 1;
                    }
                }
                TurnEndFormationPhase::Turtle => {
                    let turtle_defense = self.actor(actor_side).formations.turtle_formation_defense;
                    if self.actor(actor_side).formations.turtle_formation > 0 {
                        self.gain_defense(actor_side, turtle_defense);
                        self.actor_mut(actor_side).formations.turtle_formation -= 1;
                    }
                }
                TurnEndFormationPhase::RejuvenationTune => {
                    // 原版 oracle 在妙手回春+迷魂阵+回春曲的近满血边界中，
                    // 回春曲先把上限撑开，再让迷魂阵的大段治疗结算；
                    // 否则会有 3 点治疗被旧上限截断。
                    // oracle 锚点：c0f73571469dd090/round-14 turn35 p1 263/275
                    // -> 285/285（引擎原先 282/285）。
                    let rejuvenation_tune = self.actor(actor_side).music.rejuvenation_tune.max(0);
                    if rejuvenation_tune > 0 {
                        self.modify_actor_max_hp(actor_side, rejuvenation_tune);
                        self.modify_actor_hp(actor_side, rejuvenation_tune, false, false);
                    }
                }
                TurnEndFormationPhase::FlowerMaze => {
                    if self.actor(actor_side).formations.flower_maze_formation > 0 {
                        let target_side = opponent_side(actor_side);
                        self.add_actor_negative_status(target_side, 103, 1);
                        let drain = self.actor(target_side).status.attack_reduction;
                        self.actor_mut(actor_side).formations.flower_maze_drain = drain;
                        if drain > 0 {
                            self.modify_actor_hp(target_side, -drain, false, false);
                            self.modify_actor_hp(actor_side, drain, false, false);
                        }
                        self.actor_mut(actor_side).formations.flower_maze_formation -= 1;
                    }
                }
                TurnEndFormationPhase::Immovable => {
                    if self.actor(actor_side).formations.immovable_formation > 0 {
                        let value = self.actor(actor_side).formations.immovable_formation_value;
                        if value > 0 {
                            self.gain_defense(actor_side, value);
                            if self.actor(actor_side).turn.action_again_count == 0 {
                                self.modify_actor_max_hp(actor_side, value);
                                self.modify_actor_hp(actor_side, value, false, false);
                            }
                        }
                        self.actor_mut(actor_side).formations.immovable_formation -= 1;
                    }
                }
                TurnEndFormationPhase::MirageVitalityBloom => {
                    if self.actor(actor_side).fate.mirage_vitality_bloom > 0 {
                        let value = self
                            .actor(actor_side)
                            .fate
                            .mirage_vitality_bloom_heal
                            .max(0);
                        if value > 0 {
                            let actor = self.actor(actor_side);
                            let overheal = (actor.core.hp + value - actor.core.max_hp).max(0);
                            if overheal > 0 {
                                self.modify_actor_max_hp(actor_side, overheal);
                            }
                            self.modify_actor_hp(actor_side, value, false, false);
                            self.modify_actor_hp(actor_side, -value, false, false);
                        }
                    }
                }
            }
        }
    }

    pub(super) fn apply_eight_gates_action_again_damage(&mut self, actor_side: PlayerSide) {
        let opponent = opponent_side(actor_side);
        if self.actor(opponent).formations.eight_gates_formation <= 0 {
            return;
        }
        let damage = self.actor(opponent).formations.eight_gates_formation_damage;
        if damage > 0 {
            // Opponent's eight-gates formation damages the actor seeking action-again.
            self.apply_damage(opponent, damage, false, false, false);
        }
        self.actor_mut(opponent).formations.eight_gates_formation -= 1;
    }
}
