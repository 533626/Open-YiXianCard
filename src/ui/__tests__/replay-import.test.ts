import { describe, expect, test } from "bun:test";
import { workbenchEvidenceMode } from "../evidence-mode";
import { renderFixtureImportPanel } from "../render-fixture-import";
import {
  LOCAL_REPLAY_SCHEMA,
  LOCAL_REPLAY_SCHEMA_VERSION,
  MAX_LOCAL_REPLAY_BYTES,
  applyImportedReplay,
  importLocalReplayFileIntoState,
  localReplayImportErrorMessage,
  parseVersionedLocalReplayJson,
} from "../replay-import";
import { loadRepositoryReplayFixture } from "../repository-replay-loader";
import { readReleaseMetadata, RELEASE_META_NAMES } from "../release-metadata";
import type { UiFixtureEntry } from "../fixture-catalog";
import { baseState, publicReplayFixture } from "./layout-test-helpers";

const fixtureJson = publicReplayFixture();
const fixtureText = JSON.stringify(fixtureJson);

describe("本地版本化回放导入", () => {
  test("直接接受 records:fixtures 产出的 schemaVersion=1 JSON，且不持有原文件名", async () => {
    const state = baseState();
    const privateName = "private-original-match.json";

    await importLocalReplayFileIntoState(state, {
      name: privateName,
      text: async () => fixtureText,
    });

    expect(state.importedFixtureOrigin).toBe("local");
    expect(state.importedFixtureId).toBeNull();
    expect(state.config.sourceKind).toBe("original-fixture");
    expect(JSON.stringify(state)).not.toContain(privateName);
    expect(workbenchEvidenceMode(state).label).toBe("研究沙盒 · 本地回放未认证");
  });

  test("可选 envelope 同样按 version=1 解析", () => {
    const fixture = parseVersionedLocalReplayJson(JSON.stringify({
      schema: LOCAL_REPLAY_SCHEMA,
      version: LOCAL_REPLAY_SCHEMA_VERSION,
      fixture: fixtureJson,
    }));
    expect(fixture.firstPlayerSide).toBe(fixtureJson.firstPlayerSide as "p1" | "p2");
  });

  test("无版本、损坏 JSON 与 raw bin 均原子失败，错误不泄露文件名", async () => {
    const cases = [
      {
        name: "secret-unversioned.json",
        text: JSON.stringify(Object.fromEntries(
          Object.entries(fixtureJson).filter(([key]) => key !== "schemaVersion"),
        )),
      },
      { name: "secret-corrupt.json", text: "{broken" },
      { name: "secret-original.bin", text: fixtureText },
    ];

    for (const item of cases) {
      const state = baseState();
      const originalConfig = state.config;
      let message = "";
      try {
        await importLocalReplayFileIntoState(state, {
          name: item.name,
          text: async () => item.text,
        });
      } catch (error) {
        message = localReplayImportErrorMessage(error);
      }
      expect(message.length).toBeGreaterThan(0);
      expect(message).not.toContain(item.name);
      expect(state.config).toBe(originalConfig);
      expect(state.importedFixture ?? null).toBeNull();
      expect(JSON.stringify(state)).not.toContain(item.name);
    }
  });

  test("超大文件在读取前拒绝且不改变构筑", async () => {
    const state = baseState();
    const originalConfig = state.config;
    let readCount = 0;
    let message = "";
    try {
      await importLocalReplayFileIntoState(state, {
        name: "secret-oversized.json",
        size: MAX_LOCAL_REPLAY_BYTES + 1,
        text: async () => {
          readCount += 1;
          return fixtureText;
        },
      });
    } catch (error) {
      message = localReplayImportErrorMessage(error);
    }
    expect(readCount).toBe(0);
    expect(message).toContain("超过 5 MiB");
    expect(message).not.toContain("secret-oversized.json");
    expect(state.config).toBe(originalConfig);
  });
});

describe("玩家导入与开发目录隔离", () => {
  test("生产入口呈现战绩码、本机 .bin 与对局包，不暴露 fixture 操作", () => {
    const state = baseState();
    state.fixtureImportOpen = true;
    const html = renderFixtureImportPanel(state, []);

    expect(html).toContain(">战绩码</button>");
    expect(html).toContain(">本机记录</button>");
    expect(html).toContain(">对局包</button>");
    expect(html).toContain('data-original-replay-directory="1"');
    expect(html).toContain('data-original-replay-files="1"');
    expect(html).toContain("%USERPROFILE%\\AppData\\LocalLow\\DarkSunStudio\\YiXianPai");
    expect(html).toContain("$HOME/.config/unity3d/DarkSunStudio/YiXianPai");
    expect(html).toContain('data-copy-replay-path="%USERPROFILE%\\AppData\\LocalLow\\DarkSunStudio\\YiXianPai"');
    expect(html).toContain('data-copy-replay-path="$HOME/.config/unity3d/DarkSunStudio/YiXianPai"');
    expect(html).toContain("复制给 AI 助手");
    expect(html).not.toContain("原始 .bin 暂不支持");
    expect(html).not.toContain('data-action="import-fixture"');
    expect(html).not.toContain('data-action="quick-fixture"');
    expect(html).not.toContain("e63lwvs");
  });

  test("非 catalog ID 与不安全 catalog path 都在 fetch 前失败", async () => {
    let fetchCount = 0;
    const fetcher = async () => {
      fetchCount += 1;
      return { ok: true, text: async () => fixtureText };
    };
    const unsafeEntry: UiFixtureEntry = {
      id: "unsafe",
      path: "https://example.invalid/private.json",
      matchId: "unsafe",
      round: 1,
      steamBuild: null,
      expectedWinner: null,
      p1CharacterId: null,
      p2CharacterId: null,
    };

    for (const [id, entries] of [
      ["https://example.invalid/private.json", []],
      ["unsafe", [unsafeEntry]],
    ] as const) {
      try {
        await loadRepositoryReplayFixture(id, fetcher, entries);
      } catch {
        // Expected: both inputs are rejected before the fetch boundary.
      }
    }
    expect(fetchCount).toBe(0);
  });

  test("只有未修改的受信 catalog 配置显示原作证据模式", () => {
    const state = baseState();
    const fixture = parseVersionedLocalReplayJson(fixtureText);
    applyImportedReplay(state, fixture, { origin: "catalog", id: "admitted/round-01" });
    expect(workbenchEvidenceMode(state).label).toBe("原作证据模式");
    state.config.gameRound += 1;
    expect(workbenchEvidenceMode(state).label).toBe("研究沙盒 · 已修改回放");
  });
});

describe("发布快照元数据", () => {
  test("缺失元数据时不猜 build", () => {
    const metadata = readReleaseMetadata(null);
    expect(metadata.bound).toBe(false);
    expect(metadata.label).toBe("本地开发 · 未绑定发布快照");
  });

  test("三项 meta 完整时显示绑定快照", () => {
    const values: Record<string, string> = {
      [RELEASE_META_NAMES.steamBuild]: "test-build",
      [RELEASE_META_NAMES.ruleset]: "rules-v1",
      [RELEASE_META_NAMES.appCommit]: "1234567890abcdef",
    };
    const fakeDocument = {
      querySelector: (selector: string) => {
        const name = selector.match(/name="([^"]+)"/)?.[1];
        return name && values[name] ? { content: values[name] } : null;
      },
    } as unknown as Document;
    const metadata = readReleaseMetadata(fakeDocument);
    expect(metadata.bound).toBe(true);
    expect(metadata.label).toBe("Build test-build · Rules rules-v1 · 1234567890");
  });
});
