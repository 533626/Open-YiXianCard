use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture, test_card};
use super::{Element, ReplayState};
use crate::fixture::{FinalHp, FixtureExpected};
use crate::model::PlayerSide;

fn placeholder_expected() -> FixtureExpected {
    FixtureExpected {
        winner_side: PlayerSide::P1,
        actor_turn_count: 1,
        hp_delta_p1_minus_p2: 0,
        final_hp: None,
    }
}

#[test]
fn fate_148_converts_one_layer_from_each_positive_attack_bonus_gain() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );
    fixture.players.p1.fate_strategies = vec![148];
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.gain_attack_bonus(PlayerSide::P1, 1);
    assert_eq!(state.p1.core.attack_bonus, 0);
    assert_eq!(state.p1.elements.wood_thorn, 1);

    state.gain_attack_bonus(PlayerSide::P1, 4);
    assert_eq!(state.p1.core.attack_bonus, 3);
    assert_eq!(state.p1.elements.wood_thorn, 2);
}

#[test]
fn fate_151_reads_its_own_temp_switch_and_heals_configured_hp() {
    let mut active = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );
    active.players.p1.fate_strategies = vec![151];
    active
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10023".into(), 9);

    let active_state = ReplayState::test_from_fixture(&active);
    // 10023 体魄把 maxHp 抬到 39，ModifyHp(otherParams[1]=9) 才能从 30 补满。
    // 开关关闭时仍回血，只跳过辟邪。
    assert_eq!(active_state.p1.core.hp, 39);
    assert_eq!(active_state.p1.core.max_hp, 39);
    assert_eq!(active_state.p1.fate.exorcism, 2);

    active
        .players
        .p1
        .fate_strategy_temp_datas
        .insert("151".into(), 1);
    let disabled_state = ReplayState::test_from_fixture(&active);
    assert_eq!(disabled_state.p1.core.hp, 39);
    assert_eq!(disabled_state.p1.fate.exorcism, 0);
}

#[test]
fn fate_411_earth_activation_grants_configured_defense() {
    let mut fixture = minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );
    fixture.players.p1.fate_strategies = vec![411];
    let mut state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(state.p1.core.defense, 0);
    state.activate_element(PlayerSide::P1, Element::Earth);
    // FateStrategyConfig(411).otherParams[0] = 8（24811621）。
    assert_eq!(state.p1.core.defense, 8);
}

#[test]
fn fate_338_reuses_the_solitary_void_golden_book_deck_transform() {
    let golden_book = test_card(215, 215, "孤虚金书");
    let mut fixture = minimal_fixture(
        filler_cards(golden_book),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );
    fixture.players.p1.active_slot_count = 3;
    fixture.players.p1.fate_strategies = vec![338];

    let state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 0).id, 215);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 1).id, 217);
    assert_eq!(state.test_actor_card(PlayerSide::P1, 2).id, 216);
}

#[test]
fn card_329_sets_hp_through_defense_and_guard_then_exhausts() {
    let mut fixture = minimal_fixture(
        filler_cards(test_card(329, 329, "死劫")),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 29,
            final_hp: Some(FinalHp { p1: 30, p2: 1 }),
        },
    );
    fixture.players.p2.initial_defense = 30;
    fixture.players.p2.initial_guard = 2;
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 1);
    assert_eq!(state.p2.core.defense, 30);
    assert_eq!(state.p2.core.guard, 2);
    assert!(state.p1.deck.slots[0].skipped);
}

#[test]
fn card_372_opening_activates_all_elements_and_runs_each_deck_passive() {
    let mut ding_feng_bo = test_card(372, 372, "定风波•解封");
    ding_feng_bo.attack = Some(3);
    ding_feng_bo.attack_count = Some(2);
    ding_feng_bo.other_params = vec![5].into();
    let fixture = minimal_fixture(
        filler_cards(ding_feng_bo),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );

    let state = ReplayState::test_from_fixture(&fixture);

    assert_eq!(state.p1.elements.activated_elements.len(), 5);
    assert_eq!(state.p1.elements.activated_metal, 1);
    assert_eq!(state.p1.elements.activated_water, 1);
    assert_eq!(state.p1.elements.activated_wood, 1);
    assert_eq!(state.p1.elements.activated_fire, 1);
    assert_eq!(state.p1.elements.activated_earth, 1);
    assert_eq!(state.p2.core.hp, 5);
}

#[test]
fn card_371_attack_count_uses_wu_xing_active_number_stack_sum() {
    // Card_371.cs: attackCount + GetWuXingActiveNumber().
    // Two wood activations: distinct types=1, stack sum=2 → 2+2=4 segments.
    let mut winds = test_card(371, 371, "定风波");
    winds.attack = Some(4);
    winds.attack_count = Some(2);
    let fixture = minimal_fixture(
        filler_cards(winds),
        filler_cards(basic_attack_test_card()),
        placeholder_expected(),
    );
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p2.core.hp = 300;
    state.p2.core.max_hp = 300;
    state.activate_element(PlayerSide::P1, Element::Wood);
    state.activate_element(PlayerSide::P1, Element::Wood);
    assert_eq!(state.p1.elements.activated_elements.len(), 1);
    assert_eq!(state.p1.elements.activated_wood, 2);
    assert_eq!(state.wu_xing_active_number(PlayerSide::P1), 2);

    state.test_play_actor_turn();
    assert_eq!(state.p1.turn.attack_segments_performed, 4);
    assert_eq!(state.p2.core.hp, 284);
}
