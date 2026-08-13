import {
  type CardElement,
  type RuleEvent,
} from "./domain";
import {
  ALL_CARD_DEFINITIONS,
  CARD_OPTION_BY_BASE_ID,
  ELEMENT_LABELS,
  sideLabel,
} from "./data";
import { BATTLE_START_SETTLEMENT_TITLE } from "./battle-opening";
import { buffLabel } from "./view-utils";
import type {
  BattleFrame,
  PlayerView,
  Side,
} from "./types";

const RESOURCE_FIELDS = new Set([
  "hp",
  "maxHp",
  "defense",
  "anima",
  "momentum",
  "momentumLimit",
  "agility",
  "guard",
]);

const CARD_DEFINITION_BY_ID = new Map(
  ALL_CARD_DEFINITIONS.flatMap((definition) => [
    [definition.id, definition] as const,
    ...(definition.baseId === undefined
      ? []
      : ([[definition.baseId, definition] as const])),
  ]),
);

export type CompletedPhaseCheckpointView = {
  readonly kind: "battleStart" | "turnStart" | "turnEnd";
  readonly afterSequence: number;
  readonly actorId: string | null;
  readonly actorTurn: number;
  readonly players: Record<Side, PlayerView>;
};

export function buildFrames(
  initialPlayers: Record<Side, PlayerView>,
  events: readonly RuleEvent[],
  winnerId: string | null,
  gameRound: number,
  completedPhaseCheckpoints: readonly CompletedPhaseCheckpointView[] = [],
): BattleFrame[] {
  const mutablePlayers = clonePlayers(initialPlayers);
  const frames: BattleFrame[] = [
    {
      index: 0,
      gameRound,
      actionIndex: null,
      title: "初始状态",
      actorId: null,
      actorTurn: 0,
      sourceSlot: null,
      cardId: null,
      cardName: null,
      winnerId: null,
      players: clonePlayers(mutablePlayers),
      events: [],
      summaries: ["准备完毕"],
    },
  ];
  let actorTurn = 0;
  let pending: RuleEvent[] = [];
  let cardFrames = 0;
  let lastCompletedPhaseFrameIndex: number | null = null;
  const checkpointsBySequence = groupCheckpointsBySequence(completedPhaseCheckpoints);
  const useCompletedPhaseCheckpoints = completedPhaseCheckpoints.length > 0;

  flushCompletedPhaseCheckpoints(0);

  for (const event of events) {
    if (
      !useCompletedPhaseCheckpoints &&
      event.type === "phase" &&
      event.name === "turnStart" &&
      actorTurn === 0 &&
      pending.length === 0
    ) {
      frames.push(makeFrame(
        BATTLE_START_SETTLEMENT_TITLE,
        event.actorId ?? null,
        0,
        gameRound,
        [],
        mutablePlayers,
      ));
    }
    if (!useCompletedPhaseCheckpoints && shouldFlushPendingBefore(event, pending)) {
      frames.push(makeFrame(phaseFrameTitle(pending, actorTurn), phaseActorId(pending), actorTurn, gameRound, pending, mutablePlayers));
      pending = [];
    }
    if (event.type === "phase" && event.name === "turnStart") {
      actorTurn += 1;
    }
    applyEvent(mutablePlayers, event);
    pending.push(event);
    if (event.type === "card" && event.name === "cardCompleted") {
      cardFrames += 1;
      const cardId = numberDetail(event, "cardId");
      const sourceSlot = numberDetail(event, "sourceSlot");
      frames.push({
        ...makeFrame(
          `第 ${cardFrames} 动 · ${cardName(cardId)}`,
          event.actorId ?? null,
          actorTurn,
          gameRound,
          pending,
          mutablePlayers,
        ),
        actionIndex: cardFrames,
        sourceSlot,
        cardId,
        cardName: cardName(cardId),
      });
      pending = [];
    }
    flushCompletedPhaseCheckpoints(event.sequence);
  }

  if (
    pending.length > 0 &&
    winnerId !== null &&
    lastCompletedPhaseFrameIndex === frames.length - 1 &&
    frameIsLethal(frames[lastCompletedPhaseFrameIndex]!) &&
    pending.every((event) =>
      event.type === "checkpoint" && event.name === "deathCheckpoint"
    )
  ) {
    const completed = frames[lastCompletedPhaseFrameIndex]!;
    frames[lastCompletedPhaseFrameIndex] = {
      ...completed,
      winnerId,
      events: [...completed.events, ...pending],
      summaries: [...completed.summaries, ...pending.map(formatEvent)],
    };
    pending = [];
  }
  if (
    pending.length === 0 &&
    winnerId !== null &&
    lastCompletedPhaseFrameIndex === frames.length - 1
  ) {
    frames[lastCompletedPhaseFrameIndex] = {
      ...frames[lastCompletedPhaseFrameIndex]!,
      winnerId,
    };
  }
  if (pending.length > 0 || frames.at(-1)?.winnerId !== winnerId) {
    const title = pending.some((event) =>
      event.type === "phase" && (
        event.name === "battleStart" ||
        event.name === "turnStart" ||
        event.name === "turnEnd"
      )
    )
      ? phaseFrameTitle(pending, actorTurn)
      : "战斗结束";
    frames.push({
      ...makeFrame(title, phaseActorId(pending), actorTurn, gameRound, pending, mutablePlayers),
      winnerId,
    });
  }
  function flushCompletedPhaseCheckpoints(afterSequence: number): void {
    for (const checkpoint of checkpointsBySequence.get(afterSequence) ?? []) {
      actorTurn = Math.max(actorTurn, checkpoint.actorTurn);
      frames.push(makeFrame(
        completedPhaseTitle(checkpoint.kind, checkpoint.actorTurn),
        checkpoint.actorId,
        checkpoint.actorTurn,
        gameRound,
        pending,
        checkpoint.players,
      ));
      pending = [];
      lastCompletedPhaseFrameIndex = frames.length - 1;
    }
  }

  return frames.map((frame, index) => ({ ...frame, index }));
}

