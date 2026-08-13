import { buildBattleExplanation } from "./battle-explanation";
import type {
  BattleExplanation,
  CounterfactualReport,
  RuleImpactReport,
} from "./battle-explanation";
import { adaptHookTrace } from "./hook-trace";
import type { HookStep, RustHookTrace } from "./hook-trace";
import { CARD_OPTION_BY_BASE_ID, activeSlotCountForGameRound, getCardVariant } from "./data";
import { derivePlayerBattleStats } from "./derived-state";
import { buildReplayFixture } from "./replay-fixture-builder";
import type {
  BattleConfig,
  BattleFrame,
  PlayerConfig,
  PlayerView,
  Side,
  SimulationResult,
} from "./types";
import type { WorkbenchSolvePayload } from "./worker-protocol";
import type {
  ExactDeckSearchResult,
  SolverEvaluation,
} from "./solver-contract";
import type { OriginalCardConfig } from "./domain";
import { rustBuffs, rustElements } from "./rust-snapshot-view";

export { rustBuffs } from "./rust-snapshot-view";

declare const __OPEN_YIXIAN_ENGINE_WASM_URL__: string | undefined;

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

interface RustRun {
  readonly summary: {
    readonly winnerSide: Side;
    readonly actorTurnCount: number;
    readonly hpDeltaP1MinusP2: number;
  };
  readonly events: readonly RustEvent[];
}

interface RustSolveRun {
  readonly confidence: string;
  readonly evaluatedCount: number;
  readonly skippedDuplicateCount: number;
  readonly candidateCardCount: number;
  readonly baseline: RustSolverEvaluation;
  readonly baselineDeck: readonly number[];
  readonly results: readonly {
    readonly rank: number;
    readonly deck: readonly number[];
    readonly leftoverHandCardIds: readonly number[];
    readonly evaluation: RustSolverEvaluation;
    readonly deckKey: string;
  }[];
  readonly seedsUsed?: readonly number[];
  readonly syntheticDecisionSeedsUsed?: readonly number[];
  readonly usedSyntheticDecisions?: boolean;
}

interface RustSolverEvaluation {
  readonly side: Side;
  readonly scoreProfile: "hpDelta" | "value-v0";
  readonly winner: Side;
  readonly winForSide: boolean;
  readonly actorTurn: number;
  readonly p1Hp: number;
  readonly p2Hp: number;
  readonly hpDeltaForSide: number;
  readonly score: number;
  readonly valueMetrics?: SolverEvaluation["valueMetrics"];
  readonly warnings?: readonly string[];
  readonly decisionEvents?: SolverEvaluation["decisionEvents"];
  readonly seedAggregate?: SolverEvaluation["seedAggregate"];
}

interface RustEvent {
  readonly turn: number;
  readonly kind: "battleStart" | "turnStart" | "cardCompleted" | "turnEnd" | "battleEnd";
  readonly actor: Side;
  readonly slot: number | null;
  readonly cardId: number | null;
  readonly cardName: string | null;
  readonly p1: RustSnapshot;
  readonly p2: RustSnapshot;
}

