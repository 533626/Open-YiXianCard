import {
  normalizeBaseId,
  type OriginalReplayPlayerFixture,
} from "./domain";
import { archiveByBaseId } from "./data/source";
import { buildReplayFixture } from "./replay-fixture-builder";
import type { BattleConfig, Side } from "./types";

export type DeckIssueKind =
  | "不足8张"
  | "未注册战斗牌"
  | "仅记录牌"
  | "副职不符"
  | "境界不符";

export interface DeckIssue {
  readonly kind: DeckIssueKind;
  readonly side: Side;
  readonly cardBaseId?: number;
  readonly cardName?: string;
  readonly slot?: number;
  readonly detail: string;
}

export interface DeckDiagnosticSideResult {
  readonly side: Side;
  readonly configuredCount: number;
  readonly effectiveCount: number;
  readonly issues: readonly DeckIssue[];
  readonly simulatable: boolean;
}

export interface DeckDiagnosticResult {
  readonly configSignature: string;
  readonly sides: Readonly<Record<Side, DeckDiagnosticSideResult>>;
  readonly issueCount: number;
  readonly simulatable: boolean;
}

const REALM_LEVEL: Readonly<Record<string, number>> = {
  LianQi: 1,
  ZhuJi: 2,
  JinDan: 3,
  YuanYing: 4,
  HuaShen: 5,
  FanXu: 6,
};

/** Worker-side capability query. It classifies archive/registry metadata; it does not reproduce battle rules. */
export function diagnoseBattleConfig(config: BattleConfig): DeckDiagnosticResult {
  const fixture = buildReplayFixture(config);
  const p1 = diagnoseSide(config, "p1", fixture.players.p1);
  const p2 = diagnoseSide(config, "p2", fixture.players.p2);
  const issueCount = p1.issues.length + p2.issues.length;
  return {
    configSignature: diagnosticConfigSignature(config),
    sides: { p1, p2 },
    issueCount,
    simulatable: p1.simulatable && p2.simulatable,
  };
}

export function diagnosticConfigSignature(config: BattleConfig): string {
  return JSON.stringify(config);
}

function diagnoseSide(
  config: BattleConfig,
  side: Side,
  fixture: OriginalReplayPlayerFixture,
): DeckDiagnosticSideResult {
  const player = config.players[side];
  const configured = player.deck
    .map((slot, index) => ({ slot, index }))
    .filter(({ slot }) => slot.baseId > 0);
  const generatedCount = predictedDeckStartFillCount(player);
  const effectiveCount = configured.length + generatedCount;
  const issues: DeckIssue[] = [];
  if (effectiveCount < 8) {
    issues.push({
      kind: "不足8张",
      side,
      detail: `当前配置 ${configured.length} 张，开战后 ${effectiveCount}/8，仍需补 ${8 - effectiveCount} 张`,
    });
  }

  for (const { slot, index } of configured) {
    const baseId = normalizeBaseId(slot.baseId);
    const archive = archiveByBaseId.get(baseId);
    const cardName = archive?.name ?? `卡牌 ${baseId}`;
    const common = { side, cardBaseId: baseId, cardName, slot: index + 1 } as const;
    if (!archive || (archive.simulationScope === "battle" && !archive.registered)) {
      issues.push({ ...common, kind: "未注册战斗牌", detail: `槽位 ${index + 1} · ${cardName}：Rust 引擎无已注册战斗效果，无法模拟` });
      continue;
    }
    if (archive.simulationScope === "record-only") {
      issues.push({ ...common, kind: "仅记录牌", detail: `槽位 ${index + 1} · ${cardName}：属于战斗外机制，仅记录结果` });
      continue;
    }
    if (archive.archiveKind === "career" && archive.career) {
      const isPrimary = archive.career === player.careerName;
      const isDual = Object.values(player.dualCareerNames).includes(archive.career);
      if (!isPrimary && !isDual) {
        issues.push({ ...common, kind: "副职不符", detail: `槽位 ${index + 1} · ${cardName}：需要 ${archive.archiveLabel}，当前副职为 ${player.careerName ?? "未选择"}` });
      }
    }
    const requiredLevel = archive.realm ? REALM_LEVEL[archive.realm] : undefined;
    if (requiredLevel !== undefined && requiredLevel > player.level) {
      issues.push({ ...common, kind: "境界不符", detail: `槽位 ${index + 1} · ${cardName}：需要 ${archive.realmLabel ?? archive.realm}，当前境界等级 ${player.level}` });
    }
  }

  const blocking = issues.some((issue) =>
    issue.kind === "不足8张" || issue.kind === "未注册战斗牌" || issue.kind === "仅记录牌"
  );
  return {
    side,
    configuredCount: configured.length,
    effectiveCount,
    issues,
    simulatable: !blocking,
  };
}

/** Display-only preflight for public deck builders; Rust remains the battle implementation. */
function predictedDeckStartFillCount(player: BattleConfig["players"][Side]): number {
  const emptySlots = player.deck.filter((slot) => slot.baseId === 0).length;
  if (emptySlots === 0) return 0;
  const sevenStarsFill = player.talents.includes(52) && player.activeSlotCount >= 8;
  const loneVoidFill =
    player.deck.some((slot) => normalizeBaseId(slot.baseId) === 215) &&
    (player.talents.includes(198) || player.fateStrategies.includes(338));
  return sevenStarsFill || loneVoidFill ? 1 : 0;
}
