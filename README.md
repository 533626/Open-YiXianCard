# Open-YiXianCard 战斗引擎

非官方的《弈仙牌》单场战斗引擎，以 Rust 实现。输入已解析的战斗初始状态，模拟卡牌效果、状态、伤害和行动，输出战斗结果及事件。项目独立于游戏开发者和发行商；游戏名称、商标及其他第三方材料归各自权利人所有，详见 [NOTICE](NOTICE) 和 [LICENSE](LICENSE)。

## 引擎

引擎从已解析的战斗初始状态运行至终局，处理卡牌、状态、伤害、治疗、资源与行动，并提供战斗结果、事件和追踪数据。Rust 库同时支持原生和 WebAssembly 构建；规则以原作 Steam build **26315169** 为目标。

## HF 回放验证

使用 Hugging Face 数据集 [sharpobject/yxp_replays](https://huggingface.co/datasets/sharpobject/yxp_replays) 的普通、练习和天梯模式回放，按胜负、行动方与生命值变化三个字段进行精确比对。

累计分片 34251000–34285000 共 36 个分片，153,091 条可执行记录全部精确匹配；blocked 和 error 未计入匹配率。这是该批次、该版本引擎的验证结果。

## 构建与使用

安装 Rust 和 Cargo 后，在仓库根目录运行：

```bash
cargo check --manifest-path engine-rust/Cargo.toml --locked
cargo build --release --manifest-path engine-rust/Cargo.toml --locked
```

Rust 库入口及公开类型见 [engine-rust/src/lib.rs](engine-rust/src/lib.rs)。调用方负责准备符合引擎契约的战斗初始状态；部分数据加载接口以仓库根目录为输入。
