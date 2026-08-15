use super::effect_invocation::{EffectInvocationKind, EffectInvocationPhase};
use super::support::{has_cloud_chain, opponent_side};
use super::ReplayState;
use crate::model::{CardDefinition, PlayerSide};

struct CompletedCardTransaction {
    drawn: super::DrawnCard,
    was_slot_used: bool,
    primary_card_action_again: bool,
    event_index: usize,
}

struct PreparedCardTransaction {
    drawn: super::DrawnCard,
    physical_card: CardDefinition,
    public_adjacent_beng_quan: bool,
    was_slot_used: bool,
    event_index: usize,
}

/// Immutable per-card-play context shared by every ExecuteEffect repetition of
/// one outer transaction. Only `effect_card`, the repetition index and the
/// invocation kind vary between the ordered repeat sources, the primary play
/// and the trailing 灵阵回响 echo, so those stay as call arguments.
struct EffectRepetitionContext<'card> {
    physical_card: &'card CardDefinition,
    origin_card: &'card CardDefinition,
    source_slot: usize,
    was_slot_used: bool,
    public_adjacent_beng_quan: bool,
    card_completed_event_index: usize,
}

/// External repeat sources checked, in original order, before the selected
/// card's primary ExecuteEffect. The order is rule semantics: an earlier
/// complete effect can create the buff a later source consumes.
#[derive(Clone, Copy)]
enum RepeatSource {
    PlumBlossom,
    SecretSwordDoubleDragon,
    LeiShanErDu,
    BengQuanDoubleShadow,
    DreamMirageFireEarth,
}

const REPEAT_SOURCES: [RepeatSource; 5] = [
    RepeatSource::PlumBlossom,
    RepeatSource::SecretSwordDoubleDragon,
    // 原版 Execute 顺序（CardActionBase.cs:1304-1521）：双龙（IL_2118）→
    // 雷闪二度（IL_2281）→ 崩拳双影（IL_2555）。
    RepeatSource::LeiShanErDu,
    RepeatSource::BengQuanDoubleShadow,
    RepeatSource::DreamMirageFireEarth,
];

#[path = "flow_phases.rs"]
mod flow_phases;

pub(super) use flow_phases::{
    CardEffectPhase, CardPlayPhase, TurnEndPhase, TurnStartPhase, CARD_EFFECT_PHASES,
    CARD_PLAY_PHASES, TURN_END_PHASES, TURN_START_PHASES,
};

/// Cached one-shot read of the per-card replay trace toggle. The dump is a
/// debugging aid on the hot finish path, so the env lookup must not repeat per
/// card play in solver/GA batches.
fn replay_trace_enabled() -> bool {
    static ENABLED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    *ENABLED.get_or_init(|| std::env::var_os("YIXIAN_RUST_REPLAY_TRACE").is_some())
}

/// Immutable per-card-play context shared by every ExecuteEffect repetition of
/// one outer transaction (see `EffectRepetitionContext`).
fn repetition_context(prepared: &PreparedCardTransaction) -> EffectRepetitionContext<'_> {
    EffectRepetitionContext {
        physical_card: &prepared.physical_card,
        origin_card: &prepared.drawn.card,
        source_slot: prepared.drawn.source_slot,
        was_slot_used: prepared.was_slot_used,
        public_adjacent_beng_quan: prepared.public_adjacent_beng_quan,
        card_completed_event_index: prepared.event_index,
    }
}

impl ReplayState {
    fn turn_end_hook_pair(&self) -> super::ReplayTurnEndHookPair {
        super::ReplayTurnEndHookPair {
            p1: self.turn_end_hook_snapshot(PlayerSide::P1),
            p2: self.turn_end_hook_snapshot(PlayerSide::P2),
        }
    }

    fn turn_end_hook_snapshot(&self, side: PlayerSide) -> super::ReplayTurnEndHookSnapshot {
        let actor = self.actor(side);
        super::ReplayTurnEndHookSnapshot {
            hp: actor.core.hp,
            max_hp: actor.core.max_hp,
            defense: actor.core.defense,
            anima: actor.core.anima,
            guard: actor.core.guard,
            physique: actor.core.physique,
            momentum: actor.beng.momentum,
            water_momentum: actor.elements.water_momentum,
            attack_bonus: actor.core.attack_bonus,
            internal_injury: actor.status.internal_injury,
            weakness: actor.status.weakness,
            flaw: actor.status.flaw,
            attack_reduction: actor.status.attack_reduction,
            entangle: actor.status.entangle,
            external_injury: actor.status.external_injury,
            lose_hp_count: actor.turn.lose_hp_count,
            lose_hp_times_count: actor.turn.lose_hp_times_count,
        }
    }

    fn observe_turn_end_hook<R>(
        &mut self,
        actor_side: PlayerSide,
        hook: &'static str,
        apply: impl FnOnce(&mut Self) -> R,
    ) -> R {
        if !self.observation.mode.is_detailed() {
            return apply(self);
        }
        let before = self.turn_end_hook_pair();
        let result = apply(self);
        let after = self.turn_end_hook_pair();
        self.observation
            .turn_end_hooks
            .push(super::ReplayTurnEndHookReceipt {
                turn: self.actor_turn,
                actor: actor_side,
                hook,
                before,
                after,
            });
        result
    }

    /// Builds the initial state and executes the complete battle-start phase.
    /// Strictness must be selected before that phase because opening effects
    /// can consume decisions or require referenced card definitions.
    pub(super) fn run(&mut self) -> super::ReplaySummary {
        while self.actor_turn < self.max_actor_turns {
            self.execute_actor_turn();
            if self.evaluation_error.is_some() {
                return self.summary(self.hp_winner());
            }
            if let Some(winner_side) = self.death_winner() {
                return self.summary(winner_side);
            }
            self.current_actor = opponent_side(self.current_actor);
        }
        let winner = self.hp_winner();
        self.termination_cause = Some(super::ReplayTerminationCause::MaxTurn);
        self.summary(winner)
    }

    pub(super) fn execute_actor_turn(&mut self) {
        self.actor_turn += 1;
        let actor_side = self.current_actor;
        let raw_turn_start_event_index = if self.observation.mode.is_parity() {
            None
        } else {
            self.record_event(super::ReplayEventKind::TurnStart, actor_side, None, None);
            Some(self.observation.events.len().saturating_sub(1))
        };
        self.apply_turn_start_phases(actor_side);
        self.completed_checkpoint_count += 1;
        // The TurnStart checkpoint timing is rule semantics, not just a
        // collection choice. Parity snapshots AFTER the turn-start hooks to
        // match the original client's post-injury/heal checkpoint; the
        // event/detailed streams snapshot BEFORE them (raw_turn_start_event_index
        // above) to expose the pre-hook frame. Only the parity stream feeds
        // exact comparison, so the two are intentionally not interchangeable.
        if self.observation.mode.is_parity() {
            self.record_event(super::ReplayEventKind::TurnStart, actor_side, None, None);
        }
        self.record_detail_step(
            raw_turn_start_event_index
                .unwrap_or_else(|| self.observation.events.len().saturating_sub(1)),
            super::ReplayHookCategory::TurnStart,
            actor_side,
            None,
            None,
        );

        if self.death_winner().is_some() {
            self.termination_cause = Some(super::ReplayTerminationCause::TurnStartLethal);
            return;
        }

        if self.actor(actor_side).status.cannot_act > 0 {
            self.actor_mut(actor_side).status.cannot_act -= 1;
        } else {
            let mut continue_acting = true;
            while continue_acting {
                continue_acting = self.execute_card_transaction(actor_side);
                if self.death_winner().is_some() || self.evaluation_error.is_some() {
                    break;
                }
            }
        }
        if self.evaluation_error.is_some() {
            return;
        }
        let completed_turn = self.death_winner().is_none();
        if !completed_turn && self.termination_cause.is_none() {
            // 出牌阶段致死，但没走到 finish_card_transaction 里的分类点——例如
            // preflight 阶段的牌前效果直接打死人，卡牌本身没有完整结算，循环靠
            // `continue_acting == false` 退出。此时 TurnStartLethal 已经判过、
            // TurnEndLethal 又被 completed_turn 挡住，死因会留空，最终在
            // run_replay_fixture_with_observation 里炸成 "replay has no termination
            // cause"（2026-07-25 实测 4000005:7 self-play seed 1：actor_turn=54、
            // p1_hp=0、winner=P2，胜负和血量都已正确，缺的只是死因标签）。
            // 这里只补分类，不改任何战斗结果。
            self.termination_cause = Some(super::ReplayTerminationCause::CardLethal);
        }
        if completed_turn {
            self.apply_turn_end_phases(actor_side);
            self.completed_checkpoint_count += 1;
        }
        if completed_turn || !self.observation.mode.is_parity() {
            self.record_event(super::ReplayEventKind::TurnEnd, actor_side, None, None);
        }
        if completed_turn {
            self.record_detail_step(
                self.observation.events.len().saturating_sub(1),
                super::ReplayHookCategory::TurnEnd,
                actor_side,
                None,
                None,
            );
        }
        if completed_turn && self.death_winner().is_some() {
            self.termination_cause = Some(super::ReplayTerminationCause::TurnEndLethal);
        }
    }

