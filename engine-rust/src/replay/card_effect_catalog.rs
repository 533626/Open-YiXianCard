use crate::model::CardDefinition;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum CardEffectResolution {
    /// A typed Card_* handler or another explicit behavior arm exists.
    Executable,
    /// The original Card_* body consists only of the audited printed fields.
    VerifiedPrintedFallback,
    /// The canonical scope policy excludes this whole card from battle.
    RecordOnly,
    /// No battle behavior has been proved for this card.
    Missing,
}

impl CardEffectResolution {
    pub(super) fn executes_printed_follow_ups(self) -> bool {
        matches!(self, Self::Executable | Self::VerifiedPrintedFallback)
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CardEffectCatalog {
    schema_version: u32,
    executable_base_ids: Vec<i64>,
}

static EXECUTABLE_BASE_IDS: OnceLock<BTreeSet<i64>> = OnceLock::new();

// Machine-readable mirror of the TS scope policy's exceptional whole-card
// exclusions. Refine/Change are classified from canonical CardType below.
pub(super) const RECORD_ONLY_CARD_BASE_IDS: &[i64] = &[
    1,          // 云泉道茶
    9_000_006,  // 神秘种子
    99_000_006, // 再试一次
];

// Every entry is backed by the current-build Card_* source. The listed class
// does no more than the printed attack/anima/defense/sword-intent fields (or
// the same random range). Unknown cards must not grow this list just to make a
// replay pass.
pub(super) const VERIFIED_PRINTED_FALLBACK_BASE_IDS: &[i64] = &[
    24, // 瞒天过海：无 Card_24；FallbackCardAction=灵气+防御（TS deceiveHeavenCrossSeaCurrentFallback）
    30, // 符刀•斩怨：无 Card_30；FallbackCardAction=配置攻击（TS talismanBladeGrudgeCurrentFallback）
    145, // 一段攻击
    146, // 二段攻击
    147, // 三段攻击
    148, // 四段攻击
    149, // 五段攻击
    150, // 七段攻击
    286, // 隐藏普通攻击
    367, // 马蹄
    1_000_001, // 云剑·探云
    1_000_005, // 护身灵气
    1_000_007, // 巨虎灵剑
    1_000_010, // 剑劈
    1_000_012, // 剑挡
    1_000_013, // 骤风剑
    1_000_017, // 凝意诀
    1_000_019, // 巨鲸灵剑
    1_000_031, // 云舞诀
    1_000_032, // 三峰剑
    1_000_037, // 流云乱剑
    4_000_006, // 野马分鬃
    4_000_055, // 一掷乾坤
    6_000_001, // 调色
    7_000_055, // 五行遁术（再次行动由统一 action-again hook 判定）
    9_000_003, // 剑枝竹
    9_000_015, // 空间灵田（末两格首次跳过由统一抽牌 hook 判定）
    10_000_029, // 饿虎扑食（灵气减免由统一费用 hook 判定）
    10_000_035, // 崩拳·连崩（保留效果由统一崩拳 hook 判定）
    11_000_009, // 探灵
];

// These Card_* bodies predate the split typed modules but still have explicit,
// audited implementations in flow_card_effect_fallback.rs.
#[cfg(test)]
pub(super) const EXPLICIT_FALLBACK_BASE_IDS: &[i64] = &[
    4_000_020, 4_000_021, 4_000_023, 4_000_024, 4_000_027, 4_000_028, 4_000_029, 4_000_032,
    4_000_033, 4_000_038, 4_000_039, 4_000_061, 4_000_062, 4_000_068,
];

fn executable_base_ids() -> &'static BTreeSet<i64> {
    EXECUTABLE_BASE_IDS.get_or_init(|| {
        let catalog: CardEffectCatalog =
            serde_json::from_str(include_str!("../../data/card-effect-catalog.json"))
                .expect("Rust card-effect catalog parses");
        assert_eq!(catalog.schema_version, 1, "Rust card-effect catalog schema");
        let ids = catalog
            .executable_base_ids
            .into_iter()
            .collect::<BTreeSet<_>>();
        assert!(!ids.is_empty(), "Rust card-effect catalog is non-empty");
        ids
    })
}

/// Pure capability lookup. This function never opens an invocation, advances
/// decisions/RNG, or touches battle state.
pub(super) fn resolve_card_effect(card: &CardDefinition, base_id: i64) -> CardEffectResolution {
    let record_only_type = card.card_type.as_ref().is_some_and(|card_type| {
        card_type.name.eq_ignore_ascii_case("refine")
            || card_type.name.eq_ignore_ascii_case("change")
    });
    if record_only_type || RECORD_ONLY_CARD_BASE_IDS.contains(&base_id) {
        CardEffectResolution::RecordOnly
    } else if VERIFIED_PRINTED_FALLBACK_BASE_IDS.contains(&base_id) {
        CardEffectResolution::VerifiedPrintedFallback
    } else if executable_base_ids().contains(&base_id) {
        CardEffectResolution::Executable
    } else {
        CardEffectResolution::Missing
    }
}

#[cfg(test)]
pub(super) fn catalog_executable_base_ids() -> &'static BTreeSet<i64> {
    executable_base_ids()
}
