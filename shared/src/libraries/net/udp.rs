//! UDP-операции: отправка, приём, широковещательная рассылка

use std::net::UdpSocket;
use std::sync::Arc;
use std::time::Duration;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::value::{TypeKind, Value};

/// udp_отправить(адрес, данные) → цел
pub fn udp_send_fn() -> LibFunctionDef {
    LibFunctionDef::new("udp_отправить")
        .with_aliases(vec![Arc::from("udp_send")])
        .with_description("Отправляет данные по UDP и возвращает число отправленных байтов")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .with_param(LibParamDef::value("данные", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес' (host:port)".to_string())?;
            let data = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'данные'".to_string())?;

            let socket = UdpSocket::bind("0.0.0.0:0")
                .map_err(|e| format!("Ошибка создания UDP-сокета: {}", e))?;
            let sent = socket
                .send_to(data.as_bytes(), addr.as_str())
                .map_err(|e| format!("Ошибка отправки UDP: {}", e))?;
            Ok(Value::Number(crate::types::Number::I64(sent as i64)))
        })
}

/// udp_получить(адрес) → лит
pub fn udp_receive_fn() -> LibFunctionDef {
    LibFunctionDef::new("udp_получить")
        .with_aliases(vec![Arc::from("udp_receive"), Arc::from("udp_recv")])
        .with_description("Слушает UDP-порт и получает одну датаграмму")
        .with_param(LibParamDef::value("адрес", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let addr = args
                .first()
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'адрес' (host:port)".to_string())?;

            let socket = UdpSocket::bind(addr.as_str())
                .map_err(|e| format!("Ошибка привязки к '{}': {}", addr, e))?;
            socket
                .set_read_timeout(Some(Duration::from_secs(10)))
                .map_err(|e| format!("Ошибка таймаута: {}", e))?;

            let mut buffer = vec![0u8; 65536];
            let (n, peer) = socket
                .recv_from(&mut buffer)
                .map_err(|e| format!("Ошибка приёма UDP: {}", e))?;

            let data = String::from_utf8_lossy(&buffer[..n]).into_owned();
            Ok(Value::String(format!("от {}: {}", peer, data)))
        })
}

/// udp_широковещание(порт, данные) → цел
pub fn udp_broadcast_fn() -> LibFunctionDef {
    LibFunctionDef::new("udp_широковещание")
        .with_aliases(vec![Arc::from("udp_broadcast")])
        .with_description("Отправляет широковещательный UDP-пакет на указанный порт")
        .with_param(LibParamDef::value("порт", TypeKind::Int64))
        .with_param(LibParamDef::value("данные", TypeKind::String))
        .returns(TypeKind::Int64)
        .with_handler(|args| {
            let port = args
                .first()
                .and_then(|v| v.as_number())
                .and_then(|n| n.to_i64())
                .ok_or_else(|| "Ожидается целочисленный аргумент 'порт'".to_string())?;
            let data = args
                .get(1)
                .and_then(|v| v.as_string())
                .ok_or_else(|| "Ожидается строковый аргумент 'данные'".to_string())?;

            let socket = UdpSocket::bind("0.0.0.0:0")
                .map_err(|e| format!("Ошибка создания UDP-сокета: {}", e))?;
            socket
                .set_broadcast(true)
                .map_err(|e| format!("Ошибка включения broadcast: {}", e))?;

            let addr = format!("255.255.255.255:{}", port);
            let sent = socket
                .send_to(data.as_bytes(), &addr)
                .map_err(|e| format!("Ошибка широковещательной отправки: {}", e))?;

            Ok(Value::Number(crate::types::Number::I64(sent as i64)))
        })
}
