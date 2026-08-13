export type SolverUiTask = "order" | "hand" | "pool";
export type SolverSearchMethod = "heuristic" | "exhaustive";
export type SolverUiMode =
  | "orderBeam"
  | "order"
  | "handBeam"
  | "hand"
  | "poolBeam";

export const SOLVER_TASK_ORDER: readonly SolverUiTask[] = [
  "order",
  "hand",
  "pool",
];

export const SOLVER_METHOD_ORDER: readonly SolverSearchMethod[] = [
  "heuristic",
  "exhaustive",
];

export const SOLVER_TASKS: Readonly<Record<SolverUiTask, {
  readonly label: string;
  readonly shortLabel: string;
  readonly hint: string;
}>> = {
  order: {
    label: "场上牌排序",
    shortLabel: "牌序",
    hint: [
      "场上牌排序",
      "",
      "用途：固定当前场上牌，只比较出牌顺序。",
      "输入：当前玩家的场上牌；不会从手牌或卡池换牌。",
      "编号：1、2、3…对应求解前的场上牌槽位。",
      "应用：点击“应用推荐”只写回牌序。",
    ].join("\n"),
  },
  hand: {
    label: "对局手牌",
    shortLabel: "手牌",
    hint: [
      "对局手牌",
      "",
      "用途：从场上牌与导入对局的当前手牌中重组八张，并比较牌序。",
      "输入：必须先导入带 handCardIds 的对局记录。",
      "编号：求解基准牌从 1 开始；回放中的“普通攻击”补位也占一位，手牌随后继续编号。",
      "核对：候选牌序出现手牌编号即表示换入；行末“手N”显示实际换入张数。",
      "应用：点击“应用推荐”把候选八张牌写回当前构筑。",
    ].join("\n"),
  },
  pool: {
    label: "卡池求解",
    shortLabel: "卡池",
    hint: [
      "卡池求解",
      "",
      "用途：从当前角色、门派与副职的已实现一阶牌中搜索构筑。",
      "范围：每种牌按一张候选处理；不把未实现或战斗外卡牌送入规则内核。",
      "方法：组合空间较大，只开放启发式搜索；穷举固定置灰。",
      "应用：点击“应用推荐”写回找到的候选构筑。",
    ].join("\n"),
  },
};

export const SOLVER_METHODS: Readonly<Record<SolverSearchMethod, {
  readonly label: string;
  readonly hint: string;
  readonly maxEvaluations: number;
}>> = {
  heuristic: {
    label: "启发式",
    hint: [
      "启发式搜索",
      "",
      "做法：按搜索层级保留较优候选，不遍历全部组合。",
      "优点：反馈快，适合先得到可用建议。",
      "限制：结果不是全空间最优证明。",
      "预算：默认最多评估 2,000 个候选。",
    ].join("\n"),
    maxEvaluations: 2_000,
  },
  exhaustive: {
    label: "穷举",
    hint: [
      "穷举搜索",
      "",
      "做法：按 canonical 顺序枚举候选。",
      "结论：未截断时可比较完整搜索空间。",
      "限制：最多评估 200,000 次；达到上限会标记为截断。",
      "卡池求解：组合空间过大，不开放此方法。",
    ].join("\n"),
    maxEvaluations: 200_000,
  },
};

export const SOLVER_PRESETS: Readonly<Record<SolverUiMode, {
  readonly task: SolverUiTask;
  readonly method: SolverSearchMethod;
  readonly label: string;
  readonly shortLabel: string;
  readonly hint: string;
  readonly maxEvaluations: number;
}>> = {
  orderBeam: preset("order", "heuristic"),
  order: preset("order", "exhaustive"),
  handBeam: preset("hand", "heuristic"),
  hand: preset("hand", "exhaustive"),
  poolBeam: preset("pool", "heuristic"),
};

export function isSolverUiMode(value: string | undefined): value is SolverUiMode {
  return value !== undefined && value in SOLVER_PRESETS;
}

export function solverModeLabel(mode: SolverUiMode): string {
  return SOLVER_PRESETS[mode].label;
}

export function solverTaskForMode(mode: SolverUiMode): SolverUiTask {
  return SOLVER_PRESETS[mode].task;
}

export function solverMethodForMode(mode: SolverUiMode): SolverSearchMethod {
  return SOLVER_PRESETS[mode].method;
}

export function solverModeFor(
  task: SolverUiTask,
  method: SolverSearchMethod,
): SolverUiMode {
  if (task === "order") return method === "heuristic" ? "orderBeam" : "order";
  if (task === "hand") return method === "heuristic" ? "handBeam" : "hand";
  return "poolBeam";
}

export function isSolverUiTask(value: string | undefined): value is SolverUiTask {
  return value !== undefined && SOLVER_TASK_ORDER.includes(value as SolverUiTask);
}

export function isSolverSearchMethod(
  value: string | undefined,
): value is SolverSearchMethod {
  return value !== undefined &&
    SOLVER_METHOD_ORDER.includes(value as SolverSearchMethod);
}

function preset(
  task: SolverUiTask,
  method: SolverSearchMethod,
): {
  readonly task: SolverUiTask;
  readonly method: SolverSearchMethod;
  readonly label: string;
  readonly shortLabel: string;
  readonly hint: string;
  readonly maxEvaluations: number;
} {
  const taskConfig = SOLVER_TASKS[task];
  const methodConfig = SOLVER_METHODS[method];
  return {
    task,
    method,
    label: `${taskConfig.label} · ${methodConfig.label}`,
    shortLabel: `${taskConfig.shortLabel} · ${methodConfig.label}`,
    hint: `${taskConfig.hint}；${methodConfig.hint}`,
    maxEvaluations: methodConfig.maxEvaluations,
  };
}
