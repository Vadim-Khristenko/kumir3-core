// ============================================================================
//                    HTTP REQUEST
// ============================================================================

use std::collections::HashMap;
use std::net::SocketAddr;


use super::types::{Method, Headers, HeaderName};
use super::body::Body;
use crate::libraries::net::{NetError, NetResult};

// ============================================================================
//                    REQUEST
// ============================================================================

/// HTTP запрос.
#[derive(Debug)]
pub struct Request {
    /// HTTP метод
    method: Method,
    /// URI (путь + query string)
    uri: String,
    /// Путь (без query string)
    path: String,
    /// Query string (без ?)
    query_string: Option<String>,
    /// HTTP версия
    version: HttpVersion,
    /// Заголовки
    headers: Headers,
    /// Тело запроса
    body: Body,
    /// Адрес клиента
    remote_addr: Option<SocketAddr>,
    /// Path параметры (заполняются роутером)
    path_params: HashMap<String, String>,
    /// Расширения (для middleware и state)
    extensions: Extensions,
}

/// HTTP версия.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HttpVersion {
    Http10,
    Http11,
    Http2,
}

impl Default for HttpVersion {
    fn default() -> Self {
        HttpVersion::Http11
    }
}

impl std::fmt::Display for HttpVersion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            HttpVersion::Http10 => write!(f, "HTTP/1.0"),
            HttpVersion::Http11 => write!(f, "HTTP/1.1"),
            HttpVersion::Http2 => write!(f, "HTTP/2"),
        }
    }
}

/// Расширения запроса (типизированный storage).
#[derive(Debug, Default)]
pub struct Extensions {
    inner: HashMap<std::any::TypeId, Box<dyn std::any::Any + Send + Sync>>,
}

impl Extensions {
    pub fn new() -> Self {
        Self::default()
    }

    /// Вставляет значение по типу.
    pub fn insert<T: Send + Sync + 'static>(&mut self, val: T) {
        self.inner.insert(std::any::TypeId::of::<T>(), Box::new(val));
    }

    /// Получает значение по типу.
    pub fn get<T: Send + Sync + 'static>(&self) -> Option<&T> {
        self.inner
            .get(&std::any::TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_ref())
    }

    /// Получает мутабельную ссылку по типу.
    pub fn get_mut<T: Send + Sync + 'static>(&mut self) -> Option<&mut T> {
        self.inner
            .get_mut(&std::any::TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast_mut())
    }

    /// Удаляет значение по типу.
    pub fn remove<T: Send + Sync + 'static>(&mut self) -> Option<T> {
        self.inner
            .remove(&std::any::TypeId::of::<T>())
            .and_then(|boxed| boxed.downcast().ok())
            .map(|boxed| *boxed)
    }
}

impl Request {
    /// Создаёт новый запрос.
    pub fn new(method: Method, uri: impl Into<String>) -> Self {
        let uri = uri.into();
        let (path, query_string) = parse_uri(&uri);
        
        Self {
            method,
            uri,
            path,
            query_string,
            version: HttpVersion::default(),
            headers: Headers::new(),
            body: Body::empty(),
            remote_addr: None,
            path_params: HashMap::new(),
            extensions: Extensions::new(),
        }
    }

