use super::cards_dream_mirage::DreamMirageValue;
use super::*;
use crate::fixture::{
    BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(0)
}

fn deck_with(mut cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: Some(0),
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 1,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: Some(6),
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: BTreeMap::new(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: BTreeMap::new(),
        talent_card_params: BTreeMap::new(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards,
    }
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: Some(FixtureSource {
            steam_build: Some(
                super::original_build_profile::project_target_steam_build().to_string(),
            ),
            ..FixtureSource::default()
        }),
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
        max_actor_turns: Some(1),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers { p1, p2 },
    }
}

fn fixture_with_strategy(strategy: i64, card_id: i64) -> BattleFixture {
    let mut p1 = player(deck_with(vec![original_card(card_id)]));
    p1.fate_strategies = vec![strategy];
    fixture(p1, player(deck_with(vec![basic_attack()])))
}

#[test]
fn fate_84_grants_sword_intent_only_after_a_turn_without_attack() {
    let mut quiet = ReplayState::test_from_fixture(&fixture_with_strategy(84, 12));
    quiet.modify_dream_mirage_value(PlayerSide::P1, DreamMirageValue::SwordIntentGainDefense, 2);
    quiet.test_play_actor_turn();
    assert_eq!(quiet.p1.turn.turn_attack_segments, 0);
    assert_eq!(quiet.p1.sword.sword_intent, 1);
    assert_eq!(quiet.p1.core.defense, 2);

    let mut attacked = ReplayState::test_from_fixture(&fixture_with_strategy(84, 0));
    attacked.test_play_actor_turn();
    assert_eq!(attacked.p1.turn.turn_attack_segments, 1);
    assert_eq!(attacked.p1.sword.sword_intent, 0);
}

#[test]
fn fate_109_marks_and_rewards_only_the_first_sword_formation() {
    let mut state = ReplayState::test_from_fixture(&fixture_with_strategy(109, 48));
    assert_eq!(state.p1.fate.sword_formation_guard, 1);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.sword_formation_guard, 0);
    assert_eq!(state.p1.core.defense, 13);
    assert_eq!(state.p1.core.guard, 1);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.defense, 21);
    assert_eq!(state.p1.core.guard, 1);
}

#[test]
fn fate_322_starts_with_one_frenzy_sword_and_treats_cat_cards_as_frenzy() {
    let mut active = ReplayState::test_from_fixture(&fixture_with_strategy(322, 37));
    assert_eq!(active.p1.sword.frenzy_sword, 1);
    active.test_execute_one_card(PlayerSide::P1);
    assert_eq!(active.p1.sword.frenzy_sword, 2);

    let mut control = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![original_card(37)])),
        player(deck_with(vec![basic_attack()])),
    ));
    control.test_execute_one_card(PlayerSide::P1);
    assert_eq!(control.p1.sword.frenzy_sword, 0);
}

#[test]
fn fate_327_upgrades_the_first_upgradable_thunder_or_action_again_card() {
    let mut by_description = player(deck_with(vec![
        original_card(20_012),
        original_card(12),
        original_card(1_000_009),
    ]));
    by_description.active_slot_count = 3;
    by_description.fate_strategies = vec![327];
    let state = ReplayState::test_from_fixture(&fixture(
        by_description,
        player(deck_with(vec![basic_attack()])),
    ));
    assert_eq!(state.p1.deck.slots[0].card.id, 20_012);
    assert_eq!(state.p1.deck.slots[1].card.id, 10_012);
    assert_eq!(state.p1.deck.slots[2].card.id, 1_000_009);

    let mut by_name = player(deck_with(vec![
        original_card(1_000_012),
        original_card(1_000_009),
    ]));
    by_name.active_slot_count = 2;
    by_name.fate_strategies = vec![327];
    let state =
        ReplayState::test_from_fixture(&fixture(by_name, player(deck_with(vec![basic_attack()]))));
    assert_eq!(state.p1.deck.slots[1].card.id, 1_010_009);

    let mut inactive = player(deck_with(vec![
        original_card(1_000_012),
        original_card(1_000_009),
    ]));
    inactive.active_slot_count = 1;
    inactive.fate_strategies = vec![327];
    let state =
        ReplayState::test_from_fixture(&fixture(inactive, player(deck_with(vec![basic_attack()]))));
    assert_eq!(state.p1.deck.slots[1].card.id, 1_000_009);
}

#[test]
fn fate_327_upgrades_a_rarity_one_card_to_rarity_two() {
    let mut p1 = player(deck_with(vec![original_card(10_012)]));
    p1.fate_strategies = vec![327];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.deck.slots[0].card.id, 20_012);
}

#[test]
fn fate_327_matches_action_again_on_the_current_card_config_only() {
    for skipped_card_id in [67, 10_067] {
        let mut p1 = player(deck_with(vec![
            original_card(skipped_card_id),
            original_card(12),
        ]));
        p1.active_slot_count = 2;
        p1.fate_strategies = vec![327];
        let state =
            ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

        assert_eq!(state.p1.deck.slots[0].card.id, skipped_card_id);
        assert_eq!(state.p1.deck.slots[1].card.id, 10_012);
    }
}

