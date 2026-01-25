//! Ключевые слова языка Кумир
//!
//! Содержит таблицу всех ключевых слов и их соответствие токенам.

use std::collections::HashMap;
use once_cell::sync::Lazy;

use crate::shared::types::Token;

/// Таблица ключевых слов языка Кумир.
/// 
/// Отображает русские ключевые слова на соответствующие токены.
pub static KEYWORDS: Lazy<HashMap<&'static str, Token>> = Lazy::new(|| {
    let mut m = HashMap::new();
    
    // =========================================================================
    //                    СТРУКТУРА АЛГОРИТМА (Kumir 2)
    // =========================================================================
    m.insert("алг", Token::Alg);
    m.insert("нач", Token::Begin);
    m.insert("кон", Token::End);
    m.insert("дано", Token::Given);
    m.insert("надо", Token::Need);
    m.insert("арг", Token::Arg);
    m.insert("рез", Token::Res);
    m.insert("аргрез", Token::ArgRes);
    
    // =========================================================================
    //                    БАЗОВЫЕ ТИПЫ (Kumir 2)
    // =========================================================================
    m.insert("цел", Token::IntType);
    m.insert("вещ", Token::FloatType);
    m.insert("лог", Token::BoolType);
    m.insert("сим", Token::CharType);
    m.insert("лит", Token::StringType);
    m.insert("таб", Token::ArrayType);
    
    // =========================================================================
    //                    РАСШИРЕННЫЕ ТИПЫ (Kumir 3)
    // =========================================================================
    m.insert("указатель", Token::PointerType);
    m.insert("перечисление", Token::EnumType);
    m.insert("авто", Token::AutoType);
    m.insert("необязательно", Token::OptionalType);
    m.insert("Необязательно", Token::OptionalType);
    m.insert("optional", Token::OptionalType);
    m.insert("Optional", Token::OptionalType);
    
    // =========================================================================
    //                    ЛОГИЧЕСКИЕ ОПЕРАТОРЫ И КОНСТАНТЫ
    // =========================================================================
    m.insert("и", Token::And);
    m.insert("или", Token::Or);
    m.insert("не", Token::Not);
    m.insert("да", Token::True);
    m.insert("нет", Token::False);
    
    // =========================================================================
    //                    УПРАВЛЕНИЕ ПОТОКОМ (Kumir 2)
    // =========================================================================
    m.insert("если", Token::If);
    m.insert("то", Token::Then);
    m.insert("иначе", Token::Else);
    m.insert("все", Token::Fi);
    m.insert("всё", Token::Fi);
    m.insert("выбор", Token::Switch);
    m.insert("при", Token::Case);
    m.insert("нц", Token::Loop);
    m.insert("кц", Token::EndLoop);
    m.insert("для", Token::For);
    m.insert("от", Token::From);
    m.insert("до", Token::To);
    m.insert("шаг", Token::Step);
    m.insert("пока", Token::While);
    
    // =========================================================================
    //                    ВВОД/ВЫВОД И УПРАВЛЕНИЕ (Kumir 2)
    // =========================================================================
    m.insert("ввод", Token::Input);
    m.insert("вывод", Token::Output);
    m.insert("утв", Token::Assert);
    m.insert("пауза", Token::Pause);
    m.insert("выход", Token::Halt);
    m.insert("использовать", Token::Use);
    m.insert("вернуть", Token::Return);
    m.insert("return", Token::Return);
    m.insert("знач", Token::ResultValue);
    
    // =========================================================================
    //                    МОДУЛИ И ИМПОРТ (Kumir 3)
    // =========================================================================
    m.insert("подключить", Token::Import);
    m.insert("модуль", Token::Module);
    m.insert("экспорт", Token::Export);
    
    // =========================================================================
    //                    РАБОТА С ПАМЯТЬЮ (Kumir 3)
    // =========================================================================
    m.insert("новый", Token::New);
    m.insert("удалить", Token::Delete);
    m.insert("ссылка", Token::Ref);
    m.insert("разыменовать", Token::Deref);
    
    // =========================================================================
    //                    PATTERN MATCHING (Kumir 3)
    // =========================================================================
    m.insert("совпадение", Token::Match);
    
    // =========================================================================
    //                    RUST-ВСТАВКИ (Kumir 3)
    // =========================================================================
    m.insert("РастВставкаНЦ", Token::RustBlockStart);
    m.insert("РастВставкаКЦ", Token::RustBlockEnd);
    m.insert("ржавчина", Token::Rust);
    m.insert("Ржавчина", Token::Rust);
    m.insert("rust", Token::Rust);
    
    // =========================================================================
    //                    ФУНКЦИОНАЛЬНОЕ ПРОГРАММИРОВАНИЕ (Kumir 3)
    // =========================================================================
    m.insert("лямбда", Token::Lambda);
    
    // =========================================================================
    //                    АСИНХРОННОЕ ПРОГРАММИРОВАНИЕ (Kumir 3)
    // =========================================================================
    m.insert("асинх", Token::Async);
    m.insert("ждать", Token::Await);
    
    // =========================================================================
    //                    ОБРАБОТКА ОШИБОК (Kumir 3)
    // =========================================================================
    m.insert("попытка", Token::Try);
    m.insert("перехват", Token::Catch);
    m.insert("бросить", Token::Throw);
    m.insert("наконец", Token::Finally);
    
    // =========================================================================
    //                    СПЕЦИАЛЬНЫЕ ЗНАЧЕНИЯ (Kumir 3)
    // =========================================================================
    m.insert("Пусто", Token::None);
    m.insert("пусто", Token::None);
    m.insert("None", Token::None);
    m.insert("none", Token::None);
    m.insert("НеРеализовано", Token::NotImplemented);
    m.insert("не_реализовано", Token::NotImplemented);
    m.insert("NotImplemented", Token::NotImplemented);
    m.insert("not_implemented", Token::NotImplemented);
    
    // =========================================================================
    //                    КЛАССЫ И ООП (Kumir 3)
    // =========================================================================
    
    // class
    m.insert("класс", Token::Class);
    m.insert("Класс", Token::Class);
    m.insert("class", Token::Class);
    m.insert("Class", Token::Class);
    
    // interface
    m.insert("интерфейс", Token::Interface);
    m.insert("Интерфейс", Token::Interface);
    m.insert("interface", Token::Interface);
    m.insert("Interface", Token::Interface);
    
    // private
    m.insert("приватное", Token::Private);
    m.insert("приват", Token::Private);
    m.insert("private", Token::Private);
    
    // public
    m.insert("публичное", Token::Public);
    m.insert("публ", Token::Public);
    m.insert("public", Token::Public);
    
    // protected
    m.insert("защищённое", Token::Protected);
    m.insert("защищ", Token::Protected);
    m.insert("protected", Token::Protected);
    
    // static
    m.insert("статическое", Token::Static);
    m.insert("стат", Token::Static);
    m.insert("static", Token::Static);
    
    // virtual
    m.insert("виртуальное", Token::Virtual);
    m.insert("вирт", Token::Virtual);
    m.insert("virtual", Token::Virtual);
    
    // override
    m.insert("переопределить", Token::Override);
    m.insert("переопр", Token::Override);
    m.insert("override", Token::Override);
    
    // abstract
    m.insert("абстрактное", Token::Abstract);
    m.insert("абстр", Token::Abstract);
    m.insert("abstract", Token::Abstract);
    
    // this/self
    m.insert("это", Token::This);
    m.insert("self", Token::This);
    m.insert("this", Token::This);
    
    // super
    m.insert("родитель", Token::Super);
    m.insert("супер", Token::Super);
    m.insert("super", Token::Super);
    
    // constructor
    m.insert("конструктор", Token::Constructor);
    m.insert("констр", Token::Constructor);
    m.insert("constructor", Token::Constructor);
    
    // destructor
    m.insert("деструктор", Token::Destructor);
    m.insert("дестр", Token::Destructor);
    m.insert("destructor", Token::Destructor);
    
    // extends
    m.insert("расширяет", Token::Extends);
    m.insert("extends", Token::Extends);
    
    // implements
    m.insert("реализует", Token::Implements);
    m.insert("implements", Token::Implements);
    
    m
});

/// Проверяет, является ли строка ключевым словом.
#[inline]
pub fn is_keyword(s: &str) -> bool {
    KEYWORDS.contains_key(s)
}

/// Возвращает токен для ключевого слова, если оно существует.
#[inline]
pub fn get_keyword_token(s: &str) -> Option<Token> {
    KEYWORDS.get(s).cloned()
}
