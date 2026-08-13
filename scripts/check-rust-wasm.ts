import { join } from "node:path";

interface RustWasmExports {
  readonly memory: WebAssembly.Memory;
  readonly yixian_alloc: (length: number) => number;
  readonly yixian_dealloc: (pointer: number, length: number) => void;
  readonly yixian_simulate_json: (pointer: number, length: number) => bigint;
  readonly yixian_solve_json: (pointer: number, length: number) => bigint;
  readonly yixian_explain_json: (pointer: number, length: number) => bigint;
  readonly yixian_counterfactual_json: (pointer: number, length: number) => bigint;
  readonly yixian_trace_json: (pointer: number, length: number) => bigint;
}

const repoRoot = join(import.meta.dir, "..");
const wasmPath = join(repoRoot, "engine-rust/target/wasm32-unknown-unknown/release/yixian_engine.wasm");
const fixture = JSON.stringify(publicSmokeFixture());
const { instance } = await WebAssembly.instantiate(await Bun.file(wasmPath).arrayBuffer(), {});
const engine = instance.exports as unknown as RustWasmExports;
const simulate = callJson(engine, "yixian_simulate_json", fixture);
const response = JSON.parse(simulate) as {
  readonly ok: boolean;
  readonly run?: { readonly summary: { readonly winnerSide: string }; readonly events: readonly { readonly kind: string }[] };
  readonly error?: string;
};
if (!response.ok || !response.run) throw new Error(`Rust/WASM smoke failed: ${response.error ?? "missing run"}`);
if (response.run.events.at(-1)?.kind !== "battleEnd") throw new Error("Rust/WASM smoke did not emit a terminal battleEnd event");
const parsedFixture = JSON.parse(fixture) as Record<string, unknown>;
const solveRequest = { fixture: parsedFixture, side: "p1", mode: "order", visitOrder: "canonical", visitSeed: 0, scoreProfile: "hpDelta", topN: 2, maxEvaluations: 20, battleSeeds: [1, 1, 1] };
const solveResponse = JSON.parse(callJson(engine, "yixian_solve_json", JSON.stringify(solveRequest))) as {
  readonly ok: boolean;
  readonly run?: { readonly evaluatedCount: number; readonly results: readonly unknown[]; readonly seedsUsed?: readonly number[] };
  readonly error?: string;
};
if (!solveResponse.ok || !solveResponse.run || solveResponse.run.evaluatedCount === 0) throw new Error(`Rust/WASM solver smoke failed: ${solveResponse.error ?? "empty result"}`);
if (JSON.stringify(solveResponse.run.seedsUsed) !== JSON.stringify([1])) throw new Error(`Rust/WASM solver did not sort/deduplicate battleSeeds: ${JSON.stringify(solveResponse.run.seedsUsed)}`);
assertSolverFailure({ ...solveRequest, battleSeeds: [] }, "battleSeeds must not be empty");
assertSolverFailure({ ...solveRequest, maxEvaluations: 0, battleSeeds: [1] }, "maxEvaluations must be greater than zero");
const explain = JSON.parse(callJson(engine, "yixian_explain_json", JSON.stringify({ fixture: parsedFixture, side: "p1" }))) as {
  readonly ok: boolean;
  readonly run?: { readonly schemaVersion: string; readonly auditDeltaForSide: number; readonly checkpoints: readonly unknown[]; readonly cards: readonly unknown[] };
  readonly error?: string;
};
if (!explain.ok || !explain.run) throw new Error(`Rust/WASM explain smoke failed: ${explain.error ?? "missing run"}`);
if (explain.run.schemaVersion !== "canonical-rule-impact-v1") throw new Error(`Rust/WASM explain returned ${explain.run.schemaVersion}`);
if (Math.abs(explain.run.auditDeltaForSide) > 0.000001) throw new Error(`Rust/WASM explain audit drifted: ${explain.run.auditDeltaForSide}`);
if (explain.run.checkpoints.length === 0 || explain.run.cards.length === 0) throw new Error("Rust/WASM explain returned no checkpoints or cards");
const counterfactualFixture = structuredClone(parsedFixture) as { players: { p1: { initialGuard?: number } } };
counterfactualFixture.players.p1.initialGuard = 1;
const counterfactual = JSON.parse(callJson(engine, "yixian_counterfactual_json", JSON.stringify({ fixture: counterfactualFixture, side: "p1", elements: [{ id: "opening-guard", label: "开局护体 1 层", side: "p1", field: "guard", amount: 1 }] }))) as { readonly ok: boolean; readonly run?: { readonly schemaVersion: string; readonly elements: readonly unknown[] }; readonly error?: string };
if (!counterfactual.ok || !counterfactual.run) throw new Error(`Rust/WASM counterfactual smoke failed: ${counterfactual.error ?? "missing run"}`);
if (counterfactual.run.schemaVersion !== "canonical-counterfactual-v1" || counterfactual.run.elements.length !== 1) throw new Error("Rust/WASM counterfactual returned the wrong schema or element count");
const trace = JSON.parse(callJson(engine, "yixian_trace_json", fixture)) as { readonly ok: boolean; readonly run?: { readonly summary: { readonly winnerSide: string }; readonly steps: readonly { readonly eventIndex: number; readonly category: string; readonly p1Changes: readonly unknown[]; readonly p2Changes: readonly unknown[] }[] }; readonly error?: string };
if (!trace.ok || !trace.run) throw new Error(`Rust/WASM trace smoke failed: ${trace.error ?? "missing run"}`);
if (trace.run.summary.winnerSide !== response.run.summary.winnerSide) throw new Error("Rust/WASM trace disagreed with the exact run on the winner");
const tracedChanges = trace.run.steps.reduce((total, step) => total + step.p1Changes.length + step.p2Changes.length, 0);
if (trace.run.steps.length === 0 || tracedChanges === 0) throw new Error("Rust/WASM trace returned no hook steps or no changes");
const joinable = trace.run.steps.filter((step) => step.category !== "battleEnd");
const maxJoinableIndex = joinable.length === 0 ? -1 : Math.max(...joinable.map((step) => step.eventIndex));
if (maxJoinableIndex >= response.run.events.length) throw new Error(`Rust/WASM trace step index ${maxJoinableIndex} is past the ${response.run.events.length} parity events`);
console.log(`Rust/WASM exact smoke: events=${response.run.events.length} winner=${response.run.summary.winnerSide} solver=${solveResponse.run.evaluatedCount} explainCards=${explain.run.cards.length} counterfactuals=${counterfactual.run.elements.length} hookSteps=${trace.run.steps.length} hookChanges=${tracedChanges}`);

