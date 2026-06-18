// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Rust бэкенд компилятора
//!
//! Генерирует Rust код из IR и компилирует через rustc.

use super::Backend;
use shared::codegen::ir::{BasicBlock, BinaryOp, IrFunction, IrInstruction, IrModule};
use std::path::Path;
use std::process::Command;

/// Бэкенд генерации Rust кода.
pub struct RustBackend {
    debug: bool,
}

impl RustBackend {
    /// Создаёт новый Rust бэкенд.
    pub fn new() -> Self {
        Self { debug: false }
    }

    /// Включает режим отладки.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Компилирует Rust код в исполняемый файл через rustc.
    pub fn compile_to_exe(&self, rust_code: &str, output: &Path) -> Result<(), String> {
        if self.debug {
            eprintln!("[Rust Backend] Компиляция через rustc...");
        }

        // Создаём временный файл с Rust кодом
        let temp_dir = std::env::temp_dir();
        let temp_file = temp_dir.join("kumir_temp.rs");

        std::fs::write(&temp_file, rust_code)
            .map_err(|e| format!("Не удалось создать временный файл: {}", e))?;

        // Компилируем через rustc
        let status = Command::new("rustc")
            .arg(&temp_file)
            .arg("-o")
            .arg(output)
            .arg("-C")
            .arg("opt-level=2")
            .status()
            .map_err(|e| format!("Не удалось запустить rustc: {}", e))?;

        // Удаляем временный файл
        let _ = std::fs::remove_file(&temp_file);

        if !status.success() {
            return Err("rustc завершился с ошибкой".to_string());
        }

        if self.debug {
            eprintln!("[Rust Backend] Компиляция завершена");
        }

        Ok(())
    }

    /// Генерирует Rust код из IR модуля.
    fn generate_rust(&self, module: &IrModule) -> Result<String, String> {
        let mut code = String::new();

        // Заголовок
        code.push_str("// Сгенерировано компилятором Kumir 3\n");
        code.push_str("// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>\n\n");

        // Импорты
        code.push_str("use std::io::{self, Write};\n\n");

        // Генерируем функции
        for func in module.functions.values() {
            self.generate_function(func, &mut code)?;
            code.push('\n');
        }

        // Генерируем main если есть функция Главный
        if module.functions.contains_key("Главный") || module.functions.contains_key("main")
        {
            code.push_str("fn main() {\n");
            if module.functions.contains_key("Главный") {
                code.push_str("    Главный();\n");
            } else {
                code.push_str("    main();\n");
            }
            code.push_str("}\n");
        }

        Ok(code)
    }

    /// Генерирует Rust код для функции.
    fn generate_function(&self, func: &IrFunction, code: &mut String) -> Result<(), String> {
        // Сигнатура функции
        code.push_str(&format!("fn {}(", func.name));

        // Параметры
        for (i, (name, _typ)) in func.params.iter().enumerate() {
            if i > 0 {
                code.push_str(", ");
            }
            code.push_str(&format!("{}: i64", name)); // TODO: типы параметров
        }

        code.push(')');

        // Возвращаемый тип
        code.push_str(" -> i64"); // TODO: правильный тип

        code.push_str(" {\n");

        // Тело функции
        for block in &func.blocks {
            self.generate_block(block, code)?;
        }

        code.push_str("}\n");

        Ok(())
    }

    /// Генерирует Rust код для базового блока.
    fn generate_block(&self, block: &BasicBlock, code: &mut String) -> Result<(), String> {
        code.push_str(&format!("    // Block {} ({})\n", block.id.0, block.label));

        for instr in &block.instructions {
            self.generate_instruction(instr, code)?;
        }

        Ok(())
    }

