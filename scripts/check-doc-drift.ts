import { Glob } from "bun";
import { readFile } from "node:fs/promises";
import { join } from "node:path";
import { findUnanchoredOpenItems } from "./lib/doc-open-item";

const repoRoot = join(import.meta.dir, "..");
const failures: string[] = [];

const read = (path: string) => readFile(join(repoRoot, path), "utf8");
const manifest = JSON.parse(
  await read("research/original-game/EVIDENCE_MANIFEST.json"),
) as { steamBuild?: string };

if (!manifest.steamBuild) {
  failures.push("EVIDENCE_MANIFEST.json 缺少 steamBuild");
}

const agentContext = await read("docs/AGENT_CONTEXT.md");
if (manifest.steamBuild && !agentContext.includes(`Steam build ${manifest.steamBuild}`)) {
  failures.push(
    `docs/AGENT_CONTEXT.md 未声明当前 manifest build ${manifest.steamBuild}`,
  );
}

const publicBuildDocs = ["README.md"];
for (const path of publicBuildDocs) {
  const content = await read(path);
  if (manifest.steamBuild && !content.includes(`Steam build **${manifest.steamBuild}**`)) {
    failures.push(`${path} 未声明当前 manifest build ${manifest.steamBuild}`);
  }
}

const activeIndexPath = "research/original-game/BATTLE_RULE_INDEX.md";
const activeIndex = await read(activeIndexPath);
const archivePath = "docs/archive/research/BATTLE_RULE_INDEX-23663139.md";
const archivePresent = await read(archivePath).then(() => true).catch(() => false);
if (archivePresent && !activeIndex.includes(archivePath)) {
  failures.push(`${activeIndexPath} 未链接历史索引`);
}
if (activeIndex.split("\n").length > 120) {
  failures.push(`${activeIndexPath} 超过 120 行，应把静态盘点移入 archive`);
}
if (/Evidence snapshot|Initial evidence snapshot|Current card-type rows|Current category counts/.test(activeIndex)) {
  failures.push(`${activeIndexPath} 重新混入历史静态统计`);
}

const currentEntryPaths = [
  "research/original-game/BATTLE_RULE_INDEX.md",
  "research/original-game/SIMPLIFIED_BATTLE_RULES.md",
  "research/original-game/BASE_BATTLE_RULES.md",
];
for (const path of currentEntryPaths) {
  const content = await read(path);
  if (/当前(?:项目)?证据指纹[^\n]*Steam (?:build|构建)[^\n]*\d{7,}/.test(content)) {
    failures.push(`${path} 手写了当前 build；应引用 EVIDENCE_MANIFEST.json`);
  }
}

const activeWorkflowPaths = [
  "docs/AGENT_CONTEXT.md",
  "docs/CROSS_LINE_RUNBOOK.md",
];
for (const path of activeWorkflowPaths) {
  const content = await read(path);
  if (/trace:actions|给用户核对逐动表|获得额外基础规则回放/.test(content)) {
    failures.push(`${path} 重新引入已归档的手工逐动研究流程`);
  }
}

// 名词漂移检查（.claude/CLAUDE.md 坑 3：名词必须锚定原文或 ID）。
// 活跃 .md 里 card<id>（名）括注必须包含 CardConfig 原文名；
// bannedTerms 是已捕获漂移词登记表，抓到新漂移就追加一行。
type CardConfigRecord = { id?: number; name?: string };
const cardNameById = new Map<number, string>();
const cardConfigPath = "research/original-game/extracted/current/CardConfig.json";
try {
  const cardConfig = JSON.parse(await read(cardConfigPath)) as { records?: CardConfigRecord[] };
  for (const record of cardConfig.records ?? []) {
    if (typeof record.id === "number" && typeof record.name === "string") {
      cardNameById.set(record.id, record.name);
    }
  }
  if (cardNameById.size === 0) {
    failures.push(`${cardConfigPath} 未解析出任何 id→name，名词检查失效`);
  }
} catch {
  console.log(`documentation drift: optional local evidence cache absent; skipped card-name audit (${cardConfigPath})`);
}

