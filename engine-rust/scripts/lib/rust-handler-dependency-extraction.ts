import { readdir, readFile } from "node:fs/promises";
import { join, relative } from "node:path";

export interface RustSourceFile {
  readonly path: string;
  readonly content: string;
}

export interface RustSourceLocation {
  readonly path: string;
  readonly line: number;
  readonly function: string;
}

export interface RustHandlerRoute extends RustSourceLocation {
  readonly kind: "typed" | "fallback";
}

export interface RustHandlerDependencyIndex {
  readonly schemaVersion: "rust-static-handler-dependencies-v1";
  readonly source: string;
  readonly policy: string;
  readonly lifecycle: readonly {
    readonly id: string;
    readonly label: string;
    readonly location: RustSourceLocation;
  }[];
  readonly handlers: readonly {
    readonly baseId: number;
    readonly routes: readonly RustHandlerRoute[];
  }[];
  readonly totals: {
    readonly handlerRoutes: number;
    readonly routedCards: number;
  };
}

interface RustFunction extends RustSourceLocation {
  readonly body: string;
}

const LIFECYCLE_FUNCTIONS = [
  ["catalog-resolution", "Catalog 预检", "flow_card_effect.rs", "resolve_card_effect_before_execution"],
  ["before-execute", "出牌前钩子", "flow_card_effect.rs", "apply_before_execute_effect_hooks"],
  ["body", "卡牌主体 dispatch", "flow_card_effect.rs", "apply_card_effect_body"],
  ["printed-effects", "配置印刷字段", "flow_card_effect.rs", "apply_regular_printed_card_effects"],
  ["action-again", "再动判定", "action_again.rs", "resolve_card_action_again"],
  ["after-effect", "牌后钩子", "flow_card_effect.rs", "apply_regular_after_card_effect_hooks"],
  ["completed-hooks", "完成钩子", "elements_late.rs", "apply_card_completed_hooks"],
  ["completion", "完成态结算", "flow.rs", "complete_card_effect_repetition"],
] as const;

/** Static source scan only. It neither executes cards nor claims semantic proof. */
export async function extractRustHandlerDependencyIndex(engineRoot: string): Promise<RustHandlerDependencyIndex> {
  const replayRoot = join(engineRoot, "src", "replay");
  const sourceFiles = await readRustSources(engineRoot, replayRoot);
  return extractRustHandlerDependencyIndexFromSources(sourceFiles);
}

export function extractRustHandlerDependencyIndexFromSources(
  sourceFiles: readonly RustSourceFile[],
): RustHandlerDependencyIndex {
  const functions = sourceFiles.flatMap((source) => extractRustFunctions(source));
  const lifecycle = LIFECYCLE_FUNCTIONS.map(([id, label, fileName, functionName]) => {
    const location = functions.find((entry) =>
      entry.path.endsWith(`/replay/${fileName}`) && entry.function === functionName
    );
    if (!location) {
      throw new Error(`static lifecycle source moved or missing: ${fileName}::${functionName}`);
    }
    return {
      id,
      label,
      location: toLocation(location),
    };
  });
  const routesByCard = new Map<number, RustHandlerRoute[]>();
  for (const functionEntry of functions) {
    const kind = handlerKind(functionEntry.function);
    if (!kind) continue;
    for (const baseId of extractMatchBaseIds(functionEntry.body)) {
      const routes = routesByCard.get(baseId) ?? [];
      routes.push({ ...toLocation(functionEntry), kind });
      routesByCard.set(baseId, routes);
    }
  }
  const handlers = [...routesByCard.entries()]
    .map(([baseId, routes]) => ({
      baseId,
      routes: [...dedupeRoutes(routes)].sort(compareRoutes),
    }))
    .sort((left, right) => left.baseId - right.baseId);

  return {
    schemaVersion: "rust-static-handler-dependencies-v1",
    source: "Static scan of Rust replay card-dispatch match arms. Each route is a source location, not original-game evidence or a runtime call trace.",
    policy: "Lifecycle stages are verified by named Rust functions. Handler routes cover direct numeric match arms only; unrouted cards remain explicit rather than inferred.",
    lifecycle,
    handlers,
    totals: {
      handlerRoutes: handlers.reduce((count, handler) => count + handler.routes.length, 0),
      routedCards: handlers.length,
    },
  };
}

