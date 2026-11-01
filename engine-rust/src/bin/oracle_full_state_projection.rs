//! `oracle_full_state_projection`：用 Rust canonical 引擎（进程内 detailed
//! 事件流）为指定 fixture 生成与冻结原版全状态基线同形的投影 JSON。
//!
//! 输出（单行 JSON，camelCase，与 TS runOriginalOracleRustFullState 同形）：
//! `{"projection": {...}, "unobservedBuffIds": [...]}`
//!
//! 用法：
//! ```text
//! oracle_full_state_projection --case-id <id> --fixture <fixture.json> \
//!   [--fixture-label <label>] [--repo-root <dir>] [--declarations <path>] \
//!   [--buff-map <path>] [--buff-archive <path>]
//! ```

use std::path::{Path, PathBuf};

use yixian_engine::oracle::buff_map::load_rust_buff_key_map;
use yixian_engine::oracle::full_state::{
    load_original_oracle_contract_buff_declarations, run_original_oracle_rust_full_state,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn default_path(repo_root: &Path, relative: &str) -> PathBuf {
    repo_root.join(relative)
}

fn run() -> Result<(), String> {
    let mut case_id: Option<String> = None;
    let mut fixture: Option<PathBuf> = None;
    let mut fixture_label: Option<String> = None;
    let mut repo_root = PathBuf::from(".");
    let mut declarations: Option<PathBuf> = None;
    let mut buff_map: Option<PathBuf> = None;
    let mut buff_archive: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        let mut value = || args.next().ok_or_else(|| format!("{arg} requires a value"));
        match arg.as_str() {
            "--case-id" => case_id = Some(value()?),
            "--fixture" => fixture = Some(PathBuf::from(value()?)),
            "--fixture-label" => fixture_label = Some(value()?),
            "--repo-root" => repo_root = PathBuf::from(value()?),
            "--declarations" => declarations = Some(PathBuf::from(value()?)),
            "--buff-map" => buff_map = Some(PathBuf::from(value()?)),
            "--buff-archive" => buff_archive = Some(PathBuf::from(value()?)),
            "--help" | "-h" => {
                print_usage();
                return Ok(());
            }
            other => return Err(format!("unknown option: {other}")),
        }
    }
    let case_id = case_id.ok_or_else(|| "--case-id is required")?;
    let fixture_path = fixture.ok_or_else(|| "--fixture is required")?;
    let declarations_path = declarations.unwrap_or_else(|| {
        default_path(
            &repo_root,
            "battle-evaluator/data/modeled-original-buffs.json",
        )
    });
    let build_profiles_path = default_path(
        &repo_root,
        "battle-evaluator/data/original-build-profiles.json",
    );
    let evidence_manifest_path = default_path(&repo_root, "research/original-game/EVIDENCE_MANIFEST.json");
    let map_path = buff_map.unwrap_or_else(|| {
        default_path(&repo_root, "battle-evaluator/generated/buff-id-rust-key-map.json")
    });
    let archive_path = buff_archive.unwrap_or_else(|| {
        default_path(&repo_root, "battle-evaluator/generated/buff-category-archive.json")
    });

    let map = load_rust_buff_key_map(&map_path, &archive_path)?;
    let declarations = load_original_oracle_contract_buff_declarations(
        &declarations_path,
        &build_profiles_path,
        &evidence_manifest_path,
    )?;
    let fixture = yixian_engine::load_fixture_file(&fixture_path).map_err(|error| error.to_string())?;
    let fixture_label = fixture_label.unwrap_or_else(|| case_id.clone());
    let full_state = run_original_oracle_rust_full_state(
        &case_id,
        &fixture,
        &fixture_label,
        &declarations,
        &map,
    )?;
    println!("{}", full_state.to_json());
    Ok(())
}

fn print_usage() {
    println!(
        "Usage:\n  \
oracle_full_state_projection --case-id <id> --fixture <fixture.json> [--fixture-label <label>] \
[--repo-root <dir>] [--declarations <path>] [--buff-map <path>] [--buff-archive <path>]\n\n\
Emits one JSON line: {{\"projection\": ..., \"unobservedBuffIds\": [...]}}."
    );
}
