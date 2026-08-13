import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { CARD_OPTIONS } from "../src/ui/data/cards";

type RustCatalog = { readonly executableBaseIds?: readonly number[] };
const repoRoot = join(import.meta.dir, "..");
const catalog = JSON.parse(
  await readFile(join(repoRoot, "engine-rust/data/card-effect-catalog.json"), "utf8"),
) as RustCatalog;
const rustIds = [...new Set(catalog.executableBaseIds ?? [])].sort((a, b) => a - b);
const uiIds = CARD_OPTIONS.map((card) => card.baseId);
const duplicateUiIds = uiIds.filter((id, index) => uiIds.indexOf(id) !== index);
const rustSet = new Set(rustIds);
const missingFromRust = [...new Set(uiIds)].filter((id) => !rustSet.has(id)).sort((a, b) => a - b);

const failures: string[] = [];
if (rustIds.length === 0) failures.push("Rust card-effect catalog is empty");
if (duplicateUiIds.length > 0) failures.push(`UI card options contain duplicate base IDs: ${[...new Set(duplicateUiIds)].join(", ")}`);
if (missingFromRust.length > 0) failures.push(`UI card options are not executable in Rust: ${missingFromRust.join(", ")}`);
if (failures.length > 0) {
  console.error("public Rust → UI card catalog check failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}
console.log(`public Rust → UI card catalog check passed (${uiIds.length} normalized IDs; Rust catalog ${rustIds.length})`);
