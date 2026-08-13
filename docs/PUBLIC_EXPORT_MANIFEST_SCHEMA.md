<!-- topic: release-boundary -->

# Public export manifest 与 attestation schema

本附录是 [`PUBLIC_EXPORT_AND_ATTESTATION.md`](PUBLIC_EXPORT_AND_ATTESTATION.md) 的 versioned
字段规范。它只定义发布证明的结构；不授权读取或分发私有 corpus。

## 1. Allowlist manifest

建议在 private 主仓维护 `.release/public-export-manifest.json`，在 public-final 不保留 private
路径；其自身通过 source commit hash 绑定。字段顺序固定，JSON 使用 UTF-8 和末尾换行：

```json
{
  "schemaVersion": 1,
  "project": "Open-YiXianCard",
  "exportPolicy": "public-final-v1",
  "source": {
    "repository": "Open-YiXianCard",
    "commit": "<40-hex-source-commit>",
    "tree": "<40-hex-source-tree>",
    "policyCommit": "<40-hex-policy-commit>"
  },
  "target": {
    "repository": "Open-YiXianCard-public-final",
    "baseCommit": "<40-hex-public-base-commit>",
    "mode": "fast-forward-only"
  },
  "include": [
    "AGENTS.md", "LICENSE", "NOTICE", "CORPUS_POLICY.md", "README.md",
    ".bun-version", "bun.lock", "package.json", "jsconfig.json", "index.html",
    "engine-rust/**", "src/ui/**", "shared/**", "scripts/**", ".agents/**",
    ".github/workflows/**", "docs/AGENT_CONTEXT.md", "docs/PRODUCT_ARCHITECTURE.md",
    "docs/MAINTENANCE.md", "docs/USER_REPLAY_IMPORT.md",
    "docs/PUBLIC_EXPORT_AND_ATTESTATION.md",
    "docs/PUBLIC_EXPORT_MANIFEST_SCHEMA.md"
  ],
  "exclude": [
    "engine-ts/**", "research/**", "battle-evaluator/**", "analysis/**",
    "private-companion/**", "**/fixtures/**", "**/corpus/**", "**/oracle/**",
    "**/admission/**", "**/target/**", "dist/**", "public/build/**", "node_modules/**"
  ],
  "transforms": [
    {
      "from": "shared/data/original-build-profiles.json",
      "to": "shared/data/original-build-profiles.json",
      "mode": "public-reviewed-snapshot-v1"
    }
  ],
  "output": {
    "algorithm": "sha256",
    "treeDigest": "<canonical-output-tree-sha256>",
    "files": [
      { "path": "<normalized-path>", "mode": "100644", "sha256": "<64-hex>", "bytes": 0 }
    ]
  }
}
```

`include` 先展开、`exclude` 再减除，exclude 优先级最高；任何未匹配 include 的 tracked 文件
都必须报告。路径使用 `/`，不得绝对化、包含 `..`/NUL 或 symlink；文件模式只允许普通文件，
executable bit 需要显式 allowlist。glob 必须按 Git tree 展开，不能依赖当前工作目录 glob。

output 文件按字典序，以 `path NUL mode NUL sha256 NUL bytes NUL` 编码后计算 `treeDigest`；
receipt 不加入 digest，避免自引用。denylist 命中、未声明新增文件、重复目标路径、变换输出
不稳定或任何读取错误都 fail closed，不生成 candidate。生成时间、CI job ID 和日志路径不进入
可复现 digest。

## 2. Hash 与 release manifest v2

建议同时保留这些互补标识：

| 标识 | 计算对象 | 用途 |
| --- | --- | --- |
| `sourceCommit` | private source 的 Git commit OID | 固定完整输入历史状态 |
| `sourceTree` | source commit 的 Git tree OID | 证明输入树快照 |
| `treeDigest` | allowlist 投影的规范化 path/mode/content 摘要 | 跨仓库复核输出 |
| `rulesetTree` | `HEAD:engine-rust` 的 tree OID | 绑定 canonical Rust |
| `snapshotSha256` | 规范化后的 public shared snapshot | 绑定 Steam build 元数据 |
| `artifactInventorySha256` | release manifest inventory 的规范化摘要 | 绑定静态文件集合 |

