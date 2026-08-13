use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::MirageRonghuiValue;
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

pub(super) fn filler_cards(active: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![active];
    while cards.len() < DECK_SIZE {
        cards.push(basic_attack_test_card());
    }
    cards
}
pub(super) fn basic_attack_test_card() -> CardDefinition {
    let mut card = test_card(0, 0, "普通攻击");
    card.attack = Some(3);
    card
}

pub(super) fn test_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
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

#[test]
fn other_param_or_defaults_only_when_the_index_is_missing() {
    let mut card = test_card(999, 999, "other-param contract");
    card.other_params = vec![0, -3];

    assert_eq!(support::other_param_or(&card, 0, 1), 0);
    assert_eq!(support::other_param_or(&card, 1, 1), -3);
    assert_eq!(support::other_param_or(&card, 2, 1), 1);
    assert_eq!(support::other_param(&card, 2), 0);
}

#[test]
fn strength_grass_attack_bonus_passes_through_fate_148_conversion() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10011".to_string(), 1);
    fixture.players.p1.fate_strategies.push(148);

    let converted = ReplayState::test_from_fixture(&fixture);
    assert_eq!(converted.p1.core.attack_bonus, 0);
    assert_eq!(converted.p1.elements.wood_thorn, 1);

    fixture.players.p1.fate_strategies.clear();
    let control = ReplayState::test_from_fixture(&fixture);
    assert_eq!(control.p1.core.attack_bonus, 1);
    assert_eq!(control.p1.elements.wood_thorn, 0);
}

#[test]
fn earth_fiend_defense_uses_actual_damage_not_fire_blade_max_hp_loss() {
    let mut earth_fiend = test_card(1_000_030, 1_000_030, "地煞剑");
    earth_fiend.attack = Some(8);
    let mut fixture = minimal_fixture(
        filler_cards(earth_fiend),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents.push(67);
    let mut state = ReplayState::test_from_fixture(&fixture);

    let earth_fiend = state.test_actor_card(PlayerSide::P1, 0);
    state.begin_effect_invocation(
        PlayerSide::P1,
        &earth_fiend,
        &earth_fiend,
        &earth_fiend,
        0,
        0,
        effect_invocation::EffectInvocationKind::Played,
        true,
    );
    state.test_apply_card_effect(PlayerSide::P1, &earth_fiend, 0);

    assert_eq!(state.p2.core.max_hp, 29);
    assert_eq!(state.p2.core.hp, 21);
    assert_eq!(state.p1.core.defense, 8);
    state.end_effect_invocation(
        PlayerSide::P1,
        effect_invocation::EffectInvocationKind::Played,
    );
}

#[test]
fn parity_events_sample_completed_phase_boundaries() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 3,
            final_hp: None,
        },
    );
    fixture.players.p1.initial_defense = 6;

    let run = run_replay_fixture_with_parity_events(&fixture).expect("fixture replays");
    let kinds = run
        .events
        .iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            ReplayEventKind::BattleStart,
            ReplayEventKind::TurnStart,
            ReplayEventKind::CardCompleted,
            ReplayEventKind::TurnEnd,
            ReplayEventKind::BattleEnd,
        ]
    );
    assert_eq!(run.events[0].p1.defense, 6);
    assert_eq!(run.events[1].p1.defense, 3);
    assert_eq!(run.events[3].p1.defense, 3);
    assert_eq!(run.completed_checkpoint_count, 4);
}

#[test]
fn parity_events_do_not_invent_turn_end_after_a_lethal_card() {
    let mut lethal = basic_attack_test_card();
    lethal.attack = Some(40);
    let fixture = minimal_fixture(
        filler_cards(lethal),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 30,
            final_hp: None,
        },
    );

    let run = run_replay_fixture_with_parity_events(&fixture).expect("fixture replays");
    let kinds = run
        .events
        .iter()
        .map(|event| event.kind)
        .collect::<Vec<_>>();

    assert_eq!(
        kinds,
        vec![
            ReplayEventKind::BattleStart,
            ReplayEventKind::TurnStart,
            ReplayEventKind::CardCompleted,
            ReplayEventKind::BattleEnd,
        ]
    );
    assert_eq!(run.termination_cause, ReplayTerminationCause::CardLethal);
    assert_eq!(run.completed_checkpoint_count, 3);

    let raw_run = run_replay_fixture_with_events(&fixture).expect("fixture replays");
    assert_eq!(raw_run.completed_checkpoint_count, 3);
    assert_eq!(raw_run.events.len() - 1, 4);
}

