import type { Side } from "./types";

/**
 * 把 Rust `canonical-rule-impact-v1` 归因收敛成"这场为什么赢"的结论层。
 *
 * 口径必须保持和引擎一致：一个 `cardCompleted` 观察点的贡献包含该观察点之前所有
 * 已结算 hook，不是那张牌的单卡因果。文案只能说"这一步之后拉开多少"，
 * 不能说"这张牌造成了多少"；解释只消费 Rust canonical telemetry，不把内部研究报告带入浏览器。
 */
export interface RuleImpactContribution {
  readonly hp: number;
  readonly defense: number;
  readonly guard: number;
  readonly resource: number;
  readonly debuff: number;
  readonly tempo: number;
  readonly total: number;
  /** 反事实读数：单位是生命点，不是 value 分，且不计入 total。 */
  readonly hpLossPreventedByGuard: number;
  readonly hpLossPreventedByDefense: number;
}

export interface RuleImpactCheckpoint {
  readonly checkpointIndex: number;
  readonly kind: "battleStart" | "turnStart" | "cardCompleted" | "turnEnd" | "battleEnd";
  readonly actorTurn: number;
  readonly actor: Side;
  readonly cardActionIndex?: number;
  readonly cardId?: number;
  readonly cardName?: string;
  readonly contribution: RuleImpactContribution;
}

export interface RuleImpactReport {
  readonly schemaVersion: string;
  readonly side: Side;
  readonly startValueForSide: number;
  readonly terminalValueForSide: number;
  readonly terminalDeltaForSide: number;
  readonly auditDeltaForSide: number;
  readonly checkpoints: readonly RuleImpactCheckpoint[];
  readonly cards: readonly {
    readonly cardId?: number;
    readonly cardName: string;
    readonly count: number;
    readonly contribution: RuleImpactContribution;
  }[];
}

export interface CounterfactualElementResult {
  readonly element: {
    readonly id: string;
    readonly label: string;
    readonly side: Side;
    readonly field: string;
    readonly amount: number;
  };
  readonly firstDivergenceActorTurn: number | null;
  readonly firstDivergenceCheckpointIndex: number | null;
  readonly firstDivergenceReason: "decisionTape" | "eventSequence" | null;
  /** 反事实减基线；负数表示移除该元素后，被解释方的生命差变差。 */
  readonly preDivergenceHpDeltaChangeForSide: number;
  readonly terminalHpDeltaChangeForSide: number;
  readonly counterfactualTerminalHpDeltaForSide: number;
  readonly baselineWinner: Side;
  readonly counterfactualWinner: Side;
  readonly winnerChanged: boolean;
}

export interface CounterfactualReport {
  readonly schemaVersion: "canonical-counterfactual-v1";
  readonly side: Side;
  readonly baselineTerminalHpDeltaForSide: number;
  readonly elements: readonly CounterfactualElementResult[];
}

/**
 * value 通道键。吸收量（`hpLossPrevented*`）不是通道：它是生命点而不是 value 分，
 * 也不进 `total`，混进通道构成就变成了双算。
 */
export type ValueChannelKey = keyof Omit<
  RuleImpactContribution,
  "total" | "hpLossPreventedByGuard" | "hpLossPreventedByDefense"
>;

export interface ChannelShare {
  readonly key: ValueChannelKey;
  readonly label: string;
  readonly delta: number;
  /** 占所有通道正向绝对值的比例，0-1。 */
  readonly share: number;
}

export interface TurningPoint {
  readonly actionIndex: number;
  readonly actorTurn: number;
  readonly cardName: string;
  readonly delta: number;
  /** 该结算点是对手行动。贡献是被解释方视角的得失，不是对手自己的收益。 */
  readonly byOpponent: boolean;
  readonly leadingChannel: ChannelShare | undefined;
}

/**
 * 一张牌在全场的结算点合计。
 *
 * 口径同 `TurningPoint`：结算点包含它之前所有已结算 hook，所以这是"记在这张牌上的
 * 累计落差"，不是"这张牌造成的伤害"。同名牌打多次会合并，`count` 记次数。
 */
export interface CardAttribution {
  readonly cardName: string;
  readonly count: number;
  readonly byOpponent: boolean;
  readonly delta: number;
  /** 该牌在主导通道上的累计贡献，用来回答"生命优势是怎么来的"。 */
  readonly channelDelta: number;
}

export interface BattleExplanation {
  readonly side: Side;
  readonly valueDelta: number;
  /** 非零表示归因与终局价值变化对不上，结论层必须显式降级而不是照常展示。 */
  readonly auditDelta: number;
  readonly channels: readonly ChannelShare[];
  readonly turningPoints: readonly TurningPoint[];
  /** 同 tape 的逐元素消融；与 checkpoint 吸收量不同，这是实际重跑结果。 */
  readonly counterfactuals: readonly CounterfactualElementResult[];
  /** 主导通道上贡献最大的牌，己方在前；结论层用它回答"怎么取得的"。 */
  readonly leadingCards: readonly CardAttribution[];
  readonly headline: string;
}

const CHANNEL_LABELS: Readonly<Record<ValueChannelKey, string>> = {
  hp: "生命",
  defense: "防御",
  guard: "护体",
  resource: "资源",
  debuff: "压制",
  tempo: "节奏",
};

const TURNING_POINT_COUNT = 3;

