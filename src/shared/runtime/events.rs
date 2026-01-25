// ============================================================================
//                    СИСТЕМА СОБЫТИЙ
// ============================================================================
//
// Event-driven архитектура для реактивного программирования:
// - Event: типизированное событие
// - EventBus: шина событий с подписками
// - EventEmitter: генератор событий
// - Subscription: управление подпиской
//
// ============================================================================

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use tokio::sync::{broadcast, RwLock};

use crate::shared::types::Value;

// ============================================================================
//                    СОБЫТИЕ
// ============================================================================

/// Идентификатор подписки.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SubscriptionId(pub u64);

impl SubscriptionId {
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for SubscriptionId {
    fn default() -> Self {
        Self::new()
    }
}

/// Базовое событие.
#[derive(Debug, Clone)]
pub struct Event {
    /// Тип/имя события
    pub name: String,
    /// Данные события
    pub data: Value,
    /// Временная метка
    pub timestamp: std::time::Instant,
    /// Источник события
    pub source: Option<String>,
    /// Может ли событие быть отменено
    pub cancelable: bool,
    /// Было ли событие отменено
    cancelled: Arc<AtomicBool>,
}

impl Event {
    /// Создаёт новое событие.
    pub fn new(name: impl Into<String>, data: Value) -> Self {
        Self {
            name: name.into(),
            data,
            timestamp: std::time::Instant::now(),
            source: None,
            cancelable: false,
            cancelled: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Устанавливает источник.
    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        self.source = Some(source.into());
        self
    }

    /// Делает событие отменяемым.
    pub fn cancelable(mut self) -> Self {
        self.cancelable = true;
        self
    }

    /// Отменяет событие (если отменяемо).
    pub fn cancel(&self) -> bool {
        if self.cancelable {
            self.cancelled.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Проверяет, было ли событие отменено.
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }
}

// ============================================================================
//                    ТИПИЗИРОВАННЫЕ СОБЫТИЯ
// ============================================================================

/// Трейт для типизированных событий.
pub trait TypedEvent: Send + Sync + Clone + 'static {
    /// Имя типа события.
    fn event_name() -> &'static str;
}

/// Макрос для создания типизированных событий.
#[macro_export]
macro_rules! define_event {
    ($name:ident { $($field:ident: $ty:ty),* $(,)? }) => {
        #[derive(Debug, Clone)]
        pub struct $name {
            $(pub $field: $ty),*
        }

        impl $crate::shared::runtime::TypedEvent for $name {
            fn event_name() -> &'static str {
                stringify!($name)
            }
        }
    };
}

// ============================================================================
//                    ОБРАБОТЧИКИ СОБЫТИЙ
// ============================================================================

/// Синхронный обработчик события.
pub type SyncEventHandler = Arc<dyn Fn(&Event) + Send + Sync>;

/// Асинхронный обработчик события.
pub type AsyncEventHandler = Arc<
    dyn Fn(Event) -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync
>;

/// Тип обработчика.
enum HandlerType {
    Sync(SyncEventHandler),
    Async(AsyncEventHandler),
}

/// Хранилище обработчика.
struct HandlerEntry {
    id: SubscriptionId,
    handler: HandlerType,
    once: bool,
    priority: i32,
}

// ============================================================================
//                    ШИНА СОБЫТИЙ
// ============================================================================

/// Шина событий для pub/sub паттерна.
pub struct EventBus {
    /// Обработчики по именам событий
    handlers: RwLock<HashMap<String, Vec<HandlerEntry>>>,
    /// Глобальные обработчики (получают все события)
    global_handlers: RwLock<Vec<HandlerEntry>>,
    /// Broadcast канал для асинхронных подписчиков
    broadcaster: broadcast::Sender<Event>,
    /// История событий (если включена)
    history: RwLock<Option<Vec<Event>>>,
    /// Максимальный размер истории
    max_history: AtomicU64,
}

impl EventBus {
    /// Создаёт новую шину событий.
    pub fn new() -> Self {
        let (tx, _) = broadcast::channel(1000);
        Self {
            handlers: RwLock::new(HashMap::new()),
            global_handlers: RwLock::new(Vec::new()),
            broadcaster: tx,
            history: RwLock::new(None),
            max_history: AtomicU64::new(100),
        }
    }