function publicSmokeFixture() {
  const card = (id: number, name: string, attack: number) => ({ id, name, attack, attackCount: 1 });
  const cards = (ids: readonly [number, string, number][]) => ids.map(([id, name, attack]) => card(id, name, attack));
  return {
    schemaVersion: 1,
    source: { steamBuild: "24466094" },
    firstPlayerSide: "p1",
    decisionTape: [],
    randomFallbackTape: [],
    expected: { winnerSide: "p1", actorTurnCount: 1, hpDeltaP1MinusP2: 0 },
    players: {
      p1: { level: 5, baseMaxHp: 80, extraMaxHp: 0, characterId: null, talents: [], activeSlotCount: 8, initialDefense: 0, initialAnima: 0, initialGuard: 0, initialMomentum: 0, initialAgility: 0, cards: cards([[1000010, "金灵剑", 10], [1000005, "水灵剑", 1], [1000012, "木灵剑", 1], [1000004, "火灵剑", 1], [1000002, "土灵剑", 1], [1000003, "普通攻击", 1], [1000001, "普通攻击", 1], [1000009, "普通攻击", 1]]) },
      p2: { level: 5, baseMaxHp: 80, extraMaxHp: 0, characterId: null, talents: [], activeSlotCount: 8, initialDefense: 0, initialAnima: 0, initialGuard: 0, initialMomentum: 0, initialAgility: 0, cards: cards([[1000021, "金灵剑", 1], [1000007, "水灵剑", 1], [1000006, "木灵剑", 1], [1000008, "火灵剑", 1], [1000013, "土灵剑", 1], [1000017, "普通攻击", 1], [1000018, "普通攻击", 1], [1000019, "普通攻击", 1]]) },
    },
  };
}

function assertSolverFailure(request: Record<string, unknown>, expectedMessage: string): void {
  const response = JSON.parse(callJson(engine, "yixian_solve_json", JSON.stringify(request))) as { readonly ok: boolean; readonly error?: string };
  if (response.ok || !response.error?.includes(expectedMessage)) throw new Error(`Rust/WASM solver boundary expected ${JSON.stringify(expectedMessage)}, got ${JSON.stringify(response)}`);
}

function callJson(engine: RustWasmExports, operation: keyof Pick<RustWasmExports, "yixian_simulate_json" | "yixian_solve_json" | "yixian_explain_json" | "yixian_counterfactual_json" | "yixian_trace_json">, json: string): string {
  const input = new TextEncoder().encode(json);
  const inputPointer = engine.yixian_alloc(input.length);
  new Uint8Array(engine.memory.buffer, inputPointer, input.length).set(input);
  const packed = engine[operation](inputPointer, input.length);
  engine.yixian_dealloc(inputPointer, input.length);
  const outputPointer = Number(packed & 0xffff_ffffn);
  const outputLength = Number(packed >> 32n);
  const output = new Uint8Array(engine.memory.buffer, outputPointer, outputLength).slice();
  engine.yixian_dealloc(outputPointer, outputLength);
  return new TextDecoder().decode(output);
}
