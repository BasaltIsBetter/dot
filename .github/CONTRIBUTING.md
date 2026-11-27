# Contributing to Dot

**English** · [Français](../docs/fr/CONTRIBUTING.md) · [Español](../docs/es/CONTRIBUTING.md) · [Português](../docs/pt/CONTRIBUTING.md)

Thank you for your interest. A bug or an idea: open an issue with its form. A security
vulnerability: never in public, see [`SECURITY.md`](SECURITY.md).

## Before writing code

For anything bigger than a fix, open an issue first to talk it over. A small pull request focused on
one thing is reviewed and merged much faster.

## Build and check

Requirements and building are in the [`README`](../README.md). Before opening a pull request:

```bash
cargo fmt --all --check
cargo clippy --locked --workspace --all-targets --all-features -- -D warnings
cargo test --locked --workspace
RUSTDOCFLAGS="-D warnings" cargo doc --locked --workspace --no-deps
cargo deny check
```

Continuous integration runs the same checks on macOS and Linux.

## What a change needs

- **Tests** for every piece of logic, in `tests/<crate>/`, at the same path as the module tested,
  proving both ways: valid input passes, invalid input is refused.
- **No action of Claude without permission**, no path outside the project's folder, no secret in a
  log or an error message.
- **Measured, not assumed**: a change on a hot path (the model's stream, drawing the interface,
  startup) gives its numbers before and after (`cargo bench`).
- **No displayed text written in the code**: every text goes through `dot-i18n`, translated in each
  language (`crates/dot-i18n/locales/`: English, French, Spanish, Brazilian Portuguese). The tests
  fail if a text is missing in one of them.
- **[Conventional Commits](https://www.conventionalcommits.org/)**: `feat(desktop): …`,
  `fix(model): …`, a short description in the imperative.

By proposing a contribution, you agree that it is published under the project's license,
[Apache 2.0](../LICENSE).