export interface RustSnapshot {
  readonly hp: number;
  readonly maxHp: number;
  readonly defense: number;
  readonly anima: number;
  readonly guard: number;
  readonly physique: number;
  readonly swordIntent: number;
  readonly sharpness: number;
  readonly cloudChain: number;
  readonly cloudSea: number;
  readonly momentum: number;
  readonly agility: number;
  readonly waterMomentum: number;
  readonly activatedMetal: number;
  readonly activatedWater: number;
  readonly activatedWood: number;
  readonly activatedFire: number;
  readonly activatedEarth: number;
  readonly hexagram: number;
  readonly starPower: number;
  readonly attackBonus: number;
  readonly internalInjury: number;
  readonly weakness: number;
  readonly flaw: number;
  readonly attackReduction: number;
  readonly entangle: number;
  readonly externalInjury: number;
  readonly lostMind: number;
  readonly actionAgainCount: number;
  readonly quanStance: number;
  readonly gunStance: number;
  // 全量暴露缺口（档 1a/1b，来源于私有 evidence audit）：
  // 原版 RefreshBuff 会显示、左侧状态条此前缺失的字段，与 detail_entries 同口径。
  readonly metalRing: number;
  readonly swordEnergy: number;
  readonly waterMonthSwordFormation: number;
  readonly waterFormation: number;
  readonly metalFormation: number;
  readonly earthFormation: number;
  readonly fireFormation: number;
  readonly springFlow: number;
  readonly waterStealth: number;
  readonly metalIronBone: number;
  readonly earthEightWastes: number;
  readonly woodArray: number;
  readonly turtleFormation: number;
  readonly shatterFormation: number;
  readonly thunderFormation: number;
  readonly evilGuFormation: number;
  readonly spiritGatheringFormation: number;
  readonly heavenCycleSwordFormation: number;
  readonly heavenForceFormation: number;
  readonly flowerMazeFormation: number;
  readonly immovableFormation: number;
  readonly eightGatesFormation: number;
  readonly sixYaoFormation: number;
  readonly bengQuanCunJin: number;
  readonly bengQuanReturnProfound: number;
  readonly dreamBengQuanChain: number;
  readonly immortalBindingTune: number;
  readonly illusoryTune: number;
  readonly heartbreakTune: number;
  readonly wildDanceTune: number;
  readonly rejuvenationTune: number;
  readonly xiaoyaoTune: number;
  readonly xiaoyaoGuqin: number;
  readonly chaoticMindTune: number;
  readonly lingGuaArt: number;
  readonly starMoonFan: number;
  readonly infiniteHexagramPlate: number;
  readonly allGoesWell: number;
  readonly recovery: number;
  readonly meditation: number;
  readonly bloodCalamity: number;
  readonly loneNightWolf: number;
  readonly leafBladeFlower: number;
  readonly quietMindset: number;
  readonly reflectMindset: number;
  readonly graftFlowersToTree: number;
  readonly tide: number;
  readonly dismantleMove: number;
  readonly allThingsInauspicious: number;
  readonly fateCycle: number;
  readonly yellowBirdBehind: number;
  readonly exorcism: number;
  readonly iceSnowLotus: number;
  readonly leafShieldFlower: number;
  readonly paintFinishingTouch: number;
  readonly nextTurnDefense: number;
  readonly ignoreDefenseAttacks: number;
  readonly nextAttackShatterDefense: number;
  readonly momentumLimit: number;
  readonly lastElement: PlayerView["lastElement"];
  readonly cardQueue: readonly number[];
  readonly slots: readonly {
    readonly index: number;
    readonly cardId: number;
    readonly baseId: number;
    readonly name: string;
    readonly skipped: boolean;
    readonly hadUsed: boolean;
  }[];
}

type RustWasmResponse =
  | { readonly ok: true; readonly run: RustRun }
  | { readonly ok: false; readonly error: string };

let enginePromise: Promise<RustWasmExports> | undefined;

export async function runRustEngineSimulation(
  config: BattleConfig,
): Promise<SimulationResult> {
  const serialized = serializeBattleFixture(config);
  const response = await callRustEngine(serialized);
  if (response.ok === false) throw new Error(response.error);
  const result = adaptRustRun(config, response.run);
  // 结论层从胜方视角解释；平局/无胜者时没有"赢法"可讲，跳过。
  const winnerSide = result.winnerId as Side | null;
  const openingWinner = winnerSide ? response.run.events[0]?.[winnerSide] : undefined;
  const [explanation, hookSteps] = await Promise.all([
    winnerSide ? explainRustBattle(serialized, winnerSide, openingWinner) : undefined,
    traceRustBattle(serialized, result.frames.length),
  ]);
  return {
    ...result,
    ...(explanation ? { explanation } : {}),
    ...(hookSteps ? { hookSteps } : {}),
  };
}

