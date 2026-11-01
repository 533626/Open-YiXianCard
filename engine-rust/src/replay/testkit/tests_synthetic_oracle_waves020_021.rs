use super::cards_dream_mirage::DreamMirageValue;
use super::support::normalized_base_id;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide};
use std::collections::BTreeMap;
use std::collections::BTreeSet;

const LOGICAL_IDS: [i64; 28] = [
    177, 178, 179, 181, 185, 186, 187, 188, 189, 190, 191, 192, 193, 194, 195, 196, 198, 199, 201,
    202, 203, 206, 207, 208, 209, 210, 389, 390,
];

#[test]
fn ronghui_load_all_seventy_six_physical_rows() {
    let mut rows = vec![177, 178, 179, 181];
    for base_id in LOGICAL_IDS
        .iter()
        .copied()
        .filter(|id| *id != 177 && *id != 178 && *id != 179 && *id != 181)
    {
        rows.extend([base_id, base_id + 10_000, base_id + 20_000]);
    }
    assert_eq!(rows.len(), 76);
    assert_eq!(rows.iter().copied().collect::<BTreeSet<_>>().len(), 76);
    for card_id in rows {
        let card = original_config::original_card_definition(card_id)
            .unwrap_or_else(|| panic!("missing original card {card_id}"));
        assert!(
            LOGICAL_IDS.contains(&normalized_base_id(&card)),
            "card {card_id}"
        );
    }
}

fn basic_attack() -> CardDefinition {
    original_config::original_card_definition(0).expect("missing basic attack")
}

fn original_card(id: i64) -> CardDefinition {
    original_config::original_card_definition(id)
        .unwrap_or_else(|| panic!("missing original card {id}"))
}

fn deck(ids: &[i64]) -> Vec<CardDefinition> {
    super::test_support::resize_deck(
        ids.iter().map(|id| original_card(*id)).collect(),
        basic_attack(),
    )
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    let mut permanent_buff_temp_datas = BTreeMap::new();
    permanent_buff_temp_datas.insert("10024".to_string(), 100);
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
        active_slot_count,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas,
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

fn fixture(
    p1_ids: &[i64],
    p1_active: usize,
    p2_ids: &[i64],
    p2_active: usize,
    decisions: Vec<i64>,
) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: decisions,
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
            p1: player(deck(p1_ids), p1_active),
            p2: player(deck(p2_ids), p2_active),
        },
    }
}

fn state(p1_ids: &[i64], p1_active: usize) -> ReplayState {
    ReplayState::test_from_fixture(&fixture(p1_ids, p1_active, &[0], 1, Vec::new()))
}

fn apply_body(state: &mut ReplayState, card_id: i64, slot: usize, was_used: bool) {
    let card = original_card(card_id);
    let base_id = normalized_base_id(&card);
    assert!(state
        .apply_synthetic_oracle_ronghui_card_effect(PlayerSide::P1, &card, slot, was_used, base_id,)
        .is_some());
}

#[test]
fn direct_resource_bodies_preserve_original_ordering() {
    let mut growth = state(&[178], 1);
    growth.p1.core.hp = 400;
    apply_body(&mut growth, 178, 0, false);
    assert_eq!(growth.p1.core.hp, 400);
    assert_eq!(growth.p1.core.max_hp, 505);
    assert_eq!(growth.p1.ronghui.five_emperors_upgrade, 2);

    let mut spirit = state(&[20_185], 1);
    apply_body(&mut spirit, 20_185, 0, false);
    spirit.gain_anima(PlayerSide::P1, 8);
    assert_eq!(spirit.p1.core.defense, 40);

    let mut sunflower = state(&[20_188, 1_020_007, 1_020_019, 1_020_020], 4);
    apply_body(&mut sunflower, 20_188, 0, false);
    assert_eq!(sunflower.p1.core.anima, 9);

    let mut forge = state(&[20_207], 1);
    forge.p1.core.hp = 400;
    forge.p1.core.physique_limit = 5;
    apply_body(&mut forge, 20_207, 0, false);
    assert_eq!(forge.p1.core.physique, 8);
    assert_eq!(forge.p1.core.physique_limit, 8);
    assert_eq!(forge.p1.core.max_hp, 508);
    assert_eq!(forge.p1.core.hp, 403);
    assert_eq!(forge.p1.core.attack_bonus, 4);
}

#[test]
fn earth_fiend_shatters_then_converts_exact_hp_loss_to_defense() {
    let mut state = state(&[20_189, 20_000], 2);
    state.p2.core.defense = 2;
    apply_body(&mut state, 20_189, 0, false);
    let hp_lost = state.apply_attack(PlayerSide::P1, 9, 1);
    assert_eq!(hp_lost, 8);
    assert_eq!(state.p1.core.defense, 8);
    assert_eq!(state.p1.ronghui.earth_fiend_defense, 2);
    assert_eq!(state.p1.turn.next_attack_shatter_defense, 2);
}

