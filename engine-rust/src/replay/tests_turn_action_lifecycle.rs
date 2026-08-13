use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::*;
use crate::fixture::FixtureExpected;
use crate::model::{CardDefinition, PlayerSide};

#[test]
fn add_hp_count_survives_turn_hp_gained_reset_for_lifetime_consumers() {
    let patrol = original_card_definition_by_id(7_000_028).expect("missing wood spirit patrol");
    let fixture = minimal_fixture(
        filler_cards(patrol.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.activate_element(PlayerSide::P1, Element::Wood);

    state.modify_actor_hp(PlayerSide::P1, 7, false, false);

    assert_eq!(state.p1.hp_mutation.add_hp_count, 7);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 7);

    state.clear_turn_hp_gained_ledgers(PlayerSide::P1);

    assert_eq!(state.p1.hp_mutation.add_hp_count, 7);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 0);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &patrol, 0));
}

#[test]
fn turn_start_clears_stale_hp_gain_before_dynamic_action_again() {
    let mut card = test_card(152, 152, "炼神还虚");
    card.other_params = vec![0, 0, 0];
    let fixture = minimal_fixture(
        filler_cards(card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.dream_mirage.turn_hp_gained = 7;
        player.turn.action_again_count = 1;
    });

    state.test_play_actor_turn();

    assert_eq!(state.p1.turn.used_card_count, 1);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 0);
    assert_eq!(state.p1.turn.action_again_count, 0);
}

#[test]
fn next_turn_defense_is_applied_after_tune_negative_status_reflect() {
    // BattleCharacter.OnTurnStarted: DuanChangQu (IL_0701) and its
    // YinFuJueZhen reflect precede XiaHuiHeJiaFang (IL_0c42).
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.hp = 52;
        player.core.max_hp = 100;
        player.status.internal_injury = 7;
        player.status.yin_fu_jue_zhen = 2;
        player.music.heartbreak_tune = 1;
        player.turn.next_turn_defense = 7;
    });

    state.test_play_actor_turn();

    assert_eq!(state.p1.core.hp, 42);
    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p1.status.internal_injury, 8);
    assert_eq!(state.p1.turn.next_turn_defense, 0);
}

#[test]
fn rejuvenation_tune_opens_healing_cap_before_flower_maze_drain() {
    // oracle: c0f73571469dd090/round-14 turn35. Near full HP, the tune's
    // max-HP gain must precede the larger flower-maze heal or Talent 120
    // loses three healing to the old cap.
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.hp = 263;
        player.core.max_hp = 275;
        player.identity.talents = vec![120];
        player.music.rejuvenation_tune = 4;
        player.formations.flower_maze_formation = 13;
    });
    state.p2.core.hp = 167;
    state.p2.core.max_hp = 290;
    state.p2.status.attack_reduction = 17;

    state.trigger_turn_end_formations(PlayerSide::P1);

    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (285, 285));
    assert_eq!(state.p2.core.hp, 149);
}

#[test]
fn turn_end_consumes_turn_hp_gain_before_clearing_ledger() {
    let mut card = test_card(152, 152, "炼神还虚");
    card.other_params = vec![3, 0, 0];
    let mut fixture = minimal_fixture(
        filler_cards(card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.dream_mirage.healing_turn_end_frenzy = 1;
    });

    state.test_play_actor_turn();

    // The MengKuangEr turn-end conversion folds its stacks into the KuangJian
    // buff (sword.frenzy_sword), the counter 狂剑•二式 reads.
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 0);
    assert_eq!(state.p1.turn.action_again_count, 0);
}

#[test]
fn turn_end_formations_read_action_again_count_before_final_reset() {
    let mut card = basic_attack_test_card();
    card.action_again = Some(true);
    let fixture = minimal_fixture(
        filler_cards(card),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 6,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.formations.immovable_formation = 1;
        player.formations.immovable_formation_value = 7;
    });

    state.test_play_actor_turn();

    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p1.core.max_hp, 30);
    assert_eq!(state.p1.turn.action_again_count, 0);
}

#[test]
fn water_month_sword_formation_skips_turn_start_defense_decay() {
    let fixture = minimal_fixture(
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
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.defense = 20;
        player.sword.water_month_sword_formation = 2;
    });
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.defense, 20);
    assert_eq!(state.p1.sword.water_month_sword_formation, 1);
}

