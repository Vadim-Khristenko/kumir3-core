// Copyright (c) 2024-2026 Vadim Khristenko <just@vai-prog.ru>
// Licensed under MIT OR Apache-2.0

//! Тесты оптимизатора IR

use kumir3_compiler::{Compiler, IrOptimizer};
use shared::codegen::ir::{BinaryOp, IrFunction, IrInstruction, IrModule, IrType, IrValue, VarId};

#[test]
fn test_constant_folding_addition() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("test", vec![], IrType::Int);

    let entry = func.entry_block;

    // v0 = 5
    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(0),
            value: IrValue::Int(5),
        },
    );

    // v1 = 10
    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(1),
            value: IrValue::Int(10),
        },
    );

    // v2 = v0 + v1
    func.push_to_block(
        entry,
        IrInstruction::BinaryOp {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
        },
    );

    module.add_function(func);

    // Оптимизируем
    let optimizer = IrOptimizer::new(1);
    let optimized = optimizer.optimize(module);

    // Проверяем что v2 = 15 (константа)
    let func = optimized.functions.get("test").unwrap();
    let block = &func.blocks[0];

    // Должно быть 3 инструкции: LoadConst(5), LoadConst(10), LoadConst(15)
    assert_eq!(block.instructions.len(), 3);

    if let IrInstruction::LoadConst { dest, value } = &block.instructions[2] {
        assert_eq!(*dest, VarId(2));
        assert_eq!(*value, IrValue::Int(15));
    } else {
        panic!("Ожидалась LoadConst инструкция");
    }
}

#[test]
fn test_constant_folding_multiplication() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("test", vec![], IrType::Int);

    let entry = func.entry_block;

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(0),
            value: IrValue::Int(6),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(1),
            value: IrValue::Int(7),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::BinaryOp {
            dest: VarId(2),
            op: BinaryOp::Mul,
            left: VarId(0),
            right: VarId(1),
        },
    );

    module.add_function(func);

    let optimizer = IrOptimizer::new(1);
    let optimized = optimizer.optimize(module);

    let func = optimized.functions.get("test").unwrap();
    let block = &func.blocks[0];

    if let IrInstruction::LoadConst { value, .. } = &block.instructions[2] {
        assert_eq!(*value, IrValue::Int(42));
    } else {
        panic!("Ожидалась LoadConst инструкция с результатом 42");
    }
}

#[test]
fn test_constant_folding_comparison() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("test", vec![], IrType::Int);

    let entry = func.entry_block;

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(0),
            value: IrValue::Int(10),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(1),
            value: IrValue::Int(5),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::BinaryOp {
            dest: VarId(2),
            op: BinaryOp::Gt,
            left: VarId(0),
            right: VarId(1),
        },
    );

    module.add_function(func);

    let optimizer = IrOptimizer::new(1);
    let optimized = optimizer.optimize(module);

    let func = optimized.functions.get("test").unwrap();
    let block = &func.blocks[0];

    if let IrInstruction::LoadConst { value, .. } = &block.instructions[2] {
        assert_eq!(*value, IrValue::Bool(true));
    } else {
        panic!("Ожидалась LoadConst инструкция с результатом true");
    }
}

#[test]
fn test_dead_code_elimination() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("test", vec![], IrType::Int);

    let entry = func.entry_block;

    // Выделяем переменную, которая не используется
    func.push_to_block(
        entry,
        IrInstruction::Alloc {
            dest: VarId(0),
            typ: IrType::Int,
        },
    );

    // Выделяем переменную, которая используется
    func.push_to_block(
        entry,
        IrInstruction::Alloc {
            dest: VarId(1),
            typ: IrType::Int,
        },
    );

    // Используем v1
    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(2),
            value: IrValue::Int(42),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::Store {
            src: VarId(2),
            dest: VarId(1),
        },
    );

    module.add_function(func);

    let optimizer = IrOptimizer::new(2);
    let optimized = optimizer.optimize(module);

    let func = optimized.functions.get("test").unwrap();
    let block = &func.blocks[0];

    // v0 должна быть удалена, так как не используется
    let alloc_count = block
        .instructions
        .iter()
        .filter(|i| matches!(i, IrInstruction::Alloc { .. }))
        .count();

    assert_eq!(
        alloc_count, 1,
        "Должна остаться только одна Alloc инструкция"
    );
}

#[test]
fn test_optimizer_level_0_no_optimization() {
    let mut module = IrModule::new("test");
    let mut func = IrFunction::new("test", vec![], IrType::Int);

    let entry = func.entry_block;

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(0),
            value: IrValue::Int(5),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::LoadConst {
            dest: VarId(1),
            value: IrValue::Int(10),
        },
    );

    func.push_to_block(
        entry,
        IrInstruction::BinaryOp {
            dest: VarId(2),
            op: BinaryOp::Add,
            left: VarId(0),
            right: VarId(1),
        },
    );

    let original_instr_count = func.blocks[0].instructions.len();
    module.add_function(func);

    // Оптимизация уровня 0 не должна ничего менять
    let optimizer = IrOptimizer::new(0);
    let optimized = optimizer.optimize(module);

    let func = optimized.functions.get("test").unwrap();
    let block = &func.blocks[0];

    assert_eq!(block.instructions.len(), original_instr_count);

    // Последняя инструкция должна остаться BinaryOp
    assert!(matches!(
        block.instructions[2],
        IrInstruction::BinaryOp { .. }
    ));
}

#[test]
fn test_compiler_with_optimization() {
    let mut compiler = Compiler::new();
    compiler.set_opt_level(2);

    let source = r#"
алг Тест
нач
    цел x
    x := 5 + 10
    вывод x
кон
"#;

    let result = compiler.check(source);
    assert!(
        result.is_ok(),
        "Программа с константными выражениями должна компилироваться"
    );
}
