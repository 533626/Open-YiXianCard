use super::*;

#[test]
fn dream_windward_palms_adds_attack_after_a_defense_gain() {
    // CardConfig 10020081: 5 攻×2，otherParams=[3,1]。
    // Card_10000081.cs:67-115：主攻击完成后读取 JiLuJiaFang，若曾正向加防，
    // 追加 Attack(3, 1)；BattleCharacter.cs:10211-10215 在每次正向 ModifyDef
    // 时累加 JiLuJiaFang。引擎以 DefenseLedger（resources.rs:811-831）承载该
    // 共享标志，不能只按当前防御值判断。
    let palm = original_card_definition_by_id(10_020_081).expect("missing 梦•迎风掌");
    assert_eq!(palm.attack, Some(5));
    assert_eq!(palm.attack_count, Some(2));
    assert_eq!(palm.other_params, vec![3, 1]);

    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.initial_defense = 5;
    let mut no_gain = ReplayState::test_from_fixture(&battle);
    no_gain.test_apply_card_effect(PlayerSide::P1, &palm, 0);
    assert_eq!(no_gain.p2.core.hp, 20, "初始防御不是 JiLuJiaFang");

    let mut with_gain = ReplayState::test_from_fixture(&battle);
    with_gain.gain_defense(PlayerSide::P1, 1);
    with_gain.test_apply_card_effect(PlayerSide::P1, &palm, 0);
    assert_eq!(with_gain.p2.core.hp, 17, "已加防后追加 3 攻");
}

#[test]
fn extreme_six_yao_death_formation_config_matches_build_24610558() {
    let base = original_card_definition_by_id(4000100).expect("missing 极•六爻绝阵");
    assert_eq!(base.other_params, vec![1]);
    assert_eq!(base.action_again, Some(true));
    assert_eq!(
        base.card_type.as_ref().map(|card_type| card_type.value),
        Some(3),
        "Sustain"
    );
    assert_eq!(
        original_card_definition_by_id(4010100)
            .unwrap()
            .other_params,
        vec![2]
    );
    assert_eq!(
        original_card_definition_by_id(4020100)
            .unwrap()
            .other_params,
        vec![3]
    );
}

#[test]
fn extreme_six_yao_death_formation_grants_liu_yao_buff_and_hexagram_gain_damages() {
    // Card_4000100.cs: Cast → ModifyBuffValue(LiuYaoShaZhen, otherParams[0])。
    // 持续效果在共享 gain_hexagram hook（BattleCharacter.cs:8761-8766）：
    // 每加 1 卦象 → 对方 delta × 层数伤害（先扣防）。
    let formation = original_card_definition_by_id(4000100).expect("missing 极•六爻绝阵");
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(formation.clone()), deck(basic_attack())));

    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);

    assert_eq!(state.p1.formations.six_yao_formation, 1);
    assert_eq!(state.p2.core.hp, 30);

    // 加 2 卦象 → 对方 2×1 伤害（0 防目标）。
    state.gain_hexagram(PlayerSide::P1, 2);
    assert_eq!(state.p2.core.hp, 28);
    assert_eq!(state.p1.astrology.hexagram, 2);
}

#[test]
fn extreme_mantis_catches_cicada_config_matches_build_24610558() {
    let base = original_card_definition_by_id(4000101).expect("missing 极•螳螂捕蝉");
    assert_eq!(base.attack, Some(2));
    assert_eq!(base.attack_count, Some(2));
    assert_eq!(base.other_params, vec![2]);
    assert_eq!(
        original_card_definition_by_id(4010101)
            .unwrap()
            .other_params,
        vec![3]
    );
    assert_eq!(
        original_card_definition_by_id(4020101)
            .unwrap()
            .other_params,
        vec![4]
    );
}

