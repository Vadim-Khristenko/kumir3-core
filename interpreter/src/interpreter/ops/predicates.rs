//! Предикаты и сравнения над значениями (истинность, равенство, упорядочение, индексация).
//!
//! Разделение ответственности — как в [`super::binary`] и [`super::cast`]:
//! * **движок типов** (`shared::typesys`) — авторитет по *типизации*: определено
//!   ли сравнение для пары типов операндов;
//! * **код ниже** — вычислительное ядро: как именно упорядочить два значения.
//!
//! Движок различает равенство и упорядочение (KITE 13 § 3.8):
//! * `=` / `<>` ([`TypeOp::Eq`], [`TypeOp::Ne`]) — **тотальны**: определены для
//!   любой пары типов, результат `лог`, типовой ошибки не бывает;
//! * `<` `<=` `>` `>=` — требуют *упорядоченного* типа с обеих сторон (число,
//!   `лит`, `сим`) и взаимной совместимости, поэтому `да < нет`, `таб < таб` и
//!   `сим < лит` — типовые ошибки.
//!
//! Числа сравниваются **без потери точности**: целые — как целые (в `i128`,
//! а `нат_128` сверх `i128::MAX` — как `u128`), смешанные и вещественные пары —
//! в `вещ_128` (113 значащих бит), куда `вещ_32`, `вещ_64` и любое 64-битное
//! целое вкладываются точно. Прежняя реализация приводила ОБА операнда к `f64`
//! (53 значащих бита), из-за чего различные `цел_64`/`цел_128`/`нат_64` и
//! близкие `вещ_128` оказывались «равными».

use std::cmp::Ordering;

