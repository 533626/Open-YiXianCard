use serde::Deserialize;
use std::collections::{BTreeMap, BTreeSet};
use std::sync::LazyLock;

const ORIGINAL_BUILD_PROFILES_CONTRACT: &str =
    include_str!("../../../shared/data/original-build-profiles.json");

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum OriginalBuildCapability {
    Fate324GrantsCloudChain,
    Fate335GrantsMomentum,
    SpiritFormationEchoUsesBaseCard,
    HexagramLossDoesNotSpendCard422StarPower,
    LostHexagramLedgerAccumulatesPositiveLoss,
    DreamThunderRefundsHexagramLoss,
}

impl OriginalBuildCapability {
    fn contract_key(self) -> &'static str {
        match self {
            Self::Fate324GrantsCloudChain => "fate324GrantsCloudChain",
            Self::Fate335GrantsMomentum => "fate335GrantsMomentum",
            Self::SpiritFormationEchoUsesBaseCard => "spiritFormationEchoUsesBaseCard",
            Self::HexagramLossDoesNotSpendCard422StarPower => {
                "hexagramLossDoesNotSpendCard422StarPower"
            }
            Self::LostHexagramLedgerAccumulatesPositiveLoss => {
                "lostHexagramLedgerAccumulatesPositiveLoss"
            }
            Self::DreamThunderRefundsHexagramLoss => "dreamThunderRefundsHexagramLoss",
        }
    }
}

const ENGINE_CAPABILITIES: [OriginalBuildCapability; 6] = [
    OriginalBuildCapability::Fate324GrantsCloudChain,
    OriginalBuildCapability::Fate335GrantsMomentum,
    OriginalBuildCapability::SpiritFormationEchoUsesBaseCard,
    OriginalBuildCapability::HexagramLossDoesNotSpendCard422StarPower,
    OriginalBuildCapability::LostHexagramLedgerAccumulatesPositiveLoss,
    OriginalBuildCapability::DreamThunderRefundsHexagramLoss,
];

const REQUIRED_CURRENT_HEXAGRAM_CAPABILITIES: [OriginalBuildCapability; 3] = [
    OriginalBuildCapability::HexagramLossDoesNotSpendCard422StarPower,
    OriginalBuildCapability::LostHexagramLedgerAccumulatesPositiveLoss,
    OriginalBuildCapability::DreamThunderRefundsHexagramLoss,
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct OriginalBuildProfile {
    profile_index: usize,
}

impl OriginalBuildProfile {
    pub(super) fn has(self, capability: OriginalBuildCapability) -> bool {
        *original_build_profiles()
            .expect("validated original-build profile contract")
            .profiles[self.profile_index]
            .capabilities
            .get(capability.contract_key())
            .expect("engine capability is present in validated contract")
    }

    /// 对局录制 build 的数值形式，用于按 build 生效的数值门控（如弯弓射虎）。
    pub(super) fn steam_build_number(self) -> u64 {
        original_build_profiles()
            .expect("validated original-build profile contract")
            .profiles[self.profile_index]
            .steam_build
            .parse()
            .expect("original-build profile steamBuild is numeric")
    }
}

pub(super) fn resolve_original_build_profile(
    steam_build: Option<&str>,
) -> Result<OriginalBuildProfile, String> {
    let contract = original_build_profiles()?;
    let requested = steam_build.unwrap_or(contract.project_target_steam_build.as_str());
    let profile_index = contract
        .profiles
        .iter()
        .position(|profile| profile.steam_build == requested)
        .ok_or_else(|| {
            format!("unsupported original Steam build {requested}; no audited capability profile")
        })?;
    let profile = &contract.profiles[profile_index];
    let is_release_target = contract
        .runtime_supported_steam_builds
        .iter()
        .any(|build| build == requested);
    let is_complete_historical_profile = REQUIRED_CURRENT_HEXAGRAM_CAPABILITIES
        .iter()
        .all(|capability| profile.capabilities[capability.contract_key()]);
    if !is_release_target && !is_complete_historical_profile {
        return Err(format!(
            "unsupported original Steam build {requested}; release target: {}; historical profile is incomplete",
            contract.runtime_supported_steam_builds.join(", "),
        ));
    }
    for capability in REQUIRED_CURRENT_HEXAGRAM_CAPABILITIES {
        if !profile.capabilities[capability.contract_key()] {
            return Err(format!(
                "unsupported original Steam build {requested}; missing required runtime capability {}",
                capability.contract_key(),
            ));
        }
    }
    Ok(OriginalBuildProfile { profile_index })
}

/// 契约声明的当前 target build；调用方不得另行硬编码 build 号。
pub(super) fn project_target_steam_build() -> &'static str {
    original_build_profiles()
        .expect("validated original-build profile contract")
        .project_target_steam_build
        .as_str()
}

/// 最近一个能力不完整的历史 build，作拒绝样例；
/// 换代后 profiles 更新，样例自动跟随，无需手改测试。
#[cfg(test)]
pub(super) fn latest_retired_steam_build() -> Option<&'static str> {
    let contract = original_build_profiles().ok()?;
    contract
        .profiles
        .iter()
        .rev()
        .map(|profile| profile.steam_build.as_str())
        .find(|build| {
            let profile = contract
                .profiles
                .iter()
                .find(|profile| profile.steam_build == *build)
                .expect("iterated build has profile");
            REQUIRED_CURRENT_HEXAGRAM_CAPABILITIES
                .iter()
                .any(|capability| !profile.capabilities[capability.contract_key()])
        })
}

