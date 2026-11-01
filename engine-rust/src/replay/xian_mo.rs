//! 仙魔（build 25621897 赛季机制）策略的战斗侧门禁。
//!
//! `data/xian-mo-battle-strategies.json` 从反编译战斗源码与 `XianMoStrategyConfig.isBattleEffect` 穷举生成。选中其中任一策略而引擎尚未
//! 实现时一律 fail closed（`MissingRule`），不静默忽略——未实现的战斗效果会让终局三元失真。

use super::BattleError;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// 已按当前 build 原版源码实现并由真实回放取证的仙魔策略。
const IMPLEMENTED_XIAN_MO_STRATEGY_IDS: &[i64] = &[
    1001, 1002, 1003, 1004, 1005, 1006, 1007, 1008, 1009, 1010, 3001, 3002, 3003, 3004, 3011, 3012,
    5001, 5004, 5005, 5007, 5008, 5009, 5010, 5011, 7002, 7005, 7010, 7017, 7021, 7022, 7024, 7026,
    7028, 7031, 7032, 7035, 7036, 7037, 7042, 7048, 7049, 7050, 7051, 7052, 7053, 7054, 7056, 7057,
    7058, 7059, 7060, 7061, 7063, 7064, 7065, 7066, 7067, 7068, 7069, 7071, 7076, 7077, 7078, 7079,
    7080, 7082, 7083, 7084, 7085, 7087, 7088, 7089, 7090, 7091, 7092, 7093, 7094, 7095, 7096, 7097,
    7098, 7099, 7100, 7101, 7102, 7103, 7104, 7105, 7109, 7110, 7111, 7112, 7113, 7115, 7116, 7118,
    7120, 7122, 7123, 7127, 7128, 7129, 7130, 7132, 7133, 7135, 7136, 7138, 11001, 11002, 11003,
    11004, 13001, 13002, 13003, 13004, 13005, 13006, 13007, 13008, 13009, 13010, 13011, 13012,
    13013, 13014, 13015, 13016, 13017, 13018, 13019, 15018, 17071, 23045, 23058, 23062, 23066,
    23069, 23079, 23080, 27021, 27023, 28001,
];

impl<const OBSERVE: bool> super::ReplayState<OBSERVE> {
    #[inline]
    pub(super) fn is_xian_mo_switch_active(
        &self,
        side: crate::model::PlayerSide,
        strategy: i64,
    ) -> bool {
        let actor = self.actor(side);
        if !actor.identity.xian_mo_strategies.contains(&strategy) {
            return false;
        }
        let key = strategy.to_string();
        actor
            .identity
            .xian_mo_temp_datas
            .get(&key)
            .copied()
            .unwrap_or(0)
            == 0
    }

    pub(super) fn apply_verified_xian_mo_pending_downgrade(
        &mut self,
        side: crate::model::PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        if self.actor(side).fate.next_card_downgrade <= 0
            || drawn.card.id == 19
            || super::original_config::original_config_rarity(drawn.card.id) < 1
        {
            return;
        }
        let id = drawn.card.id - 10_000;
        let Some(card) = super::original_config::original_card_definition(id) else {
            self.missing_card_effect(
                id,
                super::support::normalize_base_id(id),
                "xianMo pending downgrade missing card definition",
            );
            return;
        };
        self.actor_mut(side).fate.next_card_downgrade -= 1;
        self.actor_mut(side).deck.slots[drawn.source_slot].card = card.clone().into();
        drawn.card = card.into();
    }

    pub(super) fn apply_verified_xian_mo_after_card(
        &mut self,
        side: crate::model::PlayerSide,
        slot: usize,
    ) {
        let actor = self.actor(side);
        if !actor.identity.xian_mo_strategies.contains(&13006)
            || actor
                .identity
                .xian_mo_temp_datas
                .get("13006")
                .copied()
                .unwrap_or(0)
                != slot as i64
        {
            return;
        }
        let Some(frame) = self.effect_invocation_stack.last() else {
            self.missing_card_effect(0, 0, "xianMo strategy 13006 missing effect use history");
            return;
        };
        // 26125017: once per battle (ZhuiJiKeWenYiChuFa 794 gate, CardActionBase.cs:4930).
        if !frame.was_used_before_effect && self.actor(side).fate.xian_mo_13006_triggered == 0 {
            self.actor_mut(side).fate.xian_mo_13006_triggered = 1;
            self.apply_attack(side, 6, slot);
        }
    }

    pub(super) fn apply_verified_xian_mo_normal_attack_copy(
        &mut self,
        side: crate::model::PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        // 26125017: CardActionBase.cs:1370 adds gridNumber != 0 (slot 0 has no
        // previous grid to copy; gridNumber is 0-based like source_slot).
        if self.actor(side).fate.normal_attack_copy_previous <= 0
            || drawn.source_slot == 0
            || !super::support::is_pu_tong_gong_ji_base_id(super::support::normalized_base_id(
                &drawn.card,
            ))
        {
            return;
        }
        self.actor_mut(side).fate.normal_attack_copy_previous -= 1;
        let index = super::support::neighbor_slot_index(self.actor(side), drawn.source_slot, -1);
        let Some(previous) = self
            .actor(side)
            .deck
            .slots
            .get(index)
            .map(|s| s.card.definition())
        else {
            self.missing_card_effect(
                drawn.card.id,
                super::support::normalized_base_id(&drawn.card),
                "xianMo strategy 5005 requires the adjacent physical slot",
            );
            return;
        };
        let mut id = previous.id;
        if id == 19 {
            return;
        }
        if super::original_config::original_config_rarity(id) == 2
            && super::original_config::original_card_no_upgrade(super::support::normalized_base_id(
                previous,
            )) == Some(false)
        {
            id -= 10_000;
        }
        let Some(card) = super::original_config::original_card_definition(id) else {
            self.missing_card_effect(
                id,
                super::support::normalize_base_id(id),
                "xianMo strategy 5005 missing copied card definition",
            );
            return;
        };
        self.actor_mut(side).deck.slots[drawn.source_slot].card = card.clone().into();
        drawn.card = card.into();
    }

