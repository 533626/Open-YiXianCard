<!-- topic: product-architecture -->

# Open-YiXianCard 产品契约与目标架构

> 本文是 Open-YiXianCard 的产品需求、目标架构与 V1 验收唯一真相源。
> 它只记录稳定边界，不记录会随运行变化的 build ID、覆盖数量、算法权重或阶段性读数。
> 当前工程基线见 `docs/AGENT_CONTEXT.md`，具体施工与门禁见
> `docs/CROSS_LINE_RUNBOOK.md` 及各主题文档。

## 1. 产品定位

Open-YiXianCard 是一个证据驱动、在用户本地运行的弈仙牌单场战斗模拟与浏览器展示工具。
它面向三类连续任务：

1. 在当前已准入的原作 build 范围内，精确复现影响单场胜负的战斗过程。
2. 允许用户自由构筑或显式导入本地数据，并查看原作内部状态、触发链、事件和局势变化。

Solver、GA 与 value 研究属于私有工程 companion，不是公开产品或公开发布物的组成部分。

“完整模拟原作战斗”不等于复刻完整游戏客户端。产品接收已经确定的战斗初始状态，从战斗开始
模拟至终局；商店、抽换牌、炼化、成长、战前资源获取、匹配、排名和战后奖励不在规则内核中重演。

## 2. 不可混淆的两种使用范围

### 原作合法模式

- 只允许当前准入快照确认的规则、卡牌、仙命、状态与组合约束。
- 输入必须能说明所属 build、来源和适用范围；缺失证据或能力时明确阻断，不以近似行为补位。
- `winner`、`actorTurn`、`hpDelta` 等精确回放契约不得为 UI、求解结果或性能目标让步。
- 只有该模式的结果可以表述为“原作合法范围内”的复现、比较或推荐。

### 研究沙盒

- 用于非法组合、未准入能力、合成压力输入、实验规则和未来机制探索。
- UI 与导出结果必须持续显示“研究沙盒”，并列出越过了哪些原作合法约束。
- 沙盒结果不得混入原作合法排行榜、value 基线、推荐结论或覆盖率。
- 研究输入只有经过原作客户端证据、精确契约与正式准入后，才可转入原作合法范围。

## 3. 单场战斗边界

规则内核负责：

```text
[Resolved initial battle state]
  -> battle-start hooks
  -> turns / card actions / effects
  -> damage / healing / buffs / resources / death
  -> terminal result and exact trace
```

规则内核不负责：

```text
shop / draw / exchange / refine / growth / matchmaking
  -> record or import final state only
post-battle rewards / rank / account progression
  -> outside product simulation scope
```

若战斗外机制已经改变了本场初始状态，导入层可以记录其最终结果与来源，但不能在战斗规则内核中
补造该过程。Analysis 可以研究“什么构筑更好”，不能借此把战前生成机制伪装成已经还原的原作规则。

## 4. 数据、隐私与公开站点边界

- Web 产品是静态应用，战斗模拟、曲线计算和 V1 诊断在用户浏览器本地完成。
- 产品不要求账号，不提供云存档，不把构筑、回放或分析输入上传到项目服务器。
- 回放或构筑数据只能由用户通过浏览器文件选择等明确动作从本地导入；不自动扫描用户磁盘。
- V1 支持原作 `RecentBattleInfo .bin` 与项目定义的版本化 JSON。`.bin` 只在浏览器 Worker
  中解码，用户必须显式选择文件或目录；解出的原版结果保持“本地输入未认证”，不能冒充准入证据。
- 用户可以从空白构筑开始，不以预置对局代替真实输入。
- 公开网站绝不提供、选择、下载或打包任何精选、脱敏或示例 fixture，也不内置对局 fixture
  索引，不把工程回放语料复制进 Web 发布物。规则代码和必要的已准入卡牌元数据不属于对局
  fixture 或对局 fixture 索引。
- 规则验收 fixture、回放 admission receipts、corpus manifests 和 attestation 是私有工程证据，不是公开站点
  的 demo 数据源；发布门禁必须阻止它们及其索引进入静态网站产物。
- 玩家回放派生的工程 corpus、replay-derived reports、analysis archives 与 ratatui/crossterm TUI
  不随网站或公开源码发布，也不适用项目 MIT License。Scheme A 当前树只保留 Rust canonical engine、
  TS compatibility archive、browser UI、公开 evaluator contracts 和规则开发文档。

