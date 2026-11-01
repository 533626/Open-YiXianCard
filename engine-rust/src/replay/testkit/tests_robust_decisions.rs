//! 稳健模式（`FixtureSource::robust_decisions`，分析专用）：无录像带的合成决策里，
//! 范围数值取 ⌊(min+max)/2⌋；无卦象的概率事件对 p1 悲观（p1 判失败 = 99，p2 判成功 = 0）。
use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::{FixtureExpected, FixtureSource};
use crate::model::PlayerSide;

fn fixture_with(p1: CardDefinition, p2: CardDefinition) -> BattleFixture {
    let mut fixture = minimal_fixture(
        filler_cards(p1),
        filler_cards(p2),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.source = Some(FixtureSource {
        synthetic_decision_fallback_seed: Some(17),
        robust_decisions: true,
        ..FixtureSource::default()
    });
    fixture
}

fn random_attack_card() -> CardDefinition {
    let mut card = test_card(1_000_001, 1_000_001, "random-range-probe");
    card.attack = Some(2);
    card.random_attack = Some(5);
    card.attack_count = Some(2);
    card
}

fn dragonfly_skims_water() -> CardDefinition {
    let mut card = test_card(4_000_035, 4_000_035, "蜻蜓点水");
    card.attack = Some(5);
    card.attack_count = Some(1);
    card.other_params = vec![10].into();
    card
}

#[test]
fn robust_random_range_takes_floor_mean_for_both_sides() {
    let card = random_attack_card();
    let fixture = fixture_with(card.clone(), card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    state.test_apply_card_effect(PlayerSide::P2, &card, 0);
    let picks: Vec<_> = state
        .decision_events
        .iter()
        .map(|e| (e.side, e.provider, e.selected_option))
        .collect();
    assert_eq!(
        picks,
        vec![
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(3)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(3)),
        ]
    );
}

#[test]
fn robust_percent_roll_is_pessimistic_for_p1() {
    let card = dragonfly_skims_water();
    let fixture = fixture_with(card.clone(), card.clone());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    state.test_apply_card_effect(PlayerSide::P2, &card, 0);
    let rolls: Vec<_> = state
        .decision_events
        .iter()
        .filter(|e| e.decision_kind == ReplayDecisionKind::PercentRoll)
        .map(|e| (e.side, e.provider, e.selected_option))
        .collect();
    assert_eq!(
        rolls,
        vec![
            (PlayerSide::P1, ReplayDecisionProvider::Robust, Some(99)),
            (PlayerSide::P2, ReplayDecisionProvider::Robust, Some(0)),
        ]
    );
}

#[test]
fn robust_mode_off_keeps_seeded_synthetic_choices() {
    let card = random_attack_card();
    let mut fixture = fixture_with(card.clone(), basic_attack_test_card());
    fixture.source.as_mut().expect("source").robust_decisions = false;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert!(state
        .decision_events
        .iter()
        .all(|e| e.provider == ReplayDecisionProvider::SeededSynthetic));
}
