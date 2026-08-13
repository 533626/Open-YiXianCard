use super::{
    support, BattleFixture, CardDefinition, PlayerSide, ReplayEvent, ReplayObservationMode,
    ReplayPlayer, ReplayState, ReplaySummary,
};

#[derive(Debug, Clone)]
pub(crate) struct FateStrategyTestSnapshot {
    pub p1_defense: i64,
    pub p1_anima: i64,
    pub p2_internal_injury: i64,
    pub p2_flaw: i64,
    pub p2_hp: i64,
    pub action_again_count: i64,
}

impl ReplayState {
    pub(crate) fn test_from_fixture(fixture: &BattleFixture) -> Self {
        let mut state =
            Self::from_fixture(fixture, false).expect("lenient test replay construction");
        state.observation.mode = ReplayObservationMode::Events;
        state
    }

    pub(crate) fn test_play_actor_turn(&mut self) {
        self.execute_actor_turn();
    }

    pub(crate) fn test_advance_actor(&mut self) {
        self.current_actor = support::opponent_side(self.current_actor);
    }

    pub(crate) fn test_execute_one_card(&mut self, actor_side: PlayerSide) -> bool {
        self.execute_card_transaction(actor_side)
    }

    pub(crate) fn test_events(&self) -> &[ReplayEvent] {
        &self.observation.events
    }

    pub(crate) fn test_apply_card_effect(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        self.apply_card_effect(actor_side, card, slot, false);
    }

    pub(crate) fn test_resolve_action_again(
        &self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        self.resolve_card_action_again(actor_side, card, slot, false, false)
    }

    pub(crate) fn test_consume_action_again(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) -> bool {
        self.consume_action_again(actor_side, card, slot, false, false, false)
    }

    pub(crate) fn test_actor_card(&self, actor_side: PlayerSide, slot: usize) -> CardDefinition {
        self.actor(actor_side).deck.slots[slot].card.clone()
    }

    pub(crate) fn test_snapshot(&self, actor_side: PlayerSide) -> FateStrategyTestSnapshot {
        let actor = self.actor(actor_side);
        FateStrategyTestSnapshot {
            p1_defense: self.p1.core.defense,
            p1_anima: self.p1.core.anima,
            p2_internal_injury: self.p2.status.internal_injury,
            p2_flaw: self.p2.status.flaw,
            p2_hp: self.p2.core.hp,
            action_again_count: actor.turn.action_again_count,
        }
    }

    pub(crate) fn test_configure_p1<F>(&mut self, configure: F)
    where
        F: FnOnce(&mut ReplayPlayer),
    {
        configure(&mut self.p1);
    }

    pub(crate) fn test_configure_p2<F>(&mut self, configure: F)
    where
        F: FnOnce(&mut ReplayPlayer),
    {
        configure(&mut self.p2);
    }

    pub(crate) fn test_run(&mut self) -> ReplaySummary {
        self.run()
    }

    pub(crate) fn test_final_hp(&self) -> (i64, i64) {
        (self.p1.core.hp, self.p2.core.hp)
    }
}
