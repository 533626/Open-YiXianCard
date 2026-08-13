use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::MirageRonghuiValue;
use super::effect_invocation::{
    EffectInvocationKind, EffectInvocationPhase, TemporaryInvocationSpec,
};
use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide};

#[test]
fn talent_52_replaces_eighth_basic_attack_with_seven_stars_stabilize_soul() {
    let mut cards = filler_cards(CardDefinition {
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
        other_params: vec![],
    });
    cards[7] = CardDefinition {
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
        other_params: vec![],
    };
    let fixture = BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 2,
            hp_delta_p1_minus_p2: 26,
            final_hp: None,
        },
        max_actor_turns: Some(2),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers {
            p1: FixturePlayer {
                level: 5,
                base_max_hp: 30,
                extra_max_hp: None,
                battle_start_hp: None,
                character_id: Some(2_000_004),
                talents: vec![52],
                fate_strategies: Vec::new(),
                fate_strategy_temp_datas: Default::default(),
                active_slot_count: 8,
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
            },
            p2: FixturePlayer {
                level: 5,
                base_max_hp: 30,
                extra_max_hp: None,
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
                cards: filler_cards(CardDefinition {
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
                    other_params: vec![],
                }),
            },
        },
    };
    let mut state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(
        super::support::normalized_base_id(&state.p1.deck.slots[7].card),
        11
    );
    assert_eq!(state.p1.deck.slots[7].card.name, "七星定魂");
    state.test_configure_p1(|player| player.deck.queue = vec![7]);
    state.test_play_actor_turn();
    assert_eq!(state.p2.core.hp, 26);
    assert_eq!(state.p2.status.cannot_act, 1);
}

#[test]
fn wood_spirit_array_increases_max_hp_without_healing_current_hp() {
    let wood_array = CardDefinition {
        id: 7_010_036,
        base_id: Some(7_000_036),
        name: "木灵阵".to_string(),
        card_type: Some(crate::model::OriginalEnumValue {
            value: 3,
            name: "Sustain".to_string(),
        }),
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
        other_params: vec![15, 3],
    };
    let fixture = minimal_fixture(
        filler_cards(wood_array),
        filler_cards(CardDefinition {
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
            other_params: vec![],
        }),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    let hp_before = state.p1.core.hp;
    let max_hp_before = state.p1.core.max_hp;
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.hp, hp_before);
    assert_eq!(state.p1.core.max_hp, max_hp_before + 15);
    assert_eq!(state.p1.elements.wood_array, 3);
}

#[test]
fn talent_198_upgrades_solitary_void_golden_book_without_adjacent_basics() {
    use super::deck_start::apply_deck_start_talent_effects;
    let mut cards = filler_cards(CardDefinition {
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
        other_params: vec![],
    });
    cards[0] = CardDefinition {
        id: 3020013,
        base_id: Some(3000013),
        name: "千里神行符".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(6),
        hp_cost: None,
        action_again: Some(true),
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
    };
    cards[1] = CardDefinition {
        id: 215,
        base_id: Some(215),
        name: "孤虚金书".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(2),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![6],
    };
    cards[2] = CardDefinition {
        id: 4010095,
        base_id: Some(4000095),
        name: "灵蛇绕柱".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: Some(2),
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![2, 1, 5],
    };
    let fixture = FixturePlayer {
        level: 5,
        base_max_hp: 75,
        extra_max_hp: Some(32),
        battle_start_hp: None,
        character_id: Some(2000006),
        talents: vec![194, 196, 195, 198],
        fate_strategies: vec![],
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
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
        last_round_used_card_base_ids: vec![],
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: vec![],
        cards: cards.clone(),
    };
    // No tiger body (125) here; the hp argument only gates talent 125.
    apply_deck_start_talent_effects(&mut cards, &fixture, fixture.base_max_hp);
    assert_eq!(cards[1].id, 20_215, "expected upgraded 孤虚金书 id 20215");
}

