// ============================================================================
//                    WEBSOCKET SUPPORT
// ============================================================================
//
// WebSocket протокол (RFC 6455):
// - Upgrade handshake
// - Text/Binary frames
// - Ping/Pong
// - Close handshake
//
// ============================================================================



use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::request::Request;
use super::response::Response;
use super::types::StatusCode;
use crate::shared::libraries::net::tcp::TcpConnection;
use crate::shared::libraries::net::{NetError, NetResult};

// ============================================================================
//                    WEBSOCKET CONSTANTS
// ============================================================================

/// WebSocket GUID для handshake.
const WS_GUID: &str = "258EAFA5-E914-47DA-95CA-C5AB0DC85B11";

/// Opcodes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opcode {
    Continuation = 0x0,
    Text = 0x1,
    Binary = 0x2,
    Close = 0x8,
    Ping = 0x9,
    Pong = 0xA,
}

impl Opcode {
    fn from_u8(byte: u8) -> Option<Self> {
        match byte & 0x0F {
            0x0 => Some(Self::Continuation),
            0x1 => Some(Self::Text),
            0x2 => Some(Self::Binary),
            0x8 => Some(Self::Close),
            0x9 => Some(Self::Ping),
            0xA => Some(Self::Pong),
            _ => None,
        }
    }
}

// ============================================================================
//                    WEBSOCKET MESSAGE
// ============================================================================

/// WebSocket сообщение.
#[derive(Debug, Clone)]
pub enum Message {
    /// Текстовое сообщение.
    Text(String),
    /// Бинарное сообщение.
    Binary(Vec<u8>),
    /// Ping.
    Ping(Vec<u8>),
    /// Pong.
    Pong(Vec<u8>),
    /// Close с опциональным кодом и причиной.
    Close(Option<(u16, String)>),
}

impl Message {
    pub fn text(s: impl Into<String>) -> Self {
        Self::Text(s.into())
    }

    pub fn binary(data: impl Into<Vec<u8>>) -> Self {
        Self::Binary(data.into())
    }

    pub fn ping(data: impl Into<Vec<u8>>) -> Self {
        Self::Ping(data.into())
    }

    pub fn pong(data: impl Into<Vec<u8>>) -> Self {
        Self::Pong(data.into())
    }

    pub fn close() -> Self {
        Self::Close(None)
    }

    pub fn close_with_reason(code: u16, reason: impl Into<String>) -> Self {
        Self::Close(Some((code, reason.into())))
    }

    /// Проверяет, является ли сообщение текстом.
    pub fn is_text(&self) -> bool {
        matches!(self, Self::Text(_))
    }

