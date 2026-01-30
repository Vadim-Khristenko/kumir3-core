// ============================================================================
//                    HTTP ROUTER (FastAPI-style)
// ============================================================================
//
// Маршрутизатор с поддержкой:
// - Path параметров: /users/{id}, /posts/{post_id}/comments/{comment_id}
// - Wildcards: /static/*path
// - Группы маршрутов с префиксами
// - Middleware на уровне роутера и маршрута
// - OpenAPI метаданные
//
// ============================================================================

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;

use super::request::Request;
use super::response::Response;
use super::middleware::{Middleware, MiddlewareChain};
use super::types::Method;


// ============================================================================
//                    HANDLER TYPE
// ============================================================================

/// Тип асинхронного обработчика.
pub type BoxFuture<T> = Pin<Box<dyn Future<Output = T> + Send>>;

/// Handler function type.
pub type HandlerFn = Arc<dyn Fn(Request) -> BoxFuture<Response> + Send + Sync>;

/// Трейт для handler'ов.
pub trait Handler: Send + Sync {
    fn call(&self, req: Request) -> BoxFuture<Response>;
}

impl<F, Fut> Handler for F
where
    F: Fn(Request) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Response> + Send + 'static,
{
    fn call(&self, req: Request) -> BoxFuture<Response> {
        Box::pin(self(req))
    }
}

// ============================================================================
//                    ROUTE
// ============================================================================

/// Маршрут.
pub struct Route {
    /// Паттерн пути
    pub pattern: String,
    /// HTTP метод
    pub method: Method,
    /// Обработчик
    pub handler: Arc<dyn Handler>,
    /// Middleware для этого маршрута
    pub middleware: Vec<Arc<dyn Middleware>>,
    /// Метаданные для OpenAPI
    pub meta: RouteMeta,
}

/// Метаданные маршрута для документации.
#[derive(Debug, Clone, Default)]
pub struct RouteMeta {
    /// Название операции
    pub operation_id: Option<String>,
    /// Краткое описание
    pub summary: Option<String>,
    /// Подробное описание
    pub description: Option<String>,
    /// Теги для группировки
    pub tags: Vec<String>,
    /// Deprecated flag
    pub deprecated: bool,
    /// Response примеры
    pub responses: HashMap<u16, String>,
    /// Параметры операции (для OpenAPI)
    pub parameters: Vec<String>,
    /// Тело запроса (content-type)
    pub request_body: Option<String>,
}

impl RouteMeta {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn operation_id(mut self, id: impl Into<String>) -> Self {
        self.operation_id = Some(id.into());
        self
    }

    pub fn summary(mut self, s: impl Into<String>) -> Self {
        self.summary = Some(s.into());
        self
    }

    pub fn description(mut self, d: impl Into<String>) -> Self {
        self.description = Some(d.into());
        self
    }

    pub fn tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    pub fn deprecated(mut self) -> Self {
        self.deprecated = true;
        self
    }
}

// ============================================================================
//                    ROUTE MATCH
// ============================================================================

/// Результат матчинга маршрута.
pub struct RouteMatch<'a> {
    /// Найденный маршрут
    pub route: &'a Route,
    /// Извлечённые path параметры
    pub params: HashMap<String, String>,
}

// ============================================================================
//                    ROUTE TREE (Trie)
// ============================================================================

/// Узел дерева маршрутов.
struct RouteNode {
    /// Сегмент пути (статический, параметр, или wildcard)
    segment: RouteSegment,
    /// Дочерние узлы
    children: Vec<RouteNode>,
    /// Handlers по методам
    handlers: HashMap<Method, Arc<Route>>,
}

#[derive(Debug, Clone, PartialEq)]
enum RouteSegment {
    /// Статический сегмент: "users", "api"
    Static(String),
    /// Параметр: {id}, {user_id}
    Param(String),
    /// Wildcard: *path (захватывает остаток пути)
    Wildcard(String),
}

impl RouteNode {
    fn new(segment: RouteSegment) -> Self {
        Self {
            segment,
            children: Vec::new(),
            handlers: HashMap::new(),
        }
    }

    fn root() -> Self {
        Self::new(RouteSegment::Static(String::new()))
    }
}

