//! Battle-end life settlement: exact port of the original client's
//! `BattleExecuter.CalLifeDamage` + `BattleCharacter.ShowDeathDamage`
//! (`research/original-game/extracted/current/decompiled/`).
//!
//! Inputs are battle-window facts (final HPs, end-of-battle tempLife and
//! CiNianQu stacks, pre-battle talents/strategies/tempDatas). Anything the
//! fixture cannot supply, or any mode whose rule lives outside these two
//! functions, returns `None` (fail closed) instead of guessing.

use std::collections::BTreeMap;

use crate::fixture::FinalLife;
use crate::model::PlayerSide;

const GAME_MODE_VERSUS: i64 = 13;
const GAME_MODE_ROGUE: i64 = 11;
const GAME_MODE_YUAN_GU: i64 = 16;
const GAME_MODE_FAST: i64 = 17;
const GAME_MODE_TA21: i64 = 121;
const GAME_MODE_TOWER: i64 = 14;

const BUFF_SHI_TAN_ZHI_JUAN: i64 = 10030;
const BUFF_TIAN_YAN_HUA_SHEN: i64 = 10053;
const BUFF_ZHAN_BAI_SHAO_KOU: i64 = 20040;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeSettleSideInputs<'a> {
    pub talents: &'a [i64],
    pub talent_temp_datas: &'a BTreeMap<String, i64>,
    pub fate_strategies: &'a [i64],
    pub xian_mo_strategies: &'a [i64],
    pub permanent_buff_temp_datas: &'a BTreeMap<String, i64>,
    pub is_mirror: bool,
    pub pre_battle_life: Option<i64>,
    pub talent_resonance_id: Option<i64>,
    pub resonance_talent_flags: &'a BTreeMap<String, i64>,
    pub battle_end_temp_life: i64,
    pub battle_end_hp: i64,
    pub battle_end_ci_nian_qu: i64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeSettleInputs<'a> {
    pub round: Option<i64>,
    pub game_mode: Option<i64>,
    pub winner_side: PlayerSide,
    pub p1: LifeSettleSideInputs<'a>,
    pub p2: LifeSettleSideInputs<'a>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct LifeSettleSideCtx {
    pub talents: Vec<i64>,
    pub talent_temp_datas: BTreeMap<String, i64>,
    pub fate_strategies: Vec<i64>,
    pub xian_mo_strategies: Vec<i64>,
    pub permanent_buff_temp_datas: BTreeMap<String, i64>,
    pub resonance_talent_flags: BTreeMap<String, i64>,
    pub is_mirror: bool,
    pub pre_battle_life: Option<i64>,
    pub talent_resonance_id: Option<i64>,
}

