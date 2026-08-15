// Build 24610558 rotation cards: 401 云剑•狂炎, 415 疯魔架势,
// 425 极•飞鸿踏雪, 426 极•锟铻金环. 证据：build-24610558 反编译
// Card_401.cs / Card_415.cs / Card_425.cs / Card_426.cs 与 CardConfig。
// 另有存量缺口契约：19+天赋 20096 改名「云剑•澄心」后叠加狂龙吞云的
// 狂剑归类（Card_19.cs:511-519 / BattleCharacter.cs:12354）。
// 第二批：1000099 极•狂剑一式 / 1000100 极•灵犀剑阵 / 4000100 极•六爻绝阵 /
// 4000101 极•螳螂捕蝉 / 10000100 极•锻骨 / 10000101 极•夜鬼啸（HF 语料
// mirror-32219000-human-01 已出现，Card_*.cs 证据见各测试注释）。
use super::*;
use crate::fixture::{BattleFixture, FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::{CardDefinition, PlayerSide, DECK_SIZE};

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
fn talent_134_guard_is_granted_at_opponent_opening_boundary_after_fire_damage() {
    // Talent 112 的火灵开局伤害来自 40_109 的先天火标记；p2 的桃枝如意
    // 必须在 p1 opening 完成后才获得护体，故这笔 7 点伤害不能被护体消费。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![110, 40_109, 112];
    battle.players.p2.talents = vec![134];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p2.core.max_hp, 23);
    assert_eq!(state.p2.core.guard, 1);

    // 无 Talent134 对照：伤害相同，但不应凭构造期默认获得护体。
    battle.players.p2.talents.clear();
    let control = ReplayState::test_from_fixture(&battle);
    assert_eq!(control.p2.core.hp, 23);
    assert_eq!(control.p2.core.max_hp, 23);
    assert_eq!(control.p2.core.guard, 0);
}

#[test]
fn ice_incantation_repeated_hp_loss_consumes_guard_before_hp() {
    // Card_3000009.cs calls ModifyHpWithFx twice.  Its shared ModifyHp path
    // consumes one HuTi per loss and returns before changing HP, so two guard
    // layers block both printed 4-point losses and leave loss counters intact.
    let ice = original_card_definition_by_id(3_000_009).expect("missing 寒冰咒");
    let mut battle = fixture(deck(ice.clone()), deck(basic_attack()));
    battle.players.p2.initial_guard = 2;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &ice, 0);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p2.turn.lose_hp_count, 0);
    assert_eq!(state.p2.turn.lose_hp_times_count, 0);
}

#[test]
fn first_actor_talent_134_guard_precedes_opponent_golden_shuttle() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![134];
    battle
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10009".to_string(), 3);

    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.core.guard, 0);
}

#[test]
fn talent_184_defense_is_granted_at_opponent_opening_boundary_after_fire_damage() {
    // Talent 112 的火灵开局伤害应先命中 p2；Talent 183 + FateStrategy 166
    // 的体魄转防则要等 p2 自己进入 OnBattleStarted 才结算。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![110, 40_109, 112];
    battle.players.p2.talents = vec![183, 184];
    battle.players.p2.fate_strategies = vec![166];
    let state = ReplayState::test_from_fixture(&battle);
    assert_eq!(state.p2.core.hp, 23);
    assert_eq!(state.p2.core.physique, 2);
    assert_eq!(state.p2.core.defense, 2);

    // Control: without Talent 184, the same opening damage has no defense
    // to consume and no constructor-time Talent184 defense is present.
    battle.players.p2.talents = vec![183];
    let control = ReplayState::test_from_fixture(&battle);
    assert_eq!(control.p2.core.hp, 23);
    assert_eq!(control.p2.core.physique, 2);
    assert_eq!(control.p2.core.defense, 0);
}

#[test]
fn first_actor_guard_and_defense_precede_opponent_innate_fire() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![134, 183, 184];
    battle.players.p1.fate_strategies = vec![166];
    battle.players.p2.talents = vec![110, 40_109, 112];

    let state = ReplayState::test_from_fixture(&battle);
    // P1's opening grants guard/defense before P2's innate fire. Fire spends
    // guard first; it must not erase the two defense granted by Talent184.
    assert_eq!(state.p1.core.hp, 25);
    assert_eq!(state.p1.core.guard, 0);
    assert_eq!(state.p1.core.defense, 2);
}

#[test]
fn cloud_sword_step_lightly_keeps_base_and_cloud_defense_as_two_gains() {
    let card = original_card_definition_by_id(1_000_060).expect("missing 云剑•凌波");
    let battle = fixture(deck(card.clone()), deck(basic_attack()));

    let mut without_f382 = ReplayState::test_from_fixture(&battle);
    without_f382.p1.sword.cloud_chain = 1;
    without_f382.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(without_f382.p1.core.defense, 6);

    let mut with_f382 = ReplayState::test_from_fixture(&battle);
    with_f382.p1.sword.cloud_chain = 1;
    with_f382.p1.identity.fate_strategies.push(382);
    with_f382.test_apply_card_effect(PlayerSide::P1, &card, 0);
    assert_eq!(with_f382.p1.core.defense, 8);
}

