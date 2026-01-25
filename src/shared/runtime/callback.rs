// ============================================================================
//                    СИСТЕМА КОЛЛБЭКОВ
// ============================================================================
//
// Предоставляет механизм регистрации и вызова коллбэков между компонентами:
// - Синхронные коллбэки (Fn)
// - Асинхронные коллбэки (async Fn)
// - Типизированные коллбэки с Value
// - Коллбэки с произвольными данными
//
// ============================================================================

use std::collections::HashMap;
use std::sync::Arc;
use std::any::Any;
use std::future::Future;
use std::pin::Pin;
use tokio::sync::Mutex as AsyncMutex;

use crate::shared::types::Value;

// ============================================================================
//                    ТИПЫ КОЛЛБЭКОВ
// ============================================================================

/// Синхронный коллбэк, принимающий и возвращающий Value.
pub type SyncCallback = Arc<dyn Fn(Vec<Value>) -> CallbackResult + Send + Sync>;

/// Асинхронный коллбэк.
pub type AsyncCallback = Arc<
    dyn Fn(Vec<Value>) -> Pin<Box<dyn Future<Output = CallbackResult> + Send>> + Send + Sync
>;

/// Сырой коллбэк с Any типами (для продвинутого использования).
pub type RawCallback = Arc<dyn Fn(Box<dyn Any + Send>) -> Box<dyn Any + Send> + Send + Sync>;

/// Результат вызова коллбэка.
#[derive(Debug, Clone)]
pub enum CallbackResult {
    /// Успешное выполнение с возвращаемым значением
    Ok(Value),
    /// Ошибка выполнения
    Error(String),
    /// Коллбэк ничего не возвращает
    Void,
}

impl CallbackResult {
    pub fn ok(value: Value) -> Self {
        CallbackResult::Ok(value)
    }

    pub fn error(msg: impl Into<String>) -> Self {
        CallbackResult::Error(msg.into())
    }

    pub fn void() -> Self {
        CallbackResult::Void
    }

    pub fn is_ok(&self) -> bool {
        matches!(self, CallbackResult::Ok(_) | CallbackResult::Void)
    }

    pub fn is_error(&self) -> bool {
        matches!(self, CallbackResult::Error(_))
    }

    pub fn unwrap(self) -> Value {
        match self {
            CallbackResult::Ok(v) => v,
            CallbackResult::Void => Value::Null,
            CallbackResult::Error(e) => panic!("CallbackResult::unwrap() вызван на Error: {}", e),
        }
    }

    pub fn unwrap_or(self, default: Value) -> Value {
        match self {
            CallbackResult::Ok(v) => v,
            CallbackResult::Void => Value::Null,
            CallbackResult::Error(_) => default,
        }
    }
}

impl From<Value> for CallbackResult {
    fn from(v: Value) -> Self {
        CallbackResult::Ok(v)
    }
}

impl From<()> for CallbackResult {
    fn from(_: ()) -> Self {
        CallbackResult::Void
    }
}

// ============================================================================
//                    ОПРЕДЕЛЕНИЕ КОЛЛБЭКА
// ============================================================================

/// Метаданные коллбэка.
#[derive(Clone)]
pub struct CallbackDef {
    /// Уникальное имя коллбэка
    pub name: String,
    /// Описание (для документации)
    pub description: String,
    /// Ожидаемое количество аргументов
    pub arity: Option<usize>,
    /// Теги/категории
    pub tags: Vec<String>,
}

impl CallbackDef {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: String::new(),
            arity: None,
            tags: Vec::new(),
        }
    }

    pub fn with_description(mut self, desc: impl Into<String>) -> Self {
        self.description = desc.into();
        self
    }

    pub fn with_arity(mut self, arity: usize) -> Self {
        self.arity = Some(arity);
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }
}

// ============================================================================
//                    ХРАНЕНИЕ КОЛЛБЭКОВ
// ============================================================================

/// Хранит зарегистрированный коллбэк.
enum StoredCallback {
    Sync(SyncCallback),
    Async(AsyncCallback),
    Raw(RawCallback),
}

/// Запись в реестре коллбэков.
struct CallbackEntry {
    def: CallbackDef,
    callback: StoredCallback,
}

// ============================================================================
//                    РЕЕСТР КОЛЛБЭКОВ
// ============================================================================

/// Реестр для управления коллбэками.
/// 
/// # Пример использования
/// 
/// ```ignore
/// let mut registry = CallbackRegistry::new();
/// 
/// // Регистрация синхронного коллбэка
/// registry.register_sync("on_data", |args| {
///     println!("Получены данные: {:?}", args);
///     CallbackResult::Void
/// });
/// 
/// // Вызов коллбэка
/// let result = registry.call_sync("on_data", vec![Value::String("test".into())]);
/// ```
pub struct CallbackRegistry {
    /// Синхронные и Raw коллбэки
    callbacks: HashMap<String, CallbackEntry>,
    /// Асинхронные коллбэки (нужен отдельный Arc<AsyncMutex> для async доступа)
    async_callbacks: Arc<AsyncMutex<HashMap<String, CallbackEntry>>>,
}

