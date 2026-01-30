// ============================================================================
//                    ПРОМЕЖУТОЧНОЕ ПРЕДСТАВЛЕНИЕ (IR)
// ============================================================================
//
// IR - Intermediate Representation - промежуточное представление кода
// между AST и финальным кодом (машинным или байт-кодом).
//
// Это основа для компилятора КуМир 3.
//
// Архитектура:
//   AST (parser) → IR (этот модуль) → Backend (codegen)
//
// Бэкенды:
//   - LLVM IR → нативный код (Win/Linux/macOS)
//   - Cranelift → нативный код (альтернатива)
//   - WASM → WebAssembly для браузера
//   - Интерпретатор → прямое выполнение IR
//
// ============================================================================

use std::collections::HashMap;

/// Типы данных в IR
#[derive(Debug, Clone, PartialEq)]
pub enum IrType {
    /// Целое число (i64)
    Int,
    /// Вещественное число (f64)
    Float,
    /// Логический тип
    Bool,
    /// Символ (Unicode)
    Char,
    /// Строка
    String,
    /// Массив с типом элементов
    Array(Box<IrType>),
    /// Структура (запись)
    Struct(String),
    /// Указатель на тип
    Ptr(Box<IrType>),
    /// Функция (аргументы, возврат)
    Function(Vec<IrType>, Box<IrType>),
    /// Пустой тип (void)
    Void,
    /// Неизвестный тип (для вывода типов)
    Unknown,
}

impl IrType {
    /// Размер типа в байтах
    pub fn size(&self) -> usize {
        match self {
            IrType::Int => 8,
            IrType::Float => 8,
            IrType::Bool => 1,
            IrType::Char => 4, // UTF-8 codepoint
            IrType::String => 16, // ptr + len
            IrType::Array(_) => 24, // ptr + len + cap
            IrType::Struct(_) => 0, // зависит от полей
            IrType::Ptr(_) => 8,
            IrType::Function(_, _) => 8, // указатель на функцию
            IrType::Void => 0,
            IrType::Unknown => 0,
        }
    }

    /// Является ли тип числовым
    pub fn is_numeric(&self) -> bool {
        matches!(self, IrType::Int | IrType::Float)
    }
}

/// Значение в IR (константа)
#[derive(Debug, Clone, PartialEq)]
pub enum IrValue {
    Int(i64),
    Float(f64),
    Bool(bool),
    Char(char),
    String(String),
    Null,
}

/// Бинарные операции
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOp {
    // Арифметика
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Pow,

    // Сравнение
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,

    // Логические
    And,
    Or,

    // Битовые
    BitAnd,
    BitOr,
    BitXor,
    Shl,
    Shr,
}

/// Унарные операции
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOp {
    Neg,    // -x
    Not,    // не x
    BitNot, // ~x
}

/// Инструкция IR (SSA-подобная форма)
#[derive(Debug, Clone)]
pub enum IrInstruction {
    /// Загрузка константы: %dest = const value
    LoadConst {
        dest: VarId,
        value: IrValue,
    },

    /// Загрузка переменной: %dest = load %src
    Load {
        dest: VarId,
        src: VarId,
    },

    /// Сохранение: store %src, %dest
    Store {
        src: VarId,
        dest: VarId,
    },

    /// Бинарная операция: %dest = %left op %right
    BinaryOp {
        dest: VarId,
        op: BinaryOp,
        left: VarId,
        right: VarId,
    },

    /// Унарная операция: %dest = op %operand
    UnaryOp {
        dest: VarId,
        op: UnaryOp,
        operand: VarId,
    },

    /// Вызов функции: %dest = call func(%args...)
    Call {
        dest: Option<VarId>,
        func: FuncId,
        args: Vec<VarId>,
    },

    /// Вызов метода: %dest = call %obj.method(%args...)
    MethodCall {
        dest: Option<VarId>,
        object: VarId,
        method: String,
        args: Vec<VarId>,
    },

    /// Возврат из функции: ret %value
    Return {
        value: Option<VarId>,
    },

    /// Безусловный переход: br label
    Branch {
        target: BlockId,
    },

    /// Условный переход: br %cond, then_label, else_label
    CondBranch {
        condition: VarId,
        then_block: BlockId,
        else_block: BlockId,
    },

    /// Phi-функция (SSA): %dest = phi [%val1, block1], [%val2, block2]
    Phi {
        dest: VarId,
        incoming: Vec<(VarId, BlockId)>,
    },

    /// Доступ к элементу массива: %dest = %array[%index]
    ArrayGet {
        dest: VarId,
        array: VarId,
        index: VarId,
    },

    /// Запись в элемент массива: %array[%index] = %value
    ArraySet {
        array: VarId,
        index: VarId,
        value: VarId,
    },

    /// Доступ к полю структуры: %dest = %struct.field
    FieldGet {
        dest: VarId,
        object: VarId,
        field: String,
    },

    /// Запись в поле структуры: %struct.field = %value
    FieldSet {
        object: VarId,
        field: String,
        value: VarId,
    },

