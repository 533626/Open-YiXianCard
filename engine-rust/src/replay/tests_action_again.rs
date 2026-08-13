use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};
use std::collections::BTreeMap;

fn basic_attack() -> CardDefinition {
    CardDefinition {
        id: 0,
        base_id: Some(0),
        name: "普通攻击".to_string(),
        card_type: None,
        attack: Some(3),
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

fn deck_with(card: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![card];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
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
fn extra_action_again_consumes_all_stacks_when_used() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(basic_attack())),
        player(deck_with(basic_attack())),
    ));
    state.test_configure_p1(|player| {
        player.turn.extra_actions = 2;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);

    assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.test_snapshot(PlayerSide::P1).action_again_count, 1);
    assert_eq!(state.p1.turn.extra_actions, 0);
}

#[test]
fn action_again_sources_consume_only_the_first_matching_priority() {
    fn run(
        card_action_again: bool,
        extra: i64,
        marrow: i64,
        gourd: i64,
        agility: i64,
    ) -> ReplayState {
        let mut card = basic_attack();
        card.id = 7_000_040;
        card.base_id = Some(7_000_040);
        card.name = "土灵•绝壁".to_string();
        card.action_again = card_action_again.then_some(true);
        let mut state = ReplayState::test_from_fixture(&fixture(
            player(deck_with(card.clone())),
            player(deck_with(basic_attack())),
        ));
        state.p1.elements.activated_elements.push(Element::Earth);
        state.p1.turn.extra_actions = extra;
        state.p1.elements.five_elements_marrow_art = marrow;
        state.p1.elements.five_elements_gourd = gourd;
        state.p1.turn.agility = agility;
        assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
        state
    }

    let card = run(true, 1, 2, 2, 12);
    assert_eq!(card.p1.turn.extra_actions, 0);
    assert_eq!(card.p1.elements.five_elements_marrow_art, 2);
    assert_eq!(card.p1.elements.five_elements_gourd, 2);
    assert_eq!(card.p1.turn.agility, 12);

    let extra = run(false, 1, 2, 2, 12);
    assert_eq!(extra.p1.turn.extra_actions, 0);
    assert_eq!(extra.p1.elements.five_elements_marrow_art, 2);
    assert_eq!(extra.p1.elements.five_elements_gourd, 2);
    assert_eq!(extra.p1.turn.agility, 12);

    let marrow = run(false, 0, 2, 2, 12);
    assert_eq!(marrow.p1.elements.five_elements_marrow_art, 1);
    assert_eq!(marrow.p1.elements.five_elements_gourd, 2);
    assert_eq!(marrow.p1.turn.agility, 12);

    let gourd = run(false, 0, 0, 2, 12);
    assert_eq!(gourd.p1.elements.five_elements_gourd, 1);
    assert_eq!(gourd.p1.turn.agility, 12);

    let agility = run(false, 0, 0, 0, 12);
    assert_eq!(agility.p1.turn.agility, 2);
}

#[test]
fn fate_strategy_348_reduces_agility_action_again_cost_to_nine() {
    let mut p1 = player(deck_with(basic_attack()));
    p1.fate_strategies = vec![348];
    p1.initial_agility = 12;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));
    let card = state.test_actor_card(PlayerSide::P1, 0);

    assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.test_snapshot(PlayerSide::P1).action_again_count, 1);
    assert_eq!(state.p1.turn.agility, 3);
}

#[test]
fn dynamic_card_action_again_is_frozen_before_after_card_attacks() {
    let mut swimming_dragon = basic_attack();
    swimming_dragon.id = 1_000_042;
    swimming_dragon.base_id = Some(1_000_042);
    swimming_dragon.name = "云剑·游龙".to_string();
    swimming_dragon.attack = Some(1);
    swimming_dragon.attack_count = Some(1);
    swimming_dragon.other_params = vec![0];

    let mut p2 = player(deck_with(basic_attack()));
    p2.initial_defense = 1;
    let mut state =
        ReplayState::test_from_fixture(&fixture(player(deck_with(swimming_dragon)), p2));
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 5;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 45);
    assert_eq!(state.p1.turn.action_again_count, 0);
}

