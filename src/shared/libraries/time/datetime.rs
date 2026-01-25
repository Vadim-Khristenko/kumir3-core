//! Функции для работы с датой и временем
//!
//! Содержит функции получения текущего времени и его компонентов.

use std::time::{SystemTime, UNIX_EPOCH};

use crate::shared::types::library::{LibFunctionDef, LibParamDef};
use crate::shared::types::type_spec::TypeSpec;
use crate::shared::types::{Value, Number};

use super::constants::*;

// ============================================================================
// ВСПОМОГАТЕЛЬНЫЕ СТРУКТУРЫ
// ============================================================================

/// Компоненты даты и времени
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct DateTimeParts {
    pub year: i32,
    pub month: u8,      // 1..=12
    pub day: u8,        // 1..=31
    pub hour: u8,       // 0..=23
    pub minute: u8,     // 0..=59
    pub second: u8,     // 0..=59
    pub weekday: u8,    // 1 (пн) .. 7 (вс)
    pub yearday: u16,   // 0..365
}

fn expect_number(args: &[Value], idx: usize, what: &str) -> Result<i64, String> {
    let v = args.get(idx).ok_or_else(|| format!("Не передан параметр: {}", what))?;
    match v {
        Value::Number(n) => n.to_i64().ok_or_else(|| format!("Ожидается целое: {}", what)),
        _ => Err(format!("Ожидается число для параметра {}", what)),
    }
}

// ============================================================================
// ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ============================================================================

/// Проверяет, является ли год високосным
#[inline]
pub fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

/// Возвращает количество дней в месяце
pub fn days_in_month(year: i32, month: u8) -> Option<u8> {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => Some(31),
        4 | 6 | 9 | 11 => Some(30),
        2 => Some(if is_leap_year(year) { 29 } else { 28 }),
        _ => None,
    }
}

/// Возвращает количество дней в году
#[inline]
pub fn days_in_year(year: i32) -> u16 {
    if is_leap_year(year) { 366 } else { 365 }
}

/// Возвращает текущее время в миллисекундах от UNIX-эпохи
pub fn system_time_ms() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .map_err(|e| format!("Ошибка получения системного времени: {}", e))
}

/// Возвращает текущее время в секундах от UNIX-эпохи
pub fn system_time_sec() -> Result<i64, String> {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .map_err(|e| format!("Ошибка получения системного времени: {}", e))
}

/// Вычисляет день недели ISO (1=пн, 7=вс) по количеству дней от эпохи
#[inline]
pub fn weekday_from_days(days: i64) -> u8 {
    // 1970-01-01 = четверг (ISO 4)
    ((days + 3).rem_euclid(7) + 1) as u8
}

/// Преобразует дни от эпохи в компоненты даты (алгоритм Howard Hinnant)
pub fn civil_from_days(days: i64) -> (i32, u8, u8, u16) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let y = yoe + era * 400;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = mp + if mp < 10 { 3 } else { -9 };
    let year = y + if m <= 2 { 1 } else { 0 };
    (year as i32, m as u8, d as u8, doy as u16)
}

/// Преобразует дату в количество дней от эпохи UNIX
pub fn days_from_civil(year: i32, month: u8, day: u8) -> Result<i64, String> {
    if !(1..=12).contains(&month) {
        return Err(format!("Месяц должен быть от 1 до 12, получено: {}", month));
    }
    if day == 0 {
        return Err("День месяца должен быть >= 1".to_string());
    }
    let max_day = days_in_month(year, month).ok_or("Некорректный месяц")?;
    if day > max_day {
        return Err(format!("День {} выходит за пределы месяца", day));
    }

    let y = year - (month <= 2) as i32;
    let m = month as i32;
    let d = day as i32;

    let era = if y >= 0 { y } else { y - 399 }.div_euclid(400);
    let yoe = y - era * 400;
    let doy = (153 * (m + if m > 2 { -3 } else { 9 }) + 2) / 5 + d - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    
    Ok(era as i64 * 146_097 + doe as i64 - 719_468)
}