Scheme A 的当前 checkout 已物理移除上述私有路径；旧 Git 历史仍可能包含它们，因此历史发布面仍需
单独的 fresh-history export、隐私审计和 provenance review。`PRIVATE_ENGINEERING_EXTRACTION.json` 与
`bun run extract:private -- <directory> <backup-root>` 保留可逆映射；私有备份位置不写入公开文档；由 `PRIVATE_ENGINEERING_EXTRACTION.json` 的显式 source-root 映射恢复。网站部署、Cloudflare、GitHub
release 和 remote history rewrite 均未执行；当前 CI 只生成并审计本地静态 artifact，不宣称已上线。

## 5. 事实曲线与分析曲线

两类曲线可以在同一时间轴上对照，但必须分层展示、分别命名、分别说明来源。

| 类型 | 来源 | 可以表达 | 不可以表达 |
| --- | --- | --- | --- |
| 事实曲线 | Engine 事件、状态帧和终局结果 | HP、资源、buff、行动、伤害等实际变化 | 未发生事件的推测价值 |
| 分析曲线 | Analysis 对同一事实轨迹的派生计算 | 局势估值、胜负倾向、替代动作收益、不确定性 | 冒充原作内部数值或改写事实轨迹 |

UI 不得自行复制战斗规则来计算事实曲线。分析曲线必须带分析方法或 profile 标识、适用范围、
截断/抽样状态和准入快照；算法变化后旧分析结果应标记过期或重算，事实轨迹本身保持不变。

## 6. 可解释诊断契约

Solver、GA 和 value 层只消费已准入的规则能力。一个可发布的诊断至少回答：

- 比较了什么初始状态、构筑、牌序、对手范围和先后手假设。
- 基线是什么，建议改变了什么，结果差异是什么。
- 哪些战斗事件、状态变化或指标支持该建议。
- 搜索是否完整，是否发生预算截断、抽样或启发式近似。
- 使用了哪个规则准入快照、评估后端、分析 profile、seed 与约束。
- 哪些结论不能外推，哪些输入属于研究沙盒。

单一总分、无来源的“强度”、只给最优解而不给约束与证据，都不满足产品诊断要求。

## 7. 目标架构

```text
[Original client and current build]
                |
                v
[Evidence extraction and admission]
  research/original-game/
                |
                v
[Versioned scope + contracts + fixtures]
  battle-evaluator/
                |
       +--------+---------+
       |                  |
       v                  v
[Rust canonical engine: native + WASM]
                |
                v
[Evaluator adapters and canonical battle telemetry]
       |
       v
[Fact trace / fact curves]
       |
       v
[Browser UI: free build + explicit local import + diagnosis]

[Private engineering companion]
  analysis / solver reports / replay corpus / ratatui TUI

[GitHub source] -> [local gates and static build]
  (website publication and Cloudflare deployment deferred)
```

依赖方向必须保持：

1. 原作证据与准入决定规则范围；私有 Analysis 不能反向成为规则证据。
2. Rust Engine 是唯一可变规则实现；TS 兼容档案跟随 Rust 移植，只保留兼容与迁移类型，不参与产品执行。
3. Battle Evaluator 提供中立契约、适配器和统一 telemetry，避免 UI 各自解释结果。
4. UI 只编排输入并展示事实结果，不在 DOM 层复制战斗规则。
5. 共享 schema 必须版本化；输入、报告与发布物能识别不兼容版本并明确失败。
6. 本地构建只绑定一个完整规则/证据快照，不静默混用不同 build 的证据；公开发布仍需单独审批。

### 7.1 Rust canonical kernel 内部架构

Rust kernel 的理想形态不是第二套通用游戏框架，而是围绕原版战斗顺序建立的单状态机：
`ReplayFixture + DecisionTape -> validate/initialize -> Canonical ReplayState ->
turn lifecycle -> PreparedCardTransaction -> ordered effect repetitions ->
CompletedCardTransaction`，出牌事务收敛到 semantic mutation kernels（HP / defense / anima /
status / …），并只投影出一条 observation 流（summary / parity events / detailed trace）。

内部边界必须满足：

