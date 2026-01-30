// ============================================================================
//                    TCP ТРАНСПОРТ
// ============================================================================
//
// Асинхронный TCP listener и connection поверх tokio::net.
//
// ============================================================================

use std::net::SocketAddr;
use std::sync::atomic::{AtomicBool, Ordering};

use tokio::net::{TcpListener as TokioListener, TcpStream};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

use super::core::{ConnectionId, ConnectionState, ShutdownSignal};
use super::{NetError, NetResult, NetworkConfig};

// ============================================================================
//                    КОНФИГУРАЦИЯ
// ============================================================================

/// Конфигурация TCP.
#[derive(Debug, Clone)]
pub struct TcpConfig {
    /// Включить TCP_NODELAY
    pub nodelay: bool,
    /// Включить SO_REUSEADDR
    pub reuse_addr: bool,
    /// Backlog для listener
    pub backlog: u32,
    /// Размер буфера чтения
    pub read_buffer_size: usize,
    /// Размер буфера записи
    pub write_buffer_size: usize,
    /// Keep-alive интервал (None = отключено)
    pub keepalive: Option<std::time::Duration>,
}

impl Default for TcpConfig {
    fn default() -> Self {
        Self {
            nodelay: true,
            reuse_addr: true,
            backlog: 1024,
            read_buffer_size: 8192,
            write_buffer_size: 8192,
            keepalive: Some(std::time::Duration::from_secs(60)),
        }
    }
}

impl From<&NetworkConfig> for TcpConfig {
    fn from(config: &NetworkConfig) -> Self {
        Self {
            nodelay: config.tcp_nodelay,
            reuse_addr: config.reuse_addr,
            backlog: config.backlog,
            read_buffer_size: config.buffer_size,
            write_buffer_size: config.buffer_size,
            keepalive: Some(std::time::Duration::from_secs(60)),
        }
    }
}

// ============================================================================
//                    TCP LISTENER
// ============================================================================

/// Асинхронный TCP listener.
pub struct TcpListener {
    /// Внутренний tokio listener
    inner: TokioListener,
    /// Конфигурация
    config: TcpConfig,
    /// Сигнал shutdown
    shutdown: ShutdownSignal,
    /// Флаг закрытия
    closed: AtomicBool,
}

impl TcpListener {
    /// Создаёт listener на указанном адресе.
    pub async fn bind(addr: SocketAddr, config: TcpConfig) -> NetResult<Self> {
        let socket = if addr.is_ipv6() {
            tokio::net::TcpSocket::new_v6()?
        } else {
            tokio::net::TcpSocket::new_v4()?
        };

        if config.reuse_addr {
            socket.set_reuseaddr(true)?;
        }

        socket.bind(addr)?;
        let listener = socket.listen(config.backlog)?;

        Ok(Self {
            inner: listener,
            config,
            shutdown: ShutdownSignal::new(),
            closed: AtomicBool::new(false),
        })
    }

    /// Принимает новое соединение.
    pub async fn accept(&self) -> NetResult<TcpConnection> {
        if self.closed.load(Ordering::SeqCst) {
            return Err(NetError::ConnectionClosed);
        }

        tokio::select! {
            result = self.inner.accept() => {
                let (stream, peer_addr) = result?;
                
                // Применяем конфигурацию
                if self.config.nodelay {
                    stream.set_nodelay(true)?;
                }
                
                Ok(TcpConnection::from_stream(stream, peer_addr, self.config.clone()))
            }
            _ = self.shutdown.wait() => {
                Err(NetError::ConnectionClosed)
            }
        }
    }

    /// Возвращает локальный адрес.
    pub fn local_addr(&self) -> NetResult<SocketAddr> {
        Ok(self.inner.local_addr()?)
    }

    /// Возвращает сигнал shutdown.
    pub fn shutdown_signal(&self) -> ShutdownSignal {
        self.shutdown.subscribe()
    }

    /// Закрывает listener.
    pub async fn close(&self) -> NetResult<()> {
        self.closed.store(true, Ordering::SeqCst);
        self.shutdown.shutdown();
        Ok(())
    }

    /// Проверяет, закрыт ли listener.
    pub fn is_closed(&self) -> bool {
        self.closed.load(Ordering::SeqCst)
    }

