// Build 24610558 rotation replay admission：pa_79bca52d 批次已
// 整体准入 fixtures/candidates/，由语料准入门禁覆盖。
// 当前文件只保留 mirror-32219000-human-01 external 批次的 fixture 测试。
// Private fixtures are gated by the `private-fixtures` cargo feature
// exactly like tests_body.rs / tests_percent_roll_decisions.rs.
use super::*;
use crate::fixture::load_fixture_file;

// Build 24610558 rotation HF corpus admission: mirror-32219000-human-01 batch
// (private companion battle-evaluator/fixtures/external). 35 fixtures,
// 31 turn exact below.
// pa_b47606fc and pa_973ca7e4 remain mismatches
// with first-deviation clues outside these four cards (round-13: 幻•藕断丝连
// 298 / 气吞山河 4000060 maxHp-growth trajectory; round-12: 崩拳 momentum
// damage factor on 崩拳•惊触) — see the 413/418 handler comments and the
// 24589371 rotation report.
const HF_MIRROR_BATCH: &str = "mirror-32219000-human-01";

fn hf_fixture_path(digest: &str, round: u32) -> std::path::PathBuf {
    std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("engine-rust has project root parent")
        .join("battle-evaluator")
        .join("fixtures")
        .join("external")
        .join(HF_MIRROR_BATCH)
        .join(digest)
        .join(format!("round-{round:02}.json"))
}

fn assert_hf_exact(digest: &str, round: u32, label: &str) {
    let path = hf_fixture_path(digest, round);
    let fixture = load_fixture_file(&path)
        .unwrap_or_else(|error| panic!("{label}: HF fixture {path:?} loads: {error}"));
    let mut state = ReplayState::test_from_fixture(&fixture);
    let summary = state.test_run();
    assert!(
        summary.matches_fixture(&fixture),
        "{label}: exact mismatch — expected winner={:?} actorTurn={} hpDelta={}, \
         actual winner={:?} actorTurn={} hpDelta={}",
        fixture.expected.winner_side,
        fixture.expected.actor_turn_count,
        fixture.expected.hp_delta_p1_minus_p2,
        summary.winner_side,
        summary.actor_turn_count,
        summary.hp_delta_p1_minus_p2,
    );
}

/// One row of the data-driven HF mirror table: each handwritten fixture test
/// expands to an identically named `#[test]` with the same digest, round, and
/// label arguments. Row order follows the original file order.
struct HfFixtureCase {
    digest: &'static str,
    round: u32,
    label: &'static str,
}

