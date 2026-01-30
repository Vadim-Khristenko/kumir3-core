// ============================================================================
//                    HTTP RESPONSE
// ============================================================================

use super::types::{StatusCode, Headers, HeaderName, mime};
use super::body::Body;
use crate::libraries::net::NetResult;

// ============================================================================
//                    RESPONSE
// ============================================================================

/// HTTP ответ.
#[derive(Debug)]
pub struct Response {
    /// Статус код
    status: StatusCode,
    /// Заголовки
    headers: Headers,
    /// Тело ответа
    body: Body,
}

impl Response {
    /// Создаёт новый ответ.
    pub fn new(status: StatusCode) -> Self {
        Self {
            status,
            headers: Headers::new(),
            body: Body::empty(),
        }
    }

    /// Создаёт билдер.
    pub fn builder() -> ResponseBuilder {
        ResponseBuilder::new()
    }

    // -------------------------------------------------------------------------
    // Shortcut constructors
    // -------------------------------------------------------------------------

    /// 200 OK с пустым телом.
    pub fn ok() -> Self {
        Self::new(StatusCode::OK)
    }

    /// 200 OK с текстом.
    pub fn text(text: impl Into<String>) -> Self {
        Self::builder()
            .status(StatusCode::OK)
            .content_type(mime::TEXT_PLAIN)
            .body(text.into())
            .build()
    }

    /// 200 OK с HTML.
    pub fn html(html: impl Into<String>) -> Self {
        Self::builder()
            .status(StatusCode::OK)
            .content_type(mime::TEXT_HTML)
            .body(html.into())
            .build()
    }

    /// 200 OK с JSON.
    pub fn json<T: serde::Serialize>(value: &T) -> NetResult<Self> {
        let json = serde_json::to_vec(value)?;
        Ok(Self::builder()
            .status(StatusCode::OK)
            .content_type(mime::APPLICATION_JSON)
            .body(json)
            .build())
    }

    /// 201 Created.
    pub fn created() -> Self {
        Self::new(StatusCode::CREATED)
    }

    /// 204 No Content.
    pub fn no_content() -> Self {
        Self::new(StatusCode::NO_CONTENT)
    }

    /// 301 Moved Permanently.
    pub fn redirect_permanent(location: impl Into<String>) -> Self {
        Self::builder()
            .status(StatusCode::MOVED_PERMANENTLY)
            .header(HeaderName::LOCATION, location.into())
            .build()
    }

    /// 302 Found (временный редирект).
    pub fn redirect(location: impl Into<String>) -> Self {
        Self::builder()
            .status(StatusCode::FOUND)
            .header(HeaderName::LOCATION, location.into())
            .build()
    }

    /// 400 Bad Request.
    pub fn bad_request(message: impl Into<String>) -> Self {
        Self::error(StatusCode::BAD_REQUEST, message)
    }

    /// 401 Unauthorized.
    pub fn unauthorized() -> Self {
        Self::error(StatusCode::UNAUTHORIZED, "Unauthorized")
    }

    /// 403 Forbidden.
    pub fn forbidden() -> Self {
        Self::error(StatusCode::FORBIDDEN, "Forbidden")
    }

    /// 404 Not Found.
    pub fn not_found() -> Self {
        Self::error(StatusCode::NOT_FOUND, "Not Found")
    }

    /// 405 Method Not Allowed.
    pub fn method_not_allowed() -> Self {
        Self::error(StatusCode::METHOD_NOT_ALLOWED, "Method Not Allowed")
    }

    /// 422 Unprocessable Entity.
    pub fn unprocessable_entity(message: impl Into<String>) -> Self {
        Self::error(StatusCode::UNPROCESSABLE_ENTITY, message)
    }

