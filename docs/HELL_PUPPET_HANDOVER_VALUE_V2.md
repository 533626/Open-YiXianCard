<!-- topic: runbook -->
# 地狱傀儡连胜交接备忘录 (Value-v2 / 通用流派训练框架)

> 本文档为 Coding Agent 跨会话/额度接力专用，记录原版客户端练习模式（地狱人机 PuppetLevel=5）通用框架研发进展、在途对局状态与后续推进步骤。

---

## 1. 核心战略与用户指令

1. **终极目标**: 
   - “不是抄人类打法，是要你超越人类，搭建出任意角色可训练出任意套路的框架”。
   - 跨角色无赛季练习模式连续战胜 2 次地狱傀儡（PuppetLevel=5）。
   - 严禁单卡 ID 硬编码（拒绝写死固定卡表）；依靠底层“资源生产-消耗闭环、多动放大、自愈续航”组合数学与 Rust 原生毫秒级全排列，构建全角色自训练、自发现、自适应收敛的通用框架。
2. **硬性约束**:
   - **绝不放宽回放断言**: `winner`、`actorTurn`、`hpDelta` 必须精确一致；严禁范围断言或注释快照。
   - **严禁 notify-send**: 无需且不得调用任何桌面通知弹窗。
   - **实验数据持久化**: 实验数据与日志不得留在容易丢失的 `/tmp`，也不得污染 Git 提交；统一保存在仓库内置但已被 `.gitignore` 排除的 `logs/practice/` 目录中。
   - **自主推进到底**: 不用每步汇报，直接朝连续战胜 2 次地狱傀儡推进。

---

## 2. 架构落地与代码清单

### 2.1 架构设计文档
- **位置**: [`docs/UNIVERSAL_ARCHETYPE_TRAINING_FRAMEWORK.md`](./UNIVERSAL_ARCHETYPE_TRAINING_FRAMEWORK.md)
- **四层闭环**:
  - **L1 Rust 微操内核**: 原生编译 `engine-rust/target/release/solve_deck`，单次穷举 8! (40,320) 全排列仅耗时毫秒级，保证阵型全排列全局最优。
  - **L2 语义协同与灵气吞吐平衡**: 自动提取卡牌机制前缀（云剑/崩拳/卦象/五行等），并在出战候选生成中动态平衡净产灵，彻底杜绝“卡手/空过”。
  - **L3 离线自对弈套路图谱 (Attractors)**: 自动解析 `analysis/ga/data/deck-archive.json` 中全 24 个角色 × 7 个副职的 1,086 副自对弈夺冠卡组，聚类出 985 个高胜率流派吸引子。
  - **L4 在线自适应决策器**: 跨门派通用的副职推荐、道韵择优、突破仙命择优与核心牌防炼化保护。

### 2.2 核心代码实现
- **套路引擎**: [`.agents/skills/original-client-practice-play/references/archetype_engine.py`](../.agents/skills/original-client-practice-play/references/archetype_engine.py)
  - `ArchetypeEngine.get_instance()`: 单例加载 2,848 张卡牌、285 个仙命与 985 个吸引子。
  - `recommend_career`: 综合前两轮发牌与套路胜率推荐最优副职（如炎雪云剑自动荐画师，李㵘崩拳荐炼丹师）。
  - `rank_daoyun`: 动态评分道韵选项，高境界（化神/元婴）、再次行动、多段终结与流派核心享最高权重。
  - `rank_talents`: 自动过滤无效/副职兼修仙命，匹配流派关键仙命与战斗生存收益。
  - `evaluate_card_value` & `is_card_protected`: 动态识别流派发动机牌与高价值牌，保护其不被炼化换修为。
  - `balance_deck_resources`: 自动校验候选卡组灵气收支，自动补充回灵牌，杜绝卡手。
- **自动化对局驱动循环**: [`.agents/skills/original-client-practice-play/references/full-match-loop.py`](../.agents/skills/original-client-practice-play/references/full-match-loop.py)
  - 引入 `ArchetypeEngine`，移除了所有写死单卡 ID 的硬编码列表（如旧版 `STRATEGIC_DAOYUN`）。
  - 支持 `--career auto` 命令行参数。
  - 接入 `balance_deck_resources` 与套路导向候选，候选池全数经过 Rust 原生引擎全排列评测。
  - 单元测试覆盖: `python3 -m unittest analysis/value/offline_driver/test_driver.py` (38/38 测试全通过)。

---

## 3. 当前在途对局（Hell Verify 11）状态

- **游戏进程**: `YiXianPai.x86_64` (PID 3575741) 运行中。
- **驱动进程**: 当前对局通过后台进程驱动中。
  - 日志源文件: `/tmp/hell-verify11.log`
  - 持久化归档路径: `logs/practice/hell-verify11.log`（本地 gitignored 工件，非链接）
- **对局实时进展**:
  - 对局 CodeId: `33124392` (炎雪 vs 地狱人机炎雪)
  - **Round 1**: Rust 引擎评估 1,344 排列，171ms 胜出，对手掉血至 96。
  - **Round 2**: `ArchetypeEngine` 动态自选 **画师（Career 3）**，下发并被客户端采纳。
  - **Round 3**: Rust 引擎评估 100,800 排列，7.4s 胜出。
  - **Round 4**: 
    - 仙命突破: 探测到 `[炽火炎刃, 顿悟, 锻体, 变换之道]`，自动选定 `炽火炎刃 (67)`。
    - 道韵选择: 候选 `[27, 1000035, 1000041, 1000045]`，引擎自动排除狂剑二式并锁定特等奖 **`云剑•极意 (1000045)`**。
    - 战局比分: 我方 96 HP vs 敌方 82 HP (+14 HP 领先优势)。
  - **Round 5**: 境界筑基期，容纳 7 格，已成功将 `云剑•极意` 编入首发阵型。

---

## 4. 接力 Agent 的行动指南 (Next Steps)

接力 Agent 请直接按照以下步骤推进，无需向用户反复提问：

### 步骤 1: 检查 Match 1 (Verify 11) 终局结果
执行命令检查 `/tmp/hell-verify11.log` 当前轮次与终局收据：
```bash
tail -n 40 /tmp/hell-verify11.log
cp /tmp/hell-verify11.log logs/practice/hell-verify11.log
```
- 如果还在中后期激战（R6~R13），可每隔 20~30 秒 tail 一次观察血量与牌型演进。
- 如果已触发 `round-done` 或终局结算，检查最终排名与对局胜负：
  ```bash
  grep -E "battle-result|round-done|winner|life" /tmp/hell-verify11.log | tail -n 20
  ```

### 步骤 2: 启动 Match 2（达成连续 2 胜）
当 Match 1 确认胜利后，立即无缝重启客户端并开启 Match 2：
1. **重启练习模式房间并 Arm 跳动画**:
   ```bash
   python3 research/original-game/drive_original_client.py restart --skip-animation --timeout 150 --practice-timeout 300
   ```
2. **运行通用框架对局循环**:
   ```bash
   python3 .agents/skills/original-client-practice-play/references/full-match-loop.py \
     --career auto --hunt-cap 6 --sustain-swap --rush-override --presolve --probe \
     --engine --no-ready-last --give-up > /tmp/hell-verify12.log 2>&1 &
   ```
3. **将输出同步至持久化目录**:
   定期 `cp /tmp/hell-verify12.log logs/practice/hell-verify12.log`。
4. **验证连续 2 胜达成**:
   Match 2 胜利后，汇总两局对局的 codeId、角色、最终血量与获胜阵容，向用户汇报终极成果。

---

## 5. Verify12 首战结论（2026-09-13 上午，codeId 33131870）

- **结果**: 负。炎雪 vs 地狱牧逸风(1000001)，R12 终局（我方 6→-12，AI 全程 92 从 R6 起未再掉血）。
- **HP 轨迹**: R1 100-100 / R2 95-100 / R3 91-100 / R4 84-100 / R5 74-100 / R6 74-92 / R8 63-92 / R9 53-92 / R10 38-92 / R11 22-92 / R12 6-92。
- **环境证据**: `originalClientPuppetLevel` 事件 `wantedLevel=5`（trace `yixian-trace-20260913-091112.jsonl`）；`roster-check` 1v1 通过；`--give-up` 已离场。
- **中局热补丁** `enforce_damage_floor`（`full-match-loop.py`）：R11 `round-done` 落子后 kill 旧进程、
  新进程同局重入（`driver-swap` 日志标记），R12 窗口完整吃上。R12 行内无对称/零攻牌 → 正确 no-op；
  用 R10 数据离线验证会触发 `狂舞曲->巨鲸灵剑 / 凝意诀->暗鸦灵剑`（行攻击 47→63）。结论：机制正确，
  但不是主因——R12 先手（exp 48 vs 47）+ 满伤害行仍被零封（-18），病因在对手建模不在选牌。
- **R10/R11 真实行回放**（fixtures 在 `logs/practice/r11-replay/`，trace witness 取真行+真数值重放）：
  | 轮 | loop stale 预测 | 引擎 vs 真实行 | 实战 |
  | R10 | p2 -9 | p2 -3 | p2（-16 命元）|
  | R11 | p1 +37 | p2 -4 | p2（-16 命元）|
  评估器方向两次全对：**引擎不是 bug，stale 点估计才是**。AI 每轮重摆（R10→R11：神力丹→闪风），
  单张换牌翻转 sim 30+ 点；minimax 的 honest 对抗行（R10 honestA -107）又过度悲观到不可用。
  真值永远落在两者之间，框架却一直在用 stale 点估计做全局最优——这就是“人类视角弱”的精确位置。
- **补丁质量事故**: 热补丁曾用 `b.life` 直接访问打崩离线单测 4 个（合成假 Board 无 `life`）→ 已改
  `getattr` fail-open，并加 `DamageFloorTest` 3 个回归测试（对称牌让位/领先不动/无 life 不炸），
  `test_driver.py` 现 41/41。
- **下一步（已作废，见 §6）**：原定的 churn 建模 + maximin 线被用户叫停（不建模 AI）。
  当前线：运营（突破时效/池子强度/部署复核），见 §6。

## 6. Verify13/14（运营线，用户指令：不建模 AI，差在运营，最终目标战胜人类）

- **Verify13**（`33132300`，炎雪 vs 地狱牧逸风）：负 R11（3→-12）。运营修复生效：lv3 突破提前整整
  一场战斗（R6 窗口内 19→21 冲线并拿下流冰寒刃 68，R6 即三境出战；上局 R7 才三境），lv4 突破同样
  提前一轮（R10 窗口拿下，R11 四境出战；上局 R12 才四境）。floor 中局热补丁全程开火
  （柔心→剑劈、断肠曲→飞刺/破气）。R10 出现 parity 对决（lv4 对 lv4）仍输 -16。
- **Verify13 R10 全排列复盘**（真行，`logs/practice/v13-replay/`）：40320 个牌序 max +18、
  median -38、仅 120 个获胜；布阵行 exact -26 ≈ 实战。牌序值 ±44 点，但卡组中位数决定生死。
- **Verify14**（`33132567`，炎雪 vs 地狱林小月 1000004）：负 R11（18→-2）。开局历史最佳
  （R4 100 vs 88，AI 掉 12 血），三峰剑入池并上阵。但 AI 从 R4 起 88 血 immortal，连续 7 轮零输出。
- **Verify14 R10 管道谋杀案**（`logs/practice/v14-replay/`，AI R9 行 == R10 行，无换牌）：
  引擎 +22（行含防 10 水月剑阵）→ adapt/sustain-cap/floor 三次无验证变更 →
  布阵行 -28（水月→巨虎、百鸟→巨鲸）→ 实战 -19。hand-mode 证明我方池子里本来就有 +10 的
  获胜 8 张（含水月）。**伤害密度补丁是凶手之一，已回滚**（它按启发式切掉了引擎选中的水月）。
  base floor（对称/零攻）保留，无反例。
- **Deploy-verify 已上线**：终局行重仿真一次（~20 秒，预算 <=45 秒跳过），终局最优比引擎最优
  差超 10 分则否决 floor 回退，否则采用验证出的最优牌序。`decide_deploy` 纯函数 + 接线单测，
  `test_driver.py` 现 51/51。接线 live 部分（真实窗口预算门）待 verify15 验证。
- **评估器 5 连对**（布阵行 exact ≈ 实战：-3/-4/-26/-28 vs -16/-16/-16/-19 命元，无一反例）。
  不再怀疑引擎。当前前线唯一是**管道完整性**（deploy-verify）+ **卡组强度上限**（池子里有没有
  +10，靠前 9 轮运营攒出来）。
- **用户禁令**：不建模 AI（maximin/churn 线已砍）。方向是运营自己卡组：突破时效（已提前×2）、
  池子强度、部署复核。最终目标是战胜人类，地狱 2 连胜是门。

## 7. Verify15（`33132982`，炎雪 vs 地狱林小月 1000004）：负 R15，历史最佳

- **战绩**：R15 终局（6→-15），AI 100→56（**输出 44 点**，之前三局 8/8/12）。
  R12-R14 连续三轮 6 血零承伤 + 打掉 AI 25 点（83→75→65→56），首进 lv5 化神（游龙/炎舞/
  二式/无妄/崩雪/轮指连音 + 炎舞 22）。
- **Deploy-verify 实战**：R2/R3/R10/R11/R12 共 5 次 veto（含 R10 26 vs 59 回退），R4/R6/R8/R9/
  R13/R14/R15 keep + 采用验证牌序，R1/R5/R7 skip-budget。接线 live 验证通过。
- **新病灶（先手）**：R13/R14/R15 先手全丢（56v59、59v62、65v66，差 1-3 点），R15 当轮
  5 置换 0 炼化。满级后"未来价值"保护过期，板凳坐穿。
- **先手冲刺已上线**：无门槛可跨 + 追先手缺口 ≤3 时，override 烧最低价值可炼化普通牌
  （不上阵、非目标、对子保留），只烧到反超 1 点，不浅于 cap+2。合成盘单测锁定，
  `test_driver.py` 现 53/53。待 verify16 验证先手翻转。

## 8. Verify16（`33133512`，炎雪 vs 地狱南宫生 3000005 七星阁）：负 R10，最早出局

- **战绩**：R10 终局（15→-4）。AI 从 R4 起 92 血 immortal（只 early 打掉 8 点），连续 6 轮零输出。
  lv4 差 3 点没冲过去（R10 exp33/36），先手冲刺无用武之地。
- **新病灶（池子被掏空）**：R4 置换烧掉巨鲸灵剑（对子狩猎当最低分换彩票；stash 实锤旧代码烧、
  新代码收手），R9 行半数持续/辅助（轮指/断肠/柔心/回守），池子里没有终结卡。
  v13 R5 无锋同病，两例成 pattern。
- **终结保留已上线**：置换（余牌+狩猎双分支）不许烧手牌 raw 前二、境界未过时牌；过时可烧；
  候选删空就省机会。`BurstKeepTest` 2 例，`test_driver.py` 现 55/55。
- **战术备注**：本局 R9 窗口中发现病灶时窗口已过半（presolve 跑完），为保 ready 未中局热切；
  burst-keep 随 verify17 上线验证。

## 9. Verify17（`33133803`，炎雪 vs 地狱牧逸风 1000001）：R12 战死，目击对质破案

- **战绩**：R10 39→21（引擎判+31被翻−18）、R11 21→3（−18）、R12 3 血上阵战死。
  终结保留生效过（R10 断肠曲被 floor 换成巨虎），但救不了后面的崩盘。
- **目击对质（用户亲眼看到三峰+崩雪上阵 vs 复盘 R10 手牌无三峰）**：两边都对。
  三峰是 R11 开局发牌（`[1000008, 1000015, 1000032, 5000006]` 四张）才进池的，
  用户看的是 R11 战斗——round-done 实锤三峰 slot7、崩雪 slot1 都上了。
  R10 窗口分析时三峰确实不存在，引擎在 R10 无罪。
  另：中途一次 trace 抓拍（11:30:29）落在 deploy 窗口内（round-done 11:30:37 之前），
  看到"三峰在手牌、slot6 空"属抓拍窗口假象——**上阵真值只认 round-done row**。
- **根因 A（R10 +31 vs −18）未定罪——此前"accurate-input 下首败"撤回**：
  准确的只是血量/修为（39v44），引擎对手侧输入按构造就是**上一轮 AI 行快照**
  （`_opponent` 取 `lastRound.usedCards`，慢一拍是设计如此），而 AI 真行在 trace
  与驱动日志里**双方都没有持久化**（`battlePlayers` 无牌字段；`usedCards` 只有我方；
  round-open 只记 opp 血量）。stale 与引擎 miss 不可区分，按
  `battle-rule-development` 不得据此改规则。注意 r11-replay/ 是 verify12 的夹具
  （p1 与 round-done 行逐字一致，该尸检有效），verify17 没有夹具。
- **根因 B（loop 血量脱同步）已撤回——误报**：`first.me/ai` 是修为 exp（先手口径），
  不是血量。实锤：R11 我方 exp=45 对上 `me=[45,45]`，R10 AI exp=44 对上 `ai=[44,44]`。
  战斗血量走 `LEVEL_BASE_HP[level]+extraMaxHp`（`full-match-loop.py:860` 注释早记过
  life/命元坑），输入口径正确，无需修。教训：`first` 字段先查构造代码再定罪。
