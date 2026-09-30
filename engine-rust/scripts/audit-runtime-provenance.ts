import { readFile, readdir } from "node:fs/promises";
import { basename, join, relative, sep } from "node:path";

const repoRoot = join(import.meta.dir, "..", "..");
// 运行时命名审计只针对 Rust canonical 引擎源码：禁止把采集批次号（wave-NNN）写进运行时路径与标识符。
const runtimeRoots = [
  { root: join(repoRoot, "engine-rust", "src") },
];

const batchName = /waves?[-_]?\d{3}/i;
const batchIdentifier = /waves?_?\d{3}/i;

interface Violation {
  readonly path: string;
  readonly line?: number;
  readonly column?: number;
  readonly kind: "file path" | "import path" | "identifier";
  readonly value: string;
}

const violations: Violation[] = [];

for (const runtime of runtimeRoots) {
  for (const path of await walk(runtime.root, ".rs")) {
    const repoPath = relative(repoRoot, path);
    if (isTestFile(repoPath)) continue;

    if (batchName.test(repoPath)) {
      violations.push({ path: repoPath, kind: "file path", value: repoPath });
    }

    const source = await readFile(path, "utf8");
    auditRust(repoPath, source);
  }
}

violations.sort((left, right) =>
  left.path.localeCompare(right.path) ||
  (left.line ?? 0) - (right.line ?? 0) ||
  (left.column ?? 0) - (right.column ?? 0) ||
  left.kind.localeCompare(right.kind) ||
  left.value.localeCompare(right.value),
);

if (violations.length > 0) {
  console.error("runtime provenance naming violations:");
  for (const violation of violations) {
    const location = violation.line === undefined
      ? violation.path
      : `${violation.path}:${violation.line}:${violation.column}`;
    console.error(`- ${location}: ${violation.kind} uses evidence batch name: ${violation.value}`);
  }
  console.error(
    "Use mechanism/lifecycle names in runtime syntax; keep wave IDs only in tests, scripts, fixtures, generated evidence, comments, and evidence string values.",
  );
  process.exit(1);
}

console.log(
  "runtime provenance naming audit ok: Rust runtime paths and identifiers are mechanism-based",
);

function auditRust(path: string, source: string): void {
  const code = maskRustCfgTestItems(maskRustCommentsAndLiterals(source));
  const identifier = /\b[A-Za-z_][A-Za-z0-9_]*\b/g;
  for (const match of code.matchAll(identifier)) {
    const value = match[0];
    const offset = match.index;
    if (!batchIdentifier.test(value)) continue;
    const { line, column } = lineAndColumn(source, offset);
    violations.push({ path, line, column, kind: "identifier", value });
  }

  for (const match of source.matchAll(
    /#\s*\[\s*path\s*=\s*"([^"]+)"\s*\]|\binclude(?:_str|_bytes)?!\s*\(\s*"([^"]+)"\s*\)/g,
  )) {
    if (code[match.index] === " ") continue;
    const value = match[1] ?? match[2];
    if (!value || !batchName.test(value)) continue;
    const valueOffset = match.index + match[0].indexOf(value);
    const { line, column } = lineAndColumn(source, valueOffset);
    violations.push({ path, line, column, kind: "import path", value });
  }
}

