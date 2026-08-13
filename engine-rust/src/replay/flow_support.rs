use super::support::normalized_base_id;
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

impl ReplayState {
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