    /// Executes `TURN_START_PHASES` in table order. `water_month_sword_formation`
    /// is snapshotted in `MarkSpiritTurtleFootwork` because the buff-duration
    /// ticks immediately after decrement the formation counter itself; the
    /// decay decision must read the pre-tick value.
    fn apply_turn_start_phases(&mut self, actor_side: PlayerSide) {
        self.attribution_block = Some(super::TraceAttributionBlock::TurnStart);
        let mut water_month_sword_formation = 0;
        for phase in TURN_START_PHASES {
            match phase {
                TurnStartPhase::ResetActionAgain => {
                    let before = self.actor(actor_side).turn.action_again_count;
                    self.actor_mut(actor_side).turn.action_again_count = 0;
                    self.record_counter_transition(
                        actor_side,
                        "回合",
                        "actionAgainCount",
                        "再次行动次数",
                        before,
                        0,
                    );
                }
                TurnStartPhase::ClearTurnHpGainedLedgers => {
                    self.clear_turn_hp_gained_ledgers(actor_side);
                }
                TurnStartPhase::MirageRonghuiTurnStart => {
                    self.apply_mirage_ronghui_turn_start(actor_side);
                }
                TurnStartPhase::RonghuiTurnStart => {
                    self.apply_ronghui_turn_start(actor_side);
                }
                TurnStartPhase::DreamMirageDurationTicks => {
                    self.apply_dream_mirage_turn_start_duration_ticks(actor_side);
                }
                TurnStartPhase::ResetTurnStartFlags => {
                    self.actor_mut(actor_side).turn.current_turn_ignore_defense = 0;
                    self.actor_mut(actor_side).astrology.pending_anima_hexagram = false;
                    self.clear_temporary_guard_at_turn_start(actor_side);
                    self.actor_mut(actor_side)
                        .beng
                        .momentum_gain_agility_triggered = 0;
                }
                TurnStartPhase::MarkSpiritTurtleFootwork => {
                    let opponent = opponent_side(actor_side);
                    if self.actor(opponent).turn.spirit_turtle_footwork > 0 {
                        self.actor_mut(opponent)
                            .turn
                            .spirit_turtle_footwork_triggered = 1;
                    }
                    water_month_sword_formation =
                        self.actor(actor_side).sword.water_month_sword_formation;
                }
                TurnStartPhase::BuffDurationTicks => {
                    self.apply_turn_start_buff_decrements(actor_side);
                }
                TurnStartPhase::ResetFateTurnFlags => {
                    self.actor_mut(actor_side).fate.dismantle_move = 0;
                    self.actor_mut(actor_side).fate.chan_xin_ju_ling_triggered = 0;
                    self.actor_mut(actor_side).fate.hot_blood_to_qi_triggered = 0;
                    self.actor_mut(actor_side)
                        .fate
                        .five_elements_gathering_triggered = 0;
                    self.actor_mut(actor_side).turn.turn_attack_segments = 0;
                }
                TurnStartPhase::VermilionBirdTearGuard => {
                    if self.actor(actor_side).fate.vermilion_bird_tear > 0
                        && self.actor(actor_side).core.hp <= 20
                    {
                        self.actor_mut(actor_side).fate.vermilion_bird_tear -= 1;
                        self.gain_guard(actor_side, 1);
                    }
                }
                TurnStartPhase::DefenseDecay => {
                    if water_month_sword_formation <= 0 {
                        let defense_before_decay = self.actor(actor_side).core.defense;
                        if self.actor(actor_side).chance.di_xuan_gui > 0 {
                            self.decay_actor_defense_percent(actor_side, 20);
                        } else {
                            self.decay_actor_defense(actor_side);
                        }
                        let defense_lost =
                            (defense_before_decay - self.actor(actor_side).core.defense).max(0);
                        self.apply_dream_mirage_defense_decay_healing(actor_side, defense_lost);
                    }
                }
                TurnStartPhase::TideWaterMomentum => {
                    let tide = self.actor(actor_side).fate.tide.max(0);
                    if tide > 0 {
                        self.gain_water_momentum(actor_side, tide);
                    }
                }
                TurnStartPhase::InfiniteHexagramPlate => {
                    if self.actor(actor_side).astrology.infinite_hexagram_plate > 0 {
                        self.gain_hexagram(
                            actor_side,
                            self.actor(actor_side).astrology.infinite_hexagram_plate,
                        );
                    }
                }
                TurnStartPhase::SecretSwordMindset => {
                    self.apply_secret_sword_mindset_at_turn_start(actor_side);
                }
                TurnStartPhase::TurnStartTuneEffects => {
                    // 原版 OnTurnStarted：断肠曲等曲效果在 IL_0701，先于
                    // 吞天赤眼兽等回合开始效果（IL_0a8a）。
                    self.apply_turn_start_tune_effects(actor_side);
                }
                TurnStartPhase::WoodSpiritAllGrowth => {
                    self.apply_turn_start_wood_spirit_all_growth(actor_side);
                }
                TurnStartPhase::MysticHeartRecovery => {
                    let mystic_heart = self
                        .actor(actor_side)
                        .fate
                        .mystic_heart_enter_profound
                        .max(0);
                    if mystic_heart > 0 {
                        self.add_actor_negative_status(actor_side, 100, mystic_heart);
                        self.actor_mut(actor_side).status.recovery += mystic_heart;
                    }
                }
                TurnStartPhase::TurnStartHealing => {
                    let recovery = self.actor(actor_side).status.recovery.max(0);
                    if recovery > 0 {
                        let multiplier = if self
                            .actor(actor_side)
                            .fate
                            .resonance_mystic_heart_enter_profound
                            > 0
                        {
                            2
                        } else {
                            1
                        };
                        self.modify_actor_hp(actor_side, recovery * multiplier, false, false);
                    }
                }
                TurnStartPhase::DreamGreatReturnPill => {
                    // 原版 OnTurnStarted IL_1448：梦•大还丹在常规治疗后、
                    // 内伤 IL_1a4c 前比较双方生命/上限。
                    self.apply_dream_great_return_pill_at_turn_start(actor_side);
                }
                TurnStartPhase::TurnStartChanceHooks => {
                    // 原版 OnTurnStarted：吞天赤眼兽吸血（IL_0a8a）在内伤 tick
                    // （IL_1a4c）之前。满血开局时吸血被 maxHp 封顶浪费，再被内伤
                    // 扣血；引擎原先反过来导致满血+内伤+吸血构型多 2 hp。
                    // oracle 锚点：4bac1c815f4268db/round-09 cp[10] p1.hp 66 vs 68。
                    self.apply_turn_start_chance_hooks(actor_side);
                }
                TurnStartPhase::NextTurnDefense => {
                    // 原版 OnTurnStarted：“下回合加防”到 IL_0c42 才兑现，晚于
                    // 断肠曲（IL_0701）与吞天赤眼兽等回合开始效果（IL_0a8a）。
                    // 该顺序使断肠曲新增负面状态触发的伤害不会提前消耗这批防。
                    // oracle 锚点：355d309d0bd7ed33/round-09 cp13 p2 hp/def
                    // 42/15（引擎原先 44/13）。
                    let next_turn_defense = self.actor(actor_side).turn.next_turn_defense;
                    if next_turn_defense > 0 {
                        self.gain_defense(actor_side, next_turn_defense);
                        let before = self.actor(actor_side).turn.next_turn_defense;
                        self.actor_mut(actor_side).turn.next_turn_defense = 0;
                        self.record_counter_transition(
                            actor_side,
                            "回合",
                            "nextTurnDefense",
                            "下回合加防",
                            before,
                            0,
                        );
                    }
                }
                TurnStartPhase::InternalInjuryTick => {
                    // 原版 OnTurnStarted：内伤 tick 在 IL_1a4c，晚于回合开始
                    // 治疗与吞天赤眼兽吸血（IL_0a8a）。
                    let internal_injury_trigger_count =
                        self.consume_internal_injury_trigger_count(actor_side);
                    let internal_injury = self.actor(actor_side).status.internal_injury.max(0);
                    if internal_injury > 0 {
                        let multiplier = if self
                            .actor(actor_side)
                            .fate
                            .resonance_mystic_heart_enter_profound
                            > 0
                        {
                            2
                        } else {
                            1
                        };
                        for _ in 0..internal_injury_trigger_count {
                            self.modify_actor_hp(
                                actor_side,
                                -(internal_injury * multiplier),
                                false,
                                false,
                            );
                        }
                        let transient = self
                            .actor(actor_side)
                            .status
                            .transient_internal_injury
                            .max(0);
                        if transient > 0 {
                            let actor = self.actor_mut(actor_side);
                            actor.status.internal_injury =
                                (actor.status.internal_injury - transient).max(0);
                            actor.status.transient_internal_injury = 0;
                        }
                    }
                }
                TurnStartPhase::PostInjuryFormations => {
                    self.apply_turn_start_post_injury_formations(actor_side);
                }
                TurnStartPhase::SpiritGatheringAnima => {
                    let spirit_gathering =
                        self.actor(actor_side).fate.spirit_gathering_mindset.max(0);
                    if spirit_gathering > 0 {
                        let mut anima_gain = spirit_gathering / 2;
                        if spirit_gathering % 2 == 1 {
                            if self.actor(actor_side).fate.half_anima > 0 {
                                anima_gain += 1;
                                self.actor_mut(actor_side).fate.half_anima -= 1;
                            } else {
                                self.actor_mut(actor_side).fate.half_anima = 1;
                            }
                        }
                        if anima_gain > 0 {
                            self.gain_anima(actor_side, anima_gain);
                        }
                    }
                }
                TurnStartPhase::DreamMirageTurnStartLate => {
                    self.apply_dream_mirage_turn_start_late(actor_side);
                }
            }
        }
        self.attribution_block = None;
    }

