//! 两层规范化（docs/showcase/01 §场景、配方与验证身份）。
//!
//! 身份计算分两层，不能把"排序 JSON 后 hash"当作合同：
//!
//! - **语义规范化（本 crate 自有规则，非 RFC 8785 内容）：**
//!   模型在类型层展开默认值后进入本层。浮点先映射到声明的 f32/f64 精度
//!   （[`FiniteF32`] / [`FiniteF64`]），再规范编码；拒绝 NaN/Inf；
//!   `-0.0` 归一为 `0.0`（需要区分位模式的测试必须使用显式整数/十六进制
//!   向量，不经普通 JSON 数值）。seed、tick 等 64 位整数经 [`CanonicalU64`]
//!   编码为规范十进制字符串，避免 JSON 数值消费方丢失精度。声明为集合的
//!   字段经 [`CanonicalSet`] 排序去重；有序列表（动作、视图）不排序。
//!   JSON 摄入先用 [`parse_strict`] 拒绝重复属性名，再做 map/schema 校验
//!   （serde_json 默认保留最后一个重复键，不满足合同）。
//! - **字节规范化（RFC 8785 JCS）：** 对已展开模型采用 JCS：属性名按
//!   UTF-16 码元排序（不是 Rust 字节序，见 [`jcs_bytes`]）、数字按
//!   ECMAScript Number→String 规则表示、字符串按 JSON 转义、输出 UTF-8；
//!   不追加未声明的 Unicode 归一化。

use std::collections::BTreeMap;
use std::fmt;

use serde::de::{self, MapAccess, SeqAccess, Visitor};
use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// 规范化编译器版本：语义展开或字节编码规则变更时必须递增，
/// 旧版本输入被拒（见 crate 级版本合同）。
pub const CANON_COMPILER_VERSION: &str = "scenario-model.canon/v1";

/// 规范化/摄入失败的统一错误。
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
pub enum CanonError {
    /// 非有限浮点（NaN/±Inf）进入语义层。
    #[error("non-finite float rejected by semantic layer")]
    NonFiniteFloat,
    /// JSON 摄入发现重复属性名（在任何 map/schema 校验之前拒绝）。
    #[error("duplicate JSON property name: {0:?}")]
    DuplicateProperty(String),
    /// 严格 JSON 解析失败（语法或重复键）。
    #[error("strict JSON parse failed: {0}")]
    StrictParse(String),
    /// 模型序列化失败。
    #[error("model serialization failed: {0}")]
    Serialize(String),
    /// 64 位整数的规范十进制字符串解析失败。
    #[error("invalid canonical u64 string: {0:?}")]
    InvalidCanonicalU64(String),
}

// ---------------------------------------------------------------------------
// 语义层：声明精度浮点
// ---------------------------------------------------------------------------

/// 声明为 f64 精度的有限浮点（语义层载体）。
///
/// 构造与反序列化均拒绝 NaN/±Inf，并把 `-0.0` 归一为 `0.0`。
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FiniteF64(f64);

impl FiniteF64 {
    /// 语义层入口：拒绝非有限值，归一 `-0.0`。
    pub fn try_new(value: f64) -> Result<Self, CanonError> {
        if !value.is_finite() {
            return Err(CanonError::NonFiniteFloat);
        }
        Ok(Self(normalize_zero(value)))
    }

    /// 取值（已保证有限且非负零）。
    #[must_use]
    pub fn get(self) -> f64 {
        self.0
    }
}

impl Serialize for FiniteF64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f64(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f64::deserialize(deserializer)?;
        Self::try_new(value).map_err(de::Error::custom)
    }
}

/// 声明为 f32 精度的有限浮点（语义层载体）。
///
/// 值先映射到 f32，再以 f32 的精确值进入 JSON 编码；其余规则同
/// [`FiniteF64`]。
#[derive(Clone, Copy, Debug, Default, PartialEq, PartialOrd)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct FiniteF32(f32);

impl FiniteF32 {
    /// 语义层入口：拒绝非有限值，归一 `-0.0`。
    pub fn try_new(value: f32) -> Result<Self, CanonError> {
        if !value.is_finite() {
            return Err(CanonError::NonFiniteFloat);
        }
        Ok(Self(if value == 0.0 { 0.0 } else { value }))
    }

    /// 取值（已保证有限且非负零）。
    #[must_use]
    pub fn get(self) -> f32 {
        self.0
    }
}

impl Serialize for FiniteF32 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_f32(self.0)
    }
}

