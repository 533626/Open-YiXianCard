use super::decisions::{
    seeded_decision_choice_index, ReplayDecisionDomain, ReplayDecisionEvent, ReplayDecisionKind,
    ReplayDecisionProvider,
};
use super::effect_invocation::TemporaryInvocationSpec;
use super::flow_card_effect::{param0, param1};
use super::support::other_param_or;
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn apply_synthetic_oracle_verified_secret_extreme_remaining_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            10_000_065 => {
                for index in 0..param0(card) {
                    let Some(selected_id) =
                        self.consume_endless_collapse_decision(actor_side, index)
                    else {
                        break;
                    };
                    self.execute_endless_collapse_temporary_card(actor_side, slot, selected_id);
                }
                Some(false)
            }
            10_000_066 => {
                self.apply_configured_physique(actor_side, card);
                let bonus = self.actor(actor_side).core.physique / other_param_or(card, 0, 1);
                self.actor_mut(actor_side).beng.return_to_simplicity += bonus.max(0);
                Some(false)
            }
            4_000_098 => {
                self.modify_actor_max_hp(actor_side, param1(card));
                self.actor_mut(actor_side).fate.quiet_mindset += param0(card);
                Some(false)
            }
            7_000_105 => {
                self.set_active_effect_wood_spirit_patrol_before_card(
                    self.check_wu_xing(actor_side, Element::Wood),
                );
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).elements.wood_thorn += param0(card);
                Some(self.attack_by_config(actor_side, card, 0, slot))
            }
            10_000_098 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, param0(card));
                self.actor_mut(actor_side).beng.beng_tian_step += 1;
                Some(false)
            }
            _ => None,
        }
    }

    pub(super) fn apply_return_to_simplicity_basic_attack(
        &mut self,
        actor_side: PlayerSide,
    ) -> i64 {
        let bonus = self.actor(actor_side).beng.return_to_simplicity.max(0);
        if bonus > 0 {
            self.gain_agility(actor_side, bonus);
        }
        bonus
    }

    pub(super) fn apply_wood_thorn_before_attack_segment(&mut self, actor_side: PlayerSide) -> i64 {
        // 原版每段攻击前置钩子（BattleCharacter.cs:11761-11770，Attack 内、
        // ApplyDamage 之前）：
        //   1) MuCi(645) 木刺：持久层数伤害 + 等量回血（:11761-11764）；
        //   2) GongJiXiQuShengMing(668) 攻击吸取生命：每段攻击消耗 1 层，
        //      目标 -1 血、自身 +1 血（:11766-11769）。
        // 两个效果互相独立，各自按存在与否触发（668 在木刺为 0 时仍生效）。
        let mut total = 0;
        let thorn = self.actor(actor_side).elements.wood_thorn.max(0);
        if thorn > 0 {
            self.modify_target_hp(actor_side, -thorn);
            self.modify_actor_hp(actor_side, thorn, false, false);
            total += thorn;
        }
        let drain = self.actor(actor_side).elements.attack_life_drain.max(0);
        if drain > 0 {
            self.actor_mut(actor_side).elements.attack_life_drain -= 1;
            self.modify_target_hp(actor_side, -1);
            self.modify_actor_hp(actor_side, 1, false, false);
            total += 1;
        }
        total
    }

    /// Card 10000065 无尽崩绝 synthetic fallback pool: base-rank, realm-level-1
    /// (LianQi), non-hidden 崩拳 cards. CardConfig.json carries exactly three
    /// such rows — 10000001/2/3 (崩拳•戳/封/弹); the level-1 梦• rows
    /// (10000071/74/76/82/89) are hidden DreamCard rows and the +10000/+20000
    /// ranks are upgraded variants, all excluded. Every candidate is
    /// re-verified against original_config at runtime (trait + realm level +
    /// base rank + visibility), so config drift shrinks the pool instead of
    /// guessing.
    fn endless_collapse_level_one_beng_quan_pool() -> Vec<i64> {
        const CANDIDATES: [i64; 3] = [10_000_001, 10_000_002, 10_000_003];
        CANDIDATES
            .into_iter()
            .filter(|id| {
                *id != 10_000_065
                    && super::support::normalize_base_id(*id) == *id
                    && super::original_config::original_card_realm_level(*id) == Some(1)
                    && !super::original_config::original_card_is_hidden(*id).unwrap_or(true)
                    && super::original_config::original_card_definition(*id)
                        .is_some_and(|def| super::support::is_beng_quan_card(&def))
            })
            .collect()
    }

    fn consume_endless_collapse_decision(
        &mut self,
        actor_side: PlayerSide,
        index: i64,
    ) -> Option<i64> {
        if !self.decision_tape.is_empty() {
            return Some(self.decision_tape.remove(0));
        }
        let Some(seed) = self.synthetic_decision_fallback_seed else {
            self.missing_decision(&format!("card:10000065:temporaryBengQuan:{}", index + 1));
            return None;
        };
        let pool = Self::endless_collapse_level_one_beng_quan_pool();
        if pool.is_empty() {
            // No eligible Level-1 崩拳: same no-op skip the original takes for
            // a -1 pick (cardItem.InitData(-1) resolves to an empty effect).
            return Some(-1);
        }
        self.decision_occurrence += 1;
        let decision_occurrence = self.decision_occurrence;
        let card_execution_occurrence = self
            .current_card_execution
            .map(|execution| execution.occurrence)
            .unwrap_or(0);
        let pick = seeded_decision_choice_index(
            seed,
            actor_side,
            self.actor_turn,
            10_000_065,
            ReplayDecisionKind::RequiredParam,
            card_execution_occurrence,
            decision_occurrence,
            index,
            pool.len(),
        );
        let selected = pool[pick];
        // Same audit split as consume_required_decision_or_synthetic
        // (note_uncaptured_synthetic_seed is private to decisions.rs, so the
        // uncaptured branch is inlined here like the sibling helper does).
        if self.capture_decision_events {
            self.decision_events.push(ReplayDecisionEvent {
                provider: ReplayDecisionProvider::SeededSynthetic,
                seed: Some(seed),
                side: actor_side,
                actor_turn: self.actor_turn,
                card_id: 10_000_065,
                decision_kind: ReplayDecisionKind::RequiredParam,
                card_execution_occurrence,
                decision_occurrence,
                ordinal: index,
                domain: ReplayDecisionDomain::Discrete {
                    legal_options: pool,
                },
                selected_option: Some(selected),
            });
        } else if !self.uncaptured_synthetic_seeds.contains(&seed) {
            self.uncaptured_synthetic_seeds.push(seed);
        }
        Some(selected)
    }

    fn execute_endless_collapse_temporary_card(
        &mut self,
        actor_side: PlayerSide,
        slot: usize,
        selected_id: i64,
    ) {
        // Card_10000065.OnExecuted runs exactly otherParams[0] picks. When a
        // pick's recorded id is -1 the original loop re-inits the card slot to
        // an empty definition (cardItem.InitData(-1, ...)) whose FindCardAction
        // resolves to a no-op ExecuteEffect: the player simply had fewer than
        // otherParams[0] eligible 崩拳 to draw. Model that miss as skipping the
        // temporary invocation instead of trying to execute the record-only
        // placeholder card:-1.
        if selected_id < 0 {
            return;
        }
        let Some(selected) = super::original_config::original_card_definition(selected_id) else {
            self.missing_decision(&format!(
                "card:10000065:temporaryBengQuanDefinition:{selected_id}"
            ));
            return;
        };
        let mut spec = TemporaryInvocationSpec::physical(slot);
        spec.inherit_parent_beng_quan = true;
        if self.apply_temporary_card_effect_with_spec(actor_side, &selected, spec) {
            self.modify_extra_actions(actor_side, 1);
        }
    }
}
