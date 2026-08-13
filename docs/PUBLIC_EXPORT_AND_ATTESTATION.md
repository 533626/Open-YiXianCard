<!-- topic: release-boundary -->

# Private → public-final 导出与一致性证明

本文定义 `Open-YiXianCard` 私有主仓到 `Open-YiXianCard-public-final` 的发布边界。
它是发布流程的设计约束，不是私有 corpus 的导出工具，也不授权把 replay、原版客户端提取物、
admission/oracle 或分析产物复制到 public-final。字段规范见
[`PUBLIC_EXPORT_MANIFEST_SCHEMA.md`](PUBLIC_EXPORT_MANIFEST_SCHEMA.md)。

## 1. 目标与当前缺口

private 主仓是唯一开发真相源；public-final 是由固定提交、固定 allowlist 和固定变换生成的
发布投影。人工复制目录、合并无共同祖先的 Git 历史，或用一个“内容看起来公开”的快照替代
导出证明，都不能作为发布依据。

现有 `build-site`/`release-artifact` 已经能证明静态产物是零 fixture、内容寻址、绑定 Steam build、
Rust tree 和 app commit，但它们只从当前 checkout 构建，不能证明 checkout 是 private 主仓的
确定性 allowlist 投影。现有 release manifest 的 `evidenceManifestSha256` 实际哈希的是
`shared/data/original-build-profiles.json`；该文件仍含有 `battle-evaluator` 与
`research/original-game` 的生成路径，因此名称和内容都需要在下一版发布协议中收口。

本方案把两个问题分开：

1. **源代码导出证明**：private commit 的 allowlist 投影与 public-final commit 完全一致；
2. **规则正确性声明**：private CI 在不公开 corpus 的情况下，声明 Rust 对私有回放基线通过
   `winner`、`actorTurn`、`hpDelta` 精确断言。

第二项不是把 corpus 隐式带入 public-final，也不是让 public 用户重新验证私有 replay。

## 2. 输入与输出

### 2.1 导出输入

导出器只接受以下输入：

| 输入 | 要求 |
| --- | --- |
| private source commit | 完整 40 位 Git commit；必须在 private 主分支上，且 checkout 无 tracked/untracked 改动 |
| export policy | 版本化的 policy commit；包含 include allowlist、exclude denylist、路径变换和工具版本 |
| public base commit | public-final 当前 `main` 的完整 40 位 commit；只能 fast-forward 或从清洁基线替换 |
| reviewed shared snapshot | 已审阅的 public projection；只保留 Rust/UI 实际消费者需要的字段 |
| parity attestation input | private CI 生成的签名声明；只输入结果承诺和规则树绑定，不输入 corpus 文件 |
| toolchain lock | `.bun-version`、`bun.lock`、Rust lock/target 与 exporter 版本摘要 |

导出器不得扫描、读取或枚举 corpus 目录来决定输出。它应使用 Git object 读取明确的 allowlist
路径；私有回放、原版客户端文件、原始提取物和 generated corpus 即使存在于 source commit，也
必须因为未匹配 allowlist 而不可达。

### 2.2 导出输出

一次成功导出产生：

- public-final 工作树或提交，只含 allowlist 结果；
- `export-receipt.json`（建议留在 private CI artifact，必要时只发布去隐私摘要）；
- 公开静态 `dist/`，由现有 `build-site` 生成并由 `check-release` 审计；
- private CI 保存的 signed parity attestation；
- 供 PR 审阅的 diff、文件清单和失败原因。

不要把 private 的 policy 文件、源路径、corpus 数量、fixture ID 或内部 receipt 原样复制进
public-final。公开仓库只需要能说明“这个提交通过了哪一版 export policy”，不需要知道 private
工作区布局。

## 3. Allowlist 导出约束

建议在 private 主仓维护 `.release/public-export-manifest.json`，在 public-final 不保留 private
路径；manifest schema、allowlist 示例、哈希规则和变换契约见
[`PUBLIC_EXPORT_MANIFEST_SCHEMA.md`](PUBLIC_EXPORT_MANIFEST_SCHEMA.md)。其自身通过 source
commit hash 绑定。