#[test]
fn talent_198_upgrades_solitary_void_golden_book_from_tier_one_without_overshooting() {
    use super::deck_start::apply_deck_start_talent_effects;
    // 孤虚金书 can enter the battle already at tier one (id 10_215) when the
    // pre-battle deck construction upgraded it once. Talent 198 still grants two
    // upgrade steps, but the original client walks one tier at a time and stops
    // at the first `noUpgrade` card (20_215). A direct jump 10_215 + 2*10_000
    // = 30_215 has no card definition, so the upgrade would silently no-op and
    // leave the card at 10_215 while the original client reaches 20_215.
    let mut cards = filler_cards(CardDefinition {
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
        other_params: vec![],
    });
    cards[0] = CardDefinition {
        id: 3020013,
        base_id: Some(3000013),
        name: "千里神行符".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(6),
        hp_cost: None,
        action_again: Some(true),
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
    };
    cards[1] = CardDefinition {
        id: 10_215,
        base_id: Some(215),
        name: "孤虚金书".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(2),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![6],
    };
    cards[2] = CardDefinition {
        id: 4010095,
        base_id: Some(4000095),
        name: "灵蛇绕柱".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: Some(2),
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![2, 1, 5],
    };
    let fixture = FixturePlayer {
        level: 5,
        base_max_hp: 75,
        extra_max_hp: Some(32),
        battle_start_hp: None,
        character_id: Some(2000006),
        talents: vec![194, 196, 195, 198],
        fate_strategies: vec![],
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
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
        last_round_used_card_base_ids: vec![],
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: vec![],
        cards: cards.clone(),
    };
    // No tiger body (125) here; the hp argument only gates talent 125.
    apply_deck_start_talent_effects(&mut cards, &fixture, fixture.base_max_hp);
    assert_eq!(
        cards[1].id, 20_215,
        "tier-one 孤虚金书 (10_215) must cap at 20_215 (noUpgrade), not overshoot to 30_215",
    );
}

#[test]
fn talent_125_checks_boundary_hp_instead_of_max_hp() {
    use super::deck_start::apply_deck_start_talent_effects;
    let mut cards = filler_cards(CardDefinition {
        id: 38,
        base_id: Some(38),
        name: "锟铻金环".to_string(),
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
        other_params: vec![2],
    });
    let fixture = FixturePlayer {
        level: 5,
        base_max_hp: 75,
        extra_max_hp: Some(58),
        battle_start_hp: None,
        character_id: Some(3_000_004),
        talents: vec![125],
        fate_strategies: vec![],
        fate_strategy_temp_datas: Default::default(),
        active_slot_count: 8,
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
        last_round_used_card_base_ids: vec![],
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: vec![],
        cards: cards.clone(),
    };
    // The gate uses the actor's hp at its own OnBattleStarted boundary (the
    // caller's sampled battleStartHp for the first actor, live hp after the
    // first actor's opening for the second), never the constructed maxHp.
    apply_deck_start_talent_effects(&mut cards, &fixture, 119);
    assert_eq!(cards[0].id, 38);
    assert_eq!(cards[0].other_params, vec![2]);

    apply_deck_start_talent_effects(&mut cards, &fixture, 120);
    assert_eq!(cards[0].id, 10_038);
    assert_eq!(cards[0].other_params, vec![3]);
}

#[test]
fn talent_125_prefers_original_whitelist_over_character_exclusivity() {
    use super::deck_start::apply_deck_start_talent_effects;
    let mut cards = filler_cards(
        super::original_config::original_card_definition(135).expect("锟铻熔火环 config exists"),
    );
    cards[1] =
        super::original_config::original_card_definition(38).expect("锟铻金环 config exists");
    let mut fixture = tiger_body_fixture(cards.clone());
    fixture.character_id = Some(3_000_004);

    apply_deck_start_talent_effects(&mut cards, &fixture, 120);

    assert_eq!(
        cards[0].id, 10_135,
        "the first original whitelist card upgrades"
    );
    assert_eq!(
        cards[1].id, 38,
        "a later character-exclusive card stays unchanged"
    );
}

