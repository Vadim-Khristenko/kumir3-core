// =============================================================================
//        МОДУЛЬ: ФУНКЦИИ ВЫСШЕГО ПОРЯДКА НАД ТАБЛИЦАМИ
// =============================================================================
// Функции этого модуля принимают другую функцию параметром. Остальные
// встроенные функции получают уже вычисленные значения; здесь так нельзя:
// имя алгоритма (`отобразить(a, удвоить)`) переменной не является и при
// вычислении дало бы «переменная не определена». Поэтому аргумент-функция
// остаётся выражением до самого вызова, а вычисляются только данные.

use shared::types::{Expr, Number, Value};

use super::super::environment::Environment;
use super::super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::super::evaluator::ExprEvaluator;
use super::Builtins;

impl Builtins {
    /// Пытается вызвать функцию высшего порядка.
    ///
    /// Проверяется ДО остальных категорий: те начинают с вычисления всех
    /// аргументов, что для аргумента-функции неверно.
    pub(crate) fn try_call_functional(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
    ) -> RuntimeResult<Option<Value>> {
        match name {
            "отобразить" | "map" => Self::hof_map(name, args, env).map(Some),
            "отобрать" | "отфильтровать" | "filter" => {
                Self::hof_filter(name, args, env).map(Some)
            }
            "свернуть" | "fold" | "reduce" => Self::hof_fold(name, args, env).map(Some),
            "любой_из" | "any" => {
                Self::hof_quantifier(name, args, env, Quantifier::Any).map(Some)
            }
            "все_из" | "all" => {
                Self::hof_quantifier(name, args, env, Quantifier::All).map(Some)
            }
            "найти_первый" | "find_first" => {
                Self::hof_find_first(name, args, env).map(Some)
            }
            "количество" | "count" => Self::hof_count(name, args, env).map(Some),
            "сортировать_по" | "sort_by" => {
                Self::hof_sort_by(name, args, env).map(Some)
            }
            _ => Ok(None),
        }
    }

    // -------------------------------------------------------------------------
    //                       ВЫЗОВ АРГУМЕНТА-ФУНКЦИИ
    // -------------------------------------------------------------------------

