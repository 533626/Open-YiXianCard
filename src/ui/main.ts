import {
  cloneBattleConfig,
  characterBaseTalentSlots,
  defaultBattleConfig,
  defaultPlayerConfig,
  EMPTY_CHARACTER_ID,
  normalizePlayerTalents,
} from "./data";
import { handleAction } from "./main-actions";
import { bootstrapReplayFromLocation } from "./main-bootstrap";
import {
  adjacentCompletedTurnFrameIndex,
  firstCompletedTurnFrameIndex,
} from "./battle-keyboard";
import {
  bindWheelNumberInputs,
  handleBuildArchivePick,
  handleBuffInput,
  handleElements,
  handleIdListCheckbox,
  handleNamedField,
  handlePermanentBuffInput,
  handleSaveNameInput,
} from "./main-fields";
import {
  configMatchesImportedFixture,
} from "./fixture-contract";
import { importLocalReplayFileIntoState } from "./replay-import";
import {
  USER_AGENT_REPLAY_IMPORT_PROMPT,
} from "./replay-import-guide";
import {
  copyReplayImportText,
  importLocalReplayFile,
  renderCardSearch,
  renderFixtureSearch,
  renderPickerSearch,
  renderReplayCodeInput,
  scanOriginalReplayFileList,
  scanOriginalReplayFiles,
} from "./main-inputs";
import { persistTargetBuilds } from "./main-storage";
import { createLevelControl } from "./main-level-control";
import { handleMainKeyDown } from "./main-keyboard";
import {
  battleConfigSignature,
  createInitialAppState,
  targetBuildSignature,
  targetParametersSignature,
} from "./main-state";
import { bindDeckDrag } from "./deck-drag";
import {
  clearDeckSlot,
  configuredFieldCardCount,
  invalidateComputedResults,
  normalizeBattleConfig,
  shouldScheduleAutoBattle,
  syncPlayerDerivedStats,
} from "./main-utils";
import { computeTargetPracticeResult } from "./target-practice-metrics";
import { GAME_TURN_LIMIT } from "./target-dummy";
import { renderApp } from "./render";
import { visibleErrorMessage } from "./view-utils";
import type { ReplayImportCandidate, Side, TargetBuild } from "./types";
import type { TargetPracticeOutcome } from "./worker-protocol";
import { workbenchWorkerClient } from "./worker-client";

const app = document.getElementById("app");
if (!app) throw new Error("缺少 #app 根节点");

document.addEventListener("click", (event) => {
  if ((event.target as HTMLElement).closest(".build-archive-wrap")) return;
  app.querySelectorAll(".build-archive-wrap.open").forEach((element) => element.classList.remove("open"));
});

const state = createInitialAppState(location.search);

let autoTimer: ReturnType<typeof setInterval> | null = null;
let autoBattleTimer: ReturnType<typeof setTimeout> | null = null;
let lastBattleConfigSig = "";
// 打靶模式：每套构筑独立签名 + 全局参数签名，只有真正变化的构筑重跑。
const lastTargetSig = new Map<string, string>();
let lastTargetParamsSig = "";
// 每套构筑的单调运行令牌：旧 run 完成时不得消费新 run 的状态或结果。
const targetRunTokens = new Map<string, number>();
const AUTO_BATTLE_DEBOUNCE_MS = 350;
const levelControl = createLevelControl(app, state, () => {
  invalidateComputedResults(state);
  render();
});

function battleConfigSig(): string {
  return battleConfigSignature(state.config);
}

function maybeScheduleAutoBattle(): void {
  if (state.solverStatus?.state === "running") return;
  if (state.workbenchMode === "target") {
    maybeScheduleTargetPractice();
    return;
  }
  if (!shouldScheduleAutoBattle(state)) return;
  if (battleConfigSig() === lastBattleConfigSig) return;
  if (autoBattleTimer) clearTimeout(autoBattleTimer);
  autoBattleTimer = setTimeout(() => {
    autoBattleTimer = null;
    if (state.pickerMode !== "none") return;
    if (state.battleStatus?.state === "running" || state.solverStatus?.state === "running") return;
    if (battleConfigSig() === lastBattleConfigSig) return;
    void runBattle();
  }, AUTO_BATTLE_DEBOUNCE_MS);
}

/**
 * 打靶自动推演：每套构筑独立守卫——有角色且场上 ≥1 张牌即触发该构筑，
 * 其它构筑不阻塞；签名未变（含在途结果已写回）不重跑。
 */
