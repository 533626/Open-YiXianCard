//! Projects internal replay state into the observation surface.
//!
//! `snapshot` feeds every ReplayEvent and the parity comparison; `detail_entries`
//! feeds the Detailed observation mode consumed by the UI and TUI. Neither may
//! change battle state: they only read what the kernel already settled.

use super::{
    Element, ReplayDetailEntry, ReplayPlayer, ReplayPlayerSnapshot, ReplayUiCardSlotSnapshot,
    ReplayUiPlayerSnapshot,
};

impl ReplayPlayer {
    pub(super) fn snapshot(&self) -> ReplayPlayerSnapshot {
        ReplayPlayerSnapshot {
            hp: self.core.hp,
            max_hp: self.core.max_hp,
            defense: self.core.defense,
            anima: self.core.anima,
            guard: self.core.guard,
            physique: self.core.physique,
            sword_intent: self.sword.sword_intent,
            sharpness: self.sword.sharpness,
            cloud_chain: self.sword.cloud_chain,
            cloud_sea: self.sword.cloud_sea,
            momentum: self.beng.momentum,
            agility: self.turn.agility,
            water_momentum: self.elements.water_momentum,
            activated_metal: self.elements.activated_metal,
            activated_water: self.elements.activated_water,
            activated_wood: self.elements.activated_wood,
            activated_fire: self.elements.activated_fire,
            activated_earth: self.elements.activated_earth,
            hexagram: self.astrology.hexagram,
            star_power: self.astrology.star_power,
            attack_bonus: self.core.attack_bonus,
            internal_injury: self.status.internal_injury,
            weakness: self.status.weakness,
            flaw: self.status.flaw,
            attack_reduction: self.status.attack_reduction,
            entangle: self.status.entangle,
            external_injury: self.status.external_injury,
            lost_mind: self.status.lost_mind,
            action_again_count: self.turn.action_again_count,
            quan_stance: self.beng.quan_stance,
            gun_stance: self.beng.gun_stance,
            // 全量暴露缺口（档 1a/1b）：与 detail_entries 同口径，见
            // build_display_gap_report.py 锚定表（原版 RefreshBuff 会显示）。
            metal_ring: self.sword.metal_ring,
            sword_energy: self.sword.sword_energy,
            water_month_sword_formation: self.sword.water_month_sword_formation,
            water_formation: self.elements.water_formation,
            metal_formation: self.elements.metal_formation,
            earth_formation: self.elements.earth_formation,
            fire_formation: self.elements.fire_formation,
            spring_flow: self.elements.spring_flow,
            water_stealth: self.elements.water_stealth,
            metal_iron_bone: self.elements.metal_iron_bone,
            earth_eight_wastes: self.elements.earth_eight_wastes,
            wood_array: self.elements.wood_array,
            turtle_formation: self.formations.turtle_formation,
            shatter_formation: self.formations.shatter_formation,
            thunder_formation: self.formations.thunder_formation,
            evil_gu_formation: self.formations.evil_gu_formation,
            spirit_gathering_formation: self.formations.spirit_gathering_formation,
            heaven_cycle_sword_formation: self.formations.heaven_cycle_sword_formation,
            heaven_force_formation: self.formations.heaven_force_formation,
            flower_maze_formation: self.formations.flower_maze_formation,
            immovable_formation: self.formations.immovable_formation,
            eight_gates_formation: self.formations.eight_gates_formation,
            six_yao_formation: self.formations.six_yao_formation,
            beng_quan_cun_jin: self.beng.beng_quan_cun_jin,
            beng_quan_return_profound: self.beng.beng_quan_return_profound,
            dream_beng_quan_chain: self.beng.dream_beng_quan_chain,
            immortal_binding_tune: self.music.immortal_binding_tune,
            illusory_tune: self.music.illusory_tune,
            heartbreak_tune: self.music.heartbreak_tune,
            wild_dance_tune: self.music.wild_dance_tune,
            rejuvenation_tune: self.music.rejuvenation_tune,
            xiaoyao_tune: self.music.xiaoyao_tune,
            xiaoyao_guqin: self.music.xiaoyao_guqin,
            chaotic_mind_tune: self.music.chaotic_mind_tune,
            ling_gua_art: self.astrology.ling_gua_art,
            star_moon_fan: self.astrology.star_moon_fan,
            infinite_hexagram_plate: self.astrology.infinite_hexagram_plate,
            all_goes_well: self.astrology.all_goes_well,
            recovery: self.status.recovery,
            meditation: self.status.meditation,
            blood_calamity: self.status.blood_calamity,
            lone_night_wolf: self.status.lone_night_wolf,
            leaf_blade_flower: self.status.leaf_blade_flower,
            quiet_mindset: self.fate.quiet_mindset,
            reflect_mindset: self.fate.reflect_mindset,
            graft_flowers_to_tree: self.fate.graft_flowers_to_tree,
            tide: self.fate.tide,
            dismantle_move: self.fate.dismantle_move,
            all_things_inauspicious: self.fate.all_things_inauspicious,
            fate_cycle: self.fate.fate_cycle,
            yellow_bird_behind: self.fate.yellow_bird_behind,
            exorcism: self.fate.exorcism,
            ice_snow_lotus: self.fate.ice_snow_lotus,
            leaf_shield_flower: self.fate.leaf_shield_flower,
            paint_finishing_touch: self.fate.paint_finishing_touch,
            next_turn_defense: self.turn.next_turn_defense,
            ignore_defense_attacks: self.turn.ignore_defense_attacks,
            next_attack_shatter_defense: self.turn.next_attack_shatter_defense,
        }
    }