    /// Executes `TURN_END_PHASES` in table order. The `statusDecay` receipt
    /// feeds the `fengMoPhysique` phase that immediately follows it.
    fn apply_turn_end_phases(&mut self, actor_side: PlayerSide) {
        self.attribution_block = Some(super::TraceAttributionBlock::TurnEnd);
        let mut decayed = 0;
        for phase in TURN_END_PHASES {
            match phase {
                TurnEndPhase::Talent66Defense => {
                    if self.actor(actor_side).identity.talents.contains(&66)
                        && self.actor(actor_side).turn.turn_attack_segments == 0
                    {
                        self.gain_defense(actor_side, 3);
                    }
                }
                TurnEndPhase::FateStrategy84SwordIntent => {
                    if self
                        .actor(actor_side)
                        .identity
                        .fate_strategies
                        .contains(&84)
                        && self.actor(actor_side).turn.turn_attack_segments == 0
                    {
                        self.modify_sword_intent(actor_side, 1);
                    }
                }
                TurnEndPhase::Ronghui => {
                    self.observe_turn_end_hook(actor_side, "ronghui", |state| {
                        state.apply_ronghui_turn_end(actor_side);
                    });
                }
                TurnEndPhase::MirageRonghui => {
                    self.observe_turn_end_hook(actor_side, "mirageRonghui", |state| {
                        state.apply_mirage_ronghui_turn_end(actor_side);
                    });
                }
                TurnEndPhase::DreamBeforeWater => {
                    self.observe_turn_end_hook(actor_side, "dreamBeforeWater", |state| {
                        state.apply_dream_mirage_turn_end_before_water(actor_side);
                    });
                }
                TurnEndPhase::Formations => {
                    // 原版 OnTurnEnded：万花迷魂阵吸取（BattleCharacter.cs:6000-6105）
                    // 先于水势伤害（:6231-6245）。护体整段抵挡只消耗 1 层，
                    // 顺序反了会把护体耗在错误那一击上，级联到七星解命。
                    // oracle 锚点：873d4eaec9236f24/round-17 cp[24] 起 t18 差 7。
                    self.observe_turn_end_hook(actor_side, "formations", |state| {
                        state.trigger_turn_end_formations(actor_side);
                    });
                }
                TurnEndPhase::WaterMomentum => {
                    self.observe_turn_end_hook(actor_side, "waterMomentum", |state| {
                        let water_momentum = state.actor(actor_side).elements.water_momentum;
                        if water_momentum > 0 {
                            state.apply_turn_end_water_momentum_damage(actor_side, water_momentum);
                        }
                    });
                }
                TurnEndPhase::TemporaryResources => {
                    self.observe_turn_end_hook(actor_side, "temporaryResources", |state| {
                        state.restore_dream_mirage_temporary_turn_resources(actor_side);
                    });
                }
                TurnEndPhase::HardBranchBamboo => {
                    self.observe_turn_end_hook(actor_side, "hardBranchBamboo", |state| {
                        let hard_branch_bamboo =
                            state.actor(actor_side).formations.hard_branch_bamboo.max(0);
                        let defense_per_damage = state
                            .actor(actor_side)
                            .formations
                            .hard_branch_bamboo_defense_per_damage
                            .max(0);
                        if hard_branch_bamboo > 0 && defense_per_damage > 0 {
                            let damage = (state.actor(actor_side).core.defense
                                / defense_per_damage)
                                * hard_branch_bamboo;
                            if damage > 0 {
                                state.apply_damage(actor_side, damage, false, false, false);
                            }
                        }
                    });
                }
                TurnEndPhase::SanWeiHuan => {
                    self.observe_turn_end_hook(actor_side, "sanWeiHuan", |state| {
                        let san_wei_huan = state.actor(actor_side).chance.san_wei_huan.max(0);
                        if san_wei_huan > 0 {
                            state.actor_mut(actor_side).fate.exorcism += san_wei_huan;
                        }
                    });
                }
                TurnEndPhase::PoisonImmunity => {
                    self.observe_turn_end_hook(actor_side, "poisonImmunity", |state| {
                        let poison_immunity = state.actor(actor_side).status.poison_immunity.max(0);
                        if poison_immunity > 0 {
                            let healing =
                                poison_immunity.min(state.negative_status_stack_count(actor_side));
                            if healing > 0 {
                                state.modify_actor_hp(actor_side, healing, false, false);
                            }
                        }
                    });
                }
                TurnEndPhase::ResetSpiritTurtleFootwork => {
                    self.actor_mut(actor_side)
                        .turn
                        .spirit_turtle_footwork_triggered = 0;
                    self.actor_mut(opponent_side(actor_side))
                        .turn
                        .spirit_turtle_footwork_triggered = 0;
                }
                TurnEndPhase::PendingHexagram => {
                    self.observe_turn_end_hook(actor_side, "pendingHexagram", |state| {
                        if state.actor(actor_side).astrology.pending_anima_hexagram {
                            state.actor_mut(actor_side).astrology.pending_anima_hexagram = false;
                            state.gain_hexagram(actor_side, 1);
                        }
                    });
                }
                TurnEndPhase::ResetHundredBeastSpiritSwordMarker => {
                    self.reset_hundred_beast_spirit_sword_formation_marker(actor_side);
                }
                TurnEndPhase::ClearDreamThunderRoundLimit => {
                    self.clear_dream_thunder_round_limit(actor_side);
                }
                TurnEndPhase::ResetActionAgain => {
                    let before = self.actor(actor_side).turn.action_again_count;
                    self.actor_mut(actor_side).turn.action_again_count = 0;
                    self.record_counter_transition(
                        actor_side,
                        "回合",
                        "actionAgainCount",
                        "再次行动次数",
                        before,
                        0,
                    );
                }
                TurnEndPhase::StatusDecay => {
                    let decay = self.observe_turn_end_hook(actor_side, "statusDecay", |state| {
                        state.actor_mut(actor_side).tick_turn_end_statuses()
                    });
                    // 原版 OnTurnEnded 对虚弱/破绽/困缚逐层走 ModifyBuffValue(-1)
                    // （BattleCharacter.cs:5686-5695），Negative 分类 delta != 0 会
                    // 触发卡 415 疯魔架势被动 → ModifyTiPo（8711-8713）。
                    decayed = decay.weakness + decay.flaw + decay.entangle;
                }
                TurnEndPhase::FengMoPhysique => {
                    self.observe_turn_end_hook(actor_side, "fengMoPhysique", |state| {
                        if decayed > 0 {
                            state.apply_feng_mo_stance_physique(actor_side, decayed);
                        }
                    });
                }
                TurnEndPhase::LedgerReset => {
                    self.observe_turn_end_hook(actor_side, "ledgerReset", |state| {
                        state.reset_mirage_ronghui_first_hp_loss_rewards();
                        state.clear_turn_hp_gained_ledgers(actor_side);
                    });
                }
            }
        }
        self.attribution_block = None;
    }

