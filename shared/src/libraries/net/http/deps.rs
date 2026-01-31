// ============================================================================
//                    DEPENDENCY INJECTION
// ============================================================================
//
// FastAPI-подобная система DI:
// - FromRequest трейт для извлечения из запроса
// - State для shared state
// - Provide/Inject patterns
//
// ============================================================================

use std::any::{Any, TypeId};
use std::collections::HashMap;
use std::future::Future;

use std::sync::Arc;

use super::request::Request;
use super::response::Response;
use super::types::StatusCode;


// ============================================================================
//                    FROM REQUEST TRAIT
// ============================================================================

/// Трейт для извлечения данных из запроса.
/// 
/// Реализуется для типов, которые могут быть извлечены из HTTP запроса.
pub trait FromRequest: Sized + Send {
    /// Тип ошибки при извлечении.
    type Error: Into<Response>;

    /// Извлекает значение из запроса.
    fn from_request(req: &mut Request) -> impl Future<Output = Result<Self, Self::Error>> + Send;
}

/// Rejection - ошибка извлечения.
#[derive(Debug, Clone)]
pub struct Rejection {
    pub status: StatusCode,
    pub message: String,
}

impl Rejection {
    pub fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }

    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::new(StatusCode::BAD_REQUEST, message)
    }

    pub fn not_found(message: impl Into<String>) -> Self {
        Self::new(StatusCode::NOT_FOUND, message)
    }

    pub fn internal(message: impl Into<String>) -> Self {
        Self::new(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        Self::new(StatusCode::UNAUTHORIZED, message)
    }
}

impl From<Rejection> for Response {
    fn from(rejection: Rejection) -> Self {
        Response::error(rejection.status, rejection.message)
    }
}

// ============================================================================
//                    STATE
// ============================================================================

/// Shared state для передачи между handlers.
/// 
/// # Пример
/// ```rust
/// struct AppState {
///     db: Database,
/// }
/// 
/// async fn handler(State(state): State<AppState>) -> Response {
///     // использование state.db
/// }
/// ```
#[derive(Clone)]
pub struct State<T: Clone + Send + Sync + 'static>(pub T);

impl<T: Clone + Send + Sync + 'static> State<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn inner(&self) -> &T {
        &self.0
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T: Clone + Send + Sync + 'static> std::ops::Deref for State<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: Clone + Send + Sync + 'static> FromRequest for State<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.extensions()
            .get::<State<T>>()
            .cloned()
            .ok_or_else(|| Rejection::internal("State not found"))
    }
}

// ============================================================================
//                    DEPENDENCY CONTAINER
// ============================================================================

/// Контейнер зависимостей.
pub struct DependencyContainer {
    singletons: HashMap<TypeId, Arc<dyn Any + Send + Sync>>,
}

impl DependencyContainer {
    pub fn new() -> Self {
        Self {
            singletons: HashMap::new(),
        }
    }

    /// Регистрирует singleton.
    pub fn singleton<T: Clone + Send + Sync + 'static>(mut self, value: T) -> Self {
        self.singletons.insert(
            TypeId::of::<T>(),
            Arc::new(value) as Arc<dyn Any + Send + Sync>,
        );
        self
    }

    /// Получает singleton.
    pub fn get<T: Clone + Send + Sync + 'static>(&self) -> Option<T> {
        self.singletons
            .get(&TypeId::of::<T>())
            .and_then(|v| v.downcast_ref::<T>())
            .cloned()
    }

    /// Проверяет наличие зависимости.
    pub fn has<T: Send + Sync + 'static>(&self) -> bool {
        self.singletons.contains_key(&TypeId::of::<T>())
    }
}

impl Default for DependencyContainer {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    DEPENDENCY TRAIT
// ============================================================================

/// Трейт для зависимостей, которые могут быть разрешены.
pub trait Dependency: Sized + Send + Sync {
    /// Разрешает зависимость из контейнера.
    fn resolve(container: &DependencyContainer) -> Option<Self>;
}

// Автоматическая реализация для Clone + Send + Sync
impl<T: Clone + Send + Sync + 'static> Dependency for T {
    fn resolve(container: &DependencyContainer) -> Option<Self> {
        container.get::<T>()
    }
}

// ============================================================================
//                    PROVIDES / INJECT
// ============================================================================

/// Маркер для значения, предоставляемого DI.
pub struct Provide<T>(pub T);

impl<T> Provide<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Provide<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

/// Inject маркер для внедрения зависимости.
pub struct Inject<T>(pub T);

impl<T> Inject<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Inject<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
//                    STANDARD FROM_REQUEST IMPLEMENTATIONS
// ============================================================================

/// String из тела.
impl FromRequest for String {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let body = req.bytes().await
            .map_err(|e| Rejection::bad_request(e.to_string()))?;
        String::from_utf8(body)
            .map_err(|e| Rejection::bad_request(format!("Invalid UTF-8: {}", e)))
    }
}

/// Vec<u8> из тела.
impl FromRequest for Vec<u8> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.bytes().await
            .map_err(|e| Rejection::bad_request(e.to_string()))
    }
}

/// Option<T> — не возвращает ошибку если не найдено.
impl<T: FromRequest> FromRequest for Option<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        match T::from_request(req).await {
            Ok(value) => Ok(Some(value)),
            Err(_) => Ok(None),
        }
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rejection() {
        let rejection = Rejection::bad_request("Invalid data");
        assert_eq!(rejection.status, StatusCode::BAD_REQUEST);
        assert_eq!(rejection.message, "Invalid data");
    }

    #[test]
    fn test_state() {
        let state = State::new(42);
        assert_eq!(*state, 42);
        assert_eq!(state.into_inner(), 42);
    }

    #[test]
    fn test_dependency_container() {
        #[derive(Clone)]
        struct Config {
            value: i32,
        }

        let container = DependencyContainer::new()
            .singleton(Config { value: 42 });

        let config: Option<Config> = container.get();
        assert!(config.is_some());
        assert_eq!(config.unwrap().value, 42);
    }
}