    /// Создаёт билдер.
    pub fn builder() -> RequestBuilder {
        RequestBuilder::new()
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    pub fn method(&self) -> Method {
        self.method
    }

    pub fn uri(&self) -> &str {
        &self.uri
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    pub fn query_string(&self) -> Option<&str> {
        self.query_string.as_deref()
    }

    pub fn version(&self) -> HttpVersion {
        self.version
    }

    pub fn headers(&self) -> &Headers {
        &self.headers
    }

    pub fn headers_mut(&mut self) -> &mut Headers {
        &mut self.headers
    }

    pub fn body(&self) -> &Body {
        &self.body
    }

    pub fn body_mut(&mut self) -> &mut Body {
        &mut self.body
    }

    pub fn remote_addr(&self) -> Option<SocketAddr> {
        self.remote_addr
    }

    pub fn extensions(&self) -> &Extensions {
        &self.extensions
    }

    pub fn extensions_mut(&mut self) -> &mut Extensions {
        &mut self.extensions
    }

    // -------------------------------------------------------------------------
    // Path Params
    // -------------------------------------------------------------------------

    /// Получает path параметр.
    pub fn param(&self, name: &str) -> Option<&str> {
        self.path_params.get(name).map(|s| s.as_str())
    }

    /// Получает path параметр и парсит в тип.
    pub fn param_parse<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.param(name)?.parse().ok()
    }

    /// Устанавливает path параметры (вызывается роутером).
    pub fn set_path_params(&mut self, params: HashMap<String, String>) {
        self.path_params = params;
    }

    /// Возвращает все path параметры.
    pub fn path_params(&self) -> &HashMap<String, String> {
        &self.path_params
    }

    // -------------------------------------------------------------------------
    // Query Params
    // -------------------------------------------------------------------------

    /// Парсит query string в HashMap.
    pub fn query_params(&self) -> HashMap<String, String> {
        parse_query_string(self.query_string.as_deref().unwrap_or(""))
    }

    /// Получает query параметр.
    pub fn query(&self, name: &str) -> Option<String> {
        self.query_params().get(name).cloned()
    }

    /// Получает query параметр и парсит в тип.
    pub fn query_parse<T: std::str::FromStr>(&self, name: &str) -> Option<T> {
        self.query(name)?.parse().ok()
    }

    // -------------------------------------------------------------------------
    // Headers shortcuts
    // -------------------------------------------------------------------------

    pub fn content_type(&self) -> Option<&str> {
        self.headers.content_type()
    }

    pub fn content_length(&self) -> Option<usize> {
        self.headers.content_length()
    }

    pub fn host(&self) -> Option<&str> {
        self.headers.get(HeaderName::HOST).map(|v| v.as_str())
    }

    pub fn user_agent(&self) -> Option<&str> {
        self.headers.get(HeaderName::USER_AGENT).map(|v| v.as_str())
    }

    /// Проверяет, является ли запрос WebSocket upgrade.
    pub fn is_websocket_upgrade(&self) -> bool {
        self.headers.get(HeaderName::UPGRADE)
            .map(|v| v.as_str().eq_ignore_ascii_case("websocket"))
            .unwrap_or(false)
    }

    // -------------------------------------------------------------------------
    // Body helpers
    // -------------------------------------------------------------------------

    /// Читает тело как байты.
    pub async fn bytes(&mut self) -> NetResult<Vec<u8>> {
        self.body.bytes().await
    }

    /// Читает тело как строку.
    pub async fn text(&mut self) -> NetResult<String> {
        self.body.text().await
    }

    /// Читает тело как JSON.
    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> NetResult<T> {
        self.body.json().await
    }

    // -------------------------------------------------------------------------
    // Setters
    // -------------------------------------------------------------------------

    pub fn set_remote_addr(&mut self, addr: SocketAddr) {
        self.remote_addr = Some(addr);
    }

    pub fn set_body(&mut self, body: Body) {
        self.body = body;
    }

    /// Забирает тело (оставляет пустое).
    pub fn take_body(&mut self) -> Body {
        std::mem::replace(&mut self.body, Body::empty())
    }
}

// ============================================================================
//                    REQUEST BUILDER
// ============================================================================

/// Билдер для Request.
pub struct RequestBuilder {
    method: Method,
    uri: String,
    version: HttpVersion,
    headers: Headers,
    body: Body,
}

impl RequestBuilder {
    pub fn new() -> Self {
        Self {
            method: Method::GET,
            uri: "/".to_string(),
            version: HttpVersion::Http11,
            headers: Headers::new(),
            body: Body::empty(),
        }
    }

    pub fn method(mut self, method: Method) -> Self {
        self.method = method;
        self
    }

    pub fn get(mut self, uri: impl Into<String>) -> Self {
        self.method = Method::GET;
        self.uri = uri.into();
        self
    }

    pub fn post(mut self, uri: impl Into<String>) -> Self {
        self.method = Method::POST;
        self.uri = uri.into();
        self
    }

    pub fn put(mut self, uri: impl Into<String>) -> Self {
        self.method = Method::PUT;
        self.uri = uri.into();
        self
    }

    pub fn delete(mut self, uri: impl Into<String>) -> Self {
        self.method = Method::DELETE;
        self.uri = uri.into();
        self
    }

    pub fn uri(mut self, uri: impl Into<String>) -> Self {
        self.uri = uri.into();
        self
    }