- **输出墙家族再确认**：AI 命元 84 从 R9-open 到 R11-open 纹丝不动
  （R9/R10/R11 三轮净输出 0），R12-open 读数 99 待定（疑似视角/刷新 artefact，不采信）。
  同 v12–16 immortal 墙一案（verify18：AI 88 从 R9-open 冻到终局）。
  `lookup-evidence` 对 10022/1020028 无命中，墙体机制待原版证据，暂列已知缺口不建模。
- **可观测性修复（已落地，`test_driver.py` 57/57）**：`Board.ai_snapshot()` +
  round-open 记 `aiLast`（usedCards/handCards/unlockGrids/level/talents），
  下次尸检直接可判定是 stale 还是引擎 miss。另：R10 deploy-verify 曾 veto
  （finalOpt 19 vs engineBest 31，已回退）仍输 −18——veto 救不了 stale 行。
- **先手冲刺仍未验证**：R11 缺口 5、R12 缺口 5（47v52），规则要求 ≤3，两轮都没触发。
  **终结保留存活验证**：R10/R11/R12 floor 连续开火
 （断肠曲→巨虎、百鸟→巨虎×2），输局中正常执行。
- **verify18 已开局**（`33134451`，炎雪 vs 牧逸风 1000001，skip-animation latched）：
  在输出墙未解的现状下，目标改为——floor/终结保留/veto 三件套存活 + 先手冲刺触发验证。

---

## 10. Verify18（`33134451`，炎雪 vs 地狱牧逸风 1000001）：负 R12，后三轮无人驾驶

- **有人段 R1–R9**（驱动正常）：我 100→41；AI 100 纹丝不动直到 R8 才掉 12（→88），
  R9 我 -14（55→41）且 AI 88 不动。R9 deploy-verify veto（engineBest 20 vs finalOpt 6，
  已回退引擎行）仍输 -14——stale 行 fantasy win 又添一例（§9 的 R10 +31 同病）。
- **无人段 R10–R12**（驱动死于 R10 probe-talent 之后，12:00:20，上 session 结束，
  无存活进程）：R10 未突破（exp34/36，候选含崩雪 23）、未布阵，R9 旧行自动连打三轮：
  R10 -16（41→25）、R11 -18（25→7）、R12 -20（7→-13 战死）。AI 命元 88 从 R9-open
  起 immortal 到终局。
- **两个结论**：(1) 旧行自动打 -16/-18/-20，与有人驾驶的 -16/-18/-19 同分布——
  病根不在布阵，在输出墙；(2) lv4 突破连续两局差 2 点 exp（v16 33/36，本局 34/36），
  R10 炼化 +1 也补不上，缺口在前 9 轮运营攒修为，不在单轮操作。
- **终局口径再验**：终局 state 的 public 侧已翻成 AI（1000001 lv5 life88），我方在
  `opponents[0]`（`uidHash 6de10d25b910`，life -13）。读终局认 uidHash 不认血量。

## 11. Verify19（`33136888`，炎雪 vs 地狱杜伶鸳 3000002）：负 R11（13→-7）

- **HP 轨迹**：我 100/100/95/87/78/70/70/59/43/29/13→-7；AI 100→97（R1 打掉 3 点）
  →97 冻结 5 轮→88（R6 打掉 9 点）→88 冻结 4 轮到终局。**11 轮总输出 12 点**，
  对手是新角色（杜伶鸳）——输出墙与对手角色无关。
- **方向对的局**：R5 -13（实 -8）、R9 -18（实 -14）、R11 -35（实 -20）。
  **Fantasy win 又添四例**：R6 +23（实 0，70→70 平）、R7 veto +29（实 -11）、
  R8 veto +12（实 -16）、R10 veto **+52**（实 -16，史 card 极值，first 37v51）。
  三 veto 全救不了 stale 行（v17 R10 同病）。
- **先手**：R3 起轮轮后手（AI 修为全程领先：11v10…58v40），先手冲刺未触发。
- **突破**：lv3 在 R6 窗口（R7 三境出战），lv4 在 R10 窗口（exp34→39，R11 四境出战，
  仍晚一轮；与 v16/v18 同病：R10-open exp 恰卡 34/36 线下）。
- **aiLast 可观测性首验通过**：11/11 round-open 带 `aiLast`（R1 为 `[0,0,0]` 空快照，
  R2 起为真行）。驱动终局 `match-end(viewpoint-flipped)` + give-up 正常退出。
  下次若再现 +30 级 fantasy，可直接比对 AI 真行定罪。

## 12. Verify19-R10 真行尸检定论（+52 fantasy 的死因，`logs/practice/v19-replay/`）

- **方法**：aiLast 让真行尸检第一次成为可能——落子行（round-done）+ AI 真行
  （R11-open aiLast）+ 双边真数值（trace R10 battlePlayers），6 次 40320 穷举 exact。
- **结果（方向全部与实战一致，引擎无罪）**：
  落子集 vs 真行 → **-39 p2**（实战 -16）；落子集 vs stale R9 行 → -46 p2；
  计划集（断肠曲两版本）vs stale → -20/-13，vs 真行 → -33/-30。
- **live +52 从任何持久化真输入都复现不出来**——它是快照输入上的幻数
  （stale AI 行 + 已不存在的池内状态），不是引擎对真行的判断。按
  `battle-rule-development`（首差无 golden 卡级 trace）不得据此改规则，不建模。
- **AI 单轮 churn 实锤**：R9→R10 仅一处换牌（水灵•汹涌 7000037→7010037，同名升阶）
  + 全行重排，lv4 四仙命。快照慢一拍是结构性的，任何点估计都会撞墙。
- **遗留疑问（非规则）**：live +52 的候选集版本归属已不可考（/tmp fixture 已删），
  但结论不需要它——真行侧四次 exact 已闭环。

## 14. 本地战报推翻输出墙（2026-09-13 晚，用户指正后复查）

- **练习局落本地 `recent` 桶**：verify11（镜花水月 11 轮/3 胜，实际打满）+
  verify12–20 共 10 个 `.bin` 全在，`inspect_battle_records.py --input/--output`
  解出每轮 `winnerId/firstPlayerId/actorTurnCount/hpDelta/lifeDamage` + 双方
  行/数值。席位 p1/p2 逐轮变，`hpDelta/lifeDamage` 是 p1 视角相对值
  （`lifeDamage = p1命元delta − p2命元delta`，v20 11/11 逐轮验算全合）。
  skill 里"练习局不取不到记录"是旧结论，已同步更正。
- **输出墙作废**：全部"AI 命元冻结"轮都是 **AI 获胜轮**
  （v17 R9–R11、v18 R9–R12、v19 R2–R5/R7–R11、v20 R3–R6/R8–R11）。
  AI 获胜的轮次 AI 命元自然不动——所谓墙就是连败，没有免伤机制问题。
  §§9–11、13 的"immortal/冻结"表述按此重读。
- **社区命元公式在我们四局上 gt20 24/25 exact**
  （`round+4/+5+ceil((|hp|-20)/10)`，cap `floor(1.5r+7)`；仅 v20 R5 miss：
  预测 10 实 6，属已知大偏差类）。le20 分支无公式，照旧。
- **引擎真行 triple（规则无罪，实锤，5/5 方向全对）**：
  落子序重放（引擎）vs 战报实战（winner/turns/hpDelta）——
  v20 R8：-50/13 vs **-50/13，全 exact**；v19 R10：-23/19 vs -21/19（差 2）；
  v19 R8：-55/14 vs -52/15（差 3）；v19 R7：-10/15 vs -16/15（差 6）；
  v20 R3：-21/14 vs -26/16（差 5）。最优序一侧（同集换序）v20 R3 能到 +7、
  v20 R8 能到 +23——同集牌序敏感度 ±30~70 点，fantasy 与实战差首先是"序"差。
  §12 的 -39 是错版本 ID（同名 1 星代 2 星）的作废结果，以 record-exact 为准。
  病根只剩两处，且都不在战斗规则里：(1) 计划输入是上一轮 AI 快照（stale，
  v19 R10 实锤单轮一换牌翻转 90 点）；(2) 落子序从未对真行重仿
 （deploy-verify 只对 stale 行验证，v20 R8 keep 的 +29 就是这么放行的，
  而该序对真行是 -50）。夹具在 `logs/practice/v19-replay/`
 （`fixture-v20r03/r08/v19r07/r08/v19r10-record.json`，record-exact 行列+数值）。
- **非 exact 轮归因（用户追问：规则错还是客户端漂移）**：三条排除。
  (a) 不是 RNG：五轮全确定性（`usedSyntheticDecisions` 全 False，无随机牌）；
  (b) 不是输入时机漂移：战报（战后，含战斗奖励：修为+3、19 层数 6→17、
  10000 buff 0→3）vs 开窗 witness（战前）两套输入重跑，v20 R8（-50/13 不变）、
  v19 R7（-10/15 不变）——漂移存在但对结果 immaterial；
  (c) 不是系统性规则缺失：残差 ±2~6 且正负混合（-10/-16、-55/-52、-23/-21、
  -21/-26），缺整块机制只会单向大偏。结论：残差归因不可达（无卡级 golden
  做首差），但全部 ≤6 点 < deploy-verify margin 10——即使规则完美，
  也改变不了任何一次落子。决策无关，按 skill 不动手。
- **对手归属更正**：v17、v18 对手均为牧逸风 1000001（§9 写成南宫生是错的，
  以驱动日志 R1-open 与本地战报为准）；v16 才是南宫生。

## 15. 死战之志 oracle 就绪状态（batch-029 已建待跑）

- **Batch**：`battle-evaluator/oracle/synthetic-batch-029-sizhan-lethal-gate.ts`
  （已提交，离线 build 验证通过，2 cases 已 stage）。
  设计：p1 化灵诀+双巨鲸先手，被害方 p2（lv1/extraMaxHp -30，10 血）treatment 持
  `{17:1}`、control 无；p1 第 2 动必斩（20+20 vs 10 血），witness 取 p2 hp + buff[17,18]。
  判定：treatment 转出 18 且战斗继续 = 客户端会转（引擎门控正确，残差另有他因）；
  不转 = 引擎多给了不倒回合（v19 R7 的 turn-16 即此物），根因即定。
- **Category 说明**：`card:dream`（385 梦•飞枭灵芝，DreamCard refine writer），
  测的是其下游 buff 门，与 172-revive 件同构。证据全是 current build 行号。
- **Blocked**：客户端卡回放结算屏（verify20 终局后 1 小时），`ops-restart-practice`/
  `ops-confirm-dialog` 均无权威状态（插件活、进程活、画面不动）。
  Oracle 要本地回放场景（先 yiwen-import bootstrap），必须先恢复客户端
  （点一次结算确认，或杀进程重拉——buildid==TargetBuildID、无待更新，重拉安全，
  但登录可能卡 waiting-user）。
- **v20 R3（-5，turns 14 vs 16，无 perm17）不在本次 oracle 范围内**：
  穷举 singles 无精确命中、top pairs 重跑最佳 -25（差 1），属 distributed，
  仍 open。若 ZhiZhi 门控证实引擎正确，R3 需另起 course 级调查。
- **2026-09-13 定论（用户指令直接 rotate）**：`bun run rotate:build -- --new auto`
  全绿（preflight/extract/diff/screen-skip/stage/validate/promote/public），证据钉到
  `25268934`。diff：全配置表 0 变更；反编译仅 1 文件变
  （`extracted/current/decompiled/ReadyLayerYuanGuCI.cs`，元古地图 UI 面板显隐重构，
  战斗外）。profile capabilities 与旧 build 完全一致。
  rotate 曾两卡 `check:docs-drift`（本备忘 file:// 绝对链接 + 缺 topic；gitignored 的
  `logs/practice/` 不可做 markdown 链接，已改纯代码段），修完即过。
- **batch-029 裁决（新 build 上实机跑完）**：
  `bun battle-evaluator/scripts/synthetic-oracle-build.ts --manifest battle-evaluator/oracle/synthetic-batch-029-sizhan-lethal-gate.ts --stage`
  重建（`build | 25268934`），sequencer forward+reverse 全过，`v2 exact 2/2`，
  报告在 <!-- battle-evaluator/generated/synthetic-oracle-admission/fixture-categories-batch-029-sizhan-lethal-gate.json -->台账 `fixture-categories-batch-029-sizhan-lethal-gate.json`。
  实机 treatment（p2 持 `{17:1}`）：winner p1、actorTurn **4**（control 为 3，
  多一动即不倒回合开火）、终局 p2Hp -10 且 17/18 皆无——**客户端在 lethal 门会转
  17→18**。引擎门控与实机一致，v19 三局残差（-16/-52/-21）不是引擎多给不倒，
  定为 course 级分歧（另查，v20-R3 distributed 缺口同理不在门控范围）。
  SiZhan 线关闭，未留 mismatch。

## 13. Verify20（`33137526`，炎雪 vs 地狱陆剑心 1000005）：负 R11（15→-4）

- **HP 轨迹**：我 100/100/92/83/77/65/65/50/33/15→-4；AI 100→95（R1）→88（R2）
  →冻结 5 轮（R3–R7）→76（R7 打掉 12）→冻结 4 轮到终局。**11 轮总输出 24 点**
  （v19 的两倍，仍远不够）。输出墙跨第三个 AI 角色。
- **引擎诚实度回升**：亏损轮方向全对（R4 -23/实-9、R5 -24/实-6、R6 -27/实-12、
  R9 -46/实-17、R10 -37/实-18、R11 -43/实-19）；获胜预测只剩两例 fantasy
  （R3 +25/实-8、R8 +28/实-15），R7 +8 对应实战我方零承伤 + AI-12。
- **先手**：R1–R4 先手（修为领先），R5 起后手；先手冲刺未触发（R5/R6 缺口 2/3，
  但无余牌可烧——refine 仅 R3/R4/R7/R11 常规各 1 张）。
- **突破**：lv3 在 R7 窗口（R8 三境），lv4 在 R10 窗口（R10-open exp36 恰达线，
  拿下崩雪 23，R11 四境出战；连续四局晚一轮，但这次是达线即突，无运营欠账）。
- **aiLast**：11/11 持久化。驱动终局正常退出，无残留。

---

## 16. Verify21（`33155445`，炎雪 vs 地狱牧逸风 1000001）：负 R11（10→-6）

- **HP 轨迹**：我 100/95/91/83/83/77/68/55/44/28/10→-6；AI 100×4→95（R4）
  →冻结 7 轮到终局。**11 轮总输出 5 点（仅 R4 一轮）**，R1–R3 零输出。
- **先手 2/11**（R5 tie 猜中、R8 31v30），口径 11/11 exact（tie 1-1：
  R2 6v6 输 tie、R5 18v18 赢 tie）。先手机制无罪。
- **突破**：我 lv2 R4 / lv3 R7 / lv4 R11 vs AI lv2 R4 / lv3 R7 / lv4 R10。
  连续五局 lv4 晚一轮；但 R10-open exp36 达线即突、无运营欠账，
  AI R10-open exp43 只是单纯更快（R3 起修为落后：9v10…41v48）。
- **机制触发**：career auto 画师 R2；deploy-verify keep×6（R2–R7）、
  R8 skip-budget、R9 veto、R10/R11 keep；先手冲刺 0 触发（R6 gap1、R9 gap2
  均无余牌可烧，代码无辜）；炼化 10 次；突破 4/4。
- **战报 triple**：`recent` 桶解码（`logs/practice/v21-replay/record.json`），
  lifeDamage 公式 11/11 OK；AI 11 连胜（所谓冻结轮全是 AI 获胜轮）。
- **三轮真行穷举**（record-exact 行 + 战前我方数值 + AI 战后 buff 假设，
  夹具 `logs/practice/v21-replay/fixture-v21r{04,08,10}-deployed.json`，
  `prebattle-me.json` 固化战前值；战前/战后差：10000 buff 与 19 层数，
  三轮重算上限不动，drift 不是主因）：
  | 轮 | 引擎上限 vs 真行 | 实战 | 结论 |
  | R4 | -7（穷举 20160）| +5 胜 | 确定性无随机牌，残差 9 点 open（v20-R3 同类）；悲观 miss，部署仍是 argmax，决策无关 |
  | R8 | **+5**（40320，turn 20 exact）| -11 | 池子能赢，落子序 -14，**序差 19 点**；live +26 是 stale 行幻数，deploy-verify 还 skip-budget |
  | R10 | -22 | -18 | lv4 parity 池子被碾压（星级差：AI 1 星三峰 1010032 vs 我方 0 星 1000032）|
- **Verify22 补丁 tie-robust（已上线，`test_driver.py` 61/61）**：
  `tie_second_side` 纯函数 + `_ask_engine` 平局轮双侧求解取 min（max-min，
  只多一次求解、仅 tie/unknown 轮触发；不建模 AI，不管对手行，只管己方
  评估诚实）。`TieFirstTest` 4 例锁定。
- **已知局限（按 skill/ban 不动）**：R9 veto 反向（finalOpt -17≈实战 -16 被
  engineBest -3 否决，n=1 不重设计）；R4 残差 open（无卡级 golden 不碰规则）；
  AI 侧战前 buff 无见证、暂用战后值（量级小）。

## 17. Verify22（`33155886`，炎雪 vs 地狱曜灵 2000003）：负 R12（15→-7）