#[test]
fn fate_329_grants_one_rear_move_bypass_only_to_the_second_player() {
    let p1 = player(deck_with(vec![basic_attack()]));
    let mut p2 = player(deck_with(vec![original_card(12)]));
    p2.fate_strategies = vec![329];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));

    assert_eq!(state.p1.fate.next_rear_move_bypass, 0);
    assert_eq!(state.p2.fate.next_rear_move_bypass, 1);
    assert!(state.check_rear_move(PlayerSide::P2, false));
    assert_eq!(state.p2.fate.next_rear_move_bypass, 0);
    assert!(!state.check_rear_move(PlayerSide::P2, false));

    let mut first = player(deck_with(vec![original_card(12)]));
    first.fate_strategies = vec![329];
    let state =
        ReplayState::test_from_fixture(&fixture(first, player(deck_with(vec![basic_attack()]))));
    assert_eq!(state.p1.fate.next_rear_move_bypass, 0);
}

#[test]
fn fate_330_tracks_consumed_and_expiring_temporary_guard_exactly() {
    let mut source = fixture_with_strategy(330, 22);
    source.players.p1.initial_guard = 2;
    let mut state = ReplayState::test_from_fixture(&source);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.guard, 5);
    assert_eq!(state.p1.core.temporary_guard, 3);

    assert_eq!(state.modify_actor_hp(PlayerSide::P1, -10, false, false), 0);
    assert_eq!(state.modify_actor_hp(PlayerSide::P1, -10, false, false), 0);
    assert_eq!((state.p1.core.guard, state.p1.core.temporary_guard), (3, 1));

    state.clear_temporary_guard_at_turn_start(PlayerSide::P1);
    assert_eq!((state.p1.core.guard, state.p1.core.temporary_guard), (2, 0));

    state.test_execute_one_card(PlayerSide::P1);
    for _ in 0..4 {
        assert_eq!(state.modify_actor_hp(PlayerSide::P1, -10, false, false), 0);
    }
    assert_eq!((state.p1.core.guard, state.p1.core.temporary_guard), (1, 0));
    state.clear_temporary_guard_at_turn_start(PlayerSide::P1);
    assert_eq!(state.p1.core.guard, 1);
}

#[test]
fn fate_330_does_not_match_unrelated_high_family_card_ids() {
    for card_id in [1_000_022, 4_000_022] {
        let mut state = ReplayState::test_from_fixture(&fixture_with_strategy(330, card_id));
        state.test_execute_one_card(PlayerSide::P1);
        assert_eq!(state.p1.core.guard, 0, "card {card_id}");
        assert_eq!(state.p1.core.temporary_guard, 0, "card {card_id}");
    }
}

#[test]
fn fate_332_applies_weakness_on_first_rear_move_check_even_when_it_fails() {
    let mut state = ReplayState::test_from_fixture(&fixture_with_strategy(332, 12));

    assert!(!state.check_rear_move(PlayerSide::P1, false));
    assert_eq!(state.p2.status.weakness, 1);
    assert!(state.check_rear_move(PlayerSide::P1, true));
    assert_eq!(state.p2.status.weakness, 1);
}

#[test]
fn fate_334_upgrades_only_the_first_upgradable_wood_spirit_card() {
    let mut p1 = player(deck_with(vec![
        original_card(7_020_003),
        original_card(7_000_003),
        original_card(7_000_004),
    ]));
    p1.active_slot_count = 3;
    p1.fate_strategies = vec![334];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.deck.slots[0].card.id, 7_020_003);
    assert_eq!(state.p1.deck.slots[1].card.id, 7_010_003);
    assert_eq!(state.p1.deck.slots[2].card.id, 7_000_004);

    let mut inactive = player(deck_with(vec![basic_attack(), original_card(7_000_003)]));
    inactive.active_slot_count = 1;
    inactive.fate_strategies = vec![334];
    let state =
        ReplayState::test_from_fixture(&fixture(inactive, player(deck_with(vec![basic_attack()]))));
    assert_eq!(state.p1.deck.slots[1].card.id, 7_000_003);
}

#[test]
fn fate_334_upgrades_a_rarity_one_card_to_rarity_two() {
    let mut p1 = player(deck_with(vec![original_card(7_010_003)]));
    p1.fate_strategies = vec![334];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.deck.slots[0].card.id, 7_020_003);
}

#[test]
fn fate_404_upgrades_only_the_first_upgradable_hexagram_card() {
    let mut p1 = player(deck_with(vec![
        original_card(4_020_003),
        original_card(4_010_003),
        original_card(4_000_002),
    ]));
    p1.active_slot_count = 3;
    p1.fate_strategies = vec![404];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.deck.slots[0].card.id, 4_020_003);
    assert_eq!(state.p1.deck.slots[1].card.id, 4_020_003);
    assert_eq!(state.p1.deck.slots[2].card.id, 4_000_002);
}

