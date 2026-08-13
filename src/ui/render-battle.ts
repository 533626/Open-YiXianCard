import { renderBattleVerdict } from "./render-battle-verdict";
import { renderEngineInsightModule } from "./render-hook-flow";
import { renderSolverPanel } from "./render-solver";
import { activeBattleModule, BATTLE_MODULES } from "./battle-modules";
import { renderResourceFlow, resourceFlowFrames } from "./render-battle-flow";
import {
  renderProgressTrack,
  timelinePoints,
} from "./render-battle-progress";
import { escapeAttribute, escapeHtml } from "./view-utils";
import type { BattleModuleId } from "./battle-modules";
import type {
  AppState,
  BattleFrame,
  FlowMetric,
} from "./types";

/** 曲线模块在 BATTLE_MODULES 里的静态引用，循环外取一次，避免在 .map 内重复 find。 */
const TRAJECTORY_MODULE = BATTLE_MODULES.find((module) => module.id === "trajectory")!;

export function renderBattleResult(state: AppState): string {
  if (!state.result) {
    const running = state.battleStatus?.state === "running";
    return `
      <section class="panel result-empty ${running ? "result-running" : ""}" aria-busy="${running ? "true" : "false"}">
        <div class="panel-title">战斗</div>
        <div class="simulator-intro" aria-label="模拟器说明">
          <div class="simulator-intro-heading">弈仙牌战斗模拟器</div>
          <dl class="simulator-intro-features">
            <dt>构筑</dt>
            <dd>选角色、仙命、副职与天衍，点卡槽选牌并切换等级；导入原版回放可一键复现真实对局。</dd>
            <dt>推演</dt>
            <dd>双方各摆一张场上牌即自动开打，逐动展示出牌、伤害、生命与状态变化，时间轴可回看任意一步。</dd>
            <dt>求解</dt>
            <dd>右下求解建议可重排出牌顺序、从手牌重组卡组、或从卡池搜索构筑，推荐方案一键写回。</dd>
          </dl>
          <ol class="simulator-intro-guide">
            <li>左侧两位玩家各选角色 → 填仙命与副职 → 点空卡槽选牌</li>
            <li>双方各至少一张场上牌后自动推演，右列出现战斗结果</li>
            <li>用时间轴或方向键逐步查看每动结算，切模块看生命曲线或结算详情</li>
            <li>点「导入」载入原版回放，或用求解建议优化当前构筑</li>
          </ol>
          ${running ? `
            <div class="battle-running-status" role="status" aria-live="polite">
              <span class="solver-spinner" aria-hidden="true"></span>
              <span>计算已离开主线程</span>
              <kbd>Esc</kbd><span>取消</span>
            </div>
          ` : ""}
        </div>
      </section>
    `;
  }
  const frame = currentFrame(state);
  const frames = state.result.frames;
  const flowFrames = resourceFlowFrames(frames);
  const finalFrame = frames.at(-1) ?? frame;
  const sideNames = { p1: frame.players.p1.name, p2: frame.players.p2.name };
  const module = activeBattleModule(state);
  const secondaryModule = module === "advice" ? "advice" : "trajectory";
  return `
    <section class="panel battle-view insight-split module-${module}">
      ${renderBattleVerdict(state, finalFrame, state.result.actionCount)}
      ${renderBattleProgressRail(state, flowFrames)}
      ${renderModuleTabs(module, state.flowMetric)}
      <div class="insight-companion" data-module="${secondaryModule}" role="tabpanel">
        ${
    secondaryModule === "advice"
      ? renderSolverPanel(state)
      : renderResourceFlow(frames, state.frameIndex, state.flowMetric, state.result!.hookSteps)
  }
      </div>
      ${
    renderModuleBody("insight", () =>
      renderEngineInsightModule(
        state,
        frame,
        sideNames,
        state.result!.actionCount,
      )
    )
  }
    </section>
  `;
}

/**
 * 宽屏时引擎透视固定在右侧，生命曲线与获胜建议在左下互斥；窄屏仍用三项选项卡
 * 一次显示一个模块，避免日志被压成窄缝。
 *
 * 曲线模块的选项卡本身就是 生命/伤害 分段开关：点哪一档就切到曲线模块并
 * 选中该口径，不再需要模块内部再放一个 toggle。
 */
function renderModuleTabs(active: BattleModuleId, flowMetric: FlowMetric | undefined): string {
  const damage = flowMetric === "damage";
  return `
    <div class="battle-module-tabs" role="tablist" aria-label="战斗模块">
      ${
    BATTLE_MODULES.map((module) => module.id === "trajectory"
      ? renderTrajectoryTab(active, damage)
      : `
        <button
          type="button"
          role="tab"
          class="battle-module-tab ${module.id === active ? "active" : ""}"
          data-action="select-battle-module"
          data-module="${module.id}"
          aria-selected="${module.id === active ? "true" : "false"}"
          title="${escapeAttribute(module.hint)}"
        >${escapeHtml(module.label)}</button>
      `).join("")
  }
    </div>
  `;
}

function renderTrajectoryTab(active: BattleModuleId, damage: boolean): string {
  return `
    <span
      class="trajectory-switch${active === "trajectory" ? " active" : ""}"
      role="group"
      aria-label="曲线类型"
      title="${escapeAttribute(TRAJECTORY_MODULE.hint)}"
    >
      <button
        type="button"
        class="trajectory-option${damage ? "" : " active"}"
        data-action="select-trajectory-metric"
        data-metric="life"
        aria-pressed="${damage ? "false" : "true"}"
        title="显示双方生命与生命差"
      >生命</button>
      <button
        type="button"
        class="trajectory-option${damage ? " active" : ""}"
        data-action="select-trajectory-metric"
        data-metric="damage"
        aria-pressed="${damage ? "true" : "false"}"
        title="显示双方每回合伤害（按卡牌来源分段的堆叠柱状图，trace 精确归因）"
      >伤害</button>
    </span>
  `;
}

function renderModuleBody(module: BattleModuleId, body: () => string): string {
  return `
    <div class="battle-module-body" data-module="${module}" role="tabpanel">${body()}</div>
  `;
}

function currentFrame(state: AppState): BattleFrame {
  if (!state.result) throw new Error("没有战斗结果");
  return state.result.frames[state.frameIndex] ?? state.result.frames[0]!;
}

function renderBattleProgressRail(
  state: AppState,
  flowFrames: readonly BattleFrame[],
): string {
  // 时间轴跟模块选择解耦：曲线、钩子链、求解都在同一份"当前动作"上，
  // 把轨道塞进某一个模块里会让其他模块没法逐动看。
  return `
    <div class="battle-progress-rail" aria-label="动作进度">
      <div class="battle-progress-track" style="--progress-action-count:${
      timelinePoints(flowFrames).length
    }">
        ${renderProgressTrack(flowFrames, state.frameIndex)}
      </div>
    </div>
  `;
}
