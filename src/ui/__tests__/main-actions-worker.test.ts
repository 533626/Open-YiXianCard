import { describe, expect, test } from "bun:test";
import type { ExactDeckSearchResult } from "../solver-contract";
import {
  buildSolverPayload,
  handleAction,
  selectTargetBuild,
  scheduleSolver,
  type ActionContext,
} from "../main-actions";
import { handleBuildArchivePick, handleNamedField, type FieldContext } from "../main-fields";
import {
  invalidateAllTargetBuilds,
  invalidateComputedResults,
} from "../main-utils";
import type { WorkbenchWorkerClient } from "../worker-client";
import type { WorkbenchSolvePayload } from "../worker-protocol";
import { defaultPlayerConfig } from "../data";
import { baseState } from "./layout-test-helpers";
import { battleFrame, simulationResult } from "./layout-test-helpers";
import type { SavedPlayerConfig, TargetBuild, TargetPracticeState } from "../types";

describe("main action Worker state", () => {

  test("战斗后修改牌级会防抖丢弃旧运行态卡组", () => {
    const state = baseState();
    state.config.players.p1.deck[0] = { baseId: 1, level: 0 };
    state.result = simulationResult([battleFrame([])]);
    state.frameIndex = 1;
    const context = actionContext(
      state,
      fakeClient({ requestId: "unused", result: Promise.resolve(solverResult(0)) }),
      () => {},
    );

    handleAction({
      currentTarget: {
        dataset: { action: "cycle-level", side: "p1", slot: "0" },
      },
    } as unknown as Event, context);

    // 等级立即切换，但 invalidate + render 由调用方（main.ts 监听器）防抖延迟。
    expect(state.config.players.p1.deck[0]!.level).toBe(1);
    expect(state.result).not.toBeNull();
    expect(state.frameIndex).toBe(1);
  });

  test("切换曲线口径会切到曲线模块，不影响战斗结果", () => {
    const state = baseState();
    state.result = simulationResult([battleFrame([])]);
    const context = actionContext(
      state,
      fakeClient({ requestId: "unused", result: Promise.resolve(solverResult(0)) }),
      () => {},
    );

    handleAction({
      currentTarget: {
        dataset: { action: "select-trajectory-metric", metric: "damage" },
      },
    } as unknown as Event, context);

    expect(state.flowMetric).toBe("damage");
    expect(state.battleModule).toBe("trajectory");
    expect(state.result).not.toBeNull();

    handleAction({
      currentTarget: {
        dataset: { action: "select-trajectory-metric", metric: "life" },
      },
    } as unknown as Event, context);

    expect(state.flowMetric).toBe("life");
    expect(state.battleModule).toBe("trajectory");
  });

  test("应用推荐先写入卡组，再清理已经过期的求解结果", () => {
    const state = baseState();
    state.solverResult = {
      side: "p1",
      results: [{
        deck: [{ id: 10_001, rarity: 1 }],
      }],
    } as never;
    const context = actionContext(
      state,
      fakeClient({ requestId: "unused", result: Promise.resolve(solverResult(0)) }),
      () => {},
    );

    handleAction({
      currentTarget: {
        dataset: { action: "apply-solver-best" },
      },
    } as unknown as Event, context);

    expect(state.config.players.p1.deck[0]).toEqual({ baseId: 1, level: 1 });
    expect(state.solverResult).toBeNull();
  });

  test("场上牌与手牌支持两种搜索方式，卡池只开放启发式", () => {
    const state = baseState();
    expect(buildSolverPayload(state, "orderBeam")).toMatchObject({
      mode: "order",
      maxEvaluations: 2_000,
      visitOrder: "stratified",
      topN: 5,
      battleSeeds: [1, 2, 3],
    });
    expect(buildSolverPayload(state, "order")).toMatchObject({
      mode: "order",
      maxEvaluations: 200_000,
      visitOrder: "canonical",
      topN: 5,
    });
    state.config.players.p1.handCardIds = [1, 2];
    expect(buildSolverPayload(state, "handBeam")).toMatchObject({
      mode: "hand",
      maxEvaluations: 2_000,
      visitOrder: "stratified",
    });
    state.config.players.p1 = defaultPlayerConfig("p1", 4_000_002, state.config.gameRound);
    const pool = buildSolverPayload(state, "poolBeam");
    expect(pool).toMatchObject({
      mode: "pool",
      maxEvaluations: 2_000,
      visitOrder: "stratified",
    });
    expect(pool.fixture.players.p1.handCards.length).toBeGreaterThan(0);
  });

  test("切到卡池会回到启发式，点击已禁用的穷举不会改状态", () => {
    const state = baseState();
    state.solverMode = "order";
    const context = actionContext(
      state,
      fakeClient({
        requestId: "unused",
        result: Promise.resolve(solverResult(0)),
      }),
      () => {},
    );

    handleAction({
      currentTarget: {
        dataset: { action: "set-solver-task", solverTask: "pool" },
      },
    } as unknown as Event, context);
    expect(String(state.solverMode)).toBe("poolBeam");

    handleAction({
      currentTarget: {
        dataset: { action: "set-solver-method", solverMethod: "exhaustive" },
      },
    } as unknown as Event, context);
    expect(String(state.solverMode)).toBe("poolBeam");
  });

  test("scheduleSolver 真正等待 Worker，并在完成后写回同 requestId", async () => {
    const state = baseState();
    let resolve!: (result: ExactDeckSearchResult) => void;
    const resultPromise = new Promise<ExactDeckSearchResult>((done) => { resolve = done; });
    const client = fakeClient({ requestId: "solve-live", result: resultPromise });
    let renders = 0;
    const context = actionContext(state, client, () => { renders += 1; });

    const pending = scheduleSolver(context);
    expect(state.solverStatus).toMatchObject({
      state: "running",
      requestId: "solve-live",
      maxEvaluations: 2_000,
    });
    expect(state.solverResult).toBeNull();

    const result = solverResult(17);
    resolve(result);
    await pending;
    expect(state.solverResult).toBe(result);
    expect(state.solverStatus).toMatchObject({
      state: "done",
      requestId: "solve-live",
      evaluatedCount: 17,
    });
    expect(renders).toBe(2);
  });

  test("取消清空当前 requestId，迟到成功结果不会回写", async () => {
    const state = baseState();
    let resolve!: (result: ExactDeckSearchResult) => void;
    const resultPromise = new Promise<ExactDeckSearchResult>((done) => { resolve = done; });
    let cancelled = 0;
    const client = fakeClient(
      { requestId: "solve-stale", result: resultPromise },
      () => { cancelled += 1; },
    );
    const context = actionContext(state, client, () => {});
    const pending = scheduleSolver(context);

    handleAction({
      currentTarget: { dataset: { action: "cancel-solver" } },
    } as unknown as Event, context);
    expect(cancelled).toBe(1);
    expect(state.solverStatus).toMatchObject({ state: "error", message: "求解已取消" });
    expect(state.solverStatus?.requestId).toBeUndefined();

    resolve(solverResult(99));
    await pending;
    expect(state.solverResult).toBeNull();
    expect(state.solverStatus).toMatchObject({ state: "error", message: "求解已取消" });
  });
});

