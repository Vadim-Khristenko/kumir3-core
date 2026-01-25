//! Функции таймера

use crate::shared::types::library::{LibFunctionDef, LibParamDef};
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::{Value, Number};

use super::datetime::{system_time_ms, system_time_sec};

/// таймер_старт() -> цел_64
pub fn timer_start_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_старт")
        .with_aliases(&["timer_start", "start_timer"])
        .with_description("Возвращает отметку времени (мс) для измерения интервалов")
        .returns(TypeSpec::Int64)
        .with_handler(|_args| {
            let ms = system_time_ms()?;
            Ok(Value::Number(Number::I64(ms)))
        })
}

/// таймер_прошло_мс(старт_мс: цел_64) -> цел_64
pub fn timer_elapsed_ms_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_прошло_мс")
        .with_aliases(&["timer_elapsed_ms"])
        .with_description("Считает, сколько миллисекунд прошло с переданной отметки")
        .with_param(LibParamDef::value("старт_мс", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let start = match args.get(0) {
                Some(Value::Number(n)) => n.to_i64().ok_or_else(|| "Ожидается целое значение".to_string())?,
                _ => return Err("Ожидается целое значение".to_string()),
            };
            let diff = elapsed_ms(start)?;
            Ok(Value::Number(Number::I64(diff)))
        })
}

/// таймер_прошло_сек(старт_сек: цел_64) -> цел_64
pub fn timer_elapsed_sec_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_прошло_сек")
        .with_aliases(&["timer_elapsed", "timer_elapsed_sec"])
        .with_description("Считает, сколько секунд прошло с переданной отметки")
        .with_param(LibParamDef::value("старт_сек", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let start = match args.get(0) {
                Some(Value::Number(n)) => n.to_i64().ok_or_else(|| "Ожидается целое значение".to_string())?,
                _ => return Err("Ожидается целое значение".to_string()),
            };
            let diff = elapsed_sec(start)?;
            Ok(Value::Number(Number::I64(diff)))
        })
}

/// Вычисляет прошедшее время в миллисекундах
pub fn elapsed_ms(start_ms: i64) -> Result<i64, String> {
    let now = system_time_ms()?;
    Ok((now - start_ms).max(0))
}

/// Вычисляет прошедшее время в секундах
pub fn elapsed_sec(start_sec: i64) -> Result<i64, String> {
    let now = system_time_sec()?;
    Ok((now - start_sec).max(0))
}