// 引擎 PGO（配置文件引导优化）构建：插桩构建 → 跑训练负载 → 合并 profile → 优化构建 → 拷入 target/release。
//
// 训练负载只用私有开发树入库的回放语料（candidates 目录：replay_slice 严格回放 + solve_deck_batch
// 常驻批评估），与任何上层策略无关；整局仿真录流上实测 ~1.4×（只用 candidates 训练与另加 external 训练持平）。
// PGO 只影响代码布局 / 内联 / 分支预测，不改语义：产物须与普通构建逐场一致（脚本末尾跑 strict 复核）。
//
// 产物拷进 target/release/：源码不变时普通 `cargo build --release` 视为最新、不覆盖；源码一变就按普通构建重编
// （规则永远是最新源码，只是暂时失去 PGO，重跑本脚本即可）。
//
//   bun run build:engine:pgo                      # solve_deck_batch / replay_slice / solve_deck
//   bun run build:engine:pgo -- --extra <dir>     # 另加一份回放语料目录参与训练（如 fixtures/external）
import { copyFile, mkdir, readFile, readdir, rm, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..", "..");
const engineRoot = join(repoRoot, "engine-rust");
const pgoRoot = join(engineRoot, "target", "pgo");
const profileDir = join(pgoRoot, "raw");
const mergedProfile = join(pgoRoot, "merged.profdata");
const BINS = ["solve_deck_batch", "replay_slice", "solve_deck"];
const candidates = join(repoRoot, "battle-evaluator", "fixtures", "candidates");

const args = process.argv.slice(2);
const extra: string[] = [];
for (let i = 0; i < args.length; i += 1) {
  if (args[i] === "--extra" && args[i + 1]) extra.push(args[(i += 1)]);
}

function run(cmd: string[], opts: { env?: Record<string, string>; stdin?: string } = {}): string {
  const proc = Bun.spawnSync(cmd, {
    cwd: repoRoot,
    env: { ...process.env, ...opts.env },
    stdin: opts.stdin ? Buffer.from(opts.stdin) : "ignore",
    stdout: "pipe",
    stderr: "inherit",
  });
  if (proc.exitCode !== 0) throw new Error(`命令失败（${proc.exitCode}）：${cmd.join(" ")}`);
  return proc.stdout.toString();
}

function llvmProfdata(): string {
  const sysroot = run(["rustc", "--print", "sysroot"]).trim();
  const host = /host: (\S+)/.exec(run(["rustc", "-vV"]))?.[1];
  const path = join(sysroot, "lib", "rustlib", host ?? "", "bin", "llvm-profdata");
  if (!existsSync(path)) {
    throw new Error(`缺 llvm-profdata：先执行 \`rustup component add llvm-tools\`（查找路径 ${path}）`);
  }
  return path;
}

function cargoBuild(targetDir: string, rustflags: string): void {
  const binArgs = BINS.flatMap((bin) => ["--bin", bin]);
  run(["cargo", "build", "--release", "--manifest-path", join(engineRoot, "Cargo.toml"), ...binArgs,
    "--target-dir", targetDir], { env: { RUSTFLAGS: rustflags } });
}

async function fixtureFiles(root: string): Promise<string[]> {
  const out: string[] = [];
  for (const entry of await readdir(root, { withFileTypes: true, recursive: true })) {
    if (entry.isFile() && entry.name.endsWith(".json")) out.push(join(entry.parentPath, entry.name));
  }
  return out.sort();
}

/** candidates 语料 → solve_deck_batch 常驻模式请求（整份 fixture + 我方现阵为卡组，compact 回复），练批服务路径。 */
async function batchTrainingInput(): Promise<string> {
  const lines: string[] = [];
  let batch: unknown[] = [];
  for (const path of await fixtureFiles(candidates)) {
    const fixture = JSON.parse(await readFile(path, "utf8"));
    const ids: number[] = (fixture?.players?.p1?.cards ?? []).map((card: { id: number }) => Number(card.id));
    if (!ids.length) continue;
    const deck = [...ids, ...Array(8).fill(ids[0])].slice(0, 8);
    batch.push({ id: String(batch.length), fixture, side: "p1", scoreProfile: "hpDelta", deck, battleSeeds: [42] });
    if (batch.length === 64) {
      lines.push(JSON.stringify({ requests: batch, compact: true }));
      batch = [];
    }
  }
  if (batch.length) lines.push(JSON.stringify({ requests: batch, compact: true }));
  return lines.join("\n") + "\n";
}

const profdata = llvmProfdata();
await rm(profileDir, { recursive: true, force: true });
await mkdir(profileDir, { recursive: true });
const genDir = join(pgoRoot, "gen");
const useDir = join(pgoRoot, "use");

console.log("1/4 插桩构建");
cargoBuild(genDir, `-Cprofile-generate=${profileDir}`);
console.log("2/4 训练负载：candidates 严格回放 + 批评估" + (extra.length ? ` + ${extra.join(", ")}` : ""));
const gen = (bin: string) => join(genDir, "release", bin);
run([gen("replay_slice"), "--strict", candidates]);
for (const dir of extra) run([gen("replay_slice"), "--admission", dir]);
run([gen("solve_deck_batch"), "--serve"], { env: { SOLVE_DECK_BATCH_THREADS: "1" }, stdin: await batchTrainingInput() });
console.log("3/4 合并 profile");
run([profdata, "merge", "-o", mergedProfile, profileDir]);
console.log("4/4 优化构建");
// 训练集没覆盖到的函数不警告（profile 只是布局提示，缺了照常编译）
cargoBuild(useDir, `-Cprofile-use=${mergedProfile} -Cllvm-args=-pgo-warn-missing-function=false`);
const release = join(engineRoot, "target", "release");
await mkdir(release, { recursive: true });
for (const bin of BINS) await copyFile(join(useDir, "release", bin), join(release, bin));

const strict = JSON.parse(run([join(release, "replay_slice"), "--strict", candidates]));
if (strict.total !== strict.exact) throw new Error(`PGO 产物 strict 复核失败：${strict.exact}/${strict.total}`);
await writeFile(join(pgoRoot, "BUILT"), `${new Date().toISOString()} strict ${strict.exact}/${strict.total}\n`);
console.log(`完成：${BINS.join(" / ")} → engine-rust/target/release（strict ${strict.exact}/${strict.total} exact）`);