#[test]
fn talent_125_whitelist_does_not_require_character_identity() {
    use super::deck_start::apply_deck_start_talent_effects;
    let mut cards = filler_cards(
        super::original_config::original_card_definition(0)
            .expect("upgradable basic attack config exists"),
    );
    cards[1] =
        super::original_config::original_card_definition(135).expect("锟铻熔火环 config exists");
    let fixture = tiger_body_fixture(cards.clone());

    apply_deck_start_talent_effects(&mut cards, &fixture, 120);

    assert_eq!(
        cards[0].id, 0,
        "fallback must not consume the earlier generic card"
    );
    assert_eq!(
        cards[1].id, 10_135,
        "whitelist matching is independent of owner"
    );
}

fn tiger_body_fixture(cards: Vec<CardDefinition>) -> FixturePlayer {
    let mut fixture = minimal_fixture(
        cards,
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    )
    .players
    .p1;
    fixture.base_max_hp = 120;
    fixture.battle_start_hp = Some(120);
    fixture.talents = vec![125];
    fixture.active_slot_count = 8;
    fixture
}

#[test]
fn chain_sword_formation_reuses_outer_slot_and_runs_temporary_on_play_hooks() {
    let mut swift_shadow = test_card(1_000_094, 1_000_094, "迅影飞剑");
    swift_shadow.attack = Some(4);
    swift_shadow.other_params = vec![1];

    let mut chain = test_card(1_000_064, 1_000_064, "连环剑阵");
    chain.defense = Some(1);

    let mut cards = filler_cards(basic_attack_test_card());
    cards[0] = swift_shadow;
    cards[7] = chain;
    let mut fixture = minimal_fixture(
        cards,
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 8;
    fixture.players.p1.talents = vec![192];
    fixture.players.p1.fate_strategies = vec![121, 325];
    fixture
        .players
        .p1
        .talent_card_params
        .insert("189".to_string(), vec![1_000_094]);

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.deck.queue = vec![7];
    state.test_execute_one_card(PlayerSide::P1);

    // Card_1000064 temporarily replaces its own CardItem before ExecuteEffect:
    // the copied 灵悟 card therefore runs the full temporary lifecycle at slot
    // 7, including OnPlayCard and the completed-card frenzy classification.
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p2.status.internal_injury, 4);
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert!(!state.p1.deck.slots[0].used);
    assert!(state.p1.deck.slots[7].used);
    assert_eq!(state.p1.turn.used_card_count, 2);

    let mut frenzy_second = test_card(1_000_035, 1_000_035, "狂剑•二式");
    frenzy_second.attack = Some(4);
    state.p1.sword.sword_intent = 0;
    state.apply_card_effect(PlayerSide::P1, &frenzy_second, 7, true);
    assert_eq!(state.p2.core.hp, 18);
}

