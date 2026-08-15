use super::deck_start::apply_deck_start_talent_effects;
use super::original_config::{
    can_upgrade_original_battle_deck_card, known_card_definition, original_card_definition,
    original_card_desc_contains_action_again, original_config_rarity, upgrade_original_card,
};
use super::player::prepare_fixture_deck_card;
use super::support::{
    fate_strategy_131_elements, is_cloud_sword, normalized_base_id, opponent_side, other_param,
    permanent_physique_key, wu_xing_count_in_deck,
};
use super::ReplayState;
use crate::fixture::{BattleFixture, FixturePlayer};
use crate::model::{CardDefinition, PlayerSide};

fn dark_heart_mark_from_talent(talent: i64) -> Option<(i64, i64)> {
    match talent {
        150 => Some((1, 1)),
        10_150 => Some((2, 2)),
        20_150 => Some((2, 2)),
        30_150 => Some((3, 3)),
        _ => None,
    }
}

fn abundant_momentum_from_talent(talent: i64) -> Option<(i64, i64)> {
    match talent {
        145 => Some((1, 0)),    // 气势充沛
        10_145 => Some((1, 1)), // 气势充沛
        20_145 => Some((1, 2)), // 气势充沛
        30_145 => Some((1, 3)), // 气势充沛
        _ => None,
    }
}