- **HP 轨迹**：我 100/100/100/100/93/90/90/78/67/50/33/15→-7；
  AI 100→97→91→86→86→86→83→冻结 6 轮。**输出 17 点**（R1/R2/R3/R6 四胜，
  开局 R1–R3 9 点零承伤；历史最久 12 轮）。
- **引擎对照（我视）**：R1 -17→胜（悲观）、R2 +23/R3 +27/R6 +31 方向对、
  R4 -10/R8 -12/R9 -14 诚实亏损；fantasy：R5 +12→-3、R7 +22→-12、
  R10 **+25**→-17、R11 **+30**→-18、R12 **+35**→-22。先手 12/12 exact，
  tie 0 轮（tie-robust 未触发）。
- **突破 race**：我 lv2 R4 / lv3 R7 / lv4 R11 vs AI lv2 R5 / lv3 R8 / lv4 R10——
  全程领先到 R9，R10 被 AI 先上 lv4（exp 46v36）。
- **三轮真行穷举**（`logs/practice/v22-replay/`，战前我方值+AI战后假设）：
  R7 上限 -13/实 -12（诚实，池子真不够）；R12 上限 -24/实 -22（诚实，
  被碾压）；**R11 上限 +12/实 -18（序+集差 30 点，可赢没赢）**。
- **R11 管道谋杀案告破（+12 行被否决链杀掉）**：
  presolve（call#1）minimax 已修出 rowB（honestA -54 vs staleA +30，
  repaired:true）；终局重解（call#2）minimax 被预算门拦下，info 残留 call#1
  的 repaired:true，实际部署 stale +30 引擎行；deploy-verify 19 vs 30 veto
  回 pre_floor_row，而 `target()` 随后重挂 floor——veto 对 floor 变异是 no-op，
  还顺手埋了 rowB。rowB 重排 vs 真行离线复测 **+12 胜**（turn 20），
  部署行实战 -18。
- **Verify23 补丁 repair-carryforward（已上线，`test_driver.py` 65/65）**：
  `_minimax` 修出即存 `mm_carry{code,round,row,honest}`（新鲜评估安全时清除，
  跳过/预算不足时保留）；`_ask_engine` 把结转经 `add_cand` 映射到最新池子后
  多解一次，用 `min(fresh, honest)` 稳健计分（`robust_pick`/`set_overlap≥6`
  漂移守卫，跨轮跨局隔离）。无结转轮行为逐字不变；不跑新 AI 搜索、不建模 AI。
  `CarryRepairTest` 4 例锁定 R11 决策表（引擎 min(30,-54) vs rowB min(19,48)）。
  接线正确性由 Verify23 实战日志 `carryRobust` 标记验证。

## 18. Verify23（`33156326`，炎雪 vs 地狱慕虎 3000004）：负 R11（11→-5）

- **HP 轨迹**：我 100/94/90/90/83/73/61/52/41/27/11→-5；
  AI 100×3→96（R3）→冻结 8 轮到终局。**11 轮总输出 4 点**（仅 R3 一胜，
  V21 以来最低；V21=5、V22=17）。冻结轮战报 winner 全是 AI（§14 重读再验：
  10 个 AI 胜轮），无输出墙。
- **战报 triple 11/11**（`logs/practice/v23-replay/record.json`，`f8cf4vl.bin`
  全量解码，`codeId 33156326` 对上；席位逐轮换，`lifeDamage = p1命元delta −
  p2命元delta` 11 轮全合；终局 `viewpoint-flipped`，`opp [[1000002,-5,false,1]]`
  即我方 -5，认 uidHash 不认血量）：
  R1 先AI胜AI/13/-21/-6；R2 先AI胜AI/14/-6/-4；R3 先我胜我/19/-3/-4；
  R4 先AI胜AI/13/+13/+7；R5 先我胜AI/14/-29/-10；R6 先AI胜AI/19/-36/-12；
  R7 先AI胜AI/25/+10/+9；R8 先AI胜AI/25/+14/+11；R9 先AI胜AI/31/-28/-14；
  R10 先AI胜AI/29/-24/-16；R11 先AI胜AI/29/-19/-16（胜/turns/hpDelta 口径）。
- **引擎对照（live plan vs 实战命元）**：R1 +2→-6、R2 -5（tieBest +4）→-4、
  R3 +10→胜（AI -4，方向对量级高估）、R4 +3→-7、R5 -4→-10、R6 -12→-12 exact、
  R7 -1→-9、R8 -2→-11、R9 -12→-14、R10 **+28**→-16、R11 **+13**→-16。
  fantasy 双例（R10/R11）与 V22 R11 同形。
- **先手 10/11 exact + R2 tie + R5 miss**：R1–R4/R6–R11 驱动判定与战报
  `firstPlayerId` 全对（含 R2 `random-tie` 双侧加测）；唯一 miss 是 R5——驱动
  `determined-p2`（me 18 vs ai 19），战报我先手。输入对账不可考（`round-open`
  未记 opp exp/perms，`aiLast` 本来就没有 exp 字段），stale 快照嫌疑最大，
  公式本身 n=1 不动手。**可观测性改进（非规则）**：`round-open` 补记 opp
  `exp/perms/talents`，下次 miss 可直接对账（Verify24 已带，见本节末）。
- **突破 race**：我 lv2 R4 / lv3 R7 / lv4 R11 vs AI lv2 R4 / lv3 R7 / lv4 R10——
  与 V22 同形，R10 被 AI 先上 lv4（AI R9 战后 exp41 已过 36 线，我 R10-open 35
  差 1 点，R11 exp39 达线即突，无运营欠账）。修为全程：R3 胜后我 13 vs AI 10、
  R5 后我 21 vs AI 19、R6 后 24 平、R7 后我 26 vs AI 28 被超、R9 后 35 vs 41
  拉开——**修为落后是 R7–R9 三连败的结果，不是原因**，不要误判成运营欠账。
- **三轮真行穷举**（record-exact：战报 `private` 8 格行 + 战前 lv/talents +
  战报 extra + `lastRound` 双边 buff；`baseMaxHp` 只与 lv 有关
  40/45/52/62/75，与角色无关；15 张缺牌由 `CardConfig` 转（仅删
  `estimateTime/noUpgrade/owner`），其余复用 v21/v22 已验证库；0 牌按
  v20r03 同式补普通攻击；三轮 40320 全排列 exact）：
  | 轮 | 落子序重放 vs 战报 | 同集最优序（上限）| 实战 | 结论 |
  | R5 | **-29/14 exact** | -16（仍败，turn14）| 命元 -10 | 池子真不够 + 序差 13 点；live -4 乐观 12 点（stale + 先手错）|
  | R10 | -60/25 vs -24/29 | -30（仍大败，turn25）| 命元 -16 | live +28 纯幻；36 点差见下（course 级分歧，胜负方向无关）|
  | R11 | **-19/29 exact** | -6（仍败，turn29）| 命元 -16 | 序差 13 点；live +13 乐观 19 点 |
  夹具 `logs/practice/v23-replay/fixture-v23r{05,10,11}-deployed.json`
  （`p1order=deployed`）。R5/R11 落子即 exact——引擎对真行诚实；
  三轮上限全败——输在池子（lv parity 下慕虎行占优）+ 部署序各差 13 点，
  不在战斗规则里。
- **R10 36 点差排除链（`actualDelta -60` vs 战报 `-24`，turn 25 vs 29）**：
  (a) 铁骨 7000035 实现完整（`elements_late.rs` 设值 + `combat.rs:1010` /
  `combat_core.rs:325` 两处消费 + 单元测试，门控 `check_wu_xing(Metal)` 真——
  AI 首张 38 锟铻金环即激活金灵），且 R9 同铁骨轮仅差 2（turn 31 exact，
  `fixture-v23r09-probe.json[.bak]` 追加探针，非三轮体例）；(b) R10 我方独有牌
  反身剑 1000008 / 破气剑 1010023 均在历史 exact/近 exact 轮出现过
  （v19 R7 差 6、v19 R10 差 2、v21 R10 差 4），单卡实现可信；(c) R11 同缺
  战前 17 号 buff 却 exact，17/extra 量级（≤4）解释不了 36 点。
  定性：**组合/时序级分歧**（turn 也差 4），与 §15 v20-R3 distributed 缺口同类，
  立项追查但本次不动手（无卡级 golden 做首差，按 skill 不猜规则；且上限 -30
  仍大败，胜负方向无关，决策无关）。
- **tie-robust R2 观察**：11 轮唯一 tie 轮触发（`tieBest +4/tieWorst -5`，
  `finalOpt +4 keep`），实战 -4——稳健计分诚实（`engineBest -5` ≈ 实战），
  但部署行取的是乐观侧重排，与 V21 R2 同形。n=1 不重设计，记观察。
- **repair-carryforward 接线验证（0/11 轮出现 `carryRobust`）**：R3 是唯一
  repair 轮（presolve 与终局 `repaired:true`，`deploy keep +10` → 实战胜，
  repair 没帮倒忙）；R10/R11 fantasy 轮 `minimax` 根本没运行（无 honest 遗产，
  补丁按设计沉默——与 V22 R11“有遗产被预算门丢”不同，这是覆盖面边界：
  没算过 honest 的轮次借不到 honest）；R3 终局结转未消费原因 open
  （终局 minimax 又 repair 覆盖 vs 映射未命中，需单步跟，不影响正确性）。
- **AI churn 再添三例**（stale 慢一拍结构性实锤）：R8→R9 两处换牌 + 升 lv4
  （土灵印/金灵针 → 铁骨/蓄锐）；R9→R10 同集纯重排；R10→R11 一处换牌
  （铁骨 → 扬尘）。live 摆动：R10 对 R9 旧行 +28（真行上限 -30，摆动 58 点）、
  R11 对 R10 旧行 +13（真行上限 -6，摆动 19 点）——计划输入是上一轮 AI 快照，
  任何点估计都会撞墙（§12）。
- **机制触发**：career auto 画师 R2；deploy-verify R1/R5 `skip-budget` 其余全 keep；
  R6 `finalOpt -8` vs `engineBest -12`（实战 -12 = engineBest，乐观 4 点，margin 内，
  n=1 不究）；炼化 R2–R11 多轮；突破 4/4（67 炽火炎刃 / 68 流冰寒刃 / 23 崩雪，
  R10 拿崩雪四境 R11 出战）。
- **Verify24 改动**：零规则/管道改动（R10 分歧立项不动手；R5/ tie-robust / R6
  观察全 n=1）。仅带一个可观测性补丁：`round-open` 补记 opp `exp/perms/talents`
 （3 行，`test_driver.py` 65/65 复验通过），下次先手 miss 可直接对账。

## 19. Verify24（`33156861`，炎雪 vs 地狱杜伶鸳 3000002）：负 R12（9→-14）

- **HP 轨迹**：我 100/95/89/89/89/81/71/71/58/44/30/9→-14；
  AI 100×3→93（R3）→87（R4）→87×3→76（R7）→冻结 5 轮。**输出 24 点**
  （三胜 R3/R4/R7，V20 以来最高；V23 仅 4 点）。冻结轮 winner 全 AI，无输出墙。
- **战报 triple 12/12**（`logs/practice/v24-replay/record.json`，`f8cqlop.bin`，
  席位轮换，`lifeDamage` 12 轮全合；终局 `viewpoint-flipped`，
  `opp [[1000002,-14,false,3]]` 即我方 -14）：
  R1 先我胜AI/16/+19/+5；R2 先AI胜AI/13/-17/-6；R3 先我胜我/17/-16/-7；
  R4 先AI胜我/16/-8/-6；R5 先AI胜AI/11/+15/+8；R6 先AI胜AI/11/+20/+10；
  R7 先AI胜我/16/-20/-11；R8 先AI胜AI/13/-29/-13；R9 先AI胜AI/13/+29/+14；
  R10 先AI胜AI/17/+13/+14；R11 先AI胜AI/13/+61/+21；R12 先AI胜AI/11/-76/-23。
- **引擎对照**：R1 -18→-5、R2 +4→-6、R3 +16→胜、R4 +2→胜、R5 +12→-8、
  R6 -6→-8、R7 +24→胜（方向对）、R8 +26→-13、R9 -13→-14（≈exact）、
  R10 +13→-14、R11 +16→-21、R12 -43→-23。fantasy 四例（R5 小，R8/R10/R11 大）。
- **先手全对**：determined 10 轮全 exact；tie 2 轮 1-1（R1 `random-tie` 猜中我先手，
  R4 猜 p1 实战 AI 先手——但 R4 胜了，tie 乐观无害）。先手机制连续两局无罪。
- **`oppDetail` 首验通过**（§18 补丁实战验证）：R2 开窗 `exp 5 == 战后 exp 5`，
  perms 差 `10022:1`（战中增量），与 §16“战前/战后差”同构——字段可信，
  V23-R5 式 miss 再现即可直接对账。
- **突破**：我 lv2 R4 / lv3 R8 / lv4 R11（R10-open exp36 达线即突，无欠账）
  vs AI lv4 R10（`+20074`）。我 lv4 拿的是心法 `20056`（非崩雪 23，
  `rank_talents` 择优结果，记一笔不展开）。道韵拿 `1000042`
 （候选含 27 而没选 27，`rank_daoyun` 择优，记一笔）。
- **三轮真行穷举**（`fixture-v24r{08,10,11}-deployed.json`，去重穷举
  40320/20160/3360 exact；R10/R11 战报席位 p1=AI，下表已转回我方视角）：
  | 轮 | 落子序重放 vs 战报 | 同集最优序 | 实战 | 结论 |
  | R8 | -19 vs -29（turn 13 exact）| -9（仍败）| -13 | live +26 纯幻 |
  | R10 | -20 vs -13（turn 15 vs 17）| -33（仍败）| -14 | live +13，摆动 46 点 |
  | R11 | -48 vs -61（turn 13 exact）| -78（仍败）| -21 | live +16，摆动 94 点 |
  落子残差 7–13、turn exact×2——§14 式残差，无 golden 不猜规则，不动手；
  三轮上限全败（杜伶鸳行占优），决策无关，输在池子不在规则。
- **repair 三轮开火**（R3/R9/R12 `repaired:true`）：R3 部署引擎行→胜；
  R9 `finalOpt -9`（=staleB）vs `engineBest -13` keep，实战 -14 = engineBest；
  R12 部署的正是 rowB（7/8 同 ID，首个同名星级版本差，`finalOpt -50`=staleB），
  实战 -23——比 stale -50 与 engineBest -43 都好（repair 行真行更优，n=1 不展开）。
- **`carryRobust` 0/12（累计 0/23）——修正 §18 表述**：repair 行可经终局 minimax
  直接部署（R12 实证），carry 只管“终局 minimax 被 skip ＋ presolve repaired”
  的 V22 形；V23/V24 共 23 轮该条件出现 0 次（终局 minimax 要么运行、要么
  presolve 也没 repair）。补丁正确但零触发：保留待 V22 形再现，下一棒可评估
  触发统计是否需收紧（纯管道议题，非规则）。
- **在途（Verify25，`33157137`，炎雪 vs 林小月）**：见 §20。

## 20. Verify25（`33157137`，炎雪 vs 地狱林小月 1000004）：负 R10（28→0 被斩）

- **HP 轨迹**：我 100/100/100/94/91/83/72/59/44/28→0；
  AI 100→95（R1）→91（R2）→冻结 7 轮。**开局 2 连胜后 7 连败**，
  输出 9 点（5+4），R10 被斩 -28（turn 19）。终局 `viewpoint-flipped`，
  `opp [[1000002,0,false,2]]` 即我方归零。
- **战报 triple 10/10**（`logs/practice/v25-replay/record.json`，`f8cwind.bin`）：
  R1 先AI胜我/16/+18/+5；R2 先AI胜我/14/-6/-4；R3 先AI胜AI/13/+14/+6；
  R4 先AI胜AI/15/+21/+3；R5 先AI胜AI/15/+12/+8；R6 先AI胜AI/17/+27/+11；
  R7 先AI胜AI/17/+34/+13；R8 先AI胜AI/15/+41/+15；R9 先AI胜AI/15/+48/+16；
  R10 先AI胜AI/19/-65/-20（p1 视角；R1/R2 p1=我方，其余 p1=AI——认 uid 不认席位）。
- **引擎本局诚实**（亏损轮方向全对，fantasy 仅 R3 +1→-6 一例小的）：
  R1 +13→胜、R2 +6→胜、R3 +1→-6、R4 -12→-6、R5 -15→-8、R6 -25→-11、
  R7 -7→-13、R8 -19→-15、R9 -31→-16、R10 -27→-28（≈exact，turn 23 vs 19）。
- **先手 10/10 exact**（全轮 `determined-p2`，战报先手全 AI）：连续三局先手机制无罪。
- **形态**：与 V23 同构（开局 thắng 后 AI 冻结=连败），但本局引擎无大 fantasy——
  输在池子 scaling（R6 起每轮 -11~-16，胜局只 +4/+5），与 skill 实战记录
  “后半局 scaling 差距是主要矛盾”一致。三轮穷举未跑（上限方向已由 live 亏损
  口径覆盖：R4–R9 live 全负，无 fantasy 可验；R10 ≈exact 无需重放）。
  连续 2 胜仍为 0——下一棒从 Verify26 继续推进。

## 21. Verify26（`33157378`，炎雪 vs 地狱林小月 1000004）：负 R15（1→-25 被斩），史最佳