impl<'de> Deserialize<'de> for FiniteF32 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let value = f32::deserialize(deserializer)?;
        Self::try_new(value).map_err(de::Error::custom)
    }
}

/// `-0.0` → `0.0`（语义层规则；`==` 判等对 ±0 均为真）。
fn normalize_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

// ---------------------------------------------------------------------------
// 语义层：64 位整数 / 集合字段
// ---------------------------------------------------------------------------

/// 64 位无符号整数（seed、tick 等）的规范载体：JSON 中编码为十进制字符串。
///
/// 反序列化只接受无多余前导零的规范十进制字符串；JSON 数值被拒，
/// 避免跨语言消费方按 f64 读数丢失精度。
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CanonicalU64(pub u64);

impl fmt::Display for CanonicalU64 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl Serialize for CanonicalU64 {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&self.0.to_string())
    }
}

impl<'de> Deserialize<'de> for CanonicalU64 {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let text = String::deserialize(deserializer)?;
        let value = text
            .parse::<u64>()
            .map_err(|_| de::Error::custom(CanonError::InvalidCanonicalU64(text.clone())))?;
        if value.to_string() != text {
            return Err(de::Error::custom(CanonError::InvalidCanonicalU64(text)));
        }
        Ok(Self(value))
    }
}

#[cfg(feature = "schema")]
impl schemars::JsonSchema for CanonicalU64 {
    fn schema_name() -> std::borrow::Cow<'static, str> {
        "CanonicalU64".into()
    }

    fn json_schema(_: &mut schemars::SchemaGenerator) -> schemars::Schema {
        schemars::json_schema!({ "type": "string", "pattern": "^(0|[1-9][0-9]*)$" })
    }
}

/// 声明为集合的字段（语义层规则：构造时排序去重）。
///
/// 有序列表（动作、视图、目录项）不得使用本类型；只有合同声明为集合的
/// 字段（效果集合、设备 feature 集合、必需观测集合等）才允许排序去重。
#[derive(Clone, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "schema", derive(schemars::JsonSchema))]
pub struct CanonicalSet<T: Ord>(Vec<T>);

impl<T: Ord> CanonicalSet<T> {
    /// 构造并按声明规则排序去重。
    pub fn new(mut items: Vec<T>) -> Self {
        items.sort();
        items.dedup();
        Self(items)
    }

    /// 有序去重后的元素视图。
    #[must_use]
    pub fn as_slice(&self) -> &[T] {
        &self.0
    }
}

impl<T: Ord + Serialize> Serialize for CanonicalSet<T> {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.0.serialize(serializer)
    }
}

impl<'de, T: Ord + Deserialize<'de>> Deserialize<'de> for CanonicalSet<T> {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        Ok(Self::new(Vec::<T>::deserialize(deserializer)?))
    }
}

// ---------------------------------------------------------------------------
// 摄入：重复属性名拒绝
// ---------------------------------------------------------------------------

/// 严格 JSON 摄入：在任何 map/schema 校验之前拒绝重复属性名。
///
/// serde_json 默认对重复键保留最后一个值；合同要求显式失败。
pub fn parse_strict(bytes: &[u8]) -> Result<serde_json::Value, CanonError> {
    serde_json::from_slice::<StrictValue>(bytes)
        .map(|StrictValue(value)| value)
        .map_err(|error| {
            let message = error.to_string();
            if let Some(rest) = message.strip_prefix(DUPLICATE_MARKER) {
                let name = rest
                    .split(" at line")
                    .next()
                    .unwrap_or(rest)
                    .trim_matches('"')
                    .to_string();
                CanonError::DuplicateProperty(name)
            } else {
                CanonError::StrictParse(message)
            }
        })
}

const DUPLICATE_MARKER: &str = "duplicate property name ";

/// 重复键检测用的 Value 包装：visit_map 时先查重再插入。
struct StrictValue(serde_json::Value);

impl<'de> Deserialize<'de> for StrictValue {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        deserializer.deserialize_any(StrictValueVisitor)
    }
}

struct StrictValueVisitor;

impl<'de> Visitor<'de> for StrictValueVisitor {
    type Value = StrictValue;

    fn expecting(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("any JSON value with unique object keys")
    }

    fn visit_bool<E>(self, value: bool) -> Result<StrictValue, E> {
        Ok(StrictValue(serde_json::Value::Bool(value)))
    }

    fn visit_i64<E>(self, value: i64) -> Result<StrictValue, E> {
        Ok(StrictValue(value.into()))
    }

