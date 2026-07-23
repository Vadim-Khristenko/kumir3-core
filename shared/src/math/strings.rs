// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! String and aggregate helpers for arithmetic operations (string multiplication/division, substring removal, value keys).

// =============================================================================
//         SECTION: IMPORTS
// =============================================================================

use super::MathErr;
use super::MathOperators;
use crate::types::{Number, Value};

// =============================================================================
//         SECTION: CORE LOGIC
// =============================================================================

impl MathOperators {
    /// Removes all occurrences of a substring from a string using KMP algorithm.
    ///
    /// # Arguments
    /// * `s` - The input string
    /// * `pat` - The substring pattern to remove
    ///
    /// # Returns
    /// * `String` - The string with all occurrences of the pattern removed
    pub(super) fn remove_all_substring_bytes(s: String, pat: &str) -> String {
        let s_bytes = s.into_bytes();
        let p = pat.as_bytes();
        let m = p.len();
        if m == 0 {
            return String::from_utf8(s_bytes).unwrap_or_default();
        }
        let n = s_bytes.len();
        let mut lps = vec![0usize; m];
        {
            let mut len = 0usize;
            let mut i = 1usize;
            while i < m {
                if p[i] == p[len] {
                    len += 1;
                    lps[i] = len;
                    i += 1;
                } else if len != 0 {
                    len = lps[len - 1];
                } else {
                    lps[i] = 0;
                    i += 1;
                }
            }
        }

        let mut out: Vec<u8> = Vec::with_capacity(n);
        let mut history: Vec<usize> = Vec::with_capacity(n);
        for &b in s_bytes.iter() {
            let mut j = *history.last().unwrap_or(&0);
            while j > 0 && p[j] != b {
                j = lps[j - 1];
            }
            if p[j] == b {
                j += 1;
            }
            out.push(b);
            history.push(j);
            if j == m {
                for _ in 0..m {
                    out.pop();
                }
                history.truncate(history.len() - m);
            }
        }
        String::from_utf8(out).unwrap_or_default()
    }

