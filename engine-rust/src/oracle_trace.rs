//! TS 原版：`battle-evaluator/scripts/lib/original-oracle-trace-slice.ts`（本文件为其逐字移植）。
//!
//! 语义与 TS 逐字对齐：按 batchId/runId 从 append-only trace 切出
//! `originalOracleBatchStarted…Completed` 的最后一次完成区间，产出 key-sorted
//! canonical JSONL 与区间证据（seq 边界 + sha256），并做 forward/reverse 重叠校验。
//!
//! canonical JSONL 与 TS `stableJson`（`JSON.stringify(sortJson(record))`）逐字节对齐，
//! 对齐点（与 `JSON.stringify` 的差异全部在侧修正）：
//!
//! - **键序**：TS `sortJson` 按 UTF-16 code unit 序（`compareText`）递归排序，而非
//!   UTF-8 字节序——非 BMP 字符（代理对 0xD800-0xDFFF）在 UTF-16 序中排在所有
//!   ≥0xE000 的 BMP 字符之前。此外 ES own-property 枚举把 array-index 键（规范
//!   数字串、值 < 2^32−1，真实 trace 的 `buffs` 键即此类）提升到最前并按**数值升序**
//!   输出——`Object.fromEntries` 挡不住这一步。故最终顺序：array-index 键数值升序
//!   在前，其余键 UTF-16 序在后（见 `array_index_key`，已由真实 trace 对拍证实）。
//! - **数字**：serde_json 的 ryu 输出 `3.0`/`1e21`，JS 是 `3`/`1e+21`。整数 token
//!   （serde PosInt/NegInt）按整数打印；Float 分支走 `js_number_to_string`
//!   （ECMAScript Number::toString，含 `e+`/`e-` 指数与 -0 → `0`）。
//! - **字符串转义**：仅 `"` `\` 与 0x00-0x1F（`\b\t\n\f\r` 缩写，其余 `\u00xx`
//!   小写十六进制），非 ASCII 原样输出——与 JS 一致，serde 不参与字符串层。
//! - JSON.parse 语义差异（TS 允许孤儿代理转义、报错文案不同）在真实 trace 数据域外，
//!   无效行/非对象/seq 非安全整数的错误文案保持逐字一致，仅 parse 异常细节文本不逐字对齐。

use serde_json::Number as JsonNumber;

// ---------- canonical JSON（对齐 JSON.stringify(sortJson(v))） ----------

/// JSON 值树。Object 保持插入序，canonical 序列化时按 UTF-16 键序排序
/// （serde_json 反序列化已去重且后者覆盖前者，与 JS `JSON.parse` 一致）。
#[derive(Debug, Clone, PartialEq)]
pub enum JsonValue {
    Null,
    Bool(bool),
    Number(JsonNumber),
    String(String),
    Array(Vec<JsonValue>),
    Object(Vec<(String, JsonValue)>),
}

impl From<&serde_json::Value> for JsonValue {
    fn from(value: &serde_json::Value) -> Self {
        match value {
            serde_json::Value::Null => JsonValue::Null,
            serde_json::Value::Bool(b) => JsonValue::Bool(*b),
            serde_json::Value::Number(n) => JsonValue::Number(n.clone()),
            serde_json::Value::String(s) => JsonValue::String(s.clone()),
            serde_json::Value::Array(items) => {
                JsonValue::Array(items.iter().map(JsonValue::from).collect())
            }
            serde_json::Value::Object(map) => JsonValue::Object(
                map.iter()
                    .map(|(k, v)| (k.clone(), JsonValue::from(v)))
                    .collect(),
            ),
        }
    }
}

/// JS `Number.prototype.toString`（radix 10）：最短往返数字，|x| ≥ 1e21 或
/// < 1e-6 时用 `e+`/`e-` 指数形式；-0 → `"0"`；非有限值 JSON.stringify 永远
/// 输出 `null`（在 `write_canonical` 处理，这里只防御）。
fn js_number_to_string(x: f64) -> String {
    if x == 0.0 {
        return "0".to_string();
    }
    if !x.is_finite() {
        return if x.is_nan() {
            "NaN".to_string()
        } else if x > 0.0 {
            "Infinity".to_string()
        } else {
            "-Infinity".to_string()
        };
    }
    let negative = x < 0.0;
    let abs = x.abs();
    // Rust 的 LowerExp 与 Display 共用最短往返数字（Grisu/Ryū），借它取有效数字与十进制阶。
    let exp_form = format!("{:e}", abs);
    let (mantissa, exponent) = exp_form.split_once('e').expect("LowerExp always has 'e'");
    let exponent: i32 = exponent.parse().expect("LowerExp exponent is an integer");
    let digits: String = mantissa.chars().filter(|c| *c != '.').collect();
    // 最短表示末位不会是 0，防御性剥离不影响正确值。
    let digits = digits.trim_end_matches('0');
    let digits = if digits.is_empty() { "0" } else { digits };
    let k = digits.len() as i32;
    let n = exponent + 1; // value = 0.<digits> × 10^n
    let body = if k <= n && n <= 21 {
        let mut s = digits.to_string();
        for _ in 0..(n - k) {
            s.push('0');
        }
        s
    } else if n > 0 && n <= 21 {
        let point = n as usize;
        format!("{}.{}", &digits[..point], &digits[point..])
    } else if n > -6 && n <= 0 {
        let mut s = String::from("0.");
        for _ in 0..(-n) {
            s.push('0');
        }
        s.push_str(digits);
        s
    } else {
        let e = n - 1;
        let mut s = String::new();
        s.push_str(&digits[..1]);
        if k > 1 {
            s.push('.');
            s.push_str(&digits[1..]);
        }
        if e >= 0 {
            s.push_str(&format!("e+{e}"));
        } else {
            s.push_str(&format!("e-{}", -e));
        }
        s
    };
    if negative {
        format!("-{body}")
    } else {
        body
    }
}