#[test]
fn dream_fire_formation_7000090_terminates_during_action_again() {
    let mut p1_cards = vec![
        original_config::original_card_definition(7_040_090).expect("梦·火灵阵 config exists"),
        original_config::original_card_definition(7_040_077).expect("梦·五行刺 config exists"),
        original_config::original_card_definition(0).expect("basic attack config exists"),
    ];
    p1_cards.resize_with(DECK_SIZE, || {
        original_config::original_card_definition(0).expect("basic attack config exists")
    });
    let p2_cards = (0..DECK_SIZE)
        .map(|_| {
            original_config::original_card_definition(20_000)
                .expect("level-three basic attack config exists")
        })
        .collect();
    let expected = FixtureExpected {
        winner_side: PlayerSide::P1,
        actor_turn_count: 51,
        hp_delta_p1_minus_p2: 103,
        final_hp: None,
    };
    let mut fixture = minimal_fixture(p1_cards, p2_cards, expected.clone());
    fixture.source = Some(crate::fixture::FixtureSource {
        steam_build: Some(super::original_build_profile::project_target_steam_build().to_string()),
        ..Default::default()
    });
    fixture.max_actor_turns = None;
    fixture.players.p1.level = 5;
    fixture.players.p1.base_max_hp = 75;
    fixture.players.p1.extra_max_hp = Some(100);
    fixture.players.p1.character_id = Some(1_000_001);
    fixture.players.p1.active_slot_count = 3;
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 5);
    fixture.players.p2.level = 5;
    fixture.players.p2.base_max_hp = 75;
    fixture.players.p2.extra_max_hp = Some(100);
    fixture.players.p2.character_id = Some(2_000_001);
    fixture.players.p2.active_slot_count = 8;
    fixture
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 5);

    let run = run_replay_fixture_with_parity_events(&fixture).expect("fixture replays");

    assert_eq!(run.summary.winner_side, expected.winner_side);
    assert_eq!(run.summary.actor_turn_count, expected.actor_turn_count);
    assert_eq!(
        run.summary.hp_delta_p1_minus_p2,
        expected.hp_delta_p1_minus_p2
    );
    assert_eq!(
        run.termination_cause,
        ReplayTerminationCause::ActionAgainLethal
    );
    let terminal = run.events.last().expect("battleEnd event exists");
    assert_eq!(terminal.kind, ReplayEventKind::BattleEnd);
    assert_eq!((terminal.p1.hp, terminal.p2.hp), (100, -3));
}

#[test]
fn turn_end_water_momentum_lethal_has_distinct_termination_cause() {
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 43,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.elements.water_momentum = 40;

    let summary = state.test_run();

    assert_eq!(summary.winner_side, PlayerSide::P1);
    assert_eq!(state.test_final_hp(), (30, -13));
    assert_eq!(
        state.termination_cause,
        Some(ReplayTerminationCause::TurnEndLethal)
    );
    assert_eq!(state.completed_checkpoint_count, 4);
}

#[test]
fn turn_start_internal_injury_lethal_has_distinct_termination_cause() {
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P2,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: -30,
            final_hp: None,
        },
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.status.internal_injury = 40;

    let summary = state.test_run();

    assert_eq!(summary.winner_side, PlayerSide::P2);
    assert_eq!(state.test_final_hp(), (-10, 30));
    assert_eq!(
        state.termination_cause,
        Some(ReplayTerminationCause::TurnStartLethal)
    );
    assert_eq!(state.completed_checkpoint_count, 2);
}