#[test]
fn turn_hooks_cover_thunder_two_polarity_and_momentum_formation() {
    let mut thunder = state(&[20_192], 1);
    thunder.p1.core.hp = 400;
    apply_body(&mut thunder, 20_192, 0, false);
    assert_eq!((thunder.p1.core.hp, thunder.p1.core.max_hp), (420, 520));
    thunder.apply_ronghui_turn_end(PlayerSide::P1);
    assert_eq!(thunder.p2.core.hp, 486);

    let mut polarity = state(&[20_194], 1);
    polarity.p1.core.hp = 400;
    polarity.p1.core.anima = 8;
    polarity.p1.astrology.hexagram = 2;
    apply_body(&mut polarity, 20_194, 0, false);
    polarity.apply_ronghui_turn_end(PlayerSide::P1);
    assert_eq!(polarity.p1.core.hp, 412);
    assert_eq!(polarity.p1.core.defense, 12);
    assert_eq!(polarity.p1.ronghui.two_polarity_vajra, 3);

    let mut momentum = state(&[20_208], 1);
    apply_body(&mut momentum, 20_208, 0, false);
    momentum.apply_ronghui_turn_start(PlayerSide::P1);
    assert_eq!(momentum.p1.core.anima, 2);
    assert_eq!(momentum.p1.beng.momentum, 2);
    assert_eq!(momentum.p1.beng.momentum_limit, 10);
    assert_eq!(momentum.p1.ronghui.momentum_formation, 3);
}

#[test]
fn reverse_gu_moves_every_debuff_layer_before_adding_flat_attack() {
    let mut state = state(&[20_210], 1);
    state.p2.status.internal_injury = 3;
    state.p2.status.weakness = 2;
    apply_body(&mut state, 20_210, 0, false);
    assert_eq!(state.p2.core.hp, 478);
    let before = state.p2.core.hp;
    state.apply_attack(PlayerSide::P1, 9, 0);
    // The transferred Weakness applies to this same attack after the +5 flat bonus.
    assert_eq!(before - state.p2.core.hp, 8);
    assert_eq!(state.p1.status.internal_injury, 3);
    assert_eq!(state.p1.status.weakness, 2);
    assert_eq!(state.p2.status.internal_injury, 0);
    assert_eq!(state.p2.status.weakness, 0);
}

#[test]
fn card_209_mirrors_exact_positive_status_delta_once() {
    let mut state = state(&[20_209], 1);
    state.p1.core.hp = 400;
    apply_body(&mut state, 20_209, 0, false);
    state.add_actor_negative_status(PlayerSide::P1, 100, 2);
    assert_eq!(state.p1.status.internal_injury, 2);
    assert_eq!(state.p2.status.internal_injury, 2);
    assert_eq!(state.p1.ronghui.snow_lotus_mirror, 1);
}

#[test]
fn opening_effects_seed_alchemy_water_and_adjacent_star_replacements() {
    let alchemy = state(&[181], 1);
    assert_eq!(alchemy.p2.ronghui.alchemy_pot, 2);

    let water = state(&[20_203], 1);
    assert_eq!(water.p1.core.anima, 1);
    assert_eq!(water.p1.elements.water_momentum, 1);

    let stars = state(&[2_020_010, 0, 20_389, 0], 4);
    assert!(stars.p1.astrology.star_slots.contains(&1));
    assert!(stars.p1.astrology.star_slots.contains(&3));
    assert_eq!(stars.p1.deck.slots[1].card.id, 4_020_038);
    assert_eq!(stars.p1.deck.slots[3].card.id, 4_020_038);
}

