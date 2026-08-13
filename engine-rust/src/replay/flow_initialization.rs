use super::support::innate_mark_elements;
use super::*;
use crate::fixture::{BattleFixture, FixturePlayer};
use std::result::Result;

impl ReplayState {
    /// Test entry point: build the full battle state with observation off.
    #[cfg(test)]
    pub(super) fn from_fixture(fixture: &BattleFixture, strict: bool) -> Result<Self, BattleError> {
        Self::from_fixture_with_mode(fixture, strict, super::ReplayObservationMode::None)
    }

    /// Builds the pre-opening state and runs the battle-start phase.
    ///
    /// Detailed observation must be active before the opening runs so opening
    /// mutations participate in the same receipt contract as turn/card
    /// mutations; every other mode keeps the established construction path
    /// (observation starts after the opening, so the opening produces no
    /// events/receipts for them).
    pub(super) fn from_fixture_with_mode(
        fixture: &BattleFixture,
        strict: bool,
        mode: super::ReplayObservationMode,
    ) -> Result<Self, BattleError> {
        let mut state = Self::build_pre_opening_state(fixture, strict)?;
        if mode.is_detailed() {
            state.observation.mode = mode;
        }
        state.apply_battle_start_phase(fixture)?;
        state.completed_checkpoint_count += 1;
        if strict {
            if let Some(error) = state.evaluation_error.as_ref() {
                return Err(error.clone());
            }
        }
        Ok(state)
    }

    /// Builds the pre-opening state without running the battle-start phase.
    /// Used by `trace_replay_fixture_hooks` to snapshot a pre-opening baseline
    /// so the `BattleStart` hook step can report what the opening changed
    /// instead of folding it into an empty baseline.
    pub(super) fn from_fixture_pre_opening(
        fixture: &BattleFixture,
        strict: bool,
    ) -> Result<Self, BattleError> {
        Self::build_pre_opening_state(fixture, strict)
    }

    fn build_pre_opening_state(fixture: &BattleFixture, strict: bool) -> Result<Self, BattleError> {
        let original_build_profile = super::original_build_profile::resolve_original_build_profile(
            fixture
                .source
                .as_ref()
                .and_then(|source| source.steam_build.as_deref()),
        )
        .map_err(|message| BattleError::UnsupportedBuild { message, turn: 0 })?;
        let p1 = super::ReplayPlayer::from_fixture(
            PlayerSide::P1,
            &fixture.players.p1,
            &fixture.historical_card_overrides,
        );
        let p2 = super::ReplayPlayer::from_fixture(
            PlayerSide::P2,
            &fixture.players.p2,
            &fixture.historical_card_overrides,
        );
        let state = Self {
            p1,
            p2,
            first_player: fixture.first_player_side,
            current_actor: fixture.first_player_side,
            original_build_profile,
            actor_turn: 0,
            max_actor_turns: fixture
                .max_actor_turns
                .unwrap_or(super::DEFAULT_MAX_ACTOR_TURNS),
            decision_tape: fixture.decision_tape.clone(),
            random_fallback_tape: fixture.random_fallback_tape.clone(),
            synthetic_decision_seed: fixture
                .source
                .as_ref()
                .and_then(|source| source.synthetic_decision_seed),
            synthetic_decision_sides: fixture
                .source
                .as_ref()
                .map(|source| source.synthetic_decision_sides.clone())
                .unwrap_or_default(),
            synthetic_decision_fallback_seed: fixture
                .source
                .as_ref()
                .and_then(|source| source.synthetic_decision_fallback_seed),
            decision_occurrence: 0,
            card_execution_occurrence: 0,
            current_card_execution: None,
            decision_events: Vec::new(),
            effect_invocation_stack: Vec::new(),
            attribution_block: None,
            fail_on_missing_decision: strict,
            evaluation_error: None,
            observation: super::ReplayObservationRuntime::default(),
            termination_cause: None,
            completed_checkpoint_count: 0,
        };
        Ok(state)
    }

    /// Runs the complete battle-start phase: actor-owned opening effects
    /// (including meditation healing, talents, fate, protective talisman, 金梭兰,
    /// innate marks and opening cards), then solver starting perturbations.
    ///
    /// Kept separate from `from_fixture`'s state construction so a caller can
    /// snapshot the pre-opening baseline before these effects mutate state —
    /// that lets the `BattleStart` hook step report what the opening changed
    /// instead of silently folding it into an empty baseline.
    pub(super) fn apply_battle_start_phase(
        &mut self,
        fixture: &BattleFixture,
    ) -> Result<(), BattleError> {
        self.attribution_block = Some(super::TraceAttributionBlock::BattleStart);
        self.apply_battle_start_opening_effects(fixture);
        self.apply_battle_start_phase_perturbations(fixture)?;
        self.attribution_block = None;
        Ok(())
    }

