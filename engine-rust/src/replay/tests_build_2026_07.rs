use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::MirageRonghuiValue;
use super::player::{
    AfterHpModifyPhase, HP_MUTATION_SCOPE_EXCLUSIONS, ORIGINAL_AFTER_HP_MODIFY_PHASES,
    ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES,
};
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, OriginalEnumValue, PlayerSide, DECK_SIZE};

mod combat_cards;

fn card(id: i64, base_id: i64, name: &str) -> CardDefinition {
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

fn basic_attack() -> CardDefinition {
    let mut attack = card(0, 0, "普通攻击");
    attack.attack = Some(3);
    attack
}

fn deck(active: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![active];
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn deck_with_cards(mut cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
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

fn fixture(p1_cards: Vec<CardDefinition>, p2_cards: Vec<CardDefinition>) -> BattleFixture {
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
            p1: player(p1_cards),
            p2: player(p2_cards),
        },
    }
}

#[test]
fn fate_divination_applies_opening_and_played_hp_loss_without_touching_defense() {
    let fate_divination =
        original_card_definition_by_id(11_000_001).expect("missing current-build fate divination");
    assert_eq!(fate_divination.base_id, Some(11_000_001));
    assert_eq!(fate_divination.other_params, vec![6, 2]);
    assert!(ReplayState::card_has_opening_effect(11_000_001));
    let mut battle = fixture(deck(fate_divination), deck(basic_attack()));
    battle.players.p2.initial_defense = 9;

    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.core.hp, 28);
    assert_eq!(state.p2.core.defense, 9);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p2.core.hp, 22);
    assert_eq!(state.p2.core.defense, 9);
}

#[test]
fn rolling_stone_seal_opening_activates_earth_and_metal() {
    let rolling_stone_seal =
        original_card_definition_by_id(10_057).expect("missing rolling stone seal");
    assert_eq!(rolling_stone_seal.base_id, Some(57));
    assert!(ReplayState::card_has_opening_effect(57));

    let state =
        ReplayState::test_from_fixture(&fixture(deck(rolling_stone_seal), deck(basic_attack())));

    assert_eq!(state.p1.elements.activated_earth, 1);
    assert_eq!(state.p1.elements.activated_metal, 1);
}

#[test]
fn all_goes_well_loses_one_duration_at_each_owner_turn_start() {
    let all_goes_well =
        original_card_definition_by_id(11_000_013).expect("missing current-build all goes well");
    let battle = fixture(deck(all_goes_well), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.astrology.all_goes_well, 3);

    for expected in [2, 1, 0] {
        state.apply_turn_start_buff_decrements(PlayerSide::P1);
        assert_eq!(state.p1.astrology.all_goes_well, expected);
    }
}

#[test]
fn spirit_formation_echo_repeats_level_one_effect() {
    let wood_array = original_card_definition_by_id(7_020_036).expect("missing wood array");
    let mut fixture = fixture(deck(wood_array), deck(basic_attack()));
    fixture.players.p1.base_max_hp = 50;
    fixture.players.p1.fate_strategies = vec![135];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_play_actor_turn();

    assert_eq!(state.p1.formations.spirit_formation_echo, 0);
    assert_eq!(state.p1.status.cannot_act, 0);
    assert_eq!(state.p1.elements.wood_array, 6);
    assert_eq!(state.p1.core.max_hp, 80);
    assert_eq!(state.p1.core.hp, 54);
}

#[test]
fn extreme_water_spirit_spring_rain_reuses_spring_rain_and_action_again_config() {
    let spring_rain = original_card_definition_by_id(20_428).expect("missing extreme spring rain");
    let mut fixture = fixture(deck(spring_rain), deck(basic_attack()));
    fixture.players.p1.base_max_hp = 20;
    fixture.players.p1.initial_anima = 1;

    let mut state = ReplayState::test_from_fixture(&fixture);
    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again);
    assert_eq!(state.p1.core.anima, 0);
    // 24589371 rotation: Card_428.cs 行为类型变更——旧「生命及上限+
    // otherParams[0]」（与卡 17 同构）→ 新「水势+otherParams[0]、海潮+
    // otherParams[1]」。20428 otherParams [6,1]：水势+6、海潮+1，不再加生命。
    assert_eq!(state.p1.elements.water_momentum, 6);
    assert_eq!(state.p1.core.max_hp, 20);
    assert_eq!(state.p1.core.hp, 20);
    assert_eq!(state.p1.fate.tide, 1);
    assert!(state.p1.deck.slots[0].skipped);
}

#[test]
fn endless_staff_stance_rewrites_stance_switch_to_momentum_limit_momentum_and_defense() {
    let mut shift_stance = card(222, 222, "合劲换式");
    shift_stance.attack = Some(5);
    shift_stance.other_params = vec![2, 3];
    let mut fixture = fixture(deck(shift_stance), deck(basic_attack()));
    fixture.players.p1.fate_strategies = vec![335];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| player.beng.quan_stance = 1);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.turn.agility, 5);
    assert_eq!(state.p1.core.defense, 3);
    assert_eq!(state.p1.beng.momentum, 1);
    assert_eq!(state.p1.beng.momentum_limit, 7);
    assert_eq!(state.p1.beng.quan_stance, 1);
    assert_eq!(state.p1.beng.gun_stance, 0);
}

