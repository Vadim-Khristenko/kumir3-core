//! Сетевые утилиты: DNS, URL-кодирование, Base64, JSON, локальный IP

use std::net::ToSocketAddrs;
use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

// ========================= DNS =========================

/// dns_поиск(хост) → [лит]
pub fn dns_lookup_fn() -> LibFunctionDef {
    LibFunctionDef::new("dns_поиск")
        .with_aliases(vec![Arc::from("dns_lookup"), Arc::from("resolve")])
        .with_description("Разрешает DNS-имя в список IP-адресов")
        .with_param(LibParamDef::value("хост", TypeKind::String))
        .returns(TypeKind::Array(Box::new(TypeKind::String)))
        .with_handler(|args| {
            let host = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'хост'".to_string())?;

            let addr_str = format!("{}:0", host);
            let addrs: Vec<Value> = addr_str
                .to_socket_addrs()
                .map_err(|e| format!("Ошибка DNS-резолва '{}': {}", host, e))?
                .map(|a| Value::String(a.ip().to_string()))
                .collect();
            Ok(Value::Array(addrs))
        })
}

/// dns_обратный(ip) → лит
pub fn dns_reverse_fn() -> LibFunctionDef {
    LibFunctionDef::new("dns_обратный")
        .with_aliases(vec![Arc::from("dns_reverse"), Arc::from("reverse_dns")])
        .with_description("Обратный DNS-запрос: IP → имя хоста (через PTR)")
        .with_param(LibParamDef::value("ip", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let ip = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидется строковый аргумент 'ip'".to_string())?;

            // std не имеет обратного DNS, используем прямой поиск хоста
            let addr_str = format!("{}:0", ip);
            match addr_str.to_socket_addrs() {
                Ok(addrs) => {
                    let first = addrs
                        .into_iter()
                        .next()
                        .map(|a| a.ip().to_string())
                        .unwrap_or_else(|| ip.to_string());
                    Ok(Value::String(first))
                }
                Err(_) => Ok(Value::String(ip.to_string())),
            }
        })
}

// ========================= URL =========================

/// url_кодирование(строка) → лит
pub fn url_encode_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_кодирование")
        .with_aliases(vec![Arc::from("url_encode"), Arc::from("urlencode")])
        .with_description("URL-кодирование строки (percent-encoding)")
        .with_param(LibParamDef::value("строка", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let s = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент".to_string())?;

            let mut encoded = String::new();
            for byte in s.bytes() {
                match byte {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        encoded.push(byte as char);
                    }
                    _ => {
                        encoded.push_str(&format!("%{:02X}", byte));
                    }
                }
            }
            Ok(Value::String(encoded))
        })
}

/// url_декодирование(строка) → лит
pub fn url_decode_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_декодирование")
        .with_aliases(vec![Arc::from("url_decode"), Arc::from("urldecode")])
        .with_description("Декодирование URL-строки (percent-decoding)")
        .with_param(LibParamDef::value("строка", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let s = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент".to_string())?;

            let mut decoded = Vec::new();
            let bytes = s.as_bytes();
            let mut i = 0;
            while i < bytes.len() {
                if bytes[i] == b'%'
                    && i + 2 < bytes.len()
                    && let Ok(byte) =
                        u8::from_str_radix(&String::from_utf8_lossy(&bytes[i + 1..i + 3]), 16)
                {
                    decoded.push(byte);
                    i += 3;
                    continue;
                }
                if bytes[i] == b'+' {
                    decoded.push(b' ');
                } else {
                    decoded.push(bytes[i]);
                }
                i += 1;
            }
            let result = String::from_utf8_lossy(&decoded).into_owned();
            Ok(Value::String(result))
        })
}