impl LifeSettleSideCtx {
    pub fn capture(player: &crate::fixture::FixturePlayer) -> Self {
        Self {
            talents: player.talents.clone(),
            talent_temp_datas: player.talent_temp_datas.clone(),
            fate_strategies: player.fate_strategies.clone(),
            xian_mo_strategies: player.xian_mo_strategies.clone(),
            permanent_buff_temp_datas: player.permanent_buff_temp_datas.clone(),
            resonance_talent_flags: player.resonance_talent_flags.clone(),
            is_mirror: player.is_mirror,
            pre_battle_life: player.last_round_life,
            talent_resonance_id: player.talent_resonance_id,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, schemars::JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LifeSettleDetail {
    pub base_damage: i64,
    /// Battle damage actually deducted (0 for mirror losers).
    pub actual_dealt: i64,
    pub survival_talent: Option<i64>,
    pub jiangsha_105: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LifeSettleOutcome {
    pub final_life: FinalLife,
    /// Recorded-convention damage, signed like hpDelta (positive when p1
    /// wins): post-mitigation dealt, except mirror losers report the base.
    pub dealt_signed: i64,
    pub detail: LifeSettleDetail,
}

fn temp_count(map: &BTreeMap<String, i64>, key: i64) -> i64 {
    map.get(&key.to_string()).copied().unwrap_or(0)
}

fn ceil_div_positive(num: i64, denom: i64) -> i64 {
    debug_assert!(num >= 0 && denom > 0);
    (num + denom - 1) / denom
}

/// Original `BattleExecuter.CalLifeDamage` without the mode arms.
fn cal_base_damage(round: i64, hp_gap: i64, fast: bool, fate_21: bool, xm_2011: bool) -> i64 {
    let num = if hp_gap <= 20 {
        round + 1 + ceil_div_positive(hp_gap, 5)
    } else {
        let extra = ceil_div_positive(hp_gap - 20, 10).min(2 + round / 2);
        round + 5 + extra
    };
    let num = if round <= 9 { num - 1 } else { num };
    let num = if fast { num * 3 / 2 } else { num };
    // FateStrategyConfig 21 otherParams[0] = 1 (not TalentConfig 21's 3).
    let num = if fate_21 { (num - 1).max(1) } else { num };
    if xm_2011 {
        (num - 1).max(1)
    } else {
        num
    }
}

fn resonance_effective_84(round: i64) -> Option<i64> {
    // 共鸣·历战不屈 (84): TalentResonanceConfig otherParams [2],
    // effectRound 6, no effectLevel gate.
    if round >= 6 {
        Some(2)
    } else {
        None
    }
}

/// Original `BattleCharacter.ShowDeathDamage`, state effects only. Returns
/// `(dealt, loser_end, survival_talent, jiangsha)` or `None` when an
/// unevaluable branch is present.
#[allow(clippy::too_many_arguments)]
fn apply_death_damage(
    base: i64,
    loser: &LifeSettleSideInputs<'_>,
    round: i64,
) -> Option<(i64, i64, Option<i64>, bool)> {
    if loser.is_mirror {
        // Application skipped, but the record carries the base (1045/1045).
        return Some((base, loser.battle_end_temp_life, None, false));
    }
    if temp_count(loser.permanent_buff_temp_datas, BUFF_SHI_TAN_ZHI_JUAN) > 0 {
        return None;
    }
    let mut damage = base;
    if loser.battle_end_ci_nian_qu > 0 && damage > 0 {
        damage = (damage - loser.battle_end_ci_nian_qu).max(1);
    }
    if loser.talent_resonance_id == Some(84) && damage > 0 {
        if let Some(reduction) = resonance_effective_84(round) {
            damage = (damage - reduction).max(1);
        }
    }
    if loser.talents.contains(&27) && damage > 20 {
        damage = 20;
    }
    if loser.talents.contains(&45) && damage > 0 && damage % 2 == 0 {
        damage /= 2;
    }
    let temp = loser.battle_end_temp_life;
    // Branch order mirrors the original else-if chain. ZhanBai's decompiled
    // branch never touches tempLife and has zero observations: arm it.
    if temp_count(loser.permanent_buff_temp_datas, BUFF_ZHAN_BAI_SHAO_KOU) > 0 {
        return None;
    }
    if let Some(selected) = loser.talent_resonance_id {
        // Resonances 27/106 carry no effectRound/effectLevel gate in
        // TalentResonanceConfig: selected + flag bit set means immune. A
        // missing flag key is unknown data (decoders may drop the map),
        // not a confirmed-unset flag: arm it.
        if selected == 27 || selected == 106 {
            let key = selected.to_string();
            if !loser.resonance_talent_flags.contains_key(&key) {
                return None;
            }
            let bit = if selected == 106 { 1 } else { 0 };
            if temp_count(loser.resonance_talent_flags, selected) & (1 << bit) != 0 {
                return Some((0, loser.battle_end_temp_life, None, false));
            }
        }
    }
    let has = |id: i64| {
        loser.talents.contains(&id)
            && loser.talent_temp_datas.contains_key(&id.to_string())
            && temp_count(loser.talent_temp_datas, id) > 0
    };
    let survival = [7, 65, 83, 154]
        .into_iter()
        .find(|id| has(*id) && damage >= temp);
    if survival.is_some()
        || (temp_count(loser.permanent_buff_temp_datas, BUFF_TIAN_YAN_HUA_SHEN) > 0
            && damage >= temp)
    {
        if temp <= 0 {
            return None;
        }
        let id = survival.or(Some(354));
        return Some((temp - 1, 1, id, false));
    }
    if loser.talents.contains(&105)
        && loser.talent_temp_datas.contains_key("105")
        && damage >= 15
        && temp_count(loser.talent_temp_datas, 105) > 0
    {
        return Some((0, loser.battle_end_temp_life, None, true));
    }
    Some((damage, loser.battle_end_temp_life - damage, None, false))
}

pub fn settle_battle_life(inputs: &LifeSettleInputs<'_>) -> Option<LifeSettleOutcome> {
    let round = inputs.round?;
    let game_mode = inputs.game_mode?;
    if game_mode == GAME_MODE_VERSUS
        || game_mode == GAME_MODE_TA21
        || game_mode == GAME_MODE_YUAN_GU
        || game_mode == GAME_MODE_TOWER
    {
        return None;
    }
    if inputs.p1.pre_battle_life.is_none() || inputs.p2.pre_battle_life.is_none() {
        return None;
    }
    // Resonance 84 without a visible selection is fine (no reduction); a
    // selected 84 below effectRound applies nothing per config.
    let (winner, loser) = match inputs.winner_side {
        PlayerSide::P1 => (&inputs.p1, &inputs.p2),
        PlayerSide::P2 => (&inputs.p2, &inputs.p1),
    };
    if loser.pre_battle_life.is_some_and(|life| life <= 0) {
        return None;
    }
    let hp_gap = winner.battle_end_hp - loser.battle_end_hp;
    if hp_gap < 0 {
        return None;
    }
    let base = cal_base_damage(
        round,
        hp_gap,
        game_mode == GAME_MODE_FAST,
        winner.fate_strategies.contains(&21),
        winner.xian_mo_strategies.contains(&2011),
    );
    if game_mode == GAME_MODE_ROGUE {
        // Rogue applies display-only damage plus a debris heal whose state is
        // not in the fixture: the base is computable, the ends are not.
        return None;
    }
    let (dealt, loser_end, survival, jiangsha) = apply_death_damage(base, loser, round)?;
    let winner_end = winner.battle_end_temp_life;
    let (p1_end, p2_end) = match inputs.winner_side {
        PlayerSide::P1 => (winner_end, loser_end),
        PlayerSide::P2 => (loser_end, winner_end),
    };
    let dealt_signed = match inputs.winner_side {
        PlayerSide::P1 => dealt,
        PlayerSide::P2 => -dealt,
    };
    let actual_dealt = if loser.is_mirror { 0 } else { dealt };
    Some(LifeSettleOutcome {
        final_life: FinalLife {
            p1: p1_end,
            p2: p2_end,
        },
        dealt_signed,
        detail: LifeSettleDetail {
            base_damage: base,
            actual_dealt,
            survival_talent: survival,
            jiangsha_105: jiangsha,
        },
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side<'a>(
        talents: &'a [i64],
        temp: &'a BTreeMap<String, i64>,
        fate: &'a [i64],
        xm: &'a [i64],
        perms: &'a BTreeMap<String, i64>,
        flags: &'a BTreeMap<String, i64>,
    ) -> LifeSettleSideInputs<'a> {
        LifeSettleSideInputs {
            talents,
            talent_temp_datas: temp,
            fate_strategies: fate,
            xian_mo_strategies: xm,
            permanent_buff_temp_datas: perms,
            is_mirror: false,
            pre_battle_life: Some(100),
            talent_resonance_id: None,
            resonance_talent_flags: flags,
            battle_end_temp_life: 100,
            battle_end_hp: 0,
            battle_end_ci_nian_qu: 0,
        }
    }

    fn inputs<'a>(
        round: i64,
        game_mode: i64,
        p1: LifeSettleSideInputs<'a>,
        p2: LifeSettleSideInputs<'a>,
    ) -> LifeSettleInputs<'a> {
        LifeSettleInputs {
            round: Some(round),
            game_mode: Some(game_mode),
            winner_side: PlayerSide::P1,
            p1,
            p2,
        }
    }

    #[test]
    fn base_small_gap_round_over_9() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 40;
        p2.battle_end_hp = 28;
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 12 + 1 + 3);
        assert_eq!(out.dealt_signed, 16);
        assert_eq!(out.final_life, FinalLife { p1: 100, p2: 84 });
    }

