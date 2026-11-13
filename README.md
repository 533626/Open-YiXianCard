# Open-YiXianCard 战斗引擎

非官方的《弈仙牌》单场战斗引擎，以 Rust 实现。输入已解析的战斗初始状态，模拟卡牌效果、状态、伤害和行动，输出战斗结果及事件。项目独立于游戏开发者和发行商；游戏名称、商标及其他第三方材料归各自权利人所有，详见 [NOTICE](NOTICE) 和 [LICENSE](LICENSE)。

## 引擎

引擎从已解析的战斗初始状态运行至终局，处理卡牌、状态、伤害、治疗、资源与行动，并提供战斗结果、事件和追踪数据。Rust 库同时支持原生和 WebAssembly 构建；规则以原作 Steam build **26614559** 为目标。

## HF 回放验证

[sharpobject/yxp_replays](https://huggingface.co/datasets/sharpobject/yxp_replays) 的 **55,916 条**可执行回放，胜负、行动方与生命值变化三项精确匹配率 **100%**。覆盖普通、练习、天梯模式的 15 个分片（34523000–34537000）；范围外与空输入未计入匹配率。

致谢：感谢 Hugging Face 数据集 [sharpobject/yxp_replays](https://huggingface.co/datasets/sharpobject/yxp_replays) 维护者 **sharpobject** 收集并持续公开弈仙牌对局语料，为引擎校验与规则精确复现提供了宝贵的数据基准。

## 构建与使用

安装 Rust 和 Cargo 后，在仓库根目录运行：

```bash
cargo check --manifest-path engine-rust/Cargo.toml --locked
cargo build --release --manifest-path engine-rust/Cargo.toml --locked
```

Rust 库入口及公开类型见 [engine-rust/src/lib.rs](engine-rust/src/lib.rs)。调用方负责准备符合引擎契约的战斗初始状态；部分数据加载接口以仓库根目录为输入。
