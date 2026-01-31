// ============================================================================
//                    МЕНЕДЖЕР ХЭНДЛОВ
// ============================================================================
//
// Управление ресурсами через хэндлы:
// - Handle: универсальный хэндл ресурса
// - HandleManager: реестр и управление жизненным циклом
// - Timer: асинхронные таймеры
// - Connection: абстракция сетевых соединений
//
// ============================================================================

use std::any::Any;
use std::collections::HashMap;
use std::future::Future;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, oneshot};
use tokio::time::{interval, timeout};

// ============================================================================
//                    ИДЕНТИФИКАТОРЫ
// ============================================================================

/// Уникальный идентификатор хэндла.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HandleId(pub u64);

impl HandleId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for HandleId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for HandleId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Handle#{}", self.0)
    }
}

// ============================================================================
//                    ТИП РЕСУРСА
// ============================================================================

/// Тип ресурса для категоризации.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum ResourceType {
    /// Таймер
    Timer,
    /// Интервал
    Interval,
    /// TCP соединение
    TcpConnection,
    /// UDP сокет
    UdpSocket,
    /// HTTP клиент
    HttpClient,
    /// HTTP сервер
    HttpServer,
    /// Файловый дескриптор
    File,
    /// Поток (thread/task)
    Task,
    /// Канал коммуникации
    Channel,
    /// Пользовательский тип
    Custom(u32),
}

// ============================================================================
//                    ХЭНДЛ РЕСУРСА
// ============================================================================

/// Метаданные хэндла.
#[derive(Debug, Clone)]
pub struct HandleMetadata {
    /// Имя ресурса
    pub name: String,
    /// Время создания
    pub created_at: Instant,
    /// Тип ресурса
    pub resource_type: ResourceType,
    /// Дополнительные данные
    pub extra: HashMap<String, String>,
}

impl HandleMetadata {
    pub fn new(name: impl Into<String>, resource_type: ResourceType) -> Self {
        Self {
            name: name.into(),
            created_at: Instant::now(),
            resource_type,
            extra: HashMap::new(),
        }
    }

    pub fn with_extra(mut self, key: impl Into<String>, value: impl Into<String>) -> Self {
        self.extra.insert(key.into(), value.into());
        self
    }

    /// Время жизни ресурса.
    pub fn age(&self) -> Duration {
        self.created_at.elapsed()
    }
}

/// Трейт для ресурсов, которые можно закрыть.
pub trait Closeable: Send + Sync {
    fn close(&self);
    fn is_closed(&self) -> bool;
}

/// Хэндл управления ресурсом.
pub struct Handle {
    id: HandleId,
    metadata: HandleMetadata,
    resource: Arc<dyn Any + Send + Sync>,
    closed: AtomicBool,
    close_callback: Mutex<Option<Box<dyn FnOnce() + Send + Sync>>>,
}

impl Handle {
    /// Создаёт новый хэндл.
    pub fn new<T: Any + Send + Sync>(
        name: impl Into<String>,
        resource_type: ResourceType,
        resource: T,
    ) -> Self {
        Self {
            id: HandleId::new(),
            metadata: HandleMetadata::new(name, resource_type),
            resource: Arc::new(resource),
            closed: AtomicBool::new(false),
            close_callback: Mutex::new(None),
        }
    }

    /// Идентификатор.
    pub fn id(&self) -> HandleId {
        self.id
    }

    /// Метаданные.
    pub fn metadata(&self) -> &HandleMetadata {
        &self.metadata
    }

    /// Тип ресурса.
    pub fn resource_type(&self) -> ResourceType {
        self.metadata.resource_type
    }

    /// Получает ресурс с типизацией.
    pub fn get<T: Any + Send + Sync>(&self) -> Option<Arc<T>> {
        if self.is_closed() {
            return None;
        }
        Arc::clone(&self.resource).downcast::<T>().ok()
    }

    /// Устанавливает callback при закрытии.
    pub async fn on_close<F: FnOnce() + Send + Sync + 'static>(&self, callback: F) {
        *self.close_callback.lock().await = Some(Box::new(callback));
    }

    /// Закрывает хэндл.
    pub async fn close(&self) {
        if self.closed.swap(true, Ordering::SeqCst) {
            return; // Уже закрыт
        }

        // Вызываем callback
        if let Some(callback) = self.close_callback.lock().await.take() {
            callback();
        }
    }

    /// Проверяет, закрыт ли хэндл.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }
}

