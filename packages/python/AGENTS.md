# Python package contributor guide

This guide applies to every file under `packages/python/`. The repository-level `AGENTS.md`
also applies.

## Scope

`openakb-validate` is a pure Python library for validating OpenAKB descriptor documents for
spec major v1. It must not provide a command-line interface, start services, perform network
access, or make storage, serving, search, freshness, authentication, or transport decisions.

## Tooling

Use `uv` for package development:

```bash
uv sync
uv run pytest
uv run ruff format .
uv run ruff check .
uv run mypy
```

When vendored schemas change, also run from the repository root:

```bash
bash scripts/ci/check-schema-sync.sh
```

## Module layout

Keep public imports shallow and intentional. Each module that exposes public names must define
`__all__`, and `src/openakb_validate/__init__.py` should re-export only the stable public API.

Place importable package code under `src/openakb_validate/`, tests under `tests/`, and package
resources under package subdirectories so they are readable with `importlib.resources`.

## Library rules

Prefer explicit typed data structures and small functions with deterministic behavior. Library
code should return diagnostics instead of printing, exiting, or mutating caller-owned state.
Do not read from or write to arbitrary paths unless the caller supplied that path or resolver.
Do not add global caches that can make validation order-dependent.

## Testing

Use test-driven development for behavior changes: write the failing test first, watch it fail,
then implement the smallest change that makes it pass. Keep tests focused on public behavior
and shared conformance fixtures where possible.

Test function names must start with `test_` and use at most five underscore-separated words
after that prefix.

## Hygiene

Keep public artifacts vendor-neutral. Do not add real product or company names except where
the repository-level rules explicitly allow tooling or technical references. Keep generated
and local environment files out of commits unless the task explicitly asks for them.
