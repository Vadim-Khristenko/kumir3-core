// ============================================================================
//                    OPENAPI GENERATION
// ============================================================================
//
// Автоматическая генерация OpenAPI спецификации:
// - Из Router metadata
// - Schema generation для типов
// - Swagger UI integration
//
// ============================================================================

use std::collections::HashMap;

use serde::{Serialize, Deserialize};

use super::router::{Router, RouteMeta};
use super::types::Method;

// ============================================================================
//                    OPENAPI SPEC
// ============================================================================

/// OpenAPI спецификация.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OpenApiSpec {
    pub openapi: String,
    pub info: Info,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub servers: Vec<Server>,
    pub paths: HashMap<String, PathItem>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub components: Option<Components>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<Tag>,
}

impl OpenApiSpec {
    /// Создаёт новую спецификацию.
    pub fn new(title: impl Into<String>, version: impl Into<String>) -> Self {
        Self {
            openapi: "3.0.3".to_string(),
            info: Info {
                title: title.into(),
                description: None,
                version: version.into(),
                terms_of_service: None,
                contact: None,
                license: None,
            },
            servers: Vec::new(),
            paths: HashMap::new(),
            components: None,
            tags: Vec::new(),
        }
    }

    /// Добавляет описание.
    pub fn description(mut self, desc: impl Into<String>) -> Self {
        self.info.description = Some(desc.into());
        self
    }

    /// Добавляет сервер.
    pub fn server(mut self, url: impl Into<String>, description: Option<String>) -> Self {
        self.servers.push(Server {
            url: url.into(),
            description,
            variables: None,
        });
        self
    }

    /// Добавляет тег.
    pub fn tag(mut self, name: impl Into<String>, description: Option<String>) -> Self {
        self.tags.push(Tag {
            name: name.into(),
            description,
            external_docs: None,
        });
        self
    }

    /// Генерирует из Router.
    pub fn from_router(router: &Router, title: impl Into<String>, version: impl Into<String>) -> Self {
        let mut spec = Self::new(title, version);
        
        for route in router.routes() {
            let path_item = spec.paths.entry(route.pattern.clone()).or_insert_with(PathItem::default);
            
            let operation = Operation::from_meta(&route.meta);
            
            match route.method {
                Method::GET => path_item.get = Some(operation),
                Method::POST => path_item.post = Some(operation),
                Method::PUT => path_item.put = Some(operation),
                Method::DELETE => path_item.delete = Some(operation),
                Method::PATCH => path_item.patch = Some(operation),
                Method::HEAD => path_item.head = Some(operation),
                Method::OPTIONS => path_item.options = Some(operation),
                _ => {}
            }
        }
        
        spec
    }

    /// Сериализует в JSON.
    pub fn to_json(&self) -> Result<String, serde_json::Error> {
        serde_json::to_string_pretty(self)
    }

    /// Сериализует в YAML (если serde_yaml доступен).
    pub fn to_yaml(&self) -> Result<String, String> {
        // Простая YAML-like сериализация
        Ok(format!("# OpenAPI Spec\nopenapi: {}\ninfo:\n  title: {}\n  version: {}\n",
            self.openapi, self.info.title, self.info.version))
    }
}

// ============================================================================
//                    INFO
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Info {
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub version: String,
    #[serde(skip_serializing_if = "Option::is_none", rename = "termsOfService")]
    pub terms_of_service: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contact: Option<Contact>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license: Option<License>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Contact {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub email: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct License {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub url: Option<String>,
}

// ============================================================================
//                    SERVER
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Server {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, ServerVariable>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerVariable {
    pub default: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default, rename = "enum")]
    pub enum_values: Vec<String>,
}

// ============================================================================
//                    PATH ITEM
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PathItem {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub get: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub post: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub put: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delete: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub patch: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub head: Option<Operation>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub options: Option<Operation>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
}

// ============================================================================
//                    OPERATION
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Operation {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "operationId")]
    pub operation_id: Option<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub parameters: Vec<Parameter>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "requestBody")]
    pub request_body: Option<RequestBody>,
    pub responses: HashMap<String, ResponseDef>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub security: Vec<HashMap<String, Vec<String>>>,
}

impl Operation {
    pub fn new() -> Self {
        let mut responses = HashMap::new();
        responses.insert("200".to_string(), ResponseDef {
            description: "Successful response".to_string(),
            content: None,
            headers: None,
        });
        
        Self {
            summary: None,
            description: None,
            operation_id: None,
            tags: Vec::new(),
            parameters: Vec::new(),
            request_body: None,
            responses,
            deprecated: None,
            security: Vec::new(),
        }
    }

