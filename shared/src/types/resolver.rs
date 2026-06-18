//! Резолвер зависимостей для Kumir 3
//!
//! Отвечает за:
//! - Разрешение версий зависимостей
//! - Построение графа зависимостей
//! - Обнаружение конфликтов
//! - Создание плана установки

use std::collections::{HashMap, HashSet, VecDeque};

use super::config::{DependencySpec, LockEntry, LockFile};
use super::environment::{LibrarySource, VersionedLibrary, VirtualEnvironment};
use super::library::LibraryDef;
use super::version::{Version, VersionSpec};

// ============================================================================
//                         ГРАФ ЗАВИСИМОСТЕЙ
// ============================================================================

/// Узел в графе зависимостей
#[derive(Debug, Clone)]
pub struct DependencyNode {
    /// Имя библиотеки
    pub name: String,
    /// Разрешённая версия
    pub version: Option<Version>,
    /// Запрошенная спецификация
    pub requested: VersionSpec,
    /// Зависимости этого узла
    pub dependencies: Vec<String>,
    /// Кто запросил эту зависимость
    pub requested_by: Vec<String>,
    /// Источник
    pub source: Option<LibrarySource>,
    /// Статус разрешения
    pub status: ResolutionStatus,
}

/// Статус разрешения зависимости
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ResolutionStatus {
    /// Ожидает разрешения
    Pending,
    /// Успешно разрешено
    Resolved,
    /// Конфликт версий
    Conflict(Vec<VersionConflict>),
    /// Не найдено
    NotFound,
    /// Циклическая зависимость
    Cyclic,
}

/// Конфликт версий
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct VersionConflict {
    /// Кто запросил
    pub requester: String,
    /// Что запросил
    pub requested: VersionSpec,
    /// С чем конфликтует
    pub conflicts_with: Version,
}

/// Граф зависимостей
#[derive(Debug, Default)]
pub struct DependencyGraph {
    /// Узлы графа
    nodes: HashMap<String, DependencyNode>,
    /// Корневые зависимости
    roots: HashSet<String>,
}

impl DependencyGraph {
    pub fn new() -> Self {
        Self::default()
    }

    /// Добавляет корневую зависимость
    pub fn add_root(&mut self, spec: &DependencySpec) {
        self.roots.insert(spec.name.clone());
        self.add_dependency(spec, "root");
    }

    /// Добавляет зависимость
    pub fn add_dependency(&mut self, spec: &DependencySpec, requested_by: &str) {
        let node = self
            .nodes
            .entry(spec.name.clone())
            .or_insert_with(|| DependencyNode {
                name: spec.name.clone(),
                version: None,
                requested: spec.version.clone(),
                dependencies: Vec::new(),
                requested_by: Vec::new(),
                source: spec.path.as_ref().map(|p| LibrarySource::Local(p.clone())),
                status: ResolutionStatus::Pending,
            });

        node.requested_by.push(requested_by.to_string());
    }

    /// Возвращает все узлы
    pub fn nodes(&self) -> &HashMap<String, DependencyNode> {
        &self.nodes
    }

    /// Возвращает узел по имени
    pub fn get(&self, name: &str) -> Option<&DependencyNode> {
        self.nodes.get(name)
    }

    /// Возвращает узел по имени (мутабельно)
    pub fn get_mut(&mut self, name: &str) -> Option<&mut DependencyNode> {
        self.nodes.get_mut(name)
    }

    /// Проверяет наличие циклов
    pub fn has_cycle(&self) -> Option<Vec<String>> {
        let mut visited = HashSet::new();
        let mut rec_stack = HashSet::new();
        let mut path = Vec::new();

        for name in self.nodes.keys() {
            if self.detect_cycle(name, &mut visited, &mut rec_stack, &mut path) {
                return Some(path);
            }
        }
        None
    }

    fn detect_cycle(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        rec_stack: &mut HashSet<String>,
        path: &mut Vec<String>,
    ) -> bool {
        if rec_stack.contains(name) {
            path.push(name.to_string());
            return true;
        }
        if visited.contains(name) {
            return false;
        }

        visited.insert(name.to_string());
        rec_stack.insert(name.to_string());
        path.push(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for dep in &node.dependencies {
                if self.detect_cycle(dep, visited, rec_stack, path) {
                    return true;
                }
            }
        }

        path.pop();
        rec_stack.remove(name);
        false
    }

