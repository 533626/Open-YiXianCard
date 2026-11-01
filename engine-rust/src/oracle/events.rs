//! `battle-evaluator/scripts/lib/rust-replay-events.ts` 的 Rust 收敛：
//! 事件流/扩展流构造改为进程内直接调 lib（`run_replay_fixture_with_parity_events`
//! / `run_replay_fixture_with_detailed_events`），不再需要进程包装。
//!
//! 行为与 `replay_slice --events` / `--events-extended` 的序列化层一致
//! （kind/actor 字符串化、extended 快照 = parity 29 字段 + 显示缺口字段 +
//! detail_entries 9 个持久计数），但输出收敛为 TS 解析后可见的形状
//! （`parseRustReplayEventStream` / `parseRustExtendedReplayEventStream`）。

use serde_json::{Map, Value};
use crate::ReplayDetailedEvent;
use crate::replay::wire::{event_snapshot, EventStream, EventRow};
use crate::{
    run_replay_fixture_with_detailed_events, run_replay_fixture_with_parity_events, BattleFixture,
    EngineError, PlayerSide, ReplayEventKind, ReplayRun,
};

use super::buff_map::{
    RustExtendedEvent, RustExtendedEventStream, RustExtendedSnapshot, RustExtendedTerminal,
    RustQueuedCard, Side,
};

/// Run the canonical Rust event protocol for one already-verified fixture，
/// 返回 TS `parseRustReplayEventStream` 解析后可见的形状。
pub fn run_replay_event_stream(
    fixture: &BattleFixture,
    fixture_label: &str,
) -> Result<RustReplayEventStream, EngineError> {
    let run = run_replay_fixture_with_parity_events(fixture)?;
    Ok(RustReplayEventStream {
        fixture: fixture_label.to_string(),
        events: run.events.iter().map(parse_replay_event).collect(),
    })
}

/// `replay_slice --events` 的线协议行（wire 合同原样）。
pub fn parity_event_row(
    event: &crate::ReplayEvent,
    run: &ReplayRun,
) -> EventRow {
    EventRow {
        kind: event_kind(event.kind).to_string(),
        turn: event.turn,
        actor: player_side(event.actor).to_string(),
        slot: event.slot,
        card_id: event.card_id,
        card_name: event.card_name.clone(),
        termination_cause: (event.kind == ReplayEventKind::BattleEnd)
            .then_some(run.termination_cause),
        completed_checkpoint_count: (event.kind == ReplayEventKind::BattleEnd)
            .then_some(run.completed_checkpoint_count),
        p1: event_snapshot(&event.p1),
        p2: event_snapshot(&event.p2),
    }
}

/// `replay_slice --events` 的线协议流。
pub fn parity_event_stream(fixture: &str, run: &ReplayRun) -> EventStream {
    EventStream {
        fixture: fixture.to_string(),
        events: run.events.iter().map(|event| parity_event_row(event, run)).collect(),
    }
}

/// TS `parseRustReplayEventStream` 后可见的 parity 事件形状。
#[derive(Debug, Clone, PartialEq)]
pub struct RustReplayEvent {
    pub kind: String,
    pub turn: i64,
    pub actor: Side,
    pub slot: Option<i64>,
    pub card_id: Option<i64>,
    pub card_name: Option<String>,
    /// 29 字段 parity 快照（serde 按协议字段序序列化）。
    pub p1: Value,
    pub p2: Value,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RustReplayEventStream {
    pub fixture: String,
    pub events: Vec<RustReplayEvent>,
}

impl RustReplayEvent {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("kind".into(), Value::String(self.kind.clone()));
        map.insert("turn".into(), Value::from(self.turn));
        map.insert("actor".into(), Value::String(self.actor.as_str().into()));
        if let Some(slot) = self.slot {
            map.insert("slot".into(), Value::from(slot));
        }
        if let Some(card_id) = self.card_id {
            map.insert("cardId".into(), Value::from(card_id));
        }
        if let Some(card_name) = &self.card_name {
            map.insert("cardName".into(), Value::String(card_name.clone()));
        }
        map.insert("p1".into(), self.p1.clone());
        map.insert("p2".into(), self.p2.clone());
        Value::Object(map)
    }
}

impl RustReplayEventStream {
    pub fn to_json(&self) -> Value {
        let mut map = Map::new();
        map.insert("fixture".into(), Value::String(self.fixture.clone()));
        map.insert(
            "events".into(),
            Value::Array(self.events.iter().map(RustReplayEvent::to_json).collect()),
        );
        Value::Object(map)
    }
}