fn canonical_number(n: &JsonNumber) -> String {
    if n.as_i64().is_some() || n.as_u64().is_some() {
        // serde_json 的 itoa 整数输出与 JS 整数字面量一致（含 u64/i64 全域）。
        n.to_string()
    } else {
        js_number_to_string(n.as_f64().unwrap_or(f64::NAN))
    }
}

/// JS `JSON.stringify` 的字符串转义：`"` `\` 与 0x00-0x1F（其中 `\b\t\n\f\r` 用
/// 两字符缩写，其余 `\u00xx` 小写十六进制）；0x7F 与非 ASCII 原样输出。
fn canonical_string(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{8}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

/// UTF-16 code unit 序（TS `compareText`）。非 BMP 字符（代理对 0xD800-0xDFFF）
/// 排在所有 ≥0xE000 的 BMP 字符之前，与 UTF-8 字节序不同。
fn utf16_key(key: &str) -> Vec<u16> {
    key.encode_utf16().collect()
}

/// JS array-index 键：规范十进制数字串且值 < 2^32−1（ES own-property 排序口径）。
/// `sortJson` 虽按 UTF-16 排序，但 `Object.fromEntries` + `JSON.stringify` 会把这些
/// 键重新提升到对象最前并按**数值升序**枚举（真实 trace 的 `buffs` 数字键即此路径），
/// 故 canonical 输出顺序为：array-index 键按数值升序在前，其余键按 UTF-16 序在后。
fn array_index_key(key: &str) -> Option<u64> {
    if key.is_empty() || key.len() > 10 || !key.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    if key.len() > 1 && key.starts_with('0') {
        return None; // "01" 等非规范数字串是普通字符串键
    }
    let value: u64 = key.parse().ok()?;
    if value <= 4_294_967_294 {
        Some(value)
    } else {
        None
    }
}

fn write_canonical(value: &JsonValue, out: &mut String) {
    match value {
        JsonValue::Null => out.push_str("null"),
        JsonValue::Bool(true) => out.push_str("true"),
        JsonValue::Bool(false) => out.push_str("false"),
        JsonValue::Number(n) => out.push_str(&canonical_number(n)),
        JsonValue::String(s) => out.push_str(&canonical_string(s)),
        JsonValue::Array(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_canonical(item, out);
            }
            out.push(']');
        }
        JsonValue::Object(entries) => {
            // 输出顺序：array-index 键按数值升序在前，其余键按 UTF-16 序在后（见 array_index_key）。
            let mut indexed: Vec<(&String, u64, &JsonValue)> = Vec::new();
            let mut named: Vec<(&String, &JsonValue)> = Vec::new();
            for (key, child) in entries {
                match array_index_key(key) {
                    Some(index) => indexed.push((key, index, child)),
                    None => named.push((key, child)),
                }
            }
            indexed.sort_by_key(|(_, index, _)| *index);
            named.sort_by(|left, right| utf16_key(left.0).cmp(&utf16_key(right.0)));
            out.push('{');
            let mut first = true;
            for (key, child) in indexed
                .iter()
                .map(|(k, _, v)| (*k, *v))
                .chain(named.iter().map(|(k, v)| (*k, *v)))
            {
                if !first {
                    out.push(',');
                }
                first = false;
                out.push_str(&canonical_string(key));
                out.push(':');
                write_canonical(child, out);
            }
            out.push('}');
        }
    }
}

/// TS `stableJson`：递归排序（array-index 键数值升序在前，其余 UTF-16 键序）后的
/// 紧凑 JSON 文本，与 `JSON.stringify(sortJson(v))` 逐字节一致。
pub fn stable_json(value: &JsonValue) -> String {
    let mut out = String::new();
    write_canonical(value, &mut out);
    out
}

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

