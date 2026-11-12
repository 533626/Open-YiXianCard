use super::original_config::{complete_with_original_card, upgrade_original_card};
use super::support::{
    adapt_fixture_card_for_replay, basic_attack_card, div_ceil, is_pu_tong_gong_ji_base_id,
    is_talent_52_replacement_slot, normalize_base_id, normalized_base_id, permanent_physique_key,
    seven_stars_stabilize_soul_card,
};
use super::{
    DrawnCard, HpMutationReceipt, ReplayAstrologyState, ReplayBengQuanState, ReplayCardSlot,
    ReplayChanceState, ReplayCoreVitals, ReplayDeckState, ReplayElementState,
    ReplayFateTalentState, ReplayFormationState, ReplayMusicState, ReplayPlayer,
    ReplayPlayerIdentity, ReplayState, ReplayStatusState, ReplaySwordState, ReplayTurnState,
};
pub(super) use crate::fixture::ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES;
use crate::fixture::{apply_historical_card_patch, FixturePlayer, HistoricalCardOverride};
use crate::model::{CardDefinition, PlayerSide};

/// BattleCharacter.AfterHpModifyEffect:9774-9834, in current-build source order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum AfterHpModifyPhase {
    SpiritTurtleFootwork,
    FirstHpLossReward,
    Talent64Defense,
    KeYin50147Defense,
    IceSnowLotus,
    DreamCliff,
    BloodCalamity,
    HpLossAttackCharge,
    YanQi,
    HpLossLedgers,
}

/// OnTurnEnded 的 虚弱/破绽/困缚 逐层衰减量（BattleCharacter.cs:5686-5695）。
#[derive(Debug, Clone, Copy, Default)]
pub(super) struct TurnEndStatusDecay {
    pub weakness: i64,
    pub flaw: i64,
    pub entangle: i64,
}

pub(super) const ORIGINAL_AFTER_HP_MODIFY_PHASES: [AfterHpModifyPhase; 10] = [
    AfterHpModifyPhase::SpiritTurtleFootwork,
    AfterHpModifyPhase::FirstHpLossReward,
    AfterHpModifyPhase::Talent64Defense,
    AfterHpModifyPhase::KeYin50147Defense,
    AfterHpModifyPhase::IceSnowLotus,
    AfterHpModifyPhase::DreamCliff,
    AfterHpModifyPhase::BloodCalamity,
    AfterHpModifyPhase::HpLossAttackCharge,
    AfterHpModifyPhase::YanQi,
    AfterHpModifyPhase::HpLossLedgers,
];

#[cfg(test)]
pub(super) const HP_MUTATION_SCOPE_EXCLUSIONS: [(&str, &str); 2] = [
    (
        "JiLuZhanDouZuiGaoShengMing",
        "BattleCharacter.cs:889,9718-9721; write-only diagnostic high-water mark",
    ),
    (
        "fengRui",
        "BattleCharacter.cs:9544,9732-9746; floating-text and FX selector only",
    ),
];

impl ReplayPlayer {
    /// Original `AddHpCount`, whose lifetime is the whole battle.
    pub(super) fn add_hp_count(&self) -> i64 {
        self.hp_mutation.add_hp_count.max(0)
    }

    pub(super) fn has_ling_qi_ben_yong(&self) -> bool {
        self.identity.talents.contains(&209) || self.identity.talents.contains(&208)
    }

