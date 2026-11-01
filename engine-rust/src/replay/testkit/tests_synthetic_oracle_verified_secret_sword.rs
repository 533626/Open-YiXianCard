use super::effect_invocation::EffectInvocationKind;
use super::support::{
    is_cloud_sword, is_frenzy_sword, is_frenzy_sword_with_options, is_spirit_sword_for_actor,
};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide};

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::resize_deck(vec![active], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, None, 1, None)
}

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
    super::test_support::default_fixture(player(p1_cards), player(p2_cards))
}

fn state_with(card: CardDefinition) -> ReplayState {
    ReplayState::test_from_fixture(&fixture(deck(card), deck(basic_attack())))
}

fn begin_direct_effect(state: &mut ReplayState, card: &CardDefinition) {
    let physical = state.p1.deck.slots[0].card.clone();
    let is_beng_quan = state.is_dream_mirage_intrinsic_beng_quan(PlayerSide::P1, card);
    state.begin_effect_invocation(
        PlayerSide::P1,
        card,
        card,
        &physical,
        0,
        0,
        EffectInvocationKind::Played,
        is_beng_quan,
    );
}

#[test]
fn shared_sword_classifiers_cover_original_cloud_and_frenzy_extensions() {
    let plain = basic_attack();
    let formation_career = original_card_definition_by_id(160).expect("missing formation career");
    let qin_career = original_card_definition_by_id(158).expect("missing qin career");
    let rare_card = original_card_definition_by_id(10_000).expect("missing rare basic attack");
    let skyward_formation =
        original_card_definition_by_id(1_000_025).expect("missing skyward formation");
    let cat_card = original_card_definition_by_id(37).expect("missing cat card");
    assert_eq!(formation_career.career_name.as_deref(), Some("ZhenFaShi"));
    assert_eq!(qin_career.career_name.as_deref(), Some("QinShi"));
    assert_eq!(rare_card.rarity, Some(1));

    let mut state = state_with(plain.clone());
    state.p1.sword.cloud_sword_heaven_cycle = 1;
    assert!(is_cloud_sword(&state.p1, &formation_career));
    assert!(!is_cloud_sword(&state.p1, &plain));

    state.p1.sword.cloud_sword_heaven_cycle = 0;
    state.p1.sword.all_cards_as_cloud_sword = 1;
    assert!(is_cloud_sword(&state.p1, &plain));

    state.p1.sword.all_cards_as_cloud_sword = 0;
    state.p1.identity.talent_resonance_id = Some(109);
    state.p1.identity.talent_resonance_temp_flags.push(109);
    assert!(is_cloud_sword(&state.p1, &plain));

    state.p1.identity.talent_resonance_id = None;
    state.p1.identity.talent_resonance_temp_flags.clear();
    state.p1.identity.ke_yin_card_ids.push(50_146);
    assert!(is_cloud_sword(&state.p1, &skyward_formation));
    assert!(is_frenzy_sword(&state.p1, &skyward_formation));
    assert!(is_spirit_sword_for_actor(&state.p1, &skyward_formation));

    state.p1.identity.ke_yin_card_ids.clear();
    state.p1.identity.fate_strategies.push(322);
    assert!(is_frenzy_sword(&state.p1, &cat_card));

    state.p1.identity.fate_strategies.clear();
    state.p1.sword.frenzy_sword_cloud_gathering = 1;
    assert!(is_frenzy_sword(&state.p1, &rare_card));
    assert!(!is_frenzy_sword(&state.p1, &plain));

    state.p1.sword.frenzy_sword_cloud_gathering = 0;
    state.p1.sword.all_purpose_sword = 1;
    state.p1.sword.all_purpose_sword_effective_count = 1;
    assert!(is_cloud_sword(&state.p1, &plain));
    assert!(is_frenzy_sword(&state.p1, &plain));
    assert!(
        !is_frenzy_sword_with_options(&state.p1, &plain, true),
        "ignoreTemporaryBuff must exclude all-purpose identity"
    );

    state.p1.sword.all_purpose_sword = 0;
    state.p1.sword.all_purpose_sword_effective_count = 0;
    state.p1.sword.next_cards_as_frenzy_sword = 1;
    state.p1.sword.next_cards_as_frenzy_sword_effective_count = 1;
    assert!(is_frenzy_sword(&state.p1, &plain));
    assert!(!is_frenzy_sword_with_options(&state.p1, &plain, true));

    state.p1.sword.next_cards_as_frenzy_sword = 0;
    state.p1.sword.next_cards_as_frenzy_sword_effective_count = 0;
    state.p1.sword.next_card_as_cloud_sword = 1;
    assert!(is_cloud_sword(&state.p1, &plain));
    state.p1.sword.frenzy_dragon_swallows_cloud = 1;
    assert!(is_frenzy_sword(&state.p1, &plain));

    state.p1.sword.next_card_as_cloud_sword = 0;
    state.p1.sword.frenzy_dragon_swallows_cloud = 0;
    let all_sword_identity = original_card_definition_by_id(261).expect("missing all-sword card");
    assert!(is_cloud_sword(&state.p1, &all_sword_identity));
    assert!(is_frenzy_sword(&state.p1, &all_sword_identity));

    state.p1.identity.fate_strategies.push(319);
    assert!(is_frenzy_sword(&state.p1, &qin_career));
}

