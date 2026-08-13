import { afterEach, describe, expect, test } from "bun:test";
import { spawnSync } from "node:child_process";
import {
  mkdir,
  mkdtemp,
  readFile,
  rm,
  unlink,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import { join, resolve } from "node:path";
import {
  assertSafeOutputDirectory,
  assertTrackedWorkingTreeClean,
  buildSite,
} from "../build-site";
import { buildUi } from "../build-ui";
import {
  checkPublicReleasePolicy,
  PUBLIC_RELEASE_POLICY_FILES,
} from "../check-public-release-policy";
import {
  assertPublicBundleMetafile,
  publicBundleBoundaryPlugin,
} from "../public-bundle-boundary";
import {
  auditReleaseArtifact,
  contentHashedAssetName,
  RELEASE_CLOUDFLARE_HEADERS,
  sha256Bytes,
  writeReleaseManifest,
  type ReleaseMetadata,
} from "../release-artifact";

const REPO_ROOT = resolve(import.meta.dir, "../..");
const TEST_METADATA: ReleaseMetadata = {
  supportedSteamBuild: "24124964",
  sharedSnapshotSha256: "a".repeat(64),
  rulesetRevision: `engine-rust-tree:${"b".repeat(40)}`,
  appCommit: "c".repeat(40),
};

const temporaryRoots: string[] = [];

afterEach(async () => {
  await Promise.all(temporaryRoots.splice(0).map((path) => rm(path, { recursive: true, force: true })));
});

describe("release artifact allowlist", () => {
  test("accepts the exact clean static artifact shape", async () => {
    const root = await createCleanArtifact();
    const audit = await auditReleaseArtifact(root);

    expect(audit.manifest.fixturePolicy).toEqual({
      bundledFixtureCount: 0,
      catalog: "empty",
      remoteFetch: false,
    });
    expect(audit.files).toHaveLength(11);
    expect(audit.manifest.inventory.files).toHaveLength(10);
    expect(RELEASE_CLOUDFLARE_HEADERS).toContain("worker-src 'self'");
    expect(RELEASE_CLOUDFLARE_HEADERS).toContain("connect-src 'self'");

    const mainPath = artifactAssetPath(audit.manifest, /^assets\/main\..+\.js$/);
    const workerPath = artifactAssetPath(
      audit.manifest,
      /^assets\/workbench-worker\..+\.js$/,
    );
    const main = await readFile(join(root, ...mainPath.split("/")), "utf8");
    const html = await readFile(join(root, "index.html"), "utf8");
    expect(main).toContain(`/${workerPath}`);
    expect(html).not.toContain(`/${workerPath}`);
  });

  test("rejects a missing hashed workbench worker", async () => {
    const root = await createCleanArtifact();
    const manifest = await readReleaseManifest(root);
    const workerPath = artifactAssetPath(
      manifest,
      /^assets\/workbench-worker\..+\.js$/,
    );
    await unlink(join(root, ...workerPath.split("/")));
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "missing hashed asset: workbench-worker.js",
    );
  });

  test("rejects an extra hashed workbench worker", async () => {
    const root = await createCleanArtifact();
    const extra = new TextEncoder().encode('self.postMessage("unexpected");\n');
    const extraName = contentHashedAssetName("workbench-worker", "js", extra);
    await writeFile(join(root, "assets", extraName), extra);
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "duplicate hashed asset: workbench-worker.js",
    );
  });

  test("rejects a main bundle that references a worker outside the artifact inventory", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(
      root,
      'const wrongWorker = "/assets/workbench-worker.0000000000000000.js";',
    );

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "main.js workbench worker reference does not match the artifact inventory",
    );
  });

  test("rejects an index that bypasses main and loads the worker directly", async () => {
    const root = await createCleanArtifact();
    const manifest = await readReleaseManifest(root);
    const workerPath = artifactAssetPath(
      manifest,
      /^assets\/workbench-worker\..+\.js$/,
    );
    const indexPath = join(root, "index.html");
    const index = await readFile(indexPath, "utf8");
    await writeFile(
      indexPath,
      index.replace("</body>", `<script src="/${workerPath}"></script></body>`),
    );
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "index.html must load the workbench worker through main.js only",
    );
  });

  test("rejects a repository fixture path even when hashes and manifest are refreshed", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const replay = "battle-evaluator/fixtures/candidates/evil/round-01.json";');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("fixture path found");
  });

  test("rejects an embedded fixture payload", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(
      root,
      'const replay = {"schemaVersion":1,"source":{"steamBuild":"1"},"firstPlayerSide":"p1","players":{"p1":{},"p2":{}}};',
    );

    await expect(auditReleaseArtifact(root)).rejects.toThrow("embedded fixture payload found");
  });

  test("rejects a binary file injection", async () => {
    const root = await createCleanArtifact();
    await writeFile(join(root, "stolen.bin"), new Uint8Array([0, 1, 2, 3]));
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow("path is not allowlisted: stolen.bin");
  });

  test("allows honest UX text about local raw .bin import", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const notice = "原始 .bin 只在浏览器本地解析，不会上传";');

    await expect(auditReleaseArtifact(root)).resolves.toBeDefined();
  });

  test("allows only the reviewed public Windows and Linux replay cache templates", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(
      root,
      String.raw`const path = "%USERPROFILE%\\AppData\\LocalLow\\DarkSunStudio\\YiXianPai";
const script = "$root = Join-Path $env:USERPROFILE 'AppData\\LocalLow\\DarkSunStudio\\YiXianPai'";
const linux = "$HOME/.config/unity3d/DarkSunStudio/YiXianPai";`,
    );

    await expect(auditReleaseArtifact(root)).resolves.toBeDefined();
  });

  test("rejects a different USERPROFILE path", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, String.raw`const leak = "%USERPROFILE%\\Desktop\\secret.bin";`);

    await expect(auditReleaseArtifact(root)).rejects.toThrow("local machine path found");
  });

  test("rejects a different Linux Unity path", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const leak = "$HOME/.config/unity3d/Other/private.bin";');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("local machine path found");
  });

  test("rejects a serialized original replay filename", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const leak = {"recentBattleFile":"x.bin"};');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("original replay source field found");
  });

  test("rejects serialized user identifiers", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const leak = {"uid":"123","username":"player"};');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("user identifier field found");
  });

  test("rejects local machine path injection", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const leak = "/home/user/.config/unity3d/private.bin";');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("local machine path found");
  });

  test("rejects unreviewed media or font assets", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const art = "data:image/png;base64,AAAA";');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("unreviewed media or font asset found");
  });

  for (const [name, source] of [
    ["fetch", 'fetch("/upload")'],
    ["XMLHttpRequest", "new XMLHttpRequest()"],
    ["WebSocket", 'new WebSocket("/socket")'],
    ["EventSource", 'new EventSource("/events")'],
    ["sendBeacon", 'navigator.sendBeacon("/upload", "secret")'],
  ] as const) {
    test(`rejects the ${name} network API`, async () => {
      const root = await createCleanArtifact();
      await replaceMainJavascript(root, source);

      await expect(auditReleaseArtifact(root)).rejects.toThrow("network upload/fetch API found");
    });
  }

  test("rejects a network API embedded in the workbench worker", async () => {
    const root = await createCleanArtifact();
    await replaceWorkerJavascript(root, 'fetch("/upload")');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("network upload/fetch API found");
  });

  test("rejects an aliased fetch identifier", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(root, 'const request = fetch; request("/upload");');

    await expect(auditReleaseArtifact(root)).rejects.toThrow("network upload/fetch API found");
  });

  for (const source of [
    'const request = globalThis["fetch"];',
    "const socket = window['WebSocket'];",
    'const events = self["EventSource"];',
    'const beacon = navigator["sendBeacon"];',
  ]) {
    test(`rejects bracket network access: ${source}`, async () => {
      const root = await createCleanArtifact();
      await replaceMainJavascript(root, source);

      await expect(auditReleaseArtifact(root)).rejects.toThrow("network upload/fetch API found");
    });
  }

  test("allows ordinary copy that names unavailable network APIs", async () => {
    const root = await createCleanArtifact();
    await replaceMainJavascript(
      root,
      'const copy = `fetch and WebSocket are unavailable; globalThis["fetch"] remains blocked`;',
    );

    await expect(auditReleaseArtifact(root)).resolves.toBeDefined();
  });

  test("rejects release provenance metadata that drifts from the manifest", async () => {
    const root = await createCleanArtifact();
    const indexPath = join(root, "index.html");
    const index = await readFile(indexPath, "utf8");
    await writeFile(
      indexPath,
      index.replace(TEST_METADATA.supportedSteamBuild, "99999999"),
    );
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "open-yixiancard:steam-build does not match release manifest",
    );
  });

  test("rejects a weakened production security header policy", async () => {
    const root = await createCleanArtifact();
    await writeFile(join(root, "_headers"), "/*\n  Content-Security-Policy: default-src *\n");
    await writeReleaseManifest(root, TEST_METADATA);

    await expect(auditReleaseArtifact(root)).rejects.toThrow(
      "_headers does not match the reviewed production security and cache policy",
    );
  });
});

