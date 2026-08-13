use super::support::normalize_base_id;
use crate::model::{CardDefinition, OriginalEnumValue};
use serde::Deserialize;
use std::collections::{HashMap, HashSet};
use std::sync::OnceLock;

#[derive(Debug, Deserialize)]
struct OriginalCardConfigRaw {
    id: i64,
    name: String,
    #[serde(default)]
    desc: Option<String>,
    #[serde(rename = "cardType", default)]
    card_type: Option<OriginalEnumValue>,
    #[serde(default)]
    hidden: Option<bool>,
    #[serde(default)]
    subcategory: Option<OriginalEnumValue>,
    #[serde(default)]
    rarity: Option<i64>,
    #[serde(default)]
    career: Option<OriginalEnumValue>,
    #[serde(rename = "noUpgrade", default)]
    no_upgrade: Option<bool>,
    #[serde(default)]
    attack: Option<i64>,
    #[serde(rename = "randomAttack", default)]
    random_attack: Option<i64>,
    #[serde(rename = "randomDef", default)]
    random_defense: Option<i64>,
    #[serde(rename = "attackCount", default)]
    attack_count: Option<i64>,
    #[serde(default)]
    level: Option<OriginalEnumValue>,
    #[serde(alias = "def", default)]
    defense: Option<i64>,
    #[serde(default)]
    damage: Option<i64>,
    #[serde(default)]
    anima: Option<i64>,
    #[serde(rename = "hpCost", default)]
    hp_cost: Option<i64>,
    #[serde(rename = "actionAgain", default)]
    action_again: Option<bool>,
    #[serde(rename = "chargeQi", default)]
    charge_qi: Option<i64>,
    #[serde(default)]
    physique: Option<i64>,
    #[serde(rename = "jianYi", default)]
    sword_intent: Option<i64>,
    #[serde(rename = "guaXiang", default)]
    hexagram: Option<i64>,
    #[serde(rename = "otherParams", default)]
    other_params: Vec<i64>,
}

#[derive(Debug, Clone)]
struct OriginalCardMeta {
    hidden: bool,
    subcategory_name: Option<String>,
    rarity: i64,
    realm_level: Option<i64>,
    no_upgrade: bool,
    charge_qi: i64,
}

#[derive(Debug)]
struct OriginalCardCatalog {
    cards: HashMap<i64, CardDefinition>,
    meta: HashMap<i64, OriginalCardMeta>,
    anima_desc_card_ids: HashSet<i64>,
    action_again_desc_card_ids: HashSet<i64>,
    wounded_desc_card_ids: HashSet<i64>,
    rear_move_desc_card_ids: HashSet<i64>,
}

static ORIGINAL_CARD_CATALOG: OnceLock<OriginalCardCatalog> = OnceLock::new();

fn catalog() -> &'static OriginalCardCatalog {
    ORIGINAL_CARD_CATALOG.get_or_init(load_original_card_catalog)
}

pub(super) fn original_card_definition(card_id: i64) -> Option<CardDefinition> {
    catalog().cards.get(&card_id).cloned()
}

/// 客户端 CardConfig.rarity 语义（无 rarity 字段 = 0）。大量隐藏牌/梦牌
/// （如 7020089 梦•火灵聚炎、7040089）配置里没有 rarity 字段，与 id 档位
/// 推断不一致；凡原版读 cardConfig.rarity 的钳制/选档逻辑必须走配置值。
pub(super) fn original_config_rarity(card_id: i64) -> i64 {
    catalog().meta.get(&card_id).map_or(0, |meta| meta.rarity)
}

#[cfg(test)]
pub(super) fn original_base_card_definitions() -> Vec<CardDefinition> {
    let mut by_base_id = std::collections::BTreeMap::new();
    for card in catalog().cards.values() {
        let base_id = card.base_id.unwrap_or_else(|| normalize_base_id(card.id));
        by_base_id
            .entry(base_id)
            .and_modify(|current: &mut CardDefinition| {
                if card.id < current.id {
                    *current = card.clone();
                }
            })
            .or_insert_with(|| card.clone());
    }
    by_base_id.into_values().collect()
}

