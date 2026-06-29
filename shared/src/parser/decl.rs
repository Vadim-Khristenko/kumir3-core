//! Kumir 3 Parser — Declarations
//!
//! [STABLE] Parses top-level declarations: algorithms, modules, enums,
//! structs, interfaces, imports, variable declarations, and the `Program`
//! entry point.
//!
//! ## Architecture
//!
//! ```text
//! parse_program()
//!   │
//!   ├── parse_import()            → Stmt::Import
//!   ├── parse_var_decl_global()   → Stmt::VarDecl
//!   ├── parse_algorithm()         → Algorithm
//!   ├── parse_enum_decl()         → Stmt::EnumDecl
//!   ├── parse_module_decl()       → Stmt::ModuleDecl
//!   ├── parse_struct_decl()       → Stmt::StructDecl(ClassDef)
//!   ├── parse_interface_decl()    → oop.rs → Stmt::InterfaceDecl(InterfaceDef)
//!   ├── parse_trait_decl()        → oop.rs → Stmt::TraitDecl(TraitDef)
//!   ├── parse_impl_block()        → oop.rs → Stmt::ImplBlock(ImplDef)
//!   └── parse_class_decl()        → oop.rs → ClassDef
//! ```
//!
//! ## Key Design Decisions
//!
//! - `check_keyword()` / `match_keyword()` / `expect_keyword()` live in
//!   `core.rs` — no duplicate here.
//! - Span tracking uses `mark()` / `since()` for precise source mapping.
//! - `comma_sep()` and `many_until()` from core are used instead of
//!   manual `loop { ... if !match_token(Comma) { break } }`.
//! - Struct field names match `Stmt` variants **exactly**:
//!   `type_kind`, `modifiers`, `items: Option<Vec<String>>`, `exports`, etc.

use std::sync::Arc;

use super::core::Parser;
use super::error::{ParseError, ParseResult};
use crate::types::{
    Algorithm, AlgorithmKind, CallConvention, ClassDef, ClassKind, EffectFlags, EnumVariant,
    NodeId, ParamMode, Parameter, Program, SourceSpan, Stmt, Token, VarModifiers,
};

impl Parser {
    // =========================================================================
    //         SECTION: PROGRAM
    // =========================================================================

    /// Parses a complete Kumir program.
    ///
    /// A program is a sequence of top-level declarations: imports,
    /// global variables, algorithms, enums, modules, classes, structs,
    /// and interfaces.  If no `алг` keyword exists, the entire body
    /// is wrapped in an anonymous algorithm.
    pub fn parse_program(&mut self) -> ParseResult<Program> {
        let mut imports = Vec::new();
        let mut globals = Vec::new();
        let mut algorithms = Vec::new();
        let mut classes = Vec::new();
        let mut main = None;
        let mut warnings = Vec::new();

        self.skip_newlines();

        // Check whether the source contains at least one algorithm keyword.
        let has_algorithm = self.has_token_ahead(&Token::Alg);

        // ── bare code (no алг) ── wrap in anonymous algorithm ──────────
        if !has_algorithm {
            warnings.push(
                "Предупреждение: программа не содержит объявления алгоритма (алг).\n\
                 Код был автоматически обёрнут в анонимный алгоритм.\n\
                 Рекомендуется использовать структуру:\n\
                 \n\
                   алг\n\
                   нач\n\
                     <ваш код>\n\
                   кон\n"
                    .to_string(),
            );

            let body = self.parse_bare_statements()?;

            main = Some(Algorithm {
                id: NodeId::default(),
                name: Arc::from(""),
                kind: AlgorithmKind::Procedure,
                type_params: Vec::new(),
                return_type: None,
                params: Vec::new(),
                precondition: None,
                postcondition: None,
                body: Some(body),
                effects: EffectFlags::default(),
                attributes: Vec::new(),
                call_conv: CallConvention::Kumir,
                span: None,
                doc: None,
            });

            return Ok(Program {
                imports,
                globals,
                algorithms,
                overloaded_algorithms: Vec::new(),
                classes,
                interfaces: Vec::new(),
                main,
                warnings,
            });
        }

        // ── standard structured program ────────────────────────────────
        while !self.is_eof() {
            match self.peek() {
                // Import: подключить / использовать
                Token::Import | Token::Use => imports.push(self.parse_import()?),

                // Algorithm
                Token::Alg => {
                    let alg = self.parse_algorithm()?;
                    if alg.name.is_empty() || alg.name.as_ref() == "главный" {
                        main = Some(alg);
                    } else {
                        algorithms.push(alg);
                    }
                }

                // Global variable declaration (starts with a type keyword)
                Token::IntType
                | Token::FloatType
                | Token::BoolType
                | Token::CharType
                | Token::StringType
                | Token::ArrayType
                | Token::AutoType
                | Token::PointerType => {
                    globals.push(self.parse_var_decl_global()?);
                }

                Token::EnumType => globals.push(self.parse_enum_decl()?),
                Token::Module => globals.push(self.parse_module_decl()?),

                // OOP: class/struct/interface/trait/impl
                // Abstract classes start with Token::Abstract
                Token::Class | Token::Abstract => {
                    classes.push(self.parse_class_decl()?);
                }
                Token::Struct => globals.push(self.parse_struct_decl()?),
                Token::Interface => {
                    let iface = self.parse_interface_decl()?;
                    globals.push(Stmt::InterfaceDecl(iface));
                }
                Token::Trait => {
                    let trait_def = self.parse_trait_decl()?;
                    globals.push(Stmt::TraitDecl(trait_def));
                }
                Token::Impl => {
                    let impl_def = self.parse_impl_block()?;
                    globals.push(Stmt::ImplBlock(impl_def));
                }

                Token::Newline | Token::Comment(_) => {
                    self.advance();
                }

                _ => {
                    return Err(ParseError::unexpected(
                        "объявление алгоритма или переменной",
                        self.peek(),
                        self.span(),
                    )
                    .into());
                }
            }
        }

        Ok(Program {
            imports,
            globals,
            algorithms,
            overloaded_algorithms: Vec::new(),
            classes,
            interfaces: Vec::new(),
            main,
            warnings,
        })
    }

