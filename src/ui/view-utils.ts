import { battleEventBuffLabels } from "./generated/battle-event-labels";

export function selectField(label: string, id: string, value: string, options: readonly [string, string][]): string {
  return `
    <label class="field">
      <span>${label}</span>
      <select id="${id}">
        ${options.map(([optionValue, optionLabel]) => `
          <option value="${escapeAttribute(optionValue)}" ${value === optionValue ? "selected" : ""}>${escapeHtml(optionLabel)}</option>
        `).join("")}
      </select>
    </label>
  `;
}

export function textField(label: string, id: string, value: string): string {
  return `
    <label class="field">
      <span>${label}</span>
      <input id="${id}" value="${escapeAttribute(value)}" />
    </label>
  `;
}

export function numberField(label: string, id: string, value: number, min: number, max: number): string {
  return `
    <label class="field">
      <span>${label}</span>
      <input type="number" id="${id}" value="${value}" min="${min}" max="${max}" />
    </label>
  `;
}

export function nullableNumberField(label: string, id: string, value: number | null): string {
  return `
    <label class="field">
      <span>${label}</span>
      <input type="number" id="${id}" value="${value ?? ""}" />
    </label>
  `;
}

export function textareaField(label: string, id: string, value: string): string {
  return `
    <label class="field textarea">
      <span>${label}</span>
      <textarea id="${id}" spellcheck="false">${escapeHtml(value)}</textarea>
    </label>
  `;
}

export function stat(label: string, value: string | number): string {
  return `<div><span>${label}</span><b>${escapeHtml(String(value))}</b></div>`;
}

