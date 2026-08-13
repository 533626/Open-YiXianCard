import {
  importReplayFixtureConfig,
  parseReplayFixtureJson,
  type ReplayFixtureWithExpected,
} from "./fixture-contract";
import type { AppState, ImportedFixtureOrigin } from "./types";

export const LOCAL_REPLAY_SCHEMA = "open-yixiancard/replay-fixture";
export const LOCAL_REPLAY_SCHEMA_VERSION = 1;
export const MAX_LOCAL_REPLAY_BYTES = 5 * 1024 * 1024;

export interface LocalReplayFileLike {
  readonly name?: string;
  readonly size?: number;
  text(): Promise<string>;
}

type ReplayImportErrorCode =
  | "raw-bin"
  | "too-large"
  | "read-failed"
  | "invalid-json"
  | "invalid-envelope"
  | "unsupported-version"
  | "invalid-fixture";

class ReplayImportError extends Error {
  constructor(readonly code: ReplayImportErrorCode, message: string) {
    super(message);
    this.name = "ReplayImportError";
  }
}

export function parseVersionedLocalReplayJson(text: string): ReplayFixtureWithExpected {
  let parsed: unknown;
  try {
    parsed = JSON.parse(text);
  } catch {
    throw importError("invalid-json");
  }
  if (!isRecord(parsed)) throw importError("invalid-envelope");
  let fixture: Record<string, unknown>;
  if (parsed.schemaVersion !== undefined) {
    if (parsed.schemaVersion !== LOCAL_REPLAY_SCHEMA_VERSION) {
      throw importError("unsupported-version");
    }
    fixture = parsed;
  } else if (parsed.schema === LOCAL_REPLAY_SCHEMA && isRecord(parsed.fixture)) {
    if (parsed.version !== LOCAL_REPLAY_SCHEMA_VERSION) {
      throw importError("unsupported-version");
    }
    fixture = parsed.fixture;
  } else {
    throw importError("invalid-envelope");
  }
  try {
    return parseReplayFixtureJson(JSON.stringify(fixture));
  } catch {
    throw importError("invalid-fixture");
  }
}

export async function importLocalReplayFileIntoState(
  state: AppState,
  file: LocalReplayFileLike,
): Promise<void> {
  if (file.size !== undefined && file.size > MAX_LOCAL_REPLAY_BYTES) {
    throw importError("too-large");
  }
  if (file.name?.toLowerCase().endsWith(".bin")) throw importError("raw-bin");
  let text: string;
  try {
    text = await file.text();
  } catch {
    throw importError("read-failed");
  }
  const fixture = parseVersionedLocalReplayJson(text);
  applyImportedReplay(state, fixture, { origin: "local" });
}

export function applyImportedReplay(
  state: AppState,
  fixture: ReplayFixtureWithExpected,
  options: { readonly origin: ImportedFixtureOrigin; readonly id?: string },
): void {
  // Construct the full next configuration before touching state. Parse or
  // adaptation failures therefore leave the user's current build intact.
  const config = importReplayFixtureConfig(fixture);
  const id = options.id ?? null;
  state.config = config;
  state.importedFixture = fixture;
  state.importedFixtureId = id;
  state.importedFixtureOrigin = options.origin;
  state.fixtureImportId = id ?? "";
  state.fixtureImportQuery = id ?? "";
  state.fixtureImportOpen = false;
  state.replayImportCandidates = [];
  state.replayImportStatus = null;
  state.replayImportCode = "";
  state.result = null;
  state.frameIndex = 0;
  state.view = "setup";
  state.pickerMode = "none";
  state.fixtureConsistency = null;
  state.solverResult = null;
  state.solverStatus = null;
  state.error = null;
  if (id) {
    state.recentFixtureIds = [
      id,
      ...(state.recentFixtureIds ?? []).filter((recent) => recent !== id),
    ].slice(0, 6);
  }
}

export function localReplayImportErrorMessage(error: unknown): string {
  return error instanceof ReplayImportError
    ? error.message
    : "本地 JSON 导入失败；当前构筑未改变。";
}

function importError(code: ReplayImportErrorCode): ReplayImportError {
  const messages: Readonly<Record<ReplayImportErrorCode, string>> = {
    "raw-bin": "请在“本机记录”中选择原版 .bin；当前构筑未改变。",
    "too-large": "本地 JSON 超过 5 MiB 限制；当前构筑未改变。",
    "read-failed": "无法读取本地文件；请重新选择版本化 JSON。当前构筑未改变。",
    "invalid-json": "所选文件不是有效 JSON。当前构筑未改变。",
    "invalid-envelope": `需要 schemaVersion=${LOCAL_REPLAY_SCHEMA_VERSION} 的回放 JSON，或 schema=${LOCAL_REPLAY_SCHEMA} envelope。当前构筑未改变。`,
    "unsupported-version": `仅支持本地回放 schema 版本 ${LOCAL_REPLAY_SCHEMA_VERSION}。当前构筑未改变。`,
    "invalid-fixture": "JSON 中的 fixture 不符合回放结构。当前构筑未改变。",
  };
  return new ReplayImportError(code, messages[code]);
}

function isRecord(value: unknown): value is Record<string, unknown> {
  return value !== null && typeof value === "object" && !Array.isArray(value);
}