    // =========================================================================
    //         SECTION: BARE STATEMENTS (no алг / нач / кон wrapper)
    // =========================================================================

    /// Parses a flat list of statements (no enclosing block).
    ///
    /// Used when the source has no `алг` keyword and the whole file is
    /// treated as an anonymous algorithm body.
    fn parse_bare_statements(&mut self) -> ParseResult<Vec<Stmt>> {
        let mut stmts = Vec::new();
        while !self.is_eof() {
            self.skip_newlines();
            if self.is_eof() {
                break;
            }
            stmts.push(self.parse_stmt()?);
        }
        Ok(stmts)
    }

    // =========================================================================
    //         SECTION: IMPORT
    // =========================================================================

    /// Parses an import statement.
    ///
    /// ```text
    /// подключить "lib.kum"
    /// использовать МояБиблиотека: функция1, функция2
    /// использовать Мат как М
    /// ```
    fn parse_import(&mut self) -> ParseResult<Stmt> {
        // Accept both подключить and использовать
        if !self.match_token(&Token::Import) && !self.match_token(&Token::Use) {
            return Err(ParseError::unexpected(
                "подключить или использовать",
                self.peek(),
                self.span(),
            )
            .into());
        }

        let path = self.expect_import_path()?;

        // Selective import: использовать Модуль: имя1, имя2
        let items = if self.match_token(&Token::Colon) {
            let mut list = Vec::new();
            list.push(self.expect_ident("имя элемента")?);
            while self.match_token(&Token::Comma) {
                list.push(self.expect_ident("имя элемента")?);
            }
            Some(list)
        } else {
            None
        };

        // Optional alias: как Алиас
        let alias = if self.check_keyword("как") {
            self.advance();
            Some(self.expect_ident("алиас")?)
        } else {
            None
        };

        self.expect_eol()?;
        Ok(Stmt::Import { path, alias, items })
    }

    // =========================================================================
    //         SECTION: ALGORITHM
    // =========================================================================

