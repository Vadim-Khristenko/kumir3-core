// ============================================================================
//                    HTTP MIDDLEWARE
// ============================================================================
//
// Система middleware для обработки запросов:
// - Logging
// - CORS
// - Authentication
// - Rate limiting
// - Compression
// - Timeout
//
// ============================================================================

use std::future::Future;

use std::sync::Arc;
use std::time::{Duration, Instant};

use super::request::Request;
use super::response::Response;
use super::router::{Handler, BoxFuture};
use super::types::{HeaderName, StatusCode};

// ============================================================================
//                    MIDDLEWARE TRAIT
// ============================================================================

/// Трейт для middleware.
pub trait Middleware: Send + Sync {
    /// Обрабатывает запрос, вызывая next для продолжения цепочки.
    fn handle(
        &self,
        req: Request,
        next: Next,
    ) -> BoxFuture<Response>;
}

/// Следующий обработчик в цепочке.
pub struct Next {
    inner: Arc<dyn Handler>,
    middleware: Arc<[Arc<dyn Middleware>]>,
    index: usize,
}

impl Next {
    /// Вызывает следующий middleware или финальный handler.
    pub async fn run(self, req: Request) -> Response {
        if self.index < self.middleware.len() {
            let mw = self.middleware[self.index].clone();
            let next = Next {
                inner: self.inner,
                middleware: self.middleware,
                index: self.index + 1,
            };
            mw.handle(req, next).await
        } else {
            self.inner.call(req).await
        }
    }
}

// ============================================================================
//                    MIDDLEWARE CHAIN
// ============================================================================

/// Цепочка middleware.
pub struct MiddlewareChain {
    handler: Arc<dyn Handler>,
    middleware: Vec<Arc<dyn Middleware>>,
}

impl MiddlewareChain {
    /// Создаёт цепочку с финальным обработчиком.
    pub fn new(handler: Arc<dyn Handler>) -> Self {
        Self {
            handler,
            middleware: Vec::new(),
        }
    }

    /// Добавляет middleware в начало цепочки.
    pub fn with(mut self, mw: Arc<dyn Middleware>) -> Self {
        self.middleware.insert(0, mw);
        self
    }

    /// Запускает цепочку.
    pub async fn run(self, req: Request) -> Response {
        let next = Next {
            inner: self.handler,
            middleware: self.middleware.into(),
            index: 0,
        };
        next.run(req).await
    }
}

// ============================================================================
//                    BUILT-IN MIDDLEWARE
// ============================================================================

// -----------------------------------------------------------------------------
// Logging Middleware
// -----------------------------------------------------------------------------

/// Middleware для логирования запросов.
pub struct LoggingMiddleware {
    /// Логировать тело запроса
    log_body: bool,
    /// Логировать заголовки
    log_headers: bool,
}

impl LoggingMiddleware {
    pub fn new() -> Self {
        Self {
            log_body: false,
            log_headers: false,
        }
    }

    pub fn with_body(mut self) -> Self {
        self.log_body = true;
        self
    }

    pub fn with_headers(mut self) -> Self {
        self.log_headers = true;
        self
    }
}

impl Default for LoggingMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for LoggingMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        Box::pin(async move {
            let start = Instant::now();
            let method = req.method();
            let path = req.path().to_string();
            let remote = req.remote_addr()
                .map(|a| a.to_string())
                .unwrap_or_else(|| "-".to_string());

            let response = next.run(req).await;

            let duration = start.elapsed();
            let status = response.status();

            // В реальном приложении здесь был бы настоящий логгер
            eprintln!(
                "{} {} {} {} {:?}",
                remote, method, path, status.code(), duration
            );

            response
        })
    }
}

// -----------------------------------------------------------------------------
// CORS Middleware
// -----------------------------------------------------------------------------

/// Конфигурация CORS.
#[derive(Clone)]
pub struct CorsConfig {
    /// Разрешённые origins (* для всех)
    pub allow_origins: Vec<String>,
    /// Разрешённые методы
    pub allow_methods: Vec<String>,
    /// Разрешённые заголовки
    pub allow_headers: Vec<String>,
    /// Expose headers
    pub expose_headers: Vec<String>,
    /// Allow credentials
    pub allow_credentials: bool,
    /// Max age для preflight
    pub max_age: Option<u64>,
}

