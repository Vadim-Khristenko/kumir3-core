// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

// =============================================================================
//         IMPORTS
// =============================================================================

use crate::f128::F128 as tF128;
use crate::types::{Number, Value};
use std::collections::HashMap;

// =============================================================================
//         TYPES
// =============================================================================

/// [STABLE] Enumeration of mathematical operation errors.
///
/// Provides detailed error messages for various math operation failures,
/// including division by zero, domain errors, and type mismatches.
#[derive(Debug, Clone)]
pub enum MathErr {
    DivisionByZero,
    NegativeSqrt,
    NegativeRoot,
    NotRealOneSqrt,
    NegativePowNonInteger,
    Overflow,
    FloatOverflow,
    DomainError(&'static str),
    TypeMismatch(&'static str),
}

impl MathErr {
    /// [STABLE] Returns a human-readable error message for the error variant.
    ///
    /// # Returns
    /// * `String` - Localized error message describing the mathematical error.
    pub fn msg(&self) -> String {
        match self {
            MathErr::DivisionByZero =>
                "[MathErr] Деление на ноль не определено".to_string(),
            MathErr::NegativeSqrt =>
                "[MathErr] Квадратный корень из отрицательного числа не определён".to_string(),
            MathErr::NegativeRoot =>
                "[MathErr] Корень чётной степени из отрицательного числа не определён".to_string(),
            MathErr::NotRealOneSqrt =>
                "[MathErr] Мнимая еденица! К сожалению пока что мы не поддерживаем такие 'Жёские вычисления', а так-то результат i".to_string(),
            MathErr::NegativePowNonInteger =>
                "[MathErr] Отрицательное основание допускается только с целой степенью".to_string(),
            MathErr::Overflow =>
                "[MathErr] Переполнение числа".to_string(),
            MathErr::FloatOverflow =>
                "[MathErr] Переполнение числа (вещественный тип)".to_string(),
            MathErr::DomainError(m) =>
                format!("[MathErr] Нарушение области определения: {}", m),
            MathErr::TypeMismatch(m) =>
                format!("[MathErr] Несовместимые типы операндов: {}", m),
        }
    }
}

/// [STABLE] Warning message for automatic type widening on overflow.
///
/// Used internally when integer operations overflow and are automatically
/// promoted to a wider type to prevent data loss.
fn warn_auto_widen() -> &'static str {
    "[MathWarn] Переполнение, выполнено автоматическое расширение типа"
}

/// [STABLE] Core mathematical operations provider.
///
/// Handles arithmetic, trigonometric, and rounding operations across all
/// supported number types, with automatic type promotion and overflow handling.
pub struct MathOperators;

/// [STABLE] Original floating-point type classification.
///
/// Used internally to track the precision level of floating-point numbers
/// for appropriate rounding and conversion operations.
#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum OrigKind {
    F32,
    F64,
    F128,
    Other,
}

// =============================================================================
//         CORE LOGIC
// =============================================================================

