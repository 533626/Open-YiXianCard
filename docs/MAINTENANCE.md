<!-- topic: maintenance -->

# 维护规约

本文约定依赖、文档和本地缓存的处理方式。目标是让项目保持可复现、可阅读、低膨胀，
同时不牺牲日常开发速度。

## 总原则

| 内容归属 | 处理方式 |
| --- | --- |
| 源码、契约、入口文档 | 提交并维护 |
| 生成报告 | 用脚本重建，不手写数量 |
| 本地缓存、编译产物 | 不提交，阶段性回收 |
| 原版证据缓存 | 保留 `current` 和当前 build；diff 冻结后删除旧 build |

单 agent 或并行施工时，按负责路径运行 scoped gate：

```bash
bun run check:worktree -- --paths src/ui index.html
```

所有 agent 停止施工后的冻结收口，再运行全仓 gate：

```bash
bun run check:worktree
```

涉及文档、脚本或目录增长时，先看：

```bash
bun run report:file-health -- --paths <负责路径...>
bun run report:rust-file-health
bun run report:footprint
```

全仓 `report:file-health` 留给稳定分支整合、准入或发布冻结，不作为每个任务的无关阻塞项。
它只覆盖文档、TypeScript、browser UI CSS 等脚本内登记的作用域；传入未覆盖的 `--paths`
会显式失败，避免空表被误读为通过。`engine-rust/**` 单独使用
`bun run report:rust-file-health`，该门禁采用 Rust 专属阈值并已包含在 `check:rust:quick`。
行数与附加指标阈值以该门禁为准，不再手写。

## 文档

| 类型 | 放置位置 | 规则 |
| --- | --- | --- |
| 当前入口 | `docs/README.md`、`docs/AGENT_CONTEXT.md` | 短而准，只放当前路线 |
| 工作流说明 | 对应主题文档 | 写命令、边界和读数纪律 |
| 机器数量 | `battle-evaluator/generated/*.json` 或报告命令 | 不手写长期状态 |
| 历史读数 | `docs/archive/` | 只追溯时读取 |
| 原版规则证据 | `research/original-game/` | 规则结论必须锚定源码、配置或回放 |

文档增长超过这些信号时先整理再继续追加：

- 当前入口开始解释历史原因；
- 一个文档混入多条工作线；
- 数量、榜单、读数可以由命令生成；
- `bun run report:file-health` 出现 hard 项；
- soft 项不是单一强相关主题。

合并文档后删除旧入口，不保留“兼容链接”占位，除非有真实外部引用需求。

### 文档漂移管理

先按内容决定真相源，不允许同一个事实由多份手写文档共同维护：

| 内容 | 真相源 | 活跃文档怎么写 |
| --- | --- | --- |
| 稳定语义、边界、操作顺序 | 对应源码、契约和主题工作流 | 可手写，但必须给出源码/命令入口 |
| build、哈希、覆盖数量、exact 数量 | manifest、generated JSON 或报告命令 | 只引用产物/命令，不复制数值 |
| 当前里程碑摘要 | `docs/AGENT_CONTEXT.md` | 允许一处短快照；证据更新时同批更新 |
| dated/build-scoped 结论 | `docs/archive/<主题>/` | 文件名带日期或 build，只读，不冒充当前状态 |

一次证据或报告更新按同一事务收口：

1. 重建机器产物并运行权威报告。
2. 只更新唯一的当前摘要。
3. 把旧长表移入 archive 或删除。
4. 运行 `bun run check:docs-drift`、对应 file-health 和 scoped worktree gate。

具体纪律：

- 活跃入口只回答“现在从哪里查”，不保存可由命令重建的完整表。
- 评审需要固定读数时，把快照放入 archive；文件名写日期或 build，正文声明证据身份。
- 原作 build 更新时，先更新 `EVIDENCE_MANIFEST.json`，再同步 `AGENT_CONTEXT`；其他研究入口只引用 manifest。
- 引用 archived snapshot 必须明确写“历史”，不得把其中数量带回当前入口冒充实时值。
- 删除或归档旧方法后，门禁应能识别其典型标题或命令，防止旧入口回流。

