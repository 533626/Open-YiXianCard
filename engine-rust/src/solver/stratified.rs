use super::candidate_decks::{deck_key, deck_sort_key, enumerate_decks, enumerate_next};
use super::*;

pub(super) fn collect_stratified_candidate_decks(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    visit_seed: u64,
    max_evaluations: usize,
) -> CandidateDeckCollection {
    // Native analysis can afford a million-candidate population before seeded
    // stratification. Browser WASM has a fixed linear memory and otherwise
    // traps on ordinary current-sect pools long before evaluating the requested
    // 2k candidates. Keep native certification unchanged while bounding the
    // browser population to a useful multiple of its visible evaluation budget.
    #[cfg(target_arch = "wasm32")]
    let population_limit =
        STRATIFIED_POPULATION_LIMIT.min(max_evaluations.saturating_mul(8).max(max_evaluations));
    #[cfg(not(target_arch = "wasm32"))]
    let population_limit = STRATIFIED_POPULATION_LIMIT;
    let collection = collect_stratified_candidate_decks_parallel(
        buckets,
        mode,
        visit_seed,
        max_evaluations,
        population_limit,
    );
    if collection.skipped_duplicate_count == 0 {
        return collection;
    }
    collect_stratified_candidate_decks_single_thread(
        buckets,
        mode,
        visit_seed,
        max_evaluations,
        population_limit,
    )
}

pub(super) fn collect_stratified_candidate_decks_single_thread(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    visit_seed: u64,
    max_evaluations: usize,
    population_limit: usize,
) -> CandidateDeckCollection {
    let mut seen_deck_keys = BTreeSet::new();
    let mut skipped_duplicate_count = 0_usize;
    let mut population_truncated = false;
    let mut decks = Vec::new();

    enumerate_decks(buckets, DECK_SIZE, &mut |deck, leftovers| {
        let key = deck_key(deck);
        if !seen_deck_keys.insert(key.clone()) {
            skipped_duplicate_count += 1;
            return true;
        }
        if decks.len() >= population_limit {
            population_truncated = true;
            return false;
        }
        decks.push(candidate_deck_from_parts(key, deck, leftovers, mode));
        true
    });

    sort_stratified_candidate_decks_single_thread(&mut decks, visit_seed);
    let budget_truncated = decks.len() > max_evaluations;
    decks.truncate(max_evaluations);

    CandidateDeckCollection {
        decks,
        skipped_duplicate_count,
        truncated: population_truncated || budget_truncated,
    }
}

pub(super) fn collect_stratified_candidate_decks_parallel(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    visit_seed: u64,
    max_evaluations: usize,
    population_limit: usize,
) -> CandidateDeckCollection {
    let StratifiedPopulation {
        mut decks,
        skipped_duplicate_count,
        population_truncated,
    } = collect_stratified_population_parallel(buckets, mode, population_limit);

    sort_stratified_candidate_decks_parallel(&mut decks, visit_seed);
    let budget_truncated = decks.len() > max_evaluations;
    decks.truncate(max_evaluations);

    CandidateDeckCollection {
        decks,
        skipped_duplicate_count,
        truncated: population_truncated || budget_truncated,
    }
}

fn collect_stratified_population_parallel(
    buckets: &[CandidateBucket],
    mode: SolverMode,
    population_limit: usize,
) -> StratifiedPopulation {
    let total_count = count_deck_sequences(buckets, DECK_SIZE, population_limit.saturating_add(1));
    let population_truncated = total_count > population_limit;
    let target_count = total_count.min(population_limit);
    if target_count == 0 {
        return StratifiedPopulation {
            decks: Vec::new(),
            skipped_duplicate_count: 0,
            population_truncated,
        };
    }

    let shards = plan_stratified_shards(buckets, target_count);
    let shard_outputs = collect_stratified_shards_parallel(&shards, mode);
    let (decks, skipped_duplicate_count) =
        merge_stratified_shard_outputs(shard_outputs, population_limit);

    StratifiedPopulation {
        decks,
        skipped_duplicate_count,
        population_truncated,
    }
}

pub(super) fn plan_stratified_shards(
    buckets: &[CandidateBucket],
    target_count: usize,
) -> Vec<StratifiedShard> {
    let mut working = buckets.to_vec();
    let mut prefix = Vec::with_capacity(DECK_SIZE);
    let mut remaining = target_count;
    let mut shards = Vec::new();
    plan_stratified_shards_next(&mut working, &mut prefix, &mut remaining, &mut shards);
    shards
}

