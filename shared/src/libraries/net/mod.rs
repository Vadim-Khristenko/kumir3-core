//! Сетевая библиотека для КуМир 3
//!
//! Упрощённая синхронная сетевая библиотека на основе std::net.
//! Предоставляет:
//! - TCP сокеты (клиент/сервер)
//! - UDP сокеты
//! - HTTP-запросы (минимальный, без внешних зависимостей)
//! - Утилиты: URL-кодирование, Base64, DNS, JSON
//!
//! Без внешних зависимостей, только std.

mod http;
mod tcp;
mod udp;
mod utils;

use std::sync::Arc;

use crate::types::Number;
use crate::types::library::{LibConstantDef, LibVersion, LibraryDef};
use crate::types::value::{TypeKind, Value};

pub use http::*;
pub use tcp::*;
pub use udp::*;
pub use utils::*;

/// Создаёт библиотеку net
pub fn create_net_library() -> LibraryDef {
    let mut lib = LibraryDef::new("net", "Сеть");
    lib.aliases = vec![
        Arc::from("net"),
        Arc::from("сеть"),
        Arc::from("network"),
        Arc::from("networking"),
    ];
    lib.description = Some(Arc::from(
        "TCP/UDP сокеты, HTTP-запросы, URL-кодирование, Base64, DNS",
    ));
    lib.author = Arc::from("Vadim Khristenko <just@vai-prog.ru>");
    lib.version = LibVersion::new(2, 0, 0);
    lib.stable = false;

    lib.functions = vec![
        // === TCP ===
        tcp_connect_fn(),
        tcp_send_fn(),
        tcp_receive_fn(),
        tcp_listen_fn(),
        // === UDP ===
        udp_send_fn(),
        udp_receive_fn(),
        udp_broadcast_fn(),
        // === HTTP ===
        http_get_fn(),
        http_post_fn(),
        http_head_fn(),
        http_request_fn(),
        // === DNS ===
        dns_lookup_fn(),
        dns_reverse_fn(),
        // === URL ===
        url_encode_fn(),
        url_decode_fn(),
        url_parse_fn(),
        // === Base64 ===
        base64_encode_fn(),
        base64_decode_fn(),
        // === JSON ===
        json_parse_fn(),
        json_stringify_fn(),
        // === Утилиты ===
        local_ip_fn(),
    ];

    lib.constants = vec![
        LibConstantDef {
            name: Arc::from("HTTP_ПОРТ"),
            aliases: vec![Arc::from("HTTP_PORT")],
            const_type: TypeKind::Int64,
            value: Value::Number(Number::I64(80)),
            description: Some(Arc::from("Стандартный порт HTTP")),
        },
        LibConstantDef {
            name: Arc::from("HTTPS_ПОРТ"),
            aliases: vec![Arc::from("HTTPS_PORT")],
            const_type: TypeKind::Int64,
            value: Value::Number(Number::I64(443)),
            description: Some(Arc::from("Стандартный порт HTTPS")),
        },
        LibConstantDef {
            name: Arc::from("LOCALHOST"),
            aliases: vec![Arc::from("ЛОКАЛЬНЫЙ_ХОСТ")],
            const_type: TypeKind::String,
            value: Value::String("127.0.0.1".to_string()),
            description: Some(Arc::from("Адрес локального хоста")),
        },
        LibConstantDef {
            name: Arc::from("МАКС_ПОРТ"),
            aliases: vec![Arc::from("MAX_PORT")],
            const_type: TypeKind::Int64,
            value: Value::Number(Number::I64(65535)),
            description: Some(Arc::from("Максимально допустимый номер порта")),
        },
    ];

    lib
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_library_creation() {
        let lib = create_net_library();
        assert_eq!(lib.id.as_ref(), "net");
        assert!(!lib.functions.is_empty());
        assert!(!lib.constants.is_empty());
    }

    #[test]
    fn test_url_encode_decode() {
        let enc = url_encode_fn();
        let result = enc
            .call(&[Value::String("Привет мир".to_string())])
            .unwrap();
        match &result {
            Value::String(s) => {
                // Проверим, что оригинал восстанавливается
                let dec = url_decode_fn();
                let decoded = dec.call(&[Value::String(s.clone())]).unwrap();
                assert_eq!(decoded, Value::String("Привет мир".to_string()));
            }
            _ => panic!("Expected String"),
        }
    }

    #[test]
    fn test_base64() {
        let enc = base64_encode_fn();
        let result = enc.call(&[Value::String("Hello".to_string())]).unwrap();
        assert_eq!(result, Value::String("SGVsbG8=".to_string()));

        let dec = base64_decode_fn();
        let decoded = dec.call(&[Value::String("SGVsbG8=".to_string())]).unwrap();
        assert_eq!(decoded, Value::String("Hello".to_string()));
    }

    #[test]
    fn test_dns_lookup() {
        let f = dns_lookup_fn();
        let result = f.call(&[Value::String("localhost".to_string())]);
        // localhost должен резолвиться в 127.0.0.1 или ::1
        assert!(result.is_ok());
    }
}