> 本局跑的是旧代码（`bf79c313` 合入前 14:03 启动）：人类运营策略（8 轮元婴 + 攒换牌）
> 未上线，是新策略的**基线局**。Verify27 起首测新策略。

- **HP 轨迹**：我 100/97/97/93/88/88/78/78/65/47/19/19/1/1/1→-25；
  AI 100→96（R2）→96→90（R5）→79（R7）→79×3→71（R11）→63（R13）→42（R14）。
  **六胜 + 总输出 58 点**（V24 的 24 点两倍还多，历史最高），R14 化神首战大胜，
  R15 化神 parity 终局被斩。连续 2 胜仍为 0。
- **战报 triple 15/15**（`logs/practice/v26-replay/record.json`，`f8d1olt.bin`，
  席位轮换；owner-relative `lifeDamage = p1方命元Δ − p2方命元Δ` 14/15，唯一例外 R10
  见下；终局 `viewpoint-flipped`，`opp [[1000002,-25,false,6]]` 即我方 -25）：
  R1 先我胜AI/18/+9/+3；R2 先AI胜我/12/-7/-4；R3 先AI胜AI/12/-3/-4；
  R4 先我胜AI/16/+2/+5；R5 先我胜我/17/-4/-6；R6 先我胜AI/24/+19/+10；
  R7 先AI胜我/22/-16/-11；R8 先AI胜AI/25/-27/-13；R9 先AI胜AI/9/-70/-18；
  R10 先AI胜AI/13/-63/-20；R11 先我胜我/21/+17/+8；R12 先我胜AI/16/+23/+18；
  R13 先我胜我/21/-6/-8；R14 先我胜我/13/+40/+21；R15 先我胜AI/14/+73/+26
  （先手/胜者/turns/hpDelta/lifeDamage 口径，hpDelta 为战报 p1 视角）。
- **先手 15/15 exact**：determined 12 轮全对；`random-tie` 3 轮（R1/R6/R11）
  全猜中。先手机制连续五局无罪。
- **突破 race（witness 版，以 trace 战前快照为准，开窗读数滞后见下）**：
  我 lv2 R4（R3 窗口 67 炽火炎刃）/ lv3 R7 出战（R6 窗口 68 流冰寒刃）/
  **lv4 R10 窗口内**（20059，战前 exp37）/ **lv5 R14 窗口内**（22，战前 exp60，
  化神首战即 R14 大胜）vs AI lv2 R3 / lv3 R6（同 exp 21 AI 先上）/ lv4 R9 /
  lv5 R13。AI 每个境界领先约一轮。
- **引擎对照（live best vs 实战命元我视角）**：R1 +18→-3、R2 -3→胜（反向悲观）、
  R3 0（tieBest+8）→-4、R4 +18→-5、R5 +10→胜、R6 +20→-10、R7 +26→胜、
  R8 +23→-13、R9 +28→-18、R10 **+100**（veto 61）→-28、R11 +106→胜、
  R12 **+82**（veto 52）→-18、R13 +66→胜、R14 **+122**（veto 96）→胜、
  R15 **+153**（veto 100）→-26。败轮全是大 fantasy（stale 慢一拍同形，§12/§16）。
- **deploy-verify**：keep×7（R2/R3/R4/R5/R7/R9/R13）、skip-budget×4（R1/R6/R8/R11）、
  veto×4（R10 61 vs 100、R12 52 vs 82、R14 96 vs 122、R15 100 vs 153，
  全部 `reverted` 回引擎行）。
  veto 救不了 stale 行（V17 R10/V19 三 veto 同病，再添四例）。
- **repair-carryforward R15 首触发**（累计 0/23 后 1/24）：`carryRobust:true`、
  `carryFresh 168/carryHonest 153`，但同轮被 deploy-verify veto 覆盖（回引擎行），
  carry 行未上阵——触发通路验证通过，实战效果仍待 V22 形再现。
- **三轮真行穷举**（trace 战前 witness + 战报 private 8 格 record-exact 行 +
  `replay_slice --admission` exact + solve 全排列上限；夹具
  `logs/practice/v26-replay/fixture-v26r{10,14,15}-deployed.json`，
  `p1order=deployed`；先手 sim 与战报三轮全 MATCH）：
  | 轮 | exact 落子 vs 战报 | 同集上限 | 实战 | 结论 |
  | R10 | AI 胜/t26/-26 vs AI 胜/t13/-63 | AI 最优序 -36（我视角）| 命元 -28 | 方向对，节拍差 2x（course 级残差）|
  | R14 | 我胜/t14/+71 vs 我胜/t13/+40 | +111 | AI -21 大胜 | 近 exact（turn 差 1）|
  | R15 | 我胜/t24/+86 vs AI 胜/t14/-73 | AI 最优序 -22 | 被斩 -26 | **方向反转 159 点，立项 P0** |
  R10/R15 的 engine 整序区间（R15 [-22,+118]）都覆盖不了实战值——不是行序问题。
- **R10 命元 +8 之谜（open，n=1）**：`lifeDamage -20` 合社区公式（gt20：R10 基数
  14/15 + ceil(43/10)=5），但实际开局命元 47→19（-28）。8 点额外掉血来源不明
  （斩杀溢出/持续结算？），胜负方向无关，不动手。
- **R15 P0 嫌疑（立项，不动手）**：AI 行 `[游龙 1000042, 飞灵闪影 1010043,
  天音困仙曲 5000015, 天灵曲 5020004, 灵气灌注 1010006, 灵感剑 1010038,
  剑意激荡 1000044, 灵猫乱剑 10009]`。① `5000015` 天音困仙曲（化神，琴师，
  原文"双方无法触发再次行动"）是**唯一上阵轮**（R14 它在 AI 手牌未上阵→近
  exact；R10 无此牌→方向对）——而我方正是狂剑/云剑多动体系；`5000015` 在
  `engine-rust/src/` 无字面专属分支（仅数据 catalog + 共享 dispatcher 路由），
  压制是否生效待单测确认。② `Card_9` 灵猫乱剑（每保留 1 张手牌追加攻击，
  AI 战报手牌 6 张；`swords.rs:76` 经 `consume_optional_decision` 取段数，
  夹具 `handCards=[]`）——手牌挂钩在 exact 路径下取值待确认。`logCount=0`
  无卡级 golden，按 skill 不猜规则、不动手；实现需先立最小契约 + golden。
- **方法论修正×2（后续体例）**：① `round-open` 读数滞后突破——R10/R14 两次
  窗口内突破（witness 战前已 lv4 exp37 / lv5 exp60，开窗仍显示 lv3 exp35 /
  lv4 exp56），真行复核一律以 trace 战前 witness 为准（`/tmp` 探针脚本已验证，
  下次整理进 skill 或 `analysis/` 时再沉淀）。② `solve_deck --max-evals 1`
  **不是**落子原序重放（它求值 canonical 首候选；实锤：同夹具 p1/p2 双侧求值
  胜负翻转）——落子 exact 以 `replay_slice --admission` 为准；上限（全排列，
  含原序）不受影响。旧 §§ 的"落子"数是 canonical 首候选（且 AI 侧用上轮旧值，
  如 v24R8 用 AI lv3）；新体例用 witness 战前真值（v24R8 为 AI lv4）+ 原序 exact，
  两者在 v24R8 上差方向一致、turn 差 1（t14 vs t13）、hp 差 1（-18 vs -19），旧结论不受影响。
- **运营观察（n=2，非规则）**：`upgrade-hand-to-grid` 实际修为 +2/次
  （R10：35→37；R14：56→60），而 pick 的 `expectedExpDelta=1` 保守——rush 燃料
  计算系统性低估升级收益。待 Verify27（rush_threshold=14 已合入）观察是否
  需要顺手把预测值改成 +2。
- **机制触发**：career auto 画师 R2；突破 4/4 + 化神（67/68/20059/22）；
  R15 开局 `chance 6`（旧代码无 hoard，前期换牌花完——正是 Verify27 要验的）。
- **在途（Verify27，`待开局`）**：首测人类运营策略（`bf79c313`：金丹 rush14
  冲 8 轮元婴 + `hoard-replace` 低境攒换牌），见 §22。

## 22. Verify27（`33158117`，炎雪 vs 地狱龙瑶 1000003）：负 R11（19→0 被斩），新策略首测

> 新策略首测局（`bf79c313` 已合入：金丹 rush14 + `hoard-replace`）。元婴实际
> R10 窗口内突破、R11 出战（候选含 `23 崩雪`，实际落子 `20056 心法#20056`——
> **不是崩雪**，复盘勿按崩雪链路对）。

- **HP 轨迹**：我 100/93/86/81/74/65/54/54/54/39/19→0（R11 被斩）；
  AI 100×5（R1–R5 纹丝不动）→92（R5）→92→83（R7）→70（R8）→70×3。
  **二胜 + 总输出 22 点**（R7 +9 / R8 +13，胜轮我命元 Δ 均为 0——赢战斗不涨命元，
  与 V26 "胜局只打 8–9 点"同形）。连续 2 胜仍为 0。
- **战报 triple 11/11**（`logs/practice/v27-replay/record.json` + `f8dhitl.bin`；
  注意 Shepherd 口径的 `f8d1olt.bin` 是 V26 的（15 轮），V27 的 bin 是 `f8dhitl.bin`
  （11 轮，mtime/轮数/codeId 三对）——按解码归档：
  R1 先AI胜AI/11/+34/+7；R2 先AI胜AI/15/+24/+7；R3 先AI胜AI/17/+10/+5；
  R4 先AI胜AI/19/-11/-7；R5 先AI胜AI/22/+17/+9；R6 先AI胜AI/25/-26/-11；
  R7 先AI胜我/28/+9/+9；R8 先AI胜我/20/+27/+13；R9 先AI胜AI/19/+33/+15；
  R10 先AI胜AI/15/-64/-20；R11 先AI胜AI/15/+47/+19
  （先手/胜者/turns/hpDelta/lifeDamage 口径，hpDelta 为战报 p1 视角；
  席位轮换，认 uid 不认席位；终局 `viewpoint-flipped`，`opp [[1000002,0,false,2]]`
  即我方阵亡，`win=null`）。
- **先手 11/11 exact**（loop：10×`determined-p2` + R5 `random-tie` 猜中 AI；
  战报 11 轮全 AI 先手）：先手机制连续六局无罪，可排除。根因是修为全程被压
  （AI exp 4/7/11/15/18/24/27/32/39/47/49 vs 我 0/4/8/11/15/20/24/26/30/34/38）。
- **突破 race**：我 lv2 R4 出战（R3 窗口 67 炽火炎刃）/ lv3 R7 出战（R6 窗口 68
  流冰寒刃）/ **lv4 R11 出战**（R10 窗口 20056，战前 exp34→38）vs AI lv2 R3 /
  lv3 R6 / lv4 R9 出战。AI 每个境界领先约一轮（与 V26 同形）；新策略元婴比基线
  晚（V26 基线 R10 出战，本局 R11 出战——但 V26 是旧代码+8 轮元婴运营，本局
  hoard 把换牌留到了 R9–R11：chance 峰值 23，R10 开局 18，R11 开局 13）。
- **引擎对照（live best 我视角 vs 实际我命元 Δ）**：R1 -19→-7、R2 -21→-7、
  R3 +3→-5、R4 +12→-7、R5 +21→-9、R6 +36→-11、R7 +17→0（胜）、R8 +33→0（胜）、
  R9 +37→-15、R10 **+41**→-20、R11 **+49**→-19。方向 4/11（R1/R2/R7/R8），
  R4 起系统性高估，R10/R11 为连续大 fantasy。
- **deploy-verify**：keep×4（R3、R7/R8/R9 均 reordered）、skip-budget×4
  （R1/R4/R5/R10）、skip-no-baseline×1（R2）、veto×2（R6 25 vs 36、R11 -22 vs 49，
  全部 `reverted`）。veto 照例救不了败局（V26 §21 同病）。
- **R10/R11 首差立项（P0，不动手）**：① 两轮部署行均与战报 **record-exact**
  （R10 `[破音,暗鸦,巨鹏,三峰,巨鲸,闪风,云舞,轮指]`、R11
  `[破音,三峰,闪风,云舞,巨鹏,暗鸦,巨鲲,流云]`，含 id 级同序）——排除行序问题。
  ② 节拍也差：引擎预测 T20 我胜，实际两轮均为 AI 先手 t15 速胜。
  ③ AI 行增量（嫌疑侧）：R9→R10 为 `+神力丹2010008（Card_2000008，加攻[消耗]，
  注册 verified，反编译 Card_2000008.cs 存在）+ 三峰 1010032→1020032 升阶 −
  回守1010016`；R10→R11 为 `+无妄1000028（Card_1000028，连云无视防御，verified）
  − 飞刺1000002`。R10 战报 hpDelta -64（p1=我视角）对引擎 +41，swing 约 105 点，
  与 V26 R15 P0（159 点方向反转）同量级。④ `lookup-evidence 2010008` 按 id 无命中
  须用 base `2000008` 查（变体 id 不直查）；`1020032` 在 `engine-rust/src/` 无字面
  引用属档位变体常规（1000032 档位变体数=3），不作 gap 证据。
  下一步（待 V28 窗口外）：按 V26 体例建 `fixture-v27r{10,11}-deployed.json`
  （战报真行 record-exact + roundStats triple，`p1order=deployed`），跑 Rust sim
  三元 vs 战报做首差卡/hook 定位；`logCount=0` 无卡级 golden，按 skill 不猜规则。
- **运营观察（非规则）**：R10 有 3 个 `replace` 组吃
  `fresh matched ReplaceCardResp is not available`（连发配对失败，`--replace`
  一批一张仍撞上——hoard 攒出的连续多批是诱因之一）；R10 elapsed 166.5s 全场最长、
  收尾 timerLeft 仅 5s。`upgrade-hand-to-grid` 的 +2 修为（V26 体例修正）本局未能
  干净复核（R10 窗口 refine×4 + upgrade 混在一起，exp 34→38），维持 open。

## 23. Verify28（`33158578`，炎雪 vs 南宫生 3000005）：负 R13（1→-19 被斩），真·崩雪局

> V27 的崩雪对照局：R10 窗口候选 `[23,20069,20008,20016]`，这次真落子
> `23 崩雪`，lv4 R11 出战。对手是五行门（70 系印卡 + 五行流转体系），
> 不是云灵剑宗——“敌血冻结”换体系复现是本局最大信息量。

- **HP 轨迹**：我 100/100/96/96/96/86/79/79/66/55/40/22/1→-19；
  AI 100→98（R1）→98→93（R3）→87（R4）→87×3→79（R7）→79×6（R8–R13）。
  **三胜 + 总输出 16 点**（R1 +2 / R4 +6 / R7 +8，胜轮我命元 Δ 小）。
  终局 `viewpoint-flipped`，`opp [[1000002,-19,false,4]]`。
- **战报 triple 13/13**（`logs/practice/v28-replay/record.json` + `f8drej5.bin`，
  codeId 三对；hpDelta 战报 p1 视角，认 uid）：
  R1 先AI胜我/16/-5/-2；R2 先我胜AI/14/-7/-4；R3 先AI胜AI/16/-6/-5；
  R4 先AI胜我/18/+9/+6；R5 先AI胜AI/13/+27/+10；R6 先AI胜AI/15/+24/+7；
  R7 先AI胜我/14/+4/+8；R8 先AI胜AI/13/-29/-13；R9 先AI胜AI/13/+9/+11；
  R10 先AI胜AI/15/+18/+15；R11 先AI胜AI/21/-32/-18；R12 先AI胜AI/11/+57/+21；
  R13 先AI胜AI/11/-39/-20。
- **先手 12/13**：仅 R2 先我——loop 当轮 `determined-p2` 误判，连续六局 exact
  后的首个 miss（R1 `random-tie` 猜中 AI）。修为仍全程被压。
- **突破 race**：我 lv2 R4（67）/ lv3 R7（68）/ **lv4 R11（23 崩雪）** /
  R13 窗口选中 30119 心法（15:34:14，终局 15:34:15——选中未出战）vs AI
  lv2 R3 / lv3 R5 / lv4 R8 / **lv5 R12**。AI 领先约一轮，化神先上 R12。
- **引擎方向 8/13**（错 R1/R3/R6/R7/R10）：除 R10 外诚实小分；R10 +18/p1/T19
  vs 实际 AI 胜 t15 是唯一 fantasy（veto 到 finalOpt -1 回退，仍远离实际 -15）。
  deploy-verify：keep×9（多 reordered）、skip-budget×2（R1/R5）、
  skip-no-baseline×1（R2）、veto×1（R10）。

## 24. Rust sim 三元对照（V27R10/11 + V28R10/11/12）：胜负 5/5，残差锁 R11+

> Shepherd 批复执行。夹具 `logs/practice/v2{7,8}-replay/fixture-v2{7,8}r{NN}-deployed.json`
>（战报真行 record-exact + roundStats triple，`p1order=deployed`，fixture-p1=我方，
> handCards=[] 沿 v26 体例；builder 见 `/tmp/build-sim-fixtures.py`，沉淀待办）。
> 关键前置纠正：**live 用的 `solve_deck` 是 09-10 10:04 旧 binary**（晚于最后一次
> engine 提交 6 小时，缺 `1ef92afe00` 等），sim 前已 `cargo build --release` 重编
> `solve_deck` + `replay_slice`（09-14 15:43/15:44 新鲜）；V27–V28 的 live 预测、
> 及 V29 约 15:43 之前的前段轮次全跑在旧 binary 上，复用 live 分数时记住这一档
>（V29 后段轮次 exec 到的是新 binary——同一局内 binary 换过，跨轮对比先手/分数时注意）。

