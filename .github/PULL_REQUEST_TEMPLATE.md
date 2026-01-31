---
name: "🚀 Pull Request"
about: Submit a change to kumir3-core
---

## 📝 Summary

## 🛠 Type of Change

- [ ] **feat**: A new feature for the core engine.
- [ ] **fix**: A bug fix (compiler, runtime, or executor).
- [ ] **perf**: A code change that improves performance (please include benchmarks!).
- [ ] **refactor**: Code changes that neither fix bugs nor add features.
- [ ] **docs**: Documentation only changes.
- [ ] **test**: Adding missing tests or correcting existing ones.
- [ ] **chore**: Changes to the build process, CI/CD, or auxiliary tools.

## 🔗 Related Issues

- Fixes #- Related to #

## 🧪 Technical Depth & Testing

### Validation Plan

1. **Unit Tests:** `cargo test -p kumir3-core`
2. **Integration Tests:** (List specific `.kum` scripts used for E2E testing)
3. **Manual Verification:** (Steps taken in the debugger or CLI)

### Performance Impact

- [ ] Benchmarked using `criterion` (if applicable).
- [ ] No significant regression in compilation/execution speed.

## 📐 Implementation Details

<details>
<summary>View Technical Notes</summary>

- **Breaking Changes:** Does this change the public API or Kumir language specification? (Yes/No - if yes, explain).
- **Dependencies:** Did you add any new crates to `Cargo.toml`?
- **Safety:** Did you use any `unsafe` blocks? If so, justify them.

</details>

## ✅ Checklist

- [ ] My code follows the **Rust API Guidelines** and project-specific **BDoc** standards.
- [ ] I have performed a self-review of my own code.
- [ ] I have commented my code, particularly in hard-to-understand areas.
- [ ] I have added tests that prove my fix is effective or that my feature works.
- [ ] New and existing unit tests pass locally with my changes.
- [ ] I have updated the `CHANGELOG/<pr_id>.md` (if applicable).
- [ ] My commits follow the **Conventional Commits** specification.

## 👥 Reviewer Suggestions

---
**Thank you for contributing to Kumir3!** 🦀
