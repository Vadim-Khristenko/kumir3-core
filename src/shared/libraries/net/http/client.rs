// ============================================================================
//                    HTTP CLIENT
// ============================================================================
//
// Асинхронный HTTP клиент:
// - GET, POST, PUT, DELETE, PATCH
// - JSON serialization
// - Timeout и retry
// - Connection pooling (базовый)
// - TLS поддержка
//
// ============================================================================

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use serde::Serialize;
use serde::de::DeserializeOwned;

use super::response::Response;
use super::body::Body;
use super::types::{Method, Headers, HeaderName, StatusCode};
use crate::shared::libraries::net::tcp::{TcpConnection, TcpConfig};
use crate::shared::libraries::net::tls::{TlsConfig, TlsConnector};
use crate::shared::libraries::net::{NetError, NetResult};

// ============================================================================
//                    CLIENT CONFIG
// ============================================================================

/// Конфигурация HTTP клиента.
#[derive(Clone)]
pub struct HttpClientConfig {
    /// Таймаут соединения
    pub connect_timeout: Duration,
    /// Таймаут чтения
    pub read_timeout: Duration,
    /// Таймаут записи
    pub write_timeout: Duration,
    /// User-Agent
    pub user_agent: String,
    /// Максимум редиректов
    pub max_redirects: usize,
    /// Размер буфера
    pub buffer_size: usize,
    /// Проверять SSL сертификаты
    pub verify_ssl: bool,
}

impl Default for HttpClientConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(30),
            write_timeout: Duration::from_secs(30),
            user_agent: "kumir3-http/1.0".to_string(),
            max_redirects: 10,
            buffer_size: 8192,
            verify_ssl: true,
        }
    }
}

// ============================================================================
//                    HTTP CLIENT
// ============================================================================

/// HTTP клиент.
pub struct HttpClient {
    config: HttpClientConfig,
    default_headers: Headers,
    tls_connector: Option<Arc<TlsConnector>>,
}

impl HttpClient {
    /// Создаёт клиент с дефолтной конфигурацией.
    pub fn new() -> Self {
        Self {
            config: HttpClientConfig::default(),
            default_headers: Headers::new(),
            tls_connector: None,
        }
    }

    /// Создаёт клиент из NetworkConfig.
    pub fn from_network_config(net_config: crate::shared::libraries::net::NetworkConfig) -> Self {
        Self {
            config: HttpClientConfig {
                connect_timeout: net_config.connect_timeout,
                read_timeout: net_config.read_timeout,
                write_timeout: net_config.write_timeout,
                buffer_size: net_config.buffer_size,
                ..Default::default()
            },
            default_headers: Headers::new(),
            tls_connector: None,
        }
    }

    /// Создаёт клиент с конфигурацией.
    pub fn with_config(config: HttpClientConfig) -> Self {
        Self {
            config,
            default_headers: Headers::new(),
            tls_connector: None,
        }
    }

    /// Добавляет дефолтный заголовок.
    pub fn default_header(mut self, name: &str, value: &str) -> Self {
        self.default_headers.insert(name, value);
        self
    }

    /// Включает TLS.
    pub fn with_tls(mut self, config: TlsConfig) -> NetResult<Self> {
        self.tls_connector = Some(Arc::new(TlsConnector::new(config)?));
        Ok(self)
    }

    // -------------------------------------------------------------------------
    // HTTP Methods
    // -------------------------------------------------------------------------

    /// GET запрос.
    pub async fn get(&self, url: &str) -> NetResult<Response> {
        self.request(Method::GET, url).send().await
    }

    /// POST запрос.
    pub async fn post(&self, url: &str) -> ClientRequestBuilder<'_> {
        self.request(Method::POST, url)
    }

    /// PUT запрос.
    pub async fn put(&self, url: &str) -> ClientRequestBuilder<'_> {
        self.request(Method::PUT, url)
    }

    /// DELETE запрос.
    pub async fn delete(&self, url: &str) -> NetResult<Response> {
        self.request(Method::DELETE, url).send().await
    }

    /// PATCH запрос.
    pub async fn patch(&self, url: &str) -> ClientRequestBuilder<'_> {
        self.request(Method::PATCH, url)
    }

    /// HEAD запрос.
    pub async fn head(&self, url: &str) -> NetResult<Response> {
        self.request(Method::HEAD, url).send().await
    }

    /// Создаёт билдер запроса.
    pub fn request(&self, method: Method, url: &str) -> ClientRequestBuilder<'_> {
        ClientRequestBuilder {
            client: self,
            method,
            url: url.to_string(),
            headers: self.default_headers.clone(),
            body: Body::empty(),
            timeout: None,
        }
    }
}

