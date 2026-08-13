use super::original_config::{
    original_card_desc_contains_action_again, original_card_desc_contains_wounded,
};
use super::{Element, ReplayPlayer, BASIC_ATTACK_DAMAGE, BASIC_ATTACK_ID, PERMANENT_PHYSIQUE_KEY};
use crate::fixture::FixturePlayer;
use crate::model::{CardDefinition, PlayerSide};
use std::collections::HashMap;
use std::sync::OnceLock;

static CARD_TRAITS_BY_BASE_ID: OnceLock<HashMap<String, Vec<String>>> = OnceLock::new();

const ELEMENT_NAME_TOKENS: [(&str, Element); 5] = [
    ("金灵", Element::Metal),
    ("木灵", Element::Wood),
    ("水灵", Element::Water),
    ("火灵", Element::Fire),
    ("土灵", Element::Earth),
];

fn card_traits_by_base_id() -> &'static HashMap<String, Vec<String>> {
    CARD_TRAITS_BY_BASE_ID.get_or_init(|| {
        serde_json::from_str(include_str!("../../../shared/data/base-card-traits.json"))
            .expect("base-card-traits.json parses")
    })
}

pub(super) fn has_card_trait(card: &CardDefinition, trait_name: &str) -> bool {
    card_traits_by_base_id()
        .get(&normalized_base_id(card).to_string())
        .is_some_and(|traits| traits.iter().any(|trait_value| trait_value == trait_name))
}

fn element_trait(card: &CardDefinition) -> Option<Element> {
    if has_card_trait(card, "element:metal") {
        return Some(Element::Metal);
    }
    if has_card_trait(card, "element:water") {
        return Some(Element::Water);
    }
    if has_card_trait(card, "element:wood") {
        return Some(Element::Wood);
    }
    if has_card_trait(card, "element:fire") {
        return Some(Element::Fire);
    }
    if has_card_trait(card, "element:earth") {
        return Some(Element::Earth);
    }
    None
}

pub(super) fn has_cloud_chain(actor: &ReplayPlayer) -> bool {
    // Original BattleCharacter.HasBuff, BattleCharacter.cs:8306-8320: these virtual
    // sources all satisfy HasBuff(LianYun), even without a stored LianYun buff.
    actor.sword.cloud_chain > 0
        || actor.identity.talents.contains(&14)
        || (actor.identity.talent_resonance_id == Some(10)
            && actor.identity.talent_resonance_temp_flags.contains(&10))
        || actor.elements.long_ma_spirit > 0
}

pub(super) fn verified_ke_yin_max_hp(card_id: i64) -> i64 {
    // KeYinCardConfig 50146 (融剑阵) carries maxHp: 4 at battle initialization.
    const VERIFIED_MAX_HP: &[(i64, i64)] = &[(50_146, 4)];
    VERIFIED_MAX_HP
        .iter()
        .find_map(|(verified_id, max_hp)| (*verified_id == card_id).then_some(*max_hp))
        .unwrap_or(0)
}

pub(super) fn opponent_side(side: PlayerSide) -> PlayerSide {
    match side {
        PlayerSide::P1 => PlayerSide::P2,
        PlayerSide::P2 => PlayerSide::P1,
    }
}

pub(super) fn normalized_base_id(card: &CardDefinition) -> i64 {
    card.base_id.unwrap_or_else(|| normalize_base_id(card.id))
}

pub(super) fn card_rarity(card: &CardDefinition) -> i64 {
    card.rarity.unwrap_or((card.id % 1_000_000) / 10_000)
}

pub(super) fn normalize_base_id(card_id: i64) -> i64 {
    if card_id == 0 || card_id == 10_000 || card_id == 20_000 {
        return 0;
    }
    card_id - ((card_id % 1_000_000) / 10_000) * 10_000
}