fn plan_stratified_shards_next(
    working: &mut [CandidateBucket],
    prefix: &mut Vec<CardDefinition>,
    remaining: &mut usize,
    shards: &mut Vec<StratifiedShard>,
) {
    if *remaining == 0 {
        return;
    }
    let slots_remaining = DECK_SIZE.saturating_sub(prefix.len());
    let completion_count = count_deck_sequences(working, slots_remaining, *remaining);
    if completion_count == 0 {
        return;
    }
    let shard_limit = completion_count.min(*remaining);
    if shard_limit <= STRATIFIED_SHARD_TARGET || slots_remaining == 0 {
        shards.push(StratifiedShard {
            prefix: prefix.clone(),
            buckets: working.to_vec(),
            limit: shard_limit,
        });
        *remaining -= shard_limit;
        return;
    }

    for index in 0..working.len() {
        if working[index].count == 0 {
            continue;
        }
        working[index].count -= 1;
        prefix.push(working[index].card.clone());
        plan_stratified_shards_next(working, prefix, remaining, shards);
        prefix.pop();
        working[index].count += 1;
        if *remaining == 0 {
            return;
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
fn collect_stratified_shards_parallel(
    shards: &[StratifiedShard],
    mode: SolverMode,
) -> Vec<Vec<CandidateDeck>> {
    if shards.is_empty() {
        return Vec::new();
    }
    let worker_count = stratified_worker_count(shards.len(), 1);
    let chunk_size = shards.len().div_ceil(worker_count);
    thread::scope(|scope| {
        let mut handles = Vec::new();
        for chunk in shards.chunks(chunk_size) {
            handles.push(scope.spawn(move || {
                chunk
                    .iter()
                    .map(|shard| collect_stratified_shard(shard, mode))
                    .collect::<Vec<_>>()
            }));
        }
        handles
            .into_iter()
            .flat_map(|handle| handle.join().expect("solver shard worker panicked"))
            .collect()
    })
}

#[cfg(target_arch = "wasm32")]
fn collect_stratified_shards_parallel(
    shards: &[StratifiedShard],
    mode: SolverMode,
) -> Vec<Vec<CandidateDeck>> {
    shards
        .iter()
        .map(|shard| collect_stratified_shard(shard, mode))
        .collect()
}

pub(super) fn collect_stratified_shard(
    shard: &StratifiedShard,
    mode: SolverMode,
) -> Vec<CandidateDeck> {
    let mut working = shard.buckets.clone();
    let mut deck = shard.prefix.clone();
    let mut decks = Vec::with_capacity(shard.limit);
    enumerate_next(
        &mut working,
        &mut deck,
        DECK_SIZE,
        &mut |deck, leftovers| {
            let key = deck_key(deck);
            decks.push(candidate_deck_from_parts(key, deck, leftovers, mode));
            decks.len() < shard.limit
        },
    );
    decks
}

fn merge_stratified_shard_outputs(
    shard_outputs: Vec<Vec<CandidateDeck>>,
    population_limit: usize,
) -> (Vec<CandidateDeck>, usize) {
    let mut seen_deck_keys = BTreeSet::new();
    let mut skipped_duplicate_count = 0_usize;
    let mut decks = Vec::new();
    for shard_decks in shard_outputs {
        for candidate in shard_decks {
            if !seen_deck_keys.insert(candidate.key.clone()) {
                skipped_duplicate_count += 1;
                continue;
            }
            if decks.len() >= population_limit {
                return (decks, skipped_duplicate_count);
            }
            decks.push(candidate);
        }
    }
    (decks, skipped_duplicate_count)
}

pub(super) fn count_deck_sequences(buckets: &[CandidateBucket], slots: usize, cap: usize) -> usize {
    if cap == 0 {
        return 0;
    }
    let mut counts = vec![0_usize; slots + 1];
    counts[0] = 1;
    for bucket in buckets {
        let mut next = vec![0_usize; slots + 1];
        for used in 0..=slots {
            let current = counts[used];
            if current == 0 {
                continue;
            }
            let max_take = bucket.count.min(slots - used);
            for take in 0..=max_take {
                let interleavings = choose(used + take, take);
                let contribution = current.saturating_mul(interleavings).min(cap);
                next[used + take] = next[used + take].saturating_add(contribution).min(cap);
            }
        }
        counts = next;
    }
    counts[slots].min(cap)
}

fn choose(n: usize, k: usize) -> usize {
    let k = k.min(n - k);
    if k == 0 {
        return 1;
    }
    let mut result = 1_usize;
    for step in 1..=k {
        result = result * (n + 1 - step) / step;
    }
    result
}

fn sort_stratified_candidate_decks_single_thread(decks: &mut [CandidateDeck], visit_seed: u64) {
    decks.sort_by(|left, right| compare_stratified_decks(left, right, visit_seed));
}

fn sort_stratified_candidate_decks_parallel(decks: &mut Vec<CandidateDeck>, visit_seed: u64) {
    if decks.len() <= 1 {
        return;
    }
    let worker_count = stratified_worker_count(decks.len(), STRATIFIED_SORT_CHUNK_TARGET);
    if worker_count <= 1 {
        sort_stratified_candidate_decks_single_thread(decks, visit_seed);
        return;
    }

    let items = std::mem::take(decks)
        .into_iter()
        .map(|deck| StratifiedSortItem {
            hash: stable_hash64(visit_seed, &deck.key),
            deck,
        })
        .collect::<Vec<_>>();
    let chunks = split_stratified_sort_items(items, worker_count);
    let sorted_chunks = thread::scope(|scope| {
        let mut handles = Vec::new();
        for mut chunk in chunks {
            handles.push(scope.spawn(move || {
                chunk.sort_by(compare_stratified_sort_items);
                chunk
            }));
        }
        handles
            .into_iter()
            .map(|handle| handle.join().expect("solver sort worker panicked"))
            .collect::<Vec<_>>()
    });
    *decks = merge_stratified_sort_chunks(sorted_chunks)
        .into_iter()
        .map(|item| item.deck)
        .collect();
}

fn split_stratified_sort_items(
    items: Vec<StratifiedSortItem>,
    worker_count: usize,
) -> Vec<Vec<StratifiedSortItem>> {
    let chunk_size = items.len().div_ceil(worker_count);
    let mut chunks = Vec::with_capacity(worker_count);
    let mut chunk = Vec::with_capacity(chunk_size);
    for item in items {
        chunk.push(item);
        if chunk.len() == chunk_size {
            chunks.push(chunk);
            chunk = Vec::with_capacity(chunk_size);
        }
    }
    if !chunk.is_empty() {
        chunks.push(chunk);
    }
    chunks
}

fn merge_stratified_sort_chunks(
    mut chunks: Vec<Vec<StratifiedSortItem>>,
) -> Vec<StratifiedSortItem> {
    while chunks.len() > 1 {
        let mut merged = Vec::with_capacity(chunks.len().div_ceil(2));
        let mut iter = chunks.into_iter();
        while let Some(left) = iter.next() {
            if let Some(right) = iter.next() {
                merged.push(merge_two_stratified_sort_chunks(left, right));
            } else {
                merged.push(left);
            }
        }
        chunks = merged;
    }
    chunks.pop().unwrap_or_default()
}

fn merge_two_stratified_sort_chunks(
    left: Vec<StratifiedSortItem>,
    right: Vec<StratifiedSortItem>,
) -> Vec<StratifiedSortItem> {
    let mut left = left.into_iter().peekable();
    let mut right = right.into_iter().peekable();
    let mut merged = Vec::with_capacity(left.size_hint().0 + right.size_hint().0);
    loop {
        match (left.peek(), right.peek()) {
            (Some(left_item), Some(right_item)) => {
                if compare_stratified_sort_items(left_item, right_item) != Ordering::Greater {
                    merged.push(left.next().expect("left item exists"));
                } else {
                    merged.push(right.next().expect("right item exists"));
                }
            }
            (Some(_), None) => {
                merged.extend(left);
                break;
            }
            (None, Some(_)) => {
                merged.extend(right);
                break;
            }
            (None, None) => break,
        }
    }
    merged
}

fn stratified_worker_count(item_count: usize, chunk_target: usize) -> usize {
    let target_limited_count = if chunk_target <= 1 {
        item_count
    } else {
        item_count.div_ceil(chunk_target)
    };
    thread::available_parallelism()
        .map(usize::from)
        .unwrap_or(1)
        .min(item_count)
        .min(target_limited_count.max(1))
        .max(1)
}

fn compare_stratified_decks(
    left: &CandidateDeck,
    right: &CandidateDeck,
    visit_seed: u64,
) -> Ordering {
    stable_hash64(visit_seed, &left.key)
        .cmp(&stable_hash64(visit_seed, &right.key))
        .then(left.key.cmp(&right.key))
}

fn compare_stratified_sort_items(
    left: &StratifiedSortItem,
    right: &StratifiedSortItem,
) -> Ordering {
    left.hash
        .cmp(&right.hash)
        .then(left.deck.key.cmp(&right.deck.key))
}

pub(super) fn candidate_deck_from_parts(
    key: String,
    deck: &[CardDefinition],
    leftovers: &[CandidateBucket],
    mode: SolverMode,
) -> CandidateDeck {
    let leftover_hand_card_ids = if mode == SolverMode::Hand {
        leftovers
            .iter()
            .flat_map(|bucket| std::iter::repeat_n(bucket.card.id, bucket.count))
            .collect()
    } else {
        Vec::new()
    };
    CandidateDeck {
        key,
        sort_key: deck_sort_key(deck),
        deck: deck.to_vec(),
        leftover_hand_card_ids,
    }
}

fn stable_hash64(seed: u64, text: &str) -> u64 {
    let mut hash = FNV_OFFSET_BASIS ^ seed;
    for byte in text.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(FNV_PRIME);
    }
    hash
}