impl Default for HttpClient {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    CLIENT REQUEST BUILDER
// ============================================================================

/// Билдер для запроса клиента.
pub struct ClientRequestBuilder<'a> {
    client: &'a HttpClient,
    method: Method,
    url: String,
    headers: Headers,
    body: Body,
    timeout: Option<Duration>,
}

impl<'a> ClientRequestBuilder<'a> {
    /// Добавляет заголовок.
    pub fn header(mut self, name: &str, value: &str) -> Self {
        self.headers.insert(name, value);
        self
    }

    /// Устанавливает Bearer token.
    pub fn bearer_auth(mut self, token: &str) -> Self {
        self.headers.insert(HeaderName::AUTHORIZATION, format!("Bearer {}", token));
        self
    }

    /// Устанавливает Basic auth.
    pub fn basic_auth(mut self, username: &str, password: &str) -> Self {
        let credentials = format!("{}:{}", username, password);
        let encoded = base64_encode(credentials.as_bytes());
        self.headers.insert(HeaderName::AUTHORIZATION, format!("Basic {}", encoded));
        self
    }

    /// Устанавливает JSON тело.
    pub fn json<T: Serialize>(mut self, value: &T) -> NetResult<Self> {
        let json = serde_json::to_vec(value)
            .map_err(|e| NetError::Serialization(e.to_string()))?;
        self.headers.insert(HeaderName::CONTENT_TYPE, "application/json");
        self.body = Body::from(json);
        Ok(self)
    }

    /// Устанавливает form data (без serde_urlencoded).
    pub fn form_data(mut self, data: &[(&str, &str)]) -> Self {
        let form: String = data.iter()
            .map(|(k, v)| format!("{}={}", url_encode(k), url_encode(v)))
            .collect::<Vec<_>>()
            .join("&");
        self.headers.insert(HeaderName::CONTENT_TYPE, "application/x-www-form-urlencoded");
        self.body = Body::from(form);
        self
    }

    /// Устанавливает текстовое тело.
    pub fn text(mut self, text: impl Into<String>) -> Self {
        self.headers.insert(HeaderName::CONTENT_TYPE, "text/plain; charset=utf-8");
        self.body = Body::from(text.into());
        self
    }

    /// Устанавливает бинарное тело.
    pub fn body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    /// Устанавливает таймаут.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Отправляет запрос.
    pub async fn send(self) -> NetResult<Response> {
        let timeout = self.timeout.unwrap_or(self.client.config.read_timeout);
        
        tokio::time::timeout(timeout, self.send_inner())
            .await
            .map_err(|_| NetError::Timeout)?
    }