pub(super) fn adapt_fixture_card_for_replay(
    mut card: CardDefinition,
    fixture: &FixturePlayer,
) -> CardDefinition {
    if normalized_base_id(&card) == 1_000_010 && card.sword_intent.is_none() {
        card.sword_intent = Some(card_rarity(&card) + 2);
    }
    if normalized_base_id(&card) == 1_000_012 && card.sword_intent.is_none() {
        card.sword_intent = Some(card_rarity(&card) + 2);
    }
    if card.id == 19 {
        let wuji_bonus = if fixture.talents.contains(&30_096) {
            1
        } else {
            0
        };
        let stable_sword_edge = fixture.talents.contains(&20_093);
        let forged_sword_attack = if fixture.talents.contains(&92) {
            fixture.talent_temp_datas.get("92").copied().unwrap_or(0) + wuji_bonus
        } else {
            0
        };
        let sword_forging_bonus = if fixture.talents.contains(&10_093) {
            3
        } else {
            0
        };
        let moon_shadow_penalty = if fixture.talents.contains(&20_096) {
            // TalentConfig 20096 otherParams[1]; Steam build 24217566: 7 -> 3.
            3
        } else {
            0
        };
        let sharp_sword_pattern_penalty = if fixture.talents.contains(&30_094) {
            2
        } else {
            0
        };
        let attack = card.attack.unwrap_or(0) + forged_sword_attack + sword_forging_bonus
            - if stable_sword_edge { 2 } else { 0 }
            - sharp_sword_pattern_penalty
            - moon_shadow_penalty;
        card.attack = Some(attack.max(1));
        if stable_sword_edge {
            card.defense = Some(9 + wuji_bonus);
        }
    }
    card
}

pub(super) fn other_param(card: &CardDefinition, index: usize) -> i64 {
    other_param_or(card, index, 0)
}

pub(super) fn other_param_or(card: &CardDefinition, index: usize, default: i64) -> i64 {
    card.other_params.get(index).copied().unwrap_or(default)
}

pub(super) fn wu_xing_count_in_deck(actor: &ReplayPlayer) -> i64 {
    let mut elements = std::collections::BTreeSet::new();
    let mut mirage_sky_seal_count = 0;
    let mut primordial_count = 0;
    for card in active_deck_cards(actor) {
        let base_id = normalized_base_id(card);
        for element in elements_in_card_name(card) {
            elements.insert(element);
        }
        if base_id == 292 {
            mirage_sky_seal_count = other_param_or(card, 1, mirage_sky_seal_count);
        }
        if base_id == 7_000_101 {
            primordial_count += other_param_or(card, 0, 1);
        }
    }
    // BattleCharacter.GetWuXingCountInDeck (BattleCharacter.cs:12118-12177):
    // with fate strategy 417 the talent-199 card params' names also count
    // toward the distinct 五行 set. oracle 锚点: mirror-32219000
    // 4eea252403546e4b/round-13 checkpoint[4] 混元化灵 p1.anima 7 (引擎 6:
    // talent199 土灵•合八荒 漏计); round-16 checkpoint[4] anima 13 (引擎 10:
    // 金/水/土 三系漏计)。
    if actor.identity.fate_strategies.contains(&417) {
        for card_id in actor.identity.talent_199_card_ids.iter().copied() {
            if card_id <= 0 {
                continue;
            }
            if let Some(card) = super::original_config::original_card_definition(card_id) {
                for element in elements_in_card_name(&card) {
                    elements.insert(element);
                }
            }
        }
    }
    let ke_yin_bonus = actor
        .identity
        .ke_yin_card_ids
        .iter()
        .filter(|&&card_id| matches!(card_id, 40_076 | 50_076))
        .count() as i64;
    let fire_bonus =
        if actor.identity.fate_strategies.contains(&147) && elements.contains(&Element::Fire) {
            1
        } else {
            0
        };
    elements.len() as i64 + mirage_sky_seal_count + primordial_count + ke_yin_bonus + fire_bonus
}

pub(super) fn active_deck_cards(actor: &ReplayPlayer) -> impl Iterator<Item = &CardDefinition> {
    actor
        .deck
        .slots
        .iter()
        .take(actor.deck.active_slot_count.min(actor.deck.slots.len()))
        .map(|slot| &slot.card)
}

pub(super) fn elements_in_card_name(card: &CardDefinition) -> Vec<Element> {
    ELEMENT_NAME_TOKENS
        .iter()
        .filter_map(|(token, element)| card.name.contains(token).then_some(*element))
        .collect()
}

