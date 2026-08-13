use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use crate::original_card_definition_by_id;
use crate::replay::BASIC_ATTACK_ID;
use std::collections::BTreeMap;

fn player(cards: Vec<CardDefinition>, fate_strategies: Vec<i64>, anima: i64) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: Some(0),
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        talent_resonance_id: None,
        fate_strategies,
        fate_strategy_temp_datas: BTreeMap::new(),
        active_slot_count: 1,
        initial_defense: 0,
        initial_anima: anima,
        initial_guard: 0,
        initial_momentum: 12,
        initial_momentum_limit: Some(20),
        initial_agility: 0,
        initial_battle_buffs: BTreeMap::new(),
        permanent_buff_temp_datas: BTreeMap::new(),
        talent_temp_datas: BTreeMap::new(),
        talent_card_params: BTreeMap::new(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        used_ke_yin_cards: Vec::new(),
        cards,
    }
}

fn fixture(card: CardDefinition, fate_strategies: Vec<i64>, anima: i64) -> BattleFixture {
    let basic_attack =
        original_card_definition_by_id(BASIC_ATTACK_ID).expect("missing basic attack");
    let mut p1_cards = vec![card];
    p1_cards.resize(DECK_SIZE, basic_attack.clone());
    let p2_cards = vec![basic_attack; DECK_SIZE];
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
            p1: player(p1_cards, fate_strategies, anima),
            p2: player(p2_cards, Vec::new(), 0),
        },
    }
}

fn attack_damage(anima: i64, after_action: bool) -> (i64, i64) {
    let card = original_card_definition_by_id(10_000_013).expect("missing 崩拳•撼");
    let mut state = ReplayState::test_from_fixture(&fixture(card.clone(), vec![432], anima));
    let kind = super::super::effect_invocation::EffectInvocationKind::Played;
    state.begin_effect_invocation(PlayerSide::P1, &card, &card, &card, 0, 0, kind, true);
    state.set_active_effect_after_action(after_action);
    let damage = state.apply_attack(PlayerSide::P1, card.attack.expect("崩拳•撼 attack"), 0);
    state.end_effect_invocation(PlayerSide::P1, kind);
    (damage, state.p2.core.hp)
}

#[test]
fn fate_432_adds_capped_anima_bonus_before_momentum_factor() {
    // BattleCharacter.cs:11422-11427; anima=11 gives min(11/2, 4)=4.
    // The existing 12 momentum then makes the factor 220%: (10+4)*220%=30.
    assert_eq!(attack_damage(11, false), (30, 70));
}

#[test]
fn fate_432_does_not_apply_in_after_action_window() {
    // The original guard is !HasBuff(BuffType.AfterCardAciton), so the
    // same attack remains 10*220%=22 during after-card hooks.
    assert_eq!(attack_damage(11, true), (22, 78));
}