#[test]
fn talent_101_grants_peach_blossom_extra_action_without_generating_chain() {
    use super::Element;
    let peach = CardDefinition {
        id: 10_020,
        base_id: Some(20),
        name: "木灵•桃花印".to_string(),
        card_type: None,
        attack: Some(2),
        random_attack: None,
        random_defense: None,
        attack_count: Some(3),
        defense: None,
        damage: None,
        anima: Some(3),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
    };
    let cards = filler_cards(peach.clone());
    let fixture = minimal_fixture(
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
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.identity.talents = vec![101];
        player.elements.last_element = Some(Element::Wood);
        // Steam build 24217566: the peach-blossom extra action additionally
        // requires a resolved water-spirit card (YiYongGuoShuiLingPai).
        player.elements.used_water_spirit_card = 1;
        player.core.anima = 10;
    });
    assert!(
        state.test_execute_one_card(PlayerSide::P1),
        "talent-101 peach blossom should grant action again independently of generating"
    );
    assert_eq!(state.p1.core.max_hp, 30);
}

#[test]
fn talent_101_blocks_peach_blossom_extra_action_without_water_spirit() {
    use super::Element;
    let peach = CardDefinition {
        id: 10_020,
        base_id: Some(20),
        name: "木灵•桃花印".to_string(),
        card_type: None,
        attack: Some(2),
        random_attack: None,
        random_defense: None,
        attack_count: Some(3),
        defense: None,
        damage: None,
        anima: Some(3),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
    };
    let cards = filler_cards(peach.clone());
    let fixture = minimal_fixture(
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
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.identity.talents = vec![101];
        player.elements.last_element = Some(Element::Water);
        // No water-spirit card resolved this battle: the build-24217566 gate
        // blocks the extra action even though the water→wood resonance matches.
        player.core.anima = 10;
    });
    assert!(
        !state.test_execute_one_card(PlayerSide::P1),
        "talent-101 peach blossom must not grant action again without a water-spirit card"
    );
}

#[test]
fn talent_101_does_not_treat_seasonal_card_7010020_as_peach_blossom() {
    use super::Element;
    let water_spirit_retreat = test_card(7_010_020, 7_000_020, "水灵•潜遁");
    let fixture = minimal_fixture(
        filler_cards(water_spirit_retreat.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.identity.talents = vec![101];
        player.elements.last_element = Some(Element::Wood);
        player.elements.used_water_spirit_card = 1;
    });

    state.apply_selected_card_hooks(PlayerSide::P1, &water_spirit_retreat, 0);

    assert_eq!(state.p1.turn.extra_actions, 0);
}

#[test]
fn spirit_snake_coils_pillar_grants_defense_and_action_again() {
    let snake = CardDefinition {
        id: 4_000_095,
        base_id: Some(4_000_095),
        name: "灵蛇绕柱".to_string(),
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
        other_params: vec![1, 1, 5, 5],
    };
    let fixture = minimal_fixture(
        filler_cards(snake),
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
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.defense = 5;
    });
    state.test_configure_p2(|player| {
        player.status.flaw = 2;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);
    assert!(state.test_execute_one_card(PlayerSide::P1));
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_internal_injury, 1);
    assert!(snapshot.p1_defense >= 5);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &card, 0));
    state.test_execute_one_card(PlayerSide::P1);
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_internal_injury, 2);
    assert_eq!(snapshot.action_again_count, 1);
}

#[test]
fn adaptation_boosts_positive_defense_gain() {
    let fixture = minimal_fixture(
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
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.turn.adaptation = 1;
    });
    state.gain_defense(PlayerSide::P1, 12);
    assert_eq!(state.p1.core.defense, 17);
}

#[test]
fn star_chess_contest_applies_flaw_and_action_again_in_star_slot() {
    let contest = CardDefinition {
        id: 4_000_094,
        base_id: Some(4_000_094),
        name: "星弈·劫争".to_string(),
        card_type: None,
        attack: Some(1),
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(1),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![1, 2],
    };
    let fixture = minimal_fixture(
        filler_cards(contest),
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
            hp_delta_p1_minus_p2: -1,
            final_hp: None,
        },
    );
    fixture.validate().expect("fixture valid");
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.astrology.star_slots = vec![0];
        player.astrology.star_power = 2;
        player.core.anima = 1;
    });
    state.test_configure_p2(|player| {
        player.core.anima = 0;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);
    assert!(state.test_execute_one_card(PlayerSide::P1));
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_flaw, 1);
    assert_eq!(snapshot.p1_anima, 2);
    assert_eq!(snapshot.action_again_count, 1);
    assert!(state.test_resolve_action_again(PlayerSide::P1, &card, 0));
    state.test_execute_one_card(PlayerSide::P1);
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_flaw, 2);
    assert_eq!(snapshot.p1_anima, 3);
}

