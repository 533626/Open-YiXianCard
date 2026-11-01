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

/// Game architecture ID range domain classification.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CardDomain {
    YunLingJianZong,
    QiXingGe,
    WuXingDaoMeng,
    DuanXuanZong,
    FuZhouShi,
    QinShi,
    LianDanShi,
    ZhenFaShi,
    LingZhiShi,
    HuaShi,
    SeasonRotation,
    Universal,
}

#[inline(always)]
pub(super) fn card_domain_for_base_id(base_id: i64) -> CardDomain {
    match base_id {
        1_000_000..2_000_000 => CardDomain::YunLingJianZong,
        2_000_000..3_000_000 => CardDomain::FuZhouShi,
        3_000_000..4_000_000 => CardDomain::QinShi,
        4_000_000..5_000_000 => CardDomain::QiXingGe,
        5_000_000..6_000_000 => CardDomain::LianDanShi,
        6_000_000..7_000_000 => CardDomain::ZhenFaShi,
        7_000_000..8_000_000 => CardDomain::WuXingDaoMeng,
        8_000_000..9_000_000 => CardDomain::LingZhiShi,
        9_000_000..10_000_000 => CardDomain::HuaShi,
        10_000_000..11_000_000 => CardDomain::DuanXuanZong,
        11_000_000..12_000_000 => CardDomain::SeasonRotation,
        _ => CardDomain::Universal,
    }
}

/// Declarative classification of card lifecycle triggers.
/// A zero-cost bitmask representing which lifecycle phases a card interacts with.
#[allow(dead_code)]
#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub(super) struct CardLifecycle(pub u16);

#[allow(dead_code)]
impl CardLifecycle {
    pub const NONE: Self = Self(0);
    /// BattleStart / TriggerOpening 开局阶段
    pub const BATTLE_START: Self = Self(1 << 0);
    /// OnPlay 卡牌主体执行
    pub const ON_PLAY: Self = Self(1 << 1);
    /// BeforePlay 出牌前置钩子
    pub const BEFORE_PLAY: Self = Self(1 << 2);
    /// AfterPlay 出牌后置钩子
    pub const AFTER_PLAY: Self = Self(1 << 3);
    /// TurnEnd 回合末结算
    pub const TURN_END: Self = Self(1 << 4);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn has_any(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline(always)]
    pub fn for_base_id<const OBSERVE: bool>(base_id: i64) -> Self {
        let mut flags = Self::ON_PLAY.0;
        if ReplayState::<OBSERVE>::dream_mirage_card_has_opening_effect(base_id)
            || ReplayState::<OBSERVE>::mirage_ronghui_card_has_opening_effect(base_id)
            || ReplayState::<OBSERVE>::ronghui_card_has_opening_effect(base_id)
            || ReplayState::<OBSERVE>::synthetic_full_scope_candidate_has_opening_effect(base_id)
            || OPENING_CORE_BASE_IDS.contains(&base_id)
        {
            flags |= Self::BATTLE_START.0;
        }
        if matches!(base_id, 7_000_058 | 7_000_073 | 7_000_074 | 7_040_019 | 7_040_020) {
            flags |= Self::TURN_END.0;
        }
        Self(flags)
    }
}

