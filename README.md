# Open-YiXianCard

Open-YiXianCard 是一个非官方、本地优先的《弈仙牌》单场战斗模拟器与浏览器工作台。它聚焦于一场已经确定初始状态的战斗:配置双方、运行 canonical 规则引擎、检视结果时间线,并导入你自己的受支持回放数据做本地对照。

本项目为独立项目,与游戏开发者或发行商无隶属、无背书、无赞助关系。权利与署名说明见 [`NOTICE`](NOTICE) 与 [`LICENSE`](LICENSE)。

## 范围与限制

- 引擎从已经解析好的战斗初始状态开始,模拟战斗开始钩子、回合、卡牌效果、伤害、治疗、资源、状态变化、死亡与终局结果。
- 它**不**模拟游戏的商店、抽换牌、炼化、成长、匹配、排名、账号进度或战后奖励。
- Rust 是规则权威。TypeScript 是兼容档案,必须跟随 Rust,不能作为新规则的证据。
- 不支持或证据不足的机制 fail closed。近似行为不会被表述为精确复现。
- 精确回放检查完整保留 `winner`、`actorTurn`、`hpDelta` 契约;本项目不会为掩盖偏差而放宽这些断言。
- 兼容性绑定到当前原作 Steam build **24666769**（见 `research/original-game/EVIDENCE_MANIFEST.json` 的机器可读证据快照）。更新的游戏 build 可能需要新的准入与兼容性审校。

## 浏览器行为

浏览器 UI 是一个静态、仅本地的应用。战斗计算在 Web Worker 中以编译为 WebAssembly 的 Rust 引擎运行。用户可以从空白构筑开始,或显式选择受支持的本地 JSON/回放文件。应用不需要账号,不上传构筑或回放,也不提供服务端战斗 API。

发布产物刻意保持**零 fixture**:不包含任何仓库回放数据或 fixture 索引,一旦私有 fixture 内容或网络上传路径进入 bundle,release 审计即 fail closed。不捆绑任何 demo 回放。

网站发布与 Cloudflare 部署**不属于本次变更**。本仓库不声明任何公开站点 URL。构建与产物检查均为本地操作,除非未来有另行审批的发布流程。

## 本地构建与运行

依赖:

- [Bun](https://bun.sh/),版本行见 [`.bun-version`](.bun-version);
- Rust/Cargo 与浏览器构建所需的 `wasm32-unknown-unknown` target。

安装依赖并构建开发浏览器 bundle:

```bash
bun install --frozen-lockfile
bun run build:ui
bun run serve
```

然后打开 `http://localhost:3001`。开发服务器仅本地运行。`bun run build:site` 会在 `dist/` 生成经审计的静态产物;它不执行部署。

常用公开检查:

```bash
bun run check:public-boundary
bun run check:docs-drift
bun run audit:boundaries
bun run test:release
bun run build:site
bun run check:release
bun run check:ui
bun run check:rust:quick
bun run check:evaluator:types
bun run check:ts:types
bun run test:ts
```

回放 corpus 与回放派生的准入/oracle 证据是私有工程材料,有意从公开源码边界中排除。完整回放准入与原版客户端提取属于私有 companion。Rust 依赖 corpus 的测试使用 opt-in 的 `--features private-fixtures` 开关,不属于默认公开检查的一部分。不得通过把回放文件复制进本仓库或加入 release 产物来使其公开。

## 贡献规则

规则改动需要原版证据、精确的最小契约,以及 TS 兼容更新之前的精确 Rust 实现。改变行为前,请先阅读 [`docs/PRODUCT_ARCHITECTURE.md`](docs/PRODUCT_ARCHITECTURE.md)、[`engine-ts/README.md`](engine-ts/README.md) 与 [`research/original-game/BATTLE_RULE_INDEX.md`](research/original-game/BATTLE_RULE_INDEX.md)。公开文档聚焦引擎、契约、浏览器 UI 与稳定的规则开发工作流;内部分析结果与操作交接保留在私有 companion。

公开仓库不是完整游戏客户端、不是在线服务、也不是官方游戏项目。游戏名称、商标、客户端文件、美术、音频、字体与其他第三方材料归各自权利人所有。