pub(super) fn effective_anima_cost(
    card: &CardDefinition,
    actor: &ReplayPlayer,
    source_slot: Option<usize>,
) -> i64 {
    let base_id = normalized_base_id(card);
    let printed_cost = card
        .anima
        .filter(|value| *value < 0)
        .map_or(0, |value| -value);
    let is_free = (base_id == 1_000_025 && actor.core.defense > 0)
        || ((base_id == 7_000_019 || base_id == 271) && check_water_billow_anima_free(actor));
    let base = if is_free {
        0
    } else if matches!(base_id, 1_000_098 | 1_000_093 | 10_000_093) && has_cloud_chain(actor) {
        // 原版 CheckAnima: 追风/旧碎骨在连云或 talent-14 下不耗灵。
        0
    } else {
        printed_cost
    };
    let hundred_bird_reduction = if is_spirit_sword_for_actor(actor, card) {
        actor.sword.hundred_bird_spirit_sword_art.max(0)
    } else {
        0
    };
    // CardActionBase.cs:5026 — 7000095 五行灵击与 7000107 极•五行灵击
    // 同分支：卡组每有 1 种不同五行少耗 1 灵气（上限 0 灵）。
    let five_elements_spirit_strike_reduction = if matches!(base_id, 7_000_095 | 7_000_107) {
        wu_xing_count_in_deck(actor)
    } else {
        0
    };
    let hungry_tiger_reduction = if base_id == 10_000_029 {
        actor.status.internal_injury.max(0)
            + actor.status.weakness.max(0)
            + actor.status.attack_reduction.max(0)
            + actor.status.flaw.max(0)
            + actor.status.entangle.max(0)
            + actor.status.external_injury.max(0)
            + actor.status.meditation.max(0)
            + actor.status.lost_mind.max(0)
    } else {
        0
    };
    let ling_long_travel_reduction = if actor.identity.fate_strategies.contains(&321)
        && original_card_desc_contains_action_again(card)
    {
        1
    } else {
        0
    };
    let star_moon_fan_reduction =
        if source_slot.is_some_and(|slot| actor.astrology.star_slots.contains(&slot)) {
            actor.astrology.star_moon_fan.max(0)
        } else {
            0
        };
    (base
        - five_elements_spirit_strike_reduction
        - hungry_tiger_reduction
        - ling_long_travel_reduction
        - hundred_bird_reduction
        - star_moon_fan_reduction
        - actor.turn.next_card_anima_cost_reduction.max(0))
    .max(0)
}

fn check_water_billow_anima_free(actor: &ReplayPlayer) -> bool {
    // CardActionBase.CheckAnima（build 24646245）：
    // base 7000019/271 在 CheckWuXing(src, JiHuoShuiLing) 为真时不耗灵气。
    // CheckWuXing（CardActionBase.cs:5322-5356）除激活/龙马精神/UsedWuXing
    // 相生链外，卡组含 7030077|7040077 五行刺恒真时恒真。
    // oracle 锚点：hf-32308000 c92e455bf4f24b5b/round-09 cp3（p2 卡组含
    // 7030077，turn4 原版 0 灵气打出 7020019 水灵•波澜；引擎漏判恒真，
    // 灵气短缺跳过该回合 → actorTurn 4 vs 5）。
    actor.elements.activated_elements.contains(&Element::Water)
        || actor.elements.last_element == Some(Element::Water)
        || actor.elements.last_element.is_some_and(|last| {
            is_element_generated_by(last, Element::Water, actor.identity.talents.contains(&137))
        })
        || actor.elements.long_ma_spirit > 0
        || active_deck_cards(actor).any(|card| matches!(card.id, 7_030_077 | 7_040_077))
}

pub(super) fn effective_hp_cost(card: &CardDefinition, actor: &ReplayPlayer) -> i64 {
    match normalized_base_id(card) {
        10_000_005 => {
            let base = card.hp_cost.unwrap_or(0).max(0);
            (base - actor.core.physique / 2).max(0)
        }
        _ => card.hp_cost.unwrap_or(0).max(0),
    }
}

pub(super) fn is_beng_quan(base_id: i64) -> bool {
    card_traits_by_base_id()
        .get(&base_id.to_string())
        .is_some_and(|traits| traits.iter().any(|trait_value| trait_value == "bengQuan"))
}

pub(super) fn is_beng_quan_card(card: &CardDefinition) -> bool {
    is_beng_quan(normalized_base_id(card)) || has_card_trait(card, "bengQuan")
}

