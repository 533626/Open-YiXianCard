use super::*;
use crate::fixture::{load_fixture_file, BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn original_card(id: i64) -> CardDefinition {
    super::test_support::original_card(id)
}

fn basic_attack() -> CardDefinition {
    super::test_support::original_card(0)
}

fn deck_with(cards: Vec<CardDefinition>) -> Vec<CardDefinition> {
    super::test_support::fill_deck(cards, basic_attack())
}

fn player(cards: Vec<CardDefinition>) -> FixturePlayer {
    super::test_support::make_player(cards, 5, 50, Some(0), 8, Some(6))
}

fn fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    super::test_support::default_fixture(p1, p2)
}

fn custom_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    super::test_support::test_card(id, base_id, name)
}

#[test]
fn drunken_fist_stance_flaw_consumes_star_erosion() {
    let p1 = player(deck_with(vec![original_card(10_000_090)]));
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p2.talents = vec![103];
    let mut state = ReplayState::test_from_fixture(&fixture(p1, p2));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.status.flaw, 3);
    assert_eq!(state.p1.astrology.star_erosion, 0);
}

#[test]
fn drunken_fist_stance_accumulates_before_each_flaw_loss() {
    // 原版 Card_10000090.cs:145-148 累加醉拳架势，BattleCharacter.cs:8684-8691
    // 在每次失去破绽时按层数结算一次加攻。
    let run = |card_ids: &[i64]| {
        let cards: Vec<_> = card_ids.iter().map(|id| original_card(*id)).collect();
        let mut state = ReplayState::test_from_fixture(&fixture(
            player(deck_with(cards.clone())),
            player(deck_with(vec![basic_attack()])),
        ));

        for (slot, card) in cards.iter().enumerate() {
            state.test_apply_card_effect(PlayerSide::P1, card, slot);
        }
        let stance_before_flaw_loss = state.p1.status.drunken_fist_stance;
        let flaw_before_loss = state.p1.status.flaw;

        // 回合结束自然失去 1 层破绽，覆盖与真实 turnEnd -> turnStart 相同的结算入口。
        state.p1.tick_turn_end_statuses();

        (
            stance_before_flaw_loss,
            flaw_before_loss,
            state.p1.status.flaw,
            state.p1.core.attack_bonus,
        )
    };

    assert_eq!(run(&[10_000_090]), (1, 2, 1, 1));
    assert_eq!(run(&[10_000_090, 10_020_090]), (2, 4, 3, 2));
}

#[test]
fn ming_ye_mi_zong_uses_current_stack_for_first_hp_loss_reward() {
    let card = original_card(139);
    assert_eq!((card.id, card.other_params.to_vec()), (139, vec![2, 2]));
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![card.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    state.modify_actor_hp(PlayerSide::P1, -1, true, true);

    assert_eq!(
        (
            state.p1.mirage_ronghui.first_hp_loss_reward,
            state.p1.core.anima,
            state.p1.turn.agility,
        ),
        (1, 1, 3)
    );
}

#[test]
fn ming_ye_mi_zong_resets_after_all_late_turn_end_hp_changes() {
    let fixture = load_fixture_file("tests/data-driven/mirror_32028000_round-14.json")
        .expect("data-driven A2 fixture");
    let mut state = ReplayState::test_from_fixture(&fixture);

    for _ in 0..3 {
        state.test_play_actor_turn();
        state.test_advance_actor();
    }
    assert_eq!(
        (
            state.p1.mirage_ronghui.first_hp_loss_reward,
            state.p1.mirage_ronghui.first_hp_loss_reward_triggered,
            state.p1.core.anima,
        ),
        (1, 0, 1)
    );

    state.test_play_actor_turn();
    assert_eq!(state.p1.core.anima, 2);
}

#[test]
fn dream_frenzy_sword_zero_upgrades_only_the_first_rank_zero_frenzy_sword() {
    // 原版 BattleCharacter.cs:2411-2415 按牌组里 1030076/1040076 的张数授予
    // ShengJiXiaCiKuangJian(671)；CardActionBase.cs:1187-1193 在打出 rarity=0 的
    // 狂剑时消耗一层、扣 1030076.otherParams[2]=8 生命并就地升阶。
    let p1 = player(deck_with(vec![
        original_card(1_000_035),
        original_card(1_000_035),
        original_card(1_040_076),
    ]));
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![]))));
    let start_hp = state.p1.core.hp;
    assert_eq!(state.p1.sword.upgrade_next_frenzy_sword, 1);

    state.test_play_actor_turn();

    // 首张狂剑•二式升阶，代价是固定 8 点生命。
    assert_eq!(state.p1.deck.slots[0].card.id, 1_010_035);
    assert_eq!(state.p1.core.hp, start_hp - 8);
    assert_eq!(state.p1.sword.upgrade_next_frenzy_sword, 0);

    state.test_advance_actor();
    state.test_play_actor_turn();
    state.test_advance_actor();
    state.test_play_actor_turn();

    // 层数已耗尽，第二张同名狂剑保持基础阶，也不再扣生命。
    assert_eq!(state.p1.deck.slots[1].card.id, 1_000_035);
}

