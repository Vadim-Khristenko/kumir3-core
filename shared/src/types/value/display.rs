//! `Display` implementation for [`Value`].

use std::fmt;

use super::{PromiseStatus, Value};

// =============================================================================
//         SECTION: DISPLAY IMPLEMENTATION
// =============================================================================

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Number(n) => write!(f, "{}", n),
            Value::String(s) => write!(f, "{}", s),
            Value::Boolean(b) => write!(f, "{}", if *b { "да" } else { "нет" }),
            Value::Char(c) => write!(f, "'{}'", c),
            Value::Array(a) => {
                write!(f, "[")?;
                for (i, v) in a.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Range {
                start,
                end,
                inclusive,
                step,
            } => {
                write!(
                    f,
                    "{}..{}{}{}",
                    start,
                    if *inclusive { "=" } else { "" },
                    end,
                    if *step != 1 {
                        format!(" шаг {}", step)
                    } else {
                        String::new()
                    }
                )
            }
            Value::Bytes(b) => {
                write!(f, "байты[")?;
                for (i, byte) in b.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", byte)?;
                }
                write!(f, "]")
            }
            Value::Tuple(t) => {
                write!(f, "(")?;
                for (i, v) in t.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, ")")
            }
            Value::Pair(a, b) => write!(f, "({}, {})", a, b),
            Value::Triple(a, b, c) => write!(f, "({}, {}, {})", a, b, c),
            Value::Set(s) => {
                write!(f, "{{")?;
                for (i, v) in s.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "}}")
            }
            Value::Map(m) => {
                write!(f, "{{")?;
                for (i, (k, v)) in m.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}")
            }
            Value::Option(o) => match o.as_ref() {
                Some(v) => write!(f, "Некоторое({})", v),
                None => write!(f, "Ничего"),
            },
            Value::Result(r) => match r.as_ref() {
                Ok(v) => write!(f, "Успех({})", v),
                Err(e) => write!(f, "Ошибка({})", e),
            },
            Value::Pointer(p) => write!(f, "&{}", p),
            Value::Reference { target, mutable } => {
                write!(f, "&{}{}", if *mutable { "измен " } else { "" }, target)
            }
            Value::Enum {
                name,
                variant,
                data,
            } => {
                write!(f, "{}::{}", name, variant)?;
                if let Some(d) = data {
                    write!(f, "({})", d)?;
                }
                Ok(())
            }
            Value::Object { type_id, fields } => {
                write!(f, "<Объект #{} {{", type_id.0)?;
                for (i, (k, v)) in fields.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}: {}", k, v)?;
                }
                write!(f, "}}>")
            }
            Value::NativeObject { type_name, .. } => write!(f, "<{}>", type_name),
            Value::Lambda(l) => {
                write!(f, "<лямбда({})>", l.params.join(", "))
            }
            Value::PartialApp { func, applied_args } => {
                write!(
                    f,
                    "<частичное применение: {} с {} арг>",
                    func,
                    applied_args.len()
                )
            }
            Value::Promise {
                task_id,
                status,
                result,
                error,
            } => match status {
                PromiseStatus::Pending => write!(f, "<Promise #{} ожидает>", task_id),
                PromiseStatus::Resolved => {
                    if let Some(v) = result {
                        write!(f, "<Promise #{} выполнен: {}>", task_id, v)
                    } else {
                        write!(f, "<Promise #{} выполнен>", task_id)
                    }
                }
                PromiseStatus::Rejected => {
                    if let Some(e) = error {
                        write!(f, "<Promise #{} отклонён: {}>", task_id, e)
                    } else {
                        write!(f, "<Promise #{} отклонён>", task_id)
                    }
                }
            },
            Value::Generator { id, state, .. } => {
                write!(f, "<Генератор #{} {:?}>", id, state)
            }
            Value::Channel {
                id,
                capacity,
                closed,
            } => {
                write!(
                    f,
                    "<Канал #{} вместимость={} закрыт={}>",
                    id, capacity, closed
                )
            }
            Value::Null => write!(f, "пусто"),
            Value::Undefined => write!(f, "неопределено"),
            Value::Type(t) => write!(f, "<Тип: {}>", t),
            Value::Error { message, kind, .. } => {
                write!(f, "<Ошибка {}: {}>", kind, message)
            }
        }
    }
}