/// TS `sha256Text`：UTF-8 文本的 sha256 十六进制摘要。
pub fn sha256_text(value: &str) -> String {
    sha256::hex(value.as_bytes())
}

// ---------- 切片 ----------

/// TS `OriginalOracleTraceSliceEvidence`。
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSliceEvidence {
    pub batch_id: String,
    pub run_id: String,
    pub start_seq: i64,
    pub end_seq: i64,
    pub record_count: usize,
    pub sha256: String,
}

/// TS `OriginalOracleTraceSlice`。
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSlice {
    pub evidence: TraceSliceEvidence,
    pub jsonl: String,
}

/// TS `OriginalOracleTraceSliceReport`（`format: "canonical-jsonl-seq-v1"`）。
#[derive(Debug, Clone, PartialEq)]
pub struct TraceSliceReport {
    pub combined_sha256: String,
    pub forward: TraceSliceEvidence,
    pub reverse: TraceSliceEvidence,
}

/// 错误文案与 TS `throw new Error(...)` 逐字一致（对拍比较 stderr 用）。
#[derive(Debug, thiserror::Error)]
#[error("{message}")]
pub struct TraceSliceError {
    pub message: String,
}

fn err<T>(message: impl Into<String>) -> Result<T, TraceSliceError> {
    Err(TraceSliceError { message: message.into() })
}

#[derive(Debug)]
struct ParsedTraceRecord {
    line: usize,
    seq: i64,
    record: JsonValue,
}

impl ParsedTraceRecord {
    fn get(&self, key: &str) -> Option<&JsonValue> {
        match &self.record {
            JsonValue::Object(entries) => {
                entries.iter().rev().find(|(k, _)| k == key).map(|(_, v)| v)
            }
            _ => None,
        }
    }

    fn string_field(&self, key: &str) -> Option<&str> {
        match self.get(key) {
            Some(JsonValue::String(s)) => Some(s.as_str()),
            _ => None,
        }
    }

    fn field_is(&self, key: &str, expected: &str) -> bool {
        self.string_field(key) == Some(expected)
    }

    fn kind_is(&self, expected: &str) -> bool {
        self.field_is("kind", expected)
    }

    fn kind_str(&self) -> &str {
        self.string_field("kind").unwrap_or("")
    }
}

/// TS `Number.isSafeInteger`：有限且整值、|x| ≤ 2^53−1。
fn is_safe_integer(n: &JsonNumber) -> bool {
    if let Some(i) = n.as_i64() {
        return i.abs() <= 9_007_199_254_740_991;
    }
    if n.as_u64().is_some() {
        return false; // > i64::MAX 必然超出 2^53−1
    }
    let f = n.as_f64().unwrap_or(f64::NAN);
    f.is_finite() && f.fract() == 0.0 && f.abs() <= 9_007_199_254_740_991.0
}

fn parse_trace_records(jsonl: &str) -> Result<Vec<ParsedTraceRecord>, TraceSliceError> {
    // TS `split(/\r?\n/)`：按 "\n" 或 "\r\n" 切；孤 "\r" 不切。
    let mut records = Vec::new();
    for (index, raw_line) in jsonl.split('\n').enumerate() {
        let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
        if line.trim().is_empty() {
            continue;
        }
        let parsed: serde_json::Value = match serde_json::from_str(line) {
            Ok(value) => value,
            // TS 错误文案含 JSON.parse 原始 message，两者实现不同、文本不逐字对齐。
            Err(error) => return err(format!("invalid trace JSONL line {}: {error}", index + 1)),
        };
        if !parsed.is_object() {
            return err(format!(
                "invalid trace JSONL line {}: record is not an object",
                index + 1
            ));
        }
        let Some(seq_number) = parsed.get("seq").and_then(|v| v.as_number()) else {
            return err(format!(
                "invalid trace JSONL line {}: seq is not a safe integer",
                index + 1
            ));
        };
        if !is_safe_integer(&seq_number) {
            return err(format!(
                "invalid trace JSONL line {}: seq is not a safe integer",
                index + 1
            ));
        }
        // 安全整数已验证，i64 无损。
        let seq = seq_number.as_i64().unwrap_or_else(|| seq_number.as_f64().unwrap() as i64);
        records.push(ParsedTraceRecord {
            line: index + 1,
            seq,
            record: JsonValue::from(&parsed),
        });
    }
    Ok(records)
}

fn find_last_index(
    records: &[ParsedTraceRecord],
    predicate: impl Fn(&ParsedTraceRecord, usize) -> bool,
) -> Option<usize> {
    for index in (0..records.len()).rev() {
        if predicate(&records[index], index) {
            return Some(index);
        }
    }
    None
}