#[test]
fn used_ke_yin_50146_flows_into_identity_and_battle_initial_max_hp() {
    let skyward_formation =
        original_card_definition_by_id(1_000_025).expect("missing skyward formation");
    let mut active_fixture = fixture(deck(skyward_formation.clone()), deck(basic_attack()));
    active_fixture.players.p1.used_ke_yin_cards = vec![50_146];
    let state = ReplayState::test_from_fixture(&active_fixture);

    assert_eq!(state.p1.identity.ke_yin_card_ids, vec![50_146]);
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (54, 54));
    assert!(is_cloud_sword(&state.p1, &skyward_formation));
    assert!(is_frenzy_sword(&state.p1, &skyward_formation));
    assert!(is_spirit_sword_for_actor(&state.p1, &skyward_formation));

    let mut ordered_fixture = fixture(deck(basic_attack()), deck(basic_attack()));
    ordered_fixture.players.p1.base_max_hp = 19;
    ordered_fixture.players.p1.fate_strategies = vec![140];
    ordered_fixture.players.p1.used_ke_yin_cards = vec![50_146];
    let ordered = ReplayState::test_from_fixture(&ordered_fixture);
    assert_eq!(
        (ordered.p1.core.hp, ordered.p1.core.max_hp),
        (24, 24),
        "KeYin max HP is applied after percentage-based opening effects"
    );

    let mut cross_player_fixture = fixture(deck(basic_attack()), deck(basic_attack()));
    cross_player_fixture.players.p1.base_max_hp = 1;
    cross_player_fixture.players.p1.used_ke_yin_cards = vec![50_146];
    cross_player_fixture.players.p2.fate_strategies = vec![146];
    let cross_player = ReplayState::test_from_fixture(&cross_player_fixture);
    assert_eq!(
        (cross_player.p1.core.hp, cross_player.p1.core.max_hp),
        (3, 3),
        "the first player's KeYin resolves before the second player's opening effects"
    );
}

#[test]
fn used_ke_yin_50146_makes_skyward_formation_trigger_the_real_sword_energy_after_hook() {
    let skyward_formation =
        original_card_definition_by_id(1_000_025).expect("missing skyward formation");
    let mut active_fixture = fixture(deck(skyward_formation), deck(basic_attack()));
    active_fixture.players.p1.used_ke_yin_cards = vec![50_146];
    let mut state = ReplayState::test_from_fixture(&active_fixture);
    state.p1.sword.sword_energy = 7;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 44);
    assert_eq!(state.p1.sword.sword_energy, 6);
}