    /// Генерирует Rust код для инструкции.
    fn generate_instruction(&self, instr: &IrInstruction, code: &mut String) -> Result<(), String> {
        use shared::codegen::ir::{IrValue, UnaryOp};

        match instr {
            IrInstruction::Alloc { dest, .. } => {
                code.push_str(&format!("    let mut v{} = 0i64;\n", dest.0));
            }
            IrInstruction::Store { src, dest } => {
                code.push_str(&format!("    v{} = v{};\n", dest.0, src.0));
            }
            IrInstruction::Load { dest, src } => {
                code.push_str(&format!("    let v{} = v{};\n", dest.0, src.0));
            }
            IrInstruction::LoadConst { dest, value } => {
                let val_str = match value {
                    IrValue::Int(i) => format!("{}i64", i),
                    IrValue::Float(f) => format!("{}f64", f),
                    IrValue::Bool(b) => (if *b { "1i64" } else { "0i64" }).to_string(),
                    IrValue::Char(c) => format!("'{}' as i64", c),
                    IrValue::String(s) => format!("\"{}\"", s.replace("\"", "\\\"")),
                    IrValue::Null => "0i64".to_string(),
                    _ => return Err(format!("Неподдерживаемое значение: {:?}", value)),
                };
                code.push_str(&format!("    let v{} = {};\n", dest.0, val_str));
            }
            IrInstruction::BinaryOp {
                dest,
                op,
                left,
                right,
            } => {
                let op_str = match op {
                    BinaryOp::Add => "+",
                    BinaryOp::Sub => "-",
                    BinaryOp::Mul => "*",
                    BinaryOp::Div => "/",
                    BinaryOp::Mod => "%",
                    BinaryOp::Pow => {
                        code.push_str(&format!(
                            "    let v{} = (v{} as f64).powf(v{} as f64) as i64;\n",
                            dest.0, left.0, right.0
                        ));
                        return Ok(());
                    }
                    BinaryOp::Eq => "==",
                    BinaryOp::Ne => "!=",
                    BinaryOp::Lt => "<",
                    BinaryOp::Gt => ">",
                    BinaryOp::Le => "<=",
                    BinaryOp::Ge => ">=",
                    BinaryOp::And => "&&",
                    BinaryOp::Or => "||",
                    _ => return Err(format!("Неизвестная операция: {:?}", op)),
                };

                // Для логических операций результат должен быть 0 или 1
                if matches!(
                    op,
                    BinaryOp::Eq
                        | BinaryOp::Ne
                        | BinaryOp::Lt
                        | BinaryOp::Gt
                        | BinaryOp::Le
                        | BinaryOp::Ge
                        | BinaryOp::And
                        | BinaryOp::Or
                ) {
                    code.push_str(&format!(
                        "    let v{} = if v{} {} v{} {{ 1i64 }} else {{ 0i64 }};\n",
                        dest.0, left.0, op_str, right.0
                    ));
                } else {
                    code.push_str(&format!(
                        "    let v{} = v{} {} v{};\n",
                        dest.0, left.0, op_str, right.0
                    ));
                }
            }
            IrInstruction::UnaryOp { dest, op, operand } => {
                let op_str = match op {
                    UnaryOp::Neg => "-",
                    UnaryOp::Not => "!",
                    _ => return Err(format!("Неизвестная унарная операция: {:?}", op)),
                };

                if matches!(op, UnaryOp::Not) {
                    code.push_str(&format!(
                        "    let v{} = if v{} == 0 {{ 1i64 }} else {{ 0i64 }};\n",
                        dest.0, operand.0
                    ));
                } else {
                    code.push_str(&format!(
                        "    let v{} = {}v{};\n",
                        dest.0, op_str, operand.0
                    ));
                }
            }
            IrInstruction::Call { dest, func, args } => {
                // Обработка встроенных функций
                let func_name = &func.0;
                if func_name == "print" || func_name == "вывод" {
                    for arg in args {
                        code.push_str(&format!("    println!(\"{{:?}}\", v{});\n", arg.0));
                    }
                    if let Some(d) = dest {
                        code.push_str(&format!("    let v{} = 0i64;\n", d.0));
                    }
                } else if func_name == "input" || func_name == "ввод" {
                    if let Some(d) = dest {
                        code.push_str("    let mut input = String::new();\n");
                        code.push_str("    io::stdin().read_line(&mut input).unwrap();\n");
                        code.push_str(&format!(
                            "    let v{} = input.trim().parse::<i64>().unwrap_or(0);\n",
                            d.0
                        ));
                    }
                } else if func_name == "assert" {
                    if let Some(arg) = args.first() {
                        code.push_str(&format!(
                            "    assert!(v{} != 0, \"Assertion failed\");\n",
                            arg.0
                        ));
                    }
                    if let Some(d) = dest {
                        code.push_str(&format!("    let v{} = 0i64;\n", d.0));
                    }
                } else {
                    // Обычный вызов функции
                    if let Some(d) = dest {
                        code.push_str(&format!("    let v{} = {}(", d.0, func_name));
                    } else {
                        code.push_str(&format!("    {}(", func_name));
                    }
                    for (i, arg) in args.iter().enumerate() {
                        if i > 0 {
                            code.push_str(", ");
                        }
                        code.push_str(&format!("v{}", arg.0));
                    }
                    code.push_str(");\n");
                }
            }
            IrInstruction::Return { value } => {
                if let Some(v) = value {
                    code.push_str(&format!("    return v{};\n", v.0));
                } else {
                    code.push_str("    return 0i64;\n");
                }
            }
            IrInstruction::Branch { target } => {
                code.push_str(&format!("    // goto block {}\n", target.0));
            }
            IrInstruction::CondBranch {
                condition,
                then_block,
                else_block,
            } => {
                code.push_str(&format!("    if v{} != 0 {{\n", condition.0));
                code.push_str(&format!("        // goto block {}\n", then_block.0));
                code.push_str("    } else {\n");
                code.push_str(&format!("        // goto block {}\n", else_block.0));
                code.push_str("    }\n");
            }
            IrInstruction::Comment(text) => {
                code.push_str(&format!("    // {}\n", text));
            }
            _ => {
                code.push_str(&format!("    // TODO: {:?}\n", instr));
            }
        }

        Ok(())
    }
}

impl Backend for RustBackend {
    fn generate(&self, module: &IrModule) -> Result<String, String> {
        self.generate_rust(module)
    }

    fn compile(&self, code: &str, output: &Path) -> Result<(), String> {
        self.compile_to_exe(code, output)
    }
}

impl Default for RustBackend {
    fn default() -> Self {
        Self::new()
    }
}
