// ============================================================================
//                    СЕТЕВАЯ БИБЛИОТЕКА KUMIR3
// ============================================================================
//
// Модульная async-first сетевая библиотека поверх Tokio:
// - TCP/UDP транспорт
// - HTTP/1.1 сервер и клиент
// - WebSocket поддержка
// - TLS через tokio-rustls
// - Middleware, Dependency Injection, валидация через serde
//
// Философия: каждый модуль независим и может использоваться отдельно.
//
// ============================================================================

pub mod core;
pub mod tcp;
pub mod udp;
pub mod tls;
pub mod http;

// Реэкспорты для удобного доступа
pub use core::*;
pub use tcp::{TcpListener, TcpConnection, TcpConfig};
pub use udp::{UdpSocket, UdpConfig};
pub use tls::{TlsConfig, TlsAcceptor, TlsConnector};
pub use http::{
    HttpServer, HttpClient,
    Request, Response, Body, BodyStream,
    Router, Route, Handler, Middleware,
    StatusCode, Method, Headers,
    Json, Query, Path, State,
};

use std::net::SocketAddr;
use std::time::Duration;

// Импорт глобального runtime для async операций
use crate::runtime::{global_runtime, init_global_runtime};

// ============================================================================
//                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ ДЛЯ ASYNC
// ============================================================================

/// Получает tokio handle для выполнения async операций.
/// 
/// Сначала пытается использовать текущий runtime (если код запущен в async контексте).
/// Если нет, использует глобальный KumirRuntime.
fn get_tokio_handle() -> Result<tokio::runtime::Handle, String> {
    // Сначала проверяем есть ли уже активный runtime (например, из тестов или async контекста)
    if let Ok(handle) = tokio::runtime::Handle::try_current() {
        return Ok(handle);
    }
    
    // Инициализируем глобальный runtime если ещё не инициализирован
    if let Err(e) = init_global_runtime() {
        return Err(format!("Не удалось инициализировать runtime: {}", e));
    }
    
    // Получаем handle из глобального runtime
    let runtime = global_runtime();
    runtime.tokio_handle()
        .cloned()
        .ok_or_else(|| "Tokio runtime не инициализирован".to_string())
}

// ============================================================================
//                    СЕТЕВОЙ RUNTIME
// ============================================================================

/// Глобальный сетевой runtime — точка входа для создания серверов и клиентов.
/// 
/// # Пример
/// ```rust
/// let net = NetworkRuntime::new();
/// 
/// // HTTP сервер
/// let server = net.http_server()
///     .bind("0.0.0.0:8080")
///     .router(my_router)
///     .serve()
///     .await?;
/// 
/// // TCP клиент
/// let conn = net.tcp_connect("example.com:80").await?;
/// ```
pub struct NetworkRuntime {
    /// Tokio runtime handle
    tokio_handle: tokio::runtime::Handle,
    /// Конфигурация по умолчанию
    default_config: NetworkConfig,
}

/// Конфигурация сетевого runtime.
#[derive(Debug, Clone)]
pub struct NetworkConfig {
    /// Таймаут подключения
    pub connect_timeout: Duration,
    /// Таймаут чтения
    pub read_timeout: Duration,
    /// Таймаут записи
    pub write_timeout: Duration,
    /// Размер буфера по умолчанию
    pub buffer_size: usize,
    /// Включить TCP_NODELAY
    pub tcp_nodelay: bool,
    /// Включить SO_REUSEADDR
    pub reuse_addr: bool,
    /// Backlog для listener'ов
    pub backlog: u32,
}

impl Default for NetworkConfig {
    fn default() -> Self {
        Self {
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            write_timeout: Duration::from_secs(60),
            buffer_size: 8192,
            tcp_nodelay: true,
            reuse_addr: true,
            backlog: 1024,
        }
    }
}

impl NetworkRuntime {
    /// Создаёт новый NetworkRuntime с текущим tokio handle.
    pub fn new() -> Self {
        Self {
            tokio_handle: tokio::runtime::Handle::current(),
            default_config: NetworkConfig::default(),
        }
    }

    /// Создаёт с явным tokio handle.
    pub fn with_handle(handle: tokio::runtime::Handle) -> Self {
        Self {
            tokio_handle: handle,
            default_config: NetworkConfig::default(),
        }
    }

    /// Создаёт с пользовательской конфигурацией.
    pub fn with_config(config: NetworkConfig) -> Self {
        Self {
            tokio_handle: tokio::runtime::Handle::current(),
            default_config: config,
        }
    }

