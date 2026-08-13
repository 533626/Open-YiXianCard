use super::*;

#[cfg(not(target_arch = "wasm32"))]
pub(super) fn evaluate_candidate_decks(
    options: &CandidateEvaluationOptions<'_>,
    decks: &[CandidateDeck],
) -> Vec<SolverDeckResult> {
    if decks.is_empty() {
        return Vec::new();
    }
    let worker_count = thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(decks.len());
    let chunk_size = decks.len().div_ceil(worker_count);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in decks.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .map(|candidate| {
                        let evaluation = evaluate_fixture_deck_across_battle_seeds(
                            options.fixture,
                            options.side,
                            &candidate.deck,
                            if options.mode == SolverMode::Hand {
                                Some(candidate.leftover_hand_card_ids.clone())
                            } else {
                                None
                            },
                            options.score_profile,
                            options.battle_seeds,
                        );
                        SolverDeckResult {
                            rank: 0,
                            score: evaluation.score,
                            deck: card_ids(&candidate.deck),
                            leftover_hand_card_ids: candidate.leftover_hand_card_ids.clone(),
                            evaluation,
                            deck_key: candidate.key.clone(),
                            sort_key: candidate.sort_key.clone(),
                        }
                    })
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("solver worker panicked"))
            .collect()
    })
}

#[cfg(target_arch = "wasm32")]
pub(super) fn evaluate_candidate_decks(
    options: &CandidateEvaluationOptions<'_>,
    decks: &[CandidateDeck],
) -> Vec<SolverDeckResult> {
    decks
        .iter()
        .map(|candidate| {
            let evaluation = evaluate_fixture_deck_across_battle_seeds(
                options.fixture,
                options.side,
                &candidate.deck,
                if options.mode == SolverMode::Hand {
                    Some(candidate.leftover_hand_card_ids.clone())
                } else {
                    None
                },
                options.score_profile,
                options.battle_seeds,
            );
            SolverDeckResult {
                rank: 0,
                score: evaluation.score,
                deck: card_ids(&candidate.deck),
                leftover_hand_card_ids: candidate.leftover_hand_card_ids.clone(),
                evaluation,
                deck_key: candidate.key.clone(),
                sort_key: candidate.sort_key.clone(),
            }
        })
        .collect()
}

pub(super) fn bucket_cards(cards: Vec<CardDefinition>, mode: SolverMode) -> Vec<CandidateBucket> {
    let mut by_key: BTreeMap<String, CandidateBucket> = BTreeMap::new();
    for card in cards {
        let key = original_card_config_key(&card);
        if let Some(existing) = by_key.get_mut(&key) {
            existing.count += 1;
        } else {
            by_key.insert(key.clone(), CandidateBucket { card, count: 1 });
        }
    }
    let mut buckets = by_key.into_values().collect::<Vec<_>>();
    buckets.sort_by(|left, right| compare_bucket_cards(left, right, mode));
    buckets
}

fn compare_bucket_cards(
    left: &CandidateBucket,
    right: &CandidateBucket,
    mode: SolverMode,
) -> Ordering {
    match mode {
        SolverMode::Order => left
            .card
            .name
            .cmp(&right.card.name)
            .then(left.card.id.cmp(&right.card.id)),
        SolverMode::Hand => compare_ts_zh_names(&left.card.name, &right.card.name)
            .then(left.card.id.cmp(&right.card.id)),
    }
}

