import { describe, expect, test } from "bun:test";
import { defaultPlayerConfig, EMPTY_CHARACTER_ID } from "../data";
import { buildReplayFixture } from "../replay-fixture-builder";
import {
  buildTargetPracticeFixture,
  DUMMY_HP,
  silentDummyCard,
  TARGET_DAMAGE_THRESHOLD_DEFAULT,
  GAME_TURN_LIMIT,
} from "../target-dummy";

describe("静默木桩卡", () => {
  test("零攻零段：id/baseId 0 + attack 0 + attackCount 0，无其它效果字段", () => {
    const card = silentDummyCard();
    expect(card.id).toBe(0);
    expect(card.baseId).toBe(0);
    expect(card.name).toBe("木桩");
    expect(card.attack).toBe(0);
    expect(card.attackCount).toBe(0);
    expect(card.damage).toBeUndefined();
    expect(card.anima).toBeUndefined();
    expect(card.hpCost).toBeUndefined();
    expect(card.def).toBeUndefined();
    expect(card.otherParams).toBeUndefined();
    expect(card.cardType).toBeUndefined();
  });
});

describe("buildTargetPracticeFixture", () => {
  const build = defaultPlayerConfig("p1", 4_000_004, 16);

  test("默认常量：阈值 120、游戏回合上限 32（常量，不可调）", () => {
    expect(TARGET_DAMAGE_THRESHOLD_DEFAULT).toBe(120);
    expect(GAME_TURN_LIMIT).toBe(32);
  });

  test("maxActorTurns = GAME_TURN_LIMIT * 2 = 64（原作整场上限，游戏常量）", () => {
    const fixture = buildTargetPracticeFixture(build, 16);
    expect(fixture.maxActorTurns).toBe(64);
    expect(fixture.firstPlayerSide).toBe("p1");
  });

  test("木桩：高血量、满级 8 格、无仙命/天衍、8 张静默卡", () => {
    const fixture = buildTargetPracticeFixture(build, 16);
    const dummy = fixture.players.p2;
    expect(dummy.side).toBe("p2");
    expect(dummy.label).toBe("木桩");
    expect(dummy.hp).toBe(DUMMY_HP);
    expect(dummy.maxHp).toBe(DUMMY_HP);
    expect(dummy.level).toBe(6);
    expect(dummy.activeSlotCount).toBe(8);
    expect(dummy.talents).toEqual([]);
    expect(dummy.fateStrategies).toEqual([]);
    expect(dummy.deck).toHaveLength(8);
    for (const slot of dummy.deck) {
      expect(slot.baseId).toBe(0);
      expect(slot.originalConfig?.attack).toBe(0);
      expect(slot.originalConfig?.attackCount).toBe(0);
    }
  });

  test("我方 side 固定 p1，且沿用构筑数据", () => {
    const fixture = buildTargetPracticeFixture(build, 16);
    expect(fixture.players.p1.side).toBe("p1");
    expect(fixture.players.p1.characterId).toBe(build.characterId);
    expect(fixture.players.p1.deck).toEqual(build.deck);
  });

  test("buildReplayFixture 走 originalConfig 通道：木桩卡进 fixture 后仍是零攻", () => {
    const fixture = buildTargetPracticeFixture(build, 16);
    const replay = buildReplayFixture(fixture);
    expect(replay.firstPlayerSide).toBe("p1");
    expect(replay.maxActorTurns).toBe(64);
    expect(replay.players.p2.baseMaxHp).toBe(DUMMY_HP);
    expect(replay.players.p2.cards).toHaveLength(8);
    for (const card of replay.players.p2.cards) {
      expect(card.attack).toBe(0);
      expect(card.attackCount).toBe(0);
      expect(card.name).toBe("木桩");
    }
  });

  test("无角色空构筑也能构造 fixture（角色由用户选择后才有伤害）", () => {
    const empty = defaultPlayerConfig("p1", EMPTY_CHARACTER_ID, 16);
    const fixture = buildTargetPracticeFixture(empty, 16);
    expect(fixture.players.p1.characterId).toBe(0);
    expect(fixture.players.p2.maxHp).toBe(DUMMY_HP);
  });
});