    pub fn from_meta(meta: &RouteMeta) -> Self {
        let mut op = Self::new();
        op.summary = meta.summary.clone();
        op.description = meta.description.clone();
        op.operation_id = meta.operation_id.clone();
        op.tags = meta.tags.clone();
        op.deprecated = if meta.deprecated { Some(true) } else { None };
        
        // Конвертируем простые параметры (строки как path параметры)
        for param_name in &meta.parameters {
            op.parameters.push(Parameter {
                name: param_name.clone(),
                r#in: "path".to_string(),
                description: None,
                required: Some(true),
                schema: Some(Schema {
                    r#type: Some("string".to_string()),
                    ..Default::default()
                }),
                ..Default::default()
            });
        }
        
        // Request body (если указан content-type)
        if let Some(content_type) = &meta.request_body {
            let mut content = HashMap::new();
            content.insert(content_type.clone(), MediaType {
                schema: Some(Schema {
                    r#type: Some("object".to_string()),
                    ..Default::default()
                }),
                example: None,
                examples: None,
            });
            
            op.request_body = Some(RequestBody {
                description: None,
                content,
                required: Some(true),
            });
        }
        
        // Responses (u16 -> String description)
        for (status_code, description) in &meta.responses {
            op.responses.insert(status_code.to_string(), ResponseDef {
                description: description.clone(),
                content: None,
                headers: None,
            });
        }
        
        // Default 200 response if none specified
        if op.responses.is_empty() {
            op.responses.insert("200".to_string(), ResponseDef {
                description: "Success".to_string(),
                content: None,
                headers: None,
            });
        }
        
        op
    }
}

impl Default for Operation {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    PARAMETER
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Parameter {
    pub name: String,
    #[serde(rename = "in")]
    pub r#in: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
}

// ============================================================================
//                    REQUEST BODY
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RequestBody {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    pub content: HashMap<String, MediaType>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub required: Option<bool>,
}

// ============================================================================
//                    RESPONSE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResponseDef {
    pub description: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content: Option<HashMap<String, MediaType>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub headers: Option<HashMap<String, Header>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Header {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
}

// ============================================================================
//                    MEDIA TYPE
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MediaType {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub schema: Option<Schema>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub examples: Option<HashMap<String, Example>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Example {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub summary: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<serde_json::Value>,
}

// ============================================================================
//                    SCHEMA
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Schema {
    #[serde(skip_serializing_if = "Option::is_none", rename = "$ref")]
    pub r#ref: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub r#type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub nullable: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub properties: Option<HashMap<String, Schema>>,
    #[serde(skip_serializing_if = "Vec::is_empty", default)]
    pub required: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub items: Option<Box<Schema>>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "enum")]
    pub enum_values: Option<Vec<serde_json::Value>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub example: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub minimum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub maximum: Option<f64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "minLength")]
    pub min_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "maxLength")]
    pub max_length: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub pattern: Option<String>,
}

impl Schema {
    pub fn string() -> Self {
        Self { r#type: Some("string".to_string()), ..Default::default() }
    }

    pub fn integer() -> Self {
        Self { r#type: Some("integer".to_string()), ..Default::default() }
    }

    pub fn number() -> Self {
        Self { r#type: Some("number".to_string()), ..Default::default() }
    }

    pub fn boolean() -> Self {
        Self { r#type: Some("boolean".to_string()), ..Default::default() }
    }

    pub fn array(items: Schema) -> Self {
        Self { 
            r#type: Some("array".to_string()), 
            items: Some(Box::new(items)),
            ..Default::default() 
        }
    }

    pub fn object() -> Self {
        Self { r#type: Some("object".to_string()), ..Default::default() }
    }

    pub fn reference(name: &str) -> Self {
        Self { 
            r#ref: Some(format!("#/components/schemas/{}", name)), 
            ..Default::default() 
        }
    }
}

// ============================================================================
//                    COMPONENTS
// ============================================================================

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Components {
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub schemas: HashMap<String, Schema>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default, rename = "securitySchemes")]
    pub security_schemes: HashMap<String, SecurityScheme>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub responses: HashMap<String, ResponseDef>,
    #[serde(skip_serializing_if = "HashMap::is_empty", default)]
    pub parameters: HashMap<String, Parameter>,
}

// ============================================================================
//                    SECURITY
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SecurityScheme {
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "in")]
    pub r#in: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub scheme: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "bearerFormat")]
    pub bearer_format: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub flows: Option<OAuthFlows>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "openIdConnectUrl")]
    pub open_id_connect_url: Option<String>,
}

