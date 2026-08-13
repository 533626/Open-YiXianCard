import { describe, expect, test } from "bun:test";
import {
  CHARACTER_OPTIONS,
  characterBaseTalentSlots,
  defaultBattleConfig,
  defaultPlayerConfig,
  fateStrategyDisplayName,
  fateStrategyOptionsForCharacter,
  fateStrategySummary,
  isBattleIrrelevantTalent,
  isFateStrategySelectableForCharacter,
  isTalentSelectableForCharacter,
  scopedTalentOptions,
  talentChoiceGroupsForSlot,
  talentPickerColumn,
  TALENT_OPTION_BY_ID,
  talentArchiveRows,
  talentsGroupedForPicker,
} from "../data";
import { sanitizePlayerScope } from "../main-utils";
import { renderTalentCandidate, renderTalentPopup } from "../render-pickers";
import type { AppState } from "../types";

const MU_HU_ID = 3_000_004;
const JI_FANG_SHENG_ID = 4_000_004;
const FENG_XU_ID = 2_000_006;
const LI_MAN_ID = 4_000_005;
const TIGER_BODY_TALENT_ID = 125;
const TIGER_BODY_FATE_STRATEGY_ID = 140;
const PLACEHOLDER_TALENT_ID = 999_120;

const BATCH_009_FATE_STRATEGIES = [
  [84, 1_000_001, "天衍-剑不出鞘", "自身回合未攻击时，回合结束加 1 剑意"],
  [109, 1_000_006, "护体剑阵", "每场首次使用剑阵时加 5 防和 1 层护体"],
  [322, 1_000_004, "猫之狂念", "开局算作已用 1 次狂剑，猫名牌也视作狂剑"],
  [327, 2_000_002, "恃风雷", "开局临时升级首张名字含雷或描述含再次行动的牌"],
  [329, 2_000_005, "谋定后动", "后手开局令下一次后招直接触发"],
  [330, 2_000_005, "料敌机先", "使用算无遗策时获 3 层临时护体，下回合开始移除"],
  [332, 2_000_006, "引虚", "含后招牌首次使用时向对方施加 1 层虚弱"],
  [334, 3_000_006, "百草织霞", "开局临时升级首张木灵牌"],
  [335, LI_MAN_ID, "无尽棍势", "锁定棍姿；切姿改为气势及上限 +1 并加 3 防"],
  [349, LI_MAN_ID, "拳风破空", "锁定拳姿；切姿改为向对方造成 3 伤害"],
] as const;

function motherCount(archiveKey: string): number {
  return new Set(
    talentArchiveRows
      .filter((row) => row.archiveKey === archiveKey)
      .map((row) => row.id >= 10_000 ? row.id % 10_000 : row.id),
  ).size;
}

function talentPickerState(): AppState {
  return {
    view: "setup",
    workbenchMode: "duel",
    target: null,
    config: defaultBattleConfig(),
    activeSide: "p1",
    pickerMode: "talent",
    selectedSlot: 0,
    selectedTalentSlot: 1,
    cardSearch: "",
    cardArchiveKind: "all",
    cardArchiveKey: "all",
    cardType: "all",
    frameIndex: 0,
    autoPlaying: false,
    result: null,
    solverResult: null,
    solverCollapsed: false,
    error: null,
    savedBuilds: [],
    saveDraftNames: { p1: "", p2: "" },
    selectedBuildIds: { p1: "", p2: "" },
  };
}