// ============================================================================
//                    МЕНЕДЖЕР ХЭНДЛОВ
// ============================================================================

/// Менеджер для управления всеми хэндлами.
pub struct HandleManager {
    handles: HashMap<HandleId, Handle>,
    by_type: HashMap<ResourceType, Vec<HandleId>>,
}

impl HandleManager {
    /// Создаёт новый менеджер.
    pub fn new() -> Self {
        Self {
            handles: HashMap::new(),
            by_type: HashMap::new(),
        }
    }

    /// Регистрирует хэндл.
    pub fn register(&mut self, handle: Handle) -> HandleId {
        let id = handle.id();
        let resource_type = handle.resource_type();
        
        self.handles.insert(id, handle);
        self.by_type.entry(resource_type).or_insert_with(Vec::new).push(id);
        
        id
    }

    /// Создаёт и регистрирует хэндл.
    pub fn create<T: Any + Send + Sync>(
        &mut self,
        name: impl Into<String>,
        resource_type: ResourceType,
        resource: T,
    ) -> HandleId {
        let handle = Handle::new(name, resource_type, resource);
        self.register(handle)
    }

    /// Получает хэндл по ID.
    pub fn get(&self, id: HandleId) -> Option<&Handle> {
        self.handles.get(&id)
    }

    /// Получает ресурс из хэндла.
    pub fn get_resource<T: Any + Send + Sync>(&self, id: HandleId) -> Option<Arc<T>> {
        self.handles.get(&id).and_then(|h| h.get())
    }

    /// Закрывает хэндл.
    pub async fn close(&mut self, id: HandleId) -> bool {
        if let Some(handle) = self.handles.get(&id) {
            handle.close().await;
            true
        } else {
            false
        }
    }

    /// Удаляет закрытый хэндл.
    pub fn remove(&mut self, id: HandleId) -> bool {
        if let Some(handle) = self.handles.remove(&id) {
            let resource_type = handle.resource_type();
            if let Some(ids) = self.by_type.get_mut(&resource_type) {
                ids.retain(|&x| x != id);
            }
            true
        } else {
            false
        }
    }

    /// Получает все хэндлы определённого типа.
    pub fn get_by_type(&self, resource_type: ResourceType) -> Vec<HandleId> {
        self.by_type.get(&resource_type).cloned().unwrap_or_default()
    }

    /// Закрывает все хэндлы определённого типа.
    pub async fn close_all_of_type(&mut self, resource_type: ResourceType) {
        let ids = self.get_by_type(resource_type);
        for id in ids {
            self.close(id).await;
        }
    }

    /// Закрывает все хэндлы.
    pub async fn close_all(&mut self) {
        let ids: Vec<_> = self.handles.keys().copied().collect();
        for id in ids {
            self.close(id).await;
        }
    }

    /// Очищает закрытые хэндлы.
    pub fn cleanup(&mut self) {
        let closed: Vec<_> = self.handles
            .iter()
            .filter(|(_, h)| h.is_closed())
            .map(|(id, _)| *id)
            .collect();
        
        for id in closed {
            self.remove(id);
        }
    }

    /// Количество активных хэндлов.
    pub fn count(&self) -> usize {
        self.handles.values().filter(|h| !h.is_closed()).count()
    }

    /// Список всех хэндлов с метаданными.
    pub fn list(&self) -> Vec<(HandleId, &HandleMetadata, bool)> {
        self.handles
            .iter()
            .map(|(id, h)| (*id, h.metadata(), h.is_closed()))
            .collect()
    }
}

impl Default for HandleManager {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ТАЙМЕР
// ============================================================================

/// Состояние таймера.
struct TimerState {
    cancelled: AtomicBool,
    completed: AtomicBool,
}

/// Асинхронный таймер.
pub struct Timer {
    id: HandleId,
    state: Arc<TimerState>,
    duration: Duration,
}

impl Timer {
    /// Создаёт новый таймер.
    pub fn new(duration: Duration) -> Self {
        Self {
            id: HandleId::new(),
            state: Arc::new(TimerState {
                cancelled: AtomicBool::new(false),
                completed: AtomicBool::new(false),
            }),
            duration,
        }
    }