#[test]
fn temporary_effect_restores_outer_context_and_clears_shared_attack_window() {
    let outer = basic_attack_test_card();
    let fixture = minimal_fixture(
        filler_cards(outer.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    // This lifecycle-only card deliberately reuses the audited one-hit body.
    let mut selected = test_card(9_999_991, 145, "震雷剑·临时");
    selected.attack = Some(10);
    state.p1.astrology.thunder_mindset = 40;
    let outer_effect = test_card(777, 777, "外层牌");
    state.begin_effect_invocation(
        PlayerSide::P1,
        &outer_effect,
        &outer_effect,
        &outer,
        0,
        0,
        EffectInvocationKind::Played,
        true,
    );
    state.set_active_effect_action_again(true);
    state.set_active_effect_after_action(true);
    state.p1.formations.heaven_cycle_sword_formation = 1;
    state.p1.formations.heaven_cycle_sword_formation_damage = 1;
    let outer_frame = state
        .active_effect_frame()
        .expect("outer effect frame")
        .clone();
    state.p1.fate.rear_move_succeeded = true;

    assert!(!state.apply_temporary_card_effect(PlayerSide::P1, &selected, 0));

    assert_eq!(state.p2.core.hp, 16);
    assert_eq!(state.active_effect_attacks(), 0);
    assert_eq!(state.p1.deck.slots[0].card.id, outer_effect.id);
    assert_eq!(outer_frame.physical.card.card_id, outer.id);
    assert!(state.p1.deck.slots[0].used);
    assert_eq!(state.active_effect_frame(), Some(&outer_frame));
    assert!(state.p1.fate.rear_move_succeeded);

    // The original DanKaGongJiJiShu counter is shared with nested effects and
    // removed by every OnAfterExecuted. The temporary card's attack therefore
    // cannot make the outer non-attacking card consume Heaven Cycle Formation.
    state.apply_regular_after_card_effect_hooks(PlayerSide::P1, &outer_effect, 0, false);
    assert_eq!(state.p2.core.hp, 16);
    assert_eq!(state.p1.formations.heaven_cycle_sword_formation, 1);
    state.end_effect_invocation(PlayerSide::P1, EffectInvocationKind::Played);
    assert!(state.active_effect_frame().is_none());
}

#[test]
fn chain_sword_formation_replays_only_nearest_sword_formation_recursively() {
    let formation = CardDefinition {
        id: 1_000_092,
        base_id: Some(1_000_092),
        name: "灵枢剑阵".to_string(),
        card_type: None,
        attack: Some(1),
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: Some(1),
        damage: None,
        anima: Some(1),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![1, 1],
    };
    let first_chain = CardDefinition {
        id: 1_000_064,
        base_id: Some(1_000_064),
        name: "连环剑阵".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: Some(1),
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
    };
    let second_chain = CardDefinition {
        id: 1_010_064,
        base_id: Some(1_000_064),
        name: "连环剑阵".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: Some(1),
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
    };
    let mut cards = filler_cards(second_chain.clone());
    cards[0] = formation;
    cards[1] = first_chain;
    cards[2] = second_chain;
    let mut fixture = minimal_fixture(
        cards,
        filler_cards(CardDefinition {
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
            other_params: vec![],
        }),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 6,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.actor_mut(PlayerSide::P1).deck.queue = vec![2];
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.defense, 3);
    assert_eq!(state.p2.core.hp, 24);
}

#[test]
fn impact_pattern_doubles_attack_when_array_echo_is_active() {
    let sustain = CardDefinition {
        id: 8_000_001,
        base_id: Some(8_000_001),
        name: "引雷阵".to_string(),
        card_type: Some(crate::model::OriginalEnumValue {
            value: 3,
            name: "Sustain".to_string(),
        }),
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
        other_params: vec![1, 1],
    };
    let impact = CardDefinition {
        id: 8_000_003,
        base_id: Some(8_000_003),
        name: "冲击阵纹".to_string(),
        card_type: None,
        attack: Some(4),
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
        other_params: vec![2],
    };
    let mut cards = filler_cards(impact.clone());
    cards[0] = sustain;
    cards[1] = impact;
    let mut fixture = minimal_fixture(
        cards,
        filler_cards(CardDefinition {
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
            other_params: vec![],
        }),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 8;
    fixture.max_actor_turns = Some(4);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.actor_mut(PlayerSide::P1).deck.queue = vec![0, 1];
    state.test_play_actor_turn();
    state.test_advance_actor();
    state.test_play_actor_turn();
    state.test_advance_actor();
    state.test_play_actor_turn();
    assert_eq!(
        state
            .actor(PlayerSide::P1)
            .formations
            .array_echo_persistent_card,
        1
    );
    assert_eq!(state.p2.core.hp, 23);
}

#[test]
fn turn_start_illusory_tune_death_skips_card_play() {
    let mut fixture = minimal_fixture(
        filler_cards(CardDefinition {
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
            other_params: vec![],
        }),
        filler_cards(CardDefinition {
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
            other_params: vec![],
        }),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 10,
            final_hp: None,
        },
    );
    fixture.first_player_side = PlayerSide::P2;
    fixture.max_actor_turns = Some(4);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.hp = 10;
        player.core.max_hp = 10;
    });
    state.test_configure_p2(|player| {
        player.core.hp = 3;
        player.core.max_hp = 10;
        player.music.illusory_tune = 3;
    });
    state.current_actor = PlayerSide::P2;
    state.test_play_actor_turn();
    assert_eq!(state.p2.core.hp, 0);
    assert_eq!(state.p1.core.hp, 10);
    assert!(!state.p2.deck.slots.iter().any(|slot| slot.used));
}

#[path = "tests_execution_sources.rs"]
mod execution_sources;
#[path = "tests_invocation_frames.rs"]
mod invocation_frames;
