// ============================================================================
//                    TLS ПОДДЕРЖКА
// ============================================================================
//
// TLS обёртки поверх tokio-rustls для безопасных соединений.
//
// ============================================================================

use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

use rustls::pki_types::{CertificateDer, PrivateKeyDer, ServerName};
use tokio::net::TcpStream;
use tokio_rustls::{
    TlsAcceptor as TokioTlsAcceptor,
    TlsConnector as TokioTlsConnector,
    client::TlsStream as ClientTlsStream,
    server::TlsStream as ServerTlsStream,
};

use super::{NetError, NetResult};
use super::tcp::TcpConnection;

// ============================================================================
//                    КОНФИГУРАЦИЯ
// ============================================================================

/// Тип приватного ключа для клонирования.
#[derive(Clone)]
enum KeyKind {
    Pkcs1(Vec<u8>),
    Pkcs8(Vec<u8>),
    Sec1(Vec<u8>),
}

impl KeyKind {
    fn to_private_key_der(&self) -> PrivateKeyDer<'static> {
        match self {
            KeyKind::Pkcs1(bytes) => PrivateKeyDer::Pkcs1(bytes.clone().into()),
            KeyKind::Pkcs8(bytes) => PrivateKeyDer::Pkcs8(bytes.clone().into()),
            KeyKind::Sec1(bytes) => PrivateKeyDer::Sec1(bytes.clone().into()),
        }
    }

    fn from_private_key_der(key: PrivateKeyDer<'static>) -> Self {
        match key {
            PrivateKeyDer::Pkcs1(data) => KeyKind::Pkcs1(data.secret_pkcs1_der().to_vec()),
            PrivateKeyDer::Pkcs8(data) => KeyKind::Pkcs8(data.secret_pkcs8_der().to_vec()),
            PrivateKeyDer::Sec1(data) => KeyKind::Sec1(data.secret_sec1_der().to_vec()),
            _ => KeyKind::Pkcs8(Vec::new()), // fallback
        }
    }
}

/// Конфигурация TLS.
#[derive(Clone)]
pub struct TlsConfig {
    /// Сертификаты (для сервера)
    certs: Option<Vec<CertificateDer<'static>>>,
    /// Приватный ключ (для сервера) - хранится как байты для Clone
    key: Option<KeyKind>,
    /// Доверенные корневые сертификаты (для клиента)
    root_certs: Option<Arc<rustls::RootCertStore>>,
    /// Требовать клиентский сертификат
    client_auth: bool,
    /// ALPN протоколы
    alpn_protocols: Vec<Vec<u8>>,
    /// Минимальная версия TLS
    min_version: TlsVersion,
    /// Максимальная версия TLS
    max_version: TlsVersion,
}

/// Версия TLS.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TlsVersion {
    Tls12,
    Tls13,
}

impl Default for TlsConfig {
    fn default() -> Self {
        Self {
            certs: None,
            key: None,
            root_certs: None,
            client_auth: false,
            alpn_protocols: vec![],
            min_version: TlsVersion::Tls12,
            max_version: TlsVersion::Tls13,
        }
    }
}

impl TlsConfig {
    /// Создаёт новую конфигурацию.
    pub fn new() -> Self {
        Self::default()
    }

    /// Создаёт конфигурацию для сервера.
    pub fn server() -> TlsConfigBuilder {
        TlsConfigBuilder::new(TlsConfigKind::Server)
    }

    /// Создаёт конфигурацию для клиента.
    pub fn client() -> TlsConfigBuilder {
        TlsConfigBuilder::new(TlsConfigKind::Client)
    }

    /// Проверяет, есть ли сертификаты сервера.
    pub fn has_server_certs(&self) -> bool {
        self.certs.is_some() && self.key.is_some()
    }

    /// Проверяет, есть ли корневые сертификаты.
    pub fn has_root_certs(&self) -> bool {
        self.root_certs.is_some()
    }
}

#[derive(Debug, Clone, Copy)]
enum TlsConfigKind {
    Server,
    Client,
}

/// Билдер для TlsConfig.
pub struct TlsConfigBuilder {
    kind: TlsConfigKind,
    config: TlsConfig,
}

impl TlsConfigBuilder {
    fn new(kind: TlsConfigKind) -> Self {
        Self {
            kind,
            config: TlsConfig::default(),
        }
    }

