use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

fn filler_cards(active: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![active];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack_test_card());
    }
    cards
}

fn basic_attack_test_card() -> CardDefinition {
    let mut card = test_card(0, 0, "普通攻击");
    card.attack = Some(3);
    card
}

fn test_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
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
        other_params: vec![],
    }
}

fn fixture_player(cards: Vec<CardDefinition>) -> FixturePlayer {
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

fn hold_fixture(p1_card: CardDefinition, p2_card: CardDefinition) -> BattleFixture {
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
            p1: fixture_player(filler_cards(p1_card)),
            p2: fixture_player(filler_cards(p2_card)),
        },
    }
}

#[test]
fn hold_jindan_dream_beng_quan_chain_keeps_decision_for_later_effects() {
    let card =
        original_card_definition_by_id(10_020_089).expect("missing JinDan dream beng quan chain");
    let mut fixture = hold_fixture(card.clone(), basic_attack_test_card());
    fixture.decision_tape = vec![2, 99];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.beng.dream_beng_quan_chain, 2);
    assert_eq!(state.decision_tape, vec![2, 99]);
}

#[test]
fn hold_yuanying_dream_beng_quan_chain_consumes_and_adds_one_decision() {
    let card =
        original_card_definition_by_id(10_030_089).expect("missing YuanYing dream beng quan chain");
    let mut fixture = hold_fixture(card.clone(), basic_attack_test_card());
    fixture.decision_tape = vec![2, 99];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.beng.dream_beng_quan_chain, 4);
    assert_eq!(state.decision_tape, vec![99]);
}

#[test]
fn hold_dream_beng_quan_chain_releases_two_attack_per_stack_after_next_beng_quan() {
    let beng_quan = original_card_definition_by_id(10_000_001).expect("missing Beng Quan Chuo");
    let fixture = hold_fixture(beng_quan.clone(), basic_attack_test_card());
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.beng.dream_beng_quan_chain = 2;

    state.test_apply_card_effect(PlayerSide::P1, &beng_quan, 0);

    let printed_damage = beng_quan.attack.unwrap_or(0) * beng_quan.attack_count.unwrap_or(1);
    assert_eq!(state.p2.core.hp, 30 - printed_damage - 4);
    assert_eq!(state.p1.beng.dream_beng_quan_chain, 0);
    assert_eq!(state.p1.beng.triggered_dream_beng_quan_chain, 0);
}

#[test]
fn hold_talent_174_hp_cost_reduces_max_hp_not_current_hp() {
    let mut hp_cost = test_card(10_000_024, 10_000_024, "崩拳·截脉");
    hp_cost.hp_cost = Some(4);
    hp_cost.attack = Some(0);
    let mut fixture = hold_fixture(hp_cost, basic_attack_test_card());
    fixture.players.p1.talents = vec![174];
    fixture.players.p1.base_max_hp = 40;
    fixture.players.p1.extra_max_hp = None;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 30;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!((state.p1.core.max_hp, state.p1.core.hp), (36, 30));
}

#[test]
fn hold_negative_status_gain_hooks_apply() {
    let mut fixture = hold_fixture(basic_attack_test_card(), basic_attack_test_card());
    fixture.players.p1.talents = vec![177];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 20;
    state.add_actor_negative_status(PlayerSide::P1, 103, 2);
    assert_eq!(
        (state.p1.status.attack_reduction, state.p1.core.hp),
        (2, 22)
    );
}

#[test]
fn hold_falling_flower_internal_injury_consumes_star_erosion() {
    let mut card = test_card(4_000_021, 4_000_021, "落花有意");
    card.other_params = vec![1];
    let mut fixture = hold_fixture(card, basic_attack_test_card());
    fixture.players.p1.talents = vec![103];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        (
            state.p2.status.internal_injury,
            state.p2.astrology.star_erosion
        ),
        (2, 0)
    );
}

#[test]
fn hold_mysterious_crystal_heart_mirror_grants_defense_and_guard() {
    let mut mirror = test_card(99_000_106, 99_000_106, "玄晶护心镜");
    mirror.defense = Some(20);
    mirror.other_params = vec![2];
    let mut fixture = hold_fixture(mirror, basic_attack_test_card());
    fixture.players.p1.initial_anima = 3;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!((state.p1.core.defense, state.p1.core.guard), (20, 2));
}

