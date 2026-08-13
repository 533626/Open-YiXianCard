use super::*;

#[test]
fn lava_seal_opening_activates_fire_before_instant_burn_samples_action_again() {
    let lava_seal = original_card_definition_by_id(56).expect("missing lava seal");
    let instant_burn =
        original_card_definition_by_id(7_000_038).expect("missing fire spirit instant burn");
    let mut fixture = fixture(
        deck_with_cards(vec![lava_seal, instant_burn]),
        deck(basic_attack()),
    );
    fixture.players.p1.active_slot_count = 2;

    let mut state = ReplayState::test_from_fixture(&fixture);
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Fire));
    assert!(state
        .p1
        .elements
        .activated_elements
        .contains(&Element::Earth));

    state.p1.deck.queue = vec![1];
    assert!(state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.turn.action_again_count, 1);
}

#[test]
fn fortune_avoid_misfortune_skips_opening_effect_cards_like_spirit_detection() {
    let mut fortune = card(11_000_007, 11_000_007, "天运•避凶");
    fortune.other_params = vec![2, 3, 2];
    let spirit_detection = card(11_010_009, 11_000_009, "探灵");
    let mut fixture = fixture(
        deck_with_cards(vec![fortune, spirit_detection]),
        deck(basic_attack()),
    );
    fixture.players.p1.active_slot_count = 2;
    fixture.max_actor_turns = Some(2);
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.core.max_hp = 40;
    state.p1.core.hp = 20;

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.formations.fortune_avoid_misfortune, 1);
    assert_eq!(state.p1.core.defense, 3);
    assert_eq!(state.p1.core.hp, 22);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.formations.fortune_avoid_misfortune, 1);
    assert_eq!(state.p1.core.defense, 3);
    assert_eq!(state.p1.core.hp, 22);
}

#[test]
fn qi_swallow_mountains_uses_max_hp_modifier_with_adaptation() {
    let mut qi_swallow = card(4_010_060, 4_000_060, "气吞山河");
    qi_swallow.other_params = vec![16, 31];
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(qi_swallow.clone()), deck(basic_attack())));
    state.p1.turn.adaptation = 1;

    state.test_apply_card_effect(PlayerSide::P1, &qi_swallow, 0);

    assert_eq!(state.p1.core.max_hp, 53);
}

#[test]
fn water_accepts_all_rivers_applies_adaptation_to_max_hp_and_healing() {
    let mut accepts_all_rivers = card(7_000_057, 7_000_057, "水灵•纳百川");
    accepts_all_rivers.other_params = vec![3, 6];
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck(accepts_all_rivers.clone()),
        deck(basic_attack()),
    ));
    state.p1.turn.adaptation = 1;
    state.activate_element(PlayerSide::P1, Element::Water);

    state.test_apply_card_effect(PlayerSide::P1, &accepts_all_rivers, 0);

    assert_eq!(state.p1.elements.water_momentum, 3);
    assert_eq!(state.p1.core.max_hp, 43);
    assert_eq!(state.p1.core.hp, 43);
}

#[test]
fn water_spirit_sea_dragon_roar_gains_momentum_and_stops_the_opponent() {
    let mut sea_dragon_roar = card(7_000_044, 7_000_044, "水灵•海龙啸");
    sea_dragon_roar.other_params = vec![4];
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck(sea_dragon_roar.clone()),
        deck(basic_attack()),
    ));
    state.p1.elements.water_momentum = 5;
    state.activate_element(PlayerSide::P1, Element::Water);

    state.test_apply_card_effect(PlayerSide::P1, &sea_dragon_roar, 0);

    assert_eq!(state.p1.elements.water_momentum, 9);
    assert_eq!(state.p2.status.cannot_act, 1);
}

#[test]
fn thunder_hexagram_uses_original_hexagram_and_self_resolution_when_payload_lacks_params() {
    let mut thunder_hexagram = card(4_010_003, 4_000_003, "震卦");
    thunder_hexagram.attack = Some(7);
    let mut fixture = fixture(deck(thunder_hexagram), deck(basic_attack()));
    fixture.players.p1.talents = vec![197];
    fixture.players.p1.last_round_used_card_base_ids = vec![4_000_001, 4_000_002];

    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.astrology.hexagram, 1);
    assert_eq!(state.p2.status.flaw, 4);
    assert_eq!(state.p2.core.hp, 23);
}

