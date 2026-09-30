//! 灵猫乱剑（Card 9）段数：服务端 battleParams 给出；评估 fixture 没有服务端队列时，
//! 按牌面「每保留 1 张手牌追加 1 次攻击（最多追加 otherParams[0] 次）」由开局手牌数推出。
use super::original_config::original_card_definition;
use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture};
use super::*;
use crate::fixture::FixtureExpected;
use crate::model::PlayerSide;

fn fixture_with_hand(hand: usize, tape: Vec<i64>) -> (BattleFixture, CardDefinition) {
    let card = original_card_definition(9).expect("card 9 灵猫乱剑");
    let mut fixture = minimal_fixture(
        filler_cards(card.clone()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    );
    fixture.players.p1.hand_cards = vec![1_000_001; hand];
    fixture.decision_tape = tape;
    (fixture, card)
}

fn damage_dealt(hand: usize, tape: Vec<i64>) -> i64 {
    let (fixture, card) = fixture_with_hand(hand, tape);
    let mut state = ReplayState::test_from_fixture(&fixture);
    let before = state.p2.core.hp;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    before - state.p2.core.hp
}

#[test]
fn empty_tape_derives_segments_from_hand_count() {
    let card = original_card_definition(9).expect("card 9");
    let attack = card.attack.unwrap_or(0);
    let base = card.attack_count.unwrap_or(0);
    // 语料 / live 33371977 R11：基础 2 段 + 手牌 3 张 = 队列 [5]
    assert_eq!(damage_dealt(3, Vec::new()), attack * (base + 3));
    // 追加段数封顶 otherParams[0]
    assert_eq!(damage_dealt(12, Vec::new()), attack * (base + 3));
    assert_eq!(damage_dealt(0, Vec::new()), attack * base);
}

#[test]
fn server_tape_still_wins_over_derivation() {
    let card = original_card_definition(9).expect("card 9");
    let attack = card.attack.unwrap_or(0);
    assert_eq!(damage_dealt(3, vec![2]), attack * 2);
}

/// 评估入口（execute_replay_fixture）以 strict 启动：旧条件只认 !fail_on_missing_decision，推导永不生效
/// （live g127 r9 空队列 +65、实际 −41）。评估 fixture 带 syntheticDecisionFallbackSeed 时同样推导。
fn damage_dealt_strict(hand: usize, tape: Vec<i64>, synthetic_seed: Option<u32>) -> i64 {
    let (mut fixture, card) = fixture_with_hand(hand, tape);
    let mut source = fixture.source.clone().unwrap_or_default();
    source.synthetic_decision_fallback_seed = synthetic_seed;
    fixture.source = Some(source);
    let mut state = ReplayState::from_fixture(&fixture, true).expect("strict replay construction");
    let before = state.p2.core.hp;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    before - state.p2.core.hp
}

#[test]
fn strict_evaluation_fixture_with_synthetic_seed_derives_segments() {
    let card = original_card_definition(9).expect("card 9");
    let attack = card.attack.unwrap_or(0);
    let base = card.attack_count.unwrap_or(0);
    assert_eq!(damage_dealt_strict(3, Vec::new(), Some(42)), attack * (base + 3));
    // 服务端队列仍优先
    assert_eq!(damage_dealt_strict(3, vec![2], Some(42)), attack * 2);
}

#[test]
fn strict_replay_without_seed_still_fails_closed() {
    // 真实回放（无合成种子）空队列不推导：照旧按缺失决策处理，不冒充服务端段数
    let card = original_card_definition(9).expect("card 9");
    let attack = card.attack.unwrap_or(0);
    let base = card.attack_count.unwrap_or(0);
    assert_ne!(damage_dealt_strict(3, Vec::new(), None), attack * (base + 3));
}