#[test]
fn cloud_sword_raging_flame_config_matches_build_24610558() {
    let base = original_card_definition_by_id(401).expect("missing 云剑•狂炎");
    assert_eq!(base.attack, Some(4));
    assert_eq!(base.attack_count, Some(1));
    assert_eq!(base.other_params, vec![1]);
    assert_eq!(base.action_again, Some(true));
    // 稀有度档位：10401 攻击 10、20401 攻击 16，otherParams[0] 恒为 1。
    assert_eq!(
        original_card_definition_by_id(10_401).unwrap().attack,
        Some(10)
    );
    assert_eq!(
        original_card_definition_by_id(20_401).unwrap().attack,
        Some(16)
    );
    assert_eq!(
        original_card_definition_by_id(10_401).unwrap().other_params,
        vec![1]
    );
    assert_eq!(
        original_card_definition_by_id(20_401).unwrap().other_params,
        vec![1]
    );
}

#[test]
fn cloud_sword_raging_flame_attacks_and_grants_external_injury_and_extra_action() {
    // Card_401.cs: Attack(attack, attackCount)；随后 HasBuff(WoundedCount)
    // 成立 → dst WaiShang+otherParams[0]、src ExActionAgain+1。
    let raging = original_card_definition_by_id(401).expect("missing 云剑•狂炎");
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(raging.clone()), deck(basic_attack())));

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again); // CardConfig actionAgain=true
    assert_eq!(state.p2.core.hp, 26); // 4 攻
    assert_eq!(state.p2.status.external_injury, 1); // 击伤 → 外伤+1
                                                    // ExActionAgain+1 在同一事务的再次行动结算后被消费
                                                    // （BattleExecuter.cs:2040-2042 在成功再次行动后 RemoveBuff），
                                                    // 与配置 actionAgain 合并为一次重复行动。
    assert_eq!(state.p1.turn.extra_actions, 0);
    // 此牌同时视作云剑和狂剑：IsYunJian（名称含云剑）与
    // IsKuangJian（GetBaseCardId==401，BattleCharacter.cs:12400 附近）。
    assert!(super::support::is_cloud_sword(&state.p1, &raging));
    assert!(super::support::is_frenzy_sword_for_actor(
        &state.p1, &raging
    ));
    assert_eq!(state.p1.sword.frenzy_sword, 1);
    assert_eq!(state.p1.sword.cloud_chain, 1);
}

#[test]
fn cloud_sword_raging_flame_attack_fully_absorbed_by_defense_is_not_a_wound() {
    // 原版 ApplyDamage 先扣防再判 num>0（BattleCharacter.cs:10842-10854），
    // 攻击被防吃满时不计数 WoundedCount，[击伤] 分支不触发。
    let raging = original_card_definition_by_id(401).expect("missing 云剑•狂炎");
    let mut battle = fixture(deck(raging), deck(basic_attack()));
    battle.players.p2.initial_defense = 10;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 6);
    assert_eq!(state.p2.status.external_injury, 0);
    assert_eq!(state.p1.turn.extra_actions, 0);
}

#[test]
fn frenzy_stance_config_matches_build_24610558() {
    let base = original_card_definition_by_id(415).expect("missing 疯魔架势");
    assert_eq!(base.other_params, vec![3]);
    // 稀有度档位 冥 4/5。
    assert_eq!(
        original_card_definition_by_id(10_415).unwrap().other_params,
        vec![4]
    );
    assert_eq!(
        original_card_definition_by_id(20_415).unwrap().other_params,
        vec![5]
    );
}

#[test]
fn frenzy_stance_grants_min_and_physique_on_own_negative_status_gain() {
    // Card_415.cs: ModifyBuffValue(Min, otherParams[0])；
    // 被动 BattleCharacter.cs:8711-8713：负面状态 delta != 0 且牌组含 415
    // → ModifyTiPo(abs(delta))。自己的冥 +3 也计入 → 3 体魄。
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(stance), deck(basic_attack())));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.status.meditation, 3); // 冥+3
    assert_eq!(state.p1.core.hp, 21); // 冥副作用 abs(delta)*3（BattleCharacter.cs:8715-8730）
    assert_eq!(state.p1.core.physique, 3); // 被动：获得 3 层负面状态 → 3 体魄
    assert_eq!(state.p1.core.max_hp, 33); // 体魄同步生命上限
}

#[test]
fn frenzy_stance_passive_fires_on_both_gain_and_loss_of_negative_statuses() {
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(stance), deck(basic_attack())));
    // 原版体魄上限 5；体魄超过上限的部分按 1:1 回血。
    state.test_configure_p1(|player| player.core.physique_limit = 5);

    // 获得 2 层内伤 → 2 体魄。
    state.modify_actor_negative_status(PlayerSide::P1, 100, 2);
    assert_eq!(state.p1.status.internal_injury, 2);
    assert_eq!(state.p1.core.physique, 2);
    assert_eq!(state.p1.core.hp, 30);

    // 失去 1 层 → 再 +1 体魄。
    state.modify_actor_negative_status(PlayerSide::P1, 100, -1);
    assert_eq!(state.p1.status.internal_injury, 1);
    assert_eq!(state.p1.core.physique, 3);
    assert_eq!(state.p1.core.hp, 30);

    // 获得/失去冥（367）同样触发被动，并附带冥的扣血/回血副作用。
    // 冥 +3：体魄 3→6（上限 5，超出 1 → 回血 1）；冥副作用 -9。
    state.modify_actor_negative_status(PlayerSide::P1, 367, 3);
    assert_eq!(state.p1.status.meditation, 3);
    assert_eq!(state.p1.core.hp, 22); // 30 - 9 + 1
    assert_eq!(state.p1.core.physique, 6); // +3

    // 冥 -1：冥副作用再扣 3（BattleCharacter.cs:8715-8730 对非 4000003
    // 角色获得与失去都按 abs(delta)*3 扣血）；被动 +1 体魄 → 超限回血 1。
    state.modify_actor_negative_status(PlayerSide::P1, 367, -1);
    assert_eq!(state.p1.status.meditation, 2);
    assert_eq!(state.p1.core.hp, 20); // 22 - 3 + 1
    assert_eq!(state.p1.core.physique, 7); // 失去也触发被动 +1
}

