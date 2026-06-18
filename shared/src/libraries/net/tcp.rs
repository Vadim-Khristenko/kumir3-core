//! TCP-операции: подключение, отправка, приём, прослушивание

use std::io::{Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::time::Duration;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// tcp_подключение(адрес) → лит
pub fn tcp_connect_fn() -> LibFunctionDef {
    LibFunctionDef::new("tcp_подключение")
        .with_aliases(vec![Arc::from("tcp_connect")])
        .with_description("Подключается к TCP-серверу и возвращает дескриптор соединения")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес' (host:port)".to_string())?;
            // Проверяем соединение (с таймаутом 10 сек)
            let stream = TcpStream::connect_timeout(
                &addr
                    .parse()
                    .map_err(|e| format!("Неверный адрес '{}': {}", addr, e))?,
                Duration::from_secs(10),
            )
            .map_err(|e| format!("Ошибка подключения к '{}': {}", addr, e))?;

            let local = stream
                .local_addr()
                .map(|a| a.to_string())
                .unwrap_or_default();
            let peer = stream
                .peer_addr()
                .map(|a| a.to_string())
                .unwrap_or_default();

            // Закрываем (КуМир 3 не держит handle)
            drop(stream);
            Ok(Value::String(format!("{}→{}", local, peer)))
        })
}

/// tcp_отправить(адрес, данные) → цел
pub fn tcp_send_fn() -> LibFunctionDef {
    LibFunctionDef::new("tcp_отправить")
        .with_aliases(vec![Arc::from("tcp_send")])
        .with_description("Отправляет данные по TCP и возвращает количество отправленных байтов")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .with_param(LibParamDef::value("данные", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес'".to_string())?;
            let data = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'данные'".to_string())?;

            let mut stream = TcpStream::connect(addr.as_str())
                .map_err(|e| format!("Ошибка подключения: {}", e))?;
            stream
                .set_write_timeout(Some(Duration::from_secs(10)))
                .map_err(|e| format!("Ошибка установки таймаута: {}", e))?;

            let bytes_written = stream
                .write(data.as_bytes())
                .map_err(|e| format!("Ошибка отправки: {}", e))?;
            stream.flush().map_err(|e| format!("Ошибка flush: {}", e))?;

            Ok(Value::Number(crate::types::Number::I64(
                bytes_written as i64,
            )))
        })
}

/// tcp_получить(адрес) → лит
pub fn tcp_receive_fn() -> LibFunctionDef {
    LibFunctionDef::new("tcp_получить")
        .with_aliases(vec![Arc::from("tcp_receive"), Arc::from("tcp_recv")])
        .with_description("Подключается к TCP-серверу и читает ответ")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес'".to_string())?;

            let mut stream = TcpStream::connect(addr.as_str())
                .map_err(|e| format!("Ошибка подключения: {}", e))?;
            stream
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|e| format!("Ошибка установки таймаута: {}", e))?;

            let mut buffer = vec![0u8; 65536];
            let n = stream
                .read(&mut buffer)
                .map_err(|e| format!("Ошибка чтения: {}", e))?;

            let response = String::from_utf8_lossy(&buffer[..n]).into_owned();
            Ok(Value::String(response))
        })
}

/// tcp_слушать(адрес) → лит
pub fn tcp_listen_fn() -> LibFunctionDef {
    LibFunctionDef::new("tcp_слушать")
        .with_aliases(vec![Arc::from("tcp_listen")])
        .with_description("Слушает TCP-порт, принимает одно соединение и читает данные")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес' (host:port)".to_string())?;

            let listener = std::net::TcpListener::bind(addr.as_str())
                .map_err(|e| format!("Ошибка привязки к '{}': {}", addr, e))?;

            // Принимаем одно соединение (с таймаутом неблокирующего accept)
            listener
                .set_nonblocking(false)
                .map_err(|e| format!("Ошибка: {}", e))?;

            let (mut stream, peer) = listener
                .accept()
                .map_err(|e| format!("Ошибка приёма соединения: {}", e))?;

            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .map_err(|e| format!("Ошибка таймаута: {}", e))?;

            let mut buffer = vec![0u8; 65536];
            let n = stream.read(&mut buffer).unwrap_or(0);
            let data = String::from_utf8_lossy(&buffer[..n]).to_string();

            Ok(Value::String(format!("от {}: {}", peer, data)))
        })
}
