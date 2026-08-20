<!-- topic: runbook -->

# 原作 build 换代 Runbook

> 目标：每次 Steam 新 build 换代流畅、低消耗、发布合规准确。
> 本文是执行手册：证据细节见私有 `research/original-game/REPLAY_EXTRACTION.md` 与各
> `BUILD_*-to-*_RULE_DELTA.md`；门禁与边界见 `AGENTS.md` / `docs/AGENT_CONTEXT.md`。
> 流程教训来自 2026-08-07/08 的 24466094 → 24589371 → 24610558 两次实机换代。

## 0. 换代前置（先确认再动手）

1. **先查 Steam appmanifest（`steamapps/appmanifest_1948800.acf`，安装目录以本机为准）**：
   `buildid` 与 `TargetBuildID` 不一致 = 有待应用更新。
   **启动 Steam 或重启客户端本身就会应用待更新**（2026-08-07 实机两次踩坑：
   24589371→24610558 就是启动 Steam 时被自动应用的）。提取/采集前必须确认两值一致，
   否则证据链中途跳 build、全部 fail-closed。
2. 确认工具链就位：Steam 安装、`research/original-game/tools/{bin/ilspycmd,dotnet,venv}`。
3. **私有 companion 与公开树一致性**：`extracted/current`、`EVIDENCE_MANIFEST.json`、
   `original-build-profiles.json` 三处必须同一 build。私有树的 `extracted/current` 经常落后
   （边界拆分后无人自动同步），导入含新卡的回放会直接 KeyError。

## 0.5 单入口状态机（推荐）

新 build 不再手工串接多个 promote 命令，使用：

```bash
bun run rotate:build -- --new auto --public-target /path/to/Open-YiXianCard-public-final
# 中断后从 .rotation/<old>-to-<new>/state.json 继续
bun run rotate:build -- --new auto --resume --public-target /path/to/Open-YiXianCard-public-final
```

非 `--dry-run` 轮换必须提供 public target。候选 staging 会先执行一次
`export:public --dry-run`（allowlist、边界与 100 MiB size gate），通过后才 promote；默认只做
门禁，不发布到 public target。明确需要发布时再加 `--publish-public`。这样 public gate 不会
在 promote 后才第一次失败，也不会为重复的 dry-run 延长流程。

状态机按 `preflight → extract → diff → screen → stage → validate → promote → public` 记录每步
开始/结束时间、耗时、Steam manifest/candidate 哈希和事务状态。`--new <id>` 可用于已知
候选；`--dry-run` 只做 manifest、工具链、候选和 diff 身份检查。候选先写
`extracted/build-<id>`，只有生成 profile/card/combine/archive、根 manifest、共享快照和
文档元数据并通过门禁后，才以可回滚事务切换 `extracted/current`；任何生成器或门禁失败都会
恢复 current、根 manifest、生成文件与文档，不留下半切换状态。`--resume` 会先恢复一个
上次中断的 prepared transaction，再从最后一个成功步骤继续。

`lookup-evidence.ts` 无 `--build` 时严格读 `extracted/current`；候选证据必须显式
`--build build-<id>`，因此换代中不会把未验证候选混入规则调查。

## 1. 证据提取与切锚（机械步骤）

```bash
python3 research/original-game/extract_build.py --build <new>   # 纯提取（不碰 current）
python3 research/original-game/diff_builds.py <old> <new>       # 生成 builds/<old>-to-<new>.diff.{json,md}
python3 research/original-game/build_evidence_manifest.py \
  research/original-game/extracted/build-<new> research/original-game/EVIDENCE_MANIFEST.json \
  --steam-build <new>                                           # 切锚
bun run generate:original-build-profiles                        # profiles 重生成（链 tip=manifest）
python3 research/original-game/extract_build.py --promote-only --build <new>  # current + combines + archive
bun run evidence:card-configs                                   # 卡配置（含新卡）
```

要点：

- `EVIDENCE_MANIFEST` 必须与 diff `new` 节逐文件一致（`validateEvidenceChain` 精确匹配，
  不一致生成器直接失败）。
- **源码 0 变更的热更**（如 24610558：仅 1 卡 desc + 17 条 fate strategy 标记）capability
  无需新登记，profiles 只加 profile 行。源码变更时逐条核对正交性（方法级 diff），
  **非正交登记必须标注「待真实 reverify」**（先例：422 星力机制三处触发点改写）。
