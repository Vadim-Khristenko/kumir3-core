//! Функции паузы выполнения

use std::sync::Arc;
use std::time::Duration;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::{TypeKind, Value};

/// пауза(сек: нат_64)
pub fn sleep_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза")
        .with_aliases(vec![
            Arc::from("sleep"),
            Arc::from("sleep_sec"),
            Arc::from("пауза_сек"),
        ])
        .with_description("Блокирует выполнение на указанное число секунд")
        .with_param(LibParamDef::value("сек", TypeKind::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.first()
                && let Some(v) = n.to_i64()
                && v >= 0
            {
                std::thread::sleep(Duration::from_secs(v as u64));
                return Ok(Value::Null);
            }
            Err("Ожидается неотрицательное целое число секунд".to_string())
        })
}

/// пауза_мс(мс: нат_64)
pub fn sleep_ms_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза_мс")
        .with_aliases(vec![Arc::from("sleep_ms")])
        .with_description("Блокирует выполнение на указанное число миллисекунд")
        .with_param(LibParamDef::value("мс", TypeKind::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.first()
                && let Some(v) = n.to_i64()
                && v >= 0
            {
                std::thread::sleep(Duration::from_millis(v as u64));
                return Ok(Value::Null);
            }
            Err("Ожидается неотрицательное целое число миллисекунд".to_string())
        })
}

/// пауза_мин(мин: нат_64)
pub fn sleep_min_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза_мин")
        .with_aliases(vec![Arc::from("sleep_min")])
        .with_description("Блокирует выполнение на указанное число минут")
        .with_param(LibParamDef::value("мин", TypeKind::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.first()
                && let Some(v) = n.to_i64()
                && v >= 0
            {
                std::thread::sleep(Duration::from_secs((v as u64).saturating_mul(60)));
                return Ok(Value::Null);
            }
            Err("Ожидается неотрицательное целое число минут".to_string())
        })
}

/// пауза_мкс(мкс: нат_64)
pub fn sleep_us_fn() -> LibFunctionDef {
    LibFunctionDef::new("пауза_мкс")
        .with_aliases(vec![Arc::from("sleep_us"), Arc::from("sleep_micros")])
        .with_description("Блокирует выполнение на указанное число микросекунд")
        .with_param(LibParamDef::value("мкс", TypeKind::UInt64))
        .as_procedure()
        .with_handler(|args| {
            if let Some(Value::Number(n)) = args.first()
                && let Some(v) = n.to_i64()
                && v >= 0
            {
                std::thread::sleep(Duration::from_micros(v as u64));
                return Ok(Value::Null);
            }
            Err("Ожидается неотрицательное число микросекунд".to_string())
        })
}
