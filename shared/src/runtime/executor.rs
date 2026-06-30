// ============================================================================
//                    ИСПОЛНИТЕЛЬ ЗАДАЧ
// ============================================================================
//
// Предоставляет управление async задачами:
// - TaskExecutor: управление пулом задач
// - Task: представление отдельной задачи
// - TaskHandle: хэндл для управления запущенной задачей
// - Scheduler: планировщик отложенных и периодических задач
//
// ============================================================================

use std::collections::HashMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::time::{Duration, Instant};
use tokio::sync::{Notify, RwLock};
use tokio::task::JoinHandle;

use crate::types::Value;

/// Boxed future type used by the scheduler.
type TaskFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

// ============================================================================
//                    ИДЕНТИФИКАТОРЫ
// ============================================================================

/// Уникальный идентификатор задачи.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct TaskId(pub u64);

impl TaskId {
    /// Генерирует новый уникальный ID.
    pub fn new() -> Self {
        static COUNTER: AtomicU64 = AtomicU64::new(1);
        Self(COUNTER.fetch_add(1, Ordering::SeqCst))
    }
}

impl Default for TaskId {
    fn default() -> Self {
        Self::new()
    }
}

impl std::fmt::Display for TaskId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Task#{}", self.0)
    }
}

// ============================================================================
//                    СОСТОЯНИЕ ЗАДАЧИ
// ============================================================================

/// Состояние задачи.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TaskState {
    /// Ожидает запуска
    Pending,
    /// Выполняется
    Running,
    /// Приостановлена
    Paused,
    /// Успешно завершена
    Completed,
    /// Отменена
    Cancelled,
    /// Завершилась с ошибкой
    Failed,
}

impl TaskState {
    pub fn is_finished(&self) -> bool {
        matches!(
            self,
            TaskState::Completed | TaskState::Cancelled | TaskState::Failed
        )
    }

    pub fn is_active(&self) -> bool {
        matches!(self, TaskState::Running | TaskState::Paused)
    }
}

// ============================================================================
//                    МЕТАДАННЫЕ ЗАДАЧИ
// ============================================================================

/// Метаданные задачи.
#[derive(Debug, Clone)]
pub struct TaskMetadata {
    /// Имя задачи (для отладки)
    pub name: String,
    /// Время создания
    pub created_at: Instant,
    /// Время начала выполнения
    pub started_at: Option<Instant>,
    /// Время завершения
    pub finished_at: Option<Instant>,
    /// Приоритет (выше = важнее)
    pub priority: u8,
    /// Теги для группировки
    pub tags: Vec<String>,
}

impl TaskMetadata {
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            created_at: Instant::now(),
            started_at: None,
            finished_at: None,
            priority: 5,
            tags: Vec::new(),
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.priority = priority;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.tags.push(tag.into());
        self
    }

    /// Возвращает время выполнения (если завершена).
    pub fn duration(&self) -> Option<Duration> {
        match (self.started_at, self.finished_at) {
            (Some(start), Some(end)) => Some(end.duration_since(start)),
            _ => None,
        }
    }
}

impl Default for TaskMetadata {
    fn default() -> Self {
        Self::new("unnamed")
    }
}

// ============================================================================
//                    РЕЗУЛЬТАТ ЗАДАЧИ
// ============================================================================

/// Результат выполнения задачи.
#[derive(Debug, Clone)]
pub enum TaskResult {
    /// Успешное завершение с результатом
    Success(Value),
    /// Ошибка выполнения
    Error(String),
    /// Задача была отменена
    Cancelled,
    /// Задача ещё не завершена
    Pending,
}

impl TaskResult {
    pub fn is_success(&self) -> bool {
        matches!(self, TaskResult::Success(_))
    }

    pub fn is_error(&self) -> bool {
        matches!(self, TaskResult::Error(_))
    }

    pub fn unwrap(self) -> Value {
        match self {
            TaskResult::Success(v) => v,
            TaskResult::Error(e) => panic!("TaskResult::unwrap() на Error: {}", e),
            TaskResult::Cancelled => panic!("TaskResult::unwrap() на Cancelled"),
            TaskResult::Pending => panic!("TaskResult::unwrap() на Pending"),
        }
    }

    pub fn unwrap_or(self, default: Value) -> Value {
        match self {
            TaskResult::Success(v) => v,
            _ => default,
        }
    }
}

// ============================================================================
//                    ХЭНДЛ ЗАДАЧИ
// ============================================================================

