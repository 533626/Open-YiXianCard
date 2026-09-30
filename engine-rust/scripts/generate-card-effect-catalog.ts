// Rust card-effect catalog（data/card-effect-catalog.json）是 Rust 可执行卡牌的审计清单，
// 由人随 Card_* handler 增删维护；Rust 单测按 handler 双向校验。本脚本只做规范化
// （去重、归一化 base id、升序），--check 校验文件已是规范形。
import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const engineRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputPath = join(engineRoot, "data/card-effect-catalog.json");

function normalizeBaseId(cardId: number): number {
  if (cardId === 0 || cardId === 10_000 || cardId === 20_000) return 0;
  return cardId - Math.trunc((cardId % 1_000_000) / 10_000) * 10_000;
}

const current = await readFile(outputPath, "utf8");
const parsed = JSON.parse(current) as { schemaVersion?: unknown; executableBaseIds?: unknown };
if (
  parsed.schemaVersion !== 1 ||
  !Array.isArray(parsed.executableBaseIds) ||
  parsed.executableBaseIds.length === 0 ||
  !parsed.executableBaseIds.every((id) => Number.isInteger(id))
) {
  throw new Error("Rust card-effect catalog is malformed");
}
const executableBaseIds = [...new Set((parsed.executableBaseIds as number[]).map(normalizeBaseId))]
  .sort((left, right) => left - right);

const output = `${JSON.stringify(
  {
    schemaVersion: 1,
    source:
      "Audited Rust card-effect dispatch; normalized base IDs verified bidirectionally against Rust handlers",
    executableBaseIds,
  },
  null,
  2,
)}\n`;

if (process.argv.includes("--check")) {
  if (current !== output) {
    throw new Error(
      "Rust card-effect catalog is not normalized; run bun engine-rust/scripts/generate-card-effect-catalog.ts",
    );
  }
} else {
  await writeFile(outputPath, output);
}