describe("public bundle dependency boundary", () => {
  test("rejects a direct fixture dependency during module resolution", async () => {
    const root = await temporaryDirectory();
    const fixtureDirectory = join(root, "battle-evaluator", "fixtures", "candidates");
    await mkdir(fixtureDirectory, { recursive: true });
    await writeFile(join(fixtureDirectory, "private.json"), "{}\n");
    await writeFile(
      join(root, "entry.ts"),
      'import fixture from "./battle-evaluator/fixtures/candidates/private.json"; console.log(fixture);\n',
    );

    await expectBundleBoundaryFailure(Bun.build({
      entrypoints: [join(root, "entry.ts")],
      outdir: join(root, "out"),
      target: "browser",
      plugins: [publicBundleBoundaryPlugin()],
      metafile: true,
    }), "depends on private fixture content");
  });

  test("rejects a private fixture input discovered in a build metafile", () => {
    const metafile = {
      inputs: {
        "battle-evaluator/fixtures/candidates/private.json": {
          bytes: 2,
          imports: [],
        },
      },
      outputs: {},
    } as Bun.BuildMetafile;

    expect(() => assertPublicBundleMetafile(metafile, "worker.js")).toThrow(
      "metafile input depends on private fixture content",
    );
  });

  for (const dependency of [
    "./src/ui/generated/fixture-index.json",
    "./src/ui/repository-replay-loader.ts",
  ]) {
    test(`rejects production dependency on ${dependency}`, async () => {
      const root = await temporaryDirectory();
      const dependencyPath = join(root, ...dependency.slice(2).split("/"));
      await mkdir(resolve(dependencyPath, ".."), { recursive: true });
      await writeFile(
        dependencyPath,
        dependency.endsWith(".json") ? "[]\n" : "export const catalog = [];\n",
      );
      await writeFile(join(root, "entry.ts"), `import "${dependency}";\n`);

      await expectBundleBoundaryFailure(Bun.build({
        entrypoints: [join(root, "entry.ts")],
        outdir: join(root, "out"),
        target: "browser",
        plugins: [publicBundleBoundaryPlugin({ forbidRepositoryCatalog: true })],
        metafile: true,
      }),
        "depends on a development-only replay catalog module",
      );
    });
  }

  test("allows the repository catalog module in a development-only bundle", async () => {
    const root = await temporaryDirectory();
    const dependencyPath = join(root, "src", "ui", "repository-replay-loader.ts");
    await mkdir(resolve(dependencyPath, ".."), { recursive: true });
    await writeFile(dependencyPath, "export const catalog = [];\n");
    await writeFile(
      join(root, "entry.ts"),
      'import { catalog } from "./src/ui/repository-replay-loader"; console.log(catalog);\n',
    );

    const result = await Bun.build({
      entrypoints: [join(root, "entry.ts")],
      outdir: join(root, "out"),
      target: "browser",
      plugins: [publicBundleBoundaryPlugin()],
      metafile: true,
    });

    expect(result.success).toBe(true);
  });
});

