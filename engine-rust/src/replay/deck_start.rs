use super::original_config::{
    can_upgrade_original_card, complete_with_original_card, original_card_definition,
    upgrade_original_card,
};
use crate::fixture::FixturePlayer;
use crate::model::CardDefinition;

const SOLITARY_VOID_GOLDEN_BOOK_TALENT: i64 = 198;
const SOLITARY_VOID_GOLDEN_BOOK_BASE_ID: i64 = 215;
const BACK_SOLITUDE_CARD_ID: i64 = 216;
const STRIKE_VOID_CARD_ID: i64 = 217;
const TIGER_BODY_TALENT: i64 = 125;
const TIGER_BODY_HP_THRESHOLD: i64 = 120;
// BattleCharacter.OnBattleStarted case 125 uses this exact base-card whitelist.
// 426 is absent from build 24371489, but remains here to preserve the original rule.
const TIGER_BODY_PREFERRED_CARD_IDS: [i64; 5] = [38, 18, 21, 135, 426];

pub(super) fn apply_deck_start_talent_effects(
    cards: &mut [CardDefinition],
    fixture: &FixturePlayer,
    battle_start_hp: i64,
) {
    apply_tiger_body_deck_upgrade(cards, fixture, battle_start_hp);
    if !fixture.talents.contains(&SOLITARY_VOID_GOLDEN_BOOK_TALENT)
        && !fixture.fate_strategies.contains(&338)
    {
        return;
    }
    let active_slot_count = fixture.active_slot_count;
    for slot_index in 0..active_slot_count {
        if normalize_base_id(cards[slot_index].id) != SOLITARY_VOID_GOLDEN_BOOK_BASE_ID {
            continue;
        }
        let mut upgrade = 2_i64;
        let previous_grid = previous_active_grid(slot_index, active_slot_count);
        if normalize_base_id(cards[previous_grid].id) == 0 {
            cards[previous_grid] = original_card_template(BACK_SOLITUDE_CARD_ID);
            upgrade -= 1;
        }
        let next_grid = next_active_grid(slot_index, active_slot_count);
        if normalize_base_id(cards[next_grid].id) == 0 {
            cards[next_grid] = original_card_template(STRIKE_VOID_CARD_ID);
            upgrade -= 1;
        }
        cards[slot_index] = upgrade_original_card(&cards[slot_index], upgrade);
    }
}

fn apply_tiger_body_deck_upgrade(
    cards: &mut [CardDefinition],
    fixture: &FixturePlayer,
    battle_start_hp: i64,
) {
    if !fixture.talents.contains(&TIGER_BODY_TALENT) {
        return;
    }
    // BattleCharacter.OnBattleStarted case 125 (IL_1748) gates the deck
    // upgrade on the current battleTempData.hp at this actor's own
    // OnBattleStarted boundary, not on maxHp: the second actor's check
    // already includes the first actor's opening effects. The caller passes
    // that boundary hp (first actor: battleStartHp persistent sample when
    // present, else constructed hp; second actor: live hp after the first
    // actor's opening).
    if battle_start_hp < TIGER_BODY_HP_THRESHOLD {
        return;
    }
    let slot_index = find_tiger_body_upgrade_slot(cards, fixture, true)
        .or_else(|| find_tiger_body_upgrade_slot(cards, fixture, false));
    let Some(slot_index) = slot_index else {
        return;
    };
    cards[slot_index] = upgrade_original_card(&cards[slot_index], 1);
}

fn find_tiger_body_upgrade_slot(
    cards: &[CardDefinition],
    fixture: &FixturePlayer,
    require_exclusive: bool,
) -> Option<usize> {
    for (slot_index, card) in cards.iter().enumerate().take(fixture.active_slot_count) {
        let card = complete_with_original_card(card);
        if !is_tiger_body_upgrade_candidate(&card, require_exclusive) {
            continue;
        }
        return Some(slot_index);
    }
    None
}

fn is_tiger_body_upgrade_candidate(card: &CardDefinition, require_preferred: bool) -> bool {
    // The original only checks rarity/noUpgrade. Requiring the next-tier config
    // to exist is our defensive guard against manufacturing an unknown card ID.
    if !can_upgrade_original_card(card.id) {
        return false;
    }
    if require_preferred {
        TIGER_BODY_PREFERRED_CARD_IDS.contains(&card.id)
    } else {
        true
    }
}

fn previous_active_grid(slot_index: usize, active_slot_count: usize) -> usize {
    (slot_index + active_slot_count - 1) % active_slot_count
}

fn next_active_grid(slot_index: usize, active_slot_count: usize) -> usize {
    (slot_index + 1) % active_slot_count
}

fn normalize_base_id(card_id: i64) -> i64 {
    if card_id == 0 || card_id == 10_000 || card_id == 20_000 {
        return 0;
    }
    card_id - ((card_id % 1_000_000) / 10_000) * 10_000
}

fn original_card_template(card_id: i64) -> CardDefinition {
    if let Some(card) = original_card_definition(card_id) {
        return card;
    }
    CardDefinition {
        id: card_id,
        base_id: Some(normalize_base_id(card_id)),
        name: format!("card:{card_id}"),
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
        other_params: Vec::new(),
    }
}