pub(super) fn original_card_desc_contains_anima(card: &CardDefinition) -> bool {
    let base_id = card.base_id.unwrap_or_else(|| normalize_base_id(card.id));
    catalog().anima_desc_card_ids.contains(&card.id)
        || catalog().anima_desc_card_ids.contains(&base_id)
}

pub(super) fn original_card_desc_contains_action_again(card: &CardDefinition) -> bool {
    catalog().action_again_desc_card_ids.contains(&card.id)
}

/// `BattleCharacter.IsKuangJian` has a FateStrategy 381 branch that classifies
/// any card whose current config description contains `[击伤]` as 狂剑. Keep
/// this as an original-config lookup instead of inferring it from a card name:
/// the branch is intentionally broader than the printed 狂剑 family.
pub(super) fn original_card_desc_contains_wounded(card: &CardDefinition) -> bool {
    let base_id = card.base_id.unwrap_or_else(|| normalize_base_id(card.id));
    catalog().wounded_desc_card_ids.contains(&card.id)
        || catalog().wounded_desc_card_ids.contains(&base_id)
}

/// BattleCharacter.IsHouZhao uses the printed `[后招]：` marker, not the
/// broader replay `rearMove` trait (which also includes 黄雀在后 itself).
pub(super) fn original_card_desc_contains_rear_move(card: &CardDefinition) -> bool {
    let base_id = card.base_id.unwrap_or_else(|| normalize_base_id(card.id));
    catalog().rear_move_desc_card_ids.contains(&card.id)
        || catalog().rear_move_desc_card_ids.contains(&base_id)
}

pub(super) fn original_card_charge_qi(card_id: i64) -> i64 {
    original_card_meta(card_id)
        .map(|meta| meta.charge_qi.max(0))
        .unwrap_or(0)
}

pub(super) fn known_card_definition(
    catalog_cards: &[CardDefinition],
    card_id: i64,
) -> Option<CardDefinition> {
    if let Some(catalog_card) = catalog_cards.iter().find(|card| card.id == card_id) {
        return Some(complete_with_original_card(catalog_card));
    }
    original_card_definition(card_id)
}

pub(super) fn original_card_realm_level(card_id: i64) -> Option<i64> {
    original_card_meta(card_id).and_then(|meta| meta.realm_level)
}

pub(super) fn complete_with_original_card(card: &CardDefinition) -> CardDefinition {
    let Some(original) = original_card_definition(card.id) else {
        return card.clone();
    };
    merge_card_with_original(card, &original)
}

pub(super) fn upgrade_original_card(card: &CardDefinition, upgrade_times: i64) -> CardDefinition {
    if upgrade_times <= 0 {
        return card.clone();
    }
    // BattleCharacter.OnBattleStarted (talent 198 / 孤虚金书) upgrades one tier
    // at a time, stopping at the first `noUpgrade` card. A direct jump
    // (`card.id + upgrade_times * 10_000`) overshoots cards that already entered
    // the battle at a non-zero tier: e.g. 孤虚金书 id 10215 + 2*10_000 = 30215,
    // which has no card definition, so the upgrade silently no-ops and the card
    // stays at 10215 while the original client stops at 20215 (noUpgrade). Walk
    // the tiers like the original loop so a tier-1 孤虚金书 reaches 20215.
    let upgraded_id = original_card_echo_upgrade_id(card.id, upgrade_times);
    if upgraded_id == card.id {
        return card.clone();
    }
    original_card_definition(upgraded_id).unwrap_or_else(|| card.clone())
}

/// Card_8000012.cs 回响阵纹 echo chain: step +10000 per `upgrade_times`,
/// checking each current id's `noUpgrade` before stepping (missing config
/// metadata is treated as upgradable, mirroring TS's `?.noUpgrade === true`
/// guard). Unlike `upgrade_original_card`, this walks one level at a time so
/// an intermediate `noUpgrade` id stops the chain early.
pub(super) fn original_card_echo_upgrade_id(card_id: i64, upgrade_times: i64) -> i64 {
    let mut upgraded_id = card_id;
    for _ in 0..upgrade_times.max(0) {
        if original_card_meta(upgraded_id).is_some_and(|meta| meta.no_upgrade) {
            break;
        }
        upgraded_id += 10_000;
    }
    upgraded_id
}