pub(super) fn is_effective_beng_quan_card(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    is_beng_quan_card(card) || actor.beng.beng_tian_step > 0
}

pub(super) fn applies_beng_quan_inherited_effects(
    actor: &ReplayPlayer,
    card: &CardDefinition,
) -> bool {
    is_beng_quan_card(card) || actor.beng.beng_tian_step > 0
}

pub(super) fn is_cloud_sword(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    let base_id = normalized_base_id(card);
    has_card_trait(card, "cloudSword")
        || card.name.contains("云剑")
        || has_temporary_cloud_sword_identity(actor)
        || (base_id == 19 && actor.identity.talents.contains(&20_096))
        || (actor.sword.frenzy_dragon_swallows_cloud > 0 && is_frenzy_sword(actor, card))
        || (actor.sword.cloud_sword_heaven_cycle > 0
            && card.career_name.as_deref() == Some("ZhenFaShi"))
        || base_id == 213
        || (actor.identity.talents.contains(&192)
            && actor.sword.ling_wu_card_base_ids.contains(&base_id))
        || (has_ke_yin_type(actor, 146) && base_id == 1_000_025)
        || base_id == 261
        || has_temporary_all_purpose_sword_identity(actor)
        || (actor.identity.talent_resonance_id == Some(109)
            && actor.identity.talent_resonance_temp_flags.contains(&109))
        || actor.sword.all_cards_as_cloud_sword > 0
}

pub(super) fn is_frenzy_sword(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    is_frenzy_sword_with_options(actor, card, false)
}

pub(super) fn is_frenzy_sword_with_options(
    actor: &ReplayPlayer,
    card: &CardDefinition,
    ignore_temporary_buff: bool,
) -> bool {
    let base_id = normalized_base_id(card);
    (base_id == 19 && actor.identity.talents.contains(&10_096))
        || card.name.contains("狂剑")
        || is_frenzy_sword_base_id(base_id)
        || base_id == 213
        || (actor.sword.frenzy_dragon_swallows_cloud > 0
            && (has_card_trait(card, "cloudSword")
                || card.name.contains("云剑")
                // Card_19.UpdateCardInfo（Card_19.cs:511-519）把 19 改名
                // 「云剑•澄心」；叠加狂龙吞云后 IsKuangJian 成立
                // （BattleCharacter.cs:12354）。引擎不重命名卡，fixture 卡名
                // 仍是「澄心剑胚」，故按 is_cloud_sword 的 19+20096 同口径
                // 在狂剑侧补改名等价分支。
                || (base_id == 19 && actor.identity.talents.contains(&20_096))
                || has_temporary_cloud_sword_identity(actor)))
        || matches!(base_id, 331 | 401 | 261)
        || (actor.identity.talents.contains(&192)
            && actor.sword.ling_wu_card_base_ids.contains(&base_id))
        || (actor.identity.fate_strategies.contains(&322) && card.name.contains('猫'))
        || (card.rarity.unwrap_or(0) >= 1 && actor.sword.frenzy_sword_cloud_gathering > 0)
        || (has_ke_yin_type(actor, 146) && base_id == 1_000_025)
        || (!ignore_temporary_buff
            && (has_temporary_all_purpose_sword_identity(actor)
                || has_temporary_frenzy_sword_identity(actor)))
        || (actor.identity.fate_strategies.contains(&319)
            && card.career_name.as_deref() == Some("QinShi"))
        || (actor.identity.fate_strategies.contains(&381)
            && original_card_desc_contains_wounded(card))
}

pub(super) fn is_frenzy_sword_for_actor(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    is_frenzy_sword(actor, card)
}

pub(super) fn has_ke_yin_type(actor: &ReplayPlayer, type_id: i64) -> bool {
    actor
        .identity
        .ke_yin_card_ids
        .iter()
        .any(|card_id| card_id.rem_euclid(10_000) == type_id.rem_euclid(10_000))
}

fn has_temporary_all_purpose_sword_identity(actor: &ReplayPlayer) -> bool {
    actor.sword.all_purpose_sword > 0
        && actor.sword.all_purpose_sword_effective_count <= actor.sword.all_purpose_sword
}

fn has_temporary_cloud_sword_identity(actor: &ReplayPlayer) -> bool {
    actor.sword.next_card_as_cloud_sword > 0
}

