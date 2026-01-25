// ============================================================================
//                         ПАРСЕР ЯЗЫКА КУМИР 3
// ============================================================================
//
// Модульная структура парсера:
//
// - error.rs      — типы ошибок и результаты
// - precedence.rs — приоритеты операторов
// - core.rs       — ядро парсера (навигация по токенам)
// - types.rs      — парсинг типов
// - expr.rs       — парсинг выражений
// - pattern.rs    — парсинг паттернов для match
// - stmt.rs       — парсинг инструкций
// - decl.rs       — парсинг объявлений (алгоритмы, модули, enum)
// - oop.rs        — парсинг классов и ООП
//
// ============================================================================

mod error;
mod precedence;
mod core;
mod types;
mod expr;
mod pattern;
mod stmt;
mod decl;
mod oop;

// Реэкспорты
pub use error::{ParseError, ParseResult};
pub use core::Parser;

use crate::shared::types::{Program, Expr};

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
    use crate::shared::types::{Token, Stmt, Value, Number, TypeSpec};
    
    #[test]
    fn test_simple_algorithm() {
        let program = parse("алг Тест\nнач\nкон\n").unwrap();
        assert_eq!(program.algorithms.len(), 1);
        assert_eq!(program.algorithms[0].name, "Тест");
    }
    
    #[test]
    fn test_algorithm_with_return_type() {
        let program = parse("алг цел Сумма(арг цел a, арг цел b)\nнач\nкон\n").unwrap();
        assert_eq!(program.algorithms[0].return_type, Some(TypeSpec::Int));
        assert_eq!(program.algorithms[0].params.len(), 2);
    }
    
    #[test]
    fn test_if_statement() {
        let program = parse("алг Тест\nнач\nесли x > 0 то\nвывод 1\nвсе\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::If { .. }));
    }
    
    #[test]
    fn test_for_loop() {
        let program = parse("алг Тест\nнач\nнц для i от 1 до 10\nвывод i\nкц\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::LoopFor { .. }));
    }
    
    #[test]
    fn test_expression_precedence() {
        let expr = parse_expression("2 + 3 * 4").unwrap();
        // Должно быть: 2 + (3 * 4)
        if let Expr::BinaryOp(left, Token::Plus, right) = expr {
            assert!(matches!(*left, Expr::Literal(Value::Number(Number::I64(2)))));
            assert!(matches!(*right, Expr::BinaryOp(_, Token::Star, _)));
        } else {
            panic!("Expected BinaryOp");
        }
    }
    
    #[test]
    fn test_function_call() {
        let expr = parse_expression("sin(x)").unwrap();
        assert!(matches!(expr, Expr::Call(name, _) if name == "sin"));
    }
    
    #[test]
    fn test_array_access() {
        let expr = parse_expression("arr[i, j]").unwrap();
        assert!(matches!(expr, Expr::ArrayAccess(name, idx) if name == "arr" && idx.len() == 2));
    }
    
    #[test]
    fn test_lambda() {
        let expr = parse_expression("лямбда(x, y) -> x + y").unwrap();
        assert!(matches!(expr, Expr::Lambda { params, .. } if params.len() == 2));
    }
    
    #[test]
    fn test_var_declaration() {
        let program = parse("алг Тест\nнач\nцел x := 42\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::VarDecl { .. }));
    }
    
    #[test]
    fn test_auto_var() {
        let program = parse("алг Тест\nнач\nавто x := 42\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::AutoVarDecl { name, .. } if name == "x"));
    }
    
    #[test]
    fn test_input_output() {
        let program = parse("алг Тест\nнач\nввод x, y\nвывод x + y\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::Input(vars) if vars.len() == 2));
        assert!(matches!(&program.algorithms[0].body[1], Stmt::Output(_)));
    }
    
    #[test]
    fn test_import() {
        let program = parse("подключить \"math.kum\"\nалг Тест\nнач\nкон\n").unwrap();
        assert_eq!(program.imports.len(), 1);
    }
    
    #[test]
    fn test_enum() {
        let program = parse("перечисление Цвет\nКрасный\nЗелёный\nСиний\nкон\nалг Тест\nнач\nкон\n").unwrap();
        assert!(matches!(&program.globals[0], Stmt::EnumDecl { name, variants } if name == "Цвет" && variants.len() == 3));
    }
    
    #[test]
    fn test_try_catch() {
        let program = parse("алг Тест\nнач\nпопытка\nвывод 1\nперехват e\nвывод e\nкон\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::TryCatch { .. }));
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
    
    #[test]
    fn test_logical_operators() {
        let expr = parse_expression("a и b или не c").unwrap();
        assert!(matches!(expr, Expr::BinaryOp(_, Token::Or, _)));
    }
    
    #[test]
    fn test_compound_assignment() {
        let program = parse("алг Тест\nнач\nх += 1\nкон\n").unwrap();
        assert!(matches!(&program.algorithms[0].body[0], Stmt::Assignment(_, Expr::BinaryOp(_, Token::Plus, _))));
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
    
    #[test]
    fn test_self_ref() {
        let expr = parse_expression("это").unwrap();
        assert!(matches!(expr, Expr::SelfRef));
    }
    
    #[test]
    fn test_field_access() {
        let expr = parse_expression("объект.поле").unwrap();
        assert!(matches!(expr, Expr::FieldAccess(_, field) if field == "поле"));
    }
    
    #[test]
    fn test_method_call() {
        let expr = parse_expression("объект.метод(1, 2)").unwrap();
        assert!(matches!(expr, Expr::MethodCall { method, args, .. } if method == "метод" && args.len() == 2));
    }
    
    #[test]
    fn test_new_instance() {
        let expr = parse_expression("новый Точка(1, 2)").unwrap();
        assert!(matches!(expr, Expr::NewInstance { class_name, args } if class_name == "Точка" && args.len() == 2));
    }
}
