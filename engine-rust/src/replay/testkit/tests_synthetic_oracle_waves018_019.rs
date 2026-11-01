use super::cards_dream_mirage::DreamMirageValue;
use super::cards_mirage_ronghui::{
    MirageRonghuiValue, SYNTHETIC_ORACLE_WAVE018_CARD_IDS, SYNTHETIC_ORACLE_WAVE019_CARD_IDS,
};
use super::support::{normalized_base_id, other_param};
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};
use std::collections::{BTreeMap, BTreeSet};

fn basic_attack() -> CardDefinition {
    original_config::original_card_definition(0).expect("missing basic attack")
}

fn original_card(id: i64) -> CardDefinition {
    original_config::original_card_definition(id)
        .unwrap_or_else(|| panic!("missing original card {id}"))
}

fn deck(ids: &[i64]) -> Vec<CardDefinition> {
    super::test_support::resize_deck(
        ids.iter().map(|id| original_card(*id)).collect(),
        basic_attack(),
    )
}

fn player(
    cards: Vec<CardDefinition>,
    active_slot_count: usize,
    last_round_exp: i64,
) -> FixturePlayer {
    let mut permanent_buff_temp_datas = BTreeMap::new();
    permanent_buff_temp_datas.insert("10024".to_string(), 100);
    FixturePlayer {
        level: 5,
        base_max_hp: 500,
        extra_max_hp: None,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        xian_mo_strategies: Vec::new(),
        xian_mo_temp_datas: Default::default(),
        active_slot_count,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: None,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas,
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp,
        hand_cards: Vec::new(),
        cards,
    }
}

fn fixture(p1_ids: &[i64], p1_active: usize, last_round_exp: i64) -> BattleFixture {
    super::test_support::default_fixture(
        player(deck(p1_ids), p1_active, last_round_exp),
        player(deck(&[0]), 1, 0),
    )
}

fn state(p1_ids: &[i64], p1_active: usize, last_round_exp: i64) -> ReplayState {
    ReplayState::test_from_fixture(&fixture(p1_ids, p1_active, last_round_exp))
}

fn apply_body(state: &mut ReplayState, card_id: i64, slot: usize) {
    let card = original_card(card_id);
    let base_id = normalized_base_id(&card);
    assert!(state
        .apply_synthetic_oracle_mirage_ronghui_card_effect(
            PlayerSide::P1,
            &card,
            slot,
            false,
            base_id,
        )
        .is_some());
}

#[test]
fn temporary_semantics_mirage_copy_reads_physical_had_used_and_completes_before_settlement() {
    let mut first_use = state(&[173, 12], 2, 0);
    apply_body(&mut first_use, 173, 0);
    assert_eq!(first_use.p1.turn.extra_actions, 0);
    assert!(first_use.p1.deck.slots[0].used);

    let mut repeated = state(&[173, 12], 2, 0);
    repeated.p1.deck.slots[0].used = true;
    apply_body(&mut repeated, 173, 0);
    assert_eq!(repeated.p1.turn.extra_actions, 1);

    let mut damaging_copy = state(&[173, 0], 2, 0);
    apply_body(&mut damaging_copy, 173, 0);
    assert_eq!(damaging_copy.p2.core.hp, 497);
    assert_eq!(
        damaging_copy.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalActualDamage,),
        3,
    );
}

#[test]
fn mirage_ronghui_register_exactly_fifty_two_logical_ids() {
    let all: Vec<_> = SYNTHETIC_ORACLE_WAVE018_CARD_IDS
        .into_iter()
        .chain(SYNTHETIC_ORACLE_WAVE019_CARD_IDS)
        .collect();
    assert_eq!(all.len(), 52);
    assert_eq!(all.iter().copied().collect::<BTreeSet<_>>().len(), 52);
    for card_id in all {
        let card = original_card(card_id);
        assert_eq!(normalized_base_id(&card), card_id, "card {card_id}");
    }
}

#[test]
fn last_round_exp_selects_card_323_high_count() {
    let card = original_card(20_323);
    let threshold = other_param(&card, 1);
    let expected = other_param(&card, 2);
    let mut state = state(&[20_323], 1, threshold);
    apply_body(&mut state, 20_323, 0);
    assert_eq!(state.p1.identity.last_round_exp, threshold);
    assert_eq!(
        state.mirage_ronghui_value(
            PlayerSide::P1,
            MirageRonghuiValue::OrdinarySwordActionAgainCards,
        ),
        expected,
    );
}