    pub(super) fn execute_card_transaction(&mut self, actor_side: PlayerSide) -> bool {
        let mut prepared: Option<PreparedCardTransaction> = None;
        let mut effects_executed = 0_i64;
        let mut primary_card_action_again = false;
        for phase in CARD_PLAY_PHASES {
            match phase {
                CardPlayPhase::PreflightResolution => {
                    if !self.preflight_card_transaction(actor_side) {
                        return false;
                    }
                }
                CardPlayPhase::PrepareTransaction => {
                    prepared = match self.prepare_card_transaction(actor_side) {
                        Some(transaction) => Some(transaction),
                        None => return false,
                    };
                }
                CardPlayPhase::ExternalRepeatSources => {
                    let prepared_ref = prepared
                        .as_ref()
                        .expect("prepare phase must precede repeat sources");
                    let repetition_ctx = repetition_context(prepared_ref);
                    // CardActionBase.Execute checks each external repeat source only when
                    // control reaches that branch. Do not aggregate a count up front: an
                    // earlier complete effect may create the buff consumed by a later
                    // source. Source order is 梅开 -> 双龙 -> 崩拳双影 -> 聚焰 ->
                    // primary -> 灵阵回响.
                    for source in REPEAT_SOURCES {
                        if self.consume_repeat_source(
                            source,
                            actor_side,
                            &prepared_ref.drawn.card,
                            prepared_ref.public_adjacent_beng_quan,
                        ) {
                            self.execute_selected_card_effect_repetition(
                                actor_side,
                                &repetition_ctx,
                                &prepared_ref.drawn.card,
                                effects_executed,
                                EffectInvocationKind::Repeated,
                            );
                            effects_executed += 1;
                        }
                    }
                }
                CardPlayPhase::PrimaryEffect => {
                    let prepared_ref = prepared
                        .as_ref()
                        .expect("prepare phase must precede primary effect");
                    let repetition_ctx = repetition_context(prepared_ref);
                    // Only the selected card's primary ExecuteEffect supplies the outer
                    // transaction's dynamic action-again snapshot. The trailing echo runs
                    // a base-card lifecycle, then the original restores the effective
                    // selected config before final action-again resolution.
                    primary_card_action_again = self.execute_selected_card_effect_repetition(
                        actor_side,
                        &repetition_ctx,
                        &prepared_ref.drawn.card,
                        effects_executed,
                        EffectInvocationKind::Played,
                    );
                    effects_executed += 1;
                }
                CardPlayPhase::SpiritFormationEcho => {
                    let prepared_ref = prepared
                        .as_ref()
                        .expect("prepare phase must precede spirit-formation echo");
                    if self.apply_spirit_formation_echo_setup(actor_side, &prepared_ref.drawn.card)
                    {
                        let echo_card =
                            self.spirit_formation_echo_card(actor_side, &prepared_ref.drawn.card);
                        let repetition_ctx = repetition_context(prepared_ref);
                        self.execute_selected_card_effect_repetition(
                            actor_side,
                            &repetition_ctx,
                            &echo_card,
                            effects_executed,
                            EffectInvocationKind::Repeated,
                        );
                    }
                    self.clear_spirit_formation_echo_triggered(actor_side);
                    debug_assert!(self.effect_invocation_stack.is_empty());
                }
                CardPlayPhase::FinishTransaction => {
                    let prepared = prepared.take().expect("prepare phase must precede finish");
                    return self.finish_card_transaction(
                        actor_side,
                        CompletedCardTransaction {
                            drawn: prepared.drawn,
                            was_slot_used: prepared.was_slot_used,
                            primary_card_action_again,
                            event_index: prepared.event_index,
                        },
                    );
                }
            }
        }
        unreachable!("FinishTransaction is the terminal card-play phase")
    }

    /// CardActionBase.CheckAnima's alternative-payment cascade. Each branch is
    /// a distinct original payment path with its own side effects; the
    /// repeated assignment mirrors the shared `num = anima` tail of the source.
    #[allow(clippy::if_same_then_else)]
    fn prepare_card_transaction(
        &mut self,
        actor_side: PlayerSide,
    ) -> Option<PreparedCardTransaction> {
        self.actor_mut(actor_side).fate.rear_move_succeeded = false;
        let skip_limit = self.nameless_white_deer_skip_limit();
        let mut drawn = self.actor_mut(actor_side).draw_next_card(skip_limit)?;
        // FateStrategy 398 applies its +5 only when the FS398 skip loop itself
        // skipped the fifth grid (and that grid holds a 星弈 card). If a prior
        // mechanism in the original's skip chain (星弈断 XingYi_Duan, 梦EJie…)
        // already consumed the fifth grid, the FS398 while loop re-enters on the
        // next card and never heals — see BattleExecuter.cs:1857-1864.
        // Keep this at the BattleExecuter selection boundary so the HP gain
        // precedes the next card's cost/effect hooks.
        if drawn.fate_398_skipped_fifth_grid
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&398)
            && self
                .actor(actor_side)
                .identity
                .fate_strategy_temp_datas
                .get("398")
                .copied()
                .unwrap_or(0)
                == 0
            && self
                .actor(actor_side)
                .deck
                .slots
                .get(4)
                .is_some_and(|slot| slot.card.name.contains("星弈"))
        {
            self.modify_actor_hp(actor_side, 5, false, false);
        }
        while self.should_skip_card_with_instant_shadow_strike(actor_side, &drawn) {
            self.trigger_skipped_opening_effects(actor_side, &drawn);
            self.apply_instant_shadow_strike_skip(actor_side, &drawn);
            drawn = self.actor_mut(actor_side).draw_next_card(skip_limit)?;
        }
        self.trigger_skipped_opening_effects(actor_side, &drawn);
        let physical_card = drawn.card.clone();
        let public_adjacent_beng_quan =
            self.dream_mirage_public_adjacent_beng_quan(actor_side, drawn.source_slot);
        let selected_card_is_beng_quan = public_adjacent_beng_quan
            || self.is_dream_mirage_intrinsic_beng_quan(actor_side, &drawn.card);
        let event_index = self.observation.events.len();

