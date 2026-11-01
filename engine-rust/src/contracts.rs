//! 引擎线合同的 JSON Schema 导出。
//!
//! Schema 一律由 serde 结构体经 schemars 生成，入库
//! `battle-evaluator/generated/contracts/`；[`check`] 逐字节比对入库文件，
//! 类型改动未重新导出即失败。Python/JS 消费方只按这些 Schema 校验，
//! 不重声明字段清单（TS 镜像 `RUST_SNAPSHOT_FIELDS` 等随迁移退役）。

use std::fs;
use std::path::Path;

use schemars::Schema;

use crate::identity::EngineIdentity;
use crate::replay::wire::{
    AdmissionReport, EventStream, ExtendedEventStream,
};

/// 一条线合同的导出项：入库文件名与对应 serde 类型的 Schema。
pub struct WireContract {
    pub file_name: &'static str,
    pub schema: Schema,
}

/// 当前引擎对外 JSON 线合同全集。新增线协议时在此登记。
pub fn wire_contracts() -> Vec<WireContract> {
    vec![
        WireContract {
            file_name: "replay-events.schema.json",
            schema: schemars::schema_for!(EventStream),
        },
        WireContract {
            file_name: "replay-events-extended.schema.json",
            schema: schemars::schema_for!(ExtendedEventStream),
        },
        WireContract {
            file_name: "replay-admission-report.schema.json",
            schema: schemars::schema_for!(AdmissionReport),
        },
        WireContract {
            file_name: "engine-identity.schema.json",
            schema: schemars::schema_for!(EngineIdentity),
        },
    ]
}

fn render(contract: &WireContract) -> String {
    let mut json = serde_json::to_string_pretty(&contract.schema)
        .expect("wire contract schema serializes");
    json.push('\n');
    json
}

/// 把全部线合同 Schema 写入目录（已存在则覆盖）。
pub fn write(dir: &Path) -> Result<(), String> {
    fs::create_dir_all(dir).map_err(|e| format!("create {}: {e}", dir.display()))?;
    for contract in wire_contracts() {
        let path = dir.join(contract.file_name);
        fs::write(&path, render(&contract))
            .map_err(|e| format!("write {}: {e}", path.display()))?;
    }
    Ok(())
}

/// 逐字节校验目录内已入库 Schema 与当前类型一致。
/// 缺文件或字节不同都算过期；只读，不写盘。
pub fn check(dir: &Path) -> Result<(), String> {
    let mut stale = Vec::new();
    for contract in wire_contracts() {
        let path = dir.join(contract.file_name);
        let expected = render(&contract);
        match fs::read_to_string(&path) {
            Ok(actual) if actual == expected => {}
            Ok(_) => stale.push(format!("{}: bytes differ — re-run export_contracts", path.display())),
            Err(e) => stale.push(format!("{}: missing ({e})", path.display())),
        }
    }
    if stale.is_empty() {
        Ok(())
    } else {
        Err(stale.join("\n"))
    }
}
