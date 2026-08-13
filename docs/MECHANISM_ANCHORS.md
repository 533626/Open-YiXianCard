<!-- topic: mechanism -->

# Versioned Mechanism Anchors

机制锚点用于 Rust 任务收口层的 changed validation：开发者选择稳定 `mechanismId`，控制面根据
`research/original-game/EVIDENCE_MANIFEST.json` 的当前 Steam build 解析已审查证据，而不是永久绑定
某个历史路径。锚点类型和稳定索引保留在公开树；回放见证、原作客户端 oracle、admission payload
以及 changed gate 命令属于 private companion，不是公开 package surface。

## 数据与公共验证

公开 checkout 只提供 Rust canonical engine、稳定规则索引和无 fixture 的公共门禁：

```bash
bun run check:rust:quick
bun run check:rust:contracts
bun run check:rust:wasm
```

以上命令不会伪造机制见证，也不会消费私有 replay/oracle payload。需要锚点审计或按机制运行
changed gate 时，先按 `PRIVATE_ENGINEERING_EXTRACTION.json` 恢复 companion，再在 companion 中运行
其私有命令。公开 checkout 不注册这些依赖私有证据的门禁，因此不能把缺少私有证据误报为验证通过。

## 版本变化

reanchor 流程只在 private companion 中执行：先标 `unverified_for_current_build`（不得回退历史
锚点）；有服务器真实样本用 server-recorded fixture，否则用 eligible synthetic original oracle；
证据必须自带 new_build provenance，witness 必须能证明机制实际触发；旧锚点保留为 historical。

禁止修改旧 fixture expected 来冒充新 build；证据路径变化也必须新增或明确迁移对应 build 条目。

当前有两类 resolver 输出：

| evidenceKind | resolver 输出 | witness | changed gate |
| --- | --- | --- | --- |
| `server-recorded` | `server-recorded-fixture` | `same-turn-card-sequence` | candidates fixture strict + event diff |
| `synthetic-original-oracle` | `synthetic-original-oracle-pair` | `treatment-control-field-delta` | 临时物化 control/treatment，strict + event diff |

私有 companion 中的 synthetic resolver 必须同时绑定 source manifest、admission report、当前 build、
source fingerprint、target-level eligibility、forward/reverse stability、冻结 exact 字段和 original
baseline。公开 checkout 不保存这些 payload 或路径；恢复后的临时 Rust 输入只写到 companion 管理的
临时目录，门禁结束即删除，绝不进入公开候选目录。上述 resolver 细节仅供 companion 维护，不是
公开命令或公开构建依赖。

`treatment-control-field-delta` 从已冻结的原作客户端 baseline 比较指定 side/field 和方向；它不能只
证明牌在卡组里，也不能从 TS 私有状态反推。历史 admission 的 legacy TS exact 只作为冻结证据字段
保留；当前 changed gate 验证同一 control/treatment 输入的 Rust terminal 与 event parity。

## 新增锚点

add_anchor 流程：`mechanism_id` 必须稳定、含义明确、不含 match ID；优先找当前 build 的
server-recorded 样本，没有就跑 full synthetic original-client admission（产物不得复制或改写为
candidates fixture）；`build` 从证据取，不从当前 manifest 反填历史样本；witness 必须定义
可观测触发，不只是“卡牌存在于卡组”；写 build-scoped anchor 后复验 current-build 与
all-build audit 及 Rust changed gate。

准入只跑本批；Rust replay/event 指纹失效时先选择性 `freeze:replay`，发布时仍按发布面选择完整门禁。