#[test]
fn immortal_binding_tune_blocks_action_again() {
    let mut state = ReplayState::test_from_fixture(&minimal_fixture(
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
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    ));
    state.test_configure_p1(|player| {
        player.music.immortal_binding_tune = 1;
        player.turn.extra_actions = 1;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);
    assert!(!state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.test_snapshot(PlayerSide::P1).action_again_count, 0);
    assert_eq!(state.p1.turn.extra_actions, 0);
}

#[test]
fn entangle_blocks_action_again_before_ling_qi_ben_yong_reward() {
    let mut state = ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(CardDefinition {
            id: 57,
            base_id: Some(57),
            name: "滚石印".to_string(),
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
            other_params: vec![2],
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
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 17;
        player.status.entangle = 1;
        player.identity.talents.push(208); // 灵炁奔涌
        player.identity.fate_strategies.push(340);
        player.beng.gun_stance = 1;
        player.chance.ying_xiao_tu = 10;
        player.fate.ice_snow_lotus = 1;
    });
    state.test_configure_p2(|player| {
        player.music.immortal_binding_vine = 2;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);
    assert!(!state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.test_snapshot(PlayerSide::P1).action_again_count, 1);
    assert_eq!(state.p1.status.entangle, 0);
    assert_eq!(state.p1.status.external_injury, 2);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.hp, 17);
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.fate.ice_snow_lotus, 1);
}

#[test]
fn eight_gates_formation_damages_actor_on_action_again() {
    let mut state = ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(CardDefinition {
            id: 57,
            base_id: Some(57),
            name: "滚石印".to_string(),
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
            other_params: vec![2],
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
            hp_delta_p1_minus_p2: -3,
            final_hp: None,
        },
    ));
    state.test_configure_p2(|player| {
        player.formations.eight_gates_formation = 1;
        player.formations.eight_gates_formation_damage = 3;
    });
    let card = state.test_actor_card(PlayerSide::P1, 0);
    let p1_hp_before = state.p1.core.hp;
    assert!(state.test_consume_action_again(PlayerSide::P1, &card, 0));
    assert_eq!(state.p1.core.hp, p1_hp_before - 3);
    assert_eq!(state.p2.formations.eight_gates_formation, 0);
}

#[test]
fn reflect_mindset_damages_attacker_when_attack_is_fully_absorbed() {
    let strike = CardDefinition {
        id: 0,
        base_id: Some(0),
        name: "普通攻击".to_string(),
        card_type: None,
        attack: Some(3),
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
    };
    let mut state = ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(strike),
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
    ));
    state.test_configure_p1(|player| {
        player.core.hp = 20;
        player.core.max_hp = 20;
    });
    state.test_configure_p2(|player| {
        player.core.hp = 30;
        player.core.max_hp = 30;
        player.core.defense = 10;
        player.fate.reflect_mindset = 2;
    });
    state.test_execute_one_card(PlayerSide::P1);
    let snapshot = state.test_snapshot(PlayerSide::P1);
    assert_eq!(snapshot.p2_hp, 30);
    assert_eq!(state.p1.core.hp, 18);
}

#[test]
fn internal_injury_ticks_after_turn_start_healing() {
    // 原版 OnTurnStarted：治疗结算先于内伤 tick（IL_1a4c）。满血边界下
    // 治疗先被 maxHp 封顶再被内伤扣血；顺序反了治疗会补回内伤扣掉的血。
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.core.hp = 97;
        player.core.max_hp = 100;
        player.status.recovery = 5;
        player.status.internal_injury = 4;
    });

    state.test_play_actor_turn();

    // 97+5 被 100 封顶，再 -4 → 96；若内伤先结算则 97-4+5=98。
    assert_eq!(state.p1.core.hp, 96);
    assert_eq!(state.p1.status.internal_injury, 4);
}

#[test]
fn turn_start_phase_table_keeps_documented_order() {
    use super::flow::{TurnStartPhase, TURN_START_PHASES};
    let index = |phase: TurnStartPhase| {
        TURN_START_PHASES
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("phase must be present in the table")
    };
    // 断肠曲负面状态反射（IL_0701）→ 吞天赤眼兽等 chance hooks（IL_0a8a）
    // → “下回合加防”兑现（IL_0c42）→ 内伤 tick（IL_1a4c）。
    assert!(
        index(TurnStartPhase::TurnStartTuneEffects) < index(TurnStartPhase::TurnStartChanceHooks)
    );
    assert!(index(TurnStartPhase::TurnStartChanceHooks) < index(TurnStartPhase::NextTurnDefense));
    assert!(index(TurnStartPhase::NextTurnDefense) < index(TurnStartPhase::InternalInjuryTick));
    // 回合开始治疗先于内伤 tick：治疗先封顶、内伤再扣血。
    assert!(index(TurnStartPhase::TurnStartHealing) < index(TurnStartPhase::InternalInjuryTick));
    // 水月剑阵快照（MarkSpiritTurtleFootwork）必须先于 buff duration tick
    // （BuffDurationTicks，其会递减阵层数本身）再进入防御衰减（DefenseDecay，
    // 读的是快照）。快照驱动局部默认 0 = “衰减生效”，乱序会静默改变行为。
    assert!(
        index(TurnStartPhase::MarkSpiritTurtleFootwork) < index(TurnStartPhase::BuffDurationTicks)
    );
    assert!(index(TurnStartPhase::BuffDurationTicks) < index(TurnStartPhase::DefenseDecay));
}

