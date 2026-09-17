use super::support::normalized_base_id;
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
    fn turn_end_hook_pair(&self) -> super::ReplayTurnEndHookPair {
        super::ReplayTurnEndHookPair {
            p1: self.turn_end_hook_snapshot(PlayerSide::P1),
            p2: self.turn_end_hook_snapshot(PlayerSide::P2),
        }
    }

    fn turn_end_hook_snapshot(&self, side: PlayerSide) -> super::ReplayTurnEndHookSnapshot {
        let actor = self.actor(side);
        super::ReplayTurnEndHookSnapshot {
            hp: actor.core.hp,
            max_hp: actor.core.max_hp,
            defense: actor.core.defense,
            anima: actor.core.anima,
            guard: actor.core.guard,
            physique: actor.core.physique,
            momentum: actor.beng.momentum,
            water_momentum: actor.elements.water_momentum,
            attack_bonus: actor.core.attack_bonus,
            internal_injury: actor.status.internal_injury,
            weakness: actor.status.weakness,
            flaw: actor.status.flaw,
            attack_reduction: actor.status.attack_reduction,
            entangle: actor.status.entangle,
            external_injury: actor.status.external_injury,
            lose_hp_count: actor.turn.lose_hp_count,
            lose_hp_times_count: actor.turn.lose_hp_times_count,
        }
    }

    pub(super) fn observe_turn_end_hook<R>(
        &mut self,
        actor_side: PlayerSide,
        hook: &'static str,
        apply: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if !self.observation.mode.is_detailed() {
            return apply(self);
        }
        let before = self.turn_end_hook_pair();
        let result = apply(self);
        let after = self.turn_end_hook_pair();
        self.observation
            .turn_end_hooks
            .push(super::ReplayTurnEndHookReceipt {
                turn: self.actor_turn,
                actor: actor_side,
                hook,
                before,
                after,
            });
        result
    }

    pub(super) fn apply_you_ming_xu_hun_quan_replacement(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).chance.you_ming_xu_hun_quan <= 0 {
            return drawn;
        }
        // CardActionBase reads CardConfig.rarity here. Upgraded-looking ids
        // with an explicit/default rarity of zero still become base attack.
        let rarity = drawn.card.rarity.unwrap_or(0);
        let replacement = super::original_config::original_card_definition(rarity * 10_000)
            .unwrap_or_else(super::support::basic_attack_card);
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = replacement.clone();
        }
        self.actor_mut(actor_side).chance.you_ming_xu_hun_quan -= 1;
        drawn.card = replacement;
        drawn
    }

    pub(super) fn spirit_formation_echo_card(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
    ) -> CardDefinition {
        if !self
            .actor(actor_side)
            .formations
            .spirit_formation_echo_triggered
            || !self.original_build_has_capability(
                super::original_build_profile::OriginalBuildCapability::SpiritFormationEchoUsesBaseCard,
            )
            || !card.name.contains("灵阵")
        {
            return card.clone();
        }
        super::original_config::original_card_definition(normalized_base_id(card))
            .unwrap_or_else(|| card.clone())
    }
}