- 同步 `docs/AGENT_CONTEXT.md` build 号（项目入口，push 前必查）。
- 门禁：`check:original-build-profiles`、`test:evaluator`、`test:ts`、`check:rust:quick`、
  `check:docs-drift`；测试里机械 build 号期望（original-build-flags 等）随轮换更新。

## 2. 规则影响评估（一次做透，别挤牙膏）

- `bun battle-evaluator/scripts/triage-build-diff.ts <diff.json> --corpus <私有语料根>`
  分类：已实现 / 语料可达（应实现）/ 零覆盖（fail-closed 挂起）/ 需人工判定。
- 产出 `research/original-game/BUILD_<旧>-to-<新>_RULE_DELTA.md`：15 源文件方法级差异表、
  capability 逐条正交性结论（含行号证据）、新卡/变更卡行为清单、引擎后续动作分批建议。
- 纪律：**反编译里存在分支 ≠ 实现理由**；判据只有「真实回放里出现过」。
  纯表现变更（皮肤 FX、相机）明确标注不实现。

## 3. 验证闭环（push 前必须完整走一遍）

rotate 的 `screen` 不是只写计划：带 `--corpus` 时会实际依次执行 affected、
regression/drift anchors、full-once 三个 tier。三个 tier 都必须显式提供真实命令；仓库提供
的 `screen:rotation` 会构建/复用当前 revision 的 `replay_slice`，用严格 admission 比较
`winner`、`actorTurn`、`hpDelta`，并写机器 receipt。例如（每个路径必须是已导出的 fixture
文件或目录）：

```bash
bun run rotate:build -- --new auto --public-target /path/to/public-final \
  --corpus /private/fixtures/affected \
  --affected-command 'bun run screen:rotation -- --tier affected --fixtures "$ROTATION_CORPUS" --diff "$ROTATION_DIFF" --receipt "$ROTATION_STATE_DIR/screen-affected.json" --build "$ROTATION_NEW_BUILD"' \
  --anchors-command 'bun run screen:rotation -- --tier regression-drift-anchors --fixtures /private/fixtures/anchors --receipt "$ROTATION_STATE_DIR/screen-anchors.json" --build "$ROTATION_NEW_BUILD"' \
  --full-command 'bun run screen:rotation -- --tier full-once --fixtures /private/fixtures/full --receipt "$ROTATION_STATE_DIR/screen-full.json" --build "$ROTATION_NEW_BUILD"'
```

每个命令都在 `state.json` 的 `screen.tiers` 及
`.rotation/<old>-to-<new>/screen-*.receipt` 留下状态、耗时和 stdout/stderr；命令非零或
缺失会 fail-closed；没有 `--corpus` 时三个 tier 和 `steps.screen` 明确记为 `skipped`，不能
被解释为已完成的 replay gate。HF 镜像使用单入口 `bun run screen:hf -- ...`，它会增量构建
并常驻 `screen_hf_worker.py`/`replay_slice --admission-stream`，同时把 source revision 与
binary SHA-256 写入 receipt。anchors 命令负责一方 exact 三字段门禁及 third-party baseline
的 no-new-mismatch/signature 稳定性；外部 drift 不要求清零。

顺序按成本从低到高：

1. **yiwen 首方导入 + screen**（需客户端）：最新弈闻记录 → fixture → 引擎 screen。
   mismatch 用 `dump-aligned-checkpoints.ts` + oracle 首差定位 → 每根因一个 `fix(replay:)` 小提交。
2. **oracle 事件级复核**：`oracle:mirror:run` / 首方 oracle，逐 checkpoint 对齐。
   教训：**崩拳动量公式假设曾被 oracle 证伪**（引擎与原版结构等价，真根因是缺失的
   fate 策略 427/429）。oracle 是唯一能证伪「凭数值猜规则」的工具，猜之前先采。