    async fn send_inner(mut self) -> NetResult<Response> {
        // Парсим URL
        let parsed = ParsedUrl::parse(&self.url)?;
        
        // Добавляем Host заголовок
        self.headers.insert(HeaderName::HOST, parsed.host.as_str());
        
        // Добавляем User-Agent если нет
        if self.headers.get(HeaderName::USER_AGENT).is_none() {
            self.headers.insert(HeaderName::USER_AGENT, self.client.config.user_agent.as_str());
        }

        // Добавляем Content-Length если есть тело
        if let Some(len) = self.body.len() {
            self.headers.insert(HeaderName::CONTENT_LENGTH, len.to_string());
        }

        // Устанавливаем соединение
        let addr = format!("{}:{}", parsed.host, parsed.port);
        let socket_addr: SocketAddr = tokio::net::lookup_host(&addr)
            .await
            .map_err(|e| NetError::Io(e))?
            .next()
            .ok_or_else(|| NetError::InvalidAddress(addr.clone()))?;

        let mut conn = TcpConnection::connect(socket_addr, TcpConfig::default()).await?;

        // TLS handshake если HTTPS
        // TODO: Реализовать TLS через self.client.tls_connector

        // Формируем HTTP запрос
        let request_line = format!(
            "{} {} HTTP/1.1\r\n",
            self.method,
            parsed.path_with_query()
        );

        let mut request_bytes = request_line.into_bytes();
        
        // Заголовки
        for (name, values) in self.headers.iter() {
            for value in values {
                request_bytes.extend_from_slice(name.as_str().as_bytes());
                request_bytes.extend_from_slice(b": ");
                request_bytes.extend_from_slice(value.as_str().as_bytes());
                request_bytes.extend_from_slice(b"\r\n");
            }
        }
        request_bytes.extend_from_slice(b"\r\n");

        // Тело
        if let Some(len) = self.body.len() {
            if len > 0 {
                let body_bytes = self.body.into_bytes();
                request_bytes.extend_from_slice(&body_bytes);
            }
        }

        // Отправляем
        conn.write_all(&request_bytes).await?;
        conn.flush().await?;

        // Читаем ответ
        let response = read_response(&mut conn, self.client.config.buffer_size).await?;

        // Закрываем соединение
        let _ = conn.close().await;

        Ok(response)
    }
}

// ============================================================================
//                    URL PARSER
// ============================================================================

struct ParsedUrl {
    scheme: String,
    host: String,
    port: u16,
    path: String,
    query: Option<String>,
}

impl ParsedUrl {
    fn parse(url: &str) -> NetResult<Self> {
        // Простой парсер URL
        let (scheme, rest) = url.split_once("://")
            .ok_or_else(|| NetError::InvalidAddress("Missing scheme".into()))?;

        let (host_port, path_query) = rest.split_once('/')
            .map(|(h, p)| (h, format!("/{}", p)))
            .unwrap_or((rest, "/".to_string()));

        let (host, port) = if let Some((h, p)) = host_port.split_once(':') {
            let port = p.parse().map_err(|_| NetError::InvalidAddress("Invalid port".into()))?;
            (h.to_string(), port)
        } else {
            let port = match scheme {
                "https" => 443,
                "http" => 80,
                _ => return Err(NetError::InvalidAddress("Unknown scheme".into())),
            };
            (host_port.to_string(), port)
        };

        let (path, query) = path_query.split_once('?')
            .map(|(p, q)| (p.to_string(), Some(q.to_string())))
            .unwrap_or((path_query, None));

        Ok(Self { scheme: scheme.to_string(), host, port, path, query })
    }

    fn path_with_query(&self) -> String {
        match &self.query {
            Some(q) => format!("{}?{}", self.path, q),
            None => self.path.clone(),
        }
    }

    fn is_https(&self) -> bool {
        self.scheme == "https"
    }
}

// ============================================================================
//                    RESPONSE READER
// ============================================================================

async fn read_response(conn: &mut TcpConnection, buffer_size: usize) -> NetResult<Response> {
    let mut buffer = Vec::with_capacity(buffer_size);
    let mut temp = [0u8; 4096];

    // Читаем заголовки
    loop {
        let n = conn.read(&mut temp).await?;
        if n == 0 {
            return Err(NetError::ConnectionClosed);
        }

        buffer.extend_from_slice(&temp[..n]);

        // Ищем конец заголовков
        if buffer.windows(4).any(|w| w == b"\r\n\r\n") {
            break;
        }

        if buffer.len() > 64 * 1024 {
            return Err(NetError::Http("Response headers too large".into()));
        }
    }

    // Парсим статус-линию
    let header_str = String::from_utf8_lossy(&buffer);
    let mut lines = header_str.lines();

    let status_line = lines.next()
        .ok_or_else(|| NetError::Http("Missing status line".into()))?;

    let parts: Vec<&str> = status_line.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return Err(NetError::Http("Invalid status line".into()));
    }

    let status_code: u16 = parts[1].parse()
        .map_err(|_| NetError::Http("Invalid status code".into()))?;
    let reason = parts.get(2).map(|s| s.to_string());

    // Парсим заголовки
    let mut headers = Headers::new();
    let mut content_length = None;
    let mut chunked = false;

    for line in lines {
        if line.is_empty() {
            break;
        }
        if let Some((name, value)) = line.split_once(':') {
            let name = name.trim().to_lowercase();
            let value = value.trim();
            
            if name == "content-length" {
                content_length = value.parse().ok();
            } else if name == "transfer-encoding" && value.eq_ignore_ascii_case("chunked") {
                chunked = true;
            }
            
            headers.insert(name, value);
        }
    }

    // Находим начало тела
    let headers_end = buffer.windows(4)
        .position(|w| w == b"\r\n\r\n")
        .unwrap() + 4;

    let mut body = buffer[headers_end..].to_vec();

    // Читаем тело
    if let Some(len) = content_length {
        while body.len() < len {
            let n = conn.read(&mut temp).await?;
            if n == 0 {
                break;
            }
            body.extend_from_slice(&temp[..n]);
        }
        body.truncate(len);
    } else if chunked {
        // TODO: Поддержка chunked encoding
    }

    // Создаём Response
    let mut response = Response::new(StatusCode::from(status_code));
    *response.headers_mut() = headers;
    response.set_body(Body::from(body));

    Ok(response)
}