impl Default for CorsConfig {
    fn default() -> Self {
        Self {
            allow_origins: vec!["*".to_string()],
            allow_methods: vec![
                "GET".to_string(),
                "POST".to_string(),
                "PUT".to_string(),
                "DELETE".to_string(),
                "PATCH".to_string(),
                "OPTIONS".to_string(),
            ],
            allow_headers: vec!["*".to_string()],
            expose_headers: Vec::new(),
            allow_credentials: false,
            max_age: Some(86400),
        }
    }
}

impl CorsConfig {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn allow_origin(mut self, origin: impl Into<String>) -> Self {
        self.allow_origins.push(origin.into());
        self
    }

    pub fn allow_any_origin(mut self) -> Self {
        self.allow_origins = vec!["*".to_string()];
        self
    }

    pub fn allow_methods(mut self, methods: Vec<&str>) -> Self {
        self.allow_methods = methods.into_iter().map(String::from).collect();
        self
    }

    pub fn allow_headers(mut self, headers: Vec<&str>) -> Self {
        self.allow_headers = headers.into_iter().map(String::from).collect();
        self
    }

    pub fn allow_credentials(mut self) -> Self {
        self.allow_credentials = true;
        self
    }

    pub fn max_age(mut self, seconds: u64) -> Self {
        self.max_age = Some(seconds);
        self
    }
}

/// CORS Middleware.
pub struct CorsMiddleware {
    config: CorsConfig,
}

impl CorsMiddleware {
    pub fn new(config: CorsConfig) -> Self {
        Self { config }
    }

    pub fn permissive() -> Self {
        Self::new(CorsConfig::default())
    }
}

impl Middleware for CorsMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        let config = self.config.clone();
        
        Box::pin(async move {
            // Handle preflight
            if req.method() == super::types::Method::OPTIONS {
                let mut response = Response::no_content();
                apply_cors_headers(&mut response, &config);
                return response;
            }

            let mut response = next.run(req).await;
            apply_cors_headers(&mut response, &config);
            response
        })
    }
}

fn apply_cors_headers(response: &mut Response, config: &CorsConfig) {
    let headers = response.headers_mut();
    
    headers.insert(
        HeaderName::CORS_ORIGIN,
        config.allow_origins.join(", ")
    );
    
    headers.insert(
        HeaderName::CORS_METHODS,
        config.allow_methods.join(", ")
    );
    
    headers.insert(
        HeaderName::CORS_HEADERS,
        config.allow_headers.join(", ")
    );

    if config.allow_credentials {
        headers.insert("access-control-allow-credentials", "true");
    }

    if let Some(max_age) = config.max_age {
        headers.insert("access-control-max-age", max_age.to_string());
    }

    if !config.expose_headers.is_empty() {
        headers.insert(
            "access-control-expose-headers",
            config.expose_headers.join(", ")
        );
    }
}

// -----------------------------------------------------------------------------
// Timeout Middleware
// -----------------------------------------------------------------------------

/// Middleware для таймаута запросов.
pub struct TimeoutMiddleware {
    timeout: Duration,
}

impl TimeoutMiddleware {
    pub fn new(timeout: Duration) -> Self {
        Self { timeout }
    }

    pub fn seconds(secs: u64) -> Self {
        Self::new(Duration::from_secs(secs))
    }
}

impl Middleware for TimeoutMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        let timeout = self.timeout;
        
        Box::pin(async move {
            match tokio::time::timeout(timeout, next.run(req)).await {
                Ok(response) => response,
                Err(_) => Response::error(
                    StatusCode::GATEWAY_TIMEOUT,
                    "Request timeout"
                ),
            }
        })
    }
}

// -----------------------------------------------------------------------------
// Request ID Middleware
// -----------------------------------------------------------------------------

/// Middleware для добавления X-Request-ID.
pub struct RequestIdMiddleware;

impl RequestIdMiddleware {
    pub fn new() -> Self {
        Self
    }
}

impl Default for RequestIdMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for RequestIdMiddleware {
    fn handle(&self, mut req: Request, next: Next) -> BoxFuture<Response> {
        Box::pin(async move {
            // Генерируем или используем существующий ID
            let request_id = req.headers()
                .get(HeaderName::X_REQUEST_ID)
                .map(|v| v.as_str().to_string())
                .unwrap_or_else(|| generate_request_id());

            // Сохраняем в extensions
            req.extensions_mut().insert(RequestId(request_id.clone()));

            let mut response = next.run(req).await;
            
            // Добавляем в ответ
            response.headers_mut().insert(HeaderName::X_REQUEST_ID, request_id);
            
            response
        })
    }
}

