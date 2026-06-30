//! Встроенные функции интерпретатора Кумир 3
//!
//! Реализация стандартных функций: математика, строки, массивы,
//! ввод/вывод, работа с типами и т.д.

use std::time::{SystemTime, UNIX_EPOCH};

use shared::types::{Expr, Number, Value};

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::evaluator::ExprEvaluator;

pub mod array;
pub mod io;
pub mod math;
pub mod string;
pub mod types;

/// Встроенные функции.
pub struct Builtins;

impl Builtins {
    /// Пытается вызвать встроенную функцию.
    /// Возвращает Some(value) если функция найдена, None если нет.
    pub fn try_call(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        if let Some(v) = Self::try_call_math(name, args, env)? {
            return Ok(Some(v));
        }
        if let Some(v) = Self::try_call_string(name, args, env)? {
            return Ok(Some(v));
        }
        if let Some(v) = Self::try_call_array(name, args, env)? {
            return Ok(Some(v));
        }
        if let Some(v) = Self::try_call_type(name, args, env)? {
            return Ok(Some(v));
        }
        if let Some(v) = Self::try_call_io(name, args, env)? {
            return Ok(Some(v));
        }
        Ok(None)
    }

    /// Вычисляет аргументы вызова.
    fn eval_args(args: &[Expr], env: &mut Environment) -> RuntimeResult<Vec<Value>> {
        args.iter()
            .map(|e| ExprEvaluator::evaluate(e, env))
            .collect()
    }

    // =========================================================================
    //                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
    // =========================================================================

    fn check_args(name: &str, args: &[Value], expected: usize) -> RuntimeResult<()> {
        if args.len() != expected {
            Err(RuntimeError::argument_count(name, expected, args.len()))
        } else {
            Ok(())
        }
    }

    fn to_f64(value: &Value) -> RuntimeResult<f64> {
        match value {
            Value::Number(n) => n
                .to_f64()
                .ok_or_else(|| RuntimeError::type_mismatch("число", "не число")),
            Value::String(s) => s
                .parse::<f64>()
                .map_err(|_| RuntimeError::type_mismatch("число", "строка")),
            _ => Err(RuntimeError::type_mismatch("число", "другой тип")),
        }
    }

    fn to_int(value: &Value) -> RuntimeResult<i64> {
        match value {
            Value::Number(n) => n
                .to_i64()
                .ok_or_else(|| RuntimeError::type_mismatch("целое", "не целое")),
            Value::String(s) => s
                .parse::<i64>()
                .map_err(|_| RuntimeError::type_mismatch("целое", "строка")),
            Value::Boolean(b) => Ok(if *b { 1 } else { 0 }),
            _ => Err(RuntimeError::type_mismatch("целое", "другой тип")),
        }
    }

    fn abs(value: &Value) -> RuntimeResult<Value> {
        match value {
            Value::Number(n) => {
                let result = match n {
                    Number::I8(v) => Number::I8(v.abs()),
                    Number::I16(v) => Number::I16(v.abs()),
                    Number::I32(v) => Number::I32(v.abs()),
                    Number::I64(v) => Number::I64(v.abs()),
                    Number::I128(v) => Number::I128(v.abs()),
                    Number::U8(v) => Number::U8(*v),
                    Number::U16(v) => Number::U16(*v),
                    Number::U32(v) => Number::U32(*v),
                    Number::U64(v) => Number::U64(*v),
                    Number::U128(v) => Number::U128(*v),
                    Number::F32(v) => Number::F32(v.abs()),
                    Number::F64(v) => Number::F64(v.abs()),
                    Number::F128(v) => Number::F128(v.abs()),
                };
                Ok(Value::Number(result))
            }
            _ => Err(RuntimeError::type_mismatch("число", "не число")),
        }
    }

    fn trig_func<F>(value: &Value, f: F) -> RuntimeResult<Value>
    where
        F: Fn(f64) -> f64,
    {
        let x = Self::to_f64(value)?;
        Ok(Value::Number(Number::F64(f(x))))
    }

    fn compare_values(a: &Value, b: &Value) -> RuntimeResult<i32> {
        match (a, b) {
            (Value::Number(na), Value::Number(nb)) => {
                let fa = na.to_f64().unwrap_or(0.0);
                let fb = nb.to_f64().unwrap_or(0.0);
                Ok(fa.partial_cmp(&fb).map(|o| o as i32).unwrap_or(0))
            }
            (Value::String(sa), Value::String(sb)) => Ok(sa.cmp(sb) as i32),
            _ => Err(RuntimeError::type_mismatch("сравнимые типы", "несравнимые")),
        }
    }

    fn type_name(value: &Value) -> String {
        match value {
            Value::Number(Number::I64(_)) => "цел".to_string(),
            Value::Number(Number::F64(_)) => "вещ".to_string(),
            Value::Number(_) => "число".to_string(),
            Value::String(_) => "лит".to_string(),
            Value::Boolean(_) => "лог".to_string(),
            Value::Char(_) => "сим".to_string(),
            Value::Array(_) => "таб".to_string(),
            Value::Range { .. } => "диапазон".to_string(),
            Value::Bytes(_) => "байты".to_string(),
            Value::Pair(_, _) => "пара".to_string(),
            Value::Triple(_, _, _) => "тройка".to_string(),
            Value::Tuple(_) => "кортеж".to_string(),
            Value::Set(_) => "множество".to_string(),
            Value::Map(_) => "словарь".to_string(),
            Value::Option(_) => "опция".to_string(),
            Value::Result(_) => "результат".to_string(),
            Value::Pointer(_) => "указатель".to_string(),
            Value::Enum { .. } => "перечисление".to_string(),
            Value::Object { .. } => "объект".to_string(),
            Value::NativeObject { type_name, .. } => type_name.clone(),
            Value::Promise { .. } => "промис".to_string(),
            Value::Reference { .. } => "ссылка".to_string(),
            Value::Lambda(_) => "лямбда".to_string(),
            Value::PartialApp { .. } => "частичное_применение".to_string(),
            Value::Generator { .. } => "генератор".to_string(),
            Value::Channel { .. } => "канал".to_string(),
            Value::Error { .. } => "ошибка".to_string(),
            Value::Type(_) => "тип".to_string(),
            Value::Null => "пусто".to_string(),
            Value::Undefined => "неопределено".to_string(),
        }
    }

    // Простой генератор псевдослучайных чисел
    fn simple_random() -> f64 {
        use std::cell::Cell;
        use std::time::{SystemTime, UNIX_EPOCH};

        thread_local! {
            static SEED: Cell<u64> = Cell::new(
                SystemTime::now()
                    .duration_since(UNIX_EPOCH)
                    .unwrap_or_default()
                    .as_nanos() as u64
            );
        }

        SEED.with(|seed| {
            let mut s = seed.get();
            s ^= s << 13;
            s ^= s >> 17;
            s ^= s << 5;
            seed.set(s);
            (s as f64) / (u64::MAX as f64)
        })
    }
}