    /// Возвращает tokio handle.
    pub fn handle(&self) -> &tokio::runtime::Handle {
        &self.tokio_handle
    }

    /// Возвращает конфигурацию.
    pub fn config(&self) -> &NetworkConfig {
        &self.default_config
    }

    // -------------------------------------------------------------------------
    // TCP
    // -------------------------------------------------------------------------

    /// Создаёт TCP listener на указанном адресе.
    pub async fn tcp_bind(&self, addr: impl Into<SocketAddr>) -> Result<TcpListener, NetError> {
        TcpListener::bind(addr.into(), TcpConfig::from(&self.default_config)).await
    }

    /// Подключается к TCP серверу.
    pub async fn tcp_connect(&self, addr: impl Into<SocketAddr>) -> Result<TcpConnection, NetError> {
        TcpConnection::connect(addr.into(), TcpConfig::from(&self.default_config)).await
    }

    // -------------------------------------------------------------------------
    // UDP
    // -------------------------------------------------------------------------

    /// Создаёт UDP сокет на указанном адресе.
    pub async fn udp_bind(&self, addr: impl Into<SocketAddr>) -> Result<UdpSocket, NetError> {
        UdpSocket::bind(addr.into(), UdpConfig::from(&self.default_config)).await
    }

    // -------------------------------------------------------------------------
    // HTTP
    // -------------------------------------------------------------------------

    /// Создаёт билдер HTTP сервера.
    pub fn http_server(&self) -> http::server::HttpServerBuilder {
        http::server::HttpServerBuilder::new(self.default_config.clone())
    }

    /// Создаёт HTTP клиент.
    pub fn http_client(&self) -> HttpClient {
        HttpClient::from_network_config(self.default_config.clone())
    }

    // -------------------------------------------------------------------------
    // TLS
    // -------------------------------------------------------------------------

    /// Создаёт TLS acceptor для сервера.
    pub fn tls_acceptor(&self, config: TlsConfig) -> Result<TlsAcceptor, NetError> {
        TlsAcceptor::new(config)
    }

    /// Создаёт TLS connector для клиента.
    pub fn tls_connector(&self, config: TlsConfig) -> Result<TlsConnector, NetError> {
        TlsConnector::new(config)
    }
}

impl Default for NetworkRuntime {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ОШИБКИ
// ============================================================================

/// Ошибки сетевой библиотеки.
#[derive(Debug)]
pub enum NetError {
    /// Ошибка ввода-вывода
    Io(std::io::Error),
    /// Таймаут операции
    Timeout,
    /// Соединение закрыто
    ConnectionClosed,
    /// Ошибка адреса
    InvalidAddress(String),
    /// Ошибка TLS
    Tls(String),
    /// Ошибка HTTP протокола
    Http(String),
    /// Ошибка WebSocket протокола
    WebSocket(String),
    /// Ошибка парсинга
    Parse(String),
    /// Ошибка валидации
    Validation(String),
    /// Ошибка сериализации/десериализации
    Serde(String),
    /// Ошибка сериализации (альтернативное имя)
    Serialization(String),
    /// Маршрут не найден
    NotFound,
    /// Метод не разрешён
    MethodNotAllowed,
    /// Внутренняя ошибка
    Internal(String),
}

impl std::fmt::Display for NetError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Io(e) => write!(f, "Ошибка ввода-вывода: {}", e),
            Self::Timeout => write!(f, "Таймаут операции"),
            Self::ConnectionClosed => write!(f, "Соединение закрыто"),
            Self::InvalidAddress(s) => write!(f, "Неверный адрес: {}", s),
            Self::Tls(s) => write!(f, "Ошибка TLS: {}", s),
            Self::Http(s) => write!(f, "Ошибка HTTP: {}", s),
            Self::WebSocket(s) => write!(f, "Ошибка WebSocket: {}", s),
            Self::Parse(s) => write!(f, "Ошибка парсинга: {}", s),
            Self::Validation(s) => write!(f, "Ошибка валидации: {}", s),
            Self::Serde(s) => write!(f, "Ошибка сериализации: {}", s),
            Self::Serialization(s) => write!(f, "Ошибка сериализации: {}", s),
            Self::NotFound => write!(f, "Маршрут не найден"),
            Self::MethodNotAllowed => write!(f, "Метод не разрешён"),
            Self::Internal(s) => write!(f, "Внутренняя ошибка: {}", s),
        }
    }
}

impl std::error::Error for NetError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Io(e) => Some(e),
            _ => None,
        }
    }
}

impl From<std::io::Error> for NetError {
    fn from(e: std::io::Error) -> Self {
        Self::Io(e)
    }
}