impl CallbackRegistry {
    /// Создаёт новый пустой реестр.
    pub fn new() -> Self {
        Self {
            callbacks: HashMap::new(),
            async_callbacks: Arc::new(AsyncMutex::new(HashMap::new())),
        }
    }

    // ========================================================================
    //                    СИНХРОННЫЕ КОЛЛБЭКИ
    // ========================================================================

    /// Регистрирует синхронный коллбэк.
    pub fn register_sync<F>(&mut self, name: impl Into<String>, callback: F)
    where
        F: Fn(Vec<Value>) -> CallbackResult + Send + Sync + 'static,
    {
        let name = name.into();
        self.callbacks.insert(name.clone(), CallbackEntry {
            def: CallbackDef::new(&name),
            callback: StoredCallback::Sync(Arc::new(callback)),
        });
    }

    /// Регистрирует синхронный коллбэк с метаданными.
    pub fn register_sync_with_def<F>(&mut self, def: CallbackDef, callback: F)
    where
        F: Fn(Vec<Value>) -> CallbackResult + Send + Sync + 'static,
    {
        let name = def.name.clone();
        self.callbacks.insert(name, CallbackEntry {
            def,
            callback: StoredCallback::Sync(Arc::new(callback)),
        });
    }

    /// Вызывает синхронный коллбэк.
    pub fn call_sync(&self, name: &str, args: Vec<Value>) -> CallbackResult {
        match self.callbacks.get(name) {
            Some(entry) => {
                // Проверяем арность если указана
                if let Some(arity) = entry.def.arity {
                    if args.len() != arity {
                        return CallbackResult::Error(format!(
                            "Коллбэк '{}' ожидает {} аргументов, получено {}",
                            name, arity, args.len()
                        ));
                    }
                }
                
                match &entry.callback {
                    StoredCallback::Sync(cb) => cb(args),
                    _ => CallbackResult::Error(format!(
                        "Коллбэк '{}' не является синхронным", name
                    )),
                }
            }
            None => CallbackResult::Error(format!("Коллбэк '{}' не найден", name)),
        }
    }

    // ========================================================================
    //                    АСИНХРОННЫЕ КОЛЛБЭКИ
    // ========================================================================

    /// Регистрирует асинхронный коллбэк.
    pub async fn register_async<F, Fut>(&self, name: impl Into<String>, callback: F)
    where
        F: Fn(Vec<Value>) -> Fut + Send + Sync + 'static,
        Fut: Future<Output = CallbackResult> + Send + 'static,
    {
        let name = name.into();
        let wrapped: AsyncCallback = Arc::new(move |args| {
            Box::pin(callback(args)) as Pin<Box<dyn Future<Output = CallbackResult> + Send>>
        });

        let mut async_cbs = self.async_callbacks.lock().await;
        async_cbs.insert(name.clone(), CallbackEntry {
            def: CallbackDef::new(&name),
            callback: StoredCallback::Async(wrapped),
        });
    }

    /// Вызывает асинхронный коллбэк.
    pub async fn call_async(&self, name: &str, args: Vec<Value>) -> CallbackResult {
        let async_cbs = self.async_callbacks.lock().await;
        
        match async_cbs.get(name) {
            Some(entry) => {
                if let Some(arity) = entry.def.arity {
                    if args.len() != arity {
                        return CallbackResult::Error(format!(
                            "Коллбэк '{}' ожидает {} аргументов, получено {}",
                            name, arity, args.len()
                        ));
                    }
                }

                match &entry.callback {
                    StoredCallback::Async(cb) => {
                        let future = cb(args);
                        drop(async_cbs); // Освобождаем lock перед await
                        future.await
                    }
                    _ => CallbackResult::Error(format!(
                        "Коллбэк '{}' не является асинхронным", name
                    )),
                }
            }
            None => CallbackResult::Error(format!("Асинхронный коллбэк '{}' не найден", name)),
        }
    }

    // ========================================================================
    //                    RAW КОЛЛБЭКИ
    // ========================================================================

    /// Регистрирует сырой коллбэк с Any типами.
    pub fn register_raw<F>(&mut self, name: impl Into<String>, callback: F)
    where
        F: Fn(Box<dyn Any + Send>) -> Box<dyn Any + Send> + Send + Sync + 'static,
    {
        let name = name.into();
        self.callbacks.insert(name.clone(), CallbackEntry {
            def: CallbackDef::new(&name),
            callback: StoredCallback::Raw(Arc::new(callback)),
        });
    }

    /// Вызывает сырой коллбэк.
    pub fn call_raw(&self, name: &str, arg: Box<dyn Any + Send>) -> Option<Box<dyn Any + Send>> {
        self.callbacks.get(name).and_then(|entry| {
            match &entry.callback {
                StoredCallback::Raw(cb) => Some(cb(arg)),
                _ => None,
            }
        })
    }