    /// Идентификатор таймера.
    pub fn id(&self) -> HandleId {
        self.id
    }

    /// Ожидает истечения таймера.
    pub async fn wait(&self) -> bool {
        if self.state.cancelled.load(Ordering::SeqCst) {
            return false;
        }

        tokio::time::sleep(self.duration).await;

        if self.state.cancelled.load(Ordering::SeqCst) {
            return false;
        }

        self.state.completed.store(true, Ordering::SeqCst);
        true
    }

    /// Отменяет таймер.
    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::SeqCst);
    }

    /// Проверяет, был ли таймер отменён.
    pub fn is_cancelled(&self) -> bool {
        self.state.cancelled.load(Ordering::SeqCst)
    }

    /// Проверяет, завершился ли таймер.
    pub fn is_completed(&self) -> bool {
        self.state.completed.load(Ordering::SeqCst)
    }
}

/// Создаёт одноразовый таймер и возвращает future.
pub async fn sleep(duration: Duration) {
    tokio::time::sleep(duration).await;
}

/// Создаёт таймер с callback.
pub fn set_timeout<F, Fut>(duration: Duration, callback: F) -> Timer
where
    F: FnOnce() -> Fut + Send + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let timer = Timer::new(duration);
    let state = Arc::clone(&timer.state);

    tokio::spawn(async move {
        tokio::time::sleep(duration).await;
        if !state.cancelled.load(Ordering::SeqCst) {
            state.completed.store(true, Ordering::SeqCst);
            callback().await;
        }
    });

    timer
}

// ============================================================================
//                    ИНТЕРВАЛ
// ============================================================================

/// Периодический интервал.
pub struct Interval {
    id: HandleId,
    period: Duration,
    running: Arc<AtomicBool>,
    stop_tx: Option<oneshot::Sender<()>>,
}

impl Interval {
    /// Создаёт новый интервал.
    pub fn new(period: Duration) -> Self {
        Self {
            id: HandleId::new(),
            period,
            running: Arc::new(AtomicBool::new(false)),
            stop_tx: None,
        }
    }

    /// Идентификатор.
    pub fn id(&self) -> HandleId {
        self.id
    }

    /// Запускает интервал с callback.
    pub fn start<F, Fut>(&mut self, callback: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        if self.running.load(Ordering::SeqCst) {
            return;
        }

        self.running.store(true, Ordering::SeqCst);
        let (stop_tx, mut stop_rx) = oneshot::channel();
        self.stop_tx = Some(stop_tx);

        let running = Arc::clone(&self.running);
        let period = self.period;

        tokio::spawn(async move {
            let mut interval = interval(period);
            
            loop {
                tokio::select! {
                    _ = interval.tick() => {
                        if !running.load(Ordering::SeqCst) {
                            break;
                        }
                        callback().await;
                    }
                    _ = &mut stop_rx => {
                        break;
                    }
                }
            }
        });
    }

    /// Останавливает интервал.
    pub fn stop(&mut self) {
        self.running.store(false, Ordering::SeqCst);
        if let Some(tx) = self.stop_tx.take() {
            let _ = tx.send(());
        }
    }

    /// Проверяет, запущен ли интервал.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

/// Создаёт интервал с callback.
pub fn set_interval<F, Fut>(period: Duration, callback: F) -> Interval
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    let mut interval = Interval::new(period);
    interval.start(callback);
    interval
}

// ============================================================================
//                    TIMEOUT WRAPPER
// ============================================================================

/// Выполняет future с таймаутом.
pub async fn with_timeout<T, F>(duration: Duration, future: F) -> Result<T, TimeoutError>
where
    F: Future<Output = T>,
{
    timeout(duration, future)
        .await
        .map_err(|_| TimeoutError::Elapsed(duration))
}

/// Ошибка таймаута.
#[derive(Debug, Clone)]
pub enum TimeoutError {
    Elapsed(Duration),
}

impl std::fmt::Display for TimeoutError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            TimeoutError::Elapsed(d) => write!(f, "Таймаут истёк ({:?})", d),
        }
    }
}

impl std::error::Error for TimeoutError {}

// ============================================================================
//                    DEBOUNCE / THROTTLE
// ============================================================================

