import { createHash } from "node:crypto";
import {
  lstat,
  readFile,
  readdir,
  writeFile,
} from "node:fs/promises";
import { basename, join, relative, resolve, sep } from "node:path";

export const RELEASE_PRODUCT = "Open-YiXianCard";
export const RELEASE_SCHEMA_VERSION = 2;
export const RELEASE_MANIFEST_FILENAME = "release-manifest.json";
export const MAX_PUBLIC_ARTIFACT_BYTES = 100_000_000;
export const RELEASE_CLOUDFLARE_HEADERS = `/*
  Content-Security-Policy: default-src 'self'; script-src 'self'; worker-src 'self'; style-src 'self'; img-src 'none'; font-src 'none'; connect-src 'self'; object-src 'none'; base-uri 'none'; frame-ancestors 'none'; form-action 'none'
  Permissions-Policy: camera=(), microphone=(), geolocation=(), payment=(), usb=()
  Referrer-Policy: no-referrer
  X-Content-Type-Options: nosniff
  X-Frame-Options: DENY

/assets/*
  Cache-Control: public, max-age=31536000, immutable

/index.html
  Cache-Control: no-store

/release-manifest.json
  Cache-Control: no-store
`;

const HASHED_ASSET_PATH = /^assets\/(?<name>main|workbench-worker|yixian-engine|base|setup|battle|target-chart|responsive)\.(?<hash>[a-f0-9]{16})\.(?<extension>js|css|wasm)$/;
const EXPECTED_ASSET_NAMES = new Set([
  "main.js",
  "workbench-worker.js",
  "yixian-engine.wasm",
  "base.css",
  "setup.css",
  "battle.css",
  "target-chart.css",
  "responsive.css",
]);
const HTML_REFERENCED_ASSET_NAMES = new Set([
  "main.js",
  "base.css",
  "setup.css",
  "battle.css",
  "target-chart.css",
  "responsive.css",
]);
const RELEASE_META_NAMES = {
  supportedSteamBuild: "open-yixiancard:steam-build",
  rulesetRevision: "open-yixiancard:ruleset",
  appCommit: "open-yixiancard:app-commit",
} as const;

