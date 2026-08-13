/** Runtime-neutral contracts shared by the Rust/WASM browser surface. */
export interface OriginalEnumValue {
  readonly value: number;
  readonly name: string;
}

export type CardElement = "metal" | "water" | "wood" | "fire" | "earth";

export type CardTrait =
  | "cloudSword"
  | "bengQuan"
  | "spiritSword"
  | "rearMove"
  | `element:${CardElement}`;

export interface OriginalCardConfig {
  readonly id: number;
  readonly baseId?: number;
  readonly name: string;
  readonly desc?: string;
  readonly sect?: OriginalEnumValue | null;
  readonly career?: OriginalEnumValue;
  readonly subcategory?: OriginalEnumValue;
  readonly level?: OriginalEnumValue;
  readonly cardType?: OriginalEnumValue | null;
  readonly anima?: number;
  readonly chargeQi?: number;
  readonly hpCost?: number;
  readonly actionAgain?: boolean;
  readonly attack?: number;
  readonly randomAttack?: number;
  readonly attackCount?: number;
  readonly def?: number;
  readonly defense?: number;
  readonly randomDef?: number;
  readonly randomDefense?: number;
  readonly damage?: number;
  readonly physique?: number;
  readonly jianYi?: number;
  readonly guaXiang?: number;
  readonly otherParams?: readonly number[];
  readonly seasonMechanics?: readonly number[];
  readonly traits?: readonly CardTrait[];
  readonly rarity?: number;
  readonly hidden?: boolean;
  readonly noUpgrade?: boolean;
  readonly owner?: number;
}

export type EvaluationSide = "p1" | "p2";
export type BattleDecisionKind = "negative-status" | "random-range" | "percent-roll";
export type BattleDecisionProviderId =
  | "replay-tape"
  | "seeded-synthetic"
  | "hexagram"
  | "random-fallback-tape"
  | "default-value";

export type BattleDecisionEvent = {
  readonly side: EvaluationSide;
  readonly actorTurn: number;
  readonly cardId: number;
  readonly decisionKind: BattleDecisionKind;
  readonly cardExecutionOccurrence: number;
  readonly decisionOccurrence: number;
  readonly ordinal: number;
  readonly legalOptions?: readonly number[];
  readonly legalRange?: {
    readonly minInclusive: number;
    readonly maxInclusive: number;
  };
  readonly provider: BattleDecisionProviderId;
  readonly seed: number | null;
  readonly selectedOption: number | null;
};
