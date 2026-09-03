# Open-YiXianCard

Open-YiXianCard 是一个非官方、本地优先的《弈仙牌》单场战斗模拟器与浏览器工作台。给定已经确定的战斗初始状态后，它可以运行规则、查看时间线，并导入受支持的本地回放做对照。

本项目独立于游戏开发者和发行商。权利与署名说明见 [`NOTICE`](NOTICE) 与 [`LICENSE`](LICENSE)。

## 范围与限制

- 从已解析的战斗初始状态开始，模拟战斗钩子、回合、卡牌效果、伤害、治疗、资源、状态变化、死亡与终局结果。
- 不模拟商店、抽换牌、炼化、成长、匹配、排名、账号进度或战后奖励。
- 不支持或证据不足的机制会 fail closed，不用近似行为冒充精确复现。
- 回放校验严格比对胜负、行动方和生命值变化，不为掩盖偏差而放宽断言。
- 当前兼容原作 Steam build **25093011**；游戏更新后可能需要重新审校。

## 本地运行

应用是静态、仅本地运行的浏览器工作台。战斗计算在 Web Worker 中运行；不需要账号、不上传构筑或回放，也不提供服务端战斗 API。发布产物保持零 fixture，不捆绑仓库回放或 demo 数据。

需要 [Bun](https://bun.sh/)（版本见 [`.bun-version`](.bun-version)）、Rust/Cargo，以及 `wasm32-unknown-unknown` target：

```bash
bun install --frozen-lockfile
bun run build:ui
bun run serve
```

打开 `http://localhost:3001`。`bun run build:site` 会在 `dist/` 生成经审计的静态产物，不执行部署。

## 贡献

规则改动需要原版证据和精确的最小契约。改变行为前请阅读 [`docs/PRODUCT_ARCHITECTURE.md`](docs/PRODUCT_ARCHITECTURE.md) 与 [`research/original-game/BATTLE_RULE_INDEX.md`](research/original-game/BATTLE_RULE_INDEX.md)，再按 `bun run check:affected -- --dry-run` 选择检查。

公开仓库不是完整游戏客户端、在线服务或官方项目。游戏名称、商标、客户端文件、美术、音频、字体与其他第三方材料归各自权利人所有。
