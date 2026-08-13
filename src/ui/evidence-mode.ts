import { configMatchesImportedFixture } from "./fixture-contract";
import type { AppState } from "./types";

export interface WorkbenchEvidenceMode {
  readonly kind: "evidence" | "sandbox";
  readonly label: string;
  readonly detail: string;
}

export function workbenchEvidenceMode(state: AppState): WorkbenchEvidenceMode {
  const importedUnchanged = state.importedFixture
    ? safelyMatchesImportedFixture(state)
    : false;
  if (importedUnchanged && state.importedFixtureOrigin === "catalog") {
    return {
      kind: "evidence",
      label: "原作证据模式",
      detail: "准入目录回放；未展示未核实 build",
    };
  }
  if (importedUnchanged && state.importedFixtureOrigin === "local") {
    return {
      kind: "sandbox",
      label: "研究沙盒 · 本地回放未认证",
      detail: "版本化本地输入；尚未由本站证明准入身份",
    };
  }
  if (state.importedFixture) {
    return {
      kind: "sandbox",
      label: "研究沙盒 · 已修改回放",
      detail: "当前构筑已偏离导入证据",
    };
  }
  return {
    kind: "sandbox",
    label: "研究沙盒 · 未认证",
    detail: "手工自由构筑；不代表原作可达或合法",
  };
}

function safelyMatchesImportedFixture(state: AppState): boolean {
  try {
    return Boolean(
      state.importedFixture &&
      configMatchesImportedFixture(state.importedFixture, state.config)
    );
  } catch {
    return false;
  }
}
