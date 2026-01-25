// ============================================================================
//                    HTTP SERVER
// ============================================================================
//
// Асинхронный HTTP сервер поверх TCP:
// - Graceful shutdown
// - Keep-alive connections
// - TLS поддержка
// - Конфигурируемые лимиты
//
// ============================================================================

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::request::{Request, parse_request};
use super::response::Response;
use super::router::Router;
use super::body::Body;
use crate::shared::libraries::net::tcp::{TcpListener, TcpConnection, TcpConfig};
use crate::shared::libraries::net::tls::{TlsConfig, TlsAcceptor};
use crate::shared::libraries::net::core::ShutdownSignal;
use crate::shared::libraries::net::{NetError, NetResult, NetworkConfig};

// ============================================================================
//                    SERVER CONFIG
// ============================================================================

/// Конфигурация HTTP сервера.
#[derive(Clone)]
pub struct HttpServerConfig {
    /// Максимальный размер запроса (байт)
    pub max_request_size: usize,
    /// Максимальный размер заголовков
    pub max_header_size: usize,
    /// Таймаут чтения запроса
    pub read_timeout: Duration,
    /// Таймаут записи ответа
    pub write_timeout: Duration,
    /// Keep-alive таймаут
    pub keepalive_timeout: Duration,
    /// Максимум запросов на соединение (keep-alive)
    pub max_requests_per_connection: usize,
    /// Размер буфера чтения
    pub read_buffer_size: usize,
}

impl Default for HttpServerConfig {
    fn default() -> Self {
        Self {
            max_request_size: 10 * 1024 * 1024, // 10 MB
            max_header_size: 8 * 1024,           // 8 KB
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            keepalive_timeout: Duration::from_secs(60),
            max_requests_per_connection: 100,
            read_buffer_size: 8192,
        }
    }
}

// ============================================================================
//                    HTTP SERVER
// ============================================================================

/// HTTP сервер.
pub struct HttpServer {
    /// Адрес
    addr: SocketAddr,
    /// Роутер
    router: Arc<Router>,
    /// Конфигурация
    config: HttpServerConfig,
    /// TLS acceptor (опционально)
    tls: Option<Arc<TlsAcceptor>>,
    /// Сигнал shutdown
    shutdown: ShutdownSignal,
    /// Счётчик активных соединений
    active_connections: Arc<AtomicU64>,
    /// Флаг работы
    running: Arc<AtomicBool>,
}

impl HttpServer {
    /// Запускает сервер и блокирует до shutdown.
    pub async fn run(self) -> NetResult<()> {
        let listener = TcpListener::bind(self.addr, TcpConfig::default()).await?;
        
        eprintln!("🚀 HTTP Server listening on {}", self.addr);
        if self.tls.is_some() {
            eprintln!("🔒 TLS enabled");
        }

        self.running.store(true, Ordering::SeqCst);

        loop {
            tokio::select! {
                result = listener.accept() => {
                    match result {
                        Ok(conn) => {
                            let server = HttpConnection {
                                router: Arc::clone(&self.router),
                                config: self.config.clone(),
                                tls: self.tls.clone(),
                                shutdown: self.shutdown.subscribe(),
                                active_connections: Arc::clone(&self.active_connections),
                            };

                            self.active_connections.fetch_add(1, Ordering::SeqCst);
                            
                            tokio::spawn(async move {
                                if let Err(e) = server.handle(conn).await {
                                    eprintln!("Connection error: {}", e);
                                }
                            });
                        }
                        Err(e) => {
                            if !self.shutdown.is_shutdown() {
                                eprintln!("Accept error: {}", e);
                            }
                        }
                    }
                }
                _ = self.shutdown.wait() => {
                    eprintln!("🛑 Shutting down...");
                    break;
                }
            }
        }

        // Ждём завершения активных соединений
        self.wait_for_connections(Duration::from_secs(30)).await;
        
        self.running.store(false, Ordering::SeqCst);
        eprintln!("✅ Server stopped");

        Ok(())
    }

    /// Ожидает завершения всех соединений.
    async fn wait_for_connections(&self, timeout: Duration) {
        let start = std::time::Instant::now();
        
        while self.active_connections.load(Ordering::SeqCst) > 0 {
            if start.elapsed() > timeout {
                eprintln!("⚠️ Timeout waiting for connections, {} still active",
                    self.active_connections.load(Ordering::SeqCst));
                break;
            }
            tokio::time::sleep(Duration::from_millis(100)).await;
        }
    }