use shared::f128::F128;
use shared::types::{Number, Value};
use shared::typesys::{TypeOp, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

/// Числовое значение в форме, пригодной для сравнения без потерь.
enum Numeric {
    /// Целое, представимое в `i128` — это все целые типы языка, кроме `нат_128`
    /// со значением больше `i128::MAX`.
    Int(i128),
    /// `нат_128` больше `i128::MAX`: знаковый тип его не вмещает.
    Big(u128),
    /// Вещественное; `вещ_32` и `вещ_64` вкладываются в `вещ_128` точно.
    Real(F128),
}

impl TypeOps {
    /// Проверяет "истинность" значения.
    pub fn is_truthy(value: &Value) -> bool {
        match value {
            Value::Boolean(b) => *b,
            Value::Number(n) => n.to_f64().map(|f| f != 0.0).unwrap_or(false),
            Value::String(s) => !s.is_empty(),
            Value::Array(a) => !a.is_empty(),
            Value::Null | Value::Undefined => false,
            Value::Option(opt) => opt.is_some(),
            _ => true,
        }
    }

    /// Сравнивает два значения на равенство.
    ///
    /// Равенство **тотально**: движок ([`TypeOp::Eq`]) определяет его для любой
    /// пары типов и никогда не возвращает ошибку, поэтому вердикт здесь не
    /// запрашивается — спрашивать не о чем. Тотальность зафиксирована тестом
    /// `char_engine_equality_is_total_for_every_type_pair`.
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        a == b
    }

    /// Сравнивает два значения (`<`, `<=`, `>`, `>=`).
    pub fn compare<F>(a: &Value, b: &Value, cmp: F) -> RuntimeResult<Value>
    where
        F: Fn(Ordering) -> bool,
    {
        // [typesys-seam: подключён] типовой вердикт движка — до сравнения.
        Self::check_ordered(a, b)?;
        let ordering = Self::order(a, b).ok_or_else(Self::incomparable)?;
        Ok(Value::Boolean(cmp(ordering)))
    }

    /// Спрашивает движок, определено ли упорядочение для типов операндов.
    ///
    /// У всех четырёх операторов упорядочения одно правило типизации, поэтому
    /// достаточно спросить про [`TypeOp::Lt`]. Точное имя оператора в
    /// диагностике обеспечивает [`TypeOps::binary`]: он спрашивает движок
    /// раньше и уже с тем оператором, который написан в программе; сообщение
    /// отсюда видно только при прямом вызове `compare`.
    fn check_ordered(a: &Value, b: &Value) -> RuntimeResult<()> {
        default_engine()
            .result_of_binop(TypeOp::Lt, &a.type_kind(), &b.type_kind())
            .map(|_| ())
            .map_err(|err| RuntimeError::new(err.to_string(), RuntimeErrorKind::TypeMismatch))
    }

    /// Диагностика для пары, которую движок пропустил, а ядро упорядочить не
    /// умеет. Недостижима: множество упорядоченных типов движка (число, `лит`,
    /// `сим`) совпадает с набором ветвей [`TypeOps::order`].
    fn incomparable() -> RuntimeError {
        RuntimeError::type_mismatch("сравнимые типы", "несравнимые типы")
    }

    /// Вычислительное ядро: порядок двух уже разрешённых движком значений.
    fn order(a: &Value, b: &Value) -> Option<Ordering> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => Some(Self::order_numbers(na, nb)),
            (Value::String(sa), Value::String(sb)) => Some(sa.cmp(sb)),
            (Value::Char(ca), Value::Char(cb)) => Some(ca.cmp(cb)),
            _ => None,
        }
    }

    /// Порядок двух чисел без промежуточного округления до `f64`.
    ///
    /// `NaN` сохраняет прежнее поведение: несравнимую пару считаем равной
    /// (`partial_cmp` → `None` → [`Ordering::Equal`]).
    fn order_numbers(a: &Number, b: &Number) -> Ordering {
        match (Self::classify(a), Self::classify(b)) {
            (Numeric::Int(x), Numeric::Int(y)) => x.cmp(&y),
            (Numeric::Big(x), Numeric::Big(y)) => x.cmp(&y),
            // `Big` по определению больше `i128::MAX`, а значит и любого `Int`.
            (Numeric::Int(_), Numeric::Big(_)) => Ordering::Less,
            (Numeric::Big(_), Numeric::Int(_)) => Ordering::Greater,
            (x, y) => Self::widen(x)
                .partial_cmp(&Self::widen(y))
                .unwrap_or(Ordering::Equal),
        }
    }

    /// Раскладывает число на точную форму для сравнения.
    fn classify(n: &Number) -> Numeric {
        match n {
            Number::I8(v) => Numeric::Int(*v as i128),
            Number::I16(v) => Numeric::Int(*v as i128),
            Number::I32(v) => Numeric::Int(*v as i128),
            Number::I64(v) => Numeric::Int(*v as i128),
            Number::I128(v) => Numeric::Int(*v),
            Number::U8(v) => Numeric::Int(*v as i128),
            Number::U16(v) => Numeric::Int(*v as i128),
            Number::U32(v) => Numeric::Int(*v as i128),
            Number::U64(v) => Numeric::Int(*v as i128),
            Number::U128(v) => match i128::try_from(*v) {
                Ok(i) => Numeric::Int(i),
                Err(_) => Numeric::Big(*v),
            },
            Number::F32(v) => Numeric::Real(F128::from(*v)),
            Number::F64(v) => Numeric::Real(F128::from(*v)),
            Number::F128(v) => Numeric::Real(*v),
        }
    }

    /// Приводит любую форму к самому широкому представлению — `вещ_128`.
    ///
    /// Целые до 113 значащих бит переводятся точно; только `цел_128`/`нат_128`
    /// сверх этого предела округляются (но и тогда — на 60 бит точнее, чем при
    /// прежнем приведении к `f64`).
    fn widen(n: Numeric) -> F128 {
        match n {
            Numeric::Real(f) => f,
            Numeric::Int(i) => Self::int_to_real(i.unsigned_abs(), i < 0),
            Numeric::Big(u) => Self::int_to_real(u, false),
        }
    }

    /// Строит `вещ_128` из знака и модуля целого, разложив модуль на две
    /// 64-битные половины (обе переводятся в `вещ_128` точно).
    fn int_to_real(magnitude: u128, negative: bool) -> F128 {
        /// 2^64 — точно представимо и в `f64`, и в `вещ_128`.
        const TWO_POW_64: f64 = 18_446_744_073_709_551_616.0;

        let low = F128::from(magnitude as u64);
        let high = (magnitude >> 64) as u64;
        let value = if high == 0 {
            low
        } else {
            F128::from(high) * F128::from(TWO_POW_64) + low
        };
        if negative { -value } else { value }
    }

    /// Преобразует значение в индекс.
    pub fn to_index(value: &Value) -> RuntimeResult<i64> {
        value
            .as_int()
            .ok_or_else(|| RuntimeError::type_mismatch("целое число", "не целое"))
    }
}