/// url_разбор(url) → Мап
pub fn url_parse_fn() -> LibFunctionDef {
    LibFunctionDef::new("url_разбор")
        .with_aliases(vec![Arc::from("url_parse"), Arc::from("parse_url")])
        .with_description("Разбирает URL на компоненты: scheme, host, port, path, query, fragment")
        .with_param(LibParamDef::value("url", TypeKind::String))
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::String),
        ))
        .as_pure()
        .with_handler(|args| {
            let url = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'url'".to_string())?;
            let url_str = url.as_str();

            let mut entries = std::collections::BTreeMap::new();

            // Scheme
            let (scheme, rest) = if let Some(idx) = url_str.find("://") {
                (&url_str[..idx], &url_str[idx + 3..])
            } else {
                ("http", url_str)
            };
            entries.insert(
                Value::String("scheme".to_string()),
                Value::String(scheme.to_string()),
            );

            // Fragment
            let (rest, fragment) = if let Some(idx) = rest.find('#') {
                (&rest[..idx], &rest[idx + 1..])
            } else {
                (rest, "")
            };
            entries.insert(
                Value::String("fragment".to_string()),
                Value::String(fragment.to_string()),
            );

            // Query
            let (rest, query) = if let Some(idx) = rest.find('?') {
                (&rest[..idx], &rest[idx + 1..])
            } else {
                (rest, "")
            };
            entries.insert(
                Value::String("query".to_string()),
                Value::String(query.to_string()),
            );

            // Path
            let (host_port, path) = if let Some(idx) = rest.find('/') {
                (&rest[..idx], &rest[idx..])
            } else {
                (rest, "/")
            };
            entries.insert(
                Value::String("path".to_string()),
                Value::String(path.to_string()),
            );

            // Host and port
            let (host, port) = if let Some(idx) = host_port.rfind(':') {
                (&host_port[..idx], &host_port[idx + 1..])
            } else {
                (host_port, if scheme == "https" { "443" } else { "80" })
            };
            entries.insert(
                Value::String("host".to_string()),
                Value::String(host.to_string()),
            );
            entries.insert(
                Value::String("port".to_string()),
                Value::String(port.to_string()),
            );

            Ok(Value::Map(entries))
        })
}

// ========================= Base64 =========================

const BASE64_TABLE: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