#[test]
fn hold_blood_calamity_opening_consumes_star_erosion() {
    let mut blood_calamity = test_card(11_010_024, 11_000_024, "血光之灾");
    blood_calamity.other_params = vec![2, 1];
    let mut fixture = hold_fixture(basic_attack_test_card(), blood_calamity);
    fixture.players.p2.talents = vec![30_103];
    let state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(
        (
            state.p1.status.internal_injury,
            state.p1.astrology.star_erosion
        ),
        (4, 0)
    );
}

#[test]
fn hold_original_fallback_uses_random_attack_value() {
    let mut dice = test_card(4_000_055, 4_000_055, "一掷乾坤");
    dice.attack = Some(2);
    dice.random_attack = Some(8);
    dice.defense = Some(2);
    let mut fixture = hold_fixture(dice, basic_attack_test_card());
    fixture.decision_tape = vec![5];
    let mut state = ReplayState::test_from_fixture(&fixture);
    let card = state.p1.deck.slots[0].card.clone();
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!((state.p2.core.hp, state.p1.core.defense), (25, 2));
}

#[test]
fn hold_drunk_lie_leisure_grants_agility_and_basic_attack_stacks() {
    let mut drunk_lie = test_card(10_000_056, 10_000_056, "醉卧逍遥");
    drunk_lie.attack = Some(3);
    drunk_lie.other_params = vec![10, 4];
    let mut state =
        ReplayState::test_from_fixture(&hold_fixture(drunk_lie, basic_attack_test_card()));
    let card = state.p1.deck.slots[0].card.clone();
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(
        (
            state.p2.core.hp,
            state.p1.turn.agility,
            state.p1.status.drunken_leisure
        ),
        (27, 10, 4)
    );
    let mut basic_state = ReplayState::test_from_fixture(&hold_fixture(
        basic_attack_test_card(),
        basic_attack_test_card(),
    ));
    basic_state.p1.status.drunken_leisure = 2;
    basic_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        (
            basic_state.p2.core.hp,
            basic_state.p1.status.drunken_leisure
        ),
        (21, 0)
    );
}

#[test]
fn hold_water_billow_cost_is_free_when_previous_element_generates_water() {
    let mut water_billow = test_card(7_010_019, 7_000_019, "水灵·波澜");
    water_billow.anima = Some(-2);
    let mut state = ReplayState::test_from_fixture(&hold_fixture(
        water_billow.clone(),
        basic_attack_test_card(),
    ));
    state.p1.elements.last_element = Some(Element::Metal);
    assert_eq!(
        support::effective_anima_cost(&water_billow, &state.p1, Some(0)),
        0
    );
}

#[test]
fn hold_hungry_tiger_anima_cost_reduced_by_negative_status_stacks() {
    let mut hungry_tiger = test_card(10_000_029, 10_000_029, "饿虎扑食");
    hungry_tiger.anima = Some(-4);
    let mut state = ReplayState::test_from_fixture(&hold_fixture(
        hungry_tiger.clone(),
        basic_attack_test_card(),
    ));
    state.p1.status.internal_injury = 2;
    state.p1.status.flaw = 1;
    assert_eq!(
        support::effective_anima_cost(&hungry_tiger, &state.p1, Some(0)),
        1
    );

    state.p1.status.weakness = 1;
    assert_eq!(
        support::effective_anima_cost(&hungry_tiger, &state.p1, Some(0)),
        0
    );
}

#[test]
fn hold_ling_long_travel_reduces_action_again_card_anima_cost() {
    let bracelet = original_card_definition_by_id(16).expect("missing nine-heavens bracelet");
    let mut state =
        ReplayState::test_from_fixture(&hold_fixture(bracelet.clone(), basic_attack_test_card()));

    assert_eq!(
        support::effective_anima_cost(&bracelet, &state.p1, Some(0)),
        1
    );

    state.p1.identity.fate_strategies.push(321);
    assert_eq!(
        support::effective_anima_cost(&bracelet, &state.p1, Some(0)),
        0
    );
}