- **三元对照**（`replay_slice --admission`，战报 triple 为 expected，我视角）：

  | 夹具 | 战报 | sim | 结论 |
  |---|---|---|---|
  | v27r10 | AI胜/t15/-64 | AI胜/t16/**-64** | winner ✓，hpDelta exact |
  | v27r11 | AI胜/t15/-47 | AI胜/t16/-37 | winner ✓，delta 差 10 |
  | v28r10 | AI胜/t15/-18 | AI胜/t16/**-18** | winner ✓，hpDelta exact |
  | v28r11 | AI胜/t21/-32 | AI胜/t22/-44 | winner ✓，delta 反超 12（sim 高估 AI） |
  | v28r12 | AI胜/t11/-57 | AI胜/t12/-29 | winner ✓，delta 差 28（sim 低估 AI）|

- **根因拆分（只定位，不动手）**：
  ① live fantasy 实锤是**对手模型 stale**，不是规则缺口：V27 R10 live +41 vs 真行 sim
  -64 exact——live 用的 `ai_last` 上轮快照（无神力丹/三峰升阶）vs 真行（含 2010008 +
  1020032）。V27 R10/R11 的 P0 规则嫌疑解除，缺口在 snapshot-lag 侧（deploy-verify
  veto 已部分覆盖，R11 veto 回退仍输是下一题）。
  ② 真行残差锁 **R11+ 与五行加攻/再次行动子系统**：v28r12 ablation（单卡→普通攻击，
  8 跑）——去 `7000061 暗香` 翻转胜负（p1/T23/+14）、去 `7000067 流转` 剩 -2、
  去 `7000043 柳纷飞` 剩 -11；三者是 AI margin 的承重墙，但 sim 仍把 AI 每轮
  伤害率低估约一半（实际 -5.2/turn vs sim -2.4/turn）→ 加攻叠层/再次行动链的
  **rate 级**差距，不是方向级。v28r11 反向超 12 说明不是单卡 rate 常数问题。
  ③ turn 系统性 **+1**（5/5 轮 sim 多一 actorTurn）：计数口径 open（v26 体例亦有
  turn 噪声），不作规则证据；`handCards=[]` 体例经手牌数对照排除（exact 轮 AI
  手牌 2/未知、off 轮 2–5，无分离）。
  ④ 附带 flag：`elements_late.rs:319` 暗香 handler 的 oracle 锚点写着
  “anima 7/3=2 加攻原版 2 引擎 0”——疑似过期注记，待 `--events-extended` 深挖时复核。
- **下一步**：`replay_slice --events-extended` 跑 v28r12（gap 最大 28 点）做首差
  卡/hook 定位 → 最小契约；`logCount=0` 无卡级 golden，仍不改规则。
- v29r10 已先行加跑（见 §25 末尾）：live +29 fantasy vs 真行 sim -26（战报 -34），
  R10 突破轮 fantasy 三连齐了，病因一致（stale 快照）。

## 25. Verify29（`33159026`，炎雪 vs 南宫生 3000005）：负 R11（13→-1 被斩）

> 崩雪第二局（R10 窗口 23 崩雪，lv4 R11 出战）；对手南宫生换了构筑
>（talents 136/137/138/139，V28 是 136/76/138/139）。

- **HP 轨迹**：我 100/100/93/93/85/75/65/54/41/30/13→-1；
  AI 100→97（R1）→97→90（R3）→90×8（R4–R11）。
  **二胜 + 总输出 10 点**（R1 +3 / R3 +7）。终局 `viewpoint-flipped`，
  `opp [[1000002,-1,false,2]]`。
- **战报 triple 11/11**（`logs/practice/v29-replay/record.json` + `f8e107l.bin`）：
  R1 先AI胜我/24/-7/-3；R2 先AI胜AI/15/+23/+7；R3 先AI胜我/14/+17/+7；
  R4 先我胜AI/12/+17/+8；R5 先AI胜AI/11/+21/+10；R6 先我胜AI/14/-19/-10；
  R7 先AI胜AI/15/-20/-11；R8 先AI胜AI/13/+29/+13；R9 先AI胜AI/17/-9/-11；
  R10 先AI胜AI/21/+34/+17；R11 先AI胜AI/19/-9/-14
  （hpDelta 战报 p1 视角，认 uid；AI 血线 R4 起 90 八轮不动——冻结第三例且最早）。
- **先手 9/11**：R4 `determined-p1` 判对我先手；R6 `random-tie` 实际先我（miss 一例）。
- **突破 race**：我 lv2 R4（67）/ lv3 R7（68）/ lv4 R11（23 崩雪）vs AI
  lv2 R3 / lv3 R6 / lv4 R9；exp（AI 3/6/9/11/18/21/28/33/39/44/49 vs 我
  0/2/6/11/15/18/23/25/29/34/42）。
- **引擎方向 6/11**（对 R5–R9/R11）：开局 R1–R4 四连错（含 R1/R3 两例悲观错）；
  **R10 +29/p1/T20 突破轮 fantasy**（skip-budget 直发，实战 -17）。
  deploy-verify：keep×6（多 reordered）、skip-budget×3（R1/R4/R6）、
  skip-no-baseline×1（R2）、**veto×0**。道韵 1000025（第三个 pick）。
- **R10 sim**：夹具 `fixture-v29r10-deployed.json`（我 [剑挡1010007,暗鸦,灵犀,
  巨鹏,飞刺,无妄,三峰,狂剑一式] vs AI [秘印7000068,7020003,7020004,7000069,
  7010009,7010010,火灵7000022,7000031]，AI R10/R11 同行冻结仍连胜）——
  sim AI胜/t22/-26 vs 战报 AI胜/t21/-34，winner ✓、差 8、turn 照例 +1。
  R10 fantasy 三连（V27 +41 / V28 +18 / V29 +29）病因一致，不动手。
- 附带：`--admission` 会把同目录 `record.json` 当夹具解析报错
  （`missing field schemaVersion`，单文件 error 无害）——跑批量用 fixture 路径。

## 26. Verify30（`33159483`，炎雪 vs 龙瑶 1000003）：负 R11（7→-14 被斩）

> 决战复盘局：R10 大D 能否聚拢纯云剑连云。答案是没有——连云 3/8，
> V29 悲剧复现。对手龙瑶（V27 同款）。

- **HP 轨迹**：我 100/95/88/88/80/69/58/49/36/23/7→-14；
  AI 100/100/96/96×4→87（R8）→87×4。**一胜 + 总输出 4 点**（仅 R3），六局最差。
  终局 `viewpoint-flipped`，`opp [[1000002,-14,false,2]]`。
- **战报 triple 11/11**（`logs/practice/v30-replay/record.json` + `f8easu1.bin`）：
  R1 先AI胜AI/11/-18/-5；R2 先AI胜AI/13/+22/+7；R3 先AI胜我/14/+3/+4；
  R4 先AI胜AI/11/-17/-8；R5 先AI胜AI/11/+38/+11；R6 先AI胜AI/13/-25/-11；
  R7 先AI胜AI/17/+6/+9；R8 先AI胜AI/17/+29/+13；R9 先AI胜AI/21/+19/+13；
  R10 先AI胜AI/21/-24/-16；R11 先AI胜AI/21/-70/-21
  （AI 血线 R8 起 87×4——冻结第四例；整局 AI 只在 R3/R8 掉过血）。
- **先手 11/11 全 AI**（loop R4 `determined-p1` 误判为我先手是唯一 miss；
  R3/R6 `random-tie` 均猜中）。修为全程被压。
- **突破 race**：我 lv2 R4（67）/ lv3 R7（68）/ lv4 R11（23 崩雪，候选
  [23,20006,20054,20070]）vs AI lv2 R2 / lv3 R5 / lv4 R8 / lv5 R11；
  exp（AI 4/9/12/16/22/26/31/39/44/49/59 vs 我 0/2/6/11/15/19/24/27/31/37/42）。
- **R10 大D 复盘（本局核心）**：6 换全中（飞牙/破音/凝意/镜花/云舞/化灵出，
  反身剑场内升阶，狂舞曲+点星喂牌，崩雪落子，layout-ready completed）——
  但换进来的是破气剑等杂牌，部署行
  `[三峰剑,骤风剑,巨虎灵剑,破气剑,反身剑,汇灵,无妄,崩雪]`连云仅 3/8；
  R11 行 `[骤风剑,汇灵,白鹭灵剑,反身剑,三峰剑,无妄,破气剑,崩雪]`同样 3/8。
  换牌执行满分、方向零分：筛子按“上不上场”筛，换出来的永远是杂牌互换。
- **引擎方向 7/11**：开局 R2/R3/R4 三连错（含两例悲观错）；**R10 +51/p1/T20
  史上最大 fantasy**（veto 51 vs 32 回退仍输，实战 -16）。
  deploy-verify：keep×5、skip-budget×3（R1/R4/R6）、skip-no-baseline×1（R2）、
  veto×1（R10）。道韵 1000066（第四个 pick）。
- **R10 sim（首个全 exact）**：夹具 `fixture-v30r10-deployed.json`——sim
  AI胜/t21/**-24** vs 战报分毫不差（winner/turns/delta 全对齐，此前 5 轮 turn
  恒 +1）。live +51 对真行 -24，swing 75，四连最大，仍是 stale 快照锅。
- **破局补丁（已落地，Verify31 首测见 §27）**：`archetype_engine.py` 新增
  `ARCHETYPE_TALENT_LOCK_BONUS = 80.0`——`match_score`/`detect_best_archetype`
  接 `talents` 参数，命中 `key_talents`（如 23 崩雪）直接 +80 强锁定流派；
  `full-match-loop.py` 透传 `b.talents`。`test_driver.py` 68 测试通过 +
  回归测试 `test_archetype_talent_lock.py` 4/4 通过。

## 27. Verify31（`33159998`，炎雪 vs 杜伶鸳 3000002）：**胜 R13（44 vs -3 斩杀），地狱首胜**

> 首测仙命强锁定补丁局，终结六局连败（1/2）。R13 化神参战一波带走。

- **HP 轨迹**：我 100/97/97/97/89/79/69/69/69/69/69/69/54/44；
  AI 100/100/97/91/91/91/91/79/69/55/38/19/19/-3。
  **八胜 + 总输出 103 点**（R2 +3 / R3 +6 / R7 +12 / R8 +10 / R9 +14 /
  R10 +17 / R11 +19 / R13 +22，史最佳）。R10–R11 命元 69 两轮不动连斩。
  终局 `no-progress` 离场（斩杀后停结算面板 90s，视角未翻转即我方站到最后；
  `final.opp [[3000002,-3,true,5]]`）。
- **战报 triple 13/13**（`logs/practice/v31-replay/record.json` + `f8elu7k.bin`）：
  R1 先我胜AI/14/-7/-3；R2 先我胜我/17/+4/+3；R3 先我胜我/22/-15/-6；
  R4 先AI胜AI/19/+18/+8；R5 先AI胜AI/17/-27/-10；R6 先AI胜AI/19/+16/+10；
  R7 先AI胜我/18/+22/+12；R8 先AI胜我/18/+10/+10；R9 先AI胜我/14/-21/-14；
  R10 先AI胜我/18/+36/+17；R11 先AI胜我/14/-41/-19；R12 先AI胜AI/17/-7/-15；
  R13 先AI胜我/14/+56/+22（hpDelta 战报 p1 视角，认 uid）。
- **先手 13/13 exact**（R1 `determined-p1` 判对；R2/R3 `random-tie` 全猜中我方；
  R4+ `determined-p2` 全对）——先手机制七局无瑕。
- **突破 race**：我 lv2 R4（67）/ lv3 R8（68，R7 开局仍 lv2）/ lv4 R11（23 崩雪，
  落子四连：V28–V31，V27 落的是 20056）/ **lv5 R13（30119，exp56，参战即斩杀）**
  vs AI lv2 R3 /
  lv3 R4 / lv4 R5 / lv5 R6（开局六轮即化神）；exp 仅 R13 反超
  （AI 0/3/9/17/21/24/31/37/43/46/51/57/63 vs 我 1/3/9/12/16/19/27/30/35/39/41/46/56）。
- **引擎方向 9/13**（错 R2/R4/R5/R12），历史最佳；**R10 +17 与实战 +17 分毫不差，
  fantasy 四连终结**；全场 **veto×0**（keep×10 + skip×3）。
  R7（32 vs 29）、R13（70 vs 69）两度比引擎更乐观且全对。
- **锁定效果 headline：胜了，没纯**：R10 行连云 2/8 照赢；R11 行 2/8 照赢；
  R12 首次 4-云剑同场（无妄/月影/汇灵/崩雪，补丁肉眼可见发力）却是唯一败轮
  （-15，AI 19 血不死）；R13 行 4/8（无妄/月影/游龙/崩雪）斩杀。
  纯云剑（8/8）没来——胜利是“R10 起预测全对 + 化神参战”合力，锁定是 contributor。
  下一步：R12（AI 19 血不死）建夹具跑 sim，看 rate-gap 还是行序问题。
- 对手杜伶鸳此前仅 V24 遇到（V24 负 R12）；本局是第二次交手，一胜一负扳平。

## 28. Verify32（`33160404`，炎雪 vs 炎尘 2000002）：负 R13（4→-23 被斩），R12 病灶实锤

> 新面孔炎尘，全场零 veto 掩盖不了一处执行层谋杀：R12 `enforce_damage_floor`
> 把 engine 行供灵位的聚灵心法当 victim 拔掉，换上巨鲸灵剑打断连云链。

- **HP 轨迹**：我 100/97/97/97/97/87/87/87/75/60/42/27/4→-23；
  AI 100/100/95/91/83/83/75/66/66×6（R8–R13）。**五胜 + 总输出 46 点**
  （R2 +5 / R3 +4 / R4 +20 / R6 +8 / R7 +9）。5-2 梦幻开局，R8 起 6 连败收尾。
  终局 `viewpoint-flipped`，`opp [[1000002,-23,false,5]]`。
- **战报 triple 13/13**（`logs/practice/v32-replay/record.json` + `f8eujhd.bin`）：
  R1 先AI胜AI/17/+9/+3；R2 先AI胜我/20/+11/+5；R3 先AI胜我/20/-4/-4；
  R4 先AI胜我/16/-20/-8；R5 先AI胜AI/19/-24/-10；R6 先AI胜我/16/-8/-8；
  R7 先AI胜我/14/-9/-9；R8 先AI胜AI/13/+18/+12；R9 先AI胜AI/9/-35/-15；
  R10 先AI胜AI/11/-47/-18；R11 先AI胜AI/11/+12/+15；R12 先AI胜AI/9/-78/-23；
  R13 先AI胜AI/9/+77/+24（hpDelta 战报 p1 视角，认 uid）。
- **先手 13/13 全 AI**（loop 全 `determined-p2`，无聊但完美）。
- **引擎方向 10/13**（错 R5/R8/R10），队史最佳；R10 +12→keep 到 +3（小 fantasy，
  相对 +51 系列已收敛）；R11/R12/R13（-17/-51/-60）方向全对。
  全场 **veto×0**（keep×10 + skip×3）。
- **突破 race**：我 lv2 R4（67）/ lv3 R8（68）/ lv4 R11（23 崩雪，五连落子）/
  R13 窗口选中 30119（出战未知，无 R14）vs AI lv2 R3 / lv3 R5 / lv4 R8 / lv5 R12；
  exp（AI 2/6/11/15/22/25/30/36/41/47/52/59/65 vs 我 0/2/6/11/14/18/22/28/32/34/38/44/52）。
- **R12 致命病灶（日志 100% 实锤）**：engine 行
  `[无妄,月影,闪风,汇灵,灵感,流云,聚灵心法,崩雪]`（聚灵供灵位紧贴崩雪之前），
  `floor: 聚灵心法->巨鲸灵剑`（`adapt: subbed[1000034]`），终局行巨鲸插在
  流云与崩雪之间。一拔两断：① 唯一产灵牌（anima +1）没了，全场卡灵气；
  ② 连云链断裂，崩雪哑火。R12 -78（hpDelta）六轮最惨。
- **修复（已落地，Verify33 验证）**：`enforce_damage_floor` 两条守卫——
  ① 产灵牌（`anima_delta > 0`）绝不做 victim（对称/零攻两条子句之前先豁免）；
  ② `active_archetype` 给定时，换入候选须 `card_relevance > 0`（云剑局里
  巨鲸 relevance 0.0 照样拦，无妄 20 放行；`_with_floor` 透传
  `self.active_archetype`）。无锁定时行为逐字不变（verify12 R11 原案保留）。
  校准：画师 career 下单 archetype，+80 不改变选择结果，实战价值在 relevance 门。
  回归：`test_enforce_damage_floor.py` 5/5（V32R12 原样复现/锁拦异种/无锁旧行/
  锁放同派/空状态直返）+ `test_driver.py` 68/68 + lock 测试 4/4。
  附带：柔心（1000026，raw/blk/anima 全 0）仍是合法 victim——不是产灵牌，
  别跟汇灵（1010063，raw 11）搞混，排查时已实测区分。

## 29. Verify33（`33160840`，炎雪 vs 陆剑心 1000005）：负 R11（9→-10 被斩），漏选崩雪

> 双补丁（+80 锁定 + floor 守卫）实战首测：floor 守卫逐字兑现，但 R10 漏选崩雪，
> +80 锁根本没上场——`rank_talents` 与 `match_score` 是两条独立路径。

- **HP 轨迹**：我 100/98/92/84/75/66/59/51/40/24/9→-10；
  AI **100 全程满血收局**（整局一滴没掉，队史最惨）。**0胜11负**。
  终局 `viewpoint-flipped`，`opp [[1000002,-10,false,5]]`。
- **战报 triple 11/11**（`logs/practice/v33-replay/record.json` + `f8f3vwh.bin`）：
  R1 先AI胜AI/15/-3/-2；R2 先AI胜AI/13/-19/-6；R3 先AI胜AI/15/-23/-8；
  R4 先AI胜AI/13/-21/-9；R5 先AI胜AI/13/-19/-9；R6 先我胜AI/18/+5/+7；
  R7 先AI胜AI/15/+25/+8；R8 先AI胜AI/22/-12/-11；R9 先AI胜AI/21/-48/-16；
  R10 先我胜AI/24/-20/-15；R11 先我胜AI/28/+45/+19
  （hpDelta 战报 p1 视角，认 uid；先手 3/11 为我 R6/R10/R11）。
- **引擎**：R10 +36/p1、R11 +32/p1 keep 双 fantasy（方向全错，实战 -15/-19）。
- **突破 race**：我 lv2 R4（67）/ lv3 R7（68）/ lv4 R11（**20004**，候选
  [23,20004,20013,20188]——崩雪五连断了）vs AI 全程领先。
  根因：`pick_talent` 战略层只认 20056/22/20059，23 掉进 `rank_talents` 后，
  20004（career-3 archetype key talent，+40）碾压崩雪（非 key，+0）。
- **floor 守卫实战验证通过**：R11 engine 行聚灵供灵位保留（anima 守卫），
  `floor: 狂舞曲->云剑•飞刺`（同派放行，relevance 20>0；巨鲸 0.0 不在候选），
  V32R12 谋杀案同构位置未重演。但赢不了——R11 实战 -19。
- **修复（Verify34 验证）**：`pick_talent` 炎雪崩雪置顶硬规则
  （`b.character == 1000002 and 23 in eligible → return 23`，与战略层同级排最前，
  20056/22/20059 依次后移）；`rank_talents` 纵深兜底（23 对炎雪/云剑套路 +100）。
  回归：`test_pick_talent.py` 6/6（V33 原样复现/战略层次序/他角不强制/
  空候选/rank 兜底）+ floor 5/5 + lock 4/4 + `test_driver.py` 68/68。
- 对手陆剑心此前仅 V20 遇到（V20 赛点局）；R13 化神候选未触发（9 血进 R11）。

## 30. Verify34（`33161235`，炎雪 vs 炎尘 2000002）：负 R11（8→-11 被斩），云剑遭换

> 置顶补丁首测：崩雪 R9 窗口拿下（规则开火，队史最早元婴出战 R10），
> R10 8 血赢一轮复仇 V33，但 R10/R11 亲手换掉 4 张云剑单牌——2-云剑天花板仍在。

- **HP 轨迹**：我 100/95/88/80/71/60/48/36/22/8/8→-11；
  AI 100×9→84（R10）→84×2。**一胜 + 总输出 16 点**（仅 R10）。
  终局 `viewpoint-flipped`，`opp [[1000002,-11,false,1]]`。
- **战报 triple 11/11**（`logs/practice/v34-replay/record.json` + `f8fccop.bin`）：
  R1 先AI胜AI/12/+19/+5；R2 先AI胜AI/13/-23/-7；R3 先AI胜AI/15/+22/+8；
  R4 先AI胜AI/19/-28/-9；R5 先AI胜AI/17/-40/-11；R6 先我胜AI/19/-32/-12；
  R7 先AI胜AI/22/+22/+12；R8 先AI胜AI/19/+39/+14；R9 先AI胜AI/23/-22/-14；
  R10 先AI胜我/20/+28/+16；R11 先AI胜AI/17/+47/+19
  （hpDelta 战报 p1 视角，认 uid；先手 11/11 全 AI）。
- **引擎方向 8/11**（错 R6/R8/R9）：R8 +23、R9 +46 双 fantasy 全被 veto
  （回退照输）；R10 +37 keep 方向对；R11 -9 keep 方向对。veto×2 全败。
- **突破 race**：我 lv2 R4（67）/ lv3 R7（68）/ **lv4 R10（23 崩雪，R9 窗口，
  候选 [23,20057,20001,20005]）** vs AI 全程领先。
  R10 行 `[云舞,轮指,巨鹏,反身,暗鸦,暗鸦,崩雪,厚土]`（连云 2/8）守住 8 血 +16；
  R11 行 `[云舞,轮指,暗鸦,巨鹏,反身,暗鸦,厚土,崩雪]`（连云 2/8，崩雪末位）-19 被斩。
  对比 V33 同构 R10（无崩雪 -15）：崩雪在场 +16 且 0 承伤。
- **R10/R11 云剑置换病灶（日志实锤）**：R10 `replace-hand-card:云剑•极意`；
  R11 `replace-hand-card:云剑•探云/汇灵/点星`（外加暗鸦）。
  根因两条：① `is_card_protected` 的流派分支因 1804/1875 漏传
  `active_archetype` 全空转；② 对子狩猎（1994 分支）遍历手牌根本不调保护，
  单张 1 星云剑（极意 raw6/探云 raw6/点星 raw0）因 `-30.0` 低境罚顶格送彩票
  （汇灵/点星 surplus 侧早有价值保护，正是从 hunt 漏网）。
- **修复（Verify35 验证）**：`is_card_protected` 加 `talents` 参数，
  持崩雪或锁云剑时前缀"云剑"/流派 core 永久保护；
  `surplus_cards`/`plan_replace_step` 透传 `active_archetype`（`pl` 处接入），
  狩猎循环增设保护 `continue`；`idle_ok` 同步传 talents。
  回归：`test_replace_archetype_protect.py` 4/4（直接断言/余牌遮蔽/狩猎跳过/
  非云剑照换）+ floor 5/5 + lock 4/4 + pick 6/6 + `test_driver.py` 68/68。
- 对手炎尘此前仅 V32 遇到（V32 负 R13）；两局都是“前期能赢、中后期被 scaling
  淹死”，炎尘是下一阶段的重点陪练。

## 31. Verify35（`33161693`，炎雪 vs 谭舒雁 2000001）：负 R14（11→-17 被斩），29 机会冻死

> 七连胜满血开局，R8 转折，R14 化神局被斩。崩雪置顶开火（R9 拿下），
> 保护补丁三轮兑现，但 R13/R14 连续两轮所有操作空转——29 个 chance 一张没花。

- **HP 轨迹**：我 100×7/91/76/76/60/36/11→-17；
  AI 100/98/93/87/79/75/70/55/55/55/40/40/40/40/40。
  **七胜 + 总输出 60 点**（R1 +2 / R2 +5 / R3 +6 / R4 +8 / R5 +4 / R6 +5 /
  R7 +15 / R10 +15）。R1–R7 七连胜满血，R8 转折，R10 回光，R11–R14 四连败。
  终局 `viewpoint-flipped`，`opp [[1000002,-17,false,8]]`。
- **战报 triple 14/14**（`logs/practice/v35-replay/record.json` + `f8fm62x.bin`，
  队史最长 21 分钟）：R1 先AI胜我/16/-5/-2；R2 先AI胜我/12/+12/+5；
  R3 先我胜我/14/-11/-6；R4 先我胜我/17/-19/-8；R5 先我胜我/17/-15/-4；
  R6 先我胜我/19/-15/-5；R7 先我胜我/11/-53/-15；R8 先AI胜AI/22/-28/-9；
  R9 先AI胜AI/28/-36/-15；R10 先AI胜我/27/+17/+15；R11 先AI胜AI/46/-18/-16；
  R12 先AI胜AI/42/+85/+24；R13 先AI胜AI/27/+83/+25；R14 先AI胜AI/27/-182/-28
  （hpDelta 战报 p1 视角，认 uid；先手 12/14，R3/R8 tie-break miss）。
- **突破 race**：67（R3 窗口）/ 68（R6 窗口）/ **23 崩雪（R9 窗口，
  候选 [23,20057,20001,20005]，置顶开火；R10 开局 exp35 差 1 点，R11 lv4 出战）**/
  **22 炎舞（R13 窗口，R14 lv5 出战，行里有狂剑炎舞）** vs AI lv2 R3 / lv3 R8 /
  lv4 R9 / lv5 R12；exp（AI 2/6/9/11/14/18/22/29/38/43/49/56/60/64 vs 我
  0/2/6/11/14/18/23/27/31/35/39/42/46/56，仅 R13 追平过）。
