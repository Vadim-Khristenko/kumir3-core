// ============================================================================
//                    АСИНХРОННЫЕ КАНАЛЫ
// ============================================================================
//
// Обёртки над tokio каналами для удобной коммуникации:
// - MessageChannel: MPSC канал для сообщений между компонентами
// - BroadcastChannel: Broadcast канал для широковещательных событий
// - OneShotChannel: Одноразовый канал для ответа на запрос
// - ValueChannel: Типизированный канал для Value
//
// ============================================================================

use std::sync::Arc;
use tokio::sync::{mpsc, broadcast, oneshot, Mutex};

use crate::types::Value;

// ============================================================================
//                    СООБЩЕНИЯ
// ============================================================================

/// Сообщение передаваемое через каналы.
#[derive(Debug, Clone)]
pub struct Message {
    /// Тип/имя сообщения
    pub kind: String,
    /// Полезная нагрузка
    pub payload: Value,
    /// Метаданные (опционально)
    pub metadata: Option<MessageMetadata>,
}

/// Метаданные сообщения.
#[derive(Debug, Clone)]
pub struct MessageMetadata {
    /// Источник сообщения
    pub source: Option<String>,
    /// Идентификатор корреляции (для request-response)
    pub correlation_id: Option<u64>,
    /// Временная метка
    pub timestamp: std::time::Instant,
    /// Приоритет
    pub priority: u8,
}

impl Message {
    pub fn new(kind: impl Into<String>, payload: Value) -> Self {
        Self {
            kind: kind.into(),
            payload,
            metadata: None,
        }
    }

    pub fn with_metadata(mut self, metadata: MessageMetadata) -> Self {
        self.metadata = Some(metadata);
        self
    }

    pub fn with_source(mut self, source: impl Into<String>) -> Self {
        let meta = self.metadata.get_or_insert(MessageMetadata::default());
        meta.source = Some(source.into());
        self
    }

    pub fn with_correlation_id(mut self, id: u64) -> Self {
        let meta = self.metadata.get_or_insert(MessageMetadata::default());
        meta.correlation_id = Some(id);
        self
    }
}

impl Default for MessageMetadata {
    fn default() -> Self {
        Self {
            source: None,
            correlation_id: None,
            timestamp: std::time::Instant::now(),
            priority: 0,
        }
    }
}

// ============================================================================
//                    MPSC КАНАЛ
// ============================================================================

/// Отправитель MPSC канала.
#[derive(Clone)]
pub struct MessageSender {
    inner: mpsc::Sender<Message>,
}

impl MessageSender {
    /// Отправляет сообщение (async).
    pub async fn send(&self, msg: Message) -> Result<(), ChannelError> {
        self.inner.send(msg).await
            .map_err(|_| ChannelError::SendFailed)
    }

    /// Пытается отправить сообщение без ожидания.
    pub fn try_send(&self, msg: Message) -> Result<(), ChannelError> {
        self.inner.try_send(msg)
            .map_err(|e| match e {
                mpsc::error::TrySendError::Full(_) => ChannelError::Full,
                mpsc::error::TrySendError::Closed(_) => ChannelError::Closed,
            })
    }

    /// Отправляет простое сообщение.
    pub async fn emit(&self, kind: impl Into<String>, payload: Value) -> Result<(), ChannelError> {
        self.send(Message::new(kind, payload)).await
    }
}

/// Получатель MPSC канала.
pub struct MessageReceiver {
    inner: mpsc::Receiver<Message>,
}

impl MessageReceiver {
    /// Получает сообщение (async, блокирующий).
    pub async fn recv(&mut self) -> Option<Message> {
        self.inner.recv().await
    }

    /// Пытается получить сообщение без ожидания.
    pub fn try_recv(&mut self) -> Result<Message, ChannelError> {
        self.inner.try_recv()
            .map_err(|e| match e {
                mpsc::error::TryRecvError::Empty => ChannelError::Empty,
                mpsc::error::TryRecvError::Disconnected => ChannelError::Closed,
            })
    }

    /// Закрывает канал.
    pub fn close(&mut self) {
        self.inner.close();
    }
}

/// Создаёт MPSC канал для сообщений.
pub fn message_channel(buffer: usize) -> (MessageSender, MessageReceiver) {
    let (tx, rx) = mpsc::channel(buffer);
    (
        MessageSender { inner: tx },
        MessageReceiver { inner: rx },
    )
}

// ============================================================================
//                    BROADCAST КАНАЛ
// ============================================================================

