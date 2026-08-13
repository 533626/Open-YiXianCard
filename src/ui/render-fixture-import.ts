import {
  filterFixtureEntries,
  fixtureCatalogEntries,
  fixtureEntryById,
  fixtureOptionLabel,
  type UiFixtureEntry,
} from "./fixture-catalog";
import {
  escapeAttribute,
  escapeHtml,
} from "./view-utils";
import type { AppState } from "./types";
import {
  normalizeRecordCode,
  replayMatchesRecordCode,
} from "./replay-record-code";
import {
  LINUX_REPLAY_DATA_PATH,
  USER_AGENT_REPLAY_IMPORT_PROMPT,
  WINDOWS_REPLAY_DATA_PATH,
} from "./replay-import-guide";
import { CHARACTER_BY_ID } from "./data";

export function renderFixtureImportPanel(
  state: AppState,
  catalog: readonly UiFixtureEntry[] = fixtureCatalogEntries(),
): string {
  if (!state.fixtureImportOpen) return "";
  const query = state.fixtureImportQuery ?? "";
  const selectedId = state.fixtureImportId ?? "";
  const matches = filterFixtureEntries(query, 80, catalog);
  const selected = fixtureEntryById(selectedId, catalog) ?? matches[0] ?? null;
  const recentIds = (state.recentFixtureIds ?? [])
    .filter((id) => fixtureEntryById(id, catalog));
  const catalogAvailable = state.replayImportDeveloperMode === true && catalog.length > 0;
  const activeTab = state.replayImportTab ?? "code";
  const candidates = matchingReplayCandidates(
    state.replayImportCandidates ?? [],
    activeTab === "code" ? state.replayImportCode ?? "" : "",
  );
  const importGuide = [
    "导入对局",
    "",
    "战绩码：在你授权的原版缓存目录里匹配，不请求原版服务器。",
    "本机记录：原始 .bin 在浏览器 Worker 中解码，不会上传。",
    "对局包：兼容 Open-YiXianCard 版本化 JSON。",
    "导入内容包括角色、构筑、手牌、初始状态与对局结果；本地结果保持未认证。",
  ].join("\n");
  return `
    <section
      class="fixture-import-panel replay-import-dialog"
      role="dialog"
      aria-label="导入对局"
      title="${escapeAttribute(importGuide)}"
      data-replay-dropzone="1"
    >
      <header class="replay-import-head">
        <div>
          <b>导入对局</b>
          <span>文件只在当前浏览器本地解析，不会上传</span>
        </div>
        <button type="button" class="fixture-import-close" data-action="toggle-fixture-import" title="关闭导入" aria-label="关闭导入">×</button>
      </header>
      <nav class="replay-import-tabs" aria-label="导入方式">
        ${renderImportTab("code", "战绩码", activeTab)}
        ${renderImportTab("computer", "本机记录", activeTab)}
        ${renderImportTab("package", "对局包", activeTab)}
      </nav>
      <div class="replay-import-body">
        ${renderImportBody(state, activeTab, candidates)}
      </div>
      ${renderAgentGuide()}
      ${catalogAvailable
        ? `
          <details class="fixture-dev-catalog">
            <summary>回放实验室 <span>?devReplay=1 · ${catalog.length} 项工程证据</span></summary>
            <div class="fixture-catalog-controls">
              <input
                type="search"
                class="fixture-search"
                value="${escapeAttribute(query)}"
                placeholder="搜索开发回放编号"
                aria-label="搜索开发回放编号"
                data-fixture-query="1"
                list="fixture-import-options"
                autocomplete="off"
              />
              <datalist id="fixture-import-options">
                ${matches.map((entry) => `
                  <option value="${escapeAttribute(entry.id)}">${escapeHtml(fixtureOptionLabel(entry))}</option>
                `).join("")}
              </datalist>
              <button type="button" class="primary fixture-import-submit" data-action="import-fixture" ${selected ? "" : "disabled"}>导入</button>
              <button type="button" class="primary fixture-import-run" data-action="import-fixture-and-run" ${selected ? "" : "disabled"}>导入并战斗</button>
            </div>
          </details>
        `
        : ""}
      ${catalogAvailable && recentIds.length > 0 ? `
        <nav class="fixture-recent-row" aria-label="最近回放">
          <b>最近</b>
          ${recentIds.map((id) => `
            <button type="button" data-action="quick-fixture" data-fixture-id="${escapeAttribute(id)}" title="导入并开始战斗">${escapeHtml(id)}</button>
          `).join("")}
        </nav>
      ` : ""}
    </section>
  `;
}

