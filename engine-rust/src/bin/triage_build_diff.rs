//! 把 build diff 的新增/变更条目按「是否该进引擎」分流，供换代评审只看存疑项。
//!
//! TS 原版：`battle-evaluator/scripts/triage-build-diff.ts`（本文件为其逐字移植，
//! 判据链与缓存格式以该文件头注释为准，这里不重复解释语义）。移植约定：
//!
//! - 语义、stdout/stderr 文案、退出码、`.corpus-index.cache` 路径与格式逐字对齐；
//!   usage 行已随 TS 退役改成本 bin 的 cargo 调用形（对拍期曾保持 TS 原文）。
//! - JS Number 语义用 f64 模拟；id/season 仅在整数值时收进 i64 键
//!   （真实语料/diff 中恒为整数；非整数键在 TS 里也永远无法与 diff 条目互查）。
//! - 仓库根从 cwd（其次 exe 路径）逐级上溯找 `engine-rust/data/card-effect-catalog.json`
//!   定位，catalog 以数据文件读取，不编译期耦合。
//!
//! 用法：`triage_build_diff <diff.json> [--corpus <fixtures-dir>] [--index <index.json>] [--refresh-index] [--json]`

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::fs;
use std::io;
use std::path::Path;

// 0 无赛季、9 天衍仙命、10 仙魔（build 25621897 起）。Set 迭代序为插入序：0/9/10。
const CURRENT_SEASON_MECHANISMS: [f64; 3] = [0.0, 9.0, 10.0];
const CURRENT_SEASON_NOTE: &str = "0/9/10";

// ---------- sha256（TS 侧 node:crypto createHash("sha256") 的等价实现，不引新依赖） ----------

mod sha256 {
    const K: [u32; 64] = [
        0x428a2f98, 0x71374491, 0xb5c0fbcf, 0xe9b5dba5, 0x3956c25b, 0x59f111f1, 0x923f82a4, 0xab1c5ed5,
        0xd807aa98, 0x12835b01, 0x243185be, 0x550c7dc3, 0x72be5d74, 0x80deb1fe, 0x9bdc06a7, 0xc19bf174,
        0xe49b69c1, 0xefbe4786, 0x0fc19dc6, 0x240ca1cc, 0x2de92c6f, 0x4a7484aa, 0x5cb0a9dc, 0x76f988da,
        0x983e5152, 0xa831c66d, 0xb00327c8, 0xbf597fc7, 0xc6e00bf3, 0xd5a79147, 0x06ca6351, 0x14292967,
        0x27b70a85, 0x2e1b2138, 0x4d2c6dfc, 0x53380d13, 0x650a7354, 0x766a0abb, 0x81c2c92e, 0x92722c85,
        0xa2bfe8a1, 0xa81a664b, 0xc24b8b70, 0xc76c51a3, 0xd192e819, 0xd6990624, 0xf40e3585, 0x106aa070,
        0x19a4c116, 0x1e376c08, 0x2748774c, 0x34b0bcb5, 0x391c0cb3, 0x4ed8aa4a, 0x5b9cca4f, 0x682e6ff3,
        0x748f82ee, 0x78a5636f, 0x84c87814, 0x8cc70208, 0x90befffa, 0xa4506ceb, 0xbef9a3f7, 0xc67178f2,
    ];

    pub fn digest(data: &[u8]) -> [u8; 32] {
        let mut h: [u32; 8] = [
            0x6a09e667, 0xbb67ae85, 0x3c6ef372, 0xa54ff53a, 0x510e527f, 0x9b05688c, 0x1f83d9ab, 0x5be0cd19,
        ];
        let mut msg = data.to_vec();
        let bit_len = (data.len() as u64) * 8;
        msg.push(0x80);
        while msg.len() % 64 != 56 {
            msg.push(0);
        }
        msg.extend_from_slice(&bit_len.to_be_bytes());
        for block in msg.chunks_exact(64) {
            let mut w = [0u32; 64];
            for (i, word) in block.chunks_exact(4).enumerate() {
                w[i] = u32::from_be_bytes([word[0], word[1], word[2], word[3]]);
            }
            for i in 16..64 {
                let s0 = w[i - 15].rotate_right(7) ^ w[i - 15].rotate_right(18) ^ (w[i - 15] >> 3);
                let s1 = w[i - 2].rotate_right(17) ^ w[i - 2].rotate_right(19) ^ (w[i - 2] >> 10);
                w[i] = w[i - 16]
                    .wrapping_add(s0)
                    .wrapping_add(w[i - 7])
                    .wrapping_add(s1);
            }
            let (mut a, mut b, mut c, mut d, mut e, mut f, mut g, mut hh) =
                (h[0], h[1], h[2], h[3], h[4], h[5], h[6], h[7]);
            for i in 0..64 {
                let s1 = e.rotate_right(6) ^ e.rotate_right(11) ^ e.rotate_right(25);
                let ch = (e & f) ^ ((!e) & g);
                let temp1 = hh
                    .wrapping_add(s1)
                    .wrapping_add(ch)
                    .wrapping_add(K[i])
                    .wrapping_add(w[i]);
                let s0 = a.rotate_right(2) ^ a.rotate_right(13) ^ a.rotate_right(22);
                let maj = (a & b) ^ (a & c) ^ (b & c);
                let temp2 = s0.wrapping_add(maj);
                hh = g;
                g = f;
                f = e;
                e = d.wrapping_add(temp1);
                d = c;
                c = b;
                b = a;
                a = temp1.wrapping_add(temp2);
            }
            h[0] = h[0].wrapping_add(a);
            h[1] = h[1].wrapping_add(b);
            h[2] = h[2].wrapping_add(c);
            h[3] = h[3].wrapping_add(d);
            h[4] = h[4].wrapping_add(e);
            h[5] = h[5].wrapping_add(f);
            h[6] = h[6].wrapping_add(g);
            h[7] = h[7].wrapping_add(hh);
        }
        let mut out = [0u8; 32];
        for (i, word) in h.iter().enumerate() {
            out[i * 4..i * 4 + 4].copy_from_slice(&word.to_be_bytes());
        }
        out
    }

