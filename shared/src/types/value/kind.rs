//! Methods and `Display` for the static type descriptor [`TypeKind`].

use std::fmt;

use super::TypeKind;

// =============================================================================
//         SECTION: TYPE KIND METHODS
// =============================================================================

impl TypeKind {
    // -------------------------------------------------------------------------
    // Default Type Constants
    // -------------------------------------------------------------------------

    pub const INT: TypeKind = TypeKind::Int64;
    pub const UINT: TypeKind = TypeKind::UInt64;
    pub const FLOAT: TypeKind = TypeKind::Float64;

    // -------------------------------------------------------------------------
    // Constructors
    // -------------------------------------------------------------------------

    pub fn array(elem: TypeKind) -> Self {
        TypeKind::Array(Box::new(elem))
    }
    pub fn range(elem: TypeKind) -> Self {
        TypeKind::Range(Box::new(elem))
    }
    pub fn pair(a: TypeKind, b: TypeKind) -> Self {
        TypeKind::Pair(Box::new(a), Box::new(b))
    }
    pub fn triple(a: TypeKind, b: TypeKind, c: TypeKind) -> Self {
        TypeKind::Triple(Box::new(a), Box::new(b), Box::new(c))
    }
    pub fn tuple(elements: Vec<TypeKind>) -> Self {
        TypeKind::Tuple(elements)
    }
    pub fn set(elem: TypeKind) -> Self {
        TypeKind::Set(Box::new(elem))
    }
    pub fn map(key: TypeKind, value: TypeKind) -> Self {
        TypeKind::Map(Box::new(key), Box::new(value))
    }
    pub fn option(inner: TypeKind) -> Self {
        TypeKind::Option(Box::new(inner))
    }
    pub fn result(ok: TypeKind, err: TypeKind) -> Self {
        TypeKind::Result {
            ok: Box::new(ok),
            err: Box::new(err),
        }
    }
    pub fn pointer(inner: TypeKind) -> Self {
        TypeKind::Pointer(Box::new(inner))
    }
    pub fn reference(inner: TypeKind, mutable: bool) -> Self {
        TypeKind::Reference {
            inner: Box::new(inner),
            mutable,
        }
    }
    pub fn function(params: Vec<TypeKind>, result: Option<TypeKind>) -> Self {
        TypeKind::Function {
            params,
            result: result.map(Box::new),
        }
    }
    pub fn promise(inner: TypeKind) -> Self {
        TypeKind::Promise(Box::new(inner))
    }
    pub fn channel(inner: TypeKind) -> Self {
        TypeKind::Channel(Box::new(inner))
    }

    // -------------------------------------------------------------------------
    // Type Checks
    // -------------------------------------------------------------------------

