use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

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
        other_params: Vec::new(),
    }
}

fn basic_attack() -> CardDefinition {
    let mut card = test_card(0, 0, "普通攻击");
    card.attack = Some(3);
    card
}

fn deck_with(active: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![active];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack());
    }
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    FixturePlayer {
        level: 5,
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

fn fixture(cards: Vec<CardDefinition>) -> BattleFixture {
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
            p1: player(cards),
            p2: player(deck_with(basic_attack())),
        },
    }
}

#[test]
fn original_granted_curiosity_and_secret_cards_match_v2_contracts() {
    let mut flame = test_card(2, 2, "狂剑•炎舞");
    flame.attack = Some(2);
    flame.other_params = vec![1];
    let mut flame_state = ReplayState::test_from_fixture(&fixture(deck_with(flame)));
    flame_state.p1.sword.frenzy_sword = 1;
    assert!(flame_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(flame_state.p2.core.hp, 28);
    assert_eq!(flame_state.p2.status.external_injury, 1);
    assert_eq!(flame_state.p1.turn.action_again_count, 1);

    let mut vigorous = test_card(18, 18, "金灵•刚劲");
    vigorous.attack = Some(5);
    let mut vigorous_state = ReplayState::test_from_fixture(&fixture(deck_with(vigorous)));
    vigorous_state
        .p1
        .elements
        .activated_elements
        .push(Element::Metal);
    vigorous_state.p1.sword.sharpness = 3;
    assert!(!vigorous_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(vigorous_state.p2.core.hp, 22);
    assert_eq!(vigorous_state.p1.sword.sharpness, 3);
    assert_eq!(vigorous_state.p1.elements.no_sharpness_for_attack, 0);

    let mut thunder = test_card(29, 29, "狂雷电闪");
    thunder.other_params = vec![2];
    let other_thunder = test_card(28, 28, "五雷轰顶");
    let mut thunder_cards = deck_with(thunder);
    thunder_cards[1] = other_thunder;
    let mut thunder_fixture = fixture(thunder_cards);
    thunder_fixture.players.p1.active_slot_count = 2;
    let mut thunder_state = ReplayState::test_from_fixture(&thunder_fixture);
    assert!(thunder_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(thunder_state.p2.status.flaw, 2);
    assert_eq!(thunder_state.p1.turn.action_again_count, 1);

    let mut bronze_cat = test_card(37, 37, "青铜猫");
    bronze_cat.defense = Some(5);
    bronze_cat.other_params = vec![2];
    let mut cat_state = ReplayState::test_from_fixture(&fixture(deck_with(bronze_cat)));
    cat_state.p1.sword.sword_intent = 3;
    assert!(!cat_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(cat_state.p1.core.defense, 11);

    let mut earth_secret = test_card(7_000_070, 7_000_070, "土灵秘印");
    earth_secret.defense = Some(6);
    earth_secret.other_params = vec![6];
    let mut earth_state = ReplayState::test_from_fixture(&fixture(deck_with(earth_secret)));
    assert!(!earth_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(earth_state.p1.core.defense, 6);
    assert_eq!(earth_state.p1.turn.next_turn_defense, 6);
    assert!(earth_state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Earth));
}

#[test]
fn dream_thunder_hexagram_art_restores_low_loss_ledger_and_installs_high_hook() {
    let mut low = test_card(4_000_088, 4_000_088, "梦•御雷卦诀");
    low.attack = Some(1);
    low.random_attack = Some(10);
    let mut low_fixture = fixture(deck_with(low));
    low_fixture.decision_tape = vec![1];
    let mut low_state = ReplayState::test_from_fixture(&low_fixture);
    low_state.p1.astrology.hexagram = 3;

    assert!(!low_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(low_state.p1.astrology.hexagram, 3);
    assert_eq!(low_state.original_lost_hexagram_ledger(PlayerSide::P1), 1);

    let mut high = test_card(4_040_088, 4_000_088, "梦•御雷卦诀");
    high.attack = Some(1);
    high.random_attack = Some(8);
    let mut high_fixture = fixture(deck_with(high));
    high_fixture.decision_tape = vec![1];
    let mut high_state = ReplayState::test_from_fixture(&high_fixture);
    high_state.p1.astrology.hexagram = 8;

    assert!(!high_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(high_state.p1.astrology.hexagram, 7);
    assert_eq!(high_state.p1.astrology.dream_thunder_hexagram, 1);

    high_state.modify_hexagram(PlayerSide::P1, -1);
    high_state.modify_hexagram(PlayerSide::P1, -1);
    assert_eq!(high_state.p1.astrology.hexagram, 6);
    assert_eq!(high_state.p1.astrology.dream_thunder_round_limit, 1);
    assert_eq!(high_state.original_lost_hexagram_ledger(PlayerSide::P1), 3);
}

#[test]
fn cloud_sword_hidden_dragon_body_uses_preexisting_chain_and_talent_222() {
    let mut hidden_dragon = test_card(331, 331, "云剑•潜龙");
    hidden_dragon.attack = Some(7);
    hidden_dragon.other_params = vec![1, 2];

    let mut ordinary = ReplayState::test_from_fixture(&fixture(deck_with(hidden_dragon.clone())));
    assert_eq!(
        ordinary.apply_sword_card_effect(PlayerSide::P1, &hidden_dragon, 0),
        Some(true)
    );
    assert_eq!(ordinary.p2.core.hp, 23);
    assert_eq!(ordinary.p1.sword.all_purpose_sword, 1);
    assert_eq!(ordinary.p1.sword.all_purpose_sword_effective_count, 0);
    assert_eq!(ordinary.p1.turn.extra_actions, 0);

    let mut chained = ReplayState::test_from_fixture(&fixture(deck_with(hidden_dragon.clone())));
    // Talent 14 is a virtual HasBuff(LianYun) source in the original, so it
    // selects otherParams[1] even without a stored cloud-chain stack.
    chained.p1.identity.talents.extend([14, 222]);
    chained.p1.sword.frenzy_sword = 1;
    assert_eq!(
        chained.apply_sword_card_effect(PlayerSide::P1, &hidden_dragon, 0),
        Some(true)
    );
    assert_eq!(chained.p2.core.hp, 23);
    assert_eq!(chained.p1.sword.all_purpose_sword, 2);
    assert_eq!(chained.p1.sword.all_purpose_sword_effective_count, 0);
    assert_eq!(chained.p1.turn.extra_actions, 1);
}