    #[test]
    fn base_large_gap_caps_extra() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 100;
        p2.battle_end_hp = -50;
        let out = settle_battle_life(&inputs(10, 2, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 10 + 5 + (2 + 5));
        assert_eq!(out.final_life.p2, 100 - 22);
    }

    #[test]
    fn base_early_round_minus_one() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 5;
        p2.battle_end_hp = 3;
        let out = settle_battle_life(&inputs(7, 2, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 7 + 1 + 1 - 1);
        assert_eq!(out.dealt_signed, 8);
    }

    #[test]
    fn winner_reductions_floor_at_one() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[21], &[2011], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 5;
        p2.battle_end_hp = 3;
        let out = settle_battle_life(&inputs(1, 2, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 1);
        // FateStrategy 21 reduces by exactly 1 (27/27 shard battles).
        let mut q1 = side(&[], &t, &[21], &[], &t, &t);
        let mut q2 = side(&[], &t, &[], &[], &t, &t);
        q1.battle_end_hp = 40;
        q2.battle_end_hp = 28;
        let out = settle_battle_life(&inputs(12, 2, q1, q2)).unwrap();
        assert_eq!(out.detail.base_damage, 15);
    }

    #[test]
    fn cini_then_halve_order() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[45], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 109;
        p2.battle_end_hp = 0;
        p2.battle_end_ci_nian_qu = 4;
        // round 7, gap 109: 7+5+min(9,5)=17, -1 = 16; -4 = 12; /2 = 6.
        let out = settle_battle_life(&inputs(7, 3, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 16);
        assert_eq!(out.dealt_signed, 6);
        assert_eq!(out.final_life.p2, 94);
    }