#[test]
fn dream_anima_infusion_forces_wounded_count_through_defense_for_action_again() {
    let mut swimming_dragon = basic_attack();
    swimming_dragon.id = 1_000_042;
    swimming_dragon.base_id = Some(1_000_042);
    swimming_dragon.name = "云剑•游龙".to_string();
    swimming_dragon.attack = Some(1);
    swimming_dragon.attack_count = Some(1);
    swimming_dragon.other_params = vec![0];

    let mut dream_anima_infusion = basic_attack();
    dream_anima_infusion.id = 1_040_067;
    dream_anima_infusion.base_id = Some(1_000_067);
    dream_anima_infusion.name = "梦•灵气灌注".to_string();
    dream_anima_infusion.attack = None;

    let mut p1 = player(deck_with(swimming_dragon));
    p1.cards[1] = dream_anima_infusion;
    p1.active_slot_count = 2;
    let mut p2 = player(deck_with(basic_attack()));
    p2.initial_defense = 10;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));

    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 50);
    assert_eq!(state.p2.core.defense, 9);
    assert_eq!(state.p1.turn.action_again_count, 1);
}

#[test]
fn first_frenzy_sword_does_not_gain_action_again_from_its_completed_stack() {
    let mut frenzy_sword = basic_attack();
    frenzy_sword.id = 2;
    frenzy_sword.base_id = Some(2);
    frenzy_sword.name = "狂剑•炎舞".to_string();
    frenzy_sword.attack = Some(4);
    frenzy_sword.other_params = vec![2];

    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(frenzy_sword)),
        player(deck_with(basic_attack())),
    ));

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert_eq!(state.p1.turn.action_again_count, 0);
}

#[test]
fn fate_381_classifies_wound_description_cards_as_frenzy_swords() {
    // BattleCharacter.IsKuangJian (build 24610558) includes the shared
    // `FateStrategy 381 && cardConfig.desc.Contains("[击伤]")` branch. The
    // preceding 飞灵闪影剑 therefore writes KuangJian in OnAfterExecuted,
    // which Card_2 reads when deciding its action-again flag.
    let mut wound_card = basic_attack();
    wound_card.id = 1000043;
    wound_card.base_id = Some(1000043);
    wound_card.name = "飞灵闪影剑".to_string();
    wound_card.attack = Some(1);
    wound_card.attack_count = Some(4);

    let mut frenzy_sword = basic_attack();
    frenzy_sword.id = 2;
    frenzy_sword.base_id = Some(2);
    frenzy_sword.name = "狂剑•炎舞".to_string();
    frenzy_sword.attack = Some(2);

    let mut cards = deck_with(wound_card);
    cards[1] = frenzy_sword;
    let mut p1 = player(cards);
    p1.fate_strategies = vec![381];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(basic_attack()))));

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.turn.action_again_count, 1);
}

#[test]
fn flash_wind_snapshots_cloud_chain_at_each_effect_entry() {
    let mut flash_wind = basic_attack();
    flash_wind.id = 1_000_039;
    flash_wind.base_id = Some(1_000_039);
    flash_wind.name = "云剑•闪风".to_string();
    flash_wind.attack = Some(4);

    let mut ordinary = ReplayState::test_from_fixture(&fixture(
        player(deck_with(flash_wind.clone())),
        player(deck_with(basic_attack())),
    ));
    assert!(!ordinary.test_execute_one_card(PlayerSide::P1));
    assert_eq!(ordinary.p1.sword.cloud_chain, 1);

    let mut repeated = ReplayState::test_from_fixture(&fixture(
        player(deck_with(flash_wind.clone())),
        player(deck_with(basic_attack())),
    ));
    repeated.p1.fate.plum_blossom_twice = 1;
    assert!(repeated.test_execute_one_card(PlayerSide::P1));
    assert_eq!(repeated.p1.sword.cloud_chain, 2);
    assert_eq!(repeated.p2.core.hp, 42);

    let mut cloud_sea_player = player(deck_with(flash_wind));
    cloud_sea_player.talents = vec![14];
    let mut cloud_sea = ReplayState::test_from_fixture(&fixture(
        cloud_sea_player,
        player(deck_with(basic_attack())),
    ));
    assert!(cloud_sea.test_execute_one_card(PlayerSide::P1));
    assert_eq!(cloud_sea.p1.sword.cloud_chain, 1);
}

#[test]
fn devouring_ancient_vine_drains_after_successful_action_again() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(basic_attack())),
        player(deck_with(basic_attack())),
    ));
    state.test_configure_p1(|player| {
        player.turn.extra_actions = 1;
    });
    state.test_configure_p2(|player| {
        player.core.hp = 40;
        player.music.devouring_ancient_vine = 6; // 噬仙古藤
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);

    assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.p1.core.hp, 44);
    assert_eq!(state.p2.core.hp, 46);
    assert_eq!(state.test_snapshot(PlayerSide::P1).action_again_count, 1);
}