1. `ReplayState` 是唯一战斗真相；不保留能独立演进的第二套 public battle state。
2. 所有 replay 输出共享一条执行管线，只通过 observation mode 决定采集 summary、精确 parity
   event 或详细 trace，不能各自重演规则。唯一的例外是 TurnStart checkpoint 的采样时点本身属于
   规则语义：parity 在回合开始钩子之后采样，以对齐原版客户端的伤后/回血 checkpoint；event 与
   detailed 流在钩子之前采样以暴露 pre-hook 帧。只有 parity 流参与 exact 比对，两者不可互换。
3. fixture、decision、rule coverage 和内部 invariant 失败使用结构化 `BattleError`；CLI 文案只是
   投影，调用方不得靠解析字符串分类。
4. 单次出牌按 `preflight -> prepare -> ordered effects -> finish` 的类型化 transaction 推进；
   cost、临时牌、重复执行、牌后钩子、死亡和再动不能散落成可任意跳过的布尔分支。
5. 卡牌主体由显式有序的 handler-family registry 调度。registry 顺序是语义：少量重叠 ID 会由
   早期 handler 明确返回 `None`，再落到后续权威实现。静态生成的
   `rule-handler-dependencies.json` 负责依赖与重叠审计，不得生成会把一个 ID 强行绑定到单一路由
   的运行时表。
6. 新 handler 逐步改为只接收与该机制有关的 effect context；共享资源写入只经 semantic kernel。
   在迁移完成前，可按牌族或 fallback 纵向切片，禁止为“统一接口”一次性开放整个
   `ReplayState` 的公共可变访问。
7. telemetry 是执行结果的只读投影，不是驱动规则的 event bus；当前项目不引入 ECS、全局消息总线
   或可重排 middleware，以免原版调用顺序变成隐式行为。

这套边界已经落到 canonical state、统一 observation、结构化错误、类型化 card transaction、
有序 handler registry 和 printed-fallback effect context。后续规则开发应沿这些 seam 收紧访问，
不再新增平行执行入口；架构迁移本身仍须由现有 exact fixture 与 canonical replay 门禁证明零语义漂移。

## 8. 原作 build 更新闭环

```text
[Detect new build]
  -> freeze identity and evidence inputs
  -> diff configs / code / runtime behavior
  -> quarantine affected capabilities
  -> add evidence and exact contracts
  -> restore Rust exactness
  -> invalidate and recompute affected analysis
  -> rebuild UI metadata and static assets
  -> run local build and release-audit gates
  -> defer GitHub source publication and Cloudflare deployment to a separately approved export
```

新 build 未完成准入时，公开版本继续指向上一个完整准入快照，并清楚显示它不是最新原作 build；
不得把局部通过的能力拼成“已更新”。每次发布必须能追溯到源码提交、准入快照、schema 版本和门禁结果，
并能回退到上一份完整静态产物。

当前已实现从 evidence manifest 变化开始的内容指纹失效、门禁、不可变静态产物与可追溯发布；
Steam 新 build 的自动发现、证据提取和受影响能力隔离仍需 build watcher，不能把现有 push/release CI
表述为已经自动跟踪原作更新。

## 9. 当前差距与目标收口

下表描述当前实现相对目标产品的结构性差距，不是完成声明。精确实现状态仍以代码、门禁和
`docs/AGENT_CONTEXT.md` 为准。

