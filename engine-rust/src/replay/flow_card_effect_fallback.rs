use super::card_effect_catalog::{resolve_card_effect, CardEffectResolution};
use super::support::{opponent_side, other_param};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

struct PrintedFallbackContext<'state, 'card> {
    state: &'state mut ReplayState,
    actor_side: PlayerSide,
    card: &'card CardDefinition,
    slot: usize,
}

impl PrintedFallbackContext<'_, '_> {
    fn execute(&mut self) {
        let base_attack = self.card.attack.unwrap_or(0).max(0);
        let random_attack = self.card.random_attack.unwrap_or(0).max(0);
        let mut attack_count = self.card.attack_count.unwrap_or(1).max(0);
        if attack_count == 0 {
            attack_count = 1;
        }
        if base_attack > 0 || random_attack > 0 {
            for _ in 0..attack_count {
                let attack = if random_attack > base_attack {
                    self.state
                        .consume_random_range(self.actor_side, base_attack, random_attack)
                } else {
                    base_attack
                };
                if attack > 0 {
                    self.state.apply_attack(self.actor_side, attack, self.slot);
                }
            }
        }

        self.state
            .apply_configured_anima(self.actor_side, self.card);

        let defense = self.card.defense.unwrap_or(0).max(0);
        let random_defense = self.card.random_defense.unwrap_or(0).max(0);
        if defense > 0 || random_defense > 0 {
            let defense_gain = if random_defense > 0 {
                self.state
                    .consume_random_range(self.actor_side, defense, random_defense)
            } else {
                defense
            };
            self.state.gain_defense(self.actor_side, defense_gain);
        }
        if let Some(hexagram) = self.card.hexagram.filter(|value| *value > 0) {
            self.state.gain_hexagram(self.actor_side, hexagram);
        }
    }
}