const HF_MIRROR_CASES: &[HfFixtureCase] = &[
    HfFixtureCase {
        digest: "073f9e6c4e9963c6",
        round: 14,
        label: "pa_eafbc8eb (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "167c6665e3ef28f8",
        round: 15,
        label: "pa_33279f92 (10413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "167c6665e3ef28f8",
        round: 18,
        label: "pa_ee4d141b (10413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "36ac37885e37c4b4",
        round: 14,
        label: "pa_61b2d307 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "36ac37885e37c4b4",
        round: 17,
        label: "pa_57450ccc (10413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "3bf731f16858e616",
        round: 16,
        label: "pa_70987759 (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "3d54050707d38d77",
        round: 13,
        label: "pa_0df8fcd9 (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "3d54050707d38d77",
        round: 14,
        label: "pa_d90d77ec (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "3d54050707d38d77",
        round: 16,
        label: "pa_f208abd4 (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "3d54050707d38d77",
        round: 17,
        label: "pa_a8369899 (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "6ada98e501e67070",
        round: 17,
        label: "pa_dc250e57 (Talent134 桃枝如意 opening order)",
    },
    HfFixtureCase {
        digest: "435256f36e896c94",
        round: 16,
        label: "pa_85243e94 (10417 万玄锻灵)",
    },
    HfFixtureCase {
        digest: "82cf25598c61b5c8",
        round: 17,
        label: "pa_f4a9246d (10417 万玄锻灵)",
    },
    HfFixtureCase {
        digest: "85faaea4669c6130",
        round: 16,
        label: "pa_c8b239fa (10413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "8c10f0be4e2f099b",
        round: 11,
        label: "pa_29f56fb7 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "9e9ea97cd313a53f",
        round: 15,
        label: "pa_51a51aee (417 万玄锻灵)",
    },
    HfFixtureCase {
        digest: "a595d9d78225cfaf",
        round: 13,
        label: "pa_da1e4388 (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "bb633f318b3e215f",
        round: 14,
        label: "pa_c6e0dc78 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "be62ce7be1a5e35d",
        round: 13,
        label: "pa_7edc0a46 (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "c0fee931ab7a652d",
        round: 14,
        label: "pa_10b208c3 (417 万玄锻灵)",
    },
    HfFixtureCase {
        digest: "c0fee931ab7a652d",
        round: 18,
        label: "pa_96f7c5c0 (10417 万玄锻灵)",
    },
    HfFixtureCase {
        digest: "c5c32437ee6bcd44",
        round: 12,
        label: "pa_724c7114 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "c5c32437ee6bcd44",
        round: 15,
        label: "pa_1e049df0 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "c5c32437ee6bcd44",
        round: 17,
        label: "pa_95c1e0c2 (413 火灵•焚花印)",
    },
    HfFixtureCase {
        digest: "cc6c32a30b8afe37",
        round: 12,
        label: "pa_13d6a3a1 (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "cc6c32a30b8afe37",
        round: 13,
        label: "pa_7fbfc96d (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "d2fd4c30fdcb3411",
        round: 15,
        label: "pa_e171391e (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "d2fd4c30fdcb3411",
        round: 18,
        label: "pa_efe9f3b6 (418 极•盛气凌人)",
    },
    HfFixtureCase {
        digest: "ece1189c4548ffe2",
        round: 12,
        label: "pa_02d2a5b4 (424 极•云剑猫爪)",
    },
    HfFixtureCase {
        digest: "fcb156fa3df7dbe1",
        round: 12,
        label: "pa_8c1c7900 (413 火灵•焚花印)",
    },
    // 423 水灵•劲浪 / 7000107 极•五行灵击 HF admission（mirror-32219000-human-01）。
    // 423 共 15 个 blocked 轮次转 exact 14 个；pa_7173814e 仍
    // mismatch（猛虎之躯升级/蓄锐/剑意/影枭兔交互的存量缺口，见 handler 注释）。
    // 7000107 共 9 个 blocked 轮次全部转 exact。
    // 429 阴符绝阵 11 个 blocked 轮次转 mismatch：反伤方向与数额经
    // pa_2f49c701（turn 17 匹配）与 pa_6e665f83
    // （turn 13 匹配）验证为原版语义（BattleCharacter.cs:8644-8648），残余偏差
    // 属崩拳自伤/孤虚金书/玄灵愈体等存量缺口，全部留待存量修复任务，不做放宽断言。
    HfFixtureCase {
        digest: "7c1ff23708320972",
        round: 12,
        label: "pa_dbc33736 (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "7c1ff23708320972",
        round: 15,
        label: "pa_981e4950 (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "7da15306319b8105",
        round: 13,
        label: "pa_11b4e8a6 (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "7da15306319b8105",
        round: 16,
        label: "pa_8cfe19d8 (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 12,
        label: "pa_53b60c7f (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 13,
        label: "pa_0c9d2e22 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 14,
        label: "pa_8913e8f9 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 16,
        label: "pa_265d7c85 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 17,
        label: "pa_b5619dd2 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 18,
        label: "pa_8e499ea2 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "9a7fb61248a9c6e6",
        round: 19,
        label: "pa_7b55a34f (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "c3582d5916717083",
        round: 14,
        label: "pa_1cef54be (423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "c3582d5916717083",
        round: 17,
        label: "pa_196ef564 (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "c7d990656fbca696",
        round: 17,
        label: "pa_d92f9d4b (10423 水灵•劲浪)",
    },
    HfFixtureCase {
        digest: "834b4df4a54fc7bf",
        round: 13,
        label: "pa_64d43b3f (7010107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "834b4df4a54fc7bf",
        round: 14,
        label: "pa_d453f16a (7020107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "03ec1dba2ceb1320",
        round: 14,
        label: "pa_d30f61f0 (7010107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "03ec1dba2ceb1320",
        round: 17,
        label: "pa_e9d58e4f (7010107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "b0f3478e712ac92e",
        round: 13,
        label: "pa_9461fc3c (7010107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "e99bc53318fdee48",
        round: 13,
        label: "pa_bb7c32f8 (7010107 极•五行灵击)",
    },
    HfFixtureCase {
        digest: "111ebbf264674c13",
        round: 15,
        label: "pa_1bc4750d (7020107 极•五行灵击)",
    },
    // FateStrategy 429 强攻架势 + 427 迅身掌：
    // pa_973ca7e4 原首差 p1.attackBonus 1vs0（转势切棍后缺
    // 强攻架势 +1 加攻，BattleCharacter.SwitchJiaShi CardActionBase.cs:5604-5614）；
    // pa_ac9b6dc9 / pa_34a231f6 / f3e18665b5c33132/
    // round-14 原首差均为 +2 身法缺失（迅身掌 427，FateStrategyFunctions.cs:
    // 791-794，名字含「掌」的牌 +2 身法）。
    HfFixtureCase {
        digest: "de7ad80b6043cb21",
        round: 12,
        label: "pa_973ca7e4 (429 强攻架势 + 崩拳)",
    },
    HfFixtureCase {
        digest: "9874c3ab697ed8ec",
        round: 10,
        label: "pa_ac9b6dc9 (427 迅身掌)",
    },
    HfFixtureCase {
        digest: "f3e18665b5c33132",
        round: 14,
        label: "pa_2451344c (426 极•锟铻金环 + 427 迅身掌)",
    },
    // FateStrategy 424 月魂爪 + 152 灵爪 + 436 七星借命
    // （DIAG_20260808_hf_32219000.md 415/428 两簇残余轮）：
    // - 415 簇 4 轮（pa_f9d9294f/12、pa_68807780/13）
    //   首差全在 4000003 玩家第 8 格攻击（迎风掌 10010028 / 百杀破境掌）——
    //   缺 424 月魂爪「第 8 格攻击按自身负面状态数/3 加攻」（BattleCharacter.cs
    //   11413-11421）；
    // - 428 簇 2 轮（pa_d592bbb0/16）首差在 4000001 玩家攻击
    //   （双鬼拍门 anima<0）——缺 152 灵爪 anima<0 分支 +3 攻（BattleCharacter.cs
    //   11557-11560）；
    // - pa_f9d9294f 尾盘另缺 436 七星借命：首次生命≤0 时按
    //   （卦象+星力）×3 转换生命及上限（FateStrategyFunctions.cs:587-589 +
    //   BattleExecuter.CharacterResurrectionCheckAsync），p1 星力5 → -3 复活
    //   为 12，随后内伤 -2 → 10 并战至 turn18 取胜。
    HfFixtureCase {
        digest: "2995be139404d0ed",
        round: 10,
        label: "pa_f9d9294f (424 月魂爪 + 436 七星借命)",
    },
    HfFixtureCase {
        digest: "2995be139404d0ed",
        round: 12,
        label: "pa_809d6bdc (424 月魂爪)",
    },
    HfFixtureCase {
        digest: "fcb156fa3df7dbe1",
        round: 9,
        label: "pa_68807780 (424 月魂爪)",
    },
    HfFixtureCase {
        digest: "fcb156fa3df7dbe1",
        round: 13,
        label: "pa_e52bcaae (424 月魂爪)",
    },
    HfFixtureCase {
        digest: "5d19850f298ccfce",
        round: 12,
        label: "pa_d592bbb0 (152 灵爪)",
    },
    HfFixtureCase {
        digest: "5d19850f298ccfce",
        round: 16,
        label: "pa_b678ae72 (152 灵爪)",
    },
    // FateStrategy 437 搏命之勇 / 431 风灵锻躯 / 430 截拳式（2026-08-08
    // oracle 采集 A 修复，DIAG_20260808_hf_32219000.md「采集 A」）：
    // - 437：OnBattleStart 加攻 +1 / 内伤 +2（FateStrategyFunctions.cs:591-594，
    //   otherParams=[1,2]，IsSwitchActive 判定）；pa_6c98fca9/17
    //   原首差 cp0 p2/p1 attackBonus 2vs1 / 1vs0。
    // - 431：OnBattleStart 发 TianYanFengLingDuanQu(771)=5，每次加身法消耗
    //   1 层换 1 体魄（FateStrategyFunctions.cs:479-481 + BattleCharacter.cs
    //   :8733-8737）；pa_26b4c776/13 原首差 cp0 p2 maxHp 212vs211 /
    //   150vs149，pa_ccae5d2b 原首差 cp2 p1 maxHp 116vs115。
    // - 430：OnBattleStart 发 JieQuanShi(770)=1，首次实际击伤时消耗 1 层，
    //   拳架势 → 目标减攻，否则 → 虚弱（FateStrategyFunctions.cs:583-585 +
    //   BattleCharacter.cs:10981-10990）；pa_8cd6f54a 原首差
    //   cp1 p1 attackReduction 1vs0。
    // 复核：pa_68807780/13（415 簇鹤步首卡）不含 431/437/430
    // （p2 fate [14,376,424]），由 424 月魂爪修复转换 exact，与本轮无关，
    // 下方 hf_fcb156fa3df7dbe1_* 既有测试继续守护。
    HfFixtureCase {
        digest: "4b1ec427bba401c0",
        round: 13,
        label: "pa_6c98fca9 (437 搏命之勇)",
    },
    HfFixtureCase {
        digest: "4b1ec427bba401c0",
        round: 17,
        label: "pa_6d58ab42 (437 搏命之勇)",
    },
    HfFixtureCase {
        digest: "a0bc55ed878b63ea",
        round: 20,
        label: "pa_26b4c776 (431 风灵锻躯)",
    },
    HfFixtureCase {
        digest: "a0bc55ed878b63ea",
        round: 13,
        label: "pa_6df6c022 (431 风灵锻躯)",
    },
    HfFixtureCase {
        digest: "f53c00aa5c2e8f5d",
        round: 8,
        label: "pa_ccae5d2b (431 风灵锻躯)",
    },
    HfFixtureCase {
        digest: "23277929a0794ec4",
        round: 6,
        label: "pa_8cd6f54a (430 截拳式)",
    },
    HfFixtureCase {
        digest: "8d6734aeef1385e6",
        round: 10,
        label: "pa_b0a38575 (Fate417+talent199 五行数 → 水势记账)",
    },
    HfFixtureCase {
        digest: "93c5c8aa3e3f1cf6",
        round: 10,
        label: "pa_84f495cd (FS27 开局生命% 在 FS140 后采样)",
    },
    // 五行激活链（7000101 混元化灵 五行数灵气 / 417+talent199 计数 / 万物生长叠层
    // 转加攻）与 F382 霜华守护 加防+1。oracle 锚点见 DIAG_20260808_hf_32219000.md
    // 采集 A：pa_09f4fe94 cp4 anima 13（引擎 10）、
    // pa_cbd30daa cp3 attackBonus 3（引擎 2）、
    // pa_f920d356 cp2 defense 3（引擎 2）。
    HfFixtureCase {
        digest: "4eea252403546e4b",
        round: 16,
        label: "pa_09f4fe94 (7000101 混元化灵 417+talent199 五行数)",
    },
    HfFixtureCase {
        digest: "04bcf167c7fe26d7",
        round: 13,
        label: "pa_cbd30daa (万物生长叠层转加攻)",
    },
    HfFixtureCase {
        digest: "04bcf167c7fe26d7",
        round: 16,
        label: "pa_d78a42b3 (万物生长叠层转加攻)",
    },
    HfFixtureCase {
        digest: "3cba3714dc255204",
        round: 12,
        label: "pa_f920d356 (F382 霜华守护 加防+1)",
    },
    // 采集 B A 组（DIAG_20260808_hf_32219000.md）五行流转簇修复：
    // 混元无极阵 7000058 充能按 GetWuXingCountInDeck 全口径
    // （Card_7000058.cs:115，含 417 混元玉瓶/talent199 卡名）→ 4eea r13
    // 充能 4，t10 混元化灵 激活木灵 触发第 4 次再次行动（原版 t10u2 暗香），
    // 消除 winner 翻转（原版 p1/14/29）。
    HfFixtureCase {
        digest: "4eea252403546e4b",
        round: 13,
        label: "pa_b5731607 (混元无极阵充能全口径 → t10 再次行动链, winner 翻转修复)",
    },
    // 采集 B（DIAG_20260808_hf_32219000.md）修复：
    // F397 灯灵星耀（FateStrategyFunctions.cs:543-546）开局星力 +1、exp 领先
    // ≥ otherParams[0]=5 再 +1（oracle 锚点 cp0 星力 4 vs 2）；
    // F340 乘雁而行（FateStrategyFunctions.cs:487-489）开局 maxHp+20，经 326
    // 砚气（BattleCharacter.cs:9981-9986）回 20% 新上限，Talent64 +1 防御
    // （oracle 锚点 cp0 maxHp/hp 104 vs 84）。
    HfFixtureCase {
        digest: "64e07edecaeef655",
        round: 12,
        label: "pa_62c67ebb (F397 灯灵星耀 开局星力 4)",
    },
    HfFixtureCase {
        digest: "7035f55163951763",
        round: 14,
        label: "pa_71aa57a0 (F397 灯灵星耀 开局星力 4)",
    },
    HfFixtureCase {
        digest: "64e07edecaeef655",
        round: 16,
        label: "pa_fa681ffb (126 澄心剑阵 talent ledger)",
    },
];