    /// Инициирует graceful shutdown.
    pub fn shutdown(&self) {
        self.shutdown.shutdown();
    }

    /// Возвращает сигнал shutdown для подписки.
    pub fn shutdown_signal(&self) -> ShutdownSignal {
        self.shutdown.subscribe()
    }

    /// Проверяет, работает ли сервер.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }

    /// Количество активных соединений.
    pub fn active_connections(&self) -> u64 {
        self.active_connections.load(Ordering::SeqCst)
    }

    /// Адрес сервера.
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }
}

// ============================================================================
//                    HTTP CONNECTION
// ============================================================================

/// Обработчик одного HTTP соединения.
struct HttpConnection {
    router: Arc<Router>,
    config: HttpServerConfig,
    tls: Option<Arc<TlsAcceptor>>,
    shutdown: ShutdownSignal,
    active_connections: Arc<AtomicU64>,
}

impl HttpConnection {
    async fn handle(self, mut conn: TcpConnection) -> NetResult<()> {
        let peer_addr = conn.peer_addr();
        let mut requests_count = 0;

        loop {
            // Проверяем shutdown
            if self.shutdown.is_shutdown() {
                break;
            }

            // Проверяем лимит запросов
            if requests_count >= self.config.max_requests_per_connection {
                break;
            }

            // Читаем запрос с таймаутом
            let request_result = tokio::time::timeout(
                self.config.read_timeout,
                self.read_request(&mut conn, peer_addr)
            ).await;

            match request_result {
                Ok(Ok(Some(request))) => {
                    requests_count += 1;

                    // Проверяем Connection: close
                    let close_after = request.headers()
                        .get("connection")
                        .map(|v| v.as_str().eq_ignore_ascii_case("close"))
                        .unwrap_or(false);

                    // Обрабатываем запрос
                    let mut response = self.router.handle(request).await;

                    // Отправляем ответ
                    let response_bytes = response.to_bytes().await?;
                    conn.write_all(&response_bytes).await?;
                    conn.flush().await?;

                    if close_after {
                        break;
                    }
                }
                Ok(Ok(None)) => {
                    // Соединение закрыто клиентом
                    break;
                }
                Ok(Err(e)) => {
                    // Ошибка парсинга — отправляем 400
                    let response = Response::bad_request(e.to_string());
                    let mut response = response;
                    if let Ok(bytes) = response.to_bytes().await {
                        let _ = conn.write_all(&bytes).await;
                    }
                    break;
                }
                Err(_) => {
                    // Таймаут
                    break;
                }
            }
        }

        // Закрываем соединение
        let _ = conn.close().await;
        self.active_connections.fetch_sub(1, Ordering::SeqCst);

        Ok(())
    }

    async fn read_request(
        &self,
        conn: &mut TcpConnection,
        peer_addr: SocketAddr,
    ) -> NetResult<Option<Request>> {
        let mut buffer = Vec::with_capacity(self.config.read_buffer_size);
        let mut temp = [0u8; 4096];

        // Читаем заголовки
        loop {
            let n = conn.read(&mut temp).await?;
            if n == 0 {
                if buffer.is_empty() {
                    return Ok(None); // Чистое закрытие
                }
                return Err(NetError::ConnectionClosed);
            }

            buffer.extend_from_slice(&temp[..n]);

            // Проверяем лимит заголовков
            if buffer.len() > self.config.max_header_size {
                return Err(NetError::Http("Headers too large".into()));
            }

            // Ищем конец заголовков
            if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
                break;
            }
        }

        // Парсим запрос
        let mut request = parse_request(&buffer, Some(peer_addr))?;

        // Читаем тело если есть Content-Length
        if let Some(content_length) = request.content_length() {
            if content_length > self.config.max_request_size {
                return Err(NetError::Http("Request body too large".into()));
            }

            // Находим начало тела
            let headers_end = buffer.windows(4)
                .position(|w| w == b"\r\n\r\n")
                .unwrap() + 4;

            let mut body = buffer[headers_end..].to_vec();

            // Дочитываем остаток тела
            while body.len() < content_length {
                let remaining = content_length - body.len();
                let to_read = remaining.min(temp.len());
                let n = conn.read(&mut temp[..to_read]).await?;
                if n == 0 {
                    return Err(NetError::ConnectionClosed);
                }
                body.extend_from_slice(&temp[..n]);
            }

            request.set_body(Body::from(body));
        }

