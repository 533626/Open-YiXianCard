import { describe, expect, test } from "bun:test";
import {
  defaultPlayerConfig,
  defaultBattleConfig,
  defaultHpForGameRound,
  derivePlayerBattleStats,
  levelForGameRound,
  physiqueLimitForPlayer,
} from "../data";
import { isTalentSelectableForCharacter, normalizePlayerTalents } from "../data/talents";
import { applyGameRoundDefaults } from "../main-utils";
import { syncPlayerDerivedStats } from "../main-utils";
import { handleAction } from "../main-actions";
import { handleBuffInput, handleNamedField } from "../main-fields";
import type { AppState, SimulationResult } from "../types";
import { CoreBuff } from "../domain";

describe("UI 入战前派生状态", () => {
  function configuredBattleConfig() {
    const config = defaultBattleConfig();
    config.players.p1 = defaultPlayerConfig("p1", 4_000_004, config.gameRound);
    config.players.p2 = defaultPlayerConfig("p2", 4_000_005, config.gameRound);
    return config;
  }

  test("默认生命按修炼轮次累计基础上限和轮次上限收益", () => {
    expect(levelForGameRound(16)).toBe(5);
    expect(defaultHpForGameRound(16)).toBe(105);
  });

  test("锻玄默认体魄只提高生命上限，不把当前生命抬满", () => {
    const config = configuredBattleConfig();
    expect(config.players.p1.hp).toBe(105);
    expect(config.players.p1.buffs[CoreBuff.PhysiqueLimit]).toBe(85);
    expect(config.players.p1.buffs[CoreBuff.Physique]).toBe(80);
    expect(config.players.p1.maxHp).toBe(185);
    expect(derivePlayerBattleStats(config.players.p1)).toMatchObject({
      hp: 105,
      maxHp: 185,
      maxHpWithoutPhysique: 105,
      extraMaxHp: 0,
    });
  });

  test("锻玄第一轮基础体魄为零，仙命体魄加成照常生效", () => {
    const tuKui = defaultPlayerConfig("p1", 4_000_002, 1);
    const jiFangsheng = defaultPlayerConfig("p1", 4_000_004, 1);

    expect(tuKui.buffs[CoreBuff.PhysiqueLimit]).toBe(9);
    expect(tuKui.buffs[CoreBuff.Physique]).toBe(3);
    expect(jiFangsheng.jiFangshengInitialFateRank).toBe(0);
    expect(jiFangsheng.buffs[CoreBuff.PhysiqueLimit]).toBe(6);
    expect(jiFangsheng.buffs[CoreBuff.Physique]).toBe(0);
  });

  test("浏览器手工模拟李㵘默认带拳架势", () => {
    const liYan = defaultPlayerConfig("p1", 4_000_005, 16);

    expect(liYan.talents).toContain(208);
    expect(liYan.buffs[CoreBuff.FistStance]).toBe(1);
  });

  test("陨星淬体入战前同时增加当前生命和生命上限", () => {
    const config = configuredBattleConfig();
    config.players.p1.talents[1] = 30_059;

    expect(derivePlayerBattleStats(config.players.p1)).toMatchObject({
      hp: 122,
      maxHp: 202,
      maxHpWithoutPhysique: 105,
      extraMaxHp: 17,
    });
  });

  test("锻体仙命入战前同时增加当前生命和生命上限", () => {
    const config = configuredBattleConfig();
    config.players.p1.talents[1] = 30_001;

    expect(derivePlayerBattleStats(config.players.p1)).toMatchObject({
      hp: 117,
      maxHp: 197,
      maxHpWithoutPhysique: 105,
      extraMaxHp: 12,
    });
  });

  test("体修入道同步体魄上限和开局体魄", () => {
    const config = configuredBattleConfig();
    const player = config.players.p1;
    player.talents[1] = 30_146;

    syncPlayerDerivedStats(player, config.gameRound, true);

    expect(player.buffs[CoreBuff.PhysiqueLimit]).toBe(97);
    expect(player.buffs[CoreBuff.Physique]).toBe(88);
    expect(player.maxHp).toBe(193);
  });

  test("搏命之勇同步体魄和体魄上限", () => {
    const player = defaultPlayerConfig("p1", 4_000_002, 16);

    expect(player.talents).toContain(171);
    syncPlayerDerivedStats(player, 16, true);

    expect(player.buffs[CoreBuff.PhysiqueLimit]).toBe(84);
    expect(player.buffs[CoreBuff.Physique]).toBe(79);
    expect(player.maxHp).toBe(184);
    expect(derivePlayerBattleStats(player)).toMatchObject({
      hp: 105,
      maxHp: 184,
      maxHpWithoutPhysique: 105,
      extraMaxHp: 0,
    });
  });

  test("选择仙命后会刷新派生生命和体魄", () => {
    const config = configuredBattleConfig();
    const target = {
      dataset: { action: "pick-talent", talentId: "30146" },
    } as unknown as HTMLElement;

    handleAction({ currentTarget: target } as unknown as Event, {
      state: {
        view: "setup",
        workbenchMode: "duel",
        target: null,
        config,
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
      },
      render: () => {},
      runBattle: () => {},
      resetBattle: () => {},
      stopAuto: () => {},
      toggleAuto: () => {},
      adjacentCompletedTurnFrameIndex: (_result, currentIndex) => currentIndex,
    });

    expect(config.players.p1.talents[1]).toBe(30_146);
    expect(config.players.p1.buffs[CoreBuff.PhysiqueLimit]).toBe(97);
    expect(config.players.p1.buffs[CoreBuff.Physique]).toBe(88);
  });

  test("选中副职兼修仙命后直接跳转副职页，普通仙命不跳转", () => {
    const config = configuredBattleConfig();
    const state: AppState = {
      view: "setup",
      workbenchMode: "duel",
      target: null,
      config,
      activeSide: "p1",
      pickerMode: "talent",
      selectedSlot: 0,
      selectedTalentSlot: 2,
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
    const callbacks = {
      render: () => {},
      runBattle: () => {},
      resetBattle: () => {},
      stopAuto: () => {},
      toggleAuto: () => {},
      adjacentCompletedTurnFrameIndex: (_result: SimulationResult, currentIndex: number) => currentIndex,
    };

    // 金丹槽选中副职兼修仙命（10_188）→ 直接进副职页补齐兼修副职
    handleAction(
      { currentTarget: { dataset: { action: "pick-talent", talentId: "10188", talentSlot: "2" } } } as unknown as Event,
      { state, ...callbacks },
    );
    expect(config.players.p1.talents[2]).toBe(10_188);
    expect(state.pickerMode).toBe("career");

    // 普通仙命选中后停留在仙命页
    handleAction(
      { currentTarget: { dataset: { action: "pick-talent", talentId: "30146", talentSlot: "3" } } } as unknown as Event,
      { state, ...callbacks },
    );
    expect(config.players.p1.talents[3]).toBe(30_146);
    expect(state.pickerMode).toBe("talent");
  });

  test("姬方生初始仙命档位参与体魄上限派生", () => {
    const player = defaultPlayerConfig("p1", 4_000_004, 16);
    expect(player.jiFangshengInitialFateRank).toBe(4);
    expect(physiqueLimitForPlayer(player, 16)).toBe(85);
  });

  test("修炼轮变化同步双方生命、生命上限和激活卡槽", () => {
    const config = configuredBattleConfig();
    config.gameRound = 4;

    applyGameRoundDefaults(config);

    expect(config.players.p1.gameRound).toBe(4);
    expect(config.players.p2.gameRound).toBe(4);
    expect(config.players.p1.activeSlotCount).toBe(6);
    expect(config.players.p2.activeSlotCount).toBe(6);
    expect(config.players.p1.hp).toBe(defaultHpForGameRound(4));
    expect(config.players.p2.hp).toBe(defaultHpForGameRound(4));
    expect(config.players.p1.maxHp).toBe(defaultHpForGameRound(4) + (config.players.p1.buffs[CoreBuff.Physique] ?? 0));
    expect(config.players.p2.maxHp).toBe(defaultHpForGameRound(4) + (config.players.p2.buffs[CoreBuff.Physique] ?? 0));
  });

  test("生命修正会同步派生当前生命和生命上限", () => {
    const config = configuredBattleConfig();
    const input = { id: "player-p1-lifeModifier", value: "14" } as HTMLInputElement;

    handleNamedField({ currentTarget: input } as unknown as Event, {
      state: {
        view: "setup",
        workbenchMode: "duel",
        target: null,
        config,
        activeSide: "p1",
        pickerMode: "none",
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
      },
      render: () => {},
    });

    expect(config.players.p1.hp).toBe(119);
    expect(config.players.p1.maxHp).toBe(199);
  });

  test("非锻玄角色生命和上限始终同步派生", () => {
    const config = configuredBattleConfig();
    config.players.p1 = defaultPlayerConfig("p1", 1_000_001, 16);
    const input = { id: "player-p1-lifeModifier", value: "3" } as HTMLInputElement;

    handleNamedField({ currentTarget: input } as unknown as Event, {
      state: {
        view: "setup",
        workbenchMode: "duel",
        target: null,
        config,
        activeSide: "p1",
        pickerMode: "none",
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
      },
      render: () => {},
    });

    expect(config.players.p1.hp).toBe(108);
    expect(config.players.p1.maxHp).toBe(108);
  });

  test("开局体魄值不能超过体魄上限", () => {
    const config = configuredBattleConfig();
    const input = {
      dataset: { side: "p1", buff: CoreBuff.Physique },
      value: "999",
    } as unknown as HTMLInputElement;

    handleBuffInput({ currentTarget: input } as unknown as Event, {
      state: {
        view: "setup",
        workbenchMode: "duel",
        target: null,
        config,
        activeSide: "p1",
        pickerMode: "none",
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
      },
      render: () => {},
    });

    expect(config.players.p1.buffs[CoreBuff.PhysiqueLimit]).toBe(85);
    expect(config.players.p1.buffs[CoreBuff.Physique]).toBe(85);
    expect(config.players.p1.maxHp).toBe(190);
  });
});

describe("李㵘得炁仙命入战改写", () => {
  test("选了得炁后炼气固定槽变为灵炁奔涌，不再锁凡躯", () => {
    const player = defaultPlayerConfig("p1", 4_000_005, 16);
    expect(player.talents[0]).toBe(204);
    player.talents[4] = 208;
    normalizePlayerTalents(player);
    expect(player.talents[0]).toBe(209);
    expect(player.talents).toContain(208);
    expect(player.talents).not.toContain(204);
  });
});

describe("李㵘灵炁奔涌入战校验", () => {
  test("灵炁奔涌属于李㵘合法入战仙命，可开战", () => {
    
    expect(isTalentSelectableForCharacter(4_000_005, 209)).toBe(true);
    expect(isTalentSelectableForCharacter(4_000_004, 209)).toBe(false);
  });
});
