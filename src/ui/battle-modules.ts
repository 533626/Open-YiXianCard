import type { AppState } from "./types";

/**
 * 右列的模块选择。
 *
 * 一次只显示一个模块，选中的那个吃掉右列剩余的全部高度 —— 三块折叠面板堆在下方时
 * 每块只剩一条缝，钩子链和曲线都读不了。左列构筑页任何时候都不被覆盖。
 */
export type BattleModuleId = "insight" | "trajectory" | "advice";

export interface BattleModule {
  readonly id: BattleModuleId;
  readonly label: string;
  /** 选项卡上的一句话，说明这块在回答什么问题。 */
  readonly hint: string;
}

export const BATTLE_MODULES: readonly BattleModule[] = [
  {
    id: "insight",
    label: "引擎透视",
    hint: [
      "引擎透视",
      "",
      "内容：按步进轮播，一次步进展示该回合（actorTurn）的全部钩子与字段变化。",
      "导航：方向键按初始态、相邻回合结束依次移动；致死回合停在最后实际帧。",
      "范围：没有改动的钩子也列出，看的是引擎实际走了哪些钩子。",
    ].join("\n"),
  },
  {
    id: "trajectory",
    label: "生命曲线",
    hint: [
      "生命曲线",
      "",
      "内容：展示双方生命、生命差与整场变化轨迹；选项卡上可直接切到伤害曲线。",
      "伤害曲线：按回合（双方各动一次）聚合的堆叠柱状图，柱内分段 = 各卡贡献，",
      "数据来自 yixian_trace_json 的逐段攻击采样与 hp/防 变化，精确归因。",
      "同步：当前动作与上方回放导航保持一致。",
      "用途：快速定位伤害、回复与生命差反转的动作。",
    ].join("\n"),
  },
  {
    id: "advice",
    label: "获胜建议",
    hint: [
      "获胜建议",
      "",
      "任务：场上牌排序、对局手牌重组或卡池构筑。",
      "方法：按任务选择启发式或穷举；卡池只开放启发式。",
      "结果：按 Value 排名，并同时显示胜负、hpDelta 与 actorTurn。",
      "编号：候选数字可追溯到场上牌、手牌或卡池来源。",
      "应用：点击“应用推荐”写回当前构筑。",
    ].join("\n"),
  },
];

export const DEFAULT_BATTLE_MODULE: BattleModuleId = "trajectory";

export function activeBattleModule(state: AppState): BattleModuleId {
  const requested = state.battleModule;
  return BATTLE_MODULES.some((module) => module.id === requested)
    ? requested!
    : DEFAULT_BATTLE_MODULE;
}

export function battleModuleFromValue(value: string | undefined): BattleModuleId | null {
  const module = BATTLE_MODULES.find((candidate) => candidate.id === value);
  return module ? module.id : null;
}