#[test]
fn sword_intent_circulation_preserves_exactly_one_attacked_card_spend() {
    let card = original_card_definition_by_id(1_000_050).expect("missing sword circulation");
    let mut state = state_with(card.clone());

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_sword_card_effect(
            PlayerSide::P1,
            &card,
            0,
            1_000_050,
        ),
        Some(false)
    );
    assert_eq!(state.p1.sword.sword_intent, 2);
    assert_eq!(state.p1.sword.sword_intent_circulation, 1);

    assert!(!state.preserve_secret_sword_intent_with_circulation(PlayerSide::P1));
    assert_eq!(state.p1.sword.sword_intent_circulation, 1);
    begin_direct_effect(&mut state, &card);
    state.set_active_effect_pending_sword_intent(2);
    assert!(state.preserve_secret_sword_intent_with_circulation(PlayerSide::P1));
    assert_eq!(state.p1.sword.sword_intent, 2);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.p1.sword.sword_intent_circulation, 0);
    assert!(!state.preserve_secret_sword_intent_with_circulation(PlayerSide::P1));
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn dark_void_art_only_appends_attack_after_non_temporary_sword_formation() {
    let art = original_card_definition_by_id(1_020_051).expect("missing dark void art");
    let formation = original_card_definition_by_id(48).expect("missing azure dragon formation");
    let non_formation = original_card_definition_by_id(1_000_012).expect("missing sword block");
    let mut state = state_with(art.clone());

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_sword_card_effect(
            PlayerSide::P1,
            &art,
            0,
            1_000_051,
        ),
        Some(false)
    );
    assert_eq!(state.p1.sword.dark_void_sword_formation_art, 13);
    assert!(state.apply_secret_sword_formation_follow_up(PlayerSide::P1, &formation, 0, false,));
    assert_eq!(state.p2.core.hp, 37);
    assert!(!state.apply_secret_sword_formation_follow_up(
        PlayerSide::P1,
        &non_formation,
        0,
        false,
    ));
    assert!(!state.apply_secret_sword_formation_follow_up(PlayerSide::P1, &formation, 0, true,));
    assert_eq!(state.p2.core.hp, 37);
}

#[test]
fn spirit_sword_mindset_adds_its_stored_amount_at_owner_turn_start() {
    let mindset = original_card_definition_by_id(1_000_053).expect("missing spirit sword mindset");
    let mut state = state_with(mindset.clone());

    state.apply_synthetic_oracle_verified_secret_sword_card_effect(
        PlayerSide::P1,
        &mindset,
        0,
        1_000_053,
    );
    assert_eq!(state.p1.sword.spirit_sword_mindset, 2);
    assert!(state.apply_secret_sword_mindset_at_turn_start(PlayerSide::P1));
    assert_eq!(state.p1.sword.sword_intent, 2);
    assert!(!state.apply_secret_sword_mindset_at_turn_start(PlayerSide::P2));
}

#[test]
fn single_heart_uses_sword_intent_multiplier_without_weakness_penalty() {
    let card = original_card_definition_by_id(1_000_056).expect("missing single heart");
    let mut state = state_with(card.clone());
    state.p1.sword.sword_intent = 2;
    state.p1.status.weakness = 3;
    begin_direct_effect(&mut state, &card);

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_sword_card_effect(
            PlayerSide::P1,
            &card,
            0,
            1_000_056,
        ),
        Some(true)
    );
    assert_eq!(state.p2.core.hp, 36);
    assert_eq!(state.p1.status.weakness, 3);
    assert_eq!(state.p1.turn.ignore_weakness_attacks, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 2);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn single_heart_defaults_a_missing_sword_intent_multiplier_to_one() {
    let mut card = original_card_definition_by_id(1_000_056).expect("missing single heart");
    card.other_params = Default::default();
    let mut state = state_with(card.clone());
    state.p1.sword.sword_intent = 2;
    begin_direct_effect(&mut state, &card);

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_sword_card_effect(
            PlayerSide::P1,
            &card,
            0,
            1_000_056,
        ),
        Some(true)
    );
    assert_eq!(state.p2.core.hp, 42);
    assert_eq!(state.active_effect_pending_sword_intent(), 2);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
}

