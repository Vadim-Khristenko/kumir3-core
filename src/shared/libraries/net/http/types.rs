// ============================================================================
//                    HTTP ТИПЫ
// ============================================================================

use std::collections::HashMap;
use std::str::FromStr;

// ============================================================================
//                    HTTP МЕТОД
// ============================================================================

/// HTTP метод.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Method {
    GET,
    POST,
    PUT,
    DELETE,
    PATCH,
    HEAD,
    OPTIONS,
    CONNECT,
    TRACE,
}

impl Method {
    /// Все методы.
    pub const ALL: [Method; 9] = [
        Method::GET,
        Method::POST,
        Method::PUT,
        Method::DELETE,
        Method::PATCH,
        Method::HEAD,
        Method::OPTIONS,
        Method::CONNECT,
        Method::TRACE,
    ];

    /// Проверяет, может ли метод иметь тело.
    pub fn has_body(&self) -> bool {
        matches!(self, Method::POST | Method::PUT | Method::PATCH)
    }

    /// Проверяет, идемпотентен ли метод.
    pub fn is_idempotent(&self) -> bool {
        !matches!(self, Method::POST | Method::PATCH)
    }

    /// Проверяет, безопасен ли метод (не модифицирует ресурсы).
    pub fn is_safe(&self) -> bool {
        matches!(self, Method::GET | Method::HEAD | Method::OPTIONS | Method::TRACE)
    }

    /// Возвращает строковое представление.
    pub fn as_str(&self) -> &'static str {
        match self {
            Method::GET => "GET",
            Method::POST => "POST",
            Method::PUT => "PUT",
            Method::DELETE => "DELETE",
            Method::PATCH => "PATCH",
            Method::HEAD => "HEAD",
            Method::OPTIONS => "OPTIONS",
            Method::CONNECT => "CONNECT",
            Method::TRACE => "TRACE",
        }
    }
}

impl FromStr for Method {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "GET" => Ok(Method::GET),
            "POST" => Ok(Method::POST),
            "PUT" => Ok(Method::PUT),
            "DELETE" => Ok(Method::DELETE),
            "PATCH" => Ok(Method::PATCH),
            "HEAD" => Ok(Method::HEAD),
            "OPTIONS" => Ok(Method::OPTIONS),
            "CONNECT" => Ok(Method::CONNECT),
            "TRACE" => Ok(Method::TRACE),
            _ => Err(()),
        }
    }
}

impl std::fmt::Display for Method {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

// ============================================================================
//                    HTTP СТАТУС КОД
// ============================================================================

/// HTTP статус код.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StatusCode(pub u16);

impl StatusCode {
    // 1xx Informational
    pub const CONTINUE: StatusCode = StatusCode(100);
    pub const SWITCHING_PROTOCOLS: StatusCode = StatusCode(101);
    pub const PROCESSING: StatusCode = StatusCode(102);

    // 2xx Success
    pub const OK: StatusCode = StatusCode(200);
    pub const CREATED: StatusCode = StatusCode(201);
    pub const ACCEPTED: StatusCode = StatusCode(202);
    pub const NO_CONTENT: StatusCode = StatusCode(204);
    pub const PARTIAL_CONTENT: StatusCode = StatusCode(206);

    // 3xx Redirection
    pub const MOVED_PERMANENTLY: StatusCode = StatusCode(301);
    pub const FOUND: StatusCode = StatusCode(302);
    pub const SEE_OTHER: StatusCode = StatusCode(303);
    pub const NOT_MODIFIED: StatusCode = StatusCode(304);
    pub const TEMPORARY_REDIRECT: StatusCode = StatusCode(307);
    pub const PERMANENT_REDIRECT: StatusCode = StatusCode(308);

    // 4xx Client Error
    pub const BAD_REQUEST: StatusCode = StatusCode(400);
    pub const UNAUTHORIZED: StatusCode = StatusCode(401);
    pub const FORBIDDEN: StatusCode = StatusCode(403);
    pub const NOT_FOUND: StatusCode = StatusCode(404);
    pub const METHOD_NOT_ALLOWED: StatusCode = StatusCode(405);
    pub const CONFLICT: StatusCode = StatusCode(409);
    pub const GONE: StatusCode = StatusCode(410);
    pub const UNPROCESSABLE_ENTITY: StatusCode = StatusCode(422);
    pub const TOO_MANY_REQUESTS: StatusCode = StatusCode(429);

    // 5xx Server Error
    pub const INTERNAL_SERVER_ERROR: StatusCode = StatusCode(500);
    pub const NOT_IMPLEMENTED: StatusCode = StatusCode(501);
    pub const BAD_GATEWAY: StatusCode = StatusCode(502);
    pub const SERVICE_UNAVAILABLE: StatusCode = StatusCode(503);
    pub const GATEWAY_TIMEOUT: StatusCode = StatusCode(504);