// ============================================================================
//                    ROUTER
// ============================================================================

/// HTTP маршрутизатор.
pub struct Router {
    /// Корень дерева маршрутов
    root: RouteNode,
    /// Глобальные middleware
    middleware: Vec<Arc<dyn Middleware>>,
    /// Fallback handler (404)
    fallback: Option<Arc<dyn Handler>>,
    /// Все маршруты (для OpenAPI)
    routes: Vec<Arc<Route>>,
}

impl Router {
    /// Создаёт новый пустой роутер.
    pub fn new() -> Self {
        Self {
            root: RouteNode::root(),
            middleware: Vec::new(),
            fallback: None,
            routes: Vec::new(),
        }
    }

    // -------------------------------------------------------------------------
    // Route Registration
    // -------------------------------------------------------------------------

    /// Добавляет маршрут.
    pub fn route(
        mut self,
        method: Method,
        pattern: impl Into<String>,
        handler: impl Handler + 'static,
    ) -> Self {
        let pattern = pattern.into();
        let route = Arc::new(Route {
            pattern: pattern.clone(),
            method,
            handler: Arc::new(handler),
            middleware: Vec::new(),
            meta: RouteMeta::default(),
        });

        self.insert_route(&pattern, method, Arc::clone(&route));
        self.routes.push(route);
        self
    }

    /// Добавляет маршрут с метаданными.
    pub fn route_with_meta(
        mut self,
        method: Method,
        pattern: impl Into<String>,
        handler: impl Handler + 'static,
        meta: RouteMeta,
    ) -> Self {
        let pattern = pattern.into();
        let route = Arc::new(Route {
            pattern: pattern.clone(),
            method,
            handler: Arc::new(handler),
            middleware: Vec::new(),
            meta,
        });

        self.insert_route(&pattern, method, Arc::clone(&route));
        self.routes.push(route);
        self
    }

