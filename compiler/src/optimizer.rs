// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Оптимизатор IR
//!
//! Выполняет оптимизации промежуточного представления:
//! - Свёртка констант (constant folding)
//! - Удаление мёртвого кода (dead code elimination)
//! - Упрощение выражений

use shared::codegen::ir::{BinaryOp, IrFunction, IrInstruction, IrModule, IrValue, UnaryOp, VarId};
use std::collections::{HashMap, HashSet};

/// Оптимизатор IR модуля.
pub struct IrOptimizer {
    /// Уровень оптимизации (0-3)
    level: u8,
    /// Режим отладки
    debug: bool,
}

impl IrOptimizer {
    /// Создаёт новый оптимизатор.
    pub fn new(level: u8) -> Self {
        Self {
            level: level.min(3),
            debug: false,
        }
    }

    /// Включает режим отладки.
    pub fn with_debug(mut self, debug: bool) -> Self {
        self.debug = debug;
        self
    }

    /// Оптимизирует IR модуль.
    pub fn optimize(&self, mut module: IrModule) -> IrModule {
        if self.level == 0 {
            return module;
        }

        if self.debug {
            eprintln!("[Optimizer] Оптимизация уровня {}", self.level);
        }

        // Оптимизируем каждую функцию
        for func in module.functions.values_mut() {
            self.optimize_function(func);
        }

        module
    }

    /// Оптимизирует функцию.
    fn optimize_function(&self, func: &mut IrFunction) {
        // Уровень 1: свёртка констант
        if self.level >= 1 {
            self.constant_folding(func);
        }

        // Уровень 2: удаление мёртвого кода
        if self.level >= 2 {
            self.dead_code_elimination(func);
        }

        // Уровень 3: агрессивные оптимизации
        if self.level >= 3 {
            self.aggressive_optimizations(func);
        }
    }

    /// Свёртка констант.
    fn constant_folding(&self, func: &mut IrFunction) {
        let mut constants: HashMap<VarId, IrValue> = HashMap::new();

        for block in &mut func.blocks {
            let mut new_instructions = Vec::new();

            for instr in &block.instructions {
                match instr {
                    // Запоминаем константы
                    IrInstruction::LoadConst { dest, value } => {
                        constants.insert(*dest, value.clone());
                        new_instructions.push(instr.clone());
                    }

                    // Свёртка бинарных операций
                    IrInstruction::BinaryOp {
                        dest,
                        op,
                        left,
                        right,
                    } => {
                        if let (Some(left_val), Some(right_val)) =
                            (constants.get(left), constants.get(right))
                            && let Some(result) = self.fold_binary_op(*op, left_val, right_val)
                        {
                            // Заменяем операцию на константу
                            constants.insert(*dest, result.clone());
                            new_instructions.push(IrInstruction::LoadConst {
                                dest: *dest,
                                value: result,
                            });
                            continue;
                        }
                        new_instructions.push(instr.clone());
                    }

                    // Свёртка унарных операций
                    IrInstruction::UnaryOp { dest, op, operand } => {
                        if let Some(operand_val) = constants.get(operand)
                            && let Some(result) = self.fold_unary_op(*op, operand_val)
                        {
                            constants.insert(*dest, result.clone());
                            new_instructions.push(IrInstruction::LoadConst {
                                dest: *dest,
                                value: result,
                            });
                            continue;
                        }
                        new_instructions.push(instr.clone());
                    }

                    // Остальные инструкции
                    _ => {
                        new_instructions.push(instr.clone());
                    }
                }
            }

            block.instructions = new_instructions;
        }
    }