/**
 * 打靶模式推演：与 simulate 同一条引擎通道（yixian_simulate_json +
 * yixian_trace_json，不新增 WASM export），但 trace 是**必需**读数——
 * 伤害归因完全依赖 hookSteps，不能依赖 simulate 内部的 fail-open trace。
 * trace 取不到时直接抛错（打靶结果会是「0 伤」假象，比报错更糟）。
 */
export async function runRustTargetPractice(
  config: BattleConfig,
): Promise<{ readonly frames: readonly BattleFrame[]; readonly hookSteps: readonly HookStep[] }> {
  const serialized = serializeBattleFixture(config);
  const response = await callRustEngine(serialized);
  if (response.ok === false) throw new Error(response.error);
  const result = adaptRustRun(config, response.run);
  const hookSteps = await traceRustBattle(serialized, result.frames.length);
  if (!hookSteps || hookSteps.length === 0) {
    throw new Error("钩子链不可用：打靶伤害归因需要 yixian_trace_json 数据");
  }
  return { frames: result.frames, hookSteps };
}

/** simulate 与打靶共用的 fixture 序列化（expected 是占位，无人比对）。 */
function serializeBattleFixture(config: BattleConfig): string {
  const fixture = {
    schemaVersion: 1,
    expected: {
      winnerSide: "p1",
      actorTurnCount: 0,
      hpDeltaP1MinusP2: 0,
    },
    ...buildReplayFixture(config),
  };
  return JSON.stringify(fixture);
}

/**
 * 钩子链是附加读数，和结论层一样不能让已经算出的战斗消失。
 */
async function traceRustBattle(
  serializedFixture: string,
  frameCount: number,
): Promise<readonly HookStep[] | undefined> {
  try {
    const response = await callRustJson<
      { ok: true; run: RustHookTrace } | { ok: false; error: string }
    >("yixian_trace_json", serializedFixture);
    if (response.ok === false) return undefined;
    return adaptHookTrace(response.run, frameCount);
  } catch {
    return undefined;
  }
}

/**
 * 结论层解释是附加读数，不是战斗结果本身：解释失败不能让已经算出的战斗消失。
 */
async function explainRustBattle(
  serializedFixture: string,
  side: Side,
  opening: RustSnapshot | undefined,
): Promise<BattleExplanation | undefined> {
  try {
    const elements = openingCounterfactualElements(side, opening);
    const [response, counterfactual] = await Promise.all([
      callRustJson<
        { ok: true; run: RuleImpactReport } | { ok: false; error: string }
      >("yixian_explain_json", `{"side":"${side}","fixture":${serializedFixture}}`),
      loadRustCounterfactuals(serializedFixture, side, elements),
    ]);
    if (response.ok === false) return undefined;
    return buildBattleExplanation(response.run, counterfactual);
  } catch {
    return undefined;
  }
}

interface CounterfactualRequestElement {
  readonly id: string;
  readonly label: string;
  readonly side: Side;
  readonly field: "defense" | "guard";
  readonly amount: number;
}

function openingCounterfactualElements(
  side: Side,
  opening: RustSnapshot | undefined,
): readonly CounterfactualRequestElement[] {
  if (!opening) return [];
  return [
    ...(opening.guard > 0
      ? [{
        id: "opening-guard",
        label: `开局护体 ${opening.guard} 层`,
        side,
        field: "guard" as const,
        amount: opening.guard,
      }]
      : []),
    ...(opening.defense > 0
      ? [{
        id: "opening-defense",
        label: `开局防御 ${opening.defense}`,
        side,
        field: "defense" as const,
        amount: opening.defense,
      }]
      : []),
  ];
}

async function loadRustCounterfactuals(
  serializedFixture: string,
  side: Side,
  elements: readonly CounterfactualRequestElement[],
): Promise<CounterfactualReport | undefined> {
  if (elements.length === 0) return undefined;
  try {
    const response = await callRustJson<
      { ok: true; run: CounterfactualReport } | { ok: false; error: string }
    >(
      "yixian_counterfactual_json",
      `{"side":"${side}","elements":${JSON.stringify(elements)},"fixture":${serializedFixture}}`,
    );
    return response.ok ? response.run : undefined;
  } catch {
    return undefined;
  }
}