describe("public release policy", () => {
  test("rejects a repository with missing owner policy decisions", async () => {
    const root = await temporaryDirectory();

    await expect(checkPublicReleasePolicy(root)).rejects.toThrow(
      "missing required repository-root policy file: LICENSE",
    );
  });

  test("rejects obvious placeholder policy content", async () => {
    const root = await temporaryDirectory();
    await writeCompletePublicReleasePolicy(root);
    await writeFile(
      join(root, "CORPUS_POLICY.md"),
      "# Corpus policy\n\nTODO: this replay fixture distribution decision is still pending owner review. ".repeat(2),
    );

    await expect(checkPublicReleasePolicy(root)).rejects.toThrow(
      "policy file still contains placeholder language: CORPUS_POLICY.md",
    );
  });

  test("accepts substantive repository-root license, notice, and corpus decisions", async () => {
    const root = await temporaryDirectory();
    await writeCompletePublicReleasePolicy(root);

    const audit = await checkPublicReleasePolicy(root);

    expect(audit.files.map((file) => file.path)).toEqual([...PUBLIC_RELEASE_POLICY_FILES]);
  });
});

describe("production site build", () => {
  const testWasm = new Uint8Array([0x00, 0x61, 0x73, 0x6d, 0x01, 0x00, 0x00, 0x00]);

  test("dev build hashes the worker URL and follows the checkout cache policy", async () => {
    const outdir = await temporaryDirectory();
    const testIndex = join(outdir, "index.html");
    const sourceIndexPath = join(REPO_ROOT, "index.html");
    const sourceIndexBefore = await readFile(sourceIndexPath, "utf8");
    await writeFile(testIndex, sourceIndexBefore);

    const result = await buildUi({
      repoRoot: REPO_ROOT,
      outdir,
      indexHtml: testIndex,
      generateFixtureIndex: false,
      rustWasmBytes: testWasm,
    });
    const worker = await readFile(join(outdir, "workbench-worker.js"));
    const wasm = await readFile(join(outdir, "yixian-engine.wasm"));
    const main = await readFile(join(outdir, "main.js"), "utf8");
    const testIndexAfter = await readFile(testIndex, "utf8");

    expect(result.workerVersion).toBe(sha256Bytes(worker).slice(0, 12));
    expect(worker.length).toBeGreaterThan(0);
    expect(wasm.length).toBeGreaterThan(0);
    expect(result.workerUrl).toBe(
      `/public/build/workbench-worker.js?v=${result.workerVersion}`,
    );
    expect(main).toContain(result.workerUrl);
    // 私有开发树单调递增缓存号；确定性 public 投影保持 tracked index 不变。
    const prevVersionMatch = sourceIndexBefore.match(/\/public\/build\/main\.js\?v=(\d+)/);
    const prevVersion = prevVersionMatch ? Number(prevVersionMatch[1]) : 0;
    const publicProjection = !(await Bun.file(
      join(REPO_ROOT, "engine-ts/FROZEN_ENGINE_SHA256"),
    ).exists());
    const expectedMainVersion = String(publicProjection ? prevVersion : prevVersion + 1);
    expect(result.mainVersion).toBe(expectedMainVersion);
    expect(testIndexAfter).toContain(`/public/build/main.js?v=${result.mainVersion}`);
    expect(testIndexAfter).not.toContain("/public/build/workbench-worker.js");
    expect(await readFile(sourceIndexPath, "utf8")).toBe(sourceIndexBefore);
  });

  test("rejects repository source directories as release output", async () => {
    await expect(
      assertSafeOutputDirectory(REPO_ROOT, join(REPO_ROOT, "src")),
    ).rejects.toThrow("repository release output must be exactly");
  });

  test("rejects an unrelated non-empty output directory", async () => {
    const outdir = await temporaryDirectory();
    await writeFile(join(outdir, "foreign.txt"), "do not delete\n");

    await expect(
      assertSafeOutputDirectory(REPO_ROOT, outdir),
    ).rejects.toThrow("not an audited Open-YiXianCard release");
    expect(await readFile(join(outdir, "foreign.txt"), "utf8")).toBe("do not delete\n");
  });

  test("rejects tracked dirty input unless a local/test caller opts in explicitly", async () => {
    const repo = await temporaryDirectory();
    await writeFile(join(repo, "tracked.txt"), "clean\n");
    runGit(repo, ["init"]);
    runGit(repo, ["config", "user.email", "release-test@example.invalid"]);
    runGit(repo, ["config", "user.name", "Release Test"]);
    runGit(repo, ["add", "tracked.txt"]);
    runGit(repo, ["commit", "-m", "initial"]);
    assertTrackedWorkingTreeClean(repo, false);

    await writeFile(join(repo, "tracked.txt"), "dirty\n");
    expect(() => assertTrackedWorkingTreeClean(repo, false)).toThrow("clean tracked working tree");
    expect(() => assertTrackedWorkingTreeClean(repo, true)).not.toThrow();
  });

  test("builds a hashed, audited, zero-fixture dist without editing tracked index.html", async () => {
    const outdir = await temporaryDirectory();
    const sourceIndexBefore = await readFile(join(REPO_ROOT, "index.html"), "utf8");
    const audit = await buildSite({
      repoRoot: REPO_ROOT,
      outdir,
      appCommit: "d".repeat(40),
      allowTrackedDirty: true,
      rustWasmBytes: testWasm,
    });
    const sourceIndexAfter = await readFile(join(REPO_ROOT, "index.html"), "utf8");
    const javascriptPath = audit.files.find((file) =>
      /^assets\/main\.[a-f0-9]{16}\.js$/.test(file.path)
    )?.path;
    const workerPath = audit.files.find((file) =>
      /^assets\/workbench-worker\.[a-f0-9]{16}\.js$/.test(file.path)
    )?.path;
    expect(javascriptPath).toBeDefined();
    expect(workerPath).toBeDefined();
    const javascript = await readFile(join(outdir, ...javascriptPath!.split("/")), "utf8");
    const worker = await readFile(join(outdir, ...workerPath!.split("/")), "utf8");
    const publicSnapshot = JSON.parse(
      await readFile(join(REPO_ROOT, "shared/data/original-build-profiles.json"), "utf8"),
    ) as { readonly projectTargetSteamBuild: string };

    expect(sourceIndexAfter).toBe(sourceIndexBefore);
    expect(audit.manifest.supportedSteamBuild).toBe(publicSnapshot.projectTargetSteamBuild);
    expect(audit.manifest.fixturePolicy.bundledFixtureCount).toBe(0);
    expect(javascript).not.toContain("battle-evaluator/fixtures");
    expect(javascript).not.toContain("cq55h00/round-15");
    expect(javascript).not.toContain("e63lwvs/round-16");
    for (const bundle of [javascript]) {
      expect(bundle).not.toMatch(
        /(?:\bfetch\s*\(|\bXMLHttpRequest\b|\bWebSocket\b|\bEventSource\b|\bnavigator\s*\.\s*sendBeacon\s*\()/,
      );
      expect(bundle).not.toContain("battle-evaluator/fixtures");
      expect(bundle).not.toContain("cq55h00/round-15");
      expect(bundle).not.toContain("e63lwvs/round-16");
    }
    expect(worker).toMatch(/fetch\(["']\/assets\/yixian-engine\.[a-f0-9]{16}\.wasm["']\)/);
    expect(javascript).toContain("选择弈仙牌文件夹");
    expect(javascript).toContain("选择 .bin");
    expect(javascript).toContain(`/${workerPath}`);
    expect(javascript).not.toContain("open-yixiancard/workbench-worker");
    expect(worker).toContain("open-yixiancard/workbench-worker");
    for (const handlerMarker of ["simulation-failed", "solver-failed", "replay-decode-failed"]) {
      expect(javascript).not.toContain(handlerMarker);
      expect(worker).toContain(handlerMarker);
    }
    for (const heavyEntryIdentifier of [
      "prepareOriginalReplay",
      "solveExactDeckSearch",
      "solveBeamDeckSearch",
      "solveOrderBeamSearch",
      "solveTalentCardBeamSearch",
    ]) {
      expect(javascript).not.toContain(heavyEntryIdentifier);
    }
    const mainBytes = audit.files.find((file) => file.path === javascriptPath)?.bytes ?? 0;
    const workerBytes = audit.files.find((file) => file.path === workerPath)?.bytes ?? 0;
    expect(mainBytes).toBeGreaterThan(0);
    expect(mainBytes).toBeLessThan(2_500_000);
    expect(workerBytes).toBeGreaterThan(0);
    expect(workerBytes).toBeLessThan(2_500_000);
    expect(workerPath).toBe(
      `assets/${contentHashedAssetName("workbench-worker", "js", new TextEncoder().encode(worker))}`,
    );
    const productionIndex = await readFile(join(outdir, "index.html"), "utf8");
    expect(productionIndex).toContain(
      `<meta name="open-yixiancard:steam-build" content="${audit.manifest.supportedSteamBuild}" />`,
    );
    expect(productionIndex).toContain(
      `<meta name="open-yixiancard:ruleset" content="${audit.manifest.rulesetRevision}" />`,
    );
    expect(productionIndex).toContain(
      `<meta name="open-yixiancard:app-commit" content="${audit.manifest.appCommit}" />`,
    );
    expect(productionIndex).not.toContain(`/${workerPath}`);
    expect(audit.files.map((file) => file.path)).toEqual(expect.arrayContaining([
      "index.html",
      "release-manifest.json",
      "_headers",
    ]));
    expect(audit.files.every((file) =>
      !file.path.startsWith("assets/") || /\.[a-f0-9]{16}\.(?:js|css|wasm)$/.test(file.path)
    )).toBe(true);
  });

  test("builds byte-identical artifact inventories for the same release revision", async () => {
    const firstOutdir = await temporaryDirectory();
    const secondOutdir = await temporaryDirectory();
    const options = {
      repoRoot: REPO_ROOT,
      appCommit: "e".repeat(40),
      allowTrackedDirty: true,
      rustWasmBytes: testWasm,
    } as const;

    const first = await buildSite({ ...options, outdir: firstOutdir });
    const second = await buildSite({ ...options, outdir: secondOutdir });

    expect(second.manifestSha256).toBe(first.manifestSha256);
    expect(second.files).toEqual(first.files);
  });
});

describe("site workflow contract", () => {
  test("builds and audits the artifact without deployment", async () => {
    const workflow = await readFile(join(REPO_ROOT, ".github/workflows/site.yml"), "utf8");

    expect(workflow).not.toContain("release:");
    expect(workflow).not.toContain("wrangler");
    if (workflow.includes("Fresh-clone Rust UI release gates")) {
      expect(workflow).toContain("run: bun run check");
      expect(workflow).toContain("run: bun run check:rust:contracts && bun run check:rust:wasm");
      expect(workflow).toContain("run: bun run check:ui");
      expect(workflow).toContain("run: bun run test:release");
      expect(workflow).not.toContain("engine-ts");
      expect(workflow).not.toContain("battle-evaluator");
      expect(workflow).not.toContain("check:tui");
      expect(workflow).toContain("no deployment");
      return;
    }
    expect(workflow).toContain("run: bun run audit:boundaries");
    expect(workflow).toContain("run: bun run test:ui");
    expect(workflow).toContain("run: bun run smoke:ui");
    expect(workflow).toContain("run: bun run report:file-health -- --paths README.md docs/AGENT_CONTEXT.md docs/CROSS_LINE_RUNBOOK.md docs/PRODUCT_ARCHITECTURE.md battle-evaluator/README.md battle-evaluator/contracts battle-evaluator/data battle-evaluator/diagnostics battle-evaluator/ts-adapter battle-evaluator/rust-adapter engine-ts/src engine-ts/scripts src/ui/scripts/lib/ui-audit-scenarios.ts index.html");
    expect(workflow).not.toContain("check:tui");
    expect(workflow).toContain("run: bun run check:rust:quick");
    expect(workflow).toContain("run: bun run check:ts:types");
    expect(workflow).toContain("run: bun run check:public-boundary");
    expect(workflow).toContain("no deployment");
    expect(workflow).not.toContain("run: bun run check:surfaces");
  });
});

async function writeCompletePublicReleasePolicy(root: string): Promise<void> {
  await writeFile(
    join(root, "LICENSE"),
    [
      "Open-YiXianCard test license",
      "Permission is granted to use, copy, modify, and distribute this test package under the stated terms.",
      "The copyright and permission notices must remain with every distributed copy.",
    ].join("\n"),
  );
  await writeFile(
    join(root, "NOTICE"),
    [
      "Open-YiXianCard test notice",
      "This package contains original project code and references third-party game terminology for compatibility research.",
      "No third-party artwork, audio, fonts, or private player records are included in the public artifact.",
    ].join("\n"),
  );
  await writeFile(
    join(root, "CORPUS_POLICY.md"),
    [
      "# Engineering corpus distribution policy",
      "Synthetic replay fixtures specifically approved by project owners may be publicly distributed.",
      "Private original-game replay fixtures, player records, and extracted client corpus remain excluded from public releases.",
    ].join("\n"),
  );
}

async function expectBundleBoundaryFailure(
  build: Promise<Bun.BuildOutput>,
  expected: string,
): Promise<void> {
  try {
    await build;
  } catch (error) {
    const details = [
      error instanceof Error ? error.message : String(error),
      ...(
        error && typeof error === "object" && "errors" in error && Array.isArray(error.errors)
          ? error.errors.map(String)
          : []
      ),
    ].join("\n");
    expect(details).toContain(expected);
    return;
  }
  throw new Error(`expected bundle boundary failure containing: ${expected}`);
}

async function createCleanArtifact(): Promise<string> {
  const root = await temporaryDirectory();
  await mkdir(join(root, "assets"), { recursive: true });
  const wasmBytes = new Uint8Array([0, 97, 115, 109, 1, 0, 0, 0]);
  const wasmName = contentHashedAssetName("yixian-engine", "wasm", wasmBytes);
  const workerContent = `fetch("/assets/${wasmName}"); self.addEventListener("message", () => self.postMessage("ok"));\n`;
  const workerBytes = new TextEncoder().encode(workerContent);
  const workerName = contentHashedAssetName("workbench-worker", "js", workerBytes);
  const assets = new Map<string, string | Uint8Array>([
    ["main.js", `document.querySelector("#app"); new Worker("/assets/${workerName}");\n`],
    ["workbench-worker.js", workerContent],
    ["base.css", "body { margin: 0; }\n"],
    ["setup.css", ".setup { display: grid; }\n"],
    ["battle.css", ".battle { display: grid; }\n"],
    ["target-chart.css", ".target-chart { display: block; }\n"],
    ["responsive.css", "@media (max-width: 1280px) { body { min-width: 980px; } }\n"],
    ["yixian-engine.wasm", wasmBytes],
  ]);
  const builtNames = new Map<string, string>();
  for (const [logicalName, content] of assets) {
    const [name, extension] = logicalName.split(".") as [string, "js" | "css" | "wasm"];
    const bytes = typeof content === "string" ? new TextEncoder().encode(content) : content;
    const builtName = contentHashedAssetName(name, extension, bytes);
    builtNames.set(logicalName, builtName);
    await writeFile(join(root, "assets", builtName), bytes);
  }
  await writeFile(join(root, "index.html"), cleanIndexHtml(builtNames));
  await writeFile(join(root, "_headers"), RELEASE_CLOUDFLARE_HEADERS);
  await writeReleaseManifest(root, TEST_METADATA);
  return root;
}

async function replaceMainJavascript(root: string, content: string): Promise<void> {
  const manifest = await readReleaseManifest(root);
  const oldPath = manifest.inventory.files.find((file) => /^assets\/main\..+\.js$/.test(file.path))?.path;
  const workerPath = artifactAssetPath(
    manifest,
    /^assets\/workbench-worker\..+\.js$/,
  );
  if (!oldPath) throw new Error("test artifact has no main.js");
  await unlink(join(root, ...oldPath.split("/")));
  const bytes = new TextEncoder().encode(`${content}\nconst workerUrl = "/${workerPath}";\n`);
  const newName = contentHashedAssetName("main", "js", bytes);
  await writeFile(join(root, "assets", newName), bytes);
  const indexPath = join(root, "index.html");
  const index = await readFile(indexPath, "utf8");
  await writeFile(indexPath, index.replace(`/${oldPath}`, `/assets/${newName}`));
  await writeReleaseManifest(root, TEST_METADATA);
}

async function replaceWorkerJavascript(root: string, content: string): Promise<void> {
  const manifest = await readReleaseManifest(root);
  const oldPath = artifactAssetPath(
    manifest,
    /^assets\/workbench-worker\..+\.js$/,
  );
  await unlink(join(root, ...oldPath.split("/")));
  const bytes = new TextEncoder().encode(`${content}\n`);
  const newName = contentHashedAssetName("workbench-worker", "js", bytes);
  await writeFile(join(root, "assets", newName), bytes);
  await writeReleaseManifest(root, TEST_METADATA);
}

type TestReleaseManifest = {
  readonly inventory: { readonly files: readonly { readonly path: string }[] };
};

async function readReleaseManifest(root: string): Promise<TestReleaseManifest> {
  return JSON.parse(
    await readFile(join(root, "release-manifest.json"), "utf8"),
  ) as TestReleaseManifest;
}

function artifactAssetPath(manifest: TestReleaseManifest, pattern: RegExp): string {
  const path = manifest.inventory.files.find((file) => pattern.test(file.path))?.path;
  if (!path) throw new Error(`test artifact has no asset matching ${pattern}`);
  return path;
}

function cleanIndexHtml(assets: ReadonlyMap<string, string>): string {
  const links = ["base.css", "setup.css", "battle.css", "target-chart.css", "responsive.css"]
    .map((name) => `    <link rel="stylesheet" href="/assets/${assets.get(name)}" />`)
    .join("\n");
  return `<!doctype html>
<html lang="zh-CN">
  <head>
    <meta name="open-yixiancard:steam-build" content="${TEST_METADATA.supportedSteamBuild}" />
    <meta name="open-yixiancard:ruleset" content="${TEST_METADATA.rulesetRevision}" />
    <meta name="open-yixiancard:app-commit" content="${TEST_METADATA.appCommit}" />
${links}
  </head>
  <body>
    <div id="app"></div>
    <script type="module" src="/assets/${assets.get("main.js")}"></script>
  </body>
</html>
`;
}

async function temporaryDirectory(): Promise<string> {
  const root = await mkdtemp(join(tmpdir(), "open-yixiancard-release-test-"));
  temporaryRoots.push(root);
  return root;
}

function runGit(cwd: string, args: readonly string[]): void {
  const result = spawnSync("git", args, { cwd, encoding: "utf8" });
  if (result.status !== 0) throw new Error(result.stderr || result.stdout);
}
