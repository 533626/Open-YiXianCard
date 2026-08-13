import { describe, expect, test } from "bun:test";
import { CARD_OPTION_BY_BASE_ID, TALENT_OPTION_BY_ID, cardDetailText, talentDetailText } from "../data";

const FIVE_THUNDER_STRIKES_ID = 4_000_046;
const GU_XU_JIN_SHU_ID = 215;
const DUAN_TI_TALENT_ID = 1;

describe("鼠标悬停中文详情", () => {
  test("卡牌详情不与当前等级联动，同模板合并为 v1/v2/v3 减少重复文字", () => {
    const card = CARD_OPTION_BY_BASE_ID.get(FIVE_THUNDER_STRIKES_ID)!;
    expect(card.variants.length).toBe(3);
    const detail = cardDetailText(card);
    expect(detail).toBe("五雷轰顶\n重复5次：\n30%概率8/10/12攻");
    expect(detail).not.toContain("{");
    expect(detail).not.toContain("}");
    expect(detail).not.toContain("[");
    expect(detail).not.toContain("]");
    expect(detail).not.toContain("<");
  });

  test("相同数值跨等级不重复展示，只有变化的数值才合并为 v1/v2/v3", () => {
    const card = CARD_OPTION_BY_BASE_ID.get(4_000_006)!; // 野马分鬃：attackCount 三阶一致，attack 各阶不同
    const detail = cardDetailText(card);
    expect(detail).toContain("3/4/5攻×2");
    expect(detail).not.toContain("×2/2/2");
  });

  test("各阶描述文本本身不同的卡牌，仍按阶分段展示而非强行合并", () => {
    const card = CARD_OPTION_BY_BASE_ID.get(GU_XU_JIN_SHU_ID)!;
    const detail = cardDetailText(card);
    expect(detail).toContain("一阶：");
    expect(detail).toContain("三阶：");
  });

  test("卡牌详情没有注册等级数据时回退为卡名", () => {
    const empty = { ...CARD_OPTION_BY_BASE_ID.get(FIVE_THUNDER_STRIKES_ID)!, variants: [] };
    expect(cardDetailText(empty)).toBe(empty.name);
  });

  test("回放原始牌只给出单份配置时按该配置展示详情", () => {
    const card = CARD_OPTION_BY_BASE_ID.get(FIVE_THUNDER_STRIKES_ID)!;
    const fallbackOnly = { ...card, variants: [] };
    const detail = cardDetailText(fallbackOnly, card.variants[0]!.config);
    expect(detail).toBe("五雷轰顶\n重复5次：\n30%概率8攻");
  });

  test("详情去除原文富文本标签，如遇缺失字段则去除占位符而非展示英文", () => {
    const wildHorse = CARD_OPTION_BY_BASE_ID.get(4_000_006)!;
    const detail = cardDetailText(wildHorse);
    expect(detail).not.toMatch(/[A-Za-z]{2,}/);
    expect(detail).not.toContain("<");
    expect(detail).not.toContain("{");
  });

  test("仙命详情按具体档位 otherParams 替换", () => {
    expect(TALENT_OPTION_BY_ID.get(DUAN_TI_TALENT_ID)?.name).toBe("锻体");
    expect(talentDetailText(DUAN_TI_TALENT_ID)).toBe("锻体\n生命上限+5");
  });

  test("未知仙命 id 返回空字符串", () => {
    expect(talentDetailText(-1)).toBe("");
  });
});
