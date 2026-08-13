use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing card {id}"))
}

fn basic_attack() -> CardDefinition {
    original_card(0)
}

fn deck_with(cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    let mut deck = cards;
    while deck.len() < DECK_SIZE {
        deck.push(basic_attack());
    }
    deck
}

fn sustain(value: i64) -> OriginalEnumValue {
    OriginalEnumValue {
        value,
        name: "Sustain".to_string(),
    }
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
        base_max_hp: 50,
        extra_max_hp: Some(0),
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
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

#[test]
fn chance_nether_three_point_hand_freezes_max_hp_bonus_for_all_attack_segments() {
    let card = original_card(10_000_067);
    let mut p1 = player(deck_with(vec![card.clone()]));
    p1.base_max_hp = 89;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.p1.turn.wood_spring_turns = 3;

    let handled = state.apply_chance_card_effect(PlayerSide::P1, &card, 0, false, 10_000_067);

    assert_eq!(handled, Some(true));
    assert_eq!(state.p1.core.max_hp, 95);
    assert_eq!(state.p2.core.hp, 38);
    assert_eq!(state.p1.turn.attack_segments_performed, 3);
}

#[test]
fn chance_nether_three_point_hand_guards_zero_divisor() {
    let mut card = original_card(10_000_067);
    card.other_params = vec![0];
    let mut p1 = player(deck_with(vec![card.clone()]));
    p1.base_max_hp = 10;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    let handled = state.apply_chance_card_effect(PlayerSide::P1, &card, 0, false, 10_000_067);

    assert_eq!(handled, Some(true));
    assert_eq!(state.p2.core.hp, 14);
    assert_eq!(state.p1.turn.attack_segments_performed, 3);
}

#[test]
fn chance_heavenly_fire_soul_refining_flag_loses_hp_before_max_hp() {
    let card = original_card(99_000_102);
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.base_max_hp = 100;
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(vec![card.clone()])), p2));
    state.p2.core.hp = 90;

    let handled = state.apply_chance_card_effect(PlayerSide::P1, &card, 0, false, 99_000_102);

    assert_eq!(handled, Some(false));
    assert_eq!(state.p2.core.hp, 75);
    assert_eq!(state.p2.core.max_hp, 85);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn chance_void_rift_spear_sets_hp_zero_and_blocks_revive() {
    let mut p1 = player(deck_with(vec![original_card(99_000_105)]));
    p1.initial_anima = 3;
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.base_max_hp = 100;
    p2.last_round_life = Some(4);
    p2.permanent_buff_temp_datas.insert("10072".to_string(), 10);
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.temp_life, 0);
    assert_eq!(state.p2.core.hp, 0);
    assert_eq!(state.p2.chance.cannot_revive, 1);
    assert_eq!(state.death_winner(), Some(PlayerSide::P1));
}

#[test]
fn chance_hunyuan_god_sealing_pearl_deals_damage_before_weakness() {
    let pearl = original_card(99_000_103);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![pearl.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p2.core.defense = 5;

    state.test_apply_card_effect(PlayerSide::P1, &pearl, 0);

    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.p2.status.weakness, 3);
}

#[test]
fn chance_nine_nether_seal_clears_guard_and_defense_before_damage() {
    let seal = original_card(99_000_104);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![seal.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p2.core.guard = 2;
    state.p2.core.temporary_guard = 1;
    state.p2.core.defense = 5;

    state.test_apply_card_effect(PlayerSide::P1, &seal, 0);

    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p2.core.temporary_guard, 0);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.turn.lost_defense_count, 5);
    assert_eq!(state.p2.core.hp, 20);
}

