//! Функции выполнения команд и процессов

use std::collections::BTreeMap;
use std::process::Command;
use std::sync::Arc;

use crate::types::library::{LibFunctionDef, LibParamDef};
use crate::types::{Number, TypeKind, Value};

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

fn run_shell(cmd: &str) -> Result<(i32, String, String), String> {
    let output = if cfg!(windows) {
        Command::new("cmd").args(["/C", cmd]).output()
    } else {
        Command::new("sh").args(["-c", cmd]).output()
    }
    .map_err(|e| format!("Не удалось запустить команду: {}", e))?;

    let code = output.status.code().unwrap_or(-1);
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    Ok((code, stdout, stderr))
}

pub fn run_fn() -> LibFunctionDef {
    LibFunctionDef::new("выполнить")
        .with_aliases(vec![
            Arc::from("run"),
            Arc::from("exec"),
            Arc::from("shell"),
        ])
        .with_description(
            "Выполняет команду через оболочку. Возвращает словарь: code, stdout, stderr",
        )
        .with_param(LibParamDef::value("команда", TypeKind::String))
        .returns(TypeKind::Map(
            Box::new(TypeKind::String),
            Box::new(TypeKind::Any),
        ))
        .with_handler(|args| {
            let cmd = expect_string(args, 0, "команда")?;
            let (code, stdout, stderr) = run_shell(&cmd)?;
            let mut map = BTreeMap::new();
            map.insert(
                Value::String("code".into()),
                Value::Number(Number::I32(code)),
            );
            map.insert(Value::String("stdout".into()), Value::String(stdout));
            map.insert(Value::String("stderr".into()), Value::String(stderr));
            Ok(Value::Map(map))
        })
}

pub fn system_fn() -> LibFunctionDef {
    LibFunctionDef::new("система")
        .with_aliases(vec![Arc::from("system"), Arc::from("os_system")])
        .with_description("Выполняет команду и возвращает только код возврата")
        .with_param(LibParamDef::value("команда", TypeKind::String))
        .returns(TypeKind::Int32)
        .with_handler(|args| {
            let cmd = expect_string(args, 0, "команда")?;
            let (code, _, _) = run_shell(&cmd)?;
            Ok(Value::Number(Number::I32(code)))
        })
}

pub fn popen_fn() -> LibFunctionDef {
    LibFunctionDef::new("вывод_команды")
        .with_aliases(vec![
            Arc::from("popen"),
            Arc::from("getoutput"),
            Arc::from("command_output"),
        ])
        .with_description("Выполняет команду и возвращает её stdout")
        .with_param(LibParamDef::value("команда", TypeKind::String))
        .returns(TypeKind::String)
        .with_handler(|args| {
            let cmd = expect_string(args, 0, "команда")?;
            let (_, stdout, _) = run_shell(&cmd)?;
            Ok(Value::String(stdout.trim_end().to_string()))
        })
}

pub fn run_success_fn() -> LibFunctionDef {
    LibFunctionDef::new("успех_команды")
        .with_aliases(vec![Arc::from("run_success"), Arc::from("command_success")])
        .with_description("Выполняет команду и возвращает да, если код возврата 0")
        .with_param(LibParamDef::value("команда", TypeKind::String))
        .returns(TypeKind::Bool)
        .with_handler(|args| {
            let cmd = expect_string(args, 0, "команда")?;
            let (code, _, _) = run_shell(&cmd)?;
            Ok(Value::Boolean(code == 0))
        })
}
