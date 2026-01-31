# CONTRIBUTING (English)

Thanks for your interest in Kumir 3! We welcome contributions — here are the guidelines to help keep things friendly and efficient.

---

## Key principles

- Work in branches; do not push directly to `dev`/`master`.
- Open PRs from your branch or a fork.
- Create an Issue for major changes and discuss the design first.
- Add tests for new features and bug fixes.
- Document code using BDoc (see `arch/en/FAQ.md`).

---

## Commit messages — Conventional Commits 1.0.0

We use Conventional Commits for a clean history and automated changelogs.

Format:

```
<type>[optional scope]: <description>

[optional body]

[optional footer(s)]
```

Types (not exhaustive): `feat`, `fix`, `docs`, `chore`, `test`, `refactor`.

Examples:
- `feat(parser): add ability to parse arrays`
- `fix: prevent racing of requests`
- `docs: update README`

BREAKING CHANGES must include `!` in the header or `BREAKING CHANGE:` in the footer.

---

## Branches & PRs

- Use clear branch names: `feature/cool-thing`, `fix/bug-123`.
- Keep PRs small and focused. For larger work, open an issue first.
- Include a summary, test cases, and how to reproduce in the PR description.

---

## Review & Merge

- Final merges go through the project lead (see CHANGES). Preliminary approvals are fine but do not guarantee merge.
- Community-driven features may be merged without final maintainer approval when requested by the community.

---

## Code style & docs

- Follow BDoc: section banners, detailed docs for structs and functions.
- Exceptions: `LEGACY FUNCTION`, `STILL-COOKING`, `NOT FOR DIRECT USAGE`, `EXPERIMENTAL`.

---

## Tests & CI

- Run tests locally before opening a PR.
- Add integration/regression tests when fixing bugs or adding features.

---

## Memes & Etiquette

- Memes are welcome but keep PRs professional.
- Light Astolfo references are tolerated but be respectful.

---

## Contact

Project lead for PRs and decisions: Vadim Khristenko — https://vadim-khristenko.github.io

---

Thanks for contributing — let’s make Kumir 3 great!
