# Rust Package Conventions

This guide extends the repository root `AGENTS.md`. If this file conflicts with the root guide or the normative specification, the root guide and normative specification win. `CLAUDE.md` is a relative symlink to this file.

## What This Crate Is

`openakb-validate` is the Rust validator for OpenAKB agent knowledge base descriptors. Per ADR-0001, it remains a pure library: no binary target, no CLI, and no network runtime.

The public API starts at `src/lib.rs` re-exports. Keep module internals private until an API is intentionally exposed there.

The acceptance suite is `../../conformance` plus `../../examples`, using the shared grader path when present in the repository automation.

## Toolchain

Use stable Rust, edition 2024, and MSRV 1.88. Commit `Cargo.lock`; CI runs Cargo with `--locked`.

## Required Checks

After every change, run:

```bash
cargo fmt
cargo check
cargo clippy --all-targets --all-features -- -D warnings
cargo test
cargo llvm-cov --all-features --fail-under-lines 95
```

Before a pull request, also run from the repository root:

```bash
bash scripts/ci/check-schema-sync.sh
cargo run --locked --example conformance_report > packages/rust/conformance-report.json
node scripts/ci/check-conformance-report.mjs packages/rust/conformance-report.json
```

Run the shared grader command when the repository automation provides it.

## Library Rules

Validation must be total: return diagnostics instead of panicking or aborting. The crate has no network behavior; filesystem and content I/O must sit behind an async `Resolver`.

Stable diagnostic codes `AKB001` through `AKB012` are the contract. Treat changes to their meaning, spelling, or ordering as compatibility decisions.

No panics are allowed in `src`. Cargo lints deny `unwrap`, `expect`, `panic`, `todo`, and `unimplemented`.

Embedded schemas in `schemas/` must be byte-identical copies of `../../schema/v1/`. Runtime dependencies are design decisions; keep them purposeful and review their feature sets.

Do not put magic values directly in validation logic. Add named entries to the catalog and use them from implementation and tests.

## Style

Use `thiserror` for error variants. Design public APIs first, then implement private helpers behind them.

Group imports as standard library, external crates, then crate-local imports. Prefer small functions with shallow nesting and narrow parameter lists; split logic when a function becomes hard to scan.

Every lint suppression needs a local justification. For future builder APIs, prefer `TypedBuilder` only when the type-state benefit is clearer than a plain constructor.

## Testing

Tests live out of source under `tests/unit` and `tests/integration`. Do not add `cfg(test)` modules in `src`.

Leaf test files use the `_tests.rs` suffix. Test names start with `test_` and use at most five words after the prefix.

Expected values come from the spec, schema, or fixtures rather than restating implementation internals. Keep line coverage at or above 95%.

## Publishing

A `rust-v<version>` tag triggers the publish workflow later with `CARGO_REGISTRY_TOKEN`. The tag version must equal `rust-v` plus the `Cargo.toml` package version.
