//! Константы для работы со временем

use crate::types::library::LibConstantDef;
use crate::types::type_spec::TypeSpec;
use crate::types::{Value, Number};

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

// ============================================================================
// НАЗВАНИЯ ДНЕЙ И МЕСЯЦЕВ
// ============================================================================

/// Краткие названия дней недели (ISO: 1 = пн, 7 = вс)
pub const WEEKDAY_RU_SHORT: [&str; 7] = ["пн", "вт", "ср", "чт", "пт", "сб", "вс"];

/// Полные названия дней недели (ISO: 1 = пн, 7 = вс)
pub const WEEKDAY_RU_LONG: [&str; 7] = [
    "понедельник", "вторник", "среда", "четверг", 
    "пятница", "суббота", "воскресенье"
];

/// Краткие названия месяцев (1..12)
pub const MONTH_RU_SHORT: [&str; 12] = [
    "янв", "фев", "мар", "апр", "май", "июн", 
    "июл", "авг", "сен", "окт", "ноя", "дек"
];

/// Полные названия месяцев (1..12)
pub const MONTH_RU_LONG: [&str; 12] = [
    "январь", "февраль", "март", "апрель", "май", "июнь",
    "июль", "август", "сентябрь", "октябрь", "ноябрь", "декабрь"
];

// ============================================================================
// ЭКСПОРТ КОНСТАНТ ДЛЯ БИБЛИОТЕКИ
// ============================================================================

pub fn seconds_per_minute_const() -> LibConstantDef {
    LibConstantDef {
        name: "СЕКУНД_В_МИНУТЕ",
        aliases: &["SECONDS_PER_MINUTE"],
        const_type: TypeSpec::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_MINUTE)),
        description: "Количество секунд в минуте (60)",
    }
}

pub fn seconds_per_hour_const() -> LibConstantDef {
    LibConstantDef {
        name: "СЕКУНД_В_ЧАСЕ",
        aliases: &["SECONDS_PER_HOUR"],
        const_type: TypeSpec::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_HOUR)),
        description: "Количество секунд в часе (3600)",
    }
}

pub fn seconds_per_day_const() -> LibConstantDef {
    LibConstantDef {
        name: "СЕКУНД_В_СУТКАХ",
        aliases: &["SECONDS_PER_DAY"],
        const_type: TypeSpec::Int64,
        value: Value::Number(Number::I64(SECONDS_PER_DAY)),
        description: "Количество секунд в сутках (86400)",
    }
}

pub fn millis_per_second_const() -> LibConstantDef {
    LibConstantDef {
        name: "МС_В_СЕКУНДЕ",
        aliases: &["MILLIS_PER_SECOND"],
        const_type: TypeSpec::Int64,
        value: Value::Number(Number::I64(MILLIS_PER_SECOND)),
        description: "Количество миллисекунд в секунде (1000)",
    }
}