impl From<serde_json::Error> for NetError {
    fn from(e: serde_json::Error) -> Self {
        Self::Serde(e.to_string())
    }
}

// ============================================================================
//                    РЕЗУЛЬТАТ
// ============================================================================

/// Alias для Result с NetError.
pub type NetResult<T> = Result<T, NetError>;

// ============================================================================
//                    БИБЛИОТЕКА KUMIR
// ============================================================================

use crate::types::library::{
    LibraryDef, LibFunctionDef, LibParamDef, LibConstantDef, 
    ClassDef, LibFieldDef, LibVersion,
};
use crate::types::type_spec::TypeSpec;
use crate::types::Value;
use std::collections::BTreeMap;

// ============================================================================
//                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
// ============================================================================

fn expect_string(args: &[Value], idx: usize, name: &str) -> Result<String, String> {
    let v = args.get(idx).ok_or_else(|| format!("Не передан параметр: {}", name))?;
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(format!("Ожидается строка для параметра {}", name)),
    }
}

fn opt_string(args: &[Value], idx: usize, default: &str) -> String {
    args.get(idx)
        .and_then(|v| match v { Value::String(s) => Some(s.clone()), _ => None })
        .unwrap_or_else(|| default.to_string())
}

// URL encode/decode
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

fn url_decode(input: &str) -> String {
    let input = input.replace('+', " ");
    let mut result = String::new();
    let mut chars = input.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '%' {
            let hex: String = chars.by_ref().take(2).collect();
            if let Ok(byte) = u8::from_str_radix(&hex, 16) {
                result.push(byte as char);
            } else {
                result.push('%');
                result.push_str(&hex);
            }
        } else {
            result.push(c);
        }
    }
    result
}

// Base64
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
        result.push(if chunk.len() > 1 { BASE64_ALPHABET[((combined >> 6) & 0x3F) as usize] as char } else { '=' });
        result.push(if chunk.len() > 2 { BASE64_ALPHABET[(combined & 0x3F) as usize] as char } else { '=' });
    }
    result
}

fn base64_decode(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let mut result = Vec::new();
    let decode_char = |c: char| -> Result<u8, String> {
        match c {
            'A'..='Z' => Ok(c as u8 - b'A'),
            'a'..='z' => Ok(c as u8 - b'a' + 26),
            '0'..='9' => Ok(c as u8 - b'0' + 52),
            '+' => Ok(62),
            '/' => Ok(63),
            _ => Err(format!("Невалидный символ Base64: {}", c)),
        }
    };
    let chars: Vec<char> = input.chars().collect();
    for chunk in chars.chunks(4) {
        let mut values = [0u8; 4];
        for (i, &c) in chunk.iter().enumerate() {
            values[i] = decode_char(c)?;
        }
        let combined = ((values[0] as u32) << 18) | ((values[1] as u32) << 12)
            | ((values[2] as u32) << 6) | (values[3] as u32);
        result.push(((combined >> 16) & 0xFF) as u8);
        if chunk.len() > 2 { result.push(((combined >> 8) & 0xFF) as u8); }
        if chunk.len() > 3 { result.push((combined & 0xFF) as u8); }
    }
    Ok(result)
}

// JSON конвертация
fn json_to_value(json: &str) -> Result<Value, String> {
    let parsed: serde_json::Value = serde_json::from_str(json)
        .map_err(|e| format!("Ошибка парсинга JSON: {}", e))?;
    fn convert(v: serde_json::Value) -> Value {
        match v {
            serde_json::Value::Null => Value::Null,
            serde_json::Value::Bool(b) => Value::Boolean(b),
            serde_json::Value::Number(n) => {
                if let Some(i) = n.as_i64() { Value::Number(crate::types::Number::from(i)) }
                else if let Some(f) = n.as_f64() { Value::Number(crate::types::Number::from(f)) }
                else { Value::Null }
            }
            serde_json::Value::String(s) => Value::String(s),
            serde_json::Value::Array(arr) => Value::Array(arr.into_iter().map(convert).collect()),
            serde_json::Value::Object(obj) => {
                Value::Map(obj.into_iter().map(|(k, v)| (Value::String(k), convert(v))).collect())
            }
        }
    }
    Ok(convert(parsed))
}

fn value_to_json(value: &Value) -> Result<String, String> {
    fn convert(v: &Value) -> serde_json::Value {
        match v {
            Value::Null => serde_json::Value::Null,
            Value::Boolean(b) => serde_json::Value::Bool(*b),
            Value::Number(n) => {
                if let Some(i) = n.to_i64() { serde_json::Value::Number(i.into()) }
                else if let Some(f) = n.to_f64() { serde_json::Number::from_f64(f).map(serde_json::Value::Number).unwrap_or(serde_json::Value::Null) }
                else { serde_json::Value::Null }
            }
            Value::String(s) => serde_json::Value::String(s.clone()),
            Value::Char(c) => serde_json::Value::String(c.to_string()),
            Value::Array(arr) => serde_json::Value::Array(arr.iter().map(convert).collect()),
            Value::Map(map) => {
                let obj: serde_json::Map<String, serde_json::Value> = map.iter()
                    .filter_map(|(k, v)| match k { Value::String(s) => Some((s.clone(), convert(v))), _ => None })
                    .collect();
                serde_json::Value::Object(obj)
            }
            _ => serde_json::Value::Null,
        }
    }
    serde_json::to_string(&convert(value)).map_err(|e| format!("Ошибка сериализации JSON: {}", e))
}

// ============================================================================
//                    ФУНКЦИИ БИБЛИОТЕКИ
// ============================================================================

/// http_получить(url) -> лит
fn http_get_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_получить")
        .with_aliases(&["http_get", "получить_http"])
        .with_description("Выполняет HTTP GET запрос и возвращает тело ответа")
        .with_param(LibParamDef::value("url", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_example("результат := http_получить(\"https://api.example.com/data\")")
        .with_handler(|args| {
            let url = expect_string(args, 0, "url")?;
            let rt = get_tokio_handle()?;
            rt.block_on(async {
                let client = HttpClient::new();
                match client.get(&url).await {
                    Ok(mut response) => match response.read_text().await {
                        Ok(text) => Ok(Value::String(text)),
                        Err(e) => Err(format!("Ошибка чтения ответа: {}", e)),
                    },
                    Err(e) => Err(format!("Ошибка HTTP запроса: {}", e)),
                }
            })
        })
}

/// http_запрос(url, метод, тело) -> словарь
fn http_request_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_запрос")
        .with_aliases(&["http_request", "http_отправить"])
        .with_description("Выполняет HTTP запрос указанным методом")
        .with_param(LibParamDef::value("url", TypeSpec::String))
        .with_param(LibParamDef::value("метод", TypeSpec::String))
        .with_param(LibParamDef::value("тело", TypeSpec::String))
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::Any)))
        .with_example("ответ := http_запрос(\"https://api.example.com\", \"POST\", json)")
        .with_handler(|args| {
            let url = expect_string(args, 0, "url")?;
            let method = expect_string(args, 1, "метод")?;
            let body = opt_string(args, 2, "");
            let rt = get_tokio_handle()?;
            rt.block_on(async {
                let client = HttpClient::new();
                let method_enum = match method.to_uppercase().as_str() {
                    "GET" => http::Method::GET, "POST" => http::Method::POST,
                    "PUT" => http::Method::PUT, "DELETE" => http::Method::DELETE,
                    "PATCH" => http::Method::PATCH, "HEAD" => http::Method::HEAD,
                    "OPTIONS" => http::Method::OPTIONS,
                    _ => return Err(format!("Неизвестный HTTP метод: {}", method)),
                };
                let response = client.request(method_enum, &url).text(body).send().await
                    .map_err(|e| format!("Ошибка HTTP запроса: {}", e))?;
                let mut result = BTreeMap::new();
                result.insert(Value::String("статус".to_string()), Value::Number(crate::types::Number::from(response.status().code() as i64)));
                result.insert(Value::String("status".to_string()), Value::Number(crate::types::Number::from(response.status().code() as i64)));
                Ok(Value::Map(result))
            })
        })
}

/// http_json_получить(url) -> любой
fn http_get_json_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_json_получить")
        .with_aliases(&["http_get_json", "получить_json"])
        .with_description("Выполняет GET запрос и парсит JSON ответ")
        .with_param(LibParamDef::value("url", TypeSpec::String))
        .returns(TypeSpec::Any)
        .with_example("данные := http_json_получить(\"https://api.example.com/users\")")
        .with_handler(|args| {
            let url = expect_string(args, 0, "url")?;
            let rt = get_tokio_handle()?;
            rt.block_on(async {
                let client = HttpClient::new();
                let mut response = client.get(&url).await.map_err(|e| format!("Ошибка HTTP: {}", e))?;
                let text = response.read_text().await.map_err(|e| format!("Ошибка чтения: {}", e))?;
                json_to_value(&text)
            })
        })
}