#[test]
fn cloud_step_only_grants_attack_bonus_after_cloud_sword() {
    let cloud_step = original_card_definition_by_id(1_020_057).expect("missing cloud step");
    let cloud = original_card_definition_by_id(1_000_014).expect("missing cloud sword");
    let non_cloud = original_card_definition_by_id(1_000_012).expect("missing sword block");
    let mut state = state_with(cloud_step.clone());

    state.apply_synthetic_oracle_verified_secret_sword_card_effect(
        PlayerSide::P1,
        &cloud_step,
        0,
        1_000_057,
    );
    assert_eq!(state.p1.sword.cloud_step, 2);
    assert!(state.apply_secret_sword_cloud_step_after_cloud_sword(PlayerSide::P1, &cloud));
    assert_eq!(state.p1.core.attack_bonus, 2);
    assert!(!state.apply_secret_sword_cloud_step_after_cloud_sword(PlayerSide::P1, &non_cloud,));
    assert_eq!(state.p1.core.attack_bonus, 2);
}

#[test]
fn double_dragon_stores_and_spends_one_layer_only_for_frenzy_sword() {
    let double_dragon = original_card_definition_by_id(1_000_061).expect("missing double dragon");
    let frenzy = original_card_definition_by_id(2).expect("missing frenzy sword");
    let plain = basic_attack();
    let mut state = state_with(double_dragon.clone());

    assert_eq!(
        state.apply_synthetic_oracle_verified_secret_sword_card_effect(
            PlayerSide::P1,
            &double_dragon,
            0,
            1_000_061,
        ),
        Some(true)
    );
    assert_eq!(state.p2.core.hp, 48);
    assert_eq!(state.p1.sword.frenzy_sword_double_effect, 1);
    assert_eq!(
        state.consume_secret_sword_double_dragon_repetition(PlayerSide::P1, &plain),
        0
    );
    assert_eq!(state.p1.sword.frenzy_sword_double_effect, 1);
    assert_eq!(
        state.consume_secret_sword_double_dragon_repetition(PlayerSide::P1, &frenzy),
        1
    );
    assert_eq!(state.p1.sword.frenzy_sword_double_effect, 0);
}

#[test]
fn sword_intent_circulation_runs_through_the_shared_transaction_without_double_gain() {
    let card = original_card_definition_by_id(1_000_050).expect("missing sword circulation");
    let mut state = state_with(card);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.sword.sword_intent, 2);
    assert_eq!(state.p1.sword.sword_intent_circulation, 1);
}

#[test]
fn dark_void_and_cloud_step_run_from_shared_after_card_hooks() {
    let art = original_card_definition_by_id(1_020_051).expect("missing dark void art");
    let formation = original_card_definition_by_id(48).expect("missing azure dragon formation");
    let mut art_deck = deck(art);
    art_deck[1] = formation;
    let mut art_fixture = fixture(art_deck, deck(basic_attack()));
    art_fixture.players.p1.active_slot_count = 2;
    let mut art_state = ReplayState::test_from_fixture(&art_fixture);

    art_state.test_execute_one_card(PlayerSide::P1);
    art_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(art_state.p2.core.hp, 22);

    let cloud_step = original_card_definition_by_id(1_020_057).expect("missing cloud step");
    let cloud = original_card_definition_by_id(1_000_014).expect("missing cloud sword");
    let mut cloud_deck = deck(cloud_step);
    cloud_deck[1] = cloud;
    let mut cloud_fixture = fixture(cloud_deck, deck(basic_attack()));
    cloud_fixture.players.p1.active_slot_count = 2;
    let mut cloud_state = ReplayState::test_from_fixture(&cloud_fixture);

    cloud_state.test_execute_one_card(PlayerSide::P1);
    cloud_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(cloud_state.p1.core.attack_bonus, 2);
}

#[test]
fn spirit_sword_mindset_runs_at_the_next_shared_turn_start() {
    let mindset = original_card_definition_by_id(1_000_053).expect("missing spirit sword mindset");
    let mut support = basic_attack();
    support.name = "test support".to_string().into();
    support.attack = Some(0);
    support.attack_count = Some(0);
    let mut cards = deck(mindset);
    cards[1] = support;
    let mut active_fixture = fixture(cards, deck(basic_attack()));
    active_fixture.players.p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&active_fixture);

    state.test_execute_one_card(PlayerSide::P1);
    state.test_play_actor_turn();

    assert_eq!(state.p1.sword.sword_intent, 2);
}