/// Преобразует UNIX timestamp в компоненты даты-времени
pub fn timestamp_to_parts(secs: i64) -> Result<DateTimeParts, String> {
    if secs < 0 {
        return Err(format!("Отрицательные штампы времени не поддерживаются: {}", secs));
    }

    let days = secs.div_euclid(SECONDS_PER_DAY);
    let seconds_in_day = secs.rem_euclid(SECONDS_PER_DAY);

    let hour = (seconds_in_day / SECONDS_PER_HOUR) as u8;
    let minute = ((seconds_in_day % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u8;
    let second = (seconds_in_day % SECONDS_PER_MINUTE) as u8;

    let (year, month, day, yearday) = civil_from_days(days);
    let weekday = weekday_from_days(days);

    Ok(DateTimeParts { year, month, day, hour, minute, second, weekday, yearday })
}

/// Собирает компоненты даты-времени в UNIX timestamp
pub fn parts_to_timestamp(year: i32, month: u8, day: u8, hour: u8, minute: u8, second: u8) -> Result<i64, String> {
    if hour > 23 {
        return Err(format!("Часы должны быть 0..23, получено: {}", hour));
    }
    if minute > 59 {
        return Err(format!("Минуты должны быть 0..59, получено: {}", minute));
    }
    if second > 59 {
        return Err(format!("Секунды должны быть 0..59, получено: {}", second));
    }

    let days = days_from_civil(year, month, day)?;
    Ok(days * SECONDS_PER_DAY + (hour as i64) * SECONDS_PER_HOUR + (minute as i64) * SECONDS_PER_MINUTE + (second as i64))
}

// ============================================================================
// ОПРЕДЕЛЕНИЯ ФУНКЦИЙ БИБЛИОТЕКИ
// ============================================================================

/// текущее_время_мс() -> цел_64
pub fn now_ms_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущее_время_мс")
        .with_aliases(&["now_ms", "time_ms"])
        .with_description("Возвращает количество миллисекунд с 1 января 1970 UTC")
        .returns(TypeSpec::Int64)
        .with_handler(|_args| {
            let ms = system_time_ms()?;
            Ok(Value::Number(Number::I64(ms)))
        })
}

/// текущее_время_сек() -> цел_64
pub fn now_sec_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущее_время_сек")
        .with_aliases(&["now", "time", "now_sec"])
        .with_description("Возвращает количество секунд с 1 января 1970 UTC")
        .returns(TypeSpec::Int64)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            Ok(Value::Number(Number::I64(secs)))
        })
}

/// текущий_год() -> цел_32
pub fn current_year_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущий_год")
        .with_aliases(&["year", "current_year"])
        .with_description("Возвращает текущий год (UTC)")
        .returns(TypeSpec::Int32)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::I32(parts.year)))
        })
}

/// текущий_месяц() -> нат_8
pub fn current_month_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущий_месяц")
        .with_aliases(&["month", "current_month"])
        .with_description("Возвращает текущий месяц (1..12, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.month)))
        })
}

/// текущий_день() -> нат_8
pub fn current_day_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущий_день")
        .with_aliases(&["day", "current_day"])
        .with_description("Возвращает текущий день месяца (1..31, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.day)))
        })
}

/// текущий_час() -> нат_8
pub fn current_hour_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущий_час")
        .with_aliases(&["hour", "current_hour"])
        .with_description("Возвращает текущий час (0..23, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.hour)))
        })
}

/// текущая_минута() -> нат_8
pub fn current_minute_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущая_минута")
        .with_aliases(&["minute", "current_minute"])
        .with_description("Возвращает текущую минуту (0..59, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.minute)))
        })
}

/// текущая_секунда() -> нат_8
pub fn current_second_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущая_секунда")
        .with_aliases(&["second", "current_second"])
        .with_description("Возвращает текущую секунду (0..59, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.second)))
        })
}

/// текущий_день_недели() -> нат_8
pub fn current_weekday_fn() -> LibFunctionDef {
    LibFunctionDef::new("текущий_день_недели")
        .with_aliases(&["weekday", "current_weekday"])
        .with_description("Возвращает текущий день недели ISO (1=пн, 7=вс, UTC)")
        .returns(TypeSpec::UInt8)
        .with_handler(|_args| {
            let secs = system_time_sec()?;
            let parts = timestamp_to_parts(secs)?;
            Ok(Value::Number(Number::U8(parts.weekday)))
        })
}

/// високосный_год(год: цел_32) -> лог
pub fn is_leap_year_fn() -> LibFunctionDef {
    LibFunctionDef::new("високосный_год")
        .with_aliases(&["is_leap_year", "leap_year"])
        .with_description("Проверяет, является ли год високосным")
        .with_param(LibParamDef::value("год", TypeSpec::Int32))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let year = expect_number(args, 0, "год")?;
            Ok(Value::Boolean(is_leap_year(year as i32)))
        })
}