    pub(super) fn ui_snapshot(&self) -> ReplayUiPlayerSnapshot {
        ReplayUiPlayerSnapshot {
            parity: self.snapshot(),
            momentum_limit: self.beng.momentum_limit,
            last_element: self.elements.last_element.map(|element| match element {
                Element::Metal => "metal",
                Element::Water => "water",
                Element::Wood => "wood",
                Element::Fire => "fire",
                Element::Earth => "earth",
            }),
            card_queue: self.deck.queue.clone(),
            slots: self
                .deck
                .slots
                .iter()
                .enumerate()
                .map(|(index, slot)| ReplayUiCardSlotSnapshot {
                    index,
                    card_id: slot.card.id,
                    base_id: slot.card.base_id.unwrap_or(slot.card.id),
                    name: slot.card.name.clone(),
                    skipped: slot.skipped,
                    had_used: slot.used,
                })
                .collect(),
        }
    }

    pub(super) fn detail_entries(&self) -> Vec<ReplayDetailEntry> {
        let mut entries = Vec::new();
        push_nonzero(&mut entries, "核心", "hp", "生命", self.core.hp);
        push_nonzero(&mut entries, "核心", "maxHp", "生命上限", self.core.max_hp);
        push_nonzero(
            &mut entries,
            "核心",
            "tempLife",
            "命元",
            self.core.temp_life,
        );
        push_nonzero(&mut entries, "核心", "defense", "防", self.core.defense);
        push_nonzero(&mut entries, "核心", "anima", "灵气", self.core.anima);
        push_nonzero(&mut entries, "核心", "guard", "护体", self.core.guard);
        push_nonzero(
            &mut entries,
            "核心",
            "attackBonus",
            "加攻",
            self.core.attack_bonus,
        );
        push_nonzero(&mut entries, "核心", "physique", "体魄", self.core.physique);
        push_nonzero(
            &mut entries,
            "核心",
            "physiqueLimit",
            "体魄上限",
            self.core.physique_limit,
        );

        push_nonzero(
            &mut entries,
            "剑系",
            "swordIntent",
            "剑意",
            self.sword.sword_intent,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "swordEnergy",
            "剑气",
            self.sword.sword_energy,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "sharpness",
            "锋锐",
            self.sword.sharpness,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "metalRing",
            "锟铻金环",
            self.sword.metal_ring,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "cloudChain",
            "连云",
            self.sword.cloud_chain,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "cloudSea",
            "云海",
            self.sword.cloud_sea,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "frenzySword",
            "狂剑",
            self.sword.frenzy_sword,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "swordFormationCount",
            "剑阵计数",
            self.sword.sword_formation_count,
        );
        push_nonzero(
            &mut entries,
            "剑系",
            "waterMonthSwordFormation",
            "水月剑阵",
            self.sword.water_month_sword_formation,
        );

        push_nonzero(
            &mut entries,
            "五行",
            "waterMomentum",
            "水势",
            self.elements.water_momentum,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "waterFormation",
            "水灵阵",
            self.elements.water_formation,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "metalFormation",
            "金灵阵",
            self.elements.metal_formation,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "earthFormation",
            "土灵阵",
            self.elements.earth_formation,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "fireFormation",
            "火灵阵",
            self.elements.fire_formation,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "springFlow",
            "泉涌",
            self.elements.spring_flow,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "waterStealth",
            "潜遁",
            self.elements.water_stealth,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "metalIronBone",
            "铁骨",
            self.elements.metal_iron_bone,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "earthEightWastes",
            "合八荒",
            self.elements.earth_eight_wastes,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "woodArray",
            "木灵阵",
            self.elements.wood_array,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "activatedMetal",
            "激活金灵",
            self.elements.activated_metal,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "activatedWater",
            "激活水灵",
            self.elements.activated_water,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "activatedWood",
            "激活木灵",
            self.elements.activated_wood,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "activatedFire",
            "激活火灵",
            self.elements.activated_fire,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "activatedEarth",
            "激活土灵",
            self.elements.activated_earth,
        );
        push_nonzero(
            &mut entries,
            "五行",
            "fiveElementsGourd",
            "五行玉瓶",
            self.elements.five_elements_gourd,
        );

        push_nonzero(
            &mut entries,
            "阵法",
            "turtleFormation",
            "龟甲阵",
            self.formations.turtle_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "shatterFormation",
            "碎杀阵",
            self.formations.shatter_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "thunderFormation",
            "引雷阵",
            self.formations.thunder_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "evilGuFormation",
            "邪蛊阵",
            self.formations.evil_gu_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "spiritGatheringFormation",
            "聚灵阵",
            self.formations.spirit_gathering_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "heavenCycleSwordFormation",
            "周天剑阵",
            self.formations.heaven_cycle_sword_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "heavenForceFormation",
            "天罡聚力阵",
            self.formations.heaven_force_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "flowerMazeFormation",
            "万花迷魂阵",
            self.formations.flower_maze_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "immovableFormation",
            "不动金刚阵",
            self.formations.immovable_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "eightGatesFormation",
            "八门金锁阵",
            self.formations.eight_gates_formation,
        );
        push_nonzero(
            &mut entries,
            "阵法",
            "sixYaoFormation",
            "六爻煞阵",
            self.formations.six_yao_formation,
        );

        push_nonzero(&mut entries, "锻玄", "momentum", "气势", self.beng.momentum);
        push_nonzero(
            &mut entries,
            "锻玄",
            "momentumLimit",
            "气势上限",
            self.beng.momentum_limit,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "quanStance",
            "拳架势",
            self.beng.quan_stance,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "gunStance",
            "棍架势",
            self.beng.gun_stance,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "bengQuanCunJin",
            "崩拳寸劲",
            self.beng.beng_quan_cun_jin,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "bengQuanDefense",
            "崩拳加防",
            self.beng.beng_quan_defense,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "bengQuanReturnProfound",
            "崩拳返玄",
            self.beng.beng_quan_return_profound,
        );
        push_nonzero(
            &mut entries,
            "锻玄",
            "dreamBengQuanChain",
            "梦崩拳连崩",
            self.beng.dream_beng_quan_chain,
        );

        push_nonzero(
            &mut entries,
            "琴曲",
            "immortalBindingTune",
            "天音困仙曲",
            self.music.immortal_binding_tune,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "illusoryTune",
            "幻音曲",
            self.music.illusory_tune,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "heartbreakTune",
            "断肠曲",
            self.music.heartbreak_tune,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "wildDanceTune",
            "狂舞曲",
            self.music.wild_dance_tune,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "rejuvenationTune",
            "回春曲",
            self.music.rejuvenation_tune,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "xiaoyaoTune",
            "逍遥曲",
            self.music.xiaoyao_tune,
        );
        push_nonzero(
            &mut entries,
            "琴",
            "xiaoyaoGuqin",
            "逍遥古琴",
            self.music.xiaoyao_guqin,
        );
        push_nonzero(
            &mut entries,
            "琴曲",
            "chaoticMindTune",
            "万魔蚀心曲",
            self.music.chaotic_mind_tune,
        );

        push_nonzero(
            &mut entries,
            "卦星",
            "hexagram",
            "卦象",
            self.astrology.hexagram,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "hexagramEffectiveCount",
            "卦象生效次数",
            self.astrology.hexagram_effective_count,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "lingGuaArt",
            "灵卦术",
            self.astrology.ling_gua_art,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "starPower",
            "星力",
            self.astrology.star_power,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "starMoonFan",
            "星月折扇",
            self.astrology.star_moon_fan,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "starChessBreak",
            "星弈断",
            self.astrology.star_chess_break,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "infiniteHexagramPlate",
            "无极卦盘",
            self.astrology.infinite_hexagram_plate,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "allGoesWell",
            "万事如意",
            self.astrology.all_goes_well,
        );
        push_nonzero(
            &mut entries,
            "卦星",
            "starErosion",
            "星蚀",
            self.astrology.star_erosion,
        );

        push_nonzero(
            &mut entries,
            "状态",
            "externalInjury",
            "外伤",
            self.status.external_injury,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "internalInjury",
            "内伤",
            self.status.internal_injury,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "flameHeartUrging",
            "灯焰催心",
            self.status.flame_heart_urging,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "recovery",
            "恢复",
            self.status.recovery,
        );
        // 原版 BuffType.Min = 367（冥），BuffConfig 分类 Negative；
        // 旧 label「冥想」未锚定原版枚举，按 AGENTS.md 原文优先改为「冥」。
        push_nonzero(
            &mut entries,
            "状态",
            "meditation",
            "冥",
            self.status.meditation,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "weakness",
            "虚弱",
            self.status.weakness,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "attackReduction",
            "减攻",
            self.status.attack_reduction,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "cannotAct",
            "无法行动",
            self.status.cannot_act,
        );
        push_nonzero(&mut entries, "状态", "flaw", "破绽", self.status.flaw);
        push_nonzero(
            &mut entries,
            "状态",
            "entangle",
            "困缚",
            self.status.entangle,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "lostMind",
            "食滞",
            self.status.lost_mind,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "bloodCalamity",
            "血光之灾",
            self.status.blood_calamity,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "loneNightWolf",
            "孤夜狼",
            self.status.lone_night_wolf,
        );
        push_nonzero(
            &mut entries,
            "状态",
            "leafBladeFlower",
            "叶刃花",
            self.status.leaf_blade_flower,
        );

        push_nonzero(
            &mut entries,
            "仙命",
            "quietMindset",
            "静气心法",
            self.fate.quiet_mindset,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "reflectMindset",
            "反震心法",
            self.fate.reflect_mindset,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "graftFlowersToTree",
            "移花接木",
            self.fate.graft_flowers_to_tree,
        );
        push_nonzero(&mut entries, "仙命", "tide", "海潮", self.fate.tide);
        push_nonzero(
            &mut entries,
            "仙命",
            "dismantleMove",
            "拆招",
            self.fate.dismantle_move,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "allThingsInauspicious",
            "诸事不宜",
            self.fate.all_things_inauspicious,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "fateCycle",
            "命运轮回",
            self.fate.fate_cycle,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "qiXingLianZhu",
            "七星连珠",
            self.fate.qi_xing_lian_zhu,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "yellowBirdBehind",
            "黄雀在后",
            self.fate.yellow_bird_behind,
        );
        push_nonzero(&mut entries, "仙命", "exorcism", "辟邪", self.fate.exorcism);
        push_nonzero(
            &mut entries,
            "仙命",
            "iceSnowLotus",
            "冰封雪莲",
            self.fate.ice_snow_lotus,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "leafShieldFlower",
            "叶盾花",
            self.fate.leaf_shield_flower,
        );
        push_nonzero(
            &mut entries,
            "仙命",
            "paintFinishingTouch",
            "画龙点睛",
            self.fate.paint_finishing_touch,
        );

        push_nonzero(
            &mut entries,
            "回合",
            "lostDefenseCount",
            "失防次数",
            self.turn.lost_defense_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "nextTurnDefense",
            "下回合加防",
            self.turn.next_turn_defense,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "usedCardCount",
            "已用牌数",
            self.turn.used_card_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "ignoreDefenseAttacks",
            "无视防御",
            self.turn.ignore_defense_attacks,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "nextAttackShatterDefense",
            "下次攻击碎防",
            self.turn.next_attack_shatter_defense,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "extraActions",
            "再次行动",
            self.turn.extra_actions,
        );
        push_nonzero(&mut entries, "回合", "agility", "身法", self.turn.agility);
        push_nonzero(
            &mut entries,
            "回合",
            "actionAgainCount",
            "再次行动次数",
            self.turn.action_again_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "battlePhysiqueGainCount",
            "本场体魄增加",
            self.turn.battle_physique_gain_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "addHpCount",
            "累计获得生命",
            self.hp_mutation.add_hp_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "hpGainEventCount",
            "获得生命次数",
            self.dream_mirage.hp_gain_event_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "loseHpCount",
            "累计失去生命",
            self.turn.lose_hp_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "loseHpTimesCount",
            "失去生命次数",
            self.turn.lose_hp_times_count,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "actualDamageCarry",
            "实际伤害",
            self.turn.actual_damage_carry,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "woundedCountCarry",
            "击伤计数",
            self.turn.wounded_count_carry,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "jiLuZongJiShangZhi",
            "累计总击伤",
            self.turn.ji_lu_zong_ji_shang_zhi,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "hpGained",
            "已加生命",
            self.dream_mirage.turn_hp_gained,
        );
        push_nonzero(
            &mut entries,
            "回合",
            "nextCardAnimaCostReduction",
            "下张牌减耗",
            self.turn.next_card_anima_cost_reduction,
        );
        entries
    }
}

fn push_nonzero(
    entries: &mut Vec<ReplayDetailEntry>,
    group: &'static str,
    key: &'static str,
    label: &'static str,
    value: i64,
) {
    if value != 0 {
        entries.push(ReplayDetailEntry {
            group,
            key,
            label,
            value,
        });
    }
}
