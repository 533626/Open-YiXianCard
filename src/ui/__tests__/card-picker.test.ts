import { describe, expect, test } from "bun:test";
import {
  CARD_INDEX_OPTIONS,
  CARD_OPTION_BY_BASE_ID,
  DEFAULT_CAREER_ID,
  EMPTY_CHARACTER_ID,
  canPickCardForDeckSlot,
  cardsGroupedForPicker,
  defaultBattleConfig,
  defaultPlayerConfig,
  derivedCardOption,
  isCardUnlockedByTalents,
  scopedCardIndexOptions,
  scopedCardOptions,
  scopedTalentOptions,
} from "../data";
import { sanitizePlayerScope } from "../main-utils";
import type { DeckSlotConfig } from "../types";

const QI_XING_CHARACTER_ID = 2_000_004;
const FIVE_THUNDER_STRIKES_ID = 4_000_046;
const SMALL_RESTORE_PILL_ID = 2_000_001;
const CULTIVATE_PILL_ID = 2_000_002;
const EARTH_SPIRIT_PILL_ID = 2_000_003;
const EXTREME_BENG_TIAN_STEP_ID = 10_000_098;
const BENG_QUAN_FAN_XUAN_ID = 140;
const FIVE_COLOR_MENDING_STONE_ID = 175;