#[test]
fn battle_start_nonpositive_hp_still_terminates_after_turn_start() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 32,
            final_hp: None,
        },
    );
    fixture.players.p2.base_max_hp = 3;
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10009".to_string(), 5);

    let run = run_replay_fixture_with_parity_events(&fixture).expect("fixture replays");

    assert_eq!(
        run.termination_cause,
        ReplayTerminationCause::TurnStartLethal
    );
    assert_eq!(run.summary.actor_turn_count, 1);
    assert_eq!(run.events[0].kind, ReplayEventKind::BattleStart);
    assert_eq!(run.events[0].p2.hp, -2);
    assert_eq!(run.events[1].kind, ReplayEventKind::TurnStart);
    assert_eq!(run.events[2].kind, ReplayEventKind::BattleEnd);
    assert_eq!(run.completed_checkpoint_count, 2);
}

#[test]
fn actor_order_controls_protective_talisman_and_golden_shuttle_orchid() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10009".to_string(), 3);
    fixture
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10047".to_string(), 2);

    let state = ReplayState::test_from_fixture(&fixture);

    // P1 enters OnBattleStarted first: its 金梭兰 hits P2 before P2's
    // 护身法宝 is granted. P2 then receives both guard layers, untouched.
    assert_eq!(state.p2.core.hp, 27);
    assert_eq!(state.p2.core.guard, 2);

    // Reverse ownership: P1's guard is ready before P2's 金梭兰, so one of
    // its two layers absorbs the damage.
    fixture.players.p1.permanent_buff_temp_datas.clear();
    fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10047".to_string(), 2);
    fixture.players.p2.permanent_buff_temp_datas.clear();
    fixture
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10009".to_string(), 3);
    let reverse = ReplayState::test_from_fixture(&fixture);
    assert_eq!(reverse.p1.core.hp, 30);
    assert_eq!(reverse.p1.core.guard, 1);
}

#[test]
fn surviving_last_turn_uses_max_turn_termination_cause() {
    let fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 3,
            final_hp: None,
        },
    );

    let run = run_replay_fixture_with_parity_events(&fixture).expect("fixture replays");

    assert_eq!(run.termination_cause, ReplayTerminationCause::MaxTurn);
    assert_eq!(run.completed_checkpoint_count, 4);
}

#[test]
fn forget_worries_reduces_every_negative_status_by_the_configured_amount() {
    let mut card = test_card(218, 218, "忘忧");
    card.other_params = vec![0, 0, 2];
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
    state.p1.status.internal_injury = 3;
    state.p1.status.weakness = 1;
    state.p1.status.flaw = 2;
    state.p2.status.internal_injury = 2;
    state.p2.status.weakness = 3;
    state.p2.status.flaw = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.status.internal_injury, 1);
    assert_eq!(state.p1.status.weakness, 0);
    assert_eq!(state.p1.status.flaw, 0);
    assert_eq!(state.p2.status.internal_injury, 0);
    assert_eq!(state.p2.status.weakness, 1);
    assert_eq!(state.p2.status.flaw, 0);
}

pub(super) fn minimal_fixture(
    p1_cards: Vec<CardDefinition>,
    p2_cards: Vec<CardDefinition>,
    expected: FixtureExpected,
) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected,
        max_actor_turns: Some(1),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers {
            p1: FixturePlayer {
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
                cards: p1_cards,
            },
            p2: FixturePlayer {
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
                cards: p2_cards,
            },
        },
    }
}

#[test]
fn talent_208_ling_qi_ben_yong_grants_agility_on_momentum_gain() {
    let mut fixture = minimal_fixture(
        filler_cards(crate::replay::support::basic_attack_card()),
        filler_cards(crate::replay::support::basic_attack_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.character_id = Some(4_000_005);
    fixture.players.p1.talents = vec![204, 208];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.modify_momentum(PlayerSide::P1, 4);

    assert_eq!(state.p1.beng.momentum, 4);
    assert_eq!(state.p1.turn.agility, 2);
    assert_eq!(state.p1.beng.quan_stance, 1);
    assert_eq!(state.p1.beng.momentum_gain_agility_triggered, 1);

    state.modify_momentum(PlayerSide::P1, 1);
    assert_eq!(state.p1.beng.momentum, 5);
    assert_eq!(state.p1.turn.agility, 2);
}

#[test]
fn fate_strategy_on_play_basic_attack_modifies_before_attack_body() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.fate_strategies = vec![32, 36];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.attack_bonus = 4;
    state.p2.core.guard = 5;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.attack_bonus, 2);
    assert_eq!(state.p2.core.attack_bonus, 1);
    assert_eq!(state.p2.core.guard, 1);
    assert_eq!(state.p2.core.hp, 30);
}