function shouldFlushPendingBefore(event: RuleEvent, pending: readonly RuleEvent[]): boolean {
  if (pending.length === 0) return false;
  if (event.type === "phase" && event.name === "turnStart") return true;
  if (
    event.type === "phase" &&
    event.name === "turnEnd" &&
    pending.some((item) => item.type === "phase" && item.name === "turnStart")
  ) {
    return true;
  }
  const hasCardOrQueue = pending.some((item) => item.type === "card" || item.type === "queue");
  return !hasCardOrQueue && (event.type === "card" || event.type === "queue");
}

function groupCheckpointsBySequence(
  checkpoints: readonly CompletedPhaseCheckpointView[],
): ReadonlyMap<number, readonly CompletedPhaseCheckpointView[]> {
  const grouped = new Map<number, CompletedPhaseCheckpointView[]>();
  for (const checkpoint of checkpoints) {
    const list = grouped.get(checkpoint.afterSequence) ?? [];
    list.push(checkpoint);
    grouped.set(checkpoint.afterSequence, list);
  }
  return grouped;
}

function completedPhaseTitle(
  kind: CompletedPhaseCheckpointView["kind"],
  actorTurn: number,
): string {
  if (kind === "battleStart") return BATTLE_START_SETTLEMENT_TITLE;
  if (kind === "turnStart") {
    return `第 ${battleRoundNumber(actorTurn)} 回合开始结算`;
  }
  return `第 ${battleRoundNumber(actorTurn)} 回合结束结算`;
}

function frameIsLethal(frame: BattleFrame): boolean {
  return frame.players.p1.hp <= 0 || frame.players.p2.hp <= 0;
}

function phaseFrameTitle(pending: readonly RuleEvent[], actorTurn: number): string {
  if (actorTurn <= 0) return BATTLE_START_SETTLEMENT_TITLE;
  if (pending.some((event) => event.type === "phase" && event.name === "turnEnd")) {
    return `第 ${battleRoundNumber(actorTurn)} 回合结束结算`;
  }
  if (pending.some((event) => event.type === "phase" && event.name === "turnStart")) {
    return `第 ${battleRoundNumber(actorTurn)} 回合开始结算`;
  }
  return "阶段结算";
}

function phaseActorId(pending: readonly RuleEvent[]): "p1" | "p2" | null {
  const event = pending.find((item) => item.actorId === "p1" || item.actorId === "p2");
  return event?.actorId === "p1" || event?.actorId === "p2" ? event.actorId : null;
}