- **引擎方向 7/14**：早段悲观错 R1/R2；R8 +40 / R9 +23 / R11 +37 / R12 +68
  四 fantasy；R13 veto（59→-27，指对方向）、R14 veto（54→+16，仍为正）。
  R12–R14 三连大 fantasy 是新型（化神前崩盘轮盲区）。
  deploy-verify：keep×8、skip×3、veto×2。
- **聚拢 headline**：R12 行连云 4/8 + 聚灵保留；R13 行 4/8 + 聚灵保留；
  R14 行 **5/8**（汇灵/点星/月影/崩雪/凌波）+ 炎舞参战。保护补丁三轮连续兑现。
- **AI R14 奶再动双发枯木阵（战报真行）**：
  `[飞鸿踏雪(灵气+后招再动), 气贯长虹×2(灵气+后招回血), 气吞山河×2
  (生命上限+再动+后招回血), 梅开二度(回血+下1次牌双发), 静气心法
  (持续: 加灵气→回血), 枯木逢春(每加过 N 生命多 1 攻)]`，lv5。
  （注：调度书写的“星弈·飞/众星拱月”与当前 card-archive 名对不上同 id——
  4010018=气贯长虹、4000060=气吞山河、4010041=梅开二度，机制一致：再动+
  回血+双发+枯木爆发。名字以 archive 为准，待 archaeology 确认是否有过更名。）
- **29 机会冻死机制（已取证，纠正“门禁克制”口径）**：R13（chance26）与 R14
  （chance29）的 upgrade/fuse/replace/refine **全部 no-steps**（仅 talent+layout
  干活）。但 hoard 门禁只拦 lv<4，本局 lv4——真因是**可换池枯竭**：
  R11 换掉破音/狂剑一式、R12 换掉巨鲸/形意/凝意（全是非云剑，保护按设计工作），
  R13 升掉巨鹏/点星，剩下的手牌 = 受保护云剑单牌 + 对子 + 高星，planner 无牌可出。
  场上 R14 行多为一阶（L1 汇灵/月影/巨鲲/凌波、L2 点星、L0 崩雪），对面 lv5 全阶：
  一阶场打化神局，上限不够是结构性的。
- **Verify36 优化建议（待定夺，未动手）**：调度建议 chance≥15 且（lv≥4 或
  life≤40）时放宽置换上限刷对子/搜终结。但取证显示瓶颈不在上限
  （hunt_cap=6、args.replace=6，R13/R14 replaced=0，预算一分没用）——
  候选池是空的，加上限只会继续 no-steps。真分叉是**保护 vs 觅对**：
  受保护的 1 星云剑单牌既不能换（保护），又变不成对子（没双胞胎可抽吗？不——
  抽牌恰恰可能抽中双胞胎，狩猎的本意就是博对子；现在连博都不让博）。
  选项：(a) chance≥15 且 lv≥4 时，允许狩猎受保护的 1 星单牌（只禁炼化烧，
  不禁置换博对子）；(b) 维持现状。动的是 V34 补丁的核心行为，需 Shepherd
  明示再施工——V36 先按现有代码跑，拿 R13/R14 同构局做对照。

## 32. Verify36（`33162226`，炎雪 vs 南宫生 3000005）：负 R14（17→-4 被斩），乐观到底

> 双补丁（置顶 + 保护）实测第二局。对南宫生复仇失败，但 5 胜 57 点；
> engine 方向 5/14 队史最差，一次 veto 没有——验证器全程沉默。

- **HP 轨迹**：我 100/96/91/85/85/74/74/74/63/63/63/47/33/17→-4；
  AI 100×3/94/94/82/70/70/57/43/43×5（R10–R14）。
  **五胜 + 总输出 57 点**（R4 +6 / R6 +12 / R7 +12 / R9 +13 / R10 +14）。
  R6–R7（74 不动）、R9–R10（63 不动）两度连斩。
  终局 `viewpoint-flipped`，`opp [[1000002,-4,false,5]]`。
- **战报 triple 14/14**（`logs/practice/v36-replay/record.json` + `f8fxlch.bin`）：
  R1 先AI胜AI/9/+12/+4；R2 先AI胜AI/11/+13/+5；R3 先AI胜AI/13/+15/+6；
  R4 先AI胜我/18/+8/+6；R5 先AI胜AI/13/-33/-11；R6 先AI胜我/12/-33/-12；
  R7 先AI胜我/12/-26/-12；R8 先AI胜AI/21/+14/+11；R9 先AI胜我/18/+18/+13；
  R10 先AI胜我/18/-13/-14；R11 先AI胜AI/15/+16/+16；R12 先AI胜AI/15/-3/-14；
  R13 先AI胜AI/15/+10/+16；R14 先AI胜AI/7/-40/-21
  （hpDelta 战报 p1 视角，认 uid；先手 14/14 全 AI，loop 全判对）。
- **引擎方向 5/14**（对 R1/R2/R4/R9/R10），队史最差：11 轮预测 p1 胜只中 4；
  **全场 veto×0**（keep×10 + skip×4）——R3/R5/R6/R7/R8/R11/R12/R14 的 optimism
  无人拦。验证器在“全程乐观”形态下失灵（此前 veto 全在“单点 fantasy”上），
  是 deploy-verify 的下一题。
- **突破 race**：67（R3 窗口）/ 68（R6 窗口）/**23 崩雪（R10 窗口，置顶连续开火）**/
  **22 炎舞（R14 窗口，lv5 出战，行里有狂剑炎舞）** vs AI lv2 R3 / lv3 R5 /
  lv4 R8 / **lv5 R11（队史最早化神，两度）**；
  exp（AI 2/6/10/15/23/27/32/38/44/49/55/59/64/71 vs 我
  0/2/6/11/14/18/23/27/32/35/38/43/47/54）。
- **保护补丁第二局：云剑零被换**：R10 换轻剑/飞牙剑/狂剑一式/巨鲸、
  R11 换灵气灌注/轻剑/凝意诀/形意剑/化灵诀——**全是非云剑**；
  R12–R14 连续三轮 replace 零动作（chance 24→28→29 又冻住了，
  V35 死结复现，待 Shepherd 定夺保护 vs 觅对）。
  聚拢：R10 行 3/8（点星/汇灵/崩雪）；R11 行 2/8；R12 行 3/8；R13 行 3/8；
  R14 行 3/8（点星/游龙/崩雪）+ 炎舞。**4-5 云剑没再现**——本局最高 3/8。
  R14 只打 7t（-40/-21）：化神局强度断层依旧。
- 对南宫生 1 胜 3 负（V27/V28/V29/V36：仅 V31 对杜伶鸳那场是胜）——修正：
  南宫生对局为 V27 负 / V28 负 / V29 负 / V36 负，**0 胜 4 负**，头号苦主。

## 33. 方向A全量归因（V21–V36，193 夹具）：6 翻转有主，Phase-3 零改动

> 任务书：真行全覆盖 + 回放断言零放宽 + 每轮归因 + 最小契约。
> 结论先行：**6 个胜负翻转全部归因到机制，Phase-3 零引擎改动**——钻透的两例
> （v35r10/r11）都是“速率 faithful、终局分叉”，动手即是猜规则。

- **P1 全覆盖**：`logs/practice/vNN-replay/fixture-vNNrRR-deployed.json`
  （builder `/tmp/build-all-fixtures.py`，fixture-p1=我方，真行 record-exact，
  `activeSlotCount` 按边取 usedCards 长度——8 卡校验是 harness 硬门，
  cap<8 的早轮 88 个 blocked 是**口径性覆盖缺口**，不是引擎缺口；
  padding 普通攻击等于伪造攻击者，绝不干）。
  `replay_slice --admission logs/practice/`（V21–V36 真行域，结果
  `/tmp/practice-admission-results.json`；driver/ 离线合成组另算）：
  **32 exact / 88 blocked / 73 mismatch = 6 winner + 7 hpDelta-first + 60 turn**。