    /// Загружает сертификаты из PEM файла.
    pub fn with_cert_file(mut self, path: impl AsRef<Path>) -> NetResult<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| NetError::Tls(format!("Не удалось открыть файл сертификата: {}", e)))?;
        let mut reader = BufReader::new(file);
        
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
            .filter_map(|r| r.ok())
            .collect();
        
        if certs.is_empty() {
            return Err(NetError::Tls("Сертификаты не найдены в файле".into()));
        }
        
        self.config.certs = Some(certs);
        Ok(self)
    }

    /// Загружает приватный ключ из PEM файла.
    pub fn with_key_file(mut self, path: impl AsRef<Path>) -> NetResult<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| NetError::Tls(format!("Не удалось открыть файл ключа: {}", e)))?;
        let mut reader = BufReader::new(file);
        
        // Пробуем разные форматы ключей
        let key = rustls_pemfile::private_key(&mut reader)
            .map_err(|e| NetError::Tls(format!("Не удалось прочитать ключ: {}", e)))?
            .ok_or_else(|| NetError::Tls("Приватный ключ не найден в файле".into()))?;
        
        self.config.key = Some(KeyKind::from_private_key_der(key));
        Ok(self)
    }

    /// Устанавливает сертификаты напрямую.
    pub fn with_certs(mut self, certs: Vec<CertificateDer<'static>>) -> Self {
        self.config.certs = Some(certs);
        self
    }

    /// Устанавливает приватный ключ напрямую.
    pub fn with_key(mut self, key: PrivateKeyDer<'static>) -> Self {
        self.config.key = Some(KeyKind::from_private_key_der(key));
        self
    }

    /// Использует системные корневые сертификаты.
    pub fn with_native_roots(mut self) -> Self {
        let mut roots = rustls::RootCertStore::empty();
        roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
        self.config.root_certs = Some(Arc::new(roots));
        self
    }

    /// Загружает корневые сертификаты из файла.
    pub fn with_root_cert_file(mut self, path: impl AsRef<Path>) -> NetResult<Self> {
        let file = File::open(path.as_ref())
            .map_err(|e| NetError::Tls(format!("Не удалось открыть файл корневых сертификатов: {}", e)))?;
        let mut reader = BufReader::new(file);
        
        let mut roots = rustls::RootCertStore::empty();
        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut reader)
            .filter_map(|r| r.ok())
            .collect();
        
        for cert in certs {
            roots.add(cert)
                .map_err(|e| NetError::Tls(format!("Не удалось добавить корневой сертификат: {}", e)))?;
        }
        
        self.config.root_certs = Some(Arc::new(roots));
        Ok(self)
    }

    /// Требовать клиентский сертификат.
    pub fn with_client_auth(mut self, required: bool) -> Self {
        self.config.client_auth = required;
        self
    }

    /// Устанавливает ALPN протоколы.
    pub fn with_alpn(mut self, protocols: Vec<&str>) -> Self {
        self.config.alpn_protocols = protocols.into_iter()
            .map(|s| s.as_bytes().to_vec())
            .collect();
        self
    }

    /// Устанавливает минимальную версию TLS.
    pub fn with_min_version(mut self, version: TlsVersion) -> Self {
        self.config.min_version = version;
        self
    }

    /// Устанавливает максимальную версию TLS.
    pub fn with_max_version(mut self, version: TlsVersion) -> Self {
        self.config.max_version = version;
        self
    }

    /// Собирает конфигурацию.
    pub fn build(self) -> TlsConfig {
        self.config
    }
}

// ============================================================================
//                    TLS ACCEPTOR (Сервер)
// ============================================================================

/// TLS acceptor для серверных соединений.
pub struct TlsAcceptor {
    inner: TokioTlsAcceptor,
}

impl TlsAcceptor {
    /// Создаёт новый TLS acceptor.
    pub fn new(config: TlsConfig) -> NetResult<Self> {
        let certs = config.certs
            .ok_or_else(|| NetError::Tls("Сертификаты сервера не указаны".into()))?;
        let key_kind = config.key
            .ok_or_else(|| NetError::Tls("Приватный ключ не указан".into()))?;
        let key = key_kind.to_private_key_der();

        let server_config = rustls::ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| NetError::Tls(format!("Ошибка конфигурации TLS: {}", e)))?;

        Ok(Self {
            inner: TokioTlsAcceptor::from(Arc::new(server_config)),
        })
    }

    /// Принимает TLS соединение.
    pub async fn accept(&self, stream: TcpStream) -> NetResult<TlsServerStream> {
        let tls_stream = self.inner.accept(stream).await
            .map_err(|e| NetError::Tls(format!("TLS handshake failed: {}", e)))?;
        
        Ok(TlsServerStream { inner: tls_stream })
    }

    /// Принимает TLS соединение из TcpConnection.
    pub async fn accept_connection(&self, conn: TcpConnection) -> NetResult<TlsServerStream> {
        let stream = conn.into_inner()
            .ok_or(NetError::ConnectionClosed)?;
        self.accept(stream).await
    }
}

// ============================================================================
//                    TLS CONNECTOR (Клиент)
// ============================================================================