#[test]
fn metal_spirit_returning_blade_refunded_sharpness_uses_metal_ring_bonus() {
    let mut returning_blade = card(7_000_099, 7_000_099, "金灵•回锋刃");
    returning_blade.attack = Some(6);
    returning_blade.attack_count = Some(2);
    returning_blade.other_params = vec![60];
    let mut fixture = fixture(deck(returning_blade.clone()), deck(basic_attack()));
    fixture.players.p2.base_max_hp = 250;
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.test_configure_p1(|player| {
        player.sword.sharpness = 89;
        player.sword.metal_ring = 3; // 锟铻金环
    });
    state.activate_element(PlayerSide::P1, Element::Metal); // 激活金灵

    state.test_apply_card_effect(PlayerSide::P1, &returning_blade, 0);

    assert_eq!(state.p2.core.hp, 92);
    assert_eq!(state.p1.sword.sharpness, 38);
}

#[test]
fn metal_spirit_returning_blade_checks_wound_after_weakness_multiplier() {
    let mut one_segment = card(7_010_099, 7_000_099, "金灵•回锋刃");
    one_segment.attack = Some(6);
    one_segment.attack_count = Some(1);
    one_segment.other_params = vec![60];
    let mut first =
        ReplayState::test_from_fixture(&fixture(deck(one_segment.clone()), deck(basic_attack())));
    first.p1.status.weakness = 3;
    first.p1.sword.sharpness = 54;
    first.p2.core.defense = 5;
    first.activate_element(PlayerSide::P1, Element::Metal);

    first.test_apply_card_effect(PlayerSide::P1, &one_segment, 0);

    assert_eq!(first.p2.core.defense, 2);
    assert_eq!(first.p2.core.hp, 30);
    assert_eq!(first.p1.sword.sharpness, 54);

    let mut three_segments = one_segment.clone();
    three_segments.attack_count = Some(3);
    let mut full = ReplayState::test_from_fixture(&fixture(
        deck(three_segments.clone()),
        deck(basic_attack()),
    ));
    full.p1.status.weakness = 3;
    full.p1.sword.sharpness = 54;
    full.p1.sword.metal_ring = 3;
    full.p2.core.hp = 95;
    full.p2.core.max_hp = 109;
    full.p2.core.defense = 5;
    full.activate_element(PlayerSide::P1, Element::Metal);

    full.test_apply_card_effect(PlayerSide::P1, &three_segments, 0);

    assert_eq!(full.p2.core.defense, 0);
    assert_eq!(full.p2.core.hp, 1);
    assert_eq!(full.p1.sword.sharpness, 25);
}

#[test]
fn all_goes_well_minimum_is_applied_before_sharpness_wound_resolution() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.astrology.all_goes_well = 1;
    state.p1.sword.sharpness = 4;
    state.p2.core.defense = 5;

    let hp_lost = state.apply_attack(PlayerSide::P1, 1, 0);

    assert_eq!(hp_lost, 5);
    assert_eq!(state.p2.core.hp, 25);
    assert_eq!(state.p2.core.defense, 0);
    assert_eq!(state.p1.sword.sharpness, 0);
}

#[test]
fn water_blade_multiplier_does_not_turn_a_blocked_attack_into_a_wound() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.elements.water_blade_seal = 1;
    state.p1.sword.sharpness = 6;
    state.p2.core.defense = 5;

    let hp_lost = state.apply_attack(PlayerSide::P1, 4, 0);

    assert_eq!(hp_lost, 0);
    assert_eq!(state.p2.core.hp, 30);
    assert_eq!(state.p2.core.defense, 1);
    assert_eq!(state.p1.sword.sharpness, 6);
}

#[test]
fn fire_blade_talent_forces_sharpness_past_remaining_defense() {
    let mut source = fixture(deck(basic_attack()), deck(basic_attack()));
    source.players.p1.talents = vec![67];
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 20;
    state.p2.core.defense = 10;

    let hp_lost = state.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(hp_lost, 4);
    assert_eq!(state.p2.core.hp, 16);
    assert_eq!(state.p2.core.max_hp, 29);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.sword.sharpness, 0);
}

