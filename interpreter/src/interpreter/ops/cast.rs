//! Приведение и проверка типов значений (хвосты `eval_cast` / `eval_type_check`).
//!
//! Разделение ответственности — как в [`super::binary`]:
//! * **движок типов** (`shared::typesys`) — авторитет по *типизации*: разрешено
//!   ли преобразование и какого оно рода ([`Coercion`]), является ли тип
//!   значения подтипом проверяемого ([`TypeSystem::is_subtype`]);
//! * **код ниже** — вычислительное ядро: как именно преобразовать значение.
//!
//! Движок спрашивается ПЕРЕД преобразованием, поэтому заведомо невозможные
//! приведения (`"текст" как цел`, `да как сим`) отбраковываются сразу и с
//! сообщением, называющим оба типа по-русски.

use shared::types::{Number, TypeKind, Value};
use shared::typesys::{Coercion, TypeError, default_engine};

use super::TypeOps;
use crate::interpreter::error::{RuntimeError, RuntimeErrorKind, RuntimeResult};

impl TypeOps {
    /// Приводит значение к целевому типу (`значение как Тип`).
    pub fn cast(value: Value, target: &TypeKind) -> RuntimeResult<Value> {
        // [typesys-seam: подключён] план преобразования от движка — до вычисления.
        let source = value.type_kind();
        let plan = default_engine().coercion(target, &source);
        if plan == Coercion::Forbidden && !Self::cast_extension(target, &source) {
            return Err(Self::cast_forbidden(target, &source));
        }

        Self::convert(value, target, plan)
    }

    /// Приведения, которые язык определяет ШИРЕ структурных правил движка.
    /// Для них отрицательный вердикт движка не является вето.
    ///
    /// Список исчерпывающе повторяет таблицу KITE 13 § 3.16 «Приведение `как`»
    /// в тех строках, где источником служит НЕ подтип цели:
    /// * `лит` ← любое — строковое представление определено для всех значений;
    /// * `лог` ← любое — истинность (§ 3.20) определена для всех значений;
    /// * `вещ` ← `лит` — разбор десятичной записи (`"2.5" как вещ`);
    /// * `сим` ← целое (кодовая точка) | `лит` (строка длины 1).
    ///
    /// Всё остальное движок выражает сам: числовые сужения/расширения — через
    /// [`Coercion::Cast`]/[`Coercion::Widen`], `любой` — через верхний тип,
    /// `Тип ← тот же Тип` — через [`Coercion::Identity`].
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

    /// Диагностика запрещённого движком приведения.
    ///
    /// Русские имена типов берутся из ошибки движка, поэтому цель и источник
    /// названы ровно так же, как в остальных типовых сообщениях.
    fn cast_forbidden(target: &TypeKind, source: &TypeKind) -> RuntimeError {
        let message = match default_engine().check_assignable(target, source) {
            Err(TypeError::NotAssignable { target, source }) => format!(
                "Приведение значения типа '{}' к типу '{}' не определено",
                source, target
            ),
            Err(other) => other.to_string(),
            // Недостижимо: сюда попадают только несовместимые пары.
            Ok(_) => format!(
                "Приведение значения типа '{}' к типу '{}' не определено",
                source.russian_name(),
                target.russian_name()
            ),
        };
        RuntimeError::new(message, RuntimeErrorKind::TypeMismatch)
    }

    /// Вычислительное ядро: как получить значение уже разрешённого приведения.
    fn convert(value: Value, target: &TypeKind, plan: Coercion) -> RuntimeResult<Value> {
        match target {
            // `любой` (top type): приведение к нему — тождество; принимает любое
            // значение и никогда не ошибается. Всё является подтипом `любой`.
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
            // [W0] `как сим`: символ ← целое (код) | строка длины 1 | символ.
            // Всё остальное — ясная ошибка (раньше был not_implemented).
            TypeKind::Char => Self::cast_to_char(value),
            // `T?`: движок планирует упаковку ([`Coercion::Wrap`]); внутреннее
            // значение приводится к `T` тем же путём, `пусто` даёт пустой
            // необязательный.
            TypeKind::Option(inner) => match value {
                Value::Null => Ok(Value::Option(Box::new(None))),
                Value::Option(_) => Ok(value),
                other => Ok(Value::Option(Box::new(Some(Self::cast(other, inner)?)))),
            },
            // Остальные типы ядро не преобразует. Тождественное приведение
            // (`значение как ЕгоЖеТип`) движок распознаёт сам — оно ничего не
            // меняет и потому допустимо.
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

    /// [W0] Реализация `значение как сим`.
    ///
    /// * `Value::Char(c)` → сам символ (тождество);
    /// * целое число → символ с этим кодом Unicode (скалярным значением);
    /// * строка ровно из одного символа → этот символ;
    /// * всё остальное (в т.ч. строка другой длины, вещественное, логическое,
    ///   массив, объект) → ясная ошибка выполнения.
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

    /// Единая формулировка ошибки приведения к `сим`.
    fn char_cast_error(details: &str) -> RuntimeError {
        RuntimeError::type_mismatch("сим (символ: целое-код или строка длины 1)", details)
    }

    /// Проверяет, соответствует ли значение указанному типу (`значение это Тип`).
    ///
    /// Вопрос «является ли тип значения подтипом проверяемого» целиком
    /// принадлежит движку: подтипирование, числовое расширение (`вещ_32`
    /// это `вещ`), элементы массивов и верхний тип `любой` (всё — его подтип,
    /// поэтому `значение это любой` всегда истинно) выражены его правилами.
    pub fn type_check(value: &Value, check: &TypeKind) -> bool {
        // [typesys-seam: подключён] вердикт conformance целиком у движка.
        default_engine().is_subtype(&value.type_kind(), check)
    }
}