/// дней_в_месяце(год: цел_32, месяц: нат_8) -> нат_8
pub fn days_in_month_fn() -> LibFunctionDef {
    LibFunctionDef::new("дней_в_месяце")
        .with_aliases(&["days_in_month"])
        .with_description("Возвращает количество дней в указанном месяце года")
        .with_param(LibParamDef::value("год", TypeSpec::Int32))
        .with_param(LibParamDef::value("месяц", TypeSpec::UInt8))
        .returns(TypeSpec::UInt8)
        .with_handler(|args| {
            let year = expect_number(args, 0, "год")? as i32;
            let month = expect_number(args, 1, "месяц")? as u8;
            let days = days_in_month(year, month).ok_or_else(|| "Некорректный месяц".to_string())?;
            Ok(Value::Number(Number::U8(days)))
        })
}

/// дней_в_году(год: цел_32) -> нат_16
pub fn days_in_year_fn() -> LibFunctionDef {
    LibFunctionDef::new("дней_в_году")
        .with_aliases(&["days_in_year"])
        .with_description("Возвращает количество дней в указанном году (365 или 366)")
        .with_param(LibParamDef::value("год", TypeSpec::Int32))
        .returns(TypeSpec::UInt16)
        .with_handler(|args| {
            let year = expect_number(args, 0, "год")? as i32;
            Ok(Value::Number(Number::U16(days_in_year(year))))
        })
}

/// разница_сек(штамп1: цел_64, штамп2: цел_64) -> цел_64
pub fn diff_seconds_fn() -> LibFunctionDef {
    LibFunctionDef::new("разница_сек")
        .with_aliases(&["diff_seconds", "diff_sec"])
        .with_description("Возвращает разницу между двумя timestamp в секундах (ts2 - ts1)")
        .with_param(LibParamDef::value("штамп1", TypeSpec::Int64))
        .with_param(LibParamDef::value("штамп2", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let ts1 = expect_number(args, 0, "штамп1")?;
            let ts2 = expect_number(args, 1, "штамп2")?;
            Ok(Value::Number(Number::I64(ts2 - ts1)))
        })
}

/// разница_дней(штамп1: цел_64, штамп2: цел_64) -> цел_64
pub fn diff_days_fn() -> LibFunctionDef {
    LibFunctionDef::new("разница_дней")
        .with_aliases(&["diff_days"])
        .with_description("Возвращает разницу между двумя timestamp в днях")
        .with_param(LibParamDef::value("штамп1", TypeSpec::Int64))
        .with_param(LibParamDef::value("штамп2", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let ts1 = expect_number(args, 0, "штамп1")?;
            let ts2 = expect_number(args, 1, "штамп2")?;
            Ok(Value::Number(Number::I64((ts2 - ts1) / SECONDS_PER_DAY)))
        })
}

/// добавить_сек(штамп: цел_64, секунды: цел_64) -> цел_64
pub fn add_seconds_fn() -> LibFunctionDef {
    LibFunctionDef::new("добавить_сек")
        .with_aliases(&["add_seconds", "add_sec"])
        .with_description("Добавляет указанное количество секунд к timestamp")
        .with_param(LibParamDef::value("штамп", TypeSpec::Int64))
        .with_param(LibParamDef::value("секунды", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let ts = expect_number(args, 0, "штамп")?;
            let delta = expect_number(args, 1, "секунды")?;
            Ok(Value::Number(Number::I64(ts + delta)))
        })
}

/// добавить_дней(штамп: цел_64, дни: цел_64) -> цел_64
pub fn add_days_fn() -> LibFunctionDef {
    LibFunctionDef::new("добавить_дней")
        .with_aliases(&["add_days"])
        .with_description("Добавляет указанное количество дней к timestamp")
        .with_param(LibParamDef::value("штамп", TypeSpec::Int64))
        .with_param(LibParamDef::value("дни", TypeSpec::Int64))
        .returns(TypeSpec::Int64)
        .with_handler(|args| {
            let ts = expect_number(args, 0, "штамп")?;
            let days = expect_number(args, 1, "дни")?;
            Ok(Value::Number(Number::I64(ts + days * SECONDS_PER_DAY)))
        })
}