/// http_json_отправить(url, данные) -> любой
fn http_post_json_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_json_отправить")
        .with_aliases(&["http_post_json", "отправить_json"])
        .with_description("Отправляет JSON данные методом POST")
        .with_param(LibParamDef::value("url", TypeSpec::String))
        .with_param(LibParamDef::value("данные", TypeSpec::Any))
        .returns(TypeSpec::Any)
        .with_example("ответ := http_json_отправить(\"https://api.example.com/users\", пользователь)")
        .with_handler(|args| {
            let url = expect_string(args, 0, "url")?;
            let data = args.get(1).cloned().unwrap_or(Value::Null);
            let json_str = value_to_json(&data)?;
            let rt = get_tokio_handle()?;
            rt.block_on(async {
                let client = HttpClient::new();
                let mut response = client.post(&url).await.header("Content-Type", "application/json")
                    .text(json_str).send().await.map_err(|e| format!("Ошибка HTTP: {}", e))?;
                let text = response.read_text().await.map_err(|e| format!("Ошибка чтения: {}", e))?;
                json_to_value(&text)
            })
        })
}

/// url_кодировать(строка) -> лит
fn url_encode_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_кодировать")
        .with_aliases(&["url_encode", "urlencode"])
        .with_description("Кодирует строку для URL (percent encoding)")
        .with_param(LibParamDef::value("строка", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let input = expect_string(args, 0, "строка")?;
            Ok(Value::String(url_encode(&input)))
        })
}

/// url_декодировать(строка) -> лит
fn url_decode_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_декодировать")
        .with_aliases(&["url_decode", "urldecode"])
        .with_description("Декодирует URL-кодированную строку")
        .with_param(LibParamDef::value("строка", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let input = expect_string(args, 0, "строка")?;
            Ok(Value::String(url_decode(&input)))
        })
}

/// url_разобрать(url) -> словарь
fn url_parse_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_разобрать")
        .with_aliases(&["url_parse", "parse_url"])
        .with_description("Разбирает URL на компоненты: схема, хост, порт, путь, запрос")
        .with_param(LibParamDef::value("url", TypeSpec::String))
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::String)))
        .with_handler(|args| {
            let url = expect_string(args, 0, "url")?;
            let mut result = BTreeMap::new();
            let (scheme, rest) = url.split_once("://").unwrap_or(("", &url));
            result.insert(Value::String("схема".to_string()), Value::String(scheme.to_string()));
            result.insert(Value::String("scheme".to_string()), Value::String(scheme.to_string()));
            let (host_port, path_query) = rest.split_once('/').unwrap_or((rest, ""));
            let (host, port) = host_port.split_once(':').map(|(h, p)| (h.to_string(), p.to_string())).unwrap_or((host_port.to_string(), String::new()));
            result.insert(Value::String("хост".to_string()), Value::String(host.clone()));
            result.insert(Value::String("host".to_string()), Value::String(host));
            result.insert(Value::String("порт".to_string()), Value::String(port.clone()));
            result.insert(Value::String("port".to_string()), Value::String(port));
            let (path, query) = path_query.split_once('?').unwrap_or((path_query, ""));
            result.insert(Value::String("путь".to_string()), Value::String(format!("/{}", path)));
            result.insert(Value::String("path".to_string()), Value::String(format!("/{}", path)));
            result.insert(Value::String("запрос".to_string()), Value::String(query.to_string()));
            result.insert(Value::String("query".to_string()), Value::String(query.to_string()));
            Ok(Value::Map(result))
        })
}

/// json_разобрать(строка) -> любой
fn json_parse_fn() -> LibFunctionDef {
    LibFunctionDef::new("json_разобрать")
        .with_aliases(&["json_parse", "parse_json"])
        .with_description("Парсит JSON строку в значение КуМир")
        .with_param(LibParamDef::value("строка", TypeSpec::String))
        .returns(TypeSpec::Any)
        .with_handler(|args| {
            let input = expect_string(args, 0, "строка")?;
            json_to_value(&input)
        })
}

/// json_строка(значение) -> лит
fn json_stringify_fn() -> LibFunctionDef {
    LibFunctionDef::new("json_строка")
        .with_aliases(&["json_stringify", "to_json", "в_json"])
        .with_description("Преобразует значение в JSON строку")
        .with_param(LibParamDef::value("значение", TypeSpec::Any))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let value = args.get(0).cloned().unwrap_or(Value::Null);
            value_to_json(&value).map(Value::String)
        })
}

/// base64_кодировать(строка) -> лит
fn base64_encode_fn() -> LibFunctionDef {
    LibFunctionDef::new("base64_кодировать")
        .with_aliases(&["base64_encode"])
        .with_description("Кодирует строку в Base64")
        .with_param(LibParamDef::value("строка", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let input = expect_string(args, 0, "строка")?;
            Ok(Value::String(base64_encode(input.as_bytes())))
        })
}