    /// Итератор по входящим соединениям.
    pub fn incoming(&self) -> IncomingConnections<'_> {
        IncomingConnections { listener: self }
    }
}

/// Итератор входящих соединений.
pub struct IncomingConnections<'a> {
    listener: &'a TcpListener,
}

impl<'a> IncomingConnections<'a> {
    /// Получает следующее соединение.
    pub async fn next(&self) -> Option<NetResult<TcpConnection>> {
        if self.listener.is_closed() {
            return None;
        }
        Some(self.listener.accept().await)
    }
}

// ============================================================================
//                    TCP CONNECTION
// ============================================================================

/// Асинхронное TCP соединение.
pub struct TcpConnection {
    /// Уникальный ID
    id: ConnectionId,
    /// Внутренний поток (Option для возможности take)
    stream: Option<TcpStream>,
    /// Адрес пира
    peer_addr: SocketAddr,
    /// Локальный адрес
    local_addr: SocketAddr,
    /// Конфигурация
    config: TcpConfig,
    /// Состояние
    state: ConnectionState,
}

impl TcpConnection {
    /// Создаёт соединение из существующего потока.
    pub fn from_stream(stream: TcpStream, peer_addr: SocketAddr, config: TcpConfig) -> Self {
        let local_addr = stream.local_addr().unwrap_or_else(|_| {
            SocketAddr::from(([0, 0, 0, 0], 0))
        });

        Self {
            id: ConnectionId::new(),
            stream: Some(stream),
            peer_addr,
            local_addr,
            config,
            state: ConnectionState::Connected,
        }
    }

    /// Подключается к удалённому адресу.
    pub async fn connect(addr: SocketAddr, config: TcpConfig) -> NetResult<Self> {
        let stream = TcpStream::connect(addr).await?;

        if config.nodelay {
            stream.set_nodelay(true)?;
        }

        let local_addr = stream.local_addr()?;

        Ok(Self {
            id: ConnectionId::new(),
            stream: Some(stream),
            peer_addr: addr,
            local_addr,
            config,
            state: ConnectionState::Connected,
        })
    }

    /// Возвращает ID соединения.
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    /// Возвращает адрес пира.
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// Возвращает локальный адрес.
    pub fn local_addr(&self) -> SocketAddr {
        self.local_addr
    }

    /// Возвращает состояние.
    pub fn state(&self) -> ConnectionState {
        self.state
    }

    /// Проверяет, открыто ли соединение.
    pub fn is_open(&self) -> bool {
        matches!(self.state, ConnectionState::Connected)
    }

    /// Читает данные в буфер.
    pub async fn read(&mut self, buf: &mut [u8]) -> NetResult<usize> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        let n = stream.read(buf).await?;
        if n == 0 {
            self.state = ConnectionState::Closed;
        }
        Ok(n)
    }

    /// Читает точно указанное количество байт.
    pub async fn read_exact(&mut self, buf: &mut [u8]) -> NetResult<()> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        stream.read_exact(buf).await?;
        Ok(())
    }

    /// Читает до указанного разделителя.
    pub async fn read_until(&mut self, delimiter: u8, buf: &mut Vec<u8>) -> NetResult<usize> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        let mut total = 0;
        let mut byte = [0u8; 1];
        
        loop {
            let n = stream.read(&mut byte).await?;
            if n == 0 {
                self.state = ConnectionState::Closed;
                break;
            }
            buf.push(byte[0]);
            total += 1;
            if byte[0] == delimiter {
                break;
            }
        }
        
        Ok(total)
    }

    /// Читает строку до \n.
    pub async fn read_line(&mut self) -> NetResult<String> {
        let mut buf = Vec::new();
        self.read_until(b'\n', &mut buf).await?;
        
        // Убираем \r\n или \n
        if buf.ends_with(&[b'\n']) {
            buf.pop();
        }
        if buf.ends_with(&[b'\r']) {
            buf.pop();
        }
        
        String::from_utf8(buf)
            .map_err(|e| NetError::Parse(e.to_string()))
    }

    /// Записывает данные.
    pub async fn write(&mut self, buf: &[u8]) -> NetResult<usize> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        Ok(stream.write(buf).await?)
    }

    /// Записывает все данные.
    pub async fn write_all(&mut self, buf: &[u8]) -> NetResult<()> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        stream.write_all(buf).await?;
        Ok(())
    }

    /// Сбрасывает буферы.
    pub async fn flush(&mut self) -> NetResult<()> {
        let stream = self.stream.as_mut()
            .ok_or(NetError::ConnectionClosed)?;
        
        stream.flush().await?;
        Ok(())
    }

    /// Закрывает соединение.
    pub async fn close(&mut self) -> NetResult<()> {
        if let Some(mut stream) = self.stream.take() {
            self.state = ConnectionState::Closing;
            stream.shutdown().await?;
            self.state = ConnectionState::Closed;
        }
        Ok(())
    }

    /// Разделяет на читающую и пишущую половины.
    pub fn split(self) -> NetResult<(TcpReadHalf, TcpWriteHalf)> {
        let stream = self.stream
            .ok_or(NetError::ConnectionClosed)?;
        
        let (read, write) = stream.into_split();
        
        Ok((
            TcpReadHalf { 
                inner: read, 
                id: self.id,
                config: self.config.clone(),
            },
            TcpWriteHalf { 
                inner: write, 
                id: self.id,
                config: self.config,
            },
        ))
    }

    /// Возвращает внутренний TcpStream (забирает ownership).
    pub fn into_inner(mut self) -> Option<TcpStream> {
        self.stream.take()
    }
}

