use super::original_config::{complete_with_original_card, upgrade_original_card};
use super::support::{
    adapt_fixture_card_for_replay, basic_attack_card, div_ceil, is_talent_52_replacement_slot,
    normalize_base_id, normalized_base_id, permanent_physique_key, seven_stars_stabilize_soul_card,
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

#[allow(dead_code)]
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

    pub(super) fn from_fixture(
        side: PlayerSide,
        fixture: &FixturePlayer,
        historical_overrides: &[HistoricalCardOverride],
    ) -> Self {
        let mut hp = fixture.base_max_hp + fixture.extra_max_hp.unwrap_or(0);
        let max_hp_base = hp;
        // FateStrategy 140 (天衍-猛虎之躯) 的 hp/10 增益在开局钩子结算
        // （apply_battle_start_opening_effects）中按当时当前生命计算：
        // 先手方开局伤害（如 11_000_001 天谕系）先扣血，140 再按扣后生命
        // 采样（oracle 锚点：mirror-32299000 608126353bbde8d4/round-10 cp0
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
        let talent_physique = if fixture.talents.contains(&183) {
            if fixture.fate_strategies.contains(&166) {
                2
            } else {
                1
            }
        } else {
            0
        };
        let initial_physique = permanent_physique + talent_physique;
        let initial_momentum_limit = fixture.initial_momentum_limit.unwrap_or(6).max(0);
        let initial_temp_life = fixture.last_round_life.unwrap_or(max_hp_base).max(0);
        let initial_defense = fixture.initial_defense.max(0);
        // 筋骨健壮（176 系）的战斗开始加血（ModifyHp(floor(TiPo / 7)) 等）在
        // battle_start.rs 的 OnBattleStarted 天赋循环位置采样当前体魄执行，
        // 不在构造期预置：原版 case 176（BattleCharacter.cs IL_1411）读取的是
        // 该天赋在天赋循环中的实时 TiPo，入冥（179）先触发 415 疯魔架势
        // ModifyTiPo(+1) 再被 20176 采样（oracle 锚点：4e59d5176f512748/round-14
        // cp0 p2 10 = 70/7，构造期 69/7 = 9 少 1 血）。
        let endless_staff_stance =
            fixture.character_id == Some(4_000_005) && fixture.fate_strategies.contains(&335);
        let initial_quan_stance = if endless_staff_stance {
            0
        } else if fixture.talents.contains(&204) || fixture.character_id == Some(4_000_005) {
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
        if initial_meditation > 0 && character_id != Some(4_000_003) {
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
                fate_strategies: fixture.fate_strategies.clone(),
                fate_strategy_temp_datas: fixture.fate_strategy_temp_datas.clone(),
                talents: fixture.talents.clone(),
                talent_resonance_id: fixture.talent_resonance_id,
                talent_resonance_temp_flags: Vec::new(),
                ke_yin_card_ids: fixture.used_ke_yin_cards.clone(),
                last_round_exp: fixture.last_round_exp,
                talent_199_card_ids: fixture
                    .talent_card_params
                    .get("199")
                    .cloned()
                    .unwrap_or_default(),
            },
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
                physique_limit: fixture
                    .permanent_buff_temp_datas
                    .get("10024")
                    .copied()
                    .unwrap_or(5),
                lost_max_hp_count: 0,
            },
            sword: ReplaySwordState {
                sword_intent: initial_sword_intent,
                sword_energy: 0,
                // FateStrategy 333's metal activation + +2 sharpness now
                // route through ReplayState::activate_element/gain_sharpness
                // in battle_start.rs (post-construction), matching engine-ts
                // activateElement's shared side-effect hooks.
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
            },
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
            },
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
            },
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
                quan_stance: initial_quan_stance,
                gun_stance: initial_gun_stance,
            },
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
            },
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
                star_slots: vec![2, 5],
            },
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
                cu_ju_fei_xi: 0,     // 同上（416 促局飞袭）
                wu_you_ling_niang: 0, // 同上（407 无忧灵酿）
                wu_jing_gua_yan: 0,  // 同上（395 无尽卦衍）
                xing_yuan: 0,        // 同上（402 星缘）
                ke_yin_jing_lei: 0,  // 同上（396 惊雷破敌）
                fate_cycle: 0,
                fate_cycle_slots: [0, 0],
                plum_blossom_twice: 0,
                qi_xing_lian_zhu: 0,
                yellow_bird_behind: 0,
                reverse_card_direction: 0,
                sword_formation_guard: 0,
                next_rear_move_bypass: 0,
                used_rear_move_check: 0,
                rear_move_succeeded: false,
                exorcism: 0,
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
            },
            chance: ReplayChanceState::default(),
            dream_mirage: super::ReplayDreamMirageState {
                action_again_limit: 1,
                ..Default::default()
            },
            mirage_ronghui: super::ReplayMirageRonghuiState::default(),
            ronghui: super::ReplayRonghuiState {
                five_elements_bottle_card_id: fixture
                    .talent_card_params
                    .get("199")
                    .and_then(|cards| cards.first())
                    .copied()
                    .unwrap_or(0),
                ..Default::default()
            },
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
                ignore_weakness_attacks: 0,
                next_attack_shatter_defense: 0,
                attack_segments_performed: 0,
                extra_actions: 0,
                agility: fixture.initial_agility.max(0),
                action_again_count: 0,
                wind_spirit_body_forge_count: 0,
                battle_physique_gain_count: talent_physique.max(0),
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
                        ),
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
    pub(super) fn draw_next_card(
        &mut self,
        nameless_white_deer_skip_limit: i64,
    ) -> Option<DrawnCard> {
        let active_slot_count = self.deck.queue.len();
        if active_slot_count == 0 {
            return None;
        }
        let mut skipped_slot_indices = Vec::new();
        let mut skipped_opening_slot_indices = Vec::new();
        let mut fate_398_skipped_fifth_grid = false;
        for _ in 0..active_slot_count {
            let source_slot = self.deck.queue.remove(0);
            let was_skipped = self
                .deck
                .slots
                .get(source_slot)
                .is_some_and(|slot| slot.skipped);
            // 原版 BattleExecuter.cs:1751-1755 先消费 XingYi_Duan 跳过当前牌，
            // 再执行 BattleExecuter.cs:1796-1800 的空间灵田跳过；顺序不能交换。
            let skip_star_chess_break = !was_skipped && self.astrology.star_chess_break > 0;
            if skip_star_chess_break {
                self.astrology.star_chess_break = (self.astrology.star_chess_break - 1).max(0);
                skipped_slot_indices.push(source_slot);
                if skipped_slot_indices.len() == active_slot_count {
                    return Some(DrawnCard {
                        source_slot,
                        card: basic_attack_card(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid: false,
                    });
                }
                self.deck.queue.push(source_slot);
                continue;
            }
            let skip_space_spirit_field = self.deck.slots.get(source_slot).is_some_and(|slot| {
                matches!(normalized_base_id(&slot.card), 350 | 9_000_015)
                    && source_slot + 2 >= active_slot_count
                    && !slot.used
            });
            if skip_space_spirit_field {
                self.deck.slots[source_slot].used = true;
            }
            let skip_calamity = !was_skipped
                && (self.dream_mirage.calamity_skip_mask & (1_i64 << source_slot)) != 0;
            if skip_calamity {
                self.dream_mirage.calamity_skip_mask &= !(1_i64 << source_slot);
                skipped_slot_indices.push(source_slot);
                self.deck.queue.push(source_slot);
                continue;
            }
            let skip_five_elements_spirit_field =
                self.deck.slots.get(source_slot).is_some_and(|slot| {
                    normalized_base_id(&slot.card) == 202 && source_slot + 1 >= active_slot_count
                });
            if skip_five_elements_spirit_field {
                self.deck.slots[source_slot].used = true;
                skipped_slot_indices.push(source_slot);
                if skipped_slot_indices.len() == active_slot_count {
                    return Some(DrawnCard {
                        source_slot,
                        card: basic_attack_card(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid: false,
                    });
                }
                self.deck.queue.push(source_slot);
                continue;
            }
            // FateStrategy 398（老鼠偷油）在开关启用时跳过自身卡组第 5 格；
            // BattleExecuter.cs:1857-1864 只在该格是星弈牌时先加
            // otherParams[0]（5）生命，实际加血由 ReplayState 在取牌后结算。
            // 与原版一样，这个跳过是按 gridNumber 判断，不依赖该格是否已被
            // 其他跳过机制先轮到。只有本分支跳掉第 5 格才算 FS398 触发——
            // 若第 5 格先被更靠前的星弈断等机制跳掉，原版 while 链重入时
            // FS398 循环看到的是新 currentCard，根本不会加血。已带 skip 标记
            // （用过的持续/消耗牌）的牌在原版 ShiftCard 里先被过滤，同样
            // 不会进入 FS398 循环。
            let skip_rat_oil = source_slot == 4
                && !was_skipped
                && self.identity.fate_strategies.contains(&398)
                && self
                    .identity
                    .fate_strategy_temp_datas
                    .get("398")
                    .copied()
                    .unwrap_or(0)
                    == 0;
            if skip_rat_oil {
                fate_398_skipped_fifth_grid = true;
                skipped_slot_indices.push(source_slot);
                if skipped_slot_indices.len() == active_slot_count {
                    return Some(DrawnCard {
                        source_slot,
                        card: basic_attack_card(),
                        fallback_basic_attack: true,
                        skipped_slots: skipped_slot_indices,
                        skipped_opening_slots: skipped_opening_slot_indices,
                        fate_398_skipped_fifth_grid: true,
                    });
                }
                self.deck.queue.push(source_slot);
                continue;
            }
            let skip_fate_cycle = !was_skipped
                && !skip_space_spirit_field
                && self.should_fate_cycle_skip(source_slot);
            if skip_fate_cycle {
                self.fate.fate_cycle = (self.fate.fate_cycle - 1).max(0);
            }
            if !was_skipped && !skip_space_spirit_field {
                if skip_fate_cycle {
                    skipped_slot_indices.push(source_slot);
                    skipped_opening_slot_indices.push(source_slot);
                    if skipped_slot_indices.len() == active_slot_count {
                        return Some(DrawnCard {
                            source_slot,
                            card: basic_attack_card(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid: false,
                        });
                    }
                    self.deck.queue.push(source_slot);
                    continue;
                }
                let ronghui_star_jump = self.ronghui.star_chess_jump > 0;
                let is_star_chess = self
                    .deck
                    .slots
                    .get(source_slot)
                    .is_some_and(|slot| slot.card.name.contains("星弈"));
                if ronghui_star_jump && !is_star_chess {
                    skipped_slot_indices.push(source_slot);
                    skipped_opening_slot_indices.push(source_slot);
                    if skipped_slot_indices.len() == active_slot_count {
                        return Some(DrawnCard {
                            source_slot,
                            card: basic_attack_card(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid: false,
                        });
                    }
                    self.deck.queue.push(source_slot);
                    continue;
                }
                if ronghui_star_jump && is_star_chess {
                    self.ronghui.star_chess_jump = 0;
                }
                let skip_nameless_white_deer = nameless_white_deer_skip_limit > 0
                    && (source_slot as i64) < nameless_white_deer_skip_limit
                    && self.deck.slots.get(source_slot).is_some_and(|slot| {
                        slot.card
                            .card_type
                            .as_ref()
                            .is_some_and(|card_type| card_type.value == super::CARD_TYPE_SUSTAIN)
                    });
                if skip_nameless_white_deer {
                    skipped_slot_indices.push(source_slot);
                    if skipped_slot_indices.len() == active_slot_count {
                        return Some(DrawnCard {
                            source_slot,
                            card: basic_attack_card(),
                            fallback_basic_attack: true,
                            skipped_slots: skipped_slot_indices,
                            skipped_opening_slots: skipped_opening_slot_indices,
                            fate_398_skipped_fifth_grid: false,
                        });
                    }
                    self.deck.queue.push(source_slot);
                    continue;
                }
                let card = self.deck.slots[source_slot].card.clone();
                return Some(DrawnCard {
                    source_slot,
                    card,
                    fallback_basic_attack: false,
                    skipped_slots: skipped_slot_indices,
                    skipped_opening_slots: skipped_opening_slot_indices,
                    fate_398_skipped_fifth_grid,
                });
            }
            skipped_slot_indices.push(source_slot);
            skipped_opening_slot_indices.push(source_slot);
            if skipped_slot_indices.len() == active_slot_count {
                return Some(DrawnCard {
                    source_slot,
                    card: basic_attack_card(),
                    fallback_basic_attack: true,
                    skipped_slots: skipped_slot_indices,
                    skipped_opening_slots: skipped_opening_slot_indices,
                    fate_398_skipped_fifth_grid: false,
                });
            }
            self.deck.queue.push(source_slot);
        }
        None
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
                slot.card = basic_attack_card();
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
                slot.card = basic_attack_card();
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
        if self.fate.last_stand_unyielding > 0 {
            self.fate.last_stand_unyielding = 0;
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
        if self.identity.fate_strategies.contains(&148) {
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

impl ReplayState {
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
        // 天髓葫芦-style extra-action grant used to be a raw field write
        // inside the runtime layer; it now lives here (receipt-capable layer)
        // so `extraActions` changes are recorded with the same attribution
        // context as the hp mutation that triggered them.
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
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&394)
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
            self.actor_mut(actor_side)
                .prevention
                .hp_loss_prevented_by_guard += (-delta).max(0);
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
            }
        }
        let mut receipt = self
            .actor_mut(actor_side)
            .apply_hp_delta_raw(delta, is_cost);
        receipt.requested = requested;
        self.apply_after_hp_modify_pipeline(actor_side, receipt.applied);
        if !is_cost && receipt.resolved > 0 {
            self.apply_heavenly_secret_reverse_from_gain(actor_side, receipt.resolved);
            self.apply_dream_mirage_positive_resource_gain_damage(
                actor_side,
                receipt.resolved,
                false,
            );
        }
        if is_cost
            && receipt.applied < 0
            && self.actor(actor_side).identity.talent_resonance_id == Some(50)
        {
            self.modify_actor_hp(actor_side, 2, false, false);
        }
        if is_cost
            && receipt.applied < 0
            && self
                .actor(actor_side)
                .identity
                .fate_strategies
                .contains(&149)
        {
            self.apply_physique_amount(actor_side, 1);
        }
        receipt
    }

    /// The single executable entry for BattleCharacter.AfterHpModifyEffect.
    fn apply_after_hp_modify_pipeline(&mut self, actor_side: PlayerSide, actual_delta: i64) {
        for phase in ORIGINAL_AFTER_HP_MODIFY_PHASES {
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
                        // 原版 BattleCharacter.cs:9902-9905 走
                        // ModifyBuffValue(WaiShang, 1)，必须经过负面状态共享管线；
                        // 首次负面状态会在这里消耗星蚀，而不能延后到同张牌随后
                        // 显式发放的虚弱。
                        self.add_actor_negative_status(actor_side, 105, 1);
                        self.modify_blood_calamity(actor_side, -1);
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
        }
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
            .applied
    }
}