运行：

```bash
bun run check:docs-drift
```

当前门禁会校验 manifest build 与 `AGENT_CONTEXT` 同步、活跃规则索引保持短入口、历史完整索引
确实位于 archive，以及手工逐动表命令不会重新进入当前研究流程。它不替代人工语义审查；新增一种
易漂移事实时，应同时扩充门禁或明确唯一真相源。

名词漂移（`AGENTS.md` 坑 3：名词必须锚定原文或 ID）也在此门禁内：活跃 `.md`（archive、out、
extracted 等除外）中 `card<id>（名）` 括注必须包含 `extracted/current/CardConfig.json` 的原文名；
凭记忆写名、写错一个字都会被拦下。已捕获的漂移别名登记在 `scripts/check-doc-drift.ts` 的
`bannedTerms`（如 card 82 名、physique 属性名的误写），每次人工抓到新漂移就追加一行，让同一
错误只犯一次。

## 缓存和构建产物

| 路径 | 角色 | 推荐处理 |
| --- | --- | --- |
| `engine-rust/target/debug/` | Rust debug/test 增量缓存 | Rust 开发中保留；里程碑后删除 |
| `engine-rust/target/release/` | release 构建产物 | 保留公共 native engine/replay_slice；solver 与 TUI 二进制属于私有 companion，缓存子目录可删 |
| `research/original-game/tools/` | dotnet / ilspy / venv 工具缓存 | 只在重新提取原版证据时需要 |
| `research/original-game/inventory/` | Unity 资源盘点生成物 | 盘点后可删 |
| `research/original-game/out/` | 解码回放中间 JSON | fixture 导出后可删 |
| `research/original-game/extracted/current/` | 当前原版证据缓存 | 默认保留 |
| `research/original-game/extracted/build-*` | build 提取物 | 保留当前 build；前一 build 仅在 diff 期间临时保留，摘要冻结后删除 |
| private companion / analysis outputs | GA、solver、value 与本地 run fingerprint | 不进入公开树；按 `PRIVATE_ENGINEERING_EXTRACTION.json` 在私有 companion 重建 |
| `research/original-game/*.py` 工具链 | 客户端 inventory/decode/decompile/index 编排脚本 | 随 companion 恢复；其产物（`EVIDENCE_MANIFEST.json`、build diff、stable index）留在公开树 |
| `battle-evaluator/data/current-build.ts` / `original-build-profiles.json` | Steam build 权威输入 | 公开树保留审阅产物；源文件随 companion 恢复 |
| `logs/` | 长任务日志及 PID/path 指针 | 本地运维状态；整目录忽略，不提交，确认进程结束后可清理旧文件 |
| `public/build/` | UI 构建输出 | 由 `bun run build:ui` 生成，不手改 |
| `dist/` | 正式静态发布产物 | 由 `bun run build:site` 生成并用 `check:release` 复验；不提交，不手工增删文件 |
| `LICENSE` / `NOTICE` / `CORPUS_POLICY.md` | 公开发布所有者决定 | V1 使用 MIT、第三方 NOTICE 与私有 corpus 政策；策略变化必须显式评审并继续通过政策门禁 |

清理时按可重建性从高到低：`target/debug/` → `out/` → `inventory/` → `tools/` →
release 缓存子目录 → diff 摘要已冻结的旧 `extracted/build-*`。删除前仍需确认目标是明确路径、
相关进程已结束，并且当前任务不再依赖该缓存。

不要清理：

- `.git/`；
- `research/original-game/EVIDENCE_MANIFEST.json`；
- 当前任务需要直接查询的 `extracted/current/`；
- `battle-evaluator/generated/*.json`，除非同一批变更会用权威命令重建并提交对应产物。
