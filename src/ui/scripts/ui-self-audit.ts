import { mkdir, writeFile } from "node:fs/promises";
import { join } from "node:path";
import { tmpdir } from "node:os";
import { withHeadlessPage } from "./lib/headless-page";
import {
  AUDIT_SCENARIOS,
  collectFindingsInPage,
  prepareScenarioInPage,
} from "./lib/ui-audit-scenarios";
import type { AuditFinding } from "./lib/ui-audit-scenarios";
import baselineJson from "../audit-baseline.json";

/**
 * UI 自验证：按场景驱动 headless UI，用机器可判定的规则找出"看起来就不对"的东西，
 * 并把截图和结论落到系统临时目录（`/tmp` 下的 `open-yixiancard-ui-audit/`），不必每轮让人截图反馈。
 *
 * 基线台账 `src/ui/audit-baseline.json` 记录当前已知未修项。新出现的违规直接失败；
 * 已修好的条目留在台账里也失败，这样台账只能变短，不会变成"永久豁免清单"。
 */

interface BaselineEntry {
  readonly id: string;
  readonly note: string;
}

const baseline = baselineJson as readonly BaselineEntry[];
const outputDir = join(tmpdir(), "open-yixiancard-ui-audit");
const port = Number(process.env.UI_AUDIT_PORT ?? 3002);
const debugPort = Number(process.env.UI_AUDIT_DEBUG_PORT ?? 9224);
const width = Number(process.env.UI_AUDIT_WIDTH ?? 1600);
const height = Number(process.env.UI_AUDIT_HEIGHT ?? 1000);

await mkdir(outputDir, { recursive: true });

const findings = await withHeadlessPage(
  { port, debugPort, width, height },
  async (page) => {
    const collected: AuditFinding[] = [];
    for (const scenario of AUDIT_SCENARIOS) {
      await page.navigate(scenario.path);
      const setup = await page.evaluate(
        `(${prepareScenarioInPage.toString()})(${JSON.stringify(scenario.module ?? null)})`,
        true,
      );
      const setupResult = setup as { ok?: boolean; error?: string } | undefined;
      if (setupResult?.ok !== true) {
        collected.push({
          scenario: scenario.name,
          rule: "scenario-setup",
          key: scenario.path,
          detail: setupResult?.error ?? "场景准备没有返回结果",
        });
        continue;
      }
      const raw = await page.evaluate(
        `(${collectFindingsInPage.toString()})(${JSON.stringify(scenario.name)})`,
      );
      collected.push(...(raw as AuditFinding[]));
      await writeFile(
        join(outputDir, `${scenario.name}.png`),
        Buffer.from(await page.screenshot(), "base64"),
      );
    }
    return collected;
  },
);

const deduped = dedupe(findings);
const seen = new Set(deduped.map(findingId));
const knownIds = new Set(baseline.map((entry) => entry.id));
const fresh = deduped.filter((finding) => !knownIds.has(findingId(finding)));
const stale = baseline.filter((entry) => !seen.has(entry.id));

await writeFile(
  join(outputDir, "report.json"),
  `${JSON.stringify({ width, height, findings: deduped, fresh, stale }, null, 2)}\n`,
);

console.log(`UI self-audit ${width}x${height} | 场景 ${AUDIT_SCENARIOS.length}`);
console.log(
  `发现 ${deduped.length} 类（${findings.length} 处），其中台账已记 ${
    deduped.length - fresh.length
  } 类`,
);
console.log(`产物 | ${outputDir}`);

if (fresh.length > 0) {
  console.log("");
  console.log("规则 | 场景 | 位置 | 说明");
  console.log("--- | --- | --- | ---");
  for (const finding of fresh.slice(0, 60)) {
    console.log(
      `${finding.rule} | ${finding.scenario} | ${finding.key} | ${finding.detail}`,
    );
  }
  if (fresh.length > 60) console.log(`... | | | 另有 ${fresh.length - 60} 项`);
}

if (stale.length > 0) {
  console.log("");
  console.log("台账中已不再出现，请删除条目：");
  for (const entry of stale) console.log(`- ${entry.id} · ${entry.note}`);
}

if (fresh.length > 0 || stale.length > 0) process.exitCode = 1;

/**
 * 同一条规则在同一位置往往命中很多次（8 个卡槽都截断同一段文案）。
 * 台账按"类"记账，否则一次改动会牵动十几行豁免。
 */
function dedupe(items: readonly AuditFinding[]): AuditFinding[] {
  const byId = new Map<string, { finding: AuditFinding; count: number }>();
  for (const finding of items) {
    const id = findingId(finding);
    const existing = byId.get(id);
    if (existing) existing.count += 1;
    else byId.set(id, { finding, count: 1 });
  }
  return [...byId.values()].map(({ finding, count }) =>
    count === 1 ? finding : { ...finding, detail: `${finding.detail}（${count} 处）` }
  );
}

function findingId(finding: AuditFinding): string {
  return `${finding.scenario}|${finding.rule}|${finding.key}`;
}