    pub(super) fn from_fixture_view(
        side: PlayerSide,
        fixture: &crate::fixture::FixturePlayerView<'_>,
        historical_overrides: &[HistoricalCardOverride],
    ) -> Self {
        let mut hp = fixture.base_max_hp + fixture.extra_max_hp.unwrap_or(0);
        let max_hp_base = hp;
        // FateStrategy 140 (天衍-猛虎之躯) 的 hp/10 增益在开局钩子结算
        // （apply_battle_start_opening_effects）中按当时当前生命计算：
        // 先手方开局伤害（如 11_000_001 天谕系）先扣血，140 再按扣后生命
        // 采样（oracle 锚点：mirror-32299000 pa_1c8ad428 cp0
        // p2.hp 97 vs 98：100 - 3 = 97 → 97/10 = 9，引擎构造期 100/10 = 10）。
        let permanent_physique = fixture
            .permanent_buff_temp_datas
            .get(permanent_physique_key())
            .copied()
            .unwrap_or(0);
        let active_slot_count = fixture.active_slot_count;
        let ling_wu_card_base_ids = if fixture.talents.contains(&192) {
            fixture
                .talent_card_params
                .get("189")
                .into_iter()
                .flatten()
                .map(|card_id| normalize_base_id(*card_id))
                .collect()
        } else {
            Vec::new()
        };
        let initial_physique = permanent_physique;
        let initial_momentum_limit = fixture.initial_momentum_limit.unwrap_or(6).max(0);
        let initial_temp_life = fixture.last_round_life.unwrap_or(max_hp_base).max(0);
        let initial_defense = fixture.initial_defense.max(0);
        // 筋骨健壮（176 系）的战斗开始加血（ModifyHp(floor(TiPo / 7)) 等）在
        // battle_start.rs 的 OnBattleStarted 天赋循环位置采样当前体魄执行，
        // 不在构造期预置：原版 case 176（BattleCharacter.cs IL_1411）读取的是
        // 该天赋在天赋循环中的实时 TiPo，入冥（179）先触发 415 疯魔架势
        // ModifyTiPo(+1) 再被 20176 采样（oracle 锚点：pa_fcf69621
        // cp0 p2 10 = 70/7，构造期 69/7 = 9 少 1 血）。
        // OnBattleStarted:899 角色 4000005 或仙魔 27023 开局获得架势；335/7111 时为棍架势，否则拳。
        let staff_owner =
            fixture.character_id == Some(4_000_005) || fixture.xian_mo_strategies.contains(&27023);
        let endless_staff_stance = staff_owner
            && (fixture.fate_strategies.contains(&335)
                || fixture.xian_mo_strategies.contains(&7111));
        let initial_quan_stance = if endless_staff_stance {
            0
        } else if fixture.talents.contains(&204) || staff_owner {
            1
        } else {
            0
        };
        let initial_gun_stance = if endless_staff_stance { 1 } else { 0 };
        let initial_sword_intent = (if fixture.talents.contains(&16) { 1 } else { 0 })
            + (if fixture.talents.contains(&10_016) {
                1
            } else {
                0
            })
            + (if fixture.talents.contains(&20_016) {
                1
            } else {
                0
            })
            + (if fixture.talents.contains(&30_016) {
                2
            } else {
                0
            });
        let initial_star_power = 0;
        let initial_anima = fixture.initial_anima.max(0);
        let initial_hexagram = 0;
        let initial_water_momentum = 0;
        let character_id = fixture.character_id;
        let talent_meditation = if fixture.talents.contains(&179) { 1 } else { 0 };
        let fate_meditation = if fixture.fate_strategies.contains(&161) {
            2
        } else {
            0
        };
        let initial_meditation = talent_meditation + fate_meditation;
        if initial_meditation > 0
            && character_id != Some(4_000_003)
            && !fixture.xian_mo_strategies.contains(&27021)
        {
            // Other characters pay the setup HP loss directly. The forging
            // character's positive conversion is applied after ReplayState is
            // assembled so it can enter the normal HP mutation pipeline.
            hp -= initial_meditation * 3;
        }
        let generating_interaction_upgrade = (if fixture.talents.contains(&127) { 1 } else { 0 })
            + (if fixture.talents.contains(&10_127) {
                2
            } else {
                0
            })
            + (if fixture.talents.contains(&20_127) {
                2
            } else {
                0
            })
            + (if fixture.talents.contains(&30_127) {
                3
            } else {
                0
            });
        let initial_battle_buff = |name: &str| {
            fixture
                .initial_battle_buffs
                .get(name)
                .copied()
                .unwrap_or(0)
                .max(0)
        };
        let _ = side;
        Self {
            prevention: super::ReplayPreventionState::default(),
            identity: ReplayPlayerIdentity {
                level: fixture.level,
                character_id,
                fate_strategies: fixture.fate_strategies.clone().into(),
                fate_strategy_temp_datas: fixture.fate_strategy_temp_datas.clone(),
                xian_mo_strategies: fixture.xian_mo_strategies.clone().into(),
                xian_mo_temp_datas: fixture.xian_mo_temp_datas.clone(),
                talents: fixture.talents.clone().into(),
                sword_forging_attack: fixture.talent_temp_datas.get("92").copied().unwrap_or(0),
                talent_resonance_id: fixture.talent_resonance_id,
                talent_resonance_temp_flags: Vec::new(),
                ke_yin_card_ids: fixture.used_ke_yin_cards.clone(),
                last_round_exp: fixture.last_round_exp,
                talent_199_card_ids: fixture
                    .talent_card_params
                    .get("199")
                    .cloned()
                    .unwrap_or_default(),
                hand_card_count: fixture.hand_cards.len() as i64,
            }
            .into(),
            core: ReplayCoreVitals {
                hp,
                max_hp: max_hp_base + initial_physique,
                temp_life: initial_temp_life,
                defense: initial_defense,
                anima: initial_anima,
                guard: fixture.initial_guard.max(0),
                temporary_guard: 0,
                // Talent 171's JiaGong is also an OnBattleStarted mutation;
                // keep it beside WaiShang so opening order remains observable.
                attack_bonus: 0,
                physique: initial_physique,
                // BattleCharacter.cs:149-153 只在上场永久值 > 0 时 SetBuffValue(TiPoShangXian)；
                // 缺失时 GetBuffValue 为 0，:10256 的溢出判定按 0 比较。
                physique_limit: fixture
                    .permanent_buff_temp_datas
                    .get("10024")
                    .copied()
                    .unwrap_or(0),
                lost_max_hp_count: 0,
            },
            sword: ReplaySwordState {
                sword_intent: initial_sword_intent,
                pending_sword_intent_outside_card: 0,
                sword_energy: 0,
                // FateStrategy 333's metal activation + +2 sharpness now
                // route through ReplayState::activate_element/gain_sharpness
                // in battle_start.rs (post-construction) so the shared
                // activate-element side-effect hooks run.
                sharpness: 0,
                metal_ring: 0,
                cloud_chain: 0,
                cloud_sea: 0,
                cloud_sword_soft_heart: 0,
                cloud_sword_heart: 0,
                frenzy_dragon_swallows_cloud: 0,
                frenzy_sword: (if fixture.talents.contains(&20_070) {
                    1
                } else {
                    0
                }) + (if fixture.talents.contains(&30_070) {
                    1
                } else {
                    0
                }),
                frenzy_sword_zero: 0,
                upgrade_next_frenzy_sword: 0,
                sword_formation_count: 0,
                hundred_bird_spirit_sword_art: 0,
                hundred_bird_trailing_shadow_art: 0,
                hundred_beast_spirit_sword_formation: 0,
                hundred_beast_spirit_sword_formation_triggered: false,
                water_month_sword_formation: 0,
                sword_intent_circulation: 0,
                dark_void_sword_formation_art: 0,
                spirit_sword_mindset: 0,
                cloud_step: 0,
                frenzy_sword_double_effect: 0,
                ling_wu_card_base_ids,
                cloud_sword_heaven_cycle: 0,
                all_cards_as_cloud_sword: 0,
                frenzy_sword_cloud_gathering: 0,
                all_purpose_sword: 0,
                all_purpose_sword_effective_count: 0,
                next_card_as_cloud_sword: 0,
                next_cards_as_frenzy_sword: 0,
                next_cards_as_frenzy_sword_effective_count: 0,
                yun_jian_mao_ying: 0,
                chain_sword_temporary_cursor: None,
            }
            .into(),
            elements: ReplayElementState {
                water_momentum: initial_water_momentum,
                water_formation: 0,
                metal_formation: 0,
                earth_formation: 0,
                fire_formation: 0,
                spring_flow: 0,
                water_stealth: 0,
                metal_iron_bone: 0,
                swift_burn_seal: if fixture.talents.contains(&140) { 1 } else { 0 },
                metal_cauldron_drop: 0,
                no_sharpness_for_attack: 0,
                water_blade_seal: 0,
                earth_cliff_counter: 0,
                earth_eight_wastes: 0,
                dream_two_polarity_defense: 0,
                dream_two_polarity_hp: 0,
                primordial_infinity_formation: 0,
                five_elements_marrow_art: 0,
                wood_array: 0,
                wood_healing_formation: 0,
                wood_spirit_all_growth: 0,
                wood_spirit_all_growth_attack: 0,
                wood_thorn: 0,
                wood_scattered_branches: 0,
                attack_life_drain: 0,
                activated_metal: 0,
                activated_water: 0,
                activated_wood: 0,
                activated_fire: 0,
                activated_earth: 0,
                next_card_activate_element: if fixture.talents.contains(&82) { 1 } else { 0 },
                seal_suppressing_mindset: 0,
                water_momentum_gain_count: 0,
                five_elements_gourd: 0,
                // FateStrategy 333's metal activation now routes through
                // ReplayState::activate_element in battle_start.rs.
                activated_elements: Vec::new(),
                last_element: None,
                used_water_spirit_card: 0,
                long_ma_spirit: 0,
                synthetic_ding_feng_bo_candidate: 0,
                zhu_chen_bu: 0,
            }
            .into(),
            formations: ReplayFormationState {
                turtle_formation: 0,
                turtle_formation_defense: 0,
                hard_branch_bamboo: 0,
                hard_branch_bamboo_defense_per_damage: 0,
                forge_bone_attacks: 0,
                forge_bone_attack_bonus: 0,
                fortune_avoid_misfortune: 0,
                fortune_avoid_misfortune_defense: 0,
                fortune_avoid_misfortune_healing: 0,
                shatter_formation: 0,
                shatter_formation_bonus: 0,
                thunder_formation: 0,
                thunder_formation_damage: 0,
                evil_gu_formation: 0,
                evil_gu_formation_value: 0,
                spirit_gathering_formation: 0,
                spirit_gathering_formation_value: 0,
                heaven_cycle_sword_formation: 0,
                heaven_cycle_sword_formation_damage: 0,
                heaven_force_formation: 0,
                flower_maze_formation: 0,
                flower_maze_drain: 0,
                immovable_formation: 0,
                immovable_formation_value: 0,
                eight_gates_formation: 0,
                eight_gates_formation_damage: 0,
                array_echo_persistent_card: 0,
                body_observation: 0,
                soul_injury_curse_formation: 0,
                six_yao_formation: 0,
                spirit_formation_echo: 0,
                spirit_formation_echo_triggered: false,
            }
            .into(),
            beng: ReplayBengQuanState {
                next_beng_quan_hp_cost_damage: 0,
                beng_quan_double_shadow: 0,
                momentum: fixture.initial_momentum.max(0),
                momentum_limit: initial_momentum_limit,
                pending_momentum_bonus: 0,
                momentum_multiplier: 0,
                momentum_gain_agility_triggered: 0,
                beng_quan_cun_jin: 0,
                beng_quan_bounce: 0,
                beng_quan_chan: 0,
                beng_quan_han: 0,
                beng_quan_flash_agility: 0,
                beng_quan_chuo: 0,
                consumed_beng_quan_chuo: 0,
                beng_quan_defense: 0,
                beng_quan_tu: 0,
                beng_quan_meridian: 0,
                beng_quan_startled_touch: 0,
                triggered_startled_touch: 0,
                beng_quan_return_profound: 0,
                beng_tian_step: 0,
                beng_mei_mindset: 0,
                momentum_before_attack: 0,
                unceasing_momentum: 0,
                return_to_simplicity: 0,
                dream_beng_quan_chain: 0,
                triggered_dream_beng_quan_chain: 0,
                beng_quan_fu_hu: 0,
                triggered_beng_quan_fu_hu: 0,
                beng_quan_fei_zhou: 0,
                quan_stance: initial_quan_stance,
                gun_stance: initial_gun_stance,
            }
            .into(),
            music: ReplayMusicState {
                music_cards_played: 0,
                immortal_binding_tune: 0,
                immortal_binding_vine: 0,
                devouring_ancient_vine: 0,
                illusory_tune: 0,
                heartbreak_tune: 0,
                wild_dance_tune: 0,
                rejuvenation_tune: 0,
                xiaoyao_tune: 0,
                xiaoyao_guqin: 0,
                chaotic_mind_tune: 0,
            }
            .into(),
            astrology: ReplayAstrologyState {
                hexagram: initial_hexagram,
                lost_hexagram: 0,
                hexagram_effective_count: 0,
                ling_gua_art: 0,
                star_power: initial_star_power,
                star_moon_fan: 0,
                anima_to_star_power: 0,
                pending_anima_hexagram: false,
                star_chess_break: 0,
                infinite_hexagram_plate: 0,
                all_goes_well: 0,
                star_erosion: 0,
                thunder_mindset: 0,
                dream_thunder_hexagram: 0,
                dream_thunder_round_limit: 0,
                zi_mang_xing_bao: 0,
                lei_shan_er_du: 0,
                ling_yuan_wai_fang: 0,
                star_slots: vec![2, 5],
            }
            .into(),
            status: ReplayStatusState {
                // Talent 171's opening 外伤 is an OnBattleStarted ModifyBuffValue,
                // not constructor state.  It must pass through the shared
                // negative-status kernel so opposing 星蚀 (30103) can amplify
                // the first negative-status gain.
                external_injury: 0,
                internal_injury: 0,
                transient_internal_injury: 0,
                flame_heart_urging: 0,
                recovery: fixture
                    .permanent_buff_temp_datas
                    .get("10010")
                    .copied()
                    .unwrap_or(0)
                    .max(0),
                meditation: initial_meditation,
                min_night: 0,
                mystic_soul: 0,
                weakness: 0,
                attack_reduction: 0,
                cannot_act: 0,
                flaw: 0,
                drunken_fist_stance: 0,
                drunken_leisure: 0,
                entangle: 0,
                yin_fu: 0,
                yin_fu_jue_zhen: 0,
                back_solitude: 0,
                strike_void: 0,
                blood_calamity: 0,
                lone_night_wolf: 0,
                leaf_pluck_flying_leaf: 0,
                leaf_blade_flower: 0,
                poison_immunity: 0,
                lost_mind: 0,
            },
            hp_mutation: super::ReplayHpMutationState {
                add_hp_count: 0,
                no_hp_loss_before_next_turn: initial_battle_buff(
                    ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES[0],
                ),
                appetite: initial_battle_buff(ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES[1]),
                red_date_zongzi: initial_battle_buff(ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES[2]),
                egg_yolk_zongzi: initial_battle_buff(ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES[3]),
                immortal_egg_yolk_zongzi: initial_battle_buff(
                    ORIGINAL_HP_MUTATION_RUNTIME_BUFF_NAMES[4],
                ),
            },
            fate: ReplayFateTalentState {
                quiet_mindset: 0,
                reflect_mindset: 0,
                graft_flowers_to_tree: 0,
                paint_finishing_touch: 0,
                generating_interaction_upgrade,
                tide: 0,
                spirit_gathering_mindset: 0,
                five_elements_gathering_triggered: 0,
                half_anima: 0,
                mystic_heart_enter_profound: 0,
                dismantle_move: 0,
                dismantle_move_reflect: 0,
                all_things_inauspicious: 0,
                last_stand_intent: fixture
                    .permanent_buff_temp_datas
                    .get("17")
                    .copied()
                    .unwrap_or(0),
                last_stand_unyielding: 0,
                flame_soul_return: if fixture.talents.contains(&139) { 1 } else { 0 },
                fire_phoenix_revive_hp: fixture
                    .permanent_buff_temp_datas
                    .get("10072")
                    .copied()
                    .unwrap_or(0)
                    .max(0),
                qi_xing_jie_ming: 0, // FateStrategyFunctions.OnBattleStart 在 battle_start.rs 发放
                jie_quan_shi: 0,     // FateStrategyFunctions.OnBattleStart 在 battle_start.rs 发放
                tian_yan_feng_ling_duan_qu: 0, // 同上（431 风灵锻躯）
                next_anima_gain_bonus: 0,
                dragon_first_attack_stealth: 0,
                normal_attack_copy_previous: 0,
                normal_attack_guard_and_exhaust: 0,
                normal_attack_bonus_and_exhaust: 0,
                next_damage_internal_injury: 0,
                next_card_downgrade: 0,
                xian_mo_13006_triggered: 0,
                xian_mo_5011_pending_pu_gong: 0,
                xian_mo_11003_wuxing_upgrades: 0,
                skip_hp_deltas: Vec::new(),
                skip_anima_deltas: Vec::new(),
                cu_ju_fei_xi: 0,      // 同上（416 促局飞袭）
                wu_you_ling_niang: 0, // 同上（407 无忧灵酿）
                wu_jing_gua_yan: 0,   // 同上（395 无尽卦衍）
                xing_yuan: 0,         // 同上（402 星缘）
                ke_yin_jing_lei: 0,   // 同上（396 惊雷破敌）
                fate_cycle: 0,
                fate_cycle_slots: [0, 0],
                plum_blossom_twice: 0,
                qi_xing_lian_zhu: 0,
                xing_yi_lian: 0,
                yellow_bird_behind: 0,
                reverse_card_direction: 0,
                sword_formation_guard: 0,
                next_rear_move_bypass: 0,
                used_rear_move_check: 0,
                rear_move_succeeded: false,
                exorcism: 0,
                xian_guo_bao_zong: 0,
                heavenly_secret_reverse: 0,
                vitality_bloom: 0,
                first_strike: 0,
                mirage_vitality_bloom: 0,
                mirage_vitality_bloom_heal: 0,
                fortune_seek_auspicious: 0,
                fortune_seek_auspicious_damage: 0,
                ice_snow_lotus: 0,
                leaf_shield_flower: 0,
                sheng_qi_ling_ren: 0,
                wild_ferry_seal: 0,
                yan_qi: 0,
                feng_ling_zhan_yi: 0,
                chan_xin_ju_ling_triggered: 0,
                hot_blood_to_qi_triggered: 0,
                vermilion_bird_tear: if fixture.fate_strategies.contains(&344) {
                    1
                } else {
                    0
                },
                resonance_mystic_heart_enter_profound: 0,
                instant_shadow_strike: 0,
            }
            .into(),
            chance: ReplayChanceState::default().into(),
            dream_mirage: super::ReplayDreamMirageState {
                action_again_limit: 1,
                ..Default::default()
            }
            .into(),
            mirage_ronghui: super::ReplayMirageRonghuiState::default().into(),
            ronghui: super::ReplayRonghuiState {
                five_elements_bottle_card_id: fixture
                    .talent_card_params
                    .get("199")
                    .and_then(|cards| cards.first())
                    .copied()
                    .unwrap_or(0),
                ..Default::default()
            }
            .into(),
            turn: ReplayTurnState {
                lost_defense_count: 0,
                turn_attack_segments: 0,
                next_turn_defense: 0,
                spirit_control_anima_gain_defense: 0,
                spirit_control_anima_loss_defense: 0,
                attack_applies_internal_injury_turns: 0,
                wood_spring_turns: 0,
                blood_shadow: 0,
                hp_cost_cards_used: 0,
                used_card_count: 0,
                adaptation: 0,
                ignore_defense_attacks: if fixture.talents.contains(&25) { 2 } else { 0 },
                ignore_guard_attacks: 0,
                ignore_weakness_attacks: 0,
                next_attack_shatter_defense: 0,
                attack_segments_performed: 0,
                extra_actions: 0,
                agility: fixture.initial_agility.max(0),
                action_again_count: 0,
                action_again_blocked_turns: 0,
                successful_action_again_total: 0,
                own_turn_count: 0,
                keyin_shan_feng: 0,
                wind_spirit_body_forge_count: 0,
                battle_physique_gain_count: 0,
                lose_hp_count: 0,
                lose_hp_times_count: 0,
                actual_damage_carry: 0,
                wounded_count_carry: 0,
                dan_ka_gong_ji_ji_shu: 0,
                ji_lu_zong_ji_shang_zhi: 0,
                spirit_turtle_footwork: 0,
                spirit_turtle_footwork_triggered: 0,
                next_card_action_again: 0,
                current_turn_ignore_defense: 0,
                agility_gain_damage: 0,
                anima_gain_count: 0,
                next_attack_bonus: 0,
                next_attack_wound_bonus: 0,
                guaranteed_wound: 0,
                next_card_anima_cost_reduction: 0,
                jump_to_previous_card: 0,
                last_round_used_card_base_ids: fixture.last_round_used_card_base_ids.clone(),
            },
            deck: ReplayDeckState {
                slots: fixture
                    .cards
                    .iter()
                    .enumerate()
                    .map(|(slot_index, card)| ReplayCardSlot {
                        // 牌组升级（虎体 125 / 孤虚金书 198/338）不在构造期
                        // 执行：原版在各自 OnBattleStarted 的天赋循环里做，
                        // 晚于首位 actor 的 [开局] 降级（见 battle_start.rs
                        // apply_actor_deck_start_effects 的注释与 oracle 锚点）。
                        card: prepare_fixture_deck_card(
                            fixture,
                            historical_overrides,
                            side,
                            slot_index,
                            card.clone(),
                        )
                        .into(),
                        skipped: false,
                        used: false,
                    })
                    .collect(),
                queue: (0..active_slot_count).collect(),
                active_slot_count,
            },
        }
    }
}

