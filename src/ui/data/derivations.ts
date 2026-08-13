import type { CardOption } from "../types";

export const LU_JIANXIN_CHARACTER_ID = 1_000_005;

type TalentChoiceFamily = {
  readonly characterId: number;
  readonly parentTalentId: number;
  readonly choiceTalentIds: readonly number[];
  readonly derivedCardBaseId?: number;
};

type TalentCardGrant = {
  readonly talentId: number;
  readonly cardBaseId: number;
};

type CardTalentPresentation = {
  readonly cardBaseId: number;
  readonly talentId: number;
  readonly name?: string;
};

const TALENT_CHOICE_FAMILIES: readonly TalentChoiceFamily[] = [
  { characterId: LU_JIANXIN_CHARACTER_ID, parentTalentId: 93, choiceTalentIds: [10_093, 20_093], derivedCardBaseId: 19 },
  { characterId: LU_JIANXIN_CHARACTER_ID, parentTalentId: 94, choiceTalentIds: [10_094, 20_094, 30_094], derivedCardBaseId: 19 },
  { characterId: LU_JIANXIN_CHARACTER_ID, parentTalentId: 95, choiceTalentIds: [10_095, 20_095, 30_095], derivedCardBaseId: 19 },
  { characterId: LU_JIANXIN_CHARACTER_ID, parentTalentId: 96, choiceTalentIds: [10_096, 20_096, 30_096], derivedCardBaseId: 19 },
];

const TALENT_CARD_GRANTS: readonly TalentCardGrant[] = [
  { talentId: 187, cardBaseId: 82 }, // 万玄破魔掌
];

const CARD_TALENT_PRESENTATIONS: readonly CardTalentPresentation[] = [
  { cardBaseId: 19, talentId: 10_096, name: "狂剑•澄心" },
  { cardBaseId: 19, talentId: 20_096, name: "云剑•澄心" },
  { cardBaseId: 19, talentId: 30_096, name: "澄心•无极" },
];

const choiceFamilyByParent = new Map(
  TALENT_CHOICE_FAMILIES.map((family) => [family.parentTalentId, family] as const),
);
const choiceFamilyByChoice = new Map(
  TALENT_CHOICE_FAMILIES.flatMap((family) =>
    family.choiceTalentIds.map((talentId) => [talentId, family] as const)),
);
const grantsByCard = new Map<number, readonly TalentCardGrant[]>(
  [...new Set(TALENT_CARD_GRANTS.map((grant) => grant.cardBaseId))]
    .map((cardBaseId) => [
      cardBaseId,
      TALENT_CARD_GRANTS.filter((grant) => grant.cardBaseId === cardBaseId),
    ] as const),
);

export function derivedTalentChoiceIds(
  characterId: number,
  parentTalentId: number,
): readonly number[] {
  const family = choiceFamilyByParent.get(parentTalentId);
  return family?.characterId === characterId ? family.choiceTalentIds : [];
}

export function isDerivedTalentChoiceForCharacter(
  characterId: number,
  talentId: number,
): boolean {
  return choiceFamilyByChoice.get(talentId)?.characterId === characterId;
}

export function isCardUnlockedByTalents(
  cardBaseId: number,
  talentIds: readonly number[],
): boolean {
  const grants = grantsByCard.get(cardBaseId);
  return !grants || grants.some((grant) => talentIds.includes(grant.talentId));
}

export function derivedCardOption(
  card: CardOption,
  talentIds: readonly number[],
): CardOption {
  const presentation = CARD_TALENT_PRESENTATIONS.find((candidate) =>
    candidate.cardBaseId === card.baseId && talentIds.includes(candidate.talentId));
  return presentation?.name ? { ...card, name: presentation.name } : card;
}

export function cardDerivationTalentIds(
  cardBaseId: number,
  talentIds: readonly number[],
): readonly number[] {
  return [
    ...TALENT_CARD_GRANTS
      .filter((grant) => grant.cardBaseId === cardBaseId && talentIds.includes(grant.talentId))
      .map((grant) => grant.talentId),
    ...CARD_TALENT_PRESENTATIONS
      .filter((entry) => entry.cardBaseId === cardBaseId && talentIds.includes(entry.talentId))
      .map((entry) => entry.talentId),
    ...TALENT_CHOICE_FAMILIES
      .filter((family) => family.derivedCardBaseId === cardBaseId)
      .flatMap((family) => family.choiceTalentIds)
      .filter((talentId) => talentIds.includes(talentId)),
  ].filter((talentId, index, all) => all.indexOf(talentId) === index);
}