#[test]
fn xing_yi_duan_is_consumed_before_last_two_slot_skip() {
    // 原版 BattleExecuter.cs:1748 起的 while(flag) 循环里，1751-1755 的
    // XingYi_Duan 跳槽排在 1796-1800 的空间灵田（基础 id 9000015）跳槽之前。
    // 两者落在同一格时，先消费的必须是 XingYi_Duan。
    let mut p1 = player(deck_with(vec![
        original_card(9_000_015),
        original_card(1_020_010),
    ]));
    p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![]))));
    state.p1.astrology.star_chess_break = 1;

    state.test_play_actor_turn();

    let first_p1_card = state
        .test_events()
        .iter()
        .find(|event| event.kind == ReplayEventKind::CardCompleted && event.actor == PlayerSide::P1)
        .map(|event| (event.slot, event.card_id));
    // XingYi_Duan 先吃掉 slot 0 并把层数清零，空间灵田此时已无层数可再跳，
    // 于是 slot 1 正常出牌。若顺序反过来，slot 0 会被空间灵田跳掉、
    // XingYi_Duan 残留一层再把 slot 1 也跳掉，最终退化成基础攻击。
    assert_eq!(first_p1_card, Some((Some(1), Some(1_020_010))));
    assert_eq!(state.p1.astrology.star_chess_break, 0);
}

#[test]
fn dark_heart_mark_start_talent_grants_internal_injury_and_recovery() {
    let mut p1 = player(deck_with(vec![basic_attack()]));
    let mut p2 = player(deck_with(vec![basic_attack()]));
    p1.talents = vec![30_103];
    p2.talents = vec![10_150, 30_150];

    let state = ReplayState::test_from_fixture(&fixture(p1, p2));

    assert_eq!(state.p2.status.internal_injury, 8);
    assert_eq!(state.p2.status.recovery, 5);
    assert_eq!(state.p2.astrology.star_erosion, 0);
}

#[test]
fn spirit_turtle_footwork_anima_reward_respects_talent_204() {
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.identity.talents = vec![204].into();
    state.p1.turn.spirit_turtle_footwork = 1;
    let max_hp_before = state.p1.core.max_hp;
    let hp_before = state.p1.core.hp;

    state.modify_actor_hp(PlayerSide::P1, -1, false, false);

    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.core.max_hp, max_hp_before + 1);
    assert_eq!(state.p1.core.hp, hp_before - 1);
    assert_eq!(state.p1.turn.agility, 1);
}

#[test]
fn gain_anima_pure_fan_qu_204_converts_to_physique() {
    // 纯凡躯(204、无得炁 208)：加灵仍全部转体魄，灵气恒 0（既有行为不变）。
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.identity.talents = vec![204].into();
    let physique_before = state.p1.core.physique;

    state.gain_anima(PlayerSide::P1, 3);

    assert_eq!(state.p1.core.anima, 0, "凡躯把加灵转体魄，灵气不涨");
    assert_eq!(state.p1.core.physique, physique_before + 3);
}

#[test]
fn gain_anima_de_qi_208_stops_fan_qu_conversion() {
    // 得炁(208) 后凡躯改为灵炁奔涌：加灵进灵气池、不再转体魄；否则李㵘棍系
    // (再次行动加灵)会被清成 0 灵气而卡灵（见 li-man-gun-combo-stall-bug）。
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.identity.talents = vec![204, 208].into();
    let physique_before = state.p1.core.physique;

    state.gain_anima(PlayerSide::P1, 3);

    assert_eq!(state.p1.core.anima, 3, "得炁后加灵正常进灵气池");
    assert_eq!(state.p1.core.physique, physique_before, "得炁后不再转体魄");
}

#[test]
fn gain_anima_resonance_137_stops_fan_qu_conversion() {
    // 共鸣·得炁(137) 同样关闭凡躯转换（即便仍带 204、无 208）。镜像 TS
    // modifyAnima 的 `talentResonanceId !== 137`。
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.identity.talents = vec![204].into();
    state.p1.identity.talent_resonance_id = Some(137);
    let physique_before = state.p1.core.physique;

    state.gain_anima(PlayerSide::P1, 3);

    assert_eq!(state.p1.core.anima, 3, "共鸣137 关闭凡躯转换，灵气正常");
    assert_eq!(state.p1.core.physique, physique_before);
}