#[test]
fn single_heart_shared_transaction_clears_its_marker_and_spends_sword_intent() {
    let card = original_card_definition_by_id(1_000_056).expect("missing single heart");
    let mut state = state_with(card);
    state.p1.sword.sword_intent = 2;
    state.p1.status.weakness = 3;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 36);
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.p1.turn.ignore_weakness_attacks, 0);
}

#[test]
fn double_dragon_repetition_accumulates_after_plum_blossom_sets_two() {
    let double_dragon = original_card_definition_by_id(1_000_061).expect("missing double dragon");
    let frenzy = original_card_definition_by_id(2).expect("missing frenzy sword");
    let mut cards = deck(double_dragon);
    cards[1] = frenzy;
    let mut active_fixture = fixture(cards, deck(basic_attack()));
    active_fixture.players.p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&active_fixture);

    state.test_execute_one_card(PlayerSide::P1);
    state.p1.fate.plum_blossom_twice = 1;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.turn.attack_segments_performed, 4);
    assert_eq!(state.p2.core.hp, 39);
    assert_eq!(state.p1.sword.frenzy_sword_double_effect, 0);
    assert_eq!(state.p1.fate.plum_blossom_twice, 0);
}

#[test]
fn double_dragon_repeats_the_full_completion_boundary_and_spends_sword_intent_once() {
    let frenzy = original_card_definition_by_id(2).expect("missing frenzy sword");
    let mut state = state_with(frenzy);
    state.p1.sword.sword_intent = 3;
    state.p1.sword.frenzy_sword_double_effect = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 42);
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.p1.sword.frenzy_sword, 2);
    assert_eq!(state.p1.sword.frenzy_sword_double_effect, 0);
    assert_eq!(state.p1.turn.attack_segments_performed, 2);
    assert_eq!(state.p1.turn.used_card_count, 2);
    assert_eq!(state.active_effect_actual_damage(), 0);
    assert_eq!(state.active_effect_wounded_count(), 0);
    assert_eq!(
        state
            .test_events()
            .iter()
            .filter(|event| event.kind == ReplayEventKind::CardCompleted)
            .count(),
        1
    );
}

#[test]
fn keyin_skyward_double_dragon_repeats_on_before_and_all_sword_classifications() {
    let skyward_formation =
        original_card_definition_by_id(1_000_025).expect("missing skyward formation");
    let mut active_fixture = fixture(deck(skyward_formation), deck(basic_attack()));
    active_fixture.players.p1.used_ke_yin_cards = vec![50_146];
    active_fixture.players.p1.fate_strategies = vec![97];
    active_fixture.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&active_fixture);
    state.p1.sword.cloud_step = 2;
    state.p1.sword.frenzy_sword_double_effect = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.sword.cloud_sea, 2);
    assert_eq!(state.p1.sword.cloud_chain, 2);
    assert_eq!(state.p1.core.attack_bonus, 4);
    assert_eq!(state.p1.sword.frenzy_sword, 2);
    assert_eq!(state.p1.sword.sword_formation_count, 2);
    assert_eq!(state.p1.turn.used_card_count, 2);
}

#[test]
fn repeated_sustain_pays_hp_cost_and_completes_the_outer_transaction_once() {
    let mut sustain = basic_attack();
    sustain.name = "test repeated sustain".to_string().into();
    sustain.card_type = Some(OriginalEnumValue {
        value: CARD_TYPE_SUSTAIN,
        name: "Sustain".to_string().into(),
    });
    sustain.hp_cost = Some(4);
    let mut state = state_with(sustain);
    state.p1.fate.plum_blossom_twice = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 46);
    assert_eq!(state.p2.core.hp, 44);
    assert_eq!(state.p1.turn.hp_cost_cards_used, 2);
    assert_eq!(state.p1.turn.used_card_count, 2);
    assert_eq!(state.p1.formations.array_echo_persistent_card, 2);
    assert_eq!(state.active_effect_actual_damage(), 0);
    assert_eq!(state.active_effect_wounded_count(), 0);
    assert!(state.p1.deck.slots[0].used);
    assert!(state.p1.deck.slots[0].skipped);
    assert_eq!(
        state
            .test_events()
            .iter()
            .filter(|event| event.kind == ReplayEventKind::CardCompleted)
            .count(),
        1
    );
}