/// Debounce - откладывает выполнение до прекращения вызовов.
pub struct Debounce<F> {
    delay: Duration,
    callback: Arc<F>,
    pending: Arc<Mutex<Option<tokio::task::JoinHandle<()>>>>,
}

impl<F, Fut> Debounce<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    pub fn new(delay: Duration, callback: F) -> Self {
        Self {
            delay,
            callback: Arc::new(callback),
            pending: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn call(&self) {
        let mut pending = self.pending.lock().await;
        
        // Отменяем предыдущий вызов
        if let Some(handle) = pending.take() {
            handle.abort();
        }

        let delay = self.delay;
        let callback = Arc::clone(&self.callback);
        
        *pending = Some(tokio::spawn(async move {
            tokio::time::sleep(delay).await;
            callback().await;
        }));
    }

    pub async fn cancel(&self) {
        if let Some(handle) = self.pending.lock().await.take() {
            handle.abort();
        }
    }
}

/// Throttle - ограничивает частоту вызовов.
pub struct Throttle<F> {
    interval: Duration,
    callback: Arc<F>,
    last_call: Arc<Mutex<Option<Instant>>>,
}

impl<F, Fut> Throttle<F>
where
    F: Fn() -> Fut + Send + Sync + 'static,
    Fut: Future<Output = ()> + Send + 'static,
{
    pub fn new(interval: Duration, callback: F) -> Self {
        Self {
            interval,
            callback: Arc::new(callback),
            last_call: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn call(&self) -> bool {
        let mut last = self.last_call.lock().await;
        let now = Instant::now();

        if let Some(last_time) = *last {
            if now.duration_since(last_time) < self.interval {
                return false;
            }
        }

        *last = Some(now);
        drop(last);

        (self.callback)().await;
        true
    }

    pub async fn reset(&self) {
        *self.last_call.lock().await = None;
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_handle_id() {
        let id1 = HandleId::new();
        let id2 = HandleId::new();
        assert_ne!(id1, id2);
        assert!(!id1.is_null());
        assert!(HandleId::null().is_null());
    }

    #[tokio::test]
    async fn test_handle_manager() {
        let mut manager = HandleManager::new();
        
        let id = manager.create("test_resource", ResourceType::Custom(1), "data".to_string());
        assert_eq!(manager.count(), 1);

        let resource: Option<Arc<String>> = manager.get_resource(id);
        assert!(resource.is_some());
        assert_eq!(*resource.unwrap(), "data");

        manager.close(id).await;
        assert!(manager.get(id).unwrap().is_closed());
    }

    #[tokio::test]
    async fn test_timer() {
        let timer = Timer::new(Duration::from_millis(10));
        assert!(!timer.is_completed());
        
        let completed = timer.wait().await;
        assert!(completed);
        assert!(timer.is_completed());
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let timer = Timer::new(Duration::from_secs(10));
        timer.cancel();
        
        let completed = timer.wait().await;
        assert!(!completed);
        assert!(timer.is_cancelled());
    }

    #[tokio::test]
    async fn test_interval() {
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = Arc::clone(&counter);

        let mut interval = Interval::new(Duration::from_millis(10));
        interval.start(move || {
            let c = Arc::clone(&counter_clone);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        tokio::time::sleep(Duration::from_millis(55)).await;
        interval.stop();

        let count = counter.load(Ordering::SeqCst);
        assert!(count >= 4 && count <= 6); // ~5 тиков
    }

    #[tokio::test]
    async fn test_with_timeout_success() {
        let result = with_timeout(Duration::from_secs(1), async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        }).await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_with_timeout_elapsed() {
        let result = with_timeout(Duration::from_millis(10), async {
            tokio::time::sleep(Duration::from_secs(1)).await;
            42
        }).await;

        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_throttle() {
        let counter = Arc::new(AtomicU64::new(0));
        let counter_clone = Arc::clone(&counter);

        let throttle = Throttle::new(Duration::from_millis(50), move || {
            let c = Arc::clone(&counter_clone);
            async move {
                c.fetch_add(1, Ordering::SeqCst);
            }
        });

        // Вызываем много раз подряд
        for _ in 0..10 {
            throttle.call().await;
        }

        // Должен был выполниться только один раз
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }
}