    /// 伏魔 5009 (CardActionBase.cs:1470→IL_3658→IL_3703→IL_37d2)：primary
    /// ExecuteEffect 之后，若所出的牌境界≤筑基(2)、非隐藏、且位于伏魔格
    /// (xianMoData.tempDatas[5009]，缺省 0，与 source_slot 同为 0-based)，
    /// 则把同一张牌再执行一次。再执行走 ExecuteEffect 直发，不重入
    /// Execute 的 1470 检查，故无递归。Batch2 曾误判 1470 块纯装饰
    /// （只读了换肤/浮字/延迟，没跟 goto 链）；R-855f195a 证伪。
    /// 取证：pa_99a27961+17（slot6 驱邪丹防/净化双倍），
    /// oracle pa_feaf734f；对照 pa_e63d7c05（无 5009 单倍）。
    pub(super) fn fu_mo_grid_repeats_card(
        &self,
        side: crate::model::PlayerSide,
        card_id: i64,
        slot: usize,
    ) -> bool {
        let actor = self.actor(side);
        if !actor.identity.xian_mo_strategies.contains(&5009) {
            return false;
        }
        if actor
            .identity
            .xian_mo_temp_datas
            .get("5009")
            .copied()
            .unwrap_or(0)
            != slot as i64
        {
            return false;
        }
        if super::original_config::original_card_is_hidden(card_id) != Some(false) {
            return false;
        }
        matches!(
            super::original_config::original_card_realm_level(card_id),
            Some(level) if level <= 2
        )
    }

    pub(super) fn apply_verified_xian_mo_first_use_upgrade(
        &mut self,
        side: crate::model::PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        let actor = self.actor(side);
        if !actor.identity.xian_mo_strategies.contains(&13003)
            || actor
                .identity
                .xian_mo_temp_datas
                .get("13003")
                .copied()
                .unwrap_or(0)
                != drawn.source_slot as i64
            || actor.deck.slots[drawn.source_slot].used
            || super::original_config::original_card_is_hidden(drawn.card.id) != Some(false)
            || !super::original_config::can_upgrade_original_battle_deck_card(drawn.card.id)
        {
            return;
        }
        let upgraded = super::original_config::upgrade_original_card(&drawn.card, 1);
        self.actor_mut(side).deck.slots[drawn.source_slot].card = upgraded.clone().into();
        drawn.card = upgraded.into();
    }

    /// 26125017 new: WuXingMiJuanShengJi(792) consumed to upgrade the executing
    /// WuXingDaoMeng card one tier (CardActionBase.cs:1752-1843, running after the
    /// frenzy upgrade IL_1d74 in Execute). RefreshSpecialCard is a no-op for
    /// non-19/126/TA27 cards, so only the tier walk applies.
    pub(super) fn apply_verified_xian_mo_wuxing_mijuan_upgrade(
        &mut self,
        side: crate::model::PlayerSide,
        drawn: &mut super::DrawnCard,
    ) {
        if self.actor(side).fate.xian_mo_11003_wuxing_upgrades <= 0 {
            return;
        }
        if super::original_config::original_card_sect_name(drawn.card.id).as_deref()
            != Some("WuXingDaoMeng")
            || !super::original_config::can_upgrade_original_battle_deck_card(drawn.card.id)
        {
            return;
        }
        self.actor_mut(side).fate.xian_mo_11003_wuxing_upgrades -= 1;
        let upgraded = super::original_config::upgrade_original_card(&drawn.card, 1);
        self.actor_mut(side).deck.slots[drawn.source_slot].card = upgraded.clone().into();
        drawn.card = upgraded.into();
    }

    pub(super) fn apply_verified_xian_mo_before_battle_start(&mut self) {
        use crate::model::PlayerSide::{P1, P2};
        // BattleExecuter.BeforeBattleStarted:1002. The two OR conditions
        // are independent in the original (ownership and active switch).
        let owns = [P1, P2]
            .into_iter()
            .any(|side| self.actor(side).identity.xian_mo_strategies.contains(&1009));
        // IsSwitchActive(side, 1009) 先要求该侧持有策略；缺 tempData 视为 0（开启）。
        let active = [P1, P2].into_iter().any(|side| {
            let identity = &self.actor(side).identity;
            identity.xian_mo_strategies.contains(&1009)
                && identity
                    .xian_mo_temp_datas
                    .get("1009")
                    .copied()
                    .unwrap_or(0)
                    == 0
        });
        if !owns || !active {
            return;
        }
        for side in [P1, P2] {
            let count = self.actor(side).deck.active_slot_count;
            for index in count.saturating_sub(3)..count {
                let card = &self.actor(side).deck.slots[index].card;
                let rarity = super::original_config::original_config_rarity(card.id);
                if rarity >= 2
                    || super::original_config::original_card_is_hidden(card.id) != Some(false)
                    || !super::original_config::can_upgrade_original_battle_deck_card(card.id)
                {
                    continue;
                }
                let upgraded = super::original_config::upgrade_original_card(card, 2 - rarity);
                self.actor_mut(side).deck.slots[index].card = upgraded.into();
            }
        }
    }

