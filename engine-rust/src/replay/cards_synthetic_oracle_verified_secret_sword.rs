use super::support::{
    is_cloud_sword, is_frenzy_sword_for_actor, is_sword_formation_card, other_param, other_param_or,
};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    /// Verified card bodies backed by the synthetic, client-executable original-game oracle.
    pub(super) fn apply_synthetic_oracle_verified_secret_sword_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        base_id: i64,
    ) -> Option<bool> {
        match base_id {
            1_000_050 => {
                // 剑意流转：牌面剑意属于牌体，且每次牌体执行均叠一层保留标记。
                self.modify_sword_intent(actor_side, card.sword_intent.unwrap_or(0).max(0));
                self.actor_mut(actor_side).sword.sword_intent_circulation += 1;
                Some(false)
            }
            1_000_051 => {
                self.actor_mut(actor_side)
                    .sword
                    .dark_void_sword_formation_art += other_param(card, 0).max(0);
                Some(false)
            }
            1_000_053 => {
                self.actor_mut(actor_side).sword.spirit_sword_mindset +=
                    other_param(card, 0).max(0);
                Some(false)
            }
            1_000_056 => {
                // 一心一剑的普通剑意仍由攻击管线加入；这里只补足至牌面倍数。
                let extra_sword_intent = self.actor(actor_side).sword.sword_intent.max(0)
                    * (other_param_or(card, 0, 1) - 1).max(0);
                let previous_ignore_weakness = self.actor(actor_side).turn.ignore_weakness_attacks;
                self.actor_mut(actor_side).turn.ignore_weakness_attacks =
                    previous_ignore_weakness + 1;
                let attacked = self.attack_by_config(actor_side, card, extra_sword_intent, slot);
                self.actor_mut(actor_side).turn.ignore_weakness_attacks = previous_ignore_weakness;
                Some(attacked)
            }
            1_000_057 => {
                self.actor_mut(actor_side).sword.cloud_step += other_param(card, 0).max(0);
                Some(false)
            }
            1_000_061 => {
                let attacked = self.attack_by_config(actor_side, card, 0, slot);
                self.actor_mut(actor_side).sword.frenzy_sword_double_effect += 1;
                Some(attacked)
            }
            _ => None,
        }
    }

    /// OnAfterExecuted hook; temporary card replays intentionally do not trigger it.
    pub(super) fn apply_secret_sword_formation_follow_up(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        is_temporary_card: bool,
    ) -> bool {
        let attack = self
            .actor(actor_side)
            .sword
            .dark_void_sword_formation_art
            .max(0);
        if is_temporary_card
            || attack <= 0
            || !is_sword_formation_card(self.actor(actor_side), card)
        {
            return false;
        }
        // 原版 OnAfterExecuted 的溟空剑阵诀追击（CardActionBase.cs:3807-3812，
        // MingKongJianZhenJue）发生在 729（KaPaiBuChuFaJianYi，仅迅影飞剑
        // Card_1000094.cs:63 设置、CardActionBase.cs:4734 才移除）仍生效期间：
        // 追击攻击同样不触发剑意，且不记账 XiaoHaoJianYi，因此迅影飞剑当回合
        // 先机加的剑意不会被追击消耗（oracle 锚点：mirror-32299000
        // 12cff58989e12ff5/round-12 cp9 p1.hp 86 vs 85、cp10 43 vs 44）。
        let suppress_sword_intent = self.active_effect_base_id() == 1_000_094;
        self.apply_attack_with_options(
            actor_side,
            attack,
            slot,
            suppress_sword_intent,
            false,
            0,
            Some("buff:darkVoidSwordFormationArt"),
        );
        true
    }

    /// Owner turn-start hook, after normal turn-start resource reset.
    pub(super) fn apply_secret_sword_mindset_at_turn_start(
        &mut self,
        actor_side: PlayerSide,
    ) -> bool {
        let gain = self.actor(actor_side).sword.spirit_sword_mindset.max(0);
        if gain <= 0 {
            return false;
        }
        self.modify_sword_intent(actor_side, gain);
        true
    }

    /// Card-completed hook, immediately after the common cloud-chain update.
    pub(super) fn apply_secret_sword_cloud_step_after_cloud_sword(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> bool {
        let gain = self.actor(actor_side).sword.cloud_step.max(0);
        if gain <= 0 || !is_cloud_sword(self.actor(actor_side), card) {
            return false;
        }
        self.gain_attack_bonus(actor_side, gain);
        true
    }

    /// Card-completed hook, immediately before normal Sword Intent consumption.
    pub(super) fn preserve_secret_sword_intent_with_circulation(
        &mut self,
        actor_side: PlayerSide,
    ) -> bool {
        if self.active_effect_pending_sword_intent() <= 0
            || self.actor(actor_side).sword.sword_intent_circulation <= 0
        {
            return false;
        }
        let actor = self.actor_mut(actor_side);
        actor.sword.sword_intent_circulation -= 1;
        self.set_active_effect_pending_sword_intent(0);
        true
    }

    /// Pre-effect hook: consume one stored layer and request one extra body execution.
    pub(super) fn consume_secret_sword_double_dragon_repetition(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> i64 {
        if self.actor(actor_side).sword.frenzy_sword_double_effect <= 0
            || !is_frenzy_sword_for_actor(self.actor(actor_side), card)
        {
            return 0;
        }
        self.actor_mut(actor_side).sword.frenzy_sword_double_effect -= 1;
        1
    }
}
