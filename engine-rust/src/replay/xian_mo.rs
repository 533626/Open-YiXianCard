//! 仙魔（build 25621897 赛季机制）策略的战斗侧门禁。
//!
//! `data/xian-mo-battle-strategies.json` 由 `research/original-game/build_xian_mo_battle_strategies.py`
//! 从反编译战斗源码与 `XianMoStrategyConfig.isBattleEffect` 穷举生成。选中其中任一策略而引擎尚未
//! 实现时一律 fail closed（`MissingRule`），不静默忽略——未实现的战斗效果会让终局三元失真。

use super::BattleError;
use crate::fixture::BattleFixture;
use serde::Deserialize;
use std::collections::BTreeSet;
use std::sync::OnceLock;

/// 已按原版源码实现并有取证的仙魔策略（当前为空：等真实回放出现后逐条实现）。
const IMPLEMENTED_XIAN_MO_STRATEGY_IDS: &[i64] = &[];

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
        data.strategies.into_iter().map(|strategy| strategy.id).collect()
    })
}

/// 返回 fixture 中第一个影响战斗但未实现的仙魔策略。
pub(super) fn unimplemented_xian_mo_strategy(fixture: &BattleFixture) -> Option<BattleError> {
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
