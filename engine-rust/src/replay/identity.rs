//! Identity-owned derived classification. Mutable borrows always invalidate it.
use super::{active_mechanisms, ActiveMechanisms, ReplayPlayerIdentity};
use std::cell::Cell;

#[derive(Clone, Default)]
pub(super) struct RuntimeIdentity(ReplayPlayerIdentity, Cell<Option<ActiveMechanisms>>);

impl RuntimeIdentity {
    pub(super) fn mechanisms(&self) -> ActiveMechanisms {
        if let Some(mask) = self.1.get() {
            return mask;
        }
        let mask = active_mechanisms::identity_active_mechanisms(&self.0);
        self.1.set(Some(mask));
        mask
    }
}

impl From<ReplayPlayerIdentity> for RuntimeIdentity {
    fn from(value: ReplayPlayerIdentity) -> Self {
        Self(value, Cell::new(None))
    }
}

impl std::ops::Deref for RuntimeIdentity {
    type Target = ReplayPlayerIdentity;
    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl std::ops::DerefMut for RuntimeIdentity {
    fn deref_mut(&mut self) -> &mut Self::Target {
        self.1.set(None);
        &mut self.0
    }
}

impl std::fmt::Debug for RuntimeIdentity {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        std::fmt::Debug::fmt(&self.0, f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn check(identity: &RuntimeIdentity) {
        assert_eq!(
            identity.mechanisms(),
            active_mechanisms::identity_active_mechanisms(identity)
        );
        assert_eq!(format!("{identity:?}"), format!("{:?}", identity.0));
    }

    #[test]
    fn identity_changes_and_clone_isolation_cannot_leave_stale_bits() {
        let mut identity = RuntimeIdentity::default();
        check(&identity);
        identity.fate_strategies = vec![396, 380, 416].into();
        check(&identity);
        identity.talents = vec![14, 137, 100_085].into();
        check(&identity);
        identity.xian_mo_strategies = vec![7049].into();
        identity.xian_mo_temp_datas.insert("7049".into(), 0);
        identity.talent_resonance_id = Some(109);
        identity.ke_yin_card_ids.push(10_146);
        check(&identity);
        let original = identity.mechanisms();
        let mut clone = identity.clone();
        clone.fate_strategies = vec![].into();
        clone.talents = vec![].into();
        clone.xian_mo_strategies = vec![].into();
        clone.xian_mo_temp_datas.clear();
        clone.talent_resonance_id = None;
        clone.ke_yin_card_ids.clear();
        check(&clone);
        assert_eq!(clone.mechanisms(), ActiveMechanisms::NONE);
        assert_eq!(identity.mechanisms(), original);
        check(&identity);
    }
}
