//! Модуль вычисления выражений для интерпретатора Кумир 3
//!
//! Реализует вычисление всех типов выражений: литералы, переменные,
//! бинарные и унарные операции, вызовы алгоритмов, доступ к массивам,
//! ООП (поля, методы, создание объектов), лямбды и т.д.

mod array;
mod binary;
mod call;
mod cast;
mod field;
mod instance;
mod r#match;
mod method;
mod pipe;
mod range;
mod string;
mod unary;

use shared::types::{Expr, TypeKind, Value};

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::ops::TypeOps;

/// Вычислитель выражений.
pub struct ExprEvaluator;

impl ExprEvaluator {
    /// Вычисляет выражение.
    pub fn evaluate(expr: &Expr, env: &mut Environment) -> RuntimeResult<Value> {
        match expr {
            // Литералы
            Expr::Literal(value) => Ok(value.clone()),

            // Переменные
            Expr::Variable(name) => env.get_variable(name).cloned(),

            // Бинарные операции
            Expr::BinaryOp(left, op, right) => Self::eval_binary_op(left, op, right, env),

            // Унарные операции
            Expr::UnaryOp(op, operand) => Self::eval_unary_op(op, operand, env),

            // Вызов алгоритма
            Expr::Call(name, args) => Self::eval_call(name, args, env),

            // Доступ к элементу массива
            Expr::ArrayAccess(name, indices) => Self::eval_array_access(name, indices, env),

            // ООП: доступ к полю
            Expr::FieldAccess(object, field) => Self::eval_field_access(object, field, env),

            // ООП: вызов метода
            Expr::MethodCall {
                object,
                method,
                args,
            } => Self::eval_method_call(object, method, args, env),

            // ООП: создание экземпляра
            Expr::NewInstance { class_name, args } => {
                Self::eval_new_instance(class_name, args, env)
            }

            // [KITE 2/0002] Диапазон: 1..10 / 1..=10 [шаг n]
            Expr::Range {
                start,
                end,
                inclusive,
                step,
            } => Self::eval_range(
                start.as_deref(),
                end.as_deref(),
                *inclusive,
                step.as_deref(),
                env,
            ),

            // Ссылка на себя (this)
            Expr::SelfRef => env.get_this().cloned().ok_or_else(|| {
                RuntimeError::new(
                    "Ключевое слово 'это' можно использовать только внутри метода",
                    RuntimeErrorKind::Other,
                )
            }),

            // Ссылка на предка (super)
            Expr::SuperRef => Err(RuntimeError::not_implemented("super (предок)")),

            // Приведение типа
            Expr::Cast { expr, target_type } => Self::eval_cast(expr, target_type, env),

            // Проверка типа
            Expr::TypeCheck { expr, check_type } => Self::eval_type_check(expr, check_type, env),

            // Доступ к модулю
            Expr::ModuleAccess(module, name) => Err(RuntimeError::not_implemented(&format!(
                "доступ к модулю {}::{}",
                module, name
            ))),

            // Создание значения перечисления
            Expr::EnumConstruct {
                enum_name,
                variant,
                data,
            } => Self::eval_enum_construct(enum_name, variant, data.as_deref(), env),

            // Получение ссылки
            Expr::Ref(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }

            // Разыменование
            Expr::Deref(inner) => {
                let value = Self::evaluate(inner, env)?;
                match value {
                    Value::Pointer(inner) => Ok(*inner),
                    _ => Err(RuntimeError::type_mismatch("указатель", "не указатель")),
                }
            }

            // Создание указателя
            Expr::New(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }

            // Лямбда-выражение
            Expr::Lambda {
                params, body: _, ..
            } => {
                // Создаём замыкание (пока упрощённая реализация)
                Ok(Value::String(format!("lambda({:?})", params)))
            }

            // Pipe-выражение: x |> f
            Expr::Pipe(value, func) => Self::eval_pipe(value, func, env),

            // Условное выражение
            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond = Self::evaluate(condition, env)?;
                if Self::is_truthy(&cond) {
                    Self::evaluate(then_expr, env)
                } else {
                    Self::evaluate(else_expr, env)
                }
            }

            // Match-выражение
            Expr::MatchExpr { expr, arms } => Self::eval_match_expr(expr, arms, env),

            // Rust-вставка
            Expr::RustExpr(_code) => Err(RuntimeError::not_implemented("Rust-вставки")),

            // Пусто
            Expr::None => Ok(Value::Null),

            // Не реализовано
            Expr::NotImplemented(msg) => {
                let error_msg = msg.as_deref().unwrap_or("не указано");
                Err(RuntimeError::not_implemented(error_msg))
            }

            // Все остальные выражения (не реализованы)
            _ => Err(RuntimeError::not_implemented("данное выражение")),
        }
    }

    // =========================================================================
    //                    ВСПОМОГАТЕЛЬНЫЕ ФУНКЦИИ
    // =========================================================================

    /// Проверяет "истинность" значения (тонкий делегатор к [`TypeOps`]).
    #[inline]
    pub fn is_truthy(value: &Value) -> bool {
        TypeOps::is_truthy(value)
    }

    /// Сравнивает два значения на равенство (тонкий делегатор к [`TypeOps`]).
    #[inline]
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        TypeOps::values_equal(a, b)
    }

    /// Возвращает значение по умолчанию для типа (тонкий делегатор к [`TypeOps`]).
    #[inline]
    pub fn default_value_for_type(type_spec: &TypeKind) -> Value {
        TypeOps::default_value(type_spec)
    }
}
