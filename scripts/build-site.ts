import { spawnSync } from "node:child_process";
import {
  lstat,
  mkdir,
  mkdtemp,
  readFile,
  readdir,
  realpath,
  rm,
  writeFile,
} from "node:fs/promises";
import { tmpdir } from "node:os";
import {
  basename,
  dirname,
  isAbsolute,
  join,
  relative,
  resolve,
  sep,
} from "node:path";
import {
  auditReleaseArtifact,
  contentHashedAssetName,
  RELEASE_CLOUDFLARE_HEADERS,
  sha256Bytes,
  writeReleaseManifest,
  type ReleaseArtifactAudit,
  type ReleaseMetadata,
} from "./release-artifact";
import {
  assertPublicBundleMetafile,
  publicBundleBoundaryPlugin,
} from "./public-bundle-boundary";
import { buildRustWasm } from "./lib/build-rust-wasm";

const DEFAULT_REPO_ROOT = resolve(import.meta.dir, "..");
const CSS_ENTRY_NAMES = ["base", "setup", "battle", "target-chart", "responsive"] as const;
const ZERO_FIXTURE_NAMESPACE = "zero-fixture-production";
const PRODUCTION_BUNDLE_BOUNDARY = {
  forbidRepositoryCatalog: true,
  allowedVirtualNamespaces: [ZERO_FIXTURE_NAMESPACE],
  virtualCatalogReplacements: {
    fixtureIndex: `${ZERO_FIXTURE_NAMESPACE}:empty-catalog.json`,
    repositoryLoader: `${ZERO_FIXTURE_NAMESPACE}:disabled-catalog-loader.ts`,
  },
} as const;

export interface BuildSiteOptions {
  readonly repoRoot?: string;
  readonly outdir?: string;
  readonly appCommit?: string;
  readonly allowTrackedDirty?: boolean;
  /** Test-only escape hatch; production callers compile the canonical Rust module. */
  readonly rustWasmBytes?: Uint8Array;
}

export async function buildSite(options: BuildSiteOptions = {}): Promise<ReleaseArtifactAudit> {
  const repoRoot = resolve(options.repoRoot ?? DEFAULT_REPO_ROOT);
  const outdir = resolve(options.outdir ?? join(repoRoot, "dist"));
  await assertSafeOutputDirectory(repoRoot, outdir);
  assertTrackedWorkingTreeClean(repoRoot, options.allowTrackedDirty ?? false);

  const sharedSnapshotPath = join(repoRoot, "shared", "data", "original-build-profiles.json");
  const sharedSnapshotBytes = await readFile(sharedSnapshotPath);
  const sharedSnapshot = parseSharedSnapshot(sharedSnapshotBytes);
  const appCommit = options.appCommit ?? releaseAppCommit(repoRoot);
  const rulesetTree = gitRevision(repoRoot, "HEAD:engine-rust");
  const releaseMetadata = {
    supportedSteamBuild: sharedSnapshot.projectTargetSteamBuild,
    sharedSnapshotSha256: sha256Bytes(sharedSnapshotBytes),
    rulesetRevision: `engine-rust-tree:${rulesetTree}`,
    appCommit,
  } satisfies ReleaseMetadata;

  const temporaryRoot = await mkdtemp(join(tmpdir(), "open-yixiancard-site-"));
  try {
    const temporaryJsOutdir = join(temporaryRoot, "js");
    const javascript = await buildProductionJavascript(
      repoRoot,
      temporaryJsOutdir,
      options.rustWasmBytes,
    );
    const css = await buildProductionCss(repoRoot);

    await rm(outdir, { recursive: true, force: true });
    await mkdir(join(outdir, "assets"), { recursive: true });

    const javascriptName = contentHashedAssetName("main", "js", javascript.main);
    await writeFile(join(outdir, "assets", javascriptName), javascript.main);
    await writeFile(
      join(outdir, "assets", javascript.workerName),
      javascript.worker,
    );
    await writeFile(
      join(outdir, "assets", javascript.wasmName),
      javascript.wasm,
    );

    const cssNames = new Map<string, string>();
    for (const [name, bytes] of css) {
      const assetName = contentHashedAssetName(name, "css", bytes);
      cssNames.set(name, assetName);
      await writeFile(join(outdir, "assets", assetName), bytes);
    }

    const sourceHtml = await readFile(join(repoRoot, "index.html"), "utf8");
    const productionHtml = productionIndexHtml(
      sourceHtml,
      javascriptName,
      cssNames,
      releaseMetadata,
    );
    await writeFile(join(outdir, "index.html"), productionHtml);
    await writeFile(join(outdir, "_headers"), RELEASE_CLOUDFLARE_HEADERS);

    await writeReleaseManifest(outdir, releaseMetadata);
    return await auditReleaseArtifact(outdir);
  } finally {
    await rm(temporaryRoot, { recursive: true, force: true });
  }
}