        Ok(Some(request))
    }
}

// ============================================================================
//                    HTTP SERVER BUILDER
// ============================================================================

/// Билдер для HttpServer.
pub struct HttpServerBuilder {
    addr: Option<SocketAddr>,
    router: Option<Router>,
    config: HttpServerConfig,
    tls_config: Option<TlsConfig>,
    network_config: NetworkConfig,
}

impl HttpServerBuilder {
    pub fn new(network_config: NetworkConfig) -> Self {
        Self {
            addr: None,
            router: None,
            config: HttpServerConfig::default(),
            tls_config: None,
            network_config,
        }
    }

    /// Устанавливает адрес.
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        let addr_str = addr.into();
        self.addr = addr_str.parse().ok();
        self
    }

    /// Устанавливает адрес из SocketAddr.
    pub fn bind_addr(mut self, addr: SocketAddr) -> Self {
        self.addr = Some(addr);
        self
    }

    /// Устанавливает роутер.
    pub fn router(mut self, router: Router) -> Self {
        self.router = Some(router);
        self
    }

    /// Устанавливает конфигурацию.
    pub fn config(mut self, config: HttpServerConfig) -> Self {
        self.config = config;
        self
    }

    /// Включает TLS.
    pub fn tls(mut self, config: TlsConfig) -> Self {
        self.tls_config = Some(config);
        self
    }

    /// Устанавливает максимальный размер запроса.
    pub fn max_request_size(mut self, size: usize) -> Self {
        self.config.max_request_size = size;
        self
    }

    /// Устанавливает таймаут чтения.
    pub fn read_timeout(mut self, timeout: Duration) -> Self {
        self.config.read_timeout = timeout;
        self
    }

    /// Устанавливает таймаут записи.
    pub fn write_timeout(mut self, timeout: Duration) -> Self {
        self.config.write_timeout = timeout;
        self
    }

    /// Устанавливает keep-alive таймаут.
    pub fn keepalive_timeout(mut self, timeout: Duration) -> Self {
        self.config.keepalive_timeout = timeout;
        self
    }

    /// Строит и возвращает сервер.
    pub fn build(self) -> NetResult<HttpServer> {
        let addr = self.addr
            .ok_or_else(|| NetError::InvalidAddress("Address not specified".into()))?;

        let router = self.router.unwrap_or_else(Router::new);

        let tls = if let Some(tls_config) = self.tls_config {
            Some(Arc::new(TlsAcceptor::new(tls_config)?))
        } else {
            None
        };

        Ok(HttpServer {
            addr,
            router: Arc::new(router),
            config: self.config,
            tls,
            shutdown: ShutdownSignal::new(),
            active_connections: Arc::new(AtomicU64::new(0)),
            running: Arc::new(AtomicBool::new(false)),
        })
    }

    /// Строит и сразу запускает сервер.
    pub async fn serve(self) -> NetResult<()> {
        let server = self.build()?;
        server.run().await
    }
}

// ============================================================================
//                    QUICK SERVER FUNCTION
// ============================================================================

/// Быстрый запуск HTTP сервера.
/// 
/// # Пример
/// ```rust
/// use kumir3_net::http::{serve, Router, Response};
/// 
/// let router = Router::new()
///     .get("/", |_| async { Response::text("Hello!") });
/// 
/// serve("0.0.0.0:8080", router).await?;
/// ```
pub async fn serve(addr: impl Into<String>, router: Router) -> NetResult<()> {
    HttpServerBuilder::new(NetworkConfig::default())
        .bind(addr)
        .router(router)
        .serve()
        .await
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_server_config_defaults() {
        let config = HttpServerConfig::default();
        assert_eq!(config.max_request_size, 10 * 1024 * 1024);
        assert_eq!(config.read_timeout, Duration::from_secs(30));
    }

    #[test]
    fn test_server_builder() {
        let builder = HttpServerBuilder::new(NetworkConfig::default())
            .bind("127.0.0.1:8080")
            .max_request_size(1024 * 1024)
            .read_timeout(Duration::from_secs(10));

        assert!(builder.addr.is_some());
        assert_eq!(builder.config.max_request_size, 1024 * 1024);
    }
}
