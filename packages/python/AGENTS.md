# packages/python — contributor conventions

Conventions for the `openakb-validate` Python reference validator. They extend the
repo-root `AGENTS.md`; on conflict, the root file and the normative spec win.
(`CLAUDE.md` is a symlink to this file.)

`README.md` here is the user-facing usage guide; this file is the contributor
rulebook. Link between the two - never duplicate rules into the README.

## What this package is

The reference validator for OpenAKB spec major v1: a **pure library**
([ADR-0001](../../decisions/0001-repo-topology.md) - no console script, no CLI
module, no network I/O). The public API is exactly what
`src/openakb_validate/__init__.py` re-exports; the fixtures under
[`conformance/`](../../conformance/) and the worked examples under
[`examples/`](../../examples/) are its acceptance suite.

## Environment

- Python 3.12 floor (`requires-python`), managed with `uv`; `.venv/` is gitignored.
- Add dependencies with `uv add <package>`, never `pip install`. The runtime
  dependency set is deliberate and minimal - adding one is a design decision, not a
  convenience.
- Run everything through `uv run <command>`, not bare `python`/`pytest`.

## Checks

After changing code, run from `packages/python/` and fix findings before considering
work done:

```bash
uv run ruff format --check . && uv run ruff check .   # format + lint
uv run mypy                                           # strict typecheck
uv run pytest                                         # tests + conformance + coverage gate (>=95%)
bash ../../scripts/ci/check-schema-sync.sh            # vendored schemas match schema/v1/
uv run python tests/conformance_report.py > conformance-report.json
node ../../scripts/ci/check-conformance-report.mjs conformance-report.json
```

The last two commands emit this implementation's conformance report and grade it
with the shared checker - the single home of the match semantics and of
cross-validator agreement for every language.

Suppressions (`# noqa`, `# type: ignore[...]`, pyproject-level ignores) are a last
resort: each needs an inline comment explaining why the checker is wrong here.
When the checker flags a real smell, restructure instead of suppressing.

## Module layout

- Order definitions public-first, private-last: public classes, functions, and
  constants, then underscore-prefixed internals. Module-level constants may sit
  right after the imports even when private - configuration knobs read best near
  the top.
- Every module defines `__all__`; file layout reinforces it.
- Underscore-named modules (`_shape.py`) are internal and are never imported from
  outside the package.

## Library rules

- **Descriptors are untrusted input.** Every layer is defensive against arbitrary
  JSON shapes: unexpected types are skipped (the schema layer reports them), never
  raised on. `validate()` must not throw on any JSON value.
- **No network, ever.** Resolvers are injected; `LocalFileResolver` confines reads
  to its base directory. Anything unfetchable is `unverifiable`, never an error.
- **Stable error codes are the contract.** Only `AKB001`-`AKB012` exist. Never
  invent a code; never change a code-to-rule mapping outside a spec change.
- **No magic values.** Tunables, caps, and shared literals get named constants
  (`catalog.py` holds the spec-derived ones). One-off message strings and inherent
  literals (`0`, `1`) stay inline.
- Function bodies target <=50-60 lines and one responsibility; <=3 nesting levels;
  <=5 parameters. Comments explain why, not what, and never reference the task or
  PR that produced them.

## Testing

- **Unit tests only**, under `tests/`: hermetic, offline, seconds-fast. There is no
  integration or e2e layer - nothing here may touch the network, and the only
  filesystem access is `tmp_path` plus the repo's own `conformance/` and
  `examples/` trees (tests skip when those trees are absent, e.g. in an sdist).
- **Expected values come from the spec, the schemas, and the conformance
  fixtures - never from the implementation's own output.** Fixtures are the
  published contract: if the validator and a fixture disagree, stop and surface
  it; never edit a fixture to make a test pass.
- Test the public contract, not internals; be adversarial (error paths,
  empty/one/many, malformed shapes). A test no code change can fail is theater.
- No test hacks - blockers even when green: assertion weakening, stubs returning
  what the implementation produced, blessing changed output without proving it
  correct, `@pytest.mark.skip` without a linked issue, `time.sleep` instead of a
  real signal.
- A bug that escapes to CI or review is a missing test: add one that fails before
  the fix, at the lowest layer that would have caught it.
- Names: `test_` plus at most five `_`-separated words; detail goes in the
  docstring, not the name. Test files mirror source modules
  (`citations.py` -> `tests/test_citations.py`).
- Coverage: >=95% on the testable surface (`fail_under` gates it). Exclude a line
  only with `# not coverable in CI: <reason>`.

## Hygiene

- No debug artifacts in committed code: `print(...)`, `breakpoint()`, and
  `pdb.set_trace()` never land.
- Idiom floor: comprehensions over `.append` loops, `enumerate` over manual
  counters, `pathlib` over `os.path`, f-strings over `%`/`.format`, no bare
  `except:` or broad `except Exception` that swallows errors, no mutable default
  arguments.
- Deferred work is `TODO(#123): ...` with a real issue; bare `TODO`s don't land.
  No commented-out code - delete it or ship it.
- `README.md` here is public and vendor-neutral (root `AGENTS.md` neutrality
  rule); update it in the same PR when the public API or supported versions
  change.
