import { createHash } from "node:crypto";
import {
  mkdir,
  readFile,
  writeFile,
} from "node:fs/promises";
import {
  basename,
  join,
  resolve,
} from "node:path";
import {
  assertPublicBundleMetafile,
  publicBundleBoundaryPlugin,
} from "./public-bundle-boundary";
import { buildRustWasm } from "./lib/build-rust-wasm";

const DEFAULT_REPO_ROOT = resolve(import.meta.dir, "..");
const MAIN_OUTPUT_NAME = "main.js";
const WORKER_OUTPUT_NAME = "workbench-worker.js";
const WASM_OUTPUT_NAME = "yixian-engine.wasm";

export interface BuildUiOptions {
  readonly repoRoot?: string;
  readonly outdir?: string;
  readonly indexHtml?: string;
  readonly generateFixtureIndex?: boolean;
  /** Test-only escape hatch; production callers compile the canonical Rust module. */
  readonly rustWasmBytes?: Uint8Array;
}

export interface BuildUiResult {
  readonly mainVersion: string;
  readonly workerVersion: string;
  readonly workerUrl: string;
}

export async function buildUi(options: BuildUiOptions = {}): Promise<BuildUiResult> {
  const repoRoot = resolve(options.repoRoot ?? DEFAULT_REPO_ROOT);
  const outdir = resolve(options.outdir ?? join(repoRoot, "public", "build"));
  const indexHtml = resolve(options.indexHtml ?? join(repoRoot, "index.html"));

  if (options.generateFixtureIndex === true) throw new Error("public builds do not generate private fixture indexes");
  await mkdir(outdir, { recursive: true });

  const wasm = options.rustWasmBytes ?? await buildRustWasm(repoRoot);
  const wasmVersion = shortHash(wasm);
  await writeFile(join(outdir, WASM_OUTPUT_NAME), wasm);
  const wasmUrl = `/public/build/${WASM_OUTPUT_NAME}?v=${wasmVersion}`;

  const worker = await bundleBrowserEntry({
    entrypoint: join(repoRoot, "src", "ui", "workbench-worker.ts"),
    outdir,
    expectedOutputName: WORKER_OUTPUT_NAME,
    define: {
      __OPEN_YIXIAN_ENGINE_WASM_URL__: JSON.stringify(wasmUrl),
    },
  });
  const workerVersion = shortHash(worker);
  const workerUrl = `/public/build/${WORKER_OUTPUT_NAME}?v=${workerVersion}`;

  const main = await bundleBrowserEntry({
    entrypoint: join(repoRoot, "src", "ui", "main.ts"),
    outdir,
    expectedOutputName: MAIN_OUTPUT_NAME,
    define: {
      __OPEN_YIXIAN_WORKBENCH_WORKER_URL__: JSON.stringify(workerUrl),
    },
  });

  const html = await readFile(indexHtml, "utf8");
  const mainVersion = nextCacheVersion(html);
  const mainScriptPattern = /(?<src>\/public\/build\/main\.js\?v=)(?<version>[^"']+)/;
  if (!mainScriptPattern.test(html)) {
    throw new Error("Could not find /public/build/main.js?v=... in index.html");
  }
  const nextHtml = html.replace(mainScriptPattern, `$<src>${mainVersion}`);
  if (nextHtml !== html) await writeFile(indexHtml, nextHtml);

  return { mainVersion, workerVersion, workerUrl };
}

async function bundleBrowserEntry(options: {
  readonly entrypoint: string;
  readonly outdir: string;
  readonly expectedOutputName: string;
  readonly define?: Readonly<Record<string, string>>;
}): Promise<Uint8Array> {
  const result = await Bun.build({
    entrypoints: [options.entrypoint],
    outdir: options.outdir,
    target: "browser",
    splitting: false,
    metafile: true,
    plugins: [publicBundleBoundaryPlugin()],
    ...(options.define ? { define: options.define } : {}),
  });
  if (!result.success) {
    throw new Error(
      `${options.expectedOutputName} build failed:\n${result.logs.map(String).join("\n")}`,
    );
  }
  assertPublicBundleMetafile(result.metafile, options.expectedOutputName);
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

function shortHash(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex").slice(0, 12);
}

/**
 * UI main.js 的缓存版本号单调递增，不循环复用旧值。
 * 这样不会在代理/浏览器缓存 TTL 内重新命中历史 bundle。
 */
function nextCacheVersion(html: string): string {
  const match = html.match(/\/public\/build\/main\.js\?v=(?<version>\d+)/);
  const prev = match?.groups?.version;
  if (prev === undefined) return "0";
  const parsed = Number(prev);
  return Number.isSafeInteger(parsed) && parsed >= 0 ? String(parsed) : "0";
}

if (import.meta.main) {
  const result = await buildUi();
  console.log(
    `verified index.html main.js cache version ${result.mainVersion}; worker ${result.workerUrl}`,
  );
}
