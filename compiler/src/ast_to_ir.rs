// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Преобразование AST в IR
//!
//! Конвертирует абстрактное синтаксическое дерево (AST) в
//! промежуточное представление (IR) для дальнейшей компиляции.

use shared::codegen::ir::{
    BinaryOp, BlockId, FuncId, IrFunction, IrInstruction, IrModule, IrType, IrValue, UnaryOp, VarId,
};
use shared::types::{Algorithm, Expr, Program, Stmt, Value};
use std::collections::HashMap;

/// Конвертер AST → IR.
pub struct AstToIr {
    /// Счётчик переменных
    var_counter: u32,
    /// Счётчик блоков
    block_counter: u32,
    /// Таблица переменных (имя → ID)
    var_map: HashMap<String, VarId>,
}

impl AstToIr {
    /// Создаёт новый конвертер.
    pub fn new() -> Self {
        Self {
            var_counter: 0,
            block_counter: 0,
            var_map: HashMap::new(),
        }
    }

    /// Конвертирует программу в IR модуль.
    pub fn convert(&mut self, program: &Program) -> Result<IrModule, String> {
        let mut module = IrModule::new("main");

        // Конвертируем все алгоритмы
        for alg in &program.algorithms {
            let func = self.convert_algorithm(alg)?;
            module.add_function(func);
        }

        Ok(module)
    }

    /// Конвертирует алгоритм в IR функцию.
    fn convert_algorithm(&mut self, alg: &Algorithm) -> Result<IrFunction, String> {
        // Сбрасываем состояние для новой функции
        self.var_map.clear();
        self.var_counter = 0;
        self.block_counter = 0;

        let params: Vec<(String, IrType)> = alg
            .params
            .iter()
            .map(|p| (p.name.to_string(), IrType::Int)) // TODO: правильные типы
            .collect();

        let return_type = IrType::Void; // TODO: определить из алгоритма

        let mut func = IrFunction::new(&alg.name, params, return_type);

        // Регистрируем параметры как переменные
        for (name, _) in &func.params {
            let var_id = self.alloc_var();
            self.var_map.insert(name.clone(), var_id);
        }

        // Конвертируем тело алгоритма
        if let Some(body) = &alg.body {
            let entry_block = func.entry_block;
            for stmt in body {
                self.convert_stmt(stmt, &mut func, entry_block)?;
            }
        }

        Ok(func)
    }