    pub fn hex(data: &[u8]) -> String {
        digest(data).iter().map(|b| format!("{b:02x}")).collect()
    }
}

// ---------- JS 语义小工具 ----------

/// JS `Number(x)` 的字符串解析近似（十进制/科学计数/inf/nan；不含十六进制——真实 diff key 不出现）。
fn js_parse_number(s: &str) -> f64 {
    let t = s.trim();
    if t.is_empty() {
        return 0.0;
    }
    t.parse::<f64>().unwrap_or(f64::NAN)
}

/// JSON 值 → JS Number 语义（Number(null)=0）。
fn js_number(value: &serde_json::Value) -> f64 {
    match value {
        serde_json::Value::Number(n) => n.as_f64().unwrap_or(f64::NAN),
        serde_json::Value::String(s) => js_parse_number(s),
        serde_json::Value::Null => 0.0,
        serde_json::Value::Bool(b) => {
            if *b {
                1.0
            } else {
                0.0
            }
        }
        _ => f64::NAN,
    }
}

/// 整数值 f64 → i64（真实数据恒为整数；非整数键在 TS 的 Map 里也无法与 diff 条目互查）。
fn canonical_id(x: f64) -> Option<i64> {
    if x.is_finite() && x.fract() == 0.0 && x.abs() <= 9.007_199_254_740_991e15 {
        Some(x as i64)
    } else {
        None
    }
}

/// JS Number → string（模板串与 `${key}` 拼接用）。Rust `Display for f64` 同为
/// 最短往返表示且整数不带小数点，与 JS 一致（≥1e21 的指数表示在本工具数据域外）。
fn js_num_string(x: f64) -> String {
    format!("{x}")
}

/// JSON 数字序列化：整数值输出为整数（JS 的 311.0 序列化为 311 而非 311.0）。
struct JsNum(f64);

impl Serialize for JsNum {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match canonical_id(self.0) {
            Some(i) => serializer.serialize_i64(i),
            None => serializer.serialize_f64(self.0),
        }
    }
}

/// 与 engine-rust/src/replay/support.rs::normalize_base_id 保持一致：剥离稀有度位。
fn normalize_base_id(card_id: f64) -> f64 {
    if card_id == 0.0 || card_id == 10_000.0 || card_id == 20_000.0 {
        return 0.0;
    }
    card_id - ((card_id % 1_000_000.0) / 10_000.0).floor() * 10_000.0
}

/// JS seasons 的 `[...set].sort()`：默认比较器按字符串排序（"10" < "9"）。
fn sort_seasons_js(mut seasons: Vec<i64>) -> Vec<i64> {
    seasons.sort_by_key(|s| s.to_string());
    seasons
}

fn join_seasons(seasons: &[i64]) -> String {
    seasons
        .iter()
        .map(|s| s.to_string())
        .collect::<Vec<_>>()
        .join("/")
}

/// 近似 JS `path.resolve`：词法归一（不解析符号链接）。
fn resolve_path(p: &str) -> String {
    let cwd = std::env::current_dir()
        .map(|d| d.to_string_lossy().to_string())
        .unwrap_or_default();
    let combined = if p.starts_with('/') {
        p.to_string()
    } else {
        format!("{cwd}/{p}")
    };
    let mut parts: Vec<&str> = Vec::new();
    for comp in combined.split('/') {
        match comp {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            c => parts.push(c),
        }
    }
    if parts.is_empty() {
        return "/".to_string();
    }
    format!("/{}", parts.join("/"))
}

