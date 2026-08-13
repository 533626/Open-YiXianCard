// 本文件由 `bun run report:value-weights` 从
// `analysis/generated/value-weights-v1.json` 生成，不要手改。
//
// 权重不是手写的解释性数字：每一条都是在 GA 基础池自对弈上做起始状态
// 配对扰动测出来的 λ10（单位 0.1 HP），或第二阶段跨对手排序目标搜索
// 得到的过程项系数。改权重的正确做法是重跑训练，不是改这里的数。
// 口径见 docs/VALUE_FUNCTION.md。

/// HP_WEIGHT｜来源 unit-anchor，N 16128。内部单位定义：1 点生命 = 10 单位。不是自由参数，但仍跑扰动做恒等复核——+N 生命必须恰好回收 λ10 = 10，回收不到说明注入路径或评估口径坏了，整张表不可读。
pub(super) const HP_WEIGHT: f64 = 10.0;
/// MAX_HP_WEIGHT｜来源 perturbation，N 13728。生命上限：本身不加当前生命，只在治疗/体魄上限和上限相关结算里兑现。
pub(super) const MAX_HP_WEIGHT: f64 = 1.0;
/// DEFENSE_WEIGHT｜来源 perturbation，N 16128。普通防御：回合开始衰减，先于护体吸收伤害。
pub(super) const DEFENSE_WEIGHT: f64 = 2.0;
/// GUARD_WEIGHT｜来源 perturbation，N 16128。护体层数：原版消耗一层抵消一次普通生命损失。手写年代按每层 60（=6 点生命）定价，而 docs/VALUE_FUNCTION.md 早就指出这明显偏低——一层护体的期望价值是它将要吸收的那一发伤害。
pub(super) const GUARD_WEIGHT: f64 = 61.0;
/// ANIMA_WEIGHT｜来源 perturbation，N 16128。灵气：出牌成本资源。
pub(super) const ANIMA_WEIGHT: f64 = 37.0;
/// SWORD_INTENT_WEIGHT｜来源 perturbation，N 16128。剑意：云灵剑宗专属机制。
pub(super) const SWORD_INTENT_WEIGHT: f64 = 12.0;
/// MOMENTUM_WEIGHT｜来源 perturbation，N 16128。气势：锻玄宗专属机制。
pub(super) const MOMENTUM_WEIGHT: f64 = 5.0;
/// AGILITY_WEIGHT｜来源 perturbation，N 2688。身法：锻玄宗专属机制。
pub(super) const AGILITY_WEIGHT: f64 = 2.0;
/// HEXAGRAM_WEIGHT｜来源 perturbation，N 7296。卦象。
pub(super) const HEXAGRAM_WEIGHT: f64 = 17.0;
/// STAR_POWER_WEIGHT｜来源 perturbation，N 16128。星力：七星阁专属机制。
pub(super) const STAR_POWER_WEIGHT: f64 = 22.0;
/// ATTACK_BONUS_WEIGHT｜来源 perturbation，N 16128。加攻：每次攻击段都吃到，局长越长越值钱。
pub(super) const ATTACK_BONUS_WEIGHT: f64 = 70.0;
/// PHYSIQUE_WEIGHT｜来源 perturbation，N 15168。体魄：`apply_physique_amount` 必然 1:1 连带最大生命（resources.rs:875），所以扣掉一份 MAX_HP_WEIGHT 才是体魄自己的边际价。
pub(super) const PHYSIQUE_WEIGHT: f64 = 1.0;
/// CLOUD_CHAIN_WEIGHT｜来源 perturbation，N 0（支撑不足，按 0 计价）。连云：云灵剑宗专属机制。
pub(super) const CLOUD_CHAIN_WEIGHT: f64 = 0.0;
/// WATER_MOMENTUM_WEIGHT｜来源 perturbation，N 16128。水势：五行道盟专属机制。
pub(super) const WATER_MOMENTUM_WEIGHT: f64 = 95.0;
/// SHARPNESS_WEIGHT｜来源 perturbation，N 16128。锋锐。
pub(super) const SHARPNESS_WEIGHT: f64 = 8.0;
/// CLOUD_SEA_CHAIN_RESERVE_WEIGHT｜来源 perturbation，N 4032。云海存量：value 函数只在连云 > 0 时才给它计价，所以这条读数天然只由能开出连云的 profile 支撑，连云够不着的 profile 会因为零效应被判 inert 剔掉。
pub(super) const CLOUD_SEA_CHAIN_RESERVE_WEIGHT: f64 = 11.0;
/// ACTIVATED_ELEMENT_WEIGHT｜来源 pooled-perturbation，N 20160。五行激活：五个元素共用一个权重，所以五条扰动的样本合并成一个估计。
pub(super) const ACTIVATED_ELEMENT_WEIGHT: f64 = 53.0;
/// INTERNAL_INJURY_WEIGHT｜来源 perturbation，N 16128。内伤：负面状态，权重存正数并在 debuffPenalty 里被减掉。
pub(super) const INTERNAL_INJURY_WEIGHT: f64 = 103.0;
/// WEAKNESS_WEIGHT｜来源 perturbation，N 12000。虚弱：负面状态。
pub(super) const WEAKNESS_WEIGHT: f64 = 5.0;
/// FLAW_WEIGHT｜来源 perturbation，N 11904。破绽：负面状态。
pub(super) const FLAW_WEIGHT: f64 = 2.0;
/// ATTACK_REDUCTION_WEIGHT｜来源 perturbation，N 16128。减攻（JianGong）：攻击公式里逐段扣减，不是加攻。
pub(super) const ATTACK_REDUCTION_WEIGHT: f64 = 67.0;
/// ENTANGLE_WEIGHT｜来源 perturbation，N 4032。缠缚：再次行动结算会消耗/拦截它。
pub(super) const ENTANGLE_WEIGHT: f64 = 5.0;
/// EXTERNAL_INJURY_WEIGHT｜来源 perturbation，N 16128。外伤：攻击伤害在防御之后追加它。
pub(super) const EXTERNAL_INJURY_WEIGHT: f64 = 58.0;
/// ACTION_AGAIN_WEIGHT｜来源 ranking-search。终局 tempo 通道。`turn.action_again_count` 是逐回合计数器（回合结束清零）且被 `< 1` 上限读，起始注入 +1 反而会**堵掉**首回合的再次行动，语义相反，测不了边际价；改由第二阶段跨对手排序目标搜索。
pub(super) const ACTION_AGAIN_WEIGHT: f64 = 18.0;
/// TERMINAL_RESOURCE_DISCOUNT｜来源 ranking-search。第二阶段搜索前的过渡种子：1 = 不折价，等价于改动前的行为。
pub(super) const TERMINAL_RESOURCE_DISCOUNT: f64 = 0.0;
/// AREA_WEIGHT｜来源 ranking-search。过程面积折算系数，乘在整条轨迹的均值上，不是任何单个起始状态的价格，起始扰动测不到；由第二阶段跨对手排序目标搜索。
pub(super) const AREA_WEIGHT: f64 = 0.1;
