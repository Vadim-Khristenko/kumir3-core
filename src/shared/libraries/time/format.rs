//! Функции форматирования даты и времени

use std::collections::BTreeMap;

use crate::shared::types::library::{LibFunctionDef, LibParamDef};
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::{Value, Number};

use super::constants::*;
use super::datetime::{DateTimeParts, timestamp_to_parts, parts_to_timestamp, system_time_sec};

// ============================================================================
// ФОРМАТИРОВАНИЕ
// ============================================================================

/// Форматирует дату-время в ISO 8601 UTC формат
pub fn format_iso_utc(parts: &DateTimeParts) -> String {
    format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
        parts.year, parts.month, parts.day, parts.hour, parts.minute, parts.second
    )
}

/// Форматирует только дату в ISO формат
pub fn format_date_iso(parts: &DateTimeParts) -> String {
    format!("{:04}-{:02}-{:02}", parts.year, parts.month, parts.day)
}

/// Форматирует только время в ISO формат
pub fn format_time_iso(parts: &DateTimeParts) -> String {
    format!("{:02}:{:02}:{:02}", parts.hour, parts.minute, parts.second)
}

/// Форматирует дату по-русски: "5 декабря 2024 г."
pub fn format_date_ru(parts: &DateTimeParts) -> String {
    let month_name = MONTH_RU_LONG
        .get((parts.month - 1) as usize)
        .unwrap_or(&"???");
    format!("{} {} {} г.", parts.day, month_name, parts.year)
}

/// Форматирует дату-время по-русски
pub fn format_datetime_ru(parts: &DateTimeParts) -> String {
    format!(
        "{} {:02}:{:02}:{:02}",
        format_date_ru(parts), parts.hour, parts.minute, parts.second
    )
}

// ============================================================================
// ПАРСИНГ
// ============================================================================

/// Парсит последовательность ASCII-цифр в число
pub fn parse_digits(slice: &[u8]) -> Option<u32> {
    if slice.is_empty() {
        return None;
    }
    let mut value: u32 = 0;
    for &b in slice {
        if !b.is_ascii_digit() {
            return None;
        }
        value = value.checked_mul(10)?.checked_add((b - b'0') as u32)?;
    }
    Some(value)
}

/// Парсит ISO 8601 строку в компоненты
pub fn parse_iso(s: &str) -> Result<(i32, u8, u8, u8, u8, u8), String> {
    let s = s.trim();
    let bytes = s.as_bytes();

    if bytes.len() < 19 {
        return Err(format!("Слишком короткая строка даты: '{}'", s));
    }

    let len = bytes.len();
    if len != 19 && !(len == 20 && (bytes[19] == b'Z' || bytes[19] == b'z')) {
        return Err(format!("Некорректная длина строки даты: '{}'", s));
    }

    if bytes[4] != b'-' || bytes[7] != b'-' {
        return Err("Неверный разделитель даты".to_string());
    }
    if bytes[10] != b'T' && bytes[10] != b't' && bytes[10] != b' ' {
        return Err("Неверный разделитель даты/времени".to_string());
    }
    if bytes[13] != b':' || bytes[16] != b':' {
        return Err("Неверный разделитель времени".to_string());
    }

    let year = parse_digits(&bytes[0..4]).ok_or("Год должен быть числом")? as i32;
    let month = parse_digits(&bytes[5..7]).ok_or("Месяц должен быть числом")? as u8;
    let day = parse_digits(&bytes[8..10]).ok_or("День должен быть числом")? as u8;
    let hour = parse_digits(&bytes[11..13]).ok_or("Часы должны быть числом")? as u8;
    let minute = parse_digits(&bytes[14..16]).ok_or("Минуты должны быть числом")? as u8;
    let second = parse_digits(&bytes[17..19]).ok_or("Секунды должны быть числом")? as u8;

    Ok((year, month, day, hour, minute, second))
}

fn parts_to_value_map(parts: &DateTimeParts) -> Value {
    let mut map = BTreeMap::new();
    map.insert(Value::String("year".into()), Value::Number(Number::I32(parts.year)));
    map.insert(Value::String("month".into()), Value::Number(Number::U8(parts.month)));
    map.insert(Value::String("day".into()), Value::Number(Number::U8(parts.day)));
    map.insert(Value::String("hour".into()), Value::Number(Number::U8(parts.hour)));
    map.insert(Value::String("minute".into()), Value::Number(Number::U8(parts.minute)));
    map.insert(Value::String("second".into()), Value::Number(Number::U8(parts.second)));
    map.insert(Value::String("weekday".into()), Value::Number(Number::U8(parts.weekday)));
    map.insert(Value::String("yearday".into()), Value::Number(Number::U16(parts.yearday)));
    map.insert(Value::String("iso".into()), Value::String(format_iso_utc(parts)));
    Value::Map(map)
}

