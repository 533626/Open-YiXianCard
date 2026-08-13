use super::effect_invocation::TemporaryInvocationSpec;
use super::support::{other_param, other_param_or};
use super::{Element, ReplayState};
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    pub(super) fn apply_synthetic_oracle_verified_secret_extreme_remaining_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            10_000_065 => {
                for index in 0..other_param(card, 0).max(0) {
                    let Some(selected_id) = self.consume_endless_collapse_decision(index) else {
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
                self.modify_actor_max_hp(actor_side, other_param(card, 1).max(0));
                self.actor_mut(actor_side).fate.quiet_mindset += other_param(card, 0).max(0);
                Some(false)
            }
            7_000_105 => {
                self.set_active_effect_wood_spirit_patrol_before_card(
                    self.check_wu_xing(actor_side, Element::Wood),
                );
                self.apply_configured_anima(actor_side, card);
                self.actor_mut(actor_side).elements.wood_thorn += other_param(card, 0).max(0);
                Some(self.attack_by_config(actor_side, card, 0, slot))
            }
            10_000_098 => {
                self.apply_configured_anima(actor_side, card);
                self.gain_agility(actor_side, other_param(card, 0).max(0));
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

    fn consume_endless_collapse_decision(&mut self, index: i64) -> Option<i64> {
        if self.decision_tape.is_empty() {
            self.missing_decision(&format!("card:10000065:temporaryBengQuan:{}", index + 1));
            return None;
        }
        Some(self.decision_tape.remove(0))
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