#[test]
fn frenzy_stance_passive_requires_card_415_in_deck() {
    // 被动以 HasCardInDeck(415) 为条件；无 415 的角色获得/失去负面状态
    // 不获得体魄。
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));

    state.modify_actor_negative_status(PlayerSide::P1, 100, 2);
    assert_eq!(state.p1.core.physique, 0);
    state.modify_actor_negative_status(PlayerSide::P1, 100, -1);
    assert_eq!(state.p1.core.physique, 0);
}

#[test]
fn frenzy_stance_passive_fires_on_entangle_consumed_by_action_again() {
    // BattleExecuter.cs:2051 消耗困缚走 ModifyBuffValue(KunFu, -1)，同样触发
    // 疯魔架势被动（KunFu 为 Negative 分类）。
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(stance), deck(basic_attack())));
    state.test_configure_p1(|player| player.status.entangle = 2);

    // 直接走 action-again 消费路径：缠绕 1 层被消耗。
    state.modify_actor_negative_status(PlayerSide::P1, 104, -1);
    assert_eq!(state.p1.status.entangle, 1);
    assert_eq!(state.p1.core.physique, 1);
}

#[test]
fn frenzy_stance_battle_start_meditation_triggers_passive_and_hp_side_effect() {
    // 天赋 179 入冥 / 命运 161 在战斗开局走 ModifyBuffValue(Min, +X)
    // （BattleCharacter.cs:1671-1673），因此同样触发：
    // ① 冥副作用 abs(delta)*3 扣血（4000003 角色回血）；
    // ② 415 被动 ModifyTiPo(abs(delta))（BattleCharacter.cs:8711-8713）。
    // 非 4000003 角色：开局 冥+1 → 体魄+1 且生命-3（引擎 from_fixture 装配）。
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut battle = fixture(deck(stance.clone()), deck(basic_attack()));
    battle.players.p1.talents = vec![179];
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.status.meditation, 1); // 开局冥+1
    assert_eq!(state.p1.core.physique, 1); // 415 被动：+1 体魄
    assert_eq!(state.p1.core.max_hp, 31); // 体魄同步生命上限
    assert_eq!(state.p1.core.hp, 27); // 冥副作用 -3（非 4000003）
}

#[test]
fn frenzy_stance_battle_start_meditation_heals_character_4000003() {
    // 4000003 角色开局冥+1 的副作用是回血 +3（BattleCharacter.cs:8715-8730），
    // 且 415 被动体魄+1 同步生命上限。
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut battle = fixture(deck(stance.clone()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_003);
    battle.players.p1.talents = vec![179];
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.status.meditation, 1);
    assert_eq!(state.p1.core.physique, 1);
    assert_eq!(state.p1.core.max_hp, 31);
    // 原版 ModifyBuffValue 顺序（BattleCharacter.cs:8711-8729）：415 被动
    // ModifyTiPo(1) 先涨体魄/生命上限到 31，Min 分支 ModifyHp(3) 再按
    // 31 截断（oracle 锚点：hf-latest-32308000-16f9c778
    // e170262525adf8c7/round-09 cp0 p2.hp 81 = 79+3 被 81 截断）。
    assert_eq!(state.p1.core.hp, 31);
}

#[test]
fn battle_start_orders_later_meditation_after_earlier_opening_damage() {
    // BattleCharacter.OnBattleStarted is actor-owned and ordered by the first
    // player.  A先手卜命 opening hit must land before the后手 4000003 Talent
    // 179 冥 gain, so the latter's +3 healing is not capped at full HP.
    let divination =
        original_card_definition_by_id(11_000_001).expect("missing current-build fate divination");
    let mut battle = fixture(deck(basic_attack()), deck(divination));
    battle.first_player_side = PlayerSide::P2;
    battle.players.p1.character_id = Some(4_000_003);
    battle.players.p1.talents = vec![179];

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 3);
}

#[test]
fn wan_shi_ru_yi_opening_reads_downgraded_deck_slot_params() {
    // BattleCharacter.TriggerOpening（:11047-11124）按牌组格位**当前**卡牌
    // 结算开局（GetBattleDeckIdList()[grid] + cardConfigDict[num]）：先手
    // 厄劫缠身降级后，后手万事如意按降级后卡牌 otherParams[2] 发辟邪
    // （11010013 [6,4,2] → 11000013 [6,3,1]），而非 fixture 原卡参数。
    // 锚点：8f0ba353b4c1a831/round-14 首差（p1 辟邪 1 而非 2，杯弓蛇影
    // 内伤 2 只被挡 1 层）。
    let mut wan_shi_ru_yi = card(11_010_013, 11_000_013, "万事如意");
    wan_shi_ru_yi.other_params = vec![6, 4, 2];
    let mut calamity = card(11_000_018, 11_000_018, "厄劫缠身");
    calamity.other_params = vec![4, 6];
    let mut battle = fixture(deck(wan_shi_ru_yi.clone()), deck(calamity));
    battle.first_player_side = PlayerSide::P2;

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.deck.slots[0].card.id, 11_000_013);
    assert_eq!(state.p1.fate.exorcism, 1);

    // 对照：无厄劫缠身降级时按原卡参数发 2 层辟邪。
    let mut battle = fixture(deck(wan_shi_ru_yi), deck(basic_attack()));
    battle.first_player_side = PlayerSide::P2;
    let control = ReplayState::test_from_fixture(&battle);
    assert_eq!(control.p1.deck.slots[0].card.id, 11_010_013);
    assert_eq!(control.p1.fate.exorcism, 2);
}