    pub(super) fn apply_verified_xian_mo_battle_start_effects(
        &mut self,
        actor_side: crate::model::PlayerSide,
        player: &crate::fixture::FixturePlayerView<'_>,
    ) {
        // 25981480 XianMoStrategyFunctions.OnBattleStart: execute switches in
        // source order inside each actor's OnBattleStarted boundary. Missing
        // tempData means 0 (active); each owner applies to both characters.
        // 26125017 new: last-round permanent XianShouHuoTongXiuJiaShengMing(10060)
        // applies first and is NOT strategy-gated (XianMoStrategyFunctions.cs:248),
        // so it runs before the empty-strategies early return.
        // 原版客户端按 per-character 无条件应用双方各自的 10060，没有先手或
        // 同修为判断。曾为贴合部分服务器战报加过“仅先手”门，属错误客户端规则，
        // 已删除：玩家录像（pa_d40b4116）与 HF 战报同场对证
        // pa_d7b90378（后手 p2 10060=10）：客户端后手以 7 血存活
        // 反杀（p1=-76、p2=7），与无条件应用一致；有门时后手 -3 血死亡、
        // 复现的只是服务器战报（p1 胜、Δ=11）。服务器/客户端结算分歧由
        // diag-10060-taptap-receipt.json 冻结记录，expected 保留服务器原值。
        let xianshou_life = player
            .permanent_buff_temp_datas
            .get("10060")
            .copied()
            .unwrap_or(0)
            .max(0);
        if xianshou_life > 0 {
            self.modify_actor_max_hp(actor_side, xianshou_life);
            self.modify_actor_hp(actor_side, xianshou_life, false, false);
        }
        if player.xian_mo_strategies.is_empty() {
            return;
        }
        let target_side = super::support::opponent_side(actor_side);
        // XianMoStrategyFunctions.OnBattleStart 顺序为 1001 → 1002 → 1003
        // (IL_02ff) → 1004/1005。1003 采样必须留在此位置：后续的 7098、
        // 7105、11004 等开局分支会改变当前 HP 或上限。战斗外天赋和 Fate
        // 开局循环已在调用此方法前完成，故 1003 仍能看到原版相同的采样 HP。
        self.apply_verified_xian_mo_battle_start_switches(
            actor_side,
            player,
            &[(1001, "1001"), (1002, "1002")],
        );
        self.apply_verified_xian_mo_strategy_1003_battle_start(actor_side, player);
        self.apply_verified_xian_mo_battle_start_switches(
            actor_side,
            player,
            &[(1004, "1004"), (1005, "1005")],
        );
        self.apply_verified_xian_mo_strategy_1006_battle_start(actor_side, player);
        self.apply_verified_xian_mo_battle_start_switches(
            actor_side,
            player,
            &[(1007, "1007"), (1008, "1008")],
        );
        if player.xian_mo_strategies.contains(&3001) {
            // 26125017: 龙威震慑转移链删除，改为开局直接给对手 JianGong(103) += otherParams[0]=1
            // (XianMoStrategyFunctions.cs:1063-1066)。
            self.add_actor_negative_status(super::support::opponent_side(actor_side), 103, 1);
        }
        if player.xian_mo_strategies.contains(&3002) {
            self.gain_guard(actor_side, 1);
        }
        if player.xian_mo_strategies.contains(&3004) {
            self.actor_mut(actor_side).fate.dragon_first_attack_stealth += 1;
        }
        if player.xian_mo_strategies.contains(&3011) {
            self.actor_mut(actor_side).fate.next_anima_gain_bonus += 2;
        }
        if player.xian_mo_strategies.contains(&5005) {
            self.actor_mut(actor_side).fate.normal_attack_copy_previous += 1;
        }
        if player.xian_mo_strategies.contains(&5007) {
            self.actor_mut(actor_side)
                .fate
                .normal_attack_guard_and_exhaust += 3;
        }
        if player.xian_mo_strategies.contains(&5008) {
            self.actor_mut(actor_side)
                .fate
                .normal_attack_bonus_and_exhaust += 3;
        }
        if player.xian_mo_strategies.contains(&5011) {
            // 26125017 new: XiaCiPuGongXiaoHao(793) += 1.
            self.actor_mut(actor_side).fate.xian_mo_5011_pending_pu_gong += 1;
        }
        if player.xian_mo_strategies.contains(&7005) {
            self.gain_cloud_chain(actor_side, 1);
            self.actor_mut(actor_side).sword.cloud_sea += 2;
        }
        // 25981480 XianMoStrategyFunctions.OnBattleStart:1021-1024.
        if player.xian_mo_strategies.contains(&7017) {
            self.actor_mut(actor_side).fate.sword_formation_guard += 1;
        }
        if player.xian_mo_strategies.contains(&7021)
            && self.actor(actor_side).deck.active_slot_count >= 8
        {
            let card = &self.actor(actor_side).deck.slots[7].card;
            let base = super::support::normalized_base_id(card);
            let replacement = if base == 11
                && super::original_config::can_upgrade_original_battle_deck_card(card.id)
            {
                Some(super::original_config::upgrade_original_card(card, 1))
            } else if base == 0 {
                super::original_config::original_card_definition(11)
            } else {
                None
            };
            if let Some(replacement) = replacement {
                self.actor_mut(actor_side).deck.slots[7].card = replacement.into();
            }
        }
        if player.xian_mo_strategies.contains(&7024) {
            let amount = player
                .permanent_buff_temp_datas
                .get("10037")
                .copied()
                .unwrap_or(0)
                .max(0);
            self.modify_hexagram(actor_side, amount);
        }
        if player.xian_mo_strategies.contains(&7028) {
            for element in super::support::fate_strategy_131_elements(&player.talents) {
                self.add_direct_element_activation(actor_side, element);
            }
        }
        // IL_114e：7098 先直写 JiHuoHuoLing，再扣对手生命及上限 otherParams[0]=4。
        if player.xian_mo_strategies.contains(&7098) {
            self.add_direct_element_activation(actor_side, super::Element::Fire);
            self.modify_actor_hp(target_side, -4, false, false);
            self.modify_actor_max_hp(target_side, -4);
        }
        // IL_1339 / 13f3 / 14b2: direct activation buffs intentionally do
        // not run ActiveWuXing callbacks, unlike cards that call that method.
        if player.xian_mo_strategies.contains(&7066) {
            self.add_direct_element_activation(actor_side, super::Element::Wood);
            self.gain_anima(actor_side, 2);
        }
        if player.xian_mo_strategies.contains(&7095) {
            self.add_direct_element_activation(actor_side, super::Element::Earth);
            self.gain_defense(actor_side, 8);
        }
        if player.xian_mo_strategies.contains(&7096) {
            self.add_direct_element_activation(actor_side, super::Element::Metal);
            self.gain_sharpness(actor_side, 4);
        }
        if player.xian_mo_strategies.contains(&7031)
            && player.xian_mo_temp_datas.get("7031").copied().unwrap_or(0) == 0
        {
            self.actor_mut(actor_side).formations.spirit_formation_echo += 1;
        }
        if player.xian_mo_strategies.contains(&7036) {
            if player.xian_mo_temp_datas.get("7036").copied().unwrap_or(0) == 0 {
                self.actor_mut(actor_side).fate.exorcism += 2;
            }
            self.modify_actor_hp(actor_side, 9, false, false);
        }
        if player.xian_mo_strategies.contains(&7053) {
            self.actor_mut(actor_side).fate.yan_qi += 1;
        }
        if player.xian_mo_strategies.contains(&7054) {
            self.upgrade_first_battle_deck_card(actor_side, |card| {
                card.name.contains('雷')
                    || super::original_config::original_card_current_desc_contains_action_again(
                        card.id,
                    )
            });
        }
        if player.xian_mo_strategies.contains(&7056) {
            self.actor_mut(actor_side).fate.qi_xing_lian_zhu += 7;
        }
        if player.xian_mo_strategies.contains(&7058) {
            self.upgrade_first_battle_deck_card(actor_side, |card| card.name.contains("木灵"));
        }
        if player.xian_mo_strategies.contains(&7059) {
            self.actor_mut(actor_side).fate.feng_ling_zhan_yi += 5;
        }
        // 7063：对手获得 DengYanCuiXin(=flame_heart_urging) otherParams[0]=3 层。
        if player.xian_mo_strategies.contains(&7063) {
            self.actor_mut(target_side).status.flame_heart_urging += 3;
        }
        if player.xian_mo_strategies.contains(&7064) {
            self.actor_mut(actor_side).fate.vermilion_bird_tear += 1;
        }
        if player.xian_mo_strategies.contains(&7067) {
            self.actor_mut(actor_side).sword.sword_energy += 1;
        }
        if player.xian_mo_strategies.contains(&7069) {
            let mut count = super::support::active_deck_cards(self.actor(actor_side))
                .filter(|card| super::support::is_cloud_sword(self.actor(actor_side), card))
                .count() as i64;
            for id in player.hand_cards.iter().copied() {
                let Some(card) = super::original_config::original_card_definition(id) else {
                    self.missing_card_effect(
                        id,
                        super::support::normalize_base_id(id),
                        "xianMo strategy 7069 missing hand-card definition",
                    );
                    return;
                };
                count += i64::from(super::support::is_cloud_sword(
                    self.actor(actor_side),
                    &card,
                ));
            }
            self.gain_anima(actor_side, (count / 2).min(3));
        }
        if player.xian_mo_strategies.contains(&7077) {
            self.actor_mut(actor_side).fate.wu_jing_gua_yan += 3;
        }
        if player.xian_mo_strategies.contains(&7078) {
            self.modify_star_power(actor_side, 1);
            if self.actor(actor_side).identity.last_round_exp
                >= self.actor(target_side).identity.last_round_exp + 5
            {
                self.modify_star_power(actor_side, 1);
            }
        }
        if player.xian_mo_strategies.contains(&7079) {
            self.actor_mut(actor_side).fate.xing_yuan += 3;
        }
        if player.xian_mo_strategies.contains(&7083) {
            let amount = 3 * super::support::wu_xing_count_in_deck(self.actor(actor_side));
            self.modify_actor_max_hp(actor_side, amount);
            self.modify_actor_hp(actor_side, amount, false, false);
        }
        if player.xian_mo_strategies.contains(&7087) {
            self.actor_mut(actor_side).beng.pending_momentum_bonus += 2;
        }
        if player.xian_mo_strategies.contains(&7091) {
            self.actor_mut(actor_side).fate.tian_yan_feng_ling_duan_qu += 5;
        }
        if player.xian_mo_strategies.contains(&7093) {
            self.actor_mut(actor_side).fate.qi_xing_jie_ming += 3;
        }
        if player.xian_mo_strategies.contains(&7094)
            && player.xian_mo_temp_datas.get("7094").copied().unwrap_or(0) == 0
        {
            self.gain_attack_bonus(actor_side, 1);
            self.add_actor_negative_status(actor_side, 100, 2);
        }
        if player.xian_mo_strategies.contains(&7099) {
            // 26125017: IsXianMoStrategyEffective gate (effectRound=13).
            match self.battle_round {
                Some(round) if round >= 13 => {
                    self.actor_mut(actor_side).fate.cu_ju_fei_xi += 6;
                }
                Some(_) => {}
                None => {
                    self.missing_card_effect(0, 0, "xianMo strategy 7099 missing original round");
                }
            }
        }
        if player.xian_mo_strategies.contains(&7100) {
            match self.battle_round {
                Some(round) if round >= 13 => {
                    self.upgrade_first_battle_deck_card(actor_side, |card| {
                        card.name.contains("火灵")
                    });
                }
                Some(_) => {}
                None => {
                    self.missing_card_effect(0, 0, "xianMo strategy 7100 missing original round");
                }
            }
        }
        // 7103 在 11004 之前采样当前 HP：26315169 OnBattleStart goto 链
        // 7103(IL_15e9…)→…→7123→IL_1fdb(11004)；oracle 4809
        // (pa_1e09f55f) p2 98→110/110(AddHpCount=12，
        // 7103 +9 then 11004 +3)。勿推迟到开局块末（1003/1006 的延迟采样
        // 不适用于 7103）。
        if player.xian_mo_strategies.contains(&7103) {
            let amount = self.actor(actor_side).core.hp / 10;
            self.modify_actor_max_hp(actor_side, amount);
            self.modify_actor_hp(actor_side, amount, false, false);
        }
        if player.xian_mo_strategies.contains(&7104) {
            self.actor_mut(actor_side).fate.jie_quan_shi += 1;
        }
        if player.xian_mo_strategies.contains(&7105) {
            self.add_actor_negative_status(actor_side, 367, 2);
        }
        if player.xian_mo_strategies.contains(&7109) {
            self.actor_mut(actor_side).fate.wu_you_ling_niang += 3;
        }
        // 23066：OnBattleStart 直写 JiHuoMuLing +1（紧随 7109，先于 7112）。
        if player.xian_mo_strategies.contains(&23066) {
            self.add_direct_element_activation(actor_side, super::Element::Wood);
        }
        if player.xian_mo_strategies.contains(&7112) {
            self.modify_actor_max_hp(actor_side, 20);
        }
        if player.xian_mo_strategies.contains(&7120) {
            self.modify_momentum_limit(actor_side, 6);
        }
        if player.xian_mo_strategies.contains(&7122) {
            self.upgrade_first_battle_deck_card(actor_side, |card| card.name.contains("水灵"));
        }
        // 7123 采样当前 HP，推迟到开局块末（见 1003/1006 注释）。
        if player.xian_mo_strategies.contains(&7127) {
            self.actor_mut(actor_side).fate.ke_yin_jing_lei += 1;
        }
        if player.xian_mo_strategies.contains(&7128) {
            self.upgrade_first_battle_deck_card(actor_side, |card| {
                card.name.contains("星弈") || card.name.contains('卦')
            });
        }
        // 7129: upgrade the first eligible card whose current config description
        // contains 灵气 or 卦象 (not merely cards that grant either resource).
        if player.xian_mo_strategies.contains(&7129) {
            self.upgrade_first_battle_deck_card(actor_side, |card| {
                super::original_config::original_card_desc_has_spirit_or_hexagram(card.id)
            });
        }
        if player.xian_mo_strategies.contains(&7135)
            && player.xian_mo_temp_datas.get("7135").copied().unwrap_or(0) == 0
        {
            self.actor_mut(actor_side)
                .fate
                .resonance_mystic_heart_enter_profound += 1;
            self.actor_mut(target_side)
                .fate
                .resonance_mystic_heart_enter_profound += 1;
        }
        if player.xian_mo_strategies.contains(&7136) {
            self.actor_mut(actor_side).sword.frenzy_sword += 1;
        }
        if player.xian_mo_strategies.contains(&11002) {
            self.modify_hexagram(actor_side, 1);
            for slot in 2..8 {
                if !self.actor(actor_side).astrology.star_slots.contains(&slot) {
                    self.actor_mut(actor_side).astrology.star_slots.push(slot);
                }
            }
        }
        if player.xian_mo_strategies.contains(&11003) {
            // 26125017 new: WuXingMiJuanShengJi(792) += 1 (consumed to upgrade a
            // WuXingDaoMeng card mid-Execute, CardActionBase.cs:1752).
            self.actor_mut(actor_side)
                .fate
                .xian_mo_11003_wuxing_upgrades += 1;
        }
        if player.xian_mo_strategies.contains(&11004) {
            self.apply_physique_amount(actor_side, 3);
        }
        // 15018 采样当前 HP，推迟到开局块末（见 1003/1006 注释）。
    }

