use shared::types::{Number, TypeKind, Value};
use shared::typesys::{Coercion, TypeError, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Casts value to target type (`value as Type`).
    pub fn cast(value: Value, target: &TypeKind) -> RuntimeResult<Value> {
        // [typesys-seam: подключён] Engine conversion plan—before computation.
        let source = value.type_kind();
        let plan = default_engine().coercion(target, &source);
        if plan == Coercion::Forbidden && !Self::cast_extension(target, &source) {
            return Err(Self::cast_forbidden(target, &source));
        }

        Self::convert(value, target, plan)
    }

    /// Conversions defined by language WIDER than engine's structural rules.
    /// For these, engine's negative verdict is not veto.
    ///
    /// List exhaustively repeats KITE 13 § 3.16 "Coercion `as`" table
    /// in rows where source is NOT subtype of target:
    /// * `string` ← any — string representation defined for all values;
    /// * `bool` ← any — truthiness (§ 3.20) defined for all values;
    /// * `float` ← `string` — decimal notation parsing (`"2.5" as float`);
    /// * `char` ← integer (code point) | `string` (single character).
    ///
    /// Everything else engine expresses itself: numeric narrowing/widening
    /// via [`Coercion::Cast`]/[`Coercion::Widen`], `any` via top type,
    /// `Type ← same Type` via [`Coercion::Identity`].
    fn cast_extension(target: &TypeKind, source: &TypeKind) -> bool {
        match target {
            TypeKind::String | TypeKind::Bool => true,
            t if t.is_float() => matches!(source, TypeKind::String),
            TypeKind::Char => {
                matches!(source, TypeKind::String) || (source.is_numeric() && !source.is_float())
            }
            _ => false,
        }
    }

    /// Diagnostic for conversion forbidden by engine.
    ///
    /// Russian type names come from engine error, so target and source are named
    /// exactly as in other type messages.
    fn cast_forbidden(target: &TypeKind, source: &TypeKind) -> RuntimeError {
        let message = match default_engine().check_assignable(target, source) {
            Err(TypeError::NotAssignable { target, source }) => format!(
                "Приведение значения типа '{}' к типу '{}' не определено",
                source, target
            ),
            Err(other) => other.to_string(),
            // Unreachable: only incompatible pairs reach here.
            Ok(_) => format!(
                "Приведение значения типа '{}' к типу '{}' не определено",
                source.russian_name(),
                target.russian_name()
            ),
        };
        RuntimeError::new(message, RuntimeErrorKind::TypeMismatch)
    }

    /// Computation kernel: how to get value of already-approved conversion.
    fn convert(value: Value, target: &TypeKind, plan: Coercion) -> RuntimeResult<Value> {
        match target {
            // `any` (top type): conversion to it is identity; accepts any value
            // and never errors. Everything is subtype of `any`.
            TypeKind::Any => Ok(value),
            TypeKind::Int64 => {
                let n = value
                    .as_int()
                    .ok_or_else(|| RuntimeError::type_mismatch("цел", "не число"))?;
                Ok(Value::Number(Number::I64(n)))
            }
            TypeKind::Float64 => match &value {
                Value::Number(n) => {
                    let f = n
                        .to_f64()
                        .ok_or_else(|| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                Value::String(s) => {
                    let f: f64 = s
                        .parse()
                        .map_err(|_| RuntimeError::type_mismatch("вещ", "не число"))?;
                    Ok(Value::Number(Number::F64(f)))
                }
                _ => Err(RuntimeError::type_mismatch("вещ", "не число")),
            },
            TypeKind::String => Ok(Value::String(value.to_string())),
            TypeKind::Bool => Ok(Value::Boolean(TypeOps::is_truthy(&value))),
            // [W0] `as char`: char ← integer (code) | string of length 1 | char.
            // Everything else—clear error (was not_implemented before).
            TypeKind::Char => Self::cast_to_char(value),
            // `T?`: engine plans wrapping ([`Coercion::Wrap`]); inner value
            // is cast to `T` the same way, `null` gives empty optional.
            TypeKind::Option(inner) => match value {
                Value::Null => Ok(Value::Option(Box::new(None))),
                Value::Option(_) => Ok(value),
                other => Ok(Value::Option(Box::new(Some(Self::cast(other, inner)?)))),
            },
            // Other types kernel does not convert. Identity conversion
            // (`value as ItsType`) is recognized by engine itself—it changes nothing
            // and is therefore allowed.
            _ => {
                if plan == Coercion::Identity {
                    Ok(value)
                } else {
                    Err(RuntimeError::not_implemented(&format!(
                        "приведение к типу {:?}",
                        target
                    )))
                }
            }
        }
    }

    /// [W0] Implementation of `value as char`.
    ///
    /// * `Value::Char(c)` → char itself (identity);
    /// * integer → char with that Unicode code point (scalar value);
    /// * string of exactly one character → that character;
    /// * everything else (including string of other length, float, bool,
    ///   array, object) → clear runtime error.
    fn cast_to_char(value: Value) -> RuntimeResult<Value> {
        match &value {
            Value::Char(c) => Ok(Value::Char(*c)),
            Value::Number(n) if n.is_integer() => {
                let code = n
                    .to_i64()
                    .ok_or_else(|| Self::char_cast_error("код символа вне диапазона"))?;
                let ch = u32::try_from(code)
                    .ok()
                    .and_then(char::from_u32)
                    .ok_or_else(|| {
                        Self::char_cast_error(&format!(
                            "{} не является кодом символа Unicode",
                            code
                        ))
                    })?;
                Ok(Value::Char(ch))
            }
            Value::String(s) => {
                let mut it = s.chars();
                match (it.next(), it.next()) {
                    (Some(c), None) => Ok(Value::Char(c)),
                    _ => Err(Self::char_cast_error(&format!(
                        "строка \"{}\" должна состоять ровно из одного символа",
                        s
                    ))),
                }
            }
            other => Err(Self::char_cast_error(&format!(
                "значение типа {} нельзя привести к символу",
                other.type_name_ru()
            ))),
        }
    }

    /// Unified error message for casting to `char`.
    fn char_cast_error(details: &str) -> RuntimeError {
        RuntimeError::type_mismatch("сим (символ: целое-код или строка длины 1)", details)
    }

    /// Checks if value conforms to given type (`value is Type`).
    ///
    /// The question "is value's type a subtype of checked type" belongs entirely
    /// to engine: subtyping, numeric widening (`float32` is `float`), array elements,
    /// and top type `any` (everything is its subtype, so `value is any` is always true)
    /// are expressed by its rules.
    pub fn type_check(value: &Value, check: &TypeKind) -> bool {
        // [typesys-seam: подключён] Conformance verdict entirely with engine.
        default_engine().is_subtype(&value.type_kind(), check)
    }
}
