//! HTTP client for the "net" library.
//!
//! This module contains only what a Kumir program sees. Connection logic,
//! encryption, and response parsing live in [`super::transport`].

use std::sync::Arc;

use super::transport::{self, Response};
use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// Extracts a required string argument.
fn text_arg(args: &[Value], index: usize, name: &str) -> Result<String, String> {
    args.get(index)
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .ok_or_else(|| format!("Ожидается строка в параметре «{name}»"))
}

/// Optional string argument; if missing, returns empty string.
fn optional_text(args: &[Value], index: usize) -> String {
    args.get(index)
        .and_then(|v| v.as_string())
        .map(|s| s.to_string())
        .unwrap_or_default()
}

/// Parses headers from lines formatted as `Name: value`.
fn parse_headers(text: &str) -> Vec<(String, String)> {
    text.lines()
        .filter_map(|line| line.split_once(':'))
        .map(|(key, value)| (key.trim().to_string(), value.trim().to_string()))
        .collect()
}

/// Converts a response into a dict accessible to the program.
///
/// A dict, not a string: the response has not only a body, but also a status code
/// and headers. Previously, these were inaccessible—functions returned only the body,
/// so there was no way to distinguish valid data from a "not found" page.
fn response_to_value(response: Response) -> Value {
    use std::collections::BTreeMap;

    let mut headers = BTreeMap::new();
    for (name, value) in &response.headers {
        headers.insert(
            Value::String(name.to_lowercase()),
            Value::String(value.clone()),
        );
    }

    let mut map = BTreeMap::new();
    let mut put = |ru: &str, en: &str, value: Value| {
        map.insert(Value::String(ru.to_string()), value.clone());
        map.insert(Value::String(en.to_string()), value);
    };
    put(
        "код",
        "status",
        Value::Number(crate::types::Number::U16(response.status)),
    );
    put(
        "успех",
        "ok",
        Value::Boolean((200..300).contains(&response.status)),
    );
    put("причина", "reason", Value::String(response.reason.clone()));
    put("тело", "body", Value::String(response.body.clone()));
    put("заголовки", "headers", Value::Map(headers));

    Value::Map(map)
}

/// Response body with status code check.
///
/// A function returning a single string must report server errors: silently
/// returning a "404 not found" page instead of data would be the worst outcome,
/// since the program would continue working with garbage unaware.
fn body_or_error(response: Response, url: &str) -> Result<Value, String> {
    if (200..300).contains(&response.status) {
        return Ok(Value::String(response.body));
    }
    Err(format!(
        "Сервер ответил {} {} на запрос {url}",
        response.status, response.reason
    ))
}

// ---------------------------------------------------------------------------

/// `http_запрос(url)` → тело ответа.
pub fn http_get_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_запрос")
        .with_aliases(vec![Arc::from("http_get"), Arc::from("wget")])
        .with_description("Выполняет GET-запрос и возвращает тело ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = text_arg(args, 0, "url")?;
            let response = transport::request("GET", &url, None, &[])?;
            body_or_error(response, &url)
        })
}

/// `http_отправить(url, данные)` → тело ответа.
pub fn http_post_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_отправить")
        .with_aliases(vec![Arc::from("http_post")])
        .with_description("Выполняет POST-запрос с данными и возвращает тело ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .with_param(LibParamDef::value("данные", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = text_arg(args, 0, "url")?;
            let data = text_arg(args, 1, "данные")?;
            let response = transport::request("POST", &url, Some(&data), &[])?;
            body_or_error(response, &url)
        })
}

/// `http_заголовки(url)` → заголовки ответа одной строкой.
pub fn http_head_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_заголовки")
        .with_aliases(vec![Arc::from("http_head")])
        .with_description("Выполняет HEAD-запрос и возвращает заголовки ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = text_arg(args, 0, "url")?;
            let response = transport::request("HEAD", &url, None, &[])?;
            let text = response
                .headers
                .iter()
                .map(|(k, v)| format!("{k}: {v}"))
                .collect::<Vec<_>>()
                .join("\n");
            Ok(Value::String(text))
        })
}

/// `http_получить(url)` → словарь с кодом, заголовками и телом.
pub fn http_fetch_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_получить")
        .with_aliases(vec![Arc::from("http_fetch"), Arc::from("fetch")])
        .with_description("GET-запрос: возвращает словарь с кодом, заголовками и телом")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::Any),
        ))
        .with_handler(|args| {
            let url = text_arg(args, 0, "url")?;
            let response = transport::request("GET", &url, None, &[])?;
            Ok(response_to_value(response))
        })
}