function battleRoundNumber(actorTurn: number): number {
  return Math.max(1, Math.ceil(actorTurn / 2));
}

function makeFrame(
  title: string,
  actorId: string | null,
  actorTurn: number,
  gameRound: number,
  events: readonly RuleEvent[],
  players: Record<Side, PlayerView>,
): BattleFrame {
  return {
    index: 0,
    gameRound,
    actionIndex: null,
    title,
    actorId,
    actorTurn,
    sourceSlot: null,
    cardId: null,
    cardName: null,
    winnerId: null,
    players: clonePlayers(players),
    events: [...events],
    summaries: events.map(formatEvent),
  };
}

function applyEvent(players: Record<Side, PlayerView>, event: RuleEvent): void {
  const side = event.targetId === "p2" || event.actorId === "p2" ? "p2" : "p1";
  const targetSide = event.targetId === "p2" ? "p2" : event.targetId === "p1" ? "p1" : side;
  const actorSide = event.actorId === "p2" ? "p2" : "p1";

  if (event.type === "resource" && RESOURCE_FIELDS.has(event.name)) {
    (players[targetSide] as unknown as Record<string, number>)[event.name] =
      event.after ?? (players[targetSide] as unknown as Record<string, number>)[event.name];
  }
  if (event.type === "buff") applyBuffEvent(players[targetSide], event);
  if (event.type === "queue") applyQueueEvent(players[actorSide], event);
  if (event.type === "card" && event.name === "temporaryUpgrade") {
    const sourceSlot = numberDetail(event, "sourceSlot");
    if (sourceSlot !== null && players[actorSide].slots[sourceSlot]) {
      players[actorSide].slots[sourceSlot]!.temporarilyUpgraded = true;
    }
  }
  if (event.type === "card" && event.name === "cardCompleted") {
    applyCardCompleted(players[actorSide], event);
  }
}

function applyBuffEvent(player: PlayerView, event: RuleEvent): void {
  const after = event.after ?? 0;
  if (event.name.startsWith("element:") && after > 0) {
    const element = event.name.slice("element:".length) as CardElement;
    if (!player.activatedElements.includes(element)) player.activatedElements.push(element);
    return;
  }
  if (after === 0) delete player.buffs[event.name];
  else player.buffs[event.name] = after;
}

function applyQueueEvent(player: PlayerView, event: RuleEvent): void {
  const sourceSlot = numberDetail(event, "sourceSlot");
  if (sourceSlot === null && event.name !== "reverseQueue") return;
  switch (event.name) {
    case "drawCard":
    case "starChessBreakSkipped":
    case "fateCycleSkipped":
    case "spaceSpiritFieldFirstUseSkipped":
      removeFromQueue(player, sourceSlot!);
      if (event.name !== "drawCard") pushQueue(player, sourceSlot!);
      break;
    case "returnCardToTail":
      pushQueue(player, sourceSlot!);
      break;
    case "returnCardToFront":
      removeFromQueue(player, sourceSlot!);
      player.cardQueue.unshift(sourceSlot!);
      break;
    case "reverseQueue":
      player.cardQueue.reverse();
      break;
    case "rightMoveCardQueue":
      rotateQueueRight(player, numberDetail(event, "distance") ?? 0);
      break;
  }
}

function applyCardCompleted(player: PlayerView, event: RuleEvent): void {
  const sourceSlot = numberDetail(event, "sourceSlot");
  const cardId = numberDetail(event, "cardId");
  if (sourceSlot !== null) {
    const slot = player.slots[sourceSlot];
    if (slot) {
      slot.hadUsed = true;
      slot.skipped = booleanDetail(event, "skipped") ?? slot.skipped;
      if (cardId !== null) {
        slot.cardId = cardId;
        slot.baseId = CARD_DEFINITION_BY_ID.get(cardId)?.baseId ?? cardId;
        slot.name = cardName(cardId);
      }
    }
  }
  const element = cardElement(cardId);
  if (element !== null) player.lastElement = element;
}

function removeFromQueue(player: PlayerView, slot: number): void {
  const index = player.cardQueue.indexOf(slot);
  if (index >= 0) player.cardQueue.splice(index, 1);
}

function pushQueue(player: PlayerView, slot: number): void {
  removeFromQueue(player, slot);
  player.cardQueue.push(slot);
}