    fn apply_verified_xian_mo_battle_start_switches(
        &mut self,
        actor_side: crate::model::PlayerSide,
        player: &crate::fixture::FixturePlayerView<'_>,
        switches: &[(i64, &str)],
    ) {
        let target_side = super::support::opponent_side(actor_side);
        for (strategy, key) in switches.iter().copied() {
            if !player.xian_mo_strategies.contains(&strategy)
                || player.xian_mo_temp_datas.get(key).copied().unwrap_or(0) != 0
            {
                continue;
            }
            for side in [actor_side, target_side] {
                match strategy {
                    1001 => {
                        self.gain_anima(side, 2);
                    }
                    1002 => {
                        self.gain_attack_bonus(side, 1);
                    }
                    1004 => {
                        self.add_actor_negative_status(side, 100, 3);
                    }
                    1005 => self.actor_mut(side).turn.action_again_blocked_turns += 1,
                    1007 => {
                        self.modify_paint_finishing_touch(side, 1);
                    }
                    1008 => self.actor_mut(side).music.chaotic_mind_tune += 2,
                    _ => unreachable!("only implemented XianMo opening switches are passed"),
                }
            }
        }
    }

    fn apply_verified_xian_mo_strategy_1003_battle_start(
        &mut self,
        actor_side: crate::model::PlayerSide,
        player: &crate::fixture::FixturePlayerView<'_>,
    ) {
        if !player.xian_mo_strategies.contains(&1003)
            || player.xian_mo_temp_datas.get("1003").copied().unwrap_or(0) != 0
        {
            return;
        }
        let target_side = super::support::opponent_side(actor_side);
        // XianMoStrategyFunctions.cs IL_02ff samples both before either pair
        // of ModifyMaxHp/ModifyHp calls; the current config is 15 percent.
        let hp = [
            self.actor(actor_side).core.hp,
            self.actor(target_side).core.hp,
        ];
        for (side, starting_hp) in [actor_side, target_side].into_iter().zip(hp) {
            let amount = starting_hp * 15 / 100;
            self.modify_actor_max_hp(side, amount);
            self.modify_actor_hp(side, amount, false, false);
        }
    }

