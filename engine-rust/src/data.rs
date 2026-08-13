use crate::Result;
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::Path;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardArchive {
    pub schema_version: i64,
    pub totals: CardArchiveTotals,
    #[serde(default)]
    pub cards: Vec<CardArchiveCard>,
    #[serde(default)]
    pub groups: Vec<CardArchiveGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardArchiveTotals {
    pub card_count: usize,
    pub battle_count: usize,
    pub record_only_count: usize,
    pub hidden_count: usize,
    pub obsolete_count: usize,
    pub registered_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardArchiveGroup {
    pub kind: String,
    pub key: String,
    pub label: String,
    pub total_count: usize,
    pub battle_count: usize,
    pub registered_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CardArchiveCard {
    pub base_id: i64,
    pub name: String,
    #[serde(rename = "type")]
    pub card_type: String,
    pub simulation_scope: String,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub obsolete: bool,
    pub archive_kind: String,
    pub archive_key: String,
    pub archive_label: String,
    #[serde(default)]
    pub realm: Option<String>,
    #[serde(default)]
    pub realm_label: Option<String>,
    #[serde(default)]
    pub variant_count: usize,
    #[serde(default)]
    pub registered: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct TalentArchive {
    #[serde(rename = "schemaVersion")]
    pub schema_version: i64,
    #[serde(default)]
    pub groups: Vec<TalentArchiveGroup>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct TalentArchiveGroup {
    pub kind: String,
    pub key: String,
    pub label: String,
    pub total_count: usize,
    pub battle_count: usize,
    pub implemented_count: usize,
    pub missing_battle_count: usize,
    pub record_only_count: usize,
    pub variant_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CharacterTalentAudit {
    pub totals: CharacterTalentTotals,
    #[serde(default)]
    pub rows: Vec<CharacterTalentRow>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterTalentTotals {
    pub characters: usize,
    pub character_talent_entries: usize,
    pub unique_character_talents: usize,
    pub implemented: usize,
    pub missing_battle: usize,
    pub ignored_battle: usize,
    pub record_only: usize,
    pub unclassified_non_battle: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CharacterTalentRow {
    pub character_id: i64,
    pub character_name: String,
    pub talent_id: i64,
    pub name: String,
    pub status: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct FateStrategyCards {
    pub source: String,
    pub mechanism_id: i64,
    pub variant_count: usize,
    #[serde(default)]
    pub cards: Vec<FateStrategyCard>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FateStrategyCard {
    pub id: i64,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDataSnapshot {
    pub card_archive: CardArchive,
    pub talent_archive: TalentArchive,
    pub character_talent_audit: CharacterTalentAudit,
    pub fate_strategy_cards: FateStrategyCards,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BattleDataCounts {
    pub card_archive_card_count: usize,
    pub card_archive_battle_count: usize,
    pub card_archive_registered_count: usize,
    pub card_archive_group_count: usize,
    pub talent_archive_group_count: usize,
    pub talent_archive_total_count: usize,
    pub talent_archive_variant_count: usize,
    pub character_count: usize,
    pub character_talent_entries: usize,
    pub unique_character_talents: usize,
    pub character_talent_row_count: usize,
    pub fate_strategy_variant_count: usize,
    pub fate_strategy_card_count: usize,
}

impl BattleDataSnapshot {
    pub fn counts(&self) -> BattleDataCounts {
        BattleDataCounts {
            card_archive_card_count: self.card_archive.totals.card_count,
            card_archive_battle_count: self.card_archive.totals.battle_count,
            card_archive_registered_count: self.card_archive.totals.registered_count,
            card_archive_group_count: self.card_archive.groups.len(),
            talent_archive_group_count: self.talent_archive.groups.len(),
            talent_archive_total_count: self
                .talent_archive
                .groups
                .iter()
                .map(|group| group.total_count)
                .sum(),
            talent_archive_variant_count: self
                .talent_archive
                .groups
                .iter()
                .map(|group| group.variant_count)
                .sum(),
            character_count: self.character_talent_audit.totals.characters,
            character_talent_entries: self.character_talent_audit.totals.character_talent_entries,
            unique_character_talents: self.character_talent_audit.totals.unique_character_talents,
            character_talent_row_count: self.character_talent_audit.rows.len(),
            fate_strategy_variant_count: self.fate_strategy_cards.variant_count,
            fate_strategy_card_count: self.fate_strategy_cards.cards.len(),
        }
    }
}

pub fn load_json_file<T: DeserializeOwned>(path: impl AsRef<Path>) -> Result<T> {
    let text = fs::read_to_string(path)?;
    Ok(serde_json::from_str(&text)?)
}

pub fn load_battle_data_snapshot(project_root: impl AsRef<Path>) -> Result<BattleDataSnapshot> {
    let root = project_root.as_ref();
    Ok(BattleDataSnapshot {
        card_archive: load_json_file(root.join("shared/data/card-archive.json"))?,
        talent_archive: load_json_file(root.join("shared/data/talent-archive.json"))?,
        character_talent_audit: load_json_file(
            root.join("shared/data/character-talent-audit.json"),
        )?,
        fate_strategy_cards: load_json_file(root.join("shared/data/fate-strategy-cards.json"))?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;
    use std::path::Path;

    fn project_root() -> &'static Path {
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .expect("engine-rust has project root parent")
    }

    #[test]
    fn loads_ts_reference_data_counts() {
        let snapshot = load_battle_data_snapshot(project_root()).expect("data snapshot loads");
        let counts = snapshot.counts();

        // All totals below are machine-owned archive/fixture projections.
        // Cross-check their rows instead of copying build- or fixture-specific
        // numbers that change whenever the generated selection advances.
        assert_eq!(
            counts.card_archive_card_count,
            snapshot.card_archive.cards.len()
        );
        assert_eq!(
            counts.card_archive_battle_count,
            snapshot
                .card_archive
                .cards
                .iter()
                .filter(|card| card.simulation_scope == "battle")
                .count()
        );
        assert_eq!(
            counts.card_archive_registered_count,
            snapshot
                .card_archive
                .cards
                .iter()
                .filter(|card| card.registered)
                .count()
        );
        assert_eq!(
            counts.card_archive_group_count,
            snapshot.card_archive.groups.len()
        );

        assert_eq!(
            counts.talent_archive_group_count,
            snapshot.talent_archive.groups.len()
        );
        assert_eq!(
            counts.talent_archive_total_count,
            snapshot
                .talent_archive
                .groups
                .iter()
                .map(|group| group.total_count)
                .sum::<usize>()
        );
        assert_eq!(
            counts.talent_archive_variant_count,
            snapshot
                .talent_archive
                .groups
                .iter()
                .map(|group| group.variant_count)
                .sum::<usize>()
        );

        assert_eq!(
            counts.character_count,
            snapshot
                .character_talent_audit
                .rows
                .iter()
                .map(|row| row.character_id)
                .collect::<HashSet<_>>()
                .len()
        );
        assert_eq!(
            counts.character_talent_entries,
            snapshot.character_talent_audit.rows.len()
        );
        assert_eq!(
            counts.character_talent_row_count,
            counts.character_talent_entries
        );
        assert_eq!(
            counts.unique_character_talents,
            snapshot
                .character_talent_audit
                .rows
                .iter()
                .map(|row| row.talent_id)
                .collect::<HashSet<_>>()
                .len()
        );

        assert_eq!(
            counts.fate_strategy_variant_count,
            snapshot.fate_strategy_cards.cards.len()
        );
        assert_eq!(
            counts.fate_strategy_card_count,
            snapshot.fate_strategy_cards.cards.len()
        );
    }
}