fn fatal(msg: &str) -> ! {
    eprintln!("{msg}");
    std::process::exit(1);
}

// ---------- 参数解析 ----------

struct Args {
    diff_path: String,
    corpus_dir: String,
    index_path: String,
    refresh_index: bool,
    json: bool,
}

fn parse_args(argv: &[String]) -> Args {
    let mut diff_path = String::new();
    let mut corpus_dir = String::new();
    let mut index_path = String::new();
    let mut refresh_index = false;
    let mut json = false;
    let mut i = 0;
    while i < argv.len() {
        let arg = &argv[i];
        i += 1;
        match arg.as_str() {
            "--corpus" => {
                corpus_dir = argv.get(i).cloned().unwrap_or_default();
                i += 1;
            }
            "--index" => {
                index_path = argv.get(i).cloned().unwrap_or_default();
                i += 1;
            }
            "--refresh-index" => refresh_index = true,
            "--json" => json = true,
            other => {
                if !other.starts_with("--") {
                    diff_path = other.to_string();
                }
            }
        }
    }
    if diff_path.is_empty() {
        eprintln!(
            "usage: cargo run --release --manifest-path engine-rust/Cargo.toml --bin triage_build_diff -- <diff.json> [--corpus <fixtures-dir>] [--index <index.json>] [--refresh-index] [--json]"
        );
        std::process::exit(2);
    }
    Args {
        diff_path: resolve_path(&diff_path),
        corpus_dir: if corpus_dir.is_empty() { String::new() } else { resolve_path(&corpus_dir) },
        index_path: if index_path.is_empty() { String::new() } else { resolve_path(&index_path) },
        refresh_index,
        json,
    }
}

// ---------- diff 输入 ----------

#[derive(Deserialize)]
struct DiffRecord {
    key: serde_json::Value,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    #[allow(dead_code)]
    changes: serde_json::Value,
}

#[derive(Deserialize)]
struct TableDiff {
    #[serde(default, rename = "addedRecords")]
    added_records: Option<Vec<DiffRecord>>,
    #[serde(default)]
    changed: Option<Vec<DiffRecord>>,
}

impl TableDiff {
    fn added_records(&self) -> &[DiffRecord] {
        self.added_records.as_deref().unwrap_or(&[])
    }
    fn changed(&self) -> &[DiffRecord] {
        self.changed.as_deref().unwrap_or(&[])
    }
}

#[derive(Deserialize, Clone)]
struct Sources {
    #[serde(default)]
    added: Option<Vec<String>>,
    #[serde(default)]
    changed: Option<Vec<String>>,
}

impl Sources {
    fn added(&self) -> &[String] {
        self.added.as_deref().unwrap_or(&[])
    }
    fn changed(&self) -> &[String] {
        self.changed.as_deref().unwrap_or(&[])
    }
    fn all(&self) -> Vec<&str> {
        self.added().iter().chain(self.changed().iter()).map(String::as_str).collect()
    }
}

#[derive(Deserialize)]
struct BuildInfo {
    #[serde(default, rename = "steamBuild")]
    steam_build: Option<String>,
}

#[derive(Deserialize)]
struct BuildDiff {
    old: BuildInfo,
    new: BuildInfo,
    #[serde(default)]
    tables: BTreeMap<String, Option<TableDiff>>,
    #[serde(default)]
    sources: Option<Sources>,
}

impl BuildDiff {
    fn sources(&self) -> Sources {
        // sources 缺失时 TS 会读到 undefined?.added → 崩溃；这里按空表处理（真实 diff 恒有）。
        self.sources
            .clone()
            .unwrap_or(Sources { added: None, changed: None })
    }
    fn steam_build(info: &BuildInfo) -> &str {
        info.steam_build.as_deref().unwrap_or("undefined")
    }
}

// ---------- 语料索引 ----------

struct CorpusIndex {
    card_hits: BTreeMap<i64, BTreeSet<i64>>,
    fate_hits: BTreeMap<i64, BTreeSet<i64>>,
    buff_hits: BTreeMap<i64, BTreeSet<i64>>,
    fixture_count: usize,
    fingerprint: String,
}

#[derive(Serialize, Deserialize)]
struct SerializedCorpusIndex {
    #[serde(rename = "schemaVersion")]
    schema_version: u8,
    #[serde(rename = "corpusDir")]
    corpus_dir: String,
    fingerprint: String,
    #[serde(rename = "fixtureCount")]
    fixture_count: usize,
    #[serde(rename = "cardHits")]
    card_hits: Vec<(i64, Vec<i64>)>,
    #[serde(rename = "fateHits")]
    fate_hits: Vec<(i64, Vec<i64>)>,
    #[serde(rename = "buffHits")]
    buff_hits: Vec<(i64, Vec<i64>)>,
}