#[test]
fn chance_pet_turn_hooks_apply_start_and_end_effects() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.core.hp = 30;
    state.p1.core.max_hp = 50;
    state.p1.core.defense = 10;
    state.p1.chance.po_kong_diao = 2;
    state.p1.chance.di_xuan_gui = 3;
    state.p1.chance.pang_xian_li = 2;
    state.p1.chance.tun_tian_chi_yan_shou = 4;
    state.p1.chance.shi_xu_ling_shou = 2;
    state.p1.chance.san_wei_huan = 1;
    state.p2.core.hp = 40;
    state.p2.core.anima = 3;

    state.test_play_actor_turn();

    assert_eq!(state.p1.core.defense, 11);
    assert_eq!(state.p1.core.hp, 36);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p2.core.hp, 31);
    assert_eq!(state.p2.core.anima, 1);
    assert_eq!(state.p1.fate.exorcism, 1);
}

#[test]
fn chance_qin_and_shadow_rabbit_grant_action_again() {
    let mut qin = original_card(99_000_113);
    qin.card_type = Some(sustain(3));
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![qin, basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 45);

    let mut rabbit = original_card(99_000_210);
    rabbit.card_type = Some(sustain(3));
    let mut rabbit_state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![rabbit])),
        player(deck_with(vec![basic_attack()])),
    ));

    assert!(rabbit_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(rabbit_state.p1.core.hp, 40);
}

#[test]
fn chance_hound_replaces_next_card_with_same_rarity_basic_attack() {
    let mut hound = original_card(99_000_211);
    hound.card_type = Some(sustain(3));
    let mut p1 = player(deck_with(vec![hound]));
    p1.initial_anima = 2;
    let mut tanuki = original_card(99_010_208);
    tanuki.card_type = Some(sustain(3));
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![tanuki]))));

    state.test_execute_one_card(PlayerSide::P1);
    state.test_execute_one_card(PlayerSide::P2);

    assert_eq!(state.p2.deck.slots[0].card.id, 10_000);
    assert_eq!(state.p2.chance.you_ming_xu_hun_quan, 0);
    assert_eq!(state.p1.core.hp, 44);
}

#[test]
fn chance_parrot_copies_same_slot_card_with_recursion_guard() {
    let parrot = original_card(99_000_216);
    let mut eagle = original_card(99_000_200);
    eagle.card_type = Some(sustain(3));
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![parrot])),
        player(deck_with(vec![eagle])),
    ));
    state.p1.core.hp = 40;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 42);
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.p1.chance.po_kong_diao, 1);
    assert_eq!(state.p1.chance.huan_yu_ying_copy_guard, 0);
}

#[test]
fn chance_parrot_copy_runs_qin_card_body_and_completed_career_hook() {
    let parrot = original_card(99_000_216);
    let compassion_tune = original_card(5_000_006);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![parrot])),
        player(deck_with(vec![compassion_tune])),
    ));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.music.music_cards_played, 2);
    assert_eq!(state.p1.chance.huan_yu_ying_copy_guard, 0);
}

#[test]
fn exchange_hexagram_triggers_six_yao_before_quiet_mindset_heal() {
    let exchange = original_card(4_000_025);
    let mut p1 = player(deck_with(vec![basic_attack()]));
    p1.base_max_hp = 30;
    let mut p2 = player(deck_with(vec![exchange.clone()]));
    p2.base_max_hp = 30;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p2.formations.six_yao_formation = 3;
    state.p1.fate.quiet_mindset = 3;

    state.test_apply_card_effect(PlayerSide::P2, &exchange, 0);

    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p2.core.anima, 2);
    assert_eq!(state.p2.astrology.hexagram, 2);
}

#[test]
fn exchange_hexagram_self_resolution_enters_negative_status_kernel() {
    let exchange = original_card(4_000_025);
    let mut p2 = player(deck_with(vec![exchange.clone()]));
    p2.talents = vec![197];
    p2.last_round_used_card_base_ids = vec![4_000_001, 4_000_002];
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(vec![basic_attack()])), p2));
    state.p1.astrology.star_erosion = 1;
    state.p1.fate.exorcism = 2;

    state.test_apply_card_effect(PlayerSide::P2, &exchange, 0);

    assert_eq!(state.p1.status.internal_injury, 2);
    assert_eq!(state.p1.astrology.star_erosion, 0);
    assert_eq!(state.p1.fate.exorcism, 0);
}

