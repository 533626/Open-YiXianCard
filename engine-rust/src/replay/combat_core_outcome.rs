use super::{ReplayPlayer, ReplayState, ReplaySummary, ReviveKind, ReviveReceipt};
use crate::model::PlayerSide;

impl ReplayState {
    pub(super) fn summary(&self, winner_side: PlayerSide) -> ReplaySummary {
        ReplaySummary {
            winner_side,
            actor_turn_count: self.actor_turn,
            hp_delta_p1_minus_p2: self.p1.core.hp - self.p2.core.hp,
        }
    }

    pub(super) fn death_winner(&mut self) -> Option<PlayerSide> {
        // Revive checks run in the original client's order and return typed
        // receipts; the outcome loop only needs the final alive/dead result
        // (the mutation-contract tests assert the receipts).
        let _ = (
            self.check_flame_soul_return(PlayerSide::P1),
            self.check_fire_phoenix_revive(PlayerSide::P1),
            self.check_nine_heavens_revive(PlayerSide::P1),
            self.check_qi_xing_jie_ming(PlayerSide::P1),
            self.check_flame_soul_return(PlayerSide::P2),
            self.check_fire_phoenix_revive(PlayerSide::P2),
            self.check_nine_heavens_revive(PlayerSide::P2),
            self.check_qi_xing_jie_ming(PlayerSide::P2),
        );
        if self.p1.core.hp > 0 && self.p2.core.hp > 0 {
            return None;
        }
        let p1_continues =
            self.check_last_stand(PlayerSide::P1) && self.p1.core.hp <= self.p2.core.hp;
        let p2_continues = !p1_continues
            && self.check_last_stand(PlayerSide::P2)
            && self.p2.core.hp <= self.p1.core.hp;
        if !p1_continues && !p2_continues && (self.p1.core.hp <= 0 || self.p2.core.hp <= 0) {
            return Some(self.hp_winner());
        }
        None
    }

    pub(super) fn check_flame_soul_return(&mut self, side: PlayerSide) -> Option<ReviveReceipt> {
        if self.actor(side).core.hp > 0
            || self.actor(side).fate.flame_soul_return <= 0
            || self.actor(side).chance.cannot_revive > 0
        {
            return None;
        }
        let hp_before = self.actor(side).core.hp;
        // 原版 SetMaxHp/SetHp：直接替换生命与上限，不走加成管线。
        let actor = self.actor_mut(side);
        actor.core.max_hp = 15;
        actor.core.hp = 15;
        actor.fate.flame_soul_return = 0;
        let receipt = ReviveReceipt {
            kind: ReviveKind::FlameSoulReturn,
            hp_after: actor.core.hp,
            max_hp_after: actor.core.max_hp,
        };
        self.record_mutation_receipt(
            side,
            super::ReplayMutationKind::Revive,
            "核心",
            "hp",
            "生命",
            hp_before,
            receipt.hp_after,
            receipt.hp_after - hp_before,
        );
        Some(receipt)
    }

    pub(super) fn check_fire_phoenix_revive(&mut self, side: PlayerSide) -> Option<ReviveReceipt> {
        let revive_hp = self.actor(side).fate.fire_phoenix_revive_hp;
        if self.actor(side).core.hp > 0
            || revive_hp <= 0
            || self.actor(side).chance.cannot_revive > 0
        {
            return None;
        }
        let hp_before = self.actor(side).core.hp;
        {
            let actor = self.actor_mut(side);
            actor.core.max_hp += revive_hp;
        }
        let healing = revive_hp - self.actor(side).core.hp;
        self.modify_actor_hp(side, healing, false, false);
        self.actor_mut(side).fate.fire_phoenix_revive_hp = 0;
        let receipt = ReviveReceipt {
            kind: ReviveKind::FirePhoenix,
            hp_after: self.actor(side).core.hp,
            max_hp_after: self.actor(side).core.max_hp,
        };
        self.record_mutation_receipt(
            side,
            super::ReplayMutationKind::Revive,
            "核心",
            "hp",
            "生命",
            hp_before,
            receipt.hp_after,
            receipt.hp_after - hp_before,
        );
        Some(receipt)
    }