#[test]
fn dream_thunder_hexagram_art_restores_the_current_build_loss_ledger() {
    let mut dream_thunder = card(4_030_088, 4_000_088, "梦•御雷卦诀");
    dream_thunder.attack = Some(1);
    dream_thunder.random_attack = Some(9);
    dream_thunder.hexagram = Some(3);
    let mut fixture = fixture(deck(dream_thunder), deck(basic_attack()));
    fixture.players.p1.initial_anima = 0;
    fixture.decision_tape = vec![5];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| player.astrology.hexagram = 1);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.astrology.hexagram, 1);
    assert_eq!(state.p1.astrology.lost_hexagram, 1);
    assert_eq!(state.p1.astrology.hexagram_effective_count, 1);
    assert_eq!(state.p2.core.hp, 25);
}

#[test]
fn spirit_pivot_sword_formation_reuses_card_window_sword_intent() {
    let mut formation = card(1_010_092, 1_000_092, "灵枢剑阵");
    formation.attack = Some(1);
    formation.anima = Some(2);
    formation.defense = Some(2);
    formation.other_params = vec![1, 1];
    let mut fixture = fixture(deck(formation), deck(basic_attack()));
    fixture.players.p1.initial_anima = 1;
    fixture.players.p1.initial_defense = 23;
    fixture.players.p2.base_max_hp = 114;
    fixture.players.p2.initial_defense = 8;

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.sword.sword_intent = 2;
        player.turn.spirit_control_anima_gain_defense = 2;
    });
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 3);
    assert_eq!(state.p1.core.defense, 29);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p2.core.hp, 84);
    assert_eq!(state.p1.sword.sword_intent, 0);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
}

#[test]
fn meditation_opening_heal_updates_all_hp_gain_ledgers() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_003);
    battle.players.p1.talents = vec![179];
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 10);

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 33);
    assert_eq!(state.p1.core.max_hp, 40);
    assert_eq!(state.p1.status.meditation, 1);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 3);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 3);
    assert_eq!(state.p1.dream_mirage.hp_gain_event_count, 1);
}

#[test]
fn meditation_talent_and_fate_are_distinct_hp_mutations() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_003);
    battle.players.p1.talents = vec![179];
    battle.players.p1.fate_strategies = vec![161];
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 10);

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 39);
    assert_eq!(state.p1.core.max_hp, 40);
    assert_eq!(state.p1.status.meditation, 3);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 9);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 9);
    assert_eq!(state.p1.dream_mirage.hp_gain_event_count, 2);
}

#[test]
fn hp_mutation_receipt_separates_request_resolution_application_and_ledger() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 29;
    state.p1.core.max_hp = 30;
    state.p1.turn.adaptation = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, 5, false, false);

    assert_eq!(
        receipt,
        HpMutationReceipt {
            requested: 5,
            resolved: 7,
            applied: 1,
            ledger: 7,
            prevention: None,
        }
    );
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 7);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 7);
    assert_eq!(state.p1.dream_mirage.hp_gain_event_count, 1);

    state.p1.core.guard = 1;
    let guarded = state.mutate_actor_hp(PlayerSide::P1, -10, false, false);
    assert_eq!(
        guarded,
        HpMutationReceipt {
            requested: -10,
            resolved: 0,
            applied: 0,
            ledger: 0,
            prevention: Some(HpMutationPrevention::Guard),
        }
    );
    assert_eq!(state.p1.core.guard, 0);
}

#[test]
fn hp_gain_interceptors_follow_original_cannot_gain_conversion_and_revive_order() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.mirage_ronghui.cannot_gain_hp = 1;
    state.p1.mirage_ronghui.mirage_healing_conversion_turns = 1;

    let blocked = state.mutate_actor_hp(PlayerSide::P1, 6, false, false);
    assert_eq!(blocked, HpMutationReceipt::prevented(6));
    assert_eq!(state.p1.sword.sharpness, 0);

    state.p1.mirage_ronghui.cannot_gain_hp = 0;
    state.p1.core.hp = -3;
    state.p1.chance.cannot_revive = 1;
    let converted = state.mutate_actor_hp(PlayerSide::P1, 6, false, false);
    assert_eq!(converted, HpMutationReceipt::prevented(6));
    assert_eq!(state.p1.core.hp, -3);
    assert_eq!(state.p1.sword.sharpness, 6);

    state.p1.mirage_ronghui.mirage_healing_conversion_turns = 0;
    let cannot_revive = state.mutate_actor_hp(PlayerSide::P1, 6, false, false);
    assert_eq!(cannot_revive, HpMutationReceipt::prevented(6));
    assert_eq!(state.p1.core.hp, -3);
}

#[test]
fn revive_preserves_last_stand_intent_until_the_next_lethal_checkpoint() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 0;
    state.p1.fate.flame_soul_return = 1;
    state.p1.fate.last_stand_intent = 1;

    assert_eq!(state.death_winner(), None);
    assert_eq!(state.p1.core.hp, 15);
    assert_eq!(state.p1.fate.flame_soul_return, 0);
    assert_eq!(state.p1.fate.last_stand_intent, 1);
    assert_eq!(state.p1.fate.last_stand_unyielding, 0);

    state.p1.core.hp = 0;
    assert_eq!(state.death_winner(), None);
    assert_eq!(state.p1.fate.last_stand_intent, 0);
    assert_eq!(state.p1.fate.last_stand_unyielding, 1);
}

#[test]
fn lost_mind_rewrites_healing_before_adaptation_and_ledger_projection() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 5;
    state.p1.core.max_hp = 30;
    state.p1.status.lost_mind = 6;
    state.p1.turn.adaptation = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, 4, false, false);

    assert_eq!(
        receipt,
        HpMutationReceipt {
            requested: 4,
            resolved: 2,
            applied: 2,
            ledger: 2,
            prevention: None,
        }
    );
    assert_eq!(state.p1.core.hp, 7);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 2);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 2);
    assert_eq!(state.p1.dream_mirage.hp_gain_event_count, 1);
}

