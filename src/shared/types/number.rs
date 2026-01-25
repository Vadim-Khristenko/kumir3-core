//! Числовые типы языка Кумир
//! 
//! Поддерживаемые типы соответствуют типам Кумира:
//! - `цел` / `цел_64` → I64 (по умолчанию)
//! - `малое_цел` / `цел_32` → I32
//! - `большое_цел` / `цел_128` → I128
//! - `вещ` → F64 (по умолчанию)
//! - `малое_вещ` → F32
//! - `большое_вещ` → F128 (высокая точность)

use crate::shared::f128::F128;

/// Универсальное представление чисел в Value.
#[derive(Debug, Clone, PartialEq)]
pub enum Number {
    // Целые знаковые
    I8(i8),       // цел_8
    I16(i16),     // цел_16
    I32(i32),     // цел_32, малое_цел
    I64(i64),     // цел_64, цел (по умолчанию)
    I128(i128),   // цел_128, большое_цел

    // Целые беззнаковые
    U8(u8),       // нат_8
    U16(u16),     // нат_16
    U32(u32),     // нат_32
    U64(u64),     // нат_64
    U128(u128),   // нат_128

    // Вещественные
    F32(f32),     // вещ_32, малое_вещ
    F64(f64),     // вещ_64, вещ (по умолчанию)
    F128(F128),   // вещ_128, большое_вещ
}

impl Number {
    pub fn to_string(&self) -> String {
        match self {
            Number::I8(v) => v.to_string(),
            Number::I16(v) => v.to_string(),
            Number::I32(v) => v.to_string(),
            Number::I64(v) => v.to_string(),
            Number::I128(v) => v.to_string(),
            Number::U8(v) => v.to_string(),
            Number::U16(v) => v.to_string(),
            Number::U32(v) => v.to_string(),
            Number::U64(v) => v.to_string(),
            Number::U128(v) => v.to_string(),
            Number::F32(x) => x.to_string(),
            Number::F64(x) => x.to_string(),
            Number::F128(x) => x.to_string(),
        }
    }

    /// Преобразует в i64 (если возможно без потери значащих цифр)
    pub fn to_i64(&self) -> Option<i64> {
        match self {
            Number::I8(v) => Some(*v as i64),
            Number::I16(v) => Some(*v as i64),
            Number::I32(v) => Some(*v as i64),
            Number::I64(v) => Some(*v),
            Number::I128(v) => i64::try_from(*v).ok(),
            Number::U8(v) => Some(*v as i64),
            Number::U16(v) => Some(*v as i64),
            Number::U32(v) => Some(*v as i64),
            Number::U64(v) => i64::try_from(*v).ok(),
            Number::U128(v) => i64::try_from(*v).ok(),
            Number::F32(v) => Some(*v as i64),
            Number::F64(v) => Some(*v as i64),
            Number::F128(v) => Some(v.to_f64() as i64),
        }
    }

    /// Преобразует в f64
    pub fn to_f64(&self) -> Option<f64> {
        match self {
            Number::I8(v) => Some(*v as f64),
            Number::I16(v) => Some(*v as f64),
            Number::I32(v) => Some(*v as f64),
            Number::I64(v) => Some(*v as f64),
            Number::I128(v) => Some(*v as f64),
            Number::U8(v) => Some(*v as f64),
            Number::U16(v) => Some(*v as f64),
            Number::U32(v) => Some(*v as f64),
            Number::U64(v) => Some(*v as f64),
            Number::U128(v) => Some(*v as f64),
            Number::F32(v) => Some(*v as f64),
            Number::F64(v) => Some(*v),
            Number::F128(v) => Some(v.to_f64()),
        }
    }
}

// --- Macros for boilerplate implementations ---

macro_rules! impl_from_number {
    ($($t:ty => $v:ident),+ $(,)?) => {
        $(
            impl From<$t> for Number { 
                fn from(v: $t) -> Self { Number::$v(v) } 
            }
        )+
    };
}

impl_from_number!(
    i8 => I8, i16 => I16, i32 => I32, i64 => I64, i128 => I128,
    u8 => U8, u16 => U16, u32 => U32, u64 => U64, u128 => U128,
    f32 => F32, f64 => F64, F128 => F128
);