    /// Generates a string key representation for a Value for hashing purposes.
    ///
    /// # Arguments
    /// * `v` - The Value to generate key for
    ///
    /// # Returns
    /// * `String` - String representation of the Value
    pub(super) fn value_key(v: &Value) -> String {
        match v {
            Value::Number(n) => {
                use self::Number::*;
                match n {
                    I8(x) => format!("I8:{}", x),
                    I16(x) => format!("I16:{}", x),
                    I32(x) => format!("I32:{}", x),
                    I64(x) => format!("I64:{}", x),
                    I128(x) => format!("I128:{}", x),
                    U8(x) => format!("U8:{}", x),
                    U16(x) => format!("U16:{}", x),
                    U32(x) => format!("U32:{}", x),
                    U64(x) => format!("U64:{}", x),
                    U128(x) => format!("U128:{}", x),
                    F32(x) => format!("F32:{}", x),
                    F64(x) => format!("F64:{}", x),
                    F128(x) => format!("F128:{}", x),
                }
            }
            Value::String(s) => format!("S:{}", s),
            Value::Boolean(b) => format!("B:{}", b),
            Value::Char(c) => format!("C:{}", c),
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(Self::value_key).collect();
                format!("A:[{}]", parts.join(","))
            }
            Value::Range {
                start,
                end,
                inclusive,
                step,
            } => {
                format!(
                    "R:{}..{}{}{}",
                    start,
                    if *inclusive { "=" } else { "" },
                    end,
                    if *step != 1 {
                        format!(":{}", step)
                    } else {
                        String::new()
                    }
                )
            }
            Value::Bytes(b) => format!("Bytes:{:?}", b),
            Value::Pair(l, r) => format!("P:({},{})", Self::value_key(l), Self::value_key(r)),
            Value::Triple(a, b, c) => format!(
                "T:({},{},{})",
                Self::value_key(a),
                Self::value_key(b),
                Self::value_key(c)
            ),
            Value::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(Self::value_key).collect();
                format!("Tuple:[{}]", parts.join(","))
            }
            Value::Set(set) => {
                let parts: Vec<String> = set.iter().map(Self::value_key).collect();
                format!("Set:[{}]", parts.join(","))
            }
            Value::Map(map) => {
                let mut parts: Vec<String> = Vec::new();
                for (k, v) in map {
                    parts.push(format!("{}:{}", Self::value_key(k), Self::value_key(v)));
                }
                format!("Map:[{}]", parts.join(","))
            }
            Value::Option(opt) => match opt.as_ref() {
                Some(inner) => format!("Opt:Some({})", Self::value_key(inner)),
                None => "Opt:None".to_string(),
            },
            Value::Result(res) => match res.as_ref() {
                Ok(v) => format!("Res:Ok({})", Self::value_key(v)),
                Err(e) => format!("Res:Err({})", Self::value_key(e)),
            },
            Value::Pointer(p) => format!("Ptr:{}", Self::value_key(p)),
            Value::Reference { target, .. } => format!("Ref:{}", Self::value_key(target)),
            Value::Enum {
                name,
                variant,
                data,
            } => match data {
                Some(d) => format!("Enum:{}::{}({})", name, variant, Self::value_key(d)),
                None => format!("Enum::{}::{}", name, variant),
            },
            Value::Object { type_id, fields } => {
                let field_parts: Vec<String> = fields
                    .iter()
                    .map(|(k, v)| format!("{}:{}", k, Self::value_key(v)))
                    .collect();
                format!("Object:{}:[{}]", type_id.0, field_parts.join(","))
            }
            Value::NativeObject { type_name, .. } => format!("Native:{}", type_name),
            Value::Lambda(_) => "Lambda".to_string(),
            Value::PartialApp { .. } => "PartialApp".to_string(),
            Value::Promise {
                task_id, status, ..
            } => format!("Promise:{}:{:?}", task_id, status),
            Value::Generator { .. } => "Generator".to_string(),
            Value::Channel { .. } => "Channel".to_string(),
            Value::Error { .. } => "Error".to_string(),
            Value::Null => "Null".to_string(),
            Value::Undefined => "Undefined".to_string(),
            Value::Type(_) => "Type".to_string(),
        }
    }

    /// Multiplies a string by a number (repeats the string).
    ///
    /// # Arguments
    /// * `s` - String to multiply
    /// * `n` - Number to multiply by (must be integer)
    /// * `_fo_e` - Overflow flag (unused)
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Repeated string or error message
    pub(super) fn str_mul_string_number(
        s: String,
        n: Number,
        _fo_e: bool,
    ) -> Result<Value, MathErr> {
        use self::Number::*;
        match n {
            I8(v) => Self::str_mul_by_count(&s, v as i128),
            I16(v) => Self::str_mul_by_count(&s, v as i128),
            I32(v) => Self::str_mul_by_count(&s, v as i128),
            I64(v) => Self::str_mul_by_count(&s, v as i128),
            I128(v) => Self::str_mul_by_count(&s, v),
            U8(v) => Self::str_mul_by_count(&s, v as i128),
            U16(v) => Self::str_mul_by_count(&s, v as i128),
            U32(v) => Self::str_mul_by_count(&s, v as i128),
            U64(v) => {
                if v > i128::MAX as u64 {
                    return Err(MathErr::Overflow);
                }
                Self::str_mul_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 {
                    return Err(MathErr::Overflow);
                }
                Self::str_mul_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("умножение строки только на целое")),
        }
    }

    /// Repeats a string by a specified count.
    ///
    /// # Arguments
    /// * `s` - String to repeat
    /// * `count` - Number of repetitions (must be non-negative)
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Repeated string or error message
    pub(super) fn str_mul_by_count(s: &str, count: i128) -> Result<Value, MathErr> {
        if count < 0 {
            return Err(MathErr::DomainError(
                "умножение строки на отрицательное число",
            ));
        }
        let cnt = if count == 0 { 0 } else { count as usize };
        Ok(Value::String(s.repeat(cnt)))
    }

    /// Divides a string by a number (splits into parts).
    ///
    /// # Arguments
    /// * `s` - String to divide
    /// * `n` - Number to divide by (must be integer)
    /// * `_fo_e` - Overflow flag (unused)
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Array of string parts or error message
    pub(super) fn str_div_string_number(
        s: String,
        n: Number,
        _fo_e: bool,
    ) -> Result<Value, MathErr> {
        use self::Number::*;
        match n {
            I8(v) => Self::str_div_by_count(&s, v as i128),
            I16(v) => Self::str_div_by_count(&s, v as i128),
            I32(v) => Self::str_div_by_count(&s, v as i128),
            I64(v) => Self::str_div_by_count(&s, v as i128),
            I128(v) => Self::str_div_by_count(&s, v),
            U8(v) => Self::str_div_by_count(&s, v as i128),
            U16(v) => Self::str_div_by_count(&s, v as i128),
            U32(v) => Self::str_div_by_count(&s, v as i128),
            U64(v) => {
                if v > i128::MAX as u64 {
                    return Err(MathErr::Overflow);
                }
                Self::str_div_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 {
                    return Err(MathErr::Overflow);
                }
                Self::str_div_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("деление строки только на целое")),
        }
    }

    /// Splits a string into equal parts by character count.
    ///
    /// # Arguments
    /// * `s` - String to split
    /// * `count` - Number of parts (must be positive)
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Array of string parts or error message
    pub(super) fn str_div_by_count(s: &str, count: i128) -> Result<Value, MathErr> {
        if count == 0 {
            return Err(MathErr::DivisionByZero);
        }
        if count < 0 {
            return Err(MathErr::DomainError(
                "деление строки на отрицательное число",
            ));
        }
        let n = count as usize;
        // Split into n parts as equally as possible using character count
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        if n == 0 {
            return Err(MathErr::DivisionByZero);
        }
        if n == 1 {
            return Ok(Value::Array(vec![Value::String(s.to_string())]));
        }
        let base = len / n;
        let mut rem = len % n;
        let mut parts: Vec<Value> = Vec::with_capacity(n);
        let mut idx = 0usize;
        for _ in 0..n {
            let mut part_len = base;
            if rem > 0 {
                part_len += 1;
                rem -= 1;
            }
            let part: String = chars[idx..(idx + part_len)].iter().collect();
            parts.push(Value::String(part));
            idx += part_len;
        }
        Ok(Value::Array(parts))
    }

    /// Splits a string by a delimiter.
    ///
    /// # Arguments
    /// * `s` - String to split
    /// * `delim` - Delimiter string (must not be empty)
    /// * `_fo_e` - Overflow flag (unused)
    ///
    /// # Returns
    /// * `Result<Value, MathErr>` - Pair of (parts array, split count) or error message
    pub(super) fn str_div_string_delim(
        s: String,
        delim: String,
        _fo_e: bool,
    ) -> Result<Value, MathErr> {
        if delim.is_empty() {
            return Err(MathErr::DomainError(
                "разделитель не может быть пустой строкой",
            ));
        }
        let parts: Vec<Value> = s
            .split(&delim)
            .map(|p| Value::String(p.to_string()))
            .collect();
        let splits = if parts.is_empty() { 0 } else { parts.len() - 1 } as i64;
        Ok(Value::Pair(
            Box::new(Value::Array(parts)),
            Box::new(Value::Number(Number::I64(splits))),
        ))
    }
}
