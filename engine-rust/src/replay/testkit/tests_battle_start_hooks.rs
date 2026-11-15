use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, DECK_SIZE};
use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// Typed rows for the data-driven battle-start table: every loop iteration of
/// the migrated test below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `stored_7000094`).
struct Talent199Case {
    stored_card_id: i64,
    expected_element: Option<Element>,
}

#[test]
fn fate_strategy_battle_start_hooks_apply_through_fixture_startup() {
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
    fixture.players.p1.fate_strategies = vec![89, 138, 143, 146];

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(state.p1.sword.cloud_chain, 1);
    assert_eq!(state.p1.sword.cloud_sea, 2);
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Wood));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Earth));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Fire));
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.core.defense, 4);
    assert_eq!(state.p2.core.hp, 28);
    assert_eq!(state.p2.core.max_hp, 28);
}

#[test]
fn talent_199_activates_bottle_element_for_wood_spirit_revival() {
    let revival = CardDefinition {
        id: 7_000_018,
        base_id: Some(7_000_018),
        name: "木灵•复苏".to_string().into(),
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
        other_params: vec![3, 1].into(),
    };
    let mut fixture = minimal_fixture(
        filler_cards(crate::replay::support::basic_attack_card()),
        filler_cards(revival.clone()),
        FixtureExpected {
            winner_side: PlayerSide::P2,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.first_player_side = PlayerSide::P2;
    fixture.players.p2.talents = vec![199];
    fixture
        .players
        .p2
        .talent_card_params
        .insert("199".to_string(), vec![7_000_018]);
    fixture.players.p2.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    assert!(state
        .p2
        .elements
        .activated_elements
        .contains(&super::super::Element::Wood));
    state.p2.core.hp = 92;
    state.p2.core.max_hp = 94;
    state.p2.core.anima = 1;
    state.p2.deck.queue = vec![0];
    state.current_actor = PlayerSide::P2;
    let _ = state.test_execute_one_card(PlayerSide::P2);
    assert_eq!(state.p2.core.hp, 94);
}

const TALENT_199_CASES: &[Talent199Case] = &[
    Talent199Case {
        stored_card_id: 7_000_094,
        expected_element: None,
    },
    Talent199Case {
        stored_card_id: 7_000_097,
        expected_element: Some(Element::Wood),
    },
];

fn run_talent_199_case(case: &Talent199Case) {
    let stored_card_id = case.stored_card_id;
    let expected_element = case.expected_element;
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
    fixture.players.p1.talents = vec![199];
    fixture
        .players
        .p1
        .talent_card_params
        .insert("199".to_string(), vec![stored_card_id]);

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(
        state.p1.elements.activated_elements.first().copied(),
        expected_element,
        "stored card {stored_card_id} should resolve by original config name"
    );
}

crate::data_driven_cases! {
    table: TALENT_199_CASES,
    run: run_talent_199_case,
    suite: "replay::tests::battle_start_hooks::talent_199_resolves_out_of_deck_card_names_without_effect_dispatch",
    valid: talent_199_resolves_out_of_deck_case_table_is_valid,
    cases: [
        talent_199_resolves_out_of_deck_card_names_without_effect_dispatch_stored_7000094 => 0,
        talent_199_resolves_out_of_deck_card_names_without_effect_dispatch_stored_7000097 => 1,
    ]
}

#[test]
fn wood_spirit_all_growth_grants_hp_and_turn_start_attack_bonus() {
    let card = CardDefinition {
        id: 10_134,
        base_id: Some(134),
        name: "木灵•万物生".to_string().into(),
        card_type: Some(crate::model::OriginalEnumValue {
            value: 3,
            name: "Sustain".to_string().into(),
        }),
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: Some(-1),
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![9, 1].into(),
    };
    let mut fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(crate::replay::support::basic_attack_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.active_slot_count = 1;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.anima = 2;
    state.p1.deck.queue = vec![0];
    state.current_actor = PlayerSide::P1;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, 39);
    assert_eq!(state.p1.core.max_hp, 39);
    assert_eq!(state.p1.elements.wood_spirit_all_growth, 1);
    assert_eq!(state.p1.elements.wood_spirit_all_growth_attack, 1);
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.attack_bonus, 1);
}

#[test]
fn xian_mo_7103_samples_hp_before_11004_physique() {
    // 26315169 OnBattleStart goto 链：7103(IL_15e9…)→…→7123→IL_1fdb(11004)，
    // 7103 先采样开局 HP(+hp/10)，11004 后加体魄(+3 HP/max)。反例：4809
    // (pa_1e09f55f) p2 开局 98→110/110(AddHpCount=12，
    // 11004 +3 then 7103 +9)；若 11004 先则 98→101→111/111。
    // 最小契约(28 HP)：7103 先 28+2=30 再 11004 +3=33；反之 28+3=31
    // 再 7103 +3=34。
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
    fixture.players.p2.base_max_hp = 28;
    fixture.players.p2.permanent_buff_temp_datas = Default::default();
    fixture.players.p2.xian_mo_strategies = vec![7103, 11004];

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(state.p2.core.physique, 3);
    assert_eq!(state.p2.core.hp, 33);
    assert_eq!(state.p2.core.max_hp, 33);
}

#[test]
fn xian_mo_16024_battle_start_param_branches() {
    // XianMoStrategyFunctions.cs:598-617：16024 开局 switch(GetNextParam())，
    // 0 → 当前血 10% 加血加血上限，1 → 10% 加防，3 → 1 层虚弱(101)，
    // 其余（含空队列 -1）无效果；otherParams=[10,10,1,1]。
    // 最小契约(30 HP)：0 → 33/33，1 → 防+3，3 → 虚弱 1。
    for (tape, hp, max_hp, def, weakness) in [
        (vec![0], 33, 33, 0, 0),
        (vec![1], 30, 30, 3, 0),
        (vec![3], 30, 30, 0, 1),
        (Vec::new(), 30, 30, 0, 0),
        (vec![2], 30, 30, 0, 0),
        (vec![8], 30, 30, 0, 0),
        (vec![-1], 30, 30, 0, 0),
    ] {
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
        fixture.players.p1.xian_mo_strategies = vec![16024];
        fixture.decision_tape = tape.clone();
        let state = ReplayState::test_from_fixture(&fixture);
        assert_eq!(state.p1.core.hp, hp, "tape {tape:?}");
        assert_eq!(state.p1.core.max_hp, max_hp, "tape {tape:?}");
        assert_eq!(state.p1.core.defense, def, "tape {tape:?}");
        assert_eq!(state.p1.status.weakness, weakness, "tape {tape:?}");
        // 无效果值同样消费一个参数（含显式 -1），空队列则无可消费。
        let expected_remaining = tape.len().saturating_sub(1);
        assert_eq!(
            state.decision_tape.len(),
            expected_remaining,
            "tape {tape:?}"
        );
    }
}

#[test]
fn xian_mo_16024_two_sided_consumes_in_first_player_order() {
    // 首位完整开局先于次位（BattleExecuter.cs:1800-1801），双 16024 按
    // [first, second] 顺序各消费一个参数：tape [0,1] 首位取 0、次位取 1。
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
    fixture.players.p1.xian_mo_strategies = vec![16024];
    fixture.players.p2.xian_mo_strategies = vec![16024];
    fixture.decision_tape = vec![0, 1];
    let state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(state.p1.core.hp, 33);
    assert_eq!(state.p1.core.max_hp, 33);
    assert_eq!(state.p2.core.defense, 3);
    assert_eq!(state.decision_tape.len(), 0);
}

// Merged from tests_strict_battle_start.rs (2026-11-14).

fn original_card(id: i64) -> CardDefinition {
    super::super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    super::super::test_support::original_card(0)
}

fn full_deck(first: CardDefinition, second: CardDefinition) -> Vec<CardDefinition> {
    let mut cards = vec![first, second];
    cards.resize_with(DECK_SIZE, basic_attack);
    cards
}

fn player(cards: Vec<CardDefinition>, active_slot_count: usize) -> FixturePlayer {
    super::super::test_support::make_player(cards, 5, 100, None, active_slot_count, None)
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::super::test_support::default_fixture(p1, p2)
}

fn missing_opponent_grid_fixture() -> BattleFixture {
    fixture(
        player(vec![basic_attack(), original_card(369)], 2),
        player(vec![basic_attack()], 1),
    )
}

fn missing_opening_config_fixture() -> BattleFixture {
    let mut star_chess = original_card(389);
    star_chess.rarity = Some(99);
    fixture(
        player(full_deck(star_chess, basic_attack()), 2),
        player(full_deck(basic_attack(), basic_attack()), 2),
    )
}

fn assert_all_strict_replay_surfaces_reject(fixture: &BattleFixture, expected: &str) {
    let errors = [
        (
            "strict constructor",
            ReplayState::<true>::from_fixture(fixture, true)
                .unwrap_err()
                .to_string(),
        ),
        (
            "summary",
            run_replay_fixture(fixture).unwrap_err().to_string(),
        ),
        (
            "raw events",
            run_replay_fixture_with_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "parity events",
            run_replay_fixture_with_parity_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "detailed events",
            run_replay_fixture_with_detailed_events(fixture)
                .unwrap_err()
                .to_string(),
        ),
        (
            "fallible evaluation",
            evaluate_replay_fixture_fallible(fixture).unwrap_err(),
        ),
        (
            "fallible events",
            run_replay_fixture_with_events_fallible(fixture).unwrap_err(),
        ),
    ];

    for (surface, error) in errors {
        assert!(
            error.contains(expected),
            "{surface} returned an unrelated error: {error}"
        );
        assert!(
            error.contains("turn=0"),
            "{surface} continued beyond battle-start: {error}"
        );
    }
}

#[test]
fn every_public_replay_surface_rejects_battle_start_missing_grid_decision() {
    assert_all_strict_replay_surfaces_reject(
        &missing_opponent_grid_fixture(),
        "card:369:opponent same-grid card",
    );
}

#[test]
fn every_public_replay_surface_rejects_battle_start_missing_card_config() {
    assert_all_strict_replay_surfaces_reject(
        &missing_opening_config_fixture(),
        "card:389:opening replacement definition",
    );
}

#[test]
fn fixture_file_entry_rejects_opening_error_before_the_cli_can_run_a_turn() {
    let fixture = missing_opening_config_fixture();
    fixture
        .validate()
        .expect("file fixture must be structurally valid");
    let nonce = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock after epoch")
        .as_nanos();
    let path = std::env::temp_dir().join(format!(
        "yixian-strict-battle-start-{}-{nonce}.json",
        std::process::id()
    ));
    fs::write(
        &path,
        serde_json::to_vec(&fixture).expect("serialize fixture"),
    )
    .expect("write temporary fixture");

    let result = run_replay_fixture_file(&path);
    fs::remove_file(&path).expect("remove temporary fixture");

    let error = result.expect_err("CLI fixture entry must reject opening config gaps");
    assert!(error
        .to_string()
        .contains("card:389:opening replacement definition"));
    assert!(error.to_string().contains("turn=0"));
}
