# Руководство по использованию компилятора Kumir 3

## Обзор

Компилятор Kumir 3 (`kumir3-compiler`) преобразует программы на языке Kumir в исполняемые файлы или промежуточные представления.

## Установка

```bash
# Сборка компилятора
cargo build --release --bin kumir3-compiler

# Компилятор будет доступен по пути:
# target/release/kumir3-compiler.exe (Windows)
# target/release/kumir3-compiler (Linux/macOS)
```

## Основное использование

### Базовая компиляция

```bash
# Компиляция программы в Rust код
kumir3-compiler program.kum --target rust

# Компиляция с выводом промежуточного представления
kumir3-compiler program.kum --target ir

# Проверка синтаксиса без компиляции
kumir3-compiler program.kum --check
```

### Опции командной строки

```
ИСПОЛЬЗОВАНИЕ:
    kumir3-compiler [OPTIONS] <INPUT>

АРГУМЕНТЫ:
    <INPUT>    Входной файл .kum для компиляции

ОПЦИИ:
    -o, --output <FILE>           Выходной файл (по умолчанию: input.rs или input.ir)
    -t, --target <TARGET>         Целевая платформа [возможные: rust, ir, wasm]
    -O, --opt-level <LEVEL>       Уровень оптимизации [0-3] (по умолчанию: 2)
        --debug                   Включить отладочную информацию
        --check                   Только проверить синтаксис, не компилировать
        --emit-ir                 Сохранить промежуточное представление (IR)
        --emit-rust               Сохранить сгенерированный Rust код
    -h, --help                    Показать справку
    -V, --version                 Показать версию
```

## Примеры

### 1. Простая программа

**Файл: hello.kum**
```kumir
алг Главный
нач
    вывод "Привет, мир!"
кон
```

**Компиляция:**
```bash
# Генерация Rust кода
kumir3-compiler hello.kum --target rust --emit-rust

# Результат: hello.rs
```

**Сгенерированный код (hello.rs):**
```rust
// Сгенерировано компилятором Kumir 3
// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>

use std::io::{self, Write};

fn Главный() -> i64 {
    let v0 = "Привет, мир!";
    println!("{:?}", v0);
}

fn main() {
    Главный();
}
```

### 2. Программа с переменными и арифметикой

**Файл: calc.kum**
```kumir
алг Главный
нач
    цел x, y, сумма
    x := 10
    y := 20
    сумма := x + y
    вывод "Сумма: ", сумма
кон
```

**Компиляция:**
```bash
kumir3-compiler calc.kum --target rust --emit-rust -O 2
```

### 3. Программа с циклами

**Файл: loop.kum**
```kumir
алг Главный
нач
    цел i
    нц для i от 1 до 10
        вывод i
    кц
кон
```

**Компиляция:**
```bash
kumir3-compiler loop.kum --target rust --emit-rust
```

### 4. Проверка синтаксиса

```bash
# Только проверить, не компилировать
kumir3-compiler program.kum --check
```

**Вывод при успехе:**
```
✓ Синтаксис корректен
```

**Вывод при ошибке:**
```
✗ Ошибка парсинга: ожидалось 'кон', найдено 'EOF'
  --> program.kum:5:1
```

### 5. Уровни оптимизации

```bash
# Без оптимизаций (быстрая компиляция)
kumir3-compiler program.kum -O 0

# Базовые оптимизации (по умолчанию)
kumir3-compiler program.kum -O 2

# Агрессивные оптимизации
kumir3-compiler program.kum -O 3
```

**Оптимизации:**
- **Уровень 0**: Без оптимизаций
- **Уровень 1**: Свёртка констант (constant folding)
- **Уровень 2**: Свёртка констант + удаление мёртвого кода
- **Уровень 3**: Все оптимизации + агрессивные преобразования

### 6. Отладочный режим

```bash
# Включить отладочную информацию
kumir3-compiler program.kum --debug --emit-ir

# Вывод покажет детали компиляции
```

### 7. Промежуточное представление (IR)