实现时必须保证：

- `include` 先展开，`exclude` 再减除；exclude 优先级最高；任何未匹配 include 的 tracked 文件
  都报告出来，而不是静默丢弃；
- 路径使用 `/`、不得是绝对路径、不得包含 `..`、NUL 或 symlink；文件模式只允许普通文件，
  可审阅的 executable bit 需要显式 allowlist；
- 清单中的 glob 按 Git tree 展开，不能依赖当前工作目录的文件系统 glob；
- output 文件按字典序计算规范化 `treeDigest`，不把 receipt 自身加入 digest，避免自引用；
- 导出遇到 denylist 命中、未声明新增文件、重复目标路径、变换输出不稳定或任何读取错误时
  立即失败，不生成可发布目录；
- 生成时间、CI job ID、日志路径属于外部元数据，不进入可复现的 output digest。

`include` 是发布产品契约，不应为了让某次导出通过而扩大到整个目录。新增 Rust/UI/shared
文件必须先修改 policy，再审阅 public boundary 和 release artifact。

## 4. Shared snapshot 迁移

当前 `shared/data/original-build-profiles.json` 的运行时消费者只需要 authoritative Steam build、
runtime supported builds 和 capability boolean matrix；`generatedBy`、`evidence.diffPath`、
`changedSources`、历史 diff 路径和 source hash 是私有证据 provenance，不应进入 public snapshot。

迁移步骤：

1. 在 private 环境从现有 profile 生成一次人工审阅的 public projection；保留
   `schemaVersion`、`steamAppId`、`projectTargetSteamBuild`、`runtimeSupportedSteamBuilds`、
   每个 capability 的稳定 key 和最终 boolean 值。
2. 删除 `generatedBy: battle-evaluator/...`、`research/original-game/...`、绝对路径、原始文件名、
   fixture/replay ID 和任何可反查 private source 的字段。
3. 将 projection 规范化（排序 key、稳定缩进、末尾换行），提交到 `shared/data/`，并记录
   `sharedSnapshotSha256`。Rust `original_build_profile.rs` 的 schema 读取必须先在 private CI
   对 projection 做 contract test，再进入 public export。
4. provenance 留在 private receipt：它可以记录证据文件的内部 hash，但不能把路径或 corpus
   文件名带入 public manifest、bundle、日志或错误信息。
5. release metadata 只读取 projection；build-site 不应再把“evidence”作为公开字段名。

不能把 `research/...` 路径简单改成不存在的占位路径。public snapshot 要么是可审阅的最小数据，
要么 fail closed。若某个 capability 的来源无法在 private 侧复核，移除该 capability 的 public
support 并让 Rust 返回 unsupported，比保留无法解释的值安全。

## 5. Fail-closed gates

### Private export gate

发布 job 必须依次检查：

1. source commit 在受保护 private 主分支；工作树、submodule、生成目录无未提交变更；
2. policy schema、allowlist、denylist 和 transform version 能被锁定并计算摘要；
3. Git tree 展开只命中 include，零 denylist 命中，零 symlink/特殊文件，零未声明路径；
4. shared projection contract 通过，禁止 private provenance 字段和路径；
5. output `treeDigest` 与 export receipt 一致；
6. private exact replay gate 通过三项断言，attestation 签名有效；
7. 生成 public candidate 后在 clean clone 中重放导出，得到相同 output digest；
8. candidate public commit 只能是 public-final 当前 main 的 fast-forward，或经过明确批准的
   全新 public baseline，不允许无共同祖先 merge。

任一步失败都停止，删除临时 candidate，保留失败日志但对外日志必须经过路径/标识脱敏。

### Public candidate gate

在 public-final 的 clean clone 中执行：

```bash
bun install --frozen-lockfile
bun run check
bun run check:ui
bun run check:rust:contracts
bun run check:rust:wasm
bun run test:release
bun run build:site
bun run check:release
git diff --exit-code -- index.html
```

再确认：

- `git ls-files` 不含 `engine-ts/`、`research/`、`battle-evaluator/`、`analysis/`、私有 corpus、
  admission/oracle、绝对路径或本地缓存；
