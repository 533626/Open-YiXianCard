import { sideLabel } from "./data";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { AppState, Side } from "./types";
import { diagnosticConfigSignature } from "./deck-diagnostics";

const ISSUE_ORDER = ["不足8张", "未注册战斗牌", "仅记录牌", "副职不符", "境界不符"] as const;

/**
 * 构筑诊断不再占一个常驻入口：一切正常时它什么都不该说。
 *
 * 只有在检查中、检查失败、或真的查出问题时才出现；正常构筑下返回空串，
 * 由 `audit:ui` 的 below-fold / repeated-text 规则守住"别再多一块常驻面板"。
 */
export function renderDeckDiagnosticPanel(state: AppState): string {
  const status = state.diagnosticStatus;
  const running = status?.state === "running";
  const failed = status?.state === "error";
  const result = state.diagnosticResult?.configSignature === diagnosticConfigSignature(state.config)
    ? state.diagnosticResult
    : null;
  const blocking = result !== null && !result.simulatable;
  if (!running && !failed && !blocking) return "";
  const diagnosticGuide = [
    "构筑诊断",
    "",
    "出现条件：正在检查、检查失败，或存在会阻断战斗/求解的问题。",
    "检查范围：缺牌、未注册战斗牌、仅记录牌、副职不符与境界不符。",
    "分组：问题按玩家列出，便于回到对应构筑修正。",
    "隐藏规则：构筑正常时本面板不占用界面空间。",
  ].join("\n");
  return `
    <section
      class="deck-diagnostic-panel alert"
      aria-label="构筑诊断"
      aria-busy="${running}"
      title="${escapeAttribute(diagnosticGuide)}"
    >
      <header class="deck-diagnostic-alert-head">
        <b>${running ? "构筑诊断中" : failed ? "构筑诊断失败" : `构筑有 ${result!.issueCount} 项问题`}</b>
        ${
    running
      ? `<span>Worker 正在查询注册表、归档与合法性能力</span>`
      : failed
      ? `<span>${escapeHtml(status?.message ?? "诊断失败")}</span>`
      : ""
  }
        ${running ? '<span class="solver-spinner" aria-hidden="true"></span>' : ""}
      </header>
      ${blocking ? renderResult(result!) : ""}
    </section>
  `;
}

function renderResult(result: NonNullable<AppState["diagnosticResult"]>): string {
  return `
    <div class="diagnostic-sides">
      ${renderSide(result, "p1")}${renderSide(result, "p2")}
    </div>
  `;
}

function renderSide(result: NonNullable<AppState["diagnosticResult"]>, side: Side): string {
  const report = result.sides[side];
  return `
    <section class="diagnostic-side ${report.issues.length ? "has-issues" : "ready"}">
      <header><b>${escapeHtml(sideLabel(side))}</b><span>${report.effectiveCount}/8</span></header>
      ${report.issues.length === 0
        ? ""
        : ISSUE_ORDER.map((kind) => {
          const issues = report.issues.filter((issue) => issue.kind === kind);
          if (issues.length === 0) return "";
          return `<div class="diagnostic-group"><b>${kind}</b>${issues.map((issue) => `<span>${escapeHtml(issue.detail)}</span>`).join("")}</div>`;
        }).join("")}
    </section>
  `;
}