```bash
# Сохранить IR для анализа
kumir3-compiler program.kum --emit-ir

# Результат: program.ir
```

**Пример IR:**
```
module main

function Главный() -> i64 {
  block_0:
    v0 = alloc i64
    v1 = alloc i64
    v2 = alloc i64
    v3 = const 10
    store v0, v3
    v4 = const 20
    store v1, v4
    v5 = load v0
    v6 = load v1
    v7 = add v5, v6
    store v2, v7
    v8 = const "Сумма: "
    call print(v8)
    v9 = load v2
    call print(v9)
}
```

## Целевые платформы

### Rust (rust)
Генерирует Rust код, который можно скомпилировать в нативный исполняемый файл.

```bash
kumir3-compiler program.kum --target rust --emit-rust
rustc program.rs -o program
./program
```

### IR (ir)
Промежуточное представление для анализа и оптимизации.

```bash
kumir3-compiler program.kum --target ir
```

### WASM (wasm)
**Статус:** В разработке

Будет генерировать WebAssembly для запуска в браузере.

## Интеграция с другими инструментами

### Использование с интерпретатором

```bash
# Разработка: быстрое тестирование
interpreter-cli program.kum

# Продакшн: компиляция для производительности
kumir3-compiler program.kum --target rust
rustc program.rs -o program --release
./program
```

### Автоматизация сборки

**Makefile:**
```makefile
all: program

program: program.kum
	kumir3-compiler program.kum --target rust --emit-rust -O 3
	rustc program.rs -o program --release

clean:
	rm -f program program.rs program.ir

run: program
	./program
```

**Использование:**
```bash
make        # Компиляция
make run    # Запуск
make clean  # Очистка
```

## Производительность

### Сравнение режимов

| Режим | Время компиляции | Время выполнения | Размер файла |
|-------|------------------|------------------|--------------|
| Интерпретатор | Мгновенно | Медленно | N/A |
| Компилятор -O 0 | Быстро | Средне | Большой |
| Компилятор -O 2 | Средне | Быстро | Средний |
| Компилятор -O 3 | Медленно | Очень быстро | Маленький |

### Рекомендации

- **Разработка**: Используйте интерпретатор для быстрого тестирования
- **Тестирование**: Компилируйте с `-O 0` для быстрой сборки
- **Продакшн**: Компилируйте с `-O 3` для максимальной производительности

## Ограничения

### Текущие ограничения

1. **WASM backend**: Не реализован
2. **ООП**: Частичная поддержка (классы, интерфейсы)
3. **Async/await**: Не поддерживается в компиляторе
4. **Пользовательские библиотеки**: Работают только в интерпретаторе

### Поддерживаемые конструкции

✅ Базовые типы (цел, вещ, лог, сим, лит)
✅ Переменные и присваивания
✅ Арифметические операции
✅ Логические операции
✅ Условные операторы (если-то-иначе)
✅ Циклы (нц-кц, для, пока)
✅ Ввод/вывод
✅ Функции и алгоритмы
✅ Массивы (базовая поддержка)

## Устранение неполадок

### Ошибка: "Не удалось прочитать файл"

```bash
# Проверьте путь к файлу
ls -la program.kum

# Используйте абсолютный путь
kumir3-compiler /full/path/to/program.kum
```

### Ошибка: "Ошибка парсинга"

```bash
# Проверьте синтаксис
kumir3-compiler program.kum --check

# Используйте интерпретатор для детальной диагностики
interpreter-cli program.kum
```

### Ошибка: "rustc not found"

```bash
# Установите Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Или используйте только IR
kumir3-compiler program.kum --target ir
```

## Дополнительные ресурсы

- **Документация языка**: `docs/LANGUAGE.md`
- **Примеры программ**: `examples/`
- **Система библиотек**: `docs/USER_LIBRARIES.md`
- **Changelog**: `CHANGELOG.md`

## Поддержка

При возникновении проблем:
1. Проверьте синтаксис с `--check`
2. Попробуйте интерпретатор для диагностики
3. Изучите сгенерированный IR с `--emit-ir`
4. Создайте issue на GitHub с примером кода
