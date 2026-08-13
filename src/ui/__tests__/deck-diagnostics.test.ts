import { describe, expect, test } from "bun:test";
import { defaultBattleConfig } from "../data";
import { diagnoseBattleConfig } from "../deck-diagnostics";

describe("卡组诊断能力查询", () => {
  test("空构筑明确归类为不足8张", () => {
    const result = diagnoseBattleConfig(defaultBattleConfig());
    expect(result.simulatable).toBe(false);
    expect(result.sides.p1.issues.map((issue) => issue.kind)).toContain("不足8张");
    expect(result.sides.p2.issues.map((issue) => issue.kind)).toContain("不足8张");
  });

  test("未知牌明确归类为未注册战斗牌", () => {
    const config = defaultBattleConfig();
    config.players.p1.deck = Array.from({ length: 8 }, () => ({ baseId: 9_999_999, level: 0 }));
    const result = diagnoseBattleConfig(config);
    expect(result.sides.p1.issues.filter((issue) => issue.kind === "未注册战斗牌")).toHaveLength(8);
    expect(result.sides.p1.simulatable).toBe(false);
  });

  test("七星定魂在开战时补足第八格，不误报仍需补牌", () => {
    const config = sevenCardConfig();
    config.players.p1.talents = [52];

    const result = diagnoseBattleConfig(config).sides.p1;

    expect(result.configuredCount).toBe(7);
    expect(result.effectiveCount).toBe(8);
    expect(result.issues.map((issue) => issue.kind)).not.toContain("不足8张");
  });

  test("孤虚金书在开战时填入相邻空格，不误报仍需补牌", () => {
    const config = sevenCardConfig();
    config.players.p1.talents = [198];
    config.players.p1.deck[0] = { baseId: 215, level: 0 };

    const result = diagnoseBattleConfig(config).sides.p1;

    expect(result.configuredCount).toBe(7);
    expect(result.effectiveCount).toBe(8);
    expect(result.issues.map((issue) => issue.kind)).not.toContain("不足8张");
  });

  test("风绪的 FateStrategy 338 让孤虚金书补足第八格", () => {
    const config = sevenCardConfig();
    config.players.p1.characterId = 2_000_006;
    config.players.p1.fateStrategies = [338];
    config.players.p1.deck[0] = { baseId: 215, level: 0 };

    const result = diagnoseBattleConfig(config).sides.p1;

    expect(result.configuredCount).toBe(7);
    expect(result.effectiveCount).toBe(8);
    expect(result.issues.map((issue) => issue.kind)).not.toContain("不足8张");
  });

  test("没有开战补位效果的七张构筑仍然阻塞", () => {
    const result = diagnoseBattleConfig(sevenCardConfig()).sides.p1;

    expect(result.effectiveCount).toBe(7);
    expect(result.issues.map((issue) => issue.kind)).toContain("不足8张");
    expect(result.simulatable).toBe(false);
  });
});

function sevenCardConfig() {
  const config = defaultBattleConfig();
  config.players.p1.deck = Array.from(
    { length: 8 },
    (_, index) => ({ baseId: index < 7 ? 10_000_006 : 0, level: 0 }),
  );
  return config;
}
