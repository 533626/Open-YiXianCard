//! 整数 id 键的查表哈希（FxHash 同款乘法混合）。
//!
//! 只用于**只查不遍历**的静态表（卡牌配置、卡牌特征）：std 默认的 SipHash 为抗 HashDoS 设计，
//! 在战斗热路径上按卡 id 查表时占了约 7% 的时间。哈希函数只影响遍历顺序，不影响查找结果；
//! 用到本模块的表都不按遍历顺序产生行为。
use std::collections::{HashMap, HashSet};
use std::hash::{BuildHasherDefault, Hasher};

#[derive(Default, Clone, Copy)]
pub struct IdHasher(u64);

const SEED: u64 = 0x51_7c_c1_b7_27_22_0a_95;

impl IdHasher {
    #[inline]
    fn mix(&mut self, word: u64) {
        self.0 = (self.0.rotate_left(5) ^ word).wrapping_mul(SEED);
    }
}

impl Hasher for IdHasher {
    #[inline]
    fn write(&mut self, bytes: &[u8]) {
        for chunk in bytes.chunks(8) {
            let mut word = [0u8; 8];
            word[..chunk.len()].copy_from_slice(chunk);
            self.mix(u64::from_le_bytes(word));
        }
    }

    #[inline]
    fn write_i64(&mut self, value: i64) {
        self.mix(value as u64);
    }

    #[inline]
    fn write_u64(&mut self, value: u64) {
        self.mix(value);
    }

    #[inline]
    fn write_usize(&mut self, value: usize) {
        self.mix(value as u64);
    }

    #[inline]
    fn finish(&self) -> u64 {
        self.0
    }
}

pub type IdBuildHasher = BuildHasherDefault<IdHasher>;
pub type IdMap<V> = HashMap<i64, V, IdBuildHasher>;
pub type IdSet = HashSet<i64, IdBuildHasher>;

/// Read-only flags for normalized card base ids. Canonical ids use small
/// local numbers within million-wide families; sparse pages remove hashing
/// from repeated classifications. Other ids keep the original exact lookup.
pub(crate) struct BaseIdFlags {
    pages: [Option<Box<[u16; 512]>>; 100],
    other: IdMap<u16>,
}

impl Default for BaseIdFlags {
    fn default() -> Self {
        Self {
            pages: std::array::from_fn(|_| None),
            other: IdMap::default(),
        }
    }
}

impl BaseIdFlags {
    fn page_slot(id: i64) -> Option<(usize, usize)> {
        let id = u64::try_from(id).ok()?;
        let page = id / 1_000_000;
        let slot = id % 1_000_000;
        (page < 100 && slot < 512).then_some((page as usize, slot as usize))
    }

    pub(crate) fn insert(&mut self, id: i64, flags: u16) {
        if let Some((page, slot)) = Self::page_slot(id) {
            self.pages[page].get_or_insert_with(|| Box::new([0; 512]))[slot] = flags;
        } else {
            self.other.insert(id, flags);
        }
    }

    #[inline]
    pub(crate) fn get(&self, id: i64) -> u16 {
        if let Some((page, slot)) = Self::page_slot(id) {
            self.pages[page].as_ref().map_or(0, |flags| flags[slot])
        } else {
            self.other.get(&id).copied().unwrap_or(0)
        }
    }
}

#[cfg(test)]
mod base_id_flags_tests {
    use super::*;

    #[test]
    fn sparse_flags_match_hash_lookup_for_all_domains() {
        let mut flags = BaseIdFlags::default();
        let mut reference = IdMap::default();
        let cases = [
            -1,
            0,
            1,
            511,
            512,
            999_999,
            1_000_000,
            1_000_511,
            1_000_512,
            99_000_001,
            100_000_001,
            i64::MAX,
            i64::MIN,
        ];
        for (i, id) in cases.into_iter().enumerate() {
            let value = (i * 7) as u16;
            flags.insert(id, value);
            reference.insert(id, value);
        }
        for id in cases.into_iter().chain(-5..520).chain(999_995..1_000_520) {
            assert_eq!(
                flags.get(id),
                reference.get(&id).copied().unwrap_or(0),
                "{id}"
            );
        }
        flags.insert(1, 99);
        assert_eq!(flags.get(1), 99);
    }
}