/// 与 TS 侧同序：条目按 id 升序，seasons 按 JS 字符串排序。
fn serialize_corpus_index(corpus_dir: &str, index: &CorpusIndex) -> SerializedCorpusIndex {
    let encode = |map: &BTreeMap<i64, BTreeSet<i64>>| {
        map.iter()
            .map(|(id, seasons)| (*id, sort_seasons_js(seasons.iter().copied().collect())))
            .collect()
    };
    SerializedCorpusIndex {
        schema_version: 1,
        corpus_dir: corpus_dir.to_string(),
        fingerprint: index.fingerprint.clone(),
        fixture_count: index.fixture_count,
        card_hits: encode(&index.card_hits),
        fate_hits: encode(&index.fate_hits),
        buff_hits: encode(&index.buff_hits),
    }
}

fn deserialize_corpus_index(value: SerializedCorpusIndex, corpus_dir: &str) -> Option<CorpusIndex> {
    if value.schema_version != 1 || value.corpus_dir != corpus_dir {
        return None;
    }
    let decode = |entries: Vec<(i64, Vec<i64>)>| entries.into_iter().map(|(id, seasons)| (id, seasons.into_iter().collect())).collect();
    Some(CorpusIndex {
        fixture_count: value.fixture_count,
        fingerprint: value.fingerprint,
        card_hits: decode(value.card_hits),
        fate_hits: decode(value.fate_hits),
        buff_hits: decode(value.buff_hits),
    })
}

/// 递归收集 .json fixture，顺序与 `readdirSync` 一致（OS dirent 原序，即读即递归）。
fn collect_fixtures(root: &str) -> io::Result<Vec<String>> {
    let mut out = Vec::new();
    fn walk(dir: &str, out: &mut Vec<String>) -> io::Result<()> {
        for entry in fs::read_dir(dir)? {
            let entry = entry?;
            let path = format!("{}/{}", dir.trim_end_matches('/'), entry.file_name().to_string_lossy());
            if entry.metadata().map(|m| m.is_dir()).unwrap_or(false) {
                walk(&path, out)?;
            } else if entry.file_name().to_string_lossy().ends_with(".json") {
                out.push(path);
            }
        }
        Ok(())
    }
    walk(root, &mut out)?;
    Ok(out)
}

/// Metadata fingerprint is intentionally cheap after the initial index build;
/// --refresh-index is the explicit invalidation edge when a fixture changes.
fn corpus_fingerprint(root: &str, paths: &[String]) -> io::Result<String> {
    let mut metadata = String::new();
    for (i, path) in paths.iter().enumerate() {
        if i > 0 {
            metadata.push('\n');
        }
        let info = fs::metadata(path)?;
        let (secs, nanos) = match info.modified()? {
            std::time::SystemTime::UNIX_EPOCH => (0i64, 0u32),
            t => {
                let d = t.duration_since(std::time::SystemTime::UNIX_EPOCH).unwrap_or_default();
                (d.as_secs() as i64, d.subsec_nanos())
            }
        };
        // Node statSync().mtimeMs：秒*1000 + 纳秒/1e6 的 double。
        let mtime_ms = secs as f64 * 1000.0 + nanos as f64 / 1e6;
        metadata.push_str(&format!(
            "{}:{}:{}",
            &path[root.len().min(path.len())..],
            info.len(),
            js_num_string(mtime_ms)
        ));
    }
    Ok(sha256::hex(metadata.as_bytes()))
}

fn load_corpus_index(path: &str, corpus_dir: &str) -> Option<CorpusIndex> {
    let load = || -> Option<CorpusIndex> {
        let text = fs::read_to_string(path).ok()?;
        let value: SerializedCorpusIndex = serde_json::from_str(&text).ok()?;
        let fixtures = collect_fixtures(corpus_dir).ok()?;
        if value.fingerprint != corpus_fingerprint(corpus_dir, &fixtures).ok()? {
            eprintln!("corpus index stale: {path}");
            return None;
        }
        deserialize_corpus_index(value, corpus_dir)
    };
    load()
}

fn record(map: &mut BTreeMap<i64, BTreeSet<i64>>, id: f64, season: i64) {
    if !id.is_finite() {
        return;
    }
    let Some(id) = canonical_id(id) else { return };
    map.entry(id).or_default().insert(season);
}

