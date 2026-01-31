// ============================================================================
//                    HTTP EXTRACTORS
// ============================================================================
//
// FastAPI-подобные экстракторы:
// - Json<T> — JSON из тела
// - Query<T> — из query string
// - Path<T> — path параметры
// - Form<T> — form data
// - Header<T> — заголовок
//
// ============================================================================

use std::collections::HashMap;

use serde::de::DeserializeOwned;

use super::request::Request;
use super::deps::{FromRequest, Rejection};
use super::types::HeaderName;


// ============================================================================
//                    JSON EXTRACTOR
// ============================================================================

/// Извлекает JSON из тела запроса.
/// 
/// # Пример
/// ```rust
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct CreateUser {
///     name: String,
///     email: String,
/// }
/// 
/// async fn create_user(Json(user): Json<CreateUser>) -> Response {
///     Response::json(&user)
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Json<T>(pub T);

impl<T> Json<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Json<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T> std::ops::DerefMut for Json<T> {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequest for Json<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        // Проверяем Content-Type
        let content_type = req.headers()
            .get(HeaderName::CONTENT_TYPE)
            .map(|v| v.as_str())
            .unwrap_or("");

        if !content_type.contains("application/json") {
            return Err(Rejection::bad_request(
                format!("Expected application/json, got: {}", content_type)
            ));
        }

        // Читаем тело
        let body = req.bytes().await
            .map_err(|e| Rejection::bad_request(format!("Failed to read body: {}", e)))?;

        // Парсим JSON
        serde_json::from_slice(&body)
            .map(Json)
            .map_err(|e| Rejection::bad_request(format!("Invalid JSON: {}", e)))
    }
}

// ============================================================================
//                    QUERY EXTRACTOR
// ============================================================================

/// Извлекает параметры из query string.
/// 
/// # Пример
/// ```rust
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct Pagination {
///     page: Option<u32>,
///     limit: Option<u32>,
/// }
/// 
/// async fn list_users(Query(params): Query<Pagination>) -> Response {
///     let page = params.page.unwrap_or(1);
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Query<T>(pub T);

impl<T> Query<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Query<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequest for Query<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        // Собираем query params в HashMap для serde
        let query = req.query_params();
        
        // Сериализуем в JSON и обратно (простой способ)
        let json = serde_json::to_value(&query)
            .map_err(|e| Rejection::bad_request(format!("Query serialization error: {}", e)))?;

        serde_json::from_value(json)
            .map(Query)
            .map_err(|e| Rejection::bad_request(format!("Invalid query parameters: {}", e)))
    }
}

// ============================================================================
//                    PATH EXTRACTOR
// ============================================================================

/// Извлекает path параметры.
/// 
/// # Пример
/// ```rust
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct UserPath {
///     user_id: u64,
/// }
/// 
/// // Route: "/users/{user_id}"
/// async fn get_user(Path(params): Path<UserPath>) -> Response {
///     let user_id = params.user_id;
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Path<T>(pub T);

impl<T> Path<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Path<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequest for Path<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        // Path параметры хранятся в extensions
        let params = req.extensions()
            .get::<PathParams>()
            .ok_or_else(|| Rejection::internal("Path params not extracted"))?;

        let json = serde_json::to_value(&params.0)
            .map_err(|e| Rejection::internal(format!("Path serialization error: {}", e)))?;

        serde_json::from_value(json)
            .map(Path)
            .map_err(|e| Rejection::bad_request(format!("Invalid path parameters: {}", e)))
    }
}

/// Внутренний тип для хранения path params.
#[derive(Debug, Clone)]
pub struct PathParams(pub HashMap<String, String>);

// ============================================================================
//                    FORM EXTRACTOR
// ============================================================================

/// Извлекает form data (application/x-www-form-urlencoded).
/// 
/// # Пример
/// ```rust
/// use serde::Deserialize;
/// 
/// #[derive(Deserialize)]
/// struct LoginForm {
///     username: String,
///     password: String,
/// }
/// 
/// async fn login(Form(form): Form<LoginForm>) -> Response {
///     // ...
/// }
/// ```
#[derive(Debug, Clone)]
pub struct Form<T>(pub T);