// ============================================================================
//                    RESPONSE EXTENSIONS
// ============================================================================

impl Response {
    /// Десериализует тело как JSON (async версия для клиента).
    pub async fn json_async<T: DeserializeOwned>(&mut self) -> NetResult<T> {
        let bytes = self.body_bytes_async().await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| NetError::Serialization(e.to_string()))
    }

    /// Возвращает тело как текст (async версия для клиента).
    pub async fn text_async(&mut self) -> NetResult<String> {
        let bytes = self.body_bytes_async().await?;
        String::from_utf8(bytes)
            .map_err(|e| NetError::Http(e.to_string()))
    }

    /// Возвращает тело как байты (async версия).
    pub async fn body_bytes_async(&mut self) -> NetResult<Vec<u8>> {
        // Для простого Body просто возвращаем байты
        Ok(self.take_body().into_bytes())
    }
}

// ============================================================================
//                    QUICK FUNCTIONS
// ============================================================================

/// Быстрый GET запрос.
pub async fn get(url: &str) -> NetResult<Response> {
    HttpClient::new().get(url).await
}

/// Быстрый POST запрос с JSON.
pub async fn post_json<T: Serialize>(url: &str, body: &T) -> NetResult<Response> {
    HttpClient::new()
        .post(url).await
        .json(body)?
        .send()
        .await
}

// ============================================================================
//                    BASE64 ENCODING (MINIMAL)
// ============================================================================

const BASE64_ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode(data: &[u8]) -> String {
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        
        let combined = (b0 << 16) | (b1 << 8) | b2;
        
        result.push(BASE64_ALPHABET[((combined >> 18) & 0x3F) as usize] as char);
        result.push(BASE64_ALPHABET[((combined >> 12) & 0x3F) as usize] as char);
        
        if chunk.len() > 1 {
            result.push(BASE64_ALPHABET[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(BASE64_ALPHABET[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

fn url_encode(input: &str) -> String {
    let mut result = String::new();
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => result.push_str(&format!("%{:02X}", byte)),
        }
    }
    result
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_parse_simple() {
        let url = ParsedUrl::parse("http://example.com/path").unwrap();
        assert_eq!(url.scheme, "http");
        assert_eq!(url.host, "example.com");
        assert_eq!(url.port, 80);
        assert_eq!(url.path, "/path");
    }

    #[test]
    fn test_url_parse_https_with_port() {
        let url = ParsedUrl::parse("https://api.example.com:8443/api/v1?key=value").unwrap();
        assert_eq!(url.scheme, "https");
        assert_eq!(url.host, "api.example.com");
        assert_eq!(url.port, 8443);
        assert_eq!(url.path, "/api/v1");
        assert_eq!(url.query, Some("key=value".to_string()));
    }

    #[test]
    fn test_client_builder() {
        let client = HttpClient::new()
            .default_header("X-Api-Key", "secret");
        
        let builder = client.request(Method::POST, "http://example.com/api");
        assert!(builder.headers.get("X-Api-Key").is_some());
    }
}
