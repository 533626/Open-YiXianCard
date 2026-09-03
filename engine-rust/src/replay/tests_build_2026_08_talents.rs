use super::*;

#[test]
fn talent_171_opening_external_injury_uses_star_erosion_kernel() {
    // Talent 171（搏命之勇；BattleCharacter.cs:1180, 1738-1740）通过
    // ModifyBuffValue(WaiShang, 1) 发放开局外伤。p2 的 30103 星蚀先在
    // 自身开场为 p1 装载一次性 +3，因此 p1 必须得到 4，而不是构造期的 1。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.first_player_side = PlayerSide::P2;
    battle.players.p1.talents = vec![171];
    battle.players.p2.talents = vec![30103];

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.attack_bonus, 1);
    assert_eq!(state.p1.status.external_injury, 4);
    assert_eq!(state.p1.astrology.star_erosion, 0);
}

#[test]
fn bo_ming_zhi_yong_battle_start_grants_attack_bonus_and_internal_injury() {
    // 搏命之勇 FateStrategy 437（otherParams=[1,2]）：开局 +1 加攻 +2 内伤。
    // FateStrategyFunctions.cs:591-594（OnBattleStart 内）：
    //   if (HasFateStrategy(437) && IsSwitchActive(src, 437)) {
    //       JiaGong += otherParams[0]; NeiShang += otherParams[1]; }
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![437];
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10011".to_string(), 1); // 神力草 +1 加攻（锚 4b1ec427bba401c0 cp0 p2）
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.attack_bonus, 2); // 神力草 1 + 搏命之勇 1
    assert_eq!(state.p1.status.internal_injury, 2); // 搏命之勇 2
}

#[test]
fn bo_ming_zhi_yong_respects_switch_active_temp_data() {
    // IsSwitchActive（FateStrategyFunctions.cs:842-851）：tempDatas[id]!=0
    // 视为主动禁用 → 不发放。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![437];
    battle
        .players
        .p1
        .fate_strategy_temp_datas
        .insert("437".to_string(), 1);
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.attack_bonus, 0);
    assert_eq!(state.p1.status.internal_injury, 0);
}