function maybeScheduleTargetPractice(): void {
  const target = state.target;
  if (!target) return;
  if (state.pickerMode !== "none") return;
  const paramsSig = targetParamsSig();
  if (autoBattleTimer) clearTimeout(autoBattleTimer);
  autoBattleTimer = setTimeout(() => {
    autoBattleTimer = null;
    const current = state.target;
    if (!current || state.pickerMode !== "none") return;
    if (state.solverStatus?.state === "running") return;
    const params = targetParamsSig();
    for (const build of current.builds) {
      if (build.status === "running") continue;
      if (build.player.characterId <= 0) continue;
      if (configuredFieldCardCount(build.player) < 1) continue;
      const sig = targetBuildSig(build) + "|" + params;
      if (params === lastTargetParamsSig && lastTargetSig.get(build.id) === sig) continue;
      void runTargetPractice(build.id);
    }
  }, AUTO_BATTLE_DEBOUNCE_MS);
}

function targetBuildSig(build: TargetBuild): string {
  return targetBuildSignature(build);
}

/** 全局打靶参数签名：阈值/显示回合/修炼轮任一变化 → 全部构筑重跑。 */
function targetParamsSig(): string {
  return targetParametersSignature(state);
}

document.addEventListener("keydown", (event) => handleMainKeyDown(event, {
  app,
  state,
  levelControl,
  render,
  cancelBattle,
  maybeScheduleAutoBattle,
  stepBattleFrame,
}));

void bootstrap();

async function bootstrap(): Promise<void> {
  const shouldRun = await bootstrapReplayFromLocation(state, location.search);
  render();
  if (shouldRun) void runBattle();
}

function render(): void {
  const cardPickerScrollTop = app.querySelector<HTMLElement>("[data-card-picker-scroll]")?.scrollTop;
  const activeElement = document.activeElement as HTMLElement | null;
  const preserveTargetRoundFocus = activeElement?.id === "battle-targetDisplayRounds";
  const targetRoundValue = preserveTargetRoundFocus ? (activeElement as HTMLInputElement).value : null;
  app.innerHTML = renderApp(state);
  bindEvents();
  if (preserveTargetRoundFocus) {
    const nextRoundInput = app.querySelector<HTMLInputElement>("#battle-targetDisplayRounds");
    if (nextRoundInput && !nextRoundInput.disabled) {
      nextRoundInput.value = targetRoundValue ?? nextRoundInput.value;
      nextRoundInput.focus({ preventScroll: true });
    }
  }
  if (cardPickerScrollTop !== undefined) {
    const nextCardPicker = app.querySelector<HTMLElement>("[data-card-picker-scroll]");
    if (nextCardPicker) nextCardPicker.scrollTop = cardPickerScrollTop;
  }
  syncSelectedBattleLogItem();
  maybeScheduleAutoBattle();
  // 打靶模式：任何状态变更都走 render()，在这里持久化构筑内容（选卡/仙命/等级等
  // 编辑都随之落盘），避免逐个编辑路径漏掉 persistTargetBuilds。
  if (state.workbenchMode === "target" && state.target) {
    persistTargetBuilds(state.target.builds);
  }
}

function syncSelectedBattleLogItem(): void {
  app
    .querySelector(".battle-progress-dot.selected")
    ?.scrollIntoView({ block: "nearest", inline: "center" });
  app.querySelector(".deck-slot.active")?.scrollIntoView({ block: "nearest" });
}

