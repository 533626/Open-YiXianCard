<!-- topic: ui-fixture -->

# UI Fixture Consistency Report

生成时间：2026-07-09，UI fixture 全量一致性扫描修复完成（911 ok / 0 mismatch / 0 error）。

## 结论摘要

修复前 911 个 fixture 中 64 个问题样本（62 mismatch + 2 UI error）**全部集中在 UI 导入和
UI runtime 路径**；Engine replay 本身无失效——所有问题样本的原始 fixture 直接进 Engine 都能
跑出原始结果。

扫描口径：原始 fixture → `prepareOriginalReplay/runBattle`；导入 UI config →
`runTS EngineSimulation`；比较 `winnerSide / actorTurnCount / hpDeltaP1MinusP2 / finalHp`。

## 根因分类

| 类别 | 数量 | 根因 | 修复模拟结果 |
| --- | ---: | --- | --- |
| 初始状态污染 | 37 | `battleConfigFromReplayFixture` 从 `defaultPlayerConfig` 起步后，没有覆盖 `initialGuard` 等 replay 初始字段；花沁蕊 `3000003` 被 UI 默认多带 1 层护体 | 只补 replay initial fields 后 37/37 对齐 |
| 原始卡面未保真 | 24 | UI 卡牌索引无法映射部分原始卡，导入时降级为 `普通攻击`；涉及 `慈念曲` 24 次、`梦·崩拳连崩` 1 次 | 只保留 original card config 后 24/24 对齐 |
| 初始状态 + 原始卡面叠加 | 1 | 同一 fixture 同时受护体污染和卡面降级影响 | 两项同时补回后对齐 |
| 顶层历史补丁丢失 | 1 | `historicalCardOverrides` 未从原 fixture 进入 UI 重建 fixture | 恢复顶层 fixture metadata 后对齐 |
| 旧赛季共鸣误启用 | 1 | 普通原版回放没有启用 `historicalSeasonMechanisms=["talentResonance"]`，但 UI runtime 仍把 `talentResonanceId=61` 传入 PlayerState | 按 replay source 开关旧赛季共鸣后对齐 |

## 固化命令

UI fixture adapter 一致性不再用临时脚本检查，固定命令为：

```bash
bun run check:ui-fixtures
```

浏览器 UI 使用面门禁：

```bash
bun run check:ui
```

浏览器 UI 使用面门禁：

```bash
bun run check:ui
```

`check:surfaces` 只收口公开 browser surface。ratatui TUI 与私有 replay corpus 已移至 private companion，不由公开构建或公开门禁执行。

建议新增回归样本：

- 初始护体污染：`e63lwvs/round-16`、`e985tpk/round-02`
- 未索引原始卡面：`edrrxox/round-04`、`e7s3pcn/round-05`
- 梦牌报错：`e5c9l2c/round-06`
- 叠加问题：`ee8pea0/round-05`
- 历史补丁：`e6g9sx6/round-18`
- 旧赛季共鸣开关：`e1bjc99/round-10`

## 修复方向

1. `battleConfigFromReplayFixture` 导入原版 fixture 时，显式覆盖 `defense/anima/momentum/momentumLimit/agility/guard`，不要沿用 `defaultPlayerConfig` 的战斗初始默认值。
2. UI deck slot 对 `sourceKind="original-fixture"` 必须保留每个 slot 的 `originalConfig`，即使卡牌不在 UI 可选索引中也不能降级为 `普通攻击`。
3. `BattleConfig` 或导入态需要保留顶层 replay metadata，至少包括 `historicalCardOverrides`、`catalogCards`、完整 `source`、`maxActorTurns`、`decisionTape`、`randomFallbackTape`。
4. 原版 fixture 的旧赛季共鸣只在 `source.historicalSeasonMechanisms` 显式包含 `talentResonance` 时启用。