/// `http_скачать(url, файл)` → сколько байт записано.
pub fn http_download_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_скачать")
        .with_aliases(vec![Arc::from("http_download")])
        .with_description("Скачивает адрес в файл и возвращает число записанных байт")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .with_param(LibParamDef::value("файл", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let url = text_arg(args, 0, "url")?;
            let path = text_arg(args, 1, "файл")?;
            let response = transport::request("GET", &url, None, &[])?;
            if !(200..300).contains(&response.status) {
                return Err(format!(
                    "Сервер ответил {} {} на запрос {url}",
                    response.status, response.reason
                ));
            }
            let bytes = response.body.as_bytes();
            std::fs::write(&path, bytes)
                .map_err(|e| format!("Не удалось записать файл {path}: {e}"))?;
            Ok(Value::Number(crate::types::Number::I64(bytes.len() as i64)))
        })
}

/// `http_расширенный_запрос(метод, url, тело, заголовки)` → словарь ответа.
pub fn http_request_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_расширенный_запрос")
        .with_aliases(vec![Arc::from("http_request"), Arc::from("http_custom")])
        .with_description(
            "Произвольный HTTP-запрос: возвращает словарь с кодом, заголовками и телом",
        )
        .with_param(LibParamDef::value("метод", TypeKind::String))
        .with_param(LibParamDef::value("url", TypeKind::String))
        .with_param(LibParamDef::value("тело", TypeKind::String))
        .with_param(LibParamDef::value("заголовки", TypeKind::String))
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::Any),
        ))
        .with_handler(|args| {
            let method = text_arg(args, 0, "метод")?;
            let url = text_arg(args, 1, "url")?;
            let body = optional_text(args, 2);
            let headers = parse_headers(&optional_text(args, 3));

            let body = if body.is_empty() {
                None
            } else {
                Some(body.as_str())
            };
            let response = transport::request(&method, &url, body, &headers)?;
            Ok(response_to_value(response))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn headers_parsed_line_by_line() {
        let headers = parse_headers("Accept: text/html\nX-Ключ:  значение \nмусор без двоеточия");
        assert_eq!(
            headers,
            vec![
                ("Accept".to_string(), "text/html".to_string()),
                ("X-Ключ".to_string(), "значение".to_string()),
            ]
        );
    }

    #[test]
    fn response_dict_bilingual() {
        let response = Response {
            status: 200,
            reason: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/plain".to_string())],
            body: "привет".to_string(),
        };
        let Value::Map(map) = response_to_value(response) else {
            panic!("ожидался словарь");
        };

        for (ru, en) in [
            ("код", "status"),
            ("успех", "ok"),
            ("тело", "body"),
            ("заголовки", "headers"),
        ] {
            let ru_value = map.get(&Value::String(ru.to_string()));
            assert!(ru_value.is_some(), "нет ключа «{ru}»");
            assert_eq!(
                ru_value,
                map.get(&Value::String(en.to_string())),
                "«{ru}» и «{en}» должны совпадать"
            );
        }

        assert_eq!(
            map.get(&Value::String("успех".to_string())),
            Some(&Value::Boolean(true))
        );
    }

    /// Headers are accessible by lowercase name—otherwise the program would have to guess
    /// how the server capitalized it.
    #[test]
    fn header_names_lowercased() {
        let response = Response {
            status: 200,
            reason: "OK".to_string(),
            headers: vec![("Content-Type".to_string(), "text/html".to_string())],
            body: String::new(),
        };
        let Value::Map(map) = response_to_value(response) else {
            panic!("ожидался словарь");
        };
        let Some(Value::Map(headers)) = map.get(&Value::String("заголовки".to_string()))
        else {
            panic!("нет заголовков");
        };
        assert_eq!(
            headers.get(&Value::String("content-type".to_string())),
            Some(&Value::String("text/html".to_string()))
        );
    }

    /// Server errors must be reported as errors, not as the body of a "not found" page.
    #[test]
    fn server_error_not_silent() {
        let response = Response {
            status: 404,
            reason: "Not Found".to_string(),
            headers: Vec::new(),
            body: "<html>нет такой страницы</html>".to_string(),
        };
        let result = body_or_error(response, "http://example.org/нет");
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("404"));
    }
}