interface ProductionJavaScript {
  readonly main: Uint8Array;
  readonly worker: Uint8Array;
  readonly workerName: string;
  readonly wasm: Uint8Array;
  readonly wasmName: string;
}

async function buildProductionJavascript(
  repoRoot: string,
  outdir: string,
  rustWasmBytes?: Uint8Array,
): Promise<ProductionJavaScript> {
  const wasm = rustWasmBytes ?? await buildRustWasm(repoRoot);
  const wasmName = contentHashedAssetName("yixian-engine", "wasm", wasm);
  const worker = await buildProductionJavaScriptEntry({
    entrypoint: join(repoRoot, "src", "ui", "workbench-worker.ts"),
    outdir: join(outdir, "worker"),
    expectedOutputName: "workbench-worker.js",
    define: {
      __OPEN_YIXIAN_ENGINE_WASM_URL__: JSON.stringify(`/assets/${wasmName}`),
    },
  });
  const workerName = contentHashedAssetName("workbench-worker", "js", worker);
  const main = await buildProductionJavaScriptEntry({
    entrypoint: join(repoRoot, "src", "ui", "main.ts"),
    outdir: join(outdir, "main"),
    expectedOutputName: "main.js",
    define: {
      __OPEN_YIXIAN_REPOSITORY_FIXTURES__: "false",
      __OPEN_YIXIAN_WORKBENCH_WORKER_URL__: JSON.stringify(`/assets/${workerName}`),
    },
    plugins: [zeroFixtureProductionBoundary()],
  });
  return { main, worker, workerName, wasm, wasmName };
}

async function buildProductionJavaScriptEntry(options: {
  readonly entrypoint: string;
  readonly outdir: string;
  readonly expectedOutputName: string;
  readonly define?: Readonly<Record<string, string>>;
  readonly plugins?: readonly Bun.BunPlugin[];
}): Promise<Uint8Array> {
  await mkdir(options.outdir, { recursive: true });
  const result = await Bun.build({
    entrypoints: [options.entrypoint],
    outdir: options.outdir,
    target: "browser",
    minify: true,
    splitting: false,
    metafile: true,
    plugins: [
      ...(options.plugins ?? []),
      publicBundleBoundaryPlugin(PRODUCTION_BUNDLE_BOUNDARY),
    ],
    ...(options.define ? { define: options.define } : {}),
  });
  if (!result.success) {
    throw new Error(
      `production ${options.expectedOutputName} build failed:\n${result.logs.map(String).join("\n")}`,
    );
  }
  assertPublicBundleMetafile(
    result.metafile,
    options.expectedOutputName,
    PRODUCTION_BUNDLE_BOUNDARY,
  );
  const outputs = result.outputs.filter((output) => output.path.endsWith(".js"));
  if (outputs.length !== 1) {
    throw new Error(`expected one ${options.expectedOutputName} output, got ${outputs.length}`);
  }
  const output = outputs[0]!;
  if (basename(output.path) !== options.expectedOutputName) {
    throw new Error(
      `expected ${options.expectedOutputName}, got ${basename(output.path)}`,
    );
  }
  return new Uint8Array(await output.arrayBuffer());
}