3. **HF 镜像池全量 screen**（免客户端）：`sharpobject/yxp_replays` 最新 shard
   （按 gameVersion 筛，如 24610558=001.0007.0011）→ `records:mirror` 导出 →
   `replay_slice --admission <external 批次>` 全量。对照：**旧 build 批次必须 exact**，
   否则先查引擎而非新卡。新卡在 HF 出现即获得实现资格（blocked→实现→exact）。
   数据面：先统计 version 分布再筛，避免旧 build 尾流混入。新流程可用
   `research/original-game/screen_hf_stream.py` 读取目录、JSONL、`.gz`、`.zst` 或 `.zip`
   流，按批次交给常驻 screening worker（`--persistent-worker` 让 worker 进程只启动一次，
   每行接收 `{batchDir,resultPath,batchNumber}`；worker 可把已转换的 fixture 路径送入
   `replay_slice --admission-stream`；仓库内参考适配器是
   `research/original-game/screen_hf_worker.py`），checkpoint 绑定源指纹，
   中断可恢复；只落 mismatch/blocked/error 和精确计数，不物化 exact JSON。结果必须标记
   `provenance=third-party-mirror`，不可写入 admission baseline。
4. **合成 oracle**（HF/yiwen 均零覆盖的卡）：`oracle:synthetic:*` 链（客户端 T 态 +
   sequencer，treatment/control 各配 witness，forward/reverse 都要）。
   capability reverify 转正以此为准（先例：422 紫芒星爆 batch-027）。

准入（全部 exact 后）：`records:yiwen:admit` → 扩展 baseline（先例：3094→3373）。

## 4. 发布（合规纪律）

- 主仓库 remote push 被禁用（历史含私有语料）。发布路径：
  `bun run export:public --target <Open-YiXianCard-public-final> --replace` → public-final
  提交 → `git push origin main`。export 是 allowlist，自动剔除私有材料（analysis/corpus/TUI）。
- **push 前检查清单**：门禁全绿（cargo / test:ts / evaluator / profiles / docs-drift /
  rust:quick）；`check:worktree` clean；`AGENT_CONTEXT.md` build 号正确；无 scratch/临时
  文件；`tui.rs`/`tui_app`/ratatui 依赖只允许存在于私有树（公开树 Cargo.toml 不得引入）。
- 禁止：force-push、改历史、放宽 exact 断言（`winner`/`actorTurn`/`hpDelta` 三字段同时匹配）。

## 5. 已知坑（2026-08-07/08 实踩，先查后跑）

| 坑 | 现象 | 处置 |
| --- | --- | --- |
| 启动 Steam 应用待更新 | 客户端跳 build，全链 fail-closed | 前置查 appmanifest 两字段一致 |
| 私有树 extracted/current 落后 | 含新卡回放导出 KeyError | 轮换时同步私有树 current |
| 私有 freeze 链缺别名 | 准入卡在 prepare-current-build | 按 55bdef80^ 契约恢复 package.json 别名 |
| TUI 源/依赖误入公开树 | 边界扫描红、公开 Cargo 膨胀 | TUI 只放私有树；公开 Cargo.toml 不引 ratatui |
| 边界拆分后首次跑某门禁 | 工具链缺口首次暴露 | 先 `check:private-manifest` 对账再跑私有链 |
| 子任务超时无可见进度 | 误判卡死 | 任务 ≤1h，等待期主动轮询汇报 |

## 5.1 外部基线与首差聚类

一方原版客户端 exact 与 third-party mirror 结果分开统计。外部语料只要求相对上一次
同 build 基线 `no-new-mismatch` 且 `first divergence signature` 分布稳定；不把 server/client
drift、旧客户端残留或镜像字段损坏强行清零，也不因此改 Rust 规则。严格回放的
`winner`、`actorTurn`、`hpDelta` 三元仍必须同时匹配。

`screen_hf_stream.py` 的 receipt 会按首个 deviation 的 kind/message 聚类，保留代表样本
供 `dump-aligned-checkpoints.ts` 和原版 oracle 取证。对目录语料的 build-diff triage，首次
使用 `--corpus <dir>` 会建立 `<dir>/.corpus-index.json`；之后复用 `--index <path>`，每次
先用 fixture 路径/大小/mtime 重新计算廉价 metadata fingerprint，变更会自动拒绝旧 index
并重建；`--refresh-index` 可强制重建，避免每次重新解析数十万 JSON。

## 6. 子任务拆分纪律

- 单 subagent 任务 ≤1 小时：按卡/按根因拆；诊断与修复分离；oracle 采集单独成任务。
- 共享 worktree（多 agent 并行）：写共享文件前重读、用唯一注释锚点追加；
  各自 `bun run check:worktree -- --paths <负责路径>` 收口，不被他人改动阻塞。
- 验证跑 `bun run check:affected -- --dry-run` 按变更选门禁；发布/准入才跑全量。
