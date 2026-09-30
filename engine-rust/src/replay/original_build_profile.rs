//! 引擎只对齐最新 Steam build：不按 fixture 录制 build 分叉规则。build 画像契约
//! （`shared/data/original-build-profiles.json`）只用于声明项目目标 build。
use serde::Deserialize;
use std::sync::LazyLock;

const ORIGINAL_BUILD_PROFILES_CONTRACT: &str =
    include_str!("../../../shared/data/original-build-profiles.json");

#[derive(Debug, Deserialize)]
struct OriginalBuildProfilesContract {
    #[serde(rename = "projectTargetSteamBuild")]
    project_target_steam_build: String,
}

static PROJECT_TARGET_STEAM_BUILD: LazyLock<String> = LazyLock::new(|| {
    serde_json::from_str::<OriginalBuildProfilesContract>(ORIGINAL_BUILD_PROFILES_CONTRACT)
        .expect("valid original-build profile contract")
        .project_target_steam_build
});

/// 契约声明的当前 target build；调用方不得另行硬编码 build 号。
pub(super) fn project_target_steam_build() -> &'static str {
    PROJECT_TARGET_STEAM_BUILD.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_target_is_numeric() {
        let target = project_target_steam_build();
        assert!(!target.is_empty() && target.bytes().all(|byte| byte.is_ascii_digit()));
    }
}
