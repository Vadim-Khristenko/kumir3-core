//! Константы для работы со временем

use std::sync::Arc;

use crate::types::library::LibConstantDef;
use crate::types::{Number, TypeKind, Value};

// ============================================================================
// ВРЕМЕННЫЕ КОНСТАНТЫ
// ============================================================================

/// Секунд в минуте
pub const SECONDS_PER_MINUTE: i64 = 60;
/// Секунд в часе
pub const SECONDS_PER_HOUR: i64 = 60 * SECONDS_PER_MINUTE;
/// Секунд в сутках
pub const SECONDS_PER_DAY: i64 = 24 * SECONDS_PER_HOUR;
/// Миллисекунд в секунде
pub const MILLIS_PER_SECOND: i64 = 1000;
/// Микросекунд в секунде
pub const MICROS_PER_SECOND: i64 = 1_000_000;
/// Наносекунд в секунде
pub const NANOS_PER_SECOND: i64 = 1_000_000_000;

// ============================================================================
// НАЗВАНИЯ ДНЕЙ И МЕСЯЦЕВ
// ============================================================================

/// Краткие названия дней недели (ISO: 1 = пн, 7 = вс)
pub const WEEKDAY_RU_SHORT: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

/// Полные названия дней недели (ISO: 1 = пн, 7 = вс)
pub const WEEKDAY_RU_LONG: [&str; 7] = [
    "понедельник",
    "вторник",
    "среда",
    "четверг",
    "пятница",
    "суббота",
    "воскресенье",
];

/// Краткие названия месяцев (1..12)
pub const MONTH_RU_SHORT: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", "июл", "авг", "сен", "окт", "ноя", "дек",
];

/// Полные названия месяцев (1..12)
pub const MONTH_RU_LONG: [&str; 12] = [
    "январь",
    "февраль",
    "март",
    "апрель",
    "май",
    "июнь",
    "июль",
    "август",
    "сентябрь",
    "октябрь",
    "ноябрь",
    "декабрь",
];

// ============================================================================
// ЭКСПОРТ КОНСТАНТ ДЛЯ БИБЛИОТЕКИ
// ============================================================================

pub fn seconds_per_minute_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("СЕКУНД_В_МИНУТЕ"),
        aliases: vec![Arc::from("SECONDS_PER_MINUTE")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_MINUTE)),
        description: Some(Arc::from("Количество секунд в минуте (60)")),
    }
}

pub fn seconds_per_hour_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("СЕКУНД_В_ЧАСЕ"),
        aliases: vec![Arc::from("SECONDS_PER_HOUR")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_HOUR)),
        description: Some(Arc::from("Количество секунд в часе (3600)")),
    }
}

pub fn seconds_per_day_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("СЕКУНД_В_СУТКАХ"),
        aliases: vec![Arc::from("SECONDS_PER_DAY")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_DAY)),
        description: Some(Arc::from("Количество секунд в сутках (86400)")),
    }
}

pub fn millis_per_second_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("МС_В_СЕКУНДЕ"),
        aliases: vec![Arc::from("MILLIS_PER_SECOND")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(MILLIS_PER_SECOND)),
        description: Some(Arc::from("Количество миллисекунд в секунде (1000)")),
    }
}

pub fn micros_per_second_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("МКС_В_СЕКУНДЕ"),
        aliases: vec![Arc::from("MICROS_PER_SECOND")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(MICROS_PER_SECOND)),
        description: Some(Arc::from("Количество микросекунд в секунде (1 000 000)")),
    }
}

pub fn nanos_per_second_const() -> LibConstantDef {
    LibConstantDef {
        name: Arc::from("НС_В_СЕКУНДЕ"),
        aliases: vec![Arc::from("NANOS_PER_SECOND")],
        const_type: TypeKind::Int64,
        value: Value::Number(Number::I64(NANOS_PER_SECOND)),
        description: Some(Arc::from("Количество наносекунд в секунде (1 000 000 000)")),
    }
}