#[test]
fn forced_wound_consumes_sharpness_even_when_guard_blocks_the_hp_request() {
    let mut source = fixture(deck(basic_attack()), deck(basic_attack()));
    source.players.p1.talents = vec![67];
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 20;
    state.p2.core.defense = 10;
    state.p2.core.guard = 1;

    let hp_lost = state.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(hp_lost, 0);
    assert_eq!(state.p2.core.hp, 20);
    assert_eq!(state.p2.core.max_hp, 29);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p2.core.guard, 0);
    assert_eq!(state.p1.sword.sharpness, 0);
}

#[test]
fn long_ma_spirit_forces_sharpness_past_remaining_defense() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.elements.long_ma_spirit = 1;
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 20;
    state.p2.core.defense = 10;

    let hp_lost = state.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(hp_lost, 4);
    assert_eq!(state.p2.core.hp, 16);
    assert_eq!(state.p2.core.max_hp, 30);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.sword.sharpness, 0);
}

#[test]
fn next_attack_wound_bonus_is_post_defense_and_consumed_on_forced_wound() {
    let mut state =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    state.p1.turn.next_attack_wound_bonus = 2;
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 20;
    state.p2.core.defense = 10;

    let hp_lost = state.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(hp_lost, 6);
    assert_eq!(state.p2.core.hp, 14);
    assert_eq!(state.p2.core.max_hp, 30);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.sword.sharpness, 0);
    assert_eq!(state.p1.turn.next_attack_wound_bonus, 0);
}

#[test]
fn graft_flowers_blocks_ordinary_wound_bonuses_but_not_forced_sharpness_selection() {
    let mut ordinary =
        ReplayState::test_from_fixture(&fixture(deck(basic_attack()), deck(basic_attack())));
    ordinary.p1.sword.sharpness = 4;
    ordinary.p1.elements.water_blade_seal = 1;
    ordinary.p2.core.hp = 10;
    ordinary.p2.status.external_injury = 2;
    ordinary.p2.fate.graft_flowers_to_tree = 1;

    ordinary.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(ordinary.p2.core.hp, 13);
    assert_eq!(ordinary.p1.sword.sharpness, 4);
    assert_eq!(ordinary.p2.fate.graft_flowers_to_tree, 0);

    let mut forced_source = fixture(deck(basic_attack()), deck(basic_attack()));
    forced_source.players.p1.talents = vec![67];
    let mut forced = ReplayState::test_from_fixture(&forced_source);
    forced.p1.sword.sharpness = 4;
    forced.p2.core.hp = 10;
    forced.p2.fate.graft_flowers_to_tree = 1;

    forced.apply_attack(PlayerSide::P1, 3, 0);

    assert_eq!(forced.p2.core.hp, 17);
    assert_eq!(forced.p2.core.max_hp, 29);
    assert_eq!(forced.p1.sword.sharpness, 0);
    assert_eq!(forced.p2.fate.graft_flowers_to_tree, 0);
}

#[test]
fn adjacent_dream_anima_infusion_forces_wound_then_clears_on_card_completion() {
    let mut infusion = card(1_020_067, 1_000_067, "梦•灵气灌注");
    infusion.other_params = vec![10];
    let mut source = fixture(
        deck_with_cards(vec![basic_attack(), infusion, basic_attack()]),
        deck(basic_attack()),
    );
    source.players.p1.active_slot_count = 3;
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.sword.sharpness = 4;
    state.p2.core.hp = 20;
    state.p2.core.defense = 10;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 16);
    assert_eq!(state.p2.core.defense, 7);
    assert_eq!(state.p1.sword.sharpness, 0);
    assert_eq!(state.p1.turn.guaranteed_wound, 0);
}

#[test]
fn mirage_qi_drawing_sword_executes_selected_temporary_card_then_reads_post_anima() {
    let mut drawing = card(262, 262, "幻•引气剑");
    drawing.attack = Some(3);
    drawing.attack_count = Some(1);
    drawing.other_params = vec![1, 1];
    let mut source = fixture(deck(drawing.clone()), deck(basic_attack()));
    source.decision_tape = vec![1_000_027];
    source.players.p1.talents = vec![67, 68];
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.core.anima = 3;
    state.p1.core.defense = 14;
    state.p2.core.hp = 119;
    state.p2.core.max_hp = 119;
    state.p2.core.defense = 100;

    state.test_apply_card_effect(PlayerSide::P1, &drawing, 0);

    assert_eq!(state.p1.core.anima, 5);
    assert_eq!(state.p1.core.defense, 16);
    assert_eq!(state.p2.core.max_hp, 117);
    assert_eq!(state.p1.turn.attack_segments_performed, 2);
}

