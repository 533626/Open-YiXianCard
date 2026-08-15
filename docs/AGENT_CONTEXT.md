# Agent Context

本文件是给 coding agent 的最小入口。需要更多背景时再跳到对应报告。

当前公开证据目标为 Steam build 24705509；canonical Rust/UI public checks are fixture-free。回放 corpus、准入/oracle metadata、Analysis 与 TUI 保留在私有开发用 `main`，但不进入公开导出。

## 当前路线

```text
公开原作证据与规则索引
  -> Rust canonical battle engine
  -> typed evaluator -> browser Worker / UI
  -> immutable local dist audit

Private development surface in `main` (not public build): replay corpus / admission / client oracle / analysis / ratatui TUI
```

规则层仍以原版证据和真实回放为准；solver、GA、value 和 UI 只能消费或暴露规则能力，不能反向放宽
`winner / actorTurn / hpDelta` 的 exact 断言。`engine-ts/` 已冻结（2026-08-09）为只读兼容档案，
内容指纹由 `check:ts:frozen` 锁定，不得反向作为原版证据。
稳定产品需求、零内置 fixture 政策和目标架构见 `docs/PRODUCT_ARCHITECTURE.md`。

## 公开门禁

公开导出 checkout 不含回放 corpus、准入收据、原作客户端 oracle、镜像语料或 TUI。开发用
`main` 保留这些私有工程材料；对外投影统一使用 `bun run export:public`。常用无 fixture checks：

- `bun run check:public-boundary`
- `bun run check:private-manifest`
- `bun run check:docs-drift`
- `bun run test:evaluator` / `bun run check:evaluator`
- `bun run test:ts` / `bun run check:ts:types`
- `bun run check:ui`
- `bun run check:rust:quick` / `bun run check:rust:wasm`
- `bun run test:release`

`bun run check` 聚合这些公共规则、类型、架构和有意限定的 file-health scope。完整 file-health 报告
可另行运行，hard findings 不会被生产规则或 exact replay 断言掩盖。

需要查看当前原版证据时使用 `bun run original:query -- <名称或 ID>` 和
`research/original-game/EVIDENCE_MANIFEST.json`；不要把本机 evidence cache、客户端路径或回放 ID
写入公开文档。

## Private development surface

`main` 是唯一开发分支，也包含私有 replay corpus、准入/oracle metadata、`analysis/`、TUI 与
私有回归测试。它必须保持在私有 Git 工作区；公开 checkout 不是从 `main` 直接发布，而是由
`public-export-policy.json` 生成的 allowlist 投影。`PRIVATE_ENGINEERING_EXTRACTION.json` 仍可用于
历史 companion 备份的核对，但不再要求把日常开发内容搬到单独目录。
公开边界与跨线收口见 `docs/PUBLIC_BOUNDARY.md`、`docs/CROSS_LINE_RUNBOOK.md`。

## 当前边界

- Rust 是唯一可变战斗实现；TS 已冻结为只读兼容档案（`check:ts:frozen` 锁内容指纹）。
- UI 只消费 Rust/WASM 和中立 contracts，不复制规则。
- 新规则必须有原版 `Card_*` 或共享调用链证据、Rust 最小契约和 handler/catalog 接线。
- 私有语料未覆盖的规则不得用 `approx`、通用占位或卡面猜测代替精确行为。
- 线上部署、GitHub release、push、force-push、remote deletion 与 destructive history rewrite 均不属于本阶段。
