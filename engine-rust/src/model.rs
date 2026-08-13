use serde::{Deserialize, Serialize};

pub const DECK_SIZE: usize = 8;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerSide {
    P1,
    P2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDefinition {
    pub id: i64,
    #[serde(rename = "baseId", default)]
    pub base_id: Option<i64>,
    pub name: String,
    #[serde(rename = "cardType", default)]
    pub card_type: Option<OriginalEnumValue>,
    #[serde(default)]
    pub rarity: Option<i64>,
    #[serde(rename = "careerName", default)]
    pub career_name: Option<String>,
    #[serde(default)]
    pub attack: Option<i64>,
    #[serde(rename = "randomAttack", default)]
    pub random_attack: Option<i64>,
    #[serde(rename = "randomDef", default)]
    pub random_defense: Option<i64>,
    #[serde(rename = "attackCount", default)]
    pub attack_count: Option<i64>,
    #[serde(alias = "def", default)]
    pub defense: Option<i64>,
    #[serde(default)]
    pub damage: Option<i64>,
    #[serde(default)]
    pub anima: Option<i64>,
    #[serde(rename = "hpCost", default)]
    pub hp_cost: Option<i64>,
    #[serde(rename = "actionAgain", default)]
    pub action_again: Option<bool>,
    #[serde(default)]
    pub physique: Option<i64>,
    #[serde(rename = "jianYi", alias = "swordIntent", default)]
    pub sword_intent: Option<i64>,
    #[serde(rename = "guaXiang", default)]
    pub hexagram: Option<i64>,
    #[serde(rename = "otherParams", default)]
    pub other_params: Vec<i64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OriginalEnumValue {
    pub value: i64,
    pub name: String,
}
