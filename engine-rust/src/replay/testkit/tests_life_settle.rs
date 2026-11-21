use super::*;
use crate::fixture::{BattleFixture, FinalLife, FixturePlayer, FixtureSource};
use crate::model::{CardDefinition, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    original_card(BASIC_ATTACK_ID)
}

fn deck() -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![basic_attack()], basic_attack())
}

fn player() -> FixturePlayer {
    super::test_support::make_player(deck(), 5, 50, Some(0), 8, Some(6))
}

fn player_with(first: CardDefinition) -> FixturePlayer {
    let cards = super::test_support::fill_deck(vec![first], basic_attack());
    super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6))
}

fn quad_source(round: i64, game_mode: i64) -> FixtureSource {
    FixtureSource {
        round: Some(round),
        game_mode: Some(game_mode),
        ..FixtureSource::default()
    }
}

#[test]
fn cini_grants_both_sides_once() {
    let card = original_card(5_000_006);
    let mut state = ReplayState::test_from_fixture(&super::test_support::default_fixture(
        player_with(card.clone()),
        player(),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.music.compassionate_tune, 4);
    assert_eq!(state.p2.music.compassionate_tune, 4);
    assert_eq!(state.p1.music.music_cards_played, 2);
    let star2 = original_card(5_010_006);
    state.test_apply_card_effect(PlayerSide::P2, &star2, 0);
    assert_eq!(state.p1.music.compassionate_tune, 4);
    assert_eq!(state.p2.music.compassionate_tune, 4);
    assert_eq!(state.p2.music.music_cards_played, 2);
}

#[test]
fn full_battle_quad_is_internally_consistent() {
    let mut p1 = player();
    let mut p2 = player();
    p1.last_round_life = Some(100);
    p2.last_round_life = Some(90);
    let mut fixture = super::test_support::default_fixture(p1, p2);
    fixture.source = Some(quad_source(12, 2));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    let final_life = summary.final_life.clone().expect("quad computable");
    let dealt = summary.life_damage.expect("quad computable");
    assert!(summary.life_detail.is_some());
    match summary.winner_side {
        PlayerSide::P1 => {
            assert!(dealt >= 0);
            assert_eq!(final_life.p1, 100);
            assert_eq!(final_life.p2, 90 - dealt);
        }
        PlayerSide::P2 => {
            assert!(dealt <= 0);
            assert_eq!(final_life.p1, 100 - dealt.abs());
            assert_eq!(final_life.p2, 90);
        }
    }
}

#[test]
fn civet_heals_temp_life_unconditionally() {
    let card = original_card(433);
    let mut p2 = player();
    p2.is_mirror = true;
    let mut state = ReplayState::test_from_fixture(&super::test_support::default_fixture(
        player_with(card.clone()),
        p2,
    ));
    let before = state.p1.core.temp_life;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.temp_life, before + 2);
    let card4 = original_card(434);
    let before4 = state.p1.core.temp_life;
    state.test_apply_card_effect(PlayerSide::P1, &card4, 0);
    assert_eq!(state.p1.core.temp_life, before4);
    let mirror_before = state.p2.core.temp_life;
    let civet = original_card(433);
    state.test_apply_card_effect(PlayerSide::P2, &civet, 0);
    assert_eq!(state.p2.core.temp_life, mirror_before + 2);
}

#[test]
fn pangu_fu_costs_temp_on_lethal_once() {
    let mut state =
        ReplayState::test_from_fixture(&super::test_support::default_fixture(player(), player()));
    let before = state.p2.core.temp_life;
    state.test_configure_p2(|actor| {
        actor.core.hp = 0;
        actor.mirage_ronghui.pan_gu_fu = 20;
    });
    let winner = state.death_winner();
    assert_eq!(winner, Some(PlayerSide::P1));
    assert_eq!(state.p2.core.temp_life, before - 20);
    assert_eq!(state.p2.mirage_ronghui.pan_gu_fu, 0);
    let again = state.death_winner();
    assert_eq!(again, Some(PlayerSide::P1));
    assert_eq!(state.p2.core.temp_life, before - 20);
}

#[test]
fn temp_life_has_no_floor() {
    let mut state =
        ReplayState::test_from_fixture(&super::test_support::default_fixture(player(), player()));
    state.test_configure_p2(|actor| {
        actor.core.temp_life = 1;
    });
    state.modify_temp_life(PlayerSide::P2, -4);
    assert_eq!(state.p2.core.temp_life, -3);
}