#[test]
fn extreme_mantis_catches_cicada_attacks_twice_then_rear_move_attacks_again() {
    // Card_4000101.cs: Attack(attack, attackCount) → ModifyBuffValue(JiaGong,
    // otherParams[0]) → CheckHouZhao 成功时再 Attack(attack, attackCount)。
    let mantis = original_card_definition_by_id(4000101).expect("missing 极•螳螂捕蝉");
    let mut battle = fixture(deck(mantis.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1; // 满足费用
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &mantis, 0);

    // 主攻击 2×2 段 = 4；无后招（非首次出牌）。
    assert_eq!(state.p2.core.hp, 30 - 4);
    assert_eq!(state.p1.core.attack_bonus, 2);

    // 后招成功 → 追加同数值同段数攻击；后招命中时新获得的 2 层加攻已生效
    // （Attack() 共享结算读取 JiaGong）：主攻击 2×2=4，后招 (2+2)×2=8。
    let mut battle = fixture(deck(mantis.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_configure_p1(|player| player.fate.next_rear_move_bypass = 1);

    state.test_apply_card_effect(PlayerSide::P1, &mantis, 0);

    assert_eq!(state.p2.core.hp, 30 - 12); // 4 + 后招 8
    assert_eq!(state.p1.core.attack_bonus, 2);
}

#[test]
fn extreme_forge_bone_config_matches_build_24610558() {
    let base = original_card_definition_by_id(10000100).expect("missing 极•锻骨");
    assert_eq!(base.physique, Some(1));
    assert_eq!(base.other_params, vec![1, 3, 10]);
    assert_eq!(
        original_card_definition_by_id(10010100)
            .unwrap()
            .other_params,
        vec![2, 3, 10]
    );
    assert_eq!(
        original_card_definition_by_id(10020100)
            .unwrap()
            .other_params,
        vec![3, 3, 10]
    );
}

#[test]
fn extreme_forge_bone_grants_duan_gu_charges_and_agility() {
    // Card_10000100.cs: ModifyTiPo(physique) → ModifyBuffValue(DuanGu,
    // otherParams[0]) → ModifyBuffValue(ShenFa, otherParams[2])。DuanGu 消耗在
    // 共享攻击结算（BattleCharacter.cs:11499-11503）：每次攻击消耗 1 层、
    // 加 cardConfigDict[10000027].otherParams[2]=3 攻并 +1 体魄——本牌未打
    // 10000027 时攻击加成同样取 10000027 配置（引擎在发牌时补齐 attack
    // bonus）。
    let forge = original_card_definition_by_id(10000100).expect("missing 极•锻骨");
    let mut battle = fixture(deck(forge.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 1; // 满足费用
    let mut state = ReplayState::test_from_fixture(&battle);
    let physique_before = state.p1.core.physique;

    state.test_apply_card_effect(PlayerSide::P1, &forge, 0);

    assert_eq!(state.p1.core.physique, physique_before + 1); // 体魄+1
    assert_eq!(state.p1.formations.forge_bone_attacks, 1); // 下 1 次攻击
    assert_eq!(state.p1.formations.forge_bone_attack_bonus, 3); // 多 3 攻
    assert_eq!(state.p1.turn.agility, 10); // 身法+10

    // 下一次攻击多 3 攻并加 1 体魄，层数消耗。
    let hp_before = state.p2.core.hp;
    state.apply_attack(PlayerSide::P1, 3, 0);
    assert_eq!(hp_before - state.p2.core.hp, 6); // 3 + 锻骨 3
    assert_eq!(state.p1.formations.forge_bone_attacks, 0);
    assert_eq!(state.p1.core.physique, physique_before + 2);
}

#[test]
fn extreme_night_ghost_howl_config_matches_build_24610558() {
    let base = original_card_definition_by_id(10000101).expect("missing 极•夜鬼啸");
    assert_eq!(base.attack, Some(16));
    assert_eq!(base.anima, Some(-1), "费用 1 灵气");
    assert_eq!(base.other_params, vec![1]);
    let rare = original_card_definition_by_id(10010101).unwrap();
    assert_eq!(rare.attack, Some(18));
    assert_eq!(rare.other_params, vec![2]);
    let epic = original_card_definition_by_id(10020101).unwrap();
    assert_eq!(epic.attack, Some(20));
    assert_eq!(epic.other_params, vec![3]);
}

#[test]
fn extreme_night_ghost_howl_attacks_ignoring_defense_and_weakens_both_sides() {
    // Card_10000101.cs: ModifyBuffValue(BenLunWuShiFangYu, 1) →
    // Attack(attack, attackCount) → 双方 ModifyBuffValue(XuRuo, otherParams[0])。
    // BenLunWuShiFangYu 非消耗 buff（ApplyDamage BattleCharacter.cs:10747
    // 只查不扣），覆盖本牌攻击。
    let howl = original_card_definition_by_id(10000101).expect("missing 极•夜鬼啸");
    let mut battle = fixture(deck(howl.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 2; // 满足 1 灵气费用
    battle.players.p2.initial_defense = 20; // 防御足够吸收普通 16 攻
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &howl, 0);

    assert_eq!(state.p2.core.hp, 30 - 16); // 无视防御全量命中
    assert_eq!(state.p2.core.defense, 20); // 防御未被消耗
    assert_eq!(state.p1.status.weakness, 1); // 双方虚弱+1
    assert_eq!(state.p2.status.weakness, 1);
}

// ---- 第三批 build 24610558 新卡（HF 语料 mirror-32219000-human-01）----
// 423 水灵•劲浪 / 429 阴符绝阵 / 7000107 极•五行灵击。证据：
// build-24610558 反编译 Card_429.cs / Card_7000107.cs /
// BattleCharacter.cs:8644-8648（YinFuJueZhen 反伤）/ :10819-10824
// （423 水灵激活）/ CardActionBase.cs:5026（7000107 灵气减免），
// 与 BUILD_24589371_RULE_DELTA.md §3-b。

#[test]
fn water_spirit_surging_wave_config_matches_build_24610558() {
    // 无 Card_423 专属类（§3-b）；本体/稀有/史诗档位仅打印字段。
    let base = original_card_definition_by_id(423).expect("missing 水灵•劲浪");
    assert_eq!(base.attack, Some(9));
    assert_eq!(base.anima, Some(1));
    let rare = original_card_definition_by_id(10423).unwrap();
    assert_eq!(rare.attack, Some(12));
    assert_eq!(rare.anima, Some(2));
    let epic = original_card_definition_by_id(20423).unwrap();
    assert_eq!(epic.attack, Some(15));
    assert_eq!(epic.anima, Some(3));
}

#[test]
fn water_spirit_surging_wave_attacks_and_gains_anima() {
    let wave = original_card_definition_by_id(423).expect("missing 水灵•劲浪");
    let mut battle = fixture(deck(wave.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 3;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &wave, 0);
    assert_eq!(state.p2.core.hp, 30 - 9);
    assert_eq!(state.p1.core.anima, 4);
}

#[test]
fn water_spirit_surging_wave_activation_grants_momentum_and_hp_per_sharpness() {
    // BattleCharacter.cs:10819-10824：当前牌 base 423、水灵已激活
    // （CheckWuXing JiHuoShuiLing）、非后招（AfterCardAction）时，
    // 每消耗 1 锋锐 → 水势+1 且生命及上限+1。锋锐消耗量取 7000099
    // 回锋返还之前的 buffValue2。
    let wave = original_card_definition_by_id(423).expect("missing 水灵•劲浪");
    let mut battle = fixture(deck(wave.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 3;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.sword.sharpness = 5;
    state.p1.elements.activated_elements.push(Element::Water);
    state.test_apply_card_effect(PlayerSide::P1, &wave, 0);
    assert_eq!(state.p1.elements.water_momentum, 5);
    assert_eq!(state.p1.core.max_hp, 30 + 5);
    assert_eq!(state.p1.core.hp, 30 + 5);
    assert_eq!(state.p2.core.hp, 30 - (9 + 5));
    assert_eq!(state.p1.sword.sharpness, 0);
}

#[test]
fn yin_fu_jue_zhen_config_matches_build_24610558() {
    let base = original_card_definition_by_id(429).expect("missing 阴符绝阵");
    assert_eq!(base.other_params, vec![2, 2]);
    assert_eq!(
        original_card_definition_by_id(10429).unwrap().other_params,
        vec![2, 3]
    );
    assert_eq!(
        original_card_definition_by_id(20429).unwrap().other_params,
        vec![2, 4]
    );
}

#[test]
fn yin_fu_jue_zhen_weakens_target_then_reflects_negative_status_gains() {
    // Card_429.cs: ModifyBuffValue(XuRuo, otherParams[0]) →
    // ModifyBuffValue(YinFuJueZhen, otherParams[1])；顺序关键：首轮
    // 虚弱先于 buff，不触发反伤。持续反伤（BattleCharacter.cs:8644-8648）：
    // 对方每获得 1 层负面状态 → 对施法方造成 层数×2 ReflectDamage，
    // 豁免 BuffType.Min「冥」367。
    let formation = original_card_definition_by_id(429).expect("missing 阴符绝阵");
    let mut battle = fixture(deck(formation.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 3;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);
    assert_eq!(state.p2.status.weakness, 2);
    assert_eq!(state.p2.status.yin_fu_jue_zhen, 2);
    // 429 自身施加的虚弱不触发反伤（buff 尚未存在）
    assert_eq!(state.p1.core.hp, 30);

    // 对方获得内伤+3 → 获得负面状态且持有阴符的一方受 3×2=6 反伤。
    // 原版调用 defaultOpponentTarget.ApplyDamage(this, ...) 的 dst 是 this；
    // 反伤归属不是普通 source→opponent 伤害。
    state.add_actor_negative_status(PlayerSide::P2, 100, 3);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p2.core.hp, 30 - 6);

    // 冥（367）豁免：获得冥不触发反伤
    state.add_actor_negative_status(PlayerSide::P2, 367, 2);
    assert_eq!(state.p1.core.hp, 30);
    // 冥本身按原版规则每层扣 3 HP；这里不应再叠加阴符反伤。
    assert_eq!(state.p2.core.hp, 30 - 6 - 2 * 3);

    // 外伤+1 → 再反伤 2
    state.add_actor_negative_status(PlayerSide::P2, 105, 1);
    assert_eq!(state.p1.core.hp, 30);
    assert_eq!(state.p2.core.hp, 30 - 6 - 2 * 3 - 2);
}

#[test]
fn yin_fu_jue_zhen_fate_405_grants_opponent_anima_on_weakness() {
    // BattleCharacter.cs:8822-8825：对手获得虚弱时，持有 405 的角色灵气 +1。
    // Card_429.cs 只施加虚弱和阴符绝阵；灵气来自共享 ModifyBuffValue hook。
    let formation = original_card_definition_by_id(429).expect("missing 阴符绝阵");
    let mut battle = fixture(deck(formation.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![405];
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &formation, 0);

    assert_eq!(state.p2.status.weakness, 2);
    assert_eq!(state.p1.core.anima, 1);
}

#[test]
fn extreme_five_elements_spirit_strike_config_matches_build_24610558() {
    let base = original_card_definition_by_id(7000107).expect("missing 极•五行灵击");
    assert_eq!(base.attack, Some(12));
    assert_eq!(base.anima, Some(-5));
    assert_eq!(base.other_params, vec![2]);
    assert_eq!(
        original_card_definition_by_id(7010107)
            .unwrap()
            .other_params,
        vec![3]
    );
    assert_eq!(
        original_card_definition_by_id(7020107)
            .unwrap()
            .other_params,
        vec![4]
    );
}

#[test]
fn extreme_five_elements_spirit_strike_scales_with_remaining_anima() {
    // Card_7000107.cs: Attack(dst, attack + anima * otherParams[0], attackCount)。
    // 24705509 起 attack=12（24666769 为 8）。
    let strike = original_card_definition_by_id(7000107).expect("missing 极•五行灵击");
    let mut battle = fixture(deck(strike.clone()), deck(basic_attack()));
    battle.players.p1.initial_anima = 10;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.test_apply_card_effect(PlayerSide::P1, &strike, 0);
    assert_eq!(state.p2.core.hp, 30 - (12 + 10 * 2));
}

#[test]
fn extreme_five_elements_spirit_strike_anima_cost_reduces_per_distinct_wu_xing() {
    // CardActionBase.cs:5026 — 与 7000095 同分支：卡组每有 1 种不同
    // 五行少耗 1 灵气（上限 0 灵）。
    let strike = original_card_definition_by_id(7000107).expect("missing 极•五行灵击");
    let water = original_card_definition_by_id(7000100).expect("missing 水灵•乘风浪");
    let metal = original_card_definition_by_id(7000025).expect("missing 金灵•蓄锐");
    let battle = fixture(deck(strike.clone()), deck(basic_attack()));
    let mut state = ReplayState::test_from_fixture(&battle);
    assert_eq!(
        super::super::support::effective_anima_cost(&strike, &state.p1, Some(0)),
        5
    );
    state.p1.deck.slots[1].card = water;
    state.p1.deck.slots[2].card = metal;
    state.p1.deck.active_slot_count = 3;
    assert_eq!(
        super::super::support::effective_anima_cost(&strike, &state.p1, Some(0)),
        3
    );
}

// ---- FateStrategy 429 强攻架势（HF 崩拳簇 oracle 诊断修复）----
//
// oracle 锚点：mirror-32219000-human-01 de7ad80b6043cb21/round-12
// checkpoint[0]（转势 10222 完成后）：p1 加攻=1、身法=14、棍架势；
// 该轮由 oracle 首差（p1.attackBonus original=1 rust=0）定位，修复后
// winner/actorTurn(7)/hpDelta(64) 全部 exact。

#[test]
fn qiang_gong_attack_switch_to_gun_grants_jia_gong() {
    // 强攻架势 FateStrategy 429（otherParams=[1,1]）：切换架势后按最终架势
    // 发奖——变为棍 → +1 加攻。BattleCharacter.SwitchJiaShi
    // (CardActionBase.cs:5604-5614)：先按 QuanJiaShi/GunJiaShi 切换，
    // 再按最终 QuanJiaShi>0 与否发 +1 气势 / +1 加攻。
    let zhuan_shi = original_card_definition_by_id(10_222).expect("missing 转势");
    let mut battle = fixture(deck(zhuan_shi.clone()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_005); // 李㵘：拳架势开局
    battle.players.p1.fate_strategies = vec![429];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.beng.quan_stance, 1);
    assert_eq!(state.p1.beng.gun_stance, 0);
    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0);

    assert_eq!(state.p1.beng.quan_stance, 0);
    assert_eq!(state.p1.beng.gun_stance, 1);
    assert_eq!(state.p1.core.attack_bonus, 1); // 切到棍 → 强攻架势 +1 加攻
    assert_eq!(state.p1.turn.agility, 14); // 转势 身法+8，拳架势 +6
    assert_eq!(state.p1.beng.momentum, 0); // 未发气势
}

#[test]
fn qiang_gong_attack_switch_to_quan_grants_momentum() {
    // 强攻架势：切回拳 → +1 气势（不重复发加攻）。
    let zhuan_shi = original_card_definition_by_id(10_222).expect("missing 转势");
    let mut battle = fixture(deck(zhuan_shi.clone()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_005);
    battle.players.p1.fate_strategies = vec![429];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0); // 拳→棍
    assert_eq!(state.p1.core.attack_bonus, 1);
    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0); // 棍→拳
    assert_eq!(state.p1.beng.quan_stance, 1);
    assert_eq!(state.p1.beng.gun_stance, 0);
    assert_eq!(state.p1.beng.momentum, 1); // 切到拳 → 强攻架势 +1 气势
    assert_eq!(state.p1.core.attack_bonus, 1); // 加攻不重复发放
}

#[test]
fn qiang_gong_attack_switch_bonus_still_consumes_momentum_limit() {
    // 拳奖励走 modify_momentum 常规通道：气势上限为 0 时 +1 气势会溢出为防御
    // （BattleCharacter.ModifyBuffValue 只在下限 clamp，上限走后续 hook）。
    let zhuan_shi = original_card_definition_by_id(10_222).expect("missing 转势");
    let mut battle = fixture(deck(zhuan_shi.clone()), deck(basic_attack()));
    battle.players.p1.character_id = Some(4_000_005);
    battle.players.p1.fate_strategies = vec![429];
    battle.players.p1.initial_momentum_limit = Some(0);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0);
    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0); // 棍→拳：+1 气势，上限 0 → 溢出

    assert_eq!(state.p1.beng.momentum, 0);
    assert!(state.p1.core.defense > 0);
}

// ---- FateStrategy 427 迅身掌（HF 崩拳簇 oracle 诊断修复）----
//
// oracle 锚点：mirror-32219000-human-01 d5884a1c411a0681/round-18
// checkpoint[5]（万玄破魔掌 82 完成后）：p2 身法 6
// （= 冥影身法后 14 − 再次行动消耗 10 + 迅身掌 +2），引擎原为 4；
// 9874c3ab697ed8ec/round-10 checkpoint[11]（迎风掌后）同为 +2 缺失。
// 修复后两轮 winner/actorTurn/hpDelta 全部 exact。

#[test]
fn xun_shen_zhang_palm_card_grants_agility() {
    // 迅身掌 FateStrategy 427（otherParams=[2]）：使用名字含「掌」的牌时
    // +2 身法。FateStrategyFunctions.OnPlayCard
    // (FateStrategyFunctions.cs:791-794)：cardConfig.name.Contains("掌")。
    let palm = original_card_definition_by_id(82).expect("missing 万玄破魔掌");
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![427];
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &palm, 0);

    assert_eq!(state.p1.turn.agility, 2); // 迅身掌 +2 身法
    assert_eq!(state.p1.core.attack_bonus, 0); // 无负面状态可转，不加攻
}

#[test]
fn xun_shen_zhang_non_palm_card_grants_nothing() {
    // 非「掌」牌不触发迅身掌。
    let zhuan_shi = original_card_definition_by_id(10_222).expect("missing 转势");
    let mut battle = fixture(deck(zhuan_shi.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![427];
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &zhuan_shi, 0);

    assert_eq!(state.p1.turn.agility, 8); // 非拳架势：只有转势自身的身法+8
}

// ---- FateStrategy 424 月魂爪（HF 415 簇 oracle 诊断修复）----
//
// oracle 锚点：mirror-32219000-human-01 2995be139404d0ed/round-10
// checkpoint[14]（第 8 格 迎风掌 10010028 完成后）：p1 生命 23→-3（26 伤）
// 引擎原为 22 —— 缺 (内伤2+外伤1+冥4)/3=2 攻 × 2 段；同簇 4 轮
// （2995be139404d0ed/round-10/12、fcb156fa3df7dbe1/round-09/13）首差全部
// 落在此 fate 的第 8 格攻击上，修复后全部 exact。
// 原版 BattleCharacter.CalculateAttack 11413-11421：
// HasFateStrategy(424) && gridNumber == 7 && !AfterCardAciton →
// num += GetDebuffCount() / otherParams[0]=3（每段攻击各自计算）。

#[test]
fn month_claw_grid7_attack_grants_bonus_per_three_debuffs() {
    let palm = original_card_definition_by_id(10_010_028).expect("missing 迎风掌");
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![424];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    // 内伤 7 层（debuff 计数 7 → 7/3 = 2）。
    state.add_actor_negative_status(PlayerSide::P1, 100, 7);
    state.test_apply_card_effect(PlayerSide::P1, &palm, 7);

    // 迎风掌 10010028：5 攻 + min(floor(体魄/6), 5)=0 → 5/段 × 2 段 = 10；
    // 月魂爪 +2/段 × 2 = 4 → 共 14。
    assert_eq!(state.p2.core.hp, 30 - 14);
}

#[test]
fn month_claw_non_grid7_attack_grants_nothing() {
    let palm = original_card_definition_by_id(10_010_028).expect("missing 迎风掌");
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![424];
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.add_actor_negative_status(PlayerSide::P1, 100, 7);
    state.test_apply_card_effect(PlayerSide::P1, &palm, 6); // 非第 8 格

    assert_eq!(state.p2.core.hp, 30 - 10);
}

#[test]
fn month_claw_without_fate_424_grants_nothing() {
    let palm = original_card_definition_by_id(10_010_028).expect("missing 迎风掌");
    let mut battle = fixture(deck(palm.clone()), deck(basic_attack()));
    battle.players.p1.initial_momentum_limit = Some(6);
    let mut state = ReplayState::test_from_fixture(&battle);

    state.add_actor_negative_status(PlayerSide::P1, 100, 7);
    state.test_apply_card_effect(PlayerSide::P1, &palm, 7);

    assert_eq!(state.p2.core.hp, 30 - 10);
}

// ---- FateStrategy 152 灵爪（HF 428 簇 oracle 诊断修复）----
//
// oracle 锚点：mirror-32219000-human-01 5d19850f298ccfce/round-12
// checkpoint[8]（双鬼拍门 10010030 完成后）：p1 生命 116→101（15 伤，
// 护体挡第 1 段）；引擎原为 11 —— 缺 anima<0 分支的 +3 攻；round-16 同构
// （32 vs 22）。修复后两轮全部 exact。
// 原版 BattleCharacter.CalculateAttack 11557-11560：
// HasFateStrategy(152) && (desc.Contains("灵气") || cardConfig.anima < 0)
// → num += otherParams[0]=3。