fn assert_single_oracle_run_interval(
    selected: &[&ParsedTraceRecord],
    batch_id: &str,
    run_id: &str,
) -> Result<(), TraceSliceError> {
    for (index, entry) in selected.iter().enumerate() {
        let kind = entry.kind_str();
        if (kind == "originalOracleBatchStarted" && index != 0)
            || (kind == "originalOracleBatchCompleted" && index != selected.len() - 1)
        {
            return err(format!(
                "trace interval {batch_id}/{run_id} contains a nested original-oracle batch marker"
            ));
        }
        if kind.starts_with("originalOracleCase")
            && (!entry.field_is("batchId", batch_id) || !entry.field_is("runId", run_id))
        {
            return err(format!(
                "trace interval {batch_id}/{run_id} contains a foreign original-oracle case marker"
            ));
        }
    }
    Ok(())
}

/// 从 append-only trace 切出最后一次完成的 exact batch/run 区间。
/// Key-sorted JSONL 使区间外的后续 append 不影响已 admit 的 hash。
pub fn slice_original_oracle_run_trace(
    jsonl: &str,
    batch_id: &str,
    run_id: &str,
) -> Result<TraceSlice, TraceSliceError> {
    if batch_id.trim().is_empty() {
        return err("original-oracle trace slice batchId is empty");
    }
    if run_id.trim().is_empty() {
        return err("original-oracle trace slice runId is empty");
    }
    let records = parse_trace_records(jsonl)?;
    let end_index = find_last_index(&records, |entry, _| {
        entry.kind_is("originalOracleBatchCompleted")
            && entry.field_is("batchId", batch_id)
            && entry.field_is("runId", run_id)
    });
    let Some(end_index) = end_index else {
        return err(format!(
            "trace has no completed original-oracle batch/run {batch_id}/{run_id}"
        ));
    };
    let start_index = find_last_index(&records, |entry, index| {
        index <= end_index
            && entry.kind_is("originalOracleBatchStarted")
            && entry.field_is("batchId", batch_id)
            && entry.field_is("runId", run_id)
    });
    let Some(start_index) = start_index else {
        return err(format!(
            "trace has no start for completed original-oracle batch/run {batch_id}/{run_id}"
        ));
    };
    if start_index >= end_index {
        return err(format!("invalid original-oracle trace interval {batch_id}/{run_id}"));
    }

    let selected: Vec<&ParsedTraceRecord> = records[start_index..=end_index].iter().collect();
    for index in 1..selected.len() {
        if selected[index].seq <= selected[index - 1].seq {
            return err(format!(
                "trace sequence is not strictly increasing at line {}",
                selected[index].line
            ));
        }
    }
    assert_single_oracle_run_interval(&selected, batch_id, run_id)?;
    let mut canonical_jsonl = String::new();
    for entry in &selected {
        canonical_jsonl.push_str(&stable_json(&entry.record));
        canonical_jsonl.push('\n');
    }
    Ok(TraceSlice {
        evidence: TraceSliceEvidence {
            batch_id: batch_id.to_string(),
            run_id: run_id.to_string(),
            start_seq: selected[0].seq,
            end_seq: selected[selected.len() - 1].seq,
            record_count: selected.len(),
            sha256: sha256_text(&canonical_jsonl),
        },
        jsonl: canonical_jsonl,
    })
}

fn ranges_overlap(left: &TraceSliceEvidence, right: &TraceSliceEvidence) -> bool {
    left.start_seq <= right.end_seq && right.start_seq <= left.end_seq
}

impl TraceSliceEvidence {
    /// camelCase JSON 形状（键序由 canonical 序列化决定，与 TS 对象形状一致）。
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("batchId".into(), JsonValue::String(self.batch_id.clone())),
            ("runId".into(), JsonValue::String(self.run_id.clone())),
            ("startSeq".into(), JsonValue::Number(self.start_seq.into())),
            ("endSeq".into(), JsonValue::Number(self.end_seq.into())),
            (
                "recordCount".into(),
                JsonValue::Number((self.record_count as u64).into()),
            ),
            ("sha256".into(), JsonValue::String(self.sha256.clone())),
        ])
    }
}

impl TraceSlice {
    /// `{ evidence, jsonl }`，与 TS 内存对象同形。
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            ("evidence".into(), self.evidence.to_json()),
            ("jsonl".into(), JsonValue::String(self.jsonl.clone())),
        ])
    }
}

impl TraceSliceReport {
    /// forward/reverse 区间重叠即 fail closed；`combinedSha256` 为
    /// `sha256(stableJson({ forward, reverse }))`。
    pub fn to_json(&self) -> JsonValue {
        JsonValue::Object(vec![
            (
                "format".into(),
                JsonValue::String("canonical-jsonl-seq-v1".into()),
            ),
            (
                "combinedSha256".into(),
                JsonValue::String(self.combined_sha256.clone()),
            ),
            ("forward".into(), self.forward.to_json()),
            ("reverse".into(), self.reverse.to_json()),
        ])
    }
}

