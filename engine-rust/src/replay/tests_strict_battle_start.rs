use super::*;
use crate::fixture::{
    BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers, FixtureSource,
};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

fn original_card(card_id: i64) -> CardDefinition {
    original_card_definition_by_id(card_id)
        .unwrap_or_else(|| panic!("missing original card {card_id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(0)
}

fn full_deck(first: CardDefinition, second: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![first, second];
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        talent_resonance_id: None,
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        used_ke_yin_cards: Vec::new(),
        cards,
    }
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
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

fn missing_opponent_grid_fixture() -> BattleFixture {
    fixture(
        player(vec![basic_attack(), original_card(369)], 2),
        player(vec![basic_attack()], 1),
    )
}

fn missing_opening_config_fixture() -> BattleFixture {
    let mut star_chess = original_card(389);
    star_chess.rarity = Some(99);
    fixture(
        player(full_deck(star_chess, basic_attack()), 2),
        player(full_deck(basic_attack(), basic_attack()), 2),
    )
}

fn assert_all_strict_replay_surfaces_reject(fixture: &BattleFixture, expected: &str) {
    let errors = [
        (
            "strict constructor",
            ReplayState::from_fixture(fixture, true)
                .unwrap_err()
                .to_string(),
        ),
        (
            "summary",
            run_replay_fixture(fixture).unwrap_err().to_string(),
        ),
        (
            "raw events",
            run_replay_fixture_with_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "parity events",
            run_replay_fixture_with_parity_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "detailed events",
            run_replay_fixture_with_detailed_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "fallible evaluation",
            evaluate_replay_fixture_fallible(fixture).unwrap_err(),
        ),
        (
            "fallible events",
            run_replay_fixture_with_events_fallible(fixture).unwrap_err(),
        ),
    ];

    for (surface, error) in errors {
        assert!(
            error.contains(expected),
            "{surface} returned an unrelated error: {error}"
        );
        assert!(
            error.contains("turn=0"),
            "{surface} continued beyond battle-start: {error}"
        );
    }
}

#[test]
fn every_public_replay_surface_rejects_battle_start_missing_grid_decision() {
    assert_all_strict_replay_surfaces_reject(
        &missing_opponent_grid_fixture(),
        "card:369:opponent same-grid card",
    );
}

#[test]
fn every_public_replay_surface_rejects_battle_start_missing_card_config() {
    assert_all_strict_replay_surfaces_reject(
        &missing_opening_config_fixture(),
        "card:389:opening replacement definition",
    );
}

#[test]
fn every_public_replay_surface_rejects_an_unknown_original_build() {
    let retired_build = super::original_build_profile::latest_retired_steam_build()
        .expect("profile contract retains an audited retired build as rejection sample");
    let mut unknown = fixture(
        player(full_deck(basic_attack(), basic_attack()), 1),
        player(full_deck(basic_attack(), basic_attack()), 1),
    );
    unknown.source = Some(FixtureSource {
        steam_build: Some(retired_build.to_string()),
        ..FixtureSource::default()
    });

    assert_all_strict_replay_surfaces_reject(
        &unknown,
        &format!("unsupported original Steam build {retired_build}"),
    );
}

#[test]
fn fixture_file_entry_rejects_opening_error_before_the_cli_can_run_a_turn() {
    let fixture = missing_opening_config_fixture();
    fixture
        .validate()
        .expect("file fixture must be structurally valid");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "yixian-strict-battle-start-{}-{nonce}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        serde_json::to_vec(&fixture).expect("serialize fixture"),
    )
    .expect("write temporary fixture");

    let result = run_replay_fixture_file(&path);
    fs::remove_file(&path).expect("remove temporary fixture");

    let error = result.expect_err("CLI fixture entry must reject opening config gaps");
    assert!(error
        .to_string()
        .contains("card:389:opening replacement definition"));
    assert!(error.to_string().contains("turn=0"));
}
