---
name: battle-rule-development
description: 在 engine-rust/ 新增或修正战斗规则、排查回放 mismatch、判定某机制是否该实现时使用。覆盖证据取证顺序、首差定位、赛季归属与语料覆盖筛选、战斗内外范围划界。
---

# 战斗规则开发

## 开发顺序

从真实回放或有明确证据入口的规则缺口出发。若目标是 replay mismatch，先定位首差再谈规则：

```bash
bun battle-evaluator/scripts/dump-aligned-rust-full-state.ts <case>/round-NN [--fixture <fixture.json>]
```

它把原作逐张牌 trace 与 Rust canonical 的完整状态逐 checkpoint 对齐（核心四元 + 经
`buff-id-rust-key-map.json` 映射的 buff；stage 目录被清理或看历史批次时用 `--fixture`/`--batch`）；字段与诊断解释见
`research/original-game/REPLAY_EXTRACTION.md`。**首差定位不出来时不要只凭终局三元猜规则**，
先补齐 original-client trace。

拿到首差的卡或共享 hook 后，取证必须同时覆盖四项，缺一项都不足以动手：

- `CardConfig` 数值与 `effectClass`
- 对应 `Card_*` 类
- 共享调用链中的目标对象与执行时机
- 相关 `BuffConfig` 分类及写入、读取、消费位置

再写最小 Rust 契约、实现、接上 handler/catalog。验证要求：契约精确通过，受影响 fixture 的
`winner` / `actorTurn` / `hpDelta` 三元 exact。属于 replay admission 的，本批 Rust canonical
与 browser UI adapter 都要 exact。

查任何游戏机制先跑：

```bash
bun battle-evaluator/scripts/lookup-evidence.ts <名字|id>
```

它把仙命/卡牌原文、天衍配置、Buff、反编译位置一次拉齐。

## 证据优先级

1. `research/original-game/BASE_BATTLE_RULES.md`
2. 原游戏反编译代码与解码配置
3. Rust Engine 最小规则契约测试

UI 必须经 `src/ui/` Worker 适配 Rust/WASM。`engine-ts/` 已移出仓库（可从 git 历史取回），
Rust 是唯一实现；历史快照与旧实现不能反向作为原版规则证据。

## 战斗内 / 战斗外划界

Rust Engine 只模拟影响本场战斗胜负的过程。炼化、成长、换牌、置换、抽牌/发牌、战前构筑、
战后命元/修为/奖励等只记录不实现过程，其最终结果由回放夹具的初始状态和最终卡组承载。

`refine`、`change` 和纯战后卡牌不得注册进战斗内核；一张牌同时有战斗内主体和战后分支时，
只实现战斗内主体。Rust handler/catalog 双向完整性门禁会拒绝缺失或误注册。

范围定义在 `battle-evaluator/catalog/scope-policy.ts`，新实现必须在 Rust 契约中落地。

## 赛季机制：反编译里存在 ≠ 该实现

历史赛季机制本身不模拟：气运、命坊（含契约类持久 buff）、命运分支、遗迹法器、共鸣仙命、
幻景、刻印、梦境，以及秘境仙命、活动卡。

当前赛季为仙魔（`SeasonMechanismType.XianMo = 10`，build 25621897 起）；上一赛季天衍仙命（9）的
语料与实现继续保留。当前赛季对历史赛季有 callback，这部分要实现——判据只有一个：**它在真实回放里出现过**。
仙魔战斗策略未实现前由 `engine-rust/data/xian-mo-battle-strategies.json` fail closed，差异与待办见
`research/original-game/BUILD_25497139-to-25621897_RULE_DELTA.md`。

按反编译穷举出的「原版有、引擎没有」清单，必须先过赛季归属 + 语料覆盖两道筛选：
`source.seasonMechanism` 在本仓语料里只有 0（无赛季）、9（天衍）与 10（仙魔）三种取值；在 fixture 里出现
0 次的机制不可达也不可验证，实现它等于凭猜写规则。已证伪的清单可直接复用
`docs/GITHUB_YIXIAN_HIGH_VALUE_PROJECTS.md`「战斗内升阶缺口的赛季归属」。
