import {
  UI_FIXTURE_INDEX,
  type UiFixtureEntry,
} from "./generated/fixture-index";
import { repositoryFixtureCatalogEnabled } from "./runtime-capabilities";

export type { UiFixtureEntry };

const FIXTURE_BY_ID = new Map(UI_FIXTURE_INDEX.map((entry) => [entry.id, entry] as const));
const MAX_FIXTURE_MATCHES = 80;

export function fixtureCatalogEntries(): readonly UiFixtureEntry[] {
  return repositoryFixtureCatalogEnabled ? UI_FIXTURE_INDEX : [];
}

export function fixtureEntryById(
  id: string,
  entries: readonly UiFixtureEntry[] = fixtureCatalogEntries(),
): UiFixtureEntry | undefined {
  const normalized = id.trim();
  return entries === UI_FIXTURE_INDEX
    ? FIXTURE_BY_ID.get(normalized)
    : entries.find((entry) => entry.id === normalized);
}

export function filterFixtureEntries(
  query: string,
  limit = MAX_FIXTURE_MATCHES,
  entries: readonly UiFixtureEntry[] = fixtureCatalogEntries(),
): readonly UiFixtureEntry[] {
  const normalized = query.trim().toLowerCase();
  const matches = normalized
    ? entries.filter((entry) =>
      entry.id.toLowerCase().includes(normalized) ||
      entry.matchId.toLowerCase().includes(normalized)
    )
    : entries;
  return matches.slice(0, limit);
}

export function fixtureOptionLabel(entry: UiFixtureEntry): string {
  const winner = entry.expectedWinner ? `${entry.expectedWinner}胜` : "无 expected";
  return `${entry.id} · R${entry.round} · ${entry.p1CharacterId ?? "-"} vs ${entry.p2CharacterId ?? "-"} · ${winner}`;
}