    /// Проверяет, является ли сообщение бинарным.
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary(_))
    }

    /// Проверяет, является ли сообщение close.
    pub fn is_close(&self) -> bool {
        matches!(self, Self::Close(_))
    }

    /// Возвращает текст, если это текстовое сообщение.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Self::Text(s) => Some(s),
            _ => None,
        }
    }

    /// Возвращает данные, если это бинарное сообщение.
    pub fn as_binary(&self) -> Option<&[u8]> {
        match self {
            Self::Binary(data) => Some(data),
            _ => None,
        }
    }

    /// Размер payload.
    pub fn len(&self) -> usize {
        match self {
            Self::Text(s) => s.len(),
            Self::Binary(data) => data.len(),
            Self::Ping(data) => data.len(),
            Self::Pong(data) => data.len(),
            Self::Close(Some((_, reason))) => 2 + reason.len(),
            Self::Close(None) => 0,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

// ============================================================================
//                    WEBSOCKET CLOSE CODES
// ============================================================================

pub mod close_codes {
    pub const NORMAL: u16 = 1000;
    pub const GOING_AWAY: u16 = 1001;
    pub const PROTOCOL_ERROR: u16 = 1002;
    pub const UNSUPPORTED_DATA: u16 = 1003;
    pub const NO_STATUS: u16 = 1005;
    pub const ABNORMAL: u16 = 1006;
    pub const INVALID_PAYLOAD: u16 = 1007;
    pub const POLICY_VIOLATION: u16 = 1008;
    pub const MESSAGE_TOO_BIG: u16 = 1009;
    pub const EXTENSION_REQUIRED: u16 = 1010;
    pub const INTERNAL_ERROR: u16 = 1011;
    pub const TLS_HANDSHAKE: u16 = 1015;
}

// ============================================================================
//                    WEBSOCKET UPGRADE
// ============================================================================

/// Проверяет, является ли запрос WebSocket upgrade.
pub fn is_upgrade_request(req: &Request) -> bool {
    let upgrade = req.headers()
        .get("upgrade")
        .map(|v| v.as_str().eq_ignore_ascii_case("websocket"))
        .unwrap_or(false);

    let connection = req.headers()
        .get("connection")
        .map(|v| v.as_str().to_lowercase().contains("upgrade"))
        .unwrap_or(false);

    let version = req.headers()
        .get("sec-websocket-version")
        .map(|v| v.as_str() == "13")
        .unwrap_or(false);

    upgrade && connection && version
}

/// Создаёт WebSocket upgrade response.
pub fn create_upgrade_response(req: &Request) -> NetResult<Response> {
    let key = req.headers()
        .get("sec-websocket-key")
        .ok_or_else(|| NetError::WebSocket("Missing Sec-WebSocket-Key".into()))?;

    let accept = compute_accept_key(key.as_str());

    let mut response = Response::new(StatusCode::SWITCHING_PROTOCOLS);
    response.headers_mut().insert("Upgrade", "websocket");
    response.headers_mut().insert("Connection", "Upgrade");
    response.headers_mut().insert("Sec-WebSocket-Accept", accept);

    Ok(response)
}

/// Вычисляет Sec-WebSocket-Accept.
fn compute_accept_key(key: &str) -> String {
    use sha1::{Sha1, Digest};

    let mut hasher = Sha1::new();
    hasher.update(key.as_bytes());
    hasher.update(WS_GUID.as_bytes());
    let hash = hasher.finalize();

    base64_encode(&hash)
}

/// Minimal base64 encoder.
fn base64_encode(data: &[u8]) -> String {
    const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut result = String::new();
    
    for chunk in data.chunks(3) {
        let b0 = chunk[0] as u32;
        let b1 = chunk.get(1).copied().unwrap_or(0) as u32;
        let b2 = chunk.get(2).copied().unwrap_or(0) as u32;
        
        let combined = (b0 << 16) | (b1 << 8) | b2;
        
        result.push(ALPHABET[((combined >> 18) & 0x3F) as usize] as char);
        result.push(ALPHABET[((combined >> 12) & 0x3F) as usize] as char);
        
        if chunk.len() > 1 {
            result.push(ALPHABET[((combined >> 6) & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
        
        if chunk.len() > 2 {
            result.push(ALPHABET[(combined & 0x3F) as usize] as char);
        } else {
            result.push('=');
        }
    }
    
    result
}

// ============================================================================
//                    WEBSOCKET CONNECTION
// ============================================================================

/// WebSocket соединение.
pub struct WebSocket {
    conn: TcpConnection,
    closed: bool,
    /// Флаг — серверная сторона (маскирование не нужно)
    is_server: bool,
}

impl WebSocket {
    /// Создаёт серверный WebSocket после upgrade.
    pub fn new_server(conn: TcpConnection) -> Self {
        Self {
            conn,
            closed: false,
            is_server: true,
        }
    }

    /// Создаёт клиентский WebSocket.
    pub fn new_client(conn: TcpConnection) -> Self {
        Self {
            conn,
            closed: false,
            is_server: false,
        }
    }

    /// Читает следующее сообщение.
    pub async fn recv(&mut self) -> NetResult<Option<Message>> {
        if self.closed {
            return Ok(None);
        }

        let frame = self.read_frame().await?;
        
        match frame {
            Some(Frame { opcode: Opcode::Text, payload, .. }) => {
                let text = String::from_utf8(payload)
                    .map_err(|e| NetError::WebSocket(format!("Invalid UTF-8: {}", e)))?;
                Ok(Some(Message::Text(text)))
            }
            Some(Frame { opcode: Opcode::Binary, payload, .. }) => {
                Ok(Some(Message::Binary(payload)))
            }
            Some(Frame { opcode: Opcode::Ping, payload, .. }) => {
                // Автоматически отвечаем Pong
                self.send(Message::Pong(payload.clone())).await?;
                Ok(Some(Message::Ping(payload)))
            }
            Some(Frame { opcode: Opcode::Pong, payload, .. }) => {
                Ok(Some(Message::Pong(payload)))
            }
            Some(Frame { opcode: Opcode::Close, payload, .. }) => {
                self.closed = true;
                
                let close_info = if payload.len() >= 2 {
                    let code = u16::from_be_bytes([payload[0], payload[1]]);
                    let reason = if payload.len() > 2 {
                        String::from_utf8_lossy(&payload[2..]).to_string()
                    } else {
                        String::new()
                    };
                    Some((code, reason))
                } else {
                    None
                };

                // Отправляем Close в ответ
                self.send_frame(Opcode::Close, &payload).await?;
                
                Ok(Some(Message::Close(close_info)))
            }
            Some(Frame { opcode: Opcode::Continuation, .. }) => {
                // TODO: Поддержка фрагментированных сообщений
                Err(NetError::WebSocket("Fragmented messages not supported".into()))
            }
            None => Ok(None),
        }
    }

    /// Отправляет сообщение.
    pub async fn send(&mut self, msg: Message) -> NetResult<()> {
        if self.closed {
            return Err(NetError::WebSocket("Connection closed".into()));
        }

        match msg {
            Message::Text(text) => {
                self.send_frame(Opcode::Text, text.as_bytes()).await
            }
            Message::Binary(data) => {
                self.send_frame(Opcode::Binary, &data).await
            }
            Message::Ping(data) => {
                self.send_frame(Opcode::Ping, &data).await
            }
            Message::Pong(data) => {
                self.send_frame(Opcode::Pong, &data).await
            }
            Message::Close(info) => {
                self.closed = true;
                let payload = match info {
                    Some((code, reason)) => {
                        let mut p = code.to_be_bytes().to_vec();
                        p.extend_from_slice(reason.as_bytes());
                        p
                    }
                    None => Vec::new(),
                };
                self.send_frame(Opcode::Close, &payload).await
            }
        }
    }

    /// Закрывает соединение.
    pub async fn close(&mut self) -> NetResult<()> {
        if !self.closed {
            self.send(Message::close()).await?;
        }
        self.conn.close().await?;
        Ok(())
    }

    /// Проверяет, закрыто ли соединение.
    pub fn is_closed(&self) -> bool {
        self.closed
    }

    // -------------------------------------------------------------------------
    // Frame Reading
    // -------------------------------------------------------------------------

    async fn read_frame(&mut self) -> NetResult<Option<Frame>> {
        // Читаем первые 2 байта
        let mut header = [0u8; 2];
        match self.conn.read_exact(&mut header).await {
            Ok(()) => {}
            Err(NetError::Io(ref e)) if e.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(None),
            Err(e) => return Err(e),
        }

        let fin = header[0] & 0x80 != 0;
        let opcode = Opcode::from_u8(header[0])
            .ok_or_else(|| NetError::WebSocket("Invalid opcode".into()))?;
        let masked = header[1] & 0x80 != 0;
        let mut payload_len = (header[1] & 0x7F) as usize;

        // Расширенная длина
        if payload_len == 126 {
            let mut len_bytes = [0u8; 2];
            self.conn.read_exact(&mut len_bytes).await?;
            payload_len = u16::from_be_bytes(len_bytes) as usize;
        } else if payload_len == 127 {
            let mut len_bytes = [0u8; 8];
            self.conn.read_exact(&mut len_bytes).await?;
            payload_len = u64::from_be_bytes(len_bytes) as usize;
        }

        // Mask key
        let mask_key = if masked {
            let mut key = [0u8; 4];
            self.conn.read_exact(&mut key).await?;
            Some(key)
        } else {
            None
        };

        // Payload
        let mut payload = vec![0u8; payload_len];
        if payload_len > 0 {
            self.conn.read_exact(&mut payload).await?;
        }

        // Unmask
        if let Some(key) = mask_key {
            for (i, byte) in payload.iter_mut().enumerate() {
                *byte ^= key[i % 4];
            }
        }

        Ok(Some(Frame { fin, opcode, payload }))
    }

    // -------------------------------------------------------------------------
    // Frame Writing
    // -------------------------------------------------------------------------

    async fn send_frame(&mut self, opcode: Opcode, payload: &[u8]) -> NetResult<()> {
        let mut frame = Vec::new();

        // Первый байт: FIN + opcode
        frame.push(0x80 | (opcode as u8));

        // Длина + mask bit
        let mask_bit = if self.is_server { 0x00 } else { 0x80 };
        
        if payload.len() < 126 {
            frame.push(mask_bit | (payload.len() as u8));
        } else if payload.len() <= 65535 {
            frame.push(mask_bit | 126);
            frame.extend_from_slice(&(payload.len() as u16).to_be_bytes());
        } else {
            frame.push(mask_bit | 127);
            frame.extend_from_slice(&(payload.len() as u64).to_be_bytes());
        }

        // Mask key (только для клиента)
        if !self.is_server {
            let mask_key: [u8; 4] = rand::random();
            frame.extend_from_slice(&mask_key);
            
            // Masked payload
            for (i, byte) in payload.iter().enumerate() {
                frame.push(byte ^ mask_key[i % 4]);
            }
        } else {
            frame.extend_from_slice(payload);
        }

        self.conn.write_all(&frame).await?;
        self.conn.flush().await?;

        Ok(())
    }
}

/// Внутренняя структура фрейма.
struct Frame {
    fin: bool,
    opcode: Opcode,
    payload: Vec<u8>,
}

// ============================================================================
//                    WEBSOCKET UPGRADER
// ============================================================================

/// Upgrader для преобразования HTTP соединения в WebSocket.
pub struct WebSocketUpgrader {
    req: Request,
    conn: Option<TcpConnection>,
}

impl WebSocketUpgrader {
    /// Создаёт upgrader из запроса и соединения.
    pub fn new(req: Request, conn: TcpConnection) -> Self {
        Self { req, conn: Some(conn) }
    }

    /// Выполняет upgrade и возвращает WebSocket.
    pub async fn upgrade(mut self) -> NetResult<(Response, WebSocket)> {
        let response = create_upgrade_response(&self.req)?;
        let conn = self.conn.take()
            .ok_or_else(|| NetError::WebSocket("Connection already taken".into()))?;
        let ws = WebSocket::new_server(conn);
        Ok((response, ws))
    }

    /// Проверяет, валидный ли upgrade запрос.
    pub fn is_valid(&self) -> bool {
        is_upgrade_request(&self.req)
    }

    /// Возвращает запрос.
    pub fn request(&self) -> &Request {
        &self.req
    }
}

// ============================================================================
//                    RANDOM (SIMPLE)
// ============================================================================

mod rand {
    use std::time::{SystemTime, UNIX_EPOCH};
    use std::cell::Cell;

    thread_local! {
        static STATE: Cell<u64> = Cell::new(
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos() as u64
        );
    }

    pub fn random<T: Random>() -> T {
        T::random()
    }

    pub trait Random {
        fn random() -> Self;
    }

    impl Random for [u8; 4] {
        fn random() -> Self {
            STATE.with(|s| {
                let mut state = s.get();
                let mut result = [0u8; 4];
                for byte in &mut result {
                    // Simple xorshift
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    *byte = state as u8;
                }
                s.set(state);
                result
            })
        }
    }
}

// ============================================================================
//                    SHA1 (MINIMAL)
// ============================================================================

mod sha1 {
    pub struct Sha1 {
        state: [u32; 5],
        buffer: Vec<u8>,
        len: u64,
    }

    pub trait Digest {
        fn new() -> Self;
        fn update(&mut self, data: &[u8]);
        fn finalize(self) -> [u8; 20];
    }

    impl Digest for Sha1 {
        fn new() -> Self {
            Self {
                state: [0x67452301, 0xEFCDAB89, 0x98BADCFE, 0x10325476, 0xC3D2E1F0],
                buffer: Vec::new(),
                len: 0,
            }
        }

        fn update(&mut self, data: &[u8]) {
            self.buffer.extend_from_slice(data);
            self.len += data.len() as u64;
            
            while self.buffer.len() >= 64 {
                let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
                self.process_block(&block);
                self.buffer.drain(..64);
            }
        }

        fn finalize(mut self) -> [u8; 20] {
            let bit_len = self.len * 8;
            
            // Padding
            self.buffer.push(0x80);
            while (self.buffer.len() % 64) != 56 {
                self.buffer.push(0x00);
            }
            self.buffer.extend_from_slice(&bit_len.to_be_bytes());
            
            // Process remaining
            while self.buffer.len() >= 64 {
                let block: [u8; 64] = self.buffer[..64].try_into().unwrap();
                self.process_block(&block);
                self.buffer.drain(..64);
            }
            
            // Output
            let mut result = [0u8; 20];
            for (i, &s) in self.state.iter().enumerate() {
                result[i*4..(i+1)*4].copy_from_slice(&s.to_be_bytes());
            }
            result
        }
    }

    impl Sha1 {
        fn process_block(&mut self, block: &[u8; 64]) {
            let mut w = [0u32; 80];
            
            for i in 0..16 {
                w[i] = u32::from_be_bytes([
                    block[i*4], block[i*4+1], block[i*4+2], block[i*4+3]
                ]);
            }
            
            for i in 16..80 {
                w[i] = (w[i-3] ^ w[i-8] ^ w[i-14] ^ w[i-16]).rotate_left(1);
            }
            
            let [mut a, mut b, mut c, mut d, mut e] = self.state;
            
            for i in 0..80 {
                let (f, k) = match i {
                    0..=19 => ((b & c) | ((!b) & d), 0x5A827999u32),
                    20..=39 => (b ^ c ^ d, 0x6ED9EBA1u32),
                    40..=59 => ((b & c) | (b & d) | (c & d), 0x8F1BBCDCu32),
                    _ => (b ^ c ^ d, 0xCA62C1D6u32),
                };
                
                let temp = a.rotate_left(5)
                    .wrapping_add(f)
                    .wrapping_add(e)
                    .wrapping_add(k)
                    .wrapping_add(w[i]);
                e = d;
                d = c;
                c = b.rotate_left(30);
                b = a;
                a = temp;
            }
            
            self.state[0] = self.state[0].wrapping_add(a);
            self.state[1] = self.state[1].wrapping_add(b);
            self.state[2] = self.state[2].wrapping_add(c);
            self.state[3] = self.state[3].wrapping_add(d);
            self.state[4] = self.state[4].wrapping_add(e);
        }
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use sha1::Digest;

    #[test]
    fn test_message_types() {
        let text = Message::text("hello");
        assert!(text.is_text());
        assert_eq!(text.as_text(), Some("hello"));

        let binary = Message::binary(vec![1, 2, 3]);
        assert!(binary.is_binary());
        assert_eq!(binary.as_binary(), Some(&[1, 2, 3][..]));
    }

    #[test]
    fn test_accept_key() {
        // Пример из RFC 6455
        let key = "dGhlIHNhbXBsZSBub25jZQ==";
        let accept = compute_accept_key(key);
        assert_eq!(accept, "s3pPLMBiTxaQ9kYGzzhZRbK+xOo=");
    }

    #[test]
    fn test_sha1() {
        let mut hasher = sha1::Sha1::new();
        hasher.update(b"hello");
        let hash = hasher.finalize();
        
        // SHA1("hello") = aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d
        assert_eq!(hash[0], 0xaa);
        assert_eq!(hash[1], 0xf4);
    }
}
