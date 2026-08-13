import labels from "./battle-event-labels.json";

type GeneratedLabel = {
  readonly label: string;
  readonly sourceKind: "card" | "talent" | "fate-strategy" | "internal-counter";
  readonly sourceId?: number;
};

const generated = labels as {
  readonly buffLabels: Readonly<Record<string, GeneratedLabel>>;
  readonly talentLabels: Readonly<Record<string, GeneratedLabel>>;
  readonly fateStrategyLabels: Readonly<Record<string, GeneratedLabel>>;
  readonly sourceTokenLabels: Readonly<Record<string, GeneratedLabel>>;
};

export const battleEventBuffLabels = new Map(
  Object.entries(generated.buffLabels).map(([key, value]) => [key, value.label] as const),
);

export const battleEventSourceTokenLabels = new Map(
  Object.entries(generated.sourceTokenLabels).map(([key, value]) => [key, value.label] as const),
);

export const battleEventTalentLabels = new Map(
  Object.entries(generated.talentLabels).map(([key, value]) => [Number(key), value.label] as const),
);

export const battleEventFateStrategyLabels = new Map(
  Object.entries(generated.fateStrategyLabels).map(([key, value]) => [Number(key), value.label] as const),
);