/// base64_декодировать(строка) -> лит
fn base64_decode_fn() -> LibFunctionDef {
    LibFunctionDef::new("base64_декодировать")
        .with_aliases(&["base64_decode"])
        .with_description("Декодирует Base64 строку")
        .with_param(LibParamDef::value("строка", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let input = expect_string(args, 0, "строка")?;
            let bytes = base64_decode(&input)?;
            String::from_utf8(bytes).map(Value::String).map_err(|e| format!("Невалидный UTF-8: {}", e))
        })
}

/// http_сервер_запустить(порт, html_содержимое) -> лог
/// Запускает простой HTTP сервер который отвечает на все запросы указанным HTML
fn http_server_simple_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_сервер_запустить")
        .with_aliases(&["http_serve", "serve_http", "запустить_сервер"])
        .with_description("Запускает простой HTTP сервер на указанном порту с заданным HTML содержимым")
        .with_param(LibParamDef::value("порт", TypeSpec::Int64))
        .with_param(LibParamDef::value("html", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_example("http_сервер_запустить(8080, \"<h1>Привет мир!</h1>\")")
        .with_handler(|args| {
            let port = args.get(0)
                .and_then(|v| match v { Value::Number(n) => n.to_i64(), _ => None })
                .unwrap_or(8080) as u16;
            let html_content = expect_string(args, 1, "html")?;
            
            let rt = tokio::runtime::Builder::new_multi_thread()
                .worker_threads(2)
                .enable_all()
                .build()
                .map_err(|e| format!("Не удалось создать runtime: {}", e))?;
            
            let html = html_content.clone();
            let router = http::Router::new()
                .get("/", move |_req| {
                    let html = html.clone();
                    async move {
                        http::Response::html(html)
                    }
                });
            
            let addr = format!("0.0.0.0:{}", port);
            
            eprintln!("🚀 Запуск HTTP сервера на http://localhost:{}", port);
            eprintln!("📝 Нажмите Ctrl+C для остановки");
            
            let result = rt.block_on(async {
                // Создаём сервер
                let server = http::HttpServerBuilder::new(NetworkConfig::default())
                    .bind(&addr)
                    .router(router)
                    .build()
                    .map_err(|e| format!("Ошибка создания сервера: {}", e))?;
                
                let shutdown_signal = server.shutdown_signal();
                
                // Запускаем сервер в отдельной задаче
                let server_handle = tokio::spawn(async move {
                    server.run().await
                });
                
                // Ждём Ctrl+C
                tokio::select! {
                    _ = tokio::signal::ctrl_c() => {
                        eprintln!("\n🛑 Получен сигнал завершения, останавливаем сервер...");
                        shutdown_signal.shutdown();
                    }
                    result = server_handle => {
                        match result {
                            Ok(Ok(())) => {},
                            Ok(Err(e)) => return Err(format!("Ошибка сервера: {}", e)),
                            Err(e) => return Err(format!("Ошибка задачи: {}", e)),
                        }
                    }
                }
                
                // Даём серверу время на graceful shutdown
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
                
                Ok::<_, String>(())
            });
            
            // Корректно завершаем runtime
            rt.shutdown_timeout(std::time::Duration::from_secs(1));
            
            result?;
            Ok(Value::Boolean(true))
        })
}

/// http_сервер_фоновый(порт, html_содержимое) -> цел
/// Запускает HTTP сервер в фоновом режиме
fn http_server_background_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_сервер_фоновый")
        .with_aliases(&["http_serve_background", "фоновый_сервер"])
        .with_description("Запускает HTTP сервер в фоновом режиме на указанном порту")
        .with_param(LibParamDef::value("порт", TypeSpec::Int64))
        .with_param(LibParamDef::value("html", TypeSpec::String))
        .returns(TypeSpec::Int64)
        .with_example("id := http_сервер_фоновый(8080, \"<h1>Привет!</h1>\")")
        .with_handler(|args| {
            let port = args.get(0)
                .and_then(|v| match v { Value::Number(n) => n.to_i64(), _ => None })
                .unwrap_or(8080) as u16;
            let html_content = expect_string(args, 1, "html")?;
            
            let rt = get_tokio_handle()?;
            
            // Запускаем сервер в отдельном потоке
            let html = html_content.clone();
            let handle = std::thread::spawn(move || {
                let router = http::Router::new()
                    .get("/", move |_req| {
                        let html = html.clone();
                        async move {
                            http::Response::html(html)
                        }
                    });
                
                let addr = format!("0.0.0.0:{}", port);
                
                // Создаём новый tokio runtime для фонового потока
                let rt = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap();
                    
                rt.block_on(async {
                    let _ = http::server::serve(addr, router).await;
                });
            });
            
            eprintln!("🚀 Фоновый HTTP сервер запущен на http://localhost:{}", port);
            
            // Возвращаем ID потока (упрощённо)
            Ok(Value::Number(crate::types::Number::from(port as i64)))
        })
}