#[test]
fn before_card_transform_handles_upgrade_downgrade_and_previous_card_copy() {
    let mut upgrade = state(&[178, 0], 2);
    upgrade.p1.ronghui.five_emperors_upgrade = 1;
    let drawn = DrawnCard {
        source_slot: 1,
        card: basic_attack().into(),
        fallback_basic_attack: false,
        skipped_slots: Vec::new(),
        skipped_opening_slots: Vec::new(),
        fate_398_skipped_fifth_grid: false,
    };
    let mut upgraded = drawn;
    upgrade.apply_ronghui_five_emperors_upgrade_transform(PlayerSide::P1, &mut upgraded);
    assert_eq!(upgraded.card.id, 10_000);

    let mut alchemy = state(&[20_000], 1);
    alchemy.p1.ronghui.alchemy_pot = 1;
    let mut lowered = DrawnCard {
        source_slot: 0,
        card: original_card(20_000).into(),
        fallback_basic_attack: false,
        skipped_slots: Vec::new(),
        skipped_opening_slots: Vec::new(),
        fate_398_skipped_fifth_grid: false,
    };
    alchemy.apply_ronghui_alchemy_pot_transform(PlayerSide::P1, &mut lowered);
    assert_eq!(lowered.card.id, 10_000);

    let mut tune = state(&[20_206, 0], 2);
    tune.p1.ronghui.free_and_easy_tune = 1;
    let mut repeated = DrawnCard {
        source_slot: 1,
        card: basic_attack().into(),
        fallback_basic_attack: false,
        skipped_slots: Vec::new(),
        skipped_opening_slots: Vec::new(),
        fate_398_skipped_fifth_grid: false,
    };
    tune.apply_ronghui_free_and_easy_tune_transform(PlayerSide::P1, &mut repeated);
    assert_eq!(repeated.card.id, 20_206);
}

#[test]
fn queue_hooks_skip_last_field_and_jump_to_next_star_chess_card() {
    let mut last = state(&[20_000, 20_202], 2);
    last.p1.deck.queue = vec![1, 0];
    let drawn = last.p1.draw_next_card(0).expect("draw after 202 skip");
    assert_eq!(drawn.source_slot, 0);
    assert_eq!(drawn.skipped_slots, vec![1]);
    assert!(drawn.skipped_opening_slots.is_empty());

    let mut jump = state(&[0, 20_196, 20_203, 4_020_011], 4);
    jump.p1.ronghui.star_chess_jump = 1;
    jump.p1.deck.queue = vec![2, 3, 0, 1];
    let drawn = jump.p1.draw_next_card(0).expect("draw after star jump");
    assert_eq!(drawn.source_slot, 3);
    assert_eq!(drawn.skipped_slots, vec![2]);
    assert_eq!(drawn.skipped_opening_slots, vec![2]);
    assert_eq!(jump.p1.ronghui.star_chess_jump, 0);
}

#[test]
fn temporary_copy_cards_execute_without_leaking_recursion_guards() {
    let mut fuxi =
        ReplayState::test_from_fixture(&fixture(&[177, 0], 2, &[20_000, 0], 2, Vec::new()));
    apply_body(&mut fuxi, 177, 0, false);
    assert_eq!(fuxi.p2.core.hp, 491);
    assert_eq!(fuxi.p2.astrology.star_chess_break, 1);
    assert_eq!(fuxi.p1.ronghui.fu_xi_copy_guard, 0);

    let mut bell = state(&[20_000, 179], 2);
    bell.p1.deck.slots[0].used = true;
    apply_body(&mut bell, 179, 1, false);
    assert_eq!(bell.p2.core.hp, 491);
    assert_eq!(bell.p1.ronghui.dong_huang_copy_guard, 0);

    let mut frenzy = ReplayState::test_from_fixture(&fixture(&[20_186], 1, &[0], 1, vec![20_002]));
    frenzy.p1.core.hp = 400;
    apply_body(&mut frenzy, 20_186, 0, false);
    assert_eq!(frenzy.p1.core.hp, 408);
    assert_eq!(frenzy.p1.core.defense, 8);
    assert!(frenzy.p2.core.hp < 500);
}

#[test]
fn card_179_refreshes_copied_cheng_xin_from_the_copier_profile() {
    let mut battle = fixture(&[19, 179], 2, &[0], 1, Vec::new());
    battle.players.p1.talents = vec![92, 20_093, 30_094, 20_095, 30_096];
    battle.players.p1.talent_temp_datas.insert("92".into(), 14);
    let mut copy = ReplayState::test_from_fixture(&battle);
    copy.p1.deck.slots[0].used = true;

    // Card_179.cs calls RefreshSpecialCard before the copied card's
    // ExecuteEffect; Card_19.cs therefore restores 20093's defense plus
    // 30096's Wuji increment on the temporary Card 19.
    apply_body(&mut copy, 179, 1, false);

    assert_eq!(copy.p1.core.defense, 10);
}

#[test]
fn temporary_semantics_ronghui_reads_physical_had_used_and_completes_before_settlement() {
    let mut first_use = ReplayState::test_from_fixture(&fixture(&[177], 1, &[12], 1, Vec::new()));
    apply_body(&mut first_use, 177, 0, false);
    assert_eq!(first_use.p1.turn.extra_actions, 0);
    assert!(first_use.p1.deck.slots[0].used);

    let mut repeated = ReplayState::test_from_fixture(&fixture(&[177], 1, &[12], 1, Vec::new()));
    repeated.p1.deck.slots[0].used = true;
    apply_body(&mut repeated, 177, 0, true);
    assert_eq!(repeated.p1.turn.extra_actions, 1);

    let mut damaging_copy =
        ReplayState::test_from_fixture(&fixture(&[177], 1, &[0], 1, Vec::new()));
    apply_body(&mut damaging_copy, 177, 0, false);
    assert_eq!(damaging_copy.p2.core.hp, 497);
    assert_eq!(
        damaging_copy.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalActualDamage,),
        3,
    );
}