export async function runRustSolver(
  payload: WorkbenchSolvePayload,
): Promise<ExactDeckSearchResult> {
  const fixture = {
    schemaVersion: 1,
    expected: {
      winnerSide: "p1",
      actorTurnCount: 0,
      hpDeltaP1MinusP2: 0,
    },
    ...payload.fixture,
  };
  const response = await callRustJson<{ ok: true; run: RustSolveRun } | { ok: false; error: string }>(
    "yixian_solve_json",
    JSON.stringify({
      fixture,
      side: payload.side,
      mode: payload.mode === "order" ? "order" : "hand",
      visitOrder: payload.visitOrder,
      visitSeed: payload.visitSeed,
      scoreProfile: payload.scoring.scoreProfile ?? "hpDelta",
      topN: payload.topN,
      maxEvaluations: payload.maxEvaluations,
      ...(payload.battleSeeds ? { battleSeeds: payload.battleSeeds } : {}),
    }),
  );
  if (response.ok === false) throw new Error(response.error);
  return adaptRustSolverResult(payload, response.run);
}

async function callRustEngine(input: string): Promise<RustWasmResponse> {
  return callRustJson("yixian_simulate_json", input);
}

async function callRustJson<T>(
  operation:
    | "yixian_simulate_json"
    | "yixian_solve_json"
    | "yixian_explain_json"
    | "yixian_counterfactual_json"
    | "yixian_trace_json",
  input: string,
): Promise<T> {
  const engine = await loadRustEngine();
  const encoded = new TextEncoder().encode(input);
  const inputPointer = engine.yixian_alloc(encoded.length);
  try {
    new Uint8Array(engine.memory.buffer, inputPointer, encoded.length).set(encoded);
    const packed = engine[operation](inputPointer, encoded.length);
    const outputPointer = Number(packed & 0xffff_ffffn);
    const outputLength = Number(packed >> 32n);
    try {
      const output = new Uint8Array(
        engine.memory.buffer,
        outputPointer,
        outputLength,
      ).slice();
      return JSON.parse(new TextDecoder().decode(output)) as T;
    } finally {
      engine.yixian_dealloc(outputPointer, outputLength);
    }
  } finally {
    engine.yixian_dealloc(inputPointer, encoded.length);
  }
}

function adaptRustSolverResult(
  payload: WorkbenchSolvePayload,
  rust: RustSolveRun,
): ExactDeckSearchResult {
  const cards = solverCardMap(payload.fixture);
  const mapDeck = (ids: readonly number[]) => ids.map((id) =>
    cards.get(id) ?? ({ id, name: `card:${id}` } as OriginalCardConfig)
  );
  const baselineDeck = mapDeck(rust.baselineDeck);
  const confidence = rust.confidence === "exact" || rust.confidence === "truncated"
    ? rust.confidence
    : "heuristic";
  return {
    mode: payload.mode === "pool" ? "beam" : payload.mode,
    side: payload.side,
    confidence,
    evaluatedCount: rust.evaluatedCount,
    skippedDuplicateCount: rust.skippedDuplicateCount,
    candidateCardCount: rust.candidateCardCount,
    baseline: mapRustSolverEvaluation(rust.baseline),
    baselineDeck,
    results: rust.results.map((result) => {
      const deck = mapDeck(result.deck);
      return {
        rank: result.rank,
        confidence,
        deck,
        leftoverHandCardIds: result.leftoverHandCardIds,
        evaluation: mapRustSolverEvaluation(result.evaluation),
        changedSlots: deck.flatMap((card, slot) => {
          const from = baselineDeck[slot];
          return !from || from.id === card.id ? [] : [{ slot, from, to: card }];
        }),
        deckKey: result.deckKey,
      };
    }),
    ...(rust.seedsUsed ? { seedsUsed: rust.seedsUsed } : {}),
    ...(rust.syntheticDecisionSeedsUsed
      ? { syntheticDecisionSeedsUsed: rust.syntheticDecisionSeedsUsed }
      : {}),
    ...(rust.usedSyntheticDecisions === undefined
      ? {}
      : { usedSyntheticDecisions: rust.usedSyntheticDecisions }),
  };
}

