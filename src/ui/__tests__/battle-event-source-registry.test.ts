import { describe, expect, test } from "bun:test";
import {
  permanentSourceLabel,
  resourceSourceLabel,
  sourceRootLabel,
  sourceTokenLabel,
} from "../battle-event-source-registry";

describe("battle event source registry", () => {
  test("keeps root, permanent, resource, static, and generated labels centralized", () => {
    expect(sourceRootLabel("turnStart")).toBe("回合开始");
    expect(permanentSourceLabel("10008")).toBe("生命上限");
    expect(resourceSourceLabel("defense")).toBe("防");
    expect(sourceTokenLabel("recovery")).toBe("恢复");
    expect(sourceTokenLabel("firePhoenixRevive")).toBe("浴火凤凰");
    expect(sourceTokenLabel("counterElement")).toBe("混元逆克阵");
    expect(sourceTokenLabel("devouringAncientVine")).toBe("噬仙古藤");
    expect(sourceTokenLabel("shiXuLingShou")).toBe("噬灵虚兽");
    expect(sourceTokenLabel("youMingXuHunQuan")).toBe("幽冥虚魂犬");
  });

  test("does not guess labels for unknown registry keys", () => {
    expect(sourceRootLabel("future")).toBeNull();
    expect(sourceTokenLabel("SyntheticUnknownSuffix")).toBeNull();
  });
});