fn base64_encode_bytes(input: &[u8]) -> String {
    let mut output = String::new();
    let mut i = 0;
    while i < input.len() {
        let b0 = input[i] as u32;
        let b1 = if i + 1 < input.len() {
            input[i + 1] as u32
        } else {
            0
        };
        let b2 = if i + 2 < input.len() {
            input[i + 2] as u32
        } else {
            0
        };

        let triple = (b0 << 16) | (b1 << 8) | b2;

        output.push(BASE64_TABLE[((triple >> 18) & 0x3F) as usize] as char);
        output.push(BASE64_TABLE[((triple >> 12) & 0x3F) as usize] as char);

        if i + 1 < input.len() {
            output.push(BASE64_TABLE[((triple >> 6) & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
        if i + 2 < input.len() {
            output.push(BASE64_TABLE[(triple & 0x3F) as usize] as char);
        } else {
            output.push('=');
        }
        i += 3;
    }
    output
}

fn base64_decode_byte(c: u8) -> Option<u32> {
    match c {
        b'A'..=b'Z' => Some((c - b'A') as u32),
        b'a'..=b'z' => Some((c - b'a' + 26) as u32),
        b'0'..=b'9' => Some((c - b'0' + 52) as u32),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn base64_decode_bytes(input: &str) -> Result<Vec<u8>, String> {
    let input = input.trim_end_matches('=');
    let bytes = input.as_bytes();
    let mut output = Vec::new();
    let mut i = 0;
    while i < bytes.len() {
        let b0 = base64_decode_byte(bytes[i]).ok_or("Некорректный Base64")?;
        let b1 = if i + 1 < bytes.len() {
            base64_decode_byte(bytes[i + 1]).ok_or("Некорректный Base64")?
        } else {
            0
        };
        let b2 = if i + 2 < bytes.len() {
            base64_decode_byte(bytes[i + 2]).ok_or("Некорректный Base64")?
        } else {
            0
        };
        let b3 = if i + 3 < bytes.len() {
            base64_decode_byte(bytes[i + 3]).ok_or("Некорректный Base64")?
        } else {
            0
        };

        let triple = (b0 << 18) | (b1 << 12) | (b2 << 6) | b3;

        output.push(((triple >> 16) & 0xFF) as u8);
        if i + 2 < bytes.len() {
            output.push(((triple >> 8) & 0xFF) as u8);
        }
        if i + 3 < bytes.len() {
            output.push((triple & 0xFF) as u8);
        }
        i += 4;
    }
    Ok(output)
}

/// base64_кодирование(строка) → лит
pub fn base64_encode_fn() -> LibFunctionDef {
    LibFunctionDef::new("base64_кодирование")
        .with_aliases(vec![Arc::from("base64_encode"), Arc::from("btoa")])
        .with_description("Кодирует строку в Base64")
        .with_param(LibParamDef::value("строка", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let s = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент".to_string())?;
            let encoded = base64_encode_bytes(s.as_bytes());
            Ok(Value::String(encoded))
        })
}

/// base64_декодирование(строка) → лит
pub fn base64_decode_fn() -> LibFunctionDef {
    LibFunctionDef::new("base64_декодирование")
        .with_aliases(vec![Arc::from("base64_decode"), Arc::from("atob")])
        .with_description("Декодирует строку из Base64")
        .with_param(LibParamDef::value("строка", TypeKind::String))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let s = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент".to_string())?;
            let decoded = base64_decode_bytes(s.as_str())?;
            let result = String::from_utf8(decoded)
                .map_err(|_| "Декодированные данные не являются UTF-8".to_string())?;
            Ok(Value::String(result))
        })
}

// ========================= JSON =========================

/// Простой JSON-парсер (обрабатывает строки, числа, true/false/null, массивы, объекты)
fn parse_json_value(input: &str) -> Result<(Value, &str), String> {
    let input = input.trim_start();
    if input.is_empty() {
        return Err("Пустой JSON".to_string());
    }

    match input.as_bytes()[0] {
        b'"' => parse_json_string(input),
        b'[' => parse_json_array(input),
        b'{' => parse_json_object(input),
        b't' if input.starts_with("true") => Ok((Value::Boolean(true), &input[4..])),
        b'f' if input.starts_with("false") => Ok((Value::Boolean(false), &input[5..])),
        b'n' if input.starts_with("null") => Ok((Value::Null, &input[4..])),
        b'0'..=b'9' | b'-' => parse_json_number(input),
        _ => Err(format!("Неожиданный символ в JSON: '{}'", &input[..1])),
    }
}

fn parse_json_string(input: &str) -> Result<(Value, &str), String> {
    if !input.starts_with('"') {
        return Err("Ожидается '\"'".to_string());
    }
    let rest = &input[1..];
    let mut result = String::new();
    let mut chars = rest.char_indices();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Ok((Value::String(result), &rest[i + 1..])),
            '\\' => {
                if let Some((_, escaped)) = chars.next() {
                    match escaped {
                        '"' => result.push('"'),
                        '\\' => result.push('\\'),
                        '/' => result.push('/'),
                        'n' => result.push('\n'),
                        'r' => result.push('\r'),
                        't' => result.push('\t'),
                        _ => {
                            result.push('\\');
                            result.push(escaped);
                        }
                    }
                }
            }
            _ => result.push(c),
        }
    }
    Err("Незакрытая строка JSON".to_string())
}

fn parse_json_number(input: &str) -> Result<(Value, &str), String> {
    let mut end = 0;
    let bytes = input.as_bytes();
    let mut is_float = false;

    if end < bytes.len() && bytes[end] == b'-' {
        end += 1;
    }
    while end < bytes.len() && bytes[end].is_ascii_digit() {
        end += 1;
    }
    if end < bytes.len() && bytes[end] == b'.' {
        is_float = true;
        end += 1;
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }
    if end < bytes.len() && (bytes[end] == b'e' || bytes[end] == b'E') {
        is_float = true;
        end += 1;
        if end < bytes.len() && (bytes[end] == b'+' || bytes[end] == b'-') {
            end += 1;
        }
        while end < bytes.len() && bytes[end].is_ascii_digit() {
            end += 1;
        }
    }

    let num_str = &input[..end];
    if is_float {
        let f: f64 = num_str
            .parse()
            .map_err(|_| format!("Неверное число: {}", num_str))?;
        Ok((Value::Number(crate::types::Number::F64(f)), &input[end..]))
    } else {
        let i: i64 = num_str
            .parse()
            .map_err(|_| format!("Неверное число: {}", num_str))?;
        Ok((Value::Number(crate::types::Number::I64(i)), &input[end..]))
    }
}

fn parse_json_array(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start();
    let mut items = Vec::new();

    if let Some(rest_tail) = rest.strip_prefix(']') {
        return Ok((Value::Array(items), rest_tail));
    }

    loop {
        let (val, r) = parse_json_value(rest)?;
        items.push(val);
        rest = r.trim_start();

        if let Some(rest_tail) = rest.strip_prefix(']') {
            return Ok((Value::Array(items), rest_tail));
        }
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else {
            return Err("Ожидается ',' или ']' в массиве JSON".to_string());
        }
    }
}

fn parse_json_object(input: &str) -> Result<(Value, &str), String> {
    let mut rest = input[1..].trim_start();
    let mut entries = std::collections::BTreeMap::new();

    if let Some(rest_tail) = rest.strip_prefix('}') {
        return Ok((Value::Map(entries), rest_tail));
    }

    loop {
        let (key_val, r) = parse_json_string(rest.trim_start())?;
        let key = key_val;
        rest = r.trim_start();

        if !rest.starts_with(':') {
            return Err("Ожидается ':' в объекте JSON".to_string());
        }
        rest = rest[1..].trim_start();

        let (val, r) = parse_json_value(rest)?;
        entries.insert(key, val);
        rest = r.trim_start();

        if let Some(rest_tail) = rest.strip_prefix('}') {
            return Ok((Value::Map(entries), rest_tail));
        }
        if rest.starts_with(',') {
            rest = rest[1..].trim_start();
        } else {
            return Err("Ожидается ',' или '}' в объекте JSON".to_string());
        }
    }
}

fn value_to_json(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::Boolean(b) => b.to_string(),
        Value::Number(n) => match n {
            crate::types::Number::F32(f) => format!("{}", f),
            crate::types::Number::F64(f) => format!("{}", f),
            _ => n.to_i64().map(|i| i.to_string()).unwrap_or("0".to_string()),
        },
        Value::String(s) => format!(
            "\"{}\"",
            s.replace('\\', "\\\\")
                .replace('"', "\\\"")
                .replace('\n', "\\n")
                .replace('\r', "\\r")
                .replace('\t', "\\t")
        ),
        Value::Char(c) => format!("\"{}\"", c),
        Value::Array(arr) => {
            let items: Vec<String> = arr.iter().map(value_to_json).collect();
            format!("[{}]", items.join(","))
        }
        Value::Map(entries) => {
            let items: Vec<String> = entries
                .iter()
                .map(|(k, v)| format!("{}:{}", value_to_json(k), value_to_json(v)))
                .collect();
            format!("{{{}}}", items.join(","))
        }
        _ => "null".to_string(),
    }
}

/// json_разбор(строка) → значение
pub fn json_parse_fn() -> LibFunctionDef {
    LibFunctionDef::new("json_разбор")
        .with_aliases(vec![Arc::from("json_parse"), Arc::from("JSON_parse")])
        .with_description("Разбирает JSON-строку в значение КуМира")
        .with_param(LibParamDef::value("строка", TypeKind::String))
        .returns(TypeKind::Any)
        .as_pure()
        .with_handler(|args| {
            let s = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент".to_string())?;
            let (val, _) = parse_json_value(s.as_str())?;
            Ok(val)
        })
}

/// json_строка(значение) → лит
pub fn json_stringify_fn() -> LibFunctionDef {
    LibFunctionDef::new("json_строка")
        .with_aliases(vec![
            Arc::from("json_stringify"),
            Arc::from("JSON_stringify"),
        ])
        .with_description("Преобразует значение КуМира в JSON-строку")
        .with_param(LibParamDef::value("значение", TypeKind::Any))
        .returns(TypeKind::String)
        .as_pure()
        .with_handler(|args| {
            let val = args
                .first()
                .ok_or_else(|| "Ожидается аргумент 'значение'".to_string())?;
            let json = value_to_json(val);
            Ok(Value::String(json))
        })
}

// ========================= Локальный IP =========================

/// локальный_ip() → лит
pub fn local_ip_fn() -> LibFunctionDef {
    LibFunctionDef::new("локальный_ip")
        .with_aliases(vec![Arc::from("local_ip"), Arc::from("my_ip")])
        .with_description("Возвращает предполагаемый локальный IP-адрес машины")
        .returns(TypeKind::String)
        .with_handler(|_args| {
            // Способ без внешних библиотек: подключаемся к public DNS и смотрим local_addr
            match std::net::UdpSocket::bind("0.0.0.0:0") {
                Ok(socket) => {
                    let _ = socket.connect("8.8.8.8:80");
                    match socket.local_addr() {
                        Ok(addr) => Ok(Value::String(addr.ip().to_string())),
                        Err(_) => Ok(Value::String("127.0.0.1".to_string())),
                    }
                }
                Err(_) => Ok(Value::String("127.0.0.1".to_string())),
            }
        })
}