    pub(super) fn check_nine_heavens_revive(&mut self, side: PlayerSide) -> Option<ReviveReceipt> {
        if self.actor(side).core.hp > 0
            || self.actor(side).mirage_ronghui.nine_heavens_revive <= 0
            || self.actor(side).chance.cannot_revive > 0
        {
            return None;
        }
        let hp_before = self.actor(side).core.hp;
        if self.actor(side).core.max_hp <= 0 {
            self.modify_actor_max_hp(side, 1);
        }
        let healing = 64 - self.actor(side).core.hp;
        self.modify_actor_hp(side, healing, false, false);
        self.actor_mut(side).mirage_ronghui.nine_heavens_revive -= 1;
        let receipt = ReviveReceipt {
            kind: ReviveKind::NineHeavens,
            hp_after: self.actor(side).core.hp,
            max_hp_after: self.actor(side).core.max_hp,
        };
        self.record_mutation_receipt(
            side,
            super::ReplayMutationKind::Revive,
            "核心",
            "hp",
            "生命",
            hp_before,
            receipt.hp_after,
            receipt.hp_after - hp_before,
        );
        Some(receipt)
    }

    /// 七星借命 FateStrategy 436（BattleExecuter.CharacterResurrectionCheckAsync
    /// IL_0855 起）：战斗中首次生命 ≤ 0 且持有 QiXingJieMing 标记时，失去所有
    /// 卦象与星力，每失去 1 点按标记值（otherParams[0]=3）加生命及上限；
    /// 若转换后生命 > 0 则继续战斗（oracle 锚点：mirror-32219000-human-01
    /// 2995be139404d0ed/round-10 cp14→15：p1 星力 5 → 生命 -3 → +15 上限/生命
    /// → 12，随后内伤 -2 → 10，turn15 七星定魂继续）。
    pub(super) fn check_qi_xing_jie_ming(&mut self, side: PlayerSide) -> Option<ReviveReceipt> {
        if self.actor(side).core.hp > 0
            || self.actor(side).fate.qi_xing_jie_ming <= 0
            || self.actor(side).chance.cannot_revive > 0
        {
            return None;
        }
        let conversion = (self.actor(side).astrology.hexagram
            + self.actor(side).astrology.star_power)
            * self.actor(side).fate.qi_xing_jie_ming;
        self.actor_mut(side).fate.qi_xing_jie_ming = 0;
        if conversion > 0 {
            let hp_before = self.actor(side).core.hp;
            // 原版 RemoveBuff(GuaXiang)/RemoveBuff(XingLi)：原始移除，绕过
            // ModifyBuffValue 的卦象/星力流失转换钩子（如 422 紫芒星爆）。
            self.actor_mut(side).astrology.hexagram = 0;
            self.remove_all_star_power_for_qi_xing_jie_ming(side);
            self.modify_actor_max_hp(side, conversion);
            // 原版 ModifyHp(num4, canRevive: true)：生命 ≤ 0 时仍允许治疗。
            self.modify_actor_hp(side, conversion, false, false);
            let receipt = ReviveReceipt {
                kind: ReviveKind::QiXingJieMing,
                hp_after: self.actor(side).core.hp,
                max_hp_after: self.actor(side).core.max_hp,
            };
            self.record_mutation_receipt(
                side,
                super::ReplayMutationKind::Revive,
                "核心",
                "hp",
                "生命",
                hp_before,
                receipt.hp_after,
                receipt.hp_after - hp_before,
            );
            return Some(receipt);
        }
        None
    }

    fn check_last_stand(&mut self, side: PlayerSide) -> bool {
        if self.actor(side).core.hp <= 0 && self.actor(side).fate.last_stand_intent > 0 {
            let actor = self.actor_mut(side);
            actor.fate.last_stand_unyielding += 1;
            actor.fate.last_stand_intent = 0;
        }
        self.actor(side).fate.last_stand_unyielding > 0
    }

    pub(super) fn hp_winner(&self) -> PlayerSide {
        if self.p1.core.hp == self.p2.core.hp {
            self.first_player
        } else if self.p1.core.hp > self.p2.core.hp {
            PlayerSide::P1
        } else {
            PlayerSide::P2
        }
    }

    pub(super) fn actor(&self, side: PlayerSide) -> &ReplayPlayer {
        match side {
            PlayerSide::P1 => &self.p1,
            PlayerSide::P2 => &self.p2,
        }
    }

    pub(super) fn actor_mut(&mut self, side: PlayerSide) -> &mut ReplayPlayer {
        match side {
            PlayerSide::P1 => &mut self.p1,
            PlayerSide::P2 => &mut self.p2,
        }
    }
}
