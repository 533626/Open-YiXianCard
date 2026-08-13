import baseCardCoverage from "../../../shared/data/base-card-coverage.json";
import cardArchive from "../../../shared/data/card-archive.json";
import characterTalentAudit from "../../../shared/data/character-talent-audit.json";
import fateStrategyArchive from "../../../shared/data/fate-strategy-archive.json";
import talentArchive from "../../../shared/data/talent-archive.json";
import { ORIGINAL_CARD_CONFIGS } from "../../../shared/data/original-card-configs";
import {
  normalizeBaseId,
  type OriginalCardConfig,
} from "../domain";
import type { CardOption } from "../types";

export type GeneratedCardArchiveCard = {
  readonly baseId: number;
  readonly name: string;
  readonly type: string;
  readonly registered: boolean;
  readonly simulationScope: "battle" | "record-only";
  readonly obsolete: boolean;
  readonly archiveKind: CardOption["archiveKind"];
  readonly archiveKey: string;
  readonly archiveLabel: string;
  readonly realm?: string;
  readonly realmLabel?: string;
  readonly sect?: string;
  readonly career?: string;
  readonly registrationStatus?: string;
  readonly registrationNote?: string;
};

export type CoverageCard = {
  readonly baseId: number;
  readonly name: string;
  readonly groupType: string;
  readonly group: string;
  readonly cardType: string;
  readonly simulationScope: "battle" | "record-only";
  readonly registered: boolean;
  readonly registrationStatus?: string;
  readonly registrationNote?: string;
};

export type CharacterTalentRow = {
  readonly characterId: number;
  readonly characterName: string;
  readonly sectName: string;
  readonly slot: string;
  readonly talentId: number;
  readonly name: string;
  readonly levelName: string;
  readonly battle: boolean;
  readonly status: string;
  readonly registrationStatus?: string;
  readonly registrationNote?: string;
};

export type TalentArchiveRow = {
  readonly id: number;
  readonly name: string;
  readonly desc?: string;
  readonly otherParams?: readonly number[];
  readonly levelName: string;
  readonly battle: boolean;
  readonly status: string;
  readonly registrationStatus?: string;
  readonly registrationNote?: string;
  readonly archiveKind?: string;
  readonly archiveKey?: string;
  readonly archiveLabel?: string;
};

export type FateStrategyArchiveRow = {
  readonly strategyId: number;
  readonly nameKey: string;
  readonly archiveKey: string;
  readonly archiveLabel: string;
  readonly section: string;
  readonly sectionLabel: string;
  readonly category: string;
  readonly categoryLabel: string;
  readonly sectName?: string;
  readonly characterId?: number;
  readonly status: string;
  readonly registrationStatus?: string;
  readonly registrationNote?: string;
};

export type UiOriginalCardConfig = OriginalCardConfig & {
  readonly career?: { readonly value: number; readonly name: string };
  readonly sect?: { readonly value: number; readonly name: string } | null;
};

export { ORIGINAL_CARD_CONFIGS };

export const coverageCards = (baseCardCoverage as { cards: CoverageCard[] }).cards;
export const coverageByBaseId = new Map(coverageCards.map((card) => [card.baseId, card]));

export const archiveCards = (cardArchive as { cards: GeneratedCardArchiveCard[] }).cards;
export const archiveByBaseId = new Map(archiveCards.map((card) => [card.baseId, card] as const));

export const characterTalentRows = (characterTalentAudit as { rows: CharacterTalentRow[] }).rows;
export const characterNameById = new Map(
  characterTalentRows.map((row) => [row.characterId, row.characterName] as const),
);

export const talentArchiveRows = (talentArchive as { talents: TalentArchiveRow[] }).talents;
export const talentArchiveById = new Map(talentArchiveRows.map((row) => [row.id, row] as const));

export const fateStrategyRows = (fateStrategyArchive as { strategies: FateStrategyArchiveRow[] })
  .strategies.filter((strategy) => strategy.category !== "DaoYun");

export function normalizedOriginalBaseId(cardId: number): number {
  return normalizeBaseId(cardId);
}