    /// 500 Internal Server Error.
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::error(StatusCode::INTERNAL_SERVER_ERROR, message)
    }

    /// Создаёт error response с JSON телом.
    pub fn error(status: StatusCode, message: impl Into<String>) -> Self {
        let error = serde_json::json!({
            "error": {
                "code": status.code(),
                "message": message.into()
            }
        });
        
        Self::builder()
            .status(status)
            .content_type(mime::APPLICATION_JSON)
            .body(error.to_string())
            .build()
    }

    // -------------------------------------------------------------------------
    // Getters
    // -------------------------------------------------------------------------

    pub fn status(&self) -> StatusCode {
        self.status
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

    /// Читает тело как байты (async).
    pub async fn read_bytes(&mut self) -> NetResult<Vec<u8>> {
        self.body.bytes().await
    }

    /// Читает тело как текст (async).
    pub async fn read_text(&mut self) -> NetResult<String> {
        self.body.text().await
    }

    /// Читает тело как JSON (async).
    pub async fn read_json<T: serde::de::DeserializeOwned>(&mut self) -> NetResult<T> {
        self.body.json().await
    }

    // -------------------------------------------------------------------------
    // Setters
    // -------------------------------------------------------------------------

    pub fn set_status(&mut self, status: StatusCode) {
        self.status = status;
    }

    pub fn set_body(&mut self, body: impl Into<Body>) {
        self.body = body.into();
    }

    /// Забирает тело.
    pub fn take_body(&mut self) -> Body {
        std::mem::replace(&mut self.body, Body::empty())
    }

    // -------------------------------------------------------------------------
    // Header shortcuts
    // -------------------------------------------------------------------------

    /// Устанавливает Content-Type.
    pub fn content_type(mut self, mime: &str) -> Self {
        self.headers.set_content_type(mime);
        self
    }

    /// Добавляет заголовок.
    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    /// Устанавливает cookie.
    pub fn cookie(mut self, name: &str, value: &str) -> Self {
        self.headers.append(HeaderName::SET_COOKIE, format!("{}={}", name, value));
        self
    }

    /// Устанавливает cookie с опциями.
    pub fn cookie_with_options(
        mut self,
        name: &str,
        value: &str,
        options: CookieOptions,
    ) -> Self {
        let mut cookie = format!("{}={}", name, value);
        
        if let Some(max_age) = options.max_age {
            cookie.push_str(&format!("; Max-Age={}", max_age));
        }
        if let Some(ref domain) = options.domain {
            cookie.push_str(&format!("; Domain={}", domain));
        }
        if let Some(ref path) = options.path {
            cookie.push_str(&format!("; Path={}", path));
        }
        if options.secure {
            cookie.push_str("; Secure");
        }
        if options.http_only {
            cookie.push_str("; HttpOnly");
        }
        if let Some(ref same_site) = options.same_site {
            cookie.push_str(&format!("; SameSite={}", same_site));
        }
        
        self.headers.append(HeaderName::SET_COOKIE, cookie);
        self
    }

    // -------------------------------------------------------------------------
    // Serialization
    // -------------------------------------------------------------------------

    /// Сериализует ответ в байты (для отправки по сети).
    pub async fn to_bytes(&mut self) -> NetResult<Vec<u8>> {
        let mut output = Vec::new();
        
        // Status line
        output.extend_from_slice(
            format!("HTTP/1.1 {}\r\n", self.status).as_bytes()
        );
        
        // Получаем тело
        let body_bytes = self.body.bytes().await?;
        
        // Content-Length если не установлен
        if !self.headers.contains(HeaderName::CONTENT_LENGTH) && !body_bytes.is_empty() {
            self.headers.set_content_length(body_bytes.len());
        }
        
        // Server header
        if !self.headers.contains(HeaderName::SERVER) {
            self.headers.insert(HeaderName::SERVER, "Kumir3-Net/1.0");
        }
        
        // Headers
        for (name, values) in self.headers.iter() {
            for value in values {
                output.extend_from_slice(
                    format!("{}: {}\r\n", name, value).as_bytes()
                );
            }
        }
        
        // Empty line
        output.extend_from_slice(b"\r\n");
        
        // Body
        output.extend_from_slice(&body_bytes);
        
        Ok(output)
    }
}

impl Default for Response {
    fn default() -> Self {
        Self::ok()
    }
}

// ============================================================================
//                    RESPONSE BUILDER
// ============================================================================

/// Билдер для Response.
pub struct ResponseBuilder {
    status: StatusCode,
    headers: Headers,
    body: Body,
}

impl ResponseBuilder {
    pub fn new() -> Self {
        Self {
            status: StatusCode::OK,
            headers: Headers::new(),
            body: Body::empty(),
        }
    }

    pub fn status(mut self, status: StatusCode) -> Self {
        self.status = status;
        self
    }