/// Per-slot deck preparation shared by deck construction and the
/// battle-start deck-upgrade boundary (battle_start.rs
/// `apply_actor_deck_start_effects`): the fixture card is completed with
/// the original config, Talent-52/FateStrategy-120 replacements applied,
/// historical overrides patched and replay adaptation applied. Talent
/// 125/198 upgrades intentionally run outside this helper, on the
/// constructed slots, at each actor's own OnBattleStarted boundary.
pub(super) fn prepare_fixture_deck_card(
    fixture: &FixturePlayer,
    historical_overrides: &[HistoricalCardOverride],
    side: PlayerSide,
    slot_index: usize,
    card: CardDefinition,
) -> CardDefinition {
    let mut card = if is_talent_52_replacement_slot(fixture, &card, slot_index) {
        seven_stars_stabilize_soul_card()
    } else {
        complete_with_original_card(&card)
    };
    if fixture.fate_strategies.contains(&120) && fixture.active_slot_count >= 8 && slot_index == 7 {
        if normalized_base_id(&card) == 11 {
            card = upgrade_original_card(&card, 1);
        } else if normalized_base_id(&card) == 0 {
            card = seven_stars_stabilize_soul_card();
        }
    }
    let card = if let Some(override_entry) = historical_overrides
        .iter()
        .find(|candidate| candidate.side == side && candidate.slot_index == slot_index)
    {
        apply_historical_card_patch(card, &override_entry.patch)
    } else {
        card
    };
    adapt_fixture_card_for_replay(card, fixture)
}