        let mut anima_cost = self.reduce_ronghui_rear_move_anima_cost(
            actor_side,
            &drawn.card,
            super::support::effective_anima_cost(
                &drawn.card,
                self.actor(actor_side),
                Some(drawn.source_slot),
            ),
        );
        self.clear_next_card_anima_cost_reduction(actor_side);
        // CardActionBase.CheckAnima（build 24646245）：fate 412（慕虎专属，
        // FateStrategyConfig otherParams[0]=3）把名字含「木灵」的牌的灵气
        // 消耗改为支付等量生命（isCost），与 resonance 43 同款分支
        // （CardActionBase.cs:5053-5058）。转换在冥想之后、剑气之前，
        // 转换后 num=0 不再触发剑气/星力/灵气不足。
        let mut wood_spirit_hp_cost = 0;
        if anima_cost > 0
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&412)
            && drawn.card.name.contains("木灵")
        {
            wood_spirit_hp_cost = anima_cost * 3;
            anima_cost = 0;
        }
        let anima_before = self.actor(actor_side).core.anima;
        let anima_deficit = (anima_cost - anima_before).max(0);
        // 原版 CheckAnima（CardActionBase.cs:5059-5160）：任一替代支付路径
        // （talent 153 灵涌不竭 / 冥想 / 剑气）成功覆盖缺口后，剩余灵气消耗
        // 被固定为检查时刻的灵气量（num = -battleTempData.anima）。此后结算
        // 途中新增的灵气（如灵龟迷踪步在 talent-153 生命支付触发时 +1 灵气）
        // 不再被本卡消耗吃掉。oracle 锚点：hf-32299000 3b1d40886e1b4004/
        // round-13 cp[5]（玄心斩魄 0 灵气 + talent 153 支付 → 原版 1 / 旧引擎 0）、
        // 892e465f16556a9e/round-15 cp[6]。
        let mut remaining_anima_cost = anima_cost;
        if anima_deficit > 0
            && self.try_pay_ling_yong_bu_jue_anima_shortage(actor_side, anima_deficit)
        {
            remaining_anima_cost = anima_before;
        } else if anima_deficit > 0
            && self.try_pay_meditation_anima_shortage(actor_side, anima_deficit)
        {
            remaining_anima_cost = anima_before;
        } else if anima_deficit > 0 && self.actor(actor_side).sword.sword_energy >= anima_deficit {
            self.actor_mut(actor_side).sword.sword_energy -= anima_deficit;
            remaining_anima_cost = anima_before;
        } else if anima_deficit > 0
            && self.actor(actor_side).astrology.zi_mang_xing_bao > 0
            && self.actor(actor_side).astrology.star_power > 0
        {
            // CardActionBase.CheckAnima:5127-5142（build 24646245）— 卡 422
            // 紫芒星爆（buff 773）在灵气不足时用星力代替：扣 min(缺口, 星力)
            // 层星力（走 modify_star_power 共享结算 → 星力流失转等量加攻），
            // 剩余缺口仍按原版 AnimaShortage 拒绝出牌。触发点在剑气（JianQi）
            // 之后、AnimaShortage 之前（synthetic batch-027 pair 3 witness：
            // 72 破空爪 0 灵气 + 星力 1 + buff 773 → 星力 1→0、加攻+1、出牌）。
            let covered = anima_deficit.min(self.actor(actor_side).astrology.star_power);
            self.modify_star_power(actor_side, -covered);
            remaining_anima_cost -= covered;
        } else if anima_before < anima_cost {
            self.apply_anima_shortage_fallback(actor_side, &drawn.card);
            self.actor_mut(actor_side)
                .return_card_to_front(drawn.source_slot);
            return None;
        }
        // 紫芒星爆（buff 773）只要卡有灵气消耗且持有星力就把消耗转给星力，
        // 与灵气是否充足无关（原版 `num < 0` 即走星力支付，并非缺口分支）。
        // oracle 锚点：hf-32299000 1fdb74686d58c9b6/round-15 cp[11]-[12]
        // （五雷轰顶/星弈•断灵气不减、星力递减、星力流失转等量加攻）、
        // 30f844209b92f905/round-13 cp[8]-[9]。
        if remaining_anima_cost > 0
            && self.actor(actor_side).astrology.zi_mang_xing_bao > 0
            && self.actor(actor_side).astrology.star_power > 0
        {
            let star_paid = remaining_anima_cost.min(self.actor(actor_side).astrology.star_power);
            self.modify_star_power(actor_side, -star_paid);
            remaining_anima_cost -= star_paid;
        }
        if remaining_anima_cost > self.actor(actor_side).core.anima {
            self.apply_anima_shortage_fallback(actor_side, &drawn.card);
            self.actor_mut(actor_side)
                .return_card_to_front(drawn.source_slot);
            return None;
        }
        let anima_payment = remaining_anima_cost.min(self.actor(actor_side).core.anima);
        if anima_payment > 0 {
            self.spend_anima_unchecked(actor_side, anima_payment);
        }

        let hp_cost = super::support::effective_hp_cost(&drawn.card, self.actor(actor_side))
            + wood_spirit_hp_cost;
        let printed_hp_cost = drawn.card.hp_cost.unwrap_or(0).max(0);
        if hp_cost > 0 {
            self.pay_card_hp_cost(actor_side, hp_cost);
        }
        self.apply_after_hp_cost_hooks(
            actor_side,
            &drawn.card,
            printed_hp_cost,
            selected_card_is_beng_quan,
        );

        // BattleExecuter pays the selected physical card's cost before
        // CardActionBase.Execute. The verified transforms run in source order:
        // 复刻 -> 连音曲 -> 虚魂犬 -> 五帝 -> 相生 -> 仙蛋黄粽 -> 画龙 -> 化生壶.
        drawn = self.apply_synthetic_full_scope_replica_transform(actor_side, drawn);
        drawn = self.apply_ronghui_free_and_easy_tune_transform(actor_side, drawn);
        drawn = self.apply_you_ming_xu_hun_quan_replacement(actor_side, drawn);
        drawn = self.apply_upgrade_next_frenzy_sword(actor_side, drawn);
        drawn = self.apply_ronghui_five_emperors_upgrade_transform(actor_side, drawn);
        drawn = self.apply_generating_interaction_upgrade(actor_side, drawn);
        drawn = self.apply_immortal_egg_yolk_zongzi_upgrade(actor_side, drawn);
        let pre_paint_card_id = drawn.card.id;
        drawn = self.apply_paint_finishing_touch_upgrade(actor_side, drawn);
        if drawn.card.id != pre_paint_card_id {
            self.record_detail_step(
                event_index,
                super::ReplayHookCategory::TemporaryUpgrade,
                actor_side,
                Some(drawn.source_slot),
                Some(&drawn.card),
            );
        }
        drawn = self.apply_ronghui_alchemy_pot_transform(actor_side, drawn);
        let transformed_was_used = self
            .actor(actor_side)
            .deck
            .slots
            .get(drawn.source_slot)
            .is_some_and(|slot_state| slot_state.used);
        self.require_card_effect_before_execution(
            actor_side,
            &drawn.card,
            drawn.source_slot,
            transformed_was_used,
            true,
        )?;
        self.consume_dream_mirage_next_card_exhaust(actor_side, drawn.source_slot, false);

        let was_slot_used = self
            .actor(actor_side)
            .deck
            .slots
            .get(drawn.source_slot)
            .is_some_and(|slot| slot.used);
        self.record_detail_step(
            event_index,
            super::ReplayHookCategory::SelectCost,
            actor_side,
            Some(drawn.source_slot),
            Some(&drawn.card),
        );
        Some(PreparedCardTransaction {
            drawn,
            physical_card,
            public_adjacent_beng_quan,
            was_slot_used,
            event_index,
        })
    }

    fn preflight_card_transaction(&mut self, actor_side: PlayerSide) -> bool {
        let Some((card, source_slot, was_used_before_effect)) =
            self.preview_selected_card_for_resolution(actor_side)
        else {
            return true;
        };
        self.require_card_effect_before_execution(
            actor_side,
            &card,
            source_slot,
            was_used_before_effect,
            true,
        )
        .is_some()
    }

    fn finish_card_transaction(
        &mut self,
        actor_side: PlayerSide,
        transaction: CompletedCardTransaction,
    ) -> bool {
        let CompletedCardTransaction {
            drawn,
            was_slot_used,
            primary_card_action_again,
            event_index: card_completed_event_index,
        } = transaction;
        let card_type = drawn
            .card
            .card_type
            .as_ref()
            .map_or(0, |card_type| card_type.value);
        let marked_skipped_by_effect = self
            .actor(actor_side)
            .deck
            .slots
            .get(drawn.source_slot)
            .is_some_and(|slot| slot.skipped);
        let should_skip = !drawn.fallback_basic_attack
            && (card_type == super::CARD_TYPE_CONSUME || card_type == super::CARD_TYPE_SUSTAIN);
        let should_skip = should_skip || marked_skipped_by_effect;
        let jump_distance = self.actor(actor_side).turn.jump_to_previous_card.max(0);
        if jump_distance > 0 {
            self.actor_mut(actor_side).complete_drawn_card_with_jump(
                &drawn,
                should_skip,
                jump_distance,
            );
            self.actor_mut(actor_side).turn.jump_to_previous_card = 0;
        } else {
            self.actor_mut(actor_side)
                .complete_drawn_card(&drawn, should_skip);
        }
        self.completed_checkpoint_count += 1;
        self.record_detail_step(
            card_completed_event_index,
            super::ReplayHookCategory::AfterCard,
            actor_side,
            Some(drawn.source_slot),
            Some(&drawn.card),
        );
        self.record_event(
            super::ReplayEventKind::CardCompleted,
            actor_side,
            Some(drawn.source_slot),
            Some(&drawn.card),
        );
        if replay_trace_enabled() {
            eprintln!(
                "turn={} actor={:?} slot={} card={}({}) hp={}/{} delta={} anima={}/{} def={}/{} lostDef={}/{} sword={}/{} hex={}/{} star={}/{} internal={}/{} external={}/{} weak={}/{} flaw={}/{} atkBonus={}/{} drunken={}/{} quan={}/{} gun={}/{} yinFu={}/{}",
                self.actor_turn,
                actor_side,
                drawn.source_slot,
                drawn.card.name,
                super::support::normalized_base_id(&drawn.card),
                self.p1.core.hp,
                self.p2.core.hp,
                self.p1.core.hp - self.p2.core.hp,
                self.p1.core.anima,
                self.p2.core.anima,
                self.p1.core.defense,
                self.p2.core.defense,
                self.p1.turn.lost_defense_count,
                self.p2.turn.lost_defense_count,
                self.p1.sword.sword_intent,
                self.p2.sword.sword_intent,
                self.p1.astrology.hexagram,
                self.p2.astrology.hexagram,
                self.p1.astrology.star_power,
                self.p2.astrology.star_power,
                self.p1.status.internal_injury,
                self.p2.status.internal_injury,
                self.p1.status.external_injury,
                self.p2.status.external_injury,
                self.p1.status.weakness,
                self.p2.status.weakness,
                self.p1.status.flaw,
                self.p2.status.flaw,
                self.p1.core.attack_bonus,
                self.p2.core.attack_bonus,
                self.p1.status.drunken_fist_stance,
                self.p2.status.drunken_fist_stance,
                self.p1.beng.quan_stance,
                self.p2.beng.quan_stance,
                self.p1.beng.gun_stance,
                self.p2.beng.gun_stance,
                self.p1.status.yin_fu,
                self.p2.status.yin_fu,
            );
        }

        self.clear_rear_move_check(actor_side);

        if self.death_winner().is_some() {
            self.termination_cause = Some(super::ReplayTerminationCause::CardLethal);
            return false;
        }

        self.attribution_block = Some(super::TraceAttributionBlock::ActionAgain);
        let continue_acting = self.consume_action_again(
            actor_side,
            &drawn.card,
            drawn.source_slot,
            was_slot_used,
            false,
            primary_card_action_again,
        );
        self.attribution_block = None;
        if self.death_winner().is_some() {
            self.termination_cause = Some(super::ReplayTerminationCause::ActionAgainLethal);
        }
        self.record_detail_step(
            card_completed_event_index,
            super::ReplayHookCategory::ActionAgain,
            actor_side,
            Some(drawn.source_slot),
            Some(&drawn.card),
        );
        continue_acting
    }

    /// Select the physical card on detached state so catalog admission occurs
    /// before rear-move reset, queue movement, opening hooks, or cost payment.
    /// The real transaction repeats selection once and is the only execution
    /// allowed to advance real decisions/RNG.
    fn preview_selected_card_for_resolution(
        &mut self,
        actor_side: PlayerSide,
    ) -> Option<(CardDefinition, usize, bool)> {
        // Draw is &mut, so the peek must run on a detached copy that never
        // advances the real decisions/RNG. The only fields that grow with the
        // number of executed turns are the append-only observation/decision
        // logs; lift them out before cloning so this per-card clone stays
        // bounded by battle state, not O(events). Force observation off on the
        // detached preview so the no-read/no-append invariant is executable.
        let events = std::mem::take(&mut self.observation.events);
        let detailed_events = std::mem::take(&mut self.observation.detailed_events);
        let detailed_steps = std::mem::take(&mut self.observation.detailed_steps);
        let mutation_receipts = std::mem::take(&mut self.observation.mutation_receipts);
        let decision_events = std::mem::take(&mut self.decision_events);
        let mut preview = self.clone();
        self.observation.events = events;
        self.observation.detailed_events = detailed_events;
        self.observation.detailed_steps = detailed_steps;
        self.observation.mutation_receipts = mutation_receipts;
        self.decision_events = decision_events;
        preview.observation.mode = super::ReplayObservationMode::None;
        preview.effect_invocation_stack.clear();
        preview.actor_mut(actor_side).fate.rear_move_succeeded = false;
        let skip_limit = preview.nameless_white_deer_skip_limit();
        let mut drawn = preview.actor_mut(actor_side).draw_next_card(skip_limit)?;
        while preview.should_skip_card_with_instant_shadow_strike(actor_side, &drawn) {
            preview.trigger_skipped_opening_effects(actor_side, &drawn);
            preview.apply_instant_shadow_strike_skip(actor_side, &drawn);
            drawn = preview.actor_mut(actor_side).draw_next_card(skip_limit)?;
        }
        preview.trigger_skipped_opening_effects(actor_side, &drawn);
        let was_used_before_effect = preview
            .actor(actor_side)
            .deck
            .slots
            .get(drawn.source_slot)
            .is_some_and(|slot_state| slot_state.used);
        Some((drawn.card, drawn.source_slot, was_used_before_effect))
    }

    /// Evaluates one external repeat source in original order, applying its
    /// consumption side effect, and reports whether it fires an extra
    /// ExecuteEffect repetition for this card play.
    fn consume_repeat_source(
        &mut self,
        source: RepeatSource,
        actor_side: PlayerSide,
        card: &CardDefinition,
        public_adjacent_beng_quan: bool,
    ) -> bool {
        match source {
            RepeatSource::PlumBlossom => {
                if self.actor(actor_side).fate.plum_blossom_twice > 0 {
                    self.actor_mut(actor_side).fate.plum_blossom_twice -= 1;
                    true
                } else {
                    false
                }
            }
            RepeatSource::SecretSwordDoubleDragon => {
                self.consume_secret_sword_double_dragon_repetition(actor_side, card) > 0
            }
            RepeatSource::LeiShanErDu => {
                // CardActionBase.cs:1506-1519（build 24610558）— 卡 407 雷闪
                // 二度（buff 763 LeiShanErDu）：下一张名字含「雷」的牌在
                // 主效果之前先完整 ExecuteEffect 一次（消耗 1 层）。
                if self.actor(actor_side).astrology.lei_shan_er_du > 0 && card.name.contains('雷')
                {
                    self.actor_mut(actor_side).astrology.lei_shan_er_du -= 1;
                    true
                } else {
                    false
                }
            }
            RepeatSource::BengQuanDoubleShadow => {
                if self.actor(actor_side).beng.beng_quan_double_shadow > 0
                    && (public_adjacent_beng_quan
                        || self.is_dream_mirage_intrinsic_beng_quan(actor_side, card))
                {
                    // 崩拳·连崩 (10_000_035) keeps the double-shadow charge.
                    if super::support::normalized_base_id(card) != 10_000_035 {
                        self.actor_mut(actor_side).beng.beng_quan_double_shadow -= 1;
                    }
                    true
                } else {
                    false
                }
            }
            RepeatSource::DreamMirageFireEarth => {
                self.consume_dream_mirage_repeat_fire_or_earth(actor_side, card)
            }
        }
    }

    fn execute_selected_card_effect_repetition(
        &mut self,
        actor_side: PlayerSide,
        ctx: &EffectRepetitionContext,
        effect_card: &CardDefinition,
        repetition: i64,
        invocation_kind: EffectInvocationKind,
    ) -> bool {
        // ExecuteEffect marks CardItem.hadUsed at its tail, so every later
        // source branch observes true even though this is one outer card.
        let was_used_for_effect = ctx.was_slot_used || repetition > 0;
        if self
            .require_card_effect_before_execution(
                actor_side,
                effect_card,
                ctx.source_slot,
                was_used_for_effect,
                invocation_kind == EffectInvocationKind::Played,
            )
            .is_none()
        {
            return false;
        }
        let public_beng_for_effect = repetition == 0 && ctx.public_adjacent_beng_quan;
        // ExecuteEffect snapshots 连云 independently at each entry. A prior
        // effect's OnAfterExecuted may establish it for this effect.
        let cloud_chain_before_effect = has_cloud_chain(self.actor(actor_side));
        let effective_is_beng_quan = public_beng_for_effect
            || self.is_dream_mirage_intrinsic_beng_quan(actor_side, effect_card);
        let mut card_action_again = false;
        for phase in CARD_EFFECT_PHASES {
            match phase {
                CardEffectPhase::OpenInvocation => {
                    self.begin_effect_invocation(
                        actor_side,
                        ctx.origin_card,
                        effect_card,
                        ctx.physical_card,
                        ctx.source_slot,
                        ctx.source_slot,
                        invocation_kind,
                        effective_is_beng_quan,
                    );
                }
                CardEffectPhase::PlayCardEntry => {
                    self.apply_before_execute_effect_hooks(
                        actor_side,
                        effect_card,
                        ctx.source_slot,
                        false,
                    );
                    self.set_active_effect_phase(EffectInvocationPhase::Body);
                }
                CardEffectPhase::PreBodyHooks => {
                    self.apply_dream_mirage_before_effect_hooks(actor_side, effect_card);
                    // Set the current card's event index so every attack segment from this
                    // execution -- both the printed body and shared AfterCard follow-ups --
                    // joins onto the matching MainEffect step. Reset the per-card segment
                    // counter so段号从 0 连续编号。
                    self.observation.current_card_event_index =
                        Some(ctx.card_completed_event_index);
                    self.observation.current_attack_segment_index = 0;
                }
                CardEffectPhase::Body => {
                    self.apply_card_effect_body(
                        actor_side,
                        effect_card,
                        ctx.source_slot,
                        was_used_for_effect,
                    );
                }
                CardEffectPhase::ActionAgain => {
                    card_action_again = self.resolve_card_action_again(
                        actor_side,
                        effect_card,
                        ctx.source_slot,
                        was_used_for_effect,
                        cloud_chain_before_effect,
                    );
                    self.record_detail_step(
                        ctx.card_completed_event_index,
                        super::ReplayHookCategory::MainEffect,
                        actor_side,
                        Some(ctx.source_slot),
                        Some(effect_card),
                    );
                }
                CardEffectPhase::AfterCardHooks => {
                    self.apply_regular_after_card_effect_hooks(
                        actor_side,
                        effect_card,
                        ctx.source_slot,
                        false,
                    );
                    self.set_active_effect_phase(EffectInvocationPhase::Settlement);
                    self.complete_card_effect_repetition(actor_side, effect_card, ctx.source_slot);
                }
                CardEffectPhase::CloseInvocation => {
                    self.observation.current_card_event_index = None;
                    self.set_active_effect_after_action(false);
                    self.clear_rear_move_check(actor_side);
                    self.end_effect_invocation(actor_side, invocation_kind);
                }
            }
        }
        card_action_again
    }

    fn complete_card_effect_repetition(
        &mut self,
        actor_side: PlayerSide,
        card: &CardDefinition,
        slot: usize,
    ) {
        self.apply_card_completed_hooks(actor_side, card, slot);
        self.settle_wan_shi_ru_yi_card_19(actor_side, card);
        if card.hp_cost.unwrap_or(0) > 0 {
            self.actor_mut(actor_side).turn.hp_cost_cards_used += 1;
        }
        self.settle_sword_intent_after_card_effect(actor_side);
        if let Some(slot_state) = self.actor_mut(actor_side).deck.slots.get_mut(slot) {
            slot_state.used = true;
        }
        self.record_last_element(actor_side, card);
        if card
            .card_type
            .as_ref()
            .map_or(0, |card_type| card_type.value)
            == super::CARD_TYPE_SUSTAIN
        {
            self.actor_mut(actor_side)
                .formations
                .array_echo_persistent_card += 1;
        }
        let used_before = self.actor(actor_side).turn.used_card_count;
        self.actor_mut(actor_side).turn.used_card_count += 1;
        let used_after = self.actor(actor_side).turn.used_card_count;
        self.record_counter_transition(
            actor_side,
            "回合",
            "usedCardCount",
            "已用牌数",
            used_before,
            used_after,
        );
    }

    fn pay_card_hp_cost(&mut self, actor_side: PlayerSide, hp_cost: i64) {
        if self.actor(actor_side).identity.talents.contains(&174) {
            self.modify_actor_max_hp(actor_side, -hp_cost);
            return;
        }
        self.modify_actor_hp(actor_side, -hp_cost, true, true);
    }

    fn trigger_skipped_opening_effects(
        &mut self,
        actor_side: PlayerSide,
        drawn: &super::DrawnCard,
    ) {
        for &skipped_slot in &drawn.skipped_opening_slots {
            let Some(card) = self
                .actor(actor_side)
                .deck
                .slots
                .get(skipped_slot)
                .map(|slot| slot.card.clone())
            else {
                continue;
            };
            let base_id = super::support::normalized_base_id(&card);
            if !Self::card_has_opening_effect(base_id) || base_id == 56 {
                continue;
            }
            self.apply_opening_effect_for_card(actor_side, &card, skipped_slot);
        }
    }

    fn apply_generating_interaction_upgrade(
        &mut self,
        actor_side: PlayerSide,
        drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).fate.generating_interaction_upgrade <= 0 {
            return drawn;
        }
        let Some(current_element) = super::support::element_from_card(&drawn.card) else {
            return drawn;
        };
        let Some(previous_element) = self.actor(actor_side).elements.last_element else {
            return drawn;
        };
        if !super::support::is_element_generated_by(
            previous_element,
            current_element,
            self.actor(actor_side).identity.talents.contains(&137),
        ) {
            return drawn;
        }
        let upgraded_id = drawn.card.id + 10_000;
        let Some(upgraded) = super::original_config::original_card_definition(upgraded_id) else {
            return drawn;
        };
        self.actor_mut(actor_side)
            .fate
            .generating_interaction_upgrade -= 1;
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = upgraded.clone();
        }
        super::DrawnCard {
            source_slot: drawn.source_slot,
            card: upgraded,
            fallback_basic_attack: drawn.fallback_basic_attack,
            skipped_slots: drawn.skipped_slots,
            skipped_opening_slots: drawn.skipped_opening_slots,
            fate_398_skipped_fifth_grid: drawn.fate_398_skipped_fifth_grid,
        }
    }

    fn apply_paint_finishing_touch_upgrade(
        &mut self,
        actor_side: PlayerSide,
        drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).fate.paint_finishing_touch <= 0 {
            return drawn;
        }
        // CardActionBase.cs:877 gates 画龙点睛 with CardConfig.CanUpgrade().
        // 澄心剑胚 (19) is explicitly noUpgrade even though 10019 exists for
        // other battle-time paths.
        if !super::original_config::can_upgrade_original_battle_deck_card(drawn.card.id) {
            return drawn;
        }
        let upgraded_id = drawn.card.id + 10_000;
        let Some(upgraded) = super::original_config::original_card_definition(upgraded_id) else {
            return drawn;
        };
        self.modify_paint_finishing_touch(actor_side, -1);
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = upgraded.clone();
        }
        super::DrawnCard {
            source_slot: drawn.source_slot,
            card: upgraded,
            fallback_basic_attack: drawn.fallback_basic_attack,
            skipped_slots: drawn.skipped_slots,
            skipped_opening_slots: drawn.skipped_opening_slots,
            fate_398_skipped_fifth_grid: drawn.fate_398_skipped_fifth_grid,
        }
    }

    /// CardActionBase.cs:1187-1193；守卫紧跟 1168 的幽冥虚魂圈，同属 Execute 的
    /// 同一段状态机。消耗一层 ShengJiXiaCiKuangJian（671），扣固定生命后把当前
    /// 这张 rarity=0 的狂剑升一阶；原版 IL_18d4 会回写 cardActionBase.cardConfig，
    /// 所以升阶对**本次出牌**立即生效。
    fn apply_upgrade_next_frenzy_sword(
        &mut self,
        actor_side: PlayerSide,
        drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).sword.upgrade_next_frenzy_sword <= 0
            || drawn.card.rarity.unwrap_or(0) != 0
            || !super::original_config::can_upgrade_original_battle_deck_card(drawn.card.id)
            || !super::support::is_frenzy_sword(self.actor(actor_side), &drawn.card)
        {
            return drawn;
        }
        // 原版固定读 1030076 的 otherParams[2] 当生命代价，与牌组里实际是哪一阶无关。
        let hp_cost = super::original_config::original_card_definition(1_030_076)
            .map(|config| super::support::other_param(&config, 2).max(0))
            .unwrap_or(0);
        if self.actor(actor_side).core.hp <= hp_cost {
            return drawn;
        }
        let Some(upgraded) =
            super::original_config::original_card_definition(drawn.card.id + 10_000)
        else {
            return drawn;
        };
        self.actor_mut(actor_side).sword.upgrade_next_frenzy_sword -= 1;
        self.modify_actor_hp(actor_side, -hp_cost, false, false);
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = upgraded.clone();
        }
        super::DrawnCard {
            source_slot: drawn.source_slot,
            card: upgraded,
            fallback_basic_attack: drawn.fallback_basic_attack,
            skipped_slots: drawn.skipped_slots,
            skipped_opening_slots: drawn.skipped_opening_slots,
            fate_398_skipped_fifth_grid: drawn.fate_398_skipped_fifth_grid,
        }
    }

    /// CardActionBase.cs:1588-1605,1638-1660; immediately after 相生相成.
    fn apply_immortal_egg_yolk_zongzi_upgrade(
        &mut self,
        actor_side: PlayerSide,
        mut drawn: super::DrawnCard,
    ) -> super::DrawnCard {
        if self.actor(actor_side).hp_mutation.immortal_egg_yolk_zongzi <= 0 {
            return drawn;
        }
        self.actor_mut(actor_side)
            .hp_mutation
            .immortal_egg_yolk_zongzi -= 1;
        let can_upgrade =
            super::original_config::can_upgrade_original_battle_deck_card(drawn.card.id);
        let upgraded = can_upgrade
            .then(|| super::original_config::original_card_definition(drawn.card.id + 10_000))
            .flatten();
        let Some(upgraded) = upgraded else {
            self.actor_mut(actor_side).hp_mutation.appetite += 2;
            return drawn;
        };
        if let Some(slot) = self
            .actor_mut(actor_side)
            .deck
            .slots
            .get_mut(drawn.source_slot)
        {
            slot.card = upgraded.clone();
        }
        drawn.card = upgraded;
        drawn
    }

    fn nameless_white_deer_skip_limit(&self) -> i64 {
        self.p1
            .deck
            .slots
            .iter()
            .chain(self.p2.deck.slots.iter())
            .filter(|slot| super::support::normalized_base_id(&slot.card) == 99_000_214)
            .map(|slot| super::support::other_param(&slot.card, 0).max(0))
            .max()
            .unwrap_or(0)
    }

    fn apply_turn_start_chance_hooks(&mut self, actor_side: PlayerSide) {
        let opponent = opponent_side(actor_side);
        let spirit_drain = self.actor(actor_side).chance.shi_xu_ling_shou.max(0);
        if spirit_drain > 0 && self.actor(opponent).core.anima > 0 {
            let drain = spirit_drain.min(self.actor(opponent).core.anima);
            self.reduce_anima_unchecked(opponent, drain);
            self.gain_anima(actor_side, drain);
        }
        let sky_eagle = self.actor(actor_side).chance.po_kong_diao.max(0);
        if sky_eagle > 0 {
            self.apply_damage(actor_side, sky_eagle, false, false, false);
        }
        let chubby_tanuki = self.actor(actor_side).chance.pang_xian_li.max(0);
        if chubby_tanuki > 0 {
            self.modify_actor_hp(actor_side, chubby_tanuki, false, false);
        }
        let red_eye = self.actor(actor_side).chance.tun_tian_chi_yan_shou.max(0);
        if red_eye > 0 {
            self.modify_actor_hp(opponent, -red_eye, false, false);
            self.modify_actor_hp(actor_side, red_eye, false, false);
        }
        let earth_turtle = self.actor(actor_side).chance.di_xuan_gui.max(0);
        if earth_turtle > 0 {
            self.gain_defense(actor_side, earth_turtle);
        }
    }

    pub(super) fn clear_temporary_guard_at_turn_start(&mut self, actor_side: PlayerSide) {
        let temporary_guard = self.actor(actor_side).core.temporary_guard.max(0);
        if temporary_guard <= 0 {
            return;
        }
        self.modify_guard(actor_side, -temporary_guard);
        self.actor_mut(actor_side).core.temporary_guard = 0;
    }
}