    pub fn version(mut self, version: HttpVersion) -> Self {
        self.version = version;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn headers(mut self, headers: Headers) -> Self {
        self.headers = headers;
        self
    }

    pub fn body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    pub fn json<T: serde::Serialize>(mut self, value: &T) -> NetResult<Self> {
        let json = serde_json::to_vec(value)?;
        self.headers.set_content_type(super::types::mime::APPLICATION_JSON);
        self.body = Body::from(json);
        Ok(self)
    }

    pub fn build(self) -> Request {
        let (path, query_string) = parse_uri(&self.uri);
        
        Request {
            method: self.method,
            uri: self.uri,
            path,
            query_string,
            version: self.version,
            headers: self.headers,
            body: self.body,
            remote_addr: None,
            path_params: HashMap::new(),
            extensions: Extensions::new(),
        }
    }
}

impl Default for RequestBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    PARSING
// ============================================================================

/// Парсит URI на путь и query string.
fn parse_uri(uri: &str) -> (String, Option<String>) {
    if let Some(pos) = uri.find('?') {
        let path = uri[..pos].to_string();
        let query = uri[pos + 1..].to_string();
        (path, Some(query))
    } else {
        (uri.to_string(), None)
    }
}

/// Парсит query string в HashMap.
fn parse_query_string(query: &str) -> HashMap<String, String> {
    let mut params = HashMap::new();
    
    for pair in query.split('&') {
        if pair.is_empty() {
            continue;
        }
        
        let mut parts = pair.splitn(2, '=');
        if let Some(key) = parts.next() {
            let value = parts.next().unwrap_or("");
            let key = url_decode(key);
            let value = url_decode(value);
            params.insert(key, value);
        }
    }
    
    params
}

/// URL декодирование.
fn url_decode(s: &str) -> String {
    let mut result = String::with_capacity(s.len());
    let mut chars = s.chars().peekable();
    
    while let Some(c) = chars.next() {
        if c == '%' {
            let mut hex = String::with_capacity(2);
            if let Some(h1) = chars.next() {
                hex.push(h1);
            }
            if let Some(h2) = chars.next() {
                hex.push(h2);
            }
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            }
        } else if c == '+' {
            result.push(' ');
        } else {
            result.push(c);
        }
    }
    
    result
}

/// Парсит HTTP запрос из сырых байт.
pub fn parse_request(data: &[u8], remote_addr: Option<SocketAddr>) -> NetResult<Request> {
    let text = std::str::from_utf8(data)
        .map_err(|e| NetError::Parse(format!("Invalid UTF-8: {}", e)))?;
    
    let mut lines = text.lines();
    
    // Парсим request line
    let request_line = lines.next()
        .ok_or_else(|| NetError::Parse("Empty request".into()))?;
    
    let mut parts = request_line.split_whitespace();
    
    let method: Method = parts.next()
        .ok_or_else(|| NetError::Parse("Missing method".into()))?
        .parse()
        .map_err(|_| NetError::Parse("Invalid method".into()))?;
    
    let uri = parts.next()
        .ok_or_else(|| NetError::Parse("Missing URI".into()))?
        .to_string();
    
    let version = match parts.next() {
        Some("HTTP/1.0") => HttpVersion::Http10,
        Some("HTTP/1.1") => HttpVersion::Http11,
        Some("HTTP/2") | Some("HTTP/2.0") => HttpVersion::Http2,
        Some(v) => return Err(NetError::Parse(format!("Unsupported HTTP version: {}", v))),
        None => HttpVersion::Http11,
    };
    
    // Парсим заголовки
    let mut headers = Headers::new();
    for line in lines {
        if line.is_empty() {
            break;
        }
        
        if let Some(pos) = line.find(':') {
            let name = line[..pos].trim();
            let value = line[pos + 1..].trim();
            headers.append(name, value);
        }
    }
    
    let (path, query_string) = parse_uri(&uri);
    
    let request = Request {
        method,
        uri,
        path,
        query_string,
        version,
        headers,
        body: Body::empty(),
        remote_addr,
        path_params: HashMap::new(),
        extensions: Extensions::new(),
    };
    
    Ok(request)
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_request_builder() {
        let req = Request::builder()
            .get("/users?page=1")
            .header("Accept", "application/json")
            .build();

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.path(), "/users");
        assert_eq!(req.query_string(), Some("page=1"));
        assert_eq!(req.query("page"), Some("1".to_string()));
    }

    #[test]
    fn test_parse_query_string() {
        let params = parse_query_string("name=John&age=30&city=New%20York");
        assert_eq!(params.get("name").unwrap(), "John");
        assert_eq!(params.get("age").unwrap(), "30");
        assert_eq!(params.get("city").unwrap(), "New York");
    }

    #[test]
    fn test_parse_request() {
        let raw = b"GET /users?id=1 HTTP/1.1\r\nHost: localhost\r\nAccept: */*\r\n\r\n";
        let req = parse_request(raw, None).unwrap();

        assert_eq!(req.method(), Method::GET);
        assert_eq!(req.path(), "/users");
        assert_eq!(req.host(), Some("localhost"));
    }
}