#[test]
fn card_219_stance_defense_respects_cannot_gain_defense() {
    let card = original_card(219);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![card.clone()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.core.physique = 100;
    state.p1.beng.quan_stance = 1;
    state.p1.dream_mirage.cannot_gain_defense = 1;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.beng.quan_stance, 0);
    assert_eq!(state.p1.beng.gun_stance, 1);
}

#[test]
fn painter_random_cards_empty_param_queue_is_no_op() {
    // 原版 GetNextParam()（BattleCharacter.cs:9316）空队列 catch 返回 -1：
    // 挥毫泼墨 `nextParam != -1` 不成立、落纸云烟 switch 无匹配，两张牌都无效果。
    let ink_run = evaluate_replay_fixture_fallible(&fixture(
        player(deck_with(vec![original_card(6_000_007)])),
        player(deck_with(vec![basic_attack()])),
    ))
    .expect("挥毫泼墨 empty param queue");
    assert_eq!(ink_run.p2.internal_injury, 0);
    assert_eq!(ink_run.p2.external_injury, 0);

    let mut explicit_ink = fixture(
        player(deck_with(vec![original_card(6_000_007)])),
        player(deck_with(vec![basic_attack()])),
    );
    explicit_ink.decision_tape = vec![105];
    let explicit_ink_run =
        evaluate_replay_fixture_fallible(&explicit_ink).expect("explicit debuff id");
    assert_eq!(explicit_ink_run.p2.external_injury, 2);

    let mut no_roll_ink = fixture(
        player(deck_with(vec![original_card(6_000_007)])),
        player(deck_with(vec![basic_attack()])),
    );
    no_roll_ink.decision_tape = vec![-1];
    let no_roll_ink_run = evaluate_replay_fixture_fallible(&no_roll_ink).expect("-1 debuff no-op");
    assert_eq!(no_roll_ink_run.p2.internal_injury, 0);
    assert_eq!(no_roll_ink_run.p2.external_injury, 0);

    let smoke_run = evaluate_replay_fixture_fallible(&fixture(
        player(deck_with(vec![original_card(6_000_014)])),
        player(deck_with(vec![basic_attack()])),
    ))
    .expect("落纸云烟 empty param queue");
    assert_eq!(smoke_run.p1.defense, 0);
    assert_eq!(smoke_run.p1.guard, 0);

    let mut guard_smoke = fixture(
        player(deck_with(vec![original_card(6_000_014)])),
        player(deck_with(vec![basic_attack()])),
    );
    guard_smoke.decision_tape = vec![2];
    let guard_smoke_run =
        evaluate_replay_fixture_fallible(&guard_smoke).expect("explicit smoke branch");
    assert_eq!(guard_smoke_run.p1.guard, 1);

    let mut no_roll_smoke = fixture(
        player(deck_with(vec![original_card(6_000_014)])),
        player(deck_with(vec![basic_attack()])),
    );
    no_roll_smoke.decision_tape = vec![-1];
    let no_roll_smoke_run =
        evaluate_replay_fixture_fallible(&no_roll_smoke).expect("-1 smoke no-op");
    assert_eq!(no_roll_smoke_run.p1.defense, 0);
    assert_eq!(no_roll_smoke_run.p1.guard, 0);
    assert_eq!(no_roll_smoke_run.p1.hp, 50);
}

#[test]
fn carry_weight_forward_counts_entangle_after_star_erosion() {
    let mut card = custom_card(10_010_031, 10_000_031, "荷重前行");
    card.defense = Some(11);
    card.physique = Some(3);
    card.other_params = vec![3, 1].into();
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.core.defense = 4;
    state.p1.core.physique = 54;
    state.p1.core.max_hp = 135;
    state.p1.core.physique_limit = 50;
    state.p1.status.flaw = 2;
    state.p1.astrology.star_erosion = 1;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.status.entangle, 4);
    assert_eq!(state.p1.astrology.star_erosion, 0);
    assert_eq!(state.p1.core.defense, 21);
}

