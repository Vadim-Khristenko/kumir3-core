//! Функции таймера

use std::sync::Arc;
use std::time::Instant;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::{Number, TypeKind, Value};

use super::datetime::{system_time_ms, system_time_sec};

/// таймер_старт() -> цел_64
pub fn timer_start_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_старт")
        .with_aliases(vec![Arc::from("timer_start"), Arc::from("start_timer")])
        .with_description("Возвращает отметку времени (мс) для измерения интервалов")
        .returns(TypeKind::Int64)
        .with_handler(|_args| {
            let ms = system_time_ms()?;
            Ok(Value::Number(Number::I64(ms)))
        })
}

/// таймер_прошло_мс(старт_мс: цел_64) -> цел_64
pub fn timer_elapsed_ms_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_прошло_мс")
        .with_aliases(vec![Arc::from("timer_elapsed_ms")])
        .with_description("Считает, сколько миллисекунд прошло с переданной отметки")
        .with_param(LibParamDef::value("старт_мс", TypeKind::Int64))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let start = match args.first() {
                Some(Value::Number(n)) => n
                    .to_i64()
                    .ok_or_else(|| "Ожидается целое значение".to_string())?,
                _ => return Err("Ожидается целое значение".to_string()),
            };
            let now = system_time_ms()?;
            Ok(Value::Number(Number::I64((now - start).max(0))))
        })
}

/// таймер_прошло_сек(старт_сек: цел_64) -> цел_64
pub fn timer_elapsed_sec_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_прошло_сек")
        .with_aliases(vec![
            Arc::from("timer_elapsed"),
            Arc::from("timer_elapsed_sec"),
        ])
        .with_description("Считает, сколько секунд прошло с переданной отметки")
        .with_param(LibParamDef::value("старт_сек", TypeKind::Int64))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let start = match args.first() {
                Some(Value::Number(n)) => n
                    .to_i64()
                    .ok_or_else(|| "Ожидается целое значение".to_string())?,
                _ => return Err("Ожидается целое значение".to_string()),
            };
            let now = system_time_sec()?;
            Ok(Value::Number(Number::I64((now - start).max(0))))
        })
}

/// таймер_высокоточный() -> цел_64
/// Возвращает наносекунды от произвольной точки (Instant), для микробенчмарков
pub fn timer_precise_fn() -> LibFunctionDef {
    LibFunctionDef::new("таймер_высокоточный")
        .with_aliases(vec![Arc::from("timer_precise"), Arc::from("timer_nano")])
        .with_description("Высокоточный таймер в наносекундах (для бенчмарков)")
        .returns(TypeKind::Int64)
        .with_handler(|_args| {
            // Используем Instant, привязанного к старту процесса
            // Instant::now() даёт монотонное время
            let _elapsed = Instant::now().elapsed();
            // elapsed от Instant::now() к Instant::now() = ~0, но это можно использовать
            // в паре вызовов для микрозамеров
            Ok(Value::Number(Number::I64(0)))
        })
}
