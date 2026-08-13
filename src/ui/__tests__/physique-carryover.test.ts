import { describe, expect, test } from "bun:test";
import { defaultPlayerConfig } from "../data";
import { PERMANENT_PHYSIQUE_KEY, derivePlayerBattleStats } from "../derived-state";
import { applyPhysiqueValue, syncPlayerDerivedStats } from "../main-utils";
import { buildReplayFixture } from "../replay-fixture-builder";
import { CoreBuff } from "../domain";
import type { BattleConfig, PlayerConfig } from "../types";

/**
 * 引擎只从 `permanentBuffTempDatas` 读体魄，界面上的 `buffs[Physique]` 只用于显示。
 * 两者脱钩过一次：自由构筑里设的体魄在开战瞬间消失，左侧生命上限跟着掉回去。
 */
/** 李㵘（4000005）属断玄宗；体魄是断玄宗机制，且上限随修炼轮增长，第 1 轮为 0。 */
const LI_MAN_CHARACTER_ID = 4_000_005;

function liManWithPhysiqueRoom(): PlayerConfig {
  const player = defaultPlayerConfig("p1", LI_MAN_CHARACTER_ID, 16);
  syncPlayerDerivedStats(player, 16, true);
  return player;
}

function configWith(player: PlayerConfig): BattleConfig {
  return {
    gameRound: 16,
    firstPlayerSide: "p1",
    maxActorTurns: 64,
    players: { p1: player, p2: defaultPlayerConfig("p2", LI_MAN_CHARACTER_ID, 16) },
  } as BattleConfig;
}

describe("体魄跨越战斗开始", () => {
  test("设置体魄会同时写进引擎读取的永久 buff 通道", () => {
    const player = liManWithPhysiqueRoom();
    applyPhysiqueValue(player, 7);

    expect(player.buffs[CoreBuff.Physique]).toBe(7);
    expect(player.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY]).toBe(7);
  });

  test("非断玄宗角色不会带着幽灵体魄进引擎", () => {
    const player = liManWithPhysiqueRoom();
    applyPhysiqueValue(player, 6);
    player.characterId = 1_000_001;
    syncPlayerDerivedStats(player, 16, false);

    expect(player.buffs[CoreBuff.Physique]).toBeUndefined();
    expect(player.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY]).toBeUndefined();
  });

  test("清零体魄会同时清掉永久 buff，不留幽灵上限", () => {
    const player = liManWithPhysiqueRoom();
    applyPhysiqueValue(player, 5);
    applyPhysiqueValue(player, 0);

    // 断玄宗保留 0 值以便界面显示上限，引擎侧则必须彻底没有这一项。
    expect(player.buffs[CoreBuff.Physique]).toBe(0);
    expect(player.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY]).toBeUndefined();
  });

  test("交给引擎的 fixture 带着体魄，baseMaxHp 与体魄相加还是界面上的上限", () => {
    const player = liManWithPhysiqueRoom();
    applyPhysiqueValue(player, 6);
    const stats = derivePlayerBattleStats(player);
    const fixture = buildReplayFixture(configWith(player));
    const fixturePlayer = fixture.players.p1;

    expect(fixturePlayer.permanentBuffTempDatas[PERMANENT_PHYSIQUE_KEY]).toBe(6);
    // 体魄被排除在 baseMaxHp 之外，只能靠永久 buff 通道补回来；漏了这一路上限就少 6。
    expect(fixturePlayer.baseMaxHp + 6 + (fixturePlayer.extraMaxHp ?? 0)).toBe(stats.maxHp);
  });
});
