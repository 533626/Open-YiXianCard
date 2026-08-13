import { normalizeBaseId, type RuleEvent } from "./domain";
import {
  battleEventFateStrategyLabels,
  battleEventTalentLabels,
} from "./generated/battle-event-labels";
import { archiveByBaseId } from "./data/source";
import {
  permanentSourceLabel,
  resourceSourceLabel,
  sourceRootLabel,
  sourceTokenLabel,
} from "./battle-event-source-registry";

export type HookCategoryId =
  | "turn"
  | "select"
  | "main"
  | "after"
  | "again"
  | "status"
  | "queue"
  | "check"
  | "shared";

export interface HookCategory {
  readonly id: HookCategoryId;
}

export const HOOK_CATEGORIES: Readonly<Record<HookCategoryId, HookCategory>> = Object.freeze({
  turn: { id: "turn" },
  select: { id: "select" },
  main: { id: "main" },
  after: { id: "after" },
  again: { id: "again" },
  status: { id: "status" },
  queue: { id: "queue" },
  check: { id: "check" },
  shared: { id: "shared" },
});

const NEGATIVE_STATUS_NAMES = new Set([
  "internalInjury",
  "weakness",
  "flaw",
  "attackReduction",
  "entangle",
  "externalInjury",
]);

/** Explicit source-key registry. Unknown keys are audited, never guessed. */
export const unmappedSourceKeys = new Set<string>();

export function hookCategoryForEvent(event: RuleEvent): HookCategory {
  if (event.type === "phase") {
    if (event.name.startsWith("actionAgain")) return HOOK_CATEGORIES.again;
    return HOOK_CATEGORIES.turn;
  }
  if (event.type === "checkpoint") return HOOK_CATEGORIES.check;
  if (event.type === "queue") return HOOK_CATEGORIES.queue;
  if (event.type === "damage" || event.type === "guard") {
    return hookCategoryForSource(eventSource(event), event);
  }
  if (event.type === "buff" && isNegativeStatus(event.name)) {
    return HOOK_CATEGORIES.status;
  }
  if (event.type === "card") {
    if (event.name === "cardSelected" || event.name === "animaShortage") {
      return HOOK_CATEGORIES.select;
    }
    if (event.name === "temporaryUpgrade") return HOOK_CATEGORIES.queue;
    if (event.name === "effectBefore" || event.name === "effectAfter") {
      return HOOK_CATEGORIES.main;
    }
    if (event.name === "cardCompleted") return HOOK_CATEGORIES.after;
  }
  return hookCategoryForSource(eventSource(event), event);
}

export function eventSource(event: RuleEvent): string | null {
  const source = event.detail?.source;
  return typeof source === "string" ? source : null;
}

export function sourceLabel(source: string | null): string {
  if (!source) return "";
  const resolved = resolveSource(source);
  if (!resolved.mapped) unmappedSourceKeys.add(source);
  return resolved.label;
}

export function isSourceMapped(source: string | null): boolean {
  return !source || resolveSource(source).mapped;
}

interface SourceResolution {
  readonly label: string;
  readonly mapped: boolean;
}

interface SourceIdentity {
  readonly label: string;
  readonly mapped: boolean;
}

function resolveSource(source: string): SourceResolution {
  const [root, ...parts] = source.split(":");
  if (root === "card") return entitySource("牌面", cardIdentity(parts[0]), parts.slice(1));
  if (root === "talent") return entitySource("仙命", talentIdentity(parts[0]), parts.slice(1));
  if (root === "fateStrategy") return entitySource("天衍", fateStrategyIdentity(parts[0]), parts.slice(1));
  if (root === "buff") {
    const label = sourceTokenLabel(parts[0] ?? "");
    return entitySource(
      "状态",
      { label: label ?? "状态触发", mapped: label !== null },
      parts.slice(1),
    );
  }
  if (root === "permanentBuff") {
    const label = permanentSourceLabel(parts[0] ?? "");
    return entitySource(
      "永久",
      { label: label ?? "永久效果", mapped: label !== null },
      parts.slice(1),
    );
  }
  if (root === "damage") {
    const label = resourceSourceLabel(parts[0] ?? "");
    return entitySource(
      "伤害",
      { label: label ?? "伤害结算", mapped: label !== null },
      parts.slice(1),
    );
  }
  if (root === "cardCost") return validateHiddenSuffix("费用", parts);
  if (root === "cardSelected") return validateHiddenSuffix("选中", parts);
  if (root === "cardCompleted") return joinSourceParts("收尾", parts);
  if (root === "effectAfter") return joinSourceParts("效果后", parts);

  const rootLabel = sourceRootLabel(root ?? "");
  if (rootLabel) return joinSourceParts(rootLabel, parts);
  return { label: "机制来源", mapped: false };
}

