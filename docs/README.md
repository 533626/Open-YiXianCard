# Open-YiXianCard 文档入口

Open-YiXianCard 是证据驱动的单场战斗模拟与浏览器展示项目。公开工程由原版研究、共享评估契约、
Rust canonical 规则核和浏览器 UI 组成（原 engine-ts/ 已移出仓库，Rust 是唯一实现）。Analysis 与回放 corpus
是私有 engineering companion，不属于公开构建面；边界与可逆提取路径见
[`docs/PUBLIC_BOUNDARY.md`](PUBLIC_BOUNDARY.md)。

## 先读

| 场景 | 入口 |
| --- | --- |
| 理解产品目标、稳定边界与 V1 架构 | `docs/PRODUCT_ARCHITECTURE.md` |
| 导入 Windows / Linux 本机对局或让 AI 助手协助定位 | `docs/USER_REPLAY_IMPORT.md` |
| 接手 Rust Engine 开发、看当前基线 | `docs/AGENT_CONTEXT.md` |
| 查原版公开证据与规则索引 | `research/original-game/BATTLE_RULE_INDEX.md` |
| 快速看原游戏战斗规则骨架 | `research/original-game/SIMPLIFIED_BATTLE_RULES.md` |
| 查回放历史输入字段 | `docs/LAST_ROUND_DATA_AUDIT.md` |
| 看公开规则开发与 Rust 公共门禁 | `docs/MECHANISM_ANCHORS.md` |
| 看结算链取证与修复教训（反编译/开局顺序/drift） | `docs/RULE_DEBUGGING_LESSONS.md` |
| 看 Web UI refinement 验收指标 | `docs/WEBUI_REFINEMENT_METRICS.md` |
| 按有限清单迭代、复测与收口 Web UI | `docs/UI_ITERATION_WORKFLOW.md` |
| 看原版证据到实现的边界、五线拓扑与收口门禁 | `docs/CROSS_LINE_RUNBOOK.md` |
| 查游戏机制原文与证据 | `bun battle-evaluator/scripts/lookup-evidence.ts <名字\|id>` |
| 看公开/私有边界与迁移步骤 | `docs/PUBLIC_BOUNDARY.md` |
| 看依赖、文档和缓存维护规约 | `docs/MAINTENANCE.md` |

不要从历史 handoff 或自动生成长报告开始读。需要数量时先跑命令，再信报告。

目标产品架构、原作合法/研究沙盒边界、隐私与静态发布契约只在
`docs/PRODUCT_ARCHITECTURE.md` 定义；本页只负责导航当前工程入口。工程线拓扑见
`docs/CROSS_LINE_RUNBOOK.md`。

## 私有工程面（在 `main`，不进公开导出）

Analysis（Solver / GA / Value）、私有 replay corpus、原版研究 Python 工具链
（`research/original-game/*.py`）与 build 权威输入（`battle-evaluator/data/current-build.ts`、
`original-build-profiles.json`）都在 `main` 里日常维护，由 `public-export-policy.json` 排除出公开投影。
不要把私有报告、回放标识符或 payload 复制进会被导出的文档。边界核对见 `docs/PUBLIC_BOUNDARY.md`。

研究主线（练习模式地狱人机、通用套路训练）入口：

| 场景 | 入口 |
| --- | --- |
| 现行方法、在用命令与最新战绩 | `docs/HELL_PUPPET_HANDOVER_VALUE_V2.md` |
| 通用套路训练框架设计 | `docs/UNIVERSAL_ARCHETYPE_TRAINING_FRAMEWORK.md` |
| 无头驱动原客户端 | `.agents/skills/original-client-practice-play/SKILL.md` |
| GA 强卡组生成 | `docs/GA_DECK_SEARCH.md`（早期构筑穷举见 `docs/archive/ga/DECK_DISCOVERY.md`） |

## 当前可信来源

| 优先级 | 来源 | 用途 |
| ---: | --- | --- |
| 1 | 服务器真实回放，或冻结的原作客户端 checkpoint + Rust 最小契约 | 实现验收 |
| 2 | 原版反编译代码与配置 | 解释规则与约束合成输入 |
| 3 | `research/original-game/` 人工结论 | 索引和假设，不替代客户端采集 |

文档与可执行结果冲突时，先修文档或门禁，不放宽 `winner / actorTurn / hpDelta`。

## 文档维护规则

| 类型 | 规则 |
| --- | --- |
| 产品契约 | `docs/PRODUCT_ARCHITECTURE.md` 是稳定需求、目标架构与 V1 验收唯一真相源；不写易漂移数量 |
| 人工入口 | `docs/AGENT_CONTEXT.md` 保持短而准 |
| 机器报告 | `battle-evaluator/generated/*.json` 由脚本生成；需要数量时跑命令，不手写状态文档 |
| 历史材料 | `docs/archive/` 按内容分类；历史 handoff、旧研究方法、GA 榜单、solver 读数、value 诊断只在追溯时读 |
| 维护规约 | `docs/MAINTENANCE.md` 记录依赖、文档和缓存膨胀的处理规则 |

已经合并进当前入口的旧文档不要重新扩散。新增经验优先写进 `AGENT_CONTEXT` 或
对应工作流文档；研究主线只维护一份滚动交接 `HELL_PUPPET_HANDOVER_VALUE_V2.md`，
已结案章节原样移入 `docs/archive/handoff/`，不另开 handoff 文档。