export function buildBattleExplanation(
  report: RuleImpactReport,
  counterfactual?: CounterfactualReport,
): BattleExplanation {
  const totals = sumChannels(report.checkpoints.map((point) => point.contribution));
  const channels = rankChannels(totals);
  const turningPoints = pickTurningPoints(report.checkpoints, report.side);
  const leadingChannel = channels.find((channel) => channel.delta > 0) ?? channels[0];
  const leadingCards = attributeCards(report.checkpoints, report.side, leadingChannel?.key);
  return {
    side: report.side,
    valueDelta: round1(report.terminalDeltaForSide),
    auditDelta: round1(report.auditDeltaForSide),
    channels,
    turningPoints,
    counterfactuals: counterfactual?.elements ?? [],
    leadingCards,
    headline: buildHeadline(channels, leadingCards),
  };
}

/**
 * 按牌聚合主导通道上的落差。
 *
 * "主要靠生命通道积累优势"本身不回答任何问题 —— 要说清楚的是这份生命优势记在
 * 哪几张牌的结算点上，以及对手哪张牌打回去最多。
 */
function attributeCards(
  checkpoints: readonly RuleImpactCheckpoint[],
  side: Side,
  channel: ValueChannelKey | undefined,
): readonly CardAttribution[] {
  if (channel === undefined) return [];
  const totals = new Map<string, CardAttribution>();
  for (const point of checkpoints) {
    if (point.kind !== "cardCompleted") continue;
    const cardName = point.cardName ?? `card:${point.cardId ?? "?"}`;
    const byOpponent = point.actor !== side;
    const key = `${byOpponent ? "x" : "o"}|${cardName}`;
    const existing = totals.get(key);
    totals.set(key, {
      cardName,
      byOpponent,
      count: (existing?.count ?? 0) + 1,
      delta: (existing?.delta ?? 0) + point.contribution.total,
      channelDelta: (existing?.channelDelta ?? 0) + point.contribution[channel],
    });
  }
  return [...totals.values()]
    .filter((card) => card.channelDelta !== 0)
    .map((card) => ({
      ...card,
      delta: round1(card.delta),
      channelDelta: round1(card.channelDelta),
    }))
    .sort((left, right) => Math.abs(right.channelDelta) - Math.abs(left.channelDelta));
}

function sumChannels(
  contributions: readonly RuleImpactContribution[],
): Record<ValueChannelKey, number> {
  const totals: Record<ValueChannelKey, number> = {
    hp: 0,
    defense: 0,
    guard: 0,
    resource: 0,
    debuff: 0,
    tempo: 0,
  };
  for (const contribution of contributions) {
    for (const key of Object.keys(totals) as ValueChannelKey[]) {
      totals[key] += contribution[key];
    }
  }
  return totals;
}

function rankChannels(totals: Record<ValueChannelKey, number>): readonly ChannelShare[] {
  const magnitude = Object.values(totals).reduce((sum, value) => sum + Math.abs(value), 0);
  return (Object.keys(totals) as ValueChannelKey[])
    .map((key) => ({
      key,
      label: CHANNEL_LABELS[key],
      delta: round1(totals[key]),
      share: magnitude === 0 ? 0 : Math.abs(totals[key]) / magnitude,
    }))
    .filter((channel) => channel.delta !== 0)
    .sort((left, right) => Math.abs(right.delta) - Math.abs(left.delta));
}

function pickTurningPoints(
  checkpoints: readonly RuleImpactCheckpoint[],
  side: Side,
): readonly TurningPoint[] {
  return checkpoints
    .filter((point) => point.kind === "cardCompleted" && point.contribution.total !== 0)
    .sort((left, right) =>
      Math.abs(right.contribution.total) - Math.abs(left.contribution.total)
    )
    .slice(0, TURNING_POINT_COUNT)
    .sort((left, right) => (left.cardActionIndex ?? 0) - (right.cardActionIndex ?? 0))
    .map((point) => ({
      actionIndex: point.cardActionIndex ?? 0,
      actorTurn: point.actorTurn,
      cardName: point.cardName ?? `card:${point.cardId ?? "?"}`,
      delta: round1(point.contribution.total),
      byOpponent: point.actor !== side,
      leadingChannel: rankChannels(sumChannels([point.contribution]))[0],
    }));
}

function buildHeadline(
  channels: readonly ChannelShare[],
  leadingCards: readonly CardAttribution[],
): string {
  const gains = channels.filter((channel) => channel.delta > 0);
  if (gains.length === 0) {
    return "本场没有产生正向价值积累。";
  }
  const leading = gains[0]!;
  const mine = leadingCards.filter((card) => !card.byOpponent && card.channelDelta > 0).slice(0, 2);
  const theirs = leadingCards
    .filter((card) => card.byOpponent && card.channelDelta < 0)
    .slice(0, 1);
  // 只能说"记在这张牌的结算点上"：一个结算点包含它之前所有已结算 hook，
  // 说成"这张牌造成"会把公共钩子的结果算成单卡因果。
  const source = mine.length > 0
    ? `${leading.label}优势 ${formatSigned(leading.delta)} 主要记在${
      mine.map(cardPhrase).join("、")
    }的结算点上`
    : `${leading.label}通道净得 ${formatSigned(leading.delta)}，但没有哪张牌的结算点占主导`;
  const pushback = theirs.length > 0
    ? `；对手${cardPhrase(theirs[0]!)}打回最多`
    : "";
  return `${source}${pushback}。`;
}

function cardPhrase(card: CardAttribution): string {
  const amount = formatSigned(card.channelDelta);
  return card.count > 1
    ? `「${card.cardName}」（${card.count} 次合计 ${amount}）`
    : `「${card.cardName}」（${amount}）`;
}

export function formatSigned(value: number): string {
  return value > 0 ? `+${round1(value)}` : `${round1(value)}`;
}

function round1(value: number): number {
  return Math.round(value * 10) / 10;
}