    // HTTP method shortcuts
    pub fn get(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::GET, pattern, handler)
    }

    pub fn post(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::POST, pattern, handler)
    }

    pub fn put(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::PUT, pattern, handler)
    }

    pub fn delete(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::DELETE, pattern, handler)
    }

    pub fn patch(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::PATCH, pattern, handler)
    }

    pub fn head(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::HEAD, pattern, handler)
    }

    pub fn options(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::OPTIONS, pattern, handler)
    }

    // -------------------------------------------------------------------------
    // Route Groups
    // -------------------------------------------------------------------------

    /// Создаёт группу маршрутов с префиксом.
    pub fn group(self, prefix: impl Into<String>, configure: impl FnOnce(RouteGroup) -> RouteGroup) -> Self {
        let group = RouteGroup::new(prefix.into());
        let configured = configure(group);
        configured.apply(self)
    }

    // -------------------------------------------------------------------------
    // Middleware
    // -------------------------------------------------------------------------

    /// Добавляет глобальный middleware.
    pub fn middleware(mut self, mw: impl Middleware + 'static) -> Self {
        self.middleware.push(Arc::new(mw));
        self
    }

    /// Добавляет несколько middleware.
    pub fn with_middleware(mut self, middleware: Vec<Arc<dyn Middleware>>) -> Self {
        self.middleware.extend(middleware);
        self
    }

    // -------------------------------------------------------------------------
    // Fallback
    // -------------------------------------------------------------------------

    /// Устанавливает fallback handler для 404.
    pub fn fallback(mut self, handler: impl Handler + 'static) -> Self {
        self.fallback = Some(Arc::new(handler));
        self
    }

    // -------------------------------------------------------------------------
    // Merging
    // -------------------------------------------------------------------------

    /// Объединяет с другим роутером.
    pub fn merge(mut self, other: Router) -> Self {
        for route in other.routes {
            self.insert_route(&route.pattern, route.method, Arc::clone(&route));
            self.routes.push(route);
        }
        self.middleware.extend(other.middleware);
        self
    }

    /// Вложенный роутер с префиксом.
    pub fn nest(self, prefix: impl Into<String>, other: Router) -> Self {
        let prefix = prefix.into();
        let mut new_self = self;
        
        for route in other.routes {
            let new_pattern = format!("{}{}", prefix, route.pattern);
            let new_route = Arc::new(Route {
                pattern: new_pattern.clone(),
                method: route.method,
                handler: Arc::clone(&route.handler),
                middleware: route.middleware.clone(),
                meta: route.meta.clone(),
            });
            
            new_self.insert_route(&new_pattern, route.method, Arc::clone(&new_route));
            new_self.routes.push(new_route);
        }
        
        new_self
    }

    // -------------------------------------------------------------------------
    // Matching
    // -------------------------------------------------------------------------

    /// Ищет маршрут для запроса.
    pub fn match_route(&self, method: Method, path: &str) -> Option<RouteMatch<'_>> {
        let segments: Vec<&str> = path.split('/')
            .filter(|s| !s.is_empty())
            .collect();
        
        let mut params = HashMap::new();
        
        if let Some(route) = self.match_node(&self.root, &segments, 0, method, &mut params) {
            Some(RouteMatch { route, params })
        } else {
            None
        }
    }

    fn match_node<'a>(
        &'a self,
        node: &'a RouteNode,
        segments: &[&str],
        index: usize,
        method: Method,
        params: &mut HashMap<String, String>,
    ) -> Option<&'a Route> {
        // Если достигли конца пути
        if index >= segments.len() {
            return node.handlers.get(&method).map(|r| r.as_ref());
        }

        let segment = segments[index];

        // Ищем среди children
        for child in &node.children {
            match &child.segment {
                RouteSegment::Static(s) if s == segment => {
                    if let Some(route) = self.match_node(child, segments, index + 1, method, params) {
                        return Some(route);
                    }
                }
                RouteSegment::Param(name) => {
                    params.insert(name.clone(), segment.to_string());
                    if let Some(route) = self.match_node(child, segments, index + 1, method, params) {
                        return Some(route);
                    }
                    params.remove(name);
                }
                RouteSegment::Wildcard(name) => {
                    // Wildcard захватывает весь остаток пути
                    let rest = segments[index..].join("/");
                    params.insert(name.clone(), rest);
                    if let Some(route) = child.handlers.get(&method) {
                        return Some(route.as_ref());
                    }
                    params.remove(name);
                }
                _ => {}
            }
        }

        None
    }

    /// Проверяет, есть ли маршруты для пути (для 405).
    pub fn has_path(&self, path: &str) -> bool {
        for method in Method::ALL {
            if self.match_route(method, path).is_some() {
                return true;
            }
        }
        false
    }

    // -------------------------------------------------------------------------
    // Request Handling
    // -------------------------------------------------------------------------

    /// Обрабатывает запрос.
    pub async fn handle(&self, mut req: Request) -> Response {
        let method = req.method();
        let path = req.path().to_string();

        match self.match_route(method, &path) {
            Some(route_match) => {
                // Устанавливаем path параметры
                req.set_path_params(route_match.params);

                // Собираем middleware chain
                let mut chain = MiddlewareChain::new(Arc::clone(&route_match.route.handler));
                
                // Route-specific middleware
                for mw in route_match.route.middleware.iter().rev() {
                    chain = chain.with(Arc::clone(mw));
                }
                
                // Global middleware
                for mw in self.middleware.iter().rev() {
                    chain = chain.with(Arc::clone(mw));
                }

                chain.run(req).await
            }
            None => {
                // Проверяем 405
                if self.has_path(&path) {
                    Response::method_not_allowed()
                } else if let Some(fallback) = &self.fallback {
                    fallback.call(req).await
                } else {
                    Response::not_found()
                }
            }
        }
    }

    // -------------------------------------------------------------------------
    // Internal
    // -------------------------------------------------------------------------

    fn insert_route(&mut self, pattern: &str, method: Method, route: Arc<Route>) {
        let segments = Self::parse_pattern(pattern);
        let mut node = &mut self.root;

        for segment in segments {
            let pos = node.children.iter().position(|c| c.segment == segment);
            
            if let Some(idx) = pos {
                node = &mut node.children[idx];
            } else {
                node.children.push(RouteNode::new(segment.clone()));
                let idx = node.children.len() - 1;
                node = &mut node.children[idx];
            }
        }

        node.handlers.insert(method, route);
    }

    fn parse_pattern(pattern: &str) -> Vec<RouteSegment> {
        pattern
            .split('/')
            .filter(|s| !s.is_empty())
            .map(|s| {
                if s.starts_with('{') && s.ends_with('}') {
                    RouteSegment::Param(s[1..s.len()-1].to_string())
                } else if s.starts_with('*') {
                    RouteSegment::Wildcard(s[1..].to_string())
                } else {
                    RouteSegment::Static(s.to_string())
                }
            })
            .collect()
    }

    /// Возвращает все маршруты (для OpenAPI).
    pub fn routes(&self) -> &[Arc<Route>] {
        &self.routes
    }
}