fn parse_replay_event(event: &crate::ReplayEvent) -> RustReplayEvent {
    RustReplayEvent {
        kind: event_kind(event.kind).to_string(),
        turn: event.turn,
        actor: player_side_to_side(event.actor),
        slot: event.slot.map(|slot| slot as i64),
        card_id: event.card_id,
        card_name: event.card_name.clone(),
        p1: serde_json::to_value(event_snapshot(&event.p1)).expect("parity snapshot serializes"),
        p2: serde_json::to_value(event_snapshot(&event.p2)).expect("parity snapshot serializes"),
    }
}

/// Run the extended Rust event protocol (`--events-extended`): the 29-field
/// parity snapshot plus momentumLimit/physiqueLimit/quanStance/gunStance, so
/// original BuffType 298/10024/443/444 participate in the full buffs comparison.
pub fn run_replay_event_stream_extended(
    fixture: &BattleFixture,
    fixture_label: &str,
) -> Result<RustExtendedEventStream, EngineError> {
    let run = run_replay_fixture_with_detailed_events(fixture)?;
    Ok(extended_oracle_stream(fixture_label, &run))
}

/// 与 replay_slice `extended_event_stream` 同语义，但只保留 TS 解析层可见的
/// fixture/terminal/events（decisionEvents/attackSegments/turnEndHooks 不进
/// oracle 对照形状）。
pub fn extended_oracle_stream(fixture_label: &str, run: &crate::ReplayDetailedRun) -> RustExtendedEventStream {
    let last = run
        .events
        .last()
        .expect("detailed replay produced no events");
    RustExtendedEventStream {
        fixture: fixture_label.to_string(),
        terminal: RustExtendedTerminal {
            winner_side: player_side_to_side(run.summary.winner_side),
            actor_turn: run.summary.actor_turn_count,
            hp_delta: run.summary.hp_delta_p1_minus_p2,
            p1_hp: last.event.p1.hp,
            p2_hp: last.event.p2.hp,
            remaining_decision_params: run.remaining_decision_params as i64,
        },
        events: run.events.iter().map(extended_oracle_event).collect(),
    }
}

pub fn extended_oracle_event(detailed: &ReplayDetailedEvent) -> RustExtendedEvent {
    RustExtendedEvent {
        kind: event_kind(detailed.event.kind).to_string(),
        turn: detailed.event.turn,
        actor: player_side_to_side(detailed.event.actor),
        slot: detailed.event.slot.map(|slot| slot as i64),
        card_id: detailed.event.card_id,
        card_name: detailed.event.card_name.clone(),
        p1: RustExtendedSnapshot::from_engine(
            &detailed.event.p1,
            &detailed.p1,
            &detailed.p1_state,
        ),
        p2: RustExtendedSnapshot::from_engine(
            &detailed.event.p2,
            &detailed.p2,
            &detailed.p2_state,
        ),
        p1_card_queue: queued_cards(&detailed.p1_state.card_queue),
        p2_card_queue: queued_cards(&detailed.p2_state.card_queue),
        p1_physical_deck: queued_cards(&detailed.p1_state.physical_deck),
        p2_physical_deck: queued_cards(&detailed.p2_state.physical_deck),
    }
}

fn queued_cards(slots: &[crate::ReplayQueuedCard]) -> Vec<RustQueuedCard> {
    slots
        .iter()
        .map(|slot| RustQueuedCard {
            id: slot.id,
            grid: slot.grid as i64,
            skip: slot.skip,
            had_used: slot.had_used,
        })
        .collect()
}

pub fn event_kind(kind: ReplayEventKind) -> &'static str {
    match kind {
        ReplayEventKind::BattleStart => "battleStart",
        ReplayEventKind::TurnStart => "turnStart",
        ReplayEventKind::CardCompleted => "cardCompleted",
        ReplayEventKind::TurnEnd => "turnEnd",
        ReplayEventKind::BattleEnd => "battleEnd",
    }
}

fn player_side(side: PlayerSide) -> &'static str {
    match side {
        PlayerSide::P1 => "p1",
        PlayerSide::P2 => "p2",
    }
}

fn player_side_to_side(side: PlayerSide) -> Side {
    match side {
        PlayerSide::P1 => Side::P1,
        PlayerSide::P2 => Side::P2,
    }
}