#[test]
fn permanent_exorcism_grass_prevents_shura_roar_internal_injury() {
    let mut shura_roar = card(100_000_041, 10_000_041, "修罗吼");
    shura_roar.other_params = vec![4, 2];
    let mut fixture = fixture(deck(shura_roar), deck(basic_attack()));
    fixture
        .players
        .p2
        .permanent_buff_temp_datas
        .insert("10019".to_string(), 3);

    let mut state = ReplayState::test_from_fixture(&fixture);
    assert_eq!(state.p2.fate.exorcism, 3);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.status.internal_injury, 4);
    assert_eq!(state.p2.status.internal_injury, 1);
    assert_eq!(state.p2.fate.exorcism, 0);
}

#[test]
fn five_elements_cycle_jindan_and_huashen_branches_trigger_on_generated_element() {
    let fire_seal = card(7_000_009, 7_000_009, "火灵印");
    let mut fixture = fixture(deck(fire_seal.clone()), deck(basic_attack()));
    fixture.players.p1.talents = vec![10_102, 30_102]; // 五行循环：灵气 / 锋锐分支
    let mut state = ReplayState::test_from_fixture(&fixture);
    state.p1.elements.last_element = Some(Element::Wood);

    state.apply_selected_card_hooks(PlayerSide::P1, &fire_seal, 0);

    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.sword.sharpness, 4);
}

#[test]
fn five_elements_cycle_temporary_card_inherits_dynamic_action_again_written_by_body() {
    let water = original_card_definition_by_id(7_000_006).expect("missing water seal");
    let cycle = original_card_definition_by_id(7_000_067).expect("missing five elements cycle");
    let patrol = original_card_definition_by_id(7_000_028).expect("missing wood patrol");
    let mut source = fixture(
        deck_with_cards(vec![water, cycle, patrol]),
        deck(basic_attack()),
    );
    source.players.p1.active_slot_count = 3;
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.deck.queue = vec![1, 2, 0];
    state.activate_element(PlayerSide::P1, Element::Wood);
    state.p1.hp_mutation.add_hp_count = 1;

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again);
    assert_eq!(state.p1.turn.extra_actions, 0);

    let mut reverse = ReplayState::test_from_fixture(&source);
    reverse.p1.deck.queue = vec![1, 2, 0];
    reverse.p1.fate.reverse_card_direction = 1;
    reverse.activate_element(PlayerSide::P1, Element::Wood);
    reverse.p1.hp_mutation.add_hp_count = 1;

    assert!(!reverse.test_execute_one_card(PlayerSide::P1));
}

#[test]
fn dream_weakness_talisman_applies_weakness_then_consumes_and_acts_again() {
    let talisman = original_card_definition_by_id(387).expect("missing dream weakness talisman");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(talisman), deck(basic_attack())));

    let action_again = state.test_execute_one_card(PlayerSide::P1);

    assert!(action_again);
    assert_eq!(state.p2.status.weakness, 1);
    assert_eq!(state.p1.turn.action_again_count, 1);
    assert!(state.p1.deck.slots[0].used);
    assert!(state.p1.deck.slots[0].skipped);
}