#[test]
fn hard_branch_bamboo_sustain_divisor_survives_plain_variant() {
    // 原版 Card_9000027.cs 只在 otherParams[0] > 0 时加 YingZhiZhu；
    // 回合结束伤害除数固定读 9020027 配置 otherParams[1]=4
    // （BattleCharacter.cs:6080）。非持续变体 9010027 不得把已安装的
    // 除数清零。锚点：hf-latest-32391000-03b604c4 497310ce2b824e77/
    // round-15 t3 结束 p1 100→96（p2 防 19，19/4=4）。
    let mut sustain = card(9_020_027, 9_000_027, "硬枝竹");
    sustain.defense = Some(8);
    sustain.other_params = vec![1, 4];
    let mut plain = card(9_010_027, 9_000_027, "硬枝竹");
    plain.defense = Some(6);
    plain.other_params = vec![0];
    let mut battle = fixture(deck(basic_attack()), vec![sustain, plain]);
    battle.players.p2.active_slot_count = 2;
    battle.max_actor_turns = Some(4);
    let mut state = ReplayState::test_from_fixture(&battle);
    // p1 回合 1：普通攻击。
    state.test_play_actor_turn();
    // p2 回合 1：9020027（防 8，count 1，per 4）→ 回合结束 p1 受 8/4=2。
    state.test_advance_actor();
    state.test_play_actor_turn();
    assert_eq!(state.p1.core.hp, 28);
    // p1 回合 2：普通攻击（p2 防 8→5）。
    state.test_advance_actor();
    state.test_play_actor_turn();
    // p2 回合 2 开始：防御减半（5→2，BattleCharacter.cs OnTurnStarted
    // CeilToInt(def*0.5)）；9010027 防 +6 → 8，count 不变；除数保持 4，
    // 回合结束 p1 受 8/4=2。
    state.test_advance_actor();
    state.test_play_actor_turn();
    assert_eq!(state.p2.core.defense, 8);
    assert_eq!(state.p1.core.hp, 26); // 30 - 2 - 2；除数被清零则停在 28
}

#[test]
fn calamity_opening_damage_reads_current_downgraded_slot_card() {
    // 原版 TriggerOpening case 11000018（BattleCharacter.cs:11132-11147）
    // 读 cardItem2 = 牌组格位**当前**卡牌：先手方厄劫缠身已把后手方
    // 11020018 降级为 11010018 时，后手方开局伤害取 11010018 的
    // otherParams[1]=9，而非 fixture 原卡 11020018 的 12。
    // 锚点：hf-latest-32391000-03b604c4 d11b6adfe79418e2/round-17
    // cp0 p2.hp 96（107-11=96，而非 107-12=95）。
    let mut calamity_r2 = card(11_020_018, 11_000_018, "厄劫缠身");
    calamity_r2.other_params = vec![6, 12];
    let mut calamity_base = card(11_000_018, 11_000_018, "厄劫缠身");
    calamity_base.other_params = vec![4, 6];
    let mut battle = fixture(deck(calamity_r2), deck(calamity_base));
    battle.first_player_side = PlayerSide::P2;
    let state = ReplayState::test_from_fixture(&battle);
    // p2 先结算开局：把 p1 格 0 的 11020018 降级为 11010018。
    assert_eq!(state.p1.deck.slots[0].card.id, 11_010_018);
    // p1 再结算：目标 p2 格 0 的 11000018（rarity 0，无法降级）→
    // 造成当前格位卡（11010018）otherParams[1]=9 伤害给 p2。
    assert_eq!(state.p2.core.hp, 30 - 9);
}

#[test]
fn frenzy_stance_turn_end_decay_triggers_passive() {
    // 原版 OnTurnEnded 对 虚弱/破绽/困缚 逐层走 ModifyBuffValue(-1)
    // （BattleCharacter.cs:5686-5695），Negative 分类 delta != 0 触发
    // 415 被动 ModifyTiPo（8711-8713）。引擎 turn-end 衰减同样走该 hook。
    let stance = original_card_definition_by_id(415).expect("missing 疯魔架势");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(stance), deck(basic_attack())));
    state.test_configure_p1(|player| {
        player.status.weakness = 2;
        player.status.entangle = 1;
    });

    // p1 行动一整回合：415 冥+3 → 3 体魄；turn-end 衰减（虚弱 1 层 +
    // 困缚 1 层，每回合每状态各衰减 1 层）→ +2 体魄。
    state.test_play_actor_turn();

    assert_eq!(state.p1.status.weakness, 1);
    assert_eq!(state.p1.status.entangle, 0);
    assert_eq!(state.p1.core.physique, 5); // 3（冥）+ 2（衰减）
}