    fn visit_u64<E>(self, value: u64) -> Result<StrictValue, E> {
        Ok(StrictValue(value.into()))
    }

    fn visit_f64<E>(self, value: f64) -> Result<StrictValue, E> {
        Ok(StrictValue(value.into()))
    }

    fn visit_str<E: de::Error>(self, value: &str) -> Result<StrictValue, E> {
        Ok(StrictValue(serde_json::Value::String(value.to_string())))
    }

    fn visit_string<E>(self, value: String) -> Result<StrictValue, E> {
        Ok(StrictValue(serde_json::Value::String(value)))
    }

    fn visit_none<E>(self) -> Result<StrictValue, E> {
        Ok(StrictValue(serde_json::Value::Null))
    }

    fn visit_unit<E>(self) -> Result<StrictValue, E> {
        Ok(StrictValue(serde_json::Value::Null))
    }

    fn visit_some<D: Deserializer<'de>>(self, deserializer: D) -> Result<StrictValue, D::Error> {
        StrictValue::deserialize(deserializer)
    }

    fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<StrictValue, A::Error> {
        let mut items = Vec::with_capacity(seq.size_hint().unwrap_or(0));
        while let Some(StrictValue(item)) = seq.next_element::<StrictValue>()? {
            items.push(item);
        }
        Ok(StrictValue(serde_json::Value::Array(items)))
    }

    fn visit_map<A: MapAccess<'de>>(self, mut access: A) -> Result<StrictValue, A::Error> {
        let mut map = serde_json::Map::with_capacity(access.size_hint().unwrap_or(0));
        while let Some(key) = access.next_key::<String>()? {
            if map.contains_key(&key) {
                return Err(de::Error::custom(format!("{DUPLICATE_MARKER}{key:?}")));
            }
            let StrictValue(value) = access.next_value::<StrictValue>()?;
            map.insert(key, value);
        }
        Ok(StrictValue(serde_json::Value::Object(map)))
    }
}

// ---------------------------------------------------------------------------
// 字节层：RFC 8785 JCS
// ---------------------------------------------------------------------------

/// 对已展开模型计算规范字节（语义层 → serde_json::Value → JCS 字节）。
pub fn canonical_bytes<T: Serialize + ?Sized>(model: &T) -> Result<Vec<u8>, CanonError> {
    let value =
        serde_json::to_value(model).map_err(|error| CanonError::Serialize(error.to_string()))?;
    jcs_bytes(&value)
}

/// RFC 8785 JCS 序列化：UTF-16 码元属性排序、ECMAScript 数字表示、UTF-8。
pub fn jcs_bytes(value: &serde_json::Value) -> Result<Vec<u8>, CanonError> {
    let mut out = Vec::new();
    write_value(value, &mut out)?;
    Ok(out)
}

fn write_value(value: &serde_json::Value, out: &mut Vec<u8>) -> Result<(), CanonError> {
    match value {
        serde_json::Value::Null => out.extend_from_slice(b"null"),
        serde_json::Value::Bool(false) => out.extend_from_slice(b"false"),
        serde_json::Value::Bool(true) => out.extend_from_slice(b"true"),
        serde_json::Value::Number(number) => write_number(number, out)?,
        serde_json::Value::String(text) => write_string(text, out),
        serde_json::Value::Array(items) => {
            out.push(b'[');
            for (index, item) in items.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_value(item, out)?;
            }
            out.push(b']');
        }
        serde_json::Value::Object(map) => {
            // RFC 8785 §3.2.3：属性名按 UTF-16 码元序排序，不是 UTF-8 字节序。
            let mut entries: Vec<(&String, &serde_json::Value)> = map.iter().collect();
            entries.sort_by(|a, b| cmp_utf16(a.0, b.0));
            out.push(b'{');
            for (index, (key, item)) in entries.iter().enumerate() {
                if index > 0 {
                    out.push(b',');
                }
                write_string(key, out);
                out.push(b':');
                write_value(item, out)?;
            }
            out.push(b'}');
        }
    }
    Ok(())
}

/// UTF-16 码元序比较（RFC 8785 §3.2.3；与 Rust `str` 的字节序不同，
/// 例如 U+10000 的 UTF-16 前导代理 0xD800 小于 U+FFFD）。
fn cmp_utf16(a: &str, b: &str) -> std::cmp::Ordering {
    a.encode_utf16().cmp(b.encode_utf16())
}

