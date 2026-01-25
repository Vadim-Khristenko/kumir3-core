// ============================================================================
//                    HTTP МОДУЛЬ
// ============================================================================
//
// Полноценный HTTP/1.1 сервер и клиент с FastAPI-подобной маршрутизацией:
// - Router с path params, query, body validation
// - Middleware система
// - Dependency Injection
// - Streaming bodies
// - WebSocket upgrade
// - OpenAPI генерация
//
// ============================================================================

pub mod types;
pub mod request;
pub mod response;
pub mod body;
pub mod router;
pub mod server;
pub mod client;
pub mod middleware;
pub mod deps;
pub mod extractors;
pub mod ws;
pub mod openapi;

// Реэкспорты
pub use types::{Method, StatusCode, Headers, HeaderName, HeaderValue};
pub use request::{Request, RequestBuilder};
pub use response::{Response, ResponseBuilder, IntoResponse};
pub use body::{Body, BodyStream, BodySender};
pub use router::{Router, Route, Handler, RouteMatch, RouteGroup, RouteMeta};
pub use server::{HttpServer, HttpServerBuilder, HttpServerConfig, serve};
pub use client::{HttpClient, HttpClientConfig, ClientRequestBuilder, get, post_json};
pub use middleware::{
    Middleware, MiddlewareChain, Next,
    LoggingMiddleware, CorsMiddleware, CorsConfig,
    TimeoutMiddleware, RequestIdMiddleware, RateLimitMiddleware,
    from_fn,
};
pub use deps::{Dependency, FromRequest, State, DependencyContainer, Rejection};
pub use extractors::{
    Json, Query, Path, Form,
    ExtractedHeader, TypedHeader, Bytes, Text,
    Authorization, ContentType, UserAgent, ConnectInfo,
    PathParams,
};
pub use ws::{WebSocket, Message as WebSocketMessage, Opcode, is_upgrade_request, WebSocketUpgrader};
pub use openapi::{OpenApiSpec, swagger_ui_html, redoc_html};