#[test]
fn flying_fang_sword_restores_consumed_sword_intent_on_wounded_count() {
    let flying_fang = original_card(1_010_021);
    let mut p1 = player(deck_with(vec![flying_fang]));
    p1.base_max_hp = 30;
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.base_max_hp = 30;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p1.core.anima = 1;
    state.p1.sword.sword_intent = 3;
    state.p1.identity.talents.push(67);
    state.p2.core.defense = 20;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 29);
    assert_eq!(state.p2.core.max_hp, 29);
    assert_eq!(state.p1.sword.sword_intent, 3);
}

#[test]
fn flying_fang_defers_sword_intent_restore_until_after_formation_attack() {
    let flying_fang = original_card(1_020_021);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![flying_fang])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.core.anima = 1;
    state.p1.sword.sword_intent = 4;
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 5;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p1.sword.sword_intent, 4);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
    assert_eq!(state.active_effect_deferred_sword_intent_restore(), 0);
}

#[test]
fn canonical_secret_sword_handlers_match_frozen_ts_contracts() {
    let mut resonant = original_card(1_000_047);
    resonant.attack = Some(4);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![resonant.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &resonant, 0);
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.p1.core.anima, 4);

    let mut coiling = original_card(1_000_049);
    coiling.defense = Some(6);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![coiling.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.sword.frenzy_sword = 2;
    state.test_apply_card_effect(PlayerSide::P1, &coiling, 0);
    assert_eq!(state.p1.core.defense, 18);

    let mut diligent = original_card(1_000_054);
    diligent.attack = Some(6);
    diligent.other_params = vec![5];
    let mut p1 = player(deck_with(vec![diligent.clone()]));
    p1.last_round_exp = 17;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.test_apply_card_effect(PlayerSide::P1, &diligent, 0);
    assert_eq!(state.p2.core.hp, 41);

    let mut spirit_cloud = original_card(1_000_058);
    spirit_cloud.attack = Some(4);
    let mut p1 = player(deck_with(vec![spirit_cloud.clone()]));
    p1.initial_anima = 2;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.p1.sword.cloud_chain = 1;
    state.test_apply_card_effect(PlayerSide::P1, &spirit_cloud, 0);
    assert_eq!(state.p2.core.hp, 26);
}

#[test]
fn canonical_astrology_handlers_match_frozen_ts_contracts() {
    let mut rebirth = original_card(4_000_047);
    rebirth.other_params = vec![4];
    let mut p1 = player(deck_with(vec![rebirth.clone()]));
    p1.initial_anima = 3;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.p1.astrology.hexagram = 2;
    state.p1.core.hp = 20;
    state.test_apply_card_effect(PlayerSide::P1, &rebirth, 0);
    assert_eq!(state.p1.astrology.hexagram, 0);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.hp, 40);

    let mut contest = original_card(4_000_050);
    contest.other_params = vec![13, 2];
    let mut p1 = player(deck_with(vec![contest.clone()]));
    p1.base_max_hp = 80;
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.base_max_hp = 70;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.test_apply_card_effect(PlayerSide::P1, &contest, 0);
    assert_eq!(state.p2.core.hp, 52);

    let mut slay_dragon = original_card(4_000_051);
    slay_dragon.attack = Some(10);
    slay_dragon.other_params = vec![5];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![slay_dragon.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.astrology.star_power = 2;
    state.p1.astrology.star_slots.push(0);
    state.test_apply_card_effect(PlayerSide::P1, &slay_dragon, 0);
    assert_eq!(state.p2.core.hp, 30);

    let mut ask_way = original_card(4_000_053);
    ask_way.attack = Some(6);
    ask_way.other_params = vec![2, 2];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![ask_way.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.decision_tape = vec![1, 2];
    state.test_apply_card_effect(PlayerSide::P1, &ask_way, 0);
    assert_eq!(state.p2.core.hp, 44);
    assert_eq!(state.p2.status.weakness, 1);
    assert_eq!(state.p2.status.flaw, 2);

    let mut derivation = original_card(4_000_054);
    derivation.other_params = vec![2];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![derivation.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.apply_chance_card_effect(PlayerSide::P1, &derivation, 0, true, 4_000_054);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert!(state.p1.fate.rear_move_succeeded);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &derivation, 0));
}