#[test]
fn water_spring_rain_config_matches_build_24610558() {
    let base = original_card_definition_by_id(428).expect("missing 极•水灵春雨");
    assert_eq!(base.other_params, vec![2, 1]);
    assert_eq!(base.action_again, Some(true));
    assert_eq!(
        base.card_type.as_ref().map(|card_type| card_type.value),
        Some(3),
        "Sustain"
    );
    // 10428/20428 水势 4/6、海潮 1（24589371 起数值档位）。
    assert_eq!(
        original_card_definition_by_id(10_428).unwrap().other_params,
        vec![4, 1]
    );
    assert_eq!(
        original_card_definition_by_id(20_428).unwrap().other_params,
        vec![6, 1]
    );
}

#[test]
fn water_spring_rain_grants_water_momentum_and_tide_not_hp() {
    // Card_428.cs: ModifyBuffValue(ShuiShi, otherParams[0]) →
    // ModifyBuffValue(HaiChao, otherParams[1])。24589371 起行为类型变更：
    // 旧「生命及上限+otherParams[0]」（与卡 17 同构）→ 新「水势+otherParams[0]、
    // 海潮+otherParams[1]」。与卡 17 的旧行为分离。
    let rain = original_card_definition_by_id(428).expect("missing 极•水灵春雨");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(rain), deck(basic_attack())));

    state.test_apply_card_effect(
        PlayerSide::P1,
        &original_card_definition_by_id(428).unwrap(),
        0,
    );

    assert_eq!(state.p1.elements.water_momentum, 2); // 水势+otherParams[0]
    assert_eq!(state.p1.fate.tide, 1); // 海潮+otherParams[1]
    assert_eq!(state.p1.core.hp, 30); // 不再加生命
    assert_eq!(state.p1.core.max_hp, 30); // 不再加生命上限
}

#[test]
fn water_spring_rain_card_17_keeps_old_hp_behavior() {
    // 卡 17 水灵•春雨保持旧行为（Card_17.cs: ModifyMaxHp/ModifyHp +
    // HaiChao），不随 428 的行为类型变更。
    let rain = original_card_definition_by_id(17).expect("missing 水灵•春雨");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(rain), deck(basic_attack())));

    state.test_apply_card_effect(
        PlayerSide::P1,
        &original_card_definition_by_id(17).unwrap(),
        0,
    );

    assert_eq!(state.p1.elements.water_momentum, 0); // 17 不加直接水势
    assert_eq!(state.p1.fate.tide, 1); // 海潮+otherParams[1]
    assert_eq!(state.p1.core.hp, 34); // 生命+otherParams[0]=4
    assert_eq!(state.p1.core.max_hp, 34); // 生命上限+4
}

#[test]
fn water_spring_rain_tide_converts_to_water_momentum_at_turn_start() {
    // 428 的海潮层数在自身回合开始时转水势（BattleCharacter.cs:4249：
    // ModifyBuffValue(ShuiShi, GetBuffValue(HaiChao))）。
    let rain = original_card_definition_by_id(428).expect("missing 极•水灵春雨");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(rain), deck(basic_attack())));

    state.test_apply_card_effect(
        PlayerSide::P1,
        &original_card_definition_by_id(428).unwrap(),
        0,
    );
    assert_eq!(state.p1.fate.tide, 1);
    assert_eq!(state.p1.elements.water_momentum, 2);

    // 完整回合：回合开始海潮 1 层转水势（槽位已被直接结算标记使用，
    // 本回合打出后续普通攻击，不再重复 428 效果）。
    state.test_play_actor_turn();
    assert_eq!(state.p1.elements.water_momentum, 3); // 2 + 1（海潮转换）
    assert_eq!(state.p1.fate.tide, 1); // 海潮不消耗
}

#[test]
fn extreme_flying_snow_step_config_matches_build_24610558() {
    let base = original_card_definition_by_id(425).expect("missing 极•飞鸿踏雪");
    assert_eq!(base.anima, Some(3));
    assert_eq!(base.other_params, vec![10, 6]);
    assert_eq!(
        original_card_definition_by_id(10_425).unwrap().anima,
        Some(4)
    );
    assert_eq!(
        original_card_definition_by_id(10_425).unwrap().other_params,
        vec![10, 9]
    );
    assert_eq!(
        original_card_definition_by_id(20_425).unwrap().anima,
        Some(5)
    );
    assert_eq!(
        original_card_definition_by_id(20_425).unwrap().other_params,
        vec![10, 12]
    );
}

#[test]
fn extreme_flying_snow_step_grants_anima_and_agility_without_rear_move() {
    // Card_425.cs: ModifyAnima(anima)、ModifyBuffValue(ShenFa, otherParams[0])、
    // 后招成功才 ModifyHp(otherParams[1])。与卡 12 不同：无 rarity 分支。
    // 24705509 起 anima=3（24666769 为 2）。
    let step = original_card_definition_by_id(425).expect("missing 极•飞鸿踏雪");
    let mut battle = fixture(deck(step.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 3;
    let mut state = ReplayState::test_from_fixture(&battle);

    // 走牌体直调路径（不做再次行动结算），直接观察身法 +10。
    state.test_apply_card_effect(PlayerSide::P1, &step, 0);

    assert_eq!(state.p1.core.anima, 6); // 灵气+3
    assert_eq!(state.p1.turn.agility, 10); // 身法+10
    assert_eq!(state.p1.core.hp, 30); // 无后招 → 不回血
}

#[test]
fn extreme_flying_snow_step_transaction_grants_agility_action_again() {
    // 完整事务：身法 10 达到再次行动阈值（BattleExecuter 身法>=10 消耗 10
    // 再次行动），身法被消费为 0。
    let step = original_card_definition_by_id(425).expect("missing 极•飞鸿踏雪");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(step), deck(basic_attack())));

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again); // 身法再次行动
    assert_eq!(state.p1.turn.agility, 0); // 身法 10 被再次行动消耗
    assert_eq!(state.p1.core.anima, 3);
}