Git OID 目前是 40 位 SHA-1，必须标为 `gitCommit`/`gitTree`；文件内容和规范化树摘要使用
SHA-256。任何字段不匹配都使导出、发布或回滚检查失败。

现有 release schema 的下一版建议把 `evidenceManifestSha256` 改为 `sharedSnapshotSha256`，
并增加：

```json
{
  "export": {
    "policy": "public-final-v1",
    "sourceCommit": "<40-hex>",
    "sourceTree": "<40-hex>",
    "outputTreeDigest": "<64-hex>"
  },
  "ruleset": { "engineRustTree": "<40-hex>" },
  "sharedSnapshot": {
    "path": "shared/data/original-build-profiles.json",
    "sha256": "<64-hex>",
    "steamBuild": "24610558"
  }
}
```

兼容期旧字段只能作为严格别名，值必须完全等于 `sharedSnapshot.sha256`；不能继续把含有
私有 research 路径的文件称为 evidence manifest。

## 3. Shared snapshot projection

`shared/data/original-build-profiles.json` 的 public projection 只保留 Rust/UI 实际消费者需要的：

- `schemaVersion`、`steamAppId`、`projectTargetSteamBuild`；
- `runtimeSupportedSteamBuilds`；
- 每个 capability 的稳定 key 和最终 boolean 值。

必须删除 `generatedBy: battle-evaluator/...`、`research/original-game/...`、绝对路径、原始文件名、
fixture/replay ID 和任何可反查 private source 的字段。projection 需要排序 key、稳定缩进、末尾
换行，并由 Rust `original_build_profile.rs` contract test 验证后才进入 export。内部 provenance
可记录证据 hash，但不可进入 public manifest、bundle、日志或错误信息。release metadata 只读取
projection，不能再把它称为 evidence manifest。

无法在 private 侧复核来源的 capability 应移除 public support 并让 Rust 返回 unsupported；不能
用不存在的 `research/...` 占位路径掩盖证据缺口。

## 4. Exact parity attestation

private CI 在拥有 corpus 的环境运行 exact replay gate；public CI 不接触 corpus。断言必须同时
精确比较 `winner`、`actorTurn`、`hpDelta`，并绑定 source commit、Rust tree、shared snapshot
和 Steam build。建议的签名 JSON：

```json
{
  "schemaVersion": 1,
  "type": "open-yixiancard-exact-parity",
  "status": "passed",
  "assertions": ["winner", "actorTurn", "hpDelta"],
  "sourceCommit": "<40-hex-private-commit>",
  "engineRustTree": "<40-hex>",
  "sharedSnapshotSha256": "<64-hex>",
  "supportedSteamBuild": "24610558",
  "corpusCommitment": "hmac-sha256:<64-hex>",
  "resultCommitment": "sha256:<64-hex>",
  "signingKeyId": "<public-key-fingerprint>",
  "signature": "<detached-signature>"
}
```

隐私规则：

- 不写 fixture 总数、fixture ID、原始 replay 文件名、玩家/账号/Steam ID、时间戳、机器路径、
  首差事件、完整 expected/actual payload 或 corpus 文件清单；
- `corpusCommitment` 使用 private HMAC key 或随机保密 nonce；不要发布裸 `SHA256(corpus)`，
  以避免小型或公开可猜 fixture 的字典攻击；
- `resultCommitment` 只承诺排序后的通过结果摘要，不可由它反推出单局数据；
- 签名私钥只存在 private CI；public 可保存公钥 fingerprint/可信 key 配置，但不保存 corpus
  或私钥；
- attestation 失败时不生成 `passed` 文件，不用“部分通过”替代 exact gate。

attestation 证明“某个绑定的私有基线通过了门禁”，不证明 public 用户可以从源码重建私有
corpus。public PR 至少验证签名、字段格式、source/ruleset/snapshot 绑定和 policy 版本；私有
CI 负责验证 corpus 本体。