    /// Parses a full algorithm definition.
    ///
    /// ```text
    /// алг [async] [<return_type>] Name(<params>)
    /// дано <precondition>
    /// надо <postcondition>
    /// нач
    ///   <body>
    /// кон
    /// ```
    pub fn parse_algorithm(&mut self) -> ParseResult<Algorithm> {
        let m = self.mark();
        self.expect(&Token::Alg, "алг")?;

        // ── modifiers ──
        let mut effects = EffectFlags::default();
        if self.match_token(&Token::Async) {
            effects.is_async = true;
        }

        // ── optional return type (before name): алг цел Сумма(...) ──
        let mut return_type = self.try_parse_type();

        // ── name ──
        let name: Arc<str> = if self.is_ident() {
            Arc::from(self.expect_ident("имя алгоритма")?.as_str())
        } else {
            Arc::from("")
        };

        let kind = if return_type.is_some() {
            AlgorithmKind::Function
        } else {
            AlgorithmKind::Procedure
        };

        // ── parameters ──
        let params = if self.match_token(&Token::LParen) {
            let p = self.parse_params()?;
            self.expect(&Token::RParen, ")")?;
            p
        } else {
            Vec::new()
        };

        // ── return type after params: алг Имя(...): Тип ──
        if self.match_token(&Token::Colon) {
            return_type = Some(self.parse_type()?);
        }

        self.skip_newlines();

        // ── precondition (дано) ──
        let precondition = if self.match_token(&Token::Given) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else {
            None
        };

        // ── postcondition (надо) ──
        let postcondition = if self.match_token(&Token::Need) {
            let e = self.parse_expr()?;
            self.skip_newlines();
            Some(e)
        } else {
            None
        };

        // ── body ──
        self.expect(&Token::Begin, "нач")?;
        self.skip_newlines();

        let body = self.parse_stmts_until(&[Token::End])?;

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(Algorithm {
            id: NodeId::default(),
            name,
            kind,
            type_params: Vec::new(),
            return_type,
            params,
            precondition,
            postcondition,
            body: Some(body),
            effects,
            attributes: Vec::new(),
            call_conv: CallConvention::Kumir,
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        })
    }

    // =========================================================================
    //         SECTION: PARAMETERS
    // =========================================================================

    /// Parses a comma-separated parameter list (between parentheses).
    fn parse_params(&mut self) -> ParseResult<Vec<Parameter>> {
        if self.check(&Token::RParen) {
            return Ok(Vec::new());
        }
        self.comma_sep(&Token::RParen, |p| p.parse_param())
    }

    /// Parses a single parameter.
    ///
    /// Supports two syntaxes:
    /// 1. Classic Kumir:  `арг цел x`
    /// 2. Modern:         `x: цел`  or  `запрос: HTTPЗапрос`
    fn parse_param(&mut self) -> ParseResult<Parameter> {
        // ── optional mode keyword: арг / рез / аргрез ──
        let mode = if self.match_token(&Token::Arg) {
            ParamMode::In
        } else if self.match_token(&Token::Res) {
            ParamMode::Out
        } else if self.match_token(&Token::ArgRes) {
            ParamMode::InOut
        } else {
            ParamMode::In
        };

        // ── modern syntax: name : Type ──
        if self.is_ident() && matches!(self.peek_n(1), Token::Colon) {
            let name = Arc::from(self.expect_ident("имя параметра")?.as_str());
            self.expect(&Token::Colon, ":")?;
            let type_kind = Some(self.parse_type()?);

            let default = if self.match_token(&Token::Assign) {
                Some(self.parse_expr()?)
            } else {
                None
            };

            return Ok(Parameter {
                id: NodeId::default(),
                name,
                type_kind,
                mode,
                default,
                attributes: Vec::new(),
                span: None,
            });
        }

        // ── classic syntax: type name ──
        let type_kind = Some(self.parse_type()?);
        let name = Arc::from(self.expect_ident("имя параметра")?.as_str());

        let default = if self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        Ok(Parameter {
            id: NodeId::default(),
            name,
            type_kind,
            mode,
            default,
            attributes: Vec::new(),
            span: None,
        })
    }

    // =========================================================================
    //         SECTION: GLOBAL VARIABLE DECLARATION
    // =========================================================================

    /// Parses a global variable declaration.
    ///
    /// ```text
    /// цел x
    /// вещ a, b, c
    /// лит имя := "Привет"
    /// ```
    fn parse_var_decl_global(&mut self) -> ParseResult<Stmt> {
        let type_kind = self.parse_type()?;

        let mut names = Vec::new();
        names.push(self.expect_ident("имя переменной")?);

        while self.match_token(&Token::Comma) {
            names.push(self.expect_ident("имя переменной")?);
        }

        // Initialiser only allowed for a single variable
        let init = if names.len() == 1 && self.match_token(&Token::Assign) {
            Some(self.parse_expr()?)
        } else {
            None
        };

        self.expect_eol()?;
        Ok(Stmt::VarDecl {
            type_kind,
            names,
            init,
            modifiers: VarModifiers::default(),
        })
    }