#[test]
fn resolved_overheal_consumes_wild_ferry_before_after_hp_hooks() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 30;
    state.p1.core.max_hp = 30;
    state.p1.fate.wild_ferry_seal = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, 5, false, false);

    assert_eq!(
        receipt,
        HpMutationReceipt {
            requested: 5,
            resolved: 5,
            applied: 0,
            ledger: 5,
            prevention: None,
        }
    );
    assert_eq!(state.p1.fate.wild_ferry_seal, 0);
    assert_eq!(state.p1.turn.extra_actions, 1);
}

#[test]
fn heavenly_secret_reverse_resolves_before_dream_unmoving_gain_damage() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.max_hp = 100;
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p1.dream_mirage.dream_defense_gain_damage = 100;
    state.p2.core.hp = 99;
    state.p2.core.max_hp = 100;
    state.p2.fate.graft_flowers_to_tree = 1;

    state.mutate_actor_hp(PlayerSide::P1, 10, false, false);

    assert_eq!(state.p2.fate.graft_flowers_to_tree, 0);
    assert_eq!(state.p2.core.hp, 90);
}

#[test]
fn heavenly_secret_reverse_hp_and_defense_gain_share_damage_kernel_discriminators() {
    for trigger in ["hp", "defense"] {
        for target_case in [
            "defense",
            "guard",
            "metal-iron-bone",
            "dismantle-fist",
            "graft-flowers",
        ] {
            let mut state = ReplayState::test_from_fixture(&fixture(
                deck(basic_attack()),
                deck(basic_attack()),
            ));
            state.p1.core.hp = 20;
            state.p1.core.max_hp = 100;
            state.p1.fate.heavenly_secret_reverse = 1;
            state.p2.core.hp = 60;
            state.p2.core.max_hp = 100;

            match target_case {
                "defense" => state.p2.core.defense = 4,
                "guard" => state.p2.core.guard = 1,
                "metal-iron-bone" => state.p2.elements.metal_iron_bone = 1,
                "dismantle-fist" => {
                    state.p2.fate.dismantle_move = 1;
                    state.p2.beng.quan_stance = 1;
                }
                "graft-flowers" => state.p2.fate.graft_flowers_to_tree = 1,
                _ => unreachable!(),
            }

            match trigger {
                "hp" => {
                    state.mutate_actor_hp(PlayerSide::P1, 20, false, false);
                }
                "defense" => {
                    state.gain_defense(PlayerSide::P1, 20);
                }
                _ => unreachable!(),
            }

            let expected = match target_case {
                "defense" => (54, 0, 0, 0),
                "guard" => (60, 0, 0, 0),
                "metal-iron-bone" | "dismantle-fist" => (55, 0, 0, 0),
                "graft-flowers" => (70, 0, 0, 0),
                _ => unreachable!(),
            };
            assert_eq!(
                (
                    state.p2.core.hp,
                    state.p2.core.defense,
                    state.p2.core.guard,
                    state.p2.fate.graft_flowers_to_tree,
                ),
                expected,
                "{trigger}/{target_case}",
            );
        }
    }
}

#[test]
fn heavenly_secret_reverse_defense_gain_resolves_before_dream_unmoving_damage() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p1.dream_mirage.dream_defense_gain_damage = 100;
    state.p2.core.hp = 99;
    state.p2.core.max_hp = 100;
    state.p2.fate.graft_flowers_to_tree = 1;

    state.gain_defense(PlayerSide::P1, 10);

    assert_eq!(state.p2.fate.graft_flowers_to_tree, 0);
    assert_eq!(state.p2.core.hp, 90);
}

#[test]
fn ke_yin_50147_rewards_each_actual_hp_change_but_not_pure_overheal() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.max_hp = 30;
    state.p1.identity.ke_yin_card_ids = vec![50_147];

    state.mutate_actor_hp(PlayerSide::P1, 5, false, false);
    assert_eq!(state.p1.core.defense, 1);

    state.p1.core.hp = state.p1.core.max_hp;
    state.mutate_actor_hp(PlayerSide::P1, 5, false, false);
    assert_eq!(state.p1.core.defense, 1);
}

#[test]
fn hp_cost_runs_resonance_50_healing_before_fate_149_physique() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.max_hp = 30;
    state.p1.identity.talent_resonance_id = Some(50);
    state.p1.identity.fate_strategies.push(149);

    let receipt = state.mutate_actor_hp(PlayerSide::P1, -5, true, false);

    assert_eq!(
        receipt,
        HpMutationReceipt {
            requested: -5,
            resolved: -5,
            applied: -5,
            ledger: -5,
            prevention: None,
        }
    );
    assert_eq!(state.p1.core.hp, 7);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 2);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 2);
    assert_eq!(state.p1.turn.lose_hp_count, 5);
    assert_eq!(state.p1.core.physique, 1);
}

#[test]
fn nested_yan_qi_heal_projects_each_receipt_once() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 10;
    state.p1.core.max_hp = 100;
    state.p1.fate.yan_qi = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, 5, false, false);

    assert_eq!(
        receipt,
        HpMutationReceipt {
            requested: 5,
            resolved: 5,
            applied: 5,
            ledger: 5,
            prevention: None,
        }
    );
    assert_eq!(state.p1.core.hp, 35);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 25);
    assert_eq!(state.p1.dream_mirage.turn_hp_gained, 25);
    assert_eq!(state.p1.dream_mirage.hp_gain_event_count, 2);
}

