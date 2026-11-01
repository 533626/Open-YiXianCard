//! 原版 oracle 引擎对照核心族的 Rust 进程内移植。
//!
//! 本模块是 `battle-evaluator/scripts/lib/` 下四个 TS 文件的逐字等价实现，
//! 供 `oracle_full_state_projection` 与 `original_oracle_diff_core` 两个 bin
//! 以及对拍 harness 使用；与 TS 的对应关系：
//!
//! - [`buff_map`]：`original-oracle-rust-buff-map.ts` —— buff-id-rust-key-map
//!   加载/漂移 fail-closed 校验、`--events-extended` 91+13 字段快照流解析、
//!   核心四元 + buff 四分类逐帧单侧对照、buff-category-archive 消费。
//! - [`events`]：`rust-replay-events.ts` —— 事件流/扩展流的进程内构造
//!   （直接调 `run_replay_fixture_with_parity_events` /
//!   `run_replay_fixture_with_detailed_events`，不再 spawn `replay_slice`）。
//! - [`full_state`]：`original-oracle-full-state-rust.ts`（full-state 基线同形
//!   投影 + modeledBuffs 限定 + unobservedBuffIds），并移植 diff 依赖的
//!   `original-oracle-full-state.ts` 两个纯函数（观察投影与全状态比较）。
//! - [`diff`]：`original-oracle-diff.ts` —— trace JSONL → OriginalOracleRun
//!   解析（batch/run/status/runtimeProvenance/observations）、Rust 引擎对照
//!   （blocked/error 分类）、golden/diagnosis 逐 case 比较。
//! - [`synthetic`]：`synthetic-oracle-rust-diff.ts`（+ synthetic-oracle-witness
//!   纯函数与 synthetic-oracle 输入构造）—— 单个 synthetic case 的引擎判定，
//!   终局六元 + 见证投影逐键对照（新批次 vs 原作观测；已准入回归 vs 冻结基线）。
//!
//! 对齐原则：camelCase 字段名、错误文案、判定语义与 TS 逐字一致；JSON 记录
//! （trace buffs 等）的键序依赖 `serde_json` 的 `preserve_order` feature，
//! 与 TS `JSON.parse` 的插入序一致，保证对拍可逐字节比较。

pub mod buff_map;
pub mod diff;
pub mod events;
pub mod full_state;
pub mod synthetic;

#[cfg(test)]
mod tests;