describe("UI 选卡契约", () => {
  test("默认分组按高频构筑路径排序，低频奇遇牌沉底", () => {
    const ids = cardsGroupedForPicker(CARD_INDEX_OPTIONS).map((group) => group.id);
    expect(ids.slice(0, 4)).toEqual([
      "exclusive",
      "HuaShen",
      "YuanYing",
      "season-fate-strategy",
    ]);
    expect(ids.indexOf("chance-artifact")).toBeGreaterThan(ids.indexOf("LianQi"));
    expect(ids.indexOf("chance-pet")).toBeGreaterThan(ids.indexOf("LianQi"));
    expect(ids.indexOf("secret")).toBeGreaterThan(ids.indexOf("LianQi"));
  });

  test("副职行按境界从高到低排列（化神在前，使用频率优先）", () => {
    const careerCards = cardsGroupedForPicker(CARD_INDEX_OPTIONS)
      .find((group) => group.id === "career")?.cards ?? [];
    const realmOrder: Readonly<Record<string, number>> = {
      HuaShen: 5,
      YuanYing: 4,
      JinDan: 3,
      ZhuJi: 2,
      LianQi: 1,
    };
    const ranks = careerCards
      .filter((card) => card.implemented)
      .map((card) => realmOrder[card.realm ?? ""] ?? 0);

    expect(ranks.length).toBeGreaterThan(5);
    expect(ranks).toEqual([...ranks].sort((left, right) => right - left));
  });

  test("万玄破魔掌只在选中对应仙命时进入牌池", () => {
    expect(isCardUnlockedByTalents(82, [])).toBe(false);
    expect(isCardUnlockedByTalents(82, [187])).toBe(true);
    expect(scopedCardIndexOptions(4_000_004, "LianDanShi", []).some((card) => card.baseId === 82)).toBe(false);
    expect(scopedCardIndexOptions(4_000_004, "LianDanShi", [187]).some((card) => card.baseId === 82)).toBe(true);

    const player = defaultPlayerConfig("p1", 4_000_004);
    player.deck[0] = { baseId: 82, level: 0 };
    player.talents[4] = scopedTalentOptions(4_000_004, "HuaShen")
      .find((option) => option.id !== 187)!.id;
    sanitizePlayerScope(player);
    expect(player.deck[0]?.baseId).toBe(0);
  });

  test("副职兼修开放兼修副职牌池，主副职不变时兼修牌不可入", () => {
    // LianDanShi 主副职时，QinShi 职业牌不在范围
    expect(scopedCardIndexOptions(1_000_001, "LianDanShi", []).some((card) =>
      card.archiveKey === "career:QinShi")).toBe(false);
    // 指定 QinShi 为兼修副职后，QinShi 职业牌进入范围
    expect(scopedCardIndexOptions(1_000_001, "LianDanShi", [], { 1: "QinShi" }).some((card) =>
      card.archiveKey === "career:QinShi")).toBe(true);
  });

  test("副职牌按已选副职各自成组，主副职在前并标出兼修来源", () => {
    const cards = scopedCardIndexOptions(1_000_001, "LianDanShi", [], { 2: "QinShi" });
    const groups = cardsGroupedForPicker(cards, {
      primary: "LianDanShi",
      duals: { 2: "QinShi" },
    });
    const careerGroups = groups.filter((group) => group.id.startsWith("career:"));

    expect(careerGroups.map((group) => group.id))
      .toEqual(["career:LianDanShi", "career:QinShi"]);
    expect(careerGroups.map((group) => group.label)).toEqual(["炼丹师", "琴师"]);
    expect(careerGroups.map((group) => group.badge)).toEqual(["主", "金兼"]);
    expect(careerGroups[1]?.badgeTitle).toBe("金丹兼修");
    // 每组只含本副职的牌，不再混在同一个「副职」组里
    for (const group of careerGroups) {
      expect(group.cards.every((card) => card.archiveKey === group.id)).toBe(true);
      expect(group.cards.length).toBeGreaterThan(0);
    }
    expect(groups.some((group) => group.id === "career")).toBe(false);
  });

  test("不传副职上下文时仍是单一「副职」组（存档/契约旧口径）", () => {
    const groups = cardsGroupedForPicker(CARD_INDEX_OPTIONS);
    expect(groups.some((group) => group.id === "career")).toBe(true);
    expect(groups.some((group) => group.id.startsWith("career:"))).toBe(false);
  });

  test("未选角色时副职牌池只含默认副职（炼丹师），其他副职不成组", () => {
    const player = defaultPlayerConfig("p1", EMPTY_CHARACTER_ID);
    expect(player.careerName).toBe(DEFAULT_CAREER_ID);
    expect(DEFAULT_CAREER_ID).toBe("LianDanShi");

    const cards = scopedCardIndexOptions(
      player.characterId,
      player.careerName,
      player.talents,
      player.dualCareerNames,
    );
    // 未选角色时不得退化成全卡池：其他副职的牌不进池
    expect(cards.some((card) => card.archiveKey === "career:QinShi")).toBe(false);
    expect(cards.some((card) => card.archiveKey === "career:FuZhouShi")).toBe(false);
    expect(cards.some((card) => card.archiveKey === "career:LianDanShi")).toBe(true);

    const groups = cardsGroupedForPicker(cards, {
      primary: player.careerName,
      duals: player.dualCareerNames,
    });
    const careerGroups = groups.filter((group) => group.id.startsWith("career:"));
    expect(careerGroups.map((group) => group.id)).toEqual(["career:LianDanShi"]);
    expect(careerGroups.map((group) => group.badge)).toEqual(["主"]);
  });

  test("未选角色且副职为空时，卡池无任何副职牌，也不产生副职组", () => {
    const cards = scopedCardIndexOptions(EMPTY_CHARACTER_ID, null);
    expect(cards.some((card) => card.archiveKind === "career")).toBe(false);

    const groups = cardsGroupedForPicker(cards, { primary: null, duals: {} });
    expect(groups.some((group) => group.id.startsWith("career:"))).toBe(false);
    expect(groups.some((group) => group.id === "career")).toBe(false);
  });

  test("分组只反映已选副职：卡池里未选副职的牌不生成组", () => {
    // 即使有人往卡池里塞入未选副职的牌（绕过 scopedCard*Options），
    // 分组也不得为它开组，避免「未选副职却显示其他副职」。
    const foreignCareerCards = CARD_INDEX_OPTIONS.filter(
      (card) => card.archiveKey === "career:QinShi",
    );
    expect(foreignCareerCards.length).toBeGreaterThan(0);
    const groups = cardsGroupedForPicker(foreignCareerCards, {
      primary: "LianDanShi",
      duals: {},
    });
    expect(groups.some((group) => group.id.startsWith("career:"))).toBe(false);
  });

  test("陆剑心化神分支自动改变澄心剑胚名称", () => {
    const card = CARD_OPTION_BY_BASE_ID.get(19)!;
    expect(derivedCardOption(card, [10_096]).name).toBe("狂剑•澄心");
    expect(derivedCardOption(card, [20_096]).name).toBe("云剑•澄心");
    expect(derivedCardOption(card, [30_096]).name).toBe("澄心•无极");
    expect(derivedCardOption(card, [96]).name).toBe("澄心剑胚");
  });

  test("极·崩天步归入天衍仙命，不混入门派化神卡池", () => {
    const extremeBengTianStep = CARD_INDEX_OPTIONS.find(
      (card) => card.baseId === EXTREME_BENG_TIAN_STEP_ID,
    );
    expect(extremeBengTianStep?.name).toBe("极•崩天步");
    expect(extremeBengTianStep?.archiveKind).toBe("season");
    expect(extremeBengTianStep?.archiveKey).toBe(
      "season:base:fate-strategy:extreme:sect:DuanXuanZong",
    );

    const groups = cardsGroupedForPicker([extremeBengTianStep!]);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.id).toBe("season-fate-strategy");
    expect(groups[0]?.label).toBe("天衍仙命");
  });

  test("Rust 新转正的天衍极卡与无归属融会牌可从对应卡池选取", () => {
    const yunLing = new Set(scopedCardOptions(1_000_001, null).map((card) => card.baseId));
    const qiXing = new Set(scopedCardOptions(2_000_004, null).map((card) => card.baseId));
    const wuXing = new Set(scopedCardOptions(3_000_001, null).map((card) => card.baseId));
    const duanXuan = new Set(scopedCardOptions(4_000_004, null).map((card) => card.baseId));

    expect([...yunLing].filter((id) => [1_000_099, 1_000_100].includes(id))).toEqual([
      1_000_099,
      1_000_100,
    ]);
    expect([...qiXing].filter((id) => [4_000_100, 4_000_101].includes(id))).toEqual([
      4_000_100,
      4_000_101,
    ]);
    expect([...wuXing].filter((id) => [7_000_107, 7_000_108].includes(id))).toEqual([
      7_000_107,
      7_000_108,
    ]);
    expect([...duanXuan].filter((id) => [10_000_100, 10_000_101].includes(id))).toEqual([
      10_000_100,
      10_000_101,
    ]);

    const ronghui = [401, 403, 407, 413, 415, 417, 422, 423, 429];
    expect(ronghui.every((id) => yunLing.has(id))).toBe(true);
    expect(ronghui.every((id) => qiXing.has(id))).toBe(true);
  });

  test("原版融会结果牌统一归入遗迹法器并保留专属来源键", () => {
    const bengQuanFanXuan = CARD_INDEX_OPTIONS.find(
      (card) => card.baseId === BENG_QUAN_FAN_XUAN_ID,
    );
    const fiveColorMendingStone = CARD_INDEX_OPTIONS.find(
      (card) => card.baseId === FIVE_COLOR_MENDING_STONE_ID,
    );

    expect(bengQuanFanXuan?.archiveKey).toBe("season:past:relic:exclusive:4000004");
    expect(fiveColorMendingStone?.archiveKey).toBe(
      "season:past:relic:career:LianDanShi",
    );

    const groups = cardsGroupedForPicker([bengQuanFanXuan!, fiveColorMendingStone!]);
    expect(groups).toHaveLength(1);
    expect(groups[0]?.id).toBe("season-relic");
    expect(groups[0]?.label).toBe("遗迹法器");
    expect(groups[0]?.cards.map((card) => card.baseId).sort((a, b) => a - b)).toEqual([
      BENG_QUAN_FAN_XUAN_ID,
      FIVE_COLOR_MENDING_STONE_ID,
    ]);
  });

  test("普通牌不受消耗/持续两张总上限限制，可放入第三张五雷轰顶", () => {
    const fiveThunder = CARD_OPTION_BY_BASE_ID.get(FIVE_THUNDER_STRIKES_ID);
    expect(fiveThunder?.type).toBe("normal");
    const deck: DeckSlotConfig[] = [
      { baseId: FIVE_THUNDER_STRIKES_ID, level: 0 },
      { baseId: FIVE_THUNDER_STRIKES_ID, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
    ];

    expect(canPickCardForDeckSlot(fiveThunder!, deck, 2)).toBe(true);
  });

  test("消耗和持续牌按整套卡组合计最多两张，替换当前槽时先扣除当前槽", () => {
    const thirdLimited = CARD_OPTION_BY_BASE_ID.get(EARTH_SPIRIT_PILL_ID);
    const deck: DeckSlotConfig[] = [
      { baseId: SMALL_RESTORE_PILL_ID, level: 0 },
      { baseId: CULTIVATE_PILL_ID, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
      { baseId: 0, level: 0 },
    ];

    expect(canPickCardForDeckSlot(thirdLimited!, deck, 2)).toBe(false);
    expect(canPickCardForDeckSlot(thirdLimited!, deck, 1)).toBe(true);
  });

});
