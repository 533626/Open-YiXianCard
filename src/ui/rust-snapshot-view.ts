import type { RustSnapshot } from "./rust-wasm-engine";
import type { PlayerView } from "./types";

export function rustBuffs(snapshot: RustSnapshot): Record<string, number> {
  const values: Record<string, number> = {
    physique: snapshot.physique,
    swordIntent: snapshot.swordIntent,
    FengRui: snapshot.sharpness,
    LianYun: snapshot.cloudChain,
    YunHai: snapshot.cloudSea,
    ShuiShi: snapshot.waterMomentum,
    GuaXiang: snapshot.hexagram,
    XingLi: snapshot.starPower,
    attackBonus: snapshot.attackBonus,
    internalInjury: snapshot.internalInjury,
    weakness: snapshot.weakness,
    flaw: snapshot.flaw,
    attackReduction: snapshot.attackReduction,
    entangle: snapshot.entangle,
    externalInjury: snapshot.externalInjury,
    ShiZhi: snapshot.lostMind,
    // 李㵘锻玄架势：互斥模式标记，与钩子链 detail_entries 同一口径。
    QuanJiaShi: snapshot.quanStance,
    GunJiaShi: snapshot.gunStance,
    // 五行激活累计次数：原版是 JiHuo*Ling Buff（Neutral/卡牌区）每次激活 +1，
    // RefreshBuff 显示"激活X灵 N"。detail_entries 与 snapshot 都有，UI buffs
    // 必须同口径发射，否则左侧状态条只剩 lastElement、丢累计计数。
    JiHuoJinLing: snapshot.activatedMetal,
    JiHuoMuLing: snapshot.activatedWood,
    JiHuoShuiLing: snapshot.activatedWater,
    JiHuoHuoLing: snapshot.activatedFire,
    JiHuoTuLing: snapshot.activatedEarth,
    // 全量暴露缺口（档 1a/1b）：键名用 archive 枚举名（buff-category-archive.json），
    // 与 detail_entries 同口径，原版 RefreshBuff 会显示（Positive/Negative → 角色区、
    // Neutral → 卡牌区）。锚定表由私有 evidence audit 维护。
    KunWuJinHuan: snapshot.metalRing, // BuffType.KunWuJinHuan(273) 锟铻金环 Neutral
    JianQi: snapshot.swordEnergy, // BuffType.JianQi(625) 剑气 Positive
    ShuiYueJianZhen: snapshot.waterMonthSwordFormation, // BuffType.ShuiYueJianZhen(202) 水月剑阵 Neutral
    ShuiLingZhen: snapshot.waterFormation, // BuffType.ShuiLingZhen(244) 水灵阵 Neutral
    JinLingZhen: snapshot.metalFormation, // BuffType.JinLingZhen(242) 金灵阵 Neutral
    TuLingZhen: snapshot.earthFormation, // BuffType.TuLingZhen(246) 土灵阵 Neutral
    HuoLingZhen: snapshot.fireFormation, // BuffType.HuoLingZhen(245) 火灵阵 Neutral
    QuanYong: snapshot.springFlow, // BuffType.QuanYong(270) 泉涌 Neutral
    QianDun: snapshot.waterStealth, // BuffType.QianDun(12) 潜遁 Positive
    TieGu: snapshot.metalIronBone, // BuffType.TieGu(9) 铁骨 Positive
    HeBaHuang: snapshot.earthEightWastes, // BuffType.HeBaHuang(11) 合八荒 Positive
    MuLingZhen: snapshot.woodArray, // BuffType.MuLingZhen(243) 木灵阵 Neutral
    GuiJiaZhen: snapshot.turtleFormation, // BuffType.GuiJiaZhen(252) 龟甲阵 Neutral
    SuiShaZhen: snapshot.shatterFormation, // BuffType.SuiShaZhen(251) 碎杀阵 Neutral
    YinLeiZhen: snapshot.thunderFormation, // BuffType.YinLeiZhen(250) 引雷阵 Neutral
    XieGuZhen: snapshot.evilGuFormation, // BuffType.XieGuZhen(253) 邪蛊阵 Neutral
    JuLinZhen: snapshot.spiritGatheringFormation, // BuffType.JuLinZhen(254) 聚灵阵 Neutral
    ZhouTianJianZhen: snapshot.heavenCycleSwordFormation, // BuffType.ZhouTianJianZhen(255) 周天剑阵 Neutral
    TianGangJuLiZhen: snapshot.heavenForceFormation, // BuffType.TianGangJuLiZhen(257) 天罡聚力阵 Neutral
    WanHuaMiHunZhen: snapshot.flowerMazeFormation, // BuffType.WanHuaMiHunZhen(258) 万花迷魂阵 Neutral
    BuDongJinGangZhen: snapshot.immovableFormation, // BuffType.BuDongJinGangZhen(271) 不动金刚阵 Neutral
    BaMenJinSuoZhen: snapshot.eightGatesFormation, // BuffType.BaMenJinSuoZhen(256) 八门金锁阵 Neutral
    LiuYaoShaZhen: snapshot.sixYaoFormation, // BuffType.LiuYaoShaZhen(204) 六爻煞阵 Neutral
    BengQuanCunJin: snapshot.bengQuanCunJin, // BuffType.BengQuanCunJin(290) 崩拳寸劲 Neutral
    BengQuanFanXuan: snapshot.bengQuanReturnProfound, // BuffType.BengQuanFanXuan(418) 崩拳返玄 Neutral
    MengLianBeng: snapshot.dreamBengQuanChain, // BuffType.MengLianBeng(725) 梦崩拳连崩 Neutral
    TianYinKunXianQu: snapshot.immortalBindingTune, // BuffType.TianYinKunXianQu(215) 天音困仙曲 Neutral
    HuanYinQu: snapshot.illusoryTune, // BuffType.HuanYinQu(209) 幻音曲 Neutral
    DuanChangQu: snapshot.heartbreakTune, // BuffType.DuanChangQu(211) 断肠曲 Neutral
    KuangWuQu: snapshot.wildDanceTune, // BuffType.KuangWuQu(212) 狂舞曲 Neutral
    HuiChunQu: snapshot.rejuvenationTune, // BuffType.HuiChunQu(213) 回春曲 Neutral
    XiaoYaoQu: snapshot.xiaoyaoTune, // BuffType.XiaoYaoQu(208) 逍遥曲 Neutral
    XiaoYaoGuQin: snapshot.xiaoyaoGuqin, // BuffType.XiaoYaoGuQin(274) 逍遥古琴 Neutral
    WanMoShiXinQu: snapshot.chaoticMindTune, // BuffType.WanMoShiXinQu(214) 万魔蚀心曲 Neutral
    LingGuaShu: snapshot.lingGuaArt, // BuffType.LingGuaShu(358) 灵卦术 Neutral
    XingYueYuShan: snapshot.starMoonFan, // BuffType.XingYueYuShan(260) 星月折扇 Neutral
    WuJiGuaPan: snapshot.infiniteHexagramPlate, // BuffType.WuJiGuaPan(266) 无极卦盘 Neutral
    WanShiRuYi: snapshot.allGoesWell, // BuffType.WanShiRuYi(387) 万事如意 Neutral
    HuiFu: snapshot.recovery, // BuffType.HuiFu(248) 恢复 Positive
    Min: snapshot.meditation, // BuffType.Min(367) 冥 Negative
    XueGuangZhiZai: snapshot.bloodCalamity, // BuffType.XueGuangZhiZai(379) 血光之灾 Neutral
    GuYeLang: snapshot.loneNightWolf, // BuffType.GuYeLang(234) 孤夜狼 Neutral
    YeRenHua: snapshot.leafBladeFlower, // BuffType.YeRenHua(278) 叶刃花 Neutral
    JingQiXinFa: snapshot.quietMindset, // BuffType.JingQiXinFa(203) 静气心法 Neutral
    FanZhenXinFa: snapshot.reflectMindset, // BuffType.FanZhenXinFa(217) 反震心法 Neutral
    YiHuaJieMu: snapshot.graftFlowersToTree, // BuffType.YiHuaJieMu(216) 移花接木 Neutral
    HaiChao: snapshot.tide, // BuffType.HaiChao(247) 海潮 Neutral
    ChaiZhao: snapshot.dismantleMove, // BuffType.ChaiZhao(459) 拆招 Neutral
    ZhuShiBuYi: snapshot.allThingsInauspicious, // BuffType.ZhuShiBuYi(381) 诸事不宜 Neutral
    DongZhuJiXian: snapshot.fateCycle, // BuffType.DongZhuJiXian(390) 命运轮回 Neutral
    HuangQueZaiHou: snapshot.yellowBirdBehind, // BuffType.HuangQueZaiHou(264) 黄雀在后 Neutral
    BiXie: snapshot.exorcism, // BuffType.BiXie(13) 辟邪 Positive
    BingFengXueLian: snapshot.iceSnowLotus, // BuffType.BingFengXueLian(14) 冰封雪莲 Positive
    YeDunHua: snapshot.leafShieldFlower, // BuffType.YeDunHua(276) 叶盾花 Neutral
    HuaLongDianJing: snapshot.paintFinishingTouch, // BuffType.HuaLongDianJing(281) 画龙点睛 Neutral
    XiaHuiHeJiaFang: snapshot.nextTurnDefense, // BuffType.XiaHuiHeJiaFang(7) 下回合加防 Positive
    WuShiFangYu: snapshot.ignoreDefenseAttacks, // BuffType.WuShiFangYu(4) 无视防御 Positive
    XiaCiGongJiSuiFang: snapshot.nextAttackShatterDefense, // BuffType.XiaCiGongJiSuiFang(383) 下次攻击碎防 Neutral
  };
  // actionAgainCount is a lifetime/event counter, not the ExActionAgain buff.
  // It remains available in the engine inspector instead of masquerading as a buff.
  return Object.fromEntries(Object.entries(values).filter(([, value]) => value !== 0));
}

export function rustElements(snapshot: RustSnapshot): PlayerView["activatedElements"] {
  return ([
    ["metal", snapshot.activatedMetal],
    ["water", snapshot.activatedWater],
    ["wood", snapshot.activatedWood],
    ["fire", snapshot.activatedFire],
    ["earth", snapshot.activatedEarth],
  ] as const).filter(([, value]) => value > 0).map(([element]) => element);
}
