#[path = "cards_misc.rs"]
mod cards_misc;
#[path = "flow_card_effect_primary_early.rs"]
mod flow_card_effect_primary_early;
#[path = "flow_card_effect_primary_late.rs"]
mod flow_card_effect_primary_late;

use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CardHandlerFamily {
    Hexagram,
    PrimaryEarly,
    QiXingStarSlot,
    Misc,
    Chance,
    PrimaryLate,
    OracleVerified,
    DreamMiragePilot,
    FullScopeCandidate,
    DreamFate,
    RonghuiEarly,
    DreamMirageDirect,
    DreamDirect,
    DreamMirage,
    MirageRonghui,
    Ronghui,
    SecretSword,
    SecretMisc,
    SecretExtremeRemaining,
    Missing,
}

const CARD_HANDLER_REGISTRY: &[CardHandlerFamily] = &[
    CardHandlerFamily::Hexagram,
    CardHandlerFamily::PrimaryEarly,
    CardHandlerFamily::QiXingStarSlot,
    CardHandlerFamily::Misc,
    CardHandlerFamily::Chance,
    CardHandlerFamily::PrimaryLate,
    CardHandlerFamily::OracleVerified,
    CardHandlerFamily::DreamMiragePilot,
    CardHandlerFamily::FullScopeCandidate,
    CardHandlerFamily::DreamFate,
    CardHandlerFamily::RonghuiEarly,
    CardHandlerFamily::DreamMirageDirect,
    CardHandlerFamily::DreamDirect,
    CardHandlerFamily::DreamMirage,
    CardHandlerFamily::MirageRonghui,
    CardHandlerFamily::Ronghui,
    CardHandlerFamily::SecretSword,
    CardHandlerFamily::SecretMisc,
    CardHandlerFamily::SecretExtremeRemaining,
    CardHandlerFamily::Missing,
];

struct CardEffectContext<'state, 'card> {
    state: &'state mut ReplayState,
    actor_side: PlayerSide,
    card: &'card CardDefinition,
    slot: usize,
    was_used_before_effect: bool,
    base_id: i64,
}

impl CardEffectContext<'_, '_> {
    fn dispatch(&mut self, family: CardHandlerFamily) -> Option<bool> {
        match family {
            CardHandlerFamily::Hexagram => self.state.apply_hexagram_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::PrimaryEarly => self.state.apply_card_effect_primary_early(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::QiXingStarSlot => self.state.apply_qi_xing_star_slot_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::Misc => self.state.apply_card_effect_misc(
                self.actor_side,
                self.card,
                self.slot,
                self.base_id,
            ),
            CardHandlerFamily::Chance => self.state.apply_chance_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::PrimaryLate => self.state.apply_card_effect_primary_late(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::OracleVerified => {
                self.state.apply_synthetic_oracle_verified_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                )
            }
            CardHandlerFamily::DreamMiragePilot => self
                .state
                .apply_synthetic_oracle_dream_mirage_pilot_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                ),
            CardHandlerFamily::FullScopeCandidate => {
                self.state.apply_synthetic_full_scope_candidate_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                )
            }
            CardHandlerFamily::DreamFate => self.state.apply_dream_fate_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::RonghuiEarly => self.state.apply_ronghui_early_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::DreamMirageDirect => {
                self.state.apply_dream_mirage_direct_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.was_used_before_effect,
                    self.base_id,
                )
            }
            CardHandlerFamily::DreamDirect => self.state.apply_dream_direct_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::DreamMirage => {
                self.state.apply_synthetic_oracle_dream_mirage_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.was_used_before_effect,
                    self.base_id,
                )
            }
            CardHandlerFamily::MirageRonghui => self
                .state
                .apply_synthetic_oracle_mirage_ronghui_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.was_used_before_effect,
                    self.base_id,
                ),
            CardHandlerFamily::Ronghui => self.state.apply_synthetic_oracle_ronghui_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::SecretSword => self
                .state
                .apply_synthetic_oracle_verified_secret_sword_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                ),
            CardHandlerFamily::SecretMisc => self
                .state
                .apply_synthetic_oracle_verified_secret_misc_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                ),
            CardHandlerFamily::SecretExtremeRemaining => self
                .state
                .apply_synthetic_oracle_verified_secret_extreme_remaining_card_effect(
                    self.actor_side,
                    self.card,
                    self.slot,
                    self.base_id,
                ),
            CardHandlerFamily::Missing => self.state.apply_card_effect_missing(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
        }
    }
}

impl ReplayState {
    pub(super) fn apply_card_effect_primary(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        let mut context = CardEffectContext {
            state: self,
            actor_side,
            card,
            slot,
            was_used_before_effect,
            base_id,
        };
        for family in CARD_HANDLER_REGISTRY {
            if let Some(attacked) = context.dispatch(*family) {
                return Some(attacked);
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn card_handler_registry_is_ordered_and_unique() {
        assert_eq!(
            CARD_HANDLER_REGISTRY.first(),
            Some(&CardHandlerFamily::Hexagram)
        );
        assert_eq!(
            CARD_HANDLER_REGISTRY.last(),
            Some(&CardHandlerFamily::Missing)
        );
        assert_eq!(CARD_HANDLER_REGISTRY.len(), 20);
        assert_eq!(
            CARD_HANDLER_REGISTRY
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            CARD_HANDLER_REGISTRY.len()
        );
    }
}
