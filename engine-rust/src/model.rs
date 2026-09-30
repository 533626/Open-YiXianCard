use serde::{Deserialize, Deserializer, Serialize, Serializer};
use std::fmt;
use std::ops::Deref;
use std::sync::Arc;

pub const DECK_SIZE: usize = 8;

/// 引擎按卡名判牌种时用到的关键字（`card.name.contains(..)` 的全部字面量）。卡名驻留时预算
/// 一个「含哪些关键字」的位掩码，热路径上的 `contains` 变成一次位测试。新增关键字只需加在这里；
/// 不在表里的模式照常走 `str::contains`，结果不变。
const NAME_TOKENS: [&str; 19] = [
    "星弈", "雷", "火灵", "木灵", "金灵", "灵阵", "水灵", "掌", "土灵", "卦", "云剑", "蛇", "猫",
    "狂剑", "灵气", "灵印", "灵剑", "印", "剑",
];

// inline(always)：调用点几乎都是字面量（`card.name.contains("云剑")`），内联后字符串比较在编译期折叠成常数位号，
// 否则每次出牌要在运行时对 19 个关键字逐个比串（剖析里单列 2%）。
#[inline(always)]
fn name_token_bit(needle: &str) -> Option<u32> {
    // 与 NAME_TOKENS 下标一一对应（单测 name_token_bits_match_table 锁定）
    Some(match needle {
        "星弈" => 0,
        "雷" => 1,
        "火灵" => 2,
        "木灵" => 3,
        "金灵" => 4,
        "灵阵" => 5,
        "水灵" => 6,
        "掌" => 7,
        "土灵" => 8,
        "卦" => 9,
        "云剑" => 10,
        "蛇" => 11,
        "猫" => 12,
        "狂剑" => 13,
        "灵气" => 14,
        "灵印" => 15,
        "灵剑" => 16,
        "印" => 17,
        "剑" => 18,
        _ => return None,
    })
}

#[inline(always)]
fn char_token_bit(needle: char) -> Option<u32> {
    // 单字关键字：与 NAME_TOKENS 中的下标一一对应
    match needle {
        '雷' => Some(1),
        '掌' => Some(7),
        '卦' => Some(9),
        '蛇' => Some(11),
        '猫' => Some(12),
        '印' => Some(17),
        '剑' => Some(18),
        _ => None,
    }
}

#[derive(Debug)]
pub struct InternedStr {
    text: &'static str,
    token_mask: u32,
}

/// 只读共享字符串：全局驻留，克隆就是复制一个指针（不碰引用计数），相等比较是指针比较
/// （驻留保证同内容同地址）。战斗里卡牌定义被大量克隆（出牌预演会克隆出牌方），
/// `String` 字段每次克隆都要堆分配。卡名、副职名、枚举名都是有限集合（含夹具补丁造的
/// `card:{id}`），驻留表不会无界增长。序列化 / Debug / 排序 / 哈希与 `String` 完全一致。
#[derive(Clone, Copy)]
pub struct SharedStr(&'static InternedStr);

fn intern(value: &str) -> &'static InternedStr {
    use std::collections::HashMap;
    use std::sync::{Mutex, OnceLock};
    static TABLE: OnceLock<Mutex<HashMap<&'static str, &'static InternedStr>>> = OnceLock::new();
    let mut table = TABLE
        .get_or_init(|| Mutex::new(HashMap::new()))
        .lock()
        .expect("SharedStr intern table poisoned");
    if let Some(existing) = table.get(value) {
        return existing;
    }
    let text: &'static str = Box::leak(value.to_owned().into_boxed_str());
    let token_mask = NAME_TOKENS
        .iter()
        .enumerate()
        .filter(|(_, token)| text.contains(**token))
        .fold(0_u32, |mask, (index, _)| mask | 1 << index);
    let interned: &'static InternedStr = Box::leak(Box::new(InternedStr { text, token_mask }));
    table.insert(text, interned);
    interned
}

/// `SharedStr::contains` 接受的模式：关键字表里的走位测试，其余照常 `str::contains`。
pub trait NameNeedle {
    fn token_bit(&self) -> Option<u32>;
    fn found_in(&self, haystack: &str) -> bool;
}

impl NameNeedle for &str {
    #[inline(always)]
    fn token_bit(&self) -> Option<u32> {
        name_token_bit(self)
    }
    fn found_in(&self, haystack: &str) -> bool {
        haystack.contains(*self)
    }
}

impl NameNeedle for &&str {
    #[inline(always)]
    fn token_bit(&self) -> Option<u32> {
        name_token_bit(self)
    }
    fn found_in(&self, haystack: &str) -> bool {
        haystack.contains(**self)
    }
}

impl NameNeedle for &String {
    #[inline(always)]
    fn token_bit(&self) -> Option<u32> {
        name_token_bit(self)
    }
    fn found_in(&self, haystack: &str) -> bool {
        haystack.contains(self.as_str())
    }
}

impl NameNeedle for char {
    #[inline(always)]
    fn token_bit(&self) -> Option<u32> {
        char_token_bit(*self)
    }
    fn found_in(&self, haystack: &str) -> bool {
        haystack.contains(*self)
    }
}