fn compare_ts_zh_names(left: &str, right: &str) -> Ordering {
    let mut left_chars = left.chars();
    let mut right_chars = right.chars();
    loop {
        match (left_chars.next(), right_chars.next()) {
            (Some(left_char), Some(right_char)) => {
                let ordering = ts_zh_char_rank(left_char).cmp(&ts_zh_char_rank(right_char));
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}

fn ts_zh_char_rank(value: char) -> u32 {
    TS_ZH_CHAR_ORDER
        .iter()
        .position(|candidate| *candidate == value as u32)
        .map(|index| index as u32)
        .unwrap_or(100_000 + value as u32)
}

const TS_ZH_CHAR_ORDER: &[u32] = &[
    0x00b7, 0x2022, 0x0031, 0x0032, 0x0033, 0x0034, 0x0035, 0x0036, 0x0037, 0x6697, 0x516b, 0x767d,
    0x767e, 0x677f, 0x68d2, 0x78c5, 0x8584, 0x5b9d, 0x62b1, 0x8c79, 0x7206, 0x676f, 0x80cc, 0x5954,
    0x5d29, 0x8ff8, 0x5315, 0x7b14, 0x58c1, 0x907f, 0x8759, 0x97ad, 0x9cd6, 0x7f24, 0x51b0, 0x5175,
    0x997c, 0x6ce2, 0x52c3, 0x7934, 0x535c, 0x8865, 0x6355, 0x4e0d, 0x5e03, 0x6b65, 0x5f69, 0x83dc,
    0x6b8b, 0x82cd, 0x8349, 0x6d4b, 0x7b56, 0x8336, 0x5bdf, 0x62c6, 0x7f20, 0x8749, 0x87fe, 0x80a0,
    0x671d, 0x63a3, 0x5c18, 0x6c89, 0x8fb0, 0x6210, 0x627f, 0x4e58, 0x6f84, 0x9a70, 0x8d64, 0x7fc5,
    0x5145, 0x51b2, 0x5ba0, 0x521d, 0x9664, 0x89e6, 0x5ddd, 0x7a7f, 0x4f20, 0x6625, 0x7eaf, 0x6233,
    0x6148, 0x6b21, 0x523a, 0x7a9c, 0x6dec, 0x5bf8, 0x6253, 0x5927, 0x5f85, 0x4e39, 0x5f39, 0x86cb,
    0x5f53, 0x6321, 0x8361, 0x5200, 0x6363, 0x5230, 0x9053, 0x5730, 0x706f, 0x767b, 0x6ef4, 0x5e95,
    0x5e1d, 0x70b9, 0x7535, 0x96d5, 0x8c03, 0x8776, 0x9876, 0x9f0e, 0x5b9a, 0x4e1c, 0x52a8, 0x6597,
    0x8c46, 0x9017, 0x6bd2, 0x72ec, 0x5ea6, 0x6e21, 0x6bb5, 0x65ad, 0x953b, 0x5151, 0x7893, 0x76fe,
    0x9041, 0x593a, 0x5384, 0x997f, 0x4e8c, 0x53d1, 0x6cd5, 0x53cd, 0x8fd4, 0x65b9, 0x653e, 0x98de,
    0x5206, 0x7eb7, 0x711a, 0x98ce, 0x5c01, 0x75af, 0x5cf0, 0x950b, 0x8702, 0x51af, 0x9022, 0x51e4,
    0x4f0f, 0x62c2, 0x6d6e, 0x7b26, 0x8760, 0x5e9c, 0x65a7, 0x91dc, 0x9644, 0x590d, 0x7f1a, 0x7518,
    0x611f, 0x5e72, 0x521a, 0x7f61, 0x6208, 0x6839, 0x826e, 0x5f13, 0x529f, 0x653b, 0x62f1, 0x5b64,
    0x83c7, 0x53e4, 0x9aa8, 0x86ca, 0x5366, 0x89c2, 0x8d2f, 0x704c, 0x5149, 0x5f52, 0x9f9f, 0x8f68,
    0x9b3c, 0x6842, 0x6eda, 0x68cd, 0x679c, 0x8fc7, 0x8fd8, 0x6d77, 0x5bb3, 0x5bd2, 0x64bc, 0x6beb,
    0x6d69, 0x5408, 0x6cb3, 0x8377, 0x9e64, 0x8861, 0x8f70, 0x7ea2, 0x8679, 0x9e3f, 0x5589, 0x543c,
    0x540e, 0x539a, 0x58f6, 0x846b, 0x864e, 0x62a4, 0x82b1, 0x5316, 0x753b, 0x737e, 0x8b99, 0x73af,
    0x5e7b, 0x5524, 0x8352, 0x7687, 0x51f0, 0x9ec4, 0x6325, 0x8f89, 0x56de, 0x6c47, 0x6d51, 0x9b42,
    0x6df7, 0x706b, 0x51fb, 0x9965, 0x673a, 0x9e21, 0x8ff9, 0x6fc0, 0x5409, 0x6781, 0x75be, 0x68d8,
    0x5939, 0x7532, 0x67b6, 0x5c16, 0x95f4, 0x8327, 0x51cf, 0x7b80, 0x78b1, 0x996f, 0x5251, 0x964d,
    0x4ea4, 0x6d47, 0x9c9b, 0x89d2, 0x63a5, 0x8282, 0x52ab, 0x7ed3, 0x622a, 0x89e3, 0x754c, 0x65a4,
    0x91d1, 0x7b4b, 0x5c3d, 0x9526, 0x52b2, 0x60ca, 0x6676, 0x775b, 0x7cbe, 0x9cb8, 0x51c0, 0x5883,
    0x9759, 0x955c, 0x4e5d, 0x83ca, 0x5de8, 0x805a, 0x8bc0, 0x7edd, 0x8568, 0x519b, 0x5f00, 0x574e,
    0x58f3, 0x514b, 0x523b, 0x7a7a, 0x67af, 0x72c2, 0x5cbf, 0x8475, 0x5764, 0x951f, 0x9cb2, 0x56f0,
    0x814a, 0x8fa3, 0x6765, 0x5170, 0x6f9c, 0x63fd, 0x70c2, 0x72fc, 0x8782, 0x6d6a, 0x635e, 0x96f7,
    0x7c7b, 0x72f8, 0x79bb, 0x7483, 0x91cc, 0x529b, 0x5389, 0x7acb, 0x6817, 0x96f3, 0x8fde, 0x83b2,
    0x7ec3, 0x70bc, 0x7cae, 0x4e24, 0x4eae, 0x7597, 0x71ce, 0x70c8, 0x730e, 0x88c2, 0x6797, 0x9716,
    0x7075, 0x73b2, 0x51cc, 0x96f6, 0x6d41, 0x7409, 0x67f3, 0x516d, 0x9f99, 0x73d1, 0x82a6, 0x7089,
    0x9e7f, 0x8def, 0x9e6d, 0x9732, 0x5f8b, 0x7eff, 0x4e71, 0x63a0, 0x8f6e, 0x7f57, 0x843d, 0x9ebb,
    0x9a6c, 0x8109, 0x7792, 0x8513, 0x8292, 0x732b, 0x6bdb, 0x73ab, 0x6885, 0x95e8, 0x8499, 0x68a6,
    0x5f25, 0x8ff7, 0x79d8, 0x871c, 0x5999, 0x706d, 0x540d, 0x660e, 0x51a5, 0x6e9f, 0x547d, 0x9b54,
    0x79e3, 0x58a8, 0x6728, 0x7eb3, 0x6ce5, 0x9006, 0x5ff5, 0x9e1f, 0x51dd, 0x725b, 0x632a, 0x85d5,
    0x62cd, 0x76d8, 0x65c1, 0x80d6, 0x80da, 0x57f9, 0x84ec, 0x9e4f, 0x5288, 0x9739, 0x8f9f, 0x5e73,
    0x74f6, 0x6cfc, 0x7834, 0x9b44, 0x6251, 0x749e, 0x666e, 0x4e03, 0x5947, 0x68cb, 0x65d7, 0x8d77,
    0x6c14, 0x5f03, 0x5668, 0x5343, 0x7275, 0x524d, 0x4e7e, 0x6f5c, 0x67aa, 0x4fb5, 0x7434, 0x52e4,
    0x6c81, 0x9752, 0x8f7b, 0x6e05, 0x873b, 0x60c5, 0x7a79, 0x66f2, 0x9a71, 0x8d8b, 0x6cc9, 0x62f3,
    0x72ac, 0x96c0, 0x7fa4, 0x7136, 0x71c3, 0x6270, 0x7ed5, 0x4eba, 0x5203, 0x8ba4, 0x65e5, 0x7194,
    0x878d, 0x67d4, 0x8089, 0x5982, 0x5165, 0x9510, 0x745e, 0x6da6, 0x82e5, 0x5f31, 0x4e09, 0x626b,
    0x8272, 0x6740, 0x6c99, 0x5239, 0x7802, 0x715e, 0x5c71, 0x95ea, 0x6247, 0x4f24, 0x70e7, 0x86c7,
    0x5c04, 0x8eab, 0x6df1, 0x795e, 0x751f, 0x7ef3, 0x76db, 0x5931, 0x65bd, 0x77f3, 0x65f6, 0x8680,
    0x98df, 0x4e16, 0x5f0f, 0x4e8b, 0x52bf, 0x8bd5, 0x901d, 0x566c, 0x624b, 0x5b88, 0x9996, 0x517d,
    0x4e66, 0x67a2, 0x758f, 0x9f20, 0x672f, 0x6811, 0x53cc, 0x6c34, 0x987a, 0x77ac, 0x4e1d, 0x6b7b,
    0x56db, 0x82cf, 0x6eaf, 0x9178, 0x7b97, 0x968f, 0x9ad3, 0x788e, 0x7b0b, 0x68ad, 0x9501, 0x5854,
    0x8e0f, 0x592a, 0x8c08, 0x63a2, 0x87b3, 0x6d9b, 0x6843, 0x817e, 0x85e4, 0x8e44, 0x4f53, 0x5929,
    0x6dfb, 0x7530, 0x8df3, 0x94c1, 0x8713, 0x9706, 0x901a, 0x540c, 0x94dc, 0x6295, 0x7a81, 0x56fe,
    0x5c60, 0x571f, 0x5410, 0x5154, 0x63a8, 0x817f, 0x541e, 0x8131, 0x5f2f, 0x665a, 0x4e07, 0x60d8,
    0x5984, 0x5fd8, 0x671b, 0x5a01, 0x5c3e, 0x4e3a, 0x7eb9, 0x95ee, 0x74ee, 0x5367, 0x65e0, 0x94fb,
    0x4e94, 0x821e, 0x7269, 0x609f, 0x5438, 0x606f, 0x7280, 0x88ad, 0x6d17, 0x9699, 0x971e, 0x4ed9,
    0x5148, 0x9c9c, 0x5f26, 0x54b8, 0x663e, 0x73b0, 0x76f8, 0x9999, 0x54cd, 0x5411, 0x8c61, 0x50cf,
    0x67ad, 0x900d, 0x9704, 0x5c0f, 0x6653, 0x5578, 0x90aa, 0x5fc3, 0x4fe1, 0x661f, 0x884c, 0x5f62,
    0x51f6, 0x6c79, 0x4f11, 0x4fee, 0x9990, 0x987b, 0x865a, 0x589f, 0x7eea, 0x84c4, 0x7384, 0x60ac,
    0x65cb, 0x7a74, 0x96ea, 0x8840, 0x5de1, 0x8fc5, 0x5dfd, 0x9e26, 0x7259, 0x82bd, 0x5d16, 0x70df,
    0x5ca9, 0x708e, 0x7814, 0x884d, 0x773c, 0x7130, 0x7131, 0x71d5, 0x626c, 0x517b, 0x5996, 0x723b,
    0x7a91, 0x9065, 0x7476, 0x836f, 0x8000, 0x91ce, 0x53f6, 0x66f3, 0x591c, 0x4e00, 0x4eea, 0x5b9c,
    0x79fb, 0x9057, 0x4ee5, 0x5f08, 0x610f, 0x9634, 0x97f3, 0x94f6, 0x5f15, 0x5370, 0x5e94, 0x9e66,
    0x8fce, 0x5f71, 0x786c, 0x6d8c, 0x5fe7, 0x5e7d, 0x60a0, 0x6e38, 0x6709, 0x9c7c, 0x7fbd, 0x96e8,
    0x7389, 0x6d74, 0x57df, 0x8c15, 0x5fa1, 0x6108, 0x9e33, 0x6e0a, 0x5143, 0x539f, 0x7f18, 0x733f,
    0x6028, 0x6708, 0x5cb3, 0x8dc3, 0x4e91, 0x9668, 0x8fd0, 0x97f5, 0x6742, 0x707e, 0x518d, 0x5728,
    0x846c, 0x67a3, 0x9020, 0x6458, 0x65a9, 0x5360, 0x7efd, 0x957f, 0x638c, 0x7634, 0x62db, 0x722a,
    0x6298, 0x9488, 0x73cd, 0x771f, 0x9635, 0x9547, 0x9707, 0x4e89, 0x6b63, 0x4e4b, 0x829d, 0x679d,
    0x690d, 0x7eb8, 0x6307, 0x5236, 0x6cbb, 0x63b7, 0x4e2d, 0x949f, 0x79cd, 0x4f17, 0x91cd, 0x821f,
    0x5468, 0x5492, 0x9aa4, 0x682a, 0x73e0, 0x8bf8, 0x86db, 0x7af9, 0x4e3b, 0x6ce8, 0x67f1, 0x8f6c,
    0x9994, 0x8ffd, 0x5760, 0x62d9, 0x6349, 0x707c, 0x956f, 0x7d2b, 0x81ea, 0x5b50, 0x8e2a, 0x9b03,
    0x7cbd, 0x8d70, 0x8db3, 0x9189,
];

pub(super) fn enumerate_decks(
    buckets: &[CandidateBucket],
    deck_size: usize,
    visit: &mut impl FnMut(&[CardDefinition], &[CandidateBucket]) -> bool,
) -> bool {
    let mut working = buckets.to_vec();
    let mut deck = Vec::with_capacity(deck_size);
    enumerate_next(&mut working, &mut deck, deck_size, visit)
}

pub(super) fn enumerate_next(
    working: &mut [CandidateBucket],
    deck: &mut Vec<CardDefinition>,
    deck_size: usize,
    visit: &mut impl FnMut(&[CardDefinition], &[CandidateBucket]) -> bool,
) -> bool {
    if deck.len() == deck_size {
        let leftovers = working
            .iter()
            .filter(|bucket| bucket.count > 0)
            .cloned()
            .collect::<Vec<_>>();
        return visit(deck, &leftovers);
    }
    for index in 0..working.len() {
        if working[index].count == 0 {
            continue;
        }
        working[index].count -= 1;
        deck.push(working[index].card.clone());
        let should_continue = enumerate_next(working, deck, deck_size, visit);
        deck.pop();
        working[index].count += 1;
        if !should_continue {
            return false;
        }
    }
    true
}

pub(super) fn sort_results(results: &mut [SolverDeckResult]) {
    results.sort_by(compare_value_results);
}

#[derive(Debug, Clone)]
pub(super) struct EvaluatedResultsSummary {
    pub(super) deck_count: usize,
    pub(super) win_deck_count: usize,
    pub(super) first_win_rank: Option<usize>,
    pub(super) first_win_deck_key: Option<String>,
    pub(super) best_hp_delta_for_side: f64,
    pub(super) best_hp_delta_deck_key: String,
    pub(super) value_top_deck_key: String,
}

pub(super) fn summarize_evaluated_results(results: &[SolverDeckResult]) -> EvaluatedResultsSummary {
    let best_hp_delta = results
        .iter()
        .min_by(|left, right| compare_win_first_hp_delta_results(left, right))
        .expect("solver evaluated at least one deck");
    let value_top = results
        .iter()
        .min_by(|left, right| compare_value_results(left, right))
        .expect("solver evaluated at least one deck");
    let first_win = results
        .iter()
        .enumerate()
        .find(|(_, result)| result.evaluation.win_for_side);
    EvaluatedResultsSummary {
        deck_count: results.len(),
        win_deck_count: results
            .iter()
            .filter(|result| result.evaluation.win_for_side)
            .count(),
        first_win_rank: first_win.map(|(index, _)| index + 1),
        first_win_deck_key: first_win.map(|(_, result)| result.deck_key.clone()),
        best_hp_delta_for_side: best_hp_delta.evaluation.hp_delta_for_side,
        best_hp_delta_deck_key: best_hp_delta.deck_key.clone(),
        value_top_deck_key: value_top.deck_key.clone(),
    }
}

pub(super) fn compare_value_results(left: &SolverDeckResult, right: &SolverDeckResult) -> Ordering {
    right
        .evaluation
        .seed_aggregate
        .as_ref()
        .map_or(usize::from(right.evaluation.win_for_side), |item| {
            item.win_count
        })
        .cmp(
            &left
                .evaluation
                .seed_aggregate
                .as_ref()
                .map_or(usize::from(left.evaluation.win_for_side), |item| {
                    item.win_count
                }),
        )
        .then(compare_bool_desc(
            left.evaluation.win_for_side,
            right.evaluation.win_for_side,
        ))
        .then(compare_score_desc(
            left.evaluation.score,
            right.evaluation.score,
        ))
        .then(compare_score_desc(
            left.evaluation.hp_delta_for_side,
            right.evaluation.hp_delta_for_side,
        ))
        .then(compare_f64_asc(
            left.evaluation.actor_turn,
            right.evaluation.actor_turn,
        ))
        .then(left.sort_key.cmp(&right.sort_key))
}

pub(super) fn compare_win_first_hp_delta_results(
    left: &SolverDeckResult,
    right: &SolverDeckResult,
) -> Ordering {
    compare_bool_desc(left.evaluation.win_for_side, right.evaluation.win_for_side)
        .then(compare_score_desc(
            left.evaluation.hp_delta_for_side,
            right.evaluation.hp_delta_for_side,
        ))
        .then(compare_f64_asc(
            left.evaluation.actor_turn,
            right.evaluation.actor_turn,
        ))
        .then(left.sort_key.cmp(&right.sort_key))
}

fn compare_bool_desc(left: bool, right: bool) -> Ordering {
    right.cmp(&left)
}

fn compare_score_desc(left: f64, right: f64) -> Ordering {
    right.partial_cmp(&left).unwrap_or(Ordering::Equal)
}

fn compare_f64_asc(left: f64, right: f64) -> Ordering {
    left.partial_cmp(&right).unwrap_or(Ordering::Equal)
}

type SeedAuditFields = (Option<Vec<u32>>, Option<Vec<u32>>, Option<bool>);

pub(super) fn seed_audit_fields<'a>(
    evaluations: impl Iterator<Item = &'a SolverEvaluation>,
) -> SeedAuditFields {
    let aggregates = evaluations
        .filter_map(|evaluation| evaluation.seed_aggregate.as_ref())
        .collect::<Vec<_>>();
    let Some(first) = aggregates.first() else {
        return (None, None, None);
    };
    let synthetic_seeds = aggregates
        .iter()
        .flat_map(|aggregate| aggregate.synthetic_decision_seeds_used.iter().copied())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    (
        Some(first.seeds_used.clone()),
        Some(synthetic_seeds.clone()),
        Some(!synthetic_seeds.is_empty()),
    )
}

pub(super) fn complete_deck_ids(baseline: &[CardDefinition], ids: &[i64]) -> Vec<CardDefinition> {
    ids.iter()
        .map(|id| {
            baseline
                .iter()
                .find(|card| card.id == *id)
                .cloned()
                .or_else(|| crate::original_card_definition_by_id(*id))
                .unwrap_or_else(|| CardDefinition {
                    id: *id,
                    base_id: None,
                    name: format!("card:{id}"),
                    card_type: None,
                    attack: None,
                    random_attack: None,
                    random_defense: None,
                    attack_count: None,
                    defense: None,
                    damage: None,
                    anima: None,
                    hp_cost: None,
                    action_again: None,
                    physique: None,
                    sword_intent: None,
                    hexagram: None,
                    rarity: None,
                    career_name: None,
                    other_params: Vec::new(),
                })
        })
        .collect()
}

pub(super) fn card_ids(cards: &[CardDefinition]) -> Vec<i64> {
    cards.iter().map(|card| card.id).collect()
}

pub(super) fn deck_key(cards: &[CardDefinition]) -> String {
    cards
        .iter()
        .map(original_card_config_key)
        .collect::<Vec<_>>()
        .join("|")
}

pub(super) fn deck_sort_key(cards: &[CardDefinition]) -> String {
    cards
        .iter()
        .map(ts_original_card_config_key)
        .collect::<Vec<_>>()
        .join("|")
}

fn original_card_config_key(card: &CardDefinition) -> String {
    serde_json::json!({
        "id": card.id,
        "baseId": card.base_id,
        "name": card.name,
        "cardType": card.card_type,
        "anima": card.anima,
        "hpCost": card.hp_cost,
        "actionAgain": card.action_again,
        "attack": card.attack,
        "randomAttack": card.random_attack,
        "attackCount": card.attack_count,
        "def": null,
        "defense": card.defense,
        "randomDef": null,
        "randomDefense": card.random_defense,
        "damage": card.damage,
        "physique": card.physique,
        "jianYi": card.sword_intent,
        "guaXiang": card.hexagram,
        "otherParams": card.other_params,
        "seasonMechanics": null,
        "rarity": null,
        "owner": null,
    })
    .to_string()
}

fn ts_original_card_config_key(card: &CardDefinition) -> String {
    let mut fields = Vec::new();
    push_json_i64_field(&mut fields, "id", card.id);
    if let Some(value) = card.base_id {
        push_json_i64_field(&mut fields, "baseId", value);
    }
    push_json_field(&mut fields, "name", &card.name);
    if let Some(value) = &card.card_type {
        push_json_field(&mut fields, "cardType", value);
    }
    if let Some(value) = card.anima {
        push_json_i64_field(&mut fields, "anima", value);
    }
    if let Some(value) = card.hp_cost {
        push_json_i64_field(&mut fields, "hpCost", value);
    }
    if let Some(value) = card.action_again {
        push_json_field(&mut fields, "actionAgain", &value);
    }
    if let Some(value) = card.attack {
        push_json_i64_field(&mut fields, "attack", value);
    }
    if let Some(value) = card.random_attack {
        push_json_i64_field(&mut fields, "randomAttack", value);
    }
    if let Some(value) = card.attack_count {
        push_json_i64_field(&mut fields, "attackCount", value);
    }
    if let Some(value) = card.defense {
        push_json_i64_field(&mut fields, "defense", value);
    }
    if let Some(value) = card.random_defense {
        push_json_i64_field(&mut fields, "randomDefense", value);
    }
    if let Some(value) = card.damage {
        push_json_i64_field(&mut fields, "damage", value);
    }
    if let Some(value) = card.physique {
        push_json_i64_field(&mut fields, "physique", value);
    }
    if let Some(value) = card.sword_intent {
        push_json_i64_field(&mut fields, "jianYi", value);
    }
    if let Some(value) = card.hexagram {
        push_json_i64_field(&mut fields, "guaXiang", value);
    }
    if !card.other_params.is_empty() {
        push_json_field(&mut fields, "otherParams", &card.other_params);
    }
    format!("{{{}}}", fields.join(","))
}

fn push_json_i64_field(fields: &mut Vec<String>, name: &str, value: i64) {
    fields.push(format!("\"{name}\":{value}"));
}

fn push_json_field<T: Serialize>(fields: &mut Vec<String>, name: &str, value: &T) {
    let json = serde_json::to_string(value).expect("solver sort key field serializes");
    fields.push(format!("\"{name}\":{json}"));
}

pub(super) fn evals_per_sec(count: usize, seconds: f64) -> f64 {
    if seconds <= 0.0 {
        count as f64
    } else {
        count as f64 / seconds
    }
}