    /// Конвертирует инструкцию в IR.
    fn convert_stmt(
        &mut self,
        stmt: &Stmt,
        func: &mut IrFunction,
        block_id: BlockId,
    ) -> Result<(), String> {
        match stmt {
            Stmt::Assignment(name, expr) => {
                // Вычисляем выражение
                let value_var = self.convert_expr(expr, func, block_id)?;

                // Получаем или создаём переменную
                let dest_var = if let Some(&var) = self.var_map.get(name) {
                    var
                } else {
                    let var = self.alloc_var();
                    self.var_map.insert(name.clone(), var);
                    // Выделяем память для новой переменной
                    func.push_to_block(
                        block_id,
                        IrInstruction::Alloc {
                            dest: var,
                            typ: IrType::Int, // TODO: определить тип
                        },
                    );
                    var
                };

                // Сохраняем значение
                func.push_to_block(
                    block_id,
                    IrInstruction::Store {
                        src: value_var,
                        dest: dest_var,
                    },
                );
            }

            Stmt::Output(exprs) => {
                // Для каждого выражения генерируем вызов функции вывода
                for expr in exprs {
                    let value_var = self.convert_expr(expr, func, block_id)?;
                    func.push_to_block(
                        block_id,
                        IrInstruction::Call {
                            dest: None,
                            func: FuncId("print".to_string()),
                            args: vec![value_var],
                        },
                    );
                }
            }

            Stmt::Return => {
                func.push_to_block(block_id, IrInstruction::Return { value: None });
            }

            Stmt::ReturnValue(expr) => {
                let value_var = self.convert_expr(expr, func, block_id)?;
                func.push_to_block(
                    block_id,
                    IrInstruction::Return {
                        value: Some(value_var),
                    },
                );
            }

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                let cond_var = self.convert_expr(condition, func, block_id)?;

                let then_block = func.add_block("then");
                let else_block = func.add_block("else");
                let merge_block = func.add_block("merge");

                // Условный переход
                func.push_to_block(
                    block_id,
                    IrInstruction::CondBranch {
                        condition: cond_var,
                        then_block,
                        else_block,
                    },
                );

                // Then ветка
                for stmt in then_branch {
                    self.convert_stmt(stmt, func, then_block)?;
                }
                func.push_to_block(
                    then_block,
                    IrInstruction::Branch {
                        target: merge_block,
                    },
                );

                // Else ветка
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.convert_stmt(stmt, func, else_block)?;
                    }
                }
                func.push_to_block(
                    else_block,
                    IrInstruction::Branch {
                        target: merge_block,
                    },
                );
            }

            Stmt::LoopWhile { condition, body } => {
                let loop_cond = func.add_block("loop_cond");
                let loop_body = func.add_block("loop_body");
                let loop_exit = func.add_block("loop_exit");

                // Переход к проверке условия
                func.push_to_block(block_id, IrInstruction::Branch { target: loop_cond });

                // Проверка условия
                let cond_var = self.convert_expr(condition, func, loop_cond)?;
                func.push_to_block(
                    loop_cond,
                    IrInstruction::CondBranch {
                        condition: cond_var,
                        then_block: loop_body,
                        else_block: loop_exit,
                    },
                );

                // Тело цикла
                for stmt in body {
                    self.convert_stmt(stmt, func, loop_body)?;
                }
                func.push_to_block(loop_body, IrInstruction::Branch { target: loop_cond });
            }

            Stmt::LoopFor {
                variable,
                from,
                to,
                step,
                body,
            } => {
                // Инициализация счётчика
                let from_var = self.convert_expr(from, func, block_id)?;
                let counter_var = self.alloc_var();
                self.var_map.insert(variable.clone(), counter_var);

                func.push_to_block(
                    block_id,
                    IrInstruction::Alloc {
                        dest: counter_var,
                        typ: IrType::Int,
                    },
                );
                func.push_to_block(
                    block_id,
                    IrInstruction::Store {
                        src: from_var,
                        dest: counter_var,
                    },
                );

                let loop_cond = func.add_block("for_cond");
                let loop_body = func.add_block("for_body");
                let loop_incr = func.add_block("for_incr");
                let loop_exit = func.add_block("for_exit");

                func.push_to_block(block_id, IrInstruction::Branch { target: loop_cond });

                // Проверка условия: counter <= to
                let counter_load = self.alloc_var();
                func.push_to_block(
                    loop_cond,
                    IrInstruction::Load {
                        dest: counter_load,
                        src: counter_var,
                    },
                );
                let to_var = self.convert_expr(to, func, loop_cond)?;
                let cond_var = self.alloc_var();
                func.push_to_block(
                    loop_cond,
                    IrInstruction::BinaryOp {
                        dest: cond_var,
                        op: BinaryOp::Le,
                        left: counter_load,
                        right: to_var,
                    },
                );
                func.push_to_block(
                    loop_cond,
                    IrInstruction::CondBranch {
                        condition: cond_var,
                        then_block: loop_body,
                        else_block: loop_exit,
                    },
                );

                // Тело цикла
                for stmt in body {
                    self.convert_stmt(stmt, func, loop_body)?;
                }
                func.push_to_block(loop_body, IrInstruction::Branch { target: loop_incr });

                // Инкремент счётчика
                let step_var = if let Some(step_expr) = step {
                    self.convert_expr(step_expr, func, loop_incr)?
                } else {
                    let one = self.alloc_var();
                    func.push_to_block(
                        loop_incr,
                        IrInstruction::LoadConst {
                            dest: one,
                            value: IrValue::Int(1),
                        },
                    );
                    one
                };

                let counter_load2 = self.alloc_var();
                func.push_to_block(
                    loop_incr,
                    IrInstruction::Load {
                        dest: counter_load2,
                        src: counter_var,
                    },
                );
                let new_counter = self.alloc_var();
                func.push_to_block(
                    loop_incr,
                    IrInstruction::BinaryOp {
                        dest: new_counter,
                        op: BinaryOp::Add,
                        left: counter_load2,
                        right: step_var,
                    },
                );
                func.push_to_block(
                    loop_incr,
                    IrInstruction::Store {
                        src: new_counter,
                        dest: counter_var,
                    },
                );
                func.push_to_block(loop_incr, IrInstruction::Branch { target: loop_cond });
            }

            Stmt::Input(vars) => {
                // Для каждой переменной генерируем вызов функции ввода
                for var_name in vars {
                    let input_var = self.alloc_var();
                    func.push_to_block(
                        block_id,
                        IrInstruction::Call {
                            dest: Some(input_var),
                            func: FuncId("input".to_string()),
                            args: vec![],
                        },
                    );

                    // Получаем или создаём переменную
                    let dest_var = if let Some(&var) = self.var_map.get(var_name) {
                        var
                    } else {
                        let var = self.alloc_var();
                        self.var_map.insert(var_name.clone(), var);
                        func.push_to_block(
                            block_id,
                            IrInstruction::Alloc {
                                dest: var,
                                typ: IrType::Int,
                            },
                        );
                        var
                    };

                    func.push_to_block(
                        block_id,
                        IrInstruction::Store {
                            src: input_var,
                            dest: dest_var,
                        },
                    );
                }
            }

            Stmt::VarDecl {
                type_kind: _,
                names,
                init,
                modifiers: _,
            } => {
                for name in names {
                    let var = self.alloc_var();
                    self.var_map.insert(name.clone(), var);

                    // TODO: конвертировать TypeKind в IrType
                    func.push_to_block(
                        block_id,
                        IrInstruction::Alloc {
                            dest: var,
                            typ: IrType::Int,
                        },
                    );

                    // Инициализация если есть
                    if let Some(init_expr) = init {
                        let init_var = self.convert_expr(init_expr, func, block_id)?;
                        func.push_to_block(
                            block_id,
                            IrInstruction::Store {
                                src: init_var,
                                dest: var,
                            },
                        );
                    }
                }
            }

            Stmt::ExprStmt(expr) => {
                // Вычисляем выражение, результат игнорируем
                self.convert_expr(expr, func, block_id)?;
            }

            Stmt::Assert(expr) => {
                let cond_var = self.convert_expr(expr, func, block_id)?;
                func.push_to_block(
                    block_id,
                    IrInstruction::Call {
                        dest: None,
                        func: FuncId("assert".to_string()),
                        args: vec![cond_var],
                    },
                );
            }

            Stmt::LoopInfinite { body } => {
                let loop_body = func.add_block("infinite_body");
                let _loop_exit = func.add_block("infinite_exit");

                func.push_to_block(block_id, IrInstruction::Branch { target: loop_body });

                // Тело цикла
                for stmt in body {
                    self.convert_stmt(stmt, func, loop_body)?;
                }
                func.push_to_block(loop_body, IrInstruction::Branch { target: loop_body });
            }

            Stmt::LoopDoWhile { body, condition } => {
                let loop_body = func.add_block("do_body");
                let loop_cond = func.add_block("do_cond");
                let loop_exit = func.add_block("do_exit");

                func.push_to_block(block_id, IrInstruction::Branch { target: loop_body });

                // Тело цикла
                for stmt in body {
                    self.convert_stmt(stmt, func, loop_body)?;
                }
                func.push_to_block(loop_body, IrInstruction::Branch { target: loop_cond });

                // Проверка условия после тела
                let cond_var = self.convert_expr(condition, func, loop_cond)?;
                func.push_to_block(
                    loop_cond,
                    IrInstruction::CondBranch {
                        condition: cond_var,
                        then_block: loop_body,
                        else_block: loop_exit,
                    },
                );
            }

            Stmt::LoopForEach {
                variable,
                var_type: _,
                iterable: _,
                body: _,
            } => {
                // TODO: полная реализация итераторов
                // Пока генерируем комментарий
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("TODO: ForEach loop over {}", variable)),
                );
            }

            Stmt::ArrayAssignment(name, _indices, value) => {
                let _value_var = self.convert_expr(value, func, block_id)?;

                // TODO: правильная индексация массивов
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("TODO: ArrayAssignment {}[...] := value", name)),
                );
            }

            Stmt::ResultAssign(expr) => {
                // result := expr эквивалентно return expr
                let value_var = self.convert_expr(expr, func, block_id)?;
                func.push_to_block(
                    block_id,
                    IrInstruction::Return {
                        value: Some(value_var),
                    },
                );
            }

            Stmt::AutoVarDecl {
                name,
                init,
                modifiers: _,
            } => {
                let init_var = self.convert_expr(init, func, block_id)?;
                let var = self.alloc_var();
                self.var_map.insert(name.clone(), var);

                func.push_to_block(
                    block_id,
                    IrInstruction::Alloc {
                        dest: var,
                        typ: IrType::Int, // TODO: вывод типа
                    },
                );
                func.push_to_block(
                    block_id,
                    IrInstruction::Store {
                        src: init_var,
                        dest: var,
                    },
                );
            }

            Stmt::OutputFormatted { format, args } => {
                // Форматированный вывод
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("TODO: Formatted output: {}", format)),
                );

                // Пока просто выводим аргументы
                for arg in args {
                    let arg_var = self.convert_expr(arg, func, block_id)?;
                    func.push_to_block(
                        block_id,
                        IrInstruction::Call {
                            dest: None,
                            func: FuncId("print".to_string()),
                            args: vec![arg_var],
                        },
                    );
                }
            }

            Stmt::Block(stmts) => {
                // Блок инструкций
                for stmt in stmts {
                    self.convert_stmt(stmt, func, block_id)?;
                }
            }

            Stmt::Nop => {
                // Пустая инструкция - ничего не делаем
            }

            Stmt::Break | Stmt::Continue => {
                // TODO: требуется отслеживание контекста цикла
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(
                        "TODO: Break/Continue требует контекст цикла".to_string(),
                    ),
                );
            }

            _ => {
                // TODO: остальные инструкции
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("TODO: {:?}", stmt)),
                );
            }
        }

        Ok(())
    }

    /// Конвертирует выражение в IR.
    fn convert_expr(
        &mut self,
        expr: &Expr,
        func: &mut IrFunction,
        block_id: BlockId,
    ) -> Result<VarId, String> {
        match expr {
            Expr::Literal(val) => {
                let dest = self.alloc_var();
                let ir_value = self.convert_value(val)?;
                func.push_to_block(
                    block_id,
                    IrInstruction::LoadConst {
                        dest,
                        value: ir_value,
                    },
                );
                Ok(dest)
            }

            Expr::Variable(name) => {
                if let Some(&var) = self.var_map.get(name) {
                    let dest = self.alloc_var();
                    func.push_to_block(block_id, IrInstruction::Load { dest, src: var });
                    Ok(dest)
                } else {
                    Err(format!("Неопределённая переменная: {}", name))
                }
            }

            Expr::BinaryOp(left, op, right) => {
                let left_var = self.convert_expr(left, func, block_id)?;
                let right_var = self.convert_expr(right, func, block_id)?;
                let dest = self.alloc_var();

                let ir_op = self.convert_binary_op_token(op)?;

                func.push_to_block(
                    block_id,
                    IrInstruction::BinaryOp {
                        dest,
                        op: ir_op,
                        left: left_var,
                        right: right_var,
                    },
                );

                Ok(dest)
            }

            Expr::UnaryOp(op, operand) => {
                let operand_var = self.convert_expr(operand, func, block_id)?;
                let dest = self.alloc_var();

                let ir_op = self.convert_unary_op_token(op)?;

                func.push_to_block(
                    block_id,
                    IrInstruction::UnaryOp {
                        dest,
                        op: ir_op,
                        operand: operand_var,
                    },
                );

                Ok(dest)
            }

            Expr::Call(name, args) => {
                let mut arg_vars = Vec::new();
                for arg in args {
                    let arg_var = self.convert_expr(arg, func, block_id)?;
                    arg_vars.push(arg_var);
                }

                let dest = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::Call {
                        dest: Some(dest),
                        func: FuncId(name.to_string()),
                        args: arg_vars,
                    },
                );

                Ok(dest)
            }

            Expr::ArrayAccess(name, indices) => {
                // Загружаем базовый адрес массива
                let _array_var = if let Some(&var) = self.var_map.get(name) {
                    let dest = self.alloc_var();
                    func.push_to_block(block_id, IrInstruction::Load { dest, src: var });
                    dest
                } else {
                    return Err(format!("Неопределённый массив: {}", name));
                };

                // Вычисляем индексы
                let mut index_vars = Vec::new();
                for index in indices {
                    let index_var = self.convert_expr(index, func, block_id)?;
                    index_vars.push(index_var);
                }

                // TODO: правильная индексация многомерных массивов
                let dest = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("ArrayAccess: {}[indices]", name)),
                );
                Ok(dest)
            }

            Expr::IfExpr {
                condition,
                then_expr,
                else_expr,
            } => {
                let cond_var = self.convert_expr(condition, func, block_id)?;

                let then_block = func.add_block("if_then");
                let else_block = func.add_block("if_else");
                let merge_block = func.add_block("if_merge");

                let result_var = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::Alloc {
                        dest: result_var,
                        typ: IrType::Int, // TODO: определить тип из выражений
                    },
                );

                func.push_to_block(
                    block_id,
                    IrInstruction::CondBranch {
                        condition: cond_var,
                        then_block,
                        else_block,
                    },
                );

                // Then ветка
                let then_val = self.convert_expr(then_expr, func, then_block)?;
                func.push_to_block(
                    then_block,
                    IrInstruction::Store {
                        src: then_val,
                        dest: result_var,
                    },
                );
                func.push_to_block(
                    then_block,
                    IrInstruction::Branch {
                        target: merge_block,
                    },
                );

                // Else ветка
                let else_val = self.convert_expr(else_expr, func, else_block)?;
                func.push_to_block(
                    else_block,
                    IrInstruction::Store {
                        src: else_val,
                        dest: result_var,
                    },
                );
                func.push_to_block(
                    else_block,
                    IrInstruction::Branch {
                        target: merge_block,
                    },
                );

                // Загружаем результат
                let final_result = self.alloc_var();
                func.push_to_block(
                    merge_block,
                    IrInstruction::Load {
                        dest: final_result,
                        src: result_var,
                    },
                );

                Ok(final_result)
            }

            Expr::FieldAccess(object, field) => {
                let _object_var = self.convert_expr(object, func, block_id)?;
                let dest = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("FieldAccess: .{}", field)),
                );
                // TODO: реализовать доступ к полям через IR
                Ok(dest)
            }

            Expr::MethodCall {
                object,
                method,
                args,
            } => {
                let object_var = self.convert_expr(object, func, block_id)?;

                let mut arg_vars = vec![object_var]; // self как первый аргумент
                for arg in args {
                    let arg_var = self.convert_expr(arg, func, block_id)?;
                    arg_vars.push(arg_var);
                }

                let dest = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::Call {
                        dest: Some(dest),
                        func: FuncId(format!("method_{}", method)),
                        args: arg_vars,
                    },
                );

                Ok(dest)
            }

            Expr::Cast { expr, target_type } => {
                let _src_var = self.convert_expr(expr, func, block_id)?;
                let dest = self.alloc_var();
                // TODO: реализовать приведение типов
                func.push_to_block(
                    block_id,
                    IrInstruction::Comment(format!("Cast to {:?}", target_type)),
                );
                Ok(dest)
            }

            Expr::None => {
                let dest = self.alloc_var();
                func.push_to_block(
                    block_id,
                    IrInstruction::LoadConst {
                        dest,
                        value: IrValue::Null,
                    },
                );
                Ok(dest)
            }

            // [KITE-0002] Compiler support for ?? is not yet implemented.
            Expr::Coalesce(_, _) => {
                Err("Оператор ?? пока не поддерживается компилятором".to_string())
            }

            // [KITE-0002] Safe navigation is not yet supported by the compiler backend.
            Expr::SafeField { .. } | Expr::SafeMethod { .. } => Err(
                "Оператор безопасной навигации (?.) пока не поддерживается компилятором"
                    .to_string(),
            ),

            // [KITE-0002] Lambdas are not yet supported by the compiler backend.
            Expr::Lambda { .. } => Err("Лямбды пока не поддерживаются компилятором".to_string()),

            _ => Err(format!("Неподдерживаемое выражение: {:?}", expr)),
        }
    }

    /// Конвертирует значение в IR значение.
    fn convert_value(&self, val: &Value) -> Result<IrValue, String> {
        match val {
            Value::Number(n) => {
                // Number может быть Int или Float
                if n.is_integer() {
                    let i = n.to_i64().ok_or("Не удалось преобразовать в i64")?;
                    Ok(IrValue::Int(i))
                } else {
                    let f = n.to_f64().ok_or("Не удалось преобразовать в f64")?;
                    Ok(IrValue::Float(f))
                }
            }
            Value::Boolean(b) => Ok(IrValue::Bool(*b)),
            Value::Char(c) => Ok(IrValue::Char(*c)),
            Value::String(s) => Ok(IrValue::String(s.clone())),
            _ => Err(format!("Неподдерживаемое значение: {:?}", val)),
        }
    }

    /// Конвертирует токен бинарной операции.
    fn convert_binary_op_token(&self, op: &shared::types::Token) -> Result<BinaryOp, String> {
        use shared::types::Token;
        match op {
            Token::Plus => Ok(BinaryOp::Add),
            Token::Minus => Ok(BinaryOp::Sub),
            Token::Star => Ok(BinaryOp::Mul),
            Token::Slash => Ok(BinaryOp::Div),
            Token::Percent => Ok(BinaryOp::Mod),
            Token::Power => Ok(BinaryOp::Pow),
            Token::Equal => Ok(BinaryOp::Eq),
            Token::NotEqual => Ok(BinaryOp::Ne),
            Token::Less => Ok(BinaryOp::Lt),
            Token::LessEqual => Ok(BinaryOp::Le),
            Token::Greater => Ok(BinaryOp::Gt),
            Token::GreaterEqual => Ok(BinaryOp::Ge),
            Token::And => Ok(BinaryOp::And),
            Token::Or => Ok(BinaryOp::Or),
            _ => Err(format!("Неизвестная операция: {:?}", op)),
        }
    }

    /// Конвертирует токен унарной операции.
    fn convert_unary_op_token(&self, op: &shared::types::Token) -> Result<UnaryOp, String> {
        use shared::types::Token;
        match op {
            Token::Minus => Ok(UnaryOp::Neg),
            Token::Not => Ok(UnaryOp::Not),
            _ => Err(format!("Неизвестная унарная операция: {:?}", op)),
        }
    }

    /// Выделяет новый ID переменной.
    fn alloc_var(&mut self) -> VarId {
        let id = VarId(self.var_counter);
        self.var_counter += 1;
        id
    }

    /// Выделяет новый ID блока.
    fn _alloc_block(&mut self) -> BlockId {
        let id = BlockId(self.block_counter);
        self.block_counter += 1;
        id
    }
}

impl Default for AstToIr {
    fn default() -> Self {
        Self::new()
    }
}
