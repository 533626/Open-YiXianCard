use super::dream_mirage_hooks::deck_hook_flags;
use super::ReplayState;
use crate::model::CardDefinition;

/// Active mechanism family bitmask for hot-path hook gating.
/// If a battle has no cards, fates, talents, or sects belonging to a mechanism family,
/// calls to that family's hook functions are short-circuited immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct ActiveMechanisms(pub u32);

#[allow(dead_code)]
impl ActiveMechanisms {
    pub const NONE: Self = Self(0);
    pub const ALL: Self = Self(!0);

    pub const MIRAGE: Self = Self(1 << 0);
    pub const FORMATION: Self = Self(1 << 1);
    pub const FATE_STRATEGY: Self = Self(1 << 2);
    pub const THUNDER_MINDSET: Self = Self(1 << 3);
    pub const ELEMENT_FLOW: Self = Self(1 << 4);
    pub const XIAN_MO: Self = Self(1 << 5);
    pub const RONGHUI: Self = Self(1 << 6);
    pub const SWORD_SECT: Self = Self(1 << 7);
    pub const BENG_QUAN: Self = Self(1 << 8);
    pub const ASTROLOGY: Self = Self(1 << 9);
    pub const MUSIC: Self = Self(1 << 10);
    pub const CHANCE: Self = Self(1 << 11);

    #[inline(always)]
    pub const fn contains(self, other: Self) -> bool {
        (self.0 & other.0) == other.0
    }

    #[inline(always)]
    pub const fn has(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline(always)]
    pub const fn intersects(self, other: Self) -> bool {
        (self.0 & other.0) != 0
    }

    #[inline(always)]
    pub fn insert(&mut self, other: Self) {
        self.0 |= other.0;
    }
}

impl std::ops::BitOr for ActiveMechanisms {
    type Output = Self;
    #[inline(always)]
    fn bitor(self, rhs: Self) -> Self {
        Self(self.0 | rhs.0)
    }
}

impl std::ops::BitOrAssign for ActiveMechanisms {
    #[inline(always)]
    fn bitor_assign(&mut self, rhs: Self) {
        self.0 |= rhs.0;
    }
}