fn expect_number(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
    let v = args.get(idx).ok_or_else(|| format!("Не передан параметр: {}", what))?;
    match v {
        Value::Number(n) => n.to_i64().ok_or_else(|| format!("Ожидается целое: {}", what)),
        _ => Err(format!("Ожидается число для параметра {}", what)),
    }
}

// ============================================================================
// ОПРЕДЕЛЕНИЯ ФУНКЦИЙ
// ============================================================================

/// дата_время_iso_utc() -> лит
pub fn iso_utc_fn() -> LibFunctionDef {
    LibFunctionDef::new("дата_время_iso_utc")
        .with_aliases(&["iso_utc", "datetime_utc_iso"])
        .with_description("Текущая дата-время UTC в ISO 8601 (YYYY-MM-DDTHH:MM:SSZ)")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::String(format_iso_utc(&parts)))
        })
}

/// дата_время_iso() -> лит
pub fn iso_local_fn() -> LibFunctionDef {
    LibFunctionDef::new("дата_время_iso")
        .with_aliases(&["iso", "datetime_iso"])
        .with_description("Текущая дата-время в ISO 8601 (используется UTC)")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::String(format_iso_utc(&parts)))
        })
}

/// текущая_дата_рус() -> лит
pub fn date_ru_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущая_дата_рус")
        .with_aliases(&["date_ru", "current_date_ru"])
        .with_description("Текущая дата в русском формате: \"5 декабря 2024 г.\"")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::String(format_date_ru(&parts)))
        })
}

/// текущее_время_строка() -> лит
pub fn time_str_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущее_время_строка")
        .with_aliases(&["time_str", "current_time_str"])
        .with_description("Текущее время в формате HH:MM:SS")
        .returns(TypeSpec::String)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::String(format_time_iso(&parts)))
        })
}

/// из_штампа_сек(сек: цел_64) -> лит
pub fn from_timestamp_fn() -> LibFunctionDef {
    LibFunctionDef::new("из_штампа_сек")
        .with_aliases(&["from_timestamp", "from_ts"])
        .with_description("Преобразует секунды от эпохи в ISO-дату UTC")
        .with_param(LibParamDef::value("сек", TypeSpec::Int64))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let secs = expect_number(args, 0, "сек")?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::String(format_iso_utc(&parts)))
        })
}

/// в_штамп_сек(iso: лит) -> цел_64
pub fn to_timestamp_fn() -> LibFunctionDef {
    LibFunctionDef::new("в_штамп_сек")
        .with_aliases(&["to_timestamp", "ts"])
        .with_description("Преобразует ISO-дату UTC в секунды от эпохи")
        .with_param(LibParamDef::value("iso", TypeSpec::String))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let iso = match args.get(0) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("Ожидается строка ISO".to_string()),
            };
            let (y, m, d, h, min, s) = parse_iso(&iso)?;
            let ts = parts_to_timestamp(y, m, d, h, min, s)?;
            Ok(Value::Number(Number::I64(ts)))
        })
}

/// разобрать_штамп_сек(сек: цел_64) -> словарь
pub fn split_timestamp_fn() -> LibFunctionDef {
    LibFunctionDef::new("разобрать_штамп_сек")
        .with_aliases(&["ts_parts", "split_timestamp"])
        .with_description("Разбивает штамп секунд на компоненты (год, месяц, день, час, минута, секунда)")
        .with_param(LibParamDef::value("сек", TypeSpec::Int64))
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::Auto)))
        .with_handler(|args| {
            let secs = expect_number(args, 0, "сек")?;
            let parts = timestamp_to_parts(secs)?;
            Ok(parts_to_value_map(&parts))
        })
}

/// разобрать_iso(iso: лит) -> словарь
pub fn split_iso_fn() -> LibFunctionDef {
    LibFunctionDef::new("разобрать_iso")
        .with_aliases(&["iso_parts", "split_iso"])
        .with_description("Парсит ISO строку и возвращает словарь компонентов")
        .with_param(LibParamDef::value("iso", TypeSpec::String))
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::Auto)))
        .with_handler(|args| {
            let iso = match args.get(0) {
                Some(Value::String(s)) => s.clone(),
                _ => return Err("Ожидается строка ISO".to_string()),
            };
            let (y, m, d, h, min, s) = parse_iso(&iso)?;
            let ts = parts_to_timestamp(y, m, d, h, min, s)?;
            let parts = timestamp_to_parts(ts)?;
            Ok(parts_to_value_map(&parts))
        })
}