    /// Выделение памяти: %dest = alloc type
    Alloc {
        dest: VarId,
        typ: IrType,
    },

    /// Выделение массива: %dest = alloc_array type, %size
    AllocArray {
        dest: VarId,
        elem_type: IrType,
        size: VarId,
    },

    /// Приведение типа: %dest = cast %src to type
    Cast {
        dest: VarId,
        src: VarId,
        target_type: IrType,
    },

    /// Комментарий/отладочная информация
    Comment(String),

    /// Встроенный Rust-код (для Rust-вставок)
    InlineRust {
        code: String,
        inputs: Vec<VarId>,
        outputs: Vec<VarId>,
    },
}

/// Идентификатор переменной (SSA)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct VarId(pub u32);

/// Идентификатор функции
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct FuncId(pub String);

/// Идентификатор базового блока
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BlockId(pub u32);

/// Базовый блок (последовательность инструкций)
#[derive(Debug, Clone)]
pub struct BasicBlock {
    pub id: BlockId,
    pub label: String,
    pub instructions: Vec<IrInstruction>,
    /// Предшественники (для SSA)
    pub predecessors: Vec<BlockId>,
    /// Преемники
    pub successors: Vec<BlockId>,
}

impl BasicBlock {
    pub fn new(id: BlockId, label: &str) -> Self {
        Self {
            id,
            label: label.to_string(),
            instructions: Vec::new(),
            predecessors: Vec::new(),
            successors: Vec::new(),
        }
    }

    pub fn push(&mut self, instr: IrInstruction) {
        self.instructions.push(instr);
    }

    /// Возвращает терминатор блока (последняя инструкция)
    pub fn terminator(&self) -> Option<&IrInstruction> {
        self.instructions.last()
    }

    /// Проверяет, заканчивается ли блок терминатором
    pub fn is_terminated(&self) -> bool {
        matches!(
            self.terminator(),
            Some(IrInstruction::Branch { .. })
                | Some(IrInstruction::CondBranch { .. })
                | Some(IrInstruction::Return { .. })
        )
    }
}

/// IR-функция
#[derive(Debug, Clone)]
pub struct IrFunction {
    pub name: String,
    pub params: Vec<(String, IrType)>,
    pub return_type: IrType,
    pub blocks: Vec<BasicBlock>,
    pub locals: HashMap<String, (VarId, IrType)>,
    /// Входной блок
    pub entry_block: BlockId,
}

impl IrFunction {
    pub fn new(name: &str, params: Vec<(String, IrType)>, return_type: IrType) -> Self {
        let entry = BasicBlock::new(BlockId(0), "entry");
        Self {
            name: name.to_string(),
            params,
            return_type,
            blocks: vec![entry],
            locals: HashMap::new(),
            entry_block: BlockId(0),
        }
    }

    /// Добавляет новый блок
    pub fn add_block(&mut self, label: &str) -> BlockId {
        let id = BlockId(self.blocks.len() as u32);
        self.blocks.push(BasicBlock::new(id, label));
        id
    }

    /// Получает блок по ID
    pub fn get_block(&self, id: BlockId) -> Option<&BasicBlock> {
        self.blocks.get(id.0 as usize)
    }

    /// Получает изменяемый блок по ID
    pub fn get_block_mut(&mut self, id: BlockId) -> Option<&mut BasicBlock> {
        self.blocks.get_mut(id.0 as usize)
    }

    /// Добавляет инструкцию в блок
    pub fn push_to_block(&mut self, block_id: BlockId, instr: IrInstruction) {
        if let Some(block) = self.get_block_mut(block_id) {
            block.push(instr);
        }
    }
}

/// IR-модуль (единица компиляции)
#[derive(Debug, Clone)]
pub struct IrModule {
    pub name: String,
    pub functions: HashMap<String, IrFunction>,
    pub globals: HashMap<String, (IrType, Option<IrValue>)>,
    pub structs: HashMap<String, Vec<(String, IrType)>>,
    pub imports: Vec<String>,
}

impl IrModule {
    pub fn new(name: &str) -> Self {
        Self {
            name: name.to_string(),
            functions: HashMap::new(),
            globals: HashMap::new(),
            structs: HashMap::new(),
            imports: Vec::new(),
        }
    }

    pub fn add_function(&mut self, func: IrFunction) {
        self.functions.insert(func.name.clone(), func);
    }

    pub fn add_global(&mut self, name: &str, typ: IrType, init: Option<IrValue>) {
        self.globals.insert(name.to_string(), (typ, init));
    }

    pub fn add_struct(&mut self, name: &str, fields: Vec<(String, IrType)>) {
        self.structs.insert(name.to_string(), fields);
    }
}

/// Построитель IR (упрощает создание инструкций)
pub struct IrBuilder {
    module: IrModule,
    current_func: Option<String>,
    current_block: BlockId,
    next_var_id: u32,
}

impl IrBuilder {
    pub fn new(module_name: &str) -> Self {
        Self {
            module: IrModule::new(module_name),
            current_func: None,
            current_block: BlockId(0),
            next_var_id: 0,
        }
    }