/// 只统计当前赛季语料的出现次数。历史赛季 fixture 里出现过不构成实现理由。
fn build_corpus_index(corpus_dir: &str) -> io::Result<CorpusIndex> {
    let mut card_hits = BTreeMap::new();
    let mut fate_hits = BTreeMap::new();
    let mut buff_hits = BTreeMap::new();
    let mut fixture_count = 0usize;

    let fixture_paths = collect_fixtures(corpus_dir)?;
    let fingerprint = corpus_fingerprint(corpus_dir, &fixture_paths)?;
    for path in &fixture_paths {
        let Ok(text) = fs::read_to_string(path) else { continue };
        let Ok(fixture) = serde_json::from_str::<serde_json::Value>(&text) else { continue };
        // fixture?.source?.seasonMechanism ?? 0
        let season_f = fixture
            .get("source")
            .and_then(|s| s.get("seasonMechanism"))
            .map(js_number)
            .unwrap_or(0.0);
        if !CURRENT_SEASON_MECHANISMS.contains(&season_f) {
            continue;
        }
        let season = season_f as i64;
        fixture_count += 1;

        for side in ["p1", "p2"] {
            let Some(player) = fixture.get("players").and_then(|p| p.get(side)) else { continue };

            if let Some(cards) = player.get("cards").and_then(|v| v.as_array()) {
                for card in cards {
                    if let Some(id) = card.get("id").filter(|v| v.is_number()) {
                        record(&mut card_hits, normalize_base_id(js_number(id)), season);
                    }
                }
            }
            for field in ["handCards", "lastRoundUsedCardBaseIds"] {
                if let Some(ids) = player.get(field).and_then(|v| v.as_array()) {
                    for id in ids {
                        if id.is_number() {
                            record(&mut card_hits, normalize_base_id(js_number(id)), season);
                        }
                    }
                }
            }

            if let Some(fates) = player.get("fateStrategies").and_then(|v| v.as_array()) {
                for id in fates {
                    if id.is_number() {
                        record(&mut fate_hits, js_number(id), season);
                    }
                }
            }
            // pendings 是已抽出但未选中的候选，同样证明该条目在当前赛季可达。
            if let Some(selections) = player.get("fateStrategySelections").and_then(|v| v.as_array()) {
                for selection in selections {
                    if let Some(pendings) = selection.get("pendings").and_then(|v| v.as_array()) {
                        for id in pendings {
                            if id.is_number() {
                                record(&mut fate_hits, js_number(id), season);
                            }
                        }
                    }
                    if let Some(selected) = selection.get("selected").filter(|v| v.is_number()) {
                        record(&mut fate_hits, js_number(selected), season);
                    }
                }
            }

            for field in ["permanentBuffTempDatas", "currentPermanentBuffTempDatas"] {
                match player.get(field) {
                    Some(bag @ serde_json::Value::Object(_)) => {
                        for key in bag.as_object().unwrap().keys() {
                            record(&mut buff_hits, js_parse_number(key), season);
                        }
                    }
                    Some(bag @ serde_json::Value::Array(_)) => {
                        for i in 0..bag.as_array().unwrap().len() {
                            record(&mut buff_hits, js_parse_number(&i.to_string()), season);
                        }
                    }
                    _ => {}
                }
            }
        }
    }

    Ok(CorpusIndex {
        card_hits,
        fate_hits,
        buff_hits,
        fixture_count,
        fingerprint,
    })
}

// ---------- 判定 ----------

fn load_executable_base_ids(repo_root: &str) -> HashSet<i64> {
    let text = fs::read_to_string(format!("{repo_root}/engine-rust/data/card-effect-catalog.json"))
        .unwrap_or_else(|e| fatal(&format!("card-effect-catalog.json read failed: {e}")));
    let catalog: serde_json::Value =
        serde_json::from_str(&text).unwrap_or_else(|e| fatal(&format!("card-effect-catalog.json parse failed: {e}")));
    catalog
        .get("executableBaseIds")
        .and_then(|v| v.as_array())
        .map(|ids| ids.iter().filter_map(|v| canonical_id(js_number(v))).collect())
        .unwrap_or_default()
}

/// diff 只给 id/名字，故用 Card_<id>.cs 这类命名反查本次改动的源文件。
fn match_source_file(table: &str, key_str: &str, sources: &Sources) -> Option<String> {
    let all = sources.all();
    let prefixes: &[String] = &match table {
        "cards" => vec![format!("Card_{key_str}.cs")],
        "buffs" => vec![format!("Buff_{key_str}.cs")],
        "fateStrategies" => vec![format!("FateStrategy_{key_str}.cs")],
        _ => vec![],
    };
    for candidate in prefixes {
        if all.iter().any(|f| f == candidate) {
            return Some(candidate.clone());
        }
    }
    None
}

struct Entry {
    table: &'static str,
    key: f64,
    name: Option<String>,
    kind: &'static str,
    verdict: &'static str,
    corpus_hits: usize,
    seasons: Vec<i64>,
    source_touched: Option<String>,
    reason: String,
    /// source-only 条目在 TS 里是另一套对象字面量，JSON 键序不同（verdict 提前）。
    source_only_order: bool,
}

