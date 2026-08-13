// Ordered turn/card-play lifecycle phase tables (execution lives in flow.rs).
//
// The tables are `pub(in crate::replay)`-visible to `flow` and re-exported there; the
// enum+const-slice pattern mirrors `ORIGINAL_AFTER_HP_MODIFY_PHASES` in
// player.rs. Each entry carries the original (IL/method) anchor comment.

/// BattleCharacter.OnTurnStarted turn-start sequence, in current-build source
/// order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::replay) enum TurnStartPhase {
    ResetActionAgain,
    ClearTurnHpGainedLedgers,
    MirageRonghuiTurnStart,
    RonghuiTurnStart,
    DreamMirageDurationTicks,
    ResetTurnStartFlags,
    MarkSpiritTurtleFootwork,
    BuffDurationTicks,
    ResetFateTurnFlags,
    VermilionBirdTearGuard,
    DefenseDecay,
    TideWaterMomentum,
    InfiniteHexagramPlate,
    SecretSwordMindset,
    TurnStartTuneEffects,
    WoodSpiritAllGrowth,
    MysticHeartRecovery,
    TurnStartHealing,
    TurnStartChanceHooks,
    NextTurnDefense,
    InternalInjuryTick,
    PostInjuryFormations,
    SpiritGatheringAnima,
    DreamMirageTurnStartLate,
}

pub(in crate::replay) const TURN_START_PHASES: [TurnStartPhase; 24] = [
    TurnStartPhase::ResetActionAgain,
    TurnStartPhase::ClearTurnHpGainedLedgers,
    TurnStartPhase::MirageRonghuiTurnStart,
    TurnStartPhase::RonghuiTurnStart,
    TurnStartPhase::DreamMirageDurationTicks,
    TurnStartPhase::ResetTurnStartFlags,
    TurnStartPhase::MarkSpiritTurtleFootwork,
    TurnStartPhase::BuffDurationTicks,
    TurnStartPhase::ResetFateTurnFlags,
    TurnStartPhase::VermilionBirdTearGuard,
    TurnStartPhase::DefenseDecay,
    TurnStartPhase::TideWaterMomentum,
    TurnStartPhase::InfiniteHexagramPlate,
    TurnStartPhase::SecretSwordMindset,
    TurnStartPhase::TurnStartTuneEffects,
    TurnStartPhase::WoodSpiritAllGrowth,
    TurnStartPhase::MysticHeartRecovery,
    TurnStartPhase::TurnStartHealing,
    TurnStartPhase::TurnStartChanceHooks,
    TurnStartPhase::NextTurnDefense,
    TurnStartPhase::InternalInjuryTick,
    TurnStartPhase::PostInjuryFormations,
    TurnStartPhase::SpiritGatheringAnima,
    TurnStartPhase::DreamMirageTurnStartLate,
];

/// BattleCharacter.OnTurnEnded turn-end sequence, in current-build source
/// order. The hook-bearing variants map 1:1 onto `ReplayTurnEndHookReceipt`
/// keys; hook_trace.rs pins that receipt order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::replay) enum TurnEndPhase {
    Talent66Defense,
    FateStrategy84SwordIntent,
    Ronghui,
    MirageRonghui,
    DreamBeforeWater,
    Formations,
    WaterMomentum,
    TemporaryResources,
    HardBranchBamboo,
    SanWeiHuan,
    PoisonImmunity,
    ResetSpiritTurtleFootwork,
    PendingHexagram,
    ResetHundredBeastSpiritSwordMarker,
    ClearDreamThunderRoundLimit,
    ResetActionAgain,
    StatusDecay,
    FengMoPhysique,
    LedgerReset,
}

pub(in crate::replay) const TURN_END_PHASES: [TurnEndPhase; 19] = [
    TurnEndPhase::Talent66Defense,
    TurnEndPhase::FateStrategy84SwordIntent,
    TurnEndPhase::Ronghui,
    TurnEndPhase::MirageRonghui,
    TurnEndPhase::DreamBeforeWater,
    TurnEndPhase::Formations,
    TurnEndPhase::WaterMomentum,
    TurnEndPhase::TemporaryResources,
    TurnEndPhase::HardBranchBamboo,
    TurnEndPhase::SanWeiHuan,
    TurnEndPhase::PoisonImmunity,
    TurnEndPhase::ResetSpiritTurtleFootwork,
    TurnEndPhase::PendingHexagram,
    TurnEndPhase::ResetHundredBeastSpiritSwordMarker,
    TurnEndPhase::ClearDreamThunderRoundLimit,
    TurnEndPhase::ResetActionAgain,
    TurnEndPhase::StatusDecay,
    TurnEndPhase::FengMoPhysique,
    TurnEndPhase::LedgerReset,
];

/// BattleExecuter/CardActionBase.Execute per-card-transaction sequence, in
/// source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::replay) enum CardPlayPhase {
    PreflightResolution,
    PrepareTransaction,
    ExternalRepeatSources,
    PrimaryEffect,
    SpiritFormationEcho,
    FinishTransaction,
}

pub(in crate::replay) const CARD_PLAY_PHASES: [CardPlayPhase; 6] = [
    CardPlayPhase::PreflightResolution,
    CardPlayPhase::PrepareTransaction,
    CardPlayPhase::ExternalRepeatSources,
    CardPlayPhase::PrimaryEffect,
    CardPlayPhase::SpiritFormationEcho,
    CardPlayPhase::FinishTransaction,
];

/// CardActionBase.ExecuteEffect per-invocation sequence, in source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::replay) enum CardEffectPhase {
    OpenInvocation,
    PlayCardEntry,
    PreBodyHooks,
    Body,
    ActionAgain,
    AfterCardHooks,
    CloseInvocation,
}

pub(in crate::replay) const CARD_EFFECT_PHASES: [CardEffectPhase; 7] = [
    CardEffectPhase::OpenInvocation,
    CardEffectPhase::PlayCardEntry,
    CardEffectPhase::PreBodyHooks,
    CardEffectPhase::Body,
    CardEffectPhase::ActionAgain,
    CardEffectPhase::AfterCardHooks,
    CardEffectPhase::CloseInvocation,
];