#[test]
fn fate_strategy_on_play_clear_heart_sword_embryo_requires_exact_card_id_19() {
    let mut fixture = minimal_fixture(
        filler_cards(test_card(19, 19, "澄心剑胚")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.fate_strategies = vec![100, 101, 102, 103, 324];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.status.internal_injury = 3;
    state.p1.status.weakness = 1;
    state.p1.status.flaw = 2;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.sword.sword_energy, 2);
    assert_eq!(state.p1.status.internal_injury, 1);
    assert_eq!(state.p1.status.weakness, 0);
    assert_eq!(state.p1.status.flaw, 0);
    assert_eq!(state.p1.sword.water_month_sword_formation, 2);
    assert_eq!(state.p1.sword.cloud_sea, 4);
    assert_eq!(state.p1.sword.cloud_chain, 2);

    let mut upgraded_fixture = minimal_fixture(
        filler_cards(test_card(10_019, 19, "澄心剑胚")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    upgraded_fixture.players.p1.fate_strategies = vec![100, 101, 102, 103, 324];
    let mut upgraded_state = ReplayState::test_from_fixture(&upgraded_fixture);
    upgraded_state.p1.status.internal_injury = 3;

    upgraded_state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(upgraded_state.p1.core.anima, 0);
    assert_eq!(upgraded_state.p1.sword.sword_energy, 0);
    assert_eq!(upgraded_state.p1.status.internal_injury, 3);
    assert_eq!(upgraded_state.p1.sword.water_month_sword_formation, 0);
    assert_eq!(upgraded_state.p1.sword.cloud_sea, 0);
    assert_eq!(upgraded_state.p1.sword.cloud_chain, 0);
}

#[test]
fn clear_heart_sword_embryo_reads_talent_10095_and_30094() {
    let mut clear_heart = test_card(19, 19, "澄心剑胚");
    clear_heart.attack = Some(7);
    let mut fixture = minimal_fixture(
        filler_cards(clear_heart),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents = vec![10_095, 30_094];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 25);
    assert_eq!(state.p1.core.anima, 2);
    assert_eq!(state.p1.sword.sword_intent, 4);
}

#[test]
fn frenzy_heart_talent_gives_clear_heart_bonus_attack_and_frenzy_identity() {
    let clear_heart =
        original_card_definition_by_id(19).expect("missing current-build clear heart sword embryo");
    let mut fixture = minimal_fixture(
        filler_cards(clear_heart),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents = vec![10_096];
    fixture.players.p1.active_slot_count = 2;
    fixture.players.p2.base_max_hp = 40;

    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert_eq!(state.p1.sword.next_cards_as_frenzy_sword, 1);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.sword.frenzy_sword, 2);
    assert_eq!(state.p1.sword.next_cards_as_frenzy_sword, 0);
    assert_eq!(state.p1.sword.next_cards_as_frenzy_sword_effective_count, 0);
}

#[test]
fn sword_formation_guard_talent_triggers_on_first_slot() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents = vec![30_057];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 9);
    assert_eq!(state.p1.sword.water_month_sword_formation, 1);
}

#[test]
fn sword_formation_guard_talent_levels_stack_independently() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.talents = vec![57, 20_057];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 7);
    assert_eq!(state.p1.sword.water_month_sword_formation, 2);
}