impl Serialize for Entry {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut st = serializer.serialize_struct("Entry", 9)?;
        st.serialize_field("table", &self.table)?;
        st.serialize_field("key", &JsNum(self.key))?;
        st.serialize_field("name", &self.name)?;
        st.serialize_field("kind", &self.kind)?;
        if self.source_only_order {
            st.serialize_field("verdict", &self.verdict)?;
            st.serialize_field("corpusHits", &self.corpus_hits)?;
            st.serialize_field("seasons", &self.seasons)?;
            st.serialize_field("sourceTouched", &self.source_touched)?;
        } else {
            st.serialize_field("corpusHits", &self.corpus_hits)?;
            st.serialize_field("seasons", &self.seasons)?;
            st.serialize_field("sourceTouched", &self.source_touched)?;
            st.serialize_field("verdict", &self.verdict)?;
        }
        st.serialize_field("reason", &self.reason)?;
        st.end()
    }
}

fn lookup_seasons(map: Option<&BTreeMap<i64, BTreeSet<i64>>>, key: f64) -> Vec<i64> {
    map.and_then(|m| canonical_id(key).and_then(|k| m.get(&k)))
        .map(|set| sort_seasons_js(set.iter().copied().collect()))
        .unwrap_or_default()
}

fn classify(
    table: &'static str,
    record: &DiffRecord,
    kind: &'static str,
    executable: &HashSet<i64>,
    corpus: Option<&CorpusIndex>,
    sources: &Sources,
) -> Option<Entry> {
    let raw_key = js_number(&record.key);
    if !raw_key.is_finite() {
        return None;
    }
    // 卡表里 401/10401/20401 是同一张卡的三个稀有度变体，按 base id 归并成一条。
    let key = if table == "cards" { normalize_base_id(raw_key) } else { raw_key };

    let source_touched = match_source_file(table, &js_num_string(key), sources);
    let hit_map = match (table, corpus) {
        ("cards", Some(c)) => Some(&c.card_hits),
        ("fateStrategies", Some(c)) => Some(&c.fate_hits),
        (_, Some(c)) => Some(&c.buff_hits),
        (_, None) => None,
    };
    let seasons = lookup_seasons(hit_map, key);
    let corpus_hits = seasons.len();

    let mut entry = Entry {
        table,
        key,
        name: record.name.clone(),
        kind,
        verdict: "",
        corpus_hits,
        seasons,
        source_touched,
        reason: String::new(),
        source_only_order: false,
    };

    if table == "cards" && canonical_id(key).is_some_and(|k| executable.contains(&k)) {
        entry.verdict = "implemented";
        entry.reason = "base id 已在 card-effect-catalog".to_string();
        return Some(entry);
    }
    if corpus.is_none() {
        entry.verdict = "needs-review";
        entry.reason = "无语料索引，可达性未判定（私有 companion 缺失）".to_string();
        return Some(entry);
    }
    if corpus_hits > 0 {
        entry.verdict = "reachable";
        entry.reason = format!("当前赛季语料出现（season {}）", join_seasons(&entry.seasons));
        return Some(entry);
    }
    if let Some(source) = &entry.source_touched {
        entry.verdict = "needs-review";
        entry.reason = format!("语料 0 次，但 {source} 本次有改动");
        return Some(entry);
    }
    entry.verdict = "zero-coverage";
    entry.reason = "语料 0 次，不可达也不可验证，fail-closed".to_string();
    Some(entry)
}

/// `^Card_(\d+)\.cs$`
fn match_card_source_file(file: &str) -> Option<i64> {
    let rest = file.strip_prefix("Card_")?;
    let digits = rest.strip_suffix(".cs")?;
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    digits.parse::<i64>().ok()
}

fn find_repo_root() -> String {
    let mut candidates: Vec<String> = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        let mut dir: Option<&Path> = Some(cwd.as_path());
        while let Some(d) = dir {
            candidates.push(d.to_string_lossy().to_string());
            dir = d.parent();
        }
    }
    if let Ok(exe) = std::env::current_exe() {
        let mut dir: Option<&Path> = exe.parent();
        while let Some(d) = dir {
            candidates.push(d.to_string_lossy().to_string());
            dir = d.parent();
        }
    }
    for dir in candidates {
        if Path::new(&dir).join("engine-rust/data/card-effect-catalog.json").is_file() {
            return dir;
        }
    }
    fatal("cannot locate repo root: engine-rust/data/card-effect-catalog.json not found in any ancestor of cwd or exe");
}

#[derive(Serialize)]
struct Counts {
    implemented: usize,
    reachable: usize,
    #[serde(rename = "needs-review")]
    needs_review: usize,
    #[serde(rename = "zero-coverage")]
    zero_coverage: usize,
}