    // =========================================================================
    //         SECTION: ENUM
    // =========================================================================

    /// Parses an enum declaration.
    ///
    /// ```text
    /// перечисление Цвет
    ///   Красный
    ///   Зелёный
    ///   Синий(цел)
    /// кон
    /// ```
    fn parse_enum_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::EnumType, "перечисление")?;
        let name = self.expect_ident("имя перечисления")?;
        self.skip_newlines();

        let variants = self.many_until(&[Token::End], |p| {
            let variant_name = p.expect_ident("вариант перечисления")?;

            let data = if p.match_token(&Token::LParen) {
                let t = p.parse_type()?;
                p.expect(&Token::RParen, ")")?;
                Some(t)
            } else {
                None
            };

            p.skip_newlines();
            Ok(EnumVariant {
                name: variant_name,
                data,
                doc: None,
            })
        })?;

        self.expect(&Token::End, "кон")?;
        self.skip_newlines();

        Ok(Stmt::EnumDecl {
            name,
            variants,
            generics: Vec::new(),
        })
    }

    // =========================================================================
    //         SECTION: MODULE
    // =========================================================================

    /// Parses a module declaration.
    ///
    /// ```text
    /// модуль Математика
    ///   цел Пи_приближение := 3
    ///   алг цел Удвоить(арг цел x)
    ///   нач
    ///     знач := x * 2
    ///   кон
    /// кон
    /// ```
    fn parse_module_decl(&mut self) -> ParseResult<Stmt> {
        self.expect(&Token::Module, "модуль")?;
        let name = self.expect_ident("имя модуля")?;
        self.skip_newlines();

        let mut body = Vec::new();
        let mut algorithms = Vec::new();

        while !self.is_eof() && !self.check(&Token::End) {
            match self.peek().clone() {
                Token::Alg => {
                    algorithms.push(self.parse_algorithm()?);
                }

                Token::IntType
                | Token::FloatType
                | Token::BoolType
                | Token::CharType
                | Token::StringType
                | Token::ArrayType
                | Token::AutoType
                | Token::PointerType => {
                    body.push(self.parse_var_decl_global()?);
                }

                Token::Newline | Token::Comment(_) => {
                    self.advance();
                }

                _ => {
                    return Err(ParseError::unexpected(
                        "алгоритм или объявление переменной в модуле",
                        self.peek(),
                        self.span(),
                    )
                    .into());
                }
            }
        }

        self.expect(&Token::End, "кон")?;
        self.skip_newlines();

        Ok(Stmt::ModuleDecl {
            name,
            body,
            algorithms,
            exports: Vec::new(),
        })
    }

    // =========================================================================
    //         SECTION: STRUCT (simple data-only)
    // =========================================================================

    /// Parses a simple struct declaration (data only, no methods).
    ///
    /// Creates a `ClassDef` with `ClassKind::Struct` and wraps it
    /// in `Stmt::StructDecl`. For full struct-with-methods syntax,
    /// use `parse_class_decl` from `oop.rs` directly.
    ///
    /// ```text
    /// структура Точка
    ///   вещ x
    ///   вещ y
    /// кон
    /// ```
    fn parse_struct_decl(&mut self) -> ParseResult<Stmt> {
        let m = self.mark();
        self.expect(&Token::Struct, "структура")?;
        let name = self.expect_ident("имя структуры")?;
        self.skip_newlines();

        let fields = self.parse_fields_until(&Token::End)?;

        self.expect(&Token::End, "кон")?;
        let span = self.since(m);
        self.skip_newlines();

        Ok(Stmt::StructDecl(ClassDef {
            id: NodeId::default(),
            name: Arc::from(name.as_str()),
            kind: ClassKind::Struct,
            type_params: Vec::new(),
            parent: None,
            interfaces: Vec::new(),
            traits: Vec::new(),
            fields,
            methods: Vec::new(),
            constructors: Vec::new(),
            destructor: None,
            is_abstract: false,
            is_final: false,
            attributes: Vec::new(),
            span: Some(SourceSpan {
                file_id: None,
                start: span.start.offset,
                end: span.end.offset,
            }),
            doc: None,
        }))
    }

    // NOTE: parse_interface_decl(), parse_trait_decl(), parse_impl_block()
    // live in oop.rs. decl.rs calls them from parse_program() and wraps
    // the result in Stmt::InterfaceDecl / Stmt::TraitDecl / Stmt::ImplBlock.
}