/// Хэндл для управления запущенной задачей.
pub struct TaskHandle<T> {
    id: TaskId,
    state: Arc<RwLock<TaskState>>,
    cancel_flag: Arc<AtomicBool>,
    pause_notify: Arc<Notify>,
    join_handle: Option<JoinHandle<T>>,
}

impl<T> TaskHandle<T> {
    /// Идентификатор задачи.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Текущее состояние.
    pub async fn state(&self) -> TaskState {
        *self.state.read().await
    }

    /// Отменяет задачу.
    pub fn cancel(&self) {
        self.cancel_flag.store(true, Ordering::SeqCst);
    }

    /// Проверяет, была ли задача отменена.
    pub fn is_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// Приостанавливает задачу.
    pub async fn pause(&self) {
        let mut state = self.state.write().await;
        if *state == TaskState::Running {
            *state = TaskState::Paused;
        }
    }

    /// Возобновляет задачу.
    pub async fn resume(&self) {
        let mut state = self.state.write().await;
        if *state == TaskState::Paused {
            *state = TaskState::Running;
            self.pause_notify.notify_one();
        }
    }

    /// Ожидает завершения задачи.
    pub async fn join(mut self) -> Option<T> {
        if let Some(handle) = self.join_handle.take() {
            handle.await.ok()
        } else {
            None
        }
    }

    /// Прерывает задачу принудительно.
    pub fn abort(&self) {
        self.cancel();
    }
}

// ============================================================================
//                    ЗАДАЧА
// ============================================================================

/// Обёртка для создания задачи.
pub struct Task {
    id: TaskId,
    metadata: TaskMetadata,
    state: Arc<RwLock<TaskState>>,
    cancel_flag: Arc<AtomicBool>,
    pause_notify: Arc<Notify>,
}

impl Task {
    /// Создаёт новую задачу.
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            id: TaskId::new(),
            metadata: TaskMetadata::new(name),
            state: Arc::new(RwLock::new(TaskState::Pending)),
            cancel_flag: Arc::new(AtomicBool::new(false)),
            pause_notify: Arc::new(Notify::new()),
        }
    }

    pub fn with_priority(mut self, priority: u8) -> Self {
        self.metadata.priority = priority;
        self
    }

    pub fn with_tag(mut self, tag: impl Into<String>) -> Self {
        self.metadata.tags.push(tag.into());
        self
    }

    /// Идентификатор.
    pub fn id(&self) -> TaskId {
        self.id
    }

    /// Метаданные.
    pub fn metadata(&self) -> &TaskMetadata {
        &self.metadata
    }

    /// Проверяет флаг отмены (вызывается из задачи).
    pub fn check_cancelled(&self) -> bool {
        self.cancel_flag.load(Ordering::SeqCst)
    }

    /// Ожидает возобновления если приостановлена.
    pub async fn wait_if_paused(&self) {
        loop {
            let state = *self.state.read().await;
            if state != TaskState::Paused {
                break;
            }
            self.pause_notify.notified().await;
        }
    }

    /// Запускает задачу с future.
    pub fn spawn<F, T>(self, future: F) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let state = Arc::clone(&self.state);
        let cancel_flag = Arc::clone(&self.cancel_flag);
        let pause_notify = Arc::clone(&self.pause_notify);

        // Устанавливаем состояние Running
        let state_clone = Arc::clone(&state);
        let handle = tokio::spawn(async move {
            {
                let mut s = state_clone.write().await;
                *s = TaskState::Running;
            }
            let result = future.await;
            {
                let mut s = state_clone.write().await;
                *s = TaskState::Completed;
            }
            result
        });

        TaskHandle {
            id: self.id,
            state,
            cancel_flag,
            pause_notify,
            join_handle: Some(handle),
        }
    }
}

// ============================================================================
//                    ИСПОЛНИТЕЛЬ ЗАДАЧ
// ============================================================================

/// Информация о задаче в исполнителе.
struct TaskInfo {
    metadata: TaskMetadata,
    state: Arc<RwLock<TaskState>>,
    cancel_flag: Arc<AtomicBool>,
}

/// Исполнитель и менеджер задач.
pub struct TaskExecutor {
    /// Зарегистрированные задачи
    tasks: RwLock<HashMap<TaskId, TaskInfo>>,
    /// Счётчик активных задач
    active_count: AtomicU64,
    /// Максимальное количество параллельных задач
    max_concurrent: AtomicU64,
}