impl Default for SharedStr {
    fn default() -> Self {
        Self::from("")
    }
}

impl SharedStr {
    pub fn as_str(&self) -> &'static str {
        self.0.text
    }

    /// 同 `str::contains`；关键字表（`NAME_TOKENS`）内的模式查驻留时预算的位掩码。
    #[inline(always)]
    pub fn contains<N: NameNeedle>(&self, needle: N) -> bool {
        match needle.token_bit() {
            Some(bit) => self.0.token_mask & (1 << bit) != 0,
            None => needle.found_in(self.0.text),
        }
    }
}

impl Deref for SharedStr {
    type Target = str;
    fn deref(&self) -> &str {
        self.0.text
    }
}

impl AsRef<str> for SharedStr {
    fn as_ref(&self) -> &str {
        self.0.text
    }
}

impl std::borrow::Borrow<str> for SharedStr {
    fn borrow(&self) -> &str {
        self.0.text
    }
}

impl PartialEq for SharedStr {
    fn eq(&self, other: &Self) -> bool {
        std::ptr::eq(self.0, other.0)
    }
}

impl Eq for SharedStr {}

impl std::hash::Hash for SharedStr {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        self.0.text.hash(state)
    }
}

impl PartialOrd for SharedStr {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for SharedStr {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        self.0.text.cmp(other.0.text)
    }
}

impl fmt::Debug for SharedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(self.0.text, f)
    }
}

impl fmt::Display for SharedStr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Display::fmt(self.0.text, f)
    }
}

impl From<String> for SharedStr {
    fn from(value: String) -> Self {
        Self(intern(&value))
    }
}

impl From<&str> for SharedStr {
    fn from(value: &str) -> Self {
        Self(intern(value))
    }
}

impl From<&String> for SharedStr {
    fn from(value: &String) -> Self {
        Self(intern(value))
    }
}

impl From<SharedStr> for String {
    fn from(value: SharedStr) -> Self {
        value.0.text.to_string()
    }
}

impl PartialEq<str> for SharedStr {
    fn eq(&self, other: &str) -> bool {
        self.0.text == other
    }
}

impl PartialEq<&str> for SharedStr {
    fn eq(&self, other: &&str) -> bool {
        self.0.text == *other
    }
}

impl PartialEq<String> for SharedStr {
    fn eq(&self, other: &String) -> bool {
        self.0.text == other.as_str()
    }
}

impl PartialEq<SharedStr> for str {
    fn eq(&self, other: &SharedStr) -> bool {
        self == other.0.text
    }
}

impl PartialEq<SharedStr> for &str {
    fn eq(&self, other: &SharedStr) -> bool {
        *self == other.0.text
    }
}

impl PartialEq<SharedStr> for String {
    fn eq(&self, other: &SharedStr) -> bool {
        self.as_str() == other.0.text
    }
}

impl Serialize for SharedStr {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(self.0.text)
    }
}

impl<'de> Deserialize<'de> for SharedStr {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        String::deserialize(deserializer).map(Self::from)
    }
}

const INLINE_INTS: usize = 4;

/// 只读整数表（`otherParams`）：原版卡表最长 4 项，≤4 项内联存放、克隆是一次内存拷贝；
/// 更长的（只可能来自夹具补丁）退回共享堆存储。序列化 / Debug / 比较与 `Vec<i64>` 一致。
#[derive(Clone)]
pub struct SharedInts(IntsRepr);

#[derive(Clone)]
enum IntsRepr {
    Inline { len: u8, data: [i64; INLINE_INTS] },
    Heap(Arc<[i64]>),
}

impl SharedInts {
    fn from_slice(value: &[i64]) -> Self {
        if value.len() <= INLINE_INTS {
            let mut data = [0; INLINE_INTS];
            data[..value.len()].copy_from_slice(value);
            Self(IntsRepr::Inline {
                len: value.len() as u8,
                data,
            })
        } else {
            Self(IntsRepr::Heap(Arc::from(value)))
        }
    }
}

impl Default for SharedInts {
    fn default() -> Self {
        Self(IntsRepr::Inline {
            len: 0,
            data: [0; INLINE_INTS],
        })
    }
}

impl Deref for SharedInts {
    type Target = [i64];
    fn deref(&self) -> &[i64] {
        match &self.0 {
            IntsRepr::Inline { len, data } => &data[..*len as usize],
            IntsRepr::Heap(values) => values,
        }
    }
}

impl AsRef<[i64]> for SharedInts {
    fn as_ref(&self) -> &[i64] {
        self
    }
}

impl PartialEq for SharedInts {
    fn eq(&self, other: &Self) -> bool {
        **self == **other
    }
}

impl Eq for SharedInts {}

impl std::hash::Hash for SharedInts {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        (**self).hash(state)
    }
}

impl fmt::Debug for SharedInts {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        fmt::Debug::fmt(&**self, f)
    }
}

impl From<Vec<i64>> for SharedInts {
    fn from(value: Vec<i64>) -> Self {
        Self::from_slice(&value)
    }
}

