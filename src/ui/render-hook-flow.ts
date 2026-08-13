import { changedFieldCount, hookStepsForActorTurn } from "./hook-trace";
import { battleRound } from "./render-battle-progress";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { HookAttackSegment, HookFieldChange, HookStep } from "./hook-trace";
import type { AppState, BattleFrame, Side } from "./types";

export type SideNames = Readonly<Record<Side, string>>;

/**
 * 引擎透视：一次步进的全部 Rust canonical 钩子。
 *
 * 回放方向键在“回合结束”帧之间跳转，一次步进 = 一方完整行动（回合开始→出牌→
 * 再动→回合结束）的全部钩子。把这一整块渲染出来，相当于把原来按 actorTurn
 * 分组的整页轮播——左侧时间轴的圆点选哪一动，右侧就翻到哪一页。
 *
 * 没有改动的钩子也照样列出来 —— 要看的是“引擎实际走了哪些钩子”，把空钩子
 * 藏掉就变成了“看起来引擎没做这一步”。
 */
export function renderEngineInsightModule(
  state: AppState,
  frame: BattleFrame,
  names: SideNames,
  actionCount: number,
): string {
  const steps = hookStepsForActorTurn(state.result?.hookSteps, frame.actorTurn);
  const round = battleRound(frame.actorTurn);
  // 标题只标回合；行动方在左侧时间轴圆点 tooltip，打出的牌在下方钩子链里高亮，头部不重复。
  const subject = frame.actorTurn === 0 ? "战斗开始结算" : `第 ${round} 回合`;
  return `
    <section class="battle-module engine-insight" id="engine-insight" aria-label="引擎透视">
      <header class="engine-insight-head">
        <div class="engine-insight-current">
          <b>${escapeHtml(subject)}</b>
        </div>
        <span class="engine-insight-counts">${steps.length} 个钩子 · ${
    changedFieldCount([...steps])
  } 处改动</span>
      </header>
      <div class="engine-step-list">
        ${
    steps.length === 0
      ? `<div class="engine-step-empty">这一步没有取到钩子，引擎透视为空</div>`
      : `<ol class="hook-chain">${renderHookChain(steps, names)}</ol>`
  }
      </div>
    </section>
  `;
}

/**
 * 同一张牌的一次出牌的连续钩子（选牌→结算→牌后→再动）只显示一次牌名：
 * 牌名作为组头，组内步骤只显示钩子名，消除「千里神行符」在 4 行里
 * 重复出现的冗余。无牌步骤（回合开始/结束）保持独立一行。
 *
 * 分组单位是「一次出牌事件」（frameIndex = cardCompleted 事件下标），不是
 * 牌名：同一张牌一次出牌的所有钩子共享同一个 frameIndex（Rust 侧
 * `card_completed_event_index`）。同名同等级连续出牌（如两张鹤步、两张
 * 飞豹灵剑）的 frameIndex 不同，必须分成两组，不能按 cardName 误合并。
 * 同名出现多组时，组头带卡槽号区分（「鹤步 · 第 3 格」）。
 */
function renderHookChain(steps: readonly HookStep[], names: SideNames): string {
  const groups: Array<{
    cardName: string | null;
    frameIndex: number | null;
    steps: HookStep[];
  }> = [];
  for (const step of steps) {
    const last = groups[groups.length - 1];
    const sameCardEvent = step.cardName !== null
      && last?.cardName === step.cardName
      && last.frameIndex === step.frameIndex;
    if (sameCardEvent) {
      last.steps.push(step);
    } else {
      groups.push({
        cardName: step.cardName,
        frameIndex: step.cardName !== null ? step.frameIndex : null,
        steps: [step],
      });
    }
  }
  const groupCountByCard = new Map<string | null, number>();
  for (const group of groups) {
    groupCountByCard.set(group.cardName, (groupCountByCard.get(group.cardName) ?? 0) + 1);
  }
  return groups.map((group) => {
    if (group.cardName === null) {
      return group.steps.map((step) => renderHookStep(step, names)).join("");
    }
    const slot = group.steps[0]?.slot;
    const repeatedName = (groupCountByCard.get(group.cardName) ?? 0) > 1;
    const slotLabel = repeatedName && slot !== null && slot !== undefined
      ? `<span class="hook-card-group-slot"> · 第 ${slot + 1} 格</span>`
      : "";
    return `
      <li class="hook-card-group">
        <span class="hook-card-group-name">${escapeHtml(group.cardName)}${slotLabel}</span>
        <ol class="hook-card-group-steps">
          ${group.steps.map((step) => renderHookStep(step, names)).join("")}
        </ol>
      </li>`;
  }).join("");
}

