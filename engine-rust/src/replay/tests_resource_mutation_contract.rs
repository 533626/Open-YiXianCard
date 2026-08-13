use super::cards_dream_mirage::DreamMirageValue;
use super::tests::{basic_attack_test_card, filler_cards, minimal_fixture};
use super::*;
use crate::fixture::FixtureExpected;
use crate::model::PlayerSide;

fn state() -> ReplayState {
    ReplayState::test_from_fixture(&minimal_fixture(
        filler_cards(basic_attack_test_card()),
        filler_cards(basic_attack_test_card()),
        FixtureExpected {
            winner_side: PlayerSide::P1,
            actor_turn_count: 1,
            hp_delta_p1_minus_p2: 0,
            final_hp: None,
        },
    ))
}

#[test]
fn positive_momentum_hooks_observe_gain_before_upper_limit_set() {
    let mut state = state();
    state.p1.identity.talents = vec![209];
    state.p1.identity.ke_yin_card_ids = vec![50_109];
    state.p1.beng.quan_stance = 1;
    state.p1.beng.momentum = 1;
    state.p1.beng.momentum_limit = 1;
    state.p1.fate.sheng_qi_ling_ren = 10;

    let receipt = state.modify_momentum(PlayerSide::P1, 2);

    assert_eq!(
        receipt,
        MomentumMutationReceipt {
            requested_delta: 2,
            hook_delta: 2,
            visible_delta: 0,
            overflow_delta: 2,
            before: 1,
            after: 1,
        }
    );
    assert_eq!(state.p1.beng.momentum, 1);
    // 加防探针只剩刻印 109（hook_delta 1 倍）与 overflow 归还，各 2。
    assert_eq!(state.p1.core.defense, 4);
    assert_eq!(state.p1.turn.agility, 2);
    assert_eq!(state.p1.beng.momentum_gain_agility_triggered, 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalMomentumGained),
        2
    );
    assert_eq!(state.p2.core.hp, 10);
}

#[test]
fn fate423_pending_momentum_uses_common_cap_hooks_and_is_consumed_once() {
    let mut state = state();
    state.p1.identity.ke_yin_card_ids = vec![50_109];
    state.p1.beng.momentum = 1;
    state.p1.beng.momentum_limit = 2;
    state.p1.beng.pending_momentum_bonus = 2;

    let receipt = state.modify_momentum(PlayerSide::P1, 1);

    assert_eq!(
        receipt,
        MomentumMutationReceipt {
            requested_delta: 1,
            hook_delta: 3,
            visible_delta: 1,
            overflow_delta: 2,
            before: 1,
            after: 2,
        }
    );
    assert_eq!(state.p1.beng.pending_momentum_bonus, 0);
    // Both the ordinary gain hook and the existing upper-limit overflow path
    // observe the adjusted +3 request.
    assert_eq!(state.p1.core.defense, 5);

    let second = state.modify_momentum(PlayerSide::P1, 1);
    assert_eq!(second.hook_delta, 1);
    assert_eq!(second.overflow_delta, 1);
}

#[test]
fn negative_momentum_overshoot_separates_request_from_hook_delta() {
    let mut state = state();
    state.p1.identity.ke_yin_card_ids = vec![50_109];
    state.p1.beng.momentum = 1;
    state.p1.fate.sheng_qi_ling_ren = 10;

    let receipt = state.modify_momentum(PlayerSide::P1, -5);

    assert_eq!(
        receipt,
        MomentumMutationReceipt {
            requested_delta: -5,
            hook_delta: -1,
            visible_delta: -1,
            overflow_delta: 0,
            before: 1,
            after: 0,
        }
    );
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p2.core.hp, 20);
}

#[test]
fn attack_consumption_uses_the_common_momentum_loss_kernel() {
    let mut state = state();
    state.p1.identity.ke_yin_card_ids = vec![50_109];
    state.p1.beng.momentum = 1;
    state.p1.fate.sheng_qi_ling_ren = 2;

    state.apply_attack(PlayerSide::P1, 10, 0);

    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(state.p2.core.hp, 17);
}

#[test]
fn momentum_limit_set_raw_clamps_without_ordinary_loss_hooks() {
    let mut state = state();
    state.p1.identity.ke_yin_card_ids = vec![50_109];
    state.p1.beng.momentum = 1;
    state.p1.beng.momentum_limit = 5;
    state.p1.beng.unceasing_momentum = 4;
    state.p1.fate.sheng_qi_ling_ren = 2;

    state.modify_momentum_limit(PlayerSide::P1, -5);

    assert_eq!(state.p1.beng.momentum, 0);
    assert_eq!(state.p1.beng.momentum_limit, 0);
    assert_eq!(state.p1.core.defense, 1);
    assert_eq!(
        state.dream_mirage_value(PlayerSide::P1, DreamMirageValue::TotalMomentumGained),
        0
    );
    assert_eq!(state.p2.core.hp, 28);
}

