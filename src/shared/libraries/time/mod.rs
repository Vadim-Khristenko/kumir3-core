//! Библиотека времени для КуМир 3
//!
//! Предоставляет функции для работы со временем:
//! - Получение текущего времени
//! - Паузы выполнения программы  
//! - Таймеры для измерения интервалов
//! - Преобразование и форматирование дат
//! - Работа с часовыми поясами
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
mod sleep;
mod timer;
mod format;

use crate::shared::types::library::LibraryDef;
use crate::shared::types::library::LibVersion;

// Реэкспорт внутренних модулей
pub use constants::*;
pub use datetime::*;
pub use sleep::*;
pub use timer::*;
pub use format::*;

/// Создаёт определение библиотеки времени
pub fn create_time_library() -> LibraryDef {
    let mut lib = LibraryDef::new("time", "Время");
    lib.aliases = &["time", "время"];
    lib.description = "Работа со временем: метки эпохи, паузы, таймеры, форматирование";
    lib.author = "Vadim Khristenko <just@vai-prog.ru>";
    lib.version = LibVersion::new(1, 0, 0);
    lib.stable = true;

    // Регистрируем все функции
    lib.functions = vec![
        // === Текущее время ===
        now_ms_fn(),
        now_sec_fn(),
        
        // === Паузы ===
        sleep_fn(),
        sleep_ms_fn(),
        sleep_min_fn(),
        
        // === Таймеры ===
        timer_start_fn(),
        timer_elapsed_ms_fn(),
        timer_elapsed_sec_fn(),
        
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
    ];

    lib
}