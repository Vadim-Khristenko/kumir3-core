//! Функции паузы выполнения

use std::time::Duration;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::type_spec::TypeSpec;
use crate::types::Value;

/// пауза(сек: нат_64)
pub fn sleep_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза")
        .with_aliases(&["sleep", "sleep_sec", "пауза_сек"])
        .with_description("Блокирует выполнение на указанное число секунд")
        .with_param(LibParamDef::value("сек", TypeSpec::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.get(0) {
                if let Some(v) = n.to_i64() {
                    if v >= 0 {
                        do_sleep_sec(v as u64);
                        return Ok(Value::Null);
                    }
                }
            }
            Err("Ожидается неотрицательное целое число секунд".to_string())
        })
}

/// пауза_мс(мс: нат_64)
pub fn sleep_ms_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза_мс")
        .with_aliases(&["sleep_ms"])
        .with_description("Блокирует выполнение на указанное число миллисекунд")
        .with_param(LibParamDef::value("мс", TypeSpec::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.get(0) {
                if let Some(v) = n.to_i64() {
                    if v >= 0 {
                        do_sleep_ms(v as u64);
                        return Ok(Value::Null);
                    }
                }
            }
            Err("Ожидается неотрицательное целое число миллисекунд".to_string())
        })
}

/// пауза_мин(мин: нат_64)
pub fn sleep_min_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза_мин")
        .with_aliases(&["sleep_min"])
        .with_description("Блокирует выполнение на указанное число минут")
        .with_param(LibParamDef::value("мин", TypeSpec::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.get(0) {
                if let Some(v) = n.to_i64() {
                    if v >= 0 {
                        do_sleep_min(v as u64);
                        return Ok(Value::Null);
                    }
                }
            }
            Err("Ожидается неотрицательное целое число минут".to_string())
        })
}

/// Выполняет паузу на указанное количество секунд
pub fn do_sleep_sec(secs: u64) {
    std::thread::sleep(Duration::from_secs(secs));
}

/// Выполняет паузу на указанное количество миллисекунд
pub fn do_sleep_ms(ms: u64) {
    std::thread::sleep(Duration::from_millis(ms));
}

/// Выполняет паузу на указанное количество минут
pub fn do_sleep_min(mins: u64) {
    std::thread::sleep(Duration::from_secs(mins.saturating_mul(60)));
}