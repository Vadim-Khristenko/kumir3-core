# 📜 BetterDoc (BDoc) Standard — kumir3-core

**Document Version:** 1.2.0

**Project:** Kumir3

**Core Principle:** Architecture as Art. Code as Narrative.

---

## 🏗 1. Архитектурное Секционирование (Sectioning)

Для поддержания визуального порядка в `kumir3-core` каждый файл должен быть разделен на смысловые блоки. Это позволяет быстро ориентироваться в коде даже при просмотре "мини-карты" IDE.

Используйте баннеры фиксированной длины (80 символов). Текст внутри баннера — на **английском**:

```rust
// =============================================================================
//         SECTION: [SECTION NAME IN UPPERCASE]
// =============================================================================

```

**Рекомендуемый порядок разделов:**

1. **IMPORTS:** `use` директивы (сначала `std`, затем внешние крейты, затем локальные `crate::`).
2. **CONSTANTS & MACROS:** Публичные и внутренние константы.
3. **TYPES:** Объявления `struct`, `enum`, `union` и `type`.
4. **TRAITS:** Объявления новых типажей.
5. **TRAIT IMPLS:** Реализации типажей для типов модуля.
6. **CORE LOGIC:** Основные `impl` блоки методов.
7. **UNIT TESTS:** Внутренний модуль `mod tests`.

---

## 🏷 2. Статусная Иерархия (Status Tags)

Каждая публичная сущность должна начинаться с метки статуса. Это первый сигнал разработчику о том, насколько можно доверять данному участку кода.

| Метка | Цветовой код | Смысл |
| --- | --- | --- |
| **`[STABLE]`** | 🟢 Green | Код протестирован, документирован и архитектурно завершен. |
| **`[EXPERIMENTAL]`** | 🟡 Yellow | Новая функциональность. Интерфейс может измениться без уведомления. |
| **`[STILL-COOKING]`** | 🟠 Orange | Черновик. Логика не завершена. Не использовать в продакшн-путях. |
| **`[DEPRECATED]`** | 🔴 Red | Устарело. Обязательно указать: `-> Use [Name] instead`. |
| **`[LEGACY-HAIRBALL]`** | 🧶 Hairball | Клубок старого кода. Высокий риск регрессии. Требует рефакторинга. |
| **`[SAFETY-CRITICAL]`** | ⚡ Bolt | Содержит `unsafe`, FFI или критическую работу с памятью. |
| **`[PERF-SENSITIVE]`** | 🚀 Rocket | Горячий путь (hot path). Оптимизация приоритетнее читаемости. |
| **`[PLATFORM-SPECIFIC]`** | 🖥 OS | Поведение зависит от операционной системы. |
| **`[FIXME-SOON]`** | 🩹 Bandage | Временный костыль. Должен быть заменен в ближайшем минорном релизе. |
| **`[MEME-INSIDE]`** | 🐾 Paw | Содержит отсылки к Астольфо, котам или Monster Energy. |

---

## 📚 3. Документирование сущностей

### 3.1. Уровень типа (Structs / Enums)

Документация (doc-comments `///`) пишется на **английском**. Она должна объяснять роль объекта в "экосистеме" дома.

* **Essence:** Что это?
* **Context:** Почему это существует? (Допускается легкий юмор).
* **Resources:** Что нужно для работы (например, "Requires an initialized LLVM context").

### 3.2. Уровень функции (Method documentation)

Используйте Markdown-заголовки второго уровня (`#`) для структурирования:

```rust
/// [STABLE] Analyzes a token and returns an AST node.
///
/// # Arguments
/// * `stream` - Token stream received from the lexer.
///
/// # Returns
/// * `Ok(Node)` - A syntax tree node.
/// * `Err(ParseError)` - Error if syntax does not comply with Kumir canons.
///
/// # Panics
/// * Panics when encountering `Token::Forbidden`, indicating a critical lexer failure.
fn parse_token(...) -> ... {}

```

---

## 🧪 4. Стандарты тестирования

### 4.1. Unit-тесты (Внутренние)

Располагаются в конце каждого `.rs` файла. Имена тестов и комментарии внутри — на английском.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// [STABLE] Verification of parser behavior on an empty expression.
    fn test_parser_should_return_empty_error_on_whitespace_only() {
        // Arrange: Environment setup
        // Act: The action itself
        // Assert: Result verification
    }
}

```

### 4.2. Интеграционные тесты (`tests/*.rs`)

Внешние тесты оформляются как полноценные сценарии. Группируйте тесты баннерами.

```rust
#[test]
/// [STABLE] Full cycle: variable declaration -> assignment -> output.
fn test_integration_variable_lifecycle() {
    let mut runtime = create_house_runtime();
    let code = "алг Тест нач цел х х:=5 вывод х кон";
    
    runtime.run(code).expect("Execution failed");
    assert_eq!(runtime.last_output(), "5");
}

```

### 4.3. Тесты на языке Кумир (`.kum`)

В файлах `.kum` допустимо использование русского языка в комментариях `|`, так как это специфика языка обучения.

```kumir
| КЕЙС: Проверка рекурсивного вызова (Числа Фибоначчи)
| ОЖИДАЕМЫЙ ВЫВОД: 55 (для n=10)
алг цел Фиб(цел n)
нач
  если n <= 1 то знач := n
  иначе знач := Фиб(n-1) + Фиб(n-2)
  все
кон

```

---

## 🎨 5. Эталонный модуль (The House Way Example)

Пример того, как выглядит файл с русским BDoc-стандартом, но английским кодом:

```rust
// =============================================================================
//         SECTION: ALGO EXECUTOR
// =============================================================================

/// [STABLE] Primary bytecode execution engine.
/// 
/// 🐈 Context: Sleeps 90% of the time, but when it wakes up, it works at the speed 
/// of light. Requires 0.5L of white Monster Energy for proper operation.
pub struct AlgoExecutor {
    /// Call Stack
    stack: Vec<Value>,
    /// [SAFETY-CRITICAL] Pointer to the current instruction
    pc: *const u8,
}

impl AlgoExecutor {
    /// [PERF-SENSITIVE] Executes the next instruction in the stream.
    ///
    /// # Arguments
    /// * `instruction` - Opcode to execute.
    ///
    /// # Returns
    /// * `Ok(())` - Instruction executed successfully.
    /// * `Err(RuntimeError)` - Error occurred (division by zero, etc.).
    ///
    /// # Panics
    /// * Panics when attempting to execute a `HALT` instruction without a correct stack termination.
    pub fn step(&mut self, instruction: OpCode) -> Result<(), RuntimeError> {
        // FIXME-SOON: Optimize dispatching via Jump Tables
        match instruction {
            OpCode::Add => {
                // STILL-COOKING: Add overflow checks for integers
                self.perform_add()?;
            }
            _ => todo!("Wait for the magic flow"),
        }
        Ok(())
    }
}

// =============================================================================
//         SECTION: UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// [STABLE] Verification of empty stack initialization.
    fn test_executor_init_stack_is_empty() {
        let exec = AlgoExecutor::new();
        assert!(exec.stack.is_empty());
    }
}

```

---

## 📎 6. Культура именования и TODO

1. **Variables:** Используйте `snake_case`. Если переменная временная — `_tmp_cat`. Если важная — `core_state`.
2. **TODO:** Используйте формат `// TODO(username): description`. Описание — на английском.
3. **Comments:** Если код делает что-то странное, напишите `// NOTE: [Explanation in English]`.

**Kumir3 House: Пиши код так, чтобы его хотелось обнять. Или хотя бы задокументировать.** 🐾🦀
