use serde::Serialize;

pub const ENGINE_IDENTITY_SCHEMA_VERSION: &str = "rust-engine-identity-v1";
pub const ENGINE_SOURCE_FINGERPRINT: &str = env!("YIXIAN_ENGINE_SOURCE_FINGERPRINT");
pub const VALUE_V0_PROFILE_VERSION: &str = "value-v0.1";

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct EngineIdentity {
    pub schema_version: &'static str,
    pub source_fingerprint: &'static str,
    pub package_version: &'static str,
    pub value_v0_profile_version: &'static str,
}

pub const fn engine_identity() -> EngineIdentity {
    EngineIdentity {
        schema_version: ENGINE_IDENTITY_SCHEMA_VERSION,
        source_fingerprint: ENGINE_SOURCE_FINGERPRINT,
        package_version: env!("CARGO_PKG_VERSION"),
        value_v0_profile_version: VALUE_V0_PROFILE_VERSION,
    }
}
