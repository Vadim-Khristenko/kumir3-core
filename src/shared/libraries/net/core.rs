// ============================================================================
//                    ЯДРО СЕТЕВОЙ БИБЛИОТЕКИ
// ============================================================================
//
// Базовые абстракции и трейты для всех сетевых компонентов.
//
// ============================================================================

use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, AtomicBool, Ordering};

use super::NetResult;

// ============================================================================
//                    ИДЕНТИФИКАТОРЫ
// ============================================================================

/// Уникальный идентификатор соединения.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ConnectionId(pub u64);

impl ConnectionId {
    /// Генерирует новый уникальный ID.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }

    /// Нулевой (невалидный) ID.
    pub fn null() -> Self {
        Self(0)
    }

    pub fn is_null(&self) -> bool {
        self.0 == 0
    }
}

impl Default for ConnectionId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for ConnectionId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Conn#{}", self.0)
    }
}

// ============================================================================
//                    ТРЕЙТЫ
// ============================================================================

/// Трейт для асинхронного чтения данных.
pub trait AsyncRead: Send + Sync {
    /// Читает данные в буфер, возвращает количество прочитанных байт.
    fn poll_read(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &mut [u8],
    ) -> std::task::Poll<NetResult<usize>>;
}

/// Трейт для асинхронной записи данных.
pub trait AsyncWrite: Send + Sync {
    /// Записывает данные из буфера, возвращает количество записанных байт.
    fn poll_write(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
        buf: &[u8],
    ) -> std::task::Poll<NetResult<usize>>;

    /// Сбрасывает буферы.
    fn poll_flush(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<NetResult<()>>;

    /// Закрывает поток записи.
    fn poll_shutdown(
        self: Pin<&mut Self>,
        cx: &mut std::task::Context<'_>,
    ) -> std::task::Poll<NetResult<()>>;
}

/// Трейт для соединения (чтение + запись).
pub trait Connection: AsyncRead + AsyncWrite {
    /// Возвращает ID соединения.
    fn id(&self) -> ConnectionId;

    /// Возвращает локальный адрес.
    fn local_addr(&self) -> NetResult<std::net::SocketAddr>;

    /// Возвращает удалённый адрес.
    fn peer_addr(&self) -> NetResult<std::net::SocketAddr>;

    /// Проверяет, открыто ли соединение.
    fn is_open(&self) -> bool;

    /// Закрывает соединение.
    fn close(&mut self) -> impl Future<Output = NetResult<()>> + Send;
}

/// Трейт для listener'а (принимает соединения).
pub trait Listener: Send + Sync {
    /// Тип принимаемого соединения.
    type Connection: Connection;

    /// Принимает новое соединение.
    fn accept(&self) -> impl Future<Output = NetResult<Self::Connection>> + Send;

    /// Возвращает локальный адрес.
    fn local_addr(&self) -> NetResult<std::net::SocketAddr>;

    /// Закрывает listener.
    fn close(&mut self) -> impl Future<Output = NetResult<()>> + Send;
}

// ============================================================================
//                    БУФЕР
// ============================================================================

/// Кольцевой буфер для эффективного чтения/записи.
pub struct RingBuffer {
    data: Vec<u8>,
    read_pos: usize,
    write_pos: usize,
    capacity: usize,
}

impl RingBuffer {
    /// Создаёт новый буфер с заданной ёмкостью.
    pub fn new(capacity: usize) -> Self {
        Self {
            data: vec![0u8; capacity],
            read_pos: 0,
            write_pos: 0,
            capacity,
        }
    }

    /// Количество данных для чтения.
    pub fn len(&self) -> usize {
        if self.write_pos >= self.read_pos {
            self.write_pos - self.read_pos
        } else {
            self.capacity - self.read_pos + self.write_pos
        }
    }

    /// Пуст ли буфер.
    pub fn is_empty(&self) -> bool {
        self.read_pos == self.write_pos
    }

    /// Свободное место для записи.
    pub fn available(&self) -> usize {
        self.capacity - self.len() - 1
    }

    /// Записывает данные в буфер.
    pub fn write(&mut self, data: &[u8]) -> usize {
        let available = self.available();
        let to_write = data.len().min(available);

        for &byte in &data[..to_write] {
            self.data[self.write_pos] = byte;
            self.write_pos = (self.write_pos + 1) % self.capacity;
        }

        to_write
    }

    /// Читает данные из буфера.
    pub fn read(&mut self, buf: &mut [u8]) -> usize {
        let available = self.len();
        let to_read = buf.len().min(available);

        for byte in buf.iter_mut().take(to_read) {
            *byte = self.data[self.read_pos];
            self.read_pos = (self.read_pos + 1) % self.capacity;
        }

        to_read
    }

    /// Очищает буфер.
    pub fn clear(&mut self) {
        self.read_pos = 0;
        self.write_pos = 0;
    }
}

// ============================================================================
//                    СОСТОЯНИЕ СОЕДИНЕНИЯ
// ============================================================================

/// Состояние соединения.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    /// Соединение устанавливается
    Connecting,
    /// Соединение установлено
    Connected,
    /// Идёт закрытие
    Closing,
    /// Соединение закрыто
    Closed,
    /// Ошибка
    Error,
}

impl std::fmt::Display for ConnectionState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Connecting => write!(f, "Connecting"),
            Self::Connected => write!(f, "Connected"),
            Self::Closing => write!(f, "Closing"),
            Self::Closed => write!(f, "Closed"),
            Self::Error => write!(f, "Error"),
        }
    }
}

// ============================================================================
//                    SHUTDOWN SIGNAL
// ============================================================================

/// Сигнал для graceful shutdown.
#[derive(Clone)]
pub struct ShutdownSignal {
    /// Флаг shutdown
    flag: Arc<AtomicBool>,
    /// Notify для пробуждения ожидающих
    notify: Arc<tokio::sync::Notify>,
}

impl ShutdownSignal {
    /// Создаёт новый сигнал.
    pub fn new() -> Self {
        Self {
            flag: Arc::new(AtomicBool::new(false)),
            notify: Arc::new(tokio::sync::Notify::new()),
        }
    }

    /// Отправляет сигнал shutdown.
    pub fn shutdown(&self) {
        self.flag.store(true, Ordering::SeqCst);
        self.notify.notify_waiters();
    }

    /// Проверяет, был ли отправлен сигнал.
    pub fn is_shutdown(&self) -> bool {
        self.flag.load(Ordering::SeqCst)
    }

    /// Ожидает сигнала shutdown.
    pub async fn wait(&self) {
        if self.is_shutdown() {
            return;
        }
        self.notify.notified().await;
    }

    /// Создаёт подписчика на shutdown.
    pub fn subscribe(&self) -> ShutdownSignal {
        self.clone()
    }
}

impl Default for ShutdownSignal {
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

    #[test]
    fn test_connection_id() {
        let id1 = ConnectionId::new();
        let id2 = ConnectionId::new();
        assert_ne!(id1, id2);
        assert!(!id1.is_null());
    }

    #[test]
    fn test_ring_buffer() {
        let mut buf = RingBuffer::new(16);
        assert!(buf.is_empty());

        let written = buf.write(b"hello");
        assert_eq!(written, 5);
        assert_eq!(buf.len(), 5);

        let mut out = [0u8; 10];
        let read = buf.read(&mut out);
        assert_eq!(read, 5);
        assert_eq!(&out[..5], b"hello");
    }

    #[test]
    fn test_shutdown_signal() {
        let signal = ShutdownSignal::new();
        assert!(!signal.is_shutdown());

        signal.shutdown();
        assert!(signal.is_shutdown());
    }
}