function actionContext(
  state: ReturnType<typeof baseState>,
  client: WorkbenchWorkerClient,
  render: () => void,
): ActionContext {
  return {
    state,
    render,
    workerClient: client,
    runBattle: () => {},
    resetBattle: () => {},
    stopAuto: () => {},
    toggleAuto: () => {},
    adjacentCompletedTurnFrameIndex: () => 0,
  };
}

function fakeClient(
  task: { readonly requestId: string; readonly result: Promise<ExactDeckSearchResult> },
  cancelAll: () => void = () => {},
): WorkbenchWorkerClient {
  return {
    solve: (_payload: WorkbenchSolvePayload) => task,
    cancelAll,
  } as unknown as WorkbenchWorkerClient;
}

function solverResult(evaluatedCount: number): ExactDeckSearchResult {
  return { mode: "order", evaluatedCount } as ExactDeckSearchResult;
}

describe("打靶模式镜像与失效范围", () => {
  function targetState(): TargetPracticeState {
    return {
      builds: [
        {
          id: "tb1",
          name: "打靶构筑 1",
          player: defaultPlayerConfig("p1", 4_000_004, 16),
          result: { perTurn: [], steps: [], totalDamage: 0, stopReason: "turnLimit", reachedTurn: 32 },
          status: "done",
          errorMessage: null,
        },
        {
          id: "tb2",
          name: "打靶构筑 2",
          player: defaultPlayerConfig("p1", 4_000_002, 16),
          result: { perTurn: [], steps: [], totalDamage: 0, stopReason: "turnLimit", reachedTurn: 32 },
          status: "done",
          errorMessage: null,
        },
      ],
      activeBuildId: "tb1",
      damageThreshold: 120,
      displayRounds: 1,
      displayRoundMin: 1,
      displayRoundPending: false,
      compareMode: "overlay",
      expandedStep: null,
      expandedStepBuildId: null,
      duelP1Player: null,
    };
  }

  function savedPlayerConfig(characterId: number): SavedPlayerConfig {
    const player = defaultPlayerConfig("p1", characterId, 16);
    const { side, label, gameRound, activeSlotCount, ...saved } = player as typeof player & {
      readonly side: string;
      readonly label: string;
      readonly gameRound: number;
      readonly activeSlotCount: number;
    };
    void side; void label; void gameRound; void activeSlotCount;
    return saved as SavedPlayerConfig;
  }

  function targetMirrorState() {
    const state = baseState();
    state.workbenchMode = "target";
    state.target = targetState();
    // 镜像约定：config.players.p1 就是聚焦构筑的 player（同一对象）。
    state.config.players.p1 = state.target.builds[0]!.player;
    state.savedBuilds = [{
      id: "saved-1",
      name: "存档 A",
      updatedAt: "2026-01-01T00:00:00Z",
      player: savedPlayerConfig(4_000_001),
    }];
    return state;
  }

  test("存档下拉读档（field handler 路径）后镜像仍指向当前构筑", () => {
    const state = targetMirrorState();
    const context: FieldContext = { state, render: () => {} };
    handleBuildArchivePick({
      currentTarget: { dataset: { buildArchive: "p1" }, value: "存档 A" },
    } as unknown as Event, context);

    const active = state.target!.builds.find((build) => build.id === state.target!.activeBuildId)!;
    // 读档整体替换了 config.players.p1，resync 后构筑必须重新持有同一对象，
    // 否则后续 pick-card 等编辑会写丢、签名不更新、切换构筑后读档被还原。
    expect(active.player).toBe(state.config.players.p1);
    expect(active.player.characterId).toBe(4_000_001);
  });

  test("pick-saved-build 走 handleAction 收口，镜像同样不丢失", () => {
    const state = targetMirrorState();
    const context = actionContext(
      state,
      fakeClient({ requestId: "unused", result: Promise.resolve(solverResult(0)) }),
      () => {},
    );
    handleAction({
      currentTarget: {
        dataset: { action: "pick-saved-build", side: "p1", buildId: "saved-1" },
      },
    } as unknown as Event, context);

    const active = state.target!.builds.find((build) => build.id === state.target!.activeBuildId)!;
    expect(active.player).toBe(state.config.players.p1);
    expect(active.player.characterId).toBe(4_000_001);
  });

  test("构筑内容编辑只作废聚焦构筑的结果，其它构筑的图保留", () => {
    const state = targetMirrorState();
    expect(state.target!.builds[0]!.result).not.toBeNull();
    expect(state.target!.builds[1]!.result).not.toBeNull();
    invalidateComputedResults(state);
    expect(state.target!.builds[0]!.result).toBeNull();
    expect(state.target!.builds[1]!.result).not.toBeNull();
    // 共享参数变更才作废全部。
    invalidateAllTargetBuilds(state);
    expect(state.target!.builds[1]!.result).toBeNull();
  });

  test("阈值输入变更作废全部构筑结果并更新参数", () => {
    const state = targetMirrorState();
    const context: FieldContext = { state, render: () => {} };
    handleNamedField({
      currentTarget: { id: "battle-targetThreshold", value: "60" },
    } as unknown as Event, context);
    expect(state.target!.damageThreshold).toBe(60);
    expect(state.target!.builds[0]!.result).toBeNull();
    expect(state.target!.builds[1]!.result).toBeNull();
  });

  test("切换 active build 清理旧 expandedStep 归属", () => {
    const state = targetMirrorState();
    state.target!.expandedStep = 2;
    state.target!.expandedStepBuildId = "tb1";
    selectTargetBuild(state, "tb2");
    expect(state.target!.expandedStep).toBeNull();
    expect(state.target!.expandedStepBuildId).toBeNull();
  });

  test("显示至回合变更被钳在 [reachedTurn, 32]：从 4 达标时拖到 10 生效、拖回 2 被钳到 4", () => {
    const state = targetMirrorState();
    state.target!.builds[0]!.result = {
      perTurn: [],
      steps: [],
      totalDamage: 125,
      stopReason: "threshold",
      reachedTurn: 4,
    };
    state.target!.displayRounds = 4;
    const context: FieldContext = { state, render: () => {} };

    handleNamedField({
      currentTarget: { id: "battle-targetDisplayRounds", value: "10" },
    } as unknown as Event, context);
    expect(state.target!.displayRounds).toBe(10);
    // 共享参数变更 → 全部构筑结果作废，触发一次自动重算。
    expect(state.target!.builds[0]!.result).toBeNull();
    expect(state.target!.builds[1]!.result).toBeNull();

    // 拖回 2（低于达标回合 4）→ 钳到 4，不出现 0..3 无效窗口。
    state.target!.builds[0]!.result = {
      perTurn: [],
      steps: [],
      totalDamage: 125,
      stopReason: "threshold",
      reachedTurn: 4,
    };
    handleNamedField({
      currentTarget: { id: "battle-targetDisplayRounds", value: "2" },
    } as unknown as Event, context);
    expect(state.target!.displayRounds).toBe(4);
  });
});