fn has_temporary_frenzy_sword_identity(actor: &ReplayPlayer) -> bool {
    actor.sword.next_cards_as_frenzy_sword > 0
        && actor.sword.next_cards_as_frenzy_sword_effective_count
            <= actor.sword.next_cards_as_frenzy_sword
}

pub(super) fn is_sword_formation_card(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    let base_id = normalized_base_id(card);
    is_sword_formation_base_id(base_id)
        || base_id == 213
        || (actor.identity.talents.contains(&192)
            && actor.sword.ling_wu_card_base_ids.contains(&base_id))
        || (actor
            .deck
            .slots
            .iter()
            .any(|slot| normalized_base_id(&slot.card) == 312)
            && !super::original_config::original_card_is_hidden_or_mi_shu(card.id)
            && is_frenzy_sword(actor, card))
}

pub(super) fn is_spirit_sword_card(card: &CardDefinition) -> bool {
    has_card_trait(card, "spiritSword") || is_spirit_sword_base_id(normalized_base_id(card))
}

pub(super) fn is_spirit_sword_for_actor(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    let base_id = normalized_base_id(card);
    is_spirit_sword_card(card)
        || base_id == 213
        || (actor.identity.talents.contains(&192)
            && actor.sword.ling_wu_card_base_ids.contains(&base_id))
        || (has_ke_yin_type(actor, 146) && base_id == 1_000_025)
}

pub(super) fn is_sword_card(actor: &ReplayPlayer, card: &CardDefinition) -> bool {
    card.name.contains("剑") && (card.id != 19 || !actor.identity.talents.contains(&30_096))
}

pub(super) fn has_base_card_in_deck(actor: &ReplayPlayer, base_id: i64) -> bool {
    actor
        .deck
        .slots
        .iter()
        .any(|slot| normalized_base_id(&slot.card) == base_id)
}

pub(super) fn has_active_base_card_in_deck(actor: &ReplayPlayer, base_id: i64) -> bool {
    active_deck_cards(actor).any(|card| normalized_base_id(card) == base_id)
}

pub(super) fn is_fate_strategy_card(base_id: i64) -> bool {
    matches!(
        base_id,
        1_000_088
            | 1_000_089
            | 1_000_090
            | 1_000_091
            | 1_000_092
            | 1_000_094
            | 1_000_095
            | 1_000_098
            | 4_000_090
            | 4_000_091
            | 4_000_092
            | 4_000_093
            | 4_000_094
            | 4_000_095
            | 4_000_096
            | 4_000_097
            | 7_000_094
            | 7_000_095
            | 7_000_096
            | 7_000_097
            | 7_000_098
            | 7_000_099
            | 7_000_100
            | 7_000_101
            | 10_000_090
            | 10_000_091
            | 10_000_092
            | 10_000_093
            | 10_000_094
            | 10_000_095
            | 10_000_096
            | 10_000_097
    )
}

pub(super) fn element_from_card(card: &CardDefinition) -> Option<Element> {
    element_trait(card).or_else(|| card_element(normalized_base_id(card)))
}

pub(super) fn is_five_element_card(card: &CardDefinition) -> bool {
    element_from_card(card).is_some()
}

pub(super) fn neighbor_slot_index(
    actor: &ReplayPlayer,
    slot_index: usize,
    direction: i64,
) -> usize {
    let effective_direction = if actor.fate.reverse_card_direction > 0 {
        -direction
    } else {
        direction
    };
    let length = actor.deck.slots.len().max(crate::model::DECK_SIZE);
    (slot_index as i64 + effective_direction).rem_euclid(length as i64) as usize
}

pub(super) fn neighbor_card(
    actor: &ReplayPlayer,
    slot_index: usize,
    direction: i64,
) -> &CardDefinition {
    let index = neighbor_slot_index(actor, slot_index, direction);
    &actor.deck.slots[index].card
}

pub(super) fn active_neighbor_card(
    actor: &ReplayPlayer,
    slot_index: usize,
    direction: i64,
) -> Option<&CardDefinition> {
    let index = active_neighbor_slot_index(actor, slot_index, direction)?;
    actor.deck.slots.get(index).map(|slot| &slot.card)
}