    #[test]
    fn longlin_cap_and_survival_to_one() {
        let t = BTreeMap::new();
        let mut tt = BTreeMap::new();
        tt.insert("65".to_string(), 1);
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[27, 65], &tt, &[], &[], &t, &t);
        p1.battle_end_hp = 60;
        p2.battle_end_hp = -10;
        p2.battle_end_temp_life = 18;
        // round 12, gap 70: 12+5+min(5,8)=22; cap 20; 20>=18 triggers 65.
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.dealt_signed, 17);
        assert_eq!(out.final_life.p2, 1);
        assert_eq!(out.detail.survival_talent, Some(65));
    }

    #[test]
    fn jiangsha_immunity_needs_temp_count() {
        let t = BTreeMap::new();
        let mut tt = BTreeMap::new();
        tt.insert("105".to_string(), 1);
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[105], &tt, &[], &[], &t, &t);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.dealt_signed, 0);
        assert!(out.detail.jiangsha_105);
    }

    #[test]
    fn jiangsha_without_count_does_not_trigger() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[105], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert!(!out.detail.jiangsha_105);
        assert_eq!(out.dealt_signed, 20);
    }

    #[test]
    fn mirror_loser_keeps_temp_life() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        p2.is_mirror = true;
        p2.battle_end_temp_life = 77;
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.dealt_signed, 20);
        assert_eq!(out.detail.actual_dealt, 0);
        assert_eq!(out.final_life.p2, 77);
    }

    #[test]
    fn resonance_106_flag_immunity() {
        let t = BTreeMap::new();
        let mut flags = BTreeMap::new();
        flags.insert("106".to_string(), 2);
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &flags);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        p2.talent_resonance_id = Some(106);
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.dealt_signed, 0);
    }

    #[test]
    fn armed_modes_and_inputs() {
        let t = BTreeMap::new();
        let mk = || {
            let mut p1 = side(&[], &t, &[], &[], &t, &t);
            let mut p2 = side(&[], &t, &[], &[], &t, &t);
            p1.battle_end_hp = 50;
            p2.battle_end_hp = 0;
            (p1, p2)
        };
        for mode in [13, 121, 16, 14, 11] {
            let (p1, p2) = mk();
            assert!(
                settle_battle_life(&inputs(12, mode, p1, p2)).is_none(),
                "mode {mode}"
            );
        }
        let (p1, p2) = mk();
        let mut no_round = inputs(12, 2, p1, p2);
        no_round.round = None;
        assert!(settle_battle_life(&no_round).is_none());
        let mut shi = BTreeMap::new();
        shi.insert("10030".to_string(), 1);
        let (p1, _) = mk();
        let (_, mut p2s) = mk();
        p2s.permanent_buff_temp_datas = &shi;
        assert!(settle_battle_life(&inputs(12, 2, p1, p2s)).is_none());
    }

    #[test]
    fn zhan_bai_arms() {
        let t = BTreeMap::new();
        let mut zb = BTreeMap::new();
        zb.insert("20040".to_string(), 3);
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &zb, &t);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        assert!(settle_battle_life(&inputs(12, 2, p1, p2)).is_none());
    }

    #[test]
    fn resonance_immunity_beats_survival() {
        let t = BTreeMap::new();
        let mut tt = BTreeMap::new();
        tt.insert("7".to_string(), 1);
        let mut flags = BTreeMap::new();
        flags.insert("27".to_string(), 1);
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[7], &tt, &[], &[], &t, &flags);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        p2.battle_end_temp_life = 5;
        p2.talent_resonance_id = Some(27);
        let out = settle_battle_life(&inputs(12, 2, p1, p2)).unwrap();
        assert_eq!(out.dealt_signed, 0);
        assert_eq!(out.final_life.p2, 5);
        assert_eq!(out.detail.survival_talent, None);
    }

    #[test]
    fn pre_zero_loser_arms() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 50;
        p2.battle_end_hp = 0;
        p2.pre_battle_life = Some(0);
        assert!(settle_battle_life(&inputs(12, 2, p1, p2)).is_none());
    }

    #[test]
    fn fast_mode_scales_base() {
        let t = BTreeMap::new();
        let mut p1 = side(&[], &t, &[], &[], &t, &t);
        let mut p2 = side(&[], &t, &[], &[], &t, &t);
        p1.battle_end_hp = 40;
        p2.battle_end_hp = 28;
        let out = settle_battle_life(&inputs(12, 17, p1, p2)).unwrap();
        assert_eq!(out.detail.base_damage, 16 * 3 / 2);
    }
}