export function buffLabel(key: string): string {
  const generatedLabel = battleEventBuffLabels.get(key);
  if (generatedLabel) return generatedLabel;
  const names: Readonly<Record<string, string>> = {
    recovery: "恢复",
    internalInjury: "内伤",
    weakness: "虚弱",
    flaw: "破绽",
    attackBonus: "加攻",
    attackReduction: "减攻",
    entangle: "困缚",
    externalInjury: "外伤",
    NeiShang: "内伤",
    XuRuo: "虚弱",
    PoZhan: "破绽",
    JianGong: "减攻",
    KunFu: "困缚",
    WaiShang: "外伤",
    Min: "冥",
    ShiZhi: "食滞",
    BeiXingShi: "星蚀",
    swordIntent: "剑意",
    swordEnergy: "剑气",
    GuaXiang: "卦象",
    XingLi: "星力",
    BiXie: "辟邪",
    FengRui: "锋锐",
    SuiFang: "碎防",
    WuShiFangYu: "无视防御",
    ExActionAgain: "再次行动",
    AddHpCount: "累计获得生命",
    LoseHpCount: "累计失去生命",
    LoseHpTimesCount: "失去生命次数",
    ActualDamage: "实际伤害",
    BENLUNGONGJICISHU: "本轮攻击次数",
    BenChangZhanDouTiPoJiShu: "本场体魄计数",
    ZhanDouGongJiJiShu: "本场攻击次数",
    UsedCardCount: "已用牌数",
    WoundedCount: "受伤次数",
    LoseDefCount: "失防次数",
    YunHai: "云海",
    LianYun: "连云",
    ShuiShi: "水势",
    HaiChao: "海潮",
    GuaXiangShengXiaoCiShu: "卦象生效次数",
    WuJiGuaPan: "无极卦盘",
    LingGuaZiYanYiChuFa: "灵卦自衍",
    XiaHuiHeJiaFang: "下回合加防",
    YongGuoChiXuPai: "用过持续牌",
    YongGuoShiXuPai: "用过持续牌",
    XiaZhangPaiJiHuoWuXing: "下张牌激活五行",
    XiaZhangPaiZaiCiXingDong: "下张牌再动",
    XiaZhangPaiLingQiJianHao: "下张牌减耗",
    XiaCiGongJiSuiFang: "下次攻击碎防",
    YuLingXinFaJiaLingQiShiJiaFang: "加灵气时加防",
    YuLingXinFaHaoLingQiShiJiaFang: "耗灵气时加防",
    GongJiShiShiJiaNeiShangHuiHeShu: "攻击施加内伤",
    XiaoHaoJianYi: "待消耗剑意",
    BingFengXueLian: "冰封雪莲",
    KuangJian: "狂剑",
    LingQiBengYongQuanShengXiao: "灵气迸涌",
    ZhenYinXinFa: "镇印心法",
    ShangHunZhouZhenShiJiaNeiShangCiShu: "伤魂咒阵",
    QuanYong: "泉涌",
    KunWuJinHuan: "锟铻金环",
    TieGu: "铁骨",
    QianDun: "潜遁",
    HeBaHuang: "合八荒",
    DuanYa: "断崖",
    JuDingLuo: "巨鼎落",
    BuHaoFengRui: "不耗锋锐",
    ShunYing: "顺应",
    ChaTi: "察体",
    DuanGu: "锻骨",
    QuanJiaShi: "拳架势",
    GunJiaShi: "棍架势",
    JingQiXinFa: "静气心法",
    LiuYaoShaZhen: "六爻煞阵",
    FanZhenXinFa: "反震心法",
    XiaoYaoQu: "逍遥曲",
    HuanYinQu: "幻音曲",
    DuanChangQu: "断肠曲",
    KuangWuQu: "狂舞曲",
    HuiChunQu: "回春曲",
    WanMoShiXinQu: "万魔蚀心曲",
    TianYinKunXianQu: "天音困仙曲",
    JinLingZhen: "金灵阵",
    TuLingZhen: "土灵阵",
    ShuiLingZhen: "水灵阵",
    HuoLingZhen: "火灵阵",
    YinLeiZhen: "引雷阵",
    SuiShaZhen: "碎杀阵",
    GuiJiaZhen: "龟甲阵",
    XieGuZhen: "邪蛊阵",
    JuLinZhen: "聚灵阵",
    TianGangJuLiZhen: "天罡聚力阵",
    ZhouTianJianZhen: "周天剑阵",
    BaMenJinSuoZhen: "八门金锁阵",
    WanHuaMiHunZhen: "万花迷魂阵",
    BuDongJinGangZhen: "不动金刚阵",
    QinShiPai: "琴师牌",
    YeDunHua: "叶盾花",
    FuXianGuTeng: "缚仙古藤",
    HuangQueZaiHou: "黄雀在后",
    FanZhuanChuPai: "反转出牌",
    WuFaXingDong: "无法行动",
    TianYunBiXiong: "天运避凶",
    TianYunQuJi: "天运趋吉",
    // 档 1a 命运轮回（BuffType.DongZhuJiXian=390）：锚定链四处一致
    // （detail_entries snapshot.rs「命运轮回」/ archive id=390 名「命运轮回」/
    // localization Buff_390「命运轮回」）；「洞烛机先」是仙命名不是 Buff 名。
    DongZhuJiXian: "命运轮回",
    QiRuoXuanHe: "气若悬河",
    ChaiZhao: "拆招",
    WanShiRuYi: "万事如意",
    ZhuShiBuYi: "诸事不宜",
    NiShi: "逆势",
    XueGuangZhiZai: "血光之灾",
    Recovery: "回复",
    InternalInjury: "内伤",
    Weakness: "虚弱",
    Flaw: "破绽",
    AttackBonus: "加攻",
    AttackReduction: "减攻",
    Entangle: "困缚",
    ExternalInjury: "外伤",
    Physique: "体魄",
    PhysiqueLimit: "体魄上限",
    SwordIntent: "剑意",
    SwordEnergy: "剑气",
    Hexagram: "卦象",
    StarPower: "星力",
    Exorcism: "辟邪",
    Sharpness: "锋锐",
    ShatterDefense: "碎防",
    IgnoreDefenseAttacks: "无视防御",
    CannotAct: "无法行动",
    ExtraActionAgain: "再次行动",
    ActivatedMetal: "激活金灵",
    ActivatedWater: "激活水灵",
    ActivatedWood: "激活木灵",
    ActivatedFire: "激活火灵",
    ActivatedEarth: "激活土灵",
    HpGained: "已加生命",
    HpLost: "已失生命",
    JiHuoJinLing: "激活金灵",
    JiHuoShuiLing: "激活水灵",
    JiHuoMuLing: "激活木灵",
    JiHuoHuoLing: "激活火灵",
    JiHuoTuLing: "激活土灵",
    LostDefenseCount: "失防次数",
    BattlePhysiqueGainCount: "本场体魄增加",
    physique: "体魄",
    CloudSea: "云海",
    CloudChain: "连云",
    StarErosion: "星蚀",
    Tide: "海潮",
    WaterMomentum: "水势",
    // 全量暴露缺口（档 1a/1b）词条：键名即 archive 枚举名，与 detail_entries 同口径。
    // 注：BengQuanCunJin/BengQuanFanXuan/MengLianBeng 三键由 generated
    // battle-event-labels.json 提供词条（与 detail label 一致），不在此重复登记。
    YiHuaJieMu: "移花接木",
    YeRenHua: "叶刃花",
    GuYeLang: "孤夜狼",
    LingGuaShu: "灵卦术",
    XingYueYuShan: "星月折扇",
    ShuiYueJianZhen: "水月剑阵",
    MuLingZhen: "木灵阵",
    XiaoYaoGuQin: "逍遥古琴",
    HuaLongDianJing: "画龙点睛",
    HuiFu: "恢复",
    JianQi: "剑气",
  };
  return names[key] ?? (/^[A-Za-z][A-Za-z0-9_]*$/.test(key) ? "状态" : key);
}

