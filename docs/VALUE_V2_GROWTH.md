<!-- topic: value-function -->

# Growth Value v2（打牌模型 Phase 1：数据与设计）

目标：给**跨轮成长决策**（炼化/置换/合成/突破时机与牌组方向）学一个价值函数，
对候选成长动作排序。**不管三件事**：单轮布阵行（Rust 穷举已接近最优）、战斗
规则（canonical 不动）、`winner/actorTurn/hpDelta` exact 断言（模型只做选择
排序，与 solver 同权）。

## 为什么是成长决策，不是布阵

- Track E 结论：value-v0 在单轮排序上打不过 hpDelta，布阵没有 headroom。
- 成长动作现在全是 v1–v11 手写规则（11 次迭代），headroom 全在这里。
- 神经网络端到端不在考虑内：60 个样本训不动；正确形态是小参数拟合价值
  （value-v1 的扰动拟合传统），不是策略网络。

## 数据

`analysis/value/growth-dataset.ts` 从 full-match-loop JSONL 日志抽取逐轮记录：
状态特征（命元/修为/境界/手牌数/仙命/道韵/副职）+ 各组成长动作
（planned/completed）+ 引擎读数（baseline/best/first/minimax honest-stale）+
标签（roundNet 由相邻 round-open 算出，matchWin 由 match-end 算出）。

- fixture 语料**不适用**：单轮战斗快照，无成长上下文（炼化/置换/合成历史、
  牌组演化都没有）。
- 现状：m22–m26 共 60 轮、55 个带标签 roundNet（`bun
  analysis/value/growth-dataset.ts --log <match.log> --code <codeId>`，
  原始日志不入库，按 scratch 先例只保留抽取命令）。
- 基线信号（描述性，n=5）：胜局成长动作率 5.93/轮，高于四个负局
  （4.82–5.38）；**honestA 符号 vs 实际 roundNet 方向 35/43（81%）**——
  minimax honest 是核心价值特征，v2 锚定它。

## 拟合与评估协议

- 模型类：~6 特征 logistic/线性小模型，value-v1 式保守收缩；15–20 局
  （约 200 轮）之前不拟合权重，只积累数据，规则保持 v11。
- 离线反事实：v2 排序会不会改变实录中的拾取（m23/m25 的 23 vs 20013 类），
  改变后按引擎 honest 重估 roundNet；m20/m24/m25 翻不翻是硬指标。
- 实打 A/B：同配置对照，判据沿用整局胜负；`analysis/tests/growth-dataset.test.ts`
  锁抽取口径（2 pass）。
- RL 自对弈：明确推迟——不存在多轮成长仿真器（GA self-play 是单轮战斗），
  造它比模型本身贵一个数量级。

## 缺失项拟合状态（2026-09-11）

- offering 分布（突破/道韵候选）：已拟合。`analysis/value/offering-prior.ts`
  从 loop 收据抽候选集（7 局：21 次突破 offering、8 次道韵 offering），按
  拾取时境界/轮次分桶。初步读数：道韵 r4 的 27 候选 7/7 出现（客户端默认首位），
  套餐牌稀有（水月 2/7）；突破拾取时境界常比突破后境界低一档（成长循环内先拾取
  后突破，m24 的 20059 即此路径）。
- 抽牌分布（手牌/置换）：牌堆模型已定（`research/original-game/PUPPET_DIFFICULTY.md`
  牌库规则节）：8/6/4 副本查表 + 5 首抽/2 门派 1 副职 + 置换池内同名 -2 +
  炼化/合成/获得不碰池，自采 8 局 93 轮拟合相容。N(t) 是查表项不是拟合项。
- AI 适应：不拟合，冻结快照是方法学选择（与线上 minimax 同 caveat）。

## 离线拟合原作可行性核准（2026-09-11，结论：可行，附条件）

`trace-match.ts` 自述三条极限（AI 冻结/抽牌随机无牌堆模型/offering 随机）中，
第二条已被牌堆模型 retire，第一、三条有收敛路径。9 局 100 轮家底
（69 炼化/304 置换，全部默认简单档=已标注）：

- 反事实状态可算：成长记账全确定（炼化 +1exp/-1 手牌、置换 -1 机会/池抽、
  突破阈值读 `LevelConfig`、喂牌/合成确定，牌堆查表 + 蒙特卡洛补方差）。
- 反事实战斗可打：Rust canonical 引擎 exact，无需客户端。
- 反事实拾取有界：仙命/道韵候选收据里有原样列表（`probe-talent-candidates`），
  备选集内反事实是精确的，不用猜池子。
- 标签现成：roundNet/matchWin 口径已锁（growth-dataset 2 pass）。
- 对手侧：witness 每轮给对手 exp/境界/仙命/手牌/used（`BattleLog.json` 另有
  双方 usedCards 名字/等级/稀有度），对手轨迹按实录 replay——适应性缺口与
  minimax 同 caveat，不扩大。
- 难度标注：v2 补丁后语料自带档位（`pluginLoaded.puppetLevel` +
  `originalClientPuppetLevel` 收敛事件）；历史 9 局一律默认简单，追标即可。

附条件（缺一即退回“不可行”）：
1. 每局归档 `CardOperationLog.json`（开局截断，restart 前收尸）——精确抽牌流
   消掉蒙特卡洛方差，这是离线 rollout 从“近似”到“精确”的唯一一步。
2. offering 先验继续累积（收据零成本，每局 +3 突破/+1 道韵 offering）。
3. 反事实评估只许用“已拥有牌 + 实录候选集”，禁向池外脑补（红线不变）。
4.  live 客户端只保留 A/B 确认位，不再进拟合环。

## 红线

模型输出永不进入战斗结算输入；公开导出不含本目录新增以外的任何语料；
回放断言不放宽。