    fn apply_battle_start_phase_perturbations(
        &mut self,
        fixture: &BattleFixture,
    ) -> Result<(), BattleError> {
        if let Some(source) = &fixture.source {
            for perturbation in &source.solver_starting_perturbations {
                self.apply_solver_starting_perturbation(
                    perturbation.side,
                    &perturbation.field,
                    perturbation.amount,
                )?;
            }
        }
        Ok(())
    }

    fn apply_solver_starting_perturbation(
        &mut self,
        side: PlayerSide,
        field: &str,
        amount: i64,
    ) -> Result<(), BattleError> {
        if amount == 0 {
            return Ok(());
        }
        match field {
            "hp" => {
                if amount > 0 {
                    self.modify_actor_max_hp(side, amount);
                }
                self.modify_actor_hp(side, amount, false, false);
            }
            "maxHp" => {
                self.modify_actor_max_hp(side, amount);
            }
            "defense" if amount > 0 => {
                self.gain_defense(side, amount);
            }
            "defense" => {
                self.lose_defense(side, -amount);
            }
            "guard" => {
                self.modify_guard(side, amount);
            }
            "anima" if amount > 0 => self.gain_anima(side, amount),
            "anima" => self.reduce_anima_unchecked(side, -amount),
            "momentum" => {
                self.modify_momentum(side, amount);
            }
            "agility" => {
                self.modify_agility(side, amount);
            }
            "swordIntent" => {
                self.modify_sword_intent(side, amount);
            }
            "sharpness" if amount > 0 => self.gain_sharpness(side, amount),
            "sharpness" => {
                let actor = self.actor_mut(side);
                actor.sword.sharpness = (actor.sword.sharpness + amount).max(0);
            }
            "attackBonus" => {
                self.gain_attack_bonus(side, amount);
            }
            "physique" => {
                self.modify_physique_amount(side, amount);
            }
            "internalInjury" => {
                self.modify_actor_negative_status(side, 100, amount);
            }
            "weakness" => {
                self.modify_actor_negative_status(side, 101, amount);
            }
            "flaw" => {
                self.modify_actor_negative_status(side, 102, amount);
            }
            "attackReduction" => {
                self.modify_actor_negative_status(side, 103, amount);
            }
            "entangle" => {
                self.modify_actor_negative_status(side, 104, amount);
            }
            "externalInjury" => {
                self.modify_actor_negative_status(side, 105, amount);
            }
            "hexagram" => {
                self.modify_hexagram(side, amount);
            }
            "starPower" => {
                self.modify_star_power(side, amount);
            }
            "cloudChain" if amount > 0 => self.gain_cloud_chain(side, amount),
            "cloudChain" => {
                let actor = self.actor_mut(side);
                actor.sword.cloud_chain = (actor.sword.cloud_chain + amount).max(0);
            }
            "waterMomentum" if amount > 0 => self.gain_water_momentum(side, amount),
            "waterMomentum" => {
                let actor = self.actor_mut(side);
                actor.elements.water_momentum = (actor.elements.water_momentum + amount).max(0);
            }
            "cloudSea" => {
                let actor = self.actor_mut(side);
                actor.sword.cloud_sea = (actor.sword.cloud_sea + amount).max(0);
            }
            "activatedMetal" if amount > 0 => self.activate_element(side, super::Element::Metal),
            "activatedWater" if amount > 0 => self.activate_element(side, super::Element::Water),
            "activatedWood" if amount > 0 => self.activate_element(side, super::Element::Wood),
            "activatedFire" if amount > 0 => self.activate_element(side, super::Element::Fire),
            "activatedEarth" if amount > 0 => self.activate_element(side, super::Element::Earth),
            field if field.starts_with("activated") => {
                return Err(BattleError::Invariant {
                    message: format!("activated element perturbation cannot be negative: {field}"),
                });
            }
            _ => {
                return Err(BattleError::Invariant {
                    message: format!("unsupported solver starting perturbation field: {field}"),
                });
            }
        }
        Ok(())
    }