impl<T> Form<T> {
    pub fn new(value: T) -> Self {
        Self(value)
    }

    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for Form<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl<T: DeserializeOwned + Send> FromRequest for Form<T> {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        // Проверяем Content-Type
        let content_type = req.headers()
            .get(HeaderName::CONTENT_TYPE)
            .map(|v| v.as_str())
            .unwrap_or("");

        if !content_type.contains("application/x-www-form-urlencoded") {
            return Err(Rejection::bad_request(
                format!("Expected application/x-www-form-urlencoded, got: {}", content_type)
            ));
        }

        // Читаем тело
        let body = req.bytes().await
            .map_err(|e| Rejection::bad_request(format!("Failed to read body: {}", e)))?;

        let body_str = String::from_utf8(body)
            .map_err(|e| Rejection::bad_request(format!("Invalid UTF-8: {}", e)))?;

        // Парсим form data
        let params = parse_form_urlencoded(&body_str);

        let json = serde_json::to_value(&params)
            .map_err(|e| Rejection::internal(format!("Form serialization error: {}", e)))?;

        serde_json::from_value(json)
            .map(Form)
            .map_err(|e| Rejection::bad_request(format!("Invalid form data: {}", e)))
    }
}

fn parse_form_urlencoded(input: &str) -> HashMap<String, String> {
    let mut result = HashMap::new();
    
    for pair in input.split('&') {
        if let Some((key, value)) = pair.split_once('=') {
            let key = url_decode(key);
            let value = url_decode(value);
            result.insert(key, value);
        }
    }
    
    result
}

fn url_decode(input: &str) -> String {
    let input = input.replace('+', " ");
    let mut result = String::with_capacity(input.len());
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

// ============================================================================
//                    HEADER EXTRACTOR
// ============================================================================

/// Извлекает значение заголовка.
/// 
/// # Пример
/// ```rust
/// async fn handler(req: &mut Request) -> Response {
///     let auth = ExtractedHeader::extract(req, "Authorization");
///     // auth содержит значение заголовка
/// }
/// ```
pub struct ExtractedHeader(pub Option<String>);

impl ExtractedHeader {
    /// Извлекает значение заголовка по имени.
    pub fn extract(req: &Request, name: &str) -> Self {
        let value = req.headers()
            .get(name)
            .map(|v| v.as_str().to_string());
        ExtractedHeader(value)
    }

    pub fn value(&self) -> Option<&str> {
        self.0.as_deref()
    }

    pub fn into_inner(self) -> Option<String> {
        self.0
    }
}

// ============================================================================
//                    TYPED HEADER EXTRACTOR
// ============================================================================

/// Извлекает типизированный заголовок.
pub struct TypedHeader<T>(pub T);

impl<T> TypedHeader<T> {
    pub fn into_inner(self) -> T {
        self.0
    }
}

impl<T> std::ops::Deref for TypedHeader<T> {
    type Target = T;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

// ============================================================================
//                    COMMON HEADERS
// ============================================================================

/// Authorization header.
#[derive(Debug, Clone)]
pub struct Authorization(pub String);

impl Authorization {
    /// Возвращает схему (Bearer, Basic, etc.)
    pub fn scheme(&self) -> Option<&str> {
        self.0.split_whitespace().next()
    }

    /// Возвращает credentials.
    pub fn credentials(&self) -> Option<&str> {
        self.0.split_once(' ').map(|(_, cred)| cred)
    }

    /// Проверяет Bearer token.
    pub fn bearer_token(&self) -> Option<&str> {
        if self.scheme()?.eq_ignore_ascii_case("bearer") {
            self.credentials()
        } else {
            None
        }
    }
}

impl FromRequest for Authorization {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.headers()
            .get(HeaderName::AUTHORIZATION)
            .map(|v| Authorization(v.as_str().to_string()))
            .ok_or_else(|| Rejection::unauthorized("Authorization header required"))
    }
}

/// Content-Type header.
#[derive(Debug, Clone)]
pub struct ContentType(pub String);

impl ContentType {
    pub fn mime_type(&self) -> &str {
        self.0.split(';').next().unwrap_or(&self.0).trim()
    }

    pub fn is_json(&self) -> bool {
        self.mime_type().eq_ignore_ascii_case("application/json")
    }

    pub fn is_form(&self) -> bool {
        self.mime_type().eq_ignore_ascii_case("application/x-www-form-urlencoded")
    }
}

impl FromRequest for ContentType {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.headers()
            .get(HeaderName::CONTENT_TYPE)
            .map(|v| ContentType(v.as_str().to_string()))
            .ok_or_else(|| Rejection::bad_request("Content-Type header required"))
    }
}

