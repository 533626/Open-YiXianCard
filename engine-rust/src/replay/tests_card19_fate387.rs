use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

fn player(cards: Vec<CardDefinition>, initial_anima: i64) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 1,
        initial_defense: 0,
        initial_anima,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards,
    }
}

fn fixture() -> BattleFixture {
    let mut p1_cards = vec![original_card(19)];
    p1_cards.resize_with(DECK_SIZE, || original_card(0));
    let mut p2_cards = vec![original_card(0)];
    p2_cards.resize_with(DECK_SIZE, || original_card(0));
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
        players: FixturePlayers {
            p1: player(p1_cards, 3),
            p2: player(p2_cards, 0),
        },
    }
}

#[test]
fn card19_fate387_moves_remaining_anima_to_jianqi_and_settles_same_card() {
    let mut fixture = fixture();
    fixture.players.p1.fate_strategies = vec![101, 387];
    let mut state = ReplayState::test_from_fixture(&fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    // Card 19's printed attack is 7. Fate 101 contributes JianQi 2 and
    // Fate 387 moves the remaining 3 anima into JianQi; all 5 settle now.
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.sword.sword_energy, 5);
    assert_eq!(state.p2.core.hp, 88);
}

#[test]
fn temporary_card19_fate387_also_settles_remaining_anima() {
    let mut fixture = fixture();
    fixture.players.p1.fate_strategies = vec![387];
    let mut state = ReplayState::test_from_fixture(&fixture);
    let card = state.test_actor_card(PlayerSide::P1, 0);

    assert!(!state.apply_temporary_card_effect(PlayerSide::P1, &card, 0));
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.sword.sword_energy, 3);
    assert_eq!(state.p2.core.hp, 90);
}
