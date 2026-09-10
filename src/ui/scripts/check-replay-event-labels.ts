// Replay UI label preflight for one admission batch.
//
// Usage:
//   bun src/ui/scripts/check-replay-event-labels.ts --root <fixture-root>
//
// For every fixture under the root it parses with the shipped fixture
// contract, replays it through the canonical Rust engine in strict
// admission mode (winner/actorTurn/hpDelta must be exact), then resolves
// every referenced card/talent/fate-strategy id through the real UI label
// authorities. Any parse failure, non-exact engine result, engine warning,
// or unmapped id fails the gate.
import { readdir, readFile } from "node:fs/promises";
import { join, relative, resolve } from "node:path";
import { isSourceMapped } from "../battle-event-hooks";
import { normalizeBaseId } from "../domain";
import { parseReplayFixtureJson } from "../fixture-consistency";

const repoRoot = join(import.meta.dir, "../../..");
const replayBinary = join(repoRoot, "engine-rust", "target", "debug", "replay_slice");
const fixtureRoot = parseFixtureRoot(process.argv.slice(2));

const fixturePaths = await collectJsonFiles(fixtureRoot);
if (fixturePaths.length === 0) throw new Error(`no replay fixtures found under ${fixtureRoot}`);

const errors: Array<{ fixtureId: string; message: string }> = [];
const cardIds = new Set<number>();
const talentIds = new Set<number>();
const fateStrategyIds = new Set<number>();

for (const path of fixturePaths) {
  const fixtureId = relative(fixtureRoot, path).replaceAll("\\", "/").replace(/\.json$/, "");
  try {
    const fixture = parseReplayFixtureJson(await readFile(path, "utf8"));
    for (const side of ["p1", "p2"] as const) {
      const player = fixture.players[side];
      for (const card of player.cards) cardIds.add(card.id);
      for (const card of player.handCards ?? []) cardIds.add(card);
      for (const talent of player.talents ?? []) talentIds.add(talent);
      for (const fate of player.fateStrategies ?? []) fateStrategyIds.add(fate);
    }
  } catch (error) {
    errors.push({ fixtureId, message: String(error instanceof Error ? error.message : error) });
  }
}

await ensureReplayBinary();
try {
  const admission = await runReplayAdmission();
  for (const result of admission.results ?? []) {
    if (result.result !== "exact") {
      errors.push({
        fixtureId: String(result.fixture ?? "unknown"),
        message: `engine ${String(result.result)}: winner=${String(result.actualWinner)} turn=${String(result.actualTurn)} delta=${String(result.actualDelta)}`,
      });
    }
    for (const warning of result.warnings ?? []) {
      errors.push({ fixtureId: String(result.fixture ?? "unknown"), message: `warning: ${String(warning)}` });
    }
    for (const card of result.missingCards ?? []) {
      errors.push({ fixtureId: String(result.fixture ?? "unknown"), message: `missing card: ${JSON.stringify(card)}` });
    }
  }
} catch (error) {
  errors.push({ fixtureId: "*", message: `replay_slice --admission failed: ${shortError(error)}` });
}

let eventCount = 0;
try {
  const batchEvents = await runReplayEvents();
  for (const line of batchEvents) {
    for (const event of line.events ?? []) {
      eventCount += 1;
      if (typeof event.card_id === "number") cardIds.add(event.card_id);
    }
  }
} catch (error) {
  errors.push({ fixtureId: "*", message: `replay_slice --events-batch failed: ${shortError(error)}` });
}

const unmappedSources = new Map<string, string>();
for (const id of [...cardIds].sort((a, b) => a - b)) {
  // Route through the same card:<id> identity the UI hooks use, so the
  // archive authority always matches production (deck-picker options are a
  // narrower subset and must not be used here).
  if (!isSourceMapped(`card:${normalizeBaseId(id)}`)) unmappedSources.set(`card:${id}`, "card archive");
}
for (const id of [...talentIds].sort((a, b) => a - b)) {
  if (!isSourceMapped(`talent:${id}`)) unmappedSources.set(`talent:${id}`, "talent labels");
}
for (const id of [...fateStrategyIds].sort((a, b) => a - b)) {
  if (!isSourceMapped(`fateStrategy:${id}`)) unmappedSources.set(`fateStrategy:${id}`, "fate-strategy labels");
}

console.log([
  "Replay UI label preflight",
  `fixtures=${fixturePaths.length}`,
  `events=${eventCount}`,
  `errors=${errors.length}`,
  `unmappedSources=${unmappedSources.size}`,
].join(" "));

for (const error of errors.slice(0, 20)) {
  console.log(`error ${error.fixtureId}: ${error.message}`);
}
for (const [source, authority] of [...unmappedSources].slice(0, 20)) {
  console.log(`unmapped source ${source} (authority: ${authority})`);
}

if (errors.length > 0 || unmappedSources.size > 0) process.exitCode = 1;

function shortError(error: unknown): string {
  const message = String(error instanceof Error ? error.message : error);
  return message.split("\n")[0]!.slice(0, 300);
}

function parseFixtureRoot(args: readonly string[]): string {
  if (args.length !== 2 || args[0] !== "--root" || !args[1]) {
    throw new Error("usage: bun src/ui/scripts/check-replay-event-labels.ts --root <fixture-root>");
  }
  return resolve(repoRoot, args[1]);
}

async function collectJsonFiles(root: string): Promise<string[]> {
  const entries = await readdir(root, { withFileTypes: true });
  const paths: string[] = [];
  for (const entry of entries) {
    const path = join(root, entry.name);
    if (entry.isDirectory()) paths.push(...await collectJsonFiles(path));
    else if (entry.isFile() && entry.name.endsWith(".json")) paths.push(path);
  }
  return paths.sort();
}

async function ensureReplayBinary(): Promise<void> {
  if (await Bun.file(replayBinary).exists()) return;
  await runCommand(["cargo", "build", "--manifest-path", join(repoRoot, "engine-rust", "Cargo.toml"), "--bin", "replay_slice"]);
}

interface AdmissionResult {
  readonly fixture?: unknown;
  readonly result?: unknown;
  readonly actualWinner?: unknown;
  readonly actualTurn?: unknown;
  readonly actualDelta?: unknown;
  readonly warnings?: readonly unknown[];
  readonly missingCards?: readonly unknown[];
}

async function runReplayAdmission(): Promise<{ results?: readonly AdmissionResult[] }> {
  const output = await runCommand([replayBinary, "--admission", fixtureRoot]);
  return JSON.parse(output) as { results?: Array<Record<string, unknown>> };
}

async function runReplayEvents(): Promise<Array<{ events?: Array<{ card_id?: unknown }> }>> {
  const output = await runCommand([replayBinary, "--events-batch", fixtureRoot]);
  return output.split("\n").filter((line) => line.trim().length > 0)
    .map((line) => JSON.parse(line) as { events?: Array<{ card_id?: unknown }> });
}

async function runCommand(argv: readonly string[]): Promise<string> {
  const child = Bun.spawn([...argv], { cwd: repoRoot, stdout: "pipe", stderr: "pipe" });
  const [exitCode, stdout, stderr] = await Promise.all([
    child.exited,
    new Response(child.stdout).text(),
    new Response(child.stderr).text(),
  ]);
  if (exitCode !== 0) throw new Error(`${argv.join(" ")} exited ${exitCode}: ${stderr.slice(0, 500)}`);
  return stdout;
}