function bindEvents(): void {
  const actionContext = {
    state,
    render,
    runBattle,
    cancelBattle,
    workerClient: workbenchWorkerClient,
    resetBattle,
    stopAuto,
    toggleAuto,
    maybeScheduleAutoBattle,
    adjacentCompletedTurnFrameIndex,
    runTargetPractice,
    cancelTargetPractice,
  };
  const fieldContext = { state, render };

  app.querySelectorAll<HTMLElement>(".player-panel[data-side]").forEach((panel) => {
    panel.addEventListener("click", (event) => {
      const side = panel.dataset.side as Side | undefined;
      if (!side) return;
      state.activeSide = side;
      const target = event.target as HTMLElement;
      if (target.closest("input, select, textarea, button, a, label")) return;
      if (!target.closest("[data-action]")) render();
    }, true);
  });

  app.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    const listener = (event: Event) => {
      const action = element.dataset.action;
      if (action === "cycle-level" || action === "slot-level") {
        event.stopPropagation();
        const side = element.dataset.side as Side | undefined;
        const slot = element.dataset.slot === undefined ? null : Number(element.dataset.slot);
        handleAction(event, actionContext);
        if (side && slot !== null) levelControl.patch(side, slot);
        levelControl.schedule();
        return;
      }
      // 任何非等级操作都取消待处理的等级防抖，让它不干扰后续流程。
      levelControl.cancel();
      handleAction(event, actionContext);
    };
    if (element instanceof HTMLSelectElement) element.addEventListener("change", listener);
    else element.addEventListener("click", listener);
  });

  app.querySelectorAll<HTMLElement>(
    ".player-deck .card-face[data-action='select-slot']",
  ).forEach((element) => {
    element.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      const side = element.dataset.side as Side | undefined;
      const slot = element.dataset.slot === undefined ? null : Number(element.dataset.slot);
      if (!side || slot === null) return;
      clearDeckSlot(state, side, slot);
      state.activeSide = side;
      state.selectedSlot = slot;
      state.pickerMode = "none";
      render();
    });
  });
  bindDeckDrag(app, state, render);

  app.querySelectorAll<HTMLElement>(".talent-slot[data-action='select-talent-slot']").forEach((element) => {
    element.addEventListener("contextmenu", (event) => {
      event.preventDefault();
      const side = element.dataset.side as Side | undefined;
      const slot = element.dataset.slot === undefined ? null : Number(element.dataset.slot);
      if (!side || slot === null || slot <= 0) return;
      const player = state.config.players[side];
      const defaultTalent = characterBaseTalentSlots(player.characterId)[slot]?.id ?? 0;
      player.talents[slot] = defaultTalent;
      normalizePlayerTalents(player);
      syncPlayerDerivedStats(player, state.config.gameRound, true);
      state.activeSide = side;
      state.selectedTalentSlot = slot;
      state.pickerMode = "none";
      render();
    });
  });

  app.querySelectorAll<HTMLInputElement>("[data-element]").forEach((element) => {
    element.addEventListener("change", (event) => handleElements(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-id-list]").forEach((element) => {
    element.addEventListener("change", (event) => handleIdListCheckbox(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-buff]").forEach((element) => {
    element.addEventListener("change", (event) => handleBuffInput(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-permanent-buff]").forEach((element) => {
    element.addEventListener("change", (event) => handlePermanentBuffInput(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-save-name]").forEach((element) => {
    element.addEventListener("input", (event) => handleSaveNameInput(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-build-archive]").forEach((element) => {
    element.addEventListener("change", (event) => handleBuildArchivePick(event, fieldContext));
  });
  app.querySelectorAll<HTMLInputElement>("[data-target-build-name]").forEach((element) => {
    element.addEventListener("change", (event) => handleAction(event, actionContext));
  });
  app.querySelector<HTMLInputElement>("[data-fixture-query]")?.addEventListener("input", (event) => {
    const input = event.target as HTMLInputElement;
    state.fixtureImportQuery = input.value;
    state.fixtureImportId = input.value;
    renderFixtureSearch(app, render, input);
  });
  app.querySelector<HTMLInputElement>("[data-fixture-query]")?.addEventListener("keydown", (event) => {
    if (event.key !== "Enter") return;
    event.preventDefault();
    app.querySelector<HTMLButtonElement>("[data-action='import-fixture']")?.click();
  });
  app.querySelector<HTMLInputElement>("[data-local-replay-file]")?.addEventListener("change", (event) => {
    void importLocalReplayFile(state, render, event.target as HTMLInputElement);
  });
  app.querySelector<HTMLInputElement>("[data-original-replay-directory]")?.addEventListener("change", (event) => {
    void scanOriginalReplayFiles(state, render, event.target as HTMLInputElement, true);
  });
  app.querySelector<HTMLInputElement>("[data-original-replay-files]")?.addEventListener("change", (event) => {
    void scanOriginalReplayFiles(state, render, event.target as HTMLInputElement, false);
  });
  app.querySelector<HTMLInputElement>("[data-replay-import-code]")?.addEventListener("input", (event) => {
    renderReplayCodeInput(app, state, render, event.target as HTMLInputElement);
  });
  app.querySelector<HTMLElement>("[data-replay-dropzone]")?.addEventListener("dragover", (event) => {
    event.preventDefault();
    if (event.dataTransfer) event.dataTransfer.dropEffect = "copy";
  });
  app.querySelector<HTMLElement>("[data-replay-dropzone]")?.addEventListener("drop", (event) => {
    event.preventDefault();
    void scanOriginalReplayFileList(state, render, event.dataTransfer?.files ?? null, false);
  });
  app.querySelectorAll<HTMLElement>("[data-copy-replay-path]").forEach((button) => {
    button.addEventListener("click", () =>
      void copyReplayImportText(state, render, button.dataset.copyReplayPath ?? "", "缓存路径已复制"));
  });
  app.querySelector<HTMLElement>("[data-copy-agent-guide]")?.addEventListener("click", () => {
    void copyReplayImportText(state, render, USER_AGENT_REPLAY_IMPORT_PROMPT, "AI 助手说明已复制，格式已保留");
  });

  const cardSearch = app.querySelector<HTMLInputElement>("#cardSearch");
  cardSearch?.addEventListener("input", (event) => {
    const input = event.target as HTMLInputElement;
    state.cardSearch = input.value;
    if ((event as InputEvent).isComposing) return;
    renderCardSearch(app, render, input);
  });
  cardSearch?.addEventListener("compositionend", (event) => {
    const input = event.target as HTMLInputElement;
    state.cardSearch = input.value;
    renderCardSearch(app, render, input);
  });
  app.querySelectorAll<HTMLInputElement>(".picker-search:not(.card-picker-search)").forEach((input) => {
    input.addEventListener("input", (event) => {
      const current = event.target as HTMLInputElement;
      state.pickerSearch = current.value;
      if ((event as InputEvent).isComposing) return;
      renderPickerSearch(app, render, current);
    });
    input.addEventListener("compositionend", (event) => {
      const current = event.target as HTMLInputElement;
      state.pickerSearch = current.value;
      renderPickerSearch(app, render, current);
    });
  });
  app.querySelector("#frameRange")?.addEventListener("input", (event) => {
    state.frameIndex = Number((event.target as HTMLInputElement).value);
    render();
  });
  // 打靶「显示至回合」滑条：input 只更新临时读数（拖动预览，不排 Worker），
  // change（松手/键盘提交）才写状态、作废结果并触发一次自动重算。range 的
  // change 事件在松开或键盘提交（回车/方向键松手）时触发，与 number input 的
  // 提交语义一致；Enter 后 blur 仍会再触发一次 change，值相同则无害。
  app.querySelector<HTMLInputElement>("#battle-targetDisplayRounds")?.addEventListener("input", (event) => {
    const input = event.target as HTMLInputElement;
    const readout = input.closest(".display-round-field")?.querySelector<HTMLElement>("[data-display-round-readout]");
    if (readout) {
      readout.textContent = `R${input.value} / ${GAME_TURN_LIMIT}`;
      readout.classList.remove("waiting");
    }
    input.setAttribute("aria-valuetext", `${input.value} / ${GAME_TURN_LIMIT}`);
  });
  app.querySelectorAll<HTMLElement>("[data-action]").forEach((element) => {
    element.addEventListener("keydown", (event) => {
      const keyboardEvent = event as KeyboardEvent;
      if (keyboardEvent.key !== "Enter" && keyboardEvent.key !== " ") return;
      const action = element.dataset.action;
      if (action !== "toggle-target-step" && action !== "select-target-build") return;
      keyboardEvent.preventDefault();
      element.dispatchEvent(new MouseEvent("click", { bubbles: true, cancelable: true }));
    });
  });
  app.querySelectorAll<HTMLInputElement | HTMLTextAreaElement | HTMLSelectElement>("[id^='battle-'], [id^='player-']").forEach((element) => {
    element.addEventListener("change", (event) => handleNamedField(event, fieldContext));
  });
  bindWheelNumberInputs(app);
}


function resetBattle(): void {
  stopAuto();
  if (state.battleStatus?.state === "running" || state.solverStatus?.state === "running") {
    workbenchWorkerClient.cancelAll("工作台已重置");
  }
  if (state.workbenchMode === "target" && state.target) {
    // 打靶重置 = 全部构筑清空（保留槽位与名称），镜像重新挂到当前聚焦构筑。
    const round = state.config.gameRound;
    for (const build of state.target.builds) {
      build.player = defaultPlayerConfig("p1", EMPTY_CHARACTER_ID, round);
      build.result = null;
      build.status = "idle";
      build.errorMessage = null;
    }
    // 没有结果了，显示至回合回到初始等待值（滑块将显示禁用等待态）。
    state.target.displayRounds = 1;
    const active = state.target.builds.find((build) => build.id === state.target!.activeBuildId)
      ?? state.target.builds[0];
    if (active) state.config.players.p1 = active.player;
    lastTargetSig.clear();
    lastTargetParamsSig = "";
    state.error = null;
    render();
    return;
  }
  state.config = defaultBattleConfig();
  normalizeBattleConfig(state.config);
  state.result = null;
  state.battleStatus = null;
  state.solverResult = null;
  state.solverStatus = null;
  state.frameIndex = 0;
  state.view = "setup";
  state.pickerMode = "none";
  state.importedFixture = null;
  state.importedFixtureId = null;
  state.importedFixtureOrigin = null;
  state.fixtureConsistency = null;
  state.error = null;
  render();
}

async function runBattle(): Promise<void> {
  if (state.battleStatus?.state === "running") return;
  if (state.solverStatus?.state === "running") {
    state.error = "请先取消正在运行的求解";
    render();
    return;
  }
  stopAuto();
  state.pickerMode = "none";
  const startedAt = nowMs();
  let fixtureMatchesImport = false;
  let task: ReturnType<typeof workbenchWorkerClient.simulate>;
  try {
    fixtureMatchesImport = state.importedFixture
      ? configMatchesImportedFixture(state.importedFixture, state.config)
      : false;
    if (!fixtureMatchesImport) {
      normalizeBattleConfig(state.config);
      normalizePlayerTalents(state.config.players.p1);
      normalizePlayerTalents(state.config.players.p2);
    }
    lastBattleConfigSig = battleConfigSig();
    task = workbenchWorkerClient.simulate(
      cloneBattleConfig(state.config),
      state.importedFixture && fixtureMatchesImport
        ? structuredClone(state.importedFixture)
        : undefined,
    );
  } catch (error) {
    const message = visibleErrorMessage(error);
    state.battleStatus = {
      state: "error",
      elapsedMs: Math.round(nowMs() - startedAt),
      message,
    };
    state.error = message;
    render();
    return;
  }

  state.battleStatus = {
    state: "running",
    startedAt,
    requestId: task.requestId,
  };
  state.fixtureConsistency = null;
  state.error = null;
  render();
  focusAction("cancel-battle");

  try {
    const outcome = await task.result;
    if (!isCurrentBattleRequest(task.requestId)) return;
    const result = outcome.result;
    state.result = result;
    state.frameIndex = firstCompletedTurnFrameIndex(result);
    state.view = "battle";
    state.fixtureConsistency = outcome.fixtureConsistency;
    state.battleStatus = {
      state: "done",
      elapsedMs: Math.round(nowMs() - startedAt),
      requestId: task.requestId,
    };
    state.error = null;
  } catch (error) {
    if (!isCurrentBattleRequest(task.requestId)) return;
    const message = visibleErrorMessage(error);
    state.battleStatus = {
      state: "error",
      elapsedMs: Math.round(nowMs() - startedAt),
      message,
      requestId: task.requestId,
    };
    state.error = message;
  }
  render();
}

function cancelBattle(): void {
  const status = state.battleStatus;
  if (status?.state !== "running") return;
  workbenchWorkerClient.cancelAll("战斗推演已取消");
  state.battleStatus = {
    state: "error",
    elapsedMs: status.startedAt === undefined ? undefined : Math.round(nowMs() - status.startedAt),
    message: "战斗推演已取消",
  };
  state.error = null;
  render();
  focusAction("run");
}

/** 打靶推演：单套构筑独立运行，结果按 buildId 写回；在途期间构筑/参数已变则作废。 */
async function runTargetPractice(buildId: string): Promise<void> {
  const target = state.target;
  const build = target?.builds.find((candidate) => candidate.id === buildId);
  if (!target || !build) return;
  if (build.status === "running") return;
  if (state.solverStatus?.state === "running") {
    state.error = "请先取消正在运行的求解";
    render();
    return;
  }
  state.pickerMode = "none";
  syncPlayerDerivedStats(build.player, state.config.gameRound, false);
  normalizePlayerTalents(build.player);
  const runSig = targetBuildSig(build) + "|" + targetParamsSig();
  target.displayRoundMin = Math.min(GAME_TURN_LIMIT, Math.max(1, target.displayRoundMin ?? build.result?.reachedTurn ?? 1));
  // 单调运行令牌：编辑触发的重跑会拿到新令牌，旧 run 完成时按令牌判定过期，
  // 不得消费新 run 的 status/result（status 是单槽，旧 run 不能碰）。
  const runToken = (targetRunTokens.get(buildId) ?? 0) + 1;
  targetRunTokens.set(buildId, runToken);
  let task: ReturnType<typeof workbenchWorkerClient.targetPractice>;
  try {
    task = workbenchWorkerClient.targetPractice({
      buildId,
      build: structuredClone(build.player),
      gameRound: state.config.gameRound,
    });
  } catch (error) {
    target.displayRoundPending = false;
    build.status = "error";
    build.errorMessage = visibleErrorMessage(error);
    render();
    return;
  }
  build.status = "running";
  build.result = null;
  build.errorMessage = null;
  target.displayRoundPending = true;
  render();

  try {
    const outcome = await task.result;
    const current = state.target;
    const currentBuild = current?.builds.find((candidate) => candidate.id === buildId);
    if (!currentBuild || currentBuild.status !== "running") return;
    if (targetRunTokens.get(buildId) !== runToken) return; // 过期 run：不碰状态
    if (targetBuildSig(currentBuild) + "|" + targetParamsSig() !== runSig) {
      // 在途期间构筑/参数已变：结果作废。这里改了 status，必须立即 render——
      // 否则 DOM 停在「推演中」，而 render() 末尾的自动调度会按新签名补跑。
      currentBuild.status = "idle";
      if (current) current.displayRoundPending = false;
      lastTargetSig.delete(buildId);
      render();
      return;
    }
    currentBuild.result = computeTargetPracticeResult(
      outcome.hookSteps,
      outcome.frames,
      current!.damageThreshold,
      current!.displayRounds,
    );
    // 新结果的达标回合可能高于旧值：把 displayRounds 钳到新的
    // [reachedTurn, 32] 范围，滑块下次渲染从达标回合起（不显示无效窗口）。
    if (currentBuild.result.reachedTurn > current!.displayRounds) {
      current!.displayRounds = currentBuild.result.reachedTurn;
    }
    currentBuild.status = "done";
    current!.displayRoundPending = false;
    current!.displayRoundMin = Math.min(GAME_TURN_LIMIT, Math.max(1, currentBuild.result.reachedTurn));
    // 记账用钳制后的参数签名，否则下一次自动调度会因签名变化误判需要重跑。
    lastTargetSig.set(buildId, targetBuildSig(currentBuild) + "|" + targetParamsSig());
    lastTargetParamsSig = targetParamsSig();
  } catch (error) {
    const currentBuild = state.target?.builds.find((candidate) => candidate.id === buildId);
    if (!currentBuild) return;
    if (targetRunTokens.get(buildId) !== runToken) return; // 过期 run：不碰状态
    if (state.target) state.target.displayRoundPending = false;
    if ((error as { kind?: string }).kind === "cancelled") {
      currentBuild.status = "idle";
      currentBuild.errorMessage = null;
      lastTargetSig.delete(buildId);
    } else {
      currentBuild.status = "error";
      currentBuild.errorMessage = visibleErrorMessage(error);
      // 失败也记账，避免同一构筑无限自动重试；构筑一改签名即重跑。
      lastTargetSig.set(buildId, runSig);
      lastTargetParamsSig = targetParamsSig();
    }
  }
  render();
}

function cancelTargetPractice(): void {
  const target = state.target;
  if (!target?.builds.some((build) => build.status === "running")) return;
  workbenchWorkerClient.cancelAll("打靶推演已取消");
  for (const build of target.builds) {
    if (build.status === "running") build.status = "idle";
  }
  target.displayRoundPending = false;
  render();
}

function isCurrentBattleRequest(requestId: string): boolean {
  return state.battleStatus?.state === "running" && state.battleStatus.requestId === requestId;
}

function focusAction(action: string): void {
  queueMicrotask(() => {
    app.querySelector<HTMLButtonElement>(`[data-action='${action}']`)?.focus({ preventScroll: true });
  });
}

function nowMs(): number {
  return globalThis.performance?.now?.() ?? Date.now();
}

export function stepBattleFrame(direction: -1 | 1): void {
  if (!state.result) return;
  state.frameIndex = adjacentCompletedTurnFrameIndex(state.result, state.frameIndex, direction);
  render();
}

function toggleAuto(): void {
  stopAuto();
  if (!state.result) return;
  state.frameIndex = state.result.frames.length - 1;
  render();
}

function stopAuto(): void {
  state.autoPlaying = false;
  if (autoTimer) clearInterval(autoTimer);
  autoTimer = null;
}