    /// Включает историю событий.
    pub async fn enable_history(&self, max_size: u64) {
        self.max_history.store(max_size, Ordering::SeqCst);
        *self.history.write().await = Some(Vec::new());
    }

    /// Отключает историю.
    pub async fn disable_history(&self) {
        *self.history.write().await = None;
    }

    /// Получает историю событий.
    pub async fn get_history(&self) -> Vec<Event> {
        self.history.read().await.clone().unwrap_or_default()
    }

    // ========================================================================
    //                    ПОДПИСКА
    // ========================================================================

    /// Подписывается на событие (синхронный обработчик).
    pub async fn on<F>(&self, event_name: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.subscribe_sync(event_name, handler, false, 0).await
    }

    /// Подписывается на событие с приоритетом.
    pub async fn on_with_priority<F>(
        &self,
        event_name: impl Into<String>,
        priority: i32,
        handler: F,
    ) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.subscribe_sync(event_name, handler, false, priority).await
    }

    /// Подписывается на событие один раз.
    pub async fn once<F>(&self, event_name: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        self.subscribe_sync(event_name, handler, true, 0).await
    }

    /// Подписывается на событие (асинхронный обработчик).
    pub async fn on_async<F, Fut>(&self, event_name: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        self.subscribe_async(event_name, handler, false, 0).await
    }

    /// Глобальный обработчик (все события).
    pub async fn on_all<F>(&self, handler: F) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let entry = HandlerEntry {
            id,
            handler: HandlerType::Sync(Arc::new(handler)),
            once: false,
            priority: 0,
        };
        self.global_handlers.write().await.push(entry);
        id
    }

    /// Внутренний метод для синхронной подписки.
    async fn subscribe_sync<F>(
        &self,
        event_name: impl Into<String>,
        handler: F,
        once: bool,
        priority: i32,
    ) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let id = SubscriptionId::new();
        let name = event_name.into();
        
        let entry = HandlerEntry {
            id,
            handler: HandlerType::Sync(Arc::new(handler)),
            once,
            priority,
        };

        let mut handlers = self.handlers.write().await;
        let name_key = name.clone();
        handlers.entry(name).or_insert_with(Vec::new).push(entry);
        
        // Сортируем по приоритету (выше = раньше)
        if let Some(list) = handlers.get_mut(&name_key) {
            list.sort_by(|a, b| b.priority.cmp(&a.priority));
        }

        id
    }

    /// Внутренний метод для асинхронной подписки.
    async fn subscribe_async<F, Fut>(
        &self,
        event_name: impl Into<String>,
        handler: F,
        once: bool,
        priority: i32,
    ) -> SubscriptionId
    where
        F: Fn(Event) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let id = SubscriptionId::new();
        let name = event_name.into();
        
        let wrapped: AsyncEventHandler = Arc::new(move |event| {
            Box::pin(handler(event)) as Pin<Box<dyn Future<Output = ()> + Send>>
        });

        let entry = HandlerEntry {
            id,
            handler: HandlerType::Async(wrapped),
            once,
            priority,
        };

        let mut handlers = self.handlers.write().await;
        handlers.entry(name).or_insert_with(Vec::new).push(entry);

        id
    }

    /// Отписывается от события.
    pub async fn off(&self, id: SubscriptionId) -> bool {
        // Ищем в обычных обработчиках
        {
            let mut handlers = self.handlers.write().await;
            for list in handlers.values_mut() {
                if let Some(pos) = list.iter().position(|e| e.id == id) {
                    list.remove(pos);
                    return true;
                }
            }
        }

        // Ищем в глобальных
        {
            let mut global = self.global_handlers.write().await;
            if let Some(pos) = global.iter().position(|e| e.id == id) {
                global.remove(pos);
                return true;
            }
        }

        false
    }

    /// Отписывается от всех обработчиков события.
    pub async fn off_all(&self, event_name: &str) {
        self.handlers.write().await.remove(event_name);
    }

    // ========================================================================
    //                    ЭМИССИЯ
    // ========================================================================

    /// Испускает событие.
    pub async fn emit(&self, event: Event) {
        // Добавляем в историю
        if let Some(history) = self.history.write().await.as_mut() {
            let max = self.max_history.load(Ordering::SeqCst) as usize;
            if history.len() >= max {
                history.remove(0);
            }
            history.push(event.clone());
        }

        // Broadcast для async подписчиков
        let _ = self.broadcaster.send(event.clone());

        // Глобальные обработчики
        {
            let global = self.global_handlers.read().await;
            for entry in global.iter() {
                if let HandlerType::Sync(handler) = &entry.handler {
                    handler(&event);
                }
            }
        }

        // Обработчики конкретного события
        let mut to_remove: Vec<SubscriptionId> = Vec::new();
        
        {
            let handlers = self.handlers.read().await;
            if let Some(list) = handlers.get(&event.name) {
                for entry in list {
                    if event.is_cancelled() {
                        break;
                    }

                    match &entry.handler {
                        HandlerType::Sync(handler) => {
                            handler(&event);
                        }
                        HandlerType::Async(handler) => {
                            let future = handler(event.clone());
                            tokio::spawn(future);
                        }
                    }

                    if entry.once {
                        to_remove.push(entry.id);
                    }
                }
            }
        }

        // Удаляем one-time обработчики
        for id in to_remove {
            self.off(id).await;
        }
    }

    /// Испускает простое событие.
    pub async fn emit_simple(&self, name: impl Into<String>, data: Value) {
        self.emit(Event::new(name, data)).await;
    }

    /// Испускает событие и ждёт обработки.
    pub async fn emit_and_wait(&self, event: Event) {
        // Для синхронной обработки просто вызываем emit
        self.emit(event).await;
    }

    // ========================================================================
    //                    ПОДПИСКА НА ПОТОК
    // ========================================================================

    /// Создаёт получателя broadcast событий.
    pub fn subscribe_broadcast(&self) -> broadcast::Receiver<Event> {
        self.broadcaster.subscribe()
    }

    /// Количество подписчиков на событие.
    pub async fn listener_count(&self, event_name: &str) -> usize {
        self.handlers.read().await
            .get(event_name)
            .map(|l| l.len())
            .unwrap_or(0)
    }

    /// Список всех зарегистрированных событий.
    pub async fn event_names(&self) -> Vec<String> {
        self.handlers.read().await.keys().cloned().collect()
    }
}

