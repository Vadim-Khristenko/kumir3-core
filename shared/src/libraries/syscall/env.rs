//! Функции работы с переменными окружения

use std::collections::BTreeMap;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::type_spec::TypeSpec;
use crate::types::Value;

fn expect_string(args: &[Value], idx: usize, name: &str) -> Result<String, String> {
    let v = args
        .get(idx)
        .ok_or_else(|| format!("Не передан параметр: {}", name))?;
    match v {
        Value::String(s) => Ok(s.clone()),
        Value::Number(n) => Ok(n.to_string()),
        _ => Err(format!("Ожидается строка для параметра {}", name)),
    }
}

/// окружение(имя) -> лит | пусто
pub fn env_get_fn() -> LibFunctionDef {
    LibFunctionDef::new("окружение")
        .with_aliases(&["env", "env_get", "получить_окружение"])
        .with_description("Возвращает значение переменной окружения или пусто, если нет")
        .with_param(LibParamDef::value("имя", TypeSpec::String))
        .returns(TypeSpec::String)
        .with_handler(|args| {
            let name = expect_string(args, 0, "имя")?;
            match std::env::var(&name) {
                Ok(v) => Ok(Value::String(v)),
                Err(_) => Ok(Value::Null),
            }
        })
}

/// установить_окружение(имя, значение)
pub fn env_set_fn() -> LibFunctionDef {
    LibFunctionDef::new("установить_окружение")
        .with_aliases(&["env_set", "set_env"])
        .with_description("Устанавливает переменную окружения")
        .with_param(LibParamDef::value("имя", TypeSpec::String))
        .with_param(LibParamDef::value("значение", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let name = expect_string(args, 0, "имя")?;
            let val = expect_string(args, 1, "значение")?;
            // SAFETY: Mutating process environment is intended here
            unsafe { std::env::set_var(&name, &val) };
            Ok(Value::Null)
        })
}

/// удалить_окружение(имя)
pub fn env_unset_fn() -> LibFunctionDef {
    LibFunctionDef::new("удалить_окружение")
        .with_aliases(&["env_unset", "unset_env"])
        .with_description("Удаляет переменную окружения")
        .with_param(LibParamDef::value("имя", TypeSpec::String))
        .as_procedure()
        .with_handler(|args| {
            let name = expect_string(args, 0, "имя")?;
            // SAFETY: Removing process environment variables is intended here
            unsafe { std::env::remove_var(&name) };
            Ok(Value::Null)
        })
}

/// все_окружение() -> словарь
pub fn env_all_fn() -> LibFunctionDef {
    LibFunctionDef::new("все_окружение")
        .with_aliases(&["env_all", "get_all_env", "environ"])
        .with_description("Возвращает словарь всех переменных окружения")
        .returns(TypeSpec::Map(Box::new(TypeSpec::String), Box::new(TypeSpec::String)))
        .with_handler(|_args| {
            let mut map = BTreeMap::new();
            for (key, value) in std::env::vars() {
                map.insert(Value::String(key), Value::String(value));
            }
            Ok(Value::Map(map))
        })
}

/// есть_окружение(имя) -> лог
pub fn env_exists_fn() -> LibFunctionDef {
    LibFunctionDef::new("есть_окружение")
        .with_aliases(&["env_exists", "has_env"])
        .with_description("Проверяет существование переменной окружения")
        .with_param(LibParamDef::value("имя", TypeSpec::String))
        .returns(TypeSpec::Bool)
        .with_handler(|args| {
            let name = expect_string(args, 0, "имя")?;
            Ok(Value::Boolean(std::env::var(&name).is_ok()))
        })
}
