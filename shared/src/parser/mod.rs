// ============================================================================
//                         ПАРСЕР ЯЗЫКА КУМИР 3
// ============================================================================
//
// Модульная структура парсера:
//
// - error.rs      — типы ошибок и результаты
// - precedence.rs — приоритеты операторов
// - core.rs       — ядро парсера (навигация по токенам)
// - types.rs      — парсинг типов (TypeKind, generics)
// - expr.rs       — парсинг выражений (Expr, BinaryOp, Call, OOP, …)
// - pattern.rs    — парсинг паттернов для match / деструктуризации
// - stmt.rs       — парсинг инструкций (Stmt, циклы, if, match, …)
// - decl.rs       — парсинг объявлений (алгоритмы, модули, enum, Program)
// - oop.rs        — парсинг классов и ООП (ClassDef, Interface, Trait, Impl)
//
// ============================================================================

mod core;
mod decl;
mod error;
mod expr;
mod oop;
mod pattern;
mod precedence;
mod stmt;
mod types;

// Реэкспорты
pub use core::Parser;
pub use error::{ParseError, ParseResult};

use crate::types::{Expr, Program};

// ============================================================================
//                         УДОБНЫЕ ФУНКЦИИ
// ============================================================================

/// Парсить исходный код и вернуть программу.
pub fn parse(source: &str) -> ParseResult<Program> {
    Parser::new(source)?.parse_program()
}

/// Парсить одно выражение.
pub fn parse_expression(source: &str) -> ParseResult<Expr> {
    Parser::new(source)?.parse_expr()
}