/// создать_штамп(год, месяц, день, час, минута, секунда) -> цел_64
pub fn make_timestamp_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_штамп")
        .with_aliases(&["make_timestamp", "create_timestamp"])
        .with_description("Создаёт timestamp из компонентов даты-времени")
        .with_param(LibParamDef::value("год", TypeSpec::Int32))
        .with_param(LibParamDef::value("месяц", TypeSpec::UInt8))
        .with_param(LibParamDef::value("день", TypeSpec::UInt8))
        .with_param(LibParamDef::value("час", TypeSpec::UInt8))
        .with_param(LibParamDef::value("минута", TypeSpec::UInt8))
        .with_param(LibParamDef::value("секунда", TypeSpec::UInt8))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let y = expect_number(args, 0, "год")? as i32;
            let m = expect_number(args, 1, "месяц")? as u8;
            let d = expect_number(args, 2, "день")? as u8;
            let h = expect_number(args, 3, "час")? as u8;
            let min = expect_number(args, 4, "минута")? as u8;
            let s = expect_number(args, 5, "секунда")? as u8;
            let ts = parts_to_timestamp(y, m, d, h, min, s)?;
            Ok(Value::Number(Number::I64(ts)))
        })
}

/// создать_штамп_дата(год, месяц, день) -> цел_64
pub fn make_date_fn() -> LibFunctionDef {
    LibFunctionDef::new("создать_штамп_дата")
        .with_aliases(&["make_date", "date_to_timestamp"])
        .with_description("Создаёт timestamp из даты (время = 00:00:00)")
        .with_param(LibParamDef::value("год", TypeSpec::Int32))
        .with_param(LibParamDef::value("месяц", TypeSpec::UInt8))
        .with_param(LibParamDef::value("день", TypeSpec::UInt8))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let y = expect_number(args, 0, "год")? as i32;
            let m = expect_number(args, 1, "месяц")? as u8;
            let d = expect_number(args, 2, "день")? as u8;
            let ts = parts_to_timestamp(y, m, d, 0, 0, 0)?;
            Ok(Value::Number(Number::I64(ts)))
        })
}

/// дни_недели() -> словарь
pub fn weekdays_fn() -> LibFunctionDef {
    LibFunctionDef::new("дни_недели")
        .with_aliases(&["weekdays", "weekday_names"])
        .with_description("Возвращает словарь ISO-дней недели (1..7) -> русские названия")
        .returns(TypeSpec::Map(Box::new(TypeSpec::Int64), Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            for (i, name) in WEEKDAY_RU_LONG.iter().enumerate() {
                map.insert(Value::Number(Number::I64((i + 1) as i64)), Value::String((*name).to_string()));
            }
            Ok(Value::Map(map))
        })
}

/// дни_недели_кратко() -> словарь
pub fn weekdays_short_fn() -> LibFunctionDef {
    LibFunctionDef::new("дни_недели_кратко")
        .with_aliases(&["weekdays_short"])
        .with_description("Возвращает словарь ISO-дней недели (1..7) -> краткие названия (пн, вт...)")
        .returns(TypeSpec::Map(Box::new(TypeSpec::Int64), Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            for (i, name) in WEEKDAY_RU_SHORT.iter().enumerate() {
                map.insert(Value::Number(Number::I64((i + 1) as i64)), Value::String((*name).to_string()));
            }
            Ok(Value::Map(map))
        })
}

/// месяцы() -> словарь
pub fn months_fn() -> LibFunctionDef {
    LibFunctionDef::new("месяцы")
        .with_aliases(&["months", "month_names"])
        .with_description("Возвращает словарь месяцев (1..12) -> русские названия")
        .returns(TypeSpec::Map(Box::new(TypeSpec::Int64), Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            for (i, name) in MONTH_RU_LONG.iter().enumerate() {
                map.insert(Value::Number(Number::I64((i + 1) as i64)), Value::String((*name).to_string()));
            }
            Ok(Value::Map(map))
        })
}

/// месяцы_кратко() -> словарь
pub fn months_short_fn() -> LibFunctionDef {
    LibFunctionDef::new("месяцы_кратко")
        .with_aliases(&["months_short"])
        .with_description("Возвращает словарь месяцев (1..12) -> краткие названия (янв, фев...)")
        .returns(TypeSpec::Map(Box::new(TypeSpec::Int64), Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            for (i, name) in MONTH_RU_SHORT.iter().enumerate() {
                map.insert(Value::Number(Number::I64((i + 1) as i64)), Value::String((*name).to_string()));
            }
            Ok(Value::Map(map))
        })
}