describe("UI 仙命可选范围", () => {
  test("角色专属仙命只属于该角色，不会跨角色可选", () => {
    expect(isTalentSelectableForCharacter(MU_HU_ID, TIGER_BODY_TALENT_ID)).toBe(true);
    expect(isTalentSelectableForCharacter(JI_FANG_SHENG_ID, TIGER_BODY_TALENT_ID)).toBe(false);
    expect(scopedTalentOptions(JI_FANG_SHENG_ID).map((option) => option.id))
      .not.toContain(TIGER_BODY_TALENT_ID);
  });

  test("陆剑心专属分支按境界二或三选一，并保留门派与通用仙命", () => {
    const expected = new Map([
      [93, { levelName: "ZhuJi", choiceIds: [10_093, 20_093] }],
      [94, { levelName: "JinDan", choiceIds: [10_094, 20_094, 30_094] }],
      [95, { levelName: "YuanYing", choiceIds: [10_095, 20_095, 30_095] }],
      [96, { levelName: "HuaShen", choiceIds: [10_096, 20_096, 30_096] }],
    ]);
    for (const [parentTalentId, { levelName, choiceIds }] of expected) {
      const groups = talentChoiceGroupsForSlot(1_000_005, parentTalentId, levelName);
      expect(groups[0]?.id).toBe("exclusive");
      expect(groups[0]?.options.map((option) => option.id)).toEqual(choiceIds);
      expect(groups.some((group) => group.id === "sect")).toBe(true);
      expect(groups.some((group) => group.id === "common")).toBe(true);
      expect(choiceIds.every((id) => isTalentSelectableForCharacter(1_000_005, id))).toBe(true);
    }

    const state = talentPickerState();
    state.config.players.p1 = defaultPlayerConfig("p1", 1_000_005, state.config.gameRound);
    const html = renderTalentPopup(state);
    expect(html.match(/<section class="talent-stage/g)?.length).toBe(4);
    expect(html).toContain("锐利剑锋");
    expect(html).toContain("狂剑之心");
  });

  test("仙命弹窗替换项不展示专属列", () => {
    for (const character of CHARACTER_OPTIONS) {
      const groups = talentsGroupedForPicker(
        scopedTalentOptions(character.id).filter((option) => talentPickerColumn(option) !== "exclusive"),
      );

      expect(groups.every((group) => group.id !== "exclusive")).toBe(true);
    }
  });

  test("通用与门派仙命母项索引齐全", () => {
    expect(motherCount("common")).toBe(13);
    expect(motherCount("sect:DuanXuanZong")).toBe(8);
    expect(motherCount("sect:QiXingGe")).toBe(8);
    expect(motherCount("sect:WuXingDaoMeng")).toBe(8);
    expect(motherCount("sect:YunLingJianZong")).toBe(8);
  });

  test("未接战斗逻辑的仙命可作为占位选择", () => {
    // 当前归档已无可选 missing-battle 仙命（4297081 实现剩余门派仙命），
    // 契约改用合成 option 在渲染单元上验证：标 unimplemented 但不 disabled。
    const state = talentPickerState();
    state.config.players.p1 = defaultPlayerConfig(
      "p1",
      FENG_XU_ID,
      state.config.gameRound,
    );
    const option = {
      id: PLACEHOLDER_TALENT_ID,
      name: "测试占位仙命",
      status: "missing-battle",
    };
    const html = renderTalentCandidate(state, "sect", option, new Set<number>());

    expect(html).toContain(`data-talent-id="${PLACEHOLDER_TALENT_ID}"`);
    expect(html).toContain("unimplemented");
    expect(html).toContain("占位");
    expect(html).not.toMatch(
      new RegExp(`data-talent-id="${PLACEHOLDER_TALENT_ID}"[^>]*disabled`),
    );
    expect(html).not.toContain("disabled");

    const implementedHtml = renderTalentCandidate(
      state,
      "sect",
      { ...option, status: "implemented" },
      new Set<number>(),
    );
    expect(implementedHtml).not.toContain("unimplemented");
  });

  test("sanitize 会移除已混入的其他角色专属仙命", () => {
    const player = defaultPlayerConfig("p1", JI_FANG_SHENG_ID);
    player.talents[1] = TIGER_BODY_TALENT_ID;

    sanitizePlayerScope(player);

    expect(player.talents).not.toContain(TIGER_BODY_TALENT_ID);
  });

  test("天衍仙命同样按角色范围过滤", () => {
    expect(isFateStrategySelectableForCharacter(MU_HU_ID, TIGER_BODY_FATE_STRATEGY_ID)).toBe(true);
    expect(isFateStrategySelectableForCharacter(JI_FANG_SHENG_ID, TIGER_BODY_FATE_STRATEGY_ID))
      .toBe(false);
    expect(fateStrategyOptionsForCharacter(JI_FANG_SHENG_ID).map((option) => option.id))
      .not.toContain(TIGER_BODY_FATE_STRATEGY_ID);
  });

  test("batch 009 天衍仙命按原文显示并只在所属角色中转正", () => {
    for (const [id, characterId, name, summary] of BATCH_009_FATE_STRATEGIES) {
      const option = fateStrategyOptionsForCharacter(characterId)
        .find((candidate) => candidate.id === id);

      expect(option, `fate ${id}`).toBeDefined();
      expect(option?.status, `fate ${id}`).toBe("implemented");
      expect(fateStrategyDisplayName(option!), `fate ${id}`).toBe(name);
      expect(fateStrategySummary(option!), `fate ${id}`).toBe(summary);
    }

    const liManFates = fateStrategyOptionsForCharacter(LI_MAN_ID)
      .filter((option) => option.id === 335 || option.id === 349);
    expect(liManFates.map((option) => option.id)).toEqual([335, 349]);
    expect(liManFates.every((option) => option.status === "implemented")).toBe(true);
    const jiFangshengFates = fateStrategyOptionsForCharacter(JI_FANG_SHENG_ID)
      .map((option) => option.id);
    expect(jiFangshengFates).not.toContain(335);
    expect(jiFangshengFates).not.toContain(349);
  });

  test("sanitize 会移除已混入的其他角色专属天衍仙命", () => {
    const player = defaultPlayerConfig("p1", JI_FANG_SHENG_ID);
    player.fateStrategies = [TIGER_BODY_FATE_STRATEGY_ID];

    sanitizePlayerScope(player);

    expect(player.fateStrategies).toEqual([]);
  });

});

describe("UI 仙命浮层排序与默认标记", () => {
  const JI_FANG_SHENG = 4_000_004;

  test("isBattleIrrelevantTalent 区分抽通用牌/给专属牌/已接战斗", () => {
    const irr = (desc: string, status = "record-only") =>
      isBattleIrrelevantTalent({ id: 0, name: "x", desc, status } as never);
    // 抽通用牌（随机抽牌）与纯加命元都无关，沉底。
    expect(irr('抽{otherParams[0]}张“崩拳”牌')).toBe(true);
    expect(irr("抽{otherParams[0]}张门派牌")).toBe(true);
    expect(irr("命元减1；抽{otherParams[0]}张名字含“雷”的牌")).toBe(true);
    expect(irr("命元+2；战斗开始时加体魄", "record-only")).toBe(true);
    // 给专属牌（获得 1 张【某 ID】）相关，留在顶部。
    expect(irr("获得1张【{otherParams[0]}】")).toBe(false);
    // 已接战斗逻辑的 implemented 仙命无论抽牌/给牌都相关。
    expect(irr('抽{otherParams[0]}张“云剑”牌；战斗中加灵气', "implemented")).toBe(false);
    expect(irr("获得1张【219】", "implemented")).toBe(false);
    // 无描述的占位也按无关处理。
    expect(isBattleIrrelevantTalent({ id: 0, name: "无描述" } as never)).toBe(true);
  });

  function stageBodies(html: string): Array<[number, string]> {
    return [...html.matchAll(/<section class="talent-stage[^"]*" data-talent-stage="(\d+)"([\s\S]*?)<\/section>/g)]
      .map((m) => [Number(m[1]), m[2] as string]);
  }
  function optionIds(body: string): number[] {
    return [...body.matchAll(/data-talent-id="(\d+)"/g)].map((m) => Number(m[1]));
  }
  function firstButton(body: string): string | undefined {
    return body.match(/<button[\s\S]*?<\/button>/)?.[0];
  }

  test("角色默认仙命置顶，战斗无关仙命沉底", () => {
    const state = talentPickerState();
    state.config.players.p1 = defaultPlayerConfig("p1", JI_FANG_SHENG, state.config.gameRound);
    const html = renderTalentPopup(state);
    const base = characterBaseTalentSlots(JI_FANG_SHENG);

    const stages = stageBodies(html);
    expect(stages.length).toBeGreaterThan(0);
    for (const [slot, body] of stages) {
      const ids = optionIds(body);
      expect(ids[0]).toBe(base[slot]!.id);
      // 默认仙命在列头（不受「战斗无关沉底」约束）；其余选项里战斗无关的全排在相关之后。
      const defaultId = base[slot]!.id;
      const irrIdxs = ids
        .map((id, index) =>
          (id !== defaultId && isBattleIrrelevantTalent(TALENT_OPTION_BY_ID.get(id)!) ? index : -1))
        .filter((i) => i >= 0);
      const relIdxs = ids
        .map((id, index) =>
          (id !== defaultId && !isBattleIrrelevantTalent(TALENT_OPTION_BY_ID.get(id)!) ? index : -1))
        .filter((i) => i >= 0);
      for (const ii of irrIdxs) {
        for (const ri of relIdxs) expect(ii).toBeGreaterThan(ri);
      }
    }
  });

  test("有搜索词时不注入默认仙命，避免搜索结果混入不匹配项", () => {
    const state = talentPickerState();
    state.config.players.p1 = defaultPlayerConfig("p1", JI_FANG_SHENG, state.config.gameRound);
    state.pickerSearch = "传承";
    const html = renderTalentPopup(state);
    const base = characterBaseTalentSlots(JI_FANG_SHENG);
    for (const [slot, body] of stageBodies(html)) {
      const ids = optionIds(body);
      // 默认仙命不匹配“传承”，不应被注入到搜索结果里。
      expect(ids[0]).not.toBe(base[slot]!.id);
    }
  });
});