    fn apply_verified_xian_mo_strategy_1006_battle_start(
        &mut self,
        actor_side: crate::model::PlayerSide,
        player: &crate::fixture::FixturePlayerView<'_>,
    ) {
        if !player.xian_mo_strategies.contains(&1006)
            || player.xian_mo_temp_datas.get("1006").copied().unwrap_or(0) != 0
        {
            return;
        }
        let target_side = super::support::opponent_side(actor_side);
        // 原版 XianMoStrategyFunctions.cs:718-723 在 1005 之后、1007 之前执行，
        // 且早于 11004（锻玄秘卷）开局加体魄。此处按原版双向扣减 15% 当前与上限生命。
        let hp = [
            self.actor(actor_side).core.hp,
            self.actor(target_side).core.hp,
        ];
        for (side, starting_hp) in [actor_side, target_side].into_iter().zip(hp) {
            let amount = starting_hp * 15 / 100;
            self.modify_actor_hp(side, -amount, false, false);
            self.modify_actor_max_hp(side, -amount);
        }
    }

    /// 保留在本地 actor 开局块末处理的 HP 采样臂（7103/7123/15018）。
    ///
    /// 原版单 actor 开局顺序是天赋循环 → FateStrategy.OnBattleStart →
    /// XianMo.OnBattleStart（BattleCharacter.cs talent switch → :2825 → :2849）。
    /// 引擎把 XianMo 提前执行并用 stash 模拟 marker 消费顺序，但 HP 采样无法
    /// stash：199 玉瓶（天赋循环 case 199 → ActiveWuXingInName）触发的五行激活
    /// 经 talent 138 扣对手血后才进入 XianMo；1003 已在 XianMo 主体的正确位置
    /// 采样。oracle 锚点：pa_722e26ca pa_29e61b8c
    /// （p1 80-3=77 → 77*15/100=11，原版 88/88）。
    /// 调用点：battle_start 开局块末、stash 归还之后、同一 actor 循环内。
    pub(super) fn apply_verified_xian_mo_late_battle_start_hp_sampling_effects(
        &mut self,
        actor_side: crate::model::PlayerSide,
        player: &crate::fixture::FixturePlayerView<'_>,
    ) {
        if player.xian_mo_strategies.is_empty() {
            return;
        }
        if player.xian_mo_strategies.contains(&7123) {
            if let Some(life) = player.last_round_life {
                if life <= 50 {
                    let amount = self.actor(actor_side).core.hp * 15 / 100;
                    self.modify_actor_max_hp(actor_side, amount);
                    self.modify_actor_hp(actor_side, amount, false, false);
                }
            } else {
                self.missing_card_effect(0, 0, "xianMo strategy 7123 missing original life");
            }
        }
        if player.xian_mo_strategies.contains(&15018) && !player.hand_cards.is_empty() {
            let amount = self.actor(actor_side).core.hp * 12 / 100;
            self.modify_actor_max_hp(actor_side, amount);
            self.modify_actor_hp(actor_side, amount, false, false);
        }
    }