    /// Свёртка бинарной операции.
    fn fold_binary_op(&self, op: BinaryOp, left: &IrValue, right: &IrValue) -> Option<IrValue> {
        match (left, right) {
            (IrValue::Int(a), IrValue::Int(b)) => {
                let result = match op {
                    BinaryOp::Add => a.checked_add(*b)?,
                    BinaryOp::Sub => a.checked_sub(*b)?,
                    BinaryOp::Mul => a.checked_mul(*b)?,
                    BinaryOp::Div => if *b != 0 { Some(a / b) } else { None }?,
                    BinaryOp::Mod => if *b != 0 { Some(a % b) } else { None }?,
                    BinaryOp::Eq => return Some(IrValue::Bool(a == b)),
                    BinaryOp::Ne => return Some(IrValue::Bool(a != b)),
                    BinaryOp::Lt => return Some(IrValue::Bool(a < b)),
                    BinaryOp::Le => return Some(IrValue::Bool(a <= b)),
                    BinaryOp::Gt => return Some(IrValue::Bool(a > b)),
                    BinaryOp::Ge => return Some(IrValue::Bool(a >= b)),
                    _ => return None,
                };
                Some(IrValue::Int(result))
            }

            (IrValue::Float(a), IrValue::Float(b)) => {
                let result = match op {
                    BinaryOp::Add => a + b,
                    BinaryOp::Sub => a - b,
                    BinaryOp::Mul => a * b,
                    BinaryOp::Div => a / b,
                    BinaryOp::Eq => return Some(IrValue::Bool(a == b)),
                    BinaryOp::Ne => return Some(IrValue::Bool(a != b)),
                    BinaryOp::Lt => return Some(IrValue::Bool(a < b)),
                    BinaryOp::Le => return Some(IrValue::Bool(a <= b)),
                    BinaryOp::Gt => return Some(IrValue::Bool(a > b)),
                    BinaryOp::Ge => return Some(IrValue::Bool(a >= b)),
                    _ => return None,
                };
                Some(IrValue::Float(result))
            }

            (IrValue::Bool(a), IrValue::Bool(b)) => {
                let result = match op {
                    BinaryOp::And => *a && *b,
                    BinaryOp::Or => *a || *b,
                    BinaryOp::Eq => a == b,
                    BinaryOp::Ne => a != b,
                    _ => return None,
                };
                Some(IrValue::Bool(result))
            }

            _ => None,
        }
    }

    /// Свёртка унарной операции.
    fn fold_unary_op(&self, op: UnaryOp, operand: &IrValue) -> Option<IrValue> {
        match operand {
            IrValue::Int(n) => {
                let result = match op {
                    UnaryOp::Neg => -n,
                    _ => return None,
                };
                Some(IrValue::Int(result))
            }

            IrValue::Float(f) => {
                let result = match op {
                    UnaryOp::Neg => -f,
                    _ => return None,
                };
                Some(IrValue::Float(result))
            }

            IrValue::Bool(b) => {
                let result = match op {
                    UnaryOp::Not => !b,
                    _ => return None,
                };
                Some(IrValue::Bool(result))
            }

            _ => None,
        }
    }

    /// Удаление мёртвого кода.
    fn dead_code_elimination(&self, func: &mut IrFunction) {
        // Находим используемые переменные
        let mut used_vars: HashSet<VarId> = HashSet::new();

        for block in &func.blocks {
            for instr in &block.instructions {
                match instr {
                    IrInstruction::Load { src, .. } => {
                        used_vars.insert(*src);
                    }
                    IrInstruction::Store { src, dest } => {
                        used_vars.insert(*src);
                        used_vars.insert(*dest); // dest также используется
                    }
                    IrInstruction::BinaryOp { left, right, .. } => {
                        used_vars.insert(*left);
                        used_vars.insert(*right);
                    }
                    IrInstruction::UnaryOp { operand, .. } => {
                        used_vars.insert(*operand);
                    }
                    IrInstruction::Call { args, .. } => {
                        for arg in args {
                            used_vars.insert(*arg);
                        }
                    }
                    IrInstruction::Return { value: Some(v) } => {
                        used_vars.insert(*v);
                    }
                    IrInstruction::CondBranch { condition, .. } => {
                        used_vars.insert(*condition);
                    }
                    _ => {}
                }
            }
        }

        // Удаляем неиспользуемые Alloc инструкции
        for block in &mut func.blocks {
            block.instructions.retain(|instr| {
                if let IrInstruction::Alloc { dest, .. } = instr {
                    used_vars.contains(dest)
                } else {
                    true
                }
            });
        }
    }

    /// Агрессивные оптимизации.
    fn aggressive_optimizations(&self, _func: &mut IrFunction) {
        // TODO: инлайнинг функций, оптимизация циклов, и т.д.
        if self.debug {
            eprintln!("[Optimizer] Агрессивные оптимизации пока не реализованы");
        }
    }
}

impl Default for IrOptimizer {
    fn default() -> Self {
        Self::new(0)
    }
}
