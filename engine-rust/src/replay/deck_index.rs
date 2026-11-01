//! Derived deck membership. Every mutable vector access invalidates the index,
//! including transformations, temporary replacements, and direct slot edits.
use super::{support::normalized_base_id, ReplayCardSlot};
use std::cell::Cell;

pub(super) const PUBLIC_BENG: u8 = 1;
pub(super) const ADJACENT_DREAM: u8 = 2;
pub(super) const STAR_POINT_DREAM: u8 = 4;

#[derive(Debug, Clone)]
pub(super) struct ReplaySlots {
    slots: Vec<ReplayCardSlot>,
    dirty: Cell<bool>,
    base_ids: Cell<[i64; 8]>,
    raw_ids: Cell<[i64; 8]>,
    flags: Cell<u8>,
}
impl Default for ReplaySlots {
    fn default() -> Self {
        Vec::new().into()
    }
}
impl From<Vec<ReplayCardSlot>> for ReplaySlots {
    fn from(slots: Vec<ReplayCardSlot>) -> Self {
        Self {
            slots,
            dirty: Cell::new(true),
            base_ids: Cell::new([0; 8]),
            raw_ids: Cell::new([0; 8]),
            flags: Cell::new(0),
        }
    }
}
impl FromIterator<ReplayCardSlot> for ReplaySlots {
    fn from_iter<T: IntoIterator<Item = ReplayCardSlot>>(slots: T) -> Self {
        slots.into_iter().collect::<Vec<_>>().into()
    }
}
impl std::ops::Deref for ReplaySlots {
    type Target = Vec<ReplayCardSlot>;
    fn deref(&self) -> &Self::Target {
        &self.slots
    }
}
impl std::ops::DerefMut for ReplaySlots {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.dirty.set(true);
        &mut self.slots
    }
}
impl<'a> IntoIterator for &'a ReplaySlots {
    type Item = &'a ReplayCardSlot;
    type IntoIter = std::slice::Iter<'a, ReplayCardSlot>;
    fn into_iter(self) -> Self::IntoIter {
        self.slots.iter()
    }
}
impl<'a> IntoIterator for &'a mut ReplaySlots {
    type Item = &'a mut ReplayCardSlot;
    type IntoIter = std::slice::IterMut<'a, ReplayCardSlot>;
    fn into_iter(self) -> Self::IntoIter {
        self.dirty.set(true);
        self.slots.iter_mut()
    }
}
impl ReplaySlots {
    fn refresh(&self) {
        if !self.dirty.replace(false) {
            return;
        }
        let mut bases = [0; 8];
        let mut raw = [0; 8];
        for (i, slot) in self.slots.iter().enumerate().take(8) {
            bases[i] = normalized_base_id(&slot.card);
            raw[i] = slot.card.id;
        }
        // Used/skip edits also acquire mutable slot references. They change no
        // membership; avoid reclassifying all cards when only those flags moved.
        if self.slots.len() <= 8 && raw == self.raw_ids.get() && bases == self.base_ids.get() {
            return;
        }
        let flags = self.slots.iter().fold(0, |flags, slot| {
            flags | super::dream_mirage_hooks::deck_hook_flags(&slot.card)
        });
        self.base_ids.set(bases);
        self.raw_ids.set(raw);
        self.flags.set(flags);
    }
    #[inline]
    pub(super) fn contains_base(&self, id: i64) -> bool {
        if self.slots.len() > 8 {
            return self
                .slots
                .iter()
                .any(|slot| normalized_base_id(&slot.card) == id);
        }
        self.refresh();
        self.base_ids.get()[..self.slots.len()].contains(&id)
    }
    #[inline]
    pub(super) fn has_hooks(&self, flag: u8) -> bool {
        self.refresh();
        self.flags.get() & flag != 0
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::{original_config::original_card_definition, support::basic_attack_card};
    fn slot(card: crate::model::CardDefinition) -> ReplayCardSlot {
        ReplayCardSlot {
            card: card.into(),
            used: false,
            skipped: false,
        }
    }
    fn check(slots: &ReplaySlots) {
        for id in [-1, 0, 312, 415, 1_000_069, 1_000_075, 10_000_084] {
            assert_eq!(
                slots.contains_base(id),
                slots
                    .iter()
                    .any(|slot| normalized_base_id(&slot.card) == id),
                "{id}"
            );
        }
        let expected = slots.iter().fold(0, |flags, slot| {
            flags | crate::replay::dream_mirage_hooks::deck_hook_flags(&slot.card)
        });
        for flag in [PUBLIC_BENG, ADJACENT_DREAM, STAR_POINT_DREAM] {
            assert_eq!(slots.has_hooks(flag), expected & flag != 0);
        }
    }
    #[test]
    fn deck_membership_tracks_replacements_same_id_base_changes_and_clones() {
        let mut slots: ReplaySlots = vec![slot(basic_attack_card())].into();
        check(&slots);
        slots[0].used = true;
        check(&slots);
        slots[0].card = original_card_definition(10_000_084).unwrap().into();
        check(&slots);
        let mut patched = slots[0].card.definition().clone();
        patched.base_id = Some(1_000_069);
        slots[0].card = patched.into();
        check(&slots);
        assert!(slots.has_hooks(ADJACENT_DREAM));
        let mut copied = slots.clone();
        copied[0].card = basic_attack_card().into();
        check(&copied);
        check(&slots);
        assert!(!copied.has_hooks(ADJACENT_DREAM));
        assert!(slots.has_hooks(ADJACENT_DREAM));
        slots[0].card = basic_attack_card().into();
        check(&slots);
        assert!(!slots.has_hooks(ADJACENT_DREAM));
    }
    #[test]
    fn long_noncanonical_decks_keep_exact_membership_and_invalidation() {
        let mut slots: ReplaySlots = (0..9).map(|_| slot(basic_attack_card())).collect();
        slots[8].card = original_card_definition(10_000_084).unwrap().into();
        check(&slots);
        assert!(slots.contains_base(10_000_084));
        slots.pop();
        check(&slots);
        assert!(!slots.contains_base(10_000_084));
        slots.clear();
        check(&slots);
    }
}
