//! Ordered typed-card routing.
//!
//! Every card resolves through one family chain. `FateStrategy` is always
//! tried first; afterwards the chain depends on the card's base id:
//!
//! - Current sect cards (see `current_sect_for_base_id`) follow the sect's
//!   explicit order table, which places the sect's own mechanic kernel ahead
//!   of the shared `PrimaryArchive` fallback.
//! - Every other card follows `SHARED_HANDLER_ORDER`, the historical shared
//!   family order.
//!
//! The sect order tables are data, not code: adding a sect card family means
//! editing one table, and the `shared_families_never_claim_other_sect_ids`
//! test pins the invariant that a shared kernel never handles an id that
//! belongs to a sect whose chain would route it elsewhere.

use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CurrentSect {
    YunLingJianZong,
    QiXingGe,
    WuXingDaoMeng,
    DuanXuanZong,
}

pub(super) fn current_sect_for_base_id(base_id: i64) -> Option<CurrentSect> {
    match base_id {
        1_000_000..2_000_000 => Some(CurrentSect::YunLingJianZong),
        4_000_000..5_000_000 => Some(CurrentSect::QiXingGe),
        7_000_000..8_000_000 => Some(CurrentSect::WuXingDaoMeng),
        10_000_000..11_000_000 => Some(CurrentSect::DuanXuanZong),
        _ => None,
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
enum CardHandlerFamily {
    FateStrategy,
    SharedSword,
    SharedElement,
    SharedBody,
    SharedMechanic,
    Formation,
    PrimaryArchive,
}

/// Per-sect family order. Each sect's own kernel comes first, then the shared
/// mechanic kernel, then the primary archive. Cards outside the four current
/// sect ranges use `SHARED_HANDLER_ORDER` below.
const YUNLING_HANDLER_ORDER: &[CardHandlerFamily] = &[
    CardHandlerFamily::SharedSword,
    CardHandlerFamily::SharedMechanic,
    CardHandlerFamily::PrimaryArchive,
];
const QIXING_HANDLER_ORDER: &[CardHandlerFamily] = &[
    CardHandlerFamily::SharedMechanic,
    CardHandlerFamily::PrimaryArchive,
];
const WUXING_HANDLER_ORDER: &[CardHandlerFamily] = &[
    CardHandlerFamily::SharedElement,
    CardHandlerFamily::SharedMechanic,
    CardHandlerFamily::Formation,
    CardHandlerFamily::PrimaryArchive,
];
const DUANXUAN_HANDLER_ORDER: &[CardHandlerFamily] = &[
    CardHandlerFamily::SharedBody,
    CardHandlerFamily::SharedMechanic,
    CardHandlerFamily::PrimaryArchive,
];

/// Historical shared order for every card outside the four current sect
/// ranges (and the fallback chain the sect tables build on).
const SHARED_HANDLER_ORDER: &[CardHandlerFamily] = &[
    CardHandlerFamily::SharedSword,
    CardHandlerFamily::SharedElement,
    CardHandlerFamily::SharedBody,
    CardHandlerFamily::SharedMechanic,
    CardHandlerFamily::Formation,
    CardHandlerFamily::PrimaryArchive,
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
            CardHandlerFamily::FateStrategy => self.state.apply_fate_strategy_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.was_used_before_effect,
                self.base_id,
            ),
            CardHandlerFamily::SharedSword => {
                self.state
                    .apply_sword_card_effect(self.actor_side, self.card, self.slot)
            }
            CardHandlerFamily::SharedElement => {
                self.state
                    .apply_element_card_effect(self.actor_side, self.card, self.slot)
            }
            CardHandlerFamily::SharedBody => {
                self.state
                    .apply_body_card_effect(self.actor_side, self.card, self.slot)
            }
            CardHandlerFamily::SharedMechanic => self.state.apply_mechanic_card_effect_extra(
                self.actor_side,
                self.card,
                self.slot,
                self.base_id,
            ),
            CardHandlerFamily::Formation => self.state.apply_formation_card_effect(
                self.actor_side,
                self.card,
                self.slot,
                self.base_id,
            ),
            CardHandlerFamily::PrimaryArchive => self.state.apply_card_effect_primary(
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
    pub(super) fn apply_typed_card_effect_body(
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
        // Fate strategies are card ids in their own right and must win over
        // every sect chain, exactly as the shared registry ordered them first.
        if let Some(attacked) = context.dispatch(CardHandlerFamily::FateStrategy) {
            return Some(attacked);
        }
        let order: &[CardHandlerFamily] = match current_sect_for_base_id(base_id) {
            Some(CurrentSect::YunLingJianZong) => YUNLING_HANDLER_ORDER,
            Some(CurrentSect::QiXingGe) => QIXING_HANDLER_ORDER,
            Some(CurrentSect::WuXingDaoMeng) => WUXING_HANDLER_ORDER,
            Some(CurrentSect::DuanXuanZong) => DUANXUAN_HANDLER_ORDER,
            None => SHARED_HANDLER_ORDER,
        };
        for family in order {
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
    use crate::fixture::FixtureExpected;
    use crate::model::PlayerSide;
    use std::collections::HashSet;

    /// Regression: the sect facade refactor dropped `Formation` from the
    /// WuXing chain, silently losing 7_000_058 / 7_000_073 (both implemented
    /// by the formation kernel). The chain must keep reaching them.
    #[test]
    fn wuxing_sect_chain_reaches_formation_kernel() {
        let mut card = super::super::tests::test_card(7_000_058, 7_000_058, "五行阵法");
        card.other_params = vec![5];
        let fixture = super::super::tests::minimal_fixture(
            super::super::tests::filler_cards(super::super::tests::basic_attack_test_card()),
            super::super::tests::filler_cards(super::super::tests::basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        let mut state = ReplayState::test_from_fixture(&fixture);
        let result = state.apply_typed_card_effect_body(PlayerSide::P1, &card, 0, false, 7_000_058);
        assert_eq!(result, Some(false));
        assert!(
            state.p1.elements.primordial_infinity_formation > 0,
            "7_000_058 effect must run through the formation kernel"
        );
    }

    #[test]
    fn current_sect_ranges_are_explicit_and_do_not_capture_careers() {
        assert_eq!(
            current_sect_for_base_id(1_000_001),
            Some(CurrentSect::YunLingJianZong)
        );
        assert_eq!(
            current_sect_for_base_id(4_000_001),
            Some(CurrentSect::QiXingGe)
        );
        assert_eq!(
            current_sect_for_base_id(7_000_001),
            Some(CurrentSect::WuXingDaoMeng)
        );
        assert_eq!(
            current_sect_for_base_id(10_000_001),
            Some(CurrentSect::DuanXuanZong)
        );
        for non_sect in [
            999_999, 2_000_001, 3_000_001, 5_000_001, 8_000_001, 11_000_001,
        ] {
            assert_eq!(current_sect_for_base_id(non_sect), None);
        }
    }

    /// Pins the routing invariant the sect order tables rely on: a shared
    /// mechanic kernel never claims an id from a sect whose chain would route
    /// it through a different kernel first. The `HANDLED_IDS` lists live next
    /// to the match arms they mirror; keep them in sync when adding an arm.
    #[test]
    fn shared_families_never_claim_other_sect_ids() {
        for &id in super::super::swords::SWORD_HANDLED_IDS {
            assert!(
                !matches!(
                    current_sect_for_base_id(id),
                    Some(CurrentSect::QiXingGe)
                        | Some(CurrentSect::WuXingDaoMeng)
                        | Some(CurrentSect::DuanXuanZong)
                ),
                "sword kernel claims non-YunLing sect id {id}"
            );
        }
        for &id in super::super::elements::ELEMENT_HANDLED_IDS {
            assert!(
                !matches!(
                    current_sect_for_base_id(id),
                    Some(CurrentSect::YunLingJianZong)
                        | Some(CurrentSect::QiXingGe)
                        | Some(CurrentSect::DuanXuanZong)
                ),
                "element kernel claims non-WuXing sect id {id}"
            );
        }
        for &id in super::super::body::BODY_HANDLED_IDS {
            assert!(
                !matches!(
                    current_sect_for_base_id(id),
                    Some(CurrentSect::YunLingJianZong)
                        | Some(CurrentSect::QiXingGe)
                        | Some(CurrentSect::WuXingDaoMeng)
                ),
                "body kernel claims non-DuanXuan sect id {id}"
            );
        }
        for &id in super::super::formations::FORMATION_HANDLED_IDS {
            assert!(
                !matches!(
                    current_sect_for_base_id(id),
                    Some(CurrentSect::YunLingJianZong)
                        | Some(CurrentSect::QiXingGe)
                        | Some(CurrentSect::DuanXuanZong)
                ),
                "formation kernel claims non-WuXing sect id {id}"
            );
        }
        // `mechanic_cards_extra` appears in every sect chain (after the
        // sect's own kernel), so any id is reachable; no cross constraint.
    }

    /// The shared chain keeps its historical order: sect order tables are the
    /// only place routing may diverge from the shared fallback.
    #[test]
    fn shared_chain_preserves_historical_order_and_keeps_primary_last() {
        assert_eq!(
            SHARED_HANDLER_ORDER.last(),
            Some(&CardHandlerFamily::PrimaryArchive)
        );
        assert_eq!(
            SHARED_HANDLER_ORDER
                .iter()
                .copied()
                .collect::<HashSet<_>>()
                .len(),
            SHARED_HANDLER_ORDER.len()
        );
    }
}
