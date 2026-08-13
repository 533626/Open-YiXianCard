import {
  fixtureCatalogEntries,
  fixtureEntryById,
  type UiFixtureEntry,
} from "./fixture-catalog";
import {
  parseReplayFixtureJson,
  type ReplayFixtureWithExpected,
} from "./fixture-contract";
import { repositoryFixtureCatalogEnabled } from "./runtime-capabilities";

export type RepositoryReplayFetch = (
  url: string,
) => Promise<{ readonly ok: boolean; text(): Promise<string> }>;

export interface LoadedRepositoryReplay {
  readonly entry: UiFixtureEntry;
  readonly fixture: ReplayFixtureWithExpected;
}

export async function loadRepositoryReplayFixture(
  id: string,
  fetcher: RepositoryReplayFetch = (url) => fetch(url),
  entries: readonly UiFixtureEntry[] = fixtureCatalogEntries(),
): Promise<LoadedRepositoryReplay> {
  if (!repositoryFixtureCatalogEnabled) throw repositoryLoadError("not-found");
  const entry = fixtureEntryById(id, entries);
  if (!entry) throw repositoryLoadError("not-found");
  const path = entry.path;
  if (
    path.startsWith("/") ||
    !path.endsWith(".json") ||
    path.includes("..") ||
    path.includes("\\") ||
    path.includes("//") ||
    !/^[A-Za-z0-9._/-]+$/.test(path)
  ) {
    throw repositoryLoadError("unsafe-path");
  }
  let response: Awaited<ReturnType<RepositoryReplayFetch>>;
  try {
    response = await fetcher(`/${path}`);
  } catch {
    throw repositoryLoadError("load-failed");
  }
  if (!response.ok) throw repositoryLoadError("load-failed");
  try {
    return { entry, fixture: parseReplayFixtureJson(await response.text()) };
  } catch {
    throw repositoryLoadError("load-failed");
  }
}

function repositoryLoadError(
  code: "not-found" | "unsafe-path" | "load-failed",
): Error {
  const messages = {
    "not-found": "该回放不在开发目录中；请改用本地版本化 JSON 导入。",
    "unsafe-path": "开发目录中的回放路径不安全；已拒绝加载。",
    "load-failed": "开发目录回放加载失败；请改用本地版本化 JSON 导入。",
  } as const;
  return new Error(messages[code]);
}