#[test]
fn ke_yin_29_redirects_star_power_before_commit_and_gain_hooks() {
    let mut state = state();
    state.p1.identity.ke_yin_card_ids = vec![50_029];
    state.p1.astrology.star_power = 2;
    state.p1.mirage_ronghui.six_yao_fan_damage = 10;

    assert_eq!(state.modify_star_power(PlayerSide::P1, 3), 0);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p2.status.internal_injury, 3);
    assert_eq!(state.p2.core.hp, 30);
}

#[test]
fn fate399_redirects_star_power_when_switch_is_active() {
    let mut state = state();
    state.p1.identity.fate_strategies = vec![399];
    state.p1.astrology.star_power = 2;

    assert_eq!(state.modify_star_power(PlayerSide::P1, 1), 0);
    assert_eq!(state.p1.astrology.star_power, 2);
    assert_eq!(state.p2.status.internal_injury, 1);
}

#[test]
fn fate399_does_not_redirect_when_switch_is_disabled() {
    let mut state = state();
    state.p1.identity.fate_strategies = vec![399];
    state
        .p1
        .identity
        .fate_strategy_temp_datas
        .insert("399".to_string(), 1);

    assert_eq!(state.modify_star_power(PlayerSide::P1, 1), 1);
    assert_eq!(state.p1.astrology.star_power, 1);
    assert_eq!(state.p2.status.internal_injury, 0);
}

#[test]
fn negative_status_gain_reports_request_applied_and_stack_bounds() {
    let mut state = state();

    let receipt = state.add_actor_negative_status(PlayerSide::P1, 100, 3);

    assert_eq!(
        receipt,
        NegativeStatusMutationReceipt {
            status: 100,
            requested: 3,
            applied: 3,
            before: 0,
            after: 3,
        }
    );
    assert_eq!(state.p1.status.internal_injury, 3);
}

#[test]
fn negative_status_gain_applies_206_stance_reduction_before_writing() {
    let mut state = state();
    state.p1.identity.talents = vec![206];
    state.p1.beng.gun_stance = 1;

    let receipt = state.add_actor_negative_status(PlayerSide::P1, 103, 2);

    // 206 棍架势把减攻 delta 先减 1；receipt 必须报告扣减后的写入值，
    // 否则直写字段的业务模块（receipt.applied == requested）不会被发现。
    assert_eq!(
        receipt,
        NegativeStatusMutationReceipt {
            status: 103,
            requested: 2,
            applied: 1,
            before: 0,
            after: 1,
        }
    );
    assert_eq!(state.p1.status.attack_reduction, 1);
}

#[test]
fn negative_status_loss_reports_signed_delta() {
    let mut state = state();
    state.p1.status.internal_injury = 3;

    let receipt = state.modify_actor_negative_status(PlayerSide::P1, 100, -1);

    assert_eq!(
        receipt,
        NegativeStatusMutationReceipt {
            status: 100,
            requested: -1,
            applied: -1,
            before: 3,
            after: 2,
        }
    );
    assert_eq!(state.p1.status.internal_injury, 2);
}

#[test]
fn blood_calamity_marker_grant_and_consume_go_through_the_status_kernel() {
    let mut state = state();

    // 凶象（卡 11000023）给对方的标记发放必须走状态内核。
    let grant = state.modify_blood_calamity(PlayerSide::P2, 2);
    assert_eq!(
        grant,
        NegativeStatusMutationReceipt {
            status: 379,
            requested: 2,
            applied: 2,
            before: 0,
            after: 2,
        }
    );
    assert_eq!(state.p2.status.blood_calamity, 2);

    // 标记消费路径：损失生命时经 AfterHpModifyPhase 同时发外伤并扣一层标记。
    state.p2.core.hp = 10;
    state.mutate_actor_hp(PlayerSide::P2, -1, false, false);
    assert_eq!(state.p2.status.blood_calamity, 1);
    assert_eq!(state.p2.status.external_injury, 1);
}

#[test]
fn defense_gain_receipt_includes_382_flat_bonus() {
    let mut state = state();
    state.p1.identity.fate_strategies = vec![382];

    let receipt = state.gain_defense(PlayerSide::P1, 2);

    assert_eq!(
        receipt,
        DefenseMutationReceipt {
            requested: 2,
            applied: 3,
            visible_delta: 3,
            before: 0,
            after: 3,
        }
    );
    assert_eq!(state.p1.core.defense, 3);
}

#[test]
fn defense_gain_is_prevented_while_cannot_gain_defense() {
    let mut state = state();
    state.p1.dream_mirage.cannot_gain_defense = 1;

    let receipt = state.gain_defense(PlayerSide::P1, 3);

    assert_eq!(
        receipt,
        DefenseMutationReceipt {
            requested: 3,
            applied: 0,
            visible_delta: 0,
            before: 0,
            after: 0,
        }
    );
    assert_eq!(state.p1.core.defense, 0);
}