/// Отправитель broadcast канала.
#[derive(Clone)]
pub struct BroadcastSender {
    inner: broadcast::Sender<Message>,
}

impl BroadcastSender {
    /// Отправляет сообщение всем подписчикам.
    pub fn send(&self, msg: Message) -> Result<usize, ChannelError> {
        self.inner.send(msg)
            .map_err(|_| ChannelError::NoReceivers)
    }

    /// Создаёт нового получателя (подписку).
    pub fn subscribe(&self) -> BroadcastReceiver {
        BroadcastReceiver {
            inner: self.inner.subscribe(),
        }
    }

    /// Количество активных подписчиков.
    pub fn receiver_count(&self) -> usize {
        self.inner.receiver_count()
    }
}

/// Получатель broadcast канала.
pub struct BroadcastReceiver {
    inner: broadcast::Receiver<Message>,
}

impl BroadcastReceiver {
    /// Получает сообщение.
    pub async fn recv(&mut self) -> Result<Message, ChannelError> {
        self.inner.recv().await
            .map_err(|e| match e {
                broadcast::error::RecvError::Closed => ChannelError::Closed,
                broadcast::error::RecvError::Lagged(n) => ChannelError::Lagged(n),
            })
    }
}

/// Создаёт broadcast канал.
pub fn broadcast_channel(capacity: usize) -> (BroadcastSender, BroadcastReceiver) {
    let (tx, rx) = broadcast::channel(capacity);
    (
        BroadcastSender { inner: tx },
        BroadcastReceiver { inner: rx },
    )
}

// ============================================================================
//                    ONESHOT КАНАЛ
// ============================================================================

/// Отправитель oneshot канала (одноразовый).
pub struct OneshotSender<T> {
    inner: oneshot::Sender<T>,
}

impl<T> OneshotSender<T> {
    /// Отправляет значение и потребляет отправитель.
    pub fn send(self, value: T) -> Result<(), T> {
        self.inner.send(value)
    }
}

/// Получатель oneshot канала.
pub struct OneshotReceiver<T> {
    inner: oneshot::Receiver<T>,
}

impl<T> OneshotReceiver<T> {
    /// Ожидает значение.
    pub async fn recv(self) -> Result<T, ChannelError> {
        self.inner.await.map_err(|_| ChannelError::Closed)
    }

    /// Пытается получить значение без ожидания.
    pub fn try_recv(&mut self) -> Result<T, ChannelError> {
        self.inner.try_recv()
            .map_err(|e| match e {
                oneshot::error::TryRecvError::Empty => ChannelError::Empty,
                oneshot::error::TryRecvError::Closed => ChannelError::Closed,
            })
    }
}

/// Создаёт oneshot канал.
pub fn oneshot_channel<T>() -> (OneshotSender<T>, OneshotReceiver<T>) {
    let (tx, rx) = oneshot::channel();
    (
        OneshotSender { inner: tx },
        OneshotReceiver { inner: rx },
    )
}

// ============================================================================
//                    VALUE CHANNEL (типизированный для Value)
// ============================================================================

/// Типизированный канал для передачи Value.
pub struct ValueChannel {
    sender: mpsc::Sender<Value>,
    receiver: Arc<Mutex<mpsc::Receiver<Value>>>,
}

impl ValueChannel {
    /// Создаёт новый канал с заданным буфером.
    pub fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
        }
    }

    /// Отправляет значение.
    pub async fn send(&self, value: Value) -> Result<(), ChannelError> {
        self.sender.send(value).await
            .map_err(|_| ChannelError::SendFailed)
    }

    /// Получает значение.
    pub async fn recv(&self) -> Option<Value> {
        self.receiver.lock().await.recv().await
    }

    /// Получает клон отправителя.
    pub fn sender(&self) -> mpsc::Sender<Value> {
        self.sender.clone()
    }
}

impl Clone for ValueChannel {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: Arc::clone(&self.receiver),
        }
    }
}

// ============================================================================
//                    REQUEST-RESPONSE КАНАЛ
// ============================================================================

/// Запрос с возможностью ответа.
pub struct Request<Req, Resp> {
    pub data: Req,
    response_tx: oneshot::Sender<Resp>,
}

impl<Req, Resp> Request<Req, Resp> {
    /// Отправляет ответ на запрос.
    pub fn respond(self, response: Resp) -> Result<(), Resp> {
        self.response_tx.send(response)
    }
}