#[test]
fn extreme_flying_snow_step_heals_on_rear_move() {
    let step = original_card_definition_by_id(425).expect("missing 极•飞鸿踏雪");
    let mut battle = fixture(deck(step.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_configure_p1(|player| player.fate.next_rear_move_bypass = 1);
    state.modify_actor_hp(PlayerSide::P1, -10, false, false); // 留出回血空间

    state.test_apply_card_effect(PlayerSide::P1, &step, 0);

    assert_eq!(state.p1.core.anima, 4);
    assert_eq!(state.p1.turn.agility, 10);
    assert_eq!(state.p1.core.hp, 26); // 20 + 后招回血 6
}

#[test]
fn extreme_kunwu_metal_ring_config_matches_build_24610558() {
    let base = original_card_definition_by_id(426).expect("missing 极•锟铻金环");
    assert_eq!(base.other_params, vec![1]);
    assert_eq!(base.action_again, Some(true));
    assert_eq!(
        base.card_type.as_ref().map(|card_type| card_type.value),
        Some(3),
        "Sustain"
    );
    assert_eq!(
        original_card_definition_by_id(10_426).unwrap().other_params,
        vec![2]
    );
    assert_eq!(
        original_card_definition_by_id(20_426).unwrap().other_params,
        vec![3]
    );
}

#[test]
fn extreme_kunwu_metal_ring_activates_elements_and_enhances_defense_and_sharpness_gains() {
    // Card_426.cs: JiHuoTuLing+1、JiHuoJinLing+1、KunWuJinHuan+=otherParams[0]。
    // 持续效果由共享 hook 承担：BattleCharacter.cs:8572-8574（加锋锐时多加）、
    // 10088-10090（加防时多加）。
    let ring = original_card_definition_by_id(426).expect("missing 极•锟铻金环");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(ring), deck(basic_attack())));

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again); // 再次行动
    assert_eq!(state.p1.elements.activated_earth, 1);
    assert_eq!(state.p1.elements.activated_metal, 1);
    assert_eq!(state.p1.sword.metal_ring, 1);
    assert!(state.p1.deck.slots[0].skipped); // Sustain 卡用后移除

    // 持续：每次加防或加锋锐时多加 1。
    state.gain_defense(PlayerSide::P1, 3);
    assert_eq!(state.p1.core.defense, 4);
    state.gain_sharpness(PlayerSide::P1, 2);
    assert_eq!(state.p1.sword.sharpness, 3);
}

#[test]
fn extreme_kunwu_metal_ring_rarity_tiers_scale_the_persistent_amount() {
    // 10426/20426 的持续层数 2/3，走 base id 归一化后由同一 handler 读取
    // otherParams[0] 自动覆盖。
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck(original_card_definition_by_id(20_426).expect("missing 20426")),
        deck(basic_attack()),
    ));
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.metal_ring, 3);

    state.gain_sharpness(PlayerSide::P1, 1);
    assert_eq!(state.p1.sword.sharpness, 4);
}