    pub fn is_numeric(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8
                | TypeKind::Int16
                | TypeKind::Int32
                | TypeKind::Int64
                | TypeKind::Int128
                | TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
                | TypeKind::Float32
                | TypeKind::Float64
                | TypeKind::Float128
        )
    }

    pub fn is_integer(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8
                | TypeKind::Int16
                | TypeKind::Int32
                | TypeKind::Int64
                | TypeKind::Int128
                | TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
        )
    }

    pub fn is_float(&self) -> bool {
        matches!(
            self,
            TypeKind::Float32 | TypeKind::Float64 | TypeKind::Float128
        )
    }

    pub fn is_signed(&self) -> bool {
        matches!(
            self,
            TypeKind::Int8 | TypeKind::Int16 | TypeKind::Int32 | TypeKind::Int64 | TypeKind::Int128
        )
    }

    pub fn is_unsigned(&self) -> bool {
        matches!(
            self,
            TypeKind::UInt8
                | TypeKind::UInt16
                | TypeKind::UInt32
                | TypeKind::UInt64
                | TypeKind::UInt128
        )
    }

    pub fn is_scalar(&self) -> bool {
        self.is_numeric() || matches!(self, TypeKind::String | TypeKind::Bool | TypeKind::Char)
    }

    pub fn is_collection(&self) -> bool {
        matches!(
            self,
            TypeKind::Array(_)
                | TypeKind::Tuple(_)
                | TypeKind::Set(_)
                | TypeKind::Map(_, _)
                | TypeKind::Pair(_, _)
                | TypeKind::Triple(_, _, _)
                | TypeKind::Range(_)
        )
    }

    pub fn is_nullable(&self) -> bool {
        matches!(self, TypeKind::Option(_) | TypeKind::Null)
    }

    pub fn is_callable(&self) -> bool {
        matches!(self, TypeKind::Function { .. } | TypeKind::Lambda { .. })
    }

    pub fn is_special(&self) -> bool {
        matches!(
            self,
            TypeKind::Auto
                | TypeKind::Any
                | TypeKind::Void
                | TypeKind::Never
                | TypeKind::Null
                | TypeKind::Undefined
        )
    }

    // -------------------------------------------------------------------------
    // Size in Bytes
    // -------------------------------------------------------------------------

    pub fn size_bytes(&self) -> Option<usize> {
        match self {
            TypeKind::Int8 | TypeKind::UInt8 => Some(1),
            TypeKind::Int16 | TypeKind::UInt16 => Some(2),
            TypeKind::Int32 | TypeKind::UInt32 | TypeKind::Float32 => Some(4),
            TypeKind::Int64 | TypeKind::UInt64 | TypeKind::Float64 => Some(8),
            TypeKind::Int128 | TypeKind::UInt128 | TypeKind::Float128 => Some(16),
            TypeKind::Bool => Some(1),
            TypeKind::Char => Some(4),
            _ => None,
        }
    }

    // -------------------------------------------------------------------------
    // Russian Names
    // -------------------------------------------------------------------------

    pub fn russian_name(&self) -> String {
        match self {
            TypeKind::Int8 => "цел_8".into(),
            TypeKind::Int16 => "цел_16".into(),
            TypeKind::Int32 => "цел_32".into(),
            TypeKind::Int64 => "цел".into(),
            TypeKind::Int128 => "цел_128".into(),
            TypeKind::UInt8 => "нат_8".into(),
            TypeKind::UInt16 => "нат_16".into(),
            TypeKind::UInt32 => "нат_32".into(),
            TypeKind::UInt64 => "нат".into(),
            TypeKind::UInt128 => "нат_128".into(),
            TypeKind::Float32 => "вещ_32".into(),
            TypeKind::Float64 => "вещ".into(),
            TypeKind::Float128 => "вещ_128".into(),
            TypeKind::String => "лит".into(),
            TypeKind::Bool => "лог".into(),
            TypeKind::Char => "сим".into(),
            TypeKind::Array(elem) => format!("таб {}", elem.russian_name()),
            TypeKind::Range(elem) => format!("диапазон {}", elem.russian_name()),
            TypeKind::Pair(a, b) => format!("пара({}, {})", a.russian_name(), b.russian_name()),
            TypeKind::Triple(a, b, c) => format!(
                "тройка({}, {}, {})",
                a.russian_name(),
                b.russian_name(),
                c.russian_name()
            ),
            TypeKind::Tuple(elems) => {
                let names: Vec<_> = elems.iter().map(|e| e.russian_name()).collect();
                format!("кортеж({})", names.join(", "))
            }
            TypeKind::Set(elem) => format!("множество {}", elem.russian_name()),
            TypeKind::Map(k, v) => format!("словарь[{}: {}]", k.russian_name(), v.russian_name()),
            TypeKind::Option(inner) => format!("{}?", inner.russian_name()),
            TypeKind::Result { ok, err } => format!("{}!{}", ok.russian_name(), err.russian_name()),
            TypeKind::Pointer(inner) => format!("указатель {}", inner.russian_name()),
            TypeKind::Reference { inner, mutable } => {
                if *mutable {
                    format!("&измен {}", inner.russian_name())
                } else {
                    format!("&{}", inner.russian_name())
                }
            }
            TypeKind::Enum(name) => name.clone(),
            TypeKind::Object(name) => name.clone(),
            TypeKind::Native(name) => format!("@{}", name),
            TypeKind::Generic { name, type_args } => {
                let args: Vec<_> = type_args.iter().map(|a| a.russian_name()).collect();
                format!("{}<{}>", name, args.join(", "))
            }
            TypeKind::Function { params, result } => {
                let param_names: Vec<_> = params.iter().map(|p| p.russian_name()).collect();
                let ret = result
                    .as_ref()
                    .map(|r| r.russian_name())
                    .unwrap_or_else(|| "ничего".into());
                format!("функция({}) -> {}", param_names.join(", "), ret)
            }
            TypeKind::Lambda { params, result, .. } => {
                let param_names: Vec<_> = params.iter().map(|p| p.russian_name()).collect();
                let ret = result
                    .as_ref()
                    .map(|r| r.russian_name())
                    .unwrap_or_else(|| "ничего".into());
                format!("лямбда({}) -> {}", param_names.join(", "), ret)
            }
            TypeKind::Promise(inner) => format!("обещание {}", inner.russian_name()),
            TypeKind::Generator {
                yield_type,
                return_type,
            } => {
                format!(
                    "генератор<{}, {}>",
                    yield_type.russian_name(),
                    return_type.russian_name()
                )
            }
            TypeKind::Channel(inner) => format!("канал {}", inner.russian_name()),
            TypeKind::Null => "пусто".into(),
            TypeKind::Undefined => "неопределено".into(),
            TypeKind::Auto => "авто".into(),
            TypeKind::Any => "любой".into(),
            TypeKind::Void => "ничего".into(),
            TypeKind::Never => "никогда".into(),
            TypeKind::Type => "тип".into(),
        }
    }

    // -------------------------------------------------------------------------
    // Type Compatibility
    // -------------------------------------------------------------------------

    /// Checks if a value of type `from` can be implicitly assigned to a variable
    /// of type `self`.
    ///
    /// Thin facade over the single source of truth — the type engine
    /// ([`crate::typesys::TypeSystem`], see KITE 10). Kept for ergonomics; for
    /// coercion plans, unification or nominal subtyping use the engine directly.
    pub fn is_assignable_from(&self, from: &TypeKind) -> bool {
        crate::typesys::default_engine().is_assignable(self, from)
    }

    /// Finds a common type for two types (for binary operations).
    ///
    /// Delegates to the type engine's unification ([`crate::typesys::TypeSystem::unify`]).
    pub fn common_type(&self, other: &TypeKind) -> Option<TypeKind> {
        crate::typesys::default_engine().unify(self, other)
    }
}

impl fmt::Display for TypeKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.russian_name())
    }
}