#[test]
fn target_anima_loss_triggers_spirit_control_defense() {
    let mut fixture = minimal_fixture(
        filler_cards(test_card(7, 7, "旋灯占卦")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.cards[0].hexagram = Some(4);
    fixture.players.p1.cards[0].other_params = vec![2, 1];
    fixture.players.p2.initial_anima = 3;

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.turn.spirit_control_anima_loss_defense = 2;
    state.p2.turn.adaptation = 1;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.anima, 2);
    assert_eq!(state.p2.core.defense, 3);
}

#[test]
fn upgraded_regenerative_body_heals_two_on_first_slot() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.base_max_hp = 20;
    fixture.players.p1.talents = vec![10_149];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 10;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.core.hp, 12);
    assert_eq!(state.p2.core.hp, 27);
}

#[test]
fn regenerative_body_stacks_each_active_realm_talent_on_first_slot() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.base_max_hp = 20;
    fixture.players.p1.talents = vec![149, 10_149];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.hp = 10;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.max_hp, 22);
    assert_eq!(state.p1.core.hp, 13);
}

#[test]
fn fate_strategy_on_play_family_slot_cost_and_anima_burn_hooks() {
    let mut cloud_fixture = minimal_fixture(
        filler_cards(test_card(3, 3, "云剑•崩雪")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    cloud_fixture.players.p1.fate_strategies = vec![97, 320];
    let mut cloud_state = ReplayState::test_from_fixture(&cloud_fixture);
    cloud_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(cloud_state.p1.sword.cloud_sea, 1);
    assert_eq!(cloud_state.p1.sword.water_month_sword_formation, 1);

    let mut frenzy_fixture = minimal_fixture(
        filler_cards(test_card(2, 2, "狂剑•炎舞")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    frenzy_fixture.players.p1.fate_strategies = vec![325];
    let mut frenzy_state = ReplayState::test_from_fixture(&frenzy_fixture);
    frenzy_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(frenzy_state.p1.core.anima, 1);

    let mut metal_fixture = minimal_fixture(
        filler_cards(test_card(97_000_034, 145, "金灵测试")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    metal_fixture.players.p1.fate_strategies = vec![128];
    let mut metal_state = ReplayState::test_from_fixture(&metal_fixture);
    metal_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(metal_state.p1.core.anima, 1);

    let mut eighth_cards = vec![basic_attack_test_card(); DECK_SIZE];
    eighth_cards[7] = test_card(91_000_121, 145, "第八格测试");
    let mut eighth_fixture = minimal_fixture(
        eighth_cards,
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    eighth_fixture.players.p1.active_slot_count = 8;
    eighth_fixture.players.p1.fate_strategies = vec![121];
    let mut eighth_state = ReplayState::test_from_fixture(&eighth_fixture);
    eighth_state.p1.deck.queue = vec![7];
    eighth_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(eighth_state.p2.status.internal_injury, 2);

    let mut hp_cost = test_card(91_000_153, 145, "耗生命测试");
    hp_cost.hp_cost = Some(10);
    let mut hp_cost_fixture = minimal_fixture(
        filler_cards(hp_cost),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    hp_cost_fixture.players.p1.fate_strategies = vec![153];
    let mut hp_cost_state = ReplayState::test_from_fixture(&hp_cost_fixture);
    hp_cost_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(hp_cost_state.p1.core.hp, 20);
    assert_eq!(hp_cost_state.p2.core.hp, 26);

    let mut tear_fixture = minimal_fixture(
        filler_cards(test_card(91_000_069, 7_000_069, "朱雀之泪测试")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    tear_fixture.players.p1.initial_anima = 6;
    tear_fixture.players.p1.fate_strategies = vec![345];
    let mut tear_state = ReplayState::test_from_fixture(&tear_fixture);
    tear_state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(tear_state.p2.core.hp, 24);
    assert_eq!(tear_state.p2.core.max_hp, 24);
}

#[path = "tests_battle_start_hooks.rs"]
mod battle_start_hooks;
#[path = "tests_temporary_effects.rs"]
mod temporary_effects;
#[path = "tests_upgrades_and_transforms.rs"]
mod upgrades_and_transforms;
