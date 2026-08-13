<!-- topic: webui -->

# UI iteration workflow v1

本文是浏览器 UI refinement 的有限工作流。它用于把可复现的使用问题收敛成小批次修复，
不是无限美化清单，也不是产品架构或战斗规则的替代品。

## 边界与目标

浏览器 UI 是 Rust canonical engine、WASM/Worker 与公开 contracts 的适配和展示层。UI 可以
改善布局、状态表达、焦点、滚动和操作回路，但不得修改 solver 算法、payload、engine 规则、
牌序语义、回放事实或 `winner / actorTurn / hpDelta` 的精确契约。任何规则争议回到原版证据与
Rust 最小契约；不要为了让 UI 测试通过而放宽引擎断言。

每轮只选 1–3 个最高价值问题。公开核心方法如下：

1. 用有限的截图和无先验玩家判官测试发现“看得到但用不到”“状态不清楚”或“操作回路断裂”的
   雷达信号。
2. 把截图、DOM 几何、滚动、焦点、控制台和测试输出合成证据；截图不是根因证明。
3. 每条 finding 先验真因，再做最小一致改动；不要以视觉重排掩盖状态或数据问题。
4. 用硬指标作为防退步棘轮：无 body/document 横向溢出、主操作可达、内部滚动有边界、焦点
   可见、等待/完成/错误状态明确。指标通过不等于 UX 验收通过。
5. 修复者和复测者分离。独立复测确认行为、视口和长内容；修复者不替复测者宣称体验验收。

## 固定覆盖矩阵

每轮按需要裁剪，但至少记录命中的格子：

| 工作流 | empty | configured | running | done | error | 重点视口 |
| --- | --- | --- | --- | --- | --- | --- |
| 构筑战斗 | 未选角色/牌 | 双方可推演 | Worker 战斗中 | 帧导航与模块 | 取消/失败 | 1920×1080、1280×800 |
| 求解应用 | 未求解 | 有输入可求解 | Worker 求解中 | 候选、应用、恢复基准 | 失败/取消 | 1920×1080、1280×800 |
| 打靶 | 无构筑 | 可运行构筑 | 单/多构筑运行 | 图表、阶梯、对比 | no-trace/失败 | 1920×1080、1280×800 |
| 导入 | 空入口 | 选择来源 | 解码/加载中 | 回放结果与一致性 | 损坏/不兼容 | 1920×1080、1280×800 |

除了主路径，还要抽查 `disabled`、`selected`、拖拽、键盘 `focus-visible`、长中文名称、
内部滚动边界和 reduced-motion 相关状态。运行态瞬时按钮（例如取消、重试、应用）必须在
DOM 重建后重新定位，不应把旧节点引用当作行为证据。

## 一轮的证据循环

### 1. 先建立基线

先读 `AGENTS.md`、`docs/AGENT_CONTEXT.md` 和 frontend-design skill，再查看工作区状态与
真实 render path。对目标视口记录：

- `document.documentElement.scrollWidth/clientWidth`、`body` scroll 状态和关键区域 rect；
- 主操作、状态条、首屏结果是否在可达区域；内部滚动容器的 `scrollHeight/clientHeight`；
- 当前焦点、`focus-visible`、disabled/selected 语义和控制台错误；
- 1920×1080 的 dense desktop workbench 与 1280×800 的窄桌面内部滚动行为。

必要时先用浏览器 smoke 和 self-audit 产出截图，再让无先验玩家按目标任务操作。无先验玩家
的“找不到/不敢点/点完不知道发生什么”是雷达信号；它不能单独证明根因。

### 2. 形成 finding

每条 finding 必须能回到一个观察事实，并写清楚用户影响、触发状态、视口、根因假设和最小
修复范围。优先修复阻断可达性、错误状态或数据上下文丢失的问题；颜色、间距和装饰排在后面。

推荐报告格式：

```text
Finding: [工作流/状态] 一句话描述
Evidence: [viewport] [截图或 DOM/console/测试读数]
Impact: 用户无法完成的具体动作
Root cause: 代码路径或布局约束；未确认时标记为假设
Change: 任务相关的最小文件/行为变化
Verification: 命令、关键输出、仍未覆盖的状态
Risk: 兼容视口、焦点、长内容或复测注意事项
```

### 3. 实施与复测

只改一个连贯表面，保留 TS Engine boundary 与现有语义颜色。对每个变更同时检查 empty、
configured、running、done、error、disabled、selected 和长内容。CSS 改动后运行 `build:ui`；
涉及缓存时让 `index.html` 与构建产物版本一致。聚焦测试应锁定布局关系、状态上下文和可逆
操作，而不是锁定脆弱的截图像素。

独立复测要重新加载页面并按用户动作走完整回路。以下情况常是假阳性，必须先复核状态事实：

- 旧 DOM uid 或旧节点句柄：页面重渲染后引用自然失效，不能据此判定点击失败；
- 瞬时运行按钮：开始/取消/重试会随状态切换，需按 `data-action` 和新 DOM 重新定位；
- 自动化 timeout：先检查状态、结果、请求 ID 和控制台，再判断是否真的失败；
- `aria-valuetext` 乱码：先核对源码 UTF-8、HTML charset 与浏览器实际 DOM 文本；
- 鼠标截图看不到控件：确认是否只是内部滚动位置、焦点位置或窗口尺寸问题；
- 无先验玩家误解：区分文案/层级问题与真实状态或 engine/solver 问题，不跨层修复。

## 停手与收口

每轮最多处理 1–3 项。若只剩 nits、连续两轮没有可量化改善，或已经完成最多 3–4 轮，
停止继续 UI 修改，保留剩余风险并交给独立复测。不要因为还有审美偏好就扩大范围。

收口至少包括：聚焦 UI tests、`build:ui`、适用的 browser smoke、docs drift、`git diff --check`
和 scoped worktree 检查。完整 `check:ui` 若被既有 generated-card-registration 或环境问题
阻断，要记录原始失败，不得放宽门禁、改 generated 数据或把失败标为通过。

最后报告用户可见变化、覆盖的工作流/状态/视口、硬指标与测试输出、独立复测结论和剩余风险。
按逻辑批次提交：UI 修复与测试一批，工作流文档一批；不改历史，不吸收无关 dirty files。