/// Канал для request-response паттерна.
pub struct RequestChannel<Req, Resp> {
    sender: mpsc::Sender<Request<Req, Resp>>,
    receiver: Arc<Mutex<mpsc::Receiver<Request<Req, Resp>>>>,
}

impl<Req: Send + 'static, Resp: Send + 'static> RequestChannel<Req, Resp> {
    /// Создаёт новый request-response канал.
    pub fn new(buffer: usize) -> Self {
        let (tx, rx) = mpsc::channel(buffer);
        Self {
            sender: tx,
            receiver: Arc::new(Mutex::new(rx)),
        }
    }

    /// Отправляет запрос и ожидает ответ.
    pub async fn request(&self, data: Req) -> Result<Resp, ChannelError> {
        let (resp_tx, resp_rx) = oneshot::channel();
        
        self.sender.send(Request {
            data,
            response_tx: resp_tx,
        }).await.map_err(|_| ChannelError::SendFailed)?;

        resp_rx.await.map_err(|_| ChannelError::Closed)
    }

    /// Получает следующий запрос для обработки.
    pub async fn recv(&self) -> Option<Request<Req, Resp>> {
        self.receiver.lock().await.recv().await
    }

    /// Клонирует отправителя.
    pub fn sender(&self) -> mpsc::Sender<Request<Req, Resp>> {
        self.sender.clone()
    }
}

impl<Req, Resp> Clone for RequestChannel<Req, Resp> {
    fn clone(&self) -> Self {
        Self {
            sender: self.sender.clone(),
            receiver: Arc::clone(&self.receiver),
        }
    }
}

// ============================================================================
//                    ОШИБКИ КАНАЛОВ
// ============================================================================

/// Ошибки работы с каналами.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    /// Канал закрыт
    Closed,
    /// Буфер канала заполнен
    Full,
    /// Канал пуст
    Empty,
    /// Ошибка отправки
    SendFailed,
    /// Нет получателей
    NoReceivers,
    /// Получатель отстал от отправителя
    Lagged(u64),
}

impl std::fmt::Display for ChannelError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ChannelError::Closed => write!(f, "Канал закрыт"),
            ChannelError::Full => write!(f, "Буфер канала заполнен"),
            ChannelError::Empty => write!(f, "Канал пуст"),
            ChannelError::SendFailed => write!(f, "Ошибка отправки"),
            ChannelError::NoReceivers => write!(f, "Нет получателей"),
            ChannelError::Lagged(n) => write!(f, "Пропущено {} сообщений", n),
        }
    }
}

impl std::error::Error for ChannelError {}

// ============================================================================
//                    ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_message_channel() {
        let (tx, mut rx) = message_channel(10);

        tx.emit("test", Value::String("hello".into())).await.unwrap();
        
        let msg = rx.recv().await.unwrap();
        assert_eq!(msg.kind, "test");
        match msg.payload {
            Value::String(s) => assert_eq!(s, "hello"),
            _ => panic!("Expected string"),
        }
    }

    #[tokio::test]
    async fn test_broadcast_channel() {
        let (tx, mut rx1) = broadcast_channel(10);
        let mut rx2 = tx.subscribe();

        tx.send(Message::new("event", Value::Number(42.into()))).unwrap();

        let msg1 = rx1.recv().await.unwrap();
        let msg2 = rx2.recv().await.unwrap();

        assert_eq!(msg1.kind, msg2.kind);
    }

    #[tokio::test]
    async fn test_oneshot_channel() {
        let (tx, rx) = oneshot_channel::<i32>();
        
        tx.send(42).unwrap();
        let value = rx.recv().await.unwrap();
        
        assert_eq!(value, 42);
    }

    #[tokio::test]
    async fn test_request_response() {
        let channel: RequestChannel<String, String> = RequestChannel::new(10);
        let channel_clone = channel.clone();

        // Обработчик запросов
        tokio::spawn(async move {
            while let Some(req) = channel_clone.recv().await {
                let response = format!("Hello, {}!", req.data);
                req.respond(response).ok();
            }
        });

        // Клиент
        let response = channel.request("World".into()).await.unwrap();
        assert_eq!(response, "Hello, World!");
    }

    #[tokio::test]
    async fn test_value_channel() {
        let channel = ValueChannel::new(10);
        
        channel.send(Value::Boolean(true)).await.unwrap();
        
        let value = channel.recv().await.unwrap();
        match value {
            Value::Boolean(b) => assert!(b),
            _ => panic!("Expected boolean"),
        }
    }
}