#[test]
fn original_after_hp_modify_phase_order_is_exhaustive_and_locked() {
    assert_eq!(
        ORIGINAL_AFTER_HP_MODIFY_PHASES,
        [
            AfterHpModifyPhase::SpiritTurtleFootwork,
            AfterHpModifyPhase::FirstHpLossReward,
            AfterHpModifyPhase::Talent64Defense,
            AfterHpModifyPhase::KeYin50147Defense,
            AfterHpModifyPhase::IceSnowLotus,
            AfterHpModifyPhase::DreamCliff,
            AfterHpModifyPhase::BloodCalamity,
            AfterHpModifyPhase::HpLossAttackCharge,
            AfterHpModifyPhase::YanQi,
            AfterHpModifyPhase::HpLossLedgers,
        ]
    );
    assert_eq!(
        ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES,
        [
            "XiaHuiHeKaiShiQianBuZaiSunShiShengMing",
            "ShiYu",
            "HongZaoZong",
            "DanHuangZong",
            "XianDanHuangZong",
        ]
    );
    assert_eq!(
        HP_MUTATION_SCOPE_EXCLUSIONS.map(|(name, _)| name),
        ["JiLuZhanDouZuiGaoShengMing", "fengRui"]
    );
}

#[test]
fn initial_battle_buff_fixture_validation_shares_the_runtime_allowlist() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    for name in ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES {
        battle
            .players
            .p1
            .initial_battle_buffs
            .insert(name.to_string(), 1);
    }
    battle
        .validate()
        .expect("every runtime BattleBuffType must be accepted by fixture validation");

    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("fengRui".to_string(), 1);
    let error = battle
        .validate()
        .expect_err("unsupported BattleBuffType must fail closed");
    assert_eq!(
        error.to_string(),
        concat!(
            "invalid fixture: p1 initialBattleBuffs contains unsupported BuffType: fengRui; ",
            "supported: XiaHuiHeKaiShiQianBuZaiSunShiShengMing, ShiYu, HongZaoZong, ",
            "DanHuangZong, XianDanHuangZong"
        )
    );
}

#[test]
fn fixture_validation_rejects_zero_active_slot_count() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.active_slot_count = 0;

    assert_eq!(
        battle
            .validate()
            .expect_err("zero activeSlotCount must fail closed")
            .to_string(),
        "invalid fixture: p1 activeSlotCount must be between 1 and 8, got 0"
    );
}

#[test]
fn fixture_validation_rejects_active_slot_count_above_deck_size() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.active_slot_count = DECK_SIZE + 1;

    assert_eq!(
        battle
            .validate()
            .expect_err("oversized activeSlotCount must fail closed")
            .to_string(),
        "invalid fixture: p1 activeSlotCount must be between 1 and 8, got 9"
    );
}

#[test]
fn fixture_validation_rejects_active_slot_count_above_cards_length() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.cards.truncate(3);
    battle.players.p1.active_slot_count = 4;

    assert_eq!(
        battle
            .validate()
            .expect_err("activeSlotCount beyond the physical cards must fail closed")
            .to_string(),
        "invalid fixture: p1 activeSlotCount 4 exceeds cards length 3"
    );
}

#[test]
fn valid_active_slot_count_is_preserved_without_runtime_clamping() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.active_slot_count = 3;
    battle
        .validate()
        .expect("activeSlotCount inside the validated deck must be accepted");

    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.deck.active_slot_count, 3);
    assert_eq!(state.p1.deck.queue, vec![0, 1, 2]);
    assert!(!include_str!("player.rs").contains("active_slot_count.clamp"));
}

#[test]
fn hp_and_max_hp_raw_primitives_have_one_executable_caller_each() {
    let player_source = include_str!("player.rs");
    let resources_source = include_str!("resources.rs");

    // Definition + exactly one call. This prevents future card bodies and
    // nested hooks from bypassing ReplayState's ordered public pipelines.
    assert_eq!(player_source.matches("apply_hp_delta_raw(").count(), 2);
    assert_eq!(
        [player_source, resources_source]
            .concat()
            .matches("apply_max_hp_delta_raw(")
            .count(),
        2
    );
    assert_eq!(
        player_source
            .matches("apply_after_hp_modify_pipeline(")
            .count(),
        2
    );
    assert!(!player_source.contains("modify_hp_raw"));
    assert!(!resources_source.contains("modify_hp_raw"));
}