// ============================================================================
//                    SPLIT HALVES
// ============================================================================

/// Читающая половина TCP соединения.
pub struct TcpReadHalf {
    inner: tokio::net::tcp::OwnedReadHalf,
    id: ConnectionId,
    config: TcpConfig,
}

impl TcpReadHalf {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub async fn read(&mut self, buf: &mut [u8]) -> NetResult<usize> {
        Ok(self.inner.read(buf).await?)
    }

    pub async fn read_exact(&mut self, buf: &mut [u8]) -> NetResult<()> {
        self.inner.read_exact(buf).await?;
        Ok(())
    }
}

/// Пишущая половина TCP соединения.
pub struct TcpWriteHalf {
    inner: tokio::net::tcp::OwnedWriteHalf,
    id: ConnectionId,
    config: TcpConfig,
}

impl TcpWriteHalf {
    pub fn id(&self) -> ConnectionId {
        self.id
    }

    pub async fn write(&mut self, buf: &[u8]) -> NetResult<usize> {
        Ok(self.inner.write(buf).await?)
    }

    pub async fn write_all(&mut self, buf: &[u8]) -> NetResult<()> {
        self.inner.write_all(buf).await?;
        Ok(())
    }

    pub async fn flush(&mut self) -> NetResult<()> {
        self.inner.flush().await?;
        Ok(())
    }

    pub async fn shutdown(&mut self) -> NetResult<()> {
        self.inner.shutdown().await?;
        Ok(())
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_tcp_listener_bind() {
        let listener = TcpListener::bind(
            "127.0.0.1:0".parse().unwrap(),
            TcpConfig::default()
        ).await.unwrap();

        let addr = listener.local_addr().unwrap();
        assert_ne!(addr.port(), 0);

        listener.close().await.unwrap();
        assert!(listener.is_closed());
    }

    #[tokio::test]
    async fn test_tcp_connection() {
        // Создаём listener
        let listener = TcpListener::bind(
            "127.0.0.1:0".parse().unwrap(),
            TcpConfig::default()
        ).await.unwrap();
        let addr = listener.local_addr().unwrap();

        // Спавним задачу для принятия соединения
        let accept_task = tokio::spawn(async move {
            listener.accept().await
        });

        // Подключаемся
        let mut client = TcpConnection::connect(addr, TcpConfig::default()).await.unwrap();
        assert!(client.is_open());

        // Получаем серверное соединение
        let mut server = accept_task.await.unwrap().unwrap();

        // Отправляем данные
        client.write_all(b"Hello").await.unwrap();
        client.flush().await.unwrap();

        // Читаем данные
        let mut buf = [0u8; 5];
        server.read_exact(&mut buf).await.unwrap();
        assert_eq!(&buf, b"Hello");

        // Закрываем
        client.close().await.unwrap();
        server.close().await.unwrap();
    }
}