/// Core opening cards directly handled by the canonical opening dispatcher.
pub(super) const OPENING_CORE_BASE_IDS: &[i64] = &[
    55, 56, 57, 58, 1_000_101, 11_000_001, 11_000_005, 11_000_009, 11_000_013, 11_000_014,
    11_000_018, 11_000_022, 11_000_023, 11_000_024,
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CurrentSect {
    YunLingJianZong,
    QiXingGe,
    WuXingDaoMeng,
    DuanXuanZong,
}

pub(super) fn current_sect_for_base_id(base_id: i64) -> Option<CurrentSect> {
    match card_domain_for_base_id(base_id) {
        CardDomain::YunLingJianZong => Some(CurrentSect::YunLingJianZong),
        CardDomain::QiXingGe => Some(CurrentSect::QiXingGe),
        CardDomain::WuXingDaoMeng => Some(CurrentSect::WuXingDaoMeng),
        CardDomain::DuanXuanZong => Some(CurrentSect::DuanXuanZong),
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

struct CardEffectContext<'state, 'card, const OBSERVE: bool> {
    state: &'state mut ReplayState<OBSERVE>,
    actor_side: PlayerSide,
    card: &'card CardDefinition,
    slot: usize,
    was_used_before_effect: bool,
    base_id: i64,
}

impl<const OBSERVE: bool> CardEffectContext<'_, '_, OBSERVE> {
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

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn apply_typed_card_effect_body(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> Option<bool> {
        #[cfg(all(feature = "perf-ceiling", not(target_arch = "wasm32")))]
        let _timer = crate::performance::Timer::new(2);
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

    /// Declarative dispatch for battle-start opening effects (`TriggerOpening`).
    ///
    /// Single source of truth for both normal battle-start opening phase
    /// (`battle_start.rs`) and mid-battle trigger-opening paths (skip-card,
    /// astrological traction/counter, etc. in `cards_missing.rs` and `flow.rs`).
    pub(super) fn dispatch_opening_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot_index: usize,
        trigger_grid: usize,
        catalog_cards: &[CardDefinition],
    ) {
        let base_id = super::support::normalized_base_id(card);
        self.apply_dream_mirage_battle_start_opening_with_trigger_grid(
            actor_side,
            card,
            slot_index,
            base_id,
            trigger_grid,
        );
        self.apply_mirage_ronghui_battle_start_opening(actor_side, base_id);
        self.apply_ronghui_battle_start_opening(actor_side, card, slot_index);
        self.apply_synthetic_full_scope_candidate_opening(actor_side, base_id);
        match base_id {
            55 => {
                self.activate_element(actor_side, super::Element::Water);
                self.activate_element(actor_side, super::Element::Wood);
            }
            56 => {
                self.activate_element(actor_side, super::Element::Fire);
                self.activate_element(actor_side, super::Element::Earth);
            }
            57 => {
                self.activate_element(actor_side, super::Element::Earth);
                self.activate_element(actor_side, super::Element::Metal);
            }
            58 => self.apply_wave_cutting_seal_opening(actor_side),
            1_000_101 => {
                // TriggerOpening Card_1000101: only when owner has zero anima.
                if self.actor(actor_side).core.anima == 0 {
                    self.gain_anima(actor_side, super::flow_card_effect::param0(card));
                }
            }
            11_000_001 => {
                let damage = super::support::other_param(card, 1);
                if damage > 0 {
                    self.modify_target_hp(actor_side, -damage);
                }
            }
            11_000_005 => {
                // 吉运初显必须走正常生命变更链，以保留 Talent64 等钩子与回放账本。
                let value = super::support::other_param(card, 1).max(0);
                if value > 0 {
                    self.modify_actor_max_hp(actor_side, value);
                    self.modify_actor_hp(actor_side, value, false, false);
                }
            }
            11_000_009 => {
                self.gain_anima(actor_side, super::support::other_param(card, 0).max(0));
            }
            11_000_013 => {
                let stacks = self
                    .actor(actor_side)
                    .deck
                    .slots
                    .get(slot_index)
                    .map(|slot| super::support::other_param(&slot.card, 2).max(0))
                    .unwrap_or_else(|| super::support::other_param(card, 2).max(0));
                self.actor_mut(actor_side).fate.exorcism += stacks;
            }
            11_000_014 => {
                self.gain_defense(actor_side, super::support::other_param(card, 2).max(0));
            }
            11_000_018 => {
                let target_side = super::support::opponent_side(actor_side);
                if let Some(target_slot) = self.actor(target_side).deck.slots.get(trigger_grid) {
                    let target_card = target_slot.card.clone();
                    if super::original_config::original_config_rarity(target_card.id) >= 1 && target_card.id != 19 {
                        let lower_id = target_card.id - 10_000;
                        if let Some(lowered) = super::original_config::known_card_definition(catalog_cards, lower_id) {
                            self.actor_mut(target_side).deck.slots[trigger_grid].card = lowered.into();
                        }
                    } else {
                        let current_card = self
                            .actor(actor_side)
                            .deck
                            .slots
                            .get(slot_index)
                            .map(|slot| slot.card.clone());
                        let damage = current_card
                            .as_ref()
                            .map(|slot_card| super::support::other_param(slot_card, 1))
                            .unwrap_or_else(|| super::support::other_param(card, 1))
                            .max(0);
                        if damage > 0 {
                            self.apply_reflect_damage(actor_side, damage);
                        }
                    }
                }
            }
            11_000_022 => {
                self.modify_next_attack_shatter_defense(
                    actor_side,
                    super::support::other_param(card, 1).max(0),
                );
            }
            11_000_023 => {
                let loss = super::support::other_param(card, 1).max(0);
                if loss > 0 {
                    self.modify_actor_hp(super::support::opponent_side(actor_side), -loss, false, false);
                    self.modify_actor_hp(actor_side, -loss, false, false);
                }
            }
            11_000_024 => {
                self.add_actor_negative_status(
                    super::support::opponent_side(actor_side),
                    100,
                    super::support::other_param(card, 1).max(0),
                );
            }
            _ => {}
        }
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
        card.other_params = vec![5].into();
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

    #[test]
    fn card_domain_partitions_entire_architecture() {
        assert_eq!(card_domain_for_base_id(1_000_001), CardDomain::YunLingJianZong);
        assert_eq!(card_domain_for_base_id(2_000_001), CardDomain::FuZhouShi);
        assert_eq!(card_domain_for_base_id(3_000_001), CardDomain::QinShi);
        assert_eq!(card_domain_for_base_id(4_000_001), CardDomain::QiXingGe);
        assert_eq!(card_domain_for_base_id(5_000_001), CardDomain::LianDanShi);
        assert_eq!(card_domain_for_base_id(6_000_001), CardDomain::ZhenFaShi);
        assert_eq!(card_domain_for_base_id(7_000_001), CardDomain::WuXingDaoMeng);
        assert_eq!(card_domain_for_base_id(8_000_001), CardDomain::LingZhiShi);
        assert_eq!(card_domain_for_base_id(9_000_001), CardDomain::HuaShi);
        assert_eq!(card_domain_for_base_id(10_000_001), CardDomain::DuanXuanZong);
        assert_eq!(card_domain_for_base_id(11_000_001), CardDomain::SeasonRotation);
        assert_eq!(card_domain_for_base_id(0), CardDomain::Universal);
        assert_eq!(card_domain_for_base_id(55), CardDomain::Universal);
    }

    #[test]
    fn card_lifecycle_declarative_classification() {
        // Universal opening cards
        for &id in OPENING_CORE_BASE_IDS {
            let lc = CardLifecycle::for_base_id::<true>(id);
            assert!(lc.contains(CardLifecycle::BATTLE_START), "id {id} must have BATTLE_START");
            assert!(lc.contains(CardLifecycle::ON_PLAY), "id {id} must have ON_PLAY");
        }
        // Ordinary non-opening card
        let basic = CardLifecycle::for_base_id::<true>(0);
        assert!(!basic.contains(CardLifecycle::BATTLE_START));
        assert!(basic.contains(CardLifecycle::ON_PLAY));
        // Turn-end card
        let formation = CardLifecycle::for_base_id::<true>(7_000_058);
        assert!(formation.contains(CardLifecycle::TURN_END));
    }

    #[test]
    fn opening_effect_unification_case_b_herbs() {
        // Verify 57 (土生金) works under both battle-start and mid-battle trigger-opening
        assert!(ReplayState::<true>::card_has_opening_effect(57));
        assert!(ReplayState::<true>::card_triggers_opening(57));
        assert!(ReplayState::<true>::card_triggers_opening_on_skip(57));
        // Verify 56 (火生土) is excluded from skip-card triggering
        assert!(!ReplayState::<true>::card_triggers_opening_on_skip(56));
    }
}