impl ReplayState {
    pub(super) fn apply_battle_start_opening_effects(&mut self, fixture: &BattleFixture) {
        let first_side = fixture.first_player_side;
        for actor_side in [first_side, opponent_side(first_side)] {
            let player_fixture = match actor_side {
                PlayerSide::P1 => &fixture.players.p1,
                PlayerSide::P2 => &fixture.players.p2,
            };
            // 牌组升级（虎体 talent 125 / 孤虚金书 talent 198 + FS338）在
            // 该 actor 自己的 OnBattleStarted 天赋循环里执行，早于本 actor
            // 的 [开局] 效果；而次位 actor 的整个 OnBattleStarted 晚于首位
            // actor（BattleExecuter.cs:1720-1721 await firstCharacter
            // .OnBattleStarted() 之后才启动 secondCharacter）。因此首位
            // actor 的 [开局] 降级（369 梦•厄劫缠身 / 11000018 厄劫缠身）
            // 先作用于次位 actor 的牌组，被降级的牌随后被次位的虎体/孤虚
            // 金书再次升回。oracle 锚点：mirror-32299000
            // c76c1756b08baf69/round-14 cp1（5010014 vs 引擎 5000014：
            // 虎体把 5000014 升回 5010014）、dbe66f8ca72d8759/round-16
            // cp10（20215 vs 引擎 10215：厄劫缠身先降 215，孤虚金书再逐级
            // 升到 20215 停在 noUpgrade）。
            self.apply_actor_deck_start_effects(actor_side, player_fixture, fixture);
            // BattleCharacter.OnBattleStarted applies each actor's opening
            // effects as one ordered boundary.  In particular, the previous
            // actor's opening card may change this actor's HP before Talent
            // 179/FateStrategy 161 adds 冥.  Keep this actor-local mutation in
            // the loop instead of healing both players in a prelude.
            self.apply_battle_start_meditation_healing(player_fixture, actor_side);
            // These two talent families must follow the fixture's original
            // talent-list order. 储纳灵气 can therefore trigger 风灵锻躯
            // before a later 筋骨健壮 samples current physique. Grouping all
            // healthy-bones ranks ahead of all stored-anima ranks loses one HP
            // at the threshold (HF 6152/aa21/bf90).
            for talent in &player_fixture.talents {
                match talent {
                    13 | 10_013 | 20_013 | 30_013 => {
                        let amount = match talent {
                            13 | 10_013 => 1,
                            20_013 => 2,
                            30_013 => 3,
                            _ => unreachable!(),
                        };
                        self.gain_anima(actor_side, amount);
                    }
                    176 | 10_176 | 20_176 | 30_176 => {
                        let divisor = match talent {
                            176 => 10,
                            10_176 => 9,
                            20_176 => 7,
                            30_176 => 6,
                            _ => unreachable!(),
                        };
                        let gain = self.actor(actor_side).core.physique / divisor;
                        if gain > 0 {
                            self.modify_actor_hp(actor_side, gain, false, false);
                        }
                    }
                    _ => {}
                }
            }
            // Innate marks are part of this actor's OnBattleStarted opening,
            // not a two-sided prelude. This preserves the original ordering:
            // the first actor completes its opening before the opponent's
            // innate damage can consume its guard/defense.
            self.apply_innate_mark_element_activation(player_fixture, actor_side);
            // 桃枝如意是该 actor 的 OnBattleStarted 天赋效果；首位和次位
            // 都必须在各自的开场边界内获得，不能在双方 loop 外预置。
            if player_fixture.talents.contains(&134) {
                self.gain_guard(actor_side, 1);
            }
            // 护身法宝由该 actor 的开场 FateStrategy/永久 Buff 阶段发放。
            // 因而只有已经进入 OnBattleStarted 的 actor 才能用它承受后续
            // 金梭兰；首位 actor 的金梭兰先于次位 actor 的护身法宝。
            let protective_talisman = player_fixture
                .permanent_buff_temp_datas
                .get("10047")
                .copied()
                .unwrap_or(0)
                .max(0);
            if protective_talisman > 0 {
                self.gain_guard(actor_side, protective_talisman);
            }
            // Talent_184 converts the battle-start physique from Talent_183
            // (doubled by FateStrategy_166) into defense at this actor's own
            // OnBattleStarted boundary. It must not be constructor state:
            // the first actor's opening damage resolves before the opponent
            // reaches this boundary.
            if player_fixture.talents.contains(&184) {
                let talent_physique = if player_fixture.talents.contains(&183) {
                    if player_fixture.fate_strategies.contains(&166) {
                        2
                    } else {
                        1
                    }
                } else {
                    0
                };
                if talent_physique > 0 {
                    self.gain_defense(actor_side, talent_physique.min(5));
                }
            }
            // Talent_183 的 OnBattleStarted ModifyTiPo 仍在 constructor 已写入
            // 体魄之后补结算：新增体魄跨过 10024 上限的部分按原版 ModifyHp
            // 回血。只补相对 permanent 10023 的增量，避免把 constructor 的
            // maxHp/physique 或 Talent_184 的 defense 再算一遍。
            if player_fixture.talents.contains(&183) {
                let permanent_physique = player_fixture
                    .permanent_buff_temp_datas
                    .get(permanent_physique_key())
                    .copied()
                    .unwrap_or(0);
                let talent_physique = if player_fixture.fate_strategies.contains(&166) {
                    2
                } else {
                    1
                };
                let limit = self.actor(actor_side).core.physique_limit;
                let before_overflow = (permanent_physique - limit).max(0);
                let after_overflow = (permanent_physique + talent_physique - limit).max(0);
                let overflow_healing = after_overflow - before_overflow;
                if overflow_healing > 0 {
                    self.modify_actor_hp(actor_side, overflow_healing, false, false);
                }
            }
            if player_fixture.talents.contains(&183)
                && player_fixture.fate_strategies.contains(&163)
            {
                self.actor_mut(actor_side).fate.chan_xin_ju_ling_triggered = 1;
                self.gain_anima(actor_side, 1);
            }
            if player_fixture.talents.contains(&3) {
                self.gain_defense(actor_side, 8);
            }
            if player_fixture.talents.contains(&26) {
                self.gain_guard(actor_side, 1);
            }
            if player_fixture.talents.contains(&33)
                && !self.actor(actor_side).astrology.star_slots.contains(&6)
            {
                // Talent_33 calls AddXingWei(6); the original grid index is zero-based.
                self.actor_mut(actor_side).astrology.star_slots.push(6);
            }
            // 原版 BattleCharacter.cs:2411-2415：牌组里每有一张 1030076 / 1040076
            // 梦•狂剑零式，就给一层 ShengJiXiaCiKuangJian（671）。低阶的 1000076 /
            // 1010076 / 1020076 不带这条被动，不能按 baseId 归并。
            let dream_frenzy_sword_zero = self
                .actor(actor_side)
                .deck
                .slots
                .iter()
                .filter(|slot| matches!(slot.card.id, 1_030_076 | 1_040_076))
                .count() as i64;
            if dream_frenzy_sword_zero > 0 {
                self.actor_mut(actor_side).sword.upgrade_next_frenzy_sword +=
                    dream_frenzy_sword_zero;
            }
            self.apply_battle_start_divination(actor_side, player_fixture);
            if player_fixture.talents.contains(&171) {
                // Talent 171（搏命之勇；BattleCharacter.cs:1180, 1738-1740）
                // uses the normal ModifyBuffValue path for JiaGong and WaiShang.
                // Keep 外伤 out of ReplayPlayer construction: opposing 星蚀
                // (30103) is already armed at this actor's opening boundary and
                // must add its one-shot bonus through add_actor_negative_status.
                self.gain_attack_bonus(actor_side, 1);
                self.add_actor_negative_status(actor_side, 105, 1);
            }
            // 冥心烙印（talent 150 系）属于该 actor 的 OnBattleStarted 天赋
            // switch，必须先于 FateStrategyFunctions.OnBattleStart（原版
            // BattleCharacter.OnBattleStarted 天赋循环在命运函数之前）——
            // 其开局内伤触发 修玄不泯(177) 回血，会改变后续 fate 27
            // （生命+X%）采样的当前 HP。oracle 锚点：mirror-32299000
            // e58d484ea2ab6e3d/round-13 cp0（p2 maxHp 131 = 106+12 体魄
            // +13（27: 111×12/100），引擎原 130 = 106+12+12（27 误采样
            // 177 回血前的 106）；hp 124 vs 123）。
            self.apply_dark_heart_mark_talent(actor_side, player_fixture);
            if player_fixture.fate_strategies.contains(&342) {
                self.actor_mut(opponent_side(actor_side))
                    .status
                    .flame_heart_urging += 3;
            }
            if player_fixture.fate_strategies.contains(&379) {
                // 云之剑气（FateStrategyFunctions.cs:499）：开局 JianQi +=
                // FateStrategyConfig(379).otherParams[0] (= 1)。625 是隐藏资源，
                // 但它同时决定灵气不足替代与云剑后的追加伤害。
                self.actor_mut(actor_side).sword.sword_energy += 1;
            }
            if player_fixture.fate_strategies.contains(&436) {
                // 七星借命（FateStrategyFunctions.cs:587-589）：开局发放
                // QiXingJieMing 标记（otherParams[0]=3），战斗中首次生命 ≤ 0
                // 时由 death_winner 的复活检查按（卦象+星力）× 标记值转换
                // 为生命及上限（BattleExecuter.CharacterResurrectionCheckAsync）。
                self.actor_mut(actor_side).fate.qi_xing_jie_ming = 3;
            }
            if player_fixture.fate_strategies.contains(&430) {
                // 截拳式（FateStrategyFunctions.cs:583-585，OnBattleStart 内）：
                // JieQuanShi(770) += 1；战斗中每次造成实际攻击伤害时消耗
                // 1 层，拳架势对目标 +1 减攻，否则 +1 虚弱
                // （BattleCharacter.cs:10981-10990，combat_core.rs 攻击段钩子）。
                self.actor_mut(actor_side).fate.jie_quan_shi = 1;
            }
            if player_fixture.fate_strategies.contains(&431) {
                // 风灵锻躯（FateStrategyFunctions.cs:479-481，OnBattleStart 内）：
                // TianYanFengLingDuanQu(771) += otherParams[0]=5；随后每次
                // 加身法时消耗 1 层换 1 体魄（BattleCharacter.cs:8733-8737，
                // resources.rs modify_agility_inner 钩子）。
                self.actor_mut(actor_side).fate.tian_yan_feng_ling_duan_qu = 5;
            }
            if player_fixture.fate_strategies.contains(&407) {
                // 天衍-无忧灵酿（FateStrategyFunctions.cs:557-560，OnBattleStart）：
                // WuYouLingNiang(767) += otherParams[1]=3。每次正向
                // ModifyAnima 消耗一层并由共享入口按 otherParams[0]=4 增加
                // 生命上限及生命（BattleCharacter.cs:9516-9521）。
                self.actor_mut(actor_side).fate.wu_you_ling_niang = 3;
            }
            if player_fixture.fate_strategies.contains(&395) {
                // FateStrategyFunctions.cs:533-536：无尽卦衍开局发放
                // WuJingGuaYan(otherParams[0]=3) 标记；正向增加卦象时由
                // gain_hexagram 按 BattleCharacter.cs:8553-8557 消耗。
                self.actor_mut(actor_side).fate.wu_jing_gua_yan = 3;
            }
            if player_fixture.fate_strategies.contains(&402)
                && player_fixture
                    .fate_strategy_temp_datas
                    .get("402")
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                // 星缘（FateStrategyFunctions.cs:549-552，OnBattleStart）：
                // XingYuan(765) += otherParams[0]=3；IsSwitchActive 的
                // tempData 非零状态关闭该命运，和原版共享开关一致。
                self.actor_mut(actor_side).fate.xing_yuan = 3;
            }
            if player_fixture.fate_strategies.contains(&437) {
                // 搏命之勇（FateStrategyFunctions.cs:591-594，OnBattleStart 内）：
                // otherParams=[1,2]，加攻 +1 / 内伤 +2；IsSwitchActive（:842-851）
                // 在 fateStrategyData.tempDatas[437] == 0 时视为开启，否则禁用。
                // 走常规 ModifyBuffValue 通道（add_actor_negative_status），以保留
                // 辟邪减免 / 415 疯魔架势体魄等钩子与原版一致。
                let disabled = player_fixture
                    .fate_strategy_temp_datas
                    .get("437")
                    .copied()
                    .unwrap_or(0)
                    != 0;
                if !disabled {
                    self.gain_attack_bonus(actor_side, 1);
                    self.add_actor_negative_status(actor_side, 100, 2);
                }
            }
            if player_fixture.fate_strategies.contains(&416) {
                // 促局飞袭（FateStrategyFunctions.cs:571-573，OnBattleStart 内）：
                // CuJuFeiXi(768) += otherParams[0]=5；战斗中使用名字含「火灵」
                // 的牌（含 temp 执行）后，OnAfterExecuted 消耗全部层数并对
                // 对方追加一次 Attack(buffValue)（CardActionBase.cs:3996-4002，
                // flow_card_effect.rs 钩子）。oracle 锚点：mirror-32219000
                // cae463212f8c4c43/round-15 t5u1（temp 烈燎原 768:5→消耗，
                // 第 4 攻击段 8 = 5+加攻3）、round-12 t7u1（temp 赤焰
                // 第 4 段 8 = 5+加攻4-减攻1）。
                self.actor_mut(actor_side).fate.cu_ju_fei_xi = 5;
            }
            if player_fixture.fate_strategies.contains(&397) {
                // 灯灵星耀（FateStrategyFunctions.cs:543-546，OnBattleStart 内）：
                // 星力 +1；若自身 lastRoundExp ≥ 对方 lastRoundExp +
                // otherParams[0]=5 再 +1。oracle 锚点：mirror-32219000-human-01
                // 64e07edecaeef655/round-12 cp0（p2 星力 4 = 天元心法 2 +
                // 397×2，exp 61 ≥ 46+5）、7035f55163951763/round-14 cp0
                // （p1 星力 4，exp 72 ≥ 60+5）。
                self.modify_star_power(actor_side, 1);
                let opponent_fixture = match opponent_side(actor_side) {
                    PlayerSide::P1 => &fixture.players.p1,
                    PlayerSide::P2 => &fixture.players.p2,
                };
                if player_fixture.last_round_exp >= opponent_fixture.last_round_exp + 5 {
                    self.modify_star_power(actor_side, 1);
                }
            }
            if player_fixture.fate_strategies.contains(&396) {
                // 惊雷破敌（FateStrategyFunctions.cs:537-539）：
                // KeYinJingLei(574) += 1。CardActionBase.OnBeforeExecuted
                // 对原版雷牌逐张消费该标记并写入 SuiFang/WaiShang。
                self.actor_mut(actor_side).fate.ke_yin_jing_lei = 1;
            }
            if player_fixture.fate_strategies.contains(&140) {
                // FateStrategy 140（天衍-猛虎之躯，FateStrategyFunctions.cs:239-247）：
                // ModifyMaxHp(hp/10) + ModifyHp(hp/10)，hp 为执行时的当前生命。
                // 原版先手方开局伤害（11_000_001 天谕系扣血）先于本方
                // OnBattleStart，因此这里必须晚于对手开局结算（oracle 锚点：
                // mirror-32299000 608126353bbde8d4/round-10 cp0 p2.hp 97 vs 98
                // —— 100-3=97 → 97/10=9，引擎构造期 100/10=10 多 1）。
                let hp_gain = self.actor(actor_side).core.hp / 10;
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
            }
            if player_fixture.fate_strategies.contains(&27) && !player_fixture.hand_cards.is_empty()
            {
                // FateStrategyFunctions.OnBattleStart（decompiled build-24610558）：
                // IL_00f1 先执行 Fate 140（ModifyMaxHp(hp/10) + ModifyHp(hp/10)），
                // IL_01c1 才执行 Fate 27 —— `num3 = src.battleTempData.hp *
                // otherParams[0] / 100` 采样的已是 140 增益后的当前 HP。
                // Fate 140 在 ReplayPlayer 构造期预置，故此处直接读 core.hp
                // （含 140 增益），不再回减。oracle 锚点：mirror-32219000
                // 93c5c8aa3e3f1cf6/round-10 cp0（p2 107=62+45，fates
                // [140,9,27]）普通攻击后 hp 128 → 开局 hp 131 =
                // 107 + 10（140）+ 14（27: 117×12/100）；引擎原 129
                // = 107 + 10 + 12（27 误采样 140 前 107×12/100=12）。
                let hp_at_strategy = self.actor(actor_side).core.hp;
                let hp_gain = hp_at_strategy * 12 / 100;
                if hp_gain > 0 {
                    self.modify_actor_max_hp(actor_side, hp_gain);
                    self.modify_actor_hp(actor_side, hp_gain, false, false);
                }
            }
            // 徐如林's opening max-HP transaction precedes 雁栖. It must not
            // consume the once-per-battle YanQi charge; a later 乘雁而行 does.
            // HF b1bb round-10 retains buff 744 through checkpoint 7 and
            // consumes it only when 犀牛望月 first raises max HP.
            if player_fixture.fate_strategies.contains(&326) {
                self.actor_mut(actor_side).fate.yan_qi += 1;
            }
            if player_fixture.fate_strategies.contains(&340) {
                self.modify_actor_max_hp(actor_side, 20);
            }
            // FateStrategy 161（天衍-入冥）在命运函数链中位于 FS 27 之后
            // （FateStrategyFunctions.cs IL_088b < IL_0938/IL_09d8）：先让
            // FS 27 按当前 HP 采样，再补 161 的冥 +2 回血（含 415 被动
            // ModifyTiPo(+2)）。
            self.apply_battle_start_fate_strategy_161_meditation(player_fixture, actor_side);
            let return_origin_grass = player_fixture
                .permanent_buff_temp_datas
                .get("10008")
                .copied()
                .unwrap_or(0)
                .max(0);
            if return_origin_grass > 0 {
                self.modify_actor_max_hp(actor_side, return_origin_grass);
                self.modify_actor_hp(actor_side, return_origin_grass, false, false);
            }
            // BuffType.JinSuiCao = 10009（当前 build localized as 金梭兰）是
            // owner OnBattleStarted 的永久 Buff 效果，紧随该 owner 的 10008
            // 归元草之后对 opponent 造成伤害。不能折叠成双方预结算。
            let golden_shuttle_orchid = player_fixture
                .permanent_buff_temp_datas
                .get("10009")
                .copied()
                .unwrap_or(0)
                .max(0);
            if golden_shuttle_orchid > 0 {
                // 原版 OnBattleStarted 的 10009 分支走
                // ApplyDamage(DamageType.ReflectDamage, skipWoundCheck: true)
                // （BattleCharacter.cs:2933-2936），不是平扣：目标当前防御
                // 会吸收并消耗（BattleCharacter.cs:10756-10767），铁骨对
                // ReflectDamage 同样减伤。oracle 锚点：mirror-32219000
                // d358c0486a21ebb3/round-04、round-15（受伤方 183+184
                // 开局 1 防，原版少扣 1）。
                self.apply_damage_to(
                    actor_side,
                    opponent_side(actor_side),
                    golden_shuttle_orchid,
                    false,
                    false,
                    false,
                );
            }
            let strength_grass = player_fixture
                .permanent_buff_temp_datas
                .get("10011")
                .copied()
                .unwrap_or(0)
                .max(0);
            if strength_grass > 0 {
                // BattleCharacter.OnBattleStarted applies 神力草 through
                // ModifyBuffValue(JiaGong), so fate 148 and other gain hooks
                // must observe it instead of receiving pre-seeded JiaGong.
                self.gain_attack_bonus(actor_side, strength_grass);
            }
            if player_fixture.fate_strategies.contains(&384) {
                // 天衍-兴云布雨（FateStrategyFunctions.cs:503-531）：上轮手牌与
                // usedCards 中每 2 张云剑加 1 灵气，最多 3；CardFactory 配置经
                // original_card_definition 恢复后沿用 IsYunJian 的完整判定链。
                let yun_jian_count = player_fixture
                    .hand_cards
                    .iter()
                    .chain(player_fixture.last_round_used_card_base_ids.iter())
                    .filter_map(|card_id| original_card_definition(*card_id))
                    .filter(|card| is_cloud_sword(self.actor(actor_side), card))
                    .count() as i64;
                let cap = if player_fixture.talents.contains(&222) {
                    5
                } else {
                    3
                };
                let anima_gain = (yun_jian_count / 2).min(cap);
                if anima_gain > 0 {
                    self.gain_anima(actor_side, anima_gain);
                }
            }
            // 风灵展翼（FateStrategy 336）属于 FateStrategyFunctions 的
            // OnBattleStart 段，先于 TriggerOpening 开局牌。御风飞闪开局牌
            // 11000009 的 +灵气必须能消耗该标记并立刻获得 5 身法。
            if player_fixture.fate_strategies.contains(&336) {
                self.actor_mut(actor_side).fate.feng_ling_zhan_yi += 5;
            }
            let active_slot_count = player_fixture.active_slot_count;
            for (slot_index, card) in player_fixture
                .cards
                .iter()
                .take(active_slot_count)
                .enumerate()
            {
                let base_id = normalized_base_id(card);
                if !Self::card_has_opening_effect(base_id) {
                    continue;
                }
                let previous_card_execution = self.begin_card_execution(actor_side, card.id);
                self.apply_dream_mirage_battle_start_opening(actor_side, card, slot_index, base_id);
                self.apply_mirage_ronghui_battle_start_opening(actor_side, base_id);
                self.apply_ronghui_battle_start_opening(actor_side, card, slot_index);
                self.apply_synthetic_full_scope_candidate_opening(actor_side, base_id);
                match base_id {
                    11_000_001 => {
                        let damage = other_param(card, 1);
                        if damage > 0 {
                            self.modify_target_hp(actor_side, -damage);
                        }
                    }
                    11_000_005 => {
                        // 吉运初显必须走正常生命变更链，以保留 Talent64 等钩子与回放账本。
                        let value = other_param(card, 1).max(0);
                        if value > 0 {
                            self.modify_actor_max_hp(actor_side, value);
                            self.modify_actor_hp(actor_side, value, false, false);
                        }
                    }
                    11_000_009 => {
                        self.gain_anima(actor_side, other_param(card, 0).max(0));
                    }
                    11_000_013 => {
                        // BattleCharacter.TriggerOpening 读取牌组格位**当前**卡牌
                        // （GetBattleDeckIdList()[grid]）的 otherParams[2]；先手方
                        // 的厄劫缠身（case 11000018）可能已在本方开场结算前把这张
                        // 万事如意降级（11010013 → 11000013，参数 [6,4,2] → [6,3,1]），
                        // 因此辟邪层数必须取自牌组格位而非 fixture 卡（8f0ba353b4c1a831）。
                        let stacks = self
                            .actor(actor_side)
                            .deck
                            .slots
                            .get(slot_index)
                            .map(|slot| other_param(&slot.card, 2).max(0))
                            .unwrap_or_else(|| other_param(card, 2).max(0));
                        self.actor_mut(actor_side).fate.exorcism += stacks;
                    }
                    11_000_014 => {
                        self.gain_defense(actor_side, other_param(card, 2).max(0));
                    }
                    11_000_018 => {
                        let target_side = opponent_side(actor_side);
                        let target_slot_index = slot_index;
                        if let Some(target_slot) =
                            self.actor(target_side).deck.slots.get(target_slot_index)
                        {
                            let target_card = target_slot.card.clone();
                            // 原版 TriggerOpening case 11000018（BattleCharacter.cs:
                            // 11132-11147）读 cardItem2.cardConfig.rarity（配置值，
                            // 无 rarity 字段 = 0），不是 id 档位。梦牌/隐藏牌
                            // （如 7040074 梦•金灵阵 noUpgrade）配置 rarity=0，
                            // 原版不降级而是造成 otherParams[1] ReflectDamage。
                            // oracle 锚点：hf-latest-32308000-16f9c778
                            // fc0b428fdaf407b3/round-12 cp0 p2.hp 114（引擎 120：
                            // 7040074 被 id 档位误降级，漏 6 伤）、
                            // 8df7651618b3af10/round-13 cp0 p1.hp 118（引擎 124）。
                            if original_config_rarity(target_card.id) >= 1 && target_card.id != 19 {
                                let lower_id = target_card.id - 10_000;
                                if let Some(lowered) =
                                    known_card_definition(&fixture.catalog_cards, lower_id)
                                {
                                    self.actor_mut(target_side).deck.slots[target_slot_index]
                                        .card = lowered;
                                }
                            } else {
                                // 伤害数值取当前牌组格位卡（可能已被对方
                                // 先行的厄劫缠身降级），原版 TriggerOpening 读
                                // cardItem2 = 当前格位卡，不是 fixture 原卡
                                // （oracle 锚点：d11b6adfe79418e2/round-17
                                // cp0 p2.hp 96：11020018 被对方 11000018 先
                                // 降级为 11010018 后，伤害 9 = 11010018 的
                                // otherParams[1]，不是 fixture 原卡 12）。
                                let current_card = self
                                    .actor(actor_side)
                                    .deck
                                    .slots
                                    .get(slot_index)
                                    .map(|slot| slot.card.clone());
                                let damage = current_card
                                    .as_ref()
                                    .map(|slot_card| other_param(slot_card, 1))
                                    .unwrap_or_else(|| other_param(card, 1))
                                    .max(0);
                                if damage > 0 {
                                    self.apply_damage(actor_side, damage, false, false, false);
                                }
                            }
                        }
                    }
                    11_000_022 => {
                        // 察体 [开局]：下 otherParams[1] 次攻击附加[碎防]
                        // （BattleCharacter.TriggerOpening case 11000022）。
                        self.modify_next_attack_shatter_defense(
                            actor_side,
                            other_param(card, 1).max(0),
                        );
                    }
                    11_000_024 => {
                        self.add_actor_negative_status(
                            opponent_side(actor_side),
                            100,
                            other_param(card, 1).max(0),
                        );
                    }
                    55 => {
                        self.activate_element(actor_side, super::Element::Water);
                        self.activate_element(actor_side, super::Element::Wood);
                    }
                    56 => {
                        self.activate_element(actor_side, super::Element::Fire);
                        self.activate_element(actor_side, super::Element::Earth);
                    }
                    57 => {
                        self.activate_element(actor_side, super::Element::Earth);
                        self.activate_element(actor_side, super::Element::Metal);
                    }
                    58 => self.apply_wave_cutting_seal_opening(actor_side),
                    11_000_023 => {
                        let loss = other_param(card, 1).max(0);
                        if loss > 0 {
                            self.modify_actor_hp(opponent_side(actor_side), -loss, false, false);
                            self.modify_actor_hp(actor_side, -loss, false, false);
                        }
                    }
                    _ => {}
                }
                self.finish_card_execution(previous_card_execution);
            }
            if player_fixture.fate_strategies.contains(&131) {
                for element in fate_strategy_131_elements(&player_fixture.talents) {
                    self.activate_element(actor_side, element);
                }
            }
            if player_fixture.fate_strategies.contains(&89) {
                self.gain_cloud_chain(actor_side, 1);
                self.actor_mut(actor_side).sword.cloud_sea += 2;
            }
            if player_fixture.fate_strategies.contains(&138) {
                self.activate_element(actor_side, super::Element::Wood);
                self.gain_anima(actor_side, 1);
            }
            if player_fixture.fate_strategies.contains(&143) {
                self.activate_element(actor_side, super::Element::Earth);
                self.gain_defense(actor_side, 4);
            }
            if player_fixture.fate_strategies.contains(&146) {
                self.activate_element(actor_side, super::Element::Fire);
                self.modify_target_hp(actor_side, -2);
                self.modify_target_max_hp(actor_side, -2);
            }
            if player_fixture.fate_strategies.contains(&135) {
                self.actor_mut(actor_side).formations.spirit_formation_echo = 1;
            }
            if player_fixture.fate_strategies.contains(&333) {
                // FateStrategy 333 must route through the real activation
                // hook (not just seed activated_elements membership): it
                // also owes the activation-count buff, primordial-spirit /
                // five-elements-gathering / talent-202 side effects that
                // every other activateElement call site gets.
                self.activate_element(actor_side, super::Element::Metal);
                self.gain_sharpness(actor_side, 2);
            }
            if player_fixture.fate_strategies.contains(&331) {
                self.actor_mut(actor_side).fate.qi_xing_lian_zhu = 7;
            }
            if player_fixture.fate_strategies.contains(&164)
                && player_fixture
                    .fate_strategy_temp_datas
                    .get("164")
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                self.actor_mut(actor_side)
                    .fate
                    .resonance_mystic_heart_enter_profound += 1;
                self.actor_mut(opponent_side(actor_side))
                    .fate
                    .resonance_mystic_heart_enter_profound += 1;
            }
            if player_fixture.fate_strategies.contains(&109) {
                self.actor_mut(actor_side).fate.sword_formation_guard += 1;
            }
            if player_fixture.fate_strategies.contains(&322) {
                self.actor_mut(actor_side).sword.frenzy_sword += 1;
            }
            if player_fixture.fate_strategies.contains(&327) {
                self.upgrade_first_battle_deck_card(actor_side, |card| {
                    card.name.contains('雷') || original_card_desc_contains_action_again(card)
                });
            }
            if player_fixture.fate_strategies.contains(&334) {
                self.upgrade_first_battle_deck_card(actor_side, |card| card.name.contains("木灵"));
            }
            if player_fixture.fate_strategies.contains(&404) {
                // FateStrategy 404（build 24610558）：FateStrategyFunctions.cs:553-556
                // 在开战时把战斗牌组中第一张名称含「卦」且仍可升级的牌提升一档。
                // 这是牌组改写，不是执行时替换；沿用同一 queue/active-slot
                // 过滤与逐档升级 helper，避免误升级未激活或 noUpgrade 卡。
                self.upgrade_first_battle_deck_card(actor_side, |card| card.name.contains('卦'));
            }
            if player_fixture.fate_strategies.contains(&409) {
                // 灵树庇佑（五行道盟 sect，build 24589371 新增 fate）：
                // FateStrategyFunctions.cs:560-565 —— 开局 maxHp +=
                // otherParams[0] × GetWuXingCountInDeck()（BattleCharacter.cs:12118
                // 战斗卡组去重五行数，含 292 / 7000101 / 刻印 76 / fate 147 修正），
                // 并等量回血。FateStrategyConfig.json id 409:
                // { sect: WuXingDaoMeng, includeCharacters: [3000002],
                //   otherParams: [3], isBattleEffect: true }。
                let gain = 3 * wu_xing_count_in_deck(self.actor(actor_side)).max(0);
                if gain > 0 {
                    self.modify_actor_max_hp(actor_side, gain);
                    self.modify_actor_hp(actor_side, gain, false, false);
                }
            }
            if player_fixture.fate_strategies.contains(&412) {
                // 五行道盟 fate 412（FateStrategyFunctions.cs:568-569，
                // OnBattleStart 内）：ModifyBuffValue(JiHuoMuLing, 1)，激活木灵。
                // oracle 锚点：mirror-32299000 44030fe7c4a7c3e7/round-08 cp0
                // （p2 buffs 238:1）、d064057231b7854b/round-10 cp0、
                // dadfe449edeb0660/round-10 cp0（p1 buffs 238:1）。
                self.activate_element(actor_side, super::Element::Wood);
            }
            if player_fixture.talents.contains(&106)
                && !self.actor(actor_side).astrology.star_slots.contains(&6)
            {
                self.actor_mut(actor_side).astrology.star_slots.push(6);
            }
            if player_fixture.talents.contains(&99) {
                self.activate_element(actor_side, super::Element::Water);
                self.gain_anima(actor_side, 1);
            }
            self.apply_abundant_momentum_talent(actor_side, player_fixture);
            if player_fixture.fate_strategies.contains(&423) {
                // 气盖山河（FateStrategyFunctions.cs:575-577）：开局写入
                // XiaCiQiShiDuoJia(769) = FateStrategyConfig(423).otherParams[0]
                // = 2。此处必须晚于气势充沛等开局气势发放；原版先完成这些
                // 启动增益，769 才等待战斗中的下一次正向气势变更。
                self.actor_mut(actor_side).beng.pending_momentum_bonus += 2;
            }
            if player_fixture.fate_strategies.contains(&428) {
                // 姬方生 锻玄宗 fate 428（FateStrategyFunctions.cs:579-581）：
                // OnBattleStart 内 ModifyBuffValue(QiShiShangXian, 6)，
                // 气势上限 6 → 12。缺这条会让 冲霄破浪「每有1灵气加1气势」
                // 的溢出提前转防御（引擎把溢出 5/2 转成防御，原版上限 12 内
                // 不溢出）。oracle 锚点：mirror-32299000
                // ad2de3e47a686184/round-13 cp7（原版 p2 气势 11 上限 12，
                // 引擎上限 6 → 防御+5）、b5c5673864f69912/round-15 cp6
                // （原版 10/14，引擎 10/8 → 防御+2）。
                self.modify_momentum_limit(actor_side, 6);
            }
            if player_fixture.talents.contains(&203) {
                self.activate_element(actor_side, super::Element::Wood);
            }
            // FateStrategy 151（FateStrategyFunctions.cs:445-451）：OnBattleStart
            // 内先发辟邪（IsSwitchActive 时）再 +5 生命。原版天赋 switch
            // （OnBattleStarted，含冥心烙印 150 系开局内伤）先于
            // FateStrategyFunctions.OnBattleStart；辟邪必须晚于冥心烙印的内伤，
            // 否则会吞掉该开局内伤，导致首回合内伤 tick 缺失、玄灵愈体类
            // 加血被多算（oracle 锚点：mirror-32299000 097e95c9e752e416/round-09
            // cp0 p1.hp 88 vs 90，351=2 内伤 tick 发生在 turn1 起始）。
            // 注：冥心烙印本身已在天赋段（fate 326 之前）执行。
            if player_fixture.fate_strategies.contains(&151) {
                if player_fixture
                    .fate_strategy_temp_datas
                    .get("151")
                    .copied()
                    .unwrap_or(0)
                    == 0
                {
                    self.actor_mut(actor_side).fate.exorcism += 2;
                }
                self.modify_actor_hp(actor_side, 5, false, false);
            }
            if player_fixture.talents.contains(&199) {
                self.apply_talent_199_bottle_elements(actor_side, player_fixture);
            }
            if let Some(internal_injury_grass) =
                player_fixture.permanent_buff_temp_datas.get("10013")
            {
                let amount = (*internal_injury_grass).max(0);
                if amount > 0 {
                    self.add_actor_negative_status(opponent_side(actor_side), 100, amount);
                }
            }
            if let Some(power_loss_grass) = player_fixture.permanent_buff_temp_datas.get("10018") {
                let amount = (*power_loss_grass).max(0);
                if amount > 0 {
                    self.add_actor_negative_status(opponent_side(actor_side), 103, amount);
                }
            }
            if let Some(exorcism_grass) = player_fixture.permanent_buff_temp_datas.get("10019") {
                let amount = (*exorcism_grass).max(0);
                if amount > 0 {
                    self.actor_mut(actor_side).fate.exorcism += amount;
                }
            }
            if let Some(fire_rope_orchid) = player_fixture.permanent_buff_temp_datas.get("10017") {
                let amount = (*fire_rope_orchid).max(0);
                if amount > 0 {
                    self.modify_target_max_hp(actor_side, -amount);
                }
            }
            if let Some(early_action_grass) = player_fixture.permanent_buff_temp_datas.get("10020")
            {
                let amount = (*early_action_grass).max(0);
                if amount > 0 {
                    self.modify_actor_hp(actor_side, -amount, false, false);
                    self.modify_extra_actions(actor_side, 1);
                }
            }
            if let Some(turtle_face_grass) = player_fixture.permanent_buff_temp_datas.get("10014") {
                let amount = (*turtle_face_grass).max(0);
                if amount > 0 {
                    self.gain_defense(actor_side, amount);
                }
            }
            self.apply_verified_ke_yin_battle_start_max_hp(actor_side);
            self.initialize_last_turn_start_hp(actor_side);
        }
        let second_side = opponent_side(first_side);
        if self
            .actor(second_side)
            .identity
            .fate_strategies
            .contains(&329)
        {
            self.actor_mut(second_side).fate.next_rear_move_bypass += 1;
        }
    }

    fn upgrade_first_battle_deck_card(
        &mut self,
        actor_side: PlayerSide,
        matches: impl Fn(&CardDefinition) -> bool,
    ) {
        let upgrade = self
            .actor(actor_side)
            .deck
            .queue
            .iter()
            .filter_map(|slot_index| {
                self.actor(actor_side)
                    .deck
                    .slots
                    .get(*slot_index)
                    .map(|slot| (*slot_index, slot))
            })
            .find(|(_, slot)| {
                matches(&slot.card) && can_upgrade_original_battle_deck_card(slot.card.id)
            })
            .map(|(slot_index, slot)| (slot_index, upgrade_original_card(&slot.card, 1)));
        if let Some((slot_index, upgraded)) = upgrade {
            self.actor_mut(actor_side).deck.slots[slot_index].card = upgraded;
        }
    }

    fn apply_verified_ke_yin_battle_start_max_hp(&mut self, actor_side: PlayerSide) {
        let max_hp = self
            .actor(actor_side)
            .identity
            .ke_yin_card_ids
            .iter()
            .map(|card_id| super::support::verified_ke_yin_max_hp(*card_id))
            .sum::<i64>();
        if max_hp <= 0 {
            return;
        }
        let actor = self.actor_mut(actor_side);
        actor.core.max_hp += max_hp;
        actor.core.hp += max_hp;
    }

    /// Applies this actor's deck-upgrade talent effects (tiger body 125 /
    /// 孤虚金书 198 + FS338) inside its own OnBattleStarted boundary, on the
    /// constructed deck slots. See the call-site comment in
    /// `apply_battle_start_opening_effects` for the ordering evidence; the
    /// changed slots re-run the shared per-slot preparation tail so upgraded
    /// and inserted cards keep the construction invariants (original config
    /// completion, historical overrides, replay adaptation).
    fn apply_actor_deck_start_effects(
        &mut self,
        actor_side: PlayerSide,
        player_fixture: &FixturePlayer,
        fixture: &BattleFixture,
    ) {
        let mut cards: Vec<CardDefinition> = self
            .actor(actor_side)
            .deck
            .slots
            .iter()
            .map(|slot| slot.card.clone())
            .collect();
        // 虎体 talent 125 的阈值取该 actor 自己 OnBattleStarted 边界时的
        // battleTempData.hp（BattleCharacter.cs IL_1748）：次位 actor 的
        // 整个 OnBattleStarted 晚于首位 actor，其 hp 已含首位开局效果；
        // 首位 actor 则优先用持久化采样 battleStartHp（构造期 hp 可能
        // 低估持久生命，oracle：mirror-32308000 6b7b9f3b16156d2d/round-14
        // cp1 原版 117 < 120 不升 vs 引擎按 maxHp 124 误升）。
        let boundary_hp = if actor_side == fixture.first_player_side {
            player_fixture
                .battle_start_hp
                .unwrap_or(self.actor(actor_side).core.hp)
        } else {
            self.actor(actor_side).core.hp
        };
        apply_deck_start_talent_effects(&mut cards, player_fixture, boundary_hp);
        for (slot_index, card) in cards.into_iter().enumerate() {
            let before = self.actor(actor_side).deck.slots[slot_index].card.id;
            if before == card.id {
                continue;
            }
            let prepared = prepare_fixture_deck_card(
                player_fixture,
                &fixture.historical_card_overrides,
                actor_side,
                slot_index,
                card,
            );
            self.actor_mut(actor_side).deck.slots[slot_index].card = prepared;
        }
    }

    fn apply_abundant_momentum_talent(
        &mut self,
        actor_side: PlayerSide,
        player_fixture: &FixturePlayer,
    ) {
        for talent in &player_fixture.talents {
            let Some((momentum, limit)) = abundant_momentum_from_talent(*talent) else {
                continue;
            };
            if limit > 0 {
                self.modify_momentum_limit(actor_side, limit);
            }
            self.modify_momentum(actor_side, momentum);
        }
    }

    fn apply_dark_heart_mark_talent(
        &mut self,
        actor_side: PlayerSide,
        player_fixture: &FixturePlayer,
    ) {
        for talent in &player_fixture.talents {
            let Some((internal_injury, recovery)) = dark_heart_mark_from_talent(*talent) else {
                continue;
            };
            self.add_actor_negative_status(actor_side, 100, internal_injury);
            self.actor_mut(actor_side).status.recovery += recovery;
        }
    }

    fn apply_battle_start_divination(
        &mut self,
        actor_side: PlayerSide,
        player_fixture: &FixturePlayer,
    ) {
        // 观星、星蚀、布阵 are all talent-loop mutations. Preserve their
        // fixture order: an earlier 星蚀 must arm the opponent before a later
        // star-power grant is redirected by Fate 399.
        for talent in &player_fixture.talents {
            match talent {
                30 | 10_030 | 20_030 | 30_030 => {
                    let amount = match talent {
                        30 | 10_030 => 1,
                        20_030 => 2,
                        30_030 => 3,
                        _ => unreachable!(),
                    };
                    self.gain_hexagram(actor_side, amount);
                }
                103 | 10_103 | 20_103 | 30_103 => {
                    let amount = match talent {
                        103 | 10_103 => 1,
                        20_103 => 2,
                        30_103 => 3,
                        _ => unreachable!(),
                    };
                    self.actor_mut(opponent_side(actor_side))
                        .astrology
                        .star_erosion += amount;
                }
                31 | 10_031 | 20_031 | 30_031 => {
                    self.modify_star_power(actor_side, 1);
                }
                _ => {}
            }
        }

        let saved_hexagram = if player_fixture.fate_strategies.contains(&126) {
            player_fixture
                .permanent_buff_temp_datas
                .get("10037")
                .copied()
                .unwrap_or(0)
                .max(0)
        } else {
            0
        };
        self.gain_hexagram(actor_side, saved_hexagram);
    }

    fn apply_talent_199_bottle_elements(
        &mut self,
        actor_side: PlayerSide,
        player_fixture: &FixturePlayer,
    ) {
        let bottle_cards = player_fixture
            .talent_card_params
            .get("199")
            .into_iter()
            .flatten()
            .copied()
            .collect::<Vec<_>>();
        let count = if player_fixture.fate_strategies.contains(&309) {
            2
        } else {
            1
        };
        for card_id in bottle_cards.into_iter().take(count) {
            if let Some(card) = original_card_definition(card_id).or_else(|| {
                player_fixture
                    .cards
                    .iter()
                    .find(|card| card.id == card_id)
                    .cloned()
            }) {
                self.activate_element_by_card(actor_side, &card);
            }
        }
    }
}