impl Default for EventBus {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    EVENT EMITTER (для объектов)
// ============================================================================

/// Генератор событий для встраивания в объекты.
pub struct EventEmitter {
    bus: Arc<EventBus>,
    prefix: Option<String>,
}

impl EventEmitter {
    /// Создаёт новый эмиттер.
    pub fn new() -> Self {
        Self {
            bus: Arc::new(EventBus::new()),
            prefix: None,
        }
    }

    /// Создаёт эмиттер с префиксом для событий.
    pub fn with_prefix(prefix: impl Into<String>) -> Self {
        Self {
            bus: Arc::new(EventBus::new()),
            prefix: Some(prefix.into()),
        }
    }

    /// Создаёт эмиттер с существующей шиной.
    pub fn with_bus(bus: Arc<EventBus>) -> Self {
        Self {
            bus,
            prefix: None,
        }
    }

    fn full_name(&self, name: &str) -> String {
        match &self.prefix {
            Some(p) => format!("{}:{}", p, name),
            None => name.to_string(),
        }
    }

    /// Подписывается на событие.
    pub async fn on<F>(&self, event: impl Into<String>, handler: F) -> SubscriptionId
    where
        F: Fn(&Event) + Send + Sync + 'static,
    {
        let name = self.full_name(&event.into());
        self.bus.on(name, handler).await
    }

    /// Испускает событие.
    pub async fn emit(&self, event: impl Into<String>, data: Value) {
        let name = self.full_name(&event.into());
        self.bus.emit_simple(name, data).await;
    }

    /// Отписывается от события.
    pub async fn off(&self, id: SubscriptionId) -> bool {
        self.bus.off(id).await
    }