function renderImportTab(
  id: NonNullable<AppState["replayImportTab"]>,
  label: string,
  active: NonNullable<AppState["replayImportTab"]>,
): string {
  return `
    <button
      type="button"
      class="${id === active ? "active" : ""}"
      data-action="set-replay-import-tab"
      data-import-tab="${id}"
      aria-selected="${id === active ? "true" : "false"}"
    >${label}</button>
  `;
}

function renderImportBody(
  state: AppState,
  activeTab: NonNullable<AppState["replayImportTab"]>,
  candidates: readonly NonNullable<AppState["replayImportCandidates"]>[number][],
): string {
  if (activeTab === "package") {
    return `
      <div class="replay-package-import">
        <div>
          <b>Open-YiXianCard 对局包</b>
          <span>用于跨设备、分享或故障排查的版本化 JSON</span>
        </div>
        <label class="primary fixture-file-pick">
          选择对局包
          <input
            type="file"
            name="replay-package"
            accept=".json,.yixian-replay.json,application/json"
            data-local-replay-file="1"
          />
        </label>
        <span class="fixture-file-limit">≤ 5 MiB</span>
      </div>
    `;
  }

  const code = state.replayImportCode ?? "";
  const status = state.replayImportStatus;
  return `
    ${activeTab === "code"
      ? `
        <div class="replay-code-row">
          <label for="replay-import-code">原版战绩码</label>
          <input
            id="replay-import-code"
            name="replay-code"
            type="text"
            value="${escapeAttribute(code)}"
            placeholder="粘贴展示码或短码"
            autocomplete="off"
            spellcheck="false"
            data-replay-import-code="1"
          />
          <span>匹配已授权的本机缓存；不连接原版服务器</span>
        </div>
      `
      : `
        <div class="replay-local-copy">
          <b>选择原版数据目录或单个 .bin</b>
          <span>一份记录可包含多轮斗法，解码后再明确选择轮次</span>
        </div>
      `}
    <div class="replay-source-actions">
      <label class="primary fixture-file-pick">
        选择弈仙牌文件夹
        <input
          type="file"
          name="replay-directory"
          accept=".bin,application/octet-stream"
          multiple
          webkitdirectory
          directory
          data-original-replay-directory="1"
        />
      </label>
      <label class="fixture-file-pick secondary">
        选择 .bin
        <input
          type="file"
          name="replay-files"
          accept=".bin,application/octet-stream"
          multiple
          data-original-replay-files="1"
        />
      </label>
      <span class="replay-path-copy">
        <button type="button" data-copy-replay-path="${escapeAttribute(WINDOWS_REPLAY_DATA_PATH)}">复制 Windows 路径</button>
        <code>${escapeHtml(WINDOWS_REPLAY_DATA_PATH)}</code>
      </span>
      <span class="replay-path-copy">
        <button type="button" data-copy-replay-path="${escapeAttribute(LINUX_REPLAY_DATA_PATH)}">复制 Linux 路径</button>
        <code>${escapeHtml(LINUX_REPLAY_DATA_PATH)}</code>
      </span>
    </div>
    ${status ? `<div class="replay-import-status ${status.state}" role="status">${escapeHtml(status.message)}</div>` : ""}
    ${renderCandidateList(candidates, activeTab === "code" ? normalizeRecordCode(code) : "")}
  `;
}

