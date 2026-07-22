//! Type introspection: mapping a runtime [`Value`] to its static [`TypeKind`].

use super::super::number::Number;
use super::{TypeKind, Value};

// =============================================================================
//         SECTION: TYPE INTROSPECTION
// =============================================================================

impl Value {
    /// Returns the static type descriptor for this value.
    pub fn type_kind(&self) -> TypeKind {
        match self {
            Value::Number(n) => match n {
                Number::I8(_) => TypeKind::Int8,
                Number::I16(_) => TypeKind::Int16,
                Number::I32(_) => TypeKind::Int32,
                Number::I64(_) => TypeKind::Int64,
                Number::I128(_) => TypeKind::Int128,
                Number::U8(_) => TypeKind::UInt8,
                Number::U16(_) => TypeKind::UInt16,
                Number::U32(_) => TypeKind::UInt32,
                Number::U64(_) => TypeKind::UInt64,
                Number::U128(_) => TypeKind::UInt128,
                Number::F32(_) => TypeKind::Float32,
                Number::F64(_) => TypeKind::Float64,
                Number::F128(_) => TypeKind::Float128,
            },
            Value::String(_) => TypeKind::String,
            Value::Boolean(_) => TypeKind::Bool,
            Value::Char(_) => TypeKind::Char,
            Value::Array(arr) => {
                let elem_type = arr.first().map(|v| v.type_kind()).unwrap_or(TypeKind::Any);
                TypeKind::Array(Box::new(elem_type))
            }
            Value::Range { .. } => TypeKind::Range(Box::new(TypeKind::Int64)),
            Value::Bytes(_) => TypeKind::Array(Box::new(TypeKind::UInt8)),
            Value::Pair(a, b) => TypeKind::Pair(Box::new(a.type_kind()), Box::new(b.type_kind())),
            Value::Triple(a, b, c) => TypeKind::Triple(
                Box::new(a.type_kind()),
                Box::new(b.type_kind()),
                Box::new(c.type_kind()),
            ),
            Value::Tuple(items) => TypeKind::Tuple(items.iter().map(|v| v.type_kind()).collect()),
            Value::Set(s) => {
                let elem_type = s.first().map(|v| v.type_kind()).unwrap_or(TypeKind::Any);
                TypeKind::Set(Box::new(elem_type))
            }
            Value::Map(m) => {
                let (k, v) = m
                    .first_key_value()
                    .map(|(k, v)| (k.type_kind(), v.type_kind()))
                    .unwrap_or((TypeKind::Any, TypeKind::Any));
                TypeKind::Map(Box::new(k), Box::new(v))
            }
            Value::Option(inner) => {
                let inner_type = inner
                    .as_ref()
                    .as_ref()
                    .map(|v| v.type_kind())
                    .unwrap_or(TypeKind::Any);
                TypeKind::Option(Box::new(inner_type))
            }
            Value::Result(inner) => {
                let (ok, err) = match inner.as_ref() {
                    Ok(v) => (v.type_kind(), TypeKind::Any),
                    Err(e) => (TypeKind::Any, e.type_kind()),
                };
                TypeKind::Result {
                    ok: Box::new(ok),
                    err: Box::new(err),
                }
            }
            Value::Pointer(inner) => TypeKind::Pointer(Box::new(inner.type_kind())),
            Value::Reference { target, mutable } => TypeKind::Reference {
                inner: Box::new(target.type_kind()),
                mutable: *mutable,
            },
            Value::Enum { name, .. } => TypeKind::Enum(name.clone()),
            Value::Object { type_id, .. } => TypeKind::Object(format!("#{}", type_id.0)),
            Value::NativeObject { type_name, .. } => TypeKind::Native(type_name.clone()),
            Value::Lambda(lambda) => TypeKind::Lambda {
                params: lambda
                    .param_types
                    .iter()
                    .map(|t| t.clone().unwrap_or(TypeKind::Any))
                    .collect(),
                result: lambda.return_type.clone().map(Box::new),
                captures: lambda.captures.keys().cloned().collect(),
            },
            Value::PartialApp { func, .. } => func.type_kind(),
            Value::Promise { .. } => TypeKind::Promise(Box::new(TypeKind::Any)),
            Value::Generator { .. } => TypeKind::Generator {
                yield_type: Box::new(TypeKind::Any),
                return_type: Box::new(TypeKind::Any),
            },
            Value::Channel { .. } => TypeKind::Channel(Box::new(TypeKind::Any)),
            Value::Null => TypeKind::Null,
            Value::Undefined => TypeKind::Undefined,
            Value::Type(_) => TypeKind::Type,
            Value::Error { .. } => TypeKind::Result {
                ok: Box::new(TypeKind::Never),
                err: Box::new(TypeKind::String),
            },
        }
    }

    /// Returns the Russian name of this value's type.
    pub fn type_name_ru(&self) -> String {
        self.type_kind().russian_name()
    }
}