#[test]
fn business_modules_cannot_write_core_resources_or_negative_status_outside_semantic_kernels() {
    const KERNEL_FILES: [&str; 3] = ["combat.rs", "player.rs", "resources.rs"];
    const ORIGINAL_DIRECT_WRITES: [(&str, &str); 9] = [
        // BattleCharacter.Init applies verified KeYin battle-start bonuses as
        // initial state, before ordinary ModifyMaxHp/ModifyHp semantics exist.
        ("battle_start.rs", "actor.core.max_hp += max_hp;"),
        ("battle_start.rs", "actor.core.hp += max_hp;"),
        // Original revive bodies and Card_* SetHp calls intentionally replace
        // state instead of entering the additive mutation pipelines.
        ("combat_core_outcome.rs", "actor.core.max_hp = 15;"),
        ("combat_core_outcome.rs", "actor.core.hp = 15;"),
        ("combat_core_outcome.rs", "actor.core.max_hp += revive_hp;"),
        (
            "cards_synthetic_full_scope_candidates.rs",
            "self.actor_mut(target_side).core.hp = 1;",
        ),
        (
            "cards_synthetic_oracle_verified.rs",
            "self.actor_mut(target_side).core.hp = hp;",
        ),
        (
            "cards_dream_fate.rs",
            "self.actor_mut(target_side).core.hp = -100;",
        ),
        (
            "chance_cards.rs",
            "self.actor_mut(target_side).core.hp = 0;",
        ),
    ];
    // 负面状态家族（含 379 血光之灾标记）只允许由状态内核（combat.rs）
    // 写入；tick_turn_end_statuses 的原始衰减随 player.rs 一起豁免。
    const NEGATIVE_STATUS_FIELDS: [&str; 9] = [
        "internal_injury",
        "weakness",
        "attack_reduction",
        "flaw",
        "entangle",
        "external_injury",
        "meditation",
        "lost_mind",
        "blood_calamity",
    ];
    const ORIGINAL_DIRECT_STATUS_WRITES: [(&str, &str); 1] = [
        // 原版 OnTurnStarted 内伤 tick（TurnStartPhase::InternalInjuryTick）的
        // transient 回滚：暂记层数发放时已走完整 ModifyBuffValue 记账，这里
        // 只把暂记部分原样剥掉；走 remove_actor_negative_status 会重复触发
        // 415 疯魔架势等移除钩子。
        ("flow.rs", "actor.status.internal_injury ="),
    ];

    /// True when any occurrence of `marker` on the line is followed by an
    /// assignment suffix (`+=`, `-=`, or `=`). Checking every occurrence
    /// (not just the first) keeps a same-line read-condition before a write
    /// from evading the scan.
    fn has_write_suffix(line: &str, marker: &str) -> bool {
        line.match_indices(marker).any(|(start, matched)| {
            let suffix = line[start + matched.len()..].trim_start();
            suffix.starts_with("+=")
                || suffix.starts_with("-=")
                || (suffix.starts_with('=') && !suffix.starts_with("=="))
        })
    }

    fn writes_core_resource(line: &str) -> bool {
        const FIELDS: [&str; 6] = ["hp", "max_hp", "defense", "anima", "temp_life", "guard"];
        FIELDS
            .iter()
            .any(|field| has_write_suffix(line, &format!(".core.{field}")))
    }

    fn writes_negative_status(line: &str, fields: &[&str]) -> bool {
        fields
            .iter()
            .any(|field| has_write_suffix(line, &format!(".status.{field}")))
    }

    // A same-line read-condition before a write must still be flagged; a
    // first-occurrence-only check would evaluate the read's suffix instead.
    assert!(has_write_suffix(
        "if actor.status.weakness > 0 { actor.status.weakness -= 1; }",
        ".status.weakness",
    ));
    assert!(has_write_suffix(
        "if actor.core.hp > 0 { actor.core.hp -= 1; }",
        ".core.hp",
    ));

    let replay_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/replay");
    let mut violations = Vec::new();
    let mut seen_allowlist = Vec::new();
    let mut seen_status_allowlist = Vec::new();
    let mut entries = std::fs::read_dir(&replay_dir)
        .expect("replay source directory must be readable")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("replay source entries must be readable");
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs")
            || file_name.starts_with("tests")
            || KERNEL_FILES.contains(&file_name)
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("replay source file must be readable");
        for (line_index, line) in source.lines().enumerate() {
            let line = line.trim();
            if line.starts_with("//") {
                continue;
            }
            if writes_core_resource(line) {
                if let Some(allowed) = ORIGINAL_DIRECT_WRITES
                    .iter()
                    .copied()
                    .find(|allowed| *allowed == (file_name, line))
                {
                    seen_allowlist.push(allowed);
                } else {
                    violations.push(format!("{file_name}:{}: {line}", line_index + 1));
                }
            }
            if writes_negative_status(line, &NEGATIVE_STATUS_FIELDS) {
                if let Some(allowed) = ORIGINAL_DIRECT_STATUS_WRITES
                    .iter()
                    .copied()
                    .find(|allowed| *allowed == (file_name, line))
                {
                    seen_status_allowlist.push(allowed);
                } else {
                    violations.push(format!("{file_name}:{}: {line}", line_index + 1));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "business modules must enter the semantic mutation kernels; unexpected direct writes:\n{}",
        violations.join("\n")
    );
    for allowed in ORIGINAL_DIRECT_WRITES {
        assert!(
            seen_allowlist.contains(&allowed),
            "stale direct-write allowlist entry: {}: {}",
            allowed.0,
            allowed.1
        );
    }
    for allowed in ORIGINAL_DIRECT_STATUS_WRITES {
        assert!(
            seen_status_allowlist.contains(&allowed),
            "stale direct-write allowlist entry: {}: {}",
            allowed.0,
            allowed.1
        );
    }
}

#[test]
fn negative_after_hp_modify_pipeline_dispatches_every_outcome_hook_once() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.core.hp = 100;
    state.p1.core.max_hp = 100;
    state.p1.turn.spirit_turtle_footwork = 2;
    state.p1.mirage_ronghui.first_hp_loss_reward = 3;
    state.p1.identity.talents.push(64);
    state.p1.identity.ke_yin_card_ids = vec![50_147, 50_147];
    state.p1.fate.ice_snow_lotus = 1;
    state.p1.dream_mirage.dream_cliff = 1;
    state.p1.status.blood_calamity = 1;
    state.p1.mirage_ronghui.hp_loss_attack_bonus_charges = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, -5, false, false);

    assert_eq!(receipt.applied, -5);
    assert_eq!(state.p1.core.anima, 5);
    assert_eq!(state.p1.turn.agility, 5);
    assert_eq!(state.p1.core.defense, 8);
    assert_eq!(state.p1.turn.next_turn_defense, 5);
    assert_eq!(state.p1.status.external_injury, 1);
    assert_eq!(state.p1.status.blood_calamity, 0);
    assert_eq!(state.p1.core.attack_bonus, 1);
    assert_eq!(state.p1.mirage_ronghui.hp_loss_attack_bonus_charges, 0);
    assert_eq!(state.p1.turn.lose_hp_times_count, 1);
    assert_eq!(state.p1.turn.lose_hp_count, 5);
}

