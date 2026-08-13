import { readFile } from "node:fs/promises";
import { join } from "node:path";

export async function buildRustWasm(repoRoot: string): Promise<Uint8Array> {
  const command = [
    "cargo",
    "build",
    "--release",
    "--target",
    "wasm32-unknown-unknown",
    "--manifest-path",
    "engine-rust/Cargo.toml",
    "--lib",
  ] as const;
  const child = Bun.spawn([...command], {
    cwd: repoRoot,
    stdout: "inherit",
    stderr: "inherit",
  });
  const exitCode = await child.exited;
  if (exitCode !== 0) {
    throw new Error(`Rust/WASM build failed (${exitCode}): ${command.join(" ")}`);
  }
  return new Uint8Array(await readFile(join(
    repoRoot,
    "engine-rust",
    "target",
    "wasm32-unknown-unknown",
    "release",
    "yixian_engine.wasm",
  )));
}