// ============================================================================
//                         ТЕСТЫ
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::{Expr, Number, Stmt, Token, TypeKind, Value};

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Algorithms — basic parsing
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_simple_algorithm() {
        let program = parse("алг Тест\nнач\nкон\n").unwrap();
        assert_eq!(program.algorithms.len(), 1);
        assert_eq!(program.algorithms[0].name.as_ref(), "Тест");
    }

    #[test]
    fn test_algorithm_with_return_type() {
        let program = parse("алг цел Сумма(арг цел a, арг цел b)\nнач\nкон\n").unwrap();
        let alg = &program.algorithms[0];
        assert_eq!(alg.return_type, Some(TypeKind::Int64));
        assert_eq!(alg.params.len(), 2);
        assert_eq!(alg.params[0].name.as_ref(), "a");
        assert_eq!(alg.params[1].name.as_ref(), "b");
    }

    #[test]
    fn test_main_algorithm_anonymous() {
        let program = parse("алг\nнач\nкон\n").unwrap();
        assert!(program.main.is_some());
        assert!(program.main.as_ref().unwrap().name.is_empty());
    }

    #[test]
    fn test_bare_code_wraps_in_anon_alg() {
        let program = parse("вывод 42\n").unwrap();
        assert!(program.main.is_some());
        assert!(!program.warnings.is_empty());
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Statements
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_if_statement() {
        let program = parse("алг Тест\nнач\nесли x > 0 то\nвывод 1\nвсе\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(&body[0], Stmt::If { .. }));
    }

    #[test]
    fn test_for_loop() {
        let program = parse("алг Тест\nнач\nнц для i от 1 до 10\nвывод i\nкц\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(&body[0], Stmt::LoopFor { .. }));
    }

    #[test]
    fn test_var_declaration() {
        let program = parse("алг Тест\nнач\nцел x := 42\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(
            &body[0],
            Stmt::VarDecl {
                type_kind: TypeKind::Int64,
                ..
            }
        ));
    }

    #[test]
    fn test_auto_var() {
        let program = parse("алг Тест\nнач\nавто x := 42\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(&body[0], Stmt::AutoVarDecl { name, .. } if name == "x"));
    }

    #[test]
    fn test_input_output() {
        let program = parse("алг Тест\nнач\nввод x, y\nвывод x + y\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(&body[0], Stmt::Input(vars) if vars.len() == 2));
        assert!(matches!(&body[1], Stmt::Output(_)));
    }

    #[test]
    fn test_try_catch() {
        let program =
            parse("алг Тест\nнач\nпопытка\nвывод 1\nперехват e\nвывод e\nкон\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        assert!(matches!(&body[0], Stmt::TryCatch { .. }));
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Expressions — operators and precedence
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_expression_precedence() {
        let expr = parse_expression("2 + 3 * 4").unwrap();
        // Должно быть: 2 + (3 * 4)
        if let Expr::BinaryOp(left, Token::Plus, right) = &expr {
            assert!(matches!(
                left.as_ref(),
                Expr::Literal(Value::Number(Number::I64(2)))
            ));
            assert!(matches!(right.as_ref(), Expr::BinaryOp(_, Token::Star, _)));
        } else {
            panic!("Expected BinaryOp(+), got: {:?}", expr);
        }
    }

    #[test]
    fn test_logical_operators() {
        let expr = parse_expression("a и b или не c").unwrap();
        // или имеет меньший приоритет чем и
        assert!(matches!(expr, Expr::BinaryOp(_, Token::Or, _)));
    }

    #[test]
    fn test_none_literal() {
        let expr = parse_expression("Пусто").unwrap();
        assert!(matches!(expr, Expr::None));
    }

    #[test]
    fn test_not_implemented() {
        let expr = parse_expression("НеРеализовано").unwrap();
        assert!(matches!(expr, Expr::NotImplemented(None)));
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Expressions — calls and access
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_function_call() {
        let expr = parse_expression("sin(x)").unwrap();
        assert!(matches!(expr, Expr::Call(ref name, ref args) if name == "sin" && args.len() == 1));
    }

    #[test]
    fn test_array_access() {
        let expr = parse_expression("arr[i, j]").unwrap();
        assert!(
            matches!(expr, Expr::ArrayAccess(ref name, ref idx) if name == "arr" && idx.len() == 2)
        );
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Expressions — functional
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_lambda() {
        let expr = parse_expression("лямбда(x, y) -> x + y").unwrap();
        assert!(matches!(expr, Expr::Lambda { ref params, .. } if params.len() == 2));
    }

    #[test]
    fn test_pipe() {
        let expr = parse_expression("x |> f |> g").unwrap();
        assert!(matches!(expr, Expr::Pipe(_, _)));
    }

    #[test]
    fn test_conditional_expr() {
        let expr = parse_expression("если x > 0 то x иначе 0 все").unwrap();
        assert!(matches!(expr, Expr::IfExpr { .. }));
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Expressions — OOP
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_self_ref() {
        let expr = parse_expression("это").unwrap();
        assert!(matches!(expr, Expr::SelfRef));
    }

    #[test]
    fn test_field_access() {
        let expr = parse_expression("объект.поле").unwrap();
        assert!(matches!(expr, Expr::FieldAccess(_, ref field) if field == "поле"));
    }

    #[test]
    fn test_method_call() {
        let expr = parse_expression("объект.метод(1, 2)").unwrap();
        assert!(matches!(
            expr,
            Expr::MethodCall { ref method, ref args, .. }
            if method == "метод" && args.len() == 2
        ));
    }

    #[test]
    fn test_new_instance() {
        let expr = parse_expression("новый Точка(1, 2)").unwrap();
        assert!(matches!(
            expr,
            Expr::NewInstance { ref class_name, ref args }
            if class_name == "Точка" && args.len() == 2
        ));
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Declarations — top-level
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_import() {
        let program = parse("подключить \"math.kum\"\nалг Тест\nнач\nкон\n").unwrap();
        assert_eq!(program.imports.len(), 1);
        assert!(matches!(&program.imports[0], Stmt::Import { path, .. } if path == "math.kum"));
    }

    #[test]
    fn test_enum() {
        let src = "перечисление Цвет\nКрасный\nЗелёный\nСиний\nкон\nалг Тест\nнач\nкон\n";
        let program = parse(src).unwrap();
        assert!(matches!(
            &program.globals[0],
            Stmt::EnumDecl { name, variants, .. }
            if name == "Цвет" && variants.len() == 3
        ));
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: OOP — classes, structs, interfaces
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_struct_decl() {
        let src = "структура Точка\nвещ x\nвещ y\nкон\nалг Тест\nнач\nкон\n";
        let program = parse(src).unwrap();
        match &program.globals[0] {
            Stmt::StructDecl(class_def) => {
                assert_eq!(class_def.name.as_ref(), "Точка");
                assert_eq!(class_def.fields.len(), 2);
                assert_eq!(class_def.fields[0].name.as_ref(), "x");
                assert_eq!(class_def.fields[1].name.as_ref(), "y");
            }
            other => panic!("Expected StructDecl, got: {:?}", other),
        }
    }

    #[test]
    fn test_interface_decl() {
        let src = "интерфейс Рисуемый\nалг Нарисовать()\nкон\nалг Тест\nнач\nкон\n";
        let program = parse(src).unwrap();
        match &program.globals[0] {
            Stmt::InterfaceDecl(iface) => {
                assert_eq!(iface.name.as_ref(), "Рисуемый");
                assert_eq!(iface.methods.len(), 1);
                assert_eq!(iface.methods[0].name.as_ref(), "Нарисовать");
            }
            other => panic!("Expected InterfaceDecl, got: {:?}", other),
        }
    }

    #[test]
    fn test_class_decl_basic() {
        let src = "класс Фигура\nалг Площадь(): вещ\nнач\nкон\nкон\nалг Тест\nнач\nкон\n";
        let program = parse(src).unwrap();
        assert_eq!(program.classes.len(), 1);
        let cls = &program.classes[0];
        assert_eq!(cls.name.as_ref(), "Фигура");
        assert_eq!(cls.methods.len(), 1);
    }

    #[test]
    fn test_class_with_fields_and_constructor() {
        let src = "\
класс Точка
вещ x
вещ y
конструктор(арг вещ ax, арг вещ ay)
нач
кон
кон
алг Тест
нач
кон
";
        let program = parse(src).unwrap();
        let cls = &program.classes[0];
        assert_eq!(cls.name.as_ref(), "Точка");
        assert_eq!(cls.fields.len(), 2);
        assert_eq!(cls.constructors.len(), 1);
    }

    // ────────────────────────────────────────────────────────────────
    //  SECTION: Compound assignment
    // ────────────────────────────────────────────────────────────────

    #[test]
    fn test_compound_assignment() {
        let program = parse("алг Тест\nнач\nх += 1\nкон\n").unwrap();
        let body = program.algorithms[0].body.as_ref().unwrap();
        // += desugars into Assignment(x, BinaryOp(x, +, 1))
        assert!(matches!(
            &body[0],
            Stmt::Assignment(_, Expr::BinaryOp(_, Token::Plus, _))
        ));
    }
}
