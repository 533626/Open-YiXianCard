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
   数据面：先统计 version 分布再筛，避免旧 build 尾流混入。
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

## 6. 子任务拆分纪律

- 单 subagent 任务 ≤1 小时：按卡/按根因拆；诊断与修复分离；oracle 采集单独成任务。
- 共享 worktree（多 agent 并行）：写共享文件前重读、用唯一注释锚点追加；
  各自 `bun run check:worktree -- --paths <负责路径>` 收口，不被他人改动阻塞。
- 验证跑 `bun run check:affected -- --dry-run` 按变更选门禁；发布/准入才跑全量。