/// RFC 8785 §3.2.2.2：JSON 字符串转义。
///
/// 仅转义 `"`、`\` 与 U+0000–U+001F（其中 \b \f \n \r \t 用短形式，
/// 其余控制字符用 `\u00xx` 小写十六进制）；DEL 与非 ASCII 字符以原始
/// UTF-8 输出，不做 Unicode 归一化。
fn write_string(text: &str, out: &mut Vec<u8>) {
    out.push(b'"');
    for ch in text.chars() {
        match ch {
            '"' => out.extend_from_slice(b"\\\""),
            '\\' => out.extend_from_slice(b"\\\\"),
            '\u{08}' => out.extend_from_slice(b"\\b"),
            '\u{0C}' => out.extend_from_slice(b"\\f"),
            '\n' => out.extend_from_slice(b"\\n"),
            '\r' => out.extend_from_slice(b"\\r"),
            '\t' => out.extend_from_slice(b"\\t"),
            ch if (ch as u32) < 0x20 => {
                out.extend_from_slice(format!("\\u{:04x}", ch as u32).as_bytes());
            }
            ch => {
                let mut buf = [0u8; 4];
                out.extend_from_slice(ch.encode_utf8(&mut buf).as_bytes());
            }
        }
    }
    out.push(b'"');
}

/// RFC 8785 §3.2.2.3：数字按 ECMAScript Number→String 表示。
fn write_number(number: &serde_json::Number, out: &mut Vec<u8>) -> Result<(), CanonError> {
    if let Some(value) = number.as_u64() {
        out.extend_from_slice(value.to_string().as_bytes());
        return Ok(());
    }
    if let Some(value) = number.as_i64() {
        out.extend_from_slice(value.to_string().as_bytes());
        return Ok(());
    }
    let value = number.as_f64().ok_or(CanonError::Serialize(format!(
        "number {number} is neither integer nor f64"
    )))?;
    if !value.is_finite() {
        return Err(CanonError::NonFiniteFloat);
    }
    out.extend_from_slice(ecmascript_number_string(value).as_bytes());
    Ok(())
}

/// ECMAScript `Number.prototype.toString`（十进制最短回读）表示。
///
/// 规则（RFC 8785 §3.2.2.3 引用 ECMA-262）：
/// - `|n|` 在 `[1e-6, 1e21)` 内用十进制普通记法；
/// - 否则用指数记法：`d[.ddd]e±x`，指数为 `floor(log10(|n|))`，
///   正指数带 `+`，指数无前导零；
/// - `-0` 表示为 `0`。
///
/// 最短十进制数字串由 serde_json 的有限浮点格式化给出，本函数只负责把
/// 其记法转换为 ECMAScript 记法（指数符号、普通/指数分界），不重新生成
/// 数字位。
#[must_use]
pub fn ecmascript_number_string(value: f64) -> String {
    assert!(
        value.is_finite(),
        "ECMAScript notation requires finite input"
    );
    if value == 0.0 {
        return "0".to_string();
    }
    let negative = value.is_sign_negative();
    let rendered = serde_json::to_string(&value).expect("finite f64 always serializes");

    // 解析 serde_json 输出（普通或指数记法）为 digits × 10^exp10。
    let unsigned = rendered.strip_prefix('-').unwrap_or(&rendered);
    let (mantissa, exponent) = match unsigned.split_once(['e', 'E']) {
        Some((mantissa, exponent)) => (mantissa, exponent.parse::<i32>().expect("exponent")),
        None => (unsigned, 0),
    };
    let (int_part, frac_part) = match mantissa.split_once('.') {
        Some((int_part, frac_part)) => (int_part, frac_part),
        None => (mantissa, ""),
    };
    let mut digits: String = int_part.to_string() + frac_part;
    let mut exp10 = exponent - frac_part.len() as i32;
    // 去掉前导零（整数部分为 "0" 时出现）；值非零，digits 不会清空。
    while digits.starts_with('0') {
        digits.remove(0);
    }
    // 去掉尾部零并补偿指数，得到最短 digits。
    let trailing = digits.len() - digits.trim_end_matches('0').len();
    digits.truncate(digits.len() - trailing);
    exp10 += trailing as i32;
    debug_assert!(!digits.is_empty() && !digits.starts_with('0') && !digits.ends_with('0'));

    let k = digits.len() as i32;
    let n = k + exp10; // value = 0.digits × 10^n 的 ECMAScript 参数 n。
    let body = if k <= n && n <= 21 {
        let mut body = digits.clone();
        body.extend(std::iter::repeat_n('0', (n - k) as usize));
        body
    } else if 0 < n && n <= 21 {
        format!("{}.{}", &digits[..n as usize], &digits[n as usize..])
    } else if -6 < n && n <= 0 {
        let mut body = "0.".to_string();
        body.extend(std::iter::repeat_n('0', (-n) as usize));
        body.push_str(&digits);
        body
    } else {
        let mut body = digits[..1].to_string();
        if k > 1 {
            body.push('.');
            body.push_str(&digits[1..]);
        }
        body.push('e');
        let exponent = n - 1;
        if exponent >= 0 {
            body.push('+');
        }
        body.push_str(&exponent.to_string());
        body
    };
    if negative { format!("-{body}") } else { body }
}