#[test]
fn blood_calamity_external_injury_consumes_star_erosion() {
    // BattleCharacter.cs:9902-9905：血光之灾在失去生命后通过
    // ModifyBuffValue(WaiShang, 1) 发放外伤，因此会先吃掉星蚀。
    // oracle 锚点：hf-latest-32336000-fd629abd
    // 24a37c79e82e55ee/round-09 checkpoint[5]，星弈•虎首次攻击后
    // 外伤 3、星蚀清零；随后卡牌自身的虚弱只加 1。
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.status.blood_calamity = 1;
    state.p1.astrology.star_erosion = 2;

    state.mutate_actor_hp(PlayerSide::P1, -1, false, false);

    assert_eq!(state.p1.status.external_injury, 3);
    assert_eq!(state.p1.status.blood_calamity, 0);
    assert_eq!(state.p1.astrology.star_erosion, 0);
}

#[test]
fn buff_598_blocks_repeated_losses_without_consuming_guard_until_owner_turn_start() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.initial_guard = 1;
    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("XiaHuiHeKaiShiQianBuZaiSunShiShengMing".to_string(), 1);
    let mut state = ReplayState::test_from_fixture(&battle);

    for _ in 0..2 {
        let receipt = state.mutate_actor_hp(PlayerSide::P1, -8, false, false);
        assert_eq!((receipt.resolved, receipt.applied), (0, 0));
        assert_eq!(state.p1.core.hp, 30);
        assert_eq!(state.p1.core.guard, 1);
        assert_eq!(state.p1.hp_mutation.no_hp_loss_before_next_turn, 1);
    }

    state.apply_turn_start_buff_decrements(PlayerSide::P1);
    assert_eq!(state.p1.hp_mutation.no_hp_loss_before_next_turn, 0);

    assert_eq!(
        state
            .mutate_actor_hp(PlayerSide::P1, -8, false, false)
            .applied,
        0
    );
    assert_eq!(state.p1.core.guard, 0);
    assert_eq!(
        state
            .mutate_actor_hp(PlayerSide::P1, -8, false, false)
            .applied,
        -8
    );
    assert_eq!(state.p1.core.hp, 22);
}

#[test]
fn food_hp_modifiers_follow_original_order_and_one_shot_consumption() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("ShiYu".to_string(), 2);
    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("HongZaoZong".to_string(), 3);
    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("DanHuangZong".to_string(), 1);
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.hp = 10;
    state.p1.core.max_hp = 100;
    state.p1.status.lost_mind = 4;

    let first = state.mutate_actor_hp(PlayerSide::P1, 5, false, false);
    assert_eq!((first.resolved, first.applied), (7, 7));
    assert_eq!(state.p1.hp_mutation.appetite, 3);
    assert_eq!(state.p1.hp_mutation.egg_yolk_zongzi, 0);
    assert_eq!(state.p1.hp_mutation.red_date_zongzi, 0);

    let second = state.mutate_actor_hp(PlayerSide::P1, 5, false, false);
    assert_eq!((second.resolved, second.applied), (4, 4));
}

#[test]
fn immortal_egg_yolk_zongzi_consumes_per_selected_card_then_converts_at_max_rank() {
    let mut battle = fixture(
        deck(original_card_definition_by_id(0).expect("missing basic attack")),
        deck(basic_attack()),
    );
    battle.players.p1.base_max_hp = 100;
    battle.players.p2.base_max_hp = 100;
    battle
        .players
        .p1
        .initial_battle_buffs
        .insert("XianDanHuangZong".to_string(), 3);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.deck.slots[0].card.id, 10_000);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.deck.slots[0].card.id, 20_000);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.deck.slots[0].card.id, 20_000);
    assert_eq!(state.p1.hp_mutation.immortal_egg_yolk_zongzi, 0);
    assert_eq!(state.p1.hp_mutation.appetite, 2);
}

#[test]
fn card_324_max_hp_and_healing_both_enter_adaptation_kernels() {
    let mut vitality = card(324, 324, "幻生机绽放");
    vitality.other_params = vec![10, 1];
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(vitality.clone()), deck(basic_attack())));
    state.p1.turn.adaptation = 1;

    state.test_apply_card_effect(PlayerSide::P1, &vitality, 0);

    assert_eq!(state.p1.core.max_hp, 44);
    assert_eq!(state.p1.core.hp, 44);
    assert_eq!(state.p1.fate.mirage_vitality_bloom, 1);
}

#[test]
fn beng_quan_tu_shatter_expires_with_current_card() {
    let mut thrust = card(10_010_015, 10_000_015, "崩拳•突");
    thrust.attack = Some(10);
    thrust.other_params = vec![1];
    let mut battle = fixture(deck(thrust), deck(basic_attack()));
    battle.players.p2.initial_defense = 30;

    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_configure_p1(|player| player.beng.beng_quan_tu = 1);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 10);
    assert_eq!(state.p1.beng.beng_quan_tu, 1);
    assert_eq!(state.active_effect_shatter_defense(), 0);
    assert_eq!(state.p1.turn.next_attack_shatter_defense, 0);

    state.test_configure_p2(|player| player.core.defense = 30);
    state.apply_attack(PlayerSide::P1, 10, 0);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 20);
}

#[test]
fn stroke_of_genius_executes_global_temporary_card_definition() {
    let mut genius = card(6_010_011, 6_000_011, "神来之笔");
    genius.defense = Some(6);
    let mut battle = fixture(deck(genius), deck(basic_attack()));
    battle.decision_tape = vec![4_010_094];

    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 6);
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p2.core.hp, 24);
    assert_eq!(state.p2.status.flaw, 1);
}