impl From<&[i64]> for SharedInts {
    fn from(value: &[i64]) -> Self {
        Self::from_slice(value)
    }
}

impl From<&Vec<i64>> for SharedInts {
    fn from(value: &Vec<i64>) -> Self {
        Self::from_slice(value)
    }
}

impl PartialEq<Vec<i64>> for SharedInts {
    fn eq(&self, other: &Vec<i64>) -> bool {
        **self == **other
    }
}

impl PartialEq<[i64]> for SharedInts {
    fn eq(&self, other: &[i64]) -> bool {
        **self == *other
    }
}

impl<const N: usize> PartialEq<[i64; N]> for SharedInts {
    fn eq(&self, other: &[i64; N]) -> bool {
        **self == other[..]
    }
}

impl<'a> IntoIterator for &'a SharedInts {
    type Item = &'a i64;
    type IntoIter = std::slice::Iter<'a, i64>;
    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

impl Serialize for SharedInts {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        (**self).serialize(serializer)
    }
}

impl<'de> Deserialize<'de> for SharedInts {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Vec::<i64>::deserialize(deserializer).map(Self::from)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum PlayerSide {
    P1,
    P2,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CardDefinition {
    pub id: i64,
    #[serde(rename = "baseId", default)]
    pub base_id: Option<i64>,
    pub name: SharedStr,
    #[serde(rename = "cardType", default)]
    pub card_type: Option<OriginalEnumValue>,
    #[serde(default)]
    pub rarity: Option<i64>,
    #[serde(rename = "careerName", default)]
    pub career_name: Option<SharedStr>,
    #[serde(default)]
    pub attack: Option<i64>,
    #[serde(rename = "randomAttack", default)]
    pub random_attack: Option<i64>,
    #[serde(rename = "randomDef", default)]
    pub random_defense: Option<i64>,
    #[serde(rename = "attackCount", default)]
    pub attack_count: Option<i64>,
    #[serde(alias = "def", default)]
    pub defense: Option<i64>,
    #[serde(default)]
    pub damage: Option<i64>,
    #[serde(default)]
    pub anima: Option<i64>,
    #[serde(rename = "hpCost", default)]
    pub hp_cost: Option<i64>,
    #[serde(rename = "actionAgain", default)]
    pub action_again: Option<bool>,
    #[serde(default)]
    pub physique: Option<i64>,
    #[serde(rename = "jianYi", alias = "swordIntent", default)]
    pub sword_intent: Option<i64>,
    #[serde(rename = "guaXiang", default)]
    pub hexagram: Option<i64>,
    #[serde(rename = "otherParams", default)]
    pub other_params: SharedInts,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct OriginalEnumValue {
    pub value: i64,
    pub name: SharedStr,
}

#[cfg(test)]
mod shared_str_tests {
    use super::*;

    #[test]
    fn name_token_bits_match_table() {
        for (index, token) in NAME_TOKENS.iter().enumerate() {
            assert_eq!(name_token_bit(token), Some(index as u32), "{token}");
            let mut chars = token.chars();
            if let (Some(ch), None) = (chars.next(), chars.next()) {
                assert_eq!(char_token_bit(ch), Some(index as u32), "{token}");
            }
        }
        assert_eq!(name_token_bit("灵"), None);
        assert_eq!(char_token_bit('灵'), None);
    }

    #[test]
    fn shared_str_contains_matches_str_contains() {
        let names = [
            "云剑•飞刺",
            "狂剑•一式",
            "巨虎灵剑",
            "星弈•断",
            "火灵印",
            "",
            "普通攻击",
            "雷剑",
        ];
        let needles = [
            "云剑", "狂剑", "剑", "灵剑", "星弈", "火灵", "印", "灵印", "雷", "攻击", "", "x",
        ];
        for name in names {
            let shared = SharedStr::from(name);
            for needle in needles {
                assert_eq!(
                    shared.contains(needle),
                    name.contains(needle),
                    "{name} / {needle}"
                );
                assert_eq!(
                    shared.contains(needle),
                    name.contains(needle),
                    "{name} / {needle}"
                );
            }
            for ch in ['剑', '雷', '印', '掌', '灵', 'a'] {
                assert_eq!(shared.contains(ch), name.contains(ch), "{name} / {ch}");
            }
            assert_eq!(SharedStr::from(name.to_string()), shared);
            assert_eq!(
                serde_json::to_string(&shared).unwrap(),
                serde_json::to_string(name).unwrap()
            );
        }
        assert_ne!(SharedStr::from("云剑"), SharedStr::from("狂剑"));
    }

    #[test]
    fn shared_ints_behave_like_vec() {
        for values in [vec![], vec![1], vec![1, 2, 3, 4], vec![1, 2, 3, 4, 5, 6]] {
            let shared = SharedInts::from(values.clone());
            assert_eq!(&*shared, values.as_slice());
            assert_eq!(shared.clone(), SharedInts::from(values.as_slice()));
            assert_eq!(format!("{shared:?}"), format!("{values:?}"));
            assert_eq!(
                serde_json::to_string(&shared).unwrap(),
                serde_json::to_string(&values).unwrap()
            );
        }
    }
}