    pub(super) fn apply_battle_start_meditation_healing(
        &mut self,
        player_fixture: &FixturePlayer,
        actor_side: PlayerSide,
    ) {
        // Talent 179（入冥）在 OnBattleStarted 天赋循环中执行；FateStrategy
        // 161 的冥 +2 属于 FateStrategyFunctions.OnBattleStart 的固定顺序链
        // （在 140/27 之后，BattleCharacter.cs 的 IL_088b 块），由
        // battle_start.rs 在 FS 27 采样之后单独调用
        // apply_battle_start_fate_strategy_161_meditation。
        // 原版 ModifyBuffValue(Min, +X) 的钩子顺序（BattleCharacter.cs:
        // 8711-8729）：先 415 疯魔架势 ModifyTiPo(abs(delta))，后 Min 分支
        // ModifyHp(abs(delta)*3)。先涨体魄/生命上限再回血，回血按更高上限
        // 截断（oracle 锚点：e170262525adf8c7/round-09 cp0 p2.hp 81 = 79+3
        // 被 81 截断；引擎原 80 = 先回 3 被 80 截断再涨上限）。
        if player_fixture.talents.contains(&179) {
            self.apply_feng_mo_stance_physique(actor_side, 1);
            // 冥副作用（BattleCharacter.cs:8715-8730）：4000003 角色回
            // abs(delta)*3；非 4000003 的扣血在 from_fixture 直接扣除
            // （保持既有装配语义），此处只做 4000003 的正向转换。
            if player_fixture.character_id == Some(4_000_003) {
                self.modify_actor_hp(actor_side, 3, false, false);
            }
        }
    }

    pub(super) fn apply_battle_start_fate_strategy_161_meditation(
        &mut self,
        player_fixture: &FixturePlayer,
        actor_side: PlayerSide,
    ) {
        // FateStrategy 161（天衍-入冥，叶冥冥专属）：FateStrategyFunctions
        // .OnBattleStart 的 IL_088b 块 —— 在 FS 140/27（IL_00f1/IL_01c1）
        // 之后执行 ModifyBuffValue(Min, otherParams[0])。顺序敏感：FS 27 的
        // hp×12/100 采样先于本冥 +2（oracle 锚点：9eed310c78cd1b40/round-09
        // cp0 p1 峰值 98：83×12/100=9 → 92 → +6 = 98；引擎原 89×12/100=10
        // → 99）。钩子顺序同 ModifyBuffValue：415 先 ModifyTiPo(2)。
        if !player_fixture.fate_strategies.contains(&161) {
            return;
        }
        self.apply_feng_mo_stance_physique(actor_side, 2);
        if player_fixture.character_id == Some(4_000_003) {
            self.modify_actor_hp(actor_side, 6, false, false);
        }
    }

    /// 回合结束水势伤害（BattleCharacter.cs OnTurnEnded IL_1a03-1cbe）：
    /// 每段走同一分支——KeYinShuiRen（刻印水刃 101）/FS 137（凝水化刃）
    /// 时 Attack(水势)（CalculateAttack ×1.5），否则 ApplyDamage(水势)。
    /// 波澜（幻•水灵波澜 484）的额外触发段走完全相同的路径，不能退化成
    /// 裸 ApplyDamage（漏 415 外的加攻与 ×1.5）。oracle 锚点：
    /// hf-latest-32308000-16f9c778 56f1c06b0530592f/round-14 cp7 p2.hp
    /// 102（原版 2×10 = 7 水势×1.5×2 段；引擎 105 = 主段 10 + 额外段 7）、
    /// 8f95021d967dff1e/round-13 cp17 p2.hp 50（原版 2×12 = (4+4加攻)×1.5
    /// ×2 段；引擎 45 = 主段 12 + 额外段 4）、e1eb5c51c3f179d9/round-16
    /// cp7（原版 2×10 vs 引擎 7+10）。
    pub(super) fn apply_turn_end_water_momentum_damage(
        &mut self,
        actor_side: PlayerSide,
        water_momentum: i64,
    ) {
        if water_momentum <= 0 {
            return;
        }
        if self
            .actor(actor_side)
            .identity
            .fate_strategies
            .contains(&137)
        {
            self.actor_mut(actor_side).elements.water_blade_seal += 1;
            self.apply_attack(actor_side, water_momentum, usize::MAX);
            self.actor_mut(actor_side).elements.water_blade_seal -= 1;
        } else {
            self.apply_damage(actor_side, water_momentum, false, false, false);
        }
    }

    pub(super) fn apply_innate_mark_element_activation(
        &mut self,
        player: &FixturePlayer,
        side: PlayerSide,
    ) {
        if !player.talents.contains(&110) {
            return;
        }
        for element in innate_mark_elements(&player.talents) {
            self.activate_element(side, element);
        }
    }
}
