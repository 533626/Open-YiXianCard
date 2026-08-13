import { existsSync } from "node:fs";
import { lstat, readFile } from "node:fs/promises";
import { join, resolve } from "node:path";

const repoRoot = resolve(import.meta.dir, "..");
const forbiddenRoots = [
  "analysis",
  "battle-evaluator",
  "engine-ts",
  "research",
  "engine-rust/src/bin/tui.rs",
  "engine-rust/src/bin/tui_app",
  "engine-rust/tui-builds",
  "engine-rust/tests/replay_slice_fail_closed.rs",
] as const;
const requiredFiles = [
  "engine-rust/Cargo.toml",
  "engine-rust/data/card-effect-catalog.json",
  "src/ui/generated/fixture-index.json",
  "scripts/check-public-ui-card-catalog.ts",
] as const;

const failures: string[] = [];
for (const path of forbiddenRoots) {
  if (existsSync(join(repoRoot, path))) failures.push(`forbidden private surface exists: ${path}`);
}
for (const path of requiredFiles) {
  const absolute = join(repoRoot, path);
  const stat = await lstat(absolute).catch(() => null);
  if (stat === null || !stat.isFile()) failures.push(`required public file is missing or not regular: ${path}`);
}
if (failures.length === 0) {
  const catalog = await readFile(join(repoRoot, "src/ui/generated/fixture-index.json"), "utf8");
  if (catalog !== "[]\n") failures.push("public UI fixture catalog must be exactly []\\n");
  const cargoManifest = await readFile(join(repoRoot, "engine-rust/Cargo.toml"), "utf8");
  const cargoLock = await readFile(join(repoRoot, "engine-rust/Cargo.lock"), "utf8");
  for (const dependency of ["ratatui", "crossterm"]) {
    if (cargoManifest.includes(dependency) || cargoLock.includes(`name = "${dependency}"`)) {
      failures.push(`private TUI dependency remains in public Rust package: ${dependency}`);
    }
  }
}

if (failures.length > 0) {
  console.error("public surface check failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}
console.log("public surface check passed: Rust engine + browser UI only; fixture catalog empty");
