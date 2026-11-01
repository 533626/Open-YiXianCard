use super::cards_dream_mirage::{
    DreamMirageValue, SYNTHETIC_ORACLE_WAVE015_CARD_IDS, SYNTHETIC_ORACLE_WAVE016_CARD_IDS,
    SYNTHETIC_ORACLE_WAVE017_CARD_IDS,
};
use super::support::normalized_base_id;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::collections::{BTreeMap, BTreeSet};

fn original_card(id: i64) -> CardDefinition {
    original_config::original_card_definition(id)
        .unwrap_or_else(|| panic!("missing original card {id}"))
}

fn deck(ids: &[i64]) -> Vec<CardDefinition> {
    let mut cards: Vec<_> = ids.iter().copied().map(original_card).collect();
    cards.resize_with(DECK_SIZE, || original_card(0));
    cards
}

fn player(cards: Vec<CardDefinition>, permanent: BTreeMap<String, i64>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 500,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        xian_mo_strategies: Vec::new(),
        xian_mo_temp_datas: Default::default(),
        active_slot_count: 2,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: permanent,
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

fn state_with_permanent(permanent: BTreeMap<String, i64>) -> ReplayState {
    let fixture = BattleFixture {
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
            p1: player(deck(&[395, 0]), permanent),
            p2: player(deck(&[0]), BTreeMap::new()),
        },
    };
    ReplayState::test_from_fixture(&fixture)
}

#[test]
fn dream_mirage_freeze_exactly_seventy_nine_logical_ids() {
    let ids: Vec<_> = SYNTHETIC_ORACLE_WAVE015_CARD_IDS
        .into_iter()
        .chain(SYNTHETIC_ORACLE_WAVE016_CARD_IDS)
        .chain(SYNTHETIC_ORACLE_WAVE017_CARD_IDS)
        .collect();
    assert_eq!(ids.len(), 79);
    assert_eq!(ids.iter().copied().collect::<BTreeSet<_>>().len(), 79);
    for id in ids {
        assert_eq!(normalized_base_id(&original_card(id)), id, "card {id}");
    }
}

#[test]
fn overlapping_ids_fall_through_to_authoritative_later_modules() {
    let mut state = state_with_permanent(BTreeMap::new());
    for id in [7_000_093, 198] {
        let card = original_card(id);
        assert_eq!(
            state.apply_synthetic_oracle_dream_mirage_card_effect(
                PlayerSide::P1,
                &card,
                0,
                false,
                id,
            ),
            None,
            "card {id}",
        );
    }
}

/// Typed rows for the data-driven waves015-017 table: every loop iteration of
/// the migrated test below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `value_total_actual_damage`).
struct BuffZeroCase {
    value: DreamMirageValue,
}

const ORDINARY_BUFF_ZERO_CASES: &[BuffZeroCase] = &[
    BuffZeroCase {
        value: DreamMirageValue::RearMoveCardUsedCount,
    },
    BuffZeroCase {
        value: DreamMirageValue::TotalActualDamage,
    },
    BuffZeroCase {
        value: DreamMirageValue::LostMaxHpEventCount,
    },
    BuffZeroCase {
        value: DreamMirageValue::SnakeCardUsedCount,
    },
    BuffZeroCase {
        value: DreamMirageValue::TotalSharpnessGained,
    },
    BuffZeroCase {
        value: DreamMirageValue::TotalWaterMomentumGained,
    },
];

fn buff_zero_state() -> ReplayState {
    let permanent = [
        ("1", 3),
        ("6", 2),
        ("10", 4),
        ("634", 2),
        ("644", 7),
        ("672", 3),
        ("703", 1),
        ("721", 8),
        ("722", 6),
    ]
    .into_iter()
    .map(|(key, value)| (key.to_string(), value))
    .collect();
    state_with_permanent(permanent)
}

fn run_buff_zero_case(case: &BuffZeroCase) {
    let value = case.value;
    let state = buff_zero_state();
    assert_eq!(state.dream_mirage_value(PlayerSide::P1, value), 0);
}

crate::data_driven_cases! {
    table: ORDINARY_BUFF_ZERO_CASES,
    run: run_buff_zero_case,
    suite: "replay::tests_synthetic_oracle_waves015_017::ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime",
    valid: ordinary_battle_buff_ids_in_permanent_temp_case_table_is_valid,
    cases: [
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_rear_move_card_used_count => 0,
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_total_actual_damage => 1,
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_lost_max_hp_event_count => 2,
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_snake_card_used_count => 3,
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_total_sharpness_gained => 4,
        ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_value_total_water_momentum_gained => 5,
    ]
}

#[test]
fn ordinary_battle_buff_ids_in_permanent_temp_do_not_seed_runtime_core_resources() {
    let state = buff_zero_state();
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.p1.elements.water_momentum, 0);
}

#[test]
fn hou_zhao_count_uses_printed_marker_and_yellow_bird_reads_two() {
    let mut state = state_with_permanent(BTreeMap::new());
    // Buff634 is CardActionBase.OnAfterExecuted's count of the printed
    // `[后招]：` marker.  The two replay predecessors contribute; 黄雀在后
    // (4000066) and 梦•黄雀在后 itself (4000075) do not.
    for id in [4000033, 4000101, 4000066, 4000075] {
        let card = original_card(id);
        assert_eq!(
            original_config::original_card_desc_contains_rear_move(&card),
            matches!(id, 4000033 | 4000101),
            "card {id} marker classification",
        );
        state.complete_dream_mirage_card_classification(PlayerSide::P1, &card);
    }
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::RearMoveCardUsedCount),
        2,
    );

    let yellow_bird = original_card(4_040_075);
    let hp_before = state.p2.core.hp;
    assert_eq!(
        state.apply_synthetic_oracle_dream_mirage_card_effect(
            PlayerSide::P1,
            &yellow_bird,
            0,
            false,
            4_000_075,
        ),
        Some(true),
    );
    // 4040075 is 5 attack plus two appended 5-attack segments.
    assert_eq!(state.p2.core.hp, hp_before - 15);
}

#[test]
fn snake_shadow_and_action_limit_use_shared_runtime_state() {
    let mut state = state_with_permanent(BTreeMap::new());
    state.modify_dream_mirage_value(PlayerSide::P2, DreamMirageValue::SnakeShadow, 2);
    state.add_actor_negative_status(PlayerSide::P2, 100, 3);
    assert_eq!(state.p2.status.internal_injury, 5);

    assert_eq!(state.dream_mirage_action_again_limit(PlayerSide::P1), 1);
    assert_eq!(
        state.increase_dream_mirage_action_again_limit(PlayerSide::P1, 10, 8),
        7,
    );
    assert_eq!(state.dream_mirage_action_again_limit(PlayerSide::P1), 8);
}

#[test]
fn empty_temporary_selection_does_not_fall_back_to_basic_attack() {
    let mut state = state_with_permanent(BTreeMap::new());
    let card = original_card(395);
    let hp = state.p2.core.hp;
    assert_eq!(
        state
            .apply_synthetic_oracle_dream_mirage_card_effect(PlayerSide::P1, &card, 0, false, 395,),
        Some(false),
    );
    assert_eq!(state.p2.core.hp, hp);
}