#[test]
fn six_yao_and_conversion_hooks_settle_on_each_resource_gain() {
    let mut six_yao = state(&[20_131], 1, 0);
    apply_body(&mut six_yao, 20_131, 0);
    let multiplier =
        six_yao.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::SixYaoFanDamage);
    six_yao.p2.core.defense = multiplier * 2;
    let before = six_yao.p2.core.hp;
    six_yao.modify_star_power(PlayerSide::P1, 3);
    assert_eq!(six_yao.p2.core.defense, 0);
    assert_eq!(before - six_yao.p2.core.hp, multiplier);

    let mut sharpness = state(&[20_315], 1, 0);
    apply_body(&mut sharpness, 20_315, 0);
    let before_hp = sharpness.p2.core.hp;
    let before_max_hp = sharpness.p2.core.max_hp;
    sharpness.gain_sharpness(PlayerSide::P1, 4);
    assert_eq!(before_hp - sharpness.p2.core.hp, 8);
    assert_eq!(before_max_hp - sharpness.p2.core.max_hp, 8);
    assert_eq!(sharpness.p1.sword.sharpness, 0);

    let mut healing = state(&[20_316], 1, 0);
    apply_body(&mut healing, 20_316, 0);
    healing.p1.core.hp -= 20;
    let hp = healing.p1.core.hp;
    let sharpness_before = healing.p1.sword.sharpness;
    healing.modify_actor_hp(PlayerSide::P1, 7, false, false);
    assert_eq!(healing.p1.core.hp, hp);
    assert_eq!(healing.p1.sword.sharpness, sharpness_before + 7);
}

#[test]
fn molten_ring_applies_to_global_defense_and_target_max_hp_paths() {
    let mut state = state(&[20_135], 1, 0);
    apply_body(&mut state, 20_135, 0);
    let ring = state.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::MoltenRing);
    state.gain_defense(PlayerSide::P1, 3);
    assert_eq!(state.p1.core.defense, 3 + ring);
    let before = state.p2.core.max_hp;
    state.modify_target_max_hp(PlayerSide::P1, -3);
    assert_eq!(before - state.p2.core.max_hp, 3 + ring);
}

#[test]
fn cannot_gain_hp_expires_and_nine_heavens_revives_once() {
    let mut blocked = state(&[174], 1, 0);
    apply_body(&mut blocked, 174, 0);
    blocked.p2.core.hp -= 20;
    let hp = blocked.p2.core.hp;
    blocked.modify_actor_hp(PlayerSide::P2, 7, false, false);
    assert_eq!(blocked.p2.core.hp, hp);
    blocked.p2.tick_turn_end_statuses();
    blocked.modify_actor_hp(PlayerSide::P2, 7, false, false);
    assert_eq!(blocked.p2.core.hp, hp + 7);

    let mut revive = state(&[172], 1, 0);
    apply_body(&mut revive, 172, 0);
    revive.p1.core.hp = -5;
    assert_eq!(revive.death_winner(), None);
    assert_eq!(revive.p1.core.hp, 64);
    assert_eq!(
        revive.mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::NineHeavensRevive),
        0,
    );
}

#[test]
fn fire_phoenix_revive_sets_hp_to_its_buff_value_after_gaining_max_hp() {
    let mut revive = state(&[99_000_207], 1, 0);
    revive.test_execute_one_card(PlayerSide::P1);
    assert_eq!(revive.p1.fate.fire_phoenix_revive_hp, 10);
    revive.p1.core.max_hp = 80;
    revive.p1.core.hp = -5;

    assert_eq!(revive.death_winner(), None);
    assert_eq!(revive.p1.core.max_hp, 90);
    assert_eq!(revive.p1.core.hp, 10);
    assert_eq!(revive.p1.fate.fire_phoenix_revive_hp, 0);
}

#[test]
fn following_star_slots_wrap_over_active_slots_only() {
    let mut state = state(&[4_020_032, 20_130], 2, 0);
    state.p1.astrology.star_slots = vec![2, 5];
    state.add_following_star_slots(PlayerSide::P1, 0, 2);
    assert_eq!(state.p1.core.anima, 0);
    assert!(state.p1.astrology.star_slots.contains(&0));
    assert!(state.p1.astrology.star_slots.contains(&1));
}

#[test]
fn following_star_slots_follow_reverse_card_direction_on_the_active_grid() {
    let mut state = state(&[0, 0, 0, 0], 4, 0);
    state.p1.astrology.star_slots.clear();
    state.p1.fate.reverse_card_direction = 1;

    state.add_following_star_slots(PlayerSide::P1, 0, 2);

    assert_eq!(state.p1.astrology.star_slots, vec![3, 2]);
}