impl ReplayState {
    pub(super) fn apply_card_effect_fallback(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
        was_used_before_effect: bool,
        base_id: i64,
    ) -> CardEffectResolution {
        match base_id {
            4_000_024 => {
                self.attack_by_config(actor_side, card, 0, slot);
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                CardEffectResolution::Executable
            }
            4_000_020 => {
                self.attack_by_config(actor_side, card, 0, slot);
                if self.consume_percent_roll(actor_side) < other_param(card, 0) {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        101,
                        other_param(card, 1).max(0),
                    );
                }
                CardEffectResolution::Executable
            }
            4_000_021 => {
                self.apply_configured_anima(actor_side, card);
                self.apply_configured_defense(actor_side, card);
                self.add_actor_negative_status(
                    opponent_side(actor_side),
                    100,
                    other_param(card, 0).max(0),
                );
                CardEffectResolution::Executable
            }
            4_000_023 => {
                self.attack_by_config(actor_side, card, 0, slot);
                self.add_following_star_slots(actor_side, slot, other_param(card, 0));
                CardEffectResolution::Executable
            }
            4_000_027 => {
                self.actor_mut(actor_side).fate.quiet_mindset += other_param(card, 0).max(0);
                CardEffectResolution::Executable
            }
            4_000_028 => {
                self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.reduce_anima_unchecked(
                        opponent_side(actor_side),
                        other_param(card, 0).max(0),
                    );
                }
                CardEffectResolution::Executable
            }
            4_000_029 => {
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    let attack = card.attack.unwrap_or(0).max(0);
                    self.apply_attack(actor_side, attack, slot);
                    self.apply_attack(actor_side, attack, slot);
                    self.apply_attack(actor_side, other_param(card, 0).max(0), slot);
                } else {
                    self.attack_by_config(actor_side, card, 0, slot);
                }
                CardEffectResolution::Executable
            }
            4_000_032 => {
                self.modify_star_power(actor_side, other_param(card, 0).max(0));
                self.add_following_star_slots(actor_side, slot, other_param(card, 1));
                CardEffectResolution::Executable
            }
            4_000_033 => {
                if self.check_rear_move(actor_side, was_used_before_effect) {
                    self.apply_attack(actor_side, other_param(card, 0), slot);
                }
                CardEffectResolution::Executable
            }
            4_000_038 => {
                self.attack_by_config(actor_side, card, 0, slot);
                CardEffectResolution::Executable
            }
            4_000_039 => {
                self.attack_by_config(actor_side, card, 0, slot);
                if self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.add_actor_negative_status(
                        opponent_side(actor_side),
                        101,
                        other_param(card, 0).max(0),
                    );
                }
                CardEffectResolution::Executable
            }
            4_000_062 => {
                self.apply_configured_defense(actor_side, card);
                let hexagram = self.actor(actor_side).astrology.hexagram.max(0);
                if hexagram > 0 {
                    self.modify_hexagram(actor_side, -hexagram);
                    self.modify_star_power(actor_side, hexagram);
                    self.gain_anima(actor_side, hexagram);
                    self.modify_actor_hp(
                        actor_side,
                        hexagram * other_param(card, 0).max(0),
                        false,
                        false,
                    );
                }
                CardEffectResolution::Executable
            }
            4_000_061 => {
                self.attack_by_config(actor_side, card, 0, slot);
                if self.consume_percent_roll(actor_side) < other_param(card, 0) {
                    self.apply_attack(actor_side, other_param(card, 1), slot);
                }
                CardEffectResolution::Executable
            }
            4_000_068 => {
                let attack = self.consume_random_range(
                    actor_side,
                    card.attack.unwrap_or(0),
                    card.random_attack.unwrap_or(card.attack.unwrap_or(0)),
                );
                let effective_hexagram = self
                    .actor(actor_side)
                    .astrology
                    .hexagram_effective_count
                    .min(other_param(card, 0).max(0));
                if attack > 0 {
                    self.apply_attack(actor_side, attack, slot);
                }
                if effective_hexagram > 0 {
                    self.gain_hexagram(actor_side, effective_hexagram);
                }
                CardEffectResolution::Executable
            }
            _ => match resolve_card_effect(card, base_id) {
                CardEffectResolution::VerifiedPrintedFallback => {
                    self.apply_original_fallback_behavior(actor_side, card, slot);
                    CardEffectResolution::VerifiedPrintedFallback
                }
                CardEffectResolution::RecordOnly => CardEffectResolution::RecordOnly,
                CardEffectResolution::Missing => {
                    self.missing_card_effect(card.id, base_id, "missing executable behavior");
                    CardEffectResolution::Missing
                }
                CardEffectResolution::Executable => {
                    self.missing_card_effect(
                        card.id,
                        base_id,
                        "catalog declares executable but dispatch has no handler",
                    );
                    CardEffectResolution::Missing
                }
            },
        }
    }

    fn apply_original_fallback_behavior(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        PrintedFallbackContext {
            state: self,
            actor_side,
            card,
            slot,
        }
        .execute();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixture::FixtureExpected;
    use crate::model::OriginalEnumValue;
    use crate::replay::card_effect_catalog::{
        catalog_executable_base_ids, EXPLICIT_FALLBACK_BASE_IDS, RECORD_ONLY_CARD_BASE_IDS,
        VERIFIED_PRINTED_FALLBACK_BASE_IDS,
    };
    use crate::replay::effect_invocation::{
        TemporaryDeckIdentityMode, TemporaryHadUsedSource, TemporaryInvocationSpec,
    };
    use crate::replay::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
    use crate::replay::BattleError;

    fn state_with_active_card(card: CardDefinition) -> ReplayState {
        let fixture = minimal_fixture(
            filler_cards(card),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        ReplayState::test_from_fixture(&fixture)
    }

    #[test]
    fn unknown_hook_card_is_rejected_before_any_ordinary_execution_mutation() {
        let mut card = test_card(88_888_888, 88_888_888, "星弈·未知剑阵");
        card.attack = Some(9);
        card.anima = Some(-3);
        card.hp_cost = Some(5);
        card.defense = Some(4);
        card.damage = Some(5);
        card.physique = Some(6);
        card.sword_intent = Some(7);
        card.hexagram = Some(8);
        let mut state = state_with_active_card(card);
        state.gain_anima(PlayerSide::P1, 20);
        state.p1.fate.rear_move_succeeded = true;
        state.p1.fate.qi_xing_lian_zhu = 3;
        state.p1.fate.plum_blossom_twice = 1;
        state.p1.turn.next_card_anima_cost_reduction = 2;
        state.p1.beng.beng_tian_step = 1;
        state.p1.beng.beng_quan_defense = 7;
        state.p1.beng.beng_quan_chan = 4;
        state.p1.identity.talents.push(192);
        state.p1.sword.ling_wu_card_base_ids.push(88_888_888);
        let before = format!("{state:#?}");

        assert!(!state.test_execute_one_card(PlayerSide::P1));

        let error = state
            .evaluation_error
            .take()
            .expect("missing card is always fatal without a decision-mode flag");
        assert!(matches!(
            &error,
            BattleError::MissingRule {
                card_id: 88_888_888,
                base_id: 88_888_888,
                turn: 0,
                ..
            }
        ));
        assert!(error
            .to_string()
            .contains("card catalog error: card:88888888 base:88888888"));
        assert_eq!(format!("{state:#?}"), before);
    }

    #[test]
    fn unknown_temporary_card_is_rejected_before_invocation_or_hook_mutation() {
        let mut card = test_card(88_888_889, 88_888_889, "星弈·未知临时剑阵");
        card.attack = Some(9);
        card.anima = Some(3);
        card.hp_cost = Some(5);
        let mut state = state_with_active_card(basic_attack_test_card());
        state.p1.fate.rear_move_succeeded = true;
        state.p1.fate.qi_xing_lian_zhu = 3;
        state.p1.beng.beng_tian_step = 1;
        state.p1.beng.beng_quan_defense = 7;
        let before = format!("{state:#?}");

        assert!(!state.apply_temporary_card_effect(PlayerSide::P1, &card, 0));

        let error = state
            .evaluation_error
            .take()
            .expect("temporary missing card is fatal");
        assert!(error
            .to_string()
            .contains("card catalog error: card:88888889 base:88888889"));
        assert_eq!(format!("{state:#?}"), before);
    }

    #[test]
    fn unknown_explicit_preserve_physical_temporary_is_rejected_before_used_mutation() {
        let mut card = test_card(88_888_891, 88_888_891, "未知显式已用临时牌");
        card.attack = Some(9);
        for (physical_was_used, explicit_had_used) in [(false, true), (true, false)] {
            let mut state = state_with_active_card(basic_attack_test_card());
            state.p1.deck.slots[0].used = physical_was_used;
            state.p1.fate.rear_move_succeeded = true;
            let before = format!("{state:#?}");
            let spec = TemporaryInvocationSpec {
                physical_slot: 0,
                invocation_slot: 7,
                had_used_source: TemporaryHadUsedSource::Explicit(explicit_had_used),
                deck_identity_mode: TemporaryDeckIdentityMode::PreservePhysical,
                inherit_parent_beng_quan: true,
            };

            assert!(!state.apply_temporary_card_effect_with_spec(PlayerSide::P1, &card, spec));

            let error = state
                .evaluation_error
                .take()
                .expect("temporary missing card is fatal");
            assert!(error
                .to_string()
                .contains("card catalog error: card:88888891 base:88888891"));
            assert_eq!(format!("{state:#?}"), before);
        }
    }

    #[test]
    fn all_public_replay_surfaces_return_catalog_missing_as_an_error() {
        let card = test_card(88_888_890, 88_888_890, "未知公开入口牌");
        let fixture = minimal_fixture(
            filler_cards(card),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );
        let errors = [
            super::super::run_replay_fixture(&fixture)
                .expect_err("summary surface must fail")
                .to_string(),
            super::super::run_replay_fixture_with_events(&fixture)
                .expect_err("event surface must fail")
                .to_string(),
            super::super::run_replay_fixture_with_parity_events(&fixture)
                .expect_err("parity surface must fail")
                .to_string(),
            super::super::run_replay_fixture_with_detailed_events(&fixture)
                .expect_err("detailed surface must fail")
                .to_string(),
        ];
        assert!(errors
            .iter()
            .all(|error| { error.contains("card catalog error: card:88888890 base:88888890") }));
    }

    #[test]
    fn ordinary_public_replay_surface_is_strict_about_missing_decisions() {
        let mut random = test_card(4_000_068, 4_000_068, "落花有意");
        random.attack = Some(1);
        random.random_attack = Some(2);
        random.other_params = vec![0];
        let fixture = minimal_fixture(
            filler_cards(random),
            filler_cards(basic_attack_test_card()),
            FixtureExpected {
                winner_side: PlayerSide::P1,
                actor_turn_count: 1,
                hp_delta_p1_minus_p2: 0,
                final_hp: None,
            },
        );

        let error = super::super::run_replay_fixture(&fixture)
            .expect_err("public replay must not invent a required random decision")
            .to_string();
        assert!(error.contains("missing original decision"));
    }

    #[test]
    fn verified_printed_fallback_executes_only_for_an_audited_base_id() {
        let mut card = test_card(1_000_005, 1_000_005, "护身灵气");
        card.anima = Some(2);
        card.defense = Some(5);
        let mut state = state_with_active_card(card.clone());
        state.fail_on_missing_decision = true;

        state.test_apply_card_effect(PlayerSide::P1, &card, 0);

        assert_eq!((state.p1.core.anima, state.p1.core.defense), (2, 5));
        assert_eq!(state.evaluation_error, None);
    }

    #[test]
    fn explicit_record_only_card_is_a_no_op_without_becoming_missing() {
        let mut card = test_card(1, 1, "云泉道茶");
        card.attack = Some(99);
        card.anima = Some(99);
        card.defense = Some(99);
        let mut state = state_with_active_card(card.clone());
        state.fail_on_missing_decision = true;
        let before = (state.p1.core.anima, state.p1.core.defense, state.p2.core.hp);

        state.test_apply_card_effect(PlayerSide::P1, &card, 0);

        assert_eq!(
            (state.p1.core.anima, state.p1.core.defense, state.p2.core.hp,),
            before
        );
        assert_eq!(state.evaluation_error, None);
    }

    #[test]
    fn canonical_refine_type_is_record_only_without_an_id_allowlist() {
        let mut card = test_card(9_000_016, 9_000_016, "未列举炼化牌");
        card.card_type = Some(OriginalEnumValue {
            value: 2,
            name: "Refine".to_string(),
        });
        card.attack = Some(99);
        card.anima = Some(99);
        card.defense = Some(99);
        let mut state = state_with_active_card(card.clone());
        state.fail_on_missing_decision = true;
        let before = (state.p1.core.anima, state.p1.core.defense, state.p2.core.hp);

        state.test_apply_card_effect(PlayerSide::P1, &card, 0);

        assert_eq!(
            (state.p1.core.anima, state.p1.core.defense, state.p2.core.hp),
            before
        );
        assert_eq!(state.evaluation_error, None);
    }

    #[test]
    fn policy_lists_are_sorted_unique_and_disjoint() {
        assert!(RECORD_ONLY_CARD_BASE_IDS
            .windows(2)
            .all(|ids| ids[0] < ids[1]));
        assert!(VERIFIED_PRINTED_FALLBACK_BASE_IDS
            .windows(2)
            .all(|ids| ids[0] < ids[1]));
        assert!(EXPLICIT_FALLBACK_BASE_IDS
            .windows(2)
            .all(|ids| ids[0] < ids[1]));
        assert!(RECORD_ONLY_CARD_BASE_IDS
            .iter()
            .all(|id| !VERIFIED_PRINTED_FALLBACK_BASE_IDS.contains(id)));
        assert!(RECORD_ONLY_CARD_BASE_IDS
            .iter()
            .all(|id| !EXPLICIT_FALLBACK_BASE_IDS.contains(id)));
        assert!(VERIFIED_PRINTED_FALLBACK_BASE_IDS
            .iter()
            .all(|id| !EXPLICIT_FALLBACK_BASE_IDS.contains(id)));
    }

    #[test]
    fn executable_catalog_and_rust_dispatch_are_bidirectionally_complete() {
        let state = state_with_active_card(basic_attack_test_card());
        let catalog = catalog_executable_base_ids();
        assert!(
            catalog.contains(&0),
            "base 0 is the explicit normal-attack handler"
        );

        let mut catalog_without_handler = Vec::new();
        for &base_id in catalog {
            let Some(card) = super::super::original_card_definition_by_id(base_id) else {
                catalog_without_handler.push((base_id, "definition"));
                continue;
            };
            let handled = VERIFIED_PRINTED_FALLBACK_BASE_IDS.contains(&base_id)
                || EXPLICIT_FALLBACK_BASE_IDS.contains(&base_id)
                || state.probe_has_typed_card_effect(PlayerSide::P1, &card, 0);
            if !handled {
                catalog_without_handler.push((base_id, "handler"));
            }
        }

        let mut handler_without_catalog = Vec::new();
        for card in super::super::original_config::original_base_card_definitions() {
            let base_id = super::super::support::normalized_base_id(&card);
            let handled = VERIFIED_PRINTED_FALLBACK_BASE_IDS.contains(&base_id)
                || EXPLICIT_FALLBACK_BASE_IDS.contains(&base_id)
                || state.probe_has_typed_card_effect(PlayerSide::P1, &card, 0);
            let record_only =
                resolve_card_effect(&card, base_id) == CardEffectResolution::RecordOnly;
            if handled && !record_only && !catalog.contains(&base_id) {
                handler_without_catalog.push(base_id);
            }
        }

        assert!(
            catalog_without_handler.is_empty(),
            "catalog executable IDs without Rust handlers: {catalog_without_handler:?}"
        );
        assert!(
            handler_without_catalog.is_empty(),
            "Rust handlers missing from catalog: {handler_without_catalog:?}"
        );
    }
}
