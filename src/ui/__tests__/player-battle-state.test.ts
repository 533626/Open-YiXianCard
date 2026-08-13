import { describe, expect, test } from "bun:test";
import {
  buffArchiveEntry,
  collectBattleStatItems,
  collectBattleStatusItems,
  renderBattleStatRibbon,
} from "../player-battle-state";
import { rustBuffs, type RustSnapshot } from "../rust-wasm-engine";
import { buffLabel } from "../view-utils";
import { playerView } from "./layout-test-helpers";

/**
 * 原作战斗界面状态显示契约（BASE_BATTLE_RULES.md 第十四章，BattleCharacterUI.RefreshBuff）：
 * - BuffCategory.Hidden 不显示；体魄/体魄上限按用户约束保留在状态条显示（例外项）；
 * - 显示位置：Positive/Negative → 角色区，Permanent → 仙命区，其余（Neutral）→ 卡牌区；
 * - QuanJiaShi / GunJiaShi 只显示架势，不显示数值层数（ClearValueLabel）。
 * 分类依据 shared/data/buff-category-archive.json。
 */
describe("player-battle-state 原作 BuffCategory 显示契约", () => {
  test("Hidden 分类 buff 不显示（RefreshBuff 第一行）", () => {
    const player = playerView({
      buffs: {
        LianYun: 5, // BuffType.LianYun(304) Hidden —— 曾显示为「连云」
        SuiFang: 1, // BuffType.SuiFang(340) Hidden —— 曾显示为「碎防」
        ExActionAgain: 1, // BuffType.ExActionAgain(315) Hidden —— 曾显示为「再次行动」
        FengRui: 4, // BuffType.FengRui(8) Positive —— 对照组，应保留
      },
    });
    expect(collectBattleStatusItems(player).map(({ label, value }) => [label, value])).toEqual([
      ["锋锐", 4],
    ]);
  });

  test("体魄在状态条显示，不重复出现在 buff 状态项", () => {
    const player = playerView({
      buffs: { physique: 3, TiPoShangXian: 85 },
    });
    // 体魄/体魄上限保留在状态条显示（用户约束，不对齐原作 RefreshBuff 的 TiPo 排除）。
    expect(collectBattleStatItems(player)).toContainEqual({ label: "体魄", value: 3 });
    expect(collectBattleStatusItems(player)).toEqual([]);
    const html = renderBattleStatRibbon(playerView({
      buffs: { physique: 3, TiPoShangXian: 85, GuaXiang: 8 },
    }));
    expect(html).toContain("体魄");
    expect(html).toContain("卦象");
  });

  test("拳/棍架势只显示标签不显示数值层数（ClearValueLabel）", () => {
    const player = playerView({
      buffs: { QuanJiaShi: 1, GunJiaShi: 2, ZuiQuanJiaShi: 3 },
    });
    const items = collectBattleStatusItems(player);
    const stanceItems = items.filter((item) => item.label === "拳架势" || item.label === "棍架势");
    expect(stanceItems).toHaveLength(2);
    expect(stanceItems.every((item) => item.noValue === true)).toBe(true);
    // 醉拳架势（BuffType.ZuiQuanJiaShi=734）不在 ClearValueLabel 名单内，照常显示数值。
    const drunkStance = items.find((item) => item.label === "醉拳架势");
    expect(drunkStance?.noValue).toBeUndefined();
    expect(drunkStance?.value).toBe(3);
    const html = renderBattleStatRibbon(player);
    expect(html).toContain("拳架势");
    expect(html).toContain("棍架势");
    expect(html).not.toContain("<b>1</b>");
    expect(html).not.toContain("<b>2</b>");
    expect(html).toContain(">3<");
  });

  test("rustBuffs 发射的每个键都能解析到归档分类（防裸键兜底漏过滤）", () => {
    // 全字段非零的 Rust snapshot：rustBuffs 只发射非零键，全非零才能覆盖全部发射键。
    const snapshot: RustSnapshot = {
      hp: 50,
      maxHp: 100,
      defense: 2,
      anima: 3,
      guard: 4,
      physique: 2,
      swordIntent: 3,
      sharpness: 4,
      cloudChain: 5,
      cloudSea: 6,
      momentum: 3,
      agility: 1,
      waterMomentum: 7,
      activatedMetal: 1,
      activatedWater: 1,
      activatedWood: 1,
      activatedFire: 1,
      activatedEarth: 1,
      hexagram: 8,
      starPower: 9,
      attackBonus: 2,
      internalInjury: 2,
      weakness: 2,
      flaw: 2,
      attackReduction: 2,
      entangle: 2,
      externalInjury: 2,
      lostMind: 10,
      actionAgainCount: 11,
      quanStance: 1,
      gunStance: 1,
      // 全量暴露缺口（档 1a/1b）字段：全非零才能覆盖全部发射键。
      metalRing: 1,
      swordEnergy: 2,
      waterMonthSwordFormation: 1,
      waterFormation: 1,
      metalFormation: 1,
      earthFormation: 1,
      fireFormation: 1,
      springFlow: 1,
      waterStealth: 1,
      metalIronBone: 1,
      earthEightWastes: 1,
      woodArray: 1,
      turtleFormation: 1,
      shatterFormation: 1,
      thunderFormation: 1,
      evilGuFormation: 1,
      spiritGatheringFormation: 1,
      heavenCycleSwordFormation: 1,
      heavenForceFormation: 1,
      flowerMazeFormation: 1,
      immovableFormation: 1,
      eightGatesFormation: 1,
      sixYaoFormation: 1,
      bengQuanCunJin: 1,
      bengQuanReturnProfound: 1,
      dreamBengQuanChain: 1,
      immortalBindingTune: 1,
      illusoryTune: 1,
      heartbreakTune: 1,
      wildDanceTune: 1,
      rejuvenationTune: 1,
      xiaoyaoTune: 1,
      xiaoyaoGuqin: 1,
      chaoticMindTune: 1,
      lingGuaArt: 1,
      starMoonFan: 1,
      infiniteHexagramPlate: 1,
      allGoesWell: 1,
      recovery: 1,
      meditation: 1,
      bloodCalamity: 1,
      loneNightWolf: 1,
      leafBladeFlower: 1,
      quietMindset: 1,
      reflectMindset: 1,
      graftFlowersToTree: 1,
      tide: 1,
      dismantleMove: 1,
      allThingsInauspicious: 1,
      fateCycle: 1,
      yellowBirdBehind: 1,
      exorcism: 1,
      iceSnowLotus: 1,
      leafShieldFlower: 1,
      paintFinishingTouch: 1,
      nextTurnDefense: 1,
      ignoreDefenseAttacks: 1,
      nextAttackShatterDefense: 1,
      momentumLimit: 9,
      lastElement: "wood",
      cardQueue: [1, 0],
      slots: [],
    };
    const keys = Object.keys(rustBuffs(snapshot));
    // 钉死键集合：与 rustBuffs() 的发射清单（rust-wasm-engine.ts 的 values 字面量）
    // 完全一致，自文档化 —— rustBuffs 新增/移除/改名发射键都会使本测试失败，
    // 强制同步更新本清单与快照字段，锁的覆盖范围不依赖手工同步。
    // 下方清单按 ASCII 排序书写（与 [...keys].sort() 一致）。
    expect(keys).toHaveLength(81);
    expect([...keys].sort()).toEqual([
      "BaMenJinSuoZhen",
      "BengQuanCunJin",
      "BengQuanFanXuan",
      "BiXie",
      "BingFengXueLian",
      "BuDongJinGangZhen",
      "ChaiZhao",
      "DongZhuJiXian",
      "DuanChangQu",
      "FanZhenXinFa",
      "FengRui",
      "GuYeLang",
      "GuaXiang",
      "GuiJiaZhen",
      "GunJiaShi",
      "HaiChao",
      "HeBaHuang",
      "HuaLongDianJing",
      "HuanYinQu",
      "HuangQueZaiHou",
      "HuiChunQu",
      "HuiFu",
      "HuoLingZhen",
      "JiHuoHuoLing",
      "JiHuoJinLing",
      "JiHuoMuLing",
      "JiHuoShuiLing",
      "JiHuoTuLing",
      "JianQi",
      "JinLingZhen",
      "JingQiXinFa",
      "JuLinZhen",
      "KuangWuQu",
      "KunWuJinHuan",
      "LianYun",
      "LingGuaShu",
      "LiuYaoShaZhen",
      "MengLianBeng",
      "Min",
      "MuLingZhen",
      "QianDun",
      "QuanJiaShi",
      "QuanYong",
      "ShiZhi",
      "ShuiLingZhen",
      "ShuiShi",
      "ShuiYueJianZhen",
      "SuiShaZhen",
      "TianGangJuLiZhen",
      "TianYinKunXianQu",
      "TieGu",
      "TuLingZhen",
      "WanHuaMiHunZhen",
      "WanMoShiXinQu",
      "WanShiRuYi",
      "WuJiGuaPan",
      "WuShiFangYu",
      "XiaCiGongJiSuiFang",
      "XiaHuiHeJiaFang",
      "XiaoYaoGuQin",
      "XiaoYaoQu",
      "XieGuZhen",
      "XingLi",
      "XingYueYuShan",
      "XueGuangZhiZai",
      "YeDunHua",
      "YeRenHua",
      "YiHuaJieMu",
      "YinLeiZhen",
      "YunHai",
      "ZhouTianJianZhen",
      "ZhuShiBuYi",
      "attackBonus",
      "attackReduction",
      "entangle",
      "externalInjury",
      "flaw",
      "internalInjury",
      "physique",
      "swordIntent",
      "weakness",
    ]);
    for (const key of keys) {
      const entry = buffArchiveEntry(key);
      // 裸键兜底（BUFF_TYPE_BY_UI_KEY[key] ?? key）若查不到归档，该键会绕过
      // Hidden 过滤与三区分组 —— 任何 rustBuffs 新增键都必须能解析到分类。
      expect(entry, `rustBuffs 键 ${key} 应能解析到归档分类`).toBeDefined();
      expect(entry?.category, `rustBuffs 键 ${key} 的分类缺失`).toBeDefined();
      // 分类已知的键必须显示真实词条，不能以字面「状态」出现在状态条。
      expect(buffLabel(key), `rustBuffs 键 ${key} 缺少显示词条`).not.toBe("状态");
    }
  });

  test("分组按原作三区：角色(Positive/Negative)→仙命(Permanent)→卡牌(其余)", () => {
    const player = playerView({
      buffs: {
        GuaXiang: 8, // BuffType.GuaXiang(1) Positive —— TOP_RESOURCE，走 stat 条（角色区）
        NeiShang: 5, // BuffType.NeiShang(100) Negative → 角色区
        ChuanChangZiJue: 1, // BuffType.ChuanChangZiJue(10013) Permanent → 仙命区
        XiaoYaoQu: 1, // BuffType.XiaoYaoQu(208) Neutral → 卡牌区
        JiHuoJinLing: 1, // BuffType.JiHuoJinLing(237) Neutral → 卡牌区
      },
      sustainValues: { JianYiLiuZhuan: [2, 1] }, // 持续效果 → 卡牌区
    });
    const items = collectBattleStatusItems(player);
    expect(items.filter((item) => item.zone === "character").map((item) => item.label)).toEqual(["内伤"]);
    expect(items.filter((item) => item.zone === "talent").map((item) => item.label)).toEqual(["穿肠紫蕨"]);
    expect(items.filter((item) => item.zone === "card").map((item) => item.label)).toEqual([
      "逍遥曲",
      "激活金灵",
      "剑意流转",
    ]);
    const html = renderBattleStatRibbon(player);
    expect(html.indexOf("zone-character")).toBeLessThan(html.indexOf("zone-talent"));
    expect(html.indexOf("zone-talent")).toBeLessThan(html.indexOf("zone-card"));
    expect(html).toContain("角色");
    expect(html).toContain("仙命");
    expect(html).toContain("卡牌");
  });
});