pub(super) fn can_upgrade_original_card(card_id: i64) -> bool {
    original_card_meta(card_id).is_some_and(|meta| {
        !meta.no_upgrade && meta.rarity == 0 && original_card_definition(card_id + 10_000).is_some()
    })
}

pub(super) fn can_upgrade_original_battle_deck_card(card_id: i64) -> bool {
    original_card_meta(card_id).is_some_and(|meta| {
        !meta.no_upgrade && original_card_definition(card_id + 10_000).is_some()
    })
}

pub(super) fn original_card_is_hidden_or_mi_shu(card_id: i64) -> bool {
    let base_id = normalize_base_id(card_id);
    original_card_meta(card_id)
        .or_else(|| {
            if base_id == card_id {
                None
            } else {
                original_card_meta(base_id)
            }
        })
        .is_some_and(|meta| meta.hidden || meta.subcategory_name.as_deref() == Some("MiShu"))
}

fn original_card_meta(card_id: i64) -> Option<OriginalCardMeta> {
    catalog().meta.get(&card_id).cloned()
}

fn merge_card_with_original(card: &CardDefinition, original: &CardDefinition) -> CardDefinition {
    CardDefinition {
        id: original.id,
        base_id: original.base_id,
        name: if card.name.is_empty() || card.name.starts_with("card:") {
            original.name.clone()
        } else {
            card.name.clone()
        },
        card_type: card.card_type.clone().or(original.card_type.clone()),
        rarity: card.rarity.or(original.rarity),
        career_name: card.career_name.clone().or(original.career_name.clone()),
        attack: card.attack.or(original.attack),
        random_attack: card.random_attack.or(original.random_attack),
        random_defense: card.random_defense.or(original.random_defense),
        attack_count: card.attack_count.or(original.attack_count),
        defense: card.defense.or(original.defense),
        damage: card.damage.or(original.damage),
        anima: card.anima.or(original.anima),
        hp_cost: card.hp_cost.or(original.hp_cost),
        action_again: card.action_again.or(original.action_again),
        physique: card.physique.or(original.physique),
        sword_intent: card.sword_intent.or(original.sword_intent),
        hexagram: card.hexagram.or(original.hexagram),
        other_params: if card.other_params.is_empty() {
            original.other_params.clone()
        } else {
            card.other_params.clone()
        },
    }
}

