use super::cards_dream_mirage::DreamMirageValue;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 100,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
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

/// P1 slot3 holds 梦•厄劫缠身 (base 369); P2 slot3/slot5 hold rarity-0
/// cards so the 369 opening always takes the AddMengEJieSkipPos branch.
fn fixture() -> BattleFixture {
    let mut p1_cards = vec![original_card(0)];
    p1_cards.resize_with(DECK_SIZE, || original_card(0));
    p1_cards[3] = original_card(10_369);
    let mut p2_cards = vec![original_card(0)];
    p2_cards.resize_with(DECK_SIZE, || original_card(0));
    p2_cards[3] = original_card(7_000_027);
    p2_cards[5] = original_card(7_000_022);
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
            p1: player(p1_cards, DECK_SIZE),
            p2: player(p2_cards, DECK_SIZE),
        },
    }
}

#[test]
fn calamity_skip_mask_repeated_add_same_grid_is_idempotent() {
    let mut state = ReplayState::test_from_fixture(&fixture());
    let opener = original_card(10_369);

    // 梦•厄劫缠身开局 fires once at battle start and again each time
    // 命运轮回 skips its grid; every repeat must keep bit3, never carry
    // into bit4 (8 + 8 must stay 8, not become 16).
    state.apply_dream_mirage_battle_start_opening_with_trigger_grid(
        PlayerSide::P1,
        &opener,
        3,
        369,
        3,
    );
    state.apply_dream_mirage_battle_start_opening_with_trigger_grid(
        PlayerSide::P1,
        &opener,
        3,
        369,
        3,
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P2, DreamMirageValue::CalamitySkipMask),
        1 << 3
    );

    // A different grid still ORs in alongside the existing bit.
    state.apply_dream_mirage_battle_start_opening_with_trigger_grid(
        PlayerSide::P1,
        &opener,
        5,
        369,
        5,
    );
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P2, DreamMirageValue::CalamitySkipMask),
        (1 << 3) | (1 << 5)
    );
}

#[test]
fn calamity_skip_mask_skips_masked_slot_and_clears_only_that_bit() {
    let mut narrow = fixture();
    narrow.players.p2.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&narrow);

    state.modify_dream_mirage_value(PlayerSide::P2, DreamMirageValue::CalamitySkipMask, 1);
    let drawn = state
        .p2
        .draw_next_card(0)
        .expect("masked draw still draws a card");
    assert_eq!(drawn.skipped_slots, vec![0]);
    assert_eq!(drawn.source_slot, 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P2, DreamMirageValue::CalamitySkipMask),
        1 << 3
    );
}