    pub(super) fn apply_verified_xian_mo_after_damage(
        &mut self,
        actor_side: crate::model::PlayerSide,
        target: crate::model::PlayerSide,
    ) {
        let internal = self.actor(actor_side).fate.next_damage_internal_injury;
        if internal > 0 {
            self.actor_mut(actor_side).fate.next_damage_internal_injury = 0;
            self.add_actor_negative_status(target, 100, internal);
        }
        // 仅 DamageType.Attack 到达这里：BattleCharacter.ApplyDamage:11279 对非攻击伤害（Damage/ReflectDamage）
        // 直接 return，11338–11348 的下次攻击内伤不会被它们消耗。先清标记再改 buff，避免连锁重入。
        // （26125017 删除龙威震慑转移：3001 改为开局直接给对手减攻，见 battle-start。）
    }

    pub(super) fn apply_verified_xian_mo_on_play_card(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &crate::model::CardDefinition,
        slot: usize,
        is_temporary: bool,
    ) {
        if super::support::is_pu_tong_gong_ji_base_id(super::support::normalized_base_id(card)) {
            if self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&5004)
            {
                // 26125017: otherParams[0] 10->14 (source reads config).
                self.gain_agility(actor_side, 14);
            }
            let guard = self.actor(actor_side).fate.normal_attack_guard_and_exhaust;
            if guard > 0 {
                self.actor_mut(actor_side)
                    .fate
                    .normal_attack_guard_and_exhaust = 0;
                self.gain_guard(actor_side, guard);
                if !is_temporary {
                    self.actor_mut(actor_side).deck.slots[slot].skipped = true;
                }
            }
            let bonus = self.actor(actor_side).fate.normal_attack_bonus_and_exhaust;
            if bonus > 0 {
                self.actor_mut(actor_side)
                    .fate
                    .normal_attack_bonus_and_exhaust = 0;
                self.gain_attack_bonus(actor_side, bonus);
                if !is_temporary {
                    self.actor_mut(actor_side).deck.slots[slot].skipped = true;
                }
            }
            // 26125017 new 5011 (XianMoStrategyFunctions.cs:1231): after the 5007/5008
            // branches, a 普攻 consumes XiaCiPuGongXiaoHao(793): dst 虚弱(101) +=
            // otherParams[0]=3, 减攻(103) += otherParams[1]=1, and the card is skipped.
            if self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&5011)
                && self.actor(actor_side).fate.xian_mo_5011_pending_pu_gong > 0
            {
                self.actor_mut(actor_side).fate.xian_mo_5011_pending_pu_gong = 0;
                let target = super::support::opponent_side(actor_side);
                self.add_actor_negative_status(target, 101, 3);
                self.add_actor_negative_status(target, 103, 1);
                if !is_temporary {
                    self.actor_mut(actor_side).deck.slots[slot].skipped = true;
                }
            }
        }
        // OnPlayCard follows FateStrategyFunctions.OnPlayCard. Both can
        // consume the shared 剑阵护体 buff, once each, in that source order.
        if self.actor(actor_side).fate.sword_formation_guard > 0
            && super::support::is_sword_formation_card(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).fate.sword_formation_guard -= 1;
            self.gain_defense(actor_side, 5);
            self.gain_guard(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .is_empty()
        {
            return;
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7010)
            && super::support::is_cloud_sword(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.cloud_sea += 1;
        }
        if slot == 7
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&7022)
        {
            self.add_actor_negative_status(super::support::opponent_side(actor_side), 100, 2);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7026)
        {
            if card.name.contains("金灵") {
                self.gain_anima(actor_side, 1);
            }
            if card.name.contains("水灵") {
                self.gain_sharpness(actor_side, 2);
            }
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7052)
            && super::support::is_frenzy_sword_for_actor(self.actor(actor_side), card)
        {
            self.gain_anima(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7037)
        {
            let damage = card.hp_cost.unwrap_or(0).max(0) * 40 / 100;
            if damage > 0 {
                self.apply_damage(actor_side, damage, false, false, false);
            }
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7050)
            && super::support::is_cloud_sword(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.water_month_sword_formation += 1;
        }
        if (is_temporary
            || self
                .actor(actor_side)
                .deck
                .slots
                .get(slot)
                .is_some_and(|s| !s.used))
            && card.name.contains("卦")
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&7057)
        {
            self.add_actor_negative_status(super::support::opponent_side(actor_side), 101, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7060)
            && super::support::normalized_base_id(card) == 381
        {
            self.gain_anima(actor_side, 1);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7089)
            && card.name.contains('掌')
        {
            self.gain_agility(actor_side, 2);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7097)
            && card.name.contains("土灵")
        {
            match self.battle_round {
                Some(round) if round >= 13 => {
                    self.gain_sharpness(actor_side, 2);
                }
                Some(_) => {}
                None => {
                    self.missing_card_effect(
                        card.id,
                        card.id,
                        "xianMo strategy 7097 missing original round",
                    );
                }
            }
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7130)
            && card.id == 19
        {
            self.actor_mut(actor_side).sword.cloud_sea += 5;
            self.gain_cloud_chain(actor_side, 2);
        }
        if self
            .actor(actor_side)
            .identity
            .xian_mo_strategies
            .contains(&7132)
            && super::support::is_sword_formation_card(self.actor(actor_side), card)
        {
            self.actor_mut(actor_side).sword.cloud_sea += 1;
        }
        if !is_temporary
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&5001)
            && self
                .actor(actor_side)
                .identity
                .xian_mo_temp_datas
                .get("5001")
                .copied()
                .unwrap_or(0)
                == slot as i64
            && super::original_config::original_card_realm_level(card.id) == Some(1)
            && super::original_config::original_card_is_hidden(card.id) == Some(false)
        {
            self.modify_extra_actions(actor_side, 1);
        }
        // OnPlayCard source order, once on the selected physical slot's first
        // use. Repeated ExecuteEffect sees hadUsed=true; temporary cards are excluded.
        if !is_temporary
            && self
                .actor(actor_side)
                .deck
                .slots
                .get(slot)
                .is_some_and(|s| !s.used)
        {
            self.apply_verified_xian_mo_first_slot_enchantments(actor_side, card, slot);
        }
        // OnPlayCard:1478-1480, exact ID 19, not all normal-attack aliases.
        if card.id == 19
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&23045)
        {
            self.gain_anima(actor_side, 1);
        }
        if super::support::normalized_base_id(card) == 22
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&23079)
        {
            match self.battle_round {
                Some(round) if round >= 3 => {
                    self.gain_temporary_guard(actor_side, 3);
                }
                Some(_) => {}
                None => {
                    self.missing_card_effect(
                        card.id,
                        22,
                        "xianMo strategy 23079 missing original round",
                    );
                }
            }
        }
    }

    /// 伏魔 13017 的首次用牌附魔，单独先于星弈连执行。
    /// CardActionBase.OnBeforeExecuted：XianMo OnPlayCard（:3300，
    /// 先加 1 内伤再数负面加防）先于星弈连（:3676-3687，星位
    /// +1 星力，可经 23069 转内伤）。调用方保证首用/非临时。
    pub(super) fn apply_fumo_13017_before_star_hooks(
        &mut self,
        actor_side: crate::model::PlayerSide,
        slot: usize,
    ) {
        let actor = self.actor(actor_side);
        if !actor.identity.xian_mo_strategies.contains(&13017)
            || actor
                .identity
                .xian_mo_temp_datas
                .get("13017")
                .copied()
                .unwrap_or(0)
                != slot as i64
        {
            return;
        }
        let target = super::support::opponent_side(actor_side);
        self.add_actor_negative_status(target, 100, 1);
        let amount = self.negative_status_stack_count(target);
        if amount > 0 {
            self.gain_defense(actor_side, amount);
        }
    }

    fn apply_verified_xian_mo_first_slot_enchantments(
        &mut self,
        actor_side: crate::model::PlayerSide,
        card: &crate::model::CardDefinition,
        slot: usize,
    ) {
        let target = super::support::opponent_side(actor_side);
        for (id, key) in [
            (13001, "13001"),
            (13002, "13002"),
            (13004, "13004"),
            (13005, "13005"),
            (13007, "13007"),
            (13008, "13008"),
            (13009, "13009"),
            (13010, "13010"),
            (13011, "13011"),
            (13012, "13012"),
            (13013, "13013"),
            (13014, "13014"),
            (13015, "13015"),
            (13016, "13016"),
            // 13017 不在此循环：CardActionBase.OnBeforeExecuted 里
            // XianMo OnPlayCard（:3300）先于星弈连（:3676），13017
            // 必须在星弈连转内伤之前数负面（见
            // apply_fumo_13017_before_star_hooks）。
            (13018, "13018"),
            (13019, "13019"),
        ] {
            let actor = self.actor(actor_side);
            if !actor.identity.xian_mo_strategies.contains(&id)
                || actor
                    .identity
                    .xian_mo_temp_datas
                    .get(key)
                    .copied()
                    .unwrap_or(0)
                    != slot as i64
            {
                continue;
            }
            // Parameters from current XianMoStrategyConfig; all resource
            // changes use the same kernels as their Card_* counterparts.
            match id {
                13001 => {
                    self.actor_mut(actor_side).fate.next_damage_internal_injury += 3;
                }
                13002 => {
                    // OnPlayCard 13002（otherParams [2,4]）：4 次 GetNextParam，非 -1 则该 BuffType 减 1 层并计数；
                    // 未凑够 4 次时每缺一次回复 2 生命。参数是 HF 记录的 battleParams 队列，缺失按 missing-decision 拒绝。
                    let mut cleared = 0;
                    for _ in 0..4 {
                        let selected = self.consume_card_param(actor_side);
                        if selected == -1 {
                            continue;
                        }
                        if !matches!(selected, 100..=105 | 367 | 393) {
                            self.missing_card_effect(
                                0,
                                0,
                                "xianMo strategy 13002 selected an unmodelled buff type",
                            );
                            return;
                        }
                        self.remove_actor_negative_status(actor_side, selected, 1);
                        cleared += 1;
                    }
                    if cleared < 4 {
                        self.modify_actor_hp(actor_side, (4 - cleared) * 2, false, false);
                    }
                }
                13004 => {
                    let amount = actor.turn.lose_hp_count * 18 / 100;
                    if amount > 0 {
                        self.modify_actor_max_hp(actor_side, amount);
                        self.modify_actor_hp(actor_side, amount, false, false);
                    }
                }
                13005 => {
                    // 26125017: otherParams[0] 4->5.
                    let amount = 5 * super::support::wu_xing_count_in_deck(actor);
                    self.gain_defense(actor_side, amount);
                }
                13008 => {
                    self.gain_temporary_guard(actor_side, 3);
                }
                13007 => {
                    self.actor_mut(target).fate.next_card_downgrade += 1;
                }
                13009 => {
                    // 26125017: otherParams [5,2]->[6,2].
                    self.gain_defense(actor_side, 6);
                    self.actor_mut(actor_side).sword.water_month_sword_formation += 2;
                }
                13010 => {
                    self.add_following_star_slots(actor_side, slot, 3);
                }
                13011 => {
                    if card.name.contains("金灵") {
                        self.gain_sharpness(actor_side, 4);
                    }
                    if card.name.contains("水灵") || card.name.contains("木灵") {
                        self.gain_anima(actor_side, 2);
                    }
                    if card.name.contains("火灵") || card.name.contains("土灵") {
                        self.gain_defense(actor_side, 8);
                    }
                }
                13013 => {
                    self.modify_hexagram(actor_side, 1);
                    let amount = self.actor(actor_side).astrology.hexagram;
                    self.modify_actor_max_hp(actor_side, amount);
                    self.modify_actor_hp(actor_side, amount, false, false);
                }
                13012 => {
                    self.modify_momentum_limit(actor_side, 1);
                    self.actor_mut(actor_side).beng.momentum_before_attack += 3;
                }
                13014 => {
                    self.apply_physique_amount(actor_side, 3);
                    let amount = self.actor(actor_side).core.physique / 6;
                    self.gain_defense(actor_side, amount);
                }
                13015 => {
                    self.modify_actor_hp(target, -6, false, false);
                    self.modify_actor_max_hp(target, -6);
                    self.modify_dream_mirage_value(
                        target,
                        super::cards_dream_mirage::DreamMirageValue::ConsumeNextCard,
                        1,
                    );
                }
                13016 => {
                    self.modify_sword_intent(actor_side, 2);
                    let amount = self.actor(actor_side).sword.sword_intent / 2;
                    if amount > 0 {
                        self.apply_damage(actor_side, amount, false, false, false);
                    }
                }
                13018 => {
                    if card.name.contains("水灵") {
                        self.modify_dream_mirage_value(
                            actor_side,
                            super::cards_dream_mirage::DreamMirageValue::ExtraWaterMomentumTurnEnd,
                            1,
                        );
                    }
                    if card.name.contains("木灵") && self.actor(actor_side).core.attack_bonus > 0
                    {
                        // 26125017: transfer min(JiaGong, 2) instead of fixed 1.
                        let transfer = self.actor(actor_side).core.attack_bonus.min(2);
                        self.actor_mut(actor_side).core.attack_bonus -= transfer;
                        self.actor_mut(actor_side).elements.wood_thorn += transfer;
                    }
                }
                13019 => {
                    let amount = self.negative_status_stack_count(actor_side) + 6;
                    self.modify_actor_hp(actor_side, amount, false, false);
                }
                _ => unreachable!(),
            }
        }
    }
}