#[test]
fn feng_ling_duan_qu_agility_gain_converts_one_physique() {
    // 风灵锻躯 FateStrategy 431：开局 TianYanFengLingDuanQu(771)=5
    // （FateStrategyFunctions.cs:479-481，otherParams[0]=5）；每次加身法
    // 消耗 1 层换 1 体魄（BattleCharacter.cs:8733-8737）→ maxHp +1。
    // 锚：a0bc55ed878b63ea/round-20 cp0 p2 maxHp 212（鹤步 身法+13 后）。
    let ming_ying = original_card_definition_by_id(10_000_039).expect("missing 冥影身法");
    let mut battle = fixture(deck(ming_ying.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![431];
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.fate.tian_yan_feng_ling_duan_qu, 5); // 开局发放
    assert_eq!(state.p1.core.physique, 0);
    assert_eq!(state.p1.core.max_hp, 30);

    state.test_apply_card_effect(PlayerSide::P1, &ming_ying, 0);

    assert_eq!(state.p1.turn.agility, 10); // 冥影身法 身法+10
    assert_eq!(state.p1.fate.tian_yan_feng_ling_duan_qu, 4); // 消耗 1 层
    assert_eq!(state.p1.core.physique, 1); // +1 体魄
    assert_eq!(state.p1.core.max_hp, 31); // maxHp 同步 +1
}

#[test]
fn feng_ling_duan_qu_limited_to_five_conversions_per_battle() {
    // 每场战斗仅转换 5 次（771 计数耗尽后不再转换）。
    let ming_ying = original_card_definition_by_id(10_000_039).expect("missing 冥影身法");
    let mut battle = fixture(deck(ming_ying.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![431];
    let mut state = ReplayState::test_from_fixture(&battle);

    for _ in 0..6 {
        state.test_apply_card_effect(PlayerSide::P1, &ming_ying, 0);
    }

    assert_eq!(state.p1.core.physique, 5); // 第 6 次不再转换
    assert_eq!(state.p1.fate.tian_yan_feng_ling_duan_qu, 0);
    assert_eq!(state.p1.core.max_hp, 35);
}

#[test]
fn feng_xu_yu_feng_stacks_after_each_sustain_instance() {
    // Card_392.cs:80-82 applies ShenFa before PingXuYuFeng; therefore the
    // card's own +5 agility is not reflected, while a second instance makes
    // later gains deal 2 damage per agility.  BattleCharacter.cs:8659-8662
    // reads the accumulated BuffType.PingXuYuFeng value.  Oracle anchors:
    // mirror-32219000-human-01 7a325a5cdb60e58b/round-19 and
    // 923c90c1842c1e79/round-16.
    let wind = original_card_definition_by_id(10_392).expect("missing 冯虚御风");
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));

    state.test_apply_card_effect(PlayerSide::P1, &wind, 0);
    assert_eq!(state.p2.core.hp, 30); // first instance's own +5 does not trigger
    assert_eq!(state.p1.turn.agility_gain_damage, 1);

    state.test_apply_card_effect(PlayerSide::P1, &wind, 0);
    assert_eq!(state.p2.core.hp, 25); // second +5 × one prior instance = 5
    assert_eq!(state.p1.turn.agility_gain_damage, 2);

    state.gain_agility(PlayerSide::P1, 1);
    assert_eq!(state.p2.core.hp, 23); // later +1 × two stacked instances = 2
}

#[test]
fn jie_quan_shi_quan_stance_first_damage_applies_attack_reduction() {
    // 截拳式 FateStrategy 430：开局 JieQuanShi(770)=1（FateStrategyFunctions.cs
    // :583-585）；首次造成实际攻击伤害时消耗 1 层，拳架势 → 目标 +1 减攻
    // （BattleCharacter.cs:10981-10990）。锚：23277929a0794ec4/round-06 cp1
    // p1 attackReduction=1（李㵘 4000005 拳架势开局首次击伤）。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![430];
    battle.players.p1.character_id = Some(4_000_005); // 李㵘：拳架势开局
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.fate.jie_quan_shi, 1); // 开局发放
    assert_eq!(state.p1.beng.quan_stance, 1);
    let attack = basic_attack();
    state.test_apply_card_effect(PlayerSide::P1, &attack, 0); // 攻击 3 > 0

    assert_eq!(state.p1.fate.jie_quan_shi, 0); // 已消耗
    assert_eq!(state.p2.status.attack_reduction, 1); // 拳架势 → 减攻
    assert_eq!(state.p2.status.weakness, 0);
}

#[test]
fn jie_quan_shi_non_quan_stance_applies_weakness() {
    // 非拳架势 → 目标 +1 虚弱。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![430];
    let mut state = ReplayState::test_from_fixture(&battle);

    let attack = basic_attack();
    state.test_apply_card_effect(PlayerSide::P1, &attack, 0);

    assert_eq!(state.p1.fate.jie_quan_shi, 0);
    assert_eq!(state.p2.status.weakness, 1);
    assert_eq!(state.p2.status.attack_reduction, 0);
}

#[test]
fn jie_quan_shi_consumed_after_first_damaging_attack() {
    // 每场战斗仅触发一次（770 仅 1 层）：第二次击伤不再施加减攻。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![430];
    battle.players.p1.character_id = Some(4_000_005);
    let mut state = ReplayState::test_from_fixture(&battle);

    let attack = basic_attack();
    state.test_apply_card_effect(PlayerSide::P1, &attack, 0);
    assert_eq!(state.p2.status.attack_reduction, 1);
    state.test_apply_card_effect(PlayerSide::P1, &attack, 0);

    assert_eq!(state.p2.status.attack_reduction, 1); // 不重复发放
    assert_eq!(state.p1.fate.jie_quan_shi, 0);
}

#[test]
fn jie_quan_shi_blocked_attack_does_not_consume() {
    // 实际伤害为 0（防御完全吸收）时不触发（num > 0 条件）。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![430];
    battle.players.p1.character_id = Some(4_000_005);
    battle.players.p2.initial_defense = 5;
    let mut state = ReplayState::test_from_fixture(&battle);

    let attack = basic_attack();
    state.test_apply_card_effect(PlayerSide::P1, &attack, 0);

    assert_eq!(state.p1.fate.jie_quan_shi, 1); // 未消耗
    assert_eq!(state.p2.status.attack_reduction, 0);
}