pub(super) fn active_neighbor_slot_index(
    actor: &ReplayPlayer,
    slot_index: usize,
    direction: i64,
) -> Option<usize> {
    let active_slot_count = actor.deck.active_slot_count.min(actor.deck.slots.len());
    if active_slot_count == 0 || slot_index >= active_slot_count {
        return None;
    }
    let effective_direction = if actor.fate.reverse_card_direction > 0 {
        -direction
    } else {
        direction
    };
    Some((slot_index as i64 + effective_direction).rem_euclid(active_slot_count as i64) as usize)
}

pub(super) fn is_five_element_control(previous: &CardDefinition, next: &CardDefinition) -> bool {
    matches!(
        (element_from_card(previous), element_from_card(next)),
        (Some(Element::Metal), Some(Element::Wood))
            | (Some(Element::Wood), Some(Element::Earth))
            | (Some(Element::Earth), Some(Element::Water))
            | (Some(Element::Water), Some(Element::Fire))
            | (Some(Element::Fire), Some(Element::Metal))
    )
}

pub(super) fn card_element(base_id: i64) -> Option<Element> {
    match base_id {
        18 | 143 | 273 | 316 | 376 | 7_000_001 | 7_000_002 | 7_000_015 | 7_000_016 | 7_000_025
        | 7_000_026 | 7_000_034 | 7_000_035 | 7_000_042 | 7_000_050 | 7_000_063 | 7_000_071
        | 7_000_074 | 7_000_084 | 7_000_085 | 7_000_099 => Some(Element::Metal),
        17 | 203 | 258 | 271 | 377 | 7_000_006 | 7_000_007 | 7_000_008 | 7_000_019 | 7_000_020
        | 7_000_029 | 7_000_030 | 7_000_037 | 7_000_044 | 7_000_048 | 7_000_057 | 7_000_059
        | 7_000_072 | 7_000_076 | 7_000_079 | 7_000_100 | 7_000_103 | 7_000_104 => {
            Some(Element::Water)
        }
        20 | 134 | 201 | 272 | 373 | 7_000_003 | 7_000_004 | 7_000_017 | 7_000_018 | 7_000_027
        | 7_000_028 | 7_000_036 | 7_000_043 | 7_000_051 | 7_000_061 | 7_000_064 | 7_000_068
        | 7_000_080 | 7_000_081 | 7_000_088 | 7_000_097 | 7_000_105 => Some(Element::Wood),
        198 | 270 | 315 | 374 | 7_000_009 | 7_000_010 | 7_000_021 | 7_000_022 | 7_000_031
        | 7_000_032 | 7_000_038 | 7_000_039 | 7_000_045 | 7_000_049 | 7_000_056 | 7_000_069
        | 7_000_082 | 7_000_083 | 7_000_089 | 7_000_090 | 7_000_098 => Some(Element::Fire),
        21 | 274 | 317 | 375 | 7_000_011 | 7_000_012 | 7_000_013 | 7_000_023 | 7_000_024
        | 7_000_033 | 7_000_040 | 7_000_041 | 7_000_046 | 7_000_047 | 7_000_053 | 7_000_062
        | 7_000_070 | 7_000_086 | 7_000_091 | 7_000_096 => Some(Element::Earth),
        _ => None,
    }
}

pub(super) fn is_frenzy_sword_base_id(base_id: i64) -> bool {
    matches!(
        base_id,
        2 | 125
            | 151
            | 186
            | 312
            | 1_000_022
            | 1_000_035
            | 1_000_049
            | 1_000_061
            | 1_000_066
            | 1_000_073
            | 1_000_076
            | 1_000_087
    )
}

pub(super) fn is_sword_formation_base_id(base_id: i64) -> bool {
    // 原版 IsJianZhen（BattleCharacter.cs:12444-12462）以名字含「剑阵」为
    // 主判据（id 列表仅为引擎侧枚举）。极•灵犀剑阵（1_000_100，卡面
    // 「防+{def}；将最多{otherParams[0]}[剑意]转为[灵气]；[再次行动]」）
    // 名字含剑阵，属剑阵牌：缺它会导致百兽灵剑阵（49）的首次剑阵触发、
    // 溟空剑阵诀追击与连环剑阵链对其失效（oracle 锚点：mirror-32299000
    // 4371d0685fd859f8/round-14 cp9 p1.hp 90 vs 98、cp13 83 vs 90）。
    matches!(
        base_id,
        48 | 49
            | 126
            | 213
            | 293
            | 325
            | 332
            | 1_000_025
            | 1_000_033
            | 1_000_041
            | 1_000_046
            | 1_000_051
            | 1_000_062
            | 1_000_064
            | 1_000_077
            | 1_000_080
            | 1_000_092
            | 1_000_100
            | 8_000_008
    )
}