/// 便捷：BTreeMap 参数表（声明为 map、非集合的参数载体）。
pub type ParameterMap = BTreeMap<String, serde_json::Value>;

#[cfg(test)]
mod tests {
    use super::*;

    fn jcs(value: serde_json::Value) -> String {
        String::from_utf8(jcs_bytes(&value).expect("jcs")).expect("utf-8")
    }

    fn jcs_of(json: &str) -> String {
        jcs(parse_strict(json.as_bytes()).expect("strict parse"))
    }

    // ---- RFC 8785 字节层：数字（ECMAScript Number→String 已知答案向量）----

    #[test]
    fn es_number_notation_boundaries() {
        // 1e21 用指数记法且正指数带 '+'；1e20 仍是普通记法（RFC 8785 §3.2.2.3）。
        assert_eq!(ecmascript_number_string(1e20), "100000000000000000000");
        assert_eq!(ecmascript_number_string(1e21), "1e+21");
        assert_eq!(ecmascript_number_string(1.5e21), "1.5e+21");
        // 1e-6 是普通记法下界；1e-7 用指数记法。
        assert_eq!(ecmascript_number_string(1e-6), "0.000001");
        assert_eq!(ecmascript_number_string(1e-7), "1e-7");
        assert_eq!(ecmascript_number_string(1.5e-7), "1.5e-7");
    }

    #[test]
    fn es_number_common_values() {
        assert_eq!(ecmascript_number_string(0.0), "0");
        assert_eq!(ecmascript_number_string(-0.0), "0");
        assert_eq!(ecmascript_number_string(0.1), "0.1");
        assert_eq!(ecmascript_number_string(5.0), "5");
        assert_eq!(ecmascript_number_string(-3.0), "-3");
        assert_eq!(ecmascript_number_string(100.0), "100");
        assert_eq!(ecmascript_number_string(1234.5), "1234.5");
        assert_eq!(ecmascript_number_string(0.25), "0.25");
        assert_eq!(
            ecmascript_number_string(9007199254740992.0),
            "9007199254740992"
        );
        assert_eq!(ecmascript_number_string(5e-324), "5e-324");
        assert_eq!(
            ecmascript_number_string(1.7976931348623157e308),
            "1.7976931348623157e+308"
        );
        assert_eq!(
            ecmascript_number_string(std::f64::consts::PI),
            "3.141592653589793"
        );
    }

    // ---- RFC 8785 字节层：UTF-16 码元属性排序 ----

    #[test]
    fn property_sorting_uses_utf16_code_units() {
        // U+10000 的 UTF-16 前导代理是 0xD800，小于 U+FFFD（0xFFFD）；
        // UTF-8 字节序下 F0.. > EF..，两种排序结果相反。
        let json = "{\"\u{FFFD}\":1,\"\u{10000}\":2}";
        assert_eq!(jcs_of(json), "{\"\u{10000}\":2,\"\u{FFFD}\":1}");
        // 带重音字符与普通 ASCII：两种序一致，用于防止排序整体失效。
        let json = "{\"é\":1,\"e\":2,\"€\":3,\"\u{1F600}\":4}";
        assert_eq!(jcs_of(json), "{\"e\":2,\"é\":1,\"€\":3,\"\u{1F600}\":4}");
    }

    // ---- RFC 8785 字节层：字符串转义与 UTF-8 ----

    #[test]
    fn string_escaping_minimal_and_raw_utf8() {
        let json = "{\"s\":\"\\\"\\\\\\b\\f\\n\\r\\t\\u0001\\u001f\"}";
        assert_eq!(
            jcs_of(json),
            "{\"s\":\"\\\"\\\\\\b\\f\\n\\r\\t\\u0001\\u001f\"}"
        );
        // DEL（0x7F）不转义；非 ASCII 以原始 UTF-8 输出。
        let json = "{\"s\":\"\u{7f}é€\u{1F600}\"}";
        assert_eq!(jcs_of(json), "{\"s\":\"\u{7f}é€\u{1F600}\"}");
    }