// ---- FateStrategy 27 开局生命% 顺序 / FateStrategy 417 + talent 199 五行数 ----
// （2026-08-08 oracle 采集 A 诊断修复，DIAG_20260808_hf_32219000.md）

#[test]
fn fate_strategy_27_samples_hp_after_fate_140_gain() {
    // FateStrategyFunctions.OnBattleStart（decompiled build-24610558）：
    // IL_00f1 先执行 Fate 140（ModifyMaxHp(hp/10) + ModifyHp(hp/10)），
    // IL_01c1 才执行 Fate 27 —— `num3 = src.battleTempData.hp *
    // otherParams[0] / 100` 采样的已是 140 增益后的当前 HP。
    // oracle 锚点：mirror-32219000-human-01 93c5c8aa3e3f1cf6/round-10
    // cp0（p2 62+45=107，fates [140,9,27]，手牌非空）普通攻击后 hp 128
    // → 开局 hp/maxHp 131 = 107 + 10（140: 107/10）+ 14（27: 117×12/100）。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.base_max_hp = 62;
    battle.players.p1.extra_max_hp = Some(45);
    battle.players.p1.fate_strategies = vec![140, 27];
    battle.players.p1.hand_cards = vec![1];
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 131);
    assert_eq!(state.p1.core.max_hp, 131);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 24);
}

#[test]
fn fate_strategy_27_alone_still_scales_from_entry_hp() {
    // 无 140 时 FS27 采样进入战斗的 HP（base + extra）：107×12/100 = 12。
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.base_max_hp = 62;
    battle.players.p1.extra_max_hp = Some(45);
    battle.players.p1.fate_strategies = vec![27];
    battle.players.p1.hand_cards = vec![1];
    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 119);
    assert_eq!(state.p1.core.max_hp, 119);
}

#[test]
fn fate_417_talent_199_wu_xing_count_raises_smash_damage_and_water_momentum() {
    // 五行道盟 Fate 417：BattleCharacter.GetWuXingCountInDeck
    // （BattleCharacter.cs:12151-12167）把 talentDatas[199].commonParams
    // 卡名中的五行 token 计入去重五行数（与牌组内 token 去重合并）。
    // oracle 锚点：mirror-32219000-human-01 8d6734aeef1385e6/round-10
    // cp3（t4p1 混元碎击后 p1.waterMomentum 4）：牌组 {水灵•泉涌, 木灵•玫刺,
    // 火灵•瞬燃}=3 + talent199 {水灵印 7000006, 土灵印 7000011,
    // 金灵•铁骨 7000035} 新增 {土, 金}=5 → 混元碎击 4+5×4=24 伤 →
    // 泉涌 QuanYong（spring_flow）转化 24/5=4 水势（引擎原 3：牌组仅 3
    // 五行 → 16 伤 → 16/5=3）。
    let mut spring = card(7_000_059, 7_000_059, "水灵•泉涌");
    spring.anima = Some(2);
    spring.other_params = vec![1];
    let mut smash = card(7_000_066, 7_000_066, "混元碎击");
    smash.attack = Some(4);
    smash.other_params = vec![4];
    let mut thorn = card(7_000_027, 7_000_027, "木灵•玫刺");
    thorn.attack = Some(4);
    let mut flash = card(7_000_038, 7_000_038, "火灵•瞬燃");
    flash.other_params = vec![4];
    let mut battle = fixture(deck(smash.clone()), deck(basic_attack()));
    battle.players.p1.cards = vec![spring.clone(), smash.clone(), thorn, flash];
    battle.players.p1.active_slot_count = 4;
    battle.players.p1.talents = vec![199];
    battle.players.p1.fate_strategies = vec![417];
    battle
        .players
        .p1
        .talent_card_params
        .insert("199".to_string(), vec![7_000_006, 7_000_011, 7_000_035]);
    let mut state = ReplayState::test_from_fixture(&battle);

    // talent 199 开局激活水灵印 → 水已激活；泉涌发放 spring_flow=1。
    state.test_apply_card_effect(PlayerSide::P1, &spring, 0);
    assert_eq!(state.p1.elements.spring_flow, 1);

    // 混元碎击：4 + 5×4 = 24 伤；QuanYong 转化 24/5 = 4 水势。
    state.test_apply_card_effect(PlayerSide::P1, &smash, 0);
    assert_eq!(state.p2.core.hp, 30 - 24);
    assert_eq!(state.p1.elements.water_momentum, 4);
    assert_eq!(state.p1.elements.spring_flow, 0);
}