#[test]
fn fate_404_does_not_upgrade_inactive_hexagram_cards() {
    let mut p1 = player(deck_with(vec![original_card(4_010_003)]));
    p1.active_slot_count = 0;
    p1.fate_strategies = vec![404];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.deck.slots[0].card.id, 4_010_003);
}

fn current_build() -> &'static str {
    super::original_build_profile::project_target_steam_build()
}

fn li_fixture(strategy_ids: Vec<i64>, card_id: i64, build: &str) -> BattleFixture {
    let mut p1 = player(deck_with(vec![original_card(card_id)]));
    p1.character_id = Some(4_000_005);
    p1.fate_strategies = strategy_ids;
    let mut battle = fixture(p1, player(deck_with(vec![basic_attack()])));
    battle.source = Some(FixtureSource {
        steam_build: Some(build.to_string()),
        ..FixtureSource::default()
    });
    battle
}

#[test]
fn fate_335_locks_staff_and_rewrites_all_three_stance_switch_callers() {
    for card_id in [219, 220, 222] {
        let mut state =
            ReplayState::test_from_fixture(&li_fixture(vec![335], card_id, current_build()));
        assert_eq!(
            (state.p1.beng.quan_stance, state.p1.beng.gun_stance),
            (0, 1)
        );

        state.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (state.p1.beng.quan_stance, state.p1.beng.gun_stance),
            (0, 1)
        );
        assert_eq!(state.p1.beng.momentum_limit, 7, "card {card_id}");
        assert_eq!(state.p1.beng.momentum, 1, "card {card_id}");
        assert_eq!(state.p1.core.defense, 3, "card {card_id}");
    }
}

#[test]
fn historical_build_profiles_remain_audit_only_and_fail_closed_at_runtime() {
    let retired = super::original_build_profile::latest_retired_steam_build()
        .expect("profile contract retains an audited retired build");
    let error = ReplayState::from_fixture(&li_fixture(vec![335], 222, retired), false)
        .expect_err("historical Steam build must not enter the runtime");
    assert!(matches!(
        &error,
        BattleError::UnsupportedBuild { turn: 0, .. }
    ));
    let message = error.to_string();
    assert!(message.contains(&format!("unsupported original Steam build {retired}")));
    assert!(message.contains("historical profile is incomplete"));
}

#[test]
fn fate_379_grants_jian_qi_and_triggers_it_after_cloud_sword() {
    let mut active = ReplayState::test_from_fixture(&fixture_with_strategy(379, 1_000_039));
    assert_eq!(active.p1.sword.sword_energy, 1);

    let cloud_sword = original_card(1_000_039);
    active.apply_sword_energy_after_card_hook(PlayerSide::P1, &cloud_sword);
    assert_eq!(active.p2.core.hp, 99);
    assert_eq!(active.p1.sword.sword_energy, 1);

    let hp_after_cloud = active.p2.core.hp;
    active.apply_sword_energy_after_card_hook(PlayerSide::P1, &basic_attack());
    assert_eq!(active.p2.core.hp, hp_after_cloud);
}

#[test]
fn fate_349_adds_three_damage_without_locking_stance() {
    for card_id in [219, 220, 222] {
        let mut active =
            ReplayState::test_from_fixture(&li_fixture(vec![349], card_id, current_build()));
        let mut control =
            ReplayState::test_from_fixture(&li_fixture(Vec::new(), card_id, current_build()));
        active.test_execute_one_card(PlayerSide::P1);
        control.test_execute_one_card(PlayerSide::P1);

        assert_eq!(
            (active.p1.beng.quan_stance, active.p1.beng.gun_stance),
            (control.p1.beng.quan_stance, control.p1.beng.gun_stance)
        );
        assert_eq!(
            (control.p1.beng.quan_stance, control.p1.beng.gun_stance),
            (0, 1)
        );
        assert_eq!(active.p2.core.hp, control.p2.core.hp - 3, "card {card_id}");
    }
}

#[test]
fn fate_335_takes_precedence_over_349_when_both_are_present() {
    let mut combined =
        ReplayState::test_from_fixture(&li_fixture(vec![335, 349], 222, current_build()));
    let mut staff_only =
        ReplayState::test_from_fixture(&li_fixture(vec![335], 222, current_build()));

    combined.test_execute_one_card(PlayerSide::P1);
    staff_only.test_execute_one_card(PlayerSide::P1);

    assert_eq!(combined.p2.core.hp, staff_only.p2.core.hp - 3);
    assert_eq!(combined.p1.beng.momentum_limit, 7);
    assert_eq!(combined.p1.beng.momentum, 1);
    assert_eq!(
        (combined.p1.beng.quan_stance, combined.p1.beng.gun_stance),
        (0, 1)
    );
}