/// TLS connector для клиентских соединений.
pub struct TlsConnector {
    inner: TokioTlsConnector,
}

impl TlsConnector {
    /// Создаёт новый TLS connector.
    pub fn new(config: TlsConfig) -> NetResult<Self> {
        let roots = config.root_certs.unwrap_or_else(|| {
            let mut roots = rustls::RootCertStore::empty();
            roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());
            Arc::new(roots)
        });

        let client_config = rustls::ClientConfig::builder()
            .with_root_certificates((*roots).clone())
            .with_no_client_auth();

        Ok(Self {
            inner: TokioTlsConnector::from(Arc::new(client_config)),
        })
    }

    /// Устанавливает TLS соединение.
    pub async fn connect(&self, domain: &str, stream: TcpStream) -> NetResult<TlsClientStream> {
        let server_name = ServerName::try_from(domain.to_string())
            .map_err(|_| NetError::Tls(format!("Неверное имя сервера: {}", domain)))?;
        
        let tls_stream = self.inner.connect(server_name, stream).await
            .map_err(|e| NetError::Tls(format!("TLS handshake failed: {}", e)))?;
        
        Ok(TlsClientStream { inner: tls_stream })
    }

    /// Устанавливает TLS соединение из TcpConnection.
    pub async fn connect_with(&self, domain: &str, conn: TcpConnection) -> NetResult<TlsClientStream> {
        let stream = conn.into_inner()
            .ok_or(NetError::ConnectionClosed)?;
        self.connect(domain, stream).await
    }
}

// ============================================================================
//                    TLS STREAMS
// ============================================================================

/// TLS поток для сервера.
pub struct TlsServerStream {
    inner: ServerTlsStream<TcpStream>,
}

impl TlsServerStream {
    /// Читает данные.
    pub async fn read(&mut self, buf: &mut [u8]) -> NetResult<usize> {
        use tokio::io::AsyncReadExt;
        Ok(self.inner.read(buf).await?)
    }

    /// Записывает данные.
    pub async fn write(&mut self, buf: &[u8]) -> NetResult<usize> {
        use tokio::io::AsyncWriteExt;
        Ok(self.inner.write(buf).await?)
    }

    /// Записывает все данные.
    pub async fn write_all(&mut self, buf: &[u8]) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.write_all(buf).await?;
        Ok(())
    }

    /// Сбрасывает буферы.
    pub async fn flush(&mut self) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.flush().await?;
        Ok(())
    }

    /// Закрывает соединение.
    pub async fn shutdown(&mut self) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.shutdown().await?;
        Ok(())
    }

    /// Возвращает внутренний поток.
    pub fn into_inner(self) -> ServerTlsStream<TcpStream> {
        self.inner
    }

    /// Возвращает ссылку на внутренний поток.
    pub fn get_ref(&self) -> &ServerTlsStream<TcpStream> {
        &self.inner
    }
}

/// TLS поток для клиента.
pub struct TlsClientStream {
    inner: ClientTlsStream<TcpStream>,
}

impl TlsClientStream {
    /// Читает данные.
    pub async fn read(&mut self, buf: &mut [u8]) -> NetResult<usize> {
        use tokio::io::AsyncReadExt;
        Ok(self.inner.read(buf).await?)
    }

    /// Записывает данные.
    pub async fn write(&mut self, buf: &[u8]) -> NetResult<usize> {
        use tokio::io::AsyncWriteExt;
        Ok(self.inner.write(buf).await?)
    }

    /// Записывает все данные.
    pub async fn write_all(&mut self, buf: &[u8]) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.write_all(buf).await?;
        Ok(())
    }

    /// Сбрасывает буферы.
    pub async fn flush(&mut self) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.flush().await?;
        Ok(())
    }

    /// Закрывает соединение.
    pub async fn shutdown(&mut self) -> NetResult<()> {
        use tokio::io::AsyncWriteExt;
        self.inner.shutdown().await?;
        Ok(())
    }

    /// Возвращает внутренний поток.
    pub fn into_inner(self) -> ClientTlsStream<TcpStream> {
        self.inner
    }

    /// Возвращает ссылку на внутренний поток.
    pub fn get_ref(&self) -> &ClientTlsStream<TcpStream> {
        &self.inner
    }
}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tls_config_builder() {
        let config = TlsConfig::client()
            .with_native_roots()
            .with_alpn(vec!["h2", "http/1.1"])
            .with_min_version(TlsVersion::Tls12)
            .build();

        assert!(config.has_root_certs());
        assert_eq!(config.alpn_protocols.len(), 2);
    }

    #[test]
    fn test_tls_connector_creation() {
        let config = TlsConfig::client()
            .with_native_roots()
            .build();
        
        let connector = TlsConnector::new(config);
        assert!(connector.is_ok());
    }
}
