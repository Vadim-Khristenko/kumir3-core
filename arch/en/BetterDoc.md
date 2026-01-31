# 📜 BetterDoc (BDoc) Standard — kumir3-core

**Document Version:** 1.2.0

**Project:** Kumir3

**Core Principle:** Architecture as Art. Code as Narrative.

---

## 🏗 1. Architectural Sectioning

To maintain visual order in `kumir3-core`, every file must be divided into logical blocks. This allows for quick navigation even when using the IDE "mini-map."

Use fixed-length banners (80 characters):

```rust
// =============================================================================
//         SECTION: [SECTION NAME IN UPPERCASE]
// =============================================================================

```

**Recommended Section Order:**

1. **IMPORTS:** `use` directives (first `std`, then external crates, then local `crate::`).
2. **CONSTANTS & MACROS:** Public and internal constants.
3. **TYPES:** Declarations of `struct`, `enum`, `union`, and `type`.
4. **TRAITS:** Declarations of new traits.
5. **TRAIT IMPLS:** Trait implementations for the module's types.
6. **CORE LOGIC:** Primary `impl` blocks for methods.
7. **UNIT TESTS:** Internal `mod tests`.

---

## 🏷 2. Status Hierarchy (Status Tags)

Every public entity must start with a status tag. This serves as the primary signal to the developer regarding the reliability of the code segment.

| Tag | Color Code | Meaning |
| --- | --- | --- |
| **`[STABLE]`** | 🟢 Green | Code is tested, documented, and architecturally complete. |
| **`[EXPERIMENTAL]`** | 🟡 Yellow | New functionality. Interface may change without notice. |
| **`[STILL-COOKING]`** | 🟠 Orange | Draft. Logic is incomplete. Do not use in production paths. |
| **`[DEPRECATED]`** | 🔴 Red | Obsolete. Must specify: `-> Use [Name] instead`. |
| **`[LEGACY-HAIRBALL]`** | 🧶 Hairball | Tangled old code. High regression risk. Requires refactoring. |
| **`[SAFETY-CRITICAL]`** | ⚡ Bolt | Contains `unsafe`, FFI, or critical memory management. |
| **`[PERF-SENSITIVE]`** | 🚀 Rocket | Hot path. Optimization takes priority over readability. |
| **`[PLATFORM-SPECIFIC]`** | 🖥 OS | Behavior depends on the operating system. |
| **`[FIXME-SOON]`** | 🩹 Bandage | Temporary hack. Must be replaced in the next minor release. |
| **`[MEME-INSIDE]`** | 🐾 Paw | Contains references to Astolfo, cats, or Monster Energy. |

---

## 📚 3. Entity Documentation

### 3.1. Type Level (Structs / Enums)

Documentation should explain the object's role within the "house ecosystem."

* **Essence:** What is this?
* **Context:** Why does it exist? (Light humor is permitted).
* **Resources:** Requirements for operation (e.g., "Requires an initialized LLVM context").

### 3.2. Function Level (Method Documentation)

Use Markdown H1 headers (`#`) within the doc comments for structure:

```rust
/// [STABLE] Analyzes a token and returns an AST node.
///
/// # Arguments
/// * `stream` - Token stream received from the lexer.
///
/// # Returns
/// * `Ok(Node)` - A syntax tree node.
/// * `Err(ParseError)` - Error if syntax does not comply with Kumir canons.
///
/// # Panics
/// * Panics when encountering `Token::Forbidden`, indicating a critical lexer failure.

```

---

## 🧪 4. Testing Standards

### 4.1. Unit Tests (Internal)

Located at the end of each `.rs` file. Test names should be long and descriptive.

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// [STABLE] Verification of parser behavior on an empty expression.
    fn test_parser_should_return_empty_error_on_whitespace_only() {
        // Arrange: Environment setup
        // Act: The action itself
        // Assert: Result verification
    }
}

```

### 4.2. Integration Tests (`tests/*.rs`)

External tests verify complex interactions. They are written as full scenarios. Group tests with banners if the file becomes too large.

```rust
#[test]
/// [STABLE] Full cycle: variable declaration -> assignment -> output.
fn test_integration_variable_lifecycle() {
    let mut runtime = create_house_runtime();
    let code = "алг Тест нач цел х х:=5 вывод х кон";
    
    runtime.run(code).expect("Execution failed");
    assert_eq!(runtime.last_output(), "5");
}

```

### 4.3. Kumir Language Tests (`.kum`)

Use `.kum` files to test language logic. Use the `|` symbol for comments describing the test cases.

```kumir
| CASE: Recursive call verification (Fibonacci numbers)
| EXPECTED OUTPUT: 55 (for n=10)
алг цел Фиб(цел n)
нач
  если n <= 1 то знач := n
  иначе знач := Фиб(n-1) + Фиб(n-2)
  все
кон

алг ТестФиб
нач
  вывод Фиб(10)
кон

```

---

## 🎨 5. Reference Module (The House Way Example)

```rust
// =============================================================================
//         MODULE: ALGO EXECUTOR
// =============================================================================

/// [STABLE] Primary bytecode execution engine.
/// 
/// 🐈 Context: Sleeps 90% of the time, but when it wakes up, it works at the speed 
/// of light. Requires 0.5L of white Monster Energy for proper operation.
pub struct AlgoExecutor {
    /// Call Stack
    stack: Vec<Value>,
    /// [SAFETY-CRITICAL] Pointer to the current instruction
    pc: *const u8,
}

impl AlgoExecutor {
    /// [PERF-SENSITIVE] Executes the next instruction in the stream.
    ///
    /// # Arguments
    /// * `instruction` - Opcode to execute.
    ///
    /// # Returns
    /// * `Ok(())` - Instruction executed successfully.
    /// * `Err(RuntimeError)` - Error occurred (division by zero, etc.).
    ///
    /// # Panics
    /// * Panics when attempting to execute a `HALT` instruction without a correct stack termination.
    pub fn step(&mut self, instruction: OpCode) -> Result<(), RuntimeError> {
        // FIXME-SOON: Optimize dispatching via Jump Tables
        match instruction {
            OpCode::Add => {
                // STILL-COOKING: Add overflow checks for integers
                self.perform_add()?;
            }
            _ => todo!("Wait for the magic flow"),
        }
        Ok(())
    }
}

// =============================================================================
//         UNIT TESTS
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    /// [STABLE] Verification of empty stack initialization.
    fn test_executor_init_stack_is_empty() {
        let exec = AlgoExecutor::new();
        assert!(exec.stack.is_empty());
    }
}

```

---

## 📎 6. Naming Culture and TODOs

1. **Variables:** Use `snake_case`. Use `_tmp_cat` for temporary/throwaway variables. Use descriptive names like `core_state` for important ones.
2. **TODO:** Use the format `// TODO(username): description`. This helps identify who is responsible for the "unfinished" code.
3. **Comments:** If the code performs strangely due to Rust or LLVM specifics, write `// NOTE:`.

**Kumir3 House: Write code that you'd want to hug. Or at least document it.** 🐾🦀