function zeroFixtureProductionBoundary(): Bun.BunPlugin {
  return {
    name: "zero-fixture-production-boundary",
    setup(build) {
      build.onResolve({ filter: /(?:^|[\\/])fixture-index\.json$/ }, () => ({
        path: "empty-catalog.json",
        namespace: ZERO_FIXTURE_NAMESPACE,
      }));
      build.onResolve(
        { filter: /(?:^|[\\/])repository-replay-loader(?:\.ts)?$/ },
        () => ({
          path: "disabled-catalog-loader.ts",
          namespace: ZERO_FIXTURE_NAMESPACE,
        }),
      );
      build.onLoad({ filter: /.*/, namespace: ZERO_FIXTURE_NAMESPACE }, (args) => {
        if (args.path === "empty-catalog.json") {
          return { contents: "[]", loader: "json" };
        }
        if (args.path === "disabled-catalog-loader.ts") {
          return {
            contents: `
              export async function loadRepositoryReplayFixture(): Promise<never> {
                throw new Error("托管站无内置回放；请显式选择本地版本化 JSON。");
              }
            `,
            loader: "ts",
          };
        }
        throw new Error(`unexpected zero-fixture virtual module: ${args.path}`);
      });
    },
  };
}

async function buildProductionCss(repoRoot: string): Promise<Map<string, Uint8Array>> {
  const stylesRoot = join(repoRoot, "src", "ui", "styles");
  const output = new Map<string, Uint8Array>();
  for (const name of CSS_ENTRY_NAMES) {
    const css = await inlineCss(join(stylesRoot, `${name}.css`), stylesRoot, []);
    if (/@import\b/i.test(css)) throw new Error(`${name}.css retained an unsupported @import`);
    if (/(?:url\s*\(|@font-face)/i.test(css)) {
      throw new Error(`${name}.css references an unreviewed external/media asset`);
    }
    output.set(name, new TextEncoder().encode(css));
  }
  return output;
}

async function inlineCss(path: string, stylesRoot: string, stack: readonly string[]): Promise<string> {
  const absolutePath = resolve(path);
  const relativePath = relative(stylesRoot, absolutePath).split(sep).join("/");
  if (relativePath.startsWith("../") || relativePath === "..") {
    throw new Error(`CSS import escapes styles root: ${path}`);
  }
  if (stack.includes(absolutePath)) {
    throw new Error(`cyclic CSS import: ${[...stack, absolutePath].join(" -> ")}`);
  }
  const source = await readFile(absolutePath, "utf8");
  const importPattern = /@import\s+["'](?<specifier>[^"']+)["']\s*;/g;
  const matches = [...source.matchAll(importPattern)];
  let cursor = 0;
  let output = "";
  for (const match of matches) {
    const index = match.index ?? 0;
    output += source.slice(cursor, index);
    const specifier = match.groups?.specifier ?? "";
    const importedPath = specifier.split("?", 1)[0]!;
    if (!importedPath.startsWith("./") || !importedPath.endsWith(".css")) {
      throw new Error(`CSS import is not an allowlisted local stylesheet: ${specifier}`);
    }
    const resolvedImport = resolve(dirname(absolutePath), importedPath);
    output += `\n/* inlined ${relative(stylesRoot, resolvedImport).split(sep).join("/")} */\n`;
    output += await inlineCss(resolvedImport, stylesRoot, [...stack, absolutePath]);
    cursor = index + match[0].length;
  }
  output += source.slice(cursor);
  return output;
}

function productionIndexHtml(
  source: string,
  javascriptName: string,
  cssNames: ReadonlyMap<string, string>,
  releaseMetadata: ReleaseMetadata,
): string {
  let html = source;
  for (const name of CSS_ENTRY_NAMES) {
    const assetName = cssNames.get(name);
    if (!assetName) throw new Error(`missing built CSS asset: ${name}`);
    html = replaceExactly(
      html,
      new RegExp(`/src/ui/styles/${name}\\.css(?:\\?[^"']*)?`, "g"),
      `/assets/${assetName}`,
      `${name}.css reference`,
    );
  }
  html = replaceExactly(
    html,
    /\/public\/build\/main\.js(?:\?[^"']*)?/g,
    `/assets/${javascriptName}`,
    "main.js reference",
  );
  html = replaceMetaContent(
    html,
    "open-yixiancard:steam-build",
    releaseMetadata.supportedSteamBuild,
  );
  html = replaceMetaContent(
    html,
    "open-yixiancard:ruleset",
    releaseMetadata.rulesetRevision,
  );
  html = replaceMetaContent(
    html,
    "open-yixiancard:app-commit",
    releaseMetadata.appCommit,
  );
  if (/\/(?:src|public\/build)\//.test(html)) {
    throw new Error("production index retained a source/build-tree path");
  }
  return html.endsWith("\n") ? html : `${html}\n`;
}

function replaceMetaContent(source: string, name: string, value: string): string {
  const tags = [...source.matchAll(/<meta\b[^>]*>/gi)]
    .filter((match) => metaTagName(match[0]) === name);
  if (tags.length !== 1) {
    throw new Error(`expected exactly one ${name} release meta tag, got ${tags.length}`);
  }
  const match = tags[0]!;
  const tag = match[0];
  const content = /\bcontent\s*=\s*(?:"[^"]*"|'[^']*')/i;
  if (!content.test(tag)) throw new Error(`${name} release meta tag has no content attribute`);
  const nextTag = tag.replace(content, `content="${value}"`);
  const index = match.index ?? 0;
  return `${source.slice(0, index)}${nextTag}${source.slice(index + tag.length)}`;
}

function metaTagName(tag: string): string | null {
  const match = tag.match(/\bname\s*=\s*(?:"([^"]*)"|'([^']*)')/i);
  return match?.[1] ?? match?.[2] ?? null;
}

function replaceExactly(source: string, pattern: RegExp, replacement: string, label: string): string {
  const matches = [...source.matchAll(pattern)];
  if (matches.length !== 1) {
    throw new Error(`expected exactly one ${label}, got ${matches.length}`);
  }
  return source.replace(pattern, replacement);
}

function parseSharedSnapshot(bytes: Uint8Array): { readonly projectTargetSteamBuild: string } {
  const parsed = JSON.parse(new TextDecoder().decode(bytes)) as {
    readonly projectTargetSteamBuild?: unknown;
  };
  if (
    typeof parsed.projectTargetSteamBuild !== "string"
    || !/^\d+$/.test(parsed.projectTargetSteamBuild)
  ) {
    throw new Error("public shared snapshot has no authoritative projectTargetSteamBuild");
  }
  return { projectTargetSteamBuild: parsed.projectTargetSteamBuild };
}

function releaseAppCommit(repoRoot: string): string {
  const injected = process.env.RELEASE_APP_COMMIT
    ?? process.env.GITHUB_SHA
    ?? process.env.CF_PAGES_COMMIT_SHA;
  if (injected !== undefined) {
    const normalized = injected.trim().toLowerCase();
    if (!/^[a-f0-9]{7,64}$/.test(normalized)) throw new Error("injected app commit is not a Git revision");
    return normalized;
  }
  return gitRevision(repoRoot, "HEAD");
}

function gitRevision(repoRoot: string, revision: string): string {
  const result = spawnSync("git", ["rev-parse", revision], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`cannot resolve ${revision}: ${result.stderr.trim()}`);
  }
  const value = result.stdout.trim().toLowerCase();
  if (!/^[a-f0-9]{40}$/.test(value)) throw new Error(`invalid Git revision for ${revision}`);
  return value;
}

export async function assertSafeOutputDirectory(repoRoot: string, outdir: string): Promise<void> {
  const canonicalRepoRoot = await realpath(repoRoot);
  const canonicalOutdir = await canonicalTargetPath(outdir);
  if (pathContains(canonicalRepoRoot, canonicalOutdir)) {
    const allowedRepositoryOutdir = join(canonicalRepoRoot, "dist");
    if (canonicalOutdir !== allowedRepositoryOutdir) {
      throw new Error(`repository release output must be exactly ${allowedRepositoryOutdir}`);
    }
  } else if (pathContains(canonicalOutdir, canonicalRepoRoot)) {
    throw new Error(`release output cannot contain the repository: ${canonicalOutdir}`);
  }

  const stat = await lstat(outdir).catch((error: NodeJS.ErrnoException) => {
    if (error.code === "ENOENT") return null;
    throw error;
  });
  if (stat === null) return;
  if (stat.isSymbolicLink() || !stat.isDirectory()) {
    throw new Error(`release output must be a real directory: ${outdir}`);
  }
  const entries = await readdir(outdir);
  if (entries.length === 0) return;
  try {
    await auditReleaseArtifact(outdir);
  } catch (error) {
    throw new Error(
      `refusing to replace a non-empty directory that is not an audited Open-YiXianCard release: ${outdir}`,
      { cause: error },
    );
  }
}

async function canonicalTargetPath(path: string): Promise<string> {
  let cursor = resolve(path);
  const suffix: string[] = [];
  while (true) {
    try {
      const canonical = await realpath(cursor);
      return resolve(canonical, ...suffix.reverse());
    } catch (error) {
      const code = (error as NodeJS.ErrnoException).code;
      if (code !== "ENOENT") throw error;
      const parent = dirname(cursor);
      if (parent === cursor) throw error;
      suffix.push(basename(cursor));
      cursor = parent;
    }
  }
}

function pathContains(parent: string, candidate: string): boolean {
  const path = relative(parent, candidate);
  return path === "" || (!isAbsolute(path) && path !== ".." && !path.startsWith(`..${sep}`));
}

export function assertTrackedWorkingTreeClean(repoRoot: string, allowTrackedDirty: boolean): void {
  if (allowTrackedDirty) return;
  const result = spawnSync("git", ["status", "--porcelain=v1", "--untracked-files=no"], {
    cwd: repoRoot,
    encoding: "utf8",
  });
  if (result.status !== 0) {
    throw new Error(`cannot inspect tracked working tree: ${result.stderr.trim()}`);
  }
  const dirty = result.stdout.trim();
  if (dirty !== "") {
    throw new Error(`production build requires a clean tracked working tree:\n${dirty}`);
  }
}

function parseCliOptions(): { readonly outdir?: string; readonly allowTrackedDirty: boolean } {
  const args = process.argv.slice(2);
  let outdir: string | undefined;
  let allowTrackedDirty = false;
  for (let index = 0; index < args.length; index += 1) {
    if (args[index] === "--outdir") {
      const value = args[index + 1];
      if (!value) throw new Error("--outdir requires a value");
      outdir = value;
      index += 1;
      continue;
    }
    if (args[index]!.startsWith("--outdir=")) {
      outdir = args[index]!.slice("--outdir=".length);
      continue;
    }
    if (args[index] === "--allow-tracked-dirty") {
      allowTrackedDirty = true;
      continue;
    }
    throw new Error(`unknown argument: ${args[index]}`);
  }
  return { ...(outdir ? { outdir } : {}), allowTrackedDirty };
}

if (import.meta.main) {
  const cli = parseCliOptions();
  const audit = await buildSite(cli);
  console.log(JSON.stringify({
    artifact: resolve(cli.outdir ?? join(DEFAULT_REPO_ROOT, "dist")),
    manifestSha256: audit.manifestSha256,
    supportedSteamBuild: audit.manifest.supportedSteamBuild,
    rulesetRevision: audit.manifest.rulesetRevision,
    appCommit: audit.manifest.appCommit,
    fileCount: audit.files.length,
    totalBytes: audit.totalBytes,
  }, null, 2));
}
