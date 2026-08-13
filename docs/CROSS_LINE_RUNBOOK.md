<!-- topic: runbook -->

# 五线工程收口流程

本文固化公开 Rust engine、browser、evaluator、TS compatibility archive 与原版规则研究的日常收口流程。战斗执行已收敛为 Rust 单实现；Analysis、replay corpus、客户端 oracle 与 TUI 属于 private companion，不在公开流程中。

## 拓扑

| 线/层 | 位置 | 规则地位 |
| --- | --- | --- |
| 原版研究 | `research/original-game/` | 公开规则证据：反编译源码、配置与稳定索引 |
| Battle Evaluator | `battle-evaluator/` | 中立 contracts、审查过的共享输入、公开 adapter |
| TS Engine archive | `engine-ts/` | 只读兼容档案（2026-08-09 冻结，`check:ts:frozen` 锁指纹） |
| browser UI | `src/ui/`、`index.html` | 适配与展示层，不复制规则 |
| Rust Engine + WASM | `engine-rust/` | 唯一可变战斗实现与公开产品执行面 |
| Private companion | 私有 checkout / 明确 extraction mapping | 回放语料、准入/oracle、Analysis 与 ratatui/crossterm TUI |

## 不可协商项

1. 新战斗牌必须同时具备原版 `Card_*` 或共享调用链证据、Rust 最小契约和 handler/catalog 接线。
2. Solver / GA / Value 的读数不能反向修改战斗事实或放宽 exact 断言。
3. UI 只通过 Rust 公开规则 API、WASM 和适配层消费事实，不直接复刻规则。
4. Rust 是唯一 canonical engine；TS 已冻结为只读兼容档案，新规则不双写。
5. contracts/data 保持中立，不依赖 engine、Analysis 或私有证据控制面。

## 公开、无 fixture 的验证

先预览受影响范围：

```bash
bun run check:affected -- --dry-run
```

任务收口至少运行与改动直接相关的命令；以下是可在无私有语料、无原版客户端的公开 checkout 中运行的完整公共入口：

```bash
bun run check:public-boundary
bun run check:private-manifest
bun run check:docs-drift
bun run test:evaluator
bun run check:evaluator
bun run test:ts
bun run check:ts:types
bun run check:ui
bun run check:rust:quick
bun run check:rust:wasm
bun run test:release
```

`bun run check` 是上述公共类型、Rust、架构和有意限定的 file-health scope 的聚合入口；CI
quality job 使用相同的公共边界、文档、evaluator、TS、UI 和 Rust 命令。file-health 默认的全仓
报告仍可作为维护报告运行，但现有超长文件不是 public check 的隐藏豁免：聚合门禁只指定已审查的
公共 scope，并将其余维护债务留给独立报告。

| 改动类型 | 优先验证 |
| --- | --- |
| 文档 / 流程 | `bun run check:docs-drift`、`bun run report:file-health -- --paths <changed-docs>` |
| Rust Engine 规则 | `bun run check:rust:quick`、`bun run check:rust:contracts` |
| Rust native/WASM | `bun run check:rust:quick`、`bun run check:rust:wasm` |
| Battle Evaluator contracts/data | `bun run check:evaluator`、`bun run test:evaluator` |
| Browser UI | `bun run check:ui` |
| TS compatibility archive | `bun run check:ts:types`、`bun run test:ts` |
| Local static artifact | `bun run test:release && bun run build:site && bun run check:release` |
| Public/private boundary | `bun run check:public-boundary && bun run check:private-manifest` |

## Private companion boundary

公开 checkout 不生成或消费回放准入、原作客户端 oracle、镜像 corpus、Analysis/GA/solver/value
报告或 TUI。需要这些材料时，先在私有 companion 中恢复 `PRIVATE_ENGINEERING_EXTRACTION.json`
中的显式 source→destination mapping，再由私有流程运行 exact admission/oracle 门禁；公开流程不
伪造、放宽或替代 `winner / actorTurn / hpDelta` 三元断言。映射集合除上述材料外还覆盖原版研究
Python 工具链（`research/original-game/*.py`）与 build 权威输入
（`battle-evaluator/data/current-build.ts`、`original-build-profiles.json`）；companion 恢复后以
`bun run check:private-manifest -- --source-root <companion>` 作为映射/audit 核对入口。

## UI 边界

- browser UI 通过 `src/ui/` Worker 适配 Rust/WASM；主线程只编排和展示。
- 公开站不内置仓库 fixture 或 fixture 索引；用户只能显式导入本地输入。
- 取消重战斗或求解后，旧 Worker 请求不得回写当前状态。
- 公开发布前先过 `bun run test:release && bun run build:site && bun run check:release`；本次不触发 GitHub Release 或 Cloudflare deploy。

## 单引擎规则 SLA

`engine-rust/` 是唯一战斗语义实现；新规则只写一次 Rust 最小契约与实现。机制锚点按当前公开
证据索引解析；没有可审查锚点时失败关闭，不回退私有或历史回放。Rust 未实现的能力不进入正式
读数或候选基线，UI 不回退到 TS。

## 收手标准

在没有新回放时，规则工作从公开原版证据、Rust 最小契约和上述无 fixture checks 开始。私有
companion 的回放诊断只能在恢复私有映射后进行，不属于公共构建依赖。线上部署、GitHub release、
push 和历史 export 另行审批；本轮只做本地检查，不发布。