    /// Возвращает код.
    pub fn code(&self) -> u16 {
        self.0
    }

    /// Возвращает reason phrase.
    pub fn reason(&self) -> &'static str {
        match self.0 {
            100 => "Continue",
            101 => "Switching Protocols",
            102 => "Processing",
            200 => "OK",
            201 => "Created",
            202 => "Accepted",
            204 => "No Content",
            206 => "Partial Content",
            301 => "Moved Permanently",
            302 => "Found",
            303 => "See Other",
            304 => "Not Modified",
            307 => "Temporary Redirect",
            308 => "Permanent Redirect",
            400 => "Bad Request",
            401 => "Unauthorized",
            403 => "Forbidden",
            404 => "Not Found",
            405 => "Method Not Allowed",
            409 => "Conflict",
            410 => "Gone",
            422 => "Unprocessable Entity",
            429 => "Too Many Requests",
            500 => "Internal Server Error",
            501 => "Not Implemented",
            502 => "Bad Gateway",
            503 => "Service Unavailable",
            504 => "Gateway Timeout",
            _ => "Unknown",
        }
    }

    /// Проверяет, успешный ли код.
    pub fn is_success(&self) -> bool {
        (200..300).contains(&self.0)
    }

    /// Проверяет, редирект ли код.
    pub fn is_redirect(&self) -> bool {
        (300..400).contains(&self.0)
    }

    /// Проверяет, клиентская ли ошибка.
    pub fn is_client_error(&self) -> bool {
        (400..500).contains(&self.0)
    }

    /// Проверяет, серверная ли ошибка.
    pub fn is_server_error(&self) -> bool {
        (500..600).contains(&self.0)
    }
}

impl std::fmt::Display for StatusCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.0, self.reason())
    }
}

impl From<u16> for StatusCode {
    fn from(code: u16) -> Self {
        StatusCode(code)
    }
}

// ============================================================================
//                    HTTP ЗАГОЛОВКИ
// ============================================================================

/// Имя заголовка (case-insensitive).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct HeaderName(String);

impl HeaderName {
    // Стандартные заголовки
    pub const CONTENT_TYPE: &'static str = "content-type";
    pub const CONTENT_LENGTH: &'static str = "content-length";
    pub const CONTENT_ENCODING: &'static str = "content-encoding";
    pub const TRANSFER_ENCODING: &'static str = "transfer-encoding";
    pub const HOST: &'static str = "host";
    pub const USER_AGENT: &'static str = "user-agent";
    pub const ACCEPT: &'static str = "accept";
    pub const ACCEPT_ENCODING: &'static str = "accept-encoding";
    pub const ACCEPT_LANGUAGE: &'static str = "accept-language";
    pub const AUTHORIZATION: &'static str = "authorization";
    pub const COOKIE: &'static str = "cookie";
    pub const SET_COOKIE: &'static str = "set-cookie";
    pub const CACHE_CONTROL: &'static str = "cache-control";
    pub const CONNECTION: &'static str = "connection";
    pub const UPGRADE: &'static str = "upgrade";
    pub const SEC_WEBSOCKET_KEY: &'static str = "sec-websocket-key";
    pub const SEC_WEBSOCKET_ACCEPT: &'static str = "sec-websocket-accept";
    pub const SEC_WEBSOCKET_VERSION: &'static str = "sec-websocket-version";
    pub const LOCATION: &'static str = "location";
    pub const SERVER: &'static str = "server";
    pub const DATE: &'static str = "date";
    pub const ETAG: &'static str = "etag";
    pub const IF_NONE_MATCH: &'static str = "if-none-match";
    pub const IF_MODIFIED_SINCE: &'static str = "if-modified-since";
    pub const LAST_MODIFIED: &'static str = "last-modified";
    pub const X_REQUEST_ID: &'static str = "x-request-id";
    pub const X_FORWARDED_FOR: &'static str = "x-forwarded-for";
    pub const X_REAL_IP: &'static str = "x-real-ip";
    pub const CORS_ORIGIN: &'static str = "access-control-allow-origin";
    pub const CORS_METHODS: &'static str = "access-control-allow-methods";
    pub const CORS_HEADERS: &'static str = "access-control-allow-headers";

