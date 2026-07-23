//! Async runtime layer for Kumir 3.
//!
//! Provides infrastructure for:
//! - Async code execution (tokio)
//! - Callback system (compiler ↔ interpreter)
//! - Event-driven architecture with subscriptions
//! - Inter-component communication channels
//! - Resource and handle management

pub mod callback;
pub mod channel;
pub mod events;
pub mod executor;
pub mod handle;

pub use callback::*;
pub use channel::*;
pub use events::*;
pub use executor::*;
pub use handle::*;

use std::sync::Arc;
use tokio::sync::RwLock;

// =============================================================================
//         SECTION: GLOBAL RUNTIME
// =============================================================================

/// [STABLE] Global async runtime for Kumir 3.
///
/// Provides unified access to:
/// - Tokio runtime for async tasks
/// - Event bus with subscriptions
/// - Callback registry
/// - Task executor
/// - Resource handle manager
pub struct KumirRuntime {
    tokio_handle: Option<tokio::runtime::Handle>,
    callbacks: Arc<RwLock<CallbackRegistry>>,
    event_bus: Arc<EventBus>,
    executor: Arc<TaskExecutor>,
    handles: Arc<RwLock<HandleManager>>,
}

impl KumirRuntime {
    /// Creates a new runtime with no async executor.
    pub fn new() -> Self {
        Self {
            tokio_handle: None,
            callbacks: Arc::new(RwLock::new(CallbackRegistry::new())),
            event_bus: Arc::new(EventBus::new()),
            executor: Arc::new(TaskExecutor::new()),
            handles: Arc::new(RwLock::new(HandleManager::new())),
        }
    }

    /// Creates a runtime backed by an existing tokio handle.
    pub fn with_tokio(handle: tokio::runtime::Handle) -> Self {
        Self {
            tokio_handle: Some(handle),
            callbacks: Arc::new(RwLock::new(CallbackRegistry::new())),
            event_bus: Arc::new(EventBus::new()),
            executor: Arc::new(TaskExecutor::new()),
            handles: Arc::new(RwLock::new(HandleManager::new())),
        }
    }

    /// Initializes a dedicated tokio multi-threaded runtime.
    pub fn init_tokio(&mut self) -> Result<(), RuntimeError> {
        if self.tokio_handle.is_some() {
            return Ok(());
        }

        let rt = tokio::runtime::Builder::new_multi_thread()
            .worker_threads(4)
            .enable_all()
            .build()
            .map_err(|e| {
                RuntimeError::new(
                    RuntimeErrorKind::InitializationFailed,
                    format!("Не удалось создать tokio runtime: {}", e),
                )
            })?;

        self.tokio_handle = Some(rt.handle().clone());

        std::thread::spawn(move || {
            rt.block_on(async {
                tokio::signal::ctrl_c().await.ok();
            });
        });

        Ok(())
    }

    /// Returns the tokio handle, if initialized.
    pub fn tokio_handle(&self) -> Option<&tokio::runtime::Handle> {
        self.tokio_handle.as_ref()
    }

    /// Returns a reference to the callback registry.
    pub fn callbacks(&self) -> Arc<RwLock<CallbackRegistry>> {
        Arc::clone(&self.callbacks)
    }

    /// Returns a reference to the event bus.
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Returns a reference to the task executor.
    pub fn executor(&self) -> Arc<TaskExecutor> {
        Arc::clone(&self.executor)
    }

    /// Returns a reference to the handle manager.
    pub fn handles(&self) -> Arc<RwLock<HandleManager>> {
        Arc::clone(&self.handles)
    }

    /// Blocks on an async future using the tokio runtime.
    pub fn block_on<F: std::future::Future>(&self, future: F) -> Option<F::Output> {
        self.tokio_handle.as_ref().map(|h| h.block_on(future))
    }

    /// Spawns an async task on the tokio runtime.
    pub fn spawn<F>(&self, future: F) -> Option<tokio::task::JoinHandle<F::Output>>
    where
        F: std::future::Future + Send + 'static,
        F::Output: Send + 'static,
    {
        self.tokio_handle.as_ref().map(|h| h.spawn(future))
    }
}

impl Default for KumirRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ОШИБКИ RUNTIME
// ============================================================================

/// Виды ошибок runtime.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RuntimeErrorKind {
    /// Ошибка инициализации
    InitializationFailed,
    /// Коллбэк не найден
    CallbackNotFound,
    /// Ошибка канала
    ChannelError,
    /// Ресурс не найден
    ResourceNotFound,
    /// Таймаут операции
    Timeout,
    /// Задача отменена
    TaskCancelled,
    /// Общая ошибка
    Other,
}

/// Ошибка runtime.
#[derive(Debug, Clone)]
pub struct RuntimeError {
    pub kind: RuntimeErrorKind,
    pub message: String,
}

impl RuntimeError {
    pub fn new(kind: RuntimeErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub fn callback_not_found(name: &str) -> Self {
        Self::new(
            RuntimeErrorKind::CallbackNotFound,
            format!("Коллбэк '{}' не найден", name),
        )
    }

    pub fn resource_not_found(id: u64) -> Self {
        Self::new(
            RuntimeErrorKind::ResourceNotFound,
            format!("Ресурс с ID {} не найден", id),
        )
    }

    pub fn timeout(operation: &str) -> Self {
        Self::new(
            RuntimeErrorKind::Timeout,
            format!("Таймаут операции: {}", operation),
        )
    }
}

impl std::fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}: {}", self.kind, self.message)
    }
}

impl std::error::Error for RuntimeError {}

/// Тип результата для runtime операций.
pub type RuntimeResult<T> = Result<T, RuntimeError>;

// ============================================================================
//                    ГЛОБАЛЬНЫЙ ЭКЗЕМПЛЯР
// ============================================================================

use once_cell::sync::Lazy;
use std::sync::Mutex;

/// Глобальный runtime (lazy initialized).
static GLOBAL_RUNTIME: Lazy<Mutex<KumirRuntime>> = Lazy::new(|| Mutex::new(KumirRuntime::new()));

/// Получает доступ к глобальному runtime.
pub fn global_runtime() -> std::sync::MutexGuard<'static, KumirRuntime> {
    GLOBAL_RUNTIME.lock().unwrap()
}

/// Инициализирует глобальный runtime с tokio.
pub fn init_global_runtime() -> RuntimeResult<()> {
    global_runtime().init_tokio()
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_runtime_creation() {
        let runtime = KumirRuntime::new();
        assert!(runtime.tokio_handle().is_none());
    }

    #[test]
    fn test_global_runtime() {
        let _guard = global_runtime();
        // Просто проверяем что не паникует
    }
}