/// User-Agent header.
#[derive(Debug, Clone)]
pub struct UserAgent(pub String);

impl FromRequest for UserAgent {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        Ok(UserAgent(
            req.headers()
                .get(HeaderName::USER_AGENT)
                .map(|v| v.as_str().to_string())
                .unwrap_or_default()
        ))
    }
}

// ============================================================================
//                    BODY EXTRACTORS
// ============================================================================

/// Извлекает тело как bytes (лимит по умолчанию).
pub struct Bytes(pub Vec<u8>);

impl Bytes {
    pub fn into_inner(self) -> Vec<u8> {
        self.0
    }
}

impl std::ops::Deref for Bytes {
    type Target = [u8];

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for Bytes {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.bytes().await
            .map(Bytes)
            .map_err(|e| Rejection::bad_request(format!("Failed to read body: {}", e)))
    }
}

/// Извлекает тело как текст.
pub struct Text(pub String);

impl Text {
    pub fn into_inner(self) -> String {
        self.0
    }
}

impl std::ops::Deref for Text {
    type Target = str;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

impl FromRequest for Text {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        let bytes = req.bytes().await
            .map_err(|e| Rejection::bad_request(format!("Failed to read body: {}", e)))?;

        String::from_utf8(bytes)
            .map(Text)
            .map_err(|e| Rejection::bad_request(format!("Invalid UTF-8: {}", e)))
    }
}

// ============================================================================
//                    CONNECT INFO
// ============================================================================

/// Информация о соединении.
#[derive(Debug, Clone)]
pub struct ConnectInfo {
    pub remote_addr: std::net::SocketAddr,
}

impl FromRequest for ConnectInfo {
    type Error = Rejection;

    async fn from_request(req: &mut Request) -> Result<Self, Self::Error> {
        req.remote_addr()
            .map(|addr| ConnectInfo { remote_addr: addr })
            .ok_or_else(|| Rejection::internal("Connection info not available"))
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_url_decode() {
        assert_eq!(url_decode("hello%20world"), "hello world");
        assert_eq!(url_decode("hello+world"), "hello world");
        assert_eq!(url_decode("test%3D123"), "test=123");
    }

    #[test]
    fn test_parse_form_urlencoded() {
        let params = parse_form_urlencoded("name=John&age=30&city=New%20York");
        assert_eq!(params.get("name"), Some(&"John".to_string()));
        assert_eq!(params.get("age"), Some(&"30".to_string()));
        assert_eq!(params.get("city"), Some(&"New York".to_string()));
    }

    #[test]
    fn test_authorization_parse() {
        let auth = Authorization("Bearer token123".to_string());
        assert_eq!(auth.scheme(), Some("Bearer"));
        assert_eq!(auth.bearer_token(), Some("token123"));

        let basic = Authorization("Basic dXNlcjpwYXNz".to_string());
        assert_eq!(basic.scheme(), Some("Basic"));
        assert_eq!(basic.bearer_token(), None);
    }

    #[test]
    fn test_content_type() {
        let ct = ContentType("application/json; charset=utf-8".to_string());
        assert_eq!(ct.mime_type(), "application/json");
        assert!(ct.is_json());
    }
}
