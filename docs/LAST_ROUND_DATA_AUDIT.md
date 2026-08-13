<!-- topic: audit -->

# Last Round Data Audit

本页记录 TS Engine 真实回放 fixture 需要承载的原版 `lastRoundData.*`
字段。本页最初扫描 Steam build `23798322` 的战斗入口、牌效果、角色战斗开始和
天衍仙命共享函数；该历史反编译快照已按缓存策略删除。当前规则查询使用
`research/original-game/extracted/current/decompiled/`，如需复核当时源码语义，必须重新获取对应的旧 Steam depot。

## 字段覆盖

| 原版字段 | TS Engine fixture / state |
| --- | --- |
| `lastRoundData.usedCards` | `lastRoundUsedCardBaseIds`，当前战斗牌组另由 `cards` 承载 |
| `lastRoundData.handCards` | `handCards` / `handCardIds` |
| `lastRoundData.unlockGrids` | `activeSlotCount` |
| `lastRoundData.extraMaxHp` | `extraMaxHp` |
| `lastRoundData.permanentBuffTempDatas` | `permanentBuffTempDatas` |
| `lastRoundData.talentTempDatas` | `talentTempDatas` |
| `lastRoundData.talents` | `talents` |
| `lastRoundData.life` | `lastRoundLife` |
| `lastRoundData.exp` | `lastRoundExp` |

各字段的读取状态以代码为准，不在此手写。

## 源码证据

- `CardActionBase.YiGuaZiJieCheck` 读取 `lastRoundData.usedCards` 并统计 8 张卦牌基础 ID。
- `BattleCharacter.OnBattleStarted` 的天赋 220 兴云布雨读取 `handCards` 与 `usedCards`，按云剑数量加灵气。
- `FateStrategyFunctions.OnBattleStart` 的天衍 27 读取 `handCards.Count`。
- `BattleCharacter.OnBattleStarted` 的共鸣 8/21/35 和逆境契约读取 `lastRoundData.life`。
- `Card_43` 逆境不弃读取双方 `lastRoundData.exp`。

## 注意

不要批量重导出已经校准过的候选 fixture。部分候选夹具含手工历史补丁或
catalog 补牌，直接用 exporter 覆盖会丢失这些补丁并破坏 dashboard。新增字段应由
后续新导出的 fixture 自然带入；旧 fixture 只在对应机制需要真实数据时定点补。
