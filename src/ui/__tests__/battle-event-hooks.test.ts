import { describe, expect, test } from "bun:test";
import { isSourceMapped, sourceLabel } from "../battle-event-hooks";

describe("battle event source labels", () => {
  test("rejects an unknown buff identity instead of treating the root as mapped", () => {
    expect(sourceLabel("buff:SyntheticUnknownBuff")).toBe("状态 状态触发");
    expect(isSourceMapped("buff:SyntheticUnknownBuff")).toBe(false);
  });

  test("rejects unknown suffixes instead of silently dropping them", () => {
    expect(sourceLabel("turnStart:SyntheticUnknownSuffix")).toBe("回合开始");
    expect(isSourceMapped("turnStart:SyntheticUnknownSuffix")).toBe(false);
    expect(isSourceMapped("turnStart:recovery")).toBe(true);
  });
});