const FORBIDDEN_CONTENT: readonly { readonly label: string; readonly pattern: RegExp }[] = [
  {
    label: "fixture path",
    pattern: /(?:battle-evaluator[\\/]fixtures|fixtures[\\/](?:candidates|incoming|contracts))/i,
  },
  {
    label: "embedded fixture payload",
    pattern: /"schemaVersion"\s*:\s*\d+\s*,\s*"source"\s*:\s*\{[^}]*"steamBuild"[^}]*\}\s*,\s*"firstPlayerSide"\s*:/i,
  },
  {
    label: "fixture identifier",
    pattern: /\b[a-z0-9]{5,16}[\\/]round-\d{1,2}\b/i,
  },
  {
    label: "original replay source field",
    pattern: /["']?(?:recentBattleFile|downloadBattleFile|battleFile|sourceFile)["']?\s*:/i,
  },
  {
    label: "original binary path",
    pattern: /(?:^|["'`(=\s])(?:\.{1,2}[\\/]|~[\\/]|[a-z]:[\\/])[^"'`\s]*\.(?:bin|dll|pdb)\b/i,
  },
  {
    label: "user identifier field",
    pattern: /["']?(?:uid|username|homePlayerId|gameId)["']?\s*:/i,
  },
  {
    label: "local machine path",
    pattern: /(?:file:\/\/|\/home\/|\/Users\/|[a-z]:[\\/]Users[\\/]|%USERPROFILE%|AppData[\\/]LocalLow|\.config[\\/]unity3d)/i,
  },
  {
    label: "unreviewed media or font asset",
    pattern: /(?:data:(?:image|audio|video|font)|\.(?:png|jpe?g|gif|webp|svg|ico|woff2?|ttf|otf|mp3|ogg|wav|mp4|webm)(?:[?"'`\s]|$)|@font-face|url\s*\()/i,
  },
  {
    label: "remote asset URL",
    pattern: /https?:\/\//i,
  },
  {
    label: "development build path",
    pattern: /\/public\/build\/(?:main|workbench-worker)\.js/i,
  },
];

const REVIEWED_PUBLIC_LOCAL_PATH_COPY = [
  /%USERPROFILE%(?:\\{1,2}|\/)AppData(?:\\{1,2}|\/)LocalLow(?:\\{1,2}|\/)DarkSunStudio(?:\\{1,2}|\/)YiXianPai/gi,
  /AppData(?:\\{1,2}|\/)LocalLow(?:\\{1,2}|\/)DarkSunStudio(?:\\{1,2}|\/)YiXianPai/gi,
  /\$HOME\/\.config\/unity3d\/DarkSunStudio\/YiXianPai/gi,
  /\.config\/unity3d\/DarkSunStudio\/YiXianPai/gi,
] as const;

export interface ReleaseFileEntry {
  readonly path: string;
  readonly sha256: string;
  readonly bytes: number;
}

export interface ReleaseManifest {
  readonly product: typeof RELEASE_PRODUCT;
  readonly schemaVersion: typeof RELEASE_SCHEMA_VERSION;
  readonly supportedSteamBuild: string;
  readonly sharedSnapshotSha256: string;
  readonly rulesetRevision: string;
  readonly appCommit: string;
  readonly fixturePolicy: {
    readonly bundledFixtureCount: 0;
    readonly catalog: "empty";
    readonly remoteFetch: false;
  };
  readonly inventory: {
    readonly algorithm: "sha256";
    readonly scope: "all artifact files except release-manifest.json (self-reference)";
    readonly files: readonly ReleaseFileEntry[];
  };
}

export interface ReleaseMetadata {
  readonly supportedSteamBuild: string;
  readonly sharedSnapshotSha256: string;
  readonly rulesetRevision: string;
  readonly appCommit: string;
}

export interface ReleaseArtifactAudit {
  readonly manifest: ReleaseManifest;
  readonly manifestSha256: string;
  readonly files: readonly ReleaseFileEntry[];
  readonly totalBytes: number;
}

export class ReleaseArtifactError extends Error {
  constructor(readonly failures: readonly string[]) {
    super(`release artifact rejected:\n${failures.map((failure) => `- ${failure}`).join("\n")}`);
    this.name = "ReleaseArtifactError";
  }
}

export function sha256Bytes(bytes: Uint8Array): string {
  return createHash("sha256").update(bytes).digest("hex");
}

export function contentHashedAssetName(name: string, extension: "js" | "css", bytes: Uint8Array): string {
  return `${name}.${sha256Bytes(bytes).slice(0, 16)}.${extension}`;
}

export function publicArtifactSizeFailure(
  files: readonly Pick<ReleaseFileEntry, "bytes">[],
  maxBytes = MAX_PUBLIC_ARTIFACT_BYTES,
): string | null {
  const totalBytes = files.reduce((sum, file) => sum + file.bytes, 0);
  return totalBytes > maxBytes
    ? `artifact is ${totalBytes} bytes, exceeding the ${maxBytes}-byte limit`
    : null;
}

export async function writeReleaseManifest(
  artifactRoot: string,
  metadata: ReleaseMetadata,
): Promise<ReleaseManifest> {
  const files = await collectReleaseFiles(artifactRoot, { excludeManifest: true });
  const manifest: ReleaseManifest = {
    product: RELEASE_PRODUCT,
    schemaVersion: RELEASE_SCHEMA_VERSION,
    supportedSteamBuild: metadata.supportedSteamBuild,
    sharedSnapshotSha256: metadata.sharedSnapshotSha256,
    rulesetRevision: metadata.rulesetRevision,
    appCommit: metadata.appCommit,
    fixturePolicy: {
      bundledFixtureCount: 0,
      catalog: "empty",
      remoteFetch: false,
    },
    inventory: {
      algorithm: "sha256",
      scope: "all artifact files except release-manifest.json (self-reference)",
      files,
    },
  };
  await writeFile(
    join(artifactRoot, RELEASE_MANIFEST_FILENAME),
    `${JSON.stringify(manifest, null, 2)}\n`,
  );
  return manifest;
}

export async function auditReleaseArtifact(artifactRoot: string): Promise<ReleaseArtifactAudit> {
  const root = resolve(artifactRoot);
  const failures: string[] = [];
  const files = await collectReleaseFiles(root).catch((error) => {
    failures.push(visibleError(error));
    return [] as ReleaseFileEntry[];
  });
  const paths = new Set(files.map((file) => file.path));
  const totalBytes = files.reduce((sum, file) => sum + file.bytes, 0);
  const sizeFailure = publicArtifactSizeFailure(files);
  if (sizeFailure) failures.push(sizeFailure);

  for (const required of ["index.html", "_headers", RELEASE_MANIFEST_FILENAME]) {
    if (!paths.has(required)) failures.push(`missing required file: ${required}`);
  }

  const seenAssetNames = new Set<string>();
  const assetNameCounts = new Map<string, number>();
  for (const file of files) {
    if (["index.html", "_headers", RELEASE_MANIFEST_FILENAME].includes(file.path)) continue;
    const match = file.path.match(HASHED_ASSET_PATH);
    if (!match?.groups) {
      failures.push(`path is not allowlisted: ${file.path}`);
      continue;
    }
    const logicalName = `${match.groups.name}.${match.groups.extension}`;
    seenAssetNames.add(logicalName);
    assetNameCounts.set(logicalName, (assetNameCounts.get(logicalName) ?? 0) + 1);
    if (!file.sha256.startsWith(match.groups.hash)) {
      failures.push(`content hash mismatch in filename: ${file.path}`);
    }
  }
  for (const expected of EXPECTED_ASSET_NAMES) {
    if (!seenAssetNames.has(expected)) failures.push(`missing hashed asset: ${expected}`);
  }
  for (const actual of seenAssetNames) {
    if (!EXPECTED_ASSET_NAMES.has(actual)) failures.push(`unexpected hashed asset: ${actual}`);
  }
  for (const [logicalName, count] of assetNameCounts) {
    if (count > 1) failures.push(`duplicate hashed asset: ${logicalName}`);
  }

  for (const file of files) {
    const absolutePath = join(root, ...file.path.split("/"));
    const bytes = await readFile(absolutePath);
    if (file.path.endsWith(".wasm")) continue;
    let textContent: string;
    try {
      textContent = new TextDecoder("utf-8", { fatal: true }).decode(bytes);
    } catch {
      failures.push(`file is not UTF-8 text: ${file.path}`);
      continue;
    }
    for (const forbidden of FORBIDDEN_CONTENT) {
      const contentToAudit = forbidden.label === "local machine path"
        ? withoutReviewedPublicLocalPathCopy(textContent)
        : textContent;
      if (forbidden.pattern.test(contentToAudit)) {
        failures.push(`${forbidden.label} found in ${file.path}`);
      }
    }
    if (file.path.endsWith(".js") && hasForbiddenNetworkApi(textContent, file.path)) {
      failures.push(`network upload/fetch API found in ${file.path}`);
    }
  }

  const manifestPath = join(root, RELEASE_MANIFEST_FILENAME);
  const manifestBytes = await readFile(manifestPath).catch(() => null);
  const manifest = manifestBytes === null
    ? null
    : parseManifest(manifestBytes, failures);
  if (manifest) validateManifest(manifest, files, failures);

  const html = await readFile(join(root, "index.html"), "utf8").catch(() => "");
  const headers = await readFile(join(root, "_headers"), "utf8").catch(() => "");
  if (headers !== RELEASE_CLOUDFLARE_HEADERS) {
    failures.push("_headers does not match the reviewed production security and cache policy");
  }
  if (/\/(?:src|public\/build)\//.test(html)) {
    failures.push("index.html references source/build-tree paths");
  }
  for (const logicalName of HTML_REFERENCED_ASSET_NAMES) {
    const [name, extension] = logicalName.split(".");
    const asset = files.find((file) =>
      file.path.startsWith(`assets/${name}.`) && file.path.endsWith(`.${extension}`)
    );
    if (asset && !html.includes(`/${asset.path}`) && !html.includes(`./${asset.path}`)) {
      failures.push(`index.html does not reference ${asset.path}`);
    }
  }
  const mainAsset = files.find((file) => /^assets\/main\.[a-f0-9]{16}\.js$/.test(file.path));
  const workerAsset = files.find((file) =>
    /^assets\/workbench-worker\.[a-f0-9]{16}\.js$/.test(file.path)
  );
  const wasmAsset = files.find((file) =>
    /^assets\/yixian-engine\.[a-f0-9]{16}\.wasm$/.test(file.path)
  );
  if (/workbench-worker/i.test(html)) {
    failures.push("index.html must load the workbench worker through main.js only");
  }
  if (mainAsset && workerAsset) {
    const mainJavaScript = await readFile(join(root, ...mainAsset.path.split("/")), "utf8")
      .catch(() => "");
    const workerReferences = [
      ...mainJavaScript.matchAll(/(?:\.\/|\/)?assets\/workbench-worker\.[A-Za-z0-9._-]+\.js/g),
    ].map((match) => match[0].replace(/^\.\//, "").replace(/^\//, ""));
    const expectedWorkerReference = workerAsset.path;
    if (workerReferences.length === 0) {
      failures.push("main.js does not reference the hashed workbench worker");
    } else if (workerReferences.some((reference) => reference !== expectedWorkerReference)) {
      failures.push("main.js workbench worker reference does not match the artifact inventory");
    }
  }
  if (workerAsset && wasmAsset) {
    const workerJavaScript = await readFile(
      join(root, ...workerAsset.path.split("/")),
      "utf8",
    ).catch(() => "");
    const wasmReferences = [
      ...workerJavaScript.matchAll(/(?:\.\/|\/)?(?:assets\/)?yixian-engine\.[A-Za-z0-9._-]+\.wasm/g),
    ].map((match) => match[0].replace(/^\.\//, "").replace(/^\//, "").replace(/^assets\//, ""));
    const expectedWasmReference = basename(wasmAsset.path);
    if (wasmReferences.length === 0) {
      failures.push("workbench worker does not reference the hashed Rust/WASM engine");
    } else if (wasmReferences.some((reference) => reference !== expectedWasmReference)) {
      failures.push("workbench worker Rust/WASM reference does not match the artifact inventory");
    }
  }
  if (manifest) {
    for (const [field, name] of Object.entries(RELEASE_META_NAMES) as readonly [
      keyof typeof RELEASE_META_NAMES,
      string,
    ][]) {
      const content = htmlMetaContent(html, name, failures);
      if (content !== null && content !== manifest[field]) {
        failures.push(`index.html ${name} does not match release manifest`);
      }
    }
  }

  if (failures.length > 0 || !manifest || !manifestBytes) {
    throw new ReleaseArtifactError([...new Set(failures)]);
  }
  return {
    manifest,
    manifestSha256: sha256Bytes(manifestBytes),
    files,
    totalBytes,
  };
}

function withoutReviewedPublicLocalPathCopy(source: string): string {
  return REVIEWED_PUBLIC_LOCAL_PATH_COPY.reduce(
    (result, pattern) => result.replace(pattern, ""),
    source,
  );
}

function htmlMetaContent(html: string, name: string, failures: string[]): string | null {
  const tags = [...html.matchAll(/<meta\b[^>]*>/gi)]
    .filter((match) => metaTagName(match[0]) === name);
  if (tags.length !== 1) {
    failures.push(`index.html must contain exactly one ${name} meta tag`);
    return null;
  }
  const content = tags[0]![0].match(/\bcontent\s*=\s*(?:"([^"]*)"|'([^']*)')/i);
  if (!content) {
    failures.push(`index.html ${name} meta tag has no content attribute`);
    return null;
  }
  return content[1] ?? content[2] ?? "";
}

function metaTagName(tag: string): string | null {
  const match = tag.match(/\bname\s*=\s*(?:"([^"]*)"|'([^']*)')/i);
  return match?.[1] ?? match?.[2] ?? null;
}

export async function collectReleaseFiles(
  artifactRoot: string,
  options: { readonly excludeManifest?: boolean } = {},
): Promise<ReleaseFileEntry[]> {
  const root = resolve(artifactRoot);
  const rootStat = await lstat(root);
  if (!rootStat.isDirectory() || rootStat.isSymbolicLink()) {
    throw new Error(`artifact root is not a real directory: ${root}`);
  }
  const paths = await collectFilePaths(root, root);
  const selected = options.excludeManifest
    ? paths.filter((path) => path !== RELEASE_MANIFEST_FILENAME)
    : paths;
  return Promise.all(selected.map(async (path) => {
    const bytes = await readFile(join(root, ...path.split("/")));
    return {
      path,
      sha256: sha256Bytes(bytes),
      bytes: bytes.byteLength,
    };
  }));
}

async function collectFilePaths(root: string, directory: string): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths: string[] = [];
  for (const entry of entries.sort((left, right) => left.name.localeCompare(right.name))) {
    const absolutePath = join(directory, entry.name);
    const stat = await lstat(absolutePath);
    const artifactPath = relative(root, absolutePath).split(sep).join("/");
    if (stat.isSymbolicLink()) throw new Error(`symbolic links are forbidden: ${artifactPath}`);
    if (stat.isDirectory()) {
      paths.push(...await collectFilePaths(root, absolutePath));
      continue;
    }
    if (!stat.isFile()) throw new Error(`non-regular file is forbidden: ${artifactPath}`);
    paths.push(artifactPath);
  }
  return paths.sort();
}

function parseManifest(bytes: Uint8Array, failures: string[]): ReleaseManifest | null {
  try {
    return JSON.parse(new TextDecoder().decode(bytes)) as ReleaseManifest;
  } catch (error) {
    failures.push(`invalid release-manifest.json: ${visibleError(error)}`);
    return null;
  }
}

function validateManifest(
  manifest: ReleaseManifest,
  files: readonly ReleaseFileEntry[],
  failures: string[],
): void {
  if (manifest.product !== RELEASE_PRODUCT) failures.push(`unexpected product: ${String(manifest.product)}`);
  if (manifest.schemaVersion !== RELEASE_SCHEMA_VERSION) {
    failures.push(`unexpected manifest schemaVersion: ${String(manifest.schemaVersion)}`);
  }
  if (!/^\d+$/.test(manifest.supportedSteamBuild)) failures.push("supportedSteamBuild is not authoritative numeric data");
  if (!/^[a-f0-9]{64}$/.test(manifest.sharedSnapshotSha256)) failures.push("invalid sharedSnapshotSha256");
  if (!/^engine-rust-tree:[a-f0-9]{40}$/.test(manifest.rulesetRevision)) failures.push("invalid rulesetRevision");
  if (!/^[a-f0-9]{7,64}$/.test(manifest.appCommit)) failures.push("invalid appCommit");
  if (
    manifest.fixturePolicy?.bundledFixtureCount !== 0 ||
    manifest.fixturePolicy?.catalog !== "empty" ||
    manifest.fixturePolicy?.remoteFetch !== false
  ) {
    failures.push("fixturePolicy must be zero-fixture, empty-catalog, and no-remote-fetch");
  }
  if (manifest.inventory?.algorithm !== "sha256") failures.push("manifest inventory algorithm must be sha256");

  const actual = files.filter((file) => file.path !== RELEASE_MANIFEST_FILENAME);
  const expected = manifest.inventory?.files ?? [];
  if (JSON.stringify(expected) !== JSON.stringify(actual)) {
    failures.push("manifest file inventory does not exactly match artifact files");
  }
}

function visibleError(error: unknown): string {
  return error instanceof Error ? error.message : String(error);
}

const NETWORK_API_NAMES = new Set([
  "fetch",
  "XMLHttpRequest",
  "WebSocket",
  "EventSource",
  "sendBeacon",
]);
const NETWORK_API_IDENTIFIER = /\b(?:fetch|XMLHttpRequest|WebSocket|EventSource|sendBeacon)\b/;

function hasForbiddenNetworkApi(source: string, path: string): boolean {
  let executable = executableJavaScript(source);
  if (/^assets\/workbench-worker\.[a-f0-9]{16}\.js$/u.test(path)) {
    const hasPinnedWasmUrl = /["'](?:\.\/|\/)?(?:assets\/)?yixian-engine\.[a-f0-9]{16}\.wasm["']/u
      .test(source);
    const fetchCount = executable.match(/\bfetch\b/gu)?.length ?? 0;
    if (hasPinnedWasmUrl && fetchCount === 1) {
      executable = executable.replace(/\bfetch\b/u, "");
    }
  }
  return NETWORK_API_IDENTIFIER.test(executable);
}

function executableJavaScript(source: string): string {
  const output: string[] = [];
  let recentCode = "";

  function append(value: string): void {
    output.push(value);
    recentCode = `${recentCode}${value}`.slice(-160);
  }

  function scanCode(start: number, stopAtTemplateBrace: boolean): number {
    let index = start;
    let braceDepth = 0;
    while (index < source.length) {
      const character = source[index]!;
      const next = source[index + 1];
      if (character === '"' || character === "'") {
        const quoted = readQuotedString(index, character);
        append(isNetworkBracketProperty(quoted.value, quoted.end) ? ` ${quoted.value} ` : " ");
        index = quoted.end;
        continue;
      }
      if (character === "`") {
        append(" ");
        index = scanTemplate(index + 1);
        continue;
      }
      if (character === "/" && next === "/") {
        append(" ");
        index = skipLineComment(index + 2);
        continue;
      }
      if (character === "/" && next === "*") {
        append(" ");
        index = skipBlockComment(index + 2);
        continue;
      }
      if (stopAtTemplateBrace && character === "{") {
        braceDepth += 1;
      } else if (stopAtTemplateBrace && character === "}") {
        if (braceDepth === 0) return index + 1;
        braceDepth -= 1;
      }
      append(character);
      index += 1;
    }
    return index;
  }

  function scanTemplate(start: number): number {
    let index = start;
    while (index < source.length) {
      const character = source[index]!;
      if (character === "\\") {
        index += 2;
        continue;
      }
      if (character === "`") return index + 1;
      if (character === "$" && source[index + 1] === "{") {
        append(" ");
        index = scanCode(index + 2, true);
        append(" ");
        continue;
      }
      index += 1;
    }
    return index;
  }

  function readQuotedString(
    start: number,
    quote: string,
  ): { readonly end: number; readonly value: string } {
    let index = start + 1;
    while (index < source.length) {
      if (source[index] === "\\") {
        index += 2;
        continue;
      }
      if (source[index] === quote) {
        return {
          end: index + 1,
          value: source.slice(start + 1, index),
        };
      }
      index += 1;
    }
    return { end: index, value: source.slice(start + 1) };
  }

  function isNetworkBracketProperty(value: string, end: number): boolean {
    return NETWORK_API_NAMES.has(value) &&
      /(?:^|[^\w$])(?:globalThis|window|self|navigator)\s*\[\s*$/.test(recentCode) &&
      /^\s*\]/.test(source.slice(end));
  }

  function skipLineComment(start: number): number {
    const lineEnd = source.indexOf("\n", start);
    return lineEnd === -1 ? source.length : lineEnd + 1;
  }

  function skipBlockComment(start: number): number {
    const commentEnd = source.indexOf("*/", start);
    return commentEnd === -1 ? source.length : commentEnd + 2;
  }

  scanCode(0, false);
  return output.join("");
}
