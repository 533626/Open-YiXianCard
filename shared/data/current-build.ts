import originalBuildProfiles from "./original-build-profiles.json";

const steamBuild = originalBuildProfiles.projectTargetSteamBuild;
if (!/^\d+$/.test(steamBuild)) {
  throw new Error("original-build-profiles.json has no authoritative Steam build");
}

export const CURRENT_STEAM_BUILD = steamBuild;
