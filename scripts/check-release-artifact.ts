import { resolve } from "node:path";
import { auditReleaseArtifact } from "./release-artifact";

const args = process.argv.slice(2);
if (args.length > 1) throw new Error("usage: bun scripts/check-release-artifact.ts [artifact-directory]");

const artifact = resolve(args[0] ?? "dist");
const audit = await auditReleaseArtifact(artifact);

console.log(JSON.stringify({
  status: "release artifact accepted",
  artifact,
  manifestSha256: audit.manifestSha256,
  supportedSteamBuild: audit.manifest.supportedSteamBuild,
  rulesetRevision: audit.manifest.rulesetRevision,
  appCommit: audit.manifest.appCommit,
  fixtureCount: audit.manifest.fixturePolicy.bundledFixtureCount,
  fileCount: audit.files.length,
  totalBytes: audit.totalBytes,
}, null, 2));
