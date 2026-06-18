//! Минимальный HTTP-клиент на основе std::net (без внешних библиотек)

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// Разбирает URL на (host, port, path)
fn parse_url(url: &str) -> Result<(String, u16, String, bool), String> {
    let url_trimmed = url.trim();
    let (scheme, rest) = if url_trimmed.starts_with("https://") {
        (true, &url_trimmed[8..])
    } else if url_trimmed.starts_with("http://") {
        (false, &url_trimmed[7..])
    } else {
        (false, url_trimmed)
    };

    let (host_port, path) = match rest.find('/') {
        Some(idx) => (&rest[..idx], &rest[idx..]),
        None => (rest, "/"),
    };

    let (host, port) = match host_port.rfind(':') {
        Some(idx) => {
            let p = host_port[idx + 1..]
                .parse::<u16>()
                .map_err(|_| format!("Неверный порт в URL: {}", url))?;
            (host_port[..idx].to_string(), p)
        }
        None => (host_port.to_string(), if scheme { 443 } else { 80 }),
    };

    Ok((host, port, path.to_string(), scheme))
}

/// Выполняет HTTP-запрос и возвращает тело ответа
fn do_http_request(
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &[(String, String)],
) -> Result<String, String> {
    let (host, port, path, is_https) = parse_url(url)?;

    if is_https {
        return Err(
            "HTTPS не поддерживается без внешних зависимостей. Используйте HTTP.".to_string(),
        );
    }

    let addr = format!("{}:{}", host, port);
    let mut stream =
        TcpStream::connect(&addr).map_err(|e| format!("Ошибка подключения к {}: {}", addr, e))?;
    stream
        .set_read_timeout(Some(Duration::from_secs(30)))
        .map_err(|e| format!("Ошибка таймаута: {}", e))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| format!("Ошибка таймаута: {}", e))?;

    // Формируем запрос
    let mut request = format!(
        "{} {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n",
        method, path, host
    );

    for (key, value) in headers {
        request.push_str(&format!("{}: {}\r\n", key, value));
    }

    if let Some(b) = body {
        request.push_str(&format!("Content-Length: {}\r\n", b.len()));
        if !headers
            .iter()
            .any(|(k, _)| k.eq_ignore_ascii_case("Content-Type"))
        {
            request.push_str("Content-Type: application/x-www-form-urlencoded\r\n");
        }
        request.push_str("\r\n");
        request.push_str(b);
    } else {
        request.push_str("\r\n");
    }

    stream
        .write_all(request.as_bytes())
        .map_err(|e| format!("Ошибка отправки запроса: {}", e))?;
    stream.flush().map_err(|e| format!("Ошибка flush: {}", e))?;

    // Читаем ответ
    let mut response = Vec::new();
    loop {
        let mut buf = [0u8; 8192];
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => response.extend_from_slice(&buf[..n]),
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => break,
            Err(ref e) if e.kind() == std::io::ErrorKind::TimedOut => break,
            Err(e) => return Err(format!("Ошибка чтения: {}", e)),
        }
    }

    let response_str = String::from_utf8_lossy(&response).to_string();

    // Отделяем заголовки от тела
    if let Some(idx) = response_str.find("\r\n\r\n") {
        Ok(response_str[idx + 4..].to_string())
    } else {
        Ok(response_str)
    }
}

/// http_запрос(url) → лит
pub fn http_get_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_запрос")
        .with_aliases(vec![Arc::from("http_get"), Arc::from("wget")])
        .with_description("Выполняет GET-запрос и возвращает тело ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'url'".to_string())?;
            let body = do_http_request("GET", url.as_ref(), None, &[])?;
            Ok(Value::String(body))
        })
}

/// http_отправить(url, данные) → лит
pub fn http_post_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_отправить")
        .with_aliases(vec![Arc::from("http_post")])
        .with_description("Выполняет POST-запрос с данными и возвращает тело ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .with_param(LibParamDef::value("данные", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'url'".to_string())?;
            let data = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'данные'".to_string())?;
            let body = do_http_request("POST", url.as_ref(), Some(data.as_ref()), &[])?;
            Ok(Value::String(body))
        })
}

/// http_заголовки(url) → лит
pub fn http_head_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_заголовки")
        .with_aliases(vec![Arc::from("http_head")])
        .with_description("Выполняет HEAD-запрос и возвращает заголовки ответа")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let url = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'url'".to_string())?;

            let (host, port, path, is_https) = parse_url(url.as_ref())?;
            if is_https {
                return Err("HTTPS не поддерживается".to_string());
            }

            let addr = format!("{}:{}", host, port);
            let mut stream = TcpStream::connect(&addr).map_err(|e| format!("Ошибка: {}", e))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|_| "Ошибка таймаута".to_string())?;

            let request = format!(
                "HEAD {} HTTP/1.1\r\nHost: {}\r\nConnection: close\r\n\r\n",
                path, host
            );
            stream
                .write_all(request.as_bytes())
                .map_err(|e| format!("Ошибка: {}", e))?;
            stream.flush().map_err(|e| format!("Ошибка: {}", e))?;

            let mut response = Vec::new();
            let _ = stream.read_to_end(&mut response);
            let response_str = String::from_utf8_lossy(&response).into_owned();

            // Возвращаем только заголовки
            if let Some(idx) = response_str.find("\r\n\r\n") {
                Ok(Value::String(response_str[..idx].to_string()))
            } else {
                Ok(Value::String(response_str))
            }
        })
}

/// http_расширенный_запрос(метод, url, тело, заголовки) → лит
pub fn http_request_fn() -> LibFunctionDef {
    LibFunctionDef::new("http_расширенный_запрос")
        .with_aliases(vec![Arc::from("http_request"), Arc::from("http_custom")])
        .with_description("Выполняет произвольный HTTP-запрос (метод, URL, тело, заголовки)")
        .with_param(LibParamDef::value("метод", TypeKind::String))
        .with_param(LibParamDef::value("url", TypeKind::String))
        .with_param(LibParamDef::value("тело", TypeKind::String))
        .with_param(LibParamDef::value("заголовки", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let method = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается 'метод'".to_string())?;
            let url = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается 'url'".to_string())?;
            let body_str = args.get(2).and_then(|v| v.as_string()).unwrap_or_default();
            let headers_str = args.get(3).and_then(|v| v.as_string()).unwrap_or_default();

            // Парсим заголовки из строки формата "Key: Value\nKey2: Value2"
            let mut headers = Vec::new();
            for line in headers_str.lines() {
                if let Some(idx) = line.find(':') {
                    let key = line[..idx].trim().to_string();
                    let value = line[idx + 1..].trim().to_string();
                    headers.push((key, value));
                }
            }

            let body = if body_str.is_empty() {
                None
            } else {
                Some(body_str.as_str())
            };
            let result = do_http_request(method.as_str(), url.as_str(), body, &headers)?;
            Ok(Value::String(result))
        })
}
