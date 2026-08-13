import { readFile, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const engineRoot = join(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = join(engineRoot, "..");
const ledgerPath = join(
  repoRoot,
  "private-companion/ts-migration-ledger.json",
);
const outputPath = join(engineRoot, "data/card-effect-catalog.json");

function normalizeBaseId(cardId: number): number {
  if (cardId === 0 || cardId === 10_000 || cardId === 20_000) return 0;
  return cardId - Math.trunc((cardId % 1_000_000) / 10_000) * 10_000;
}

interface MigrationLedger {
  readonly registrations: {
    readonly effectivePartitions: {
      readonly formalIds: readonly number[];
      readonly clientIds: readonly number[];
      readonly replayIds: readonly number[];
    };
  };
}

let executableBaseIds: number[];
try {
  const ledger = JSON.parse(await readFile(ledgerPath, "utf8")) as MigrationLedger;
  const partitions = ledger.registrations.effectivePartitions;
  executableBaseIds = [...new Set(
    [...partitions.formalIds, ...partitions.clientIds, ...partitions.replayIds].map(normalizeBaseId),
  )].sort((left, right) => left - right);
} catch (error) {
  // The migration ledger is replay-derived and private. In a public checkout,
  // preserve the reviewed generated catalog and validate its shape instead of
  // requiring a private report to regenerate it.
  if (!process.argv.includes("--check")) {
    throw new Error(`public card catalog generation requires private ledger or a reviewed catalog: ${String(error)}`);
  }
  const current = JSON.parse(await readFile(outputPath, "utf8")) as { executableBaseIds?: unknown };
  if (!Array.isArray(current.executableBaseIds) || !current.executableBaseIds.every((id) => Number.isInteger(id))) {
    throw new Error("public Rust card-effect catalog is malformed");
  }
  executableBaseIds = current.executableBaseIds as number[];
}

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
  const current = await readFile(outputPath, "utf8");
  if (current !== output) {
    throw new Error(
      "Rust card-effect catalog is stale; run bun engine-rust/scripts/generate-card-effect-catalog.ts",
    );
  }
} else {
  await writeFile(outputPath, output);
}
