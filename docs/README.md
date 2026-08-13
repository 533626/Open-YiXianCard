# Open-YiXianCard 文档入口

Open-YiXianCard 是证据驱动的单场战斗模拟与浏览器展示项目。公开工程由原版研究、共享评估契约、
Rust canonical 规则核、跟随 Rust 移植的 TS 兼容档案和浏览器 UI 组成。Analysis、回放 corpus 与
ratatui TUI 是私有 engineering companion，不属于公开构建面；边界与可逆提取路径见
[`docs/PUBLIC_BOUNDARY.md`](PUBLIC_BOUNDARY.md)。网站发布与 Cloudflare 部署不在本次变更内。

## 先读

| 场景 | 入口 |
| --- | --- |
| 理解产品目标、稳定边界与 V1 架构 | `docs/PRODUCT_ARCHITECTURE.md` |
| 导入 Windows / Linux 本机对局或让 AI 助手协助定位 | `docs/USER_REPLAY_IMPORT.md` |
| 接手 Rust Engine 开发 | `docs/AGENT_CONTEXT.md` |
| 看当前基线 | `docs/AGENT_CONTEXT.md` |
| 查 TS 只读兼容档案（2026-08-09 冻结） | `engine-ts/README.md` |
| 查原版公开证据与规则索引 | `research/original-game/BATTLE_RULE_INDEX.md` |
| 快速看原游戏战斗规则骨架 | `research/original-game/SIMPLIFIED_BATTLE_RULES.md` |
| 查回放历史输入字段 | `docs/LAST_ROUND_DATA_AUDIT.md` |
| 看公开规则开发与 Rust 公共门禁 | `docs/MECHANISM_ANCHORS.md` |
| 看结算链取证与修复教训（反编译/开局顺序/drift） | `docs/RULE_DEBUGGING_LESSONS.md` |
| 看原版证据到实现的公开边界 | `docs/CROSS_LINE_RUNBOOK.md` |
| 看 Web UI refinement 验收指标 | `docs/WEBUI_REFINEMENT_METRICS.md` |
| 按有限清单迭代、复测与收口 Web UI | `docs/UI_ITERATION_WORKFLOW.md` |
| 看 UI fixture 一致性调查与固化流程 | `docs/UI_FIXTURE_CONSISTENCY_REPORT.md` |
| 看五线边界、单引擎 SLA 和收口门禁 | `docs/CROSS_LINE_RUNBOOK.md` |
| 查游戏机制原文与证据 | `bun battle-evaluator/scripts/lookup-evidence.ts <名字\|id>` |
| 看公开/私有边界与迁移步骤 | `docs/PUBLIC_BOUNDARY.md` |
| 看依赖、文档和缓存维护规约 | `docs/MAINTENANCE.md` |

不要从历史 handoff 或自动生成长报告开始读。需要数量时先跑命令，再信报告。

目标产品架构、原作合法/研究沙盒边界、隐私与静态发布契约只在
`docs/PRODUCT_ARCHITECTURE.md` 定义；本页只负责导航当前工程入口。工程线拓扑见
`docs/CROSS_LINE_RUNBOOK.md`。

## 私有 engineering companion

Analysis（Solver / GA / Value）、私有 replay corpus、ratatui TUI、原版研究 Python 工具链
（`research/original-game/*.py`）与 build 权威输入（`battle-evaluator/data/current-build.ts`、
`original-build-profiles.json`）不属于公开构建或公开文档入口。它们的源文件、报告和测试在私有
companion 中继续维护；公开仓库只保留 engine、contracts、UI 以及稳定的规则开发文档。不要把私有
报告、回放标识符或 payload 复制回公开树。恢复与核对入口见 `docs/PUBLIC_BOUNDARY.md`。

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
| 人工入口 | `docs/AGENT_CONTEXT.md`、`engine-ts/README.md`、`engine-ts/CARD_MIGRATION.md` 保持短而准 |
| 机器报告 | `battle-evaluator/generated/*.json` 由脚本生成；需要数量时跑命令，不手写状态文档 |
| 历史材料 | `docs/archive/` 按内容分类；历史 handoff、旧研究方法、GA 榜单、solver 读数、value 诊断只在追溯时读 |
| 维护规约 | `docs/MAINTENANCE.md` 记录依赖、文档和缓存膨胀的处理规则 |

已经合并进当前入口的旧文档不要重新扩散。新增经验优先写进 `AGENT_CONTEXT` 或
对应工作流文档，不要再新增 handoff 文档。
