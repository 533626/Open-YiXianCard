/**
 * Rust canonical 钩子链在浏览器侧的模型。
 *
 * 引擎每次调用钩子都会采样一次双方状态，Rust 侧（`replay/hook_trace.rs`）把相邻
 * 采样相减，只留下这次钩子改动的字段。这里做的是把结果挂到 `BattleFrame.index`
 * 上：钩子步骤带的是事件下标，而战斗视图是按帧渲染的。
 *
 * 这一层不解释规则、不给字段配权重 —— 显示什么由引擎的分类和原文标签决定，
 * 浏览器不能自己发明钩子。
 */

import type { Side } from "./types";

export type HookCategory =
  | "battleStart"
  | "turnStart"
  | "selectCost"
  | "temporaryUpgrade"
  | "mainEffect"
  | "afterCard"
  | "actionAgain"
  | "turnEnd"
  | "battleEnd";

/** 钩子分类的中文名；键必须与 Rust `ReplayHookCategory` 一一对应。 */
export const HOOK_CATEGORY_LABELS: Readonly<Record<HookCategory, string>> = {
  battleStart: "战斗开始",
  turnStart: "回合开始",
  selectCost: "选牌与费用",
  temporaryUpgrade: "临时升级",
  mainEffect: "牌面结算",
  afterCard: "牌后结算",
  actionAgain: "再动判定",
  turnEnd: "回合结束",
  battleEnd: "战斗结束",
};

export interface RustHookTraceChange {
  readonly group: string;
  readonly key: string;
  readonly label: string;
  readonly before: number;
  readonly after: number;
}

export interface RustHookTraceAttackSegment {
  readonly eventIndex: number;
  readonly target: Side;
  readonly hitIndex: number;
  readonly hpBefore: number;
  readonly hpAfter: number;
  readonly defBefore: number;
  readonly defAfter: number;
}

export interface RustHookTraceStep {
  readonly eventIndex: number;
  readonly category: HookCategory;
  readonly turn: number;
  readonly actor: Side;
  readonly slot: number | null;
  readonly cardId: number | null;
  readonly cardName: string | null;
  readonly p1Changes: readonly RustHookTraceChange[];
  readonly p2Changes: readonly RustHookTraceChange[];
  readonly attackSegments?: readonly RustHookTraceAttackSegment[];
}

export interface RustHookTrace {
  readonly steps: readonly RustHookTraceStep[];
}

export interface HookFieldChange extends RustHookTraceChange {
  readonly side: Side;
}

export interface HookAttackSegment {
  readonly target: Side;
  readonly hitIndex: number;
  readonly hpBefore: number;
  readonly hpAfter: number;
  readonly defBefore: number;
  readonly defAfter: number;
}

export interface HookStep {
  /** 已映射到 `BattleFrame.index`。 */
  readonly frameIndex: number;
  readonly category: HookCategory;
  readonly categoryLabel: string;
  readonly actorTurn: number;
  readonly actor: Side;
  readonly slot: number | null;
  readonly cardId: number | null;
  readonly cardName: string | null;
  readonly changes: readonly HookFieldChange[];
  /** 逐段攻击采样：仅 MainEffect 钩子有多段攻击牌的逐段 hp/防 before→after。 */
  readonly attackSegments: readonly HookAttackSegment[];
}

/**
 * 事件下标 i 对应帧 i（帧 0 就是战斗开始结算帧，Rust 路径不再前置战前占位帧）。
 * 战斗结束帧不进战斗视图，而 parity 事件流会省掉未走完那一回合的回合结束边界，
 * 所以落在帧序列之外的步骤直接丢掉 —— 平移下标会把钩子记到别的牌上。
 */
export function adaptHookTrace(
  trace: RustHookTrace,
  frameCount: number,
): readonly HookStep[] {
  const steps: HookStep[] = [];
  for (const step of trace.steps) {
    const frameIndex = step.eventIndex;
    if (frameIndex >= frameCount) continue;
    steps.push({
      frameIndex,
      category: step.category,
      categoryLabel: HOOK_CATEGORY_LABELS[step.category] ?? step.category,
      actorTurn: step.turn,
      actor: step.actor,
      slot: step.slot,
      cardId: step.cardId,
      cardName: step.cardName,
      changes: [
        ...step.p1Changes.map((change) => ({ ...change, side: "p1" as const })),
        ...step.p2Changes.map((change) => ({ ...change, side: "p2" as const })),
      ],
      attackSegments: (step.attackSegments ?? []).map((segment) => ({
        target: segment.target,
        hitIndex: segment.hitIndex,
        hpBefore: segment.hpBefore,
        hpAfter: segment.hpAfter,
        defBefore: segment.defBefore,
        defAfter: segment.defAfter,
      })),
    });
  }
  return steps;
}

export function hookStepsForFrame(
  steps: readonly HookStep[] | undefined,
  frameIndex: number,
): readonly HookStep[] {
  return (steps ?? []).filter((step) => step.frameIndex === frameIndex);
}

/**
 * 一次步进的全部钩子：按 actorTurn 取整块。回放方向键在“回合结束”帧之间跳转，
 * 一次步进涵盖一方完整行动（回合开始→出牌→再动→回合结束）的所有钩子，把它们
 * 整块渲染相当于把原来按 actorTurn 分组的整页轮播。
 */
export function hookStepsForActorTurn(
  steps: readonly HookStep[] | undefined,
  actorTurn: number,
): readonly HookStep[] {
  return (steps ?? []).filter((step) => step.actorTurn === actorTurn);
}

export interface HookTurnGroup {
  readonly actorTurn: number;
  readonly steps: readonly HookStep[];
}

export function hookStepsByActorTurn(
  steps: readonly HookStep[] | undefined,
): readonly HookTurnGroup[] {
  const groups = new Map<number, HookStep[]>();
  for (const step of steps ?? []) {
    const list = groups.get(step.actorTurn) ?? [];
    list.push(step);
    groups.set(step.actorTurn, list);
  }
  return [...groups.entries()].map(([actorTurn, grouped]) => ({ actorTurn, steps: grouped }));
}

export function changedFieldCount(steps: readonly HookStep[]): number {
  return steps.reduce((total, step) => total + step.changes.length, 0);
}