const bannedTerms: Array<{ wrong: string; right: string; note: string }> = [
  { wrong: "万幻破魔掌", right: "万玄破魔掌", note: "card 82 原文名" },
  { wrong: "体质", right: "体魄", note: "physique 原版属性名，localization 仅剧情文案用前者" },
];

const excludedDocPaths = [
  /^docs\/archive\//,
  /^research\/original-game\/(?:out|extracted|inventory|builds)\//,
  /^battle-evaluator\/(?:fixtures|generated|oracle)\//,
  /(?:^|\/)node_modules\//,
  /^engine-rust\/target\//,
  /(?:^|\/)dist\//,
];
const cardMentionPattern = /\bcard[ _:-]?(\d{1,8})\s*[（(]([^（）()\n]{1,32})[）)]/gi;
const livingDocs = new Glob("**/*.md");
for await (const docPath of livingDocs.scan({ cwd: repoRoot })) {
  if (excludedDocPaths.some((pattern) => pattern.test(docPath))) continue;
  const content = await read(docPath);
  for (const { wrong, right, note } of bannedTerms) {
    if (content.includes(wrong)) {
      failures.push(`${docPath} 使用漂移名词「${wrong}」，应为「${right}」（${note}）`);
    }
  }
  for (const mention of content.matchAll(cardMentionPattern)) {
    const cardId = Number(mention[1]);
    const label = mention[2];
    if (!/[一-鿿]/.test(label)) continue;
    const expected = cardNameById.get(cardId);
    if (!expected) {
      failures.push(`${docPath} 引用 card ${cardId}（${label}），CardConfig 无此 id`);
    } else if (!label.includes(expected)) {
      failures.push(`${docPath} 把 card ${cardId} 写成「${label}」，原文名是「${expected}」`);
    }
  }
  // 开放状态标记必须带可验证锚点：留作待办/TODO/未修/尚未* 等没写命令或路径，
  // 换人接手无法核实现状（209 卡点矛盾、构筑数漂移都属此类）。
  for (const violation of findUnanchoredOpenItems(content)) {
    const hint = violation.kind === "section"
      ? "「下一步」段落没有任何命令/路径/链接锚点"
      : `开放状态标记「${violation.marker}」同行没有反引号代码段或链接锚点`;
    failures.push(`${docPath}:${violation.line} ${hint}；写明验证命令/路径，或删掉已过时标记`);
  }
}

// topic 标记校验：docs/*.md（非 archive）必须含已知 <!-- topic: xxx -->
const KNOWN_TOPICS = new Set([
  "replay", "oracle", "mechanism", "runbook",
  "solver", "ga", "value", "value-function",
  "canonical-rule-impact", "action-contribution", "value-audit",
  "lambda", "lambda-log",
  "audit", "provenance",
  "webui", "ui-fixture",
  "product-architecture", "maintenance", "release-boundary",
]);
const topicPattern = /<!--\s*topic:\s*(\S+)\s*-->/g;
// Glob.scan({ cwd: repoRoot }) 返回的路径已带 docs/ 前缀
const TOPIC_EXEMPT_DOCS = new Set(["docs/README.md", "docs/AGENT_CONTEXT.md"]); // 索引/入口文件不要求 topic
for await (const docPath of new Glob("docs/*.md").scan({ cwd: repoRoot })) {
  const content = await read(docPath);
  const matches = [...content.matchAll(topicPattern)];
  if (TOPIC_EXEMPT_DOCS.has(docPath)) continue;
  if (matches.length === 0) {
    failures.push(`${docPath} 缺少 <!-- topic: xxx -->`);
  } else if (matches.length > 1) {
    failures.push(`${docPath} 有 ${matches.length} 个 topic 标记，每文件应只有一个`);
  } else {
    const topic = matches[0]![1]!;
    if (!KNOWN_TOPICS.has(topic)) {
      failures.push(`${docPath} topic "${topic}" 不在已知列表中`);
    }
  }
}

if (failures.length > 0) {
  console.error("documentation drift check failed:");
  for (const failure of failures) console.error(`- ${failure}`);
  process.exit(1);
}

console.log(`documentation drift check passed (Steam build ${manifest.steamBuild})`);