#[derive(Debug, Deserialize)]
struct OriginalBuildProfilesContract {
    #[serde(rename = "schemaVersion")]
    schema_version: u64,
    #[serde(rename = "projectTargetSteamBuild")]
    project_target_steam_build: String,
    #[serde(rename = "runtimeSupportedSteamBuilds")]
    runtime_supported_steam_builds: Vec<String>,
    capabilities: BTreeMap<String, serde_json::Value>,
    profiles: Vec<OriginalBuildProfileContract>,
}

#[derive(Debug, Deserialize)]
struct OriginalBuildProfileContract {
    #[serde(rename = "steamBuild")]
    steam_build: String,
    capabilities: BTreeMap<String, bool>,
}

static ORIGINAL_BUILD_PROFILES: LazyLock<Result<OriginalBuildProfilesContract, String>> =
    LazyLock::new(load_original_build_profiles);

fn original_build_profiles() -> Result<&'static OriginalBuildProfilesContract, String> {
    ORIGINAL_BUILD_PROFILES.as_ref().map_err(Clone::clone)
}

fn load_original_build_profiles() -> Result<OriginalBuildProfilesContract, String> {
    let contract: OriginalBuildProfilesContract =
        serde_json::from_str(ORIGINAL_BUILD_PROFILES_CONTRACT)
            .map_err(|error| format!("invalid original-build profile contract: {error}"))?;
    if contract.schema_version != 2
        || contract.capabilities.is_empty()
        || !is_numeric_build(&contract.project_target_steam_build)
        || contract.runtime_supported_steam_builds.is_empty()
    {
        return Err("invalid original-build profile contract header".to_string());
    }

    let capability_keys = contract
        .capabilities
        .keys()
        .cloned()
        .collect::<BTreeSet<_>>();
    for capability in ENGINE_CAPABILITIES {
        if !capability_keys.contains(capability.contract_key()) {
            return Err(format!(
                "original-build profile contract lacks engine capability {}",
                capability.contract_key(),
            ));
        }
    }

    let mut builds = BTreeSet::new();
    for profile in &contract.profiles {
        let profile_capability_keys = profile
            .capabilities
            .keys()
            .cloned()
            .collect::<BTreeSet<_>>();
        if !is_numeric_build(&profile.steam_build)
            || !builds.insert(profile.steam_build.clone())
            || profile_capability_keys != capability_keys
        {
            return Err(format!(
                "invalid or incomplete original-build profile {}",
                profile.steam_build,
            ));
        }
    }
    if !builds.contains(&contract.project_target_steam_build) {
        return Err("original-build profile contract lacks its project target".to_string());
    }
    let mut runtime_builds = BTreeSet::new();
    for build in &contract.runtime_supported_steam_builds {
        if !is_numeric_build(build)
            || !runtime_builds.insert(build.clone())
            || !builds.contains(build)
        {
            return Err(format!(
                "invalid runtime-supported original Steam build {build}",
            ));
        }
    }
    if !runtime_builds.contains(&contract.project_target_steam_build) {
        return Err("original-build project target is not runtime-supported".to_string());
    }
    Ok(contract)
}

fn is_numeric_build(build: &str) -> bool {
    !build.is_empty() && build.bytes().all(|byte| byte.is_ascii_digit())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_release_target_and_complete_historical_profiles() {
        let contract = original_build_profiles().expect("build profile contract loads");
        let target = project_target_steam_build();
        assert_eq!(contract.runtime_supported_steam_builds, [target]);
        assert_eq!(
            resolve_original_build_profile(None),
            resolve_original_build_profile(Some(target)),
        );
        let mut incomplete = Vec::new();
        for profile in &contract.profiles {
            let build = profile.steam_build.as_str();
            let release_target = contract
                .runtime_supported_steam_builds
                .iter()
                .any(|supported| supported == build);
            let complete = REQUIRED_CURRENT_HEXAGRAM_CAPABILITIES
                .iter()
                .all(|capability| profile.capabilities[capability.contract_key()]);
            if release_target || complete {
                assert!(resolve_original_build_profile(Some(build)).is_ok());
            } else {
                incomplete.push(build);
                let error = resolve_original_build_profile(Some(build)).unwrap_err();
                assert!(error.contains(&format!("unsupported original Steam build {build}")));
                assert!(error.contains("historical profile is incomplete"));
            }
        }
        assert!(
            !incomplete.is_empty(),
            "contract keeps at least one incomplete historical build as the rejection sample",
        );
        assert_eq!(latest_retired_steam_build(), incomplete.last().copied());
        assert!(resolve_original_build_profile(Some("99999999")).is_err());
    }

    #[test]
    fn all_capabilities_come_from_the_shared_matrix() {
        let contract = original_build_profiles().expect("build profile contract loads");
        let audit_profile = |build: &str| OriginalBuildProfile {
            profile_index: contract
                .profiles
                .iter()
                .position(|profile| profile.steam_build == build)
                .unwrap_or_else(|| panic!("missing audit profile {build}")),
        };
        let before = audit_profile("24013612");
        let adjustment = audit_profile("24120316");
        let previous_target = audit_profile("24124964");
        let target = audit_profile("24180265");
        for capability in [
            OriginalBuildCapability::Fate324GrantsCloudChain,
            OriginalBuildCapability::Fate335GrantsMomentum,
            OriginalBuildCapability::SpiritFormationEchoUsesBaseCard,
        ] {
            assert!(!before.has(capability));
            assert!(adjustment.has(capability));
            assert!(previous_target.has(capability));
            assert!(target.has(capability));
        }
        for capability in [
            OriginalBuildCapability::HexagramLossDoesNotSpendCard422StarPower,
            OriginalBuildCapability::LostHexagramLedgerAccumulatesPositiveLoss,
            OriginalBuildCapability::DreamThunderRefundsHexagramLoss,
        ] {
            assert!(!before.has(capability));
            assert!(!adjustment.has(capability));
            assert!(!previous_target.has(capability));
            assert!(target.has(capability));
        }
    }
}