#[derive(Serialize)]
struct JsonOut {
    #[serde(rename = "oldBuild")]
    old_build: String,
    #[serde(rename = "newBuild")]
    new_build: String,
    #[serde(rename = "corpusAvailable")]
    corpus_available: bool,
    #[serde(rename = "corpusFixtureCount")]
    corpus_fixture_count: usize,
    #[serde(rename = "corpusFingerprint")]
    corpus_fingerprint: Option<String>,
    counts: Counts,
    entries: Vec<Entry>,
}

fn main() {
    let argv: Vec<String> = std::env::args().skip(1).collect();
    let args = parse_args(&argv);
    let repo_root = find_repo_root();
    let diff_text = fs::read_to_string(&args.diff_path)
        .unwrap_or_else(|e| fatal(&format!("diff read failed: {}: {e}", args.diff_path)));
    let diff: BuildDiff =
        serde_json::from_str(&diff_text).unwrap_or_else(|e| fatal(&format!("diff parse failed: {e}")));
    let executable = load_executable_base_ids(&repo_root);
    let sources = diff.sources();

    let mut corpus: Option<CorpusIndex> = None;
    if !args.corpus_dir.is_empty() {
        if !Path::new(&args.corpus_dir).exists() {
            eprintln!("corpus directory not found: {}", args.corpus_dir);
            std::process::exit(2);
        }
        // Cache metadata must not be discovered as a replay fixture or fingerprint itself.
        let index_path = if args.index_path.is_empty() {
            format!("{}/.corpus-index.cache", args.corpus_dir)
        } else {
            args.index_path.clone()
        };
        corpus = if args.refresh_index {
            None
        } else {
            load_corpus_index(&index_path, &args.corpus_dir)
        };
        match &corpus {
            None => {
                let index = build_corpus_index(&args.corpus_dir)
                    .unwrap_or_else(|e| fatal(&format!("corpus index build failed: {e}")));
                let serialized = serialize_corpus_index(&args.corpus_dir, &index);
                match serde_json::to_string_pretty(&serialized) {
                    Ok(text) => match fs::write(&index_path, format!("{text}\n")) {
                        Ok(()) => eprintln!("corpus index written: {index_path}"),
                        Err(e) => eprintln!("corpus index write skipped: {e}"),
                    },
                    Err(e) => eprintln!("corpus index write skipped: {e}"),
                }
                corpus = Some(index);
            }
            Some(_) => eprintln!("corpus index reused: {index_path}"),
        }
    }

    let mut entries: Vec<Entry> = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for table in ["cards", "buffs", "fateStrategies"] {
        let Some(Some(table_diff)) = diff.tables.get(table) else { continue };
        let push = |record: &DiffRecord, kind: &'static str, entries: &mut Vec<Entry>, seen: &mut HashSet<String>| {
            let Some(entry) = classify(table, record, kind, &executable, corpus.as_ref(), &sources) else { return };
            let dedupe_key = format!("{}/{}/{}", entry.table, js_num_string(entry.key), entry.kind);
            if seen.insert(dedupe_key) {
                entries.push(entry);
            }
        };
        for record in table_diff.added_records() {
            push(record, "added", &mut entries, &mut seen);
        }
        for record in table_diff.changed() {
            push(record, "changed", &mut entries, &mut seen);
        }
    }

    // 源码改了但配置表没动的卡不会出现在任何 table 里，而这类纯行为变更恰恰最危险：
    // 配置无差异，回放却可能静默改变。从 sources 反向补齐。
    for file in sources.added().iter().chain(sources.changed().iter()) {
        let Some(raw) = match_card_source_file(file) else { continue };
        let key = normalize_base_id(raw as f64);
        let Some(key_i) = canonical_id(key) else { continue };
        if seen.contains(&format!("cards/{key_i}/added")) || seen.contains(&format!("cards/{key_i}/changed")) {
            continue;
        }
        seen.insert(format!("cards/{key_i}/source-only"));
        let seasons = lookup_seasons(corpus.as_ref().map(|c| &c.card_hits), key);
        let corpus_hits = seasons.len();
        let reachable = corpus.is_some() && corpus_hits > 0;
        entries.push(Entry {
            table: "cards",
            key,
            name: None,
            kind: "changed",
            verdict: if reachable { "reachable" } else { "needs-review" },
            corpus_hits,
            seasons: seasons.clone(),
            source_touched: Some(file.clone()),
            reason: if reachable {
                format!("{file} 行为变更，配置表无差异，当前赛季语料出现（season {}）", join_seasons(&seasons))
            } else {
                format!("{file} 行为变更，但配置表无差异——纯逻辑改动需人工核对")
            },
            source_only_order: true,
        });
    }

    // 分桶存 entries 下标，避免借用与 JsonOut 搬移冲突。
    let mut buckets: BTreeMap<&'static str, Vec<usize>> = BTreeMap::new();
    for (idx, entry) in entries.iter().enumerate() {
        buckets.entry(entry.verdict).or_default().push(idx);
    }
    let bucket = |verdict: &str| -> &[usize] {
        buckets.get(verdict).map(Vec::as_slice).unwrap_or(&[])
    };


    if args.json {
        let out = JsonOut {
            old_build: BuildDiff::steam_build(&diff.old).to_string(),
            new_build: BuildDiff::steam_build(&diff.new).to_string(),
            corpus_available: corpus.is_some(),
            corpus_fixture_count: corpus.as_ref().map_or(0, |c| c.fixture_count),
            corpus_fingerprint: corpus.as_ref().map(|c| c.fingerprint.clone()),
            counts: Counts {
                implemented: bucket("implemented").len(),
                reachable: bucket("reachable").len(),
                needs_review: bucket("needs-review").len(),
                zero_coverage: bucket("zero-coverage").len(),
            },
            entries,
        };
        println!("{}", serde_json::to_string_pretty(&out).expect("entry json serialize"));
    } else {
        let header = format!(
            "build {} -> {}",
            BuildDiff::steam_build(&diff.old),
            BuildDiff::steam_build(&diff.new)
        );
        let corpus_note = match &corpus {
            Some(c) => format!("语料 {} 份（仅计当前赛季 {CURRENT_SEASON_NOTE}）", c.fixture_count),
            None => "无语料索引：可达性未判定，全部条目落入待审".to_string(),
        };
        println!("{header}\n{corpus_note}\n");

        const LABELS: [(&str, &str); 4] = [
            ("needs-review", "需人工判定"),
            ("reachable", "语料可达（应实现）"),
            ("zero-coverage", "零覆盖（fail-closed 挂起）"),
            ("implemented", "已实现（无需动作）"),
        ];
        for (verdict, label) in LABELS {
            let list: &[usize] = buckets.get(verdict).map(Vec::as_slice).unwrap_or_else(|| empty_ref());
            println!("## {label}: {}", list.len());
            // 已实现与零覆盖不需要逐条读，列出来只会淹没真正要看的条目。
            if verdict == "implemented" || verdict == "zero-coverage" {
                println!();
                continue;
            }
            for idx in list {
                let entry = &entries[*idx];
                let name = entry
                    .name
                    .as_deref()
                    .filter(|n| !n.is_empty())
                    .map(|n| format!(" {n}"))
                    .unwrap_or_default();
                println!(
                    "  {}/{}{} [{}] — {}",
                    entry.table,
                    js_num_string(entry.key),
                    name,
                    entry.kind,
                    entry.reason
                );
            }
            println!();
        }
    }

    let blocking = buckets.get("needs-review").map_or(0, Vec::len);
    std::process::exit(if blocking > 0 { 1 } else { 0 });
}