export function extractRustFunctions(source: RustSourceFile): readonly RustFunction[] {
  const functions: RustFunction[] = [];
  const matcher = /\bfn\s+([A-Za-z_][A-Za-z0-9_]*)\s*\(/g;
  for (const match of source.content.matchAll(matcher)) {
    const functionName = match[1]!;
    const signatureStart = match.index!;
    const bodyStart = source.content.indexOf("{", signatureStart);
    if (bodyStart < 0) continue;
    const bodyEnd = matchingBrace(source.content, bodyStart);
    if (bodyEnd < 0) continue;
    functions.push({
      path: source.path,
      line: lineForOffset(source.content, signatureStart),
      function: functionName,
      body: source.content.slice(bodyStart + 1, bodyEnd),
    });
  }
  return functions;
}

export function extractMatchBaseIds(functionBody: string): readonly number[] {
  const ids = new Set<number>();
  const matcher = /\bmatch\s+(?:base_id|normalized_base_id\([^)]*\))\s*\{/g;
  for (const match of functionBody.matchAll(matcher)) {
    if (braceDepthAt(functionBody, match.index!) !== 0) continue;
    const bodyStart = match.index! + match[0].length - 1;
    const bodyEnd = matchingBrace(functionBody, bodyStart);
    if (bodyEnd < 0) continue;
    for (const pattern of topLevelMatchPatterns(functionBody.slice(bodyStart + 1, bodyEnd))) {
      if (!/^\s*\d[\d_]*(?:\s*\|\s*\d[\d_]*)*\s*$/.test(pattern)) continue;
      for (const token of pattern.match(/\d[\d_]*/g) ?? []) {
        ids.add(Number(token.replaceAll("_", "")));
      }
    }
  }
  return [...ids].sort((left, right) => left - right);
}

async function readRustSources(engineRoot: string, replayRoot: string): Promise<readonly RustSourceFile[]> {
  const paths = await collectRustPaths(replayRoot);
  return Promise.all(paths
    .filter((path) => !/\/(?:tests(?:_|\.rs)|test_support\.rs)/.test(path))
    .map(async (path) => ({
      path: relative(engineRoot, path).replaceAll("\\", "/"),
      content: await readFile(path, "utf8"),
    })));
}

async function collectRustPaths(root: string): Promise<readonly string[]> {
  const entries = await readdir(root, { withFileTypes: true });
  const nested = await Promise.all(entries.map(async (entry) => {
    const path = join(root, entry.name);
    if (entry.isDirectory()) return collectRustPaths(path);
    return entry.isFile() && path.endsWith(".rs") ? [path] : [];
  }));
  return nested.flat();
}

function handlerKind(functionName: string): RustHandlerRoute["kind"] | undefined {
  if (functionName === "apply_card_effect_fallback") return "fallback";
  return functionName.startsWith("apply_") && functionName.includes("card_effect")
    ? "typed"
    : undefined;
}

function topLevelMatchPatterns(matchBody: string): readonly string[] {
  const outerArms = hideNestedBlocks(matchBody);
  return [...outerArms.matchAll(/(?:^|\n)\s*(\d[\d_]*(?:\s*\|\s*\d[\d_]*)*)\s*=>/g)]
    .map((match) => match[1]!);
}

function hideNestedBlocks(source: string): string {
  let depth = 0;
  let result = "";
  for (let index = 0; index < source.length; index += 1) {
    const next = skipRustLiteralOrComment(source, index);
    if (next > index) {
      result += source.slice(index, next).replace(/[^\n]/g, " ");
      index = next - 1;
      continue;
    }
    const char = source[index]!;
    if (char === "{") {
      depth += 1;
      result += " ";
      continue;
    }
    if (char === "}") {
      depth = Math.max(0, depth - 1);
      result += " ";
      continue;
    }
    result += depth === 0 ? char : (char === "\n" ? "\n" : " ");
  }
  return result;
}

function matchingBrace(source: string, openingOffset: number): number {
  let depth = 0;
  for (let index = openingOffset; index < source.length; index += 1) {
    const next = skipRustLiteralOrComment(source, index);
    if (next > index) {
      index = next - 1;
      continue;
    }
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") {
      depth -= 1;
      if (depth === 0) return index;
    }
  }
  return -1;
}

function braceDepthAt(source: string, until: number): number {
  let depth = 0;
  for (let index = 0; index < until; index += 1) {
    const next = skipRustLiteralOrComment(source, index);
    if (next > index) {
      index = next - 1;
      continue;
    }
    if (source[index] === "{") depth += 1;
    if (source[index] === "}") depth = Math.max(0, depth - 1);
  }
  return depth;
}

function skipRustLiteralOrComment(source: string, index: number): number {
  if (source.startsWith("//", index)) {
    const end = source.indexOf("\n", index + 2);
    return end < 0 ? source.length : end;
  }
  if (source.startsWith("/*", index)) {
    const end = source.indexOf("*/", index + 2);
    return end < 0 ? source.length : end + 2;
  }
  const quote = source[index];
  if (quote !== "\"" && quote !== "'") return index;
  let cursor = index + 1;
  while (cursor < source.length) {
    if (source[cursor] === "\\") {
      cursor += 2;
      continue;
    }
    if (source[cursor] === quote) return cursor + 1;
    cursor += 1;
  }
  return source.length;
}

function lineForOffset(source: string, offset: number): number {
  return source.slice(0, offset).split("\n").length;
}

function toLocation(functionEntry: RustFunction): RustSourceLocation {
  return {
    path: functionEntry.path,
    line: functionEntry.line,
    function: functionEntry.function,
  };
}

function dedupeRoutes(routes: readonly RustHandlerRoute[]): readonly RustHandlerRoute[] {
  return [...new Map(routes.map((route) => [
    `${route.kind}:${route.path}:${route.line}:${route.function}`,
    route,
  ])).values()];
}

function compareRoutes(left: RustHandlerRoute, right: RustHandlerRoute): number {
  return left.path.localeCompare(right.path)
    || left.line - right.line
    || left.function.localeCompare(right.function);
}