    /// Возвращает внутреннюю шину.
    pub fn bus(&self) -> Arc<EventBus> {
        Arc::clone(&self.bus)
    }
}

impl Default for EventEmitter {
    fn default() -> Self {
        Self::new()
    }
}

impl Clone for EventEmitter {
    fn clone(&self) -> Self {
        Self {
            bus: Arc::clone(&self.bus),
            prefix: self.prefix.clone(),
        }
    }
}

// ============================================================================
//                    СТАНДАРТНЫЕ СОБЫТИЯ
// ============================================================================

/// Стандартные системные события.
pub mod system_events {
    pub const STARTUP: &str = "system:startup";
    pub const SHUTDOWN: &str = "system:shutdown";
    pub const ERROR: &str = "system:error";
    pub const WARNING: &str = "system:warning";
    pub const INFO: &str = "system:info";
    pub const DEBUG: &str = "system:debug";
    
    pub const PROGRAM_START: &str = "program:start";
    pub const PROGRAM_END: &str = "program:end";
    pub const PROGRAM_ERROR: &str = "program:error";
    
    pub const VARIABLE_CHANGED: &str = "variable:changed";
    pub const FUNCTION_CALLED: &str = "function:called";
    pub const FUNCTION_RETURNED: &str = "function:returned";
    
    // HTTP Server события
    pub const SERVER_STARTED: &str = "server:started";
    pub const SERVER_STOPPED: &str = "server:stopped";
    pub const CONNECTION_OPENED: &str = "server:connection:opened";
    pub const CONNECTION_CLOSED: &str = "server:connection:closed";
    pub const REQUEST_RECEIVED: &str = "server:request:received";
    pub const RESPONSE_SENT: &str = "server:response:sent";
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::AtomicUsize;

    #[tokio::test]
    async fn test_event_bus_on_emit() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.on("test", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }).await;

        bus.emit_simple("test", Value::Null).await;
        bus.emit_simple("test", Value::Null).await;

        assert_eq!(counter.load(Ordering::SeqCst), 2);
    }

    #[tokio::test]
    async fn test_event_bus_once() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        bus.once("once_test", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }).await;

        bus.emit_simple("once_test", Value::Null).await;
        bus.emit_simple("once_test", Value::Null).await;
        bus.emit_simple("once_test", Value::Null).await;

        // Только один раз
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_bus_off() {
        let bus = EventBus::new();
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        let id = bus.on("off_test", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }).await;

        bus.emit_simple("off_test", Value::Null).await;
        assert_eq!(counter.load(Ordering::SeqCst), 1);

        bus.off(id).await;
        bus.emit_simple("off_test", Value::Null).await;
        
        // Не должно измениться
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_cancelable() {
        let event = Event::new("cancel_test", Value::Null).cancelable();
        
        assert!(!event.is_cancelled());
        assert!(event.cancel());
        assert!(event.is_cancelled());
    }

    #[tokio::test]
    async fn test_event_emitter() {
        let emitter = EventEmitter::with_prefix("component");
        let counter = Arc::new(AtomicUsize::new(0));
        let counter_clone = Arc::clone(&counter);

        emitter.on("action", move |_| {
            counter_clone.fetch_add(1, Ordering::SeqCst);
        }).await;

        emitter.emit("action", Value::Number(42.into())).await;
        
        assert_eq!(counter.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_event_history() {
        let bus = EventBus::new();
        bus.enable_history(10).await;

        bus.emit_simple("event1", Value::Number(1.into())).await;
        bus.emit_simple("event2", Value::Number(2.into())).await;

        let history = bus.get_history().await;
        assert_eq!(history.len(), 2);
        assert_eq!(history[0].name, "event1");
        assert_eq!(history[1].name, "event2");
    }

    #[tokio::test]
    async fn test_priority() {
        use std::sync::Mutex as StdMutex;
        
        let bus = EventBus::new();
        let order = Arc::new(StdMutex::new(Vec::new()));

        let order1 = Arc::clone(&order);
        bus.on_with_priority("priority_test", 1, move |_| {
            let mut o = order1.lock().unwrap();
            o.push(1);
        }).await;

        let order2 = Arc::clone(&order);
        bus.on_with_priority("priority_test", 10, move |_| {
            let mut o = order2.lock().unwrap();
            o.push(10);
        }).await;

        let order3 = Arc::clone(&order);
        bus.on_with_priority("priority_test", 5, move |_| {
            let mut o = order3.lock().unwrap();
            o.push(5);
        }).await;

        bus.emit_simple("priority_test", Value::Null).await;

        let result = order.lock().unwrap();
        // Высокий приоритет первым
        assert_eq!(*result, vec![10, 5, 1]);
    }
}