    /// Топологическая сортировка (порядок установки)
    pub fn topological_sort(&self) -> Result<Vec<String>, String> {
        let mut result = Vec::new();
        let mut visited = HashSet::new();
        let mut temp_visited = HashSet::new();

        for name in self.nodes.keys() {
            self.topo_visit(name, &mut visited, &mut temp_visited, &mut result)?;
        }

        Ok(result)
    }

    fn topo_visit(
        &self,
        name: &str,
        visited: &mut HashSet<String>,
        temp_visited: &mut HashSet<String>,
        result: &mut Vec<String>,
    ) -> Result<(), String> {
        if temp_visited.contains(name) {
            return Err(format!("Циклическая зависимость: {}", name));
        }
        if visited.contains(name) {
            return Ok(());
        }

        temp_visited.insert(name.to_string());

        if let Some(node) = self.nodes.get(name) {
            for dep in &node.dependencies {
                self.topo_visit(dep, visited, temp_visited, result)?;
            }
        }

        temp_visited.remove(name);
        visited.insert(name.to_string());
        result.push(name.to_string());

        Ok(())
    }
}

// ============================================================================
//                         РЕЗОЛВЕР
// ============================================================================

/// Стратегия разрешения конфликтов
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConflictStrategy {
    /// Выбрать самую новую версию
    Newest,
    /// Выбрать самую старую совместимую
    Oldest,
    /// Вернуть ошибку
    Fail,
    /// Использовать версию из lock файла
    UseLock,
}

/// Провайдер библиотек (для поиска доступных версий)
pub trait LibraryProvider {
    /// Возвращает все доступные версии библиотеки
    fn available_versions(&self, name: &str) -> Vec<Version>;

    /// Возвращает определение библиотеки для версии
    fn get_library(&self, name: &str, version: &Version) -> Option<LibraryDef>;

    /// Возвращает зависимости библиотеки
    fn get_dependencies(&self, name: &str, version: &Version) -> Vec<DependencySpec>;
}

/// Резолвер зависимостей
pub struct DependencyResolver<'a> {
    /// Провайдер библиотек
    provider: &'a dyn LibraryProvider,
    /// Lock файл (если есть)
    lock_file: Option<&'a LockFile>,
    /// Стратегия разрешения конфликтов
    strategy: ConflictStrategy,
    /// Граф зависимостей
    graph: DependencyGraph,
    /// Разрешённые версии
    resolved: HashMap<String, Version>,
    /// Ошибки
    errors: Vec<ResolutionError>,
}

/// Ошибка разрешения
#[derive(Debug, Clone)]
pub enum ResolutionError {
    /// Библиотека не найдена
    NotFound { name: String },
    /// Нет совместимой версии
    NoCompatibleVersion {
        name: String,
        requested: VersionSpec,
    },
    /// Конфликт версий
    Conflict {
        name: String,
        conflicts: Vec<VersionConflict>,
    },
    /// Циклическая зависимость
    CyclicDependency { cycle: Vec<String> },
}

impl std::fmt::Display for ResolutionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ResolutionError::NotFound { name } => {
                write!(f, "Библиотека '{}' не найдена", name)
            }
            ResolutionError::NoCompatibleVersion { name, requested } => {
                write!(f, "Нет совместимой версии '{}' для {}", name, requested)
            }
            ResolutionError::Conflict { name, conflicts } => {
                write!(f, "Конфликт версий для '{}': ", name)?;
                for c in conflicts {
                    write!(f, "{} требует {}, ", c.requester, c.requested)?;
                }
                Ok(())
            }
            ResolutionError::CyclicDependency { cycle } => {
                write!(f, "Циклическая зависимость: {}", cycle.join(" -> "))
            }
        }
    }
}

impl<'a> DependencyResolver<'a> {
    pub fn new(provider: &'a dyn LibraryProvider) -> Self {
        Self {
            provider,
            lock_file: None,
            strategy: ConflictStrategy::Newest,
            graph: DependencyGraph::new(),
            resolved: HashMap::new(),
            errors: Vec::new(),
        }
    }

    /// Устанавливает lock файл
    pub fn with_lock_file(mut self, lock: &'a LockFile) -> Self {
        self.lock_file = Some(lock);
        self.strategy = ConflictStrategy::UseLock;
        self
    }

    /// Устанавливает стратегию разрешения
    pub fn with_strategy(mut self, strategy: ConflictStrategy) -> Self {
        self.strategy = strategy;
        self
    }

