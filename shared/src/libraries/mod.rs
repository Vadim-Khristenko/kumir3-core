//! Определения библиотек Kumir 3
//!
//! Этот модуль содержит определения всех стандартных библиотек.
//!
//! ## Структура
//!

pub mod registry;
pub mod time;
pub mod syscall;
pub mod files;
pub mod net;

// Реэкспорт реестра и функций поиска
pub use registry::{find_library, is_known_library};

// Реэкспорт функций создания библиотек
pub use syscall::create_syscall_library;
pub use net::create_net_library;