function entitySource(
  prefix: string,
  identity: SourceIdentity,
  suffix: readonly string[],
): SourceResolution {
  const head = identity.label.endsWith("效果")
    ? identity.label
    : `${prefix} ${identity.label}`;
  const resolvedSuffix = sourceSuffixLabels(suffix);
  return {
    label: resolvedSuffix.labels.length > 0
      ? `${head} · ${resolvedSuffix.labels.join(" · ")}`
      : head,
    mapped: identity.mapped && resolvedSuffix.mapped,
  };
}

function joinSourceParts(head: string, parts: readonly string[]): SourceResolution {
  const suffix = sourceSuffixLabels(parts);
  return {
    label: suffix.labels.length > 0 ? `${head} · ${suffix.labels.join(" · ")}` : head,
    mapped: suffix.mapped,
  };
}

function validateHiddenSuffix(
  label: string,
  parts: readonly string[],
): SourceResolution {
  return { label, mapped: sourceSuffixLabels(parts).mapped };
}

function sourceSuffixLabels(parts: readonly string[]): {
  readonly labels: string[];
  readonly mapped: boolean;
} {
  const labels: string[] = [];
  let mapped = true;
  for (let index = 0; index < parts.length; index += 1) {
    const token = parts[index]!;
    if (token === "card") {
      const identity = cardIdentity(parts[index + 1]);
      labels.push(`牌面 ${identity.label}`);
      mapped &&= identity.mapped;
      index += 1;
      continue;
    }
    if (token === "talent") {
      const identity = talentIdentity(parts[index + 1]);
      labels.push(`仙命 ${identity.label}`);
      mapped &&= identity.mapped;
      index += 1;
      continue;
    }
    if (token === "fateStrategy") {
      const identity = fateStrategyIdentity(parts[index + 1]);
      labels.push(`天衍 ${identity.label}`);
      mapped &&= identity.mapped;
      index += 1;
      continue;
    }
    if (token === "buff") {
      const label = sourceTokenLabel(parts[index + 1] ?? "");
      labels.push(`状态 ${label ?? "状态触发"}`);
      mapped &&= label !== null;
      index += 1;
      continue;
    }
    const label = sourceTokenLabel(token);
    if (label) {
      labels.push(label);
    } else if (/^\d+$/.test(token)) {
      const identity = cardIdentity(token);
      labels.push(`牌面 ${identity.label}`);
      mapped &&= identity.mapped;
    } else {
      mapped = false;
    }
  }
  return { labels, mapped };
}

function talentIdentity(key: string | undefined): SourceIdentity {
  const id = Number(key);
  if (key && Number.isFinite(id)) {
    const label = battleEventTalentLabels.get(id);
    return { label: label ?? "仙命效果", mapped: label !== undefined };
  }
  const label = sourceTokenLabel(key ?? "");
  return { label: label ?? "仙命效果", mapped: label !== null };
}

function fateStrategyIdentity(key: string | undefined): SourceIdentity {
  const id = Number(key);
  if (!key || !Number.isFinite(id)) return { label: "天衍效果", mapped: false };
  const label = battleEventFateStrategyLabels.get(id);
  return { label: label ?? "天衍效果", mapped: label !== undefined };
}

function cardIdentity(key: string | undefined): SourceIdentity {
  const id = Number(key);
  if (key && Number.isFinite(id)) {
    const label = archiveByBaseId.get(normalizeBaseId(id))?.name;
    return { label: label ?? "牌面效果", mapped: label !== undefined };
  }
  const label = sourceTokenLabel(key ?? "");
  return { label: label ?? "牌面效果", mapped: label !== null };
}

export function isNegativeStatus(name: string): boolean {
  return NEGATIVE_STATUS_NAMES.has(name);
}

function hookCategoryForSource(source: string | null, event: RuleEvent): HookCategory {
  if (!source) {
    return event.type === "buff" && isNegativeStatus(event.name)
      ? HOOK_CATEGORIES.status
      : HOOK_CATEGORIES.shared;
  }
  if (source.startsWith("turnStart:") || source.startsWith("turnEnd:") || source.startsWith("permanentBuff:")) {
    return HOOK_CATEGORIES.turn;
  }
  if (source.startsWith("cardCost:") || source.startsWith("cardSelected:")) {
    return HOOK_CATEGORIES.select;
  }
  if (source.includes("spiritFormationEcho") || source.includes("plumBlossomTwice")) {
    return HOOK_CATEGORIES.main;
  }
  if (source.startsWith("actionAgain:") || source.includes(":actionAgain")) {
    return HOOK_CATEGORIES.again;
  }
  if (source.startsWith("cardCompleted:") || source.startsWith("effectAfter:")) {
    return HOOK_CATEGORIES.after;
  }
  if (source.startsWith("card:")) return HOOK_CATEGORIES.main;
  if (event.type === "buff" && isNegativeStatus(event.name)) return HOOK_CATEGORIES.status;
  if (event.type === "damage" || event.type === "resource" || event.type === "buff") {
    return HOOK_CATEGORIES.shared;
  }
  return HOOK_CATEGORIES.after;
}