impl TaskExecutor {
    /// Создаёт новый исполнитель.
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            active_count: AtomicU64::new(0),
            max_concurrent: AtomicU64::new(100),
        }
    }

    /// Устанавливает лимит параллельных задач.
    pub fn set_max_concurrent(&self, max: u64) {
        self.max_concurrent.store(max, Ordering::SeqCst);
    }

    /// Количество активных задач.
    pub fn active_count(&self) -> u64 {
        self.active_count.load(Ordering::SeqCst)
    }

    /// Запускает задачу.
    pub async fn spawn<F, T>(&self, name: impl Into<String>, future: F) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = Task::new(name);
        let id = task.id();

        // Регистрируем задачу
        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(
                id,
                TaskInfo {
                    metadata: task.metadata().clone(),
                    state: Arc::clone(&task.state),
                    cancel_flag: Arc::clone(&task.cancel_flag),
                },
            );
        }

        self.active_count.fetch_add(1, Ordering::SeqCst);

        // Оборачиваем future чтобы уменьшить счётчик при завершении
        let active_count = &self.active_count as *const AtomicU64 as usize;
        let wrapped = async move {
            let result = future.await;
            // SAFETY: executor живёт дольше задачи
            unsafe {
                let counter = &*(active_count as *const AtomicU64);
                counter.fetch_sub(1, Ordering::SeqCst);
            }
            result
        };

        task.spawn(wrapped)
    }

    /// Запускает задачу с приоритетом.
    pub async fn spawn_with_priority<F, T>(
        &self,
        name: impl Into<String>,
        priority: u8,
        future: F,
    ) -> TaskHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let task = Task::new(name).with_priority(priority);
        let id = task.id();

        {
            let mut tasks = self.tasks.write().await;
            tasks.insert(
                id,
                TaskInfo {
                    metadata: task.metadata().clone(),
                    state: Arc::clone(&task.state),
                    cancel_flag: Arc::clone(&task.cancel_flag),
                },
            );
        }

        self.active_count.fetch_add(1, Ordering::SeqCst);
        task.spawn(future)
    }

    /// Отменяет задачу по ID.
    pub async fn cancel(&self, id: TaskId) -> bool {
        let tasks = self.tasks.read().await;
        if let Some(info) = tasks.get(&id) {
            info.cancel_flag.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Отменяет все задачи.
    pub async fn cancel_all(&self) {
        let tasks = self.tasks.read().await;
        for info in tasks.values() {
            info.cancel_flag.store(true, Ordering::SeqCst);
        }
    }

    /// Получает состояние задачи.
    pub async fn get_state(&self, id: TaskId) -> Option<TaskState> {
        let tasks = self.tasks.read().await;
        if let Some(info) = tasks.get(&id) {
            Some(*info.state.read().await)
        } else {
            None
        }
    }

    /// Список всех задач с их состояниями.
    pub async fn list_tasks(&self) -> Vec<(TaskId, String, TaskState)> {
        let tasks = self.tasks.read().await;
        let mut result = Vec::new();
        for (id, info) in tasks.iter() {
            let state = *info.state.read().await;
            result.push((*id, info.metadata.name.clone(), state));
        }
        result
    }

    /// Удаляет завершённые задачи из реестра.
    pub async fn cleanup_finished(&self) {
        let mut tasks = self.tasks.write().await;
        let mut to_remove = Vec::new();

        for (id, info) in tasks.iter() {
            let state = *info.state.read().await;
            if state.is_finished() {
                to_remove.push(*id);
            }
        }

        for id in to_remove {
            tasks.remove(&id);
        }
    }
}

impl Default for TaskExecutor {
    fn default() -> Self {
        Self::new()
    }
}

// ============================================================================
//                    ПЛАНИРОВЩИК
// ============================================================================

/// Запланированная задача.
struct ScheduledTask {
    name: String,
    interval: Option<Duration>,
    next_run: Instant,
    task_fn: Box<dyn Fn() -> Pin<Box<dyn Future<Output = ()> + Send>> + Send + Sync>,
    enabled: AtomicBool,
}

/// Планировщик периодических и отложенных задач.
pub struct Scheduler {
    tasks: RwLock<HashMap<String, ScheduledTask>>,
    running: AtomicBool,
}

impl Scheduler {
    /// Создаёт новый планировщик.
    pub fn new() -> Self {
        Self {
            tasks: RwLock::new(HashMap::new()),
            running: AtomicBool::new(false),
        }
    }

    /// Добавляет одноразовую отложенную задачу.
    pub async fn schedule_once<F, Fut>(&self, name: impl Into<String>, delay: Duration, task: F)
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let mut tasks = self.tasks.write().await;

        tasks.insert(
            name.clone(),
            ScheduledTask {
                name,
                interval: None,
                next_run: Instant::now() + delay,
                task_fn: Box::new(move || Box::pin(task())),
                enabled: AtomicBool::new(true),
            },
        );
    }

    /// Добавляет периодическую задачу.
    pub async fn schedule_interval<F, Fut>(
        &self,
        name: impl Into<String>,
        interval: Duration,
        task: F,
    ) where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = ()> + Send + 'static,
    {
        let name = name.into();
        let mut tasks = self.tasks.write().await;

        tasks.insert(
            name.clone(),
            ScheduledTask {
                name,
                interval: Some(interval),
                next_run: Instant::now() + interval,
                task_fn: Box::new(move || Box::pin(task())),
                enabled: AtomicBool::new(true),
            },
        );
    }

    /// Удаляет запланированную задачу.
    pub async fn unschedule(&self, name: &str) -> bool {
        self.tasks.write().await.remove(name).is_some()
    }

    /// Приостанавливает задачу.
    pub async fn disable(&self, name: &str) -> bool {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(name) {
            task.enabled.store(false, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Возобновляет задачу.
    pub async fn enable(&self, name: &str) -> bool {
        let tasks = self.tasks.read().await;
        if let Some(task) = tasks.get(name) {
            task.enabled.store(true, Ordering::SeqCst);
            true
        } else {
            false
        }
    }

    /// Запускает цикл планировщика.
    pub async fn run(&self) {
        self.running.store(true, Ordering::SeqCst);

        while self.running.load(Ordering::SeqCst) {
            let now = Instant::now();
            let mut to_run: Vec<(String, TaskFuture)> = Vec::new();
            let mut to_remove: Vec<String> = Vec::new();

            // Собираем задачи для выполнения
            {
                let mut tasks = self.tasks.write().await;
                for (name, task) in tasks.iter_mut() {
                    if !task.enabled.load(Ordering::SeqCst) {
                        continue;
                    }

                    if now >= task.next_run {
                        to_run.push((name.clone(), (task.task_fn)()));

                        if let Some(interval) = task.interval {
                            task.next_run = now + interval;
                        } else {
                            to_remove.push(name.clone());
                        }
                    }
                }

                // Удаляем одноразовые задачи
                for name in to_remove {
                    tasks.remove(&name);
                }
            }

            // Выполняем задачи
            for (_, future) in to_run {
                tokio::spawn(future);
            }

            // Небольшая пауза
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    }

    /// Останавливает планировщик.
    pub fn stop(&self) {
        self.running.store(false, Ordering::SeqCst);
    }

    /// Проверяет, запущен ли планировщик.
    pub fn is_running(&self) -> bool {
        self.running.load(Ordering::SeqCst)
    }
}

impl Default for Scheduler {
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
    fn test_task_id_unique() {
        let id1 = TaskId::new();
        let id2 = TaskId::new();
        assert_ne!(id1, id2);
    }

    #[tokio::test]
    async fn test_task_spawn() {
        let task = Task::new("test_task");
        let id = task.id();

        let handle = task.spawn(async {
            tokio::time::sleep(Duration::from_millis(10)).await;
            42
        });

        assert_eq!(handle.id(), id);
        let result = handle.join().await;
        assert_eq!(result, Some(42));
    }

    #[tokio::test]
    async fn test_task_cancel() {
        let task = Task::new("cancellable");
        let check = Arc::clone(&task.cancel_flag);

        let handle = task.spawn(async move {
            loop {
                if check.load(Ordering::SeqCst) {
                    return "cancelled";
                }
                tokio::time::sleep(Duration::from_millis(1)).await;
            }
        });

        handle.cancel();
        let result = handle.join().await;
        assert_eq!(result, Some("cancelled"));
    }

    #[tokio::test]
    async fn test_executor_spawn() {
        let executor = TaskExecutor::new();

        let handle = executor
            .spawn("test", async {
                tokio::time::sleep(Duration::from_millis(5)).await;
                "done"
            })
            .await;

        let result = handle.join().await;
        assert_eq!(result, Some("done"));
    }

    #[tokio::test]
    async fn test_executor_list_tasks() {
        let executor = TaskExecutor::new();

        let _h1 = executor
            .spawn("task1", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
            .await;

        let _h2 = executor
            .spawn("task2", async {
                tokio::time::sleep(Duration::from_millis(100)).await;
            })
            .await;

        let tasks = executor.list_tasks().await;
        assert_eq!(tasks.len(), 2);
    }
}