#[test]
fn canonical_five_element_handlers_match_frozen_ts_contracts() {
    let mut shake = original_card(7_000_046);
    shake.attack = Some(6);
    shake.defense = Some(6);
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.initial_anima = 5;
    p2.initial_defense = 8;
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(vec![shake.clone()])), p2));
    state.activate_element(PlayerSide::P1, Element::Earth);
    state.test_apply_card_effect(PlayerSide::P1, &shake, 0);
    assert_eq!(state.p1.core.defense, 6);
    assert_eq!(state.p2.core.anima, 2);
    assert_eq!(state.p2.core.defense, 1);

    let mut nourish = original_card(7_000_048);
    nourish.other_params = vec![2, 2];
    let mut p1 = player(deck_with(vec![nourish.clone()]));
    p1.initial_anima = 3;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.p1.core.hp = 20;
    state.activate_element(PlayerSide::P1, Element::Water);
    state.activate_element(PlayerSide::P1, Element::Wood);
    state.test_apply_card_effect(PlayerSide::P1, &nourish, 0);
    assert_eq!(state.p1.core.anima, 5);
    assert_eq!(state.p1.core.hp, 30);

    let mut explosion = original_card(7_000_052);
    explosion.attack = Some(25);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![explosion.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    for element in [Element::Wood, Element::Fire, Element::Earth] {
        state.activate_element(PlayerSide::P1, element);
    }
    state.test_apply_card_effect(PlayerSide::P1, &explosion, 0);
    assert_eq!(state.p2.core.hp, -25);

    let mut seal_throat = original_card(7_000_063);
    seal_throat.attack = Some(8);
    seal_throat.other_params = vec![2];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![seal_throat.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.activate_element(PlayerSide::P1, Element::Metal);
    state.test_apply_card_effect(PlayerSide::P1, &seal_throat, 0);
    assert_eq!(state.p2.core.hp, 42);
    assert_eq!(state.p2.mirage_ronghui.cannot_gain_hp, 2);

    let bloom = original_card(7_000_065);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![bloom.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &bloom, 0);
    assert_eq!(
        state.p1.elements.activated_elements,
        vec![
            Element::Wood,
            Element::Fire,
            Element::Earth,
            Element::Metal,
            Element::Water
        ]
    );
    // ExecuteTemporaryCard temporarily replaces the physical slot and restores
    // the original card after each nested invocation.
    assert_eq!(state.p1.deck.slots[0].card.id, bloom.id);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &bloom, 0));
}