    pub fn header(mut self, name: impl Into<String>, value: impl Into<String>) -> Self {
        self.headers.insert(name.into(), value.into());
        self
    }

    pub fn content_type(mut self, mime: &str) -> Self {
        self.headers.set_content_type(mime);
        self
    }

    pub fn body(mut self, body: impl Into<Body>) -> Self {
        self.body = body.into();
        self
    }

    pub fn json<T: serde::Serialize>(mut self, value: &T) -> NetResult<Self> {
        let json = serde_json::to_vec(value)?;
        self.headers.set_content_type(mime::APPLICATION_JSON);
        self.body = Body::from(json);
        Ok(self)
    }

    pub fn build(self) -> Response {
        Response {
            status: self.status,
            headers: self.headers,
            body: self.body,
        }
    }
}

impl Default for ResponseBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    COOKIE OPTIONS
// ============================================================================

/// Опции для cookie.
#[derive(Debug, Clone, Default)]
pub struct CookieOptions {
    /// Max-Age в секундах
    pub max_age: Option<i64>,
    /// Domain
    pub domain: Option<String>,
    /// Path
    pub path: Option<String>,
    /// Secure flag
    pub secure: bool,
    /// HttpOnly flag
    pub http_only: bool,
    /// SameSite
    pub same_site: Option<String>,
}

impl CookieOptions {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn max_age(mut self, seconds: i64) -> Self {
        self.max_age = Some(seconds);
        self
    }

    pub fn domain(mut self, domain: impl Into<String>) -> Self {
        self.domain = Some(domain.into());
        self
    }

    pub fn path(mut self, path: impl Into<String>) -> Self {
        self.path = Some(path.into());
        self
    }

    pub fn secure(mut self) -> Self {
        self.secure = true;
        self
    }

    pub fn http_only(mut self) -> Self {
        self.http_only = true;
        self
    }

    pub fn same_site_strict(mut self) -> Self {
        self.same_site = Some("Strict".into());
        self
    }

    pub fn same_site_lax(mut self) -> Self {
        self.same_site = Some("Lax".into());
        self
    }

    pub fn same_site_none(mut self) -> Self {
        self.same_site = Some("None".into());
        self
    }
}

// ============================================================================
//                    INTO RESPONSE TRAIT
// ============================================================================

/// Трейт для типов, которые можно конвертировать в Response.
pub trait IntoResponse {
    fn into_response(self) -> Response;
}

impl IntoResponse for Response {
    fn into_response(self) -> Response {
        self
    }
}

impl IntoResponse for () {
    fn into_response(self) -> Response {
        Response::no_content()
    }
}

impl IntoResponse for &str {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for String {
    fn into_response(self) -> Response {
        Response::text(self)
    }
}

impl IntoResponse for Vec<u8> {
    fn into_response(self) -> Response {
        Response::builder()
            .body(self)
            .build()
    }
}

impl<T: serde::Serialize> IntoResponse for (StatusCode, T) {
    fn into_response(self) -> Response {
        let (status, value) = self;
        match serde_json::to_vec(&value) {
            Ok(json) => Response::builder()
                .status(status)
                .content_type(mime::APPLICATION_JSON)
                .body(json)
                .build(),
            Err(e) => Response::internal_error(e.to_string()),
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
    fn test_response_builder() {
        let resp = Response::builder()
            .status(StatusCode::CREATED)
            .header("X-Custom", "value")
            .content_type(mime::APPLICATION_JSON)
            .body(r#"{"id": 1}"#)
            .build();

        assert_eq!(resp.status(), StatusCode::CREATED);
        assert!(resp.headers().contains("x-custom"));
    }

    #[test]
    fn test_response_shortcuts() {
        assert_eq!(Response::ok().status(), StatusCode::OK);
        assert_eq!(Response::not_found().status(), StatusCode::NOT_FOUND);
        assert_eq!(Response::internal_error("oops").status(), StatusCode::INTERNAL_SERVER_ERROR);
    }

    #[test]
    fn test_cookie_options() {
        let opts = CookieOptions::new()
            .max_age(3600)
            .path("/")
            .http_only()
            .same_site_strict();

        assert_eq!(opts.max_age, Some(3600));
        assert!(opts.http_only);
    }
}