function maskRustCommentsAndLiterals(source: string): string {
  const masked = source.split("");
  const mask = (start: number, end: number): void => {
    for (let index = start; index < end; index += 1) {
      if (masked[index] !== "\n" && masked[index] !== "\r") masked[index] = " ";
    }
  };

  let index = 0;
  while (index < source.length) {
    if (source.startsWith("//", index)) {
      const end = source.indexOf("\n", index + 2);
      const stop = end === -1 ? source.length : end;
      mask(index, stop);
      index = stop;
      continue;
    }
    if (source.startsWith("/*", index)) {
      let depth = 1;
      let cursor = index + 2;
      while (cursor < source.length && depth > 0) {
        if (source.startsWith("/*", cursor)) {
          depth += 1;
          cursor += 2;
        } else if (source.startsWith("*/", cursor)) {
          depth -= 1;
          cursor += 2;
        } else {
          cursor += 1;
        }
      }
      mask(index, cursor);
      index = cursor;
      continue;
    }

    const raw = source.slice(index).match(/^(?:b|c)?r(#{0,255})"/);
    if (raw) {
      const hashes = raw[1] ?? "";
      const closing = `"${hashes}`;
      const contentStart = index + raw[0].length;
      const closingIndex = source.indexOf(closing, contentStart);
      const end = closingIndex === -1
        ? source.length
        : closingIndex + closing.length;
      mask(index, end);
      index = end;
      continue;
    }

    if (source[index] === '"') {
      const end = findQuotedLiteralEnd(source, index, '"');
      mask(index, end);
      index = end;
      continue;
    }
    if (source[index] === "'") {
      const end = findRustCharLiteralEnd(source, index);
      if (end !== null) {
        mask(index, end);
        index = end;
        continue;
      }
    }
    index += 1;
  }
  return masked.join("");
}

function maskRustCfgTestItems(code: string): string {
  const masked = code.split("");
  const cfgTest = /#\s*\[\s*cfg\s*\([^\]]*\btest\b[^\]]*\)\s*\]/g;
  for (const match of code.matchAll(cfgTest)) {
    let cursor = match.index + match[0].length;
    while (cursor < code.length) {
      if (/\s/.test(code[cursor] ?? "")) {
        cursor += 1;
        continue;
      }
      if (code.startsWith("#[", cursor)) {
        const end = code.indexOf("]", cursor + 2);
        if (end === -1) break;
        cursor = end + 1;
        continue;
      }
      break;
    }

    let end = cursor;
    while (end < code.length && code[end] !== ";" && code[end] !== "{") end += 1;
    if (code[end] === ";") {
      end += 1;
    } else if (code[end] === "{") {
      let depth = 1;
      end += 1;
      while (end < code.length && depth > 0) {
        if (code[end] === "{") depth += 1;
        else if (code[end] === "}") depth -= 1;
        end += 1;
      }
    }
    for (let index = match.index; index < end; index += 1) {
      if (masked[index] !== "\n" && masked[index] !== "\r") masked[index] = " ";
    }
  }
  return masked.join("");
}

function findQuotedLiteralEnd(
  source: string,
  start: number,
  quote: '"' | "'",
): number {
  let cursor = start + 1;
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

function findRustCharLiteralEnd(source: string, start: number): number | null {
  let cursor = start + 1;
  if (source[cursor] === "\\") cursor += 2;
  else cursor += 1;
  if (source[cursor] !== "'") return null;
  return cursor + 1;
}

function addLocatedViolation(
  path: string,
  source: string,
  offset: number,
  kind: Violation["kind"],
  value: string,
): void {
  const location = lineAndColumn(source, offset);
  violations.push({
    path,
    line: location.line,
    column: location.column,
    kind,
    value,
  });
}

function lineAndColumn(source: string, offset: number): { line: number; column: number } {
  const before = source.slice(0, offset);
  const lastNewline = before.lastIndexOf("\n");
  return {
    line: before.split("\n").length,
    column: offset - lastNewline,
  };
}

function isTestFile(repoPath: string): boolean {
  const parts = repoPath.split(sep);
  const file = basename(repoPath);
  return parts.includes("__tests__") ||
    parts.includes("tests") ||
    /(?:^|[._-])(?:test|tests|spec)(?:[._-]|$)/.test(file);
}

async function walk(directory: string, extension: ".ts" | ".rs"): Promise<string[]> {
  const entries = await readdir(directory, { withFileTypes: true });
  const paths = await Promise.all(
    entries.map(async (entry) => {
      const path = join(directory, entry.name);
      if (entry.isDirectory()) return walk(path, extension);
      if (entry.isFile() && path.endsWith(extension)) return [path];
      return [];
    }),
  );
  return paths.flat();
}