function mapRustSolverEvaluation(evaluation: RustSolverEvaluation): SolverEvaluation {
  return {
    side: evaluation.side,
    scoreProfile: evaluation.scoreProfile,
    winnerSide: evaluation.winner,
    winForSide: evaluation.winForSide,
    actorTurn: evaluation.actorTurn,
    p1Hp: evaluation.p1Hp,
    p2Hp: evaluation.p2Hp,
    hpDeltaForSide: evaluation.hpDeltaForSide,
    score: evaluation.score,
    warnings: evaluation.warnings ?? [],
    completedCards: [],
    ...(evaluation.valueMetrics ? { valueMetrics: evaluation.valueMetrics } : {}),
    ...(evaluation.decisionEvents ? { decisionEvents: evaluation.decisionEvents } : {}),
    ...(evaluation.seedAggregate ? { seedAggregate: evaluation.seedAggregate } : {}),
  };
}

function solverCardMap(fixture: WorkbenchSolvePayload["fixture"]): Map<number, OriginalCardConfig> {
  return new Map([
    ...fixture.players.p1.cards,
    ...fixture.players.p2.cards,
    ...(fixture.catalogCards ?? []),
  ].map((card) => [card.id, card] as const));
}

async function loadRustEngine(): Promise<RustWasmExports> {
  enginePromise ??= instantiateRustEngine();
  return enginePromise;
}

async function instantiateRustEngine(): Promise<RustWasmExports> {
  const url = typeof __OPEN_YIXIAN_ENGINE_WASM_URL__ === "string"
    ? __OPEN_YIXIAN_ENGINE_WASM_URL__
    : "/public/build/yixian-engine.wasm";
  const response = await fetch(url);
  if (!response.ok) throw new Error(`Rust/WASM 引擎加载失败：HTTP ${response.status}`);
  const module = await WebAssembly.instantiate(await response.arrayBuffer(), {});
  return module.instance.exports as unknown as RustWasmExports;
}

function adaptRustRun(config: BattleConfig, run: RustRun): SimulationResult {
  const usedSlots: Record<Side, Set<number>> = { p1: new Set(), p2: new Set() };
  // 开局只有战斗开始结算一帧，不再前置“初始状态”占位帧：战前状态就是结算帧
  // 之前的快照，右侧引擎透视与左侧状态条都只认这一帧。
  const frames: BattleFrame[] = [];
  let actionIndex = 0;
  for (const event of run.events) {
    if (event.kind === "battleEnd") continue;
    if (event.kind === "cardCompleted") {
      actionIndex += 1;
      if (event.slot !== null) usedSlots[event.actor].add(event.slot);
    }
    const title = rustFrameTitle(event, actionIndex);
    frames.push({
      index: frames.length,
      gameRound: config.gameRound,
      actionIndex: event.kind === "cardCompleted" ? actionIndex : null,
      title,
      actorId: event.actor,
      actorTurn: event.turn,
      sourceSlot: event.slot,
      cardId: event.cardId,
      cardName: event.cardName,
      winnerId: null,
      players: {
        p1: playerViewFromRust(config.players.p1, event.p1, usedSlots.p1),
        p2: playerViewFromRust(config.players.p2, event.p2, usedSlots.p2),
      },
      events: [],
      summaries: [title],
    });
  }
  const finalFrame = frames.at(-1)!;
  frames[frames.length - 1] = { ...finalFrame, winnerId: run.summary.winnerSide };
  return {
    frames,
    events: [],
    winnerId: run.summary.winnerSide,
    warnings: [],
    finalActorTurn: run.summary.actorTurnCount,
    actionCount: actionIndex,
  };
}