export function numericValue(target: HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement): number {
  const value = Number(target.value);
  if (Number.isNaN(value)) throw new Error(`${target.id} 不是数字`);
  return value;
}

export function parseNumberList(value: string): number[] {
  const trimmed = value.trim();
  if (!trimmed) return [];
  return trimmed
    .split(/[,\s]+/)
    .filter(Boolean)
    .map((item) => {
      const value = Number(item);
      if (Number.isNaN(value)) throw new Error(`列表包含非数字：${item}`);
      return value;
    });
}

export function parseJsonRecord(value: string): Record<string, never> | Record<string, number | number[]> {
  const trimmed = value.trim();
  if (!trimmed) return {};
  const parsed = JSON.parse(trimmed) as unknown;
  if (typeof parsed !== "object" || parsed === null || Array.isArray(parsed)) {
    throw new Error("JSON 字段必须是对象");
  }
  return parsed as Record<string, number | number[]>;
}

export function visibleErrorMessage(error: unknown): string {
  const raw = error instanceof Error ? error.message : String(error);
  return raw
    .replace(/缺少原版决策：card:[^\s，。]*/g, "缺少原版决策：当前牌需要补充随机判定")
    .replace(/card:\d+(?::[A-Za-z0-9_]+)?/g, "当前牌")
    .replace(/\bCard_[A-Za-z0-9_]+\b/g, "卡牌逻辑")
    .replace(/\brecord-only\b/g, "仅归档")
    .replace(/\bundefined\b/g, "空值")
    .replace(/\bnull\b/g, "空值");
}

export function formatNumberList(values: readonly number[]): string {
  return values.join(", ");
}

export function escapeHtml(value: string): string {
  return value
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;");
}

export function escapeAttribute(value: string): string {
  return escapeHtml(value).replaceAll("'", "&#39;");
}

export function domId(value: string | number): string {
  return String(value).replaceAll(/[^a-zA-Z0-9_-]/g, "-");
}