fn run_hf_mirror_case(case: &HfFixtureCase) {
    assert_hf_exact(case.digest, case.round, case.label);
}

crate::data_driven_cases! {
    table: HF_MIRROR_CASES,
    run: run_hf_mirror_case,
    suite: "replay::tests_build_2026_08_rotation_fixtures::mirror_batch",
    valid: hf_mirror_batch_case_table_is_valid,
    cases: [
        hf_073f9e6c4e9963c6_round14_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 0,
        hf_167c6665e3ef28f8_round15_burning_flower_seal_rare_blocked_becomes_exact => 1,
        hf_167c6665e3ef28f8_round18_burning_flower_seal_rare_blocked_becomes_exact => 2,
        hf_36ac37885e37c4b4_round14_burning_flower_seal_blocked_becomes_exact => 3,
        hf_36ac37885e37c4b4_round17_burning_flower_seal_rare_blocked_becomes_exact => 4,
        hf_3bf731f16858e616_round16_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 5,
        hf_3d54050707d38d77_round13_extreme_arrogant_blocked_becomes_exact => 6,
        hf_3d54050707d38d77_round14_extreme_arrogant_blocked_becomes_exact => 7,
        hf_3d54050707d38d77_round16_extreme_arrogant_blocked_becomes_exact => 8,
        hf_3d54050707d38d77_round17_extreme_arrogant_blocked_becomes_exact => 9,
        hf_6ada98e501e67070_round17_talent_134_opening_order_becomes_exact => 10,
        hf_435256f36e896c94_round16_ten_thousand_mysteries_forge_rare_blocked_becomes_exact => 11,
        hf_82cf25598c61b5c8_round17_ten_thousand_mysteries_forge_rare_blocked_becomes_exact => 12,
        hf_85faaea4669c6130_round16_burning_flower_seal_rare_blocked_becomes_exact => 13,
        hf_8c10f0be4e2f099b_round11_burning_flower_seal_blocked_becomes_exact => 14,
        hf_9e9ea97cd313a53f_round15_ten_thousand_mysteries_forge_blocked_becomes_exact => 15,
        hf_a595d9d78225cfaf_round13_extreme_arrogant_blocked_becomes_exact => 16,
        hf_bb633f318b3e215f_round14_burning_flower_seal_blocked_becomes_exact => 17,
        hf_be62ce7be1a5e35d_round13_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 18,
        hf_c0fee931ab7a652d_round14_ten_thousand_mysteries_forge_blocked_becomes_exact => 19,
        hf_c0fee931ab7a652d_round18_ten_thousand_mysteries_forge_rare_blocked_becomes_exact => 20,
        hf_c5c32437ee6bcd44_round12_burning_flower_seal_blocked_becomes_exact => 21,
        hf_c5c32437ee6bcd44_round15_burning_flower_seal_blocked_becomes_exact => 22,
        hf_c5c32437ee6bcd44_round17_burning_flower_seal_blocked_becomes_exact => 23,
        hf_cc6c32a30b8afe37_round12_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 24,
        hf_cc6c32a30b8afe37_round13_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 25,
        hf_d2fd4c30fdcb3411_round15_extreme_arrogant_blocked_becomes_exact => 26,
        hf_d2fd4c30fdcb3411_round18_extreme_arrogant_blocked_becomes_exact => 27,
        hf_ece1189c4548ffe2_round12_extreme_cloud_sword_cat_claw_blocked_becomes_exact => 28,
        hf_fcb156fa3df7dbe1_round12_burning_flower_seal_blocked_becomes_exact => 29,
        hf_7c1ff23708320972_round12_water_spirit_surging_wave_blocked_becomes_exact => 30,
        hf_7c1ff23708320972_round15_water_spirit_surging_wave_blocked_becomes_exact => 31,
        hf_7da15306319b8105_round13_water_spirit_surging_wave_blocked_becomes_exact => 32,
        hf_7da15306319b8105_round16_water_spirit_surging_wave_blocked_becomes_exact => 33,
        hf_9a7fb61248a9c6e6_round12_water_spirit_surging_wave_blocked_becomes_exact => 34,
        hf_9a7fb61248a9c6e6_round13_water_spirit_surging_wave_blocked_becomes_exact => 35,
        hf_9a7fb61248a9c6e6_round14_water_spirit_surging_wave_blocked_becomes_exact => 36,
        hf_9a7fb61248a9c6e6_round16_water_spirit_surging_wave_blocked_becomes_exact => 37,
        hf_9a7fb61248a9c6e6_round17_water_spirit_surging_wave_blocked_becomes_exact => 38,
        hf_9a7fb61248a9c6e6_round18_water_spirit_surging_wave_blocked_becomes_exact => 39,
        hf_9a7fb61248a9c6e6_round19_water_spirit_surging_wave_blocked_becomes_exact => 40,
        hf_c3582d5916717083_round14_water_spirit_surging_wave_blocked_becomes_exact => 41,
        hf_c3582d5916717083_round17_water_spirit_surging_wave_blocked_becomes_exact => 42,
        hf_c7d990656fbca696_round17_water_spirit_surging_wave_blocked_becomes_exact => 43,
        hf_834b4df4a54fc7bf_round13_extreme_five_elements_spirit_strike_blocked_becomes_exact => 44,
        hf_834b4df4a54fc7bf_round14_extreme_five_elements_spirit_strike_blocked_becomes_exact => 45,
        hf_03ec1dba2ceb1320_round14_extreme_five_elements_spirit_strike_blocked_becomes_exact => 46,
        hf_03ec1dba2ceb1320_round17_extreme_five_elements_spirit_strike_blocked_becomes_exact => 47,
        hf_b0f3478e712ac92e_round13_extreme_five_elements_spirit_strike_blocked_becomes_exact => 48,
        hf_e99bc53318fdee48_round13_extreme_five_elements_spirit_strike_blocked_becomes_exact => 49,
        hf_111ebbf264674c13_round15_extreme_five_elements_spirit_strike_blocked_becomes_exact => 50,
        hf_de7ad80b6043cb21_round12_qiang_gong_attack_stance_switch_exact => 51,
        hf_9874c3ab697ed8ec_round10_xun_shen_zhang_palm_exact => 52,
        hf_f3e18665b5c33132_round14_xun_shen_zhang_palm_exact => 53,
        hf_2995be139404d0ed_round10_month_claw_qi_xing_jie_ming_exact => 54,
        hf_2995be139404d0ed_round12_month_claw_exact => 55,
        hf_fcb156fa3df7dbe1_round09_month_claw_exact => 56,
        hf_fcb156fa3df7dbe1_round13_month_claw_exact => 57,
        hf_5d19850f298ccfce_round12_spirit_claw_exact => 58,
        hf_5d19850f298ccfce_round16_spirit_claw_exact => 59,
        hf_4b1ec427bba401c0_round13_bo_ming_zhi_yong_exact => 60,
        hf_4b1ec427bba401c0_round17_bo_ming_zhi_yong_exact => 61,
        hf_a0bc55ed878b63ea_round20_feng_ling_duan_qu_exact => 62,
        hf_a0bc55ed878b63ea_round13_feng_ling_duan_qu_exact => 63,
        hf_f53c00aa5c2e8f5d_round08_feng_ling_duan_qu_exact => 64,
        hf_23277929a0794ec4_round06_jie_quan_shi_exact => 65,
        hf_8d6734aeef1385e6_round10_fate_417_talent199_water_momentum_exact => 66,
        hf_93c5c8aa3e3f1cf6_round10_fate_27_hp_reconstruction_exact => 67,
        hf_4eea252403546e4b_round16_hun_yuan_hua_ling_wu_xing_count_exact => 68,
        hf_04bcf167c7fe26d7_round13_wan_wu_sheng_zhang_stack_exact => 69,
        hf_04bcf167c7fe26d7_round16_wan_wu_sheng_zhang_stack_exact => 70,
        hf_3cba3714dc255204_round12_shuang_hua_shou_hu_exact => 71,
        hf_4eea252403546e4b_round13_hun_yuan_wu_ji_zhen_charge_exact => 72,
        hf_64e07edecaeef655_round12_f397_star_power_exact => 73,
        hf_7035f55163951763_round14_f397_star_power_exact => 74,
        hf_64e07edecaeef655_round16_clear_heart_formation_exact => 75,
    ]
}