    // ---- 语义层：精度映射 / 非有限拒绝 / ±0 / 64 位整数 / 集合 ----

    #[test]
    fn f32_precision_mapping_is_exact() {
        // 0.1f32 的精确 f64 值进入规范编码（声明 f32 精度先映射）。
        let json = jcs(serde_json::json!({ "v": FiniteF32::try_new(0.1).unwrap() }));
        assert_eq!(json, "{\"v\":0.10000000149011612}");
        let json = jcs(serde_json::json!({ "v": FiniteF64::try_new(0.1).unwrap() }));
        assert_eq!(json, "{\"v\":0.1}");
    }

    #[test]
    fn non_finite_floats_are_rejected() {
        assert_eq!(
            FiniteF64::try_new(f64::NAN),
            Err(CanonError::NonFiniteFloat)
        );
        assert_eq!(
            FiniteF64::try_new(f64::INFINITY),
            Err(CanonError::NonFiniteFloat)
        );
        assert_eq!(
            FiniteF32::try_new(f32::NEG_INFINITY),
            Err(CanonError::NonFiniteFloat)
        );
        assert!(serde_json::from_str::<FiniteF32>("1e999").is_err());
    }

    #[test]
    fn negative_zero_normalizes_to_zero() {
        assert_eq!(
            FiniteF64::try_new(-0.0).unwrap().get().to_bits(),
            0.0f64.to_bits()
        );
        let json = jcs(serde_json::json!({ "v": FiniteF64::try_new(-0.0).unwrap() }));
        assert_eq!(json, "{\"v\":0}");
    }

    #[test]
    fn canonical_u64_is_decimal_string() {
        let json = jcs(serde_json::json!({ "seed": CanonicalU64(9007199254740993) }));
        assert_eq!(json, "{\"seed\":\"9007199254740993\"}");
        // 只接受规范十进制字符串：拒绝数值、前导零、空串。
        assert!(serde_json::from_str::<CanonicalU64>("42").is_err());
        assert!(serde_json::from_str::<CanonicalU64>("\"042\"").is_err());
        assert!(serde_json::from_str::<CanonicalU64>("\"\"").is_err());
        assert_eq!(
            serde_json::from_str::<CanonicalU64>("\"0\"").unwrap(),
            CanonicalU64(0)
        );
    }

    #[test]
    fn canonical_set_sorts_and_dedups() {
        let set = CanonicalSet::new(vec!["b".to_string(), "a".to_string(), "a".to_string()]);
        let json = jcs(serde_json::json!({ "effects": set }));
        assert_eq!(json, "{\"effects\":[\"a\",\"b\"]}");
    }

    // ---- 摄入：重复属性名 / 未知版本 ----

    #[test]
    fn duplicate_property_names_are_rejected() {
        assert_eq!(
            parse_strict(b"{\"a\":1,\"a\":2}"),
            Err(CanonError::DuplicateProperty("a".to_string()))
        );
        // 嵌套对象同样拒绝。
        assert_eq!(
            parse_strict(b"{\"o\":{\"x\":1,\"x\":2}}"),
            Err(CanonError::DuplicateProperty("x".to_string()))
        );
        // 不同键正常。
        assert!(parse_strict(b"{\"a\":1,\"b\":2}").is_ok());
    }

    #[test]
    fn jcs_rejects_non_finite_value_numbers() {
        // serde_json::Number 无法持有非有限值（from_f64 返回 None），
        // 因此 JCS 输入在类型上永远有限；语义层另在 FiniteF* 入口拒绝。
        assert!(serde_json::Number::from_f64(f64::NAN).is_none());
        assert!(serde_json::Number::from_f64(f64::INFINITY).is_none());
    }

    #[test]
    fn jcs_structural_vectors() {
        assert_eq!(jcs_of("{\"b\":2,\"a\":1}"), "{\"a\":1,\"b\":2}");
        assert_eq!(jcs_of("[3,1,2]"), "[3,1,2]");
        assert_eq!(
            jcs_of("{\"a\":[null,true,false,\"x\"]}"),
            "{\"a\":[null,true,false,\"x\"]}"
        );
        assert_eq!(jcs_of("{}"), "{}");
    }
}
