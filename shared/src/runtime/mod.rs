// ============================================================================
//                    RUNTIME ПРОСЛОЙКА
// ============================================================================
//
// Этот модуль предоставляет общую инфраструктуру для:
// - Асинхронного выполнения кода (tokio)
// - Системы коллбэков между компилятором и интерпретатором
// - Event-driven архитектуры с подписками
// - Каналов коммуникации между компонентами
// - Управления ресурсами и хэндлами
//
// ============================================================================

pub mod callback;
pub mod channel;
pub mod events;
pub mod executor;
pub mod handle;

// Реэкспорты для удобного доступа
pub use callback::*;
pub use channel::*;
pub use events::*;
pub use executor::*;
pub use handle::*;

use std::sync::Arc;
use tokio::sync::RwLock;

// ============================================================================
//                    ГЛОБАЛЬНЫЙ RUNTIME
// ============================================================================

/// Глобальный runtime для асинхронных операций.
///
/// Предоставляет единую точку доступа к:
/// - Tokio runtime для async задач
/// - Системе событий и подписок
/// - Реестру коллбэков
/// - Каналам коммуникации
pub struct KumirRuntime {
    /// Tokio runtime handle
    tokio_handle: Option<tokio::runtime::Handle>,
    /// Менеджер коллбэков
    callbacks: Arc<RwLock<CallbackRegistry>>,
    /// Event bus для событий
    event_bus: Arc<EventBus>,
    /// Исполнитель задач
    executor: Arc<TaskExecutor>,
    /// Менеджер хэндлов ресурсов
    handles: Arc<RwLock<HandleManager>>,
}

impl KumirRuntime {
    /// Создаёт новый runtime.
    pub fn new() -> Self {
        Self {
            tokio_handle: None,
            callbacks: Arc::new(RwLock::new(CallbackRegistry::new())),
            event_bus: Arc::new(EventBus::new()),
            executor: Arc::new(TaskExecutor::new()),
            handles: Arc::new(RwLock::new(HandleManager::new())),
        }
    }

    /// Создаёт runtime с существующим tokio handle.
    pub fn with_tokio(handle: tokio::runtime::Handle) -> Self {
        Self {
            tokio_handle: Some(handle),
            callbacks: Arc::new(RwLock::new(CallbackRegistry::new())),
            event_bus: Arc::new(EventBus::new()),
            executor: Arc::new(TaskExecutor::new()),
            handles: Arc::new(RwLock::new(HandleManager::new())),
        }
    }

    /// Инициализирует собственный tokio runtime.
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

        // Держим runtime в фоне
        std::thread::spawn(move || {
            rt.block_on(async {
                // Runtime работает пока не будет остановлен
                tokio::signal::ctrl_c().await.ok();
            });
        });

        Ok(())
    }

    /// Получает tokio handle (если инициализирован).
    pub fn tokio_handle(&self) -> Option<&tokio::runtime::Handle> {
        self.tokio_handle.as_ref()
    }

    /// Регистр коллбэков.
    pub fn callbacks(&self) -> Arc<RwLock<CallbackRegistry>> {
        Arc::clone(&self.callbacks)
    }

    /// Event bus.
    pub fn event_bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.event_bus)
    }

    /// Исполнитель задач.
    pub fn executor(&self) -> Arc<TaskExecutor> {
        Arc::clone(&self.executor)
    }

    /// Менеджер хэндлов.
    pub fn handles(&self) -> Arc<RwLock<HandleManager>> {
        Arc::clone(&self.handles)
    }

    /// Выполняет async задачу блокирующе.
    pub fn block_on<F: std::future::Future>(&self, future: F) -> Option<F::Output> {
        self.tokio_handle.as_ref().map(|h| h.block_on(future))
    }

    /// Спавнит async задачу.
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