pub(super) fn is_spirit_sword_base_id(base_id: i64) -> bool {
    matches!(
        base_id,
        260 | 1_000_007
            | 1_000_019
            | 1_000_020
            | 1_000_029
            | 1_000_036
            | 1_000_045
            | 1_000_047
            | 1_000_055
            | 1_000_059
            | 1_000_065
            | 1_000_074
            | 1_000_075
            | 1_000_081
            | 1_000_089
            | 1_000_090
            | 1_000_097
    )
}

pub(super) fn element_generates(element: Element) -> Element {
    match element {
        Element::Metal => Element::Water,
        Element::Water => Element::Wood,
        Element::Wood => Element::Fire,
        Element::Fire => Element::Earth,
        Element::Earth => Element::Metal,
    }
}

pub(super) fn is_element_generated_by(
    previous: Element,
    current: Element,
    fire_generates_all: bool,
) -> bool {
    element_generates(previous) == current
        || (fire_generates_all
            && previous != current
            && (previous == Element::Fire || current == Element::Fire))
}

pub(super) fn fate_strategy_131_elements(talents: &[i64]) -> Vec<Element> {
    let mut elements = Vec::new();
    if talents.contains(&10_109) {
        elements.push(Element::Water);
        elements.push(Element::Wood);
    }
    if talents.contains(&20_109) {
        elements.push(Element::Wood);
        elements.push(Element::Fire);
    }
    if talents.contains(&30_109) {
        elements.push(Element::Fire);
        elements.push(Element::Earth);
    }
    if talents.contains(&40_109) {
        elements.push(Element::Earth);
        elements.push(Element::Metal);
    }
    if talents.contains(&50_109) {
        elements.push(Element::Metal);
        elements.push(Element::Water);
    }
    elements
}

pub(super) fn innate_mark_elements(talents: &[i64]) -> Vec<Element> {
    let mut elements = Vec::new();
    if talents.contains(&10_109) {
        elements.push(Element::Metal);
    }
    if talents.contains(&20_109) {
        elements.push(Element::Water);
    }
    if talents.contains(&30_109) {
        elements.push(Element::Wood);
    }
    if talents.contains(&40_109) {
        elements.push(Element::Fire);
    }
    if talents.contains(&50_109) {
        elements.push(Element::Earth);
    }
    elements
}

pub(super) fn is_talent_52_replacement_slot(
    fixture: &FixturePlayer,
    card: &CardDefinition,
    slot_index: usize,
) -> bool {
    if !fixture.talents.contains(&52) {
        return false;
    }
    if normalized_base_id(card) != 0 {
        return false;
    }
    fixture.active_slot_count >= 8 && slot_index == 7
}

pub(super) fn seven_stars_stabilize_soul_card() -> CardDefinition {
    CardDefinition {
        id: 11,
        base_id: Some(11),
        name: "七星定魂".to_string(),
        card_type: None,
        attack: None,
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![4],
    }
}

pub(super) fn basic_attack_card() -> CardDefinition {
    CardDefinition {
        id: BASIC_ATTACK_ID,
        base_id: Some(BASIC_ATTACK_ID),
        name: "普通攻击".to_string(),
        card_type: None,
        attack: Some(BASIC_ATTACK_DAMAGE),
        random_attack: None,
        random_defense: None,
        attack_count: None,
        defense: None,
        damage: None,
        anima: None,
        hp_cost: None,
        action_again: None,
        physique: None,
        sword_intent: None,
        hexagram: None,
        rarity: None,
        career_name: None,
        other_params: vec![],
    }
}

pub(super) fn div_ceil(value: i64, denominator: i64) -> i64 {
    if value <= 0 {
        0
    } else {
        (value + denominator - 1) / denominator
    }
}

pub(super) fn permanent_physique_key() -> &'static str {
    PERMANENT_PHYSIQUE_KEY
}