#[test]
fn mirage_flying_star_spike_attacks_before_star_gain_and_only_reacts_off_star_slot() {
    let spike = original_card_definition_by_id(294).expect("missing mirage flying star spike");
    let mut free_state =
        ReplayState::test_from_fixture(&fixture(deck(spike.clone()), deck(basic_attack())));

    assert!(free_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(free_state.p2.core.hp, 25);
    assert_eq!(free_state.p1.astrology.star_power, 1);

    let mut star_state =
        ReplayState::test_from_fixture(&fixture(deck(spike), deck(basic_attack())));
    star_state.p1.astrology.star_slots = vec![0];
    star_state.p1.astrology.star_power = 2;

    assert!(!star_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(star_state.p2.core.hp, 23);
    assert_eq!(star_state.p1.astrology.star_power, 3);
}

#[test]
fn dream_scoop_moon_requires_rear_move_and_huashen_rear_slot_adds_attack() {
    let scoop = original_card_definition_by_id(4_000_076).expect("missing dream scoop moon");
    let mut first_state =
        ReplayState::test_from_fixture(&fixture(deck(scoop), deck(basic_attack())));

    assert!(!first_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(first_state.p2.core.hp, 30);
    assert!(!first_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(first_state.p2.core.hp, 16);

    let huashen =
        original_card_definition_by_id(4_030_076).expect("missing huashen dream scoop moon");
    let mut rear_deck = vec![basic_attack(), basic_attack(), basic_attack(), huashen];
    rear_deck.resize_with(DECK_SIZE, basic_attack);
    let mut rear_state = ReplayState::test_from_fixture(&fixture(rear_deck, deck(basic_attack())));
    rear_state.p1.deck.queue = vec![3];
    rear_state.p1.deck.slots[3].used = true;

    assert!(rear_state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(rear_state.p2.core.hp, 20);
}

#[test]
fn anima_body_forge_uses_post_gain_anima_then_applies_configured_and_scaled_physique() {
    let body_forge = original_card_definition_by_id(205).expect("missing anima body forge");
    let mut body_fixture = fixture(deck(body_forge), deck(basic_attack()));
    body_fixture.players.p1.base_max_hp = 50;
    body_fixture.players.p1.initial_anima = 8;
    body_fixture
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10024".to_string(), 5);
    let mut state = ReplayState::test_from_fixture(&body_fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 11);
    assert_eq!(state.p1.core.physique, 13);
    assert_eq!(state.p1.core.max_hp, 63);
    assert_eq!(state.p1.core.hp, 58);
}

#[test]
fn mirage_double_ghost_attacks_twice_then_applies_three_statuses_to_both_sides() {
    let double_ghost =
        original_card_definition_by_id(296).expect("missing mirage double ghost knock gate");
    let mut double_fixture = fixture(deck(double_ghost), deck(basic_attack()));
    double_fixture.players.p1.base_max_hp = 50;
    double_fixture.players.p1.initial_anima = 1;
    double_fixture.players.p2.base_max_hp = 50;
    let mut state = ReplayState::test_from_fixture(&double_fixture);

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p2.core.hp, 38);
    for player in [&state.p1, &state.p2] {
        assert_eq!(player.status.internal_injury, 2);
        assert_eq!(player.status.external_injury, 1);
        assert_eq!(player.status.weakness, 1);
    }
}

#[test]
fn base_seven_stars_soul_talent_raises_max_hp_by_three_before_each_heal() {
    let gourd = original_card_definition_by_id(13).expect("missing carefree gourd");
    let mut talent_fixture = fixture(deck(gourd), deck(basic_attack()));
    talent_fixture.players.p1.talents = vec![120];
    let mut state = ReplayState::test_from_fixture(&talent_fixture);
    state.p1.core.hp = 20;

    assert!(!state.test_execute_one_card(PlayerSide::P1));
    assert_eq!(state.p1.core.anima, 1);
    assert_eq!(state.p1.core.max_hp, 33);
    assert_eq!(state.p1.core.hp, 23);
}

#[test]
fn frenzy_dragon_swallows_cloud_makes_cloud_and_frenzy_swords_cross_count() {
    let dragon = original_card_definition_by_id(10_050).expect("missing 狂龙吞云 level 2");
    // Steam build 24466094 raised the three levels from 0/1/2 to 1/2/3.
    // Card_50 consumes this config value directly after installing the
    // cross-count sustain buff, so the level-2 card now counts two frenzy swords.
    assert_eq!(dragon.other_params, vec![2]);
    let cloud_sword = card(3, 3, "云剑•崩雪");
    let frenzy_sword = card(2, 2, "狂剑•炎舞");
    let mut fixture = fixture(
        deck_with_cards(vec![dragon, cloud_sword, frenzy_sword]),
        deck(basic_attack()),
    );
    fixture.players.p1.active_slot_count = 3;
    fixture.max_actor_turns = Some(3);
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.frenzy_dragon_swallows_cloud, 1);
    assert_eq!(state.p1.sword.frenzy_sword, 2);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.cloud_chain, 1);
    assert_eq!(state.p1.sword.frenzy_sword, 3);

    state.test_execute_one_card(PlayerSide::P1);
    assert_eq!(state.p1.sword.cloud_chain, 2);
    assert_eq!(state.p1.sword.frenzy_sword, 4);
}

#[test]
fn combined_sword_formation_counts_other_formations_and_312_frenzy_branch() {
    let mut combined = card(1_000_062, 1_000_062, "合势剑阵");
    combined.attack = Some(4);
    combined.defense = Some(4);
    combined.other_params = vec![3, 3];
    let water_moon = card(1_000_041, 1_000_041, "水月剑阵");
    let hidden_frenzy = card(312, 312, "幻•狂剑盘龙");
    let frenzy = card(1_000_022, 1_000_022, "狂剑•一式");
    let fixture = fixture(
        deck_with_cards(vec![combined, water_moon, hidden_frenzy, frenzy]),
        deck(basic_attack()),
    );
    let mut state = ReplayState::test_from_fixture(&fixture);

    state.test_execute_one_card(PlayerSide::P1);

    // 合势剑阵计数其他剑阵 + 312 分支的非 hidden 狂剑。312 幻•狂剑盘龙自身 hidden=true，
    // 经 IsJianZhen 312 分支 `!cardConfig.hidden` 排除（BattleCharacter.cs:12322；TS 契约
    // fate-strategy-cards.body-seal “剑阵计数排除 hidden 卡”）。故 formationCount =
    // 水月剑阵 + 狂剑•一式 = 2：attack 4+2*3=10 → hp 30-10=20，defense 4+2*3=10。
    assert_eq!(state.p2.core.hp, 20);
    assert_eq!(state.p1.core.defense, 10);
}

#[test]
fn mirage_beng_quan_entangle_applies_injury_before_attack_and_arms_follow_up() {
    let mut entangle = card(319, 319, "幻•崩拳缠");
    entangle.attack = Some(3);
    entangle.attack_count = Some(2);
    entangle.hp_cost = Some(4);
    entangle.other_params = vec![2];
    let mut meridian = card(10_010_024, 10_000_024, "崩拳•截脉");
    meridian.attack = Some(13);
    meridian.hp_cost = Some(4);
    meridian.other_params = vec![1];
    let mut state = ReplayState::test_from_fixture(&fixture(
        deck_with_cards(vec![entangle, meridian]),
        deck(basic_attack()),
    ));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.hp, 26);
    assert_eq!(state.p2.status.external_injury, 2);
    assert_eq!(state.p2.core.hp, 20);
    assert_eq!(
        state.dream_mirage_value(
            PlayerSide::P1,
            DreamMirageValue::NextBengQuanAdditionalAttack,
        ),
        3,
    );
}

#[test]
fn break_like_bamboo_routes_momentum_gain_through_overflow_defense() {
    let mut bamboo = card(10_010_033, 10_000_033, "势如破竹");
    bamboo.attack = Some(3);
    bamboo.other_params = vec![1, 1, 3];
    let mut state = ReplayState::test_from_fixture(&fixture(deck(bamboo), deck(basic_attack())));
    state.p1.core.anima = 1;
    state.p1.beng.momentum_limit = 1;
    state.p1.beng.momentum = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.core.defense, 1);
}

