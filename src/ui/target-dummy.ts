/**
 * 打靶模式的「静默木桩」对手与 fixture 构造。
 *
 * 引擎没有无限 HP / 木桩 / 打靶概念，木桩完全由 UI 侧合成：高血量、无仙命/副职/
 * 天衍、8 张静默木桩卡。木桩的牌走正常出牌生命周期（抽牌/灵气/牌格 hadUsed 都照常），
 * 但 **0 攻击 → 0 段 → 0 伤害**，不会回打、不触发 lethal，也不往我方伤害归因里混入
 * 对方动作点的数据（归因本就只取我方对木桩侧的减少）。
 *
 * 静默木桩卡的证据链（engine-rust 源码，只读引用，不改引擎）：
 * - 通用攻击路径 `engine-rust/src/replay/combat_core.rs`：
 *   `attack = card.attack.unwrap_or(if card.id == BASIC_ATTACK_ID {3} else {0}) + bonus`；
 *   `attack_count = card.attack_count.unwrap_or(if attack > 0 {1} else {0}).max(0)`；
 *   `for _ in 0..attack_count { if attack > 0 { self.apply_attack(...) } }`。
 *   故 `attack: Some(0)` + `attack_count: Some(0)` → 循环体不执行、`apply_attack`
 *   永不被调用；即便外部把 attack_count 设为 N，循环内 `if attack > 0` 仍是第二道拦截。
 * - 基础攻击路径 `engine-rust/src/replay/flow_card_effect_primary_early.rs`
 *   `apply_basic_attack_effect`：`attack = card.attack.unwrap_or(BASIC_ATTACK_DAMAGE) + ...`，
 *   同样 `attack > 0` 才 `apply_attack`；木桩无逍遥曲/返璞 buff，bonus 恒 0。
 * - 结论：木桩每回合出牌但 0 段 0 伤害。这是对「木桩回打 3 伤害成为噪音」的正解，
 *   木桩不模拟任何原版卡牌效果，不构成 AGENTS.md 禁止的占位行为。
 */

import type { OriginalCardConfig } from "./domain";
import {
  DEFAULT_CAREER_ID,
  EMPTY_CHARACTER_ID,
  defaultPlayerConfig,
} from "./data";
import type {
  BattleConfig,
  PlayerConfig,
} from "./types";

/** 木桩生命上限：足够大，32 回合（64 actorTurn）内不可能被打死。 */
export const DUMMY_HP = 1_000_000;
/** 打靶默认累计伤害阈值。 */
export const TARGET_DAMAGE_THRESHOLD_DEFAULT = 120;
/**
 * 游戏常量回合上限：原作基础战斗最大行动回合数 64（按单角色行动计数），
 * 即 32 回合（双方各动一次算一回合，`battleRound = ceil(actorTurn/2)`）。
 * 见 engine-rust/src/replay.rs `DEFAULT_MAX_ACTOR_TURNS`。打靶模式照搬此常量，
 * 引擎始终跑满 64 actorTurn，UI 按「显示至回合」（绝对有效回合数）裁剪展示窗口。
 */
export const GAME_TURN_LIMIT = 32;
/** 木桩境界：6 阶满境界，场上 8 格。 */
const DUMMY_LEVEL = 6;
/** 木桩卡名（占位展示用，无规则含义）。 */
const DUMMY_CARD_NAME = "木桩";

/**
 * 静默木桩卡：id/baseId 0（普通攻击框架）+ `attack: 0` + `attackCount: 0`，
 * 无其它效果字段。证据见文件头注释：两条攻击路径都在 `attack > 0` 时短路，
 * 0 攻击恒等于 0 段 0 伤害。
 */
export function silentDummyCard(): OriginalCardConfig {
  return {
    id: 0,
    baseId: 0,
    name: DUMMY_CARD_NAME,
    attack: 0,
    attackCount: 0,
  };
}

/**
 * 打靶 fixture：我方（p1）= 用户构筑，对手（p2）= 静默木桩。
 * `maxActorTurns = GAME_TURN_LIMIT * 2 = 64`（游戏常量，不可调）：
 * 引擎始终跑满原作整场上限，UI 按「显示至回合」裁剪展示窗口，
 * 这样默认看打到 120 的过程，想看后续回合伤害就往后调，无需重跑引擎。
 *
 * `sourceKind: "original-fixture"` 让 `buildReplayFixture` 走
 * `slot.originalConfig` 通道，木桩 8 格挂上 `silentDummyCard()`；我方卡槽无
 * originalConfig 时照常回退 `getCardVariant(slot).config`，与双方对战同路径。
 */
export function buildTargetPracticeFixture(
  build: PlayerConfig,
  gameRound: number,
): BattleConfig {
  const round = Number.isFinite(gameRound) ? Math.max(1, Math.trunc(gameRound)) : 1;
  const dummy = defaultPlayerConfig("p2", EMPTY_CHARACTER_ID, round);
  dummy.label = "木桩";
  dummy.level = DUMMY_LEVEL;
  dummy.hp = DUMMY_HP;
  dummy.maxHp = DUMMY_HP;
  dummy.activeSlotCount = 8;
  dummy.talents = [];
  dummy.fateStrategies = [];
  dummy.careerName = DEFAULT_CAREER_ID;
  dummy.deck = Array.from({ length: 8 }, () => ({
    baseId: 0,
    level: 0,
    originalConfig: silentDummyCard(),
  }));
  return {
    sourceKind: "original-fixture",
    firstPlayerSide: "p1",
    gameRound: round,
    maxActorTurns: GAME_TURN_LIMIT * 2,
    decisionTape: [],
    randomFallbackTape: [],
    players: {
      p1: { ...build, side: "p1" },
      p2: dummy,
    },
  };
}