impl MathOperators {
    /// [STABLE] Performs addition with overflow control.
    ///
    /// # Arguments
    /// * `a` - First value
    /// * `b` - Second value
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of addition
    /// * `Err(String)` - Error message
    pub fn add(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_add(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::String(sb)) => Ok(Value::String(sa + &sb)),
            (Value::Array(mut va), Value::Array(vb)) => {
                va.extend(vb);
                Ok(Value::Array(va))
            }
            _ => Err(MathErr::TypeMismatch("операция сложения").msg()),
        }
    }

    /// [STABLE] Performs subtraction with overflow control.
    ///
    /// # Arguments
    /// * `a` - Minuend value
    /// * `b` - Subtrahend value
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of subtraction
    /// * `Err(String)` - Error message
    pub fn sub(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_sub(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::String(sb)) => {
                if sb.is_empty() {
                    Ok(Value::String(sa))
                } else {
                    let res = Self::remove_all_substring_bytes(sa, &sb);
                    Ok(Value::String(res))
                }
            }
            (Value::Array(mut va), Value::Array(vb)) => {
                if vb.len() <= 8 {
                    for item in vb {
                        if let Some(pos) = va.iter().position(|x| x == &item) {
                            va.remove(pos);
                        }
                    }
                    return Ok(Value::Array(va));
                }
                let mut counts: HashMap<String, usize> = HashMap::new();
                for item in &vb {
                    let key = Self::value_key(item);
                    *counts.entry(key).or_insert(0) += 1;
                }
                let mut out: Vec<Value> = Vec::with_capacity(va.len());
                for v in va.into_iter() {
                    let key = Self::value_key(&v);
                    if let Some(cnt) = counts.get_mut(&key)
                        && *cnt > 0
                    {
                        *cnt -= 1;
                        continue;
                    }
                    out.push(v);
                }
                Ok(Value::Array(out))
            }
            _ => Err(MathErr::TypeMismatch("операция вычитания").msg()),
        }
    }

    /// [STABLE] Performs multiplication with overflow control.
    ///
    /// # Arguments
    /// * `a` - First factor
    /// * `b` - Second factor
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of multiplication
    /// * `Err(String)` - Error message
    pub fn mul(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_mul(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::Number(nb)) => Self::str_mul_string_number(sa, nb, fo_e),
            (Value::Number(na), Value::String(sb)) => Self::str_mul_string_number(sb, na, fo_e),
            _ => Err(MathErr::TypeMismatch("операция умножения").msg()),
        }
    }

    /// [STABLE] Performs division with overflow control.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of division
    /// * `Err(String)` - Error message (division by zero, overflow)
    pub fn div(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(_), Value::Number(nb)) if Self::is_zero_num(&nb) => {
                Err(MathErr::DivisionByZero.msg())
            }
            (Value::Number(na), Value::Number(nb)) => {
                Self::num_div(na, nb, fo_e).map(Value::Number)
            }
            (Value::String(sa), Value::Number(nb)) => Self::str_div_string_number(sa, nb, fo_e),
            (Value::String(sa), Value::String(sb)) => Self::str_div_string_delim(sa, sb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция деления").msg()),
        }
    }

    /// [STABLE] Performs modulus operation.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Remainder of division
    /// * `Err(String)` - Error message
    pub fn modulus(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Self::num_mod(na, nb, fo_e)
                .map(Value::Number)
                .map_err(|e| e.msg()),
            _ => Err(MathErr::TypeMismatch("операция взятия остатка").msg()),
        }
    }

    /// [STABLE] Performs exponentiation.
    ///
    /// # Arguments
    /// * `a` - Base
    /// * `b` - Exponent
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Result of exponentiation
    /// * `Err(String)` - Error message
    pub fn pow(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Self::num_pow(na, nb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция возведения в степень").msg()),
        }
    }

    /// [STABLE] Computes square root.
    ///
    /// # Arguments
    /// * `a` - Value to take square root of (must be non-negative)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Square root result
    /// * `Err(String)` - Error message (negative input, overflow)
    pub fn sqrt(a: Value, fo_e: bool) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_sqrt(n, fo_e),
            _ => Err(MathErr::TypeMismatch("sqrt ожидает число").msg()),
        }
    }

    /// [STABLE] Computes nth root.
    ///
    /// # Arguments
    /// * `a` - Value to take root of
    /// * `n` - Root degree (must be non-zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Nth root result
    /// * `Err(String)` - Error message
    pub fn root(a: Value, n: Value, fo_e: bool) -> Result<Value, String> {
        let n_int = match n {
            Value::Number(Number::I32(v)) => v as i64,
            Value::Number(Number::I64(v)) => v,
            Value::Number(Number::I128(v)) => {
                if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
                    v as i64
                } else {
                    return Err(MathErr::DomainError("слишком большая степень корня").msg());
                }
            }
            _ => return Err(MathErr::TypeMismatch("root ожидает целую степень").msg()),
        };
        if n_int == 0 {
            return Err(MathErr::DomainError("корень нулевой степени не определён").msg());
        }
        match a {
            Value::Number(num) => Self::num_root(num, n_int, fo_e),
            _ => Err(MathErr::TypeMismatch("root ожидает число").msg()),
        }
    }

    /// [STABLE] Rounds a number to specified precision.
    ///
    /// # Arguments
    /// * `a` - Value to round
    /// * `b` - Optional precision (decimal places if positive, significant digits if negative)
    /// * `rf` - Optional rounding factor (1-9, default 5)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type.
    ///
    /// # Returns
    /// * `Ok(Value)` - Rounded value
    /// * `Err(String)` - Error message
    pub fn round(
        a: Value,
        b: Option<Value>,
        rf: Option<Value>,
        _fo_e: bool,
    ) -> Result<Value, String> {
        let prec: i32 = match b {
            Some(Value::Number(nb)) => match Self::to_i128(&nb) {
                Some(v) => v as i32,
                None => {
                    return Err(
                        MathErr::TypeMismatch("round: точность должна быть целым числом").msg(),
                    );
                }
            },
            Some(_) => {
                return Err(MathErr::TypeMismatch("round: точность должна быть числом").msg());
            }
            None => 0,
        };

        let rf_val: i8 = match rf {
            Some(Value::Number(nr)) => match Self::to_i128(&nr) {
                Some(v) => v as i8,
                None => {
                    return Err(MathErr::TypeMismatch("round: rf должна быть целым числом").msg());
                }
            },
            Some(_) => return Err(MathErr::TypeMismatch("round: rf должна быть числом").msg()),
            None => 5,
        };

        if !(1..=9).contains(&rf_val) {
            return Err(MathErr::DomainError("параметр rf должен быть в диапазоне 1..9").msg());
        }

        match a {
            Value::Number(n) => {
                let res = Self::num_round(n, prec, rf_val)?;
                Ok(Value::Number(res))
            }
            _ => Err(MathErr::TypeMismatch("round ожидает число").msg()),
        }
    }

    /// [STABLE] Computes sine of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Sine value
    /// * `Err(String)` - Error message
    pub fn sin(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_sin(n))),
            _ => Err(MathErr::TypeMismatch("sin ожидает число").msg()),
        }
    }

    /// [STABLE] Computes cosine of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Cosine value
    /// * `Err(String)` - Error message
    pub fn cos(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_cos(n))),
            _ => Err(MathErr::TypeMismatch("cos ожидает число").msg()),
        }
    }

    /// [STABLE] Computes tangent of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians
    ///
    /// # Returns
    /// * `Ok(Value)` - Tangent value
    /// * `Err(String)` - Error message
    pub fn tg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_tan(n))),
            _ => Err(MathErr::TypeMismatch("tg ожидает число").msg()),
        }
    }

    /// [STABLE] Computes cotangent of an angle.
    ///
    /// # Arguments
    /// * `a` - Angle in radians (must not be multiple of π)
    ///
    /// # Returns
    /// * `Ok(Value)` - Cotangent value
    /// * `Err(String)` - Error message (division by zero)
    pub fn ctg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_ctg(n),
            _ => Err(MathErr::TypeMismatch("ctg ожидает число").msg()),
        }
    }

    /// [STABLE] Computes absolute value.
    ///
    /// # Arguments
    /// * `a` - Numeric value
    ///
    /// # Returns
    /// * `Ok(Value)` - Absolute value
    /// * `Err(String)` - Error message
    pub fn abs(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_abs(n))),
            _ => Err(MathErr::TypeMismatch("abs ожидает число").msg()),
        }
    }

    /// Removes all occurrences of a substring from a string using KMP algorithm.
    ///
    /// # Arguments
    /// * `s` - The input string
    /// * `pat` - The substring pattern to remove
    ///
    /// # Returns
    /// * `String` - The string with all occurrences of the pattern removed
    fn remove_all_substring_bytes(s: String, pat: &str) -> String {
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
    fn value_key(v: &Value) -> String {
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

    /// Checks if a Number is zero.
    ///
    /// # Arguments
    /// * `n` - The Number to check
    ///
    /// # Returns
    /// * `bool` - True if the number is zero, false otherwise
    fn is_zero_num(n: &Number) -> bool {
        use self::Number::*;
        match n {
            I8(v) => *v == 0,
            I16(v) => *v == 0,
            I32(v) => *v == 0,
            I64(v) => *v == 0,
            I128(v) => *v == 0,
            U8(v) => *v == 0,
            U16(v) => *v == 0,
            U32(v) => *v == 0,
            U64(v) => *v == 0,
            U128(v) => *v == 0,
            F32(v) => *v == 0.0,
            F64(v) => *v == 0.0,
            F128(v) => v.is_zero(),
        }
    }

    /// Extracts signedness and rank information from an integer Number.
    ///
    /// # Arguments
    /// * `n` - The Number to analyze
    ///
    /// # Returns
    /// * `Option<(bool, u8)>` - (signed, rank) where rank indicates size (1=i8/u8, 2=i16/u16, etc.), None for non-integers
    fn int_info(n: &Number) -> Option<(bool, u8)> {
        use self::Number::*;
        let (signed, rank) = match n {
            I8(_) => (true, 1),
            I16(_) => (true, 2),
            I32(_) => (true, 3),
            I64(_) => (true, 4),
            I128(_) => (true, 5),
            U8(_) => (false, 1),
            U16(_) => (false, 2),
            U32(_) => (false, 3),
            U64(_) => (false, 4),
            U128(_) => (false, 5),
            _ => return None,
        };
        Some((signed, rank))
    }

    /// Converts a Number to i128 if possible.
    ///
    /// # Arguments
    /// * `n` - The Number to convert
    ///
    /// # Returns
    /// * `Option<i128>` - The converted value, or None if out of range or not an integer
    fn to_i128(n: &Number) -> Option<i128> {
        use self::Number::*;
        match *n {
            I8(v) => Some(v as i128),
            I16(v) => Some(v as i128),
            I32(v) => Some(v as i128),
            I64(v) => Some(v as i128),
            I128(v) => Some(v),
            U8(v) => Some(v as i128),
            U16(v) => Some(v as i128),
            U32(v) => Some(v as i128),
            U64(v) => {
                if v <= i128::MAX as u64 {
                    Some(v as i128)
                } else {
                    None
                }
            }
            U128(v) => {
                if v <= i128::MAX as u128 {
                    Some(v as i128)
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    /// Converts an i128 value back to a Number of specified signedness and rank.
    ///
    /// # Arguments
    /// * `x` - The i128 value to convert
    /// * `signed` - Whether the target type is signed
    /// * `rank` - The size rank (1=i8/u8, 2=i16/u16, etc.)
    ///
    /// # Returns
    /// * `Option<Number>` - The converted Number, or None if out of range
    fn from_i128_in_type(x: i128, signed: bool, rank: u8) -> Option<Number> {
        use self::Number::*;
        if signed {
            match rank {
                1 if x >= i8::MIN as i128 && x <= i8::MAX as i128 => Some(I8(x as i8)),
                2 if x >= i16::MIN as i128 && x <= i16::MAX as i128 => Some(I16(x as i16)),
                3 if x >= i32::MIN as i128 && x <= i32::MAX as i128 => Some(I32(x as i32)),
                4 if x >= i64::MIN as i128 && x <= i64::MAX as i128 => Some(I64(x as i64)),
                5 => Some(I128(x)),
                _ => None,
            }
        } else {
            if x < 0 {
                return None;
            }
            let ux = x as u128;
            match rank {
                1 if ux <= u8::MAX as u128 => Some(U8(ux as u8)),
                2 if ux <= u16::MAX as u128 => Some(U16(ux as u16)),
                3 if ux <= u32::MAX as u128 => Some(U32(ux as u32)),
                4 if ux <= u64::MAX as u128 => Some(U64(ux as u64)),
                5 => Some(U128(ux)),
                _ => None,
            }
        }
    }

    /// Converts a Number to F128 with full precision handling.
    ///
    /// # Arguments
    /// * `n` - The Number to convert
    ///
    /// # Returns
    /// * `tF128` - The converted F128 value
    fn to_f128_full(n: &Number) -> tF128 {
        use self::Number::*;
        match *n {
            I8(v) => tF128::from(v as i64),
            I16(v) => tF128::from(v as i64),
            I32(v) => tF128::from(v as i64),
            I64(v) => tF128::from(v),
            I128(v) => {
                if v >= i64::MIN as i128 && v <= i64::MAX as i128 {
                    tF128::from(v as i64)
                } else {
                    tF128::from(v as f64)
                }
            }
            U8(v) => tF128::from(v as u64),
            U16(v) => tF128::from(v as u64),
            U32(v) => tF128::from(v as u64),
            U64(v) => tF128::from(v),
            U128(v) => {
                if v <= u64::MAX as u128 {
                    tF128::from(v as u64)
                } else {
                    tF128::from(v as f64)
                }
            }
            F32(v) => tF128::from(v),
            F64(v) => tF128::from(v),
            F128(v) => v,
        }
    }

    /// Internal numeric addition with overflow control.
    ///
    /// # Arguments
    /// * `a` - First number
    /// * `b` - Second number
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, String>` - Result of addition or error message
    fn num_add(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_add(y), |x, y| x + y)
    }

    /// Internal numeric subtraction with overflow control.
    ///
    /// # Arguments
    /// * `a` - Minuend
    /// * `b` - Subtrahend
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, String>` - Result of subtraction or error message
    fn num_sub(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_sub(y), |x, y| x - y)
    }

    /// Internal numeric multiplication with overflow control.
    ///
    /// # Arguments
    /// * `a` - First factor
    /// * `b` - Second factor
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, String>` - Result of multiplication or error message
    fn num_mul(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_mul(y), |x, y| x * y)
    }

    /// Internal numeric division with overflow control.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, String>` - Result of division or error message
    fn num_div(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero.msg());
        }
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);
        let r = fa / fb;
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Number::F128(r))
    }

    /// Internal numeric modulus operation.
    ///
    /// # Arguments
    /// * `a` - Dividend
    /// * `b` - Divisor (must not be zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Number, MathErr>` - Remainder or error
    fn num_mod(a: Number, b: Number, fo_e: bool) -> Result<Number, MathErr> {
        use self::Number::*;
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero);
        }
        match (a, b) {
            (I64(x), I64(y)) => Ok(I64(x % y)),
            (I32(x), I32(y)) => Ok(I32(x % y)),
            (U64(x), U64(y)) => Ok(U64(x % y)),
            (U32(x), U32(y)) => Ok(U32(x % y)),
            _ => Err(MathErr::TypeMismatch("остаток только для целых")),
        }
    }

    /// Internal numeric exponentiation.
    ///
    /// # Arguments
    /// * `a` - Base
    /// * `b` - Exponent
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, String>` - Result of exponentiation or error message
    fn num_pow(a: Number, b: Number, fo_e: bool) -> Result<Value, String> {
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);

        if fa.is_sign_negative() && !Self::is_effectively_integer(fb) && fo_e {
            return Err(MathErr::NegativePowNonInteger.msg());
        }

        let r = fa.powf(fb);
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Value::Number(Number::F128(r)))
    }

    /// Internal square root computation.
    ///
    /// # Arguments
    /// * `n` - Value to take square root of (must be non-negative)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, String>` - Square root result or error message
    fn num_sqrt(n: Number, fo_e: bool) -> Result<Value, String> {
        let x = Self::to_f128_full(&n);
        // Easter egg: calc sqrt(-1) -> return specific NotRealOneSqrt error
        if x == tF128::from(-1.0_f64) {
            return Err(MathErr::NotRealOneSqrt.msg());
        }
        if x.is_sign_negative() {
            return Err(MathErr::NegativeSqrt.msg());
        }
        let r = x.sqrt();
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Value::Number(Number::F128(r)))
    }

    /// Internal nth root computation.
    ///
    /// # Arguments
    /// * `n` - Value to take root of
    /// * `k` - Root degree (must be non-zero)
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    ///
    /// # Returns
    /// * `Result<Value, String>` - Nth root result or error message
    fn num_root(n: Number, k: i64, fo_e: bool) -> Result<Value, String> {
        let x = Self::to_f128_full(&n);
        if k == 0 {
            return Err(MathErr::DomainError("root(x,0) не определён").msg());
        }
        if x.is_sign_negative() && k % 2 == 0 {
            return Err(MathErr::NegativeRoot.msg());
        }
        let kf = k as f64;
        let r = x.powf(tF128::from(1.0_f64 / kf));
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Value::Number(Number::F128(r)))
    }

    /// Internal number rounding to specified precision.
    ///
    /// # Arguments
    /// * `n` - Number to round
    /// * `prec` - Precision (decimal places if positive, significant digits if negative)
    /// * `rf` - Rounding factor (1-9, default 5)
    ///
    /// # Returns
    /// * `Result<Number, String>` - Rounded number or error message
    fn num_round(n: Number, prec: i32, rf: i8) -> Result<Number, String> {
        match n {
            Number::F128(v) => {
                let rounded = Self::round_decimal_f128(v, prec, rf);
                Ok(Number::F128(rounded))
            }
            Number::F64(v) => {
                let rounded = Self::round_decimal_f64(v, prec, rf);
                Ok(Number::F64(rounded))
            }
            Number::F32(v) => {
                let rounded = Self::round_decimal_f64(v as f64, prec, rf) as f32;
                Ok(Number::F32(rounded))
            }
            _ => {
                if prec >= 0 {
                    Ok(n)
                } else {
                    Self::round_integer(n, prec, rf)
                }
            }
        }
    }

    /// Rounds a F128 decimal number to specified precision.
    ///
    /// # Arguments
    /// * `v` - F128 value to round
    /// * `prec` - Decimal precision
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `tF128` - Rounded F128 value
    fn round_decimal_f128(v: tF128, prec: i32, rf: i8) -> tF128 {
        if !v.is_finite() || prec < 0 {
            return if prec < 0 {
                Self::round_f128_to_power_of_10(v, -prec, rf)
            } else {
                v
            };
        }

        if rf == 5 {
            return v.round_to(prec);
        }

        let factor = tF128::from(10u64).powi(prec);
        if !factor.is_finite() || factor.is_zero() {
            return v;
        }

        let scaled = v * factor;
        let truncated = scaled.trunc();
        let frac = scaled - truncated;
        let abs_frac = frac.abs();

        let threshold = tF128::from(rf as i64) / tF128::from(10);

        let rounded_scaled = if abs_frac >= threshold {
            if v.is_sign_negative() {
                truncated - tF128::ONE
            } else {
                truncated + tF128::ONE
            }
        } else {
            truncated
        };

        rounded_scaled / factor
    }

    /// Rounds a f64 decimal number to specified precision.
    ///
    /// # Arguments
    /// * `v` - f64 value to round
    /// * `prec` - Decimal precision
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `f64` - Rounded f64 value
    fn round_decimal_f64(v: f64, prec: i32, rf: i8) -> f64 {
        if !v.is_finite() {
            return v;
        }

        if rf == 5 {
            let multiplier = 10f64.powi(prec);
            return (v * multiplier).round() / multiplier;
        }

        let multiplier = 10f64.powi(prec);
        let scaled = v * multiplier;
        let truncated = scaled.trunc();
        let frac = (scaled - truncated).abs();

        let threshold = (rf as f64) / 10.0;

        if frac >= threshold {
            if v >= 0.0 {
                (truncated + 1.0) / multiplier
            } else {
                (truncated - 1.0) / multiplier
            }
        } else {
            truncated / multiplier
        }
    }

    /// Rounds F128 to nearest power of 10.
    ///
    /// # Arguments
    /// * `v` - F128 value to round
    /// * `power` - Power of 10 to round to
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `tF128` - Rounded F128 value
    fn round_f128_to_power_of_10(v: tF128, power: i32, rf: i8) -> tF128 {
        let divisor = tF128::from(10u64).powi(power);
        let scaled = v / divisor;
        let truncated = scaled.trunc();
        let frac = (scaled - truncated).abs();

        let half = tF128::from(5) / tF128::from(10);
        let threshold = tF128::from(rf as i64) / tF128::from(10);
        let epsilon = tF128::EPSILON * tF128::from(10);
        let is_half = (frac - half).abs() < epsilon;

        let rounded_scaled = if is_half || frac >= threshold {
            if v.is_sign_negative() {
                truncated - tF128::ONE
            } else {
                truncated + tF128::ONE
            }
        } else {
            truncated
        };

        rounded_scaled * divisor
    }

    /// Rounds an integer number to specified precision.
    ///
    /// # Arguments
    /// * `n` - Integer number to round
    /// * `prec` - Precision (negative for significant digits)
    /// * `rf` - Rounding factor (1-9)
    ///
    /// # Returns
    /// * `Result<Number, String>` - Rounded integer or error message
    fn round_integer(n: Number, prec: i32, rf: i8) -> Result<Number, String> {
        let exp = (-prec) as u32;
        let divisor = 10i128.pow(exp);

        let val = Self::to_i128(&n).ok_or_else(|| MathErr::Overflow.msg())?;
        let sign = if val < 0 { -1 } else { 1 };
        let abs_val = val.abs();

        let remainder = abs_val % divisor;
        let base = abs_val - remainder;

        let half = divisor / 2;
        let _threshold = (exp * rf as u32) as i128; // rf/10 * 10^exp = rf * 10^(exp-1)

        let rounded = if remainder >= half {
            base + divisor
        } else {
            base
        };

        let result = rounded * sign;

        Self::from_i128_in_type(
            result,
            val < 0,
            Self::int_info(&n).map(|(_, r)| r).unwrap_or(5),
        )
        .ok_or_else(|| MathErr::Overflow.msg())
    }

    /// Computes sine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// Computes cosine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Cosine value
    /// # Returns
    /// * `Number` - Sine value
    fn num_sin(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.sin())
    }

    /// Computes cosine of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Cosine value
    fn num_cos(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.cos())
    }

    /// Computes tangent of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians
    ///
    /// # Returns
    /// * `Number` - Tangent value
    fn num_tan(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.tan())
    }

    /// Computes cotangent of a number.
    ///
    /// # Arguments
    /// * `n` - Angle in radians (must not be multiple of π)
    ///
    /// # Returns
    /// * `Result<Value, String>` - Cotangent value or division by zero error
    fn num_ctg(n: Number) -> Result<Value, String> {
        let f = Self::to_f128_full(&n);
        let s = f.sin();
        if s.is_zero() {
            return Err(MathErr::DivisionByZero.msg());
        }
        Ok(Value::Number(Number::F128(f.ctg())))
    }

    /// Computes absolute value of a number.
    ///
    /// # Arguments
    /// * `n` - Number to take absolute value of
    ///
    /// # Returns
    /// * `Number` - Absolute value
    fn num_abs(n: Number) -> Number {
        use self::Number::*;
        match n {
            I8(v) => I8(v.abs()),
            I16(v) => I16(v.abs()),
            I32(v) => I32(v.abs()),
            I64(v) => I64(v.abs()),
            I128(v) => I128(v.abs()),
            U8(v) => U8(v),
            U16(v) => U16(v),
            U32(v) => U32(v),
            U64(v) => U64(v),
            U128(v) => U128(v),
            F32(v) => F32(v.abs()),
            F64(v) => F64(v.abs()),
            F128(v) => Number::F128(v.abs()),
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
    /// * `Result<Value, String>` - Repeated string or error message
    fn str_mul_string_number(s: String, n: Number, _fo_e: bool) -> Result<Value, String> {
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
                    return Err(MathErr::Overflow.msg());
                }
                Self::str_mul_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 {
                    return Err(MathErr::Overflow.msg());
                }
                Self::str_mul_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("умножение строки только на целое").msg()),
        }
    }

    /// Repeats a string by a specified count.
    ///
    /// # Arguments
    /// * `s` - String to repeat
    /// * `count` - Number of repetitions (must be non-negative)
    ///
    /// # Returns
    /// * `Result<Value, String>` - Repeated string or error message
    fn str_mul_by_count(s: &str, count: i128) -> Result<Value, String> {
        if count < 0 {
            return Err(MathErr::DomainError("умножение строки на отрицательное число").msg());
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
    /// * `Result<Value, String>` - Array of string parts or error message
    fn str_div_string_number(s: String, n: Number, _fo_e: bool) -> Result<Value, String> {
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
                    return Err(MathErr::Overflow.msg());
                }
                Self::str_div_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 {
                    return Err(MathErr::Overflow.msg());
                }
                Self::str_div_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("деление строки только на целое").msg()),
        }
    }

    /// Splits a string into equal parts by character count.
    ///
    /// # Arguments
    /// * `s` - String to split
    /// * `count` - Number of parts (must be positive)
    ///
    /// # Returns
    /// * `Result<Value, String>` - Array of string parts or error message
    fn str_div_by_count(s: &str, count: i128) -> Result<Value, String> {
        if count == 0 {
            return Err(MathErr::DivisionByZero.msg());
        }
        if count < 0 {
            return Err(MathErr::DomainError("деление строки на отрицательное число").msg());
        }
        let n = count as usize;
        // Split into n parts as equally as possible using character count
        let chars: Vec<char> = s.chars().collect();
        let len = chars.len();
        if n == 0 {
            return Err(MathErr::DivisionByZero.msg());
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
    /// * `Result<Value, String>` - Pair of (parts array, split count) or error message
    fn str_div_string_delim(s: String, delim: String, _fo_e: bool) -> Result<Value, String> {
        if delim.is_empty() {
            return Err(MathErr::DomainError("разделитель не может быть пустой строкой").msg());
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

    /// Performs integer or floating-point operation with overflow handling.
    ///
    /// # Arguments
    /// * `a` - First operand
    /// * `b` - Second operand
    /// * `fo_e` - If true, overflow causes error; if false, auto-widens type
    /// * `int_op` - Integer operation function
    /// * `float_op` - Floating-point operation function
    ///
    /// # Returns
    /// * `Result<Number, String>` - Result of operation or error message
    fn int_or_float(
        a: Number,
        b: Number,
        fo_e: bool,
        int_op: fn(i128, i128) -> i128,
        float_op: fn(tF128, tF128) -> tF128,
    ) -> Result<Number, String> {
        if let (Some((sa, ra)), Some((sb, rb))) = (Self::int_info(&a), Self::int_info(&b)) {
            let signed = sa || sb;
            let rank = ra.max(rb);
            if let (Some(x), Some(y)) = (Self::to_i128(&a), Self::to_i128(&b)) {
                let res = int_op(x, y);

                if let Some(n) = Self::from_i128_in_type(res, signed, rank) {
                    return Ok(n);
                }

                if fo_e {
                    return Err(MathErr::Overflow.msg());
                }

                let widened = if signed {
                    if (i128::MIN..=i128::MAX).contains(&res) {
                        Number::I128(res)
                    } else {
                        return Err(MathErr::Overflow.msg());
                    }
                } else if res >= 0 {
                    Number::U128(res as u128)
                } else {
                    return Err(MathErr::Overflow.msg());
                };

                let _ = warn_auto_widen();
                return Ok(widened);
            }
        }

        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);
        let r = float_op(fa, fb);
        if (r.is_infinite() || r.is_nan()) && fo_e {
            return Err(MathErr::FloatOverflow.msg());
        }
        Ok(Number::F128(r))
    }

    /// Checks if a F128 value is effectively an integer.
    ///
    /// # Arguments
    /// * `x` - F128 value to check
    ///
    /// # Returns
    /// * `bool` - True if the value is an integer, false otherwise
    fn is_effectively_integer(x: tF128) -> bool {
        x.is_integer()
    }
}

#[cfg(test)]
mod tests;