#[test]
fn card_19_with_talent_20096_is_frenzy_sword_under_frenzy_dragon_swallows_cloud() {
    // 澄心剑胚（19）+ 天赋 20096（云剑之心）：Card_19.UpdateCardInfo
    // （Card_19.cs:511-519）在战斗开局把 19 改名「云剑•澄心」；叠加狂龙吞云
    // （BuffType.KuangLongTunYun）后 IsKuangJian 成立
    // （BattleCharacter.cs:12354）→ OnAfterExecuted 狂剑+1
    // （CardActionBase.cs:4616-4618）。引擎不重命名卡，fixture 卡名仍是
    // 「澄心剑胚」，故在 is_frenzy_sword_with_options 的狂龙吞云分支内补
    // 19+20096 改名等价判定（与 is_cloud_sword 的 19+20096 分支同口径）。
    let mut battle = fixture(
        {
            let mut cards = vec![
                original_card_definition_by_id(50).expect("missing 狂龙吞云"),
                original_card_definition_by_id(19).expect("missing 澄心剑胚"),
                original_card_definition_by_id(1_010_022).expect("missing 狂剑•一式"),
                original_card_definition_by_id(1_000_035).expect("missing 狂剑•二式"),
            ];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.character_id = Some(1_000_005); // 剑修
    battle.players.p1.talents = vec![20_096];
    battle.players.p1.active_slot_count = 8; // 多卡按牌组顺序逐张打出
    let mut state = ReplayState::test_from_fixture(&battle);

    let cheng_xin = original_card_definition_by_id(19).expect("missing 澄心剑胚");
    // 无狂龙吞云时 19+20096 只是云剑（IsYunJian），不是狂剑（IsKuangJian）。
    assert!(super::support::is_cloud_sword(&state.p1, &cheng_xin));
    assert!(!super::support::is_frenzy_sword_for_actor(
        &state.p1, &cheng_xin
    ));

    state.test_execute_one_card(PlayerSide::P1); // 50 狂龙吞云
    assert_eq!(state.p1.sword.frenzy_dragon_swallows_cloud, 1);
    assert_eq!(state.p1.sword.frenzy_sword, 1);

    // 19 澄心剑胚（运行时已改名「云剑•澄心」）→ 狂剑+1：1→2。
    state.test_execute_one_card(PlayerSide::P1);
    assert!(super::support::is_frenzy_sword_for_actor(
        &state.p1, &cheng_xin
    ));
    assert_eq!(state.p1.sword.frenzy_sword, 2);

    // 1010022 狂剑•一式：Card_1000022.cs:72 Attack(attack + 狂剑层数×otherParams[0])。
    // 对 0 防目标 = 8 + 2×3 + 加攻0 = 14（缺口修复前为 11）。
    let hp_before = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(hp_before - state.p2.core.hp, 14);
    assert_eq!(state.p1.sword.frenzy_sword, 3); // 一式自身也计入狂剑

    // 1000035 狂剑•二式：Card_1000035.cs:85,125 命中数 = 1 + 狂剑层数。
    // 此时狂剑 3 层 → 4 段 × attack（0 防目标全段命中）。
    let second_form = original_card_definition_by_id(1_000_035).expect("missing 狂剑•二式");
    let hp_before = state.p2.core.hp;
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(
        hp_before - state.p2.core.hp,
        (1 + 3) * second_form.attack.expect("狂剑•二式 attack"),
    );
    assert_eq!(state.p1.sword.frenzy_sword, 4);
}

// ---- 第二批 build 24610558 极卡（HF 语料 mirror-32219000-human-01）----

#[test]
fn extreme_frenzy_sword_first_form_config_matches_build_24610558() {
    let base = original_card_definition_by_id(1000099).expect("missing 极•狂剑一式");
    assert_eq!(base.attack, Some(1));
    assert_eq!(base.other_params, vec![1]);
    assert_eq!(
        base.action_again,
        Some(true),
        "再次行动由配置 actionAgain 判定"
    );
    assert_eq!(
        original_card_definition_by_id(1010099)
            .unwrap()
            .other_params,
        vec![2]
    );
    assert_eq!(
        original_card_definition_by_id(1020099)
            .unwrap()
            .other_params,
        vec![3]
    );
}

#[test]
fn extreme_frenzy_sword_first_form_adds_kuang_jian_count_per_use() {
    // Card_1000099.cs: num2 = GetBuffValue(KuangJian) * otherParams[0];
    // Attack(attack + num2, attackCount)。KuangJian buff 即「用过狂剑次数」，
    // 本牌 body 先读（不含本次）、OnAfterExecuted 狂剑+1 后计数才含本次。
    let form = original_card_definition_by_id(1000099).expect("missing 极•狂剑一式");
    let mut battle = fixture(deck(form.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1; // 满足费用
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.frenzy_sword = 2;

    state.test_apply_card_effect(PlayerSide::P1, &form, 0);

    // 1 攻 + 狂剑 2 层 × otherParams[0]=1 = 3；body 阶段读取的是本次前的计数。
    assert_eq!(state.p2.core.hp, 30 - 3);
    // test_apply_card_effect 走完整出牌事务：OnAfterExecuted 狂剑+1
    // （极•狂剑一式自身算狂剑）。
    assert_eq!(state.p1.sword.frenzy_sword, 3);

    // 完整事务：OnAfterExecuted 狂剑+1（极•狂剑一式自身算狂剑），
    // 配置 actionAgain → 再次行动。
    let mut state = ReplayState::test_from_fixture(&fixture(deck(form), deck(basic_attack())));
    state.p1.sword.frenzy_sword = 2;
    let action_again = state.test_execute_one_card(PlayerSide::P1);
    assert!(action_again);
    assert_eq!(state.p1.sword.frenzy_sword, 3);
}

#[test]
fn extreme_ling_xi_sword_formation_config_matches_build_24610558() {
    let base = original_card_definition_by_id(1000100).expect("missing 极•灵犀剑阵");
    assert_eq!(base.defense, Some(4));
    assert_eq!(base.other_params, vec![8]);
    assert_eq!(base.action_again, Some(true));
    let rare = original_card_definition_by_id(1010100).unwrap();
    assert_eq!(rare.defense, Some(8));
    assert_eq!(rare.other_params, vec![12]);
    let epic = original_card_definition_by_id(1020100).unwrap();
    assert_eq!(epic.defense, Some(12));
    assert_eq!(epic.other_params, vec![16]);
}

#[test]
fn extreme_ling_xi_sword_formation_converts_capped_sword_intent_to_anima() {
    // Card_1000100.cs: def>0 → ModifyDef(def)；num3 = min(剑意, otherParams[0])，
    // 先 ModifyBuffValue(JianYi, -num3) 再 ModifyAnima(num3)。
    // 24705509 起 def=4（24666769 为 2）。
    let formation = original_card_definition_by_id(1000100).expect("missing 极•灵犀剑阵");
    let mut battle = fixture(deck(formation.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1; // 满足费用
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.sword_intent = 5;

    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);

    assert_eq!(state.p1.core.defense, 4); // 防+4
    assert_eq!(state.p1.core.anima, 6); // 灵气 1 + 剑意 5
    assert_eq!(state.p1.sword.sword_intent, 0);

    // 超过 otherParams[0]=8 的部分保留。
    let mut battle = fixture(deck(formation.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.sword_intent = 10;
    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);
    assert_eq!(state.p1.core.anima, 9); // 1 + 8
    assert_eq!(state.p1.sword.sword_intent, 2);
}

#[test]
fn spirit_claw_negative_anima_card_grants_bonus() {
    let card = original_card_definition_by_id(10_010_030).expect("missing 双鬼拍门");
    let mut battle = fixture(deck(card.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![152];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 双鬼拍门：7 攻 × 2 段 + 灵爪 +3/段 = 10/段；先给双方外伤 1，
    // 目标带外伤 → 每段 +1 → 共 22。
    assert_eq!(state.p2.core.hp, 30 - 22);
    assert_eq!(state.p2.status.internal_injury, 2);
    assert_eq!(state.p2.status.external_injury, 1);
}

#[test]
fn spirit_claw_anima_card_desc_anima_still_grants_bonus() {
    // desc 含「灵气」的卡（势如破竹 10010033）保持 +3（既有行为）。
    let card = original_card_definition_by_id(10_010_033).expect("missing 势如破竹");
    let mut battle = fixture(deck(card.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![152];
    battle.players.p1.initial_momentum_limit = Some(6);
    battle.players.p1.initial_anima = 5;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &card, 0);

    // 势如破竹 10010033：11 攻 + 3（耗 3 灵气 ×1 攻）+ 灵爪 3 = 23，
    // 耗灵获得的 3 气势 → +30% → 23×1.3 = 29.9 → 29。
    assert_eq!(state.p2.core.hp, 1);
}

#[test]
fn spirit_claw_non_anima_card_grants_nothing() {
    // 迎风掌 10000028：desc 无「灵气」、anima 无（非负）→ 不触发。
    let palm = original_card_definition_by_id(10_000_028).expect("missing 迎风掌");
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![152];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &palm, 0);

    // 迎风掌 10000028：4 攻 × 2 段 = 8。
    assert_eq!(state.p2.core.hp, 30 - 8);
}

// ---- FateStrategy 436 七星借命（HF 415 簇 oracle 诊断修复，round-10 尾盘）----
//
// oracle 锚点：mirror-32219000-human-01 2995be139404d0ed/round-10
// checkpoint[14]→[15]（turn15 七星定魂）：p1 生命 -3 → 10、上限 80 → 95
// —— 星力 5 × 标记 3 = +15 上限/生命，随后内伤 -2；星力清零。
// 原版 FateStrategyFunctions.cs:587-589（开局发 QiXingJieMing=3）+
// BattleExecuter.CharacterResurrectionCheckAsync IL_0855 起：
// 首次生命 ≤ 0 时按（卦象+星力）× 标记值转换生命及上限。

#[test]
fn seven_stars_borrowed_life_converts_star_power_on_first_death() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![436];
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.fate.qi_xing_jie_ming, 3); // 开局发放
    state.modify_star_power(PlayerSide::P1, 5);
    state.modify_actor_hp(PlayerSide::P1, -33, false, false); // 30 → -3

    assert!(state.death_winner().is_none()); // 转换后生命 > 0，继续战斗
    assert_eq!(state.p1.core.hp, 12); // -3 + 5×3
    assert_eq!(state.p1.core.max_hp, 45); // 30 + 5×3
    assert_eq!(state.p1.astrology.star_power, 0);
    assert_eq!(state.p1.fate.qi_xing_jie_ming, 0); // 标记一次性消耗

    // 标记已消耗：再次生命 ≤ 0 → 正常判负。
    state.modify_actor_hp(PlayerSide::P1, -40, false, false);
    assert_eq!(state.death_winner(), Some(PlayerSide::P2));
}

#[test]
fn seven_stars_borrowed_life_without_marker_does_not_revive() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![436];
    let mut state = ReplayState::test_from_fixture(&battle);

    state.modify_star_power(PlayerSide::P1, 5);
    state.actor_mut(PlayerSide::P1).fate.qi_xing_jie_ming = 0; // 标记已消耗
    state.modify_actor_hp(PlayerSide::P1, -33, false, false);

    assert_eq!(state.death_winner(), Some(PlayerSide::P2));
}

// ---- FateStrategy 437 搏命之勇 / 431 风灵锻躯 / 430 截拳式 ----
// （2026-08-08 oracle 采集 A 诊断修复，DIAG_20260808_hf_32219000.md）
//
// oracle 锚点：mirror-32219000-human-01
// - 4b1ec427bba401c0/round-13 checkpoint[0]：p2 buffs {2:2, 100:2}
//   （加攻 2 = 神力草 10011 1 层 + 437 1 层；内伤 2 = 437×2），
//   round-17 checkpoint[0]：p1 buffs {2:1}（无 10011，仅 437）。
// - a0bc55ed878b63ea/round-20 checkpoint[0]：p2 buffs 10023=97
//   （= 96 基础 + 431 转换 1）+ maxHp 212（= 115 基础 + 97 体魄）；
//   round-13 checkpoint[0]：maxHp 150（149 + 1）。
// - f53c00aa5c2e8f5d/round-08 checkpoint[2]（灵羽后）：p1 maxHp 116。
// - 23277929a0794ec4/round-06 checkpoint[1]（李㵘 4000005 拳架势首次
//   击伤后）：p1 attackReduction(103)=1。

#[path = "tests_build_2026_08_extreme.rs"]
mod extreme;
#[path = "tests_build_2026_08_talents.rs"]
mod talents;