impl SecurityScheme {
    pub fn bearer() -> Self {
        Self {
            r#type: "http".to_string(),
            description: None,
            name: None,
            r#in: None,
            scheme: Some("bearer".to_string()),
            bearer_format: Some("JWT".to_string()),
            flows: None,
            open_id_connect_url: None,
        }
    }

    pub fn api_key(name: &str, location: &str) -> Self {
        Self {
            r#type: "apiKey".to_string(),
            description: None,
            name: Some(name.to_string()),
            r#in: Some(location.to_string()),
            scheme: None,
            bearer_format: None,
            flows: None,
            open_id_connect_url: None,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlows {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub implicit: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "clientCredentials")]
    pub client_credentials: Option<OAuthFlow>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "authorizationCode")]
    pub authorization_code: Option<OAuthFlow>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OAuthFlow {
    #[serde(skip_serializing_if = "Option::is_none", rename = "authorizationUrl")]
    pub authorization_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "tokenUrl")]
    pub token_url: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "refreshUrl")]
    pub refresh_url: Option<String>,
    pub scopes: HashMap<String, String>,
}

// ============================================================================
//                    TAG
// ============================================================================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none", rename = "externalDocs")]
    pub external_docs: Option<ExternalDocs>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ExternalDocs {
    pub url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

// ============================================================================
//                    SWAGGER UI
// ============================================================================

/// Генерирует HTML для Swagger UI.
pub fn swagger_ui_html(spec_url: &str, title: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link rel="stylesheet" type="text/css" href="https://unpkg.com/swagger-ui-dist@5/swagger-ui.css">
</head>
<body>
    <div id="swagger-ui"></div>
    <script src="https://unpkg.com/swagger-ui-dist@5/swagger-ui-bundle.js"></script>
    <script>
        window.onload = function() {{
            SwaggerUIBundle({{
                url: "{spec_url}",
                dom_id: '#swagger-ui',
                presets: [
                    SwaggerUIBundle.presets.apis,
                    SwaggerUIBundle.SwaggerUIStandalonePreset
                ],
                layout: "BaseLayout"
            }});
        }};
    </script>
</body>
</html>"#, title = title, spec_url = spec_url)
}

/// Генерирует HTML для ReDoc.
pub fn redoc_html(spec_url: &str, title: &str) -> String {
    format!(r#"<!DOCTYPE html>
<html lang="en">
<head>
    <meta charset="UTF-8">
    <meta name="viewport" content="width=device-width, initial-scale=1.0">
    <title>{title}</title>
    <link href="https://fonts.googleapis.com/css?family=Montserrat:300,400,700|Roboto:300,400,700" rel="stylesheet">
    <style>body {{ margin: 0; padding: 0; }}</style>
</head>
<body>
    <redoc spec-url="{spec_url}"></redoc>
    <script src="https://cdn.redoc.ly/redoc/latest/bundles/redoc.standalone.js"></script>
</body>
</html>"#, title = title, spec_url = spec_url)
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openapi_spec_creation() {
        let spec = OpenApiSpec::new("Test API", "1.0.0")
            .description("A test API")
            .server("http://localhost:8080", Some("Development".to_string()));

        assert_eq!(spec.openapi, "3.0.3");
        assert_eq!(spec.info.title, "Test API");
        assert_eq!(spec.servers.len(), 1);
    }

    #[test]
    fn test_schema_builders() {
        let string_schema = Schema::string();
        assert_eq!(string_schema.r#type, Some("string".to_string()));

        let array_schema = Schema::array(Schema::integer());
        assert_eq!(array_schema.r#type, Some("array".to_string()));
        assert!(array_schema.items.is_some());
    }

    #[test]
    fn test_security_scheme() {
        let bearer = SecurityScheme::bearer();
        assert_eq!(bearer.r#type, "http");
        assert_eq!(bearer.scheme, Some("bearer".to_string()));

        let api_key = SecurityScheme::api_key("X-API-Key", "header");
        assert_eq!(api_key.r#type, "apiKey");
        assert_eq!(api_key.name, Some("X-API-Key".to_string()));
    }

    #[test]
    fn test_json_serialization() {
        let spec = OpenApiSpec::new("Test", "1.0.0");
        let json = spec.to_json().unwrap();
        assert!(json.contains("\"openapi\":"));
        assert!(json.contains("\"info\":"));
    }
}
