// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Standard libraries for Kumir 3.
//!
//! Provides built-in libraries:
//! - **time** — date, time, timers, formatting
//! - **syscall** — OS, environment, processes, paths
//! - **files** — reading/writing, directories, metadata
//! - **net** — TCP/UDP, HTTP, DNS, Base64, JSON
//!
//! Also the user library system:
//! - **user_library** — loading libraries from .kum files

pub mod files;
pub mod net;
pub mod registry;
pub mod syscall;
pub mod time;
pub mod user_library;

pub use files::create_files_library;
pub use net::create_net_library;
pub use syscall::create_syscall_library;
pub use time::create_time_library;
pub use user_library::{UserLibraryLoader, create_example_library};

pub use registry::{find_library, get_global_provider, is_known_library, register_all_builtins};