impl ReplayPlayer {
    /// XianMoStrategyFunctions.IsSwitchActive：先要求持有策略，缺省 tempData 视为 0（开启）。
    pub(super) fn xian_mo_switch_active(&self, strategy: i64) -> bool {
        self.identity.xian_mo_strategies.contains(&strategy)
            && self
                .identity
                .xian_mo_temp_datas
                .get(&strategy.to_string())
                .copied()
                .unwrap_or(0)
                == 0
    }

    fn shift_next_non_skip(
        &mut self,
        skipped: &mut Vec<usize>,
        active: usize,
        last_break: usize,
        steps: &mut usize,
        max_steps: usize,
    ) -> Result<usize, Option<usize>> {
        // BattleCharacter.ShiftCard:8479-8497 — pop queue head, filtering skip=true
        // (used sustain/consume) via Push+Shift. Each filtered skip counts toward
        // fallback (skipped len - last_break >= active → basic attack).
        // Each pop counts toward max_draw_steps (Duan progress).
        // Ok(slot) = next non-skip, Err(Some(fallback_slot)) = fallback basic,
        // Err(None) = steps exhausted or empty queue.
        loop {
            if *steps >= max_steps {
                return Err(None);
            }
            if self.deck.queue.is_empty() {
                return Err(None);
            }
            let slot = self.deck.queue.remove(0);
            *steps += 1;
            let is_skipped = self.deck.slots.get(slot).is_some_and(|s| s.skipped);
            if !is_skipped {
                return Ok(slot);
            }
            skipped.push(slot);
            self.deck.queue.push(slot);
            if skipped.len().saturating_sub(last_break) >= active {
                return Err(Some(slot));
            }
        }
    }