#[test]
fn defense_loss_receipt_clamps_at_zero() {
    let mut state = state();
    state.p1.core.defense = 2;

    let receipt = state.lose_defense(PlayerSide::P1, 5);

    assert_eq!(
        receipt,
        DefenseMutationReceipt {
            requested: 5,
            applied: 2,
            visible_delta: -2,
            before: 2,
            after: 0,
        }
    );
    assert_eq!(state.p1.core.defense, 0);
    assert_eq!(state.p1.turn.lost_defense_count, 2);
}

#[test]
fn max_hp_gain_receipt_reports_adaptation_resolved_delta() {
    let mut state = state();
    assert_eq!(state.p1.core.max_hp, 30);
    state.p1.turn.adaptation = 1;

    let receipt = state.modify_actor_max_hp(PlayerSide::P1, 10);

    assert_eq!(
        receipt,
        MaxHpMutationReceipt {
            requested: 10,
            resolved: 14,
            applied: 14,
            before: 30,
            after: 44,
        }
    );
    assert_eq!(state.p1.core.max_hp, 44);
}

#[test]
fn max_hp_loss_receipt_clamps_at_zero_and_caps_hp() {
    let mut state = state();
    state.p1.core.max_hp = 5;
    state.p1.core.hp = 5;

    let receipt = state.modify_actor_max_hp(PlayerSide::P1, -10);

    assert_eq!(
        receipt,
        MaxHpMutationReceipt {
            requested: -10,
            resolved: -10,
            applied: -5,
            before: 5,
            after: 0,
        }
    );
    assert_eq!(state.p1.core.max_hp, 0);
    assert_eq!(state.p1.core.hp, 0);
}

#[test]
fn flame_soul_return_returns_full_revive_receipt() {
    let mut state = state();
    state.p1.core.hp = -5;
    state.p1.core.max_hp = 80;
    state.p1.fate.flame_soul_return = 1;

    let receipt = state
        .check_flame_soul_return(PlayerSide::P1)
        .expect("revive");

    assert_eq!(
        receipt,
        ReviveReceipt {
            kind: ReviveKind::FlameSoulReturn,
            hp_after: 15,
            max_hp_after: 15,
        }
    );
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (15, 15));
    assert_eq!(state.p1.fate.flame_soul_return, 0);
}

#[test]
fn fire_phoenix_revive_returns_full_revive_receipt() {
    let mut state = state();
    state.p1.core.hp = -5;
    state.p1.core.max_hp = 80;
    state.p1.fate.fire_phoenix_revive_hp = 10;

    let receipt = state
        .check_fire_phoenix_revive(PlayerSide::P1)
        .expect("revive");

    assert_eq!(
        receipt,
        ReviveReceipt {
            kind: ReviveKind::FirePhoenix,
            hp_after: 10,
            max_hp_after: 90,
        }
    );
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (10, 90));
    assert_eq!(state.p1.fate.fire_phoenix_revive_hp, 0);
}

#[test]
fn nine_heavens_revive_returns_full_revive_receipt() {
    let mut state = state();
    state.p1.core.hp = -5;
    state.p1.core.max_hp = 30;
    state.p1.mirage_ronghui.nine_heavens_revive = 1;

    let receipt = state
        .check_nine_heavens_revive(PlayerSide::P1)
        .expect("revive");

    // 64 - (-5) = 69 治疗按 max_hp 30 截断。
    assert_eq!(
        receipt,
        ReviveReceipt {
            kind: ReviveKind::NineHeavens,
            hp_after: 30,
            max_hp_after: 30,
        }
    );
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (30, 30));
    assert_eq!(state.p1.mirage_ronghui.nine_heavens_revive, 0);
}

#[test]
fn qi_xing_jie_ming_revive_returns_full_revive_receipt() {
    let mut state = state();
    state.p1.core.hp = -5;
    state.p1.fate.qi_xing_jie_ming = 3;
    state.p1.astrology.hexagram = 2;
    state.p1.astrology.star_power = 3;

    let receipt = state
        .check_qi_xing_jie_ming(PlayerSide::P1)
        .expect("revive");

    // 转换 = (2 + 3) * 3 = 15，上限与生命各 +15。
    assert_eq!(
        receipt,
        ReviveReceipt {
            kind: ReviveKind::QiXingJieMing,
            hp_after: 10,
            max_hp_after: 45,
        }
    );
    assert_eq!((state.p1.core.hp, state.p1.core.max_hp), (10, 45));
    assert_eq!(state.p1.fate.qi_xing_jie_ming, 0);
}

#[test]
fn revive_receipt_is_none_when_no_revive_happens() {
    let mut state = state();
    state.p1.core.hp = -5;

    assert_eq!(state.check_flame_soul_return(PlayerSide::P1), None);
    assert_eq!(state.check_fire_phoenix_revive(PlayerSide::P1), None);
    assert_eq!(state.check_nine_heavens_revive(PlayerSide::P1), None);
    assert_eq!(state.check_qi_xing_jie_ming(PlayerSide::P1), None);

    // 持有标记但生命 > 0 时同样不复活。
    state.p1.core.hp = 5;
    state.p1.fate.fire_phoenix_revive_hp = 10;
    assert_eq!(state.check_fire_phoenix_revive(PlayerSide::P1), None);
}
