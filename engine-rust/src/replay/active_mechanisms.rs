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
    if name.contains('雷')
        || id == 29
        || id == 4_000_030
        || (id >= 4_000_070 && id <= 4_040_075)
    {
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

pub(super) fn player_active_mechanisms(player: &super::ReplayPlayer) -> ActiveMechanisms {
    let mut mask = ActiveMechanisms::NONE;

    if !player.identity.fate_strategies.is_empty() {
        mask.insert(ActiveMechanisms::FATE_STRATEGY);
        if player.identity.fate_strategies.contains(&396) {
            mask.insert(ActiveMechanisms::THUNDER_MINDSET);
        }
        if player.identity.fate_strategies.contains(&380) {
            mask.insert(ActiveMechanisms::SWORD_SECT | ActiveMechanisms::FORMATION);
        }
        if player.identity.fate_strategies.contains(&416) {
            mask.insert(ActiveMechanisms::ELEMENT_FLOW);
        }
        if player
            .identity
            .fate_strategies
            .iter()
            .any(|&id| id == 319 || id == 322 || id == 381)
        {
            mask.insert(ActiveMechanisms::SWORD_SECT);
        }
    }

    if !player.identity.xian_mo_strategies.is_empty()
        || !player.identity.xian_mo_temp_datas.is_empty()
    {
        mask.insert(ActiveMechanisms::XIAN_MO);
        if player
            .identity
            .xian_mo_strategies
            .iter()
            .any(|&id| id == 7049 || id == 7068 || id == 7132)
        {
            mask.insert(ActiveMechanisms::SWORD_SECT);
        }
    }

    if !player.identity.talents.is_empty() {
        for &talent in player.identity.talents.iter() {
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

    if matches!(player.identity.talent_resonance_id, Some(10) | Some(109)) {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    if player
        .identity
        .ke_yin_card_ids
        .iter()
        .any(|id| id.rem_euclid(10_000) == 146)
    {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    // Check non-zero initial resource states
    if player.formations != super::ReplayFormationState::default() {
        mask.insert(ActiveMechanisms::FORMATION);
    }

    if player.sword != super::ReplaySwordState::default() {
        mask.insert(ActiveMechanisms::SWORD_SECT);
    }

    if player.dream_mirage != super::ReplayDreamMirageState::default()
        || player.mirage_ronghui != super::ReplayMirageRonghuiState::default()
    {
        mask.insert(ActiveMechanisms::MIRAGE);
    }

    if player.ronghui != super::ReplayRonghuiState::default() {
        mask.insert(ActiveMechanisms::RONGHUI);
    }

    if player.fate != super::ReplayFateTalentState::default() {
        mask.insert(ActiveMechanisms::FATE_STRATEGY);
        if player.fate.ke_yin_jing_lei != 0 {
            mask.insert(ActiveMechanisms::THUNDER_MINDSET);
        }
        // 星弈•连 is a fate counter, but its OnBeforeExecuted hook is gated by
        // ASTROLOGY. The counter is often gained mid-battle.
        if player.fate.xing_yi_lian > 0 {
            mask.insert(ActiveMechanisms::ASTROLOGY);
        }
    }

    if player.beng != super::ReplayBengQuanState::default() {
        mask.insert(ActiveMechanisms::BENG_QUAN);
    }

    if player.astrology.thunder_mindset != 0 || player.astrology.lei_shan_er_du != 0 {
        mask.insert(ActiveMechanisms::THUNDER_MINDSET);
    }
    if player.astrology != super::ReplayAstrologyState::default() {
        mask.insert(ActiveMechanisms::ASTROLOGY);
    }

    if player.elements != super::ReplayElementState::default() {
        mask.insert(ActiveMechanisms::ELEMENT_FLOW);
    }

    if player.music != super::ReplayMusicState::default() {
        mask.insert(ActiveMechanisms::MUSIC);
    }

    if player.chance != super::ReplayChanceState::default() {
        mask.insert(ActiveMechanisms::CHANCE);
    }

    // Deck slots
    for slot in player.deck.slots.iter() {
        mask.insert(card_active_mechanisms(&slot.card));
    }

    mask
}

pub(super) fn detect_active_mechanisms<const OBSERVE: bool>(
    state: &ReplayState<OBSERVE>,
) -> ActiveMechanisms {
    let mut mask = ActiveMechanisms::NONE;
    mask.insert(player_active_mechanisms(&state.p1));
    mask.insert(player_active_mechanisms(&state.p2));
    mask
}