function renderCandidateList(
  candidates: readonly NonNullable<AppState["replayImportCandidates"]>[number][],
  code: string,
): string {
  if (candidates.length === 0) {
    return `
      <div class="replay-candidate-empty">
        <b>${code ? "当前缓存中没有匹配战绩" : "尚未读取本机记录"}</b>
        <span>${code
    ? "请先在原版客户端打开该战绩，使其写入下载缓存，再重新选择目录。"
    : "选择 YiXianPai 文件夹后，本站只读取其中的对局 .bin。"
  }</span>
      </div>
    `;
  }
  return `
    <div class="replay-candidate-list" aria-label="可导入轮次">
      ${candidates.slice(0, 120).map((candidate) => {
    const p1 = CHARACTER_BY_ID.get(candidate.p1CharacterId)?.name ?? `角色 ${candidate.p1CharacterId}`;
    const p2 = CHARACTER_BY_ID.get(candidate.p2CharacterId)?.name ?? `角色 ${candidate.p2CharacterId}`;
    const first = candidate.firstPlayerSide === "p1" ? p1 : p2;
    const winner = candidate.winnerSide === "p1" ? p1 : p2;
    return `
          <article class="replay-candidate">
            <div class="replay-candidate-main">
              <b>第 ${candidate.round} 轮 · ${escapeHtml(p1)} 对 ${escapeHtml(p2)}</b>
              <span>${escapeHtml(formatRecordTime(candidate.recordTimestamp))} · ${escapeHtml(candidate.gameVersion || "版本未知")}</span>
            </div>
            <div class="replay-candidate-result">
              <span>先手 ${escapeHtml(first)}</span>
              <span>胜者 ${escapeHtml(winner)}</span>
              <span>T${candidate.actorTurnCount}</span>
              <span>HPΔ ${formatSigned(candidate.hpDeltaP1MinusP2)}</span>
            </div>
            <button
              type="button"
              class="primary"
              data-action="import-local-replay-round"
              data-replay-candidate-id="${escapeAttribute(candidate.id)}"
            >导入此轮</button>
          </article>
        `;
  }).join("")}
    </div>
  `;
}

function renderAgentGuide(): string {
  return `
    <details class="replay-agent-guide">
      <summary>找不到缓存？让你的 AI 助手协助</summary>
      <div class="replay-agent-guide-head">
        <span>说明保留 Windows / Linux 路径、PowerShell / Bash 格式和只读隐私边界。</span>
        <button type="button" data-copy-agent-guide="1">复制给 AI 助手</button>
      </div>
      <pre>${escapeHtml(USER_AGENT_REPLAY_IMPORT_PROMPT)}</pre>
    </details>
  `;
}

export function matchingReplayCandidates(
  candidates: readonly NonNullable<AppState["replayImportCandidates"]>[number][],
  code: string,
): readonly NonNullable<AppState["replayImportCandidates"]>[number][] {
  const normalized = normalizeRecordCode(code);
  if (!normalized) return candidates;
  return candidates.filter((candidate) =>
    replayMatchesRecordCode({ recordCodes: candidate.recordCodes }, normalized));
}

function formatRecordTime(value: number | null): string {
  if (value === null || !Number.isFinite(value)) return "时间未知";
  return new Intl.DateTimeFormat("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
    hour12: false,
  }).format(new Date(value));
}

export function renderFixtureConsistencyBadge(state: AppState): string {
  const report = state.fixtureConsistency;
  if (!report) return "";
  const ok = report.engineMatch && report.expectedMatch !== false;
  const localInput = state.importedFixtureOrigin === "local";
  const expectedText = localInput
    ? report.expectedMatch === undefined
      ? "本地输入无 expected · 未认证"
      : report.expectedMatch
        ? "输入 expected 匹配 · 未认证"
        : "输入 expected 不匹配 · 未认证"
    : report.expectedMatch === undefined
      ? "无 expected"
      : report.expectedMatch ? "expected exact" : "expected mismatch";
  const expectedProvenance = localInput
    ? "expected 来源：用户本地输入；未经过原作回放准入认证"
    : "expected 来源：准入 catalog fixture";
  const title = [
    report.engineMatch ? "Engine-vs-UI exact" : "Engine-vs-UI mismatch",
    `UI: ${runSummary(report.ui)}`,
    `Engine: ${runSummary(report.engine)}`,
    expectedText,
    expectedProvenance,
  ].join(" | ");
  return `
    <span class="fixture-consistency ${ok ? "ok" : "bad"}" title="${escapeAttribute(title)}">
      ${escapeHtml(expectedText)}
    </span>
  `;
}

function runSummary(summary: {
  readonly winnerSide: string | null;
  readonly actorTurnCount: number;
  readonly hpDeltaP1MinusP2: number;
  readonly finalHp: Readonly<Record<"p1" | "p2", number>>;
}): string {
  return `winner ${summary.winnerSide ?? "-"}, T${summary.actorTurnCount}, HPΔ ${formatSigned(summary.hpDeltaP1MinusP2)}, final ${summary.finalHp.p1}/${summary.finalHp.p2}`;
}

function formatSigned(value: number): string {
  return value > 0 ? `+${value}` : String(value);
}