    pub(super) fn draw_next_card(
        &mut self,
        nameless_white_deer_skip_limit: i64,
    ) -> Option<DrawnCard> {
        // BattleExecuter.cs:1895-2048 outer while(flag) + inner whiles, current build.
        // Order: Duan → KeYin(99000214) → MengEJie → FS398 → 7116 → 5010 →
        // 9000015 → 202 → 350 → 11000017(ignored) → XingYiTiao(if+inner) →
        // DongZhu(while) → ShunYing(ignored).
        // Each inner while skips consecutive applicable slots without re-checking
        // earlier mechanisms in the same outer iteration; after all mechanisms,
        // outer repeats from the top if any skip occurred (flag).
        // XingYiTiao inner while skips non-stars with opening, then clears the
        // entire buff in the same outer iteration, before DongZhu (:1991-2013).
        // DongZhu inner while skips consecutive fate grids with opening, 1 per skip.
        // Anchors (all four require outer-loop, not single-loop fixed order):
        // - pa_f7c24edd #7 (consecutive fate 4→2, both openings):
        //   fate inner-while wins for grid3→grid4 without 7116 interleaving.
        // - pa_6cabbdea [7] (isolated overlap grid4, fate stays 4,
        //   no opening, anima 1 vs 2): 7116 before fate for isolated grid.
        // - pa_a66ce89f [7] (starting at star grid4 with jump,
        //   7116→jump→star): 7116 before jump in same iteration, jump preserved.
        // - pa_45faec9d [4] (jump landing on grid4 star, then 7116,
        //   jump cleared): jump landing clears before next outer iteration's 7116.
        let active_slot_count = self.deck.queue.len();
        if active_slot_count == 0 {
            return None;
        }
        let mut skipped_slot_indices = Vec::new();
        let mut skipped_opening_slot_indices = Vec::new();
        let mut fate_398_skipped_fifth_grid = false;
        // 每次星弈断消耗一层都是有限进展，允许 ShiftCard 再绕过已离场的牌。
        // 只有自上次消耗以来整圈都无法取牌，才可退化为普通攻击。
        let mut last_break_progress = 0usize;
        let max_draw_steps = active_slot_count
            .saturating_mul((self.astrology.star_chess_break.max(0) as usize).saturating_add(1));
        let mut steps: usize = 0;
        let mut current = match self.shift_next_non_skip(
            &mut skipped_slot_indices,
            active_slot_count,
            last_break_progress,
            &mut steps,
            max_draw_steps,
        ) {
            Ok(slot) => slot,
            Err(Some(fallback_slot)) => {
                return Some(DrawnCard {
                    source_slot: fallback_slot,
                    card: basic_attack_card().into(),
                    fallback_basic_attack: true,
                    skipped_slots: skipped_slot_indices,
                    skipped_opening_slots: skipped_opening_slot_indices,
                    fate_398_skipped_fifth_grid,
                });
            }
            Err(None) => return None,
        };
        loop {
            let mut flag = false;
            // XingYi_Duan while (:1898-1903).
            while self.astrology.star_chess_break > 0 {
                self.astrology.star_chess_break =
                    (self.astrology.star_chess_break - 1).max(0);
                skipped_slot_indices.push(current);
                last_break_progress = skipped_slot_indices.len();
                self.deck.queue.push(current);
                flag = true;
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // KeYin 99000214 (nameless white deer) while (:1932-1936), before MengEJie.
            // Decompiled order restored (was last, after 7116); no oracle yet hits
            // KeYin+other overlap, faithful order is fail-closed for future.
            while nameless_white_deer_skip_limit > 0
                && (current as i64) < nameless_white_deer_skip_limit
                && self.deck.slots.get(current).is_some_and(|slot| {
                    slot.card
                        .card_type
                        .as_ref()
                        .is_some_and(|card_type| card_type.value == super::CARD_TYPE_SUSTAIN)
                })
            {
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // MengEJie (calamity) while (:1937-1942).
            while (self.dream_mirage.calamity_skip_mask & (1_i64 << current)) != 0 {
                self.dream_mirage.calamity_skip_mask &= !(1_i64 << current);
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // FS398 while (:1943-1951). HP+5 for star deferred to flow.rs via flag.
            while current == 4
                && self.identity.fate_strategies.contains(&398)
                && self
                    .identity
                    .fate_strategy_temp_datas
                    .get("398")
                    .copied()
                    .unwrap_or(0)
                    == 0
            {
                fate_398_skipped_fifth_grid = true;
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 7116 while (:1952-1961), before 5010/spirit/jump/fate (decompiled).
            // Restored from after-fate (which broke ed491 isolated overlap).
            // Consecutive fate grids still fate-win via DongZhu inner-while batching.
            while current == 4 && self.xian_mo_switch_active(7116) {
                let is_star_chess = self
                    .deck
                    .slots
                    .get(current)
                    .is_some_and(|slot| slot.card.name.contains("星弈"));
                if is_star_chess {
                    self.fate.skip_hp_deltas.push(3);
                    self.fate.skip_anima_deltas.push(1);
                }
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 5010 while (:1962-1966).
            while current >= 5
                && self.xian_mo_switch_active(5010)
                && self.deck.slots.get(current).is_some_and(|slot| {
                    !is_pu_tong_gong_ji_base_id(normalized_base_id(&slot.card))
                })
            {
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 9000015 while (:1967-1972, hadUsed gate).
            while self.deck.slots.get(current).is_some_and(|slot| {
                normalized_base_id(&slot.card) == 9_000_015
                    && current + 2 >= active_slot_count
                    && !slot.used
            }) {
                if let Some(slot) = self.deck.slots.get_mut(current) {
                    slot.used = true;
                }
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 202 while (:1973-1978, no hadUsed gate).
            while self.deck.slots.get(current).is_some_and(|slot| {
                normalized_base_id(&slot.card) == 202 && current + 1 >= active_slot_count
            }) {
                if let Some(slot) = self.deck.slots.get_mut(current) {
                    slot.used = true;
                }
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 350 while (:1979-1984, hadUsed gate).
            while self.deck.slots.get(current).is_some_and(|slot| {
                normalized_base_id(&slot.card) == 350
                    && current + 2 >= active_slot_count
                    && !slot.used
            }) {
                if let Some(slot) = self.deck.slots.get_mut(current) {
                    slot.used = true;
                }
                skipped_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // 11000017 while (:1985-1990) not implemented (no fixture hits it).
            // XingYiTiao if + inner while (:1991-2013): skip non-stars with opening,
            // then clear entire buff in same outer iteration, before DongZhu.
            // Only XingYiTiao/DongZhu trigger opening (SkipCardAnim never does).
            if self.ronghui.star_chess_jump > 0 {
                while self
                    .deck
                    .slots
                    .get(current)
                    .is_some_and(|slot| !slot.card.name.contains("星弈"))
                {
                    skipped_slot_indices.push(current);
                    skipped_opening_slot_indices.push(current);
                    self.deck.queue.push(current);
                    flag = true;
                    if skipped_slot_indices.len().saturating_sub(last_break_progress)
                        >= active_slot_count
                    {
                        return Some(DrawnCard {
                            source_slot: current,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    match self.shift_next_non_skip(
                        &mut skipped_slot_indices,
                        active_slot_count,
                        last_break_progress,
                        &mut steps,
                        max_draw_steps,
                    ) {
                        Ok(slot) => current = slot,
                        Err(Some(fallback_slot)) => {
                            return Some(DrawnCard {
                                source_slot: fallback_slot,
                                card: basic_attack_card().into(),
                                fallback_basic_attack: true,
                                skipped_slots: skipped_slot_indices,
                                skipped_opening_slots: skipped_opening_slot_indices,
                                fate_398_skipped_fifth_grid,
                            });
                        }
                        Err(None) => return None,
                    }
                }
                let jump_value = self.ronghui.star_chess_jump;
                self.ronghui.star_chess_jump =
                    (self.ronghui.star_chess_jump - jump_value).max(0);
            }
            // DongZhu (fate-cycle) while (:2014-2030): consecutive fate grids with
            // opening, 1 per skip. Inner-while batching preserves fc621a 4→2.
            while self.should_fate_cycle_skip(current) {
                self.fate.fate_cycle = (self.fate.fate_cycle - 1).max(0);
                skipped_slot_indices.push(current);
                skipped_opening_slot_indices.push(current);
                self.deck.queue.push(current);
                flag = true;
                if skipped_slot_indices.len().saturating_sub(last_break_progress)
                    >= active_slot_count
                {
                    return Some(DrawnCard {
                        source_slot: current,
                        card: basic_attack_card().into(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid,
                    });
                }
                match self.shift_next_non_skip(
                    &mut skipped_slot_indices,
                    active_slot_count,
                    last_break_progress,
                    &mut steps,
                    max_draw_steps,
                ) {
                    Ok(slot) => current = slot,
                    Err(Some(fallback_slot)) => {
                        return Some(DrawnCard {
                            source_slot: fallback_slot,
                            card: basic_attack_card().into(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid,
                        });
                    }
                    Err(None) => return None,
                }
            }
            // ShunYingJi while (:2031-2047) not implemented (no fixture hits it).
            if !flag {
                let card = self.deck.slots[current].card.clone();
                return Some(DrawnCard {
                    source_slot: current,
                    card,
                    fallback_basic_attack: false,
                    skipped_slots: skipped_slot_indices,
                    skipped_opening_slots: skipped_opening_slot_indices,
                    fate_398_skipped_fifth_grid,
                });
            }
            // Outer repeats without extra pop (current already shifted); steps
            // bound pops inside shift_next_non_skip, no extra check here.
        }
    }

    pub(super) fn peek_unskipped_next_slot(
        &self,
        nameless_white_deer_skip_limit: i64,
    ) -> Option<usize> {
        let active_slot_count = self.deck.queue.len();
        let &source_slot = self.deck.queue.first()?;
        let slot = self.deck.slots.get(source_slot)?;
        let base_id = normalized_base_id(&slot.card);
        let simple =
            !slot.skipped
                && self.astrology.star_chess_break <= 0
                && !(matches!(base_id, 350 | 9_000_015)
                    && source_slot + 2 >= active_slot_count
                    && !slot.used)
                && (self.dream_mirage.calamity_skip_mask & (1_i64 << source_slot)) == 0
                && !(base_id == 202 && source_slot + 1 >= active_slot_count)
                && !(source_slot == 4 && self.identity.fate_strategies.contains(&398))
                && !(source_slot == 4 && self.xian_mo_switch_active(7116))
                && !(source_slot >= 5 && self.xian_mo_switch_active(5010))
                && !(self.fate.fate_cycle > 0
                    && self.fate.fate_cycle_slots.iter().any(|&cycle_slot| {
                        cycle_slot > 0 && cycle_slot == (source_slot + 1) as i64
                    }))
                && self.ronghui.star_chess_jump <= 0
                && !(nameless_white_deer_skip_limit > 0
                    && (source_slot as i64) < nameless_white_deer_skip_limit
                    && slot
                        .card
                        .card_type
                        .as_ref()
                        .is_some_and(|card_type| card_type.value == super::CARD_TYPE_SUSTAIN));
        simple.then_some(source_slot)
    }

    fn should_fate_cycle_skip(&mut self, source_slot: usize) -> bool {
        if self.fate.fate_cycle <= 0 {
            return false;
        }
        let slot_index = (source_slot + 1) as i64;
        self.fate
            .fate_cycle_slots
            .iter()
            .copied()
            .any(|slot| slot > 0 && slot == slot_index)
    }

    pub(super) fn complete_drawn_card(&mut self, drawn: &DrawnCard, skipped: bool) {
        if let Some(slot) = self.deck.slots.get_mut(drawn.source_slot) {
            if drawn.fallback_basic_attack {
                slot.card = basic_attack_card().into();
                slot.skipped = false;
            } else {
                slot.skipped = skipped;
                slot.used = true;
            }
        }
        self.deck.queue.push(drawn.source_slot);
    }

    pub(super) fn complete_drawn_card_with_jump(
        &mut self,
        drawn: &DrawnCard,
        skipped: bool,
        jump_distance: i64,
    ) {
        if let Some(slot) = self.deck.slots.get_mut(drawn.source_slot) {
            if drawn.fallback_basic_attack {
                slot.card = basic_attack_card().into();
                slot.skipped = false;
            } else {
                slot.skipped = skipped;
                slot.used = true;
            }
        }
        self.deck.queue.insert(0, drawn.source_slot);
        self.right_move_card_queue(jump_distance.max(0));
    }

    pub(super) fn right_move_card_queue(&mut self, distance: i64) {
        if distance <= 0 {
            return;
        }
        let mut moved_active_cards = 0_i64;
        let mut rotations = 0_i64;
        let limit = (self.deck.queue.len() as i64) * (distance + 1).max(1);
        while moved_active_cards < distance && rotations < limit {
            let Some(slot_index) = self.deck.queue.pop() else {
                break;
            };
            self.deck.queue.insert(0, slot_index);
            rotations += 1;
            if self
                .deck
                .slots
                .get(slot_index)
                .is_some_and(|slot| !slot.skipped)
            {
                moved_active_cards += 1;
            }
        }
    }

    pub(super) fn return_card_to_front(&mut self, source_slot: usize) {
        self.deck.queue.insert(0, source_slot);
    }

    pub(super) fn return_card_to_tail(&mut self, source_slot: usize) {
        self.deck.queue.push(source_slot);
    }

    pub(super) fn shift_next_raw_card(&mut self) -> Option<DrawnCard> {
        let active_slot_count = self.deck.queue.len();
        if active_slot_count == 0 {
            return None;
        }
        let mut skipped_slot_indices = Vec::new();
        for _ in 0..active_slot_count {
            let source_slot = self.deck.queue.remove(0);
            let was_skipped = self
                .deck
                .slots
                .get(source_slot)
                .is_some_and(|slot| slot.skipped);
            if was_skipped {
                skipped_slot_indices.push(source_slot);
                self.deck.queue.push(source_slot);
                continue;
            }
            let card = self.deck.slots[source_slot].card.clone();
            return Some(DrawnCard {
                source_slot,
                card,
                fallback_basic_attack: false,
                skipped_slots: skipped_slot_indices,
                skipped_opening_slots: Vec::new(),
                fate_398_skipped_fifth_grid: false,
            });
        }
        Some(DrawnCard {
            source_slot: 0,
            card: basic_attack_card().into(),
            fallback_basic_attack: true,
            skipped_slots: skipped_slot_indices,
            skipped_opening_slots: Vec::new(),
            fate_398_skipped_fifth_grid: false,
        })
    }

    pub(super) fn tick_turn_end_statuses(&mut self) -> TurnEndStatusDecay {
        let mut decay = TurnEndStatusDecay::default();
        if self.mirage_ronghui.cannot_gain_hp > 0 {
            self.mirage_ronghui.cannot_gain_hp -= 1;
        }
        if self.status.weakness > 0 {
            self.status.weakness -= 1;
            decay.weakness = 1;
        }
        if self.status.flaw > 0 {
            self.status.flaw -= 1;
            self.apply_flaw_loss_hooks(1);
            decay.flaw = 1;
        }
        if self.status.entangle > 0 {
            self.status.entangle -= 1;
            decay.entangle = 1;
        }
        if self.turn.action_again_blocked_turns > 0 {
            self.turn.action_again_blocked_turns -= 1;
        }
        decay
    }

    pub(super) fn apply_flaw_loss_hooks(&mut self, amount: i64) {
        if amount > 0 && self.status.drunken_fist_stance > 0 {
            self.gain_attack_bonus_local(amount * self.status.drunken_fist_stance);
        }
    }

    pub(super) fn gain_attack_bonus_local(&mut self, mut amount: i64) -> i64 {
        if amount <= 0 {
            return 0;
        }
        if self.identity.fate_strategies.contains(&148)
            || self.identity.xian_mo_strategies.contains(&7035)
        {
            self.elements.wood_thorn += 1;
            amount -= 1;
        }
        let converted = amount.min(self.dream_mirage.attack_bonus_to_thorns.max(0));
        if converted > 0 {
            self.elements.wood_thorn += converted;
        }
        let attack_bonus = amount - converted;
        if attack_bonus > 0 {
            self.core.attack_bonus += attack_bonus;
        }
        attack_bonus
    }

    pub(super) fn apply_adaptation_boost(&self, delta: i64) -> i64 {
        if delta > 0 && self.turn.adaptation > 0 {
            delta + div_ceil(delta * 40, 100)
        } else {
            delta
        }
    }

    pub(super) fn healing_max_hp_gain(&self) -> i64 {
        const TALENT_GAINS: &[(i64, i64)] = &[(120, 3), (10_120, 4), (20_120, 5), (30_120, 6)];
        TALENT_GAINS
            .iter()
            .filter(|(talent, _)| self.identity.talents.contains(talent))
            .map(|(_, gain)| *gain)
            .sum()
    }

    fn hp_change_ke_yin_defense(&self) -> i64 {
        // KeYinCardConfig 50147 拂清风 has otherParams[0] = 1. The original
        // GetTotalKeYinOtherparam sums every matching battle entry.
        self.identity
            .ke_yin_card_ids
            .iter()
            .filter(|&&card_id| card_id == 50_147)
            .count() as i64
    }

    pub(super) fn apply_max_hp_delta_raw(&mut self, delta: i64) -> i64 {
        let before = self.core.max_hp;
        self.core.max_hp = (self.core.max_hp + delta).max(0);
        let actual_delta = self.core.max_hp - before;
        if self.core.hp > self.core.max_hp {
            self.core.hp = self.core.max_hp;
        }
        actual_delta
    }

    fn apply_hp_delta_raw(&mut self, delta: i64, is_cost: bool) -> HpMutationReceipt {
        let requested = delta;
        let before = self.core.hp;
        self.core.hp = if is_cost {
            self.core.hp + delta
        } else {
            (self.core.hp + delta).min(self.core.max_hp)
        };
        let actual_delta = self.core.hp - before;
        let ledger = if !is_cost && delta > 0 {
            delta
        } else if actual_delta < 0 {
            actual_delta
        } else {
            0
        };
        if ledger > 0 {
            self.hp_mutation.add_hp_count += ledger;
            self.dream_mirage.turn_hp_gained += ledger;
            self.dream_mirage.hp_gain_event_count += 1;
        }
        HpMutationReceipt {
            requested,
            resolved: delta,
            applied: actual_delta,
            ledger,
            prevention: None,
        }
    }
}

impl<const OBSERVE: bool> ReplayState<OBSERVE> {
    pub(super) fn mutate_actor_hp(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
        is_cost: bool,
        ignore_guard: bool,
    ) -> HpMutationReceipt {
        let before = self.actor(actor_side).core.hp;
        let add_hp_before = self.actor(actor_side).hp_mutation.add_hp_count;
        let hp_events_before = self.actor(actor_side).dream_mirage.hp_gain_event_count;
        let hp_gained_before = self.actor(actor_side).dream_mirage.turn_hp_gained;
        let receipt = self.mutate_actor_hp_inner(actor_side, delta, is_cost, ignore_guard);
        if receipt.ledger > 0 && self.actor(actor_side).fate.wild_ferry_seal > 0 {
            self.actor_mut(actor_side).fate.wild_ferry_seal -= 1;
            self.modify_extra_actions(actor_side, 1);
        }
        self.record_mutation_receipt(
            actor_side,
            super::ReplayMutationKind::Hp,
            "核心",
            "hp",
            "生命",
            before,
            self.actor(actor_side).core.hp,
            receipt.applied,
        );
        // The three ledger fields are written inside the runtime layer
        // (`apply_hp_delta_raw`) in lockstep with `ledger`; derive their
        // receipts from it so a new ledger side effect cannot silently escape
        // the observation contract.
        if receipt.ledger > 0 {
            self.record_counter_transition(
                actor_side,
                "回合",
                "addHpCount",
                "累计获得生命",
                add_hp_before,
                add_hp_before + receipt.ledger,
            );
            self.record_counter_transition(
                actor_side,
                "回合",
                "hpGained",
                "本回合获得生命",
                hp_gained_before,
                hp_gained_before + receipt.ledger,
            );
            self.record_counter_transition(
                actor_side,
                "回合",
                "hpGainEventCount",
                "获得生命次数",
                hp_events_before,
                hp_events_before + 1,
            );
        }
        receipt
    }

    fn mutate_actor_hp_inner(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
        is_cost: bool,
        ignore_guard: bool,
    ) -> HpMutationReceipt {
        let requested = delta;
        let mut delta = delta;
        // 雪羽清风 FateStrategy 394（BattleCharacter.ModifyHp 正向分支）:
        // 每次正向生命请求额外 +1。IsSwitchActive 的 tempData 缺失/0 视为开启，
        // 非零关闭；成本、负向生命变化及被拒绝的变化不进入该分支。
        if !is_cost
            && delta > 0
            && (self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&394))
            && self
                .actor(actor_side)
                .identity
                .fate_strategy_temp_datas
                .get("394")
                .copied()
                .unwrap_or(0)
                == 0
        {
            delta += 1;
        }
        if !is_cost
            && delta > 0
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&7076)
        {
            delta += 1;
        }
        // BattleCharacter.ModifyHp:9600-9603. HP requests do not consume the
        // marker; the owner's turn-start phase decrements it.
        if !is_cost
            && delta < 0
            && self
                .actor(actor_side)
                .hp_mutation
                .no_hp_loss_before_next_turn
                > 0
        {
            delta = 0;
        }
        if !is_cost && delta > 0 {
            if self.actor(actor_side).mirage_ronghui.cannot_gain_hp > 0 {
                return HpMutationReceipt::prevented(requested);
            }
            if self
                .actor(actor_side)
                .mirage_ronghui
                .mirage_healing_conversion_turns
                > 0
            {
                self.gain_sharpness(actor_side, delta);
                return HpMutationReceipt::prevented(requested);
            }
            // The verified current build globally permits healing to revive,
            // but JinZhiFuHuo overrides both that flag and explicit permission.
            if self.actor(actor_side).core.hp <= 0
                && self.actor(actor_side).chance.cannot_revive > 0
            {
                return HpMutationReceipt::prevented(requested);
            }
        }
        if !is_cost && delta < 0 && self.actor(actor_side).core.guard > 0 && !ignore_guard {
            let temporary_guard = self.actor(actor_side).core.temporary_guard;
            self.modify_guard(actor_side, -1);
            self.actor_mut(actor_side).core.temporary_guard = (temporary_guard - 1).max(0);
            // 一层护体抵挡整笔损失，所以记的是 delta（已过 no-hp-loss 标记）而不是 requested：
            // requested 在被标记吞掉时是 0，会把"什么都没挡"记成挡了一大笔。
            if OBSERVE {
                self.actor_mut(actor_side)
                    .prevention
                    .hp_loss_prevented_by_guard += (-delta).max(0);
            }
            return HpMutationReceipt::prevented_by(requested, super::HpMutationPrevention::Guard);
        }
        if !is_cost && delta < 0 && self.actor(actor_side).fate.graft_flowers_to_tree > 0 {
            self.actor_mut(actor_side).fate.graft_flowers_to_tree -= 1;
            let mut receipt = self.mutate_actor_hp(actor_side, -delta, false, ignore_guard);
            receipt.requested = requested;
            return receipt;
        }
        if !is_cost && delta < 0 {
            delta = self.apply_dream_mirage_hp_loss_modifier(actor_side, delta);
            if self.actor(actor_side).fate.leaf_shield_flower > 0 {
                let defense_spent = (-delta / 2).min(self.actor(actor_side).core.defense).max(0);
                if defense_spent > 0 {
                    self.lose_defense(actor_side, defense_spent);
                    delta += defense_spent;
                }
            }
        }
        let mut bloom_fills_max_hp = false;
        if !is_cost && delta > 0 {
            // BattleCharacter.ModifyHp:9650-9673. Egg-yolk zongzi increases
            // Appetite before every positive modifier reads it; red-date
            // zongzi is one-shot.
            if self.actor(actor_side).hp_mutation.egg_yolk_zongzi > 0 {
                self.actor_mut(actor_side).hp_mutation.appetite += 1;
                self.actor_mut(actor_side).hp_mutation.egg_yolk_zongzi -= 1;
            }
            let red_date_zongzi = self.actor(actor_side).hp_mutation.red_date_zongzi;
            if red_date_zongzi > 0 {
                delta += red_date_zongzi;
                self.actor_mut(actor_side).hp_mutation.red_date_zongzi = 0;
            }
            delta = (delta + self.actor(actor_side).hp_mutation.appetite.max(0)
                - self.actor(actor_side).status.lost_mind.max(0))
            .max(1);
            delta = self.actor(actor_side).apply_adaptation_boost(delta);

            let max_hp_gain = self.actor(actor_side).healing_max_hp_gain();
            if max_hp_gain > 0 {
                self.modify_actor_max_hp(actor_side, max_hp_gain);
            }
            let overflow = super::cards_synthetic_oracle_verified_secret_misc::vitality_bloom_overflow_max_hp_gain(
                self.actor(actor_side),
                delta,
            );
            if overflow > 0 {
                self.modify_actor_max_hp(actor_side, overflow);
                // ModifyHp:10061-10072 sets hp to the new max even when
                // ShunYing amplified the bloom's ModifyMaxHp request.
                bloom_fills_max_hp = true;
            }
        }
        let mut receipt = self
            .actor_mut(actor_side)
            .apply_hp_delta_raw(delta, is_cost);
        receipt.requested = requested;
        if bloom_fills_max_hp {
            let extra = self.actor(actor_side).core.max_hp - self.actor(actor_side).core.hp;
            self.actor_mut(actor_side).core.hp += extra;
            receipt.applied += extra;
        }
        self.apply_after_hp_modify_pipeline(actor_side, receipt.effect_delta());
        // ModifyHp:10107 precedes LingYuanWaiFang:10112. Both can re-enter
        // the target's defense hooks, so their order affects subsequent hits.
        if !is_cost && receipt.resolved > 0 {
            self.apply_heavenly_secret_reverse_from_gain(actor_side, receipt.resolved);
        }
        if !is_cost
            && receipt.resolved > receipt.applied
            && self.actor(actor_side).astrology.ling_yuan_wai_fang > 0
        {
            self.apply_reflect_damage(actor_side, receipt.resolved - receipt.applied);
        }
        if !is_cost && receipt.resolved > 0 {
            self.apply_dream_mirage_positive_resource_gain_damage(
                actor_side,
                receipt.resolved,
                false,
            );
        }
        if is_cost
            && receipt.applied < 0
            && (self.actor(actor_side).identity.talent_resonance_id == Some(50))
        {
            self.modify_actor_hp(actor_side, 2, false, false);
        }
        if is_cost
            && receipt.applied < 0
            && (self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&149))
        {
            self.apply_physique_amount(actor_side, 1);
        }
        if is_cost
            && receipt.applied < 0
            && self
                .actor(actor_side)
                .identity
                .xian_mo_strategies
                .contains(&7102)
        {
            self.apply_physique_amount(actor_side, 1);
        }
        receipt
    }

    /// The single executable entry for BattleCharacter.AfterHpModifyEffect.
    fn apply_after_hp_modify_pipeline(&mut self, actor_side: PlayerSide, actual_delta: i64) {
        static_phases! { phase in ORIGINAL_AFTER_HP_MODIFY_PHASES [0, 1, 2, 3, 4, 5, 6, 7, 8, 9] {
            match phase {
                AfterHpModifyPhase::SpiritTurtleFootwork => {
                    let value = self.actor(actor_side).turn.spirit_turtle_footwork.max(0);
                    if actual_delta < 0
                        && value > 0
                        && self.actor(actor_side).turn.spirit_turtle_footwork_triggered <= 0
                    {
                        self.actor_mut(actor_side)
                            .turn
                            .spirit_turtle_footwork_triggered = 1;
                        self.gain_anima(actor_side, value);
                        self.gain_agility(actor_side, value);
                    }
                }
                AfterHpModifyPhase::FirstHpLossReward => {
                    let reward = self
                        .actor(actor_side)
                        .mirage_ronghui
                        .first_hp_loss_reward
                        .max(0);
                    if actual_delta < 0
                        && reward > 0
                        && self
                            .actor(actor_side)
                            .mirage_ronghui
                            .first_hp_loss_reward_triggered
                            <= 0
                    {
                        self.actor_mut(actor_side)
                            .mirage_ronghui
                            .first_hp_loss_reward_triggered = 1;
                        self.gain_anima(actor_side, reward);
                        self.gain_agility(actor_side, reward);
                    }
                }
                AfterHpModifyPhase::Talent64Defense => {
                    if actual_delta != 0 && self.actor(actor_side).identity.talents.contains(&64) {
                        self.gain_defense(actor_side, 1);
                    }
                }
                AfterHpModifyPhase::KeYin50147Defense => {
                    if actual_delta != 0 {
                        let defense = self.actor(actor_side).hp_change_ke_yin_defense();
                        if defense > 0 {
                            self.gain_defense(actor_side, defense);
                        }
                    }
                }
                AfterHpModifyPhase::IceSnowLotus => {
                    if actual_delta < 0 && self.actor(actor_side).fate.ice_snow_lotus > 0 {
                        self.actor_mut(actor_side).fate.ice_snow_lotus -= 1;
                        self.gain_defense(actor_side, -actual_delta);
                    }
                }
                AfterHpModifyPhase::DreamCliff => {
                    self.apply_dream_mirage_hp_loss_hooks(actor_side, actual_delta);
                }
                AfterHpModifyPhase::BloodCalamity => {
                    if actual_delta < 0 && self.actor(actor_side).status.blood_calamity > 0 {
                        // 原版 BattleCharacter.cs:10182-10186：
                        // ModifyBuffValue(BuffType.XueGuangZhiZai, -1) 先于
                        // ModifyBuffValue(BuffType.WaiShang, 1)。
                        // 必须先扣减血光之灾层数，再走负面状态共享管线（避免负面状态
                        // 触发反伤等二次扣血重入 ModifyHp 时递归无限触发）。
                        self.modify_blood_calamity(actor_side, -1);
                        self.add_actor_negative_status(actor_side, 105, 1);
                    }
                }
                AfterHpModifyPhase::HpLossAttackCharge => {
                    if actual_delta < 0
                        && self
                            .actor(actor_side)
                            .mirage_ronghui
                            .hp_loss_attack_bonus_charges
                            > 0
                    {
                        self.gain_attack_bonus(actor_side, 1);
                        self.actor_mut(actor_side)
                            .mirage_ronghui
                            .hp_loss_attack_bonus_charges -= 1;
                    }
                }
                AfterHpModifyPhase::YanQi => {
                    if actual_delta > 0 {
                        self.apply_yan_qi_healing(actor_side);
                    }
                }
                AfterHpModifyPhase::HpLossLedgers => {
                    if actual_delta < 0 {
                        let times_before = self.actor(actor_side).turn.lose_hp_times_count;
                        let count_before = self.actor(actor_side).turn.lose_hp_count;
                        self.actor_mut(actor_side).turn.lose_hp_times_count += 1;
                        self.actor_mut(actor_side).turn.lose_hp_count += -actual_delta;
                        self.record_counter_transition(
                            actor_side,
                            "回合",
                            "loseHpTimesCount",
                            "失去生命次数",
                            times_before,
                            self.actor(actor_side).turn.lose_hp_times_count,
                        );
                        self.record_counter_transition(
                            actor_side,
                            "回合",
                            "loseHpCount",
                            "累计失去生命",
                            count_before,
                            self.actor(actor_side).turn.lose_hp_count,
                        );
                    }
                }
            }
        } }
    }

    /// Shared YanQi continuation for both ModifyHp and ModifyMaxHp. The nested
    /// healing must re-enter the public HP pipeline; raw vitals mutation is not
    /// an executable hook surface.
    pub(super) fn apply_yan_qi_healing(&mut self, actor_side: PlayerSide) {
        if self.actor(actor_side).fate.yan_qi <= 0 {
            return;
        }
        self.actor_mut(actor_side).fate.yan_qi -= 1;
        let healing = self.actor(actor_side).core.max_hp * 20 / 100;
        if healing > 0 {
            self.modify_actor_hp(actor_side, healing, false, false);
        }
    }

    pub(super) fn modify_actor_hp(
        &mut self,
        actor_side: PlayerSide,
        delta: i64,
        is_cost: bool,
        ignore_guard: bool,
    ) -> i64 {
        self.mutate_actor_hp(actor_side, delta, is_cost, ignore_guard)
            .effect_delta()
    }
}
