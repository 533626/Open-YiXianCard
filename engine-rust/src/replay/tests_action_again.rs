use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn basic_attack() -> CardDefinition {
    super::test_support::basic_attack_card()
}

fn deck_with(card: CardDefinition) -> Vec<CardDefinition> {
    super::test_support::fill_deck(vec![card], basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6))
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
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

#[test]
fn wuxing_cards_action_again_satisfied_by_check_wu_xing_and_dream_spike() {
    // 原版 CardActionBase.CheckWuXing：卡组含 7030077/7040077（梦•五行刺）时恒真。
    // Card_7000028.cs:63 cardConfig.actionAgain = CheckWuXing(src, BuffType.JiHuoMuLing) && AddHpCount >= otherParams[0]。
    let patrol = original_card_definition_by_id(7_000_028).expect("missing wood patrol");
    let dream_spike = original_card_definition_by_id(7_030_077).expect("missing dream spike");

    // 1. 无木灵激活且卡组无五行刺：即使加过血也不触发再次行动
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(vec![
            patrol.clone(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
        ]),
        player(deck_with(basic_attack())),
    ));
    state.p1.core.anima = 2;
    state.p1.hp_mutation.add_hp_count = 1;
    assert!(!state.test_resolve_action_again(PlayerSide::P1, &patrol, 0));
    assert!(!state.test_execute_one_card(PlayerSide::P1));

    // 2. 卡组含 7030077 梦•五行刺，即使木灵未激活，CheckWuXing 恒真，加血满足即触发再次行动
    let mut state_with_spike = ReplayState::test_from_fixture(&fixture(
        player(vec![
            patrol.clone(),
            dream_spike,
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
            basic_attack(),
        ]),
        player(deck_with(basic_attack())),
    ));
    state_with_spike.p1.core.anima = 2;
    state_with_spike.p1.hp_mutation.add_hp_count = 1;
    assert!(state_with_spike.test_resolve_action_again(PlayerSide::P1, &patrol, 0));
    assert!(state_with_spike.test_execute_one_card(PlayerSide::P1));
    assert_eq!(
        state_with_spike
            .test_snapshot(PlayerSide::P1)
            .action_again_count,
        1
    );
}