// ---- 促局飞袭 FateStrategy 416（CuJuFeiXi 768）----
// 证据：FateStrategyFunctions.cs:571-573（OnBattleStart 发放 otherParams[0]=5）；
// CardActionBase.cs:3996-4002（OnAfterExecuted：名字含「火灵」的牌消耗层数
// 并对对方追加一次 Attack(buffValue)）。oracle 锚点：mirror-32219000-human-01
// cae463212f8c4c43/round-15 t5u1（temp 烈燎原 768:5→消耗、第 4 攻击段
// 8 = 5+加攻3，计入 323/493/644）、round-12 t7u1（temp 赤焰 第 4 段
// 8 = 5+加攻4-减攻1）。实卡段不出现 +8 的原因：768 已被首张「火灵」牌
// （temp 或实卡）消耗，而非 temp/实卡路径差异。

#[test]
fn cu_ju_fei_xi_fire_named_card_extra_attack_consumed_once() {
    let chi_yan = original_card_definition_by_id(7_000_022).expect("missing 火灵•赤焰");
    let mut battle = fixture(deck(chi_yan.clone()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![416];
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.fate.cu_ju_fei_xi, 5); // OnBattleStart 发放

    // 2×(attack 4) + 416 追加 5 = 13（无加攻/减攻；[火灵] 追加需激活火灵，
    // 本测试未激活不触发）。
    state.test_apply_card_effect(PlayerSide::P1, &chi_yan, 0);
    assert_eq!(state.p2.core.hp, 30 - 13);
    assert_eq!(state.p1.fate.cu_ju_fei_xi, 0); // 已消耗

    // 第二张「火灵」牌不再追加 416（768 已消耗）；但上一张 赤焰 已记录
    // lastElement=火，CheckWuXing(火灵) 成立 → [火灵] 追加 4 正常触发：
    // 2×4 + 4 = 12。
    state.test_apply_card_effect(PlayerSide::P1, &chi_yan, 0);
    assert_eq!(state.p2.core.hp, 30 - 13 - 12);
    assert_eq!(state.p1.fate.cu_ju_fei_xi, 0);
}

#[test]
fn cu_ju_fei_xi_temp_execution_five_elements_cycle_r15_anchor() {
    // 五行流转 temp 执行 + 促局飞袭：cae463212f8c4c43/round-15 t5u1
    // p2 103→59（承伤 44 = 3×11 + maxHp 钳制 3 + 416 追加 8），
    // maxHp 103→67（钩子 delta = 103-70+3 = 36）。
    let an_xiang = original_card_definition_by_id(7_000_061).expect("missing 木灵•暗香");
    let cycle = original_card_definition_by_id(7_010_067).expect("missing 五行流转");
    let lie_liao_yuan = original_card_definition_by_id(7_010_056).expect("missing 火灵•烈燎原");
    let mut battle = fixture(
        {
            let mut cards = vec![an_xiang.clone(), cycle.clone(), lie_liao_yuan.clone()];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.fate_strategies = vec![416];
    battle.players.p1.active_slot_count = 8;
    battle.players.p2.base_max_hp = 103;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.attack_bonus = 3; // 锚点：r15 t5u1 p1 加攻 3

    state.test_apply_card_effect(PlayerSide::P1, &cycle, 1);

    assert_eq!(state.p2.core.hp, 59);
    assert_eq!(state.p2.core.max_hp, 67);
    assert_eq!(state.p1.fate.cu_ju_fei_xi, 0);
}

#[test]
fn cu_ju_fei_xi_temp_execution_chi_yan_r12_anchor() {
    // cae463212f8c4c43/round-12 t7u1：temp 火灵•赤焰（7000022 基卡，
    // 五行流转 7000067 rarity 0 钳制）原版 p1 71→42（29 =
    // 2×(4+4-1) + (4+4-1) + 5+4-1，减攻 1 全段生效）；引擎修复前 21。
    let an_xiang = original_card_definition_by_id(7_000_061).expect("missing 木灵•暗香");
    let cycle = original_card_definition_by_id(7_000_067).expect("missing 五行流转");
    let chi_yan = original_card_definition_by_id(7_000_022).expect("missing 火灵•赤焰");
    let mut battle = fixture(
        {
            let mut cards = vec![an_xiang.clone(), cycle.clone(), chi_yan.clone()];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.fate_strategies = vec![416];
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.core.attack_bonus = 4; // 锚点：r12 t7u1 p2 加攻 4
    state.p1.status.attack_reduction = 1; // 锚点：减攻 1

    state.test_apply_card_effect(PlayerSide::P1, &cycle, 1);

    assert_eq!(state.p2.core.hp, 30 - 29);
    assert_eq!(state.p1.fate.cu_ju_fei_xi, 0);
}

#[test]
fn five_elements_cycle_dream_card_temp_execution_unclamped_7020089_anchor() {
    // 五行流转 temp 执行梦牌不钳制：梦•火灵聚炎（7020089）配置无 rarity
    // 字段（CardConfig.rarity=0），原版 Card_7000067.cs 的钳制比较
    // `rarity > card_.cardConfig.rarity`（0 > 0）不成立 → 整卡执行
    // （anima 3、otherParams [2,1]）。oracle 锚点：7a0da91b97b4d1e9/
    // round-07 cp[0] p2.anima 3=0+3、t3u1 p2.anima 6、t17u1 p1 15→5
    // （2+8×1，maxHp 同减）。引擎修复前按 id 档位推断 rarity=2 误钳到
    // 7000089（anima 1）。
    let an_xiang = original_card_definition_by_id(7_000_061).expect("missing 木灵•暗香");
    let cycle = original_card_definition_by_id(7_000_067).expect("missing 五行流转");
    let dream_ju_yan = original_card_definition_by_id(7_020_089).expect("missing 梦•火灵聚炎");
    assert_eq!(dream_ju_yan.anima, Some(3));
    assert_eq!(dream_ju_yan.other_params, vec![2, 1]);
    let mut battle = fixture(
        {
            let mut cards = vec![an_xiang.clone(), cycle.clone(), dream_ju_yan.clone()];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);

    // 木灵•暗香(木) → 五行流转 → 梦•火灵聚炎(火)：木生火成立。
    state.test_apply_card_effect(PlayerSide::P1, &cycle, 1);

    assert_eq!(state.p1.core.anima, 3); // 整卡 anima，非钳制 7000089 的 1
    let anima = 1;
    assert_eq!(state.p2.core.hp, 30 - (2 + 3 * anima)); // 2 + anima×1 = 5
    assert_eq!(state.p2.core.max_hp, 30 - (2 + 3 * anima));
}

#[test]
fn five_elements_cycle_dream_card_temp_execution_unclamped_7040089_anchor() {
    // 同上的化神档梦牌：7040089 梦•火灵聚炎（anima 4、otherParams [2,2]）
    // 配置 rarity 同样缺失 → 不钳制。oracle 锚点：ac9dacde7087f49d/round-13
    // cp[8] p1.anima 18=14+4、p2 81→43（2+18×2）、maxHp 99→61。
    let an_xiang = original_card_definition_by_id(7_000_061).expect("missing 木灵•暗香");
    let cycle = original_card_definition_by_id(7_000_067).expect("missing 五行流转");
    let dream_ju_yan = original_card_definition_by_id(7_040_089).expect("missing 梦•火灵聚炎");
    assert_eq!(dream_ju_yan.anima, Some(4));
    assert_eq!(dream_ju_yan.other_params, vec![2, 2]);
    let mut battle = fixture(
        {
            let mut cards = vec![an_xiang.clone(), cycle.clone(), dream_ju_yan.clone()];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &cycle, 1);

    assert_eq!(state.p1.core.anima, 4);
    assert_eq!(state.p2.core.hp, 30 - (2 + 4 * 2)); // 2 + anima×2 = 10
    assert_eq!(state.p2.core.max_hp, 30 - (2 + 4 * 2));
}

#[test]
fn five_elements_cycle_regular_card_temp_execution_still_clamped_7020022() {
    // 普通牌仍按 CardConfig.rarity 钳制：火灵•赤焰 7020022（rarity 2）
    // > 五行流转 7000067（rarity 0）→ temp 用基卡 7000022（attack 4×2、
    // otherParams [4]）。oracle 锚点：cae463212f8c4c43/round-12 t7u1
    // 承伤 29（含 416 追加）；此处无 416：2×4 + [火灵]追加 4 = 12。
    let an_xiang = original_card_definition_by_id(7_000_061).expect("missing 木灵•暗香");
    let cycle = original_card_definition_by_id(7_000_067).expect("missing 五行流转");
    let chi_yan = original_card_definition_by_id(7_020_022).expect("missing 火灵•赤焰");
    let mut battle = fixture(
        {
            let mut cards = vec![an_xiang.clone(), cycle.clone(), chi_yan.clone()];
            cards.resize_with(DECK_SIZE, basic_attack);
            cards
        },
        deck(basic_attack()),
    );
    battle.players.p1.active_slot_count = 8;
    let mut state = ReplayState::test_from_fixture(&battle);

    state.test_apply_card_effect(PlayerSide::P1, &cycle, 1);

    assert_eq!(state.p1.core.anima, 0); // temp 不付 anima 费用（赤焰 anima -1 是费用）
    assert_eq!(state.p2.core.hp, 30 - (2 * 4 + 4)); // 基卡 4×2 + [火灵]追加 4
    assert_eq!(state.p2.core.max_hp, 30);
}

// ---- 气吞山河 4000060（bfa41ba8 已实现主体；本测试固化「无持续成长 + 打出 maxHp+12 + 后招」语义，trace 锚点：wave-f c5c3/13、diag-b 4085/15）----
#[test]
fn qi_swallow_mountains_has_no_passive_turn_growth() {
    // Card_4000060.cs OnExecuted 只有 ModifyMaxHp(otherParams[0]) +
    // CheckHouZhao→ModifyHp(otherParams[1])：未打出时无任何逐回合效果。
    let mut qi_swallow = card(4_010_060, 4_000_060, "气吞山河");
    qi_swallow.other_params = vec![12, 24];
    let mut cards = vec![basic_attack(), qi_swallow.clone()];
    cards.resize_with(DECK_SIZE, basic_attack);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 2;
    battle.max_actor_turns = Some(1); // 只打 slot0 普通攻击，4000060 不打出
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.turn.adaptation = 0;
    assert_eq!(state.p1.core.max_hp, 30);
    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.core.max_hp, 30); // 未打出：无成长
}

#[test]
fn qi_swallow_mountains_play_raises_max_hp_only_and_rear_move_heals() {
    // Card_4000060.cs IL_00cd ModifyMaxHp(otherParams[0]=12)（hp 不同步，
    // 4085/15 t9u1 hp 94 不变、maxHp 115→127）；IL_015d CheckHouZhao 成立时
    // ModifyHp(otherParams[1]=24)。原版 CardActionBase.cs:5254 CheckHouZhao：
    // 后招成立条件 = cardItem.hadUsed（本卡槽再次打出），故首次打出无后招
    // （4085/15 四次打出均首次 → hp 不变；c5c3/13 t17u1 同）。
    let mut qi_swallow = card(4_010_060, 4_000_060, "气吞山河");
    qi_swallow.other_params = vec![12, 24];
    let mut cards = vec![qi_swallow.clone(), basic_attack()];
    cards.resize_with(DECK_SIZE, basic_attack);
    let mut battle = fixture(cards, deck(basic_attack()));
    battle.players.p1.active_slot_count = 2;
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p1.turn.adaptation = 0;
    state.p1.core.hp = 10;
    state.p1.core.anima = 5; // 4000060 耗 2 灵气（anima=-2，fixture 配置）
    state.test_execute_one_card(PlayerSide::P1); // slot0 4000060 首次打出
    assert_eq!(state.p1.core.max_hp, 42); // 30 + 12
    assert_eq!(state.p1.core.hp, 10); // 首次打出无后招，hp 不变
    state.test_execute_one_card(PlayerSide::P1); // slot1 普通攻击
    state.test_execute_one_card(PlayerSide::P1); // slot0 4000060 再次打出（hadUsed → 后招）
    assert_eq!(state.p1.core.max_hp, 54); // 42 + 12
    assert_eq!(state.p1.core.hp, 34); // 10 + 24（后招）
}