    /// Разрешает зависимости
    pub fn resolve(
        &mut self,
        dependencies: &[DependencySpec],
    ) -> Result<ResolutionResult, Vec<ResolutionError>> {
        // Добавляем корневые зависимости
        for dep in dependencies {
            self.graph.add_root(dep);
        }

        // BFS для разрешения зависимостей
        let mut queue: VecDeque<String> = self.graph.roots.iter().cloned().collect();
        let mut visited = HashSet::new();

        while let Some(name) = queue.pop_front() {
            if visited.contains(&name) {
                continue;
            }
            visited.insert(name.clone());

            // Разрешаем версию
            if let Err(e) = self.resolve_single(&name) {
                self.errors.push(e);
                continue;
            }

            // Добавляем зависимости в очередь
            if let Some(version) = self.resolved.get(&name) {
                let deps = self.provider.get_dependencies(&name, version);
                for dep in deps {
                    self.graph.add_dependency(&dep, &name);
                    if let Some(node) = self.graph.get_mut(&name) {
                        node.dependencies.push(dep.name.clone());
                    }
                    queue.push_back(dep.name);
                }
            }
        }

        // Проверяем циклы
        if let Some(cycle) = self.graph.has_cycle() {
            self.errors
                .push(ResolutionError::CyclicDependency { cycle });
        }

        if !self.errors.is_empty() {
            return Err(self.errors.clone());
        }

        // Формируем результат
        let install_order = self
            .graph
            .topological_sort()
            .map_err(|e| vec![ResolutionError::CyclicDependency { cycle: vec![e] }])?;

        let mut result = ResolutionResult {
            resolved: HashMap::new(),
            install_order,
            lock_entries: Vec::new(),
        };

        for (name, version) in &self.resolved {
            if self.provider.get_library(name, version).is_some() {
                let node = self.graph.get(name);
                result.resolved.insert(
                    name.clone(),
                    ResolvedPackage {
                        name: name.clone(),
                        version: version.clone(),
                        source: node
                            .and_then(|n| n.source.clone())
                            .unwrap_or(LibrarySource::Builtin),
                        dependencies: node.map(|n| n.dependencies.clone()).unwrap_or_default(),
                    },
                );

                result.lock_entries.push(LockEntry {
                    name: name.clone(),
                    version: version.clone(),
                    checksum: None,
                    source: "registry".to_string(),
                    dependencies: self
                        .provider
                        .get_dependencies(name, version)
                        .iter()
                        .map(|d| d.name.clone())
                        .collect(),
                });
            }
        }

        Ok(result)
    }

    fn resolve_single(&mut self, name: &str) -> Result<(), ResolutionError> {
        let node = self
            .graph
            .get(name)
            .ok_or_else(|| ResolutionError::NotFound {
                name: name.to_string(),
            })?;

        let requested = node.requested.clone();

        // Проверяем lock файл
        if self.strategy == ConflictStrategy::UseLock
            && let Some(lock) = self.lock_file
            && let Some(locked_version) = lock.locked_version(name)
            && requested.matches(locked_version)
        {
            self.resolved
                .insert(name.to_string(), locked_version.clone());
            if let Some(node) = self.graph.get_mut(name) {
                node.version = Some(locked_version.clone());
                node.status = ResolutionStatus::Resolved;
            }
            return Ok(());
        }

        // Получаем доступные версии
        let available = self.provider.available_versions(name);
        if available.is_empty() {
            if let Some(node) = self.graph.get_mut(name) {
                node.status = ResolutionStatus::NotFound;
            }
            return Err(ResolutionError::NotFound {
                name: name.to_string(),
            });
        }

        // Фильтруем по спецификации
        let mut matching: Vec<Version> = available
            .into_iter()
            .filter(|v| requested.matches(v))
            .collect();

        if matching.is_empty() {
            if let Some(node) = self.graph.get_mut(name) {
                node.status = ResolutionStatus::NotFound;
            }
            return Err(ResolutionError::NoCompatibleVersion {
                name: name.to_string(),
                requested: requested.clone(),
            });
        }

        // Выбираем версию по стратегии
        matching.sort();
        let version = match self.strategy {
            ConflictStrategy::Newest | ConflictStrategy::UseLock => matching.pop().unwrap(),
            ConflictStrategy::Oldest => matching.remove(0),
            ConflictStrategy::Fail => {
                if matching.len() > 1 {
                    // Есть несколько вариантов - проверяем на конфликты
                    matching.pop().unwrap()
                } else {
                    matching.pop().unwrap()
                }
            }
        };

        // Проверяем конфликт с уже разрешённой версией
        if let Some(existing) = self.resolved.get(name)
            && existing != &version
            && !requested.matches(existing)
        {
            let conflict = VersionConflict {
                requester: "previous resolution".to_string(),
                requested: requested.clone(),
                conflicts_with: existing.clone(),
            };
            if let Some(node) = self.graph.get_mut(name) {
                node.status = ResolutionStatus::Conflict(vec![conflict.clone()]);
            }
            return Err(ResolutionError::Conflict {
                name: name.to_string(),
                conflicts: vec![conflict],
            });
        }

        // Сохраняем разрешённую версию
        self.resolved.insert(name.to_string(), version.clone());
        if let Some(node) = self.graph.get_mut(name) {
            node.version = Some(version);
            node.status = ResolutionStatus::Resolved;
        }

        Ok(())
    }
}