fn empty_ref() -> &'static [usize] {
    &[]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sha256_known_vectors() {
        assert_eq!(
            sha256::hex(b""),
            "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        );
        assert_eq!(
            sha256::hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
        assert_eq!(
            sha256::hex(b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"),
            "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        );
    }

    #[test]
    fn normalize_base_id_strips_rarity() {
        assert_eq!(normalize_base_id(0.0), 0.0);
        assert_eq!(normalize_base_id(10_000.0), 0.0);
        assert_eq!(normalize_base_id(20_000.0), 0.0);
        assert_eq!(normalize_base_id(401.0), 401.0);
        assert_eq!(normalize_base_id(10_401.0), 401.0);
        assert_eq!(normalize_base_id(204_011.0), 4011.0);
        assert_eq!(normalize_base_id(1_040_101.0), 1_000_101.0);
    }

    #[test]
    fn seasons_sort_like_js_string_sort() {
        // TS [...set].sort() 默认按字符串比较："10" < "9"。
        assert_eq!(sort_seasons_js(vec![9, 10, 0]), vec![0, 10, 9]);
        assert_eq!(sort_seasons_js(vec![10, 9]), vec![10, 9]);
    }

    #[test]
    fn js_num_string_matches_js() {
        assert_eq!(js_num_string(311.0), "311");
        assert_eq!(js_num_string(3.5), "3.5");
        assert_eq!(js_num_string(0.0), "0");
    }

    #[test]
    fn card_source_file_regex() {
        assert_eq!(match_card_source_file("Card_10401.cs"), Some(10_401));
        assert_eq!(match_card_source_file("Card_104011.cs"), Some(1_040_11));
        assert_eq!(match_card_source_file("Buff_5.cs"), None);
        assert_eq!(match_card_source_file("Card_x.cs"), None);
        assert_eq!(match_card_source_file("Card_5.json"), None);
    }
}