#[test]
fn mirage_cloud_probe_adds_body_chain_and_arms_cloud_sword_heart() {
    let mut probe = card(264, 264, "幻•云剑探云");
    probe.attack = Some(6);
    probe.other_params = vec![3];
    let mut state = ReplayState::test_from_fixture(&fixture(deck(probe), deck(basic_attack())));

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.sword.cloud_chain, 3);
    assert_eq!(state.p1.sword.cloud_sword_heart, 1);
}

#[test]
fn talent_64_hp_change_defense_uses_the_adaptation_pipeline() {
    let mut divination = card(11_000_001, 11_000_001, "卜命");
    divination.other_params = vec![10];
    let mut battle = fixture(deck(divination), deck(basic_attack()));
    battle.players.p2.talents = vec![64];
    let mut state = ReplayState::test_from_fixture(&battle);
    state.p2.turn.adaptation = 1;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p2.core.hp, 20);
    assert_eq!(state.p2.core.defense, 2);
}

#[test]
fn return_origin_grass_hp_change_triggers_talent_64_defense() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.talents = vec![64];
    battle
        .players
        .p1
        .permanent_buff_temp_datas
        .insert("10008".to_string(), 5);

    let state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.core.hp, 35);
    assert_eq!(state.p1.core.max_hp, 35);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p1.hp_mutation.add_hp_count, 5);
}

