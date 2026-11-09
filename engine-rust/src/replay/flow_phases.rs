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
    MarkSpiritTurtleFootwork,
    MirageRonghuiTurnStart,
    DreamMirageDurationTicks,
    ResetTurnStartFlags,
    BuffDurationTicks,
    ResetFateTurnFlags,
    VermilionBirdTearGuard,
    DefenseDecay,
    TideWaterMomentum,
    InfiniteHexagramPlate,
    SpiritDrain,
    SecretSwordMindset,
    TurnStartTuneEffects,
    WoodSpiritAllGrowth,
    MysticHeartRecovery,
    TurnStartChanceHooks,
    NextTurnDefense,
    SpiritGatheringAnima,
    SpiritGatheringFormation,
    RonghuiTurnStart,
    TurnStartHealing,
    DreamGreatReturnPill,
    InternalInjuryTick,
    PostInjuryFormations,
    DreamMirageTurnStartLate,
}

pub(in crate::replay) const TURN_START_PHASES: [TurnStartPhase; 27] = [
    TurnStartPhase::ResetActionAgain,
    TurnStartPhase::ClearTurnHpGainedLedgers,
    // BattleCharacter.OnTurnStarted marks the opponent's 灵玄迷踪步 before
    // any later turn-start hook can make that opponent lose HP.
    TurnStartPhase::MarkSpiritTurtleFootwork,
    TurnStartPhase::MirageRonghuiTurnStart,
    TurnStartPhase::DreamMirageDurationTicks,
    TurnStartPhase::ResetTurnStartFlags,
    TurnStartPhase::BuffDurationTicks,
    TurnStartPhase::ResetFateTurnFlags,
    TurnStartPhase::VermilionBirdTearGuard,
    TurnStartPhase::DefenseDecay,
    TurnStartPhase::TideWaterMomentum,
    TurnStartPhase::SecretSwordMindset,
    TurnStartPhase::TurnStartTuneEffects,
    TurnStartPhase::WoodSpiritAllGrowth,
    TurnStartPhase::MysticHeartRecovery,
    TurnStartPhase::TurnStartChanceHooks,
    TurnStartPhase::NextTurnDefense,
    TurnStartPhase::SpiritGatheringAnima,
    TurnStartPhase::SpiritGatheringFormation,
    // Card_208 OnTurnStarted IL_0f04 follows defense decay and the two
    // spirit-gathering hooks (IL_0d84/IL_0e56), preserving new overflow defense.
    TurnStartPhase::RonghuiTurnStart,
    TurnStartPhase::TurnStartHealing,
    TurnStartPhase::DreamGreatReturnPill,
    // OnTurnStarted IL_1655 follows recovery IL_139a and 梦•大还丹 IL_14f3.
    TurnStartPhase::InfiniteHexagramPlate,
    // 梦•水灵波澜临时加倍（BattleCharacter.cs OnTurnStarted :4493-4504，内嵌的
    // ModifyAnima 连带静气心法治疗 :9729-9731）先于内伤 tick（:4273-4290 取值，
    // :4545-4558 循环结算）：治疗先落（可能触顶），再吃内伤。
    // oracle 锚点：pa_1bbf4b84 cp[22]（t24：+15 灵气 → 静气心法回 60
    // 触顶 124，再 -30 内伤到 94；旧顺序先 -30 再回满 124）。
    TurnStartPhase::DreamMirageTurnStartLate,
    // OnTurnStarted IL_0d84/IL_0e56/IL_0f04/IL_1448 all precede this
    // internal-injury tick at IL_1a4c.
    TurnStartPhase::InternalInjuryTick,
    TurnStartPhase::PostInjuryFormations,
    TurnStartPhase::SpiritDrain,
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
    Formations,
    DreamBeforeWater,
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
    TurnEndPhase::Formations,
    // HuiChunQu (IL_0b61) updates the turn healing ledger before MengKuangEr
    // (IL_18c6) tests it; both still precede ordinary water damage (IL_1cd0).
    TurnEndPhase::DreamBeforeWater,
    TurnEndPhase::WaterMomentum,
    TurnEndPhase::TemporaryResources,
    TurnEndPhase::HardBranchBamboo,
    TurnEndPhase::SanWeiHuan,
    TurnEndPhase::PoisonImmunity,
    TurnEndPhase::PendingHexagram,
    TurnEndPhase::ResetHundredBeastSpiritSwordMarker,
    TurnEndPhase::ClearDreamThunderRoundLimit,
    TurnEndPhase::ResetActionAgain,
    TurnEndPhase::StatusDecay,
    TurnEndPhase::FengMoPhysique,
    // OnTurnEnded:6350-6401 decays negative statuses before clearing 350.
    TurnEndPhase::ResetSpiritTurtleFootwork,
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

// 伏魔 5009 不是纯装饰：CardActionBase.cs:1470 块经 IL_3658→IL_3703→IL_37d2
// 把伏魔格上的筑基及以下非隐藏牌再执行一次（R-855f195a 取证）。再执行并入
// SpiritFormationEcho 阶段尾部（原版 1470-流程落子晚于灵阵回响），不另增阶段。
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