#[test]
fn stroke_of_genius_reports_unknown_temporary_card_without_fallback() {
    let mut genius = card(6_010_011, 6_000_011, "神来之笔");
    genius.defense = Some(6);
    let mut battle = fixture(deck(genius), deck(basic_attack()));
    battle.decision_tape = vec![99_999_999];

    let mut state = ReplayState::test_from_fixture(&battle);
    state.fail_on_missing_decision = true;
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 6);
    assert!(state.evaluation_error.as_ref().is_some_and(|error| error
        .to_string()
        .contains("card:6000011 temporary card definition")));
}

#[test]
fn light_sword_attacks_before_anima_gain_hooks() {
    let mut light_sword = card(1_010_004, 1_000_004, "轻剑");
    light_sword.attack = Some(5);
    light_sword.anima = Some(2);
    let mut fixture = fixture(deck(light_sword), deck(basic_attack()));
    fixture.players.p1.talents = vec![68];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| player.turn.spirit_control_anima_gain_defense = 2);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 6);
    assert_eq!(state.p2.core.hp, 25);
}

#[test]
fn skipped_good_fortune_beginning_increases_max_hp_and_hp() {
    let mut good_fortune = card(11_010_005, 11_000_005, "吉运初显");
    good_fortune.other_params = vec![3, 4];
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.apply_opening_effect_for_card(PlayerSide::P1, &good_fortune, 0);

    assert_eq!(state.p1.core.max_hp, 34);
    assert_eq!(state.p1.core.hp, 34);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 4);
}

#[test]
fn active_good_fortune_beginning_initial_hp_counts_as_hp_gained() {
    let mut good_fortune = card(11_000_005, 11_000_005, "吉运初显");
    good_fortune.other_params = vec![7, 4];
    let mut battle = fixture(deck(good_fortune), deck(basic_attack()));
    battle.players.p1.talents = vec![64];
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.max_hp, 34);
    assert_eq!(state.p1.core.hp, 34);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 4);
}

#[test]
fn good_fortune_keeps_fate_strategy_27_pre_opening_hp_order() {
    let mut good_fortune = card(11_000_005, 11_000_005, "吉运初显");
    good_fortune.other_params = vec![7, 4];
    let mut battle = fixture(deck(good_fortune), deck(basic_attack()));
    battle.players.p1.talents = vec![64];
    battle.players.p1.fate_strategies = vec![27];
    battle.players.p1.hand_cards = vec![1];

    let state = ReplayState::test_from_fixture(&battle);

    // Fate 27 samples the entry 30 HP first (+3), then 吉运初显 adds 4.
    assert_eq!(state.p1.core.max_hp, 37);
    assert_eq!(state.p1.core.hp, 37);
    assert_eq!(state.p1.core.defense, 2);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 7);
}

#[test]
fn ui_event_projects_live_momentum_limit_and_slot_lifecycle() {
    let mut awe = card(10_000_051, 10_000_051, "威震四方");
    awe.card_type = Some(OriginalEnumValue {
        value: CARD_TYPE_SUSTAIN,
        name: "持续".to_string(),
    });
    awe.defense = Some(8);
    awe.other_params = vec![3, 3];
    let run = run_replay_fixture_with_ui_events(&fixture(deck(awe), deck(basic_attack())))
        .expect("UI replay should run");
    let completed = run
        .events
        .iter()
        .find(|event| event.kind == ReplayEventKind::CardCompleted && event.actor == PlayerSide::P1)
        .expect("missing p1 card-completed UI event");

    assert_eq!(completed.p1.parity.momentum, 3);
    assert_eq!(completed.p1.momentum_limit, 9);
    assert_eq!(completed.p1.card_queue, vec![0]);
    assert!(completed.p1.slots[0].had_used);
    assert!(completed.p1.slots[0].skipped);
}

#[test]
fn ui_snapshot_projects_last_element_queue_and_runtime_card_identity() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.elements.last_element = Some(Element::Wood);
    state.p1.deck.queue = vec![3, 1, 0];
    state.p1.deck.slots[0].card = card(7_010_001, 7_000_001, "木灵·芽");
    state.p1.deck.slots[0].used = true;
    state.p1.deck.slots[0].skipped = true;

    let snapshot = state.p1.ui_snapshot();

    assert_eq!(snapshot.last_element, Some("wood"));
    assert_eq!(snapshot.card_queue, vec![3, 1, 0]);
    assert_eq!(snapshot.slots[0].card_id, 7_010_001);
    assert_eq!(snapshot.slots[0].base_id, 7_000_001);
    assert_eq!(snapshot.slots[0].name, "木灵·芽");
    assert!(snapshot.slots[0].had_used);
    assert!(snapshot.slots[0].skipped);
}

#[test]
fn detailed_hp_ledgers_distinguish_amounts_from_event_counts() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.hp_mutation.add_hp_count = 12;
    state.p1.dream_mirage.hp_gain_event_count = 3;
    state.p1.turn.lose_hp_count = 9;
    state.p1.turn.lose_hp_times_count = 2;
    let entries = state.p1.detail_entries();
    let entry = |key| {
        entries
            .iter()
            .find(|entry| entry.key == key)
            .expect("missing HP ledger detail")
    };

    assert_eq!(
        (entry("addHpCount").label, entry("addHpCount").value),
        ("累计获得生命", 12)
    );
    assert_eq!(
        (
            entry("hpGainEventCount").label,
            entry("hpGainEventCount").value
        ),
        ("获得生命次数", 3)
    );
    assert_eq!(
        (entry("loseHpCount").label, entry("loseHpCount").value),
        ("累计失去生命", 9)
    );
    assert_eq!(
        (
            entry("loseHpTimesCount").label,
            entry("loseHpTimesCount").value
        ),
        ("失去生命次数", 2)
    );
}