#[test]
fn canonical_fist_and_artifact_handlers_match_frozen_ts_contracts() {
    let mut armor = original_card(99_000_100);
    armor.defense = Some(5);
    let mut p1 = player(deck_with(vec![armor.clone()]));
    p1.initial_defense = 7;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.test_apply_card_effect(PlayerSide::P1, &armor, 0);
    assert_eq!(state.p1.core.defense, 24);

    let mut spear = original_card(99_000_107);
    spear.attack = Some(12);
    spear.other_params = vec![2];
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.initial_defense = 6;
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(vec![spear.clone()])), p2));
    state.test_apply_card_effect(PlayerSide::P1, &spear, 0);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.hp, 41);

    let mut tower = original_card(99_000_111);
    tower.other_params = vec![25, 1];
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.initial_defense = 5;
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(vec![tower.clone()])), p2));
    state.test_apply_card_effect(PlayerSide::P1, &tower, 0);
    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.status.cannot_act, 1);

    let mut struggle = original_card(10_000_054);
    struggle.physique = Some(2);
    struggle.other_params = vec![14];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![struggle.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &struggle, 0);
    assert_eq!(state.p1.core.max_hp, 38);
    assert_eq!(state.p2.core.max_hp, 36);
    assert_eq!(state.p1.core.physique, 2);

    let mut burn_boats = original_card(10_000_061);
    burn_boats.other_params = vec![80, 3];
    let mut p1 = player(deck_with(vec![burn_boats.clone()]));
    p1.base_max_hp = 100;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.test_apply_card_effect(PlayerSide::P1, &burn_boats, 0);
    assert_eq!(state.p1.core.max_hp, 20);
    assert_eq!(state.p1.core.hp, 20);
    assert_eq!(state.p1.beng.momentum, 3);
    assert_eq!(state.p1.core.guard, 3);
    assert_eq!(state.p1.core.attack_bonus, 3);

    let mut burial = original_card(10_000_064);
    burial.other_params = vec![3];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![burial.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &burial, 0);
    for actor in [&state.p1, &state.p2] {
        assert_eq!(actor.status.internal_injury, 3);
        assert_eq!(actor.status.flaw, 3);
        assert_eq!(actor.status.weakness, 3);
        assert_eq!(actor.status.entangle, 3);
        assert_eq!(actor.status.external_injury, 3);
    }
}

#[test]
fn five_element_bloom_uses_matching_rarity_for_temporary_seals() {
    let base = original_card(7_000_065);
    let upgraded = original_card(7_010_065);
    let mut base_state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![base.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    let mut upgraded_state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![upgraded.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));

    base_state.test_apply_card_effect(PlayerSide::P1, &base, 0);
    upgraded_state.test_apply_card_effect(PlayerSide::P1, &upgraded, 0);

    assert_eq!(
        upgraded_state.p1.elements.activated_elements,
        base_state.p1.elements.activated_elements
    );
    assert_ne!(upgraded_state.p1.core.anima, base_state.p1.core.anima);
}

#[test]
fn canonical_replay_handlers_cover_clear_heart_formation_and_dream_anima_infusion() {
    let mut formation = original_card(126);
    formation.defense = Some(5);
    formation.other_params = vec![2];
    let second_formation = original_card(48);
    let mut p1 = player(deck_with(vec![formation.clone(), second_formation]));
    p1.active_slot_count = 2;
    p1.talents = vec![10_094, 20_094, 10_095, 20_095];
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);
    // Card_126.cs reads TalentConfig values: 20094=+1 attack bonus,
    // 10095=+2 anima, then 20095 adds 3 defense per current anima.
    assert_eq!(state.p1.core.defense, 11);
    assert_eq!(state.p1.core.attack_bonus, 1);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p2.core.hp, 44);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &formation, 0));

    let mut low = custom_card(1_020_067, 1_000_067, "梦•灵气灌注");
    low.anima = Some(2);
    low.other_params = vec![10];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![low.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &low, 0);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.turn.next_attack_bonus, 10);
    assert_eq!(state.p1.core.attack_bonus, 0);

    let mut high = custom_card(1_040_067, 1_000_067, "梦•灵气灌注");
    high.anima = Some(2);
    high.other_params = vec![10];
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![high.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.test_apply_card_effect(PlayerSide::P1, &high, 0);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.turn.next_attack_bonus, 0);
    assert_eq!(state.p1.core.attack_bonus, 10);
}

#[test]
fn clear_heart_formation_uses_current_anima_for_yuling_condensation_defense() {
    let formation = original_card(126);
    let mut p1 = player(deck_with(vec![formation.clone()]));
    p1.initial_anima = 2;
    p1.talents = vec![20_095];
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);

    // CardConfig 126.def=2; TalentConfig 20095.otherParams[0]=3; anima=2.
    assert_eq!(state.p1.core.defense, 8);
    assert_eq!(state.p1.core.anima, 2);
}