    pub fn new(name: impl Into<String>) -> Self {
        Self(name.into().to_lowercase())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl From<&str> for HeaderName {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for HeaderName {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for HeaderName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Значение заголовка.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeaderValue(String);

impl HeaderValue {
    pub fn new(value: impl Into<String>) -> Self {
        Self(value.into())
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Парсит значение как число.
    pub fn to_int(&self) -> Option<i64> {
        self.0.parse().ok()
    }
}

impl From<&str> for HeaderValue {
    fn from(s: &str) -> Self {
        Self::new(s)
    }
}

impl From<String> for HeaderValue {
    fn from(s: String) -> Self {
        Self::new(s)
    }
}

impl std::fmt::Display for HeaderValue {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

/// Коллекция HTTP заголовков.
#[derive(Debug, Clone, Default)]
pub struct Headers {
    inner: HashMap<HeaderName, Vec<HeaderValue>>,
}

impl Headers {
    /// Создаёт пустую коллекцию.
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавляет заголовок (можно несколько с одним именем).
    pub fn append(&mut self, name: impl Into<HeaderName>, value: impl Into<HeaderValue>) {
        self.inner
            .entry(name.into())
            .or_default()
            .push(value.into());
    }

    /// Устанавливает заголовок (заменяет существующий).
    pub fn insert(&mut self, name: impl Into<HeaderName>, value: impl Into<HeaderValue>) {
        self.inner.insert(name.into(), vec![value.into()]);
    }

    /// Получает первое значение заголовка.
    pub fn get(&self, name: &str) -> Option<&HeaderValue> {
        self.inner.get(&HeaderName::new(name))?.first()
    }

    /// Получает все значения заголовка.
    pub fn get_all(&self, name: &str) -> Option<&Vec<HeaderValue>> {
        self.inner.get(&HeaderName::new(name))
    }

    /// Проверяет наличие заголовка.
    pub fn contains(&self, name: &str) -> bool {
        self.inner.contains_key(&HeaderName::new(name))
    }

    /// Удаляет заголовок.
    pub fn remove(&mut self, name: &str) -> Option<Vec<HeaderValue>> {
        self.inner.remove(&HeaderName::new(name))
    }

    /// Количество заголовков.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Пустая ли коллекция.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Итератор по заголовкам.
    pub fn iter(&self) -> impl Iterator<Item = (&HeaderName, &Vec<HeaderValue>)> {
        self.inner.iter()
    }

    /// Content-Type.
    pub fn content_type(&self) -> Option<&str> {
        self.get(HeaderName::CONTENT_TYPE).map(|v| v.as_str())
    }

    /// Content-Length.
    pub fn content_length(&self) -> Option<usize> {
        self.get(HeaderName::CONTENT_LENGTH)?.to_int().map(|n| n as usize)
    }

    /// Устанавливает Content-Type.
    pub fn set_content_type(&mut self, mime: &str) {
        self.insert(HeaderName::CONTENT_TYPE, mime);
    }

    /// Устанавливает Content-Length.
    pub fn set_content_length(&mut self, len: usize) {
        self.insert(HeaderName::CONTENT_LENGTH, len.to_string());
    }
}

// ============================================================================
//                    MIME ТИПЫ
// ============================================================================

/// Распространённые MIME типы.
pub mod mime {
    pub const TEXT_PLAIN: &str = "text/plain; charset=utf-8";
    pub const TEXT_HTML: &str = "text/html; charset=utf-8";
    pub const TEXT_CSS: &str = "text/css; charset=utf-8";
    pub const TEXT_JAVASCRIPT: &str = "text/javascript; charset=utf-8";
    pub const APPLICATION_JSON: &str = "application/json; charset=utf-8";
    pub const APPLICATION_XML: &str = "application/xml; charset=utf-8";
    pub const APPLICATION_FORM: &str = "application/x-www-form-urlencoded";
    pub const MULTIPART_FORM: &str = "multipart/form-data";
    pub const APPLICATION_OCTET_STREAM: &str = "application/octet-stream";
    pub const IMAGE_PNG: &str = "image/png";
    pub const IMAGE_JPEG: &str = "image/jpeg";
    pub const IMAGE_GIF: &str = "image/gif";
    pub const IMAGE_SVG: &str = "image/svg+xml";
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_method_from_str() {
        assert_eq!("GET".parse::<Method>().unwrap(), Method::GET);
        assert_eq!("post".parse::<Method>().unwrap(), Method::POST);
    }

    #[test]
    fn test_status_code() {
        assert!(StatusCode::OK.is_success());
        assert!(StatusCode::NOT_FOUND.is_client_error());
        assert!(StatusCode::INTERNAL_SERVER_ERROR.is_server_error());
    }

    #[test]
    fn test_headers() {
        let mut headers = Headers::new();
        headers.insert("Content-Type", "application/json");
        headers.append("Set-Cookie", "a=1");
        headers.append("Set-Cookie", "b=2");

        assert_eq!(headers.get("content-type").unwrap().as_str(), "application/json");
        assert_eq!(headers.get_all("set-cookie").unwrap().len(), 2);
    }
}
