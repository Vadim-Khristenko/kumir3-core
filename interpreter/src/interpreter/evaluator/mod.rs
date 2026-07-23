//! Expression evaluation for the Kumir 3 interpreter.
//!
//! Implements evaluation of all expression types: literals, variables, binary and unary
//! operations, algorithm calls, array access, OOP (fields, methods, object creation), lambdas, etc.

mod array;
mod binary;
mod call;
mod cast;
mod field;
mod instance;
mod r#match;
mod method;
mod module;
mod pipe;
mod range;
mod string;
mod unary;

use std::collections::{BTreeMap, HashSet};
use std::iter::FromIterator;

use shared::types::{Expr, LambdaValue, TypeKind, Value};

use super::environment::Environment;
use super::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};
use super::ops::TypeOps;

/// Вычислитель выражений.
pub struct ExprEvaluator;

impl ExprEvaluator {
    /// Вычисляет выражение.
    pub fn evaluate(expr: &Expr, env: &mut Environment) -> RuntimeResult<Value> {
        match expr {
            Expr::Literal(value) => Ok(value.clone()),
            Expr::Variable(name) => env.get_variable(name).cloned(),
            Expr::BinaryOp(left, op, right) => Self::eval_binary_op(left, op, right, env),
            Expr::UnaryOp(op, operand) => Self::eval_unary_op(op, operand, env),
            Expr::Call(name, args) => Self::eval_call(name, args, env),
            Expr::ArrayAccess(name, indices) => Self::eval_array_access(name, indices, env),
            Expr::FieldAccess(object, field) => Self::eval_field_access(object, field, env),
            Expr::MethodCall {
                object,
                method,
                args,
            } => Self::eval_method_call(object, method, args, env),

            // [KITE-0002] Safe field access: object?.field
            Expr::SafeField { object, field } => {
                let obj = Self::evaluate(object, env)?;
                if Self::is_null_or_none(&obj) {
                    return Ok(Value::Null);
                }
                Self::eval_field_access(&Expr::Literal(obj), field, env)
            }

            // [KITE-0002] Safe method call: object?.method(args)
            Expr::SafeMethod {
                object,
                method,
                args,
            } => {
                let obj = Self::evaluate(object, env)?;
                if Self::is_null_or_none(&obj) {
                    return Ok(Value::Null);
                }
                Self::eval_method_call(&Expr::Literal(obj), method, args, env)
            }

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

            // [KITE-0011] Super reference (super).
            //
            // Bare `super` is not a value: a Kumir object is a flat record of fields
            // (`Value::Object`), and there is no separate "parent object" in the model,
            // so there's no way to construct a parent value. The only meaningful form
            // is super method call, as indicated by the error below.
            Expr::SuperRef => Err(RuntimeError::new(
                "'предок' можно использовать только как вызов метода предка: \
                 'предок.метод(...)' — самостоятельным значением он не является",
                RuntimeErrorKind::TypeMismatch,
            )),

            // Приведение типа
            Expr::Cast { expr, target_type } => Self::eval_cast(expr, target_type, env),

            // Проверка типа
            Expr::TypeCheck { expr, check_type } => Self::eval_type_check(expr, check_type, env),

            // [KITE-0015] Module member access: `Module::member`
            // (resolved the same way as dotted form `Module.member`).
            Expr::ModuleAccess(module, name) => Self::eval_module_access(module, name, env),
            Expr::EnumConstruct {
                enum_name,
                variant,
                data,
            } => Self::eval_enum_construct(enum_name, variant, data.as_deref(), env),
            Expr::Ref(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }
            Expr::Deref(inner) => {
                let value = Self::evaluate(inner, env)?;
                match value {
                    Value::Pointer(inner) => Ok(*inner),
                    _ => Err(RuntimeError::type_mismatch("указатель", "не указатель")),
                }
            }
            Expr::New(inner) => {
                let value = Self::evaluate(inner, env)?;
                Ok(Value::Pointer(Box::new(value)))
            }
            Expr::Lambda {
                params,
                param_types,
                return_type,
                body,
            } => {
                let mut captures = BTreeMap::new();
                let bound = HashSet::from_iter(params.iter().cloned());
                let free = body.free_vars(&bound);
                for name in free {
                    if let Ok(value) = env.get_variable(&name) {
                        captures.insert(name, value.clone());
                    }
                }
                let param_types = param_types
                    .as_ref()
                    .map(|types| types.iter().map(|t| Some(t.clone())).collect())
                    .unwrap_or_else(|| params.iter().map(|_| None).collect());
                Ok(Value::Lambda(Box::new(LambdaValue {
                    params: params.clone(),
                    param_types,
                    return_type: return_type.clone(),
                    body: body.clone(),
                    captures,
                })))
            }

            // [KITE-0013] Function composition: f >> g  ≡  lambda(x) -> g(f(x))
            Expr::Compose(left, right) => Self::eval_compose(left, right, env),
            Expr::Pipe(value, func) => Self::eval_pipe(value, func, env),
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
            Expr::MatchExpr { expr, arms } => Self::eval_match_expr(expr, arms, env),
            Expr::RustExpr(_code) => Err(RuntimeError::not_implemented("Rust-вставки")),
            Expr::None => Ok(Value::Null),
            Expr::NotImplemented(msg) => {
                let error_msg = msg.as_deref().unwrap_or("не указано");
                Err(RuntimeError::not_implemented(error_msg))
            }
            // [KITE-0002] Null-coalescing operator: a ?? b
            Expr::Coalesce(lhs, rhs) => {
                let left_value = Self::evaluate(lhs, env)?;
                match left_value {
                    Value::Option(opt) => match *opt {
                        None => Self::evaluate(rhs, env),
                        Some(v) => Ok(v),
                    },
                    Value::Null => Self::evaluate(rhs, env),
                    other => Ok(other),
                }
            }
            // [KITE-0002] Array literal `[a, b, c]` → Value::Array.
            // Each element is evaluated by the standard evaluator.
            Expr::ArrayLiteral(elems) => {
                let values = elems
                    .iter()
                    .map(|e| Self::evaluate(e, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::Array(values))
            }
            // [KITE-0002] Tuple literal `(a, b, c)` → Value::Tuple.
            // Each element is evaluated by the standard evaluator.
            Expr::TupleExpr(elems) => {
                let values = elems
                    .iter()
                    .map(|e| Self::evaluate(e, env))
                    .collect::<RuntimeResult<Vec<_>>>()?;
                Ok(Value::Tuple(values))
            }
            // [KITE-0003] Async in expression position—synchronous passthrough.
            // Kumir's asynchrony model is currently simplified/cooperative: `await`
            // and `spawn` simply evaluate the inner expression synchronously and
            // return its value.
            Expr::Await(inner) => Self::evaluate(inner, env),
            Expr::Spawn(inner) => Self::evaluate(inner, env),
            _ => Err(RuntimeError::not_implemented("данное выражение")),
        }
    }

    // =============================================================================
    //         SECTION: HELPER FUNCTIONS
    // =============================================================================

    /// Checks value truthiness (thin delegate to [`TypeOps`]).
    #[inline]
    pub fn is_truthy(value: &Value) -> bool {
        TypeOps::is_truthy(value)
    }

    /// Checks if value is `null` or `Option::None`.
    #[inline]
    fn is_null_or_none(value: &Value) -> bool {
        match value {
            Value::Null => true,
            Value::Option(opt) => opt.is_none(),
            _ => false,
        }
    }

    /// Compares two values for equality (thin delegate to [`TypeOps`]).
    #[inline]
    pub fn values_equal(a: &Value, b: &Value) -> bool {
        TypeOps::values_equal(a, b)
    }

    /// Returns default value for a type (thin delegate to [`TypeOps`]).
    #[inline]
    pub fn default_value_for_type(type_spec: &TypeKind) -> Value {
        TypeOps::default_value(type_spec)
    }
}