function playerViewFromRust(
  player: PlayerConfig,
  snapshot: RustSnapshot,
  usedSlots: ReadonlySet<number>,
): PlayerView {
  return playerViewBase(player, snapshot, usedSlots, snapshot);
}

function playerViewBase(
  player: PlayerConfig,
  resources: Pick<RustSnapshot, "hp" | "maxHp" | "defense" | "anima" | "momentum" | "agility" | "guard">,
  usedSlots: ReadonlySet<number>,
  snapshot?: RustSnapshot,
): PlayerView {
  const activeSlotCount = activeSlotCountForGameRound(player.gameRound);
  const initialBuffs: Record<string, number> = {};
  for (const [key, value] of Object.entries(player.buffs)) {
    if (value !== undefined) initialBuffs[key] = value;
  }
  return {
    id: player.side,
    name: player.label,
    side: player.side,
    ...resources,
    momentumLimit: snapshot?.momentumLimit ?? player.momentumLimit,
    buffs: snapshot ? rustBuffs(snapshot) : initialBuffs,
    sustainValues: {},
    starSlots: [...player.starSlots],
    activatedElements: snapshot ? rustElements(snapshot) : [...player.activatedElements],
    lastElement: snapshot?.lastElement ?? player.lastElement,
    cardQueue: snapshot
      ? [...snapshot.cardQueue]
      : Array.from({ length: activeSlotCount }, (_, index) => index),
    slots: snapshot
      ? snapshot.slots.map((slot) => ({
          ...slot,
          temporarilyUpgraded: isTemporaryUpgrade(player, slot),
        }))
      : player.deck.map((slot, index) => {
          const config = slot.originalConfig ?? getCardVariant(slot).config;
          return {
            index,
            cardId: config.id,
            baseId: slot.baseId,
            name: config.name,
            skipped: false,
            hadUsed: usedSlots.has(index),
            temporarilyUpgraded: false,
          };
        }),
  };
}

function isTemporaryUpgrade(
  player: PlayerConfig,
  runtimeSlot: RustSnapshot["slots"][number],
): boolean {
  const configuredSlot = player.deck[runtimeSlot.index];
  if (!configuredSlot || runtimeSlot.baseId !== configuredSlot.baseId) return false;
  const card = CARD_OPTION_BY_BASE_ID.get(runtimeSlot.baseId);
  // 单变体卡（遗迹法器等）无升阶。
  if (card?.variantMode === "single") return false;
  // 梦境卡按境界序（整牌 id 递增）判定临时升阶；普通卡按 rarity。
  if (card?.variantMode === "realm") {
    const configuredVariant = card.variants[configuredSlot.level] ?? card.variants[0]!;
    const runtimeVariant = card.variants.find((v) => v.id === runtimeSlot.cardId);
    if (!runtimeVariant) return runtimeSlot.cardId > (configuredSlot.originalConfig?.id ?? runtimeSlot.cardId);
    return card.variants.indexOf(runtimeVariant) > card.variants.indexOf(configuredVariant);
  }
  const configured = configuredSlot.originalConfig ?? getCardVariant(configuredSlot).config;
  const runtimeLevel = card
    ?.variants.find((variant) => variant.id === runtimeSlot.cardId)
    ?.rarity;
  return runtimeLevel !== undefined
    ? runtimeLevel > configuredSlot.level
    : runtimeSlot.cardId > configured.id;
}

function rustFrameTitle(event: RustEvent, actionIndex: number): string {
  if (event.kind === "battleStart") return "战斗开始结算";
  if (event.kind === "turnStart") return `第 ${Math.max(1, Math.ceil(event.turn / 2))} 回合开始结算`;
  if (event.kind === "turnEnd") return `第 ${Math.max(1, Math.ceil(event.turn / 2))} 回合结束结算`;
  return `第 ${actionIndex} 动 · ${event.cardName ?? `卡牌 ${event.cardId ?? "?"}`}`;
}