/// TS `buildOriginalOracleTraceSliceReport`。
pub fn build_original_oracle_trace_slice_report(
    forward: &TraceSliceEvidence,
    reverse: &TraceSliceEvidence,
) -> Result<TraceSliceReport, TraceSliceError> {
    if ranges_overlap(forward, reverse) {
        return err("forward/reverse original-oracle trace slices overlap");
    }
    let combined = JsonValue::Object(vec![
        ("forward".into(), forward.to_json()),
        ("reverse".into(), reverse.to_json()),
    ]);
    Ok(TraceSliceReport {
        combined_sha256: sha256_text(&stable_json(&combined)),
        forward: forward.clone(),
        reverse: reverse.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    // ---------- TS synthetic-oracle-trace-slice.test.ts 的 fixture 构造器 ----------

    fn record(seq: i64, kind: &str) -> serde_json::Value {
        json!({"seq": seq, "kind": kind, "utc": format!("t{seq}")})
    }
    fn started(seq: i64, batch_id: &str, run_id: &str) -> serde_json::Value {
        let mut v = record(seq, "originalOracleBatchStarted");
        v["batchId"] = json!(batch_id);
        v["runId"] = json!(run_id);
        v
    }
    fn completed(seq: i64, batch_id: &str, run_id: &str) -> serde_json::Value {
        let mut v = record(seq, "originalOracleBatchCompleted");
        v["batchId"] = json!(batch_id);
        v["runId"] = json!(run_id);
        v["status"] = json!("completed");
        v
    }
    fn case_started(seq: i64, batch_id: &str, run_id: &str, case_id: &str) -> serde_json::Value {
        let mut v = record(seq, "originalOracleCaseStarted");
        v["batchId"] = json!(batch_id);
        v["runId"] = json!(run_id);
        v["caseId"] = json!(case_id);
        v["inputSha256"] = json!("a".repeat(64));
        v
    }
    fn case_completed(seq: i64, batch_id: &str, run_id: &str, case_id: &str) -> serde_json::Value {
        let mut v = record(seq, "originalOracleCaseCompleted");
        v["batchId"] = json!(batch_id);
        v["runId"] = json!(run_id);
        v["caseId"] = json!(case_id);
        v["winnerSide"] = json!("p1");
        v["actorTurn"] = json!(1);
        v["hpDelta"] = json!(1);
        v["p1Hp"] = json!(1);
        v["p2Hp"] = json!(0);
        v["remainingDecisionParams"] = json!(0);
        v
    }
    fn card(seq: i64, oracle_case_id: &str, actor_turn: i64) -> serde_json::Value {
        let mut v = record(seq, "cardCompleted");
        v["oracleCaseId"] = json!(oracle_case_id);
        v["actorTurn"] = json!(actor_turn);
        v["card"] = json!({"id": 0});
        v["src"] = json!({"side": "p1", "hp": 1});
        v["dst"] = json!({"side": "p2", "hp": 0});
        v
    }
    fn trace(records: &[serde_json::Value]) -> String {
        let lines: Vec<String> = records
            .iter()
            .map(|r| serde_json::to_string(r).unwrap())
            .collect();
        format!("{}\n", lines.join("\n"))
    }
    fn slice(jsonl: &str, batch: &str, run: &str) -> TraceSlice {
        slice_original_oracle_run_trace(jsonl, batch, run).unwrap()
    }

    // ---------- TS test 1：append 后 admitted run hash 不变 ----------

    #[test]
    fn keeps_admitted_run_hashes_stable_when_trace_is_appended() {
        let base = trace(&[
            record(1, "pluginLoaded"),
            started(2, "batch", "forward-1"),
            case_started(3, "batch", "forward-1", "case-a"),
            card(4, "case-a", 1),
            case_completed(5, "batch", "forward-1", "case-a"),
            completed(6, "batch", "forward-1"),
            started(7, "batch", "reverse-1"),
            case_started(8, "batch", "reverse-1", "case-a"),
            card(9, "case-a", 1),
            case_completed(10, "batch", "reverse-1", "case-a"),
            completed(11, "batch", "reverse-1"),
        ]);
        let appended = format!(
            "{base}{}",
            trace(&[record(12, "unrelated"), record(13, "later")])
        );

        let report_for = |text: &str| {
            let forward = slice(text, "batch", "forward-1");
            let reverse = slice(text, "batch", "reverse-1");
            (
                forward.clone(),
                reverse.clone(),
                build_original_oracle_trace_slice_report(&forward.evidence, &reverse.evidence)
                    .unwrap(),
            )
        };
        let (fwd_before, rev_before, report_before) = report_for(&base);
        let (fwd_after, rev_after, report_after) = report_for(&appended);

        assert_eq!(report_after, report_before);
        assert_eq!(fwd_after.jsonl, fwd_before.jsonl);
        assert_eq!(rev_after.jsonl, rev_before.jsonl);
        assert_eq!(fwd_before.evidence.start_seq, 2);
        assert_eq!(fwd_before.evidence.end_seq, 6);
        assert_eq!(fwd_before.evidence.record_count, 5);
        assert_eq!(rev_before.evidence.start_seq, 7);
        assert_eq!(rev_before.evidence.end_seq, 11);
        assert_eq!(rev_before.evidence.record_count, 5);

        // canonical JSONL 逐字节固定（key-sorted、无空格、换行结尾）。
        assert_eq!(
            fwd_before.jsonl,
            concat!(
                "{\"batchId\":\"batch\",\"kind\":\"originalOracleBatchStarted\",\"runId\":\"forward-1\",\"seq\":2,\"utc\":\"t2\"}\n",
                "{\"batchId\":\"batch\",\"caseId\":\"case-a\",\"inputSha256\":\"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa\",\"kind\":\"originalOracleCaseStarted\",\"runId\":\"forward-1\",\"seq\":3,\"utc\":\"t3\"}\n",
                "{\"actorTurn\":1,\"card\":{\"id\":0},\"dst\":{\"hp\":0,\"side\":\"p2\"},\"kind\":\"cardCompleted\",\"oracleCaseId\":\"case-a\",\"seq\":4,\"src\":{\"hp\":1,\"side\":\"p1\"},\"utc\":\"t4\"}\n",
                "{\"actorTurn\":1,\"batchId\":\"batch\",\"caseId\":\"case-a\",\"hpDelta\":1,\"kind\":\"originalOracleCaseCompleted\",\"p1Hp\":1,\"p2Hp\":0,\"remainingDecisionParams\":0,\"runId\":\"forward-1\",\"seq\":5,\"utc\":\"t5\",\"winnerSide\":\"p1\"}\n",
                "{\"batchId\":\"batch\",\"kind\":\"originalOracleBatchCompleted\",\"runId\":\"forward-1\",\"seq\":6,\"status\":\"completed\",\"utc\":\"t6\"}\n",
            )
        );
    }

    // ---------- TS test 2：选最后一次完成的确切 run ----------

    #[test]
    fn selects_latest_completed_exact_run_without_older_retry() {
        let jsonl = trace(&[
            started(1, "batch", "forward-1"),
            case_started(2, "batch", "forward-1", "old"),
            case_completed(3, "batch", "forward-1", "old"),
            completed(4, "batch", "forward-1"),
            record(5, "gap"),
            started(6, "batch", "forward-1"),
            case_started(7, "batch", "forward-1", "new"),
            case_completed(8, "batch", "forward-1", "new"),
            completed(9, "batch", "forward-1"),
        ]);
        let selected = slice(&jsonl, "batch", "forward-1");
        assert_eq!(selected.evidence.start_seq, 6);
        assert_eq!(selected.evidence.end_seq, 9);
        assert_eq!(selected.evidence.record_count, 4);
        assert!(selected.jsonl.contains("\"caseId\":\"new\""));
        assert!(!selected.jsonl.contains("\"caseId\":\"old\""));
    }

    // ---------- TS test 3：篡改改 hash、边界 fail closed ----------

    #[test]
    fn changes_slice_hash_on_tampering_and_fails_closed_on_bad_bounds() {
        let jsonl = trace(&[
            started(1, "batch", "forward-1"),
            case_started(2, "batch", "forward-1", "case-a"),
            case_completed(3, "batch", "forward-1", "case-a"),
            completed(4, "batch", "forward-1"),
        ]);
        let tampered = jsonl.replace("\"case-a\"", "\"case-b\"");
        assert_ne!(
            slice(&tampered, "batch", "forward-1").evidence.sha256,
            slice(&jsonl, "batch", "forward-1").evidence.sha256
        );
        let err = slice_original_oracle_run_trace(
            &trace(&[completed(1, "batch", "forward-1")]),
            "batch",
            "forward-1",
        )
        .unwrap_err();
        assert!(err.message.contains("trace has no start"), "{}", err.message);
        let err = slice_original_oracle_run_trace(
            &trace(&[
                started(2, "batch", "forward-1"),
                completed(1, "batch", "forward-1"),
            ]),
            "batch",
            "forward-1",
        )
        .unwrap_err();
        assert!(
            err.message.contains("trace sequence is not strictly increasing"),
            "{}",
            err.message
        );
    }

    // ---------- TS test 4：嵌套/外来的 oracle run 拒绝、无 batchId 的 card 行保留 ----------

    #[test]
    fn rejects_nested_or_foreign_oracle_runs_keeping_unscoped_card_rows() {
        let nested = trace(&[
            started(1, "batch", "forward-1"),
            case_started(2, "batch", "forward-1", "case-a"),
            started(3, "other", "parallel-1"),
            completed(4, "other", "parallel-1"),
            case_completed(5, "batch", "forward-1", "case-a"),
            completed(6, "batch", "forward-1"),
        ]);
        let err = slice_original_oracle_run_trace(&nested, "batch", "forward-1").unwrap_err();
        assert!(
            err.message.contains("nested original-oracle batch marker"),
            "{}",
            err.message
        );

        let foreign_case = trace(&[
            started(1, "batch", "forward-1"),
            case_started(2, "other", "parallel-1", "case-b"),
            completed(3, "batch", "forward-1"),
        ]);
        let err = slice_original_oracle_run_trace(&foreign_case, "batch", "forward-1").unwrap_err();
        assert!(
            err.message.contains("foreign original-oracle case marker"),
            "{}",
            err.message
        );

        let unscoped_card = trace(&[
            started(1, "batch", "forward-1"),
            case_started(2, "batch", "forward-1", "case-a"),
            card(3, "case-a", 1),
            case_completed(4, "batch", "forward-1", "case-a"),
            completed(5, "batch", "forward-1"),
        ]);
        assert_eq!(
            slice(&unscoped_card, "batch", "forward-1").evidence.record_count,
            5
        );
    }

    // ---------- TS verify-synthetic-oracle-trace.test.ts 的 fixture 期望 ----------

    #[test]
    fn reextracts_canonical_slices_for_exact_schema_4_and_5_fixture() {
        let trace_text = trace(&[
            record(1, "pluginLoaded"),
            started(2, "batch", "forward-1"),
            case_started(3, "batch", "forward-1", "case-a"),
            case_completed(4, "batch", "forward-1", "case-a"),
            completed(5, "batch", "forward-1"),
            started(6, "batch", "reverse-1"),
            case_started(7, "batch", "reverse-1", "case-a"),
            case_completed(8, "batch", "reverse-1", "case-a"),
            completed(9, "batch", "reverse-1"),
            record(10, "laterAppend"),
        ]);
        let forward = slice(&trace_text, "batch", "forward-1");
        let reverse = slice(&trace_text, "batch", "reverse-1");
        assert_eq!(forward.evidence.run_id, "forward-1");
        assert_eq!(forward.evidence.start_seq, 2);
        assert_eq!(forward.evidence.end_seq, 5);
        assert_eq!(forward.evidence.record_count, 4);
        assert_eq!(reverse.evidence.start_seq, 6);
        assert_eq!(reverse.evidence.end_seq, 9);
        assert_eq!(reverse.evidence.record_count, 4);
        let report =
            build_original_oracle_trace_slice_report(&forward.evidence, &reverse.evidence).unwrap();
        // traceSha256 即 combinedSha256（schema 4/5 的报告形状）。
        assert_eq!(report.combined_sha256, report.combined_sha256);
        let report_json = report.to_json();
        assert!(stable_json(&report_json).contains("\"format\":\"canonical-jsonl-seq-v1\""));
    }

    // ---------- canonical JSON 与 JSON.stringify 的字节对齐 ----------

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
    }

    #[test]
    fn canonical_numbers_match_js_json_stringify() {
        let f = |v: serde_json::Value| stable_json(&JsonValue::from(&v));
        assert_eq!(f(json!(3)), "3");
        assert_eq!(f(json!(3.0)), "3"); // JSON.parse("3.0") → 3 → "3"
        assert_eq!(f(json!(0.1)), "0.1");
        assert_eq!(f(json!(-2.5)), "-2.5");
        assert_eq!(f(json!(0)), "0");
        assert_eq!(f(serde_json::from_str::<serde_json::Value>("-0").unwrap()), "0");
        assert_eq!(f(json!(1e21)), "1e+21");
        assert_eq!(f(json!(1e20)), "100000000000000000000");
        assert_eq!(f(json!(1e-6)), "0.000001");
        assert_eq!(f(json!(1e-7)), "1e-7");
        assert_eq!(f(json!(123456789012345678.0)), "123456789012345680");
        assert_eq!(f(json!(1.5e-8)), "1.5e-8");
        assert_eq!(f(json!(14792.38)), "14792.38");
        // serde 整数全域（JS 侧 parse 后同为 double，stringify 输出相同十进制）。
        assert_eq!(f(json!(u64::MAX)), "18446744073709551615");
        assert_eq!(f(json!(i64::MIN)), "-9223372036854775808");
    }

    #[test]
    fn canonical_strings_and_key_order_match_js_json_stringify() {
        let f = |v: serde_json::Value| stable_json(&JsonValue::from(&v));
        assert_eq!(f(json!("a\"b\\c")), "\"a\\\"b\\\\c\"");
        assert_eq!(f(json!("\u{1}\u{8}\t\n\u{c}\r\u{1f}")), "\"\\u0001\\b\\t\\n\\f\\r\\u001f\"");
        assert_eq!(f(json!("中文/emoji😀")), "\"中文/emoji😀\""); // 非 ASCII 原样
        // 键序：UTF-16 code unit 序；非 BMP 键排在 ≥0xE000 BMP 键之前（与 UTF-8 字节序不同）。
        assert_eq!(
            f(json!({"b": 1, "A": 2, "a": 3})),
            "{\"A\":2,\"a\":3,\"b\":1}"
        );
        let non_bmp_first = f(serde_json::from_str::<serde_json::Value>(
            "{\"\\uE000\":1,\"\\uD83D\\uDE00\":2}",
        )
        .unwrap());
        assert_eq!(non_bmp_first, "{\"😀\":2,\"\u{e000}\":1}");
        // array-index 键：数值升序提升到最前，与 UTF-16 序无关（ES own-property 枚举）。
        assert_eq!(
            f(json!({"10024": 1, "298": 2, "6": 3, "660": 4, "b": 5})),
            "{\"6\":3,\"298\":2,\"660\":4,\"10024\":1,\"b\":5}"
        );
        // 越界（2^32-1）与带前导零的数字串是普通字符串键，走 UTF-16 序；"2" 仍是 index 键。
        assert_eq!(
            f(json!({"4294967295": 1, "2": 2, "01": 3})),
            "{\"2\":2,\"01\":3,\"4294967295\":1}"
        );
        // 2^32-2 是最后一个 index 键。
        assert_eq!(
            f(json!({"4294967295": 1, "4294967294": 2})),
            "{\"4294967294\":2,\"4294967295\":1}"
        );
        // 重复键：后者覆盖（JSON.parse 语义）。
        assert_eq!(
            f(serde_json::from_str::<serde_json::Value>("{\"a\":1,\"a\":2}").unwrap()),
            "{\"a\":2}"
        );
        // 嵌套数组与 null/bool。
        assert_eq!(f(json!([1, null, true, {"z": [], "y": {}}])), "[1,null,true,{\"y\":{},\"z\":[]}]");
    }

    #[test]
    fn seq_safety_and_empty_ids_match_ts() {
        let line = format!(
            "{{\"seq\":{},\"kind\":\"x\"}}\n",
            9_007_199_254_740_991i64
        );
        assert!(parse_trace_records(&line).is_ok());
        let too_big = "{\"seq\":9007199254740993,\"kind\":\"x\"}\n";
        let err = parse_trace_records(too_big).unwrap_err();
        assert_eq!(
            err.message,
            "invalid trace JSONL line 1: seq is not a safe integer"
        );
        let err = parse_trace_records("{\"seq\":1.5,\"kind\":\"x\"}\n").unwrap_err();
        assert_eq!(
            err.message,
            "invalid trace JSONL line 1: seq is not a safe integer"
        );
        let err = parse_trace_records("[1,2]\n").unwrap_err();
        assert_eq!(err.message, "invalid trace JSONL line 1: record is not an object");
        // /\r?\n/ 切分、空白行跳过、行号按切分序号。
        let ok = parse_trace_records("\r\n{\"seq\":1}\r\n   \n{\"seq\":2}\n\n").unwrap();
        assert_eq!(ok.len(), 2);
        assert_eq!(ok[0].line, 2);
        assert_eq!(ok[1].line, 4);
        let err = slice_original_oracle_run_trace("", " ", "run").unwrap_err();
        assert_eq!(err.message, "original-oracle trace slice batchId is empty");
        let err = slice_original_oracle_run_trace("", "batch", " ").unwrap_err();
        assert_eq!(err.message, "original-oracle trace slice runId is empty");
    }

    #[test]
    fn report_rejects_overlapping_ranges_and_combined_hash_is_stable() {
        let evidence = |run: &str, start: i64, end: i64| TraceSliceEvidence {
            batch_id: "batch".into(),
            run_id: run.into(),
            start_seq: start,
            end_seq: end,
            record_count: 4,
            sha256: "a".repeat(64),
        };
        let err = build_original_oracle_trace_slice_report(
            &evidence("f", 2, 9),
            &evidence("r", 6, 11),
        )
        .unwrap_err();
        assert_eq!(err.message, "forward/reverse original-oracle trace slices overlap");
        let report = build_original_oracle_trace_slice_report(
            &evidence("f", 2, 5),
            &evidence("r", 6, 9),
        )
        .unwrap();
        // combinedSha256 = sha256(stableJson({forward, reverse}))，键序 UTF-16。
        let combined = JsonValue::Object(vec![
            ("forward".into(), evidence("f", 2, 5).to_json()),
            ("reverse".into(), evidence("r", 6, 9).to_json()),
        ]);
        assert_eq!(report.combined_sha256, sha256_text(&stable_json(&combined)));
    }
}