function renderHookStep(step: HookStep, names: SideNames): string {
  const summary = step.changes.length === 0
    ? "无状态变化"
    : step.changes.map((change) => changeText(change)).join(" · ");
  // 牌名由组头（.hook-card-group-name）统一显示，步骤标签只保留钩子名。
  const label = `<span class="hook-step-name">${escapeHtml(step.categoryLabel)}</span>`;
  const segmentsHtml = step.attackSegments.length > 0
    ? renderAttackSegments(step.attackSegments, step.actor, names)
    : "";
  const changesHtml = step.changes.length === 0 && segmentsHtml === ""
    ? "无状态变化"
    : `${renderHookChangesBySide(step.changes, step.actor, names)}${segmentsHtml}`;
  return `
    <li
      class="hook-step"
      data-hook="${step.category}"
      data-frame="${step.frameIndex}"
      title="${escapeAttribute(`${step.categoryLabel}${step.cardName ? ` · ${step.cardName}` : ""} — ${summary}`)}"
    >
      <span class="hook-step-label">${label}</span>
      <span class="hook-step-changes${step.changes.length === 0 && segmentsHtml === "" ? " empty" : ""}">
        ${changesHtml}
      </span>
    </li>
  `;
}

/**
 * 逐段攻击渲染：原版客户端 `TmpFloatingText` 逐段显示伤害数字（百杀 4 段 8 攻
 * → 4 个独立数字），模拟器之前只有 mainEffect 钩子的净差值。这里把 Rust 引擎
 * 在 `attack_by_config` 循环里逐段采样的 hp/防 before→after 渲染成逐段条，
 * 粒度与原作对齐。每段显示该段扣的防和穿透到生命的伤害。
 */
function renderAttackSegments(
  segments: readonly HookAttackSegment[],
  actor: Side,
  names: SideNames,
): string {
  const targetSide = segments[0]?.target;
  if (targetSide === undefined) return "";
  const tag = targetSide === "p1" ? "一" : "二";
  const isActor = targetSide === actor;
  const segmentRows = segments.map((segment) => {
    const hpLost = segment.hpBefore - segment.hpAfter;
    const defLost = segment.defBefore - segment.defAfter;
    const defText = defLost > 0 ? `防 -${defLost}` : "";
    const hpText = hpLost > 0 ? `命 -${hpLost}` : "";
    const parts = [defText, hpText].filter(Boolean).join(" ");
    const text = parts || "未穿透";
    return `
      <span
        class="hook-attack-segment${hpLost > 0 ? " hit" : ""}"
        data-field="attackSegment${segment.hitIndex}"
        title="${
      escapeAttribute(
        `第 ${segment.hitIndex + 1} 段 · ${names[targetSide]} 防 ${segment.defBefore}→${segment.defAfter} 生命 ${segment.hpBefore}→${segment.hpAfter}`,
      )
    }"
      >
        <span class="hook-change-subject" data-audit-ignore="repeated-log-field">第${segment.hitIndex + 1}段</span>
        <b class="hook-change-value" data-audit-ignore="repeated-log-value">${escapeHtml(text)}</b>
      </span>`;
  }).join("");
  return `
    <span class="hook-change-side hook-attack-segments${isActor ? " actor" : ""}" data-side="${targetSide}">
      <span class="hook-change-side-tag" data-audit-ignore="repeated-log-field">${tag}</span>
      <span class="hook-change-side-list">
        ${segmentRows}
      </span>
    </span>`;
}

/**
 * 按行动方分组渲染改动：先行动方（actor）的改动，再对手的改动。
 * 每组用带侧标签的容器包裹，消除原来 p1/p2 改动混在同一个垂直列表里
 * 读不出「谁动了谁」的问题。单侧无改动时不渲染空组。
 */
function renderHookChangesBySide(
  changes: readonly HookFieldChange[],
  actor: Side,
  names: SideNames,
): string {
  const sides: readonly Side[] = ["p1", "p2"];
  return sides
    .map((side) => {
      const sideChanges = changes.filter((change) => change.side === side);
      if (sideChanges.length === 0) return "";
      const tag = side === "p1" ? "一" : "二";
      const isActor = side === actor;
      return `
        <span class="hook-change-side${isActor ? " actor" : ""}" data-side="${side}">
          <span class="hook-change-side-tag" data-audit-ignore="repeated-log-field">${tag}</span>
          <span class="hook-change-side-list">
            ${sideChanges.map((change) => renderHookChange(change, names)).join("")}
          </span>
        </span>`;
    })
    .join("");
}

function renderHookChange(change: HookFieldChange, names: SideNames): string {
  return `
    <span
      class="hook-change ${change.after > change.before ? "up" : "down"}"
      data-field="${escapeAttribute(change.key)}"
      title="${
    escapeAttribute(
      `${names[change.side]} ${change.group}·${change.label} ${change.before} → ${change.after}`,
    )
  }"
    >
      <span class="hook-change-subject" data-audit-ignore="repeated-log-field">${
    escapeHtml(change.label)
  }</span>
      <b class="hook-change-value" data-audit-ignore="repeated-log-value">${
    change.before
  }→${change.after}</b>
    </span>
  `;
}

function changeText(change: HookFieldChange): string {
  return `${change.side === "p1" ? "一" : "二"} ${change.label} ${change.before}→${change.after}`;
}