use crate::shared::f128::{F128 as tF128};
use crate::shared::types::{Number, Value};
use std::collections::HashMap;

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

fn warn_auto_widen() -> &'static str {
    "[MathWarn] Переполнение, выполнено автоматическое расширение типа"
}

pub struct MathOperators;

#[derive(Debug, Copy, Clone, PartialEq, Eq)]
enum OrigKind { F32, F64, F128, Other }

impl MathOperators {
    // ====================
    // Публичные операции
    // ====================

    /// Сложение. fo_e: true => переполнение даёт ошибку, false => авто-расширение.
    pub fn add(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_add(na, nb, fo_e).map(Value::Number),
            (Value::String(sa), Value::String(sb)) =>
                Ok(Value::String(sa + &sb)),
            (Value::Array(mut va), Value::Array(vb)) => {
                va.extend(vb);
                Ok(Value::Array(va))
            }
            _ => Err(MathErr::TypeMismatch("операция сложения").msg()),
        }
    }

    pub fn sub(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_sub(na, nb, fo_e).map(Value::Number),
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
                    if let Some(cnt) = counts.get_mut(&key) {
                        if *cnt > 0 {
                            *cnt -= 1;
                            continue;
                        }
                    }
                    out.push(v);
                }
                Ok(Value::Array(out))
            }
            _ => Err(MathErr::TypeMismatch("операция вычитания").msg()),
        }
    }

    pub fn mul(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_mul(na, nb, fo_e).map(Value::Number),
            (Value::String(sa), Value::Number(nb)) =>
                Self::str_mul_string_number(sa, nb, fo_e),
            (Value::Number(na), Value::String(sb)) =>
                Self::str_mul_string_number(sb, na, fo_e),
            _ => Err(MathErr::TypeMismatch("операция умножения").msg()),
        }
    }

    pub fn div(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(_), Value::Number(nb)) if Self::is_zero_num(&nb) =>
                Err(MathErr::DivisionByZero.msg()),
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_div(na, nb, fo_e).map(Value::Number),
            (Value::String(sa), Value::Number(nb)) =>
                Self::str_div_string_number(sa, nb, fo_e),
            (Value::String(sa), Value::String(sb)) =>
                Self::str_div_string_delim(sa, sb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция деления").msg()),
        }
    }

    pub fn modulus(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_mod(na, nb, fo_e).map(Value::Number).map_err(|e| e.msg()),
            _ => Err(MathErr::TypeMismatch("операция взятия остатка").msg()),
        }
    }

    pub fn pow(a: Value, b: Value, fo_e: bool) -> Result<Value, String> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) =>
                Self::num_pow(na, nb, fo_e),
            _ => Err(MathErr::TypeMismatch("операция возведения в степень").msg()),
        }
    }

    pub fn sqrt(a: Value, fo_e: bool) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_sqrt(n, fo_e),
            _ => Err(MathErr::TypeMismatch("sqrt ожидает число").msg()),
        }
    }

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

    pub fn round(a: Value, b: Option<Value>, rf: Option<Value>, fo_e: bool) -> Result<Value, String> {
        let prec: i32 = match b {
            Some(Value::Number(nb)) => match Self::to_i128(&nb) {
                Some(v) => v as i32,
                None => return Err(MathErr::TypeMismatch("round: точность должна быть целым числом").msg()),
            },
            Some(_) => return Err(MathErr::TypeMismatch("round: точность должна быть числом").msg()),
            None => 0,
        };

        let rf_val: i8 = match rf {
            Some(Value::Number(nr)) => match Self::to_i128(&nr) {
                Some(v) => v as i8,
                None => return Err(MathErr::TypeMismatch("round: rf должна быть целым числом").msg()),
            },
            Some(_) => return Err(MathErr::TypeMismatch("round: rf должна быть числом").msg()),
            None => 5,
        };

        if rf_val < 1 || rf_val > 9 {
            return Err(MathErr::DomainError("параметр rf должен быть в диапазоне 1..9").msg());
        }

        match a {
            Value::Number(n) => {
                let res = Self::num_round(n, prec, rf_val, fo_e)?;
                Ok(Value::Number(res))
            }
            _ => Err(MathErr::TypeMismatch("round ожидает число").msg()),
        }
    }

    pub fn sin(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_sin(n))),
            _ => Err(MathErr::TypeMismatch("sin ожидает число").msg()),
        }
    }

    pub fn cos(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_cos(n))),
            _ => Err(MathErr::TypeMismatch("cos ожидает число").msg()),
        }
    }

    pub fn tg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_tan(n))),
            _ => Err(MathErr::TypeMismatch("tg ожидает число").msg()),
        }
    }

    pub fn ctg(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Self::num_ctg(n),
            _ => Err(MathErr::TypeMismatch("ctg ожидает число").msg()),
        }
    }

    pub fn abs(a: Value) -> Result<Value, String> {
        match a {
            Value::Number(n) => Ok(Value::Number(Self::num_abs(n))),
            _ => Err(MathErr::TypeMismatch("abs ожидает число").msg()),
        }
    }

    // ====================
    // Внутренние утилиты
    // ====================
    fn remove_all_substring_bytes(s: String, pat: &str) -> String {
        let s_bytes = s.into_bytes();
        let p = pat.as_bytes();
        let m = p.len();
        if m == 0 { return String::from_utf8(s_bytes).unwrap_or_default(); }
        let n = s_bytes.len();
        let mut lps = vec![0usize; m];
        {
            let mut len = 0usize;
            let mut i = 1usize;
            while i < m {
                if p[i] == p[len] { len += 1; lps[i] = len; i += 1; }
                else if len != 0 { len = lps[len - 1]; }
                else { lps[i] = 0; i += 1; }
            }
        }

        let mut out: Vec<u8> = Vec::with_capacity(n);
        let mut history: Vec<usize> = Vec::with_capacity(n);
        for &b in s_bytes.iter() {
            let mut j = *history.last().unwrap_or(&0);
            while j > 0 && p[j] != b { j = lps[j - 1]; }
            if p[j] == b { j += 1; }
            out.push(b);
            history.push(j);
            if j == m {
                for _ in 0..m { out.pop(); }
                history.truncate(history.len() - m);
            }
        }
        String::from_utf8(out).unwrap_or_default()
    }

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
                    F128(x) => format!("F128:{}", x.to_string()),
                }
            }
            Value::String(s) => format!("S:{}", s),
            Value::Boolean(b) => format!("B:{}", b),
            Value::Char(c) => format!("C:{}", c),
            Value::Array(arr) => {
                let parts: Vec<String> = arr.iter().map(Self::value_key).collect();
                format!("A:[{}]", parts.join(","))
            }
            Value::Pair(l, r) => format!("P:({},{})", Self::value_key(l), Self::value_key(r)),
            Value::Triple(a, b, c) => format!("T:({},{},{})", Self::value_key(a), Self::value_key(b), Self::value_key(c)),
            Value::Tuple(items) => {
                let parts: Vec<String> = items.iter().map(Self::value_key).collect();
                format!("Tuple:[{}]", parts.join(","))
            }
            Value::Set(set) => {
                let parts: Vec<String> = set.iter().map(|x| Self::value_key(x)).collect();
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
            }
            Value::Result(res) => match res.as_ref() {
                Ok(v) => format!("Res:Ok({})", Self::value_key(v)),
                Err(e) => format!("Res:Err({})", Self::value_key(e)),
            }
            Value::Pointer(p) => format!("Ptr:{}", Self::value_key(p)),
            Value::Enum { name, variant, data } => {
                match data {
                    Some(d) => format!("Enum:{}::{}({})", name, variant, Self::value_key(d)),
                    None => format!("Enum::{}::{}", name, variant),
                }
            }
            Value::Object { type_id, fields } => {
                let field_parts: Vec<String> = fields.iter()
                    .map(|(k, v)| format!("{}:{}", k, Self::value_key(v)))
                    .collect();
                format!("Object:{}:[{}]", type_id.0, field_parts.join(","))
            }
            Value::NativeObject { type_name, .. } => format!("Native:{}", type_name),
            Value::Null => "Null".to_string(),
            Value::Undefined => "Undefined".to_string(),
            Value::Promise { task_id, status, .. } => format!("Promise:{}:{:?}", task_id, status),
        }
    }

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

    // --- промоушен целых ---

    fn int_info(n: &Number) -> Option<(bool, u8)> {
        use self::Number::*;
        let (signed, rank) = match n {
            I8(_)   => (true, 1),
            I16(_)  => (true, 2),
            I32(_)  => (true, 3),
            I64(_)  => (true, 4),
            I128(_) => (true, 5),
            U8(_)   => (false,1),
            U16(_)  => (false,2),
            U32(_)  => (false,3),
            U64(_)  => (false,4),
            U128(_) => (false,5),
            _ => return None,
        };
        Some((signed, rank))
    }

    fn to_i128(n: &Number) -> Option<i128> {
        use self::Number::*;
        match *n {
            I8(v)   => Some(v as i128),
            I16(v)  => Some(v as i128),
            I32(v)  => Some(v as i128),
            I64(v)  => Some(v as i128),
            I128(v) => Some(v),
            U8(v)   => Some(v as i128),
            U16(v)  => Some(v as i128),
            U32(v)  => Some(v as i128),
            U64(v)  => {
                if v <= i128::MAX as u64 { Some(v as i128) } else { None }
            }
            U128(v) => {
                if v <= i128::MAX as u128 { Some(v as i128) } else { None }
            }
            _ => None,
        }
    }

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
            if x < 0 { return None; }
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

    // --- F128 вспомогательное ---

    fn to_f128_full(n: &Number) -> tF128 {
        use self::Number::*;
        match *n {
            I8(v)   => tF128::from(v as f64),
            I16(v)  => tF128::from(v as f64),
            I32(v)  => tF128::from(v as f64),
            I64(v)  => tF128::from(v as f64),
            I128(v) => tF128::from(v as f64),
            U8(v)   => tF128::from(v as f64),
            U16(v)  => tF128::from(v as f64),
            U32(v)  => tF128::from(v as f64),
            U64(v)  => tF128::from(v as f64),
            U128(v) => tF128::from(v as f64),
            F32(v)  => tF128::from(v as f64),
            F64(v)  => tF128::from(v),
            F128(v) => v,
        }
    }

    // ====================
    // Реализация числовых операций
    // ====================

    fn num_add(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_add(y), |x, y| x + y)
    }

    fn num_sub(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_sub(y), |x, y| x - y)
    }

    fn num_mul(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        Self::int_or_float(a, b, fo_e, |x, y| x.wrapping_mul(y), |x, y| x * y)
    }

    fn num_div(a: Number, b: Number, fo_e: bool) -> Result<Number, String> {
        if Self::is_zero_num(&b) {
            return Err(MathErr::DivisionByZero.msg());
        }
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);
        let r = fa / fb;
        if r.is_infinite() || r.is_nan() {
            if fo_e {
                return Err(MathErr::FloatOverflow.msg());
            }
        }
        Ok(Number::F128(r))
    }

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
            _ => {
                if fo_e {
                    Err(MathErr::TypeMismatch("остаток только для целых"))
                } else {
                    Err(MathErr::TypeMismatch("остаток только для целых"))
                }
            }
        }
    }

    fn num_pow(a: Number, b: Number, fo_e: bool) -> Result<Value, String> {
        let fa = Self::to_f128_full(&a);
        let fb = Self::to_f128_full(&b);

        if fa.is_sign_negative() && !Self::is_effectively_integer(fb) {
            if fo_e {
                return Err(MathErr::NegativePowNonInteger.msg());
            }
        }

        let r = fa.powf(fb);
        if r.is_infinite() || r.is_nan() {
            if fo_e {
                return Err(MathErr::FloatOverflow.msg());
            }
        }
        Ok(Value::Number(Number::F128(r)))
    }

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
        if r.is_infinite() || r.is_nan() {
            if fo_e {
                return Err(MathErr::FloatOverflow.msg());
            }
        }
        Ok(Value::Number(Number::F128(r)))
    }

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
        if r.is_infinite() || r.is_nan() {
            if fo_e {
                return Err(MathErr::FloatOverflow.msg());
            }
        }
        Ok(Value::Number(Number::F128(r)))
    }

    fn num_round(n: Number, prec: i32, rf: i8, fo_e: bool) -> Result<Number, String> {
        use self::Number::*;

        fn pow10_i128(k: u32) -> i128 {
            let mut r: i128 = 1;
            for _ in 0..k { r = r.saturating_mul(10); }
            r
        }

        if let Some(x) = Self::to_i128(&n) {
            if prec >= 0 {
                return Ok(n);
            }
            let abs_prec = (-prec) as u32;
            let factor = pow10_i128(abs_prec);
            if factor == 0 { return Err(MathErr::Overflow.msg()); }
            let sign = if x < 0 { -1i128 } else { 1i128 };
            let ax = x.abs();
            let rem = ax % factor;
            let next_digit = ((rem.saturating_mul(10)) / factor) as i32;
            let mut base = ax - rem;
            if next_digit >= rf as i32 { base += factor; }
            let out_i128 = base * sign;
            if let Some(res_num) = Self::from_i128_in_type(out_i128, x < 0, Self::int_info(&n).map(|(_,r)| r).unwrap_or(5)) {
                return Ok(res_num);
            }
            if out_i128 >= 0 {
                return Ok(Number::I128(out_i128));
            } else {
                return Ok(Number::I128(out_i128));
            }
        }

        let orig_kind = match n {
            F32(_) => OrigKind::F32,
            F64(_) => OrigKind::F64,
            F128(_) => OrigKind::F128,
            _ => OrigKind::Other,
        };

        let mut d: f64 = Self::to_f128_full(&n).to_string().parse().unwrap_or(0.0);
        if d.is_nan() {
            return Ok(Number::F64(0.0));
        }
        let sign = if d.is_sign_negative() { -1.0 } else { 1.0 };
        d = d.abs();
        let abs_prec = if prec >= 0 { prec } else { -prec };
        let pow10 = 10f64.powi(abs_prec);
        let scaled: f64 = if prec >= 0 { d * pow10 } else { d / pow10 };
        let scaled_x10 = (scaled * 10.0).floor();
        let next_digit = (scaled_x10 % 10.0) as i32;
        let mut base = (scaled_x10 / 10.0).floor();
        if next_digit >= rf as i32 { base += 1.0; }
        let mut result_abs = if prec >= 0 { base / pow10 } else { base * pow10 };
        result_abs *= sign;
        if result_abs.is_nan() || result_abs.is_infinite() {
            if fo_e { return Err(MathErr::FloatOverflow.msg()); }
        }

        match orig_kind {
            OrigKind::F32 => Ok(Number::F32(result_abs as f32)),
            OrigKind::F64 => Ok(Number::F64(result_abs)),
            OrigKind::F128 => Ok(Number::F128(Self::to_f128_full(&Number::F64(result_abs)))),
            _ => Ok(Number::F64(result_abs)),
        }
    }

    fn num_sin(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.sin())
    }

    fn num_cos(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.cos())
    }

    fn num_tan(n: Number) -> Number {
        let f = Self::to_f128_full(&n);
        Number::F128(f.tan())
    }

    fn num_ctg(n: Number) -> Result<Value, String> {
        let f = Self::to_f128_full(&n);
        let s = f.sin();
        if s.is_zero() {
            return Err(MathErr::DivisionByZero.msg());
        }
        Ok(Value::Number(Number::F128(f.ctg())))
    }

    fn num_abs(n: Number) -> Number {
        use self::Number::*;
        match n {
            I8(v)   => I8(v.abs()),
            I16(v)  => I16(v.abs()),
            I32(v)  => I32(v.abs()),
            I64(v)  => I64(v.abs()),
            I128(v) => I128(v.abs()),
            U8(v)   => U8(v),
            U16(v)  => U16(v),
            U32(v)  => U32(v),
            U64(v)  => U64(v),
            U128(v) => U128(v),
            F32(v)  => F32(v.abs()),
            F64(v)  => F64(v.abs()),
            F128(v) => Number::F128(if v.is_sign_negative() { -v } else { v }),
        }
    }

    fn str_mul_string_number(s: String, n: Number, _fo_e: bool) -> Result<Value, String> {
        use self::Number::*;
        match n {
            I8(v)   => Self::str_mul_by_count(&s, v as i128),
            I16(v)  => Self::str_mul_by_count(&s, v as i128),
            I32(v)  => Self::str_mul_by_count(&s, v as i128),
            I64(v)  => Self::str_mul_by_count(&s, v as i128),
            I128(v) => Self::str_mul_by_count(&s, v),
            U8(v)   => Self::str_mul_by_count(&s, v as i128),
            U16(v)  => Self::str_mul_by_count(&s, v as i128),
            U32(v)  => Self::str_mul_by_count(&s, v as i128),
            U64(v)  => {
                if v > i128::MAX as u64 { return Err(MathErr::Overflow.msg()); }
                Self::str_mul_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 { return Err(MathErr::Overflow.msg()); }
                Self::str_mul_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("умножение строки только на целое").msg()),
        }
    }

    fn str_mul_by_count(s: &str, count: i128) -> Result<Value, String> {
        if count < 0 {
            return Err(MathErr::DomainError("умножение строки на отрицательное число").msg());
        }
        let cnt = if count == 0 { 0 } else { count as usize };
        Ok(Value::String(s.repeat(cnt)))
    }

    fn str_div_string_number(s: String, n: Number, _fo_e: bool) -> Result<Value, String> {
        use self::Number::*;
        match n {
            I8(v)   => Self::str_div_by_count(&s, v as i128),
            I16(v)  => Self::str_div_by_count(&s, v as i128),
            I32(v)  => Self::str_div_by_count(&s, v as i128),
            I64(v)  => Self::str_div_by_count(&s, v as i128),
            I128(v) => Self::str_div_by_count(&s, v),
            U8(v)   => Self::str_div_by_count(&s, v as i128),
            U16(v)  => Self::str_div_by_count(&s, v as i128),
            U32(v)  => Self::str_div_by_count(&s, v as i128),
            U64(v)  => {
                if v > i128::MAX as u64 { return Err(MathErr::Overflow.msg()); }
                Self::str_div_by_count(&s, v as i128)
            }
            U128(v) => {
                if v > i128::MAX as u128 { return Err(MathErr::Overflow.msg()); }
                Self::str_div_by_count(&s, v as i128)
            }
            _ => Err(MathErr::TypeMismatch("деление строки только на целое").msg()),
        }
    }

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
            if rem > 0 { part_len += 1; rem -= 1; }
            let part: String = chars[idx..(idx + part_len)].iter().collect();
            parts.push(Value::String(part));
            idx += part_len;
        }
        Ok(Value::Array(parts))
    }

    fn str_div_string_delim(s: String, delim: String, _fo_e: bool) -> Result<Value, String> {
        if delim.is_empty() {
            return Err(MathErr::DomainError("разделитель не может быть пустой строкой").msg());
        }
        let parts: Vec<Value> = s.split(&delim).map(|p| Value::String(p.to_string())).collect();
        let splits = if parts.len() == 0 { 0 } else { parts.len() - 1 } as i64;
        Ok(Value::Pair(Box::new(Value::Array(parts)), Box::new(Value::Number(Number::I64(splits)))))
    }

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
                    if res >= i128::MIN && res <= i128::MAX {
                        Number::I128(res)
                    } else {
                        return Err(MathErr::Overflow.msg());
                    }
                } else {
                    if res >= 0 {
                        Number::U128(res as u128)
                    } else {
                        return Err(MathErr::Overflow.msg());
                    }
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

    fn is_effectively_integer(x: tF128) -> bool {
        let d: f64 = x.to_string().parse().unwrap_or(0.0);
        return d.fract() == 0.0;
    }
}