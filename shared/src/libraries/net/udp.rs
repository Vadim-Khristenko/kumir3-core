// ============================================================================
//                    UDP ТРАНСПОРТ
// ============================================================================
//
// Асинхронный UDP сокет поверх tokio::net.
//
// ============================================================================

use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::net::UdpSocket as TokioUdpSocket;

use super::core::ConnectionId;
use super::{NetError, NetResult, NetworkConfig};

// ============================================================================
//                    КОНФИГУРАЦИЯ
// ============================================================================

/// Конфигурация UDP.
#[derive(Debug, Clone)]
pub struct UdpConfig {
    /// Включить SO_REUSEADDR
    pub reuse_addr: bool,
    /// Размер буфера приёма
    pub recv_buffer_size: usize,
    /// Размер буфера отправки
    pub send_buffer_size: usize,
    /// Включить broadcast
    pub broadcast: bool,
    /// TTL для multicast
    pub multicast_ttl: Option<u32>,
    /// Loopback для multicast
    pub multicast_loopback: bool,
}

impl Default for UdpConfig {
    fn default() -> Self {
        Self {
            reuse_addr: true,
            recv_buffer_size: 65535,
            send_buffer_size: 65535,
            broadcast: false,
            multicast_ttl: None,
            multicast_loopback: true,
        }
    }
}

impl From<&NetworkConfig> for UdpConfig {
    fn from(config: &NetworkConfig) -> Self {
        Self {
            reuse_addr: config.reuse_addr,
            recv_buffer_size: config.buffer_size,
            send_buffer_size: config.buffer_size,
            ..Default::default()
        }
    }
}

// ============================================================================
//                    UDP SOCKET
// ============================================================================

/// Асинхронный UDP сокет.
pub struct UdpSocket {
    /// Уникальный ID
    id: ConnectionId,
    /// Внутренний сокет
    inner: Arc<TokioUdpSocket>,
    /// Локальный адрес
    local_addr: SocketAddr,
    /// Конфигурация
    config: UdpConfig,
    /// Подключённый адрес (если есть)
    connected_addr: Option<SocketAddr>,
    /// Флаг закрытия
    closed: AtomicBool,
}

impl UdpSocket {
    /// Создаёт UDP сокет на указанном адресе.
    pub async fn bind(addr: SocketAddr, config: UdpConfig) -> NetResult<Self> {
        let socket = TokioUdpSocket::bind(addr).await?;
        
        if config.broadcast {
            socket.set_broadcast(true)?;
        }
        
        let local_addr = socket.local_addr()?;
        
        Ok(Self {
            id: ConnectionId::new(),
            inner: Arc::new(socket),
            local_addr,
            config,
            connected_addr: None,
            closed: AtomicBool::new(false),
        })
    }

    /// Создаёт UDP сокет на любом свободном порту.
    pub async fn any(config: UdpConfig) -> NetResult<Self> {
        Self::bind("0.0.0.0:0".parse().unwrap(), config).await
    }

    /// Подключается к удалённому адресу (для send/recv без указания адреса).
    pub async fn connect(&mut self, addr: SocketAddr) -> NetResult<()> {
        self.inner.connect(addr).await?;
        self.connected_addr = Some(addr);
        Ok(())
    }

    /// Возвращает ID.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Возвращает локальный адрес.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Возвращает подключённый адрес.
    pub fn connected_addr(&self) -> Option<SocketAddr> {
        self.connected_addr
    }

    /// Проверяет, закрыт ли сокет.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    // -------------------------------------------------------------------------
    // Отправка
    // -------------------------------------------------------------------------

    /// Отправляет данные на указанный адрес.
    pub async fn send_to(&self, buf: &[u8], addr: SocketAddr) -> NetResult<usize> {
        if self.is_closed() {
            return Err(NetError::ConnectionClosed);
        }
        Ok(self.inner.send_to(buf, addr).await?)
    }

    /// Отправляет данные на подключённый адрес.
    pub async fn send(&self, buf: &[u8]) -> NetResult<usize> {
        if self.is_closed() {
            return Err(NetError::ConnectionClosed);
        }
        if self.connected_addr.is_none() {
            return Err(NetError::InvalidAddress("Socket not connected".into()));
        }
        Ok(self.inner.send(buf).await?)
    }

    // -------------------------------------------------------------------------
    // Приём
    // -------------------------------------------------------------------------

    /// Принимает данные с любого адреса.
    pub async fn recv_from(&self, buf: &mut [u8]) -> NetResult<(usize, SocketAddr)> {
        if self.is_closed() {
            return Err(NetError::ConnectionClosed);
        }
        Ok(self.inner.recv_from(buf).await?)
    }

    /// Принимает данные с подключённого адреса.
    pub async fn recv(&self, buf: &mut [u8]) -> NetResult<usize> {
        if self.is_closed() {
            return Err(NetError::ConnectionClosed);
        }
        Ok(self.inner.recv(buf).await?)
    }

    /// Принимает датаграмму целиком (возвращает Vec).
    pub async fn recv_datagram(&self) -> NetResult<(Vec<u8>, SocketAddr)> {
        let mut buf = vec![0u8; self.config.recv_buffer_size];
        let (n, addr) = self.recv_from(&mut buf).await?;
        buf.truncate(n);
        Ok((buf, addr))
    }

    // -------------------------------------------------------------------------
    // Multicast
    // -------------------------------------------------------------------------

    /// Присоединяется к multicast группе.
    pub fn join_multicast_v4(
        &self,
        multiaddr: std::net::Ipv4Addr,
        interface: std::net::Ipv4Addr,
    ) -> NetResult<()> {
        self.inner.join_multicast_v4(multiaddr, interface)?;
        Ok(())
    }

    /// Покидает multicast группу.
    pub fn leave_multicast_v4(
        &self,
        multiaddr: std::net::Ipv4Addr,
        interface: std::net::Ipv4Addr,
    ) -> NetResult<()> {
        self.inner.leave_multicast_v4(multiaddr, interface)?;
        Ok(())
    }

    /// Присоединяется к multicast группе (IPv6).
    pub fn join_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> NetResult<()> {
        self.inner.join_multicast_v6(multiaddr, interface)?;
        Ok(())
    }

    /// Покидает multicast группу (IPv6).
    pub fn leave_multicast_v6(
        &self,
        multiaddr: &std::net::Ipv6Addr,
        interface: u32,
    ) -> NetResult<()> {
        self.inner.leave_multicast_v6(multiaddr, interface)?;
        Ok(())
    }

    /// Устанавливает TTL для multicast.
    pub fn set_multicast_ttl_v4(&self, ttl: u32) -> NetResult<()> {
        self.inner.set_multicast_ttl_v4(ttl)?;
        Ok(())
    }

    /// Устанавливает loopback для multicast.
    pub fn set_multicast_loop_v4(&self, on: bool) -> NetResult<()> {
        self.inner.set_multicast_loop_v4(on)?;
        Ok(())
    }

    // -------------------------------------------------------------------------
    // Broadcast
    // -------------------------------------------------------------------------

    /// Включает/выключает broadcast.
    pub fn set_broadcast(&self, on: bool) -> NetResult<()> {
        self.inner.set_broadcast(on)?;
        Ok(())
    }

    /// Отправляет broadcast.
    pub async fn broadcast(&self, port: u16, buf: &[u8]) -> NetResult<usize> {
        self.inner.set_broadcast(true)?;
        let addr = SocketAddr::from(([255, 255, 255, 255], port));
        self.send_to(buf, addr).await
    }

    // -------------------------------------------------------------------------
    // Утилиты
    // -------------------------------------------------------------------------

    /// Закрывает сокет.
    pub fn close(&self) {
        self.closed.store(true, Ordering::SeqCst);
    }

    /// Клонирует Arc на внутренний сокет.
    pub fn clone_inner(&self) -> Arc<TokioUdpSocket> {
        Arc::clone(&self.inner)
    }
}

// ============================================================================
//                    UDP DATAGRAM
// ============================================================================

/// Представляет UDP датаграмму.
#[derive(Debug, Clone)]
pub struct Datagram {
    /// Данные
    pub data: Vec<u8>,
    /// Адрес источника
    pub source: SocketAddr,
    /// Адрес назначения (если известен)
    pub destination: Option<SocketAddr>,
}

impl Datagram {
    /// Создаёт новую датаграмму.
    pub fn new(data: Vec<u8>, source: SocketAddr) -> Self {
        Self {
            data,
            source,
            destination: None,
        }
    }

    /// Возвращает данные как срез.
    pub fn as_bytes(&self) -> &[u8] {
        &self.data
    }

    /// Возвращает данные как строку (если валидный UTF-8).
    pub fn as_str(&self) -> Option<&str> {
        std::str::from_utf8(&self.data).ok()
    }

    /// Размер данных.
    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Пустая ли датаграмма.
    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_udp_bind() {
        let socket = UdpSocket::bind(
            "127.0.0.1:0".parse().unwrap(),
            UdpConfig::default()
        ).await.unwrap();

        assert_ne!(socket.local_addr().port(), 0);
        assert!(!socket.is_closed());
    }

    #[tokio::test]
    async fn test_udp_send_recv() {
        // Создаём два сокета
        let socket1 = UdpSocket::bind(
            "127.0.0.1:0".parse().unwrap(),
            UdpConfig::default()
        ).await.unwrap();
        
        let socket2 = UdpSocket::bind(
            "127.0.0.1:0".parse().unwrap(),
            UdpConfig::default()
        ).await.unwrap();

        let addr1 = socket1.local_addr();
        let addr2 = socket2.local_addr();

        // Отправляем данные
        socket1.send_to(b"Hello UDP", addr2).await.unwrap();

        // Принимаем данные
        let mut buf = [0u8; 16];
        let (n, from) = socket2.recv_from(&mut buf).await.unwrap();

        assert_eq!(&buf[..n], b"Hello UDP");
        assert_eq!(from, addr1);
    }

    #[tokio::test]
    async fn test_udp_connected() {
        let mut socket1 = UdpSocket::any(UdpConfig::default()).await.unwrap();
        let socket2 = UdpSocket::any(UdpConfig::default()).await.unwrap();

        let addr2 = socket2.local_addr();
        socket1.connect(addr2).await.unwrap();

        assert_eq!(socket1.connected_addr(), Some(addr2));

        // Теперь можем использовать send без адреса
        socket1.send(b"Connected").await.unwrap();

        let (data, _) = socket2.recv_datagram().await.unwrap();
        assert_eq!(&data, b"Connected");
    }
}