// ============================================================================
//                    КОНСТАНТЫ БИБЛИОТЕКИ
// ============================================================================

fn const_http_get() -> LibConstantDef {
    LibConstantDef { name: "HTTP_GET", aliases: &["МЕТОД_GET"], const_type: TypeSpec::String, value: Value::String("GET".to_string()), description: "HTTP метод GET" }
}
fn const_http_post() -> LibConstantDef {
    LibConstantDef { name: "HTTP_POST", aliases: &["МЕТОД_POST"], const_type: TypeSpec::String, value: Value::String("POST".to_string()), description: "HTTP метод POST" }
}
fn const_http_put() -> LibConstantDef {
    LibConstantDef { name: "HTTP_PUT", aliases: &["МЕТОД_PUT"], const_type: TypeSpec::String, value: Value::String("PUT".to_string()), description: "HTTP метод PUT" }
}
fn const_http_delete() -> LibConstantDef {
    LibConstantDef { name: "HTTP_DELETE", aliases: &["МЕТОД_DELETE"], const_type: TypeSpec::String, value: Value::String("DELETE".to_string()), description: "HTTP метод DELETE" }
}
fn const_status_ok() -> LibConstantDef {
    LibConstantDef { name: "HTTP_OK", aliases: &["СТАТУС_ОК"], const_type: TypeSpec::Int64, value: Value::Number(crate::types::Number::from(200i64)), description: "HTTP 200 OK" }
}
fn const_status_not_found() -> LibConstantDef {
    LibConstantDef { name: "HTTP_NOT_FOUND", aliases: &["СТАТУС_НЕ_НАЙДЕНО"], const_type: TypeSpec::Int64, value: Value::Number(crate::types::Number::from(404i64)), description: "HTTP 404 Not Found" }
}
fn const_status_error() -> LibConstantDef {
    LibConstantDef { name: "HTTP_ERROR", aliases: &["СТАТУС_ОШИБКА"], const_type: TypeSpec::Int64, value: Value::Number(crate::types::Number::from(500i64)), description: "HTTP 500 Internal Error" }
}
fn const_port_http() -> LibConstantDef {
    LibConstantDef { name: "ПОРТ_HTTP", aliases: &["HTTP_PORT"], const_type: TypeSpec::Int64, value: Value::Number(crate::types::Number::from(80i64)), description: "Стандартный порт HTTP" }
}
fn const_port_https() -> LibConstantDef {
    LibConstantDef { name: "ПОРТ_HTTPS", aliases: &["HTTPS_PORT"], const_type: TypeSpec::Int64, value: Value::Number(crate::types::Number::from(443i64)), description: "Стандартный порт HTTPS" }
}

// ============================================================================
//                    КЛАССЫ БИБЛИОТЕКИ
// ============================================================================

fn class_http_client() -> ClassDef {
    let mut class = ClassDef::new("HttpКлиент");
    class.aliases = &["HttpClient", "http_клиент"];
    class.description = "HTTP клиент для выполнения запросов";
    class.fields = vec![
        LibFieldDef { name: "таймаут", field_type: TypeSpec::Int64, description: "Таймаут в секундах", readonly: false },
        LibFieldDef { name: "базовый_url", field_type: TypeSpec::String, description: "Базовый URL", readonly: false },
    ];
    class.methods = vec!["получить", "отправить", "запрос", "get", "post", "put", "delete"];
    class.static_methods = vec!["создать", "new"];
    class.constructors = vec!["создать", "new"];
    class.is_native = true;
    class
}

fn class_http_response() -> ClassDef {
    let mut class = ClassDef::new("HttpОтвет");
    class.aliases = &["HttpResponse", "http_ответ"];
    class.description = "HTTP ответ";
    class.fields = vec![
        LibFieldDef { name: "статус", field_type: TypeSpec::Int64, description: "Код статуса", readonly: true },
        LibFieldDef { name: "заголовки", field_type: TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::String)), description: "Заголовки ответа", readonly: true },
        LibFieldDef { name: "тело", field_type: TypeSpec::String, description: "Тело ответа", readonly: true },
    ];
    class.methods = vec!["текст", "json", "байты", "text", "json", "bytes"];
    class.is_native = true;
    class
}