#[test]
fn turn_end_phase_table_keeps_documented_hook_order() {
    use super::flow::{TurnEndPhase, TURN_END_PHASES};
    let index = |phase: TurnEndPhase| {
        TURN_END_PHASES
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("phase must be present in the table")
    };
    // 融会/荣辉 turn-end 钩子与 hook_trace.rs 的 ReplayTurnEndHookReceipt
    // 稳定顺序一致：ronghui → mirageRonghui → formations → waterMomentum，
    // statusDecay 产出被紧随的 fengMoPhysique 消费。
    assert!(index(TurnEndPhase::Ronghui) < index(TurnEndPhase::MirageRonghui));
    assert!(index(TurnEndPhase::MirageRonghui) < index(TurnEndPhase::Formations));
    assert!(index(TurnEndPhase::Formations) < index(TurnEndPhase::WaterMomentum));
    assert!(index(TurnEndPhase::WaterMomentum) < index(TurnEndPhase::StatusDecay));
    assert!(index(TurnEndPhase::StatusDecay) < index(TurnEndPhase::FengMoPhysique));
    assert!(index(TurnEndPhase::FengMoPhysique) < index(TurnEndPhase::LedgerReset));
}

#[test]
fn turn_end_formation_phase_table_keeps_documented_order() {
    use super::formations::{TurnEndFormationPhase, TURN_END_FORMATION_PHASES};
    let index = |phase: TurnEndFormationPhase| {
        TURN_END_FORMATION_PHASES
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("phase must be present in the table")
    };
    // 回春曲先把上限撑开，再让迷魂阵的大段治疗结算（oracle：
    // c0f73571469dd090/round-14 turn35）。
    assert!(
        index(TurnEndFormationPhase::RejuvenationTune) < index(TurnEndFormationPhase::FlowerMaze)
    );
}

#[test]
fn card_play_phase_table_keeps_documented_order() {
    use super::flow::{CardPlayPhase, CARD_PLAY_PHASES};
    let index = |phase: CardPlayPhase| {
        CARD_PLAY_PHASES
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("phase must be present in the table")
    };
    assert!(index(CardPlayPhase::PreflightResolution) < index(CardPlayPhase::PrepareTransaction));
    assert!(index(CardPlayPhase::PrepareTransaction) < index(CardPlayPhase::ExternalRepeatSources));
    assert!(index(CardPlayPhase::ExternalRepeatSources) < index(CardPlayPhase::PrimaryEffect));
    assert!(index(CardPlayPhase::PrimaryEffect) < index(CardPlayPhase::SpiritFormationEcho));
    assert!(index(CardPlayPhase::SpiritFormationEcho) < index(CardPlayPhase::FinishTransaction));
}

#[test]
fn card_effect_phase_table_keeps_documented_order() {
    use super::flow::{CardEffectPhase, CARD_EFFECT_PHASES};
    let index = |phase: CardEffectPhase| {
        CARD_EFFECT_PHASES
            .iter()
            .position(|candidate| *candidate == phase)
            .expect("phase must be present in the table")
    };
    // 出牌前钩子 → 效果体 → 行动判定 → 出牌后钩子 → 收栈。
    assert!(index(CardEffectPhase::OpenInvocation) < index(CardEffectPhase::PlayCardEntry));
    assert!(index(CardEffectPhase::PlayCardEntry) < index(CardEffectPhase::PreBodyHooks));
    assert!(index(CardEffectPhase::PreBodyHooks) < index(CardEffectPhase::Body));
    assert!(index(CardEffectPhase::Body) < index(CardEffectPhase::ActionAgain));
    assert!(index(CardEffectPhase::ActionAgain) < index(CardEffectPhase::AfterCardHooks));
    assert!(index(CardEffectPhase::AfterCardHooks) < index(CardEffectPhase::CloseInvocation));
}
