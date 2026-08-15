use super::{BattleError, ReplayCardExecution, ReplayState};
use crate::model::PlayerSide;
use serde::Serialize;

const NEGATIVE_STATUS_DECISION_TAG: u32 = 0x6e65_6773;
const PERCENT_ROLL_DECISION_TAG: u32 = 0x7065_7263;
const RANDOM_RANGE_DECISION_TAG: u32 = 0x7261_6e67;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplayDecisionKind {
    NegativeStatus,
    PercentRoll,
    RandomRange,
}

impl ReplayDecisionKind {
    fn hash_tag(self) -> u32 {
        match self {
            Self::NegativeStatus => NEGATIVE_STATUS_DECISION_TAG,
            Self::PercentRoll => PERCENT_ROLL_DECISION_TAG,
            Self::RandomRange => RANDOM_RANGE_DECISION_TAG,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ReplayDecisionProvider {
    ReplayTape,
    SeededSynthetic,
    Hexagram,
    RandomFallbackTape,
    DefaultValue,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayDecisionIntegerRange {
    pub min_inclusive: i64,
    pub max_inclusive: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum ReplayDecisionDomain {
    Discrete {
        #[serde(rename = "legalOptions")]
        legal_options: Vec<i64>,
    },
    IntegerRange {
        #[serde(rename = "legalRange")]
        legal_range: ReplayDecisionIntegerRange,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReplayDecisionEvent {
    pub provider: ReplayDecisionProvider,
    pub seed: Option<u32>,
    pub side: PlayerSide,
    pub actor_turn: i64,
    pub card_id: i64,
    pub decision_kind: ReplayDecisionKind,
    pub card_execution_occurrence: u64,
    pub decision_occurrence: u64,
    pub ordinal: i64,
    #[serde(flatten)]
    pub domain: ReplayDecisionDomain,
    pub selected_option: Option<i64>,
}

impl ReplayDecisionEvent {
    pub fn legal_options(&self) -> Option<&[i64]> {
        match &self.domain {
            ReplayDecisionDomain::Discrete { legal_options } => Some(legal_options),
            ReplayDecisionDomain::IntegerRange { .. } => None,
        }
    }

    pub fn legal_range(&self) -> Option<ReplayDecisionIntegerRange> {
        match &self.domain {
            ReplayDecisionDomain::Discrete { .. } => None,
            ReplayDecisionDomain::IntegerRange { legal_range } => Some(*legal_range),
        }
    }
}

pub(super) enum RandomRangeDecisionResolution {
    Unscoped,
    Selected(i64),
    Missing,
}

pub(super) enum PercentRollDecisionResolution {
    Unscoped,
    Selected(i64),
    Missing,
}

impl ReplayState {
    pub(super) fn begin_card_execution(
        &mut self,
        side: PlayerSide,
        card_id: i64,
    ) -> Option<ReplayCardExecution> {
        let previous = self.current_card_execution;
        self.card_execution_occurrence += 1;
        self.current_card_execution = Some(ReplayCardExecution {
            occurrence: self.card_execution_occurrence,
            side,
            card_id,
            percent_roll_ordinal: 0,
            random_range_ordinal: 0,
        });
        previous
    }

    pub(super) fn finish_card_execution(&mut self, previous: Option<ReplayCardExecution>) {
        self.current_card_execution = previous;
    }

    pub(super) fn resolve_negative_status_decision(
        &mut self,
        side: PlayerSide,
        card_id: i64,
        ordinal: i64,
        legal_options: &[i64],
    ) -> Option<i64> {
        self.decision_occurrence += 1;
        let decision_occurrence = self.decision_occurrence;
        let execution = self
            .current_card_execution
            .expect("typed card decision requested outside a card execution");
        assert_eq!(
            (execution.side, execution.card_id),
            (side, card_id),
            "typed decision card execution mismatch"
        );
        let card_execution_occurrence = execution.occurrence;
        let synthetic_seed = self
            .synthetic_decision_seed
            .filter(|_| self.synthetic_decision_sides.contains(&side));
        let (provider, seed, selected_option) = if let Some(seed) = synthetic_seed {
            let selected = (!legal_options.is_empty()).then(|| {
                let index = seeded_decision_choice_index(
                    seed,
                    side,
                    self.actor_turn,
                    card_id,
                    ReplayDecisionKind::NegativeStatus,
                    card_execution_occurrence,
                    decision_occurrence,
                    ordinal,
                    legal_options.len(),
                );
                legal_options[index]
            });
            (
                ReplayDecisionProvider::SeededSynthetic,
                Some(seed),
                selected,
            )
        } else {
            let replay_value = if self.decision_tape.is_empty() {
                None
            } else {
                let value = self.decision_tape.remove(0);
                (value >= 0).then_some(value)
            };
            if let Some(selected) = replay_value {
                (ReplayDecisionProvider::ReplayTape, None, Some(selected))
            } else if let Some(seed) = self.synthetic_decision_fallback_seed {
                let selected = (!legal_options.is_empty()).then(|| {
                    let index = seeded_decision_choice_index(
                        seed,
                        side,
                        self.actor_turn,
                        card_id,
                        ReplayDecisionKind::NegativeStatus,
                        card_execution_occurrence,
                        decision_occurrence,
                        ordinal,
                        legal_options.len(),
                    );
                    legal_options[index]
                });
                (
                    ReplayDecisionProvider::SeededSynthetic,
                    Some(seed),
                    selected,
                )
            } else {
                (ReplayDecisionProvider::ReplayTape, None, None)
            }
        };

        self.decision_events.push(ReplayDecisionEvent {
            provider,
            seed,
            side,
            actor_turn: self.actor_turn,
            card_id,
            decision_kind: ReplayDecisionKind::NegativeStatus,
            card_execution_occurrence,
            decision_occurrence,
            ordinal,
            domain: ReplayDecisionDomain::Discrete {
                legal_options: legal_options.to_vec(),
            },
            selected_option,
        });
        if let Some(selected) = selected_option.filter(|value| !is_negative_status_id(*value)) {
            if self.evaluation_error.is_none() {
                self.evaluation_error = Some(BattleError::InvalidDecision {
                    card_id,
                    kind: "negative-status",
                    selected,
                    turn: self.actor_turn,
                });
            }
            return None;
        }
        selected_option
    }

    pub(super) fn resolve_random_range_decision(
        &mut self,
        side: PlayerSide,
        min: i64,
        max: i64,
        used_hexagram: bool,
        default_value: Option<i64>,
    ) -> RandomRangeDecisionResolution {
        let Some(execution) = self.current_card_execution else {
            return RandomRangeDecisionResolution::Unscoped;
        };
        assert_eq!(
            execution.side, side,
            "typed random-range decision actor mismatch"
        );
        let ordinal = execution.random_range_ordinal;
        self.current_card_execution
            .as_mut()
            .expect("checked current card execution")
            .random_range_ordinal += 1;
        self.decision_occurrence += 1;
        let decision_occurrence = self.decision_occurrence;
        let Some(option_count) = random_range_option_count(min, max) else {
            self.missing_decision("random range invalid range");
            return RandomRangeDecisionResolution::Missing;
        };
        let synthetic_seed = self
            .synthetic_decision_seed
            .filter(|_| self.synthetic_decision_sides.contains(&side));
        let (provider, seed, selected_option) = if synthetic_seed.is_some() && used_hexagram {
            (ReplayDecisionProvider::Hexagram, None, Some(max))
        } else if let Some(seed) = synthetic_seed {
            let index = seeded_decision_choice_index(
                seed,
                side,
                self.actor_turn,
                execution.card_id,
                ReplayDecisionKind::RandomRange,
                execution.occurrence,
                decision_occurrence,
                ordinal,
                option_count,
            );
            (
                ReplayDecisionProvider::SeededSynthetic,
                Some(seed),
                Some(min + index as i64),
            )
        } else {
            let replay_value = if self.decision_tape.is_empty() {
                None
            } else {
                let value = self.decision_tape.remove(0);
                (value >= 0).then_some(value)
            };
            if replay_value.is_some() {
                (ReplayDecisionProvider::ReplayTape, None, replay_value)
            } else if let Some(seed) = self.synthetic_decision_fallback_seed {
                if used_hexagram {
                    (ReplayDecisionProvider::Hexagram, None, Some(max))
                } else {
                    let index = seeded_decision_choice_index(
                        seed,
                        side,
                        self.actor_turn,
                        execution.card_id,
                        ReplayDecisionKind::RandomRange,
                        execution.occurrence,
                        decision_occurrence,
                        ordinal,
                        option_count,
                    );
                    (
                        ReplayDecisionProvider::SeededSynthetic,
                        Some(seed),
                        Some(min + index as i64),
                    )
                }
            } else if !self.random_fallback_tape.is_empty() {
                let fallback = self.random_fallback_tape.remove(0);
                if fallback < min || fallback > max {
                    self.missing_decision("random range fallback out of range");
                    return RandomRangeDecisionResolution::Missing;
                }
                (
                    ReplayDecisionProvider::RandomFallbackTape,
                    None,
                    Some(fallback),
                )
            } else if let Some(default_value) = default_value {
                (
                    ReplayDecisionProvider::DefaultValue,
                    None,
                    Some(default_value),
                )
            } else {
                self.missing_decision("random range");
                return RandomRangeDecisionResolution::Missing;
            }
        };
        self.decision_events.push(ReplayDecisionEvent {
            provider,
            seed,
            side,
            actor_turn: self.actor_turn,
            card_id: execution.card_id,
            decision_kind: ReplayDecisionKind::RandomRange,
            card_execution_occurrence: execution.occurrence,
            decision_occurrence,
            ordinal,
            domain: ReplayDecisionDomain::IntegerRange {
                legal_range: ReplayDecisionIntegerRange {
                    min_inclusive: min,
                    max_inclusive: max,
                },
            },
            selected_option,
        });
        selected_option.map_or(
            RandomRangeDecisionResolution::Missing,
            RandomRangeDecisionResolution::Selected,
        )
    }

    pub(super) fn resolve_percent_roll_decision(
        &mut self,
        side: PlayerSide,
        used_hexagram: bool,
        suppress_missing_error: bool,
    ) -> PercentRollDecisionResolution {
        let Some(execution) = self.current_card_execution else {
            return PercentRollDecisionResolution::Unscoped;
        };
        assert_eq!(
            execution.side, side,
            "typed percent-roll decision actor mismatch"
        );
        let ordinal = execution.percent_roll_ordinal;
        self.current_card_execution
            .as_mut()
            .expect("checked current card execution")
            .percent_roll_ordinal += 1;
        self.decision_occurrence += 1;
        let decision_occurrence = self.decision_occurrence;
        let synthetic_seed = self
            .synthetic_decision_seed
            .filter(|_| self.synthetic_decision_sides.contains(&side));

        let resolved = if synthetic_seed.is_some() && used_hexagram {
            Some((ReplayDecisionProvider::Hexagram, None, 0))
        } else if let Some(seed) = synthetic_seed {
            let selected = seeded_decision_choice_index(
                seed,
                side,
                self.actor_turn,
                execution.card_id,
                ReplayDecisionKind::PercentRoll,
                execution.occurrence,
                decision_occurrence,
                ordinal,
                100,
            ) as i64;
            Some((
                ReplayDecisionProvider::SeededSynthetic,
                Some(seed),
                selected,
            ))
        } else {
            let replay_value = if self.decision_tape.is_empty() {
                None
            } else {
                Some(self.decision_tape.remove(0))
            };
            match replay_value {
                Some(value @ 0..=99) => Some((ReplayDecisionProvider::ReplayTape, None, value)),
                Some(value) if value >= 100 => {
                    self.missing_decision("percent roll replay out of range");
                    return PercentRollDecisionResolution::Missing;
                }
                Some(_) | None if self.synthetic_decision_fallback_seed.is_some() => {
                    if used_hexagram {
                        Some((ReplayDecisionProvider::Hexagram, None, 0))
                    } else {
                        let seed = self
                            .synthetic_decision_fallback_seed
                            .expect("checked fallback seed");
                        let selected = seeded_decision_choice_index(
                            seed,
                            side,
                            self.actor_turn,
                            execution.card_id,
                            ReplayDecisionKind::PercentRoll,
                            execution.occurrence,
                            decision_occurrence,
                            ordinal,
                            100,
                        ) as i64;
                        Some((
                            ReplayDecisionProvider::SeededSynthetic,
                            Some(seed),
                            selected,
                        ))
                    }
                }
                Some(_) | None if used_hexagram => {
                    Some((ReplayDecisionProvider::Hexagram, None, 0))
                }
                Some(_) | None if !self.random_fallback_tape.is_empty() => {
                    let fallback = self.random_fallback_tape.remove(0);
                    if !(0..=99).contains(&fallback) {
                        self.missing_decision("percent roll fallback out of range");
                        return PercentRollDecisionResolution::Missing;
                    }
                    Some((ReplayDecisionProvider::RandomFallbackTape, None, fallback))
                }
                Some(_) | None => {
                    if !suppress_missing_error {
                        self.missing_decision("percent roll");
                    }
                    None
                }
            }
        };
        let Some((provider, seed, selected_option)) = resolved else {
            return PercentRollDecisionResolution::Missing;
        };
        self.decision_events.push(ReplayDecisionEvent {
            provider,
            seed,
            side,
            actor_turn: self.actor_turn,
            card_id: execution.card_id,
            decision_kind: ReplayDecisionKind::PercentRoll,
            card_execution_occurrence: execution.occurrence,
            decision_occurrence,
            ordinal,
            domain: ReplayDecisionDomain::IntegerRange {
                legal_range: ReplayDecisionIntegerRange {
                    min_inclusive: 0,
                    max_inclusive: 99,
                },
            },
            selected_option: Some(selected_option),
        });
        PercentRollDecisionResolution::Selected(selected_option)
    }
}

fn random_range_option_count(min: i64, max: i64) -> Option<usize> {
    let width = max.checked_sub(min)?.checked_add(1)?;
    if !(1..=4_294_967_296_i64).contains(&width) {
        return None;
    }
    usize::try_from(width).ok()
}

fn is_negative_status_id(value: i64) -> bool {
    matches!(value, 100 | 101 | 102 | 103 | 104 | 105 | 367 | 393)
}

#[allow(clippy::too_many_arguments)]
pub(super) fn seeded_decision_choice_index(
    seed: u32,
    side: PlayerSide,
    actor_turn: i64,
    card_id: i64,
    decision_kind: ReplayDecisionKind,
    card_execution_occurrence: u64,
    decision_occurrence: u64,
    ordinal: i64,
    option_count: usize,
) -> usize {
    assert!(
        option_count > 0 && option_count as u128 <= 4_294_967_296_u128,
        "seeded decision option count must be in 1..=4294967296"
    );
    let mut value = seed;
    for part in [
        match side {
            PlayerSide::P1 => 1_u32,
            PlayerSide::P2 => 2_u32,
        },
        actor_turn as u32,
        card_id as u32,
        decision_kind.hash_tag(),
        card_execution_occurrence as u32,
        decision_occurrence as u32,
        ordinal as u32,
    ] {
        value = mix_seeded_decision(value ^ part);
    }
    value as usize % option_count
}

fn mix_seeded_decision(input: u32) -> u32 {
    let mut value = input;
    value = (value ^ (value >> 16)).wrapping_mul(0x7feb_352d);
    value = (value ^ (value >> 15)).wrapping_mul(0x846c_a68b);
    value ^ (value >> 16)
}