#[test]
fn giant_ape_spirit_sword_attacks_then_grants_attack_bonus() {
    let giant_ape = original_card(1_000_055);
    let mut p1 = player(deck_with(vec![giant_ape]));
    p1.initial_anima = 2;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 44);
    assert_eq!(state.p1.core.attack_bonus, 3);
}

#[test]
fn spirit_claw_adds_attack_to_cards_with_anima_in_original_desc() {
    let cloud_sword_calling_rain = original_card(15);
    let mut p1 = player(deck_with(vec![cloud_sword_calling_rain]));
    p1.fate_strategies = vec![152];
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 35);
}

#[test]
fn upgraded_frenzy_obsession_starts_with_used_frenzy_sword_count() {
    let mut p1 = player(deck_with(vec![basic_attack()]));
    p1.talents = vec![30_070];
    let state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    assert_eq!(state.p1.sword.frenzy_sword, 1);
}

fn custom_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    CardDefinition {
        id,
        base_id: Some(base_id),
        name: name.to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: Vec::new(),
    }
}

fn one_slot_deck_with(active: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![active];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn one_slot_player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 1,
        base_max_hp: 30,
        extra_max_hp: None,
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
        initial_momentum_limit: None,
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

#[test]
fn talent_153_pays_missing_anima_with_hp_and_physique() {
    let mut tune = custom_card(5_010_015, 5_000_015, "TianYinKunXianQu");
    tune.card_type = Some(sustain(super::CARD_TYPE_SUSTAIN));
    tune.anima = Some(-1);

    let mut p1 = one_slot_player(one_slot_deck_with(tune));
    p1.talents = vec![153];
    p1.permanent_buff_temp_datas
        .insert(super::PERMANENT_PHYSIQUE_KEY.to_string(), 3);
    let mut state = ReplayState::test_from_fixture(&fixture(
        p1,
        one_slot_player(one_slot_deck_with(basic_attack())),
    ));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.hp, 27);
    assert_eq!(state.p1.core.max_hp, 32);
    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.music.immortal_binding_tune, 1);
}

#[test]
fn battle_start_hexagram_triggers_astrology_talent_61() {
    let mut p1 = one_slot_player(one_slot_deck_with(basic_attack()));
    p1.talents = vec![61, 20_030];

    let state = ReplayState::test_from_fixture(&fixture(
        p1,
        one_slot_player(one_slot_deck_with(basic_attack())),
    ));

    assert_eq!(state.p1.astrology.hexagram, 2);
    assert_eq!(state.p1.astrology.star_power, 1);
}

#[test]
fn double_ghost_knock_applies_injuries_before_attacking() {
    let mut double_ghost = custom_card(10_010_030, 10_000_030, "DoubleGhostKnock");
    double_ghost.anima = Some(-1);
    double_ghost.attack = Some(7);
    double_ghost.attack_count = Some(2);
    double_ghost.other_params = vec![2, 1];

    let mut p1 = one_slot_player(one_slot_deck_with(double_ghost));
    p1.initial_anima = 1;
    let mut p2 = one_slot_player(one_slot_deck_with(basic_attack()));
    p2.initial_defense = 1;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p2.core.hp, 15);
    assert_eq!(state.p1.status.internal_injury, 2);
    assert_eq!(state.p2.status.internal_injury, 2);
    assert_eq!(state.p1.status.external_injury, 1);
    assert_eq!(state.p2.status.external_injury, 1);
}

#[test]
fn double_ghost_knock_does_not_inherit_beng_quan_han() {
    let mut double_ghost = custom_card(10_010_030, 10_000_030, "DoubleGhostKnock");
    double_ghost.anima = Some(-1);
    double_ghost.attack = Some(7);
    double_ghost.attack_count = Some(2);
    double_ghost.other_params = vec![2, 1];

    let mut p1 = one_slot_player(one_slot_deck_with(double_ghost));
    p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&fixture(
        p1,
        one_slot_player(one_slot_deck_with(basic_attack())),
    ));
    state.p1.beng.beng_quan_han = 3;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.beng.beng_quan_han, 3);
}
