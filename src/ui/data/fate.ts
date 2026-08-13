import { CHARACTER_BY_ID } from "./players";
import { fateStrategyRows } from "./source";
import { battleEventFateStrategyLabels } from "../generated/battle-event-labels";
import type { FateStrategyGroup, FateStrategyOption } from "../types";

export function fateStrategyGroupsForCharacter(characterId: number): readonly FateStrategyGroup[] {
  const character = CHARACTER_BY_ID.get(characterId);
  if (!character) return [];
  const rows = fateStrategyRows.filter(
    (strategy) =>
      strategy.status !== "record-only" &&
      (
        strategy.archiveKey === "fate-strategy:common" ||
        strategy.archiveKey === `fate-strategy:sect-eight:${character.sectName}` ||
        strategy.archiveKey === `fate-strategy:sect-card-pool:${character.sectName}` ||
        strategy.archiveKey === `fate-strategy:sect-extreme-card:${character.sectName}` ||
        strategy.archiveKey === `fate-strategy:sect-extension:${character.sectName}` ||
        strategy.archiveKey === `fate-strategy:exclusive:${character.id}`
      ),
  );
  const groups = new Map<string, FateStrategyOption[]>();
  for (const row of rows) {
    const options = groups.get(row.archiveKey) ?? [];
    options.push({
      id: row.strategyId,
      name: row.nameKey,
      archiveKey: row.archiveKey,
      archiveLabel: row.archiveLabel,
      section: row.section,
      sectionLabel: row.sectionLabel,
      categoryLabel: row.categoryLabel,
      status: row.status,
    });
    groups.set(row.archiveKey, options);
  }
  return [...groups.entries()]
    .map(([id, options]) => ({
      id,
      label: options[0]?.archiveLabel ?? id,
      options: options.sort((left, right) => {
        const leftImplemented = isFateStrategyImplemented(left);
        const rightImplemented = isFateStrategyImplemented(right);
        if (leftImplemented !== rightImplemented) return leftImplemented ? -1 : 1;
        return left.id - right.id;
      }),
    }))
    .sort((left, right) =>
      fateStrategyGroupOrder(left.id) - fateStrategyGroupOrder(right.id) ||
      left.label.localeCompare(right.label, "zh"),
    );
}

export function fateStrategyOptionsForCharacter(characterId: number): readonly FateStrategyOption[] {
  return fateStrategyGroupsForCharacter(characterId).flatMap((group) => group.options);
}

export function isFateStrategySelectableForCharacter(
  characterId: number,
  strategyId: number,
): boolean {
  return fateStrategyOptionsForCharacter(characterId)
    .some((option) => option.id === strategyId);
}

export function isFateStrategyImplemented(option: FateStrategyOption): boolean {
  return option.status === "implemented";
}

export function fateStrategyDisplayName(option: FateStrategyOption): string {
  return battleEventFateStrategyLabels.get(option.id) ?? fallbackFateStrategyName(option);
}

export function fateStrategySummary(option: FateStrategyOption): string {
  return FATE_STRATEGY_SUMMARIES[option.id] ?? [
    option.archiveLabel,
    option.categoryLabel === "普通" ? "" : option.categoryLabel,
  ].filter(Boolean).join(" · ");
}

export function fateStrategyPickerColumn(option: FateStrategyOption): "common" | "sect" | "exclusive" {
  if (option.archiveKey === "fate-strategy:common") return "common";
  if (option.archiveKey.includes(":exclusive:")) return "exclusive";
  return "sect";
}

const FATE_STRATEGY_SUMMARIES: Readonly<Record<number, string>> = {
  27: "有手牌时开局增加当前生命与生命上限",
  32: "每次使用普攻令对方失去加攻与护体",
  33: "首次使用普攻获得护体",
  36: "每次使用普攻获得加攻",
  84: "自身回合未攻击时，回合结束加 1 剑意",
  89: "开局视作已用云剑并获得云海",
  97: "使用云剑得云海，云海续连时加灵气",
  100: "使用澄心剑胚时加灵气",
  101: "使用澄心剑胚时加剑气",
  102: "使用澄心剑胚时清减全部负面",
  103: "使用澄心剑胚时获得水月",
  109: "每场首次使用剑阵时加 5 防和 1 层护体",
  121: "使用第 8 格牌时对方获得内伤",
  128: "使用金灵牌时加灵气",
  131: "按先天印记额外激活相生五行",
  133: "土灵阵可触发金灵牌分支",
  135: "首张灵阵额外执行一次",
  138: "开局激活木灵并加灵气",
  140: "开局按当前生命增加生命上限并回复",
  143: "开局激活土灵并加防",
  146: "开局激活火灵并削减对方生命上限",
  153: "使用耗生命牌时按费用比例伤害对方",
  161: "开局获得冥夜层数",
  166: "禅心锻身额外加体魄",
  309: "五行玉瓶额外激活第二格存牌五行",
  319: "琴师牌参与灵剑判定",
  320: "使用云剑时获得水月",
  322: "开局算作已用 1 次狂剑，猫名牌也视作狂剑",
  324: "使用澄心剑胚时获得云海",
  325: "使用狂剑时加灵气",
  327: "开局临时升级首张名字含雷或描述含再次行动的牌",
  329: "后手开局令下一次后招直接触发",
  330: "使用算无遗策时获 3 层临时护体，下回合开始移除",
  332: "含后招牌首次使用时向对方施加 1 层虚弱",
  333: "开局激活金灵并获得锋锐",
  334: "开局临时升级首张木灵牌",
  335: "锁定棍姿；切姿改为气势及上限 +1 并加 3 防",
  337: "使用灵羽时加灵气",
  345: "使用朱雀之泪时按灵气削减生命与上限",
  349: "锁定拳姿；切姿改为向对方造成 3 伤害",
};

function fateStrategyGroupOrder(key: string): number {
  if (key === "fate-strategy:common") return 0;
  if (key.includes(":sect-eight:")) return 1;
  if (key.includes(":exclusive:")) return 2;
  if (key.includes(":sect-extension:")) return 3;
  if (key.includes(":sect-card-pool:")) return 4;
  if (key.includes(":sect-extreme-card:")) return 5;
  return 99;
}

function fallbackFateStrategyName(option: FateStrategyOption): string {
  if (!option.name.startsWith("FateStrategyName_")) return option.name;
  if (option.categoryLabel !== "普通") return option.categoryLabel;
  if (option.archiveLabel !== "通用") return option.archiveLabel;
  return "通用天衍";
}