#[derive(Deserialize)]
struct XianMoBattleStrategies {
    #[serde(rename = "schemaVersion")]
    schema_version: u32,
    strategies: Vec<XianMoBattleStrategy>,
}

#[derive(Deserialize)]
struct XianMoBattleStrategy {
    id: i64,
}

static BATTLE_STRATEGY_IDS: OnceLock<BTreeSet<i64>> = OnceLock::new();

pub(super) fn battle_relevant_xian_mo_strategy_ids() -> &'static BTreeSet<i64> {
    BATTLE_STRATEGY_IDS.get_or_init(|| {
        let data: XianMoBattleStrategies =
            serde_json::from_str(include_str!("../../data/xian-mo-battle-strategies.json"))
                .expect("xian-mo battle strategy list parses");
        assert_eq!(data.schema_version, 1, "xian-mo battle strategy schema");
        data.strategies
            .into_iter()
            .map(|strategy| strategy.id)
            // Config.isBattleEffect also controls UI icons. These have no
            // current client battle call site: speed/initial attributes and
            // pending effects are supplied by parsed initial state; crafting,
            // hand/deck/cultivation rewards and card-pool edits are outside it.
            // 26001 只写/减 Hidden 计数（HunDunQianYing 788/789），反编译全部源码里没有任何读取方，对战斗无影响。
            // 2011 唯一调用点是 BattleExecuter.CalLifeDamage（战后命元 -otherParams[0]），
            // 不进 winner/actorTurn/hpDelta 三元：已验证无战斗影响（同址 fate 21 亦不建模）。
            .filter(|id| {
                ![
                    2011, 2015, 3005, 3013, 7011, 7012, 7038, 7062, 7114, 7121, 7134, 26001,
                ]
                .contains(id)
            })
            .collect()
    })
}

/// 返回 fixture 中第一个影响战斗但未实现的仙魔策略。
pub(super) fn unimplemented_xian_mo_strategy(
    fixture: &crate::fixture::BattleFixtureView<'_>,
) -> Option<BattleError> {
    let relevant = battle_relevant_xian_mo_strategy_ids();
    [&fixture.players.p1, &fixture.players.p2]
        .into_iter()
        .flat_map(|player| player.xian_mo_strategies.iter().copied())
        .find(|id| relevant.contains(id) && !IMPLEMENTED_XIAN_MO_STRATEGY_IDS.contains(id))
        .map(|id| BattleError::MissingRule {
            card_id: 0,
            base_id: 0,
            reason: format!("xianMo strategy {id} affects battle but is not implemented"),
            turn: 0,
        })
}
