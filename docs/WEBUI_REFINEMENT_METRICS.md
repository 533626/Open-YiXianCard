<!-- topic: webui -->

# Web UI refinement metrics

本页用于评估浏览器 UI 改进是否真正改善使用面。指标只约束 UI / adapter / feedback，
不作为放宽 TS Engine 或 Rust replay-parity 的理由。

## 验收指标

| 目标 | 指标 | 当前目标值 |
| --- | --- | --- |
| 资源走势补齐 | 战斗结果含 `.resource-flow`，至少展示生命差、资源面积、防护差、负面面积中有意义的行 | 有行动帧时 100% 展示；生命差永远展示 |
| 当前帧定位 | 资源走势含 `.flow-cursor` 和 `.flow-marker`，跟随 `state.frameIndex` | 上一动/下一动/进度点跳转后游标准确 |
| 连续操作距离 | solver 主链路为 `.solver-mode-select` -> `.solver-run` -> `apply-solver-best` | 三个控件同一行；桌面宽度下水平距离 <= 260px |
| 选项合并 | solver 顶层求解按钮数量 | `data-action="solve-*"` 顶层入口 <= 1，模式复用下拉选择 |
| 等待提示 | 点击求解后先渲染 `.solver-status.running`，再执行同步搜索 | 首次反馈 <= 100ms；求解中禁用模式和求解按钮 |
| 求解可观测性 | 求解完成显示耗时、评估量和预算 | `.solver-status.done` 包含 elapsedMs 与 evaluated/maxEvaluations |
| 默认求解预算 | 默认快路径不误触重型穷举 | `排序建议` <= 2,000 eval；`卡池建议` <= 15,000 eval；`仙命构筑` <= 5,000 eval；`穷举牌序` 明确标注 |
| fixture 导入 | 按编号选择 fixture 后立即跑 UI 模拟并显示一致性 | `e63*` 可筛出 `e63lwvs/round-*`；`.fixture-consistency` 展示 UI=Engine 或 UI!=Engine |
| adapter 一致性 | UI adapter 输出与直接 engine replay 对比 | winnerSide、actorTurnCount、hpDeltaP1MinusP2、finalHp 全等 |
| TS/Rust 存档复用 | WebUI 导出/导入 Rust TUI 单方构筑存档 | JSON 为 `schemaVersion: 2`、`kind: tuiPlayerBuild`；Rust TUI 可直接读 |
| 调步操作 | 战斗导航按钮与方向键支持逐动切换 | 按钮最小 38x34px；`ArrowLeft/Up` 上一动，`ArrowRight/Down` 下一动，输入框聚焦时不抢键；引擎透视光标到达中线后居中跟随，手动滚动后下一次调步回中 |
| 布局稳定 | 关键控件不引发横向滚动或文本溢出 | 1180px 宽桌面无 body 横向滚动；按钮文本不截断 |
| 空白率上限 | 首屏关键面板非交互空白不吞掉视口 | 1440x900 下无战斗结果时不渲染右侧 60% 空战斗面板；战斗态 `.player-panel` 内裸露背景占比 <= 15% |
| 走势可读性 | `.flow-spark` 不是发丝线 | 渲染高度 >= 56px；含零线、面积填充、游标、当前点 |
| 字号层级 | 避免 9-12px 一档压平 | 核心操作按钮字号/高度明显高于次要控件；新增小字不得低于现有最小档 |
| 求解闭环引导 | 求解完成后下一步明确 | `.solver-status.done` 出现时 `apply-solver-best` 按钮有 `primary` 强调态且可点击 |
| 空槽可辨识 | 空卡槽与数值输入框视觉区分 | `.card-face.empty` 使用卡槽纹理和居中添加标记，不与普通表单控件同形 |
| DevTools 全流程跳动 | 固定 1440x900 走导入 fixture -> 求解 -> 应用 -> 调整 -> 重新战斗 -> 调步 -> 换 fixture | Chrome DevTools trace 的 lab CLS = 0；rect 采样中 `open-fixture-panel`、`step-next-prev-keyboard`、`open-fixture-panel-for-switch` 的 `maxDeltaPx <= 1`；`solver-run maxDeltaPx <= 25` |
| 模式切换跳动归类 | 导入/应用/重新战斗这类 setup/battle 模式切换单独统计 | `import-round16-and-run`、`apply-solver-best`、`rerun-after-adjust` 可有大布局变化，但必须在报告中与非模式切换跳动分开，不混入稳定性达标项 |

## 走查口径

浏览器走查固定两个视口：`1180 x 800` 与 `1440 x 900`；一键使用面门禁为 `bun run check:ui`。