fn load_original_card_catalog() -> OriginalCardCatalog {
    let source = include_str!("../../../shared/data/original-card-configs.ts");
    let marker = "JSON.parse(\"";
    let start = source
        .find(marker)
        .expect("ORIGINAL_CARD_CONFIGS marker missing")
        + marker.len();
    let end = source
        .rfind("\")")
        .expect("ORIGINAL_CARD_CONFIGS terminator missing");
    let json_text = unescape_ts_json_string(&source[start..end]);
    let configs: Vec<OriginalCardConfigRaw> =
        serde_json::from_str(&json_text).expect("original card configs parse");
    // shared/data/original-card-configs.ts 提取时丢了 randomDef 字段
    // （客户端 CardConfig.json build-24610558 有 9 张带 randomDef）。缺字段
    // 会让 [防]+{def}～{randomDef} 的随机防御退化为固定下限，且不再消耗
    // battleParams 队列 → 后续随机卡错位。oracle 锚点：mirror-32299000
    // 8a0d0312f8c5a67c/round-12 cp18（神来之笔→一掷乾坤 4000055 防御骰
    // 20：原版 p1.def=24 = 4+20，引擎 6 = 4+2）、d9fedb72578eb9c0/round-14
    // cp10（4020055 防御骰 17：原版 25 = 8+17，引擎 22 = 8+14）。
    const MISSING_RANDOM_DEFENSE: &[(i64, i64)] = &[
        (4000006, 10), // 野马分鬃 randomDef
        (4010006, 12),
        (4020006, 14),
        (4000055, 20), // 一掷乾坤 randomDef
        (4010055, 23),
        (4020055, 26),
        (6000004, 16), // 笔走龙蛇 randomDef
        (6010004, 21),
        (6020004, 26),
    ];
    let mut cards = HashMap::new();
    let mut meta = HashMap::new();
    let mut anima_desc_card_ids = HashSet::new();
    let mut action_again_desc_card_ids = HashSet::new();
    let mut wounded_desc_card_ids = HashSet::new();
    let mut rear_move_desc_card_ids = HashSet::new();
    for config in configs {
        let base_id = normalize_base_id(config.id);
        // 灵爪 FateStrategy 152（BattleCharacter.CalculateAttack
        // 11557-11560）：`cardConfig.desc.Contains("灵气") ||
        // cardConfig.anima < 0` 二选一即 +otherParams[0] 攻。desc 分支沿用
        // 既有基牌收录语义；anima<0 分支只收录当前档位 id——升级档可能改变
        // 灵气消耗符号，不能按基牌归并（oracle 锚点：mirror-32219000-human-01
        // 5d19850f298ccfce/round-12 cp8 双鬼拍门 anima=-1 15 vs 11）。
        let desc_has_anima = config
            .desc
            .as_deref()
            .is_some_and(|desc| desc.contains("灵气"));
        if desc_has_anima || config.anima.is_some_and(|anima| anima < 0) {
            anima_desc_card_ids.insert(config.id);
            if desc_has_anima {
                anima_desc_card_ids.insert(base_id);
            }
        }
        if config
            .desc
            .as_deref()
            .is_some_and(|desc| desc.contains("再次行动"))
        {
            action_again_desc_card_ids.insert(config.id);
        }
        if config
            .desc
            .as_deref()
            .is_some_and(|desc| desc.contains("[击伤]"))
        {
            wounded_desc_card_ids.insert(config.id);
            wounded_desc_card_ids.insert(base_id);
        }
        if config
            .desc
            .as_deref()
            .is_some_and(|desc| desc.contains("[后招]："))
        {
            rear_move_desc_card_ids.insert(config.id);
            rear_move_desc_card_ids.insert(base_id);
        }
        let random_defense = config.random_defense.or_else(|| {
            MISSING_RANDOM_DEFENSE
                .iter()
                .find(|(id, _)| *id == config.id)
                .map(|(_, value)| *value)
        });
        let card = CardDefinition {
            id: config.id,
            base_id: Some(base_id),
            name: config.name,
            card_type: config.card_type,
            rarity: config.rarity,
            career_name: config.career.map(|career| career.name),
            attack: config.attack,
            random_attack: config.random_attack,
            random_defense,
            attack_count: config.attack_count,
            defense: config.defense,
            damage: config.damage,
            anima: config.anima,
            hp_cost: config.hp_cost,
            action_again: config.action_again,
            physique: config.physique,
            sword_intent: config.sword_intent,
            hexagram: config.hexagram,
            other_params: config.other_params,
        };
        meta.insert(
            config.id,
            OriginalCardMeta {
                hidden: config.hidden.unwrap_or(false),
                subcategory_name: config.subcategory.map(|subcategory| subcategory.name),
                rarity: config.rarity.unwrap_or(0),
                realm_level: config.level.map(|level| level.value),
                no_upgrade: config.no_upgrade.unwrap_or(false),
                charge_qi: config.charge_qi.unwrap_or(0),
            },
        );
        cards.insert(config.id, card);
    }
    OriginalCardCatalog {
        cards,
        meta,
        anima_desc_card_ids,
        action_again_desc_card_ids,
        wounded_desc_card_ids,
        rear_move_desc_card_ids,
    }
}

fn unescape_ts_json_string(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    let mut chars = value.chars();
    while let Some(ch) = chars.next() {
        if ch == '\\' {
            match chars.next() {
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('n') => out.push('\n'),
                Some(other) => {
                    out.push('\\');
                    out.push(other);
                }
                None => out.push('\\'),
            }
        } else {
            out.push(ch);
        }
    }
    out
}
