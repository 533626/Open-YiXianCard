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