#[test]
fn hold_fate_strategy_27_runs_after_talents_before_permanent_consumables() {
    let mut fixture = hold_fixture(basic_attack_test_card(), basic_attack_test_card());
    fixture.players.p1.base_max_hp = 95;
    fixture.players.p1.character_id = Some(4_000_003);
    fixture.players.p1.talents = vec![179];
    fixture.players.p1.fate_strategies = vec![27];
    fixture.players.p1.hand_cards = vec![1];
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10008".to_string(), 10);
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10023".to_string(), 51);

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(state.p1.core.hp, 119);
    assert_eq!(state.p1.core.max_hp, 167);

    fixture.players.p1.base_max_hp = 107;
    fixture.players.p1.permanent_buff_temp_datas.remove("10008");
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10023".to_string(), 83);
    let talent_before_strategy = ReplayState::test_from_fixture(&fixture);
    assert_eq!(talent_before_strategy.p1.core.hp, 123);
    assert_eq!(talent_before_strategy.p1.core.max_hp, 203);
}

#[test]
fn hold_echo_pattern_runs_selected_hooks_for_temporary_water_formation() {
    let water_formation =
        original_card_definition_by_id(7_010_029).expect("missing water formation");
    let echo_pattern = original_card_definition_by_id(8_000_012).expect("missing echo pattern");
    let mut fixture = hold_fixture(water_formation.clone(), basic_attack_test_card());
    fixture.players.p1.active_slot_count = 4;
    fixture.players.p1.cards[3] = echo_pattern;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.anima = 6;
    state.p1.elements.water_formation = 2;
    state.p1.elements.activated_elements.push(Element::Water);
    state.p1.deck.queue = vec![3];

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 11);
    assert_eq!(state.p1.elements.water_formation, 3);
    assert!(!state.p1.deck.slots[0].used);
    assert!(state.p1.deck.slots[3].used);
}

#[test]
fn hold_sword_move_integration_keeps_selected_plain_name_cards() {
    let inspiration_sword =
        original_card_definition_by_id(1_000_038).expect("missing inspiration sword");
    let mut fixture = hold_fixture(inspiration_sword.clone(), basic_attack_test_card());
    fixture.players.p1.talents = vec![192];
    fixture
        .players
        .p1
        .talent_card_params
        .insert("189".to_string(), vec![10, 12, 1_000_038]);

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(
        state.p1.sword.ling_wu_card_base_ids,
        vec![10, 12, 1_000_038]
    );
    assert!(support::is_cloud_sword(&state.p1, &inspiration_sword));
}

#[test]
fn hold_cloud_sea_talent_preserves_chain_across_non_cloud_cards() {
    let basic = basic_attack_test_card();
    let cloud = test_card(1_000_040, 1_000_040, "云剑•月影");
    let mut fixture = hold_fixture(cloud.clone(), basic.clone());
    fixture.players.p1.talents = vec![14];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.sword.cloud_chain = 3;

    state.apply_card_classification_completed_hooks(PlayerSide::P1, &basic);
    assert_eq!(state.p1.sword.cloud_chain, 3);
    state.apply_card_classification_completed_hooks(PlayerSide::P1, &cloud);
    assert_eq!(state.p1.sword.cloud_chain, 4);
}

#[test]
fn hold_clear_heart_sword_embryo_heart_grants_next_cloud_sword_action_again() {
    let mut clear_heart = test_card(19, 19, "澄心剑胚");
    clear_heart.attack = Some(1);
    let mut moon_shadow = test_card(1_000_040, 1_000_040, "云剑•月影");
    moon_shadow.defense = Some(1);
    let mut fixture = hold_fixture(clear_heart.clone(), basic_attack_test_card());
    fixture.players.p1.talents = vec![20_096];
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_apply_card_effect(PlayerSide::P1, &clear_heart, 0);
    state.apply_selected_card_hooks(PlayerSide::P1, &moon_shadow, 1);
    assert_eq!(
        (
            state.p1.sword.cloud_sword_heart,
            state.p1.turn.extra_actions
        ),
        (0, 1)
    );
}
