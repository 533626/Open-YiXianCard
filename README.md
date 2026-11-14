# Open-YiXianCard 战斗引擎

非官方的《弈仙牌》单场战斗引擎，以 Rust 实现。输入已解析的战斗初始状态，模拟卡牌效果、状态、伤害和行动，输出战斗结果及事件。项目独立于游戏开发者和发行商；游戏名称、商标及其他第三方材料归各自权利人所有，详见 [NOTICE](NOTICE) 和 [LICENSE](LICENSE)。

## 引擎

引擎从已解析的战斗初始状态运行至终局，处理卡牌、状态、伤害、治疗、资源与行动，并提供战斗结果、事件和追踪数据。Rust 库同时支持原生和 WebAssembly 构建；规则以原作 Steam build **26614559** 为目标。

## 本地回放转换

公开树附带 `replay_import`：读取本地原版客户端 `.bin` 对局记录，或第三方镜像 / Hugging Face 风格的 JSON / JSONL 记录，转换成引擎契约后直接运行。它只依赖公开的引擎卡表与等级生命表，不需要评测层或任何私有语料。

安装 Rust 和 Cargo 后，在仓库根目录运行：

```bash
cargo run --release --manifest-path engine-rust/Cargo.toml --bin replay_import -- <record.bin|record.json|record.jsonl>
```

可选参数：

- `--round N`：只跑第 N 轮；
- `--write-fixtures DIR`：同时把每轮 fixture 写到 `DIR`。

输出为逐轮的 `expected` / `actual` / `exact` JSON 报告；任一可执行轮次不精确匹配时退出码非 0。毫秒级时间戳、玩家 UID、姓名等身份字段不进入 fixture。记录只在本地读取和处理，工具不上传数据，仓库也不分发任何对局语料。

## 回放验证

开发仓使用本地采集的记录和[sharpobject/yxp_replays](https://huggingface.co/datasets/sharpobject/yxp_replays) 的 **138,805 条**可执行回放做内部验证，胜负、行动方与生命值变化三项精确匹配率 **100%**，覆盖普通、练习、天梯模式的 36 个分片（34523000–34558000）。该语料及其 fixture 属于私有工程证据，不随公开树分发；持有对应记录的读者可以用上面的转换器在本地复跑同一流程。

致谢：感谢 Hugging Face 数据集 [sharpobject/yxp_replays](https://huggingface.co/datasets/sharpobject/yxp_replays) 维护者 **sharpobject** 收集并持续公开弈仙牌对局语料，为引擎校验与规则精确复现提供了宝贵的数据基准。

## 构建与使用

安装 Rust 和 Cargo 后，在仓库根目录运行：

```bash
cargo check --manifest-path engine-rust/Cargo.toml --locked
cargo build --release --manifest-path engine-rust/Cargo.toml --locked
cargo test --manifest-path engine-rust/Cargo.toml --locked
```

公开树的测试不依赖私有 fixture。Rust 库入口及公开类型见 [engine-rust/src/lib.rs](engine-rust/src/lib.rs)。调用方负责准备符合引擎契约的战斗初始状态；部分数据加载接口以仓库根目录为输入。
