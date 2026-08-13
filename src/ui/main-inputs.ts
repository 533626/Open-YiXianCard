import {
  importLocalReplayFileIntoState,
  localReplayImportErrorMessage,
} from "./replay-import";
import type { AppState, ReplayImportCandidate } from "./types";
import { workbenchWorkerClient } from "./worker-client";

/** 回放导入相关的 DOM 处理：本地 JSON、原版 .bin 扫描与输入框焦点保持。 */

export const MAX_ORIGINAL_REPLAY_FILES = 80;
export const MAX_ORIGINAL_REPLAY_BYTES = 5 * 1024 * 1024;

export async function importLocalReplayFile(
  state: AppState,
  render: () => void,
  input: HTMLInputElement,
): Promise<void> {
  const file = input.files?.[0];
  input.value = "";
  if (!file) return;
  try {
    await importLocalReplayFileIntoState(state, file);
  } catch (error) {
    state.error = localReplayImportErrorMessage(error);
  }
  render();
}

export async function scanOriginalReplayFiles(
  state: AppState,
  render: () => void,
  input: HTMLInputElement,
  directorySelection: boolean,
): Promise<void> {
  const files = input.files ? [...input.files] : [];
  input.value = "";
  await scanOriginalReplayFileList(state, render, files, directorySelection);
}

export async function scanOriginalReplayFileList(
  state: AppState,
  render: () => void,
  list: FileList | readonly File[] | null,
  directorySelection: boolean,
): Promise<void> {
  if (!list || list.length === 0) return;
  const files = [...list]
    .filter((file) => isOriginalReplayBin(file, directorySelection))
    .sort((left, right) => right.lastModified - left.lastModified)
    .slice(0, MAX_ORIGINAL_REPLAY_FILES);
  if (files.length === 0) {
    state.replayImportStatus = {
      state: "error",
      message: "所选位置没有 recent、download 或 star 对局 .bin。",
    };
    render();
    return;
  }

  state.replayImportStatus = {
    state: "scanning",
    message: `正在本地解码 ${files.length} 份记录…`,
  };
  state.replayImportCandidates = [];
  state.error = null;
  render();

  const candidates: ReplayImportCandidate[] = [];
  let rejected = 0;
  for (const [recordIndex, file] of files.entries()) {
    if (file.size > MAX_ORIGINAL_REPLAY_BYTES) {
      rejected += 1;
      continue;
    }
    try {
      const bytes = new Uint8Array(await file.arrayBuffer());
      const decoded = await workbenchWorkerClient.decodeReplay(bytes).result;
      for (const round of decoded.rounds) {
        candidates.push({
          id: `local-${recordIndex}-round-${round.round}`,
          recordId: `local-${recordIndex}`,
          recordIndex,
          recordTimestamp: decoded.beginTimestamp ?? file.lastModified,
          gameVersion: decoded.gameVersion,
          recordCodes: round.recordCodes,
          round: round.round,
          firstPlayerSide: round.firstPlayerSide,
          winnerSide: round.winnerSide,
          actorTurnCount: round.actorTurnCount,
          hpDeltaP1MinusP2: round.hpDeltaP1MinusP2,
          p1CharacterId: round.p1CharacterId,
          p2CharacterId: round.p2CharacterId,
          fixture: round.fixture,
        });
      }
    } catch {
      rejected += 1;
    }
  }
  state.replayImportCandidates = candidates;
  state.replayImportStatus = candidates.length > 0
    ? {
        state: "ready",
        message: `已在本地解析 ${files.length - rejected} 份记录、${candidates.length} 个轮次${rejected > 0 ? `；${rejected} 份无法识别` : ""}。`,
      }
    : {
        state: "error",
        message: "没有识别到可导入的原版对局；文件可能损坏或超出当前规则快照。",
      };
  render();
}

function isOriginalReplayBin(file: File, directorySelection: boolean): boolean {
  if (!file.name.toLowerCase().endsWith(".bin")) return false;
  if (!directorySelection || !file.webkitRelativePath) return true;
  return /\/(?:recentBattleDatas|downloadBattleDatas|starBattleDatas)\//.test(
    `/${file.webkitRelativePath.replaceAll("\\", "/")}`,
  );
}

export function renderReplayCodeInput(
  root: HTMLElement,
  state: AppState,
  render: () => void,
  input: HTMLInputElement,
): void {
  const selectionStart = input.selectionStart;
  const selectionEnd = input.selectionEnd;
  state.replayImportCode = input.value;
  render();
  const next = root.querySelector<HTMLInputElement>("[data-replay-import-code]");
  if (!next) return;
  next.focus({ preventScroll: true });
  if (selectionStart !== null && selectionEnd !== null) {
    next.setSelectionRange(selectionStart, selectionEnd);
  }
}

export async function copyReplayImportText(
  state: AppState,
  render: () => void,
  text: string,
  message: string,
): Promise<void> {
  try {
    await navigator.clipboard.writeText(text);
    state.replayImportStatus = { state: "ready", message };
  } catch {
    state.replayImportStatus = {
      state: "error",
      message: "浏览器未允许写入剪贴板，请手动复制展开的内容。",
    };
  }
  render();
}

export function renderCardSearch(
  root: HTMLElement,
  render: () => void,
  input: HTMLInputElement,
): void {
  const selectionStart = input.selectionStart;
  const selectionEnd = input.selectionEnd;
  render();
  const nextInput = root.querySelector<HTMLInputElement>("#cardSearch");
  if (!nextInput) return;
  nextInput.focus({ preventScroll: true });
  if (selectionStart !== null && selectionEnd !== null) {
    nextInput.setSelectionRange(selectionStart, selectionEnd);
  }
}

export function renderPickerSearch(
  root: HTMLElement,
  render: () => void,
  input: HTMLInputElement,
): void {
  const selectionStart = input.selectionStart;
  const selectionEnd = input.selectionEnd;
  render();
  const nextInput = root.querySelector<HTMLInputElement>(`.picker-search[data-picker-search="${input.dataset.pickerSearch}"]`);
  if (!nextInput) return;
  nextInput.focus({ preventScroll: true });
  if (selectionStart !== null && selectionEnd !== null) nextInput.setSelectionRange(selectionStart, selectionEnd);
}

export function renderFixtureSearch(
  root: HTMLElement,
  render: () => void,
  input: HTMLInputElement,
): void {
  const selectionStart = input.selectionStart;
  const selectionEnd = input.selectionEnd;
  render();
  const nextInput = root.querySelector<HTMLInputElement>("[data-fixture-query]");
  if (!nextInput) return;
  nextInput.focus({ preventScroll: true });
  if (selectionStart !== null && selectionEnd !== null) {
    nextInput.setSelectionRange(selectionStart, selectionEnd);
  }
}
