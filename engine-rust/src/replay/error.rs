#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum BattleError {
    #[error("{message}; turn={turn}")]
    UnsupportedBuild { message: String, turn: i64 },

    #[error("card catalog error: card:{card_id} base:{base_id} {reason}; turn={turn}")]
    MissingRule {
        card_id: i64,
        base_id: i64,
        reason: String,
        turn: i64,
    },

    #[error("missing original decision: {reason}; turn={turn}")]
    MissingDecision { reason: String, turn: i64 },

    #[error("invalid decision: card:{card_id}:{kind}={selected}; turn={turn}")]
    InvalidDecision {
        card_id: i64,
        kind: &'static str,
        selected: i64,
        turn: i64,
    },

    #[error("{message}")]
    Invariant { message: String },
}