| 当前短板或风险 | 风险 | V1 目标收口 |
| --- | --- | --- |
| 当前树与旧历史的隐私边界不同 | 当前 checkout 已移除 corpus、replay-derived reports 和 TUI，但旧 Git history 仍可能包含它们 | 在任何公开 push 前从保留的私有 ref 做 fresh-history export，完成路径、payload、identifier 和 license 审计；当前不改历史 |
| 可信 replay ledger resolver 尚未覆盖所有消费面 | GA 已改为完全自对弈，不再是 replay 消费者；object-only 输入仍可自报 build、eligibility 与 expected，其他消费者若直接信任仍会污染正式报告 | 所有仍消费 replay 的正式工具统一复验 ledger、内容指纹和 exact 三元组；结构完整但未认证的输入保持 `constrained` |
| Analysis 准入尚未覆盖所有生产报告 | GA fitness 已与 replay 对手解耦；value 等报告尚未全部接线 | 正式报告逐一消费 admission decision；reachability hard-invalid 与 value 样本共用同一 profile |
| Worker 只有等待态，没有增量进度 | 战斗与四类 Solver 已移出主线程，支持预算、取消、崩溃重建和陈旧结果隔离；长搜索仍看不到实时 evaluated count | 后续增加节流 progress 协议；V1 只承诺运行/完成/失败/取消状态，不伪造进度 |
| 原版缓存不能被托管站自动发现 | 版本化 JSON 与原作 `RecentBattleInfo .bin` 均可本地导入，战绩码可在用户授权目录中匹配；浏览器仍不能静默扫描 Windows / Linux 磁盘 | 保持显式文件/目录选择、Worker 本地解码和未认证标记；用详细双平台路径与 AI 助手指引降低首次定位成本 |
| UI 的轻量 Value 排名解释仍在渲染侧分类 | 战斗、fixture exact 对比和 Solver 重计算均已进 Worker，但少量 Analysis 展示分类仍在主线程 | Worker 最终返回诊断 view model；DOM 层只格式化，不拥有分析语义 |
| 本地发布审计已建立，但线上发布仍延期 | 静态 `dist`、零 fixture、零网络和内容哈希审计可在 CI 运行；没有 Cloudflare deploy、GitHub release 或 Steam build watcher | 另行审批历史 export、线上发布与只读 build watcher；准入未闭环时保持本地/上一完整快照 |

## 10. V1 验收

| 能力 | V1 通过标准 |
| --- | --- |
| 品牌与入口 | GitHub 根 README、产品架构和开发入口统一使用 Open-YiXianCard |
| 范围可见 | UI 能区分原作合法与研究沙盒，缺失准入能力时不静默近似 |
| 自由构筑 | 用户能在已准入范围内建立双方战斗初始状态并运行单场战斗 |
| 本地导入 | 只有用户显式选择受支持的本地回放/构筑文件后才解析；支持原作 `RecentBattleInfo .bin` 与版本化 JSON；无账号、无云上传 |
| 战斗复现 | 当前发布快照通过 Rust canonical 精确回放与 UI/WASM 使用面门禁 |
| 内部细节 | UI 展示规范化事件、关键状态变化、触发来源和终局诊断入口 |
| 曲线分层 | 事实曲线来自 canonical telemetry；分析曲线有独立标签、profile 和局限说明 |
| 诊断求解 | 至少提供基线、候选变化、结果差异、关键证据、搜索完整性和可复现参数 |
| Analysis 范围 | 卡池、仙命、对手与 value 样本由同一准入快照筛选，沙盒数据不混入合法结论 |
| 网站发布 | 静态产物在浏览器本地计算，零内置对局 fixture 及其索引 |
| 更新维护 | build 更新闭环可重复执行，受影响分析会失效重算，发布物可追溯和回退 |

V1 优先建立这些边界、契约和一条完整用户路径，不要求一次完成所有高级可视化、所有搜索策略或
所有战斗外体验。

## 11. 明确延期项与发布政策

V1 延期：

- 商店、抽换牌、炼化、成长、匹配、排名和战后奖励的过程模拟。
- 账号、云同步、跨设备历史、服务器求解和社区共享构筑。
- 同时运行多个原作 build、跨 build 回放转换和自动兼容旧输入。
- 通过公共站点代理请求原版服务器战绩接口；V1 战绩码只匹配用户明确授权的本机缓存。
- 从用户数据训练推荐模型，或收集任何使用遥测。
- 完整移动端体验、原生桌面安装包和多人协作分析。

已确定的 V1 发布政策：

- 项目自有代码和文档使用 MIT License；第三方权利与非隶属关系见根目录 `NOTICE`。
- 玩家回放派生 corpus 不公开、不随 MIT 再授权，也不提供精选、脱敏、示例或 demo fixture。
- 当前 checkout 是 Scheme A 的 staged public tree，但旧 Git history 仍含待 export 审计的私有路径；本次不 push、不发布源码、不发布网站，也不部署 Cloudflare。
- 公开发布前必须从保留的私有 ref 生成 fresh history export，并把 corpus、replay-derived reports、analysis、TUI/ratatui 从整个历史与发布面拆分，再经过单独隐私审计。
