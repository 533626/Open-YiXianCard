import { describe, expect, test } from "bun:test";
import { findUnanchoredOpenItems } from "../lib/doc-open-item";

describe("open-status marker anchors", () => {
  test("flags an inline marker without any anchor on the same line", () => {
    const violations = findUnanchoredOpenItems(
      "## 章节\n\n该问题留作待办。\n",
    );
    expect(violations).toEqual([
      { line: 3, kind: "inline", marker: "留作待办" },
    ]);
  });

  test("accepts a backtick anchor on the same line", () => {
    expect(
      findUnanchoredOpenItems("该问题留作待办（`scripts/foo.ts`）。\n"),
    ).toEqual([]);
  });

  test("accepts a markdown link anchor", () => {
    expect(
      findUnanchoredOpenItems("留作待办，见 [REPLAY_ADMISSION](docs/REPLAY_ADMISSION.md)。\n"),
    ).toEqual([]);
  });

  test("flags all marker synonyms", () => {
    const lines = [
      "TODO 无锚点",
      "FIXME 无锚点",
      "尚未定位：无锚点",
      "尚未完成：无锚点",
      "尚未接入：无锚点",
      "待接入：无锚点",
      "未修：无锚点",
    ].join("\n") + "\n";
    const violations = findUnanchoredOpenItems(lines);
    expect(violations.map((v) => v.marker).sort()).toEqual(
      ["TODO", "FIXME", "尚未定位", "尚未完成", "尚未接入", "待接入", "未修"].sort(),
    );
  });

  test("does not flag bare 待办 or 下一步 in prose", () => {
    expect(
      findUnanchoredOpenItems("轮询线程只负责发现 arm 与待办批次；status 的 next-action 回答下一步做什么。\n"),
    ).toEqual([]);
  });

  test("ignores markers inside fenced code blocks", () => {
    const content = [
      "```bash",
      "# TODO 这是代码注释，不是文档开放项",
      "bun run check:docs-drift",
      "```",
      "正文 留作待办，无锚点。",
    ].join("\n") + "\n";
    expect(findUnanchoredOpenItems(content)).toEqual([
      { line: 5, kind: "inline", marker: "留作待办" },
    ]);
  });

  test("下一步 section needs at least one anchor in its block", () => {
    const bad = "## 下一步\n\n1. 补覆盖度：多搜几个 profile。\n2. 继续社区知识提取。\n";
    expect(findUnanchoredOpenItems(bad)).toEqual([
      { line: 1, kind: "section", marker: "下一步" },
    ]);

    const good =
      "## 下一步\n\n1. 补覆盖度：按 `analysis/ga/data/deck-archive.json` 统计。\n2. 社区知识提取：见 `docs/GITHUB_YIXIAN_HIGH_VALUE_PROJECTS.md`。\n";
    expect(findUnanchoredOpenItems(good)).toEqual([]);
  });

  test("下一步 section ends at the next same-or-higher heading", () => {
    const content =
      "## 下一步\n\n- 无锚点的条目\n\n## 下一节\n\n`有锚点` 的内容不救上一节\n";
    expect(findUnanchoredOpenItems(content)).toEqual([
      { line: 1, kind: "section", marker: "下一步" },
    ]);
  });
});