    // ========================================================================
    //                    УТИЛИТЫ
    // ========================================================================

    /// Проверяет существование синхронного коллбэка.
    pub fn has(&self, name: &str) -> bool {
        self.callbacks.contains_key(name)
    }

    /// Проверяет существование асинхронного коллбэка.
    pub async fn has_async(&self, name: &str) -> bool {
        self.async_callbacks.lock().await.contains_key(name)
    }

    /// Удаляет синхронный коллбэк.
    pub fn unregister(&mut self, name: &str) -> bool {
        self.callbacks.remove(name).is_some()
    }

    /// Удаляет асинхронный коллбэк.
    pub async fn unregister_async(&self, name: &str) -> bool {
        self.async_callbacks.lock().await.remove(name).is_some()
    }

    /// Возвращает список всех синхронных коллбэков.
    pub fn list(&self) -> Vec<&CallbackDef> {
        self.callbacks.values().map(|e| &e.def).collect()
    }

    /// Возвращает метаданные коллбэка.
    pub fn get_def(&self, name: &str) -> Option<&CallbackDef> {
        self.callbacks.get(name).map(|e| &e.def)
    }

    /// Очищает все коллбэки.
    pub fn clear(&mut self) {
        self.callbacks.clear();
    }

    /// Очищает все асинхронные коллбэки.
    pub async fn clear_async(&self) {
        self.async_callbacks.lock().await.clear();
    }
}

impl Default for CallbackRegistry {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    CALLBACK BUILDER
// ============================================================================

/// Builder для создания коллбэков с fluent API.
pub struct CallbackBuilder<'a> {
    registry: &'a mut CallbackRegistry,
    def: CallbackDef,
}

impl<'a> CallbackBuilder<'a> {
    pub fn new(registry: &'a mut CallbackRegistry, name: impl Into<String>) -> Self {
        Self {
            registry,
            def: CallbackDef::new(name),
        }
    }

    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.def.description = desc.into();
        self
    }

    pub fn arity(mut self, arity: usize) -> Self {
        self.def.arity = Some(arity);
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.def.tags.push(tag.into());
        self
    }

    pub fn sync<F>(self, callback: F)
    where
        F: Fn(Vec<Value>) -> CallbackResult + Send + Sync + 'static,
    {
        self.registry.register_sync_with_def(self.def, callback);
    }
}

// ============================================================================
//                    УДОБНЫЕ МАКРОСЫ
// ============================================================================

/// Макрос для быстрого создания коллбэка.
#[macro_export]
macro_rules! callback {
    // Простой void коллбэк
    (|$args:ident| $body:expr) => {
        |$args: Vec<$crate::shared::types::Value>| -> $crate::shared::runtime::CallbackResult {
            $body;
            $crate::shared::runtime::CallbackResult::Void
        }
    };
    
    // Коллбэк с возвратом
    (|$args:ident| -> $ret:expr) => {
        |$args: Vec<$crate::shared::types::Value>| -> $crate::shared::runtime::CallbackResult {
            $crate::shared::runtime::CallbackResult::Ok($ret)
        }
    };
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sync_callback() {
        let mut registry = CallbackRegistry::new();
        
        registry.register_sync("test", |args| {
            if args.is_empty() {
                CallbackResult::error("No args")
            } else {
                CallbackResult::ok(args[0].clone())
            }
        });

        let result = registry.call_sync("test", vec![Value::String("hello".into())]);
        assert!(result.is_ok());
        
        match result {
            CallbackResult::Ok(Value::String(s)) => assert_eq!(s, "hello"),
            _ => panic!("Unexpected result"),
        }
    }

    #[test]
    fn test_callback_not_found() {
        let registry = CallbackRegistry::new();
        let result = registry.call_sync("nonexistent", vec![]);
        assert!(result.is_error());
    }

    #[test]
    fn test_arity_check() {
        let mut registry = CallbackRegistry::new();
        
        registry.register_sync_with_def(
            CallbackDef::new("exact2").with_arity(2),
            |_| CallbackResult::Void
        );

        // Неверное количество аргументов
        let result = registry.call_sync("exact2", vec![Value::Null]);
        assert!(result.is_error());
        
        // Верное количество
        let result = registry.call_sync("exact2", vec![Value::Null, Value::Null]);
        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_async_callback() {
        let registry = CallbackRegistry::new();
        
        registry.register_async("async_test", |args| async move {
            tokio::time::sleep(std::time::Duration::from_millis(1)).await;
            if args.is_empty() {
                CallbackResult::error("No args")
            } else {
                CallbackResult::ok(args[0].clone())
            }
        }).await;

        let result = registry.call_async("async_test", vec![Value::Number(42.into())]).await;
        assert!(result.is_ok());
    }
}
