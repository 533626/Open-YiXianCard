use super::test_support::{basic_deck, make_fixture, make_player, original_card};
use super::*;

fn player(cards: Vec<CardDefinition>) -> crate::fixture::FixturePlayer {
    let slots = cards.len();
    make_player(cards, 5, 100, None, slots, None)
}

fn fixture(p1: crate::fixture::FixturePlayer, p2: crate::fixture::FixturePlayer) -> BattleFixture {
    make_fixture(p1, p2, PlayerSide::P1, 1, 0, 1)
}

/// Typed rows for the data-driven pa_c7c95878 table: every loop iteration of the
/// migrated test below runs as an individually named `#[test]` with the
/// verbatim original body and assertions. The row identity is the generated
/// test name suffix (for example `before_neg49`).
struct WolfCrystalCase {
    before: i64,
    after: i64,
}

#[test]
fn parrot_refreshes_cheng_xin_using_the_copying_owner() {
    // Card_99000216:160-164 -> CardItem.RefreshSpecialCard:1668-1670
    // -> Card_19.UpdateCardInfo:420-455. The opponent's talent 92 is irrelevant.
    let mut p1 = player(basic_deck(original_card(99_000_216)));
    p1.talents = vec![92, 10_093, 20_094, 10_095];
    p1.talent_temp_datas.insert("92".into(), 8);
    let mut p2 = player(basic_deck(original_card(19)));
    p2.talents = vec![92, 20_093];
    p2.talent_temp_datas.insert("92".into(), 30);
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p1.core.hp = 80;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 82); // 7 + 8 + 3 attack.
    assert_eq!(
        (
            state.p1.core.hp,
            state.p1.core.anima,
            state.p1.core.attack_bonus
        ),
        (82, 2, 1)
    );
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.deck.slots[0].card.id, 99_000_216);
    assert_eq!(state.p2.deck.slots[0].card.defense, Some(9));
}

const WOLF_CRYSTAL_CASES: &[WolfCrystalCase] = &[
    WolfCrystalCase {
        before: -49,
        after: -42,
    },
    WolfCrystalCase {
        before: -15,
        after: -13,
    },
    WolfCrystalCase {
        before: -9,
        after: -8,
    },
    WolfCrystalCase {
        before: 5,
        after: 4,
    },
];

fn run_wolf_crystal_case(case: &WolfCrystalCase) {
    // Card_99000112:111 passes a signed product to CeilToInt, then
    // ModifyHp; OpenTongXiuHouShouBuChang permits healing below zero.
    let card = original_card(99_000_112);
    let f = fixture(
        player(basic_deck(card.clone())),
        player(basic_deck(original_card(0))),
    );
    let before = case.before;
    let after = case.after;
    let mut state = ReplayState::test_from_fixture(&f);
    state.p1.core.hp = before;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.core.hp, after);
    assert_eq!(state.p1.core.attack_bonus, 4);
}

crate::data_driven_cases! {
    table: WOLF_CRYSTAL_CASES,
    run: run_wolf_crystal_case,
    suite: "replay::tests_hf1012_d::wolf_crystal_uses_signed_current_hp_and_ceiling",
    valid: wolf_crystal_uses_signed_current_hp_case_table_is_valid,
    cases: [
        wolf_crystal_uses_signed_current_hp_and_ceiling_before_neg49 => 0,
        wolf_crystal_uses_signed_current_hp_and_ceiling_before_neg15 => 1,
        wolf_crystal_uses_signed_current_hp_and_ceiling_before_neg9 => 2,
        wolf_crystal_uses_signed_current_hp_and_ceiling_before_5 => 3,
    ]
}

#[test]
fn zero_anima_point_star_still_attacks_with_staff_stance() {
    // Card_1000027:62,80 calls Attack(0) after FanQu converted anima to physique.
    let mut p1 = player(basic_deck(original_card(99_000_216)));
    p1.talents = vec![204, 206];
    let p2 = player(basic_deck(original_card(1_000_027)));
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p1.beng.gun_stance = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p2.core.hp, 99);
    assert_eq!(state.p1.dream_mirage.total_actual_damage, 1);
}

