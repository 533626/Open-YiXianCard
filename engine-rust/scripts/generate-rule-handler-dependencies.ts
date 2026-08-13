import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";
import { extractRustHandlerDependencyIndex } from "./lib/rust-handler-dependency-extraction";

const engineRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const outputPath = join(engineRoot, "data", "rule-handler-dependencies.json");
const index = await extractRustHandlerDependencyIndex(engineRoot);
const output = `${JSON.stringify(index, null, 2)}\n`;

if (process.argv.includes("--check")) {
  const current = await readFile(outputPath, "utf8");
  if (current !== output) {
    throw new Error(
      "Rust static handler dependency index is stale; run bun engine-rust/scripts/generate-rule-handler-dependencies.ts",
    );
  }
} else {
  await writeFile(outputPath, output);
}