/// Request ID для извлечения в handler'ах.
#[derive(Debug, Clone)]
pub struct RequestId(pub String);

fn generate_request_id() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("{:x}", timestamp)
}

// -----------------------------------------------------------------------------
// Compression Middleware
// -----------------------------------------------------------------------------

/// Middleware для сжатия ответов (заглушка).
pub struct CompressionMiddleware {
    /// Минимальный размер для сжатия
    min_size: usize,
}

impl CompressionMiddleware {
    pub fn new() -> Self {
        Self { min_size: 1024 }
    }

    pub fn min_size(mut self, size: usize) -> Self {
        self.min_size = size;
        self
    }
}

impl Default for CompressionMiddleware {
    fn default() -> Self {
        Self::new()
    }
}

impl Middleware for CompressionMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        // Проверяем Accept-Encoding
        let _accepts_gzip = req.headers()
            .get(HeaderName::ACCEPT_ENCODING)
            .map(|v| v.as_str().contains("gzip"))
            .unwrap_or(false);

        Box::pin(async move {
            let response = next.run(req).await;
            
            // TODO: Реализовать сжатие если accepts_gzip && body.len() >= min_size
            // Пока возвращаем как есть
            response
        })
    }
}

// -----------------------------------------------------------------------------
// Rate Limiting Middleware (Simple)
// -----------------------------------------------------------------------------

use std::collections::HashMap;
use tokio::sync::RwLock;

/// Простой rate limiter (в памяти).
pub struct RateLimitMiddleware {
    /// Максимум запросов
    max_requests: u32,
    /// Окно в секундах
    window_secs: u64,
    /// Состояние (IP -> (count, window_start))
    state: Arc<RwLock<HashMap<String, (u32, Instant)>>>,
}

impl RateLimitMiddleware {
    pub fn new(max_requests: u32, window_secs: u64) -> Self {
        Self {
            max_requests,
            window_secs,
            state: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    /// 100 запросов в минуту.
    pub fn per_minute(max: u32) -> Self {
        Self::new(max, 60)
    }
}

impl Middleware for RateLimitMiddleware {
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        let max_requests = self.max_requests;
        let window_secs = self.window_secs;
        let state = Arc::clone(&self.state);
        
        Box::pin(async move {
            let key = req.remote_addr()
                .map(|a| a.ip().to_string())
                .unwrap_or_else(|| "unknown".to_string());

            let now = Instant::now();
            let window = Duration::from_secs(window_secs);

            {
                let mut state = state.write().await;
                let entry = state.entry(key.clone()).or_insert((0, now));

                // Сбрасываем если окно прошло
                if now.duration_since(entry.1) >= window {
                    entry.0 = 0;
                    entry.1 = now;
                }

                // Проверяем лимит
                if entry.0 >= max_requests {
                    let retry_after = window.as_secs() - now.duration_since(entry.1).as_secs();
                    let mut response = Response::error(
                        StatusCode::TOO_MANY_REQUESTS,
                        "Rate limit exceeded"
                    );
                    response.headers_mut().insert("Retry-After", retry_after.to_string());
                    return response;
                }

                entry.0 += 1;
            }

            next.run(req).await
        })
    }
}

// ============================================================================
//                    FUNCTION MIDDLEWARE
// ============================================================================

/// Создаёт middleware из функции.
pub fn from_fn<F, Fut>(f: F) -> FnMiddleware<F>
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    FnMiddleware { f }
}

pub struct FnMiddleware<F> {
    f: F,
}

impl<F, Fut> Middleware for FnMiddleware<F>
where
    F: Fn(Request, Next) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn handle(&self, req: Request, next: Next) -> BoxFuture<Response> {
        Box::pin((self.f)(req, next))
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_cors_config() {
        let config = CorsConfig::new()
            .allow_origin("http://localhost:3000")
            .allow_credentials()
            .max_age(3600);

        assert!(config.allow_credentials);
        assert_eq!(config.max_age, Some(3600));
    }

    #[test]
    fn test_request_id_generation() {
        let id1 = generate_request_id();
        let id2 = generate_request_id();
        assert_ne!(id1, id2);
    }
}