#[test]
fn temporary_semantics_card_179_splits_physical_effective_and_invocation_identity() {
    let mut rear_move = state(&[12, 179], 2);
    rear_move.p1.deck.slots[0].used = true;
    apply_body(&mut rear_move, 179, 1, false);
    assert_eq!(rear_move.p1.turn.extra_actions, 1);
    assert!(rear_move.p1.deck.slots[1].used);
    assert_eq!(rear_move.p1.deck.slots[1].card.id, 179);

    let mut formation = state(&[213, 179], 2);
    formation.p1.deck.slots[0].used = true;
    apply_body(&mut formation, 179, 1, false);
    assert_eq!(formation.p2.core.hp, 494);
    assert_eq!(formation.p1.core.defense, 9);
    assert!(formation.p1.deck.slots[1].used);
    assert_eq!(formation.p1.deck.slots[1].card.id, 179);
}

#[test]
fn qin_cards_count_body_and_effective_career_completion_separately() {
    let mut compassion = state(&[5_000_006], 1);
    compassion.test_execute_one_card(PlayerSide::P1);
    assert_eq!(compassion.p1.music.music_cards_played, 2);

    let mut dream_tune = state(&[340], 1);
    dream_tune.test_execute_one_card(PlayerSide::P1);
    assert_eq!(dream_tune.p1.music.music_cards_played, 1);
}

#[test]
fn string_harmony_freezes_its_counter_and_reads_physical_directional_neighbor_career() {
    let mut no_prior_tune = state(&[5_000_014, 0], 2);
    assert!(!no_prior_tune.test_execute_one_card(PlayerSide::P1));
    assert_eq!(no_prior_tune.p1.music.music_cards_played, 2);

    let mut next_fused_tune = state(&[5_000_014, 158], 2);
    assert!(next_fused_tune.test_execute_one_card(PlayerSide::P1));

    let mut reverse = state(&[5_000_014, 0, 0, 0, 0, 0, 0, 158], 8);
    reverse.p1.fate.reverse_card_direction = 1;
    assert!(reverse.test_execute_one_card(PlayerSide::P1));
}

#[test]
fn spirit_harp_body_guard_and_temporary_qin_copy_keep_distinct_counts() {
    let mut successful = ReplayState::test_from_fixture(&fixture(&[177], 1, &[0], 1, Vec::new()));
    successful.test_execute_one_card(PlayerSide::P1);
    assert_eq!(successful.p1.music.music_cards_played, 2);

    let mut copied_tune =
        ReplayState::test_from_fixture(&fixture(&[177], 1, &[5_000_006], 1, Vec::new()));
    copied_tune.test_execute_one_card(PlayerSide::P1);
    assert_eq!(copied_tune.p1.music.music_cards_played, 4);

    let mut guarded = ReplayState::test_from_fixture(&fixture(&[177], 1, &[0], 1, Vec::new()));
    guarded.p1.ronghui.fu_xi_copy_guard = 1;
    guarded.test_execute_one_card(PlayerSide::P1);
    assert_eq!(guarded.p1.music.music_cards_played, 1);
    assert_eq!(guarded.p1.ronghui.fu_xi_copy_guard, 1);
}

#[test]
fn card_179_shortage_adds_normal_anima_and_its_six_charge_qi() {
    let mut bell = state(&[179], 1);
    assert!(!bell.test_execute_one_card(PlayerSide::P1));
    assert_eq!(bell.p1.core.anima, 7);
    assert_eq!(bell.p1.deck.queue, vec![0]);
}

#[test]
fn dynamic_action_again_covers_four_debuffs_and_global_tune() {
    let card = original_card(20_193);
    let mut debuffs = state(&[20_193], 1);
    for status in [100, 101, 102, 103] {
        debuffs.add_actor_negative_status(PlayerSide::P2, status, 1);
    }
    assert!(debuffs.resolve_card_action_again(PlayerSide::P1, &card, 0, false, false));

    let mut tune = state(&[20_199], 1);
    apply_body(&mut tune, 20_199, 0, false);
    assert!(tune.resolve_card_action_again(PlayerSide::P1, &basic_attack(), 0, false, false,));
    assert!(tune.resolve_card_action_again(PlayerSide::P2, &basic_attack(), 0, false, false,));
}