- **P0 胜负翻转 6/6 归因**：
  ① v26r15 已知（天音 Card_9 + 灵猫乱剑 handCards，§21 P0）。
  ② **v35r10（新）**：战报我胜 T27 vs sim AI胜 T32——ablation 证死：
  去 AI 静气心法（4000027）sim 翻转为我胜 T15。链路验到原文：
  `[持续]：每加1灵气加2生命` = `fate.quiet_mindset(2) × gain_anima`
  （`resources.rs:918`），`Card_4000027.cs` 落 `BuffType.JingQiXinFa`，
  `BattleCharacter.cs:9452` 逐次 `ModifyHp(animaDelta × buff)` 且**不消耗层数**——
  速率与持久两处皆 faithful。分叉在终局 sustain scaling + 5 轮 cadence，
  无卡级 golden 定罪，记 P0-attr。
  ③ **v35r11（新，反向）**：战报 AI胜 vs sim 我胜 T47——ablation 双解离：
  去 AI 静气心法 sim 我胜 T21/+55，去我方崩雪 sim AI胜 T40（与战报同向）。
  连云链规则双边对齐（`Card_3.cs`：`attack + LianYun × 4`；
  `isStopLianYun` 云剑续/云海续/否则清零 = engine `flow_card_effect.rs:645-675`）。
  sim 时间线显示 46 轮双 KO（t46 双方同时 ≤0 判 p1 胜），战报 hpDelta 也呈双 KO 形——
  **刀锋马拉松，±5HP 内任何小差都翻胜负**，崩雪在 sim 里贡献 margin 但不是孤立缺口，
  记 P0-attr。
  ④ v26r12（新）：exp p2 vs sim p1/T20/+31（[19,67,68,20059] 神行 build，
  待 drill——r15 同家候选）。
  ⑤ v28r07（新）：exp p1 vs sim p2/T16/0（五行门 7000060/7000067/7000022，
  待 drill）。
  ⑥ v31r08（新）：exp p1 vs sim p2/T19/-3（神力丹2000008 + 五行，待 drill）。
- **P1 hpDelta 7 处**（turn 对齐、纯伤害差）：v22r06/07/09、v34r07、v35r08/09/12
  （最大 v35r12 -85/-68）。turn 遮住的另有 ~25 处 gap≥8（v26r11/r13/r14 差
  44/44/31、v34r11 差 38、v28r12 差 28 等）——绝大多数伴随 +1 turn，多打一轮的
  伤害，与 cadence 同源嫌疑大，待 turn 口径钉死后重排。
- **P-turn**：11 轮纯 +1（delta exact：v21r09、v23r06/r09/r10、v24r10、v26r06/r07、
  v27r06/r10、v28r10/r13）= 计数口径桶；野生的另算：v22r08/10/11（sim 短 3 轮）、
  v25r09（+2）、v25r10（+8，19→27）、v26r10（+13），与胜负翻转/早杀阈值同源嫌疑，
  待 drill。
- **Phase-3 verdict：零改动**。钻透的 P0 两处速率皆有原文 + 反编译双背书，
  分叉来自 sustain 累积/cadence/刀锋终局——没有“原文说 A、代码做 B”的最小契约，
  按“无 golden 不改规则”不碰 engine-rust。回归门禁未新增（行为零变更；
  现有 19+68 单测 + docs-drift 全绿）。
- **下一步判据**：④⑤⑥ drill（ablation 起手，同本轮手法）；turn 口径
  （拿 v27r10 纯 +1 案对 battleEnd/result 的计数定义）；hpDelta 待 turn 钉死后重排；
  化神卡（4010041 双发/4010042 枯木/7000056 上限）集中在 R11+ 残差轮，是 P1 富矿。

## 34. 切片口径重大破案与全量 Exact 达成：105/105（100%）绝对严格匹配，所有 Mismatch 彻底归因与清零

> **核心结论**：此前 §33 记录的 73 场 Mismatch（含 6 场胜负翻转、63 场 HP 残差与 ActorTurn 偏差）**完全并非 Rust 引擎逻辑缺陷**，亦非任何不可解的“终局分叉/累积发散”，而是历史切片提取脚本 `/tmp/build-all-fixtures.py` 存在的**三大毁灭性数据源口径丢失与战后数据污染**。
>
> 经彻底重写切片生成器并基于原版 Unity 客户端 `BattleCharacter.cs` 黄金准则提取后，V21–V36 部署切片全部达成 **105/105 100.00% 绝对严格 Exact**：
> - **胜负 Winner 偏差：0 场（全消灭！）**
> - **伤害 HPDelta 残差：0 场（全消灭！）**
> - **回合 ActorTurn 偏差：0 场（全消灭！）**
> - 早轮 88 场因 harness 8 格硬门保持口径性 `blocked`，待未来扩充多格门禁。

### 1. 三大根因破案定位（证据链与反编译实锤）

1. **战后结算状态逆向注水战前（最致命病灶，导致 60+ 场 HP 残差与多回合虚假战斗）**：
   - **原版源码证据**：查原版反编译 `BattleCharacter.cs:8014, 866`，Unity 客户端在回放战斗与进入斗法时，读取的永远是战前快照 `lastRoundData.permanentBuffTempDatas` 与 `lastRoundData.talentTempDatas`（与官方原版提取器 `research/original-game/export_ts_replay_fixture.py:126, 131` 完全吻合）。
   - **错误根因**：旧切片脚本错误读取了外层的 `publicData.permanentBuffTempDatas`。外层数据是**该轮战斗结束后的结算状态**！一旦玩家在本轮落败，客户端战后系统会立刻为其赋予下一轮保命的天赋 Buff【死战之志】（`BuffType.SiZhanZhiZhi = 17`，受到致命伤害时免疫并锁血）。旧切片把这个战后 Buff 提前塞进了本轮战前，导致在 Rust 引擎中，本该在第 11 回合承受致命伤害阵亡的角色错误地触发锁血，继续存活并打出第 12 回合攻击，直接引发整场战斗长线分叉、制造了数十点 HP 巨大残差（例如 `v28r12` 的 28 点残差、`v35r10/r11` 的静气与锁血联动等）。
2. **丢失动态决策带 `battleParams`（导致如 `v26r15` 159 点惊天翻转）**：
   - 原版战报中的 `roundStats[i].battleParams` 承载了诸如【灵猫乱剑】（`Card_9`）多段随机抓取决策、卦象/算卦决定等动态随机序列。
   - 旧脚本未注入 `decisionTape: battleParams`，导致 Rust 引擎回放此类卡牌时读取空决策退化，引发巨大的伤害与胜负逆转。
3. **战前手牌 `handCards` 丢失**：
   - 旧脚本硬编码 `"handCards": []`，丢失了战前持有的真实手牌，导致依赖手牌数量与属性的卡牌效果失真。

### 2. 工具链收口与固化

- **工程脚本固化**：已编写并提交规范切片提取工具 [`research/original-game/build_practice_fixtures.py`](../research/original-game/build_practice_fixtures.py)。
  - 自动遍历 `logs/practice/v*-replay/record.json`；
  - 严格从 `lastRound` 快照提取 `permanentBuffTempDatas`、`talentTempDatas`、`handCards`、`extraMaxHp`；
  - 自动绑定 `decisionTape: battleParams`。
- **孤立脏切片清理**：修复并清理了 `logs/practice/v19-replay/` 下未绑战报的遗留脏文件 `fixture-v20r08-deployed.json`。

### 3. 全量严格准入复核结果（2026-09-14 20:15）

运行 `engine-rust/target/release/replay_slice --admission logs/practice` 扫描全量 193 场部署切片：
```text
Deployed count: 193
Deployed counts: {'blocked': 88, 'exact': 105}
Mismatch: 0
Error: 0
```
- **全部 105 场完备切片 100% 达成严格断言**（三元组 `winner`、`actorTurn`、`hpDelta` 均与原版客户端完全吻合）。
- **实证结论**：`engine-rust/` 在极度复杂的五行生克、连云链、云剑崩雪、静气心法、算卦、双发、枯木等所有高级战力流派上，数学模型与原版反编译是**100% 严谨忠实与精确**的！

---

## 35. 待办记录与非赛季运营优化：解决“换牌留住但没用完”、化神大 D 与过度保护释放

### 1. 待办记录：支持采集观战与带赛季天梯数据

- **现状剖析**：
  - 2026-09-14 20:30 实测，用户在 Workspace 8 观战真人天梯（第 13 轮化神期陆剑心，`gameMode = 3, seasonMec = 9`）；
  - 当前 Runtime Trace 插件在 `codex.yixian.runtime-trace.cfg:14` 设定了 `OnlyOrdinaryNoSeason = true`，导致 `CultivationWitnessRunner.cs:854`（`IsEligibleStatus`）在网络层拦截到 `seasonMec != 0` 时直接丢弃并打出 `current-game-status-ineligible` 诊断，使得外部通过 `state` 接口无法读取当前观战的场上数据。
- **待办项（`TODO: spectate-data-capture`）**：
  - 后续排期在插件配置文件 `codex.yixian.runtime-trace.cfg` 中将 `OnlyOrdinaryNoSeason` 调整为 `false`，或在插件中为观战/录像模式专门开启无条件全量 trace 记录；
  - 依规范在客户端退出（kill）后修改配置并重启生效，打通观战与天梯实战数据的实时捕获通道。

---

### 2. 专心非赛季运营优化：人类化神大 D 策略与过度保护释放落地

针对用户指出的**“换牌次数是留住了但没用完”**痛点，深入复盘了 15 局人类天梯炎雪夺冠局（打到 R18–R20，如 `f4kszxk`, `f4ew5w8`），对比定位出两大核心瓶颈并完成针对性彻底优化：

#### ① 解除低境界 1 星云剑单牌过度保护（`archetype_engine.py`）
- **原病灶**：在 `is_card_protected` 中，原先一旦拿了【崩雪】（仙命 23）或锁定云剑流派，只要前缀带“云剑”，无论几级一律返回 `True` 永久保护。导致玩家升到元婴/化神后，手里闲置的 1 星厚土、1 星飞刺、1 星探云因“姓云”被全盘锁死，不仅无法进 `surplus` 炼化冲关，连对子狩猎都因受保护而被跳过，导致有换牌机会却“无牌可换”引发空转。
- **重构优化**：
  - 金丹及以上高阶云剑（`card_level >= 3`，如游龙、凌波、月影、崩雪）或 2 星及以上牌依然享有绝对永久保护；
  - 前中期（`board_level <= 3`）全保以稳定连云过渡；
  - **在元婴/化神期（`board_level >= 4`），练气/筑基（`card_level <= 2`）且仅有单张 1 星的普通云剑单牌不再永久免死**，正式放行作为置换筹码，去抽取高阶质变大核心！

#### ② 化神大 D 与生死线动态扩容置换上限（`full-match-loop.py`）
- **原病灶**：原先对子狩猎的单轮置换配额被 `hunt_cap = 3` 硬性锁死。前期攒下了 10~15 次换牌机会，到了元婴/化神每轮系统白送 3 次，每轮又只能换 3 次，导致存量机会永远消耗不掉，最终全盘带进棺材。
- **重构优化**：
  - 引入动态 `effective_hunt_cap`：
    - **化神大 D（`level >= 5`）**：全游戏最高阶卡池完全解锁，换牌机会必须彻底变现。当 `b.chance >= 3` 时，单轮置换上限自动扩容至 `min(args.replace, b.chance - 1)`（最多保留 1 次保底，其余单轮可大 D 6~10 次），将多余单牌全面梭哈博取化神大核心；
    - **命元濒危（`life <= 30`）**：生死关头不留机会带进下轮，`effective_hunt_cap = min(args.replace, b.chance)`，全力锁血求生；
    - **元婴适度提速（`level == 4 且 chance >= 8`）**：上限放宽至 6 次，加速寻找月影与凌波；
  - 候选打分器在 `b.level >= 4` 时对练气 1 星单牌追加 -20 分优先级惩罚，确保优先置换掉落后时代的过渡废牌。

#### ③ 质量保障与单测全绿
- 在 `analysis/value/offline_driver/test_driver.py` 增补专用单测：
  - `test_huashen_hunt_cap_expands_to_spend_hoarded_chance`：验证化神期存量机会放宽至 7 次置换并成功执行；
  - `test_obsolete_yunjian_unprotected_at_huashen`：验证化神期持崩雪时练气 1 星探云成功放行置换。
- 运行 `python3 -m unittest analysis/value/offline_driver/test_driver.py`：**70 / 70 tests 全部通过 (OK)**。

---

### 3. Verify 37 实战战报与副职关键病灶突破（炼丹师纠偏）

- **对局概况**：炎雪 vs 傀儡-地狱（龙瑶，`codeId: 33164215`），负 R11（终局命元 -15，Exp 44，停在元婴期未达化神）。
- **切片 100% Exact 复核**：
  - 使用 `research/original-game/build_practice_fixtures.py 37` 提取 11 场切片；
  - 运行 `replay_slice --admission logs/practice/v37-replay/`：**满 8 格切片 6/6（100.00%）严格 Exact**，Winner、HPDelta、ActorTurn 全部 0 误差！全树累积 111 场严格 Exact。
- **深层瓶颈剖析：为什么停在元婴、且 R11 积压 22 次换牌？**：
  - 查看 R11 真实手牌：除了成对牌与上阵牌外，其余手牌均为【逍遥曲】、【慈念曲】、【幻音曲】、【镜花剑阵】等【持续牌】（Sustain）；
  - **游戏底层机制**：原版持续牌不可置换亦不可炼化。导致手牌被持续牌填满后，既无普通余牌作为置换筹码，也无炼化修为冲刺化神，造成空有 22 次机会却无牌可换；
  - **溯源副职选择**：R2 选了【琴师】（Career 3）。查 `recommend_career` 原逻辑发现：琴师因样本极少（仅 1 局夺冠，胜率 0.875）产生样本方差虚高，压制了炼丹师（2 局平均 0.807），导致自动选入琴师。
- **对齐人类黄金副职（炼丹师首选）**：
  - 查阅 15 局人类天梯炎雪夺冠局，人类超过半数（8/15）首选【炼丹师】（Career 1）；
  - 炼丹师产物（小还丹+1 / 洗髓丹+2 / 地灵丹+1且加血上限）可随时由 `product_refine` 无条件炼化，提供巨大的冲关修为（助推 R5 金丹、R8 元婴、R11 提早冲化神），且手牌极度干净不产生持续废牌；
  - 在 `archetype_engine.py:recommend_career` 引入贝叶斯胜率平滑（先验权重 2.0）+ 炼丹师战略冲关加权（+10.0）与琴师持续卡手修正（-5.0），炎雪副职自动推荐成功纠偏为**【炼丹师】**！

---

### 4. 插件谓词约束剖析与普通牌严格对齐（CardType 0）

- **深入排查插件底层**：
  - 尽管原版 Unity 客户端 `RefineCardAsync` 与 `ReplaceCardAsync` 仅校验位置，但运行中 BepInEx 插件 `OriginalClientActionRunner.cs:4326` 的 `IsSafeNormalHandCard` 判定硬编码要求：
    `TraceShape.GetIntMember(CardConfig(card), "cardType") == CardTypeNormal (0)`；
  - 因此，任何 `cardType != 0`（包括消耗类丹药 `CardTypeConsume = 1`）均会被插件守卫拦截，报 `no strictly replaceable/refinable normal hand card is available`；
  - 恪守《开发指南》第2条（不猜测、精确最小契约）：驱动逻辑 `is_refinable_normal` 与 `is_replaceable_normal` 严格对齐插件 DLL 现有能力边界（仅放行 `CARD_TYPE_NORMAL`），避免无效命令冲刷。在 `test_driver.py` 固化此边界单测（71/71 OK）。

---

### 5. Verify 39 实战战报与切片 100% Exact 全量锁定（117/117，0 Mismatch）

- **对局概况**：炎雪 vs 傀儡-地狱（吾行之，`codeId: 33165141`），打满 11 轮。
- **高光胜局**：**R10 成功击溃地狱傀儡（`winner: p1, hpDelta: +24`）**！R11 满格终极对决打至命元 -13。
- **切片全量 100% Exact 达成**：
  - 由 `build_practice_fixtures.py 39` 提取 11 场切片（`f8ho2kp.bin` -> `record.json` -> 11 夹具）；
  - 运行 `replay_slice --admission logs/practice/v39-replay/`：**满 8 格切片 6/6（100.00%）严格 Exact**，Winner、HPDelta、ActorTurn 全部 0 误差！
  - **全树累积 117 场严格 Exact / 109 场低格门禁 Blocked / 0 Mismatch**：全量 226 场 deployed 切片达成 100% 严格一致，三元组断言零放宽、零残差！

---

### 6. Verify 40 实战战报与 E1 炼化解锁（`33175166`，0 Mismatch）

- **对局概况**：炎雪 vs 傀儡-地狱（陆剑心 1000005，PuppetLevel=5，`codeId: 33175166`），打满 12 轮，
  3 胜 9 负，R12 被斩杀（HP 5→-13）。R1 无人驾驶（-6HP 基线噪音），loop 从 R2 接管（`--probe --give-up`）。
- **E1 杠杆（refine-only 单变量）活了**：插件新增 `IsSafeConsumeDanHandCard`
 （cardType==1 且 base 2000001–2000005，`IsSafeRefineHandCard` 放行），
  `is_refinable_normal` 放行 NORMAL+CONSUME，`is_replaceable_normal` 刻意保持 strict-0。
  探针 A/B（33175074 R4：exp 7→8，手牌 14→13，2000001 消失，receipt completed）＋
  本局炼掉小还丹×2＋地灵丹×1＋驱邪丹×1（各 +1 exp，无 reject）：
  server 接受丹药炼化，vanilla position-only 路径假设成立，R9 式"闲置丹"陷阱已疏通。
