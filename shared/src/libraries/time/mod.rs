//! Библиотека времени для КуМир 3
//!
//! Предоставляет функции для работы со временем:
//! - Получение текущего времени (мс, сек, мкс, нс)
//! - Паузы выполнения программы
//! - Таймеры для измерения интервалов
//! - Преобразование и форматирование дат
//! - Работа с компонентами даты-времени
//!
//! Без внешних зависимостей, только std.
//!
//! ## Пример использования
//! ```kumir
//! использовать время
//!
//! старт := таймер_старт()
//! пауза(1)
//! вывод "Прошло:", таймер_прошло_мс(старт), "мс"
//! вывод "Сейчас:", текущая_дата_рус()
//! ```

mod constants;
mod datetime;
mod format;
mod sleep;
mod timer;

use std::sync::Arc;

use crate::types::library::LibVersion;
use crate::types::library::LibraryDef;

// Реэкспорт внутренних модулей
pub use constants::*;
pub use datetime::*;
pub use format::*;
pub use sleep::*;
pub use timer::*;

/// Создаёт определение библиотеки времени
pub fn create_time_library() -> LibraryDef {
    let mut lib = LibraryDef::new("time", "Время");
    lib.aliases = vec![Arc::from("time"), Arc::from("время")];
    lib.description = Some(Arc::from(
        "Работа со временем: метки эпохи, паузы, таймеры, форматирование",
    ));
    lib.author = Arc::from("Vadim Khristenko <just@vai-prog.ru>");
    lib.version = LibVersion::new(2, 0, 0);
    lib.stable = true;

    // Регистрируем все функции
    lib.functions = vec![
        // === Текущее время ===
        now_ms_fn(),
        now_sec_fn(),
        now_us_fn(),
        now_ns_fn(),
        // === Паузы ===
        sleep_fn(),
        sleep_ms_fn(),
        sleep_min_fn(),
        sleep_us_fn(),
        // === Таймеры ===
        timer_start_fn(),
        timer_elapsed_ms_fn(),
        timer_elapsed_sec_fn(),
        timer_precise_fn(),
        // === Форматирование ===
        iso_utc_fn(),
        iso_local_fn(),
        date_ru_fn(),
        time_str_fn(),
        // === Конвертация ===
        from_timestamp_fn(),
        to_timestamp_fn(),
        split_timestamp_fn(),
        split_iso_fn(),
        make_timestamp_fn(),
        make_date_fn(),
        // === Компоненты даты ===
        current_year_fn(),
        current_month_fn(),
        current_day_fn(),
        current_hour_fn(),
        current_minute_fn(),
        current_second_fn(),
        current_weekday_fn(),
        // === Вычисления ===
        is_leap_year_fn(),
        days_in_month_fn(),
        days_in_year_fn(),
        diff_seconds_fn(),
        diff_days_fn(),
        add_seconds_fn(),
        add_days_fn(),
        // === Словари ===
        weekdays_fn(),
        weekdays_short_fn(),
        months_fn(),
        months_short_fn(),
    ];

    // Регистрируем константы
    lib.constants = vec![
        seconds_per_minute_const(),
        seconds_per_hour_const(),
        seconds_per_day_const(),
        millis_per_second_const(),
        micros_per_second_const(),
        nanos_per_second_const(),
    ];

    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let lib = create_time_library();
        assert_eq!(lib.id.as_ref(), "time");
        assert_eq!(lib.name.as_ref(), "Время");
        assert!(!lib.functions.is_empty());
        assert!(!lib.constants.is_empty());
        assert!(lib.stable);
    }

    #[test]
    fn test_now_ms() {
        let f = now_ms_fn();
        let result = f.call(&[]).unwrap();
        match result {
            crate::types::Value::Number(n) => {
                let ms = n.to_i64().unwrap();
                assert!(ms > 0);
            }
            _ => panic!("Expected Number"),
        }
    }

    #[test]
    fn test_leap_year() {
        let f = is_leap_year_fn();
        let result = f
            .call(&[crate::types::Value::Number(crate::types::Number::I64(2024))])
            .unwrap();
        assert_eq!(result, crate::types::Value::Boolean(true));

        let result = f
            .call(&[crate::types::Value::Number(crate::types::Number::I64(2023))])
            .unwrap();
        assert_eq!(result, crate::types::Value::Boolean(false));
    }

    #[test]
    fn test_timestamp_roundtrip() {
        // 2024-01-15 12:30:45
        let ts = parts_to_timestamp(2024, 1, 15, 12, 30, 45).unwrap();
        let parts = timestamp_to_parts(ts).unwrap();
        assert_eq!(parts.year, 2024);
        assert_eq!(parts.month, 1);
        assert_eq!(parts.day, 15);
        assert_eq!(parts.hour, 12);
        assert_eq!(parts.minute, 30);
        assert_eq!(parts.second, 45);
    }

    #[test]
    fn test_iso_roundtrip() {
        let iso = "2024-06-15T10:30:00Z";
        let (y, m, d, h, min, s) = parse_iso(iso).unwrap();
        assert_eq!((y, m, d, h, min, s), (2024, 6, 15, 10, 30, 0));
    }
}