#[test]
fn paint_finishing_touch_emits_a_temporary_upgrade_hook_step() {
    let mut paint = card(6_000_013, 6_000_013, "画龙点睛");
    paint.action_again = Some(true);
    paint.other_params = vec![1];
    let basic = original_card_definition_by_id(0).expect("missing basic attack");
    let mut battle = fixture(deck_with_cards(vec![paint, basic]), deck(basic_attack()));
    battle.players.p1.active_slot_count = 2;

    let trace = trace_replay_fixture_hooks(&battle).expect("hook trace");
    let upgrade = trace
        .steps
        .iter()
        .find(|step| step.category == ReplayHookCategory::TemporaryUpgrade)
        .expect("missing temporary-upgrade hook step");
    let paint_change = upgrade
        .p1_changes
        .iter()
        .find(|change| change.key == "paintFinishingTouch")
        .expect("missing paint-finishing-touch consumption");

    assert_eq!(upgrade.slot, Some(1));
    assert_eq!(upgrade.card_id, Some(10_000));
    assert_eq!(upgrade.card_name.as_deref(), Some("普通攻击"));
    assert_eq!(
        (paint_change.label, paint_change.before, paint_change.after),
        ("画龙点睛", 1, 0)
    );
}

// ---- 天机机制簇修复回归（build 24610558 批次 mirror-32219000-human-01）----

#[test]
fn heavenly_secret_reverse_decrements_at_owner_turn_start() {
    // BattleCharacter.cs:3874-3877：NiShi 与 ShunYing/TieGu 同块逐回合 -1。
    // 此前 Rust 只减 adaptation 不减 heavenly_secret_reverse，导致逆施
    // 永久生效（4085afbca05ac04f/round-15 winner 翻转）。
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.fate.heavenly_secret_reverse = 3;
    state.test_play_actor_turn();
    assert_eq!(state.p1.fate.heavenly_secret_reverse, 2);
    state.test_advance_actor();
    state.test_play_actor_turn();
    assert_eq!(state.p1.fate.heavenly_secret_reverse, 2); // 对方回合不减
}

#[test]
fn fate_cycle_charges_accumulate_across_casts() {
    // Card_11000021.cs:82 用 ModifyBuffValue(DongZhuJiXian, otherParams[1])
    // 累加：重复打出/多张命运轮回叠加跳过次数（a5a5585e91be7466/round-11）。
    let fate_cycle = original_card_definition_by_id(11_000_021).expect("missing 命运轮回");
    let basic = original_card_definition_by_id(0).expect("missing basic attack");
    let mut battle = fixture(
        deck_with_cards(vec![fate_cycle.clone(), fate_cycle, basic]),
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 3;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.fate_cycle, 4);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.fate.fate_cycle, 8);
}

#[test]
fn star_pulling_triggers_chati_opening_before_blood_calamity() {
    // 天星•牵引触发「此牌之后的 2 个开局」：察体(11000022) 应被计入
    // （TriggerOpening case 11000022 返回 true），随后才是卜命/血光之灾
    // （6af5a266765a510c/round-09：Rust 曾漏算察体而多触发血光之灾）。
    let mut star_pulling = card(11_000_025, 11_000_025, "天星•牵引");
    star_pulling.anima = Some(2);
    star_pulling.other_params = vec![2, 1];
    let mut body_observation = card(11_000_022, 11_000_022, "察体");
    body_observation.other_params = vec![5, 1];
    let mut blood_calamity = card(11_000_024, 11_000_024, "血光之灾");
    blood_calamity.other_params = vec![1, 1];
    let basic = original_card_definition_by_id(0).expect("missing basic attack");
    let mut battle = fixture(
        deck_with_cards(vec![
            star_pulling,
            basic.clone(),
            body_observation,
            basic.clone(),
            blood_calamity,
        ]),
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 5;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_execute_one_card(PlayerSide::P1);

    // 开局阶段察体已加 1 次，天星•牵引再触发 1 次 → 2 次碎防计数。
    // 旧实现漏算察体开局，这里只有 1。
    assert_eq!(state.p1.turn.next_attack_shatter_defense, 2);
}

#[test]
fn calamity_entanglement_opening_targets_trigger_grid() {
    // 天星•牵引以自身格位作 triggerGrid：厄劫缠身的「对方同格牌降级」
    // 目标是对方在 天星牌格位的卡，而不是厄劫缠身自己的格位
    // （BattleCharacter.cs:11132-11148；318453a623bc43d5/round-12）。
    let mut star_pulling = card(11_000_025, 11_000_025, "天星•牵引");
    star_pulling.anima = Some(2);
    star_pulling.other_params = vec![2, 1];
    let mut calamity = card(11_000_018, 11_000_018, "厄劫缠身");
    calamity.other_params = vec![4, 6];
    let basic = original_card_definition_by_id(0).expect("missing basic attack");
    let upgraded_opponent = original_card_definition_by_id(10_000).expect("missing 10_000");
    let mut battle = fixture(
        deck_with_cards(vec![star_pulling, calamity, basic.clone()]),
        deck_with_cards(vec![upgraded_opponent.clone(), basic.clone(), basic]),
    );
    battle.players.p1.active_slot_count = 3;
    battle.players.p2.active_slot_count = 3;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_execute_one_card(PlayerSide::P1);

    // 开局阶段厄劫缠身同格（1 号位）是基础牌 → 降级失败造成 6 伤害；
    // 天星•牵引触发时以 0 号位（天星牌同格）为目标 → 10000 降级为 0。
    assert_eq!(state.p2.deck.slots[0].card.id, 0);
    assert_eq!(state.p2.deck.slots[1].card.id, 0);
    assert_eq!(state.p2.core.hp, 24);
}