- dist 只有 allowlisted hashed assets、`index.html`、`_headers` 和 manifest；
- release audit 报告 `bundledFixtureCount=0`、空 catalog、无 remote fetch；
- manifest 的 `source/ruleset/sharedSnapshot/export` 字段与 receipt/attestation 绑定；
- 所有生成文件可由同一 source commit 和 toolchain 重建。

## 6. Clean clone、发布、fast-forward 与回滚

### 发布顺序

1. private 主仓冻结窗口，提交规则、UI、workflow 和 snapshot；记录 source commit。
2. 生成 allowlist export candidate 和 export receipt；私有 exact parity gate 生成 signed
   attestation（schema 与隐私约束见附录）。
3. 新建临时 clean clone，仅检出 candidate public commit；执行上节 public gates。
4. 在 `Open-YiXianCard-public-final` 上确认 `origin/main` 未变化：

   ```bash
   git fetch origin main
   git merge-base --is-ancestor origin/main <candidate-public-commit>
   ```

   通过后只允许 fast-forward：

   ```bash
   git push origin <candidate-public-commit>:refs/heads/main
   ```

   如果 base 改变，整个导出和审计重做；不使用 `--allow-unrelated-histories`，不在 public-final
   手工解决三仓代码分叉。
5. 以同一 public commit 构建 dist；上传 immutable CI artifact。稳定发布另加受保护 tag、
   release notes、artifact checksum 和签名；当前 workflow 只上传待审 artifact，不应暗示已部署。

### 回滚

- 回滚目标必须是最近一个已审计的 public commit/tag，且其 export receipt、release manifest、
  ruleset tree、snapshot hash 和 attestation 仍可定位；
- 先停止部署/发布，再将 public-final fast-forward 到一个新的 revert commit，或由受保护
  release 管理器选择旧 immutable artifact；不重写 `main` 历史、不 force-push；
- 回滚后重新跑 clean-clone release audit，确认旧 artifact 没有被 source checkout 重新打包；
- 若原因是规则或隐私 gate 失败，private 主仓必须先修复并生成新 source commit；不能只回滚
  public 文件而保留不匹配的 attestation；
- 记录被撤回的 public commit、原因、替代 commit 和 artifact digest。私有 corpus 不随回滚日志
  外泄。

## 7. 已收口迁移项与当前成熟度缺口

本轮 public 投影已经收口：共享快照移除 private provenance；release manifest 使用
`sharedSnapshotSha256`；UI fixture catalog 固定为空；corpus-dependent CLI、测试与 TUI 依赖不进入
allowlist；旧 public 历史只作为 target base；导出改由 schema-v2 receipt 的 `sourceCommit`、
`sourceTree`、逐文件摘要和 `treeDigest` 绑定。clean-clone 门禁同时确认 Rust、WASM、UI 与 release
artifact 可独立构建，且发布制品的 `bundledFixtureCount=0`。

仍未成熟的项目必须继续公开披露：

| 缺口 | 发布影响与处理意见 |
| --- | --- |
| HF 最新发现池仍有 30/6368 strict mismatch，其中 fate 404 为 9/35 | 不影响已准入的 2302/2302 canonical baseline，但不得宣称外部回放全量 parity；先采集最小原版 trace，再决定规则范围 |
| exact parity attestation 目前是 private freeze receipt，尚无独立签名与公开验签链 | public commit 可作为已审计源码快照；标记稳定版前应由受保护 CI 产生只含承诺值的 detached signature |
| workflow 只上传待审 artifact，没有受保护 tag、release notes、checksum/signature 与部署审批 | 当前只发布源码与技术预览制品；补齐 immutable release 流程后再称“可复现稳定发行版” |

因此当前 public `main` 已达到自包含、可构建、可审计的技术预览标准，但仍不标记为“外部全量
parity”或“可复现稳定发行版”。

## 8. 相关规范

- [`PUBLIC_EXPORT_MANIFEST_SCHEMA.md`](PUBLIC_EXPORT_MANIFEST_SCHEMA.md)：manifest、hash、snapshot
  projection 和 exact parity attestation 的 versioned schema。