pub(super) fn card_active_mechanisms(card: &CardDefinition) -> ActiveMechanisms {
    let mut mask = ActiveMechanisms::NONE;
    let name = card.name.as_ref();
    let id = card.id;

    let base_id = super::support::normalized_base_id(card);

    // Mirage / Dream
    if name.contains('梦')
        || name.contains('幻')
        || deck_hook_flags(card) != 0
        || (id >= 1_000_065 && id <= 1_040_089)
        || (id >= 4_000_065 && id <= 4_040_089)
        || (id >= 7_000_065 && id <= 7_040_089)
        || (id >= 10_000_065 && id <= 10_040_089)
    {
        mask.insert(ActiveMechanisms::MIRAGE);
    }

    // Formation
    if name.contains('阵')
        || card.card_type.as_ref().map_or(0, |c| c.value) == super::CARD_TYPE_SUSTAIN
        || super::formations::FORMATION_HANDLED_IDS.contains(&base_id)
    {
        mask.insert(ActiveMechanisms::FORMATION);
    }

    // Thunder
    if name.contains('雷') || id == 29 || id == 4_000_030 || (id >= 4_000_070 && id <= 4_040_075) {
        mask.insert(ActiveMechanisms::THUNDER_MINDSET);
    }

    // Elements
    if name.contains("五行")
        || name.contains("金灵")
        || name.contains("木灵")
        || name.contains("水灵")
        || name.contains("火灵")
        || name.contains("土灵")
        || super::support::element_from_card(card).is_some()
    {
        mask.insert(ActiveMechanisms::ELEMENT_FLOW);
    }

    // Sword
    if name.contains('剑') || id == 19 || (id >= 1_000_000 && id < 2_000_000) {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    // Beng
    if name.contains('崩') || (id >= 10_000_000 && id < 11_000_000) {
        mask.insert(ActiveMechanisms::BENG_QUAN);
    }

    // Astrology
    if name.contains('星') || name.contains('卦') || (id >= 4_000_000 && id < 5_000_000) {
        mask.insert(ActiveMechanisms::ASTROLOGY);
    }

    // Music
    if card.career_name.as_deref() == Some("QinShi") || name.contains('琴') {
        mask.insert(ActiveMechanisms::MUSIC);
    }

    // Ronghui
    let base_id = super::support::normalized_base_id(card);
    if name.contains("融汇")
        || (base_id >= 177 && base_id <= 210)
        || base_id == 389
        || base_id == 390
        || (id >= 20_000_000 && id < 30_000_000)
    {
        mask.insert(ActiveMechanisms::RONGHUI);
    }

    // Chance
    if (id >= 99_000_000 && id < 100_000_000) || name.contains("机缘") {
        mask.insert(ActiveMechanisms::CHANCE);
    }

    mask
}

pub(super) fn identity_active_mechanisms(
    identity: &super::ReplayPlayerIdentity,
) -> ActiveMechanisms {
    let mut mask = ActiveMechanisms::NONE;

    if !identity.fate_strategies.is_empty() {
        mask.insert(ActiveMechanisms::FATE_STRATEGY);
        if identity.fate_strategies.contains(&396) {
            mask.insert(ActiveMechanisms::THUNDER_MINDSET);
        }
        if identity.fate_strategies.contains(&380) {
            mask.insert(ActiveMechanisms::SWORD_SECT | ActiveMechanisms::FORMATION);
        }
        if identity.fate_strategies.contains(&416) {
            mask.insert(ActiveMechanisms::ELEMENT_FLOW);
        }
        if identity
            .fate_strategies
            .iter()
            .any(|&id| id == 319 || id == 322 || id == 381)
        {
            mask.insert(ActiveMechanisms::SWORD_SECT);
        }
    }

    if !identity.xian_mo_strategies.is_empty() || !identity.xian_mo_temp_datas.is_empty() {
        mask.insert(ActiveMechanisms::XIAN_MO);
        if identity
            .xian_mo_strategies
            .iter()
            .any(|&id| id == 7049 || id == 7068 || id == 7132)
        {
            mask.insert(ActiveMechanisms::SWORD_SECT);
        }
    }

    if !identity.talents.is_empty() {
        for &talent in identity.talents.iter() {
            match talent {
                14 | 15 | 57 | 192 | 10_057 | 20_057 | 30_057 | 10_096 | 20_096 => {
                    mask.insert(ActiveMechanisms::SWORD_SECT);
                }
                137 => {
                    mask.insert(ActiveMechanisms::ELEMENT_FLOW);
                }
                100_085 => {
                    mask.insert(ActiveMechanisms::FORMATION);
                }
                _ => {}
            }
        }
    }

    if matches!(identity.talent_resonance_id, Some(10) | Some(109)) {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    if identity
        .ke_yin_card_ids
        .iter()
        .any(|id| id.rem_euclid(10_000) == 146)
    {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    mask
}

fn player_static_mechanisms(player: &super::ReplayPlayer) -> ActiveMechanisms {
    let mut mask = player.identity.mechanisms();
    // A true deck/identity bit already dominates its dynamic-state disjunct.
    // Keep every refresh, but do not construct/compare a large default state
    // when that comparison cannot change the resulting bit.
    for slot in player.deck.slots.iter() {
        mask.insert(slot.card.mechanisms());
    }
    mask
}

fn insert_player_dynamic_mechanisms(player: &super::ReplayPlayer, mask: &mut ActiveMechanisms) {
    if !mask.has(ActiveMechanisms::FORMATION) && player.formations.is_nondefault() {
        mask.insert(ActiveMechanisms::FORMATION);
    }

    if !mask.has(ActiveMechanisms::SWORD_SECT) && player.sword.is_nondefault() {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    if !mask.has(ActiveMechanisms::MIRAGE)
        && (player.dream_mirage.is_nondefault() || player.mirage_ronghui.is_nondefault())
    {
        mask.insert(ActiveMechanisms::MIRAGE);
    }

    if !mask.has(ActiveMechanisms::RONGHUI) && player.ronghui.is_nondefault() {
        mask.insert(ActiveMechanisms::RONGHUI);
    }

    if !mask.has(ActiveMechanisms::FATE_STRATEGY) && player.fate.is_nondefault() {
        mask.insert(ActiveMechanisms::FATE_STRATEGY);
    }
    // These counters imply nondefault fate, and affect OTHER families. They
    // must still be checked when FATE_STRATEGY was supplied by the identity.
    if player.fate.ke_yin_jing_lei != 0 {
        mask.insert(ActiveMechanisms::THUNDER_MINDSET);
    }
    if player.fate.xing_yi_lian > 0 {
        mask.insert(ActiveMechanisms::ASTROLOGY);
    }

    if !mask.has(ActiveMechanisms::BENG_QUAN) && player.beng.is_nondefault() {
        mask.insert(ActiveMechanisms::BENG_QUAN);
    }

    if player.astrology.thunder_mindset != 0 || player.astrology.lei_shan_er_du != 0 {
        mask.insert(ActiveMechanisms::THUNDER_MINDSET);
    }
    if !mask.has(ActiveMechanisms::ASTROLOGY) && player.astrology.is_nondefault() {
        mask.insert(ActiveMechanisms::ASTROLOGY);
    }

    if !mask.has(ActiveMechanisms::ELEMENT_FLOW) && player.elements.is_nondefault() {
        mask.insert(ActiveMechanisms::ELEMENT_FLOW);
    }

    if !mask.has(ActiveMechanisms::MUSIC) && player.music.is_nondefault() {
        mask.insert(ActiveMechanisms::MUSIC);
    }

    if !mask.has(ActiveMechanisms::CHANCE) && player.chance.is_nondefault() {
        mask.insert(ActiveMechanisms::CHANCE);
    }
}

#[cfg(test)]
fn player_active_mechanisms(player: &super::ReplayPlayer) -> ActiveMechanisms {
    let mut mask = player_static_mechanisms(player);
    insert_player_dynamic_mechanisms(player, &mut mask);
    mask
}

pub(super) fn detect_active_mechanisms<const OBSERVE: bool>(
    state: &ReplayState<OBSERVE>,
) -> ActiveMechanisms {
    // The detector consumes the UNION, so either side can supply a bit before
    // the dynamic disjuncts of either side are considered. No cached state.
    let mut mask = player_static_mechanisms(&state.p1) | player_static_mechanisms(&state.p2);
    insert_player_dynamic_mechanisms(&state.p1, &mut mask);
    insert_player_dynamic_mechanisms(&state.p2, &mut mask);
    mask
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use crate::replay::{original_config, ReplayCardSlot, RuntimeCard};

    fn check(card: CardDefinition) -> RuntimeCard {
        let expected = card_active_mechanisms(&card);
        let runtime: RuntimeCard = card.into();
        assert_eq!(runtime.1, expected);
        assert_eq!(runtime.clone().1, card_active_mechanisms(&runtime));
        runtime
    }

    #[test]
    fn immutable_cache_matches_original_and_same_id_fixture_overrides() {
        for card in original_config::original_base_card_definitions() {
            check(card.clone());
            // ID alone is deliberately insufficient as a cache key.
            let mut patched = card.clone();
            patched.name = "梦幻阵雷五行金灵木灵水灵火灵土灵剑崩星卦琴融汇机缘".into();
            patched.career_name = Some("QinShi".into());
            check(patched);
            let mut patched = card;
            patched.base_id = Some(389);
            patched.card_type = None;
            patched.name = "fixture-without-keywords".into();
            check(patched);
        }
    }

    #[test]
    fn deck_replacement_movement_and_removal_keep_exact_classification() {
        let definitions = original_config::original_base_card_definitions();
        let mut player = crate::replay::ReplayPlayer::default();
        for card in definitions {
            player.deck.slots.push(ReplayCardSlot {
                card: check(card),
                skipped: false,
                used: false,
            });
            if player.deck.slots.len() > 8 {
                player.deck.slots.remove(0);
            }
            player.deck.slots.reverse();
            let expected = player
                .deck
                .slots
                .iter()
                .fold(ActiveMechanisms::NONE, |mask, slot| {
                    mask | card_active_mechanisms(&slot.card)
                });
            assert_eq!(player_active_mechanisms(&player), expected);
            player.deck.slots[0].skipped = true;
            player.deck.slots[0].used = true;
            assert_eq!(player_active_mechanisms(&player), expected);
        }
    }
}