    /// Применяет аргумент-функцию к уже вычисленным значениям.
    ///
    /// Голое имя (`удвоить`) отдаётся обычному разбору вызова: он сам решает,
    /// переменная это с лямбдой, пользовательский алгоритм или встроенная
    /// функция. Любое другое выражение (`лямбда(x) -> x*2`, результат
    /// композиции `f >> g`) вычисляется и обязано дать лямбду.
    ///
    /// Значения оборачиваются в [`Expr::Literal`], поэтому переиспользуется вся
    /// существующая машинерия вызова — включая проверку числа параметров и
    /// оператор `?` внутри тела.
    fn apply(
        callee: &Expr,
        arg_values: Vec<Value>,
        env: &mut Environment,
        context: &str,
    ) -> RuntimeResult<Value> {
        let arg_exprs: Vec<Expr> = arg_values.into_iter().map(Expr::Literal).collect();

        if let Expr::Variable(name) = callee {
            return ExprEvaluator::evaluate(&Expr::Call(name.clone(), arg_exprs), env);
        }

        match ExprEvaluator::evaluate(callee, env)? {
            Value::Lambda(lambda) => ExprEvaluator::call_lambda(&lambda, &arg_exprs, env),
            other => Err(RuntimeError::new(
                format!(
                    "{context}: вторым аргументом ожидалась функция, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    /// Разбирает вызов вида `функция(таблица, функция)`.
    fn split_table_and_callee<'a>(
        name: &str,
        args: &'a [Expr],
        env: &mut Environment,
    ) -> RuntimeResult<(Vec<Value>, &'a Expr)> {
        if args.len() != 2 {
            return Err(RuntimeError::argument_count(name, 2, args.len()));
        }
        let table = Self::eval_table(name, &args[0], env)?;
        Ok((table, &args[1]))
    }

    /// Вычисляет аргумент, который обязан быть таблицей.
    fn eval_table(name: &str, expr: &Expr, env: &mut Environment) -> RuntimeResult<Vec<Value>> {
        match ExprEvaluator::evaluate(expr, env)? {
            Value::Array(items) => Ok(items),
            other => Err(RuntimeError::new(
                format!(
                    "{name}: первым аргументом ожидалась таблица, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    /// Приводит результат функции-условия к логическому значению.
    ///
    /// Требуется именно `лог`: истинность прочих типов (§ KITE 13 3.20) здесь
    /// молча скрыла бы опечатку вроде `отобрать(a, лямбда(x) -> x)`.
    fn expect_bool(name: &str, value: Value) -> RuntimeResult<bool> {
        match value {
            Value::Boolean(b) => Ok(b),
            other => Err(RuntimeError::new(
                format!(
                    "{name}: функция-условие должна возвращать лог, получено значение типа {}",
                    other.type_name_ru()
                ),
                RuntimeErrorKind::TypeMismatch,
            )),
        }
    }

    // -------------------------------------------------------------------------
    //                             РЕАЛИЗАЦИИ
    // -------------------------------------------------------------------------

    /// `отобразить(таблица, функция)` — таблица результатов функции.
    fn hof_map(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut out = Vec::with_capacity(items.len());
        for item in items {
            out.push(Self::apply(callee, vec![item], env, name)?);
        }
        Ok(Value::Array(out))
    }

    /// `отобрать(таблица, условие)` — элементы, для которых условие истинно.
    fn hof_filter(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut out = Vec::new();
        for item in items {
            let verdict = Self::apply(callee, vec![item.clone()], env, name)?;
            if Self::expect_bool(name, verdict)? {
                out.push(item);
            }
        }
        Ok(Value::Array(out))
    }

    /// `свернуть(таблица, начальное, функция)` — свёртка слева направо.
    ///
    /// Функция получает накопленное значение и очередной элемент. Начальное
    /// значение обязательно: без него свёртка пустой таблицы не определена.
    fn hof_fold(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        if args.len() != 3 {
            return Err(RuntimeError::argument_count(name, 3, args.len()));
        }
        let items = Self::eval_table(name, &args[0], env)?;
        let mut acc = ExprEvaluator::evaluate(&args[1], env)?;
        let callee = &args[2];
        for item in items {
            acc = Self::apply(callee, vec![acc, item], env, name)?;
        }
        Ok(acc)
    }

    /// `любой_из` / `все_из` — кванторы существования и всеобщности.
    ///
    /// Обход прекращается на первом решающем элементе: у `любой_из` — на
    /// истинном, у `все_из` — на ложном.
    fn hof_quantifier(
        name: &str,
        args: &[Expr],
        env: &mut Environment,
        kind: Quantifier,
    ) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        for item in items {
            let verdict = Self::apply(callee, vec![item], env, name)?;
            let verdict = Self::expect_bool(name, verdict)?;
            match kind {
                Quantifier::Any if verdict => return Ok(Value::Boolean(true)),
                Quantifier::All if !verdict => return Ok(Value::Boolean(false)),
                _ => {}
            }
        }
        // Пустая таблица: «любой» ложно, «все» истинно — как в математике.
        Ok(Value::Boolean(matches!(kind, Quantifier::All)))
    }

    /// `найти_первый(таблица, условие)` — необязательное значение.
    ///
    /// Возвращает `некоторое(элемент)` либо `ничего`, а не сам элемент: иначе
    /// «не найдено» было бы неотличимо от найденного `пусто`.
    fn hof_find_first(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        for item in items {
            let verdict = Self::apply(callee, vec![item.clone()], env, name)?;
            if Self::expect_bool(name, verdict)? {
                return Ok(Value::Option(Box::new(Some(item))));
            }
        }
        Ok(Value::Option(Box::new(None)))
    }

    /// `количество(таблица, условие)` — сколько элементов удовлетворяют условию.
    fn hof_count(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;
        let mut n: i64 = 0;
        for item in items {
            let verdict = Self::apply(callee, vec![item], env, name)?;
            if Self::expect_bool(name, verdict)? {
                n += 1;
            }
        }
        Ok(Value::Number(Number::I64(n)))
    }

    /// `сортировать_по(таблица, ключ)` — сортировка по вычисляемому ключу.
    ///
    /// Ключи вычисляются по одному разу на элемент, до сортировки: вызывать
    /// функцию из компаратора значило бы звать её порядка `n·log n` раз.
    /// Порядок устойчивый — элементы с равными ключами сохраняют исходный
    /// порядок следования.
    fn hof_sort_by(name: &str, args: &[Expr], env: &mut Environment) -> RuntimeResult<Value> {
        let (items, callee) = Self::split_table_and_callee(name, args, env)?;

        let mut keyed: Vec<(Value, Value)> = Vec::with_capacity(items.len());
        for item in items {
            let key = Self::apply(callee, vec![item.clone()], env, name)?;
            keyed.push((key, item));
        }

        // Сравнение ключей — те же правила, что и у оператора `<`
        // (KITE 13 § 3.8): несравнимая пара ключей должна быть ошибкой, а не
        // произвольным порядком.
        let mut failure = None;
        keyed.sort_by(|(a, _), (b, _)| {
            match super::super::ops::TypeOps::compare(a, b, |o| o == std::cmp::Ordering::Less) {
                Ok(Value::Boolean(true)) => std::cmp::Ordering::Less,
                Ok(_) => match super::super::ops::TypeOps::compare(b, a, |o| {
                    o == std::cmp::Ordering::Less
                }) {
                    Ok(Value::Boolean(true)) => std::cmp::Ordering::Greater,
                    Ok(_) => std::cmp::Ordering::Equal,
                    Err(e) => {
                        failure.get_or_insert(e);
                        std::cmp::Ordering::Equal
                    }
                },
                Err(e) => {
                    failure.get_or_insert(e);
                    std::cmp::Ordering::Equal
                }
            }
        });
        if let Some(e) = failure {
            return Err(e);
        }

        Ok(Value::Array(keyed.into_iter().map(|(_, v)| v).collect()))
    }
}

/// Какой квантор вычисляет [`Builtins::hof_quantifier`].
#[derive(Clone, Copy)]
enum Quantifier {
    Any,
    All,
}