#[test]
fn reverse_healing_damage_precedes_overflow_and_defense_reflection() {
    // ModifyHp:10107 NiShi, then :10112 LingYuanWaiFang; losing defense
    // reflects the first absorbed amount through TuLingJueBi (:10469-10471).
    let f = fixture(
        player(basic_deck(original_card(0))),
        player(basic_deck(original_card(0))),
    );
    let mut state = ReplayState::test_from_fixture(&f);
    state.p1.fate.heavenly_secret_reverse = 1;
    state.p1.astrology.ling_yuan_wai_fang = 1;
    state.p2.core.defense = 15;
    state.p2.elements.earth_cliff_counter = 1;

    state.modify_actor_hp(PlayerSide::P1, 20, false, false);

    assert_eq!(
        (state.p1.core.hp, state.p2.core.hp, state.p2.core.defense),
        (90, 85, 0)
    );
    assert_eq!(state.p1.turn.lose_hp_count, 10);
}

#[test]
fn vitality_bloom_fills_the_amplified_max_hp_without_overflow_damage() {
    // ModifyHp:10061-10072 assigns hp=maxHp after ShengJiZhanFang's
    // ModifyMaxHp; ShunYing also amplifies that maximum increase.
    let f = fixture(
        player(basic_deck(original_card(0))),
        player(basic_deck(original_card(0))),
    );
    let mut state = ReplayState::test_from_fixture(&f);
    state.p1.core.hp = 97;
    state.p1.core.max_hp = 106;
    state.p1.fate.vitality_bloom = 1;
    state.p1.turn.adaptation = 1;
    state.p1.astrology.ling_yuan_wai_fang = 1;

    let receipt = state.mutate_actor_hp(PlayerSide::P1, 11, false, false);

    assert_eq!(
        (state.p1.core.hp, state.p1.core.max_hp, state.p2.core.hp),
        (116, 116, 100)
    );
    assert_eq!(
        (receipt.resolved, receipt.applied, receipt.ledger),
        (16, 19, 16)
    );
    assert_eq!(receipt.effect_delta(), 16);

    state.p1.core.hp = 97;
    state.p1.core.max_hp = 106;
    assert_eq!(state.modify_actor_hp(PlayerSide::P1, 11, false, false), 16);
    assert_eq!(
        (state.p1.core.hp, state.p1.core.max_hp, state.p2.core.hp),
        (116, 116, 100)
    );
}

#[test]
fn nested_chain_attack_reads_live_temporary_frenzy_depth() {
    // CardActionBase:2704-2706 raises depth to 2 inside the copied formation;
    // IsKuangJian:12797-12800 rejects depth 2 when the granted stack is 1.
    let chain = original_card(1_000_064);
    let p1 = player(vec![original_card(1_000_108), chain.clone()]);
    let p2 = player(basic_deck(original_card(0)));
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p1.core.hp = 60;
    state.p1.sword.next_cards_as_frenzy_sword = 1;
    // At 100%, stale classification heals 1 on each of the four hits.
    // At 50%, truncation would hide that bug by making every heal zero.
    state.p1.sword.frenzy_sword_zero = 100;

    state.apply_temporary_card_effect(PlayerSide::P1, &chain, 1);

    assert_eq!((state.p1.core.hp, state.p2.core.hp), (60, 96));
    assert_eq!(state.p1.sword.next_cards_as_frenzy_sword_effective_count, 0);
}

#[test]
fn turtle_footwork_stays_suppressed_during_pending_hexagram_damage() {
    // OnTurnEnded:6326-6331 grants pending hexagram before :6395-6401
    // clears LingGuiMiZongBuShengXiao. The pending 1 hexagram deals 3 damage.
    let p1 = player(basic_deck(original_card(0)));
    let mut p2 = player(basic_deck(original_card(2_000_010)));
    p2.talents = vec![36, 195];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));
    state.p1.turn.spirit_turtle_footwork = 1;
    state.p2.formations.six_yao_formation = 3;
    state.current_actor = PlayerSide::P2;

    state.test_play_actor_turn();

    assert_eq!(
        (state.p1.core.hp, state.p1.core.anima, state.p1.turn.agility),
        (97, 0, 0)
    );
    assert_eq!(state.p1.turn.spirit_turtle_footwork_triggered, 0);
}

#[test]
fn fire_rope_orchid_clamps_hp_before_its_owner_opening_damage() {
    // OnBattleStarted:2052 HuoSuoLan precedes this owner's TriggerOpening.
    let mut p1 = player(basic_deck(original_card(11_000_023)));
    p1.permanent_buff_temp_datas.insert("10017".into(), 4);
    let p2 = player(basic_deck(original_card(0)));
    let state = ReplayState::test_from_fixture(&fixture(p1, p2));

    assert_eq!(
        (state.p1.core.hp, state.p2.core.hp, state.p2.core.max_hp),
        (97, 93, 96)
    );
    assert_eq!(state.p2.turn.lose_hp_count, 3); // max-HP clamping is not ModifyHp.
}