impl Default for Router {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ROUTE GROUP
// ============================================================================

/// Группа маршрутов с общим префиксом и middleware.
pub struct RouteGroup {
    prefix: String,
    routes: Vec<(Method, String, Arc<dyn Handler>, RouteMeta)>,
    middleware: Vec<Arc<dyn Middleware>>,
}

impl RouteGroup {
    pub fn new(prefix: String) -> Self {
        Self {
            prefix,
            routes: Vec::new(),
            middleware: Vec::new(),
        }
    }

    pub fn route(mut self, method: Method, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.routes.push((method, pattern.into(), Arc::new(handler), RouteMeta::default()));
        self
    }

    pub fn get(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::GET, pattern, handler)
    }

    pub fn post(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::POST, pattern, handler)
    }

    pub fn put(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::PUT, pattern, handler)
    }

    pub fn delete(self, pattern: impl Into<String>, handler: impl Handler + 'static) -> Self {
        self.route(Method::DELETE, pattern, handler)
    }

    pub fn middleware(mut self, mw: impl Middleware + 'static) -> Self {
        self.middleware.push(Arc::new(mw));
        self
    }

    fn apply(self, mut router: Router) -> Router {
        for (method, pattern, handler, meta) in self.routes {
            let full_pattern = format!("{}{}", self.prefix, pattern);
            let route = Arc::new(Route {
                pattern: full_pattern.clone(),
                method,
                handler,
                middleware: self.middleware.clone(),
                meta,
            });
            
            router.insert_route(&full_pattern, method, Arc::clone(&route));
            router.routes.push(route);
        }
        router
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    async fn hello(_req: Request) -> Response {
        Response::text("Hello")
    }

    async fn get_user(req: Request) -> Response {
        let id = req.param("id").unwrap_or("unknown");
        Response::text(format!("User: {}", id))
    }

    #[tokio::test]
    async fn test_basic_routing() {
        let router = Router::new()
            .get("/", hello)
            .get("/users/{id}", get_user);

        // Test exact match
        let match1 = router.match_route(Method::GET, "/");
        assert!(match1.is_some());

        // Test param match
        let match2 = router.match_route(Method::GET, "/users/123");
        assert!(match2.is_some());
        assert_eq!(match2.unwrap().params.get("id").unwrap(), "123");

        // Test 404
        let match3 = router.match_route(Method::GET, "/not-found");
        assert!(match3.is_none());

        // Test 405
        let match4 = router.match_route(Method::POST, "/");
        assert!(match4.is_none());
        assert!(router.has_path("/"));
    }

    #[tokio::test]
    async fn test_nested_params() {
        let router = Router::new()
            .get("/posts/{post_id}/comments/{comment_id}", |req: Request| async move {
                let post_id = req.param("post_id").unwrap();
                let comment_id = req.param("comment_id").unwrap();
                Response::text(format!("Post {} Comment {}", post_id, comment_id))
            });

        let m = router.match_route(Method::GET, "/posts/10/comments/5").unwrap();
        assert_eq!(m.params.get("post_id").unwrap(), "10");
        assert_eq!(m.params.get("comment_id").unwrap(), "5");
    }

    #[tokio::test]
    async fn test_route_groups() {
        let router = Router::new()
            .group("/api/v1", |g| {
                g.get("/users", hello)
                 .get("/posts", hello)
            });

        assert!(router.match_route(Method::GET, "/api/v1/users").is_some());
        assert!(router.match_route(Method::GET, "/api/v1/posts").is_some());
        assert!(router.match_route(Method::GET, "/api/v1/other").is_none());
    }
}
