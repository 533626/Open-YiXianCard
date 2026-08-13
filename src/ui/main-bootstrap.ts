import { applyImportedReplay } from "./replay-import";
import { loadRepositoryReplayFixture } from "./repository-replay-loader";
import { repositoryFixtureCatalogEnabled } from "./runtime-capabilities";
import type { AppState } from "./types";
import { visibleErrorMessage } from "./view-utils";

/** Apply development-only URL replay input before the first render. */
export async function bootstrapReplayFromLocation(
  state: AppState,
  locationSearch: string,
): Promise<boolean> {
  const params = new URLSearchParams(locationSearch);
  const fixtureName = params.get("fixture");
  let fixtureImported = false;
  if (fixtureName) {
    if (!repositoryFixtureCatalogEnabled) {
      state.error = "托管站无内置对局；请通过“导入对局”选择本机记录。";
    } else {
      try {
        const loaded = await loadRepositoryReplayFixture(fixtureName);
        applyImportedReplay(state, loaded.fixture, { origin: "catalog", id: loaded.entry.id });
        fixtureImported = true;
      } catch (error) {
        state.error = visibleErrorMessage(error);
      }
    }
  }
  return params.get("run") === "1" && (!fixtureName || fixtureImported);
}
