# Repository Guidelines

## Project Structure & Module Organization
- Workspace crates live in `allure` (public facade), `allure-core` (model/runtime, attachments, results writer), and `allure-macros` (proc macros).
- Integration tests sit under `allure/tests`; examples under `allure/examples`; generated outputs during local runs land in `allure-results/` (JSON) and `allure-report/` (HTML).
- Shared metadata lives in the workspace `Cargo.toml`; keep crate-specific settings inside each crate’s `Cargo.toml`.

## Build, Test, and Development Commands
- `cargo fmt --all` — format; CI runs `--check` so keep it clean.
- `cargo clippy --all-features -- -D warnings` — lint with warnings-as-errors (matches CI’s `RUSTFLAGS=-Dwarnings`).
- `cargo test --all-features` — full test suite (includes async/tokio paths); use `-- --nocapture` when debugging.
- `cargo build --all-features` — sanity compile before PRs; `cargo doc --no-deps --all-features` to ensure docs warn-free.
- To view reports locally after tests: `allure generate allure-results -o allure-report && allure open allure-report`.

## Coding Style & Naming Conventions
- Rust 2021, MSRV 1.75; default rustfmt (4-space indent, trailing commas). Prefer explicit module paths over glob imports.
- Snake_case for files/modules, UpperCamelCase for types, SCREAMING_SNAKE for consts. Keep macro metadata attributes above `#[allure_test]` as shown in `README.md`.
- Use `?` for error propagation and lean on `Result` returns in public APIs; keep panic!s to test-only code.
- Docs should include minimal runnable snippets; add feature flags in examples when needed.

## Testing Guidelines
- Favor `rstest` for parameterization and `tokio::test` for async cases; mirror behaviors with BDD helpers in `allure::bdd` when relevant.
- Place integration tests in `allure/tests` and unit tests next to source files in `src` using `mod tests`.
- Name tests descriptively (`test_generates_uuid`, `bdd_creates_step_hierarchy`); assert on Allure JSON where possible rather than string-matching logs.
- When adding new features, include a sample attachment/result in `allure-results` only in temporary runs—don’t commit generated artifacts.

## Commit & Pull Request Guidelines
- Prefer concise Conventional Commit prefixes (`feat:`, `fix:`, `docs:`, `chore:`); keep subjects under ~70 chars and describe the user-facing effect.
- PRs should list scope, breaking changes (if any), feature flags touched, and test evidence (commands run).
- Link issues when available; add short rationale for API changes and for new macros include a usage example.