#[test]
fn myriad_mystic_palm_converts_tape_selected_negative_statuses() {
    // 万玄破魔掌 Card_82.cs：每有 otherParams[0] 体魄读 1 个战斗参数
    // （GetNextParam），参数为要移除的负面状态 id，取尽（-1）提前结束，
    // 每消耗 1 个参数计 1 加攻。参数必须来自回放带，不带启发式兜底。
    // oracle 锚点：hf-32308000 pa_a459f081 cp25（turn14
    // 已耗尽带内 [100,103,100,103]，turn26 再出时队列空 → 0 转化）。
    let mut card = custom_card(82, 82, "万玄破魔掌");
    card.attack = Some(1);
    card.attack_count = Some(1);
    card.other_params = vec![20].into();
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.core.physique = 60;
    state.p1.status.flaw = 2;
    state.p1.status.entangle = 1;
    state.decision_tape.extend([102, 104]);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.status.flaw, 1);
    assert_eq!(state.p1.status.entangle, 0);
    assert_eq!(state.p1.core.attack_bonus, 2);
    assert_eq!(state.p2.core.hp, 47); // 1 攻 + 2 加攻 = 3 伤

    // 队列取尽后不再自选：清空在场负面状态也保持 0 转化。
    state.decision_tape.clear();
    state.p1.core.attack_bonus = 0;
    state.p1.core.physique = 60;
    state.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(state.p1.status.flaw, 1);
    assert_eq!(state.p1.core.attack_bonus, 0);
    assert_eq!(state.p2.core.hp, 46); // 仅 1 攻
}

#[test]
fn thousand_weight_fall_does_not_inherit_beng_quan_chan() {
    let card = original_card(10_000_009);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.beng.beng_quan_chan = 1;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.beng.beng_quan_chan, 1);
    assert_eq!(state.p2.status.external_injury, 0);
}

#[test]
fn extreme_windward_palm_uses_physique_after_card_gain_for_attack_bonus() {
    let palm = original_card(10_010_099);
    let mut p1 = player(deck_with(vec![palm.clone()]));
    p1.permanent_buff_temp_datas
        .insert(super::support::permanent_physique_key().to_string(), 14);
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));

    state.test_apply_card_effect(PlayerSide::P1, &palm, 0);

    assert_eq!(state.p1.core.physique, 17);
    assert_eq!(state.p2.core.hp, 38);
}

#[test]
fn mirage_flying_fang_sword_restores_consumed_sword_intent_plus_extra() {
    let mirage_fang = original_card(263);
    let mut p1 = player(deck_with(vec![mirage_fang]));
    p1.initial_anima = 1;
    let mut state =
        ReplayState::test_from_fixture(&fixture(p1, player(deck_with(vec![basic_attack()]))));
    state.p1.sword.sword_intent = 3;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 43);
    assert_eq!(state.p1.sword.sword_intent, 4);
    assert_eq!(state.active_effect_pending_sword_intent(), 0);
}

#[test]
fn real_beng_quan_inherits_beng_quan_chan() {
    let card = original_card(10_000_013);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.beng.beng_quan_chan = 1;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.beng.beng_quan_chan, 0);
    assert_eq!(state.p2.status.external_injury, 1);
}

#[test]
fn inherited_beng_quan_chan_is_prevented_by_exorcism() {
    let card = original_card(10_000_013);
    let mut state = ReplayState::test_from_fixture(&fixture(
        player(deck_with(vec![basic_attack()])),
        player(deck_with(vec![basic_attack()])),
    ));
    state.p1.beng.beng_quan_chan = 2;
    state.p2.fate.exorcism = 2;

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    assert_eq!(state.p1.beng.beng_quan_chan, 0);
    assert_eq!(state.p2.fate.exorcism, 0);
    assert_eq!(state.p2.status.external_injury, 0);
}

#[test]
fn eswmbqw_round19_wan_mo_shi_xin_qu_before_frenzy_sword_zero_lifesteal_is_exact() {
    // 偏差 C 端到端回归：pa_79bca52d/pa_43f28c44/round-19。
    // 首差在 turn3 p2 云剑•无妄（1010028，2 段）：每段攻击管道内原版顺序为
    // 「万魔噬心曲自伤（CalculateAttack）→ 伤害 → 狂剑灵石吸血（ApplyDamage）」；
    // 满血首段吸血受 maxHp 截断（original 121）。整轮须 exact
    // （winner/actorTurn/hpDelta 同时匹配，oracle 9/9 已确认服务端 expected 正确）。
    let fixture =
        load_fixture_file("tests/data-driven/eswmbqw_round-19.json")
            .expect("offline pa_43f28c44/round-19 fixture");
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert!(
        summary.matches_fixture(&fixture),
        "pa_43f28c44/round-19 exact mismatch — expected winner={:?} actorTurn={} hpDelta={}, \
         actual winner={:?} actorTurn={} hpDelta={}",
        fixture.expected.winner_side,
        fixture.expected.actor_turn_count,
        fixture.expected.hp_delta_p1_minus_p2,
        summary.winner_side,
        summary.actor_turn_count,
        summary.hp_delta_p1_minus_p2,
    );
}