#[test]
fn battle_end_intents_report_plays_and_pet_counters() {
    let rat = original_card(99_000_203);
    let huan = original_card(99_000_212);
    let fixture = super::test_support::make_fixture(
        player_with(rat.clone()),
        player_with(huan.clone()),
        PlayerSide::P1,
        1,
        0,
        64,
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert!(
        state.evaluation_error.is_none(),
        "{:?}",
        state.evaluation_error
    );
    assert!(summary
        .battle_end_intents
        .p1
        .played_base_ids
        .contains(&99_000_203));
    assert!(summary
        .battle_end_intents
        .p2
        .played_base_ids
        .contains(&99_000_212));
    assert!(summary.battle_end_intents.p1.jin_mao_shu_draws >= 1);
    assert!(summary.battle_end_intents.p2.san_wei_huan_replace >= 1);
    assert_eq!(
        summary.battle_end_intents.p1.played_base_ids.len(),
        summary.battle_end_intents.p1.played_card_stars.len()
    );
    let pos = summary
        .battle_end_intents
        .p1
        .played_base_ids
        .iter()
        .position(|&id| id == 99_000_203)
        .unwrap();
    assert_eq!(summary.battle_end_intents.p1.played_card_stars[pos], 0);
}

#[test]
fn mei_kai_er_du_marks_next_settle_card_doubled() {
    let cards = super::test_support::fill_deck(
        vec![original_card(4_000_041), original_card(434)],
        basic_attack(),
    );
    let p1 = super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6));
    let fixture = super::test_support::make_fixture(
        p1,
        player(),
        PlayerSide::P1,
        1,
        0,
        64,
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert!(
        state.evaluation_error.is_none(),
        "{:?}",
        state.evaluation_error
    );
    assert!(summary
        .battle_end_intents
        .p1
        .played_base_ids
        .contains(&434));
    assert!(summary
        .battle_end_intents
        .p1
        .doubled_effect_base_ids
        .contains(&434));
}

#[test]
fn quad_absent_without_inputs_and_triple_still_matches() {
    let mut fixture: BattleFixture = super::test_support::default_fixture(player(), player());
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert_eq!(summary.final_life, None);
    assert_eq!(summary.life_damage, None);
    assert_eq!(summary.life_detail, None);
    fixture.expected.winner_side = summary.winner_side;
    fixture.expected.actor_turn_count = summary.actor_turn_count;
    fixture.expected.hp_delta_p1_minus_p2 = summary.hp_delta_p1_minus_p2;
    assert!(summary.matches_fixture(&fixture));
}

#[cfg(feature = "private-fixtures")]
#[test]
fn real_battles_reproduce_observed_life_ends() {
    for name in [
        "mirror_34546000_quad_base_round-01.json",
        "mirror_34546000_quad_base_round-02.json",
        "mirror_34546000_quad_cini_round-05.json",
        "mirror_34546000_quad_mirror_round-01.json",
        "mirror_34546000_quad_surv_round-13.json",
    ] {
        let fixture = crate::fixture::load_fixture_file(format!("tests/data-driven/{name}"))
            .unwrap_or_else(|_| panic!("load {name}"));
        let summary =
            crate::replay::run_replay_fixture(&fixture).unwrap_or_else(|_| panic!("run {name}"));
        assert!(
            summary.matches_fixture(&fixture),
            "{name}: summary={summary:?} expected={:?}",
            fixture.expected
        );
        assert!(summary.final_life.is_some(), "{name}");
        assert!(summary.life_damage.is_some(), "{name}");
    }
}

#[test]
fn matches_fixture_enforces_quad_when_expected() {
    let mut p1 = player();
    let mut p2 = player();
    p1.last_round_life = Some(100);
    p2.last_round_life = Some(90);
    let mut fixture = super::test_support::default_fixture(p1, p2);
    fixture.source = Some(quad_source(12, 2));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    fixture.expected.winner_side = summary.winner_side;
    fixture.expected.actor_turn_count = summary.actor_turn_count;
    fixture.expected.hp_delta_p1_minus_p2 = summary.hp_delta_p1_minus_p2;
    fixture.expected.final_life = summary.final_life.clone();
    fixture.expected.life_damage = summary.life_damage;
    assert!(summary.matches_fixture(&fixture));
    fixture.expected.final_life = Some(FinalLife { p1: 1, p2: 2 });
    assert!(!summary.matches_fixture(&fixture));
    fixture.expected.final_life = summary.final_life.clone();
    fixture.expected.life_damage = Some(summary.life_damage.unwrap_or(0) + 1);
    assert!(!summary.matches_fixture(&fixture));
}