#[test]
fn fate_strategy_164_doubles_turn_start_recovery_and_internal_injury_for_both_players() {
    let mut battle = fixture(deck(basic_attack()), deck(basic_attack()));
    battle.players.p1.fate_strategies = vec![164];
    let mut state = ReplayState::test_from_fixture(&battle);

    assert_eq!(state.p1.fate.resonance_mystic_heart_enter_profound, 1);
    assert_eq!(state.p2.fate.resonance_mystic_heart_enter_profound, 1);

    state.current_actor = PlayerSide::P2;
    state.p2.core.hp = 20;
    state.p2.status.recovery = 3;
    state.p2.status.internal_injury = 4;

    state.test_play_actor_turn();

    assert_eq!(state.p2.core.hp, 18);

    let mut disabled_fixture = fixture(deck(basic_attack()), deck(basic_attack()));
    disabled_fixture.players.p1.fate_strategies = vec![164];
    disabled_fixture
        .players
        .p1
        .fate_strategy_temp_datas
        .insert("164".to_string(), 1);
    let disabled = ReplayState::test_from_fixture(&disabled_fixture);
    assert_eq!(disabled.p1.fate.resonance_mystic_heart_enter_profound, 0);
    assert_eq!(disabled.p2.fate.resonance_mystic_heart_enter_profound, 0);
}

#[test]
fn ordinary_star_gain_enters_the_shared_fan_hook() {
    let mut flying_star = card(4_000_024, 4_000_024, "飞星刺");
    flying_star.attack = Some(5);
    flying_star.other_params = vec![2];
    let source = fixture(deck(flying_star), deck(basic_attack()));
    let mut control = ReplayState::test_from_fixture(&source);
    control.test_execute_one_card(PlayerSide::P1);

    let mut state = ReplayState::test_from_fixture(&source);
    state.modify_mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::SixYaoFanDamage, 3);

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(control.p1.astrology.star_power, 2);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p2.core.hp, control.p2.core.hp - 6);
}

#[test]
fn star_loss_uses_the_clamped_actual_delta_for_card_422() {
    let card_422 = card(422, 422, "Card 422 probe");
    let mut state = ReplayState::test_from_fixture(&fixture(deck(card_422), deck(basic_attack())));
    state.p1.astrology.star_power = 2;
    // 24589371/24610558 起（BUILD_24589371_RULE_DELTA §2，synthetic
    // batch-027 转正）：星力流失→加攻的触发条件从 HasCardInDeck(422)
    // 改写为 HasBuff(ZiMangXingBao)——卡组含 422 不再生效，预置 buff 773。
    state.p1.astrology.zi_mang_xing_bao = 1;

    assert_eq!(state.modify_star_power(PlayerSide::P1, -5), -2);
    assert_eq!(state.p1.astrology.star_power, 0);
    assert_eq!(state.p1.core.attack_bonus, 2);
}

#[test]
fn ke_yin_29_redirects_star_gain_before_star_post_hooks() {
    let mut source = fixture(deck(basic_attack()), deck(basic_attack()));
    source.players.p1.used_ke_yin_cards = vec![50_029];
    let mut state = ReplayState::test_from_fixture(&source);
    state.modify_mirage_ronghui_value(PlayerSide::P1, MirageRonghuiValue::SixYaoFanDamage, 4);

    assert_eq!(state.modify_star_power(PlayerSide::P1, 3), 0);
    assert_eq!(state.p1.astrology.star_power, 0);
    assert_eq!(state.p2.status.internal_injury, 3);
    assert_eq!(state.p2.core.hp, 30);
}

#[test]
fn card_170_spends_all_three_resources_through_their_semantic_kernels() {
    let mut card_170 = card(170, 170, "玄真破妄");
    card_170.attack = Some(1);
    card_170.other_params = vec![1, 1, 1];
    let mut state = ReplayState::test_from_fixture(&fixture(deck(card_170), deck(basic_attack())));
    state.p1.beng.momentum = 2;
    state.p1.core.anima = 3;
    state.p1.turn.agility = 4;

    state.test_execute_one_card(PlayerSide::P1);

    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.core.anima, 0);
    assert_eq!(state.p1.turn.agility, 0);
    assert_eq!(state.p2.core.hp, 20);
}