function rotateQueueRight(player: PlayerView, distance: number): void {
  for (let index = 0; index < distance; index += 1) {
    const value = player.cardQueue.pop();
    if (value === undefined) return;
    player.cardQueue.unshift(value);
  }
}

function formatEvent(event: RuleEvent): string {
  const actor = playerLabel(event.actorId);
  const target = playerLabel(event.targetId);
  if (event.type === "card") {
    const name = cardName(numberDetail(event, "cardId"));
    if (event.name === "cardSelected") return `${actor} 选择 ${name}`;
    if (event.name === "cardCompleted") return `${actor} 完成 ${name}`;
    return `${actor} ${eventName(event.name)} ${name}`;
  }
  if (event.type === "damage") return `${actor} 对 ${target} 造成 ${event.amount ?? 0} 伤害`;
  if (event.type === "resource") {
    return `${target} ${resourceName(event.name)}：${event.before ?? "-"} -> ${event.after ?? "-"}`;
  }
  if (event.type === "buff") {
    if (event.name.startsWith("element:")) {
      const element = event.name.slice("element:".length) as CardElement;
      return `${target} 激活${ELEMENT_LABELS[element] ?? "五行"}`;
    }
    return `${target} ${buffLabel(event.name)}：${event.before ?? 0} -> ${event.after ?? 0}`;
  }
  if (event.type === "queue") return `${actor} 牌序 ${queueEventName(event.name)}`;
  if (event.type === "phase") return `${actor} ${phaseName(event.name)}`;
  if (event.type === "checkpoint") return checkpointName(event.name);
  return eventName(event.name);
}

function playerLabel(id: string | null | undefined): string {
  if (id === "p1" || id === "p2") return sideLabel(id);
  return "";
}

function resourceName(name: string): string {
  const names: Readonly<Record<string, string>> = {
    hp: "生命",
    maxHp: "生命上限",
    defense: "防",
    anima: "灵气",
    momentum: "气势",
    momentumLimit: "气势上限",
    agility: "身法",
    guard: "护体",
  };
  return names[name] ?? "资源";
}

function eventName(name: string): string {
  const names: Readonly<Record<string, string>> = {
    attack: "攻击",
    damage: "造成伤害",
    loseHp: "损失生命",
    maxHpDamage: "降低生命上限",
    cardSkipped: "跳过",
  };
  return names[name] ?? "触发";
}

function phaseName(name: string): string {
  const names: Readonly<Record<string, string>> = {
    battleStart: "战斗开始",
    turnStart: "行动开始",
    turnEnd: "行动结束",
    deathCheckpoint: "濒死检查",
  };
  return names[name] ?? "阶段变化";
}

function queueEventName(name: string): string {
  const names: Readonly<Record<string, string>> = {
    drawCard: "抽出",
    returnCardToTail: "回到末尾",
    returnCardToFront: "回到开头",
    reverseQueue: "反转",
    rightMoveCardQueue: "右移",
    starChessBreakSkipped: "星弈断跳过",
    fateCycleSkipped: "命运轮回跳过",
    spaceSpiritFieldFirstUseSkipped: "空间灵田跳过",
  };
  return names[name] ?? "变化";
}

function checkpointName(name: string): string {
  const names: Readonly<Record<string, string>> = {
    deathCheckpoint: "濒死检查",
  };
  return names[name] ?? "规则检查";
}

function cardName(id: number | null): string {
  if (id === null) return "-";
  return CARD_DEFINITION_BY_ID.get(id)?.name ?? CARD_OPTION_BY_BASE_ID.get(id)?.name ?? `#${id}`;
}

function cardElement(id: number | null): CardElement | null {
  if (id === null) return null;
  const card = CARD_DEFINITION_BY_ID.get(id);
  const trait = card?.traits.find((candidate) => candidate.startsWith("element:"));
  return trait ? (trait.slice("element:".length) as CardElement) : null;
}

function numberDetail(event: RuleEvent, key: string): number | null {
  const value = event.detail?.[key];
  return typeof value === "number" ? value : null;
}

function booleanDetail(event: RuleEvent, key: string): boolean | null {
  const value = event.detail?.[key];
  return typeof value === "boolean" ? value : null;
}

function clonePlayers(players: Record<Side, PlayerView>): Record<Side, PlayerView> {
  return structuredClone(players) as Record<Side, PlayerView>;
}
