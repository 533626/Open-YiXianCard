//! Cached pure default comparison; every mutable access invalidates it.
use std::cell::Cell;

#[derive(Clone)]
pub(super) struct MechanismState<T>(T, Cell<Option<bool>>);

impl<T: Default + Eq> MechanismState<T> {
    pub(super) fn is_nondefault(&self) -> bool {
        if let Some(value) = self.1.get() {
            return value;
        }
        let value = self.0 != T::default();
        self.1.set(Some(value));
        value
    }
}

impl<T: Default> Default for MechanismState<T> {
    fn default() -> Self {
        Self(T::default(), Cell::new(Some(false)))
    }
}

impl<T> From<T> for MechanismState<T> {
    fn from(value: T) -> Self {
        Self(value, Cell::new(None))
    }
}

impl<T> std::ops::Deref for MechanismState<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.0
    }
}

impl<T> std::ops::DerefMut for MechanismState<T> {
    fn deref_mut(&mut self) -> &mut T {
        self.1.set(None);
        &mut self.0
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for MechanismState<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

impl<T: PartialEq> PartialEq for MechanismState<T> {
    fn eq(&self, other: &Self) -> bool {
        self.0 == other.0
    }
}
impl<T: Eq> Eq for MechanismState<T> {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::replay::ReplayFateTalentState;

    #[test]
    fn clearing_a_counter_and_mutating_nested_vectors_invalidate_each_clone() {
        let mut state: MechanismState<ReplayFateTalentState> = MechanismState::default();
        assert!(!state.is_nondefault());
        state.xing_yi_lian = 1;
        assert!(state.is_nondefault());
        let mut clone = state.clone();
        clone.xing_yi_lian = 0;
        assert!(!clone.is_nondefault());
        assert!(state.is_nondefault());
        clone.fate_cycle_slots[0] = 3;
        assert!(clone.is_nondefault());
        clone.fate_cycle_slots[0] = 0;
        assert!(!clone.is_nondefault());
        clone.skip_hp_deltas.push(3);
        assert!(clone.is_nondefault());
        clone.skip_hp_deltas.clear();
        assert!(!clone.is_nondefault());
        assert_eq!(
            format!("{clone:?}"),
            format!("{:?}", ReplayFateTalentState::default())
        );
        assert_eq!(
            clone,
            MechanismState::from(ReplayFateTalentState::default())
        );
    }
}
