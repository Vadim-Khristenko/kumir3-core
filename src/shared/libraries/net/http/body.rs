// ============================================================================
//                    HTTP BODY
// ============================================================================
//
// Представление тела HTTP запроса/ответа с поддержкой:
// - Фиксированные данные (bytes)
// - Streaming (async iterator)
// - JSON сериализация/десериализация
//
// ============================================================================

use tokio::sync::mpsc;

use crate::shared::libraries::net::{NetError, NetResult};

// ============================================================================
//                    BODY
// ============================================================================

/// HTTP тело.
#[derive(Debug)]
pub enum Body {
    /// Пустое тело
    Empty,
    /// Фиксированные данные
    Bytes(Vec<u8>),
    /// Streaming тело (через канал)
    Stream(BodyStream),
}

impl Body {
    /// Создаёт пустое тело.
    pub fn empty() -> Self {
        Body::Empty
    }

    /// Создаёт тело из байт.
    pub fn from_bytes(data: Vec<u8>) -> Self {
        if data.is_empty() {
            Body::Empty
        } else {
            Body::Bytes(data)
        }
    }

    /// Создаёт тело из строки.
    pub fn from_string(s: String) -> Self {
        Self::from_bytes(s.into_bytes())
    }

    /// Создаёт streaming тело.
    pub fn stream() -> (BodySender, Self) {
        let (tx, rx) = mpsc::channel(16);
        let sender = BodySender { inner: tx };
        let stream = BodyStream { inner: rx, buffer: Vec::new() };
        (sender, Body::Stream(stream))
    }

    /// Создаёт тело из JSON (статический метод).
    pub fn from_json<T: serde::Serialize>(value: &T) -> NetResult<Self> {
        let bytes = serde_json::to_vec(value)?;
        Ok(Body::Bytes(bytes))
    }

    /// Проверяет, пустое ли тело.
    pub fn is_empty(&self) -> bool {
        match self {
            Body::Empty => true,
            Body::Bytes(data) => data.is_empty(),
            Body::Stream(_) => false, // Нельзя знать заранее
        }
    }

    /// Возвращает известную длину (если есть).
    pub fn len(&self) -> Option<usize> {
        match self {
            Body::Empty => Some(0),
            Body::Bytes(data) => Some(data.len()),
            Body::Stream(_) => None, // Неизвестно заранее
        }
    }

    /// Конвертирует тело в байты (синхронно, только для Bytes).
    pub fn into_bytes(self) -> Vec<u8> {
        match self {
            Body::Empty => Vec::new(),
            Body::Bytes(data) => data,
            Body::Stream(_) => Vec::new(), // Stream нельзя синхронно прочитать
        }
    }

    /// Читает тело целиком как байты (async).
    pub async fn bytes(&mut self) -> NetResult<Vec<u8>> {
        match self {
            Body::Empty => Ok(Vec::new()),
            Body::Bytes(data) => Ok(std::mem::take(data)),
            Body::Stream(stream) => stream.collect().await,
        }
    }

    /// Читает тело как строку (async).
    pub async fn text(&mut self) -> NetResult<String> {
        let bytes = self.bytes().await?;
        String::from_utf8(bytes)
            .map_err(|e| NetError::Parse(format!("Invalid UTF-8: {}", e)))
    }

    /// Читает тело как JSON (async).
    pub async fn parse_json<T: serde::de::DeserializeOwned>(&mut self) -> NetResult<T> {
        let bytes = self.bytes().await?;
        serde_json::from_slice(&bytes)
            .map_err(|e| NetError::Serde(e.to_string()))
    }

    /// Алиас для parse_json.
    pub async fn json<T: serde::de::DeserializeOwned>(&mut self) -> NetResult<T> {
        self.parse_json().await
    }
}

impl Default for Body {
    fn default() -> Self {
        Body::Empty
    }
}

impl From<Vec<u8>> for Body {
    fn from(data: Vec<u8>) -> Self {
        Body::from_bytes(data)
    }
}

impl From<&[u8]> for Body {
    fn from(data: &[u8]) -> Self {
        Body::from_bytes(data.to_vec())
    }
}

impl From<String> for Body {
    fn from(s: String) -> Self {
        Body::from_string(s)
    }
}

impl From<&str> for Body {
    fn from(s: &str) -> Self {
        Body::from_string(s.to_string())
    }
}

// ============================================================================
//                    BODY STREAM
// ============================================================================

/// Streaming тело (получатель).
#[derive(Debug)]
pub struct BodyStream {
    inner: mpsc::Receiver<BodyChunk>,
    buffer: Vec<u8>,
}

/// Чанк данных для streaming.
#[derive(Debug, Clone)]
pub enum BodyChunk {
    /// Данные
    Data(Vec<u8>),
    /// Ошибка
    Error(String),
    /// Конец потока
    End,
}

impl BodyStream {
    /// Читает следующий чанк.
    pub async fn next(&mut self) -> Option<NetResult<Vec<u8>>> {
        match self.inner.recv().await {
            Some(BodyChunk::Data(data)) => Some(Ok(data)),
            Some(BodyChunk::Error(e)) => Some(Err(NetError::Internal(e))),
            Some(BodyChunk::End) | None => None,
        }
    }

    /// Собирает весь поток в Vec.
    pub async fn collect(&mut self) -> NetResult<Vec<u8>> {
        let mut result = std::mem::take(&mut self.buffer);
        
        while let Some(chunk) = self.next().await {
            result.extend(chunk?);
        }
        
        Ok(result)
    }

    /// Читает до определённого размера.
    pub async fn read(&mut self, max_size: usize) -> NetResult<Vec<u8>> {
        // Сначала отдаём из буфера
        if !self.buffer.is_empty() {
            if self.buffer.len() <= max_size {
                return Ok(std::mem::take(&mut self.buffer));
            } else {
                let data = self.buffer.drain(..max_size).collect();
                return Ok(data);
            }
        }

        // Читаем из канала
        match self.next().await {
            Some(Ok(data)) => {
                if data.len() <= max_size {
                    Ok(data)
                } else {
                    let (ret, rest) = data.split_at(max_size);
                    self.buffer = rest.to_vec();
                    Ok(ret.to_vec())
                }
            }
            Some(Err(e)) => Err(e),
            None => Ok(Vec::new()),
        }
    }
}

// ============================================================================
//                    BODY SENDER
// ============================================================================

/// Отправитель для streaming тела.
pub struct BodySender {
    inner: mpsc::Sender<BodyChunk>,
}

impl BodySender {
    /// Отправляет чанк данных.
    pub async fn send(&self, data: Vec<u8>) -> NetResult<()> {
        self.inner.send(BodyChunk::Data(data)).await
            .map_err(|_| NetError::ConnectionClosed)
    }

    /// Отправляет данные из среза.
    pub async fn send_bytes(&self, data: &[u8]) -> NetResult<()> {
        self.send(data.to_vec()).await
    }

    /// Отправляет строку.
    pub async fn send_str(&self, s: &str) -> NetResult<()> {
        self.send(s.as_bytes().to_vec()).await
    }

    /// Отправляет ошибку.
    pub async fn send_error(&self, error: impl Into<String>) -> NetResult<()> {
        self.inner.send(BodyChunk::Error(error.into())).await
            .map_err(|_| NetError::ConnectionClosed)
    }

    /// Завершает поток.
    pub async fn finish(self) -> NetResult<()> {
        self.inner.send(BodyChunk::End).await
            .map_err(|_| NetError::ConnectionClosed)
    }

    /// Проверяет, закрыт ли канал.
    pub fn is_closed(&self) -> bool {
        self.inner.is_closed()
    }
}

// ============================================================================
//                    SIZED BODY
// ============================================================================

/// Тело с известным размером (для Content-Length).
pub struct SizedBody {
    data: Vec<u8>,
    position: usize,
}

impl SizedBody {
    pub fn new(data: Vec<u8>) -> Self {
        Self { data, position: 0 }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn remaining(&self) -> usize {
        self.data.len() - self.position
    }

    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let remaining = self.remaining();
        let to_read = buf.len().min(remaining);
        
        buf[..to_read].copy_from_slice(&self.data[self.position..self.position + to_read]);
        self.position += to_read;
        
        to_read
    }

    pub fn into_bytes(self) -> Vec<u8> {
        self.data
    }
}

// ============================================================================
//                    CHUNKED BODY ENCODER
// ============================================================================

/// Кодировщик для Transfer-Encoding: chunked.
pub struct ChunkedEncoder;

impl ChunkedEncoder {
    /// Кодирует чанк в формат chunked transfer encoding.
    pub fn encode_chunk(data: &[u8]) -> Vec<u8> {
        if data.is_empty() {
            return b"0\r\n\r\n".to_vec();
        }
        
        let mut result = Vec::new();
        result.extend_from_slice(format!("{:x}\r\n", data.len()).as_bytes());
        result.extend_from_slice(data);
        result.extend_from_slice(b"\r\n");
        result
    }

    /// Кодирует финальный чанк (завершение).
    pub fn encode_end() -> Vec<u8> {
        b"0\r\n\r\n".to_vec()
    }
}

/// Декодер для Transfer-Encoding: chunked.
pub struct ChunkedDecoder {
    buffer: Vec<u8>,
    state: ChunkedState,
    chunk_size: usize,
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum ChunkedState {
    Size,
    Data,
    DataEnd,
    Trailer,
    Done,
}

impl ChunkedDecoder {
    pub fn new() -> Self {
        Self {
            buffer: Vec::new(),
            state: ChunkedState::Size,
            chunk_size: 0,
        }
    }

    /// Добавляет данные для декодирования.
    pub fn feed(&mut self, data: &[u8]) {
        self.buffer.extend_from_slice(data);
    }

    /// Пытается декодировать следующий чанк.
    pub fn decode(&mut self) -> Option<NetResult<Vec<u8>>> {
        loop {
            match self.state {
                ChunkedState::Size => {
                    if let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
                        let size_str = String::from_utf8_lossy(&self.buffer[..pos]);
                        match usize::from_str_radix(size_str.trim(), 16) {
                            Ok(size) => {
                                self.chunk_size = size;
                                self.buffer.drain(..pos + 2);
                                
                                if size == 0 {
                                    self.state = ChunkedState::Trailer;
                                } else {
                                    self.state = ChunkedState::Data;
                                }
                            }
                            Err(e) => {
                                return Some(Err(NetError::Parse(format!("Invalid chunk size: {}", e))));
                            }
                        }
                    } else {
                        return None;
                    }
                }
                
                ChunkedState::Data => {
                    if self.buffer.len() >= self.chunk_size {
                        let data: Vec<u8> = self.buffer.drain(..self.chunk_size).collect();
                        self.state = ChunkedState::DataEnd;
                        return Some(Ok(data));
                    } else {
                        return None;
                    }
                }
                
                ChunkedState::DataEnd => {
                    if self.buffer.len() >= 2 {
                        self.buffer.drain(..2); // Убираем \r\n
                        self.state = ChunkedState::Size;
                    } else {
                        return None;
                    }
                }
                
                ChunkedState::Trailer => {
                    // Пропускаем trailers до пустой строки
                    if self.buffer.len() >= 2 && &self.buffer[..2] == b"\r\n" {
                        self.buffer.drain(..2);
                        self.state = ChunkedState::Done;
                    } else if let Some(pos) = self.buffer.windows(2).position(|w| w == b"\r\n") {
                        self.buffer.drain(..pos + 2);
                    } else {
                        return None;
                    }
                }
                
                ChunkedState::Done => {
                    return None;
                }
            }
        }
    }

    /// Проверяет, завершено ли декодирование.
    pub fn is_done(&self) -> bool {
        self.state == ChunkedState::Done
    }
}

impl Default for ChunkedDecoder {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_body_bytes() {
        let mut body = Body::from("Hello, World!");
        let bytes = body.bytes().await.unwrap();
        assert_eq!(bytes, b"Hello, World!");
    }

    #[tokio::test]
    async fn test_body_json() {
        #[derive(serde::Deserialize, PartialEq, Debug)]
        struct Data { name: String }
        
        let mut body = Body::from(r#"{"name": "Test"}"#);
        let data: Data = body.json().await.unwrap();
        assert_eq!(data.name, "Test");
    }

    #[tokio::test]
    async fn test_body_stream() {
        let (sender, mut body) = Body::stream();
        
        // Спавним отправитель
        tokio::spawn(async move {
            sender.send_str("Hello").await.unwrap();
            sender.send_str(" World").await.unwrap();
            sender.finish().await.unwrap();
        });
        
        // Собираем stream
        if let Body::Stream(ref mut stream) = body {
            let data = stream.collect().await.unwrap();
            assert_eq!(String::from_utf8(data).unwrap(), "Hello World");
        } else {
            panic!("Expected stream body");
        }
    }

    #[test]
    fn test_chunked_encoder() {
        let chunk = ChunkedEncoder::encode_chunk(b"Hello");
        assert_eq!(&chunk, b"5\r\nHello\r\n");
        
        let end = ChunkedEncoder::encode_end();
        assert_eq!(&end, b"0\r\n\r\n");
    }

    #[test]
    fn test_chunked_decoder() {
        let mut decoder = ChunkedDecoder::new();
        decoder.feed(b"5\r\nHello\r\n3\r\nBye\r\n0\r\n\r\n");
        
        let chunk1 = decoder.decode().unwrap().unwrap();
        assert_eq!(chunk1, b"Hello");
        
        let chunk2 = decoder.decode().unwrap().unwrap();
        assert_eq!(chunk2, b"Bye");
        
        assert!(decoder.decode().is_none());
        assert!(decoder.is_done());
    }
}