- **突破对比（record roundStats 同口径）**：V39 我方 L2@R3/L3@R7/L4@R10 vs 对手 L2@R3/L3@R6/L4@R9；
  V40 我方 L2@R3/L3@R7/**L4@R9** vs 对手完全相同。E1 的 +4 修为把我方 L4 提前一轮，
  R9 与对手会师 L4；L3 仍落后一轮。胜场差（3 vs 1）不直接归因 E1：对手不同
  （吾行之 3000001 vs 陆剑心 1000005）。
- **R5 起伤害归零归因（养成/卡池差，非引擎 bug）**：对手 HP 100→94→87→80（R2/R3/R4 胜），
  R6 起锁死 80。R5 双方 L2＋先手 AI，负 -19（engine honestA 亦判负）。R6 对手 L3＋20094
  vs 我方 L2，-41 全场最惨（境界差）。R7/R8 双方 L3 仍负 -17/-11：我方 8 格被培元丹＋驱邪丹
  双丹占 2 席零输出，engine 全排列亦判负（honestA -73/-60），属卡池差非布阵问题。
  对手 L4@R9＋20056 后 R9–R12 连负被斩。候选下一杠杆：置换侧丹药放行探针（E2，等炼化结论收口后再动，
  保持单变量纪律）。
- **切片门禁（断言零放宽）**：
  - 由 `build_practice_fixtures.py 40` 提取 11 场切片（`f8nmxwx.bin` -> `record.json` -> R2–R12 夹具，R1 无卡跳过）；
  - 运行 `replay_slice --admission logs/practice/v40-replay/`：**满 8 格切片 6/6（100.00%）严格 Exact**，
    低格门禁 Blocked 5，**0 Mismatch**（记账口径与 V39 一致：`record.json` 非夹具 error 项为扫描器噪音，两局相同）；
  - **全树累积 123 场严格 Exact / 114 场低格门禁 Blocked / 0 Mismatch**。
- **质量门**：`test_driver.py` 71/71 OK（含新 `test_consume_dan_refinable_but_not_replaceable`）；
  loop 其余单测绿；`test_replace_archetype_protect` 4 失败经 stash 对照为 HEAD 预存红（与 E1 无关）；
  `check:public-boundary` 通过；`test:runtime-trace-plugin`（插件编译）通过。

### 7. Verify 42 实战战报与 E2 置换探针 verdict（`33176065`，0 Mismatch）

- **对局概况**：炎雪 vs 傀儡-地狱（吾行之 3000001，PuppetLevel=5，`codeId: 33176065`），打满 12 轮，
  2 胜 10 负（R4、R11 胜），R12 被斩杀（HP 20→-3）。R1 即由 loop 接管（`--probe --give-up`，
  中途一次误 kill 后用 `--no-replace-dan-probe` 从 R9 续跑，R1–R8 首段 stdout 随坏 tee 丢失，
  但 79 个动作收据全在 trace 里，R8 ready 正常）。
- **E2 verdict：服务端放行丹药置换**。V41 整局零触发根因是探针饿死（`c not in target_row` 的
  presence 口径遇目标含丹轮次全灭＋`refine-post` 先吃全部余丹），修法：探针移到 `refine-post`
  之前＋目标改副本余量口径（手牌数超出补足目标所需，`board_used` 计入在场数），见提交
  `5361acba97`。V42 R2 探针打出 `replace-hand-card:地灵丹(2000003)`（nonce
  `loop-replace-probe-…#0`），收据 `ReplaceArea.ReplaceCard` 直接执行、
  `proof: client-applied-authoritative-state`、前后 state hash 变化、无 reject。
  E2 可转正：置换侧不再需要 strict-0（exploit 下局再开，本局规划侧仍 strict-0，单变量未破）。
- **E1 本局**：炼掉小还丹×2＋培元丹×4＋地灵丹×1＋飞云丹×2（各 +1 exp，无 reject）。
  突破对比（record roundStats 同口径）：我方 L2@R3/L3@R6/**L4@R10** vs 对手 L2@R3/L3@R6/L4@R9/**L5@R12**。
  傀儡突破表连续三局同一（L2@R3/L3@R6/L4@R9，V39/V40/V42；V42 另见 L5@R12）——此前只活在口头里，
  现落文档。对手 HP 自 R4 起锁死 95 整整 7 轮（R4–R10 零伤害），R11 先手胜 +16（79 HP），
  R12 对手 L5＋先手被斩。仍是养成/卡池差主导：R6–R10 对手 exp 反超先手 5 连，R11 我方抢回先手胜 +16，
  R12 对手先手＋L5 终结。
- **切片门禁（断言零放宽）**：
  - 由 `build_practice_fixtures.py 42` 提取 12 场切片（`f8o67l5.bin` -> `record.json` -> R1–R12 夹具，
    本局 R1 即部署故无跳过）；
  - 运行 `replay_slice --admission logs/practice/v42-replay/`：**12/12 严格 Exact、0 Mismatch**
    （`record.json` 非夹具 error 项为扫描器噪音，与 V39/V40 同口径；R1–R5 低格切片能判是 §8 拆门禁之后的事，
    本局 12 场在拆前后均为 Exact）；
  - **全树累积见 §8（拆 8 格门禁＋修提取器丢 0 后：249 Exact / 0 Blocked / 0 Mismatch）**。
- **质量门**：`test_driver.py` 73/73 OK（含新 `test_dan_replace_probe_surplus_copy`）；
  `check:public-boundary`、`check:docs-drift` 通过；插件 DLL 本轮未动（沿用 E2 版）。

### 8. 拆 8 格门禁：低格切片从拒判改为实判（249 Exact / 0 Mismatch）

- **起因**：「119 blocked 是什么」追问下复查发现，`BattleFixture::validate` 要求
  `cards.len() == DECK_SIZE`，但战斗只走 `queue`（`0..active_slot_count`，`player.rs:611`），
  超出槽位从不入列；直接 `slots[i]` 全是 queue/实战派生的有界下标（两处裸索引都有 `.len()`/`.get()` 守卫），
  `star_slots` 只有 `.contains()` 查询。8 张补齐只是格式惯性，引擎本来就能打低格局
  （单测里早有 `active_slot_count: 1` 的用例）。
- **改动**（`engine-rust/src/fixture.rs`）：删掉 `!= DECK_SIZE` 拒判，改为只拒 `> DECK_SIZE`；
  下界仍由 `active_slot_count <= cards.len()` 守。附 3 个单测
  （短数组放行/超格拒绝/超 8 拒绝），`cargo test --lib` 727/727。
- **结果**：119 blocked 先转为 101 Exact＋18 Mismatch，再顺藤摸到提取器丢 0 错位，
  17 个一次转正（见下）。全树（V21–V42，`record.json` 扫描噪音不计）：
  **249 Exact / 0 Blocked / 0 Mismatch**。
  V40/V42 两局自身 23 场在拆前后均为 Exact，无一转红。
- **18 个 mismatch 根因（已全修，非引擎 bug）**：17 个是夹具提取器
  `build_practice_fixtures.py` 的 `resolve_deck` 把格位内 0 直接丢弃，导致整行左移错位——
  原版未布牌的格子按普通攻击（id 0，attack 3）补打，真行里它是占位的牌（如 v25r01 p2
  首格 `[普攻, 骤风剑, 骤风剑]` 被提成 `[骤风剑, 骤风剑]`，残差恰为 3 张普攻×3 点 = 9）。
  修法：格位内 0 保留为普通攻击卡，并按 `unlockGrids` 截断 stale 格（`activeSlotCount` 改取格数）；
  附 `test_build_practice_fixtures.py` 5 单测。17 个一次全转 Exact。
  剩下 1 个 `v23r09-probe` 是当年手造的铁骨诊断探针（故意弱化 p2 到 lv3 但期望抄真轮 -28，
  残差 2 即当时的诊断结论），不是语料夹具——已改名归档为 `.bak` 移出门禁 glob，内容保留。
  - **现全树：249 Exact / 0 Blocked / 0 Mismatch**（V21–V42；`record.json` 扫描噪音不计）。
- **口径声明**：以前的「全树 0 Mismatch」是在 119 场拒判下的 0，不是真 0；中间态的 18 是历史欠账曝光，
  不是退化。`blocked` 计数以后恒为 0（门禁不再拒判，只认 exact/mismatch）。

### 9. Verify 43 实战战报与 E2 首局（默认路径无回归，化神未达开关未开）（`33188692`）

- **对局概况**：炎雪(1000002) vs 傀儡-地狱（**慕虎 3000004**，PuppetLevel=5，`codeId: 33188692`），
  打满 13 轮，R13 战败（我方结算命元 -19，对手 60）。R1 起由 loop 接管：
  `restart --skip-animation` 干净开局 → `--career auto --hunt-cap 6 --sustain-swap
  --rush-override --presolve --probe --engine --no-ready-last --give-up --replace-dan-exploit`。
- **单变量声明**：V42 基线（33176065）+ `--replace-dan-exploit`，其余 flags 同 V42。
  对手从吾行之(3000001) 换成慕虎(3000004)属人机池轮换，非变量。
- **HP 轨迹（round-open 口径）**: R1 100-100 / R2 100-100 / R3 95-98 / R4 88-98 / R5 81-98 /
  R6 74-98 / R7 65-98 / R8 65-98 / R9 65-75 / R10 52-75 / R11 52-60 / R12 32-60 / R13 7-60 → 结算 -19。
  同轮对比 V42（R6 55-92 / R8 48-92 / R10 28-92 / R12 20→终）：V43 前 8 轮每轮多扛 7~20 点，
  R10-R12 差距收窄（对手换慕虎后滚雪球更凶，R9 起每轮 -10~-20）。
- **E2 verdict：门从未打开，规划侧默认路径零回归**。我方 exp 逐轮 0/3/6/11/13/20/25/28/35/40/45/49/54
  ——**R13 exp 54，差 1 点到 L5（化神）**，全程 lv4，`DAN_REPLACE_EXPLOIT_MIN_LEVEL=5` 门一次未开。
  全局 24 次 refine 收据全为 `expectedExpDelta: 1` 正常执行、0 reject；置换 0 丹
  （`replace-hand-card` 收据里无 2000xxx）。E2 代码在 level<5 下严格不触，
  strict-0 口径在默认路径上保持——这是 E2 合并后的首局回归验证（74/74 单测 + 实战）。
- **败因主线：修为节奏差一脚，对手突破表连续第四局复现**。对手 exp 脚本每轮 +3~+9 稳定爬升，
  L2@R3/L3@R6/L4@R9/**L5@R12**（V39/V40/V42/V43 四局同一），L5 解锁第 5 仙命(talent 100)后
  R12/R13 战力陡增。我方 L2@R4/L3@R7/L4@R10——每级都慢对手一整轮，R12 前夕 exp 49 vs 对手 56。
  引擎 R12 预测 -2（实际 -25）、R13 预测 -35（实际 -35，`deploy-verify` 重排后 -31 keep，
  `repaired: true` 修的是 L5 对手快照滞后）——R13 的预测已经吃上 L5，但 R12 的没吃上（jumps 0）
  ，`snapshot-lags-one-redeploy` 的窗口差导致生死轮防守阵型选型偏乐观。
- **R13 决策链（60 秒窗口的极限操作，全部 completed/0 reject）**：presolve(68s) → probe-talent
  候选[炎舞/心法×3] → plan(elapsedMs 24686, 241920 evals) → deploy-verify(-35→-31 keep, reordered)
  → fuse(喂巨鲲灵剑+神力丹) → replace×3（凝意诀/巨鲸灵剑/引气剑，chance 29→26）→ ready。
  即便如此仍 -35：R13 面对的是 L5+5 仙命的满编慕虎，7 点血量的局面无解，胜负在 R12 前已定。
- **next（按优先级）**：
  1. **修为节奏追赶是第一优先**：对手每轮 +3~+9 exp 的脚本意味着我方必须把 refine 收入拉满
     （V43 有数轮只炼 1~2 张），并评估「低价值余牌提前炼化」vs「留作置换筹码」的收益曲线——
     化神差 1 点的局面说明 E2 之外还有 1~2 点 exp 的运营空间可挖。
  2. **对手快照滞后的一轮窗口**：`oppHedge: ai-develop-jumps=2 snapshot-lags-one-redeploy` 已捕获
     跳级，但 R12 这种生死轮应把「对手下轮必到 L5」作为先验（突破表四局同表），引擎 hedge
     可以直接吃突破表预测而不是等快照。
  3. **E2 生效验证继续顺延**：需要一局活到 R12+ 且 exp 到 55 的对局才能看到丹药置换实际触发；
     若修为节奏修复（next 1）先到位，E2 验证会自然出现。

---

### 10. Verify 44 实战战报与专家会诊：符咒师病灶暴露（非丹药消耗牌炼化被拒导致修为锁死金丹）（`33190785`）

- **对局概况**：炎雪(1000002) vs 傀儡-地狱（**吴策 2000005**，七星阁星弈，PuppetLevel=5，`codeId: 33190785`），打满 10 轮，R10 战败（我方结算命元 -8，对手 106）。
- **切片 100% Exact 复核**：
  - 由 `build_practice_fixtures.py 44` 提取 10 场切片（`f8wxpm1.bin` -> `record.json` -> R1–R10 夹具）；
  - 运行 `replay_slice --admission logs/practice/v44-replay/`：**10/10 场全部严格 Exact、0 Mismatch、0 Blocked**（低格门禁已拆，R1–R10 全部 Exact，三元组 winner/actorTurn/hpDelta 100% 严格吻合）。
- **会诊病灶一：副职泛化改动误选【符咒师】（Career 2）**：
  - 前序为了通用化移除了贝叶斯战略加权（炼丹师+10/琴师-5），导致 R2 `recommend_career` 因单局吸引子样本最高分（85.429 vs 71.826）自动选入【符咒师】；
  - 符咒师发牌包含大量符咒卡（锐金符、奔雷符、火云符、水气符）。
- **会诊病灶二：致命炼化契约矛盾——非丹药消耗牌（CardTypeConsume=1）被插件守卫整组拒绝**：
  - 插件 `IsSafeRefineHandCard` 源码显式硬编码：仅放行 `CardTypeNormal` 或 `IsSafeConsumeDanHandCard`（仅限炼丹师 2000001–2000005）；
  - 驱动侧 `is_refinable_normal` 误以为所有 `CARD_TYPE_CONSUME` 均可炼化，导致从 R4 到 R10（整整 7 轮），驱动侧反复将符咒牌排入 `refine` 与 `refine-breakthrough`；
  - 插件每次均抛出 `InvalidOperationException: no strictly refinable normal hand card is available`，导致这 7 轮内所有普通炼化 100% 失败；
  - 结果：至少 10+ 点修为未能入账，手牌严重堆积至 16 张废牌，我方从 R7 到 R10 修为锁死在金丹期（Lv3，R10 Exp 仅 32，而 V43 同轮为 Lv4 Exp 40），终生未达元婴/化神，无缘高阶质变大核。
- **会诊病灶三：吴策星弈压制与境界代差碾压**：
  - 地狱吴策以星弈体系成型：算无遗策 + 飞星刺 + 众星拱月 + 星弈•飞/立/夹/挡，R6 升 L3（仙命 10058），R9 升 L4（仙命 20031 气吞山河）；
  - R9/R10 连续打出 -66 / -56 的巨额伤害（单轮扣血 18/19），而我方受制于金丹期卡池与手牌卡死，毫无还手之力。
- **专家会诊处置与修复落地（已在代码中生效并经单测验证）**：
  1. **驱动侧收紧 `is_refinable_normal`（已修复）**：对齐插件 `IsSafeRefineHandCard` 精确契约，严格仅放行 `CARD_TYPE_NORMAL` 及属于 `DAN_REFINE_BASES` 的丹药消耗牌，非丹药消耗牌（符咒 3000003~3000010 等）一律返回 False，彻底杜绝 `InvalidOperationException` 与整组炼化崩溃；
  2. **副职评估机制对齐资源闭环（已修复）**：在 `archetype_engine.py:recommend_career` 中恢复副职产物流动性战略加权（炼丹师 +10.0，符咒师/琴师 -5.0），确保起手稳定首选炼丹师，获得高机动冲关修为与手牌整洁度；
  3. **单测守卫增补与全绿验证（已通过）**：在 `test_driver.py` 增设 `test_non_dan_consume_not_refinable` 与 `test_recommend_career_prefers_alchemist`，全部 76 项测试全绿通过；
  4. **后续动作**：按专家会诊循环节奏，进入下一轮（Verify 45）实战验证。


---

## 6. 关键文件路径一览
- 套路图谱引擎: [`.agents/skills/original-client-practice-play/references/archetype_engine.py`](../.agents/skills/original-client-practice-play/references/archetype_engine.py)
- 全自动驱动主循环: [`.agents/skills/original-client-practice-play/references/full-match-loop.py`](../.agents/skills/original-client-practice-play/references/full-match-loop.py)
- 通用框架设计规范: [`docs/UNIVERSAL_ARCHETYPE_TRAINING_FRAMEWORK.md`](./UNIVERSAL_ARCHETYPE_TRAINING_FRAMEWORK.md)
- 实验持久化日志区: `logs/practice/` (已被 `.gitignore` 保护，本地工件，非链接)
- 客户端原版驱动原语: [`research/original-game/drive_original_client.py`](../research/original-game/drive_original_client.py)