    /// Создаёт новую переменную
    pub fn new_var(&mut self) -> VarId {
        let id = VarId(self.next_var_id);
        self.next_var_id += 1;
        id
    }

    /// Начинает новую функцию
    pub fn begin_function(&mut self, name: &str, params: Vec<(String, IrType)>, ret: IrType) {
        let func = IrFunction::new(name, params, ret);
        self.module.add_function(func);
        self.current_func = Some(name.to_string());
        self.current_block = BlockId(0);
    }

    /// Заканчивает текущую функцию
    pub fn end_function(&mut self) {
        self.current_func = None;
    }

    /// Добавляет инструкцию в текущий блок
    fn emit(&mut self, instr: IrInstruction) {
        if let Some(ref func_name) = self.current_func {
            if let Some(func) = self.module.functions.get_mut(func_name) {
                func.push_to_block(self.current_block, instr);
            }
        }
    }

    /// Загружает константу
    pub fn load_const(&mut self, value: IrValue) -> VarId {
        let dest = self.new_var();
        self.emit(IrInstruction::LoadConst { dest, value });
        dest
    }

    /// Загружает целое число
    pub fn load_int(&mut self, value: i64) -> VarId {
        self.load_const(IrValue::Int(value))
    }

    /// Загружает вещественное число
    pub fn load_float(&mut self, value: f64) -> VarId {
        self.load_const(IrValue::Float(value))
    }

    /// Загружает строку
    pub fn load_string(&mut self, value: &str) -> VarId {
        self.load_const(IrValue::String(value.to_string()))
    }

    /// Бинарная операция
    pub fn binary(&mut self, op: BinaryOp, left: VarId, right: VarId) -> VarId {
        let dest = self.new_var();
        self.emit(IrInstruction::BinaryOp { dest, op, left, right });
        dest
    }

    /// Унарная операция
    pub fn unary(&mut self, op: UnaryOp, operand: VarId) -> VarId {
        let dest = self.new_var();
        self.emit(IrInstruction::UnaryOp { dest, op, operand });
        dest
    }

    /// Вызов функции
    pub fn call(&mut self, func: &str, args: Vec<VarId>) -> Option<VarId> {
        let dest = Some(self.new_var());
        self.emit(IrInstruction::Call {
            dest,
            func: FuncId(func.to_string()),
            args,
        });
        dest
    }

    /// Вызов процедуры (без возврата)
    pub fn call_void(&mut self, func: &str, args: Vec<VarId>) {
        self.emit(IrInstruction::Call {
            dest: None,
            func: FuncId(func.to_string()),
            args,
        });
    }

    /// Возврат значения
    pub fn ret(&mut self, value: Option<VarId>) {
        self.emit(IrInstruction::Return { value });
    }

    /// Переключает текущий блок
    pub fn switch_block(&mut self, block: BlockId) {
        self.current_block = block;
    }

    /// Создаёт новый блок в текущей функции
    pub fn new_block(&mut self, label: &str) -> BlockId {
        if let Some(ref func_name) = self.current_func {
            if let Some(func) = self.module.functions.get_mut(func_name) {
                return func.add_block(label);
            }
        }
        BlockId(0)
    }

    /// Безусловный переход
    pub fn branch(&mut self, target: BlockId) {
        self.emit(IrInstruction::Branch { target });
    }

    /// Условный переход
    pub fn cond_branch(&mut self, cond: VarId, then_block: BlockId, else_block: BlockId) {
        self.emit(IrInstruction::CondBranch {
            condition: cond,
            then_block,
            else_block,
        });
    }

    /// Возвращает построенный модуль
    pub fn build(self) -> IrModule {
        self.module
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ir_builder_simple_function() {
        let mut builder = IrBuilder::new("test");

        builder.begin_function("add", vec![
            ("a".to_string(), IrType::Int),
            ("b".to_string(), IrType::Int),
        ], IrType::Int);

        let a = builder.load_int(10);
        let b = builder.load_int(20);
        let sum = builder.binary(BinaryOp::Add, a, b);
        builder.ret(Some(sum));

        builder.end_function();

        let module = builder.build();
        assert!(module.functions.contains_key("add"));

        let func = module.functions.get("add").unwrap();
        assert_eq!(func.name, "add");
        assert_eq!(func.params.len(), 2);
    }

    #[test]
    fn test_ir_basic_block() {
        let mut block = BasicBlock::new(BlockId(0), "entry");

        block.push(IrInstruction::LoadConst {
            dest: VarId(0),
            value: IrValue::Int(42),
        });
        block.push(IrInstruction::Return { value: Some(VarId(0)) });

        assert!(block.is_terminated());
        assert_eq!(block.instructions.len(), 2);
    }

    #[test]
    fn test_ir_type_size() {
        assert_eq!(IrType::Int.size(), 8);
        assert_eq!(IrType::Bool.size(), 1);
        assert_eq!(IrType::Void.size(), 0);
    }
}