fn class_http_request() -> ClassDef {
    let mut class = ClassDef::new("HttpЗапрос");
    class.aliases = &["HttpRequest", "http_запрос"];
    class.description = "HTTP запрос";
    class.fields = vec![
        LibFieldDef { name: "метод", field_type: TypeSpec::String, description: "HTTP метод", readonly: false },
        LibFieldDef { name: "url", field_type: TypeSpec::String, description: "URL запроса", readonly: false },
        LibFieldDef { name: "заголовки", field_type: TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::String)), description: "Заголовки", readonly: false },
        LibFieldDef { name: "тело", field_type: TypeSpec::String, description: "Тело запроса", readonly: false },
    ];
    class.methods = vec!["заголовок", "тело", "отправить", "header", "body", "send"];
    class.constructors = vec!["создать", "new"];
    class.is_native = true;
    class
}

fn class_router() -> ClassDef {
    let mut class = ClassDef::new("Маршрутизатор");
    class.aliases = &["Router", "роутер"];
    class.description = "HTTP маршрутизатор для сервера";
    class.methods = vec!["get", "post", "put", "delete", "route", "группа", "middleware"];
    class.constructors = vec!["создать", "new"];
    class.is_native = true;
    class
}

fn class_http_server() -> ClassDef {
    let mut class = ClassDef::new("HttpСервер");
    class.aliases = &["HttpServer", "http_сервер"];
    class.description = "HTTP сервер";
    class.fields = vec![
        LibFieldDef { name: "адрес", field_type: TypeSpec::String, description: "Адрес сервера", readonly: true },
        LibFieldDef { name: "порт", field_type: TypeSpec::Int64, description: "Порт сервера", readonly: true },
    ];
    class.methods = vec!["запустить", "остановить", "serve", "shutdown"];
    class.constructors = vec!["создать", "new"];
    class.is_native = true;
    class
}

// ============================================================================
//                    СОЗДАНИЕ БИБЛИОТЕКИ
// ============================================================================

/// Создаёт библиотеку net для КуМир
pub fn create_net_library() -> LibraryDef {
    let mut lib = LibraryDef::new("net", "Сеть");
    lib.aliases = &["net", "network", "сеть", "http", "Сокеты", "сокеты", "socket", "sockets"];
    lib.description = "Сетевая библиотека: HTTP клиент/сервер, TCP, UDP, WebSocket";
    lib.author = "Vadim Khristenko <just@vai-prog.ru>";
    lib.version = LibVersion::new(1, 0, 0);
    lib.stable = false;

    // Функции
    lib.functions = vec![
        http_get_fn(),
        http_request_fn(),
        http_get_json_fn(),
        http_post_json_fn(),
        http_server_simple_fn(),
        http_server_background_fn(),
        url_encode_fn(),
        url_decode_fn(),
        url_parse_fn(),
        json_parse_fn(),
        json_stringify_fn(),
        base64_encode_fn(),
        base64_decode_fn(),
    ];

    // Константы
    lib.constants = vec![
        const_http_get(),
        const_http_post(),
        const_http_put(),
        const_http_delete(),
        const_status_ok(),
        const_status_not_found(),
        const_status_error(),
        const_port_http(),
        const_port_https(),
    ];

    // Классы
    lib.classes = vec![
        class_http_client(),
        class_http_response(),
        class_http_request(),
        class_router(),
        class_http_server(),
    ];

    lib
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NetworkConfig::default();
        assert_eq!(config.buffer_size, 8192);
        assert!(config.tcp_nodelay);
    }

    #[tokio::test]
    async fn test_network_runtime_creation() {
        let runtime = NetworkRuntime::new();
        assert_eq!(runtime.config().buffer_size, 8192);
    }

    #[test]
    fn test_library_creation() {
        let lib = create_net_library();
        assert_eq!(lib.name, "Сеть");
        assert!(!lib.functions.is_empty());
        assert!(!lib.constants.is_empty());
        assert!(!lib.classes.is_empty());
    }

    #[test]
    fn test_url_encode_decode() {
        assert_eq!(url_encode("Hello World"), "Hello%20World");
        assert_eq!(url_decode("Hello%20World"), "Hello World");
    }

    #[test]
    fn test_base64() {
        assert_eq!(base64_encode(b"Hello"), "SGVsbG8=");
        assert_eq!(base64_decode("SGVsbG8=").unwrap(), b"Hello");
    }
}
