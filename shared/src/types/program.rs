//! Complete program in the Kumir language.

use super::algorithm::{Algorithm, OverloadedAlgorithm};
use super::class::ClassDef;
use super::stmt::Stmt;

/// Complete program in the Kumir language.
#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    /// Imported modules
    pub imports: Vec<Stmt>,

    /// Global variable declarations
    pub globals: Vec<Stmt>,

    /// Algorithm definitions
    pub algorithms: Vec<Algorithm>,

    /// Overloaded algorithms (Kumir 3)
    pub overloaded_algorithms: Vec<OverloadedAlgorithm>,

    /// Class definitions (Kumir 3)
    pub classes: Vec<ClassDef>,

    /// Interface definitions (Kumir 3)
    pub interfaces: Vec<Stmt>,

    /// Main algorithm (entry point)
    pub main: Option<Algorithm>,

    /// Warnings encountered during parsing
    pub warnings: Vec<String>,

    /// True if the program contained no algorithm declaration, and free statements
    /// were wrapped in an anonymous algorithm.
    ///
    /// This flag is needed for interactive mode: statements entered interactively
    /// are executed in the global scope, so variables declared above remain visible below.
    pub auto_wrapped: bool,
}