// ============================================================================
//                         РЕЗУЛЬТАТ РАЗРЕШЕНИЯ
// ============================================================================

/// Разрешённый пакет
#[derive(Debug, Clone)]
pub struct ResolvedPackage {
    /// Имя
    pub name: String,
    /// Версия
    pub version: Version,
    /// Источник
    pub source: LibrarySource,
    /// Зависимости
    pub dependencies: Vec<String>,
}

/// Результат разрешения зависимостей
#[derive(Debug)]
pub struct ResolutionResult {
    /// Разрешённые пакеты
    pub resolved: HashMap<String, ResolvedPackage>,
    /// Порядок установки
    pub install_order: Vec<String>,
    /// Записи для lock файла
    pub lock_entries: Vec<LockEntry>,
}

impl ResolutionResult {
    /// Обновляет lock файл
    pub fn update_lock_file(&self, lock: &mut LockFile) {
        for entry in &self.lock_entries {
            lock.update(entry.clone());
        }
    }

    /// Применяет к окружению
    pub fn apply_to_environment(
        &self,
        env: &mut VirtualEnvironment,
        provider: &dyn LibraryProvider,
    ) {
        for name in &self.install_order {
            if let Some(pkg) = self.resolved.get(name)
                && let Some(def) = provider.get_library(name, &pkg.version)
            {
                let versioned = VersionedLibrary {
                    def,
                    version: pkg.version.clone(),
                    source: pkg.source.clone(),
                    path: None,
                    checksum: None,
                };
                env.register(versioned);
            }
        }
    }
}

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    struct MockProvider {
        libraries: HashMap<String, Vec<Version>>,
    }

    impl MockProvider {
        fn new() -> Self {
            let mut libs = HashMap::new();
            libs.insert(
                "Сокеты".to_string(),
                vec![
                    Version::new(1, 0, 0),
                    Version::new(1, 5, 0),
                    Version::new(2, 0, 0),
                ],
            );
            libs.insert("HTTP".to_string(), vec![Version::new(1, 0, 0)]);
            Self { libraries: libs }
        }
    }

    impl LibraryProvider for MockProvider {
        fn available_versions(&self, name: &str) -> Vec<Version> {
            self.libraries.get(name).cloned().unwrap_or_default()
        }

        fn get_library(&self, name: &str, version: &Version) -> Option<LibraryDef> {
            if self.libraries.get(name)?.contains(version) {
                // Для тестов возвращаем заглушку со статическими строками
                match name {
                    "Сокеты" => Some(LibraryDef::new("sockets", "Сокеты")),
                    "HTTP" => Some(LibraryDef::new("http", "HTTP")),
                    _ => None,
                }
            } else {
                None
            }
        }

        fn get_dependencies(&self, _name: &str, _version: &Version) -> Vec<DependencySpec> {
            Vec::new()
        }
    }

    #[test]
    fn test_resolve_simple() {
        let provider = MockProvider::new();
        let mut resolver = DependencyResolver::new(&provider);

        let deps = vec![DependencySpec::version(
            "Сокеты",
            VersionSpec::compatible(Version::new(1, 0, 0)),
        )];

        let result = resolver.resolve(&deps).unwrap();
        assert!(result.resolved.contains_key("Сокеты"));

        // Должна быть выбрана 1.5.0 (самая новая совместимая с ^1.0)
        let pkg = result.resolved.get("Сокеты").unwrap();
        assert_eq!(pkg.version, Version::new(1, 5, 0));
    }

    #[test]
    fn test_not_found() {
        let provider = MockProvider::new();
        let mut resolver = DependencyResolver::new(&provider);

        let deps = vec![DependencySpec::version(
            "НесуществующаяБиблиотека",
            VersionSpec::any(),
        )];

        let result = resolver.resolve(&deps);
        assert!(result.is_err());
    }
}
