use super::{
    original_card_definition_by_id, support, BattleFixture, CardDefinition, PlayerSide,
    ReplayEvent, ReplayObservationMode, ReplayPlayer, ReplayState, ReplaySummary,
};
use crate::fixture::{FixtureExpected, FixturePlayer, FixturePlayers};
use crate::model::DECK_SIZE;

/// Canonical shared builders for replay unit tests. This module is the single
/// home for the card/deck/player/fixture boilerplate that used to be
/// copy-pasted at the top of every `tests_*.rs` file; test modules keep thin
/// same-named wrappers (or import these directly) so call sites read unchanged
/// while the struct-literal duplication lives here exactly once.

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

/// Blank ad-hoc card: mirrors the `test_card` helper formerly duplicated in
/// every test module (all-`None` stats, `other_params` empty).
pub(crate) fn test_card(id: i64, base_id: i64, name: &str) -> CardDefinition {
    CardDefinition {
        id,
        base_id: Some(base_id),
        name: name.to_string(),
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

/// Inline 3-attack filler card (self-contained, no catalog dependency).
/// Delegates to the canonical production constructor so the id/damage
/// constants live in exactly one place (`support::basic_attack_card`).
pub(crate) fn basic_attack_card() -> CardDefinition {
    support::basic_attack_card()
}

/// Catalog-backed card definition; panics on unknown id like the old per-file
/// `original_card` helpers (panic text is unified; no test asserts on it).
pub(crate) fn original_card(id: i64) -> CardDefinition {
    original_card_definition_by_id(id).unwrap_or_else(|| panic!("missing original card {id}"))
}

/// Pad `cards` to [`DECK_SIZE`] by cloning `filler`. Cloning one filler
/// instance is observably identical to calling a `basic_attack()` constructor
/// per slot (`CardDefinition` is plain data; `PartialEq`-equal either way).
pub(crate) fn fill_deck(
    mut cards: Vec<CardDefinition>,
    filler: CardDefinition,
) -> Vec<CardDefinition> {
    while cards.len() < DECK_SIZE {
        cards.push(filler.clone());
    }
    cards
}

/// Resize to exactly eight slots, preserving callers that previously used `resize_with`.
pub(crate) fn resize_deck(
    mut cards: Vec<CardDefinition>,
    filler: CardDefinition,
) -> Vec<CardDefinition> {
    cards.resize(DECK_SIZE, filler);
    cards
}

#[test]
fn deck_builders_preserve_padding_and_truncation_contracts() {
    let filler = basic_attack_card();
    let oversized = vec![filler.clone(); DECK_SIZE + 1];
    assert_eq!(
        fill_deck(oversized.clone(), filler.clone()).len(),
        DECK_SIZE + 1
    );
    assert_eq!(resize_deck(oversized, filler.clone()).len(), DECK_SIZE);
    assert_eq!(fill_deck(Vec::new(), filler.clone()).len(), DECK_SIZE);
    assert_eq!(resize_deck(Vec::new(), filler).len(), DECK_SIZE);
}

/// Single active card padded with inline basic attacks.
pub(crate) fn basic_deck(active: CardDefinition) -> Vec<CardDefinition> {
    fill_deck(vec![active], basic_attack_card())
}

/// Shared [`FixturePlayer`] constructor. Every per-file `player` literal only
/// varied these six fields; all other fields are the same defaults everywhere
/// (empty talents/buffs/maps, zero initials, no character override). Callers
/// with genuinely custom players (character ids, preset talents) keep local
/// constructors and are excluded from delegation.
#[allow(clippy::too_many_arguments)]
pub(crate) fn make_player(
    cards: Vec<CardDefinition>,
    level: i64,
    base_max_hp: i64,
    extra_max_hp: Option<i64>,
    active_slot_count: usize,
    momentum_limit: Option<i64>,
) -> FixturePlayer {
    FixturePlayer {
        level,
        base_max_hp,
        extra_max_hp,
        battle_start_hp: None,
        character_id: None,
        talents: Vec::new(),
        fate_strategies: Vec::new(),
        fate_strategy_temp_datas: Default::default(),
        active_slot_count,
        initial_defense: 0,
        initial_anima: 0,
        initial_guard: 0,
        initial_momentum: 0,
        initial_momentum_limit: momentum_limit,
        initial_agility: 0,
        initial_battle_buffs: Default::default(),
        permanent_buff_temp_datas: Default::default(),
        talent_resonance_id: None,
        used_ke_yin_cards: Vec::new(),
        talent_temp_datas: Default::default(),
        talent_card_params: Default::default(),
        last_round_used_card_base_ids: Vec::new(),
        last_round_life: None,
        last_round_exp: 0,
        hand_cards: Vec::new(),
        cards,
    }
}

/// Shared [`BattleFixture`] constructor over two ready-made players.
pub(crate) fn make_fixture(
    p1: FixturePlayer,
    p2: FixturePlayer,
    winner_side: PlayerSide,
    actor_turn_count: i64,
    hp_delta_p1_minus_p2: i64,
    max_actor_turns: i64,
) -> BattleFixture {
    BattleFixture {
        schema_version: 1,
        source: None,
        first_player_side: PlayerSide::P1,
        decision_tape: Vec::new(),
        random_fallback_tape: Vec::new(),
        expected: FixtureExpected {
            winner_side,
            actor_turn_count,
            hp_delta_p1_minus_p2,
            final_hp: None,
        },
        max_actor_turns: Some(max_actor_turns),
        historical_card_overrides: Vec::new(),
        catalog_cards: Vec::new(),
        players: FixturePlayers { p1, p2 },
    }
}

/// The overwhelmingly common 1-turn P1-wins placeholder expectation.
pub(crate) fn default_fixture(p1: FixturePlayer, p2: FixturePlayer) -> BattleFixture {
    make_fixture(p1, p2, PlayerSide::P1, 1, 0, 1)
}