#[test]
fn momentum_post_hooks_observe_gain_before_upper_limit_overflow() {
    let mut source = fixture(deck(basic_attack()), deck(basic_attack()));
    source.players.p1.talents = vec![209];
    source.players.p1.used_ke_yin_cards = vec![50_109];
    let mut state = ReplayState::test_from_fixture(&source);
    state.p1.beng.quan_stance = 1;
    state.p1.beng.momentum = 1;
    state.p1.beng.momentum_limit = 1;

    assert_eq!(state.modify_momentum(PlayerSide::P1, 2).hook_delta, 2);

    assert_eq!(state.p1.beng.momentum, 1);
    assert_eq!(state.p1.turn.agility, 2);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalMomentumGained),
        2
    );
    // 加防探针只剩刻印 109（hook_delta 1 倍）与 overflow 归还，各 2。
    assert_eq!(state.p1.core.defense, 4);
}

#[test]
fn combat_resources_have_one_production_write_kernel_and_narrow_raw_set_calls() {
    const RESOURCE_FIELDS: [&str; 4] = [
        ".astrology.star_power",
        ".sword.sword_intent",
        ".turn.agility",
        ".beng.momentum",
    ];

    fn resource_write_lines(source: &str) -> Vec<usize> {
        let mut lines = Vec::new();
        for field in RESOURCE_FIELDS {
            for (offset, _) in source.match_indices(field) {
                let line_start = source[..offset].rfind('\n').map_or(0, |index| index + 1);
                let line_end = source[offset..]
                    .find('\n')
                    .map_or(source.len(), |index| offset + index);
                if source[line_start..line_end].trim_start().starts_with("//") {
                    continue;
                }
                let suffix = source[offset + field.len()..].trim_start();
                if ["+=", "-=", "*=", "/=", "%=", "&=", "|=", "^=", "<<=", ">>="]
                    .iter()
                    .any(|operator| suffix.starts_with(operator))
                    || (suffix.starts_with('=') && !suffix.starts_with("=="))
                {
                    lines.push(source[..offset].lines().count());
                }
            }
        }
        lines.sort_unstable();
        lines.dedup();
        lines
    }

    let replay_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/replay");
    let mut violations = Vec::new();
    let mut kernel_writes = Vec::new();
    let mut agility_set_callers = Vec::new();
    let mut star_remove_callers = Vec::new();
    let mut entries = std::fs::read_dir(&replay_dir)
        .expect("replay source directory must be readable")
        .collect::<std::result::Result<Vec<_>, _>>()
        .expect("replay source entries must be readable");
    entries.sort_by_key(|entry| entry.file_name());

    for entry in entries {
        let path = entry.path();
        let Some(file_name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if path.extension().and_then(|extension| extension.to_str()) != Some("rs")
            || file_name.starts_with("tests")
        {
            continue;
        }
        let source = std::fs::read_to_string(&path).expect("replay source file must be readable");
        assert!(
            !source.contains("gain_dream_mirage_sword_intent(")
                && !source.contains("gain_dream_mirage_star_power(")
                && !source.contains("gain_mirage_ronghui_star_power("),
            "obsolete resource helper remains in {file_name}"
        );
        for line_number in resource_write_lines(&source) {
            let line = source
                .lines()
                .nth(line_number - 1)
                .unwrap_or_default()
                .trim();
            if file_name == "resources.rs" {
                kernel_writes.push(line.to_string());
            } else {
                violations.push(format!("{file_name}:{line_number}: {line}"));
            }
        }
        for line in source.lines() {
            let line = line.trim();
            if file_name != "resources.rs" && line.contains("set_agility_from_original_card_291(") {
                agility_set_callers.push(file_name.to_string());
            }
            if file_name != "resources.rs"
                && line.contains("remove_star_power_from_original_card_4000065(")
            {
                star_remove_callers.push(file_name.to_string());
            }
        }
    }

    assert!(
        violations.is_empty(),
        "combat resources must enter resources.rs semantic kernels:\n{}",
        violations.join("\n")
    );
    assert_eq!(
        kernel_writes,
        [
            "CombatResource::StarPower => actor.astrology.star_power = after,",
            "CombatResource::SwordIntent => actor.sword.sword_intent = after,",
            "CombatResource::Agility => actor.turn.agility = after,",
            "CombatResource::Momentum => actor.beng.momentum = after,",
        ]
    );
    assert_eq!(agility_set_callers, ["cards_missing.rs"]);
    assert_eq!(
        star_remove_callers,
        ["cards_synthetic_oracle_verified_secret_misc.rs"]
    );
}
