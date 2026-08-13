import { join } from "node:path";

const repoRoot = join(import.meta.dir, "..");
const requestedChannel = (
  await Bun.file(join(repoRoot, ".bun-version")).text()
).trim();

if (requestedChannel !== "canary") {
  throw new Error(
    `Bun 版本源应为 .bun-version=canary，当前为 ${JSON.stringify(requestedChannel)}`,
  );
}

const revisionProcess = Bun.spawnSync([process.execPath, "--revision"], {
  stdout: "pipe",
  stderr: "pipe",
});
if (revisionProcess.exitCode !== 0) {
  throw new Error(
    `无法读取 Bun revision：${new TextDecoder().decode(revisionProcess.stderr).trim()}`,
  );
}

const revision = new TextDecoder().decode(revisionProcess.stdout).trim();
if (!/^1\.4\.\d+$/.test(Bun.version) || !revision.startsWith(`${Bun.version}-canary.`)) {
  throw new Error(
    `项目当前要求 Bun 1.4 canary，实际为 version=${Bun.version} revision=${revision}`,
  );
}

console.log(`Bun runtime gate passed: ${revision} (.bun-version=${requestedChannel})`);
