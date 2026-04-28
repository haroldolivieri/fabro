# Fixtures Build Agent

You are building the `taps-keys-fixtures` Python package. This package contains golden fixture data (generated from a Java reference implementation) and a contract test runner that validates any Python implementation of the TAPS keys encoding library against those fixtures.

## Working Directory

All work happens inside `taps-keys-fixtures/`. Do not modify files outside this directory.

## FIRST: Create .gitignore

Before creating any other file, create `taps-keys-fixtures/.gitignore`:
```
dist/
build/
*.egg-info/
__pycache__/
*.pyc
.pytest_cache/
```

## IMPORTANT: No duplicate files at root

Golden fixture JSON files belong ONLY inside `taps_keys_fixtures/fixtures/`. Do NOT copy them to the repo root.

## Package Structure

```
taps-keys-fixtures/
  taps_keys_fixtures/
    __init__.py
    fixtures/
      golden_encodings.json
      golden_signatures.json
    runner/
      __init__.py
      test_runner.py
      error_formatter.py
  tests/
    test_runner_selftest.py
  pyproject.toml
  Makefile
  README.md
  CLAUDE.md
```

## Step 1: Copy Golden Fixture Files

Copy the golden fixture JSON files from `/tmp/taps-keys-fixtures/` (the raw files placed there by a prior workflow stage) into the package at `taps_keys_fixtures/fixtures/`.

Create the directory structure:

```
taps_keys_fixtures/
  __init__.py          (empty)
  fixtures/
    golden_encodings.json
    golden_signatures.json
```

The fixture files are immutable artifacts generated from the Java reference implementation. Never modify their contents.

## Step 2: Fixture JSON Schema Reference

### golden_encodings.json

An array of objects. Each entry has:

| Field | Type | Description |
|---|---|---|
| `schema` | string | Schema name (e.g. `"OneWay"`, `"Return"`) |
| `prefix` | string | Schema prefix byte(s) |
| `input_set` | string | One of `"A"`, `"B"`, `"C"`, `"D"`, `"E"` |
| `origin` | string | Origin airport code |
| `destination` | string | Destination airport code |
| `carrier` | string | Marketing carrier code |
| `outbound_date` | string | Outbound date (ISO format or components) |
| `inbound_date` | string or null | Inbound date (null for OneWay) |
| `is_direct` | boolean or string | `true`, `false`, or `"*"` for wildcard (Set E) |
| `encoded_key` | string | The golden encoded key (hex or base64) |
| `to_string` | string | Expected output of `key.to_string()` |
| `to_string_pipe` | string | Expected output of pipe-delimited toString variant |
| `schema_to_string` | string | Expected output of `schema.to_string()` |
| `encoded_length` | integer | Expected byte length of the encoded key |
| `open_jaw_filter` | boolean | Expected result of open-jaw filter check |

### golden_signatures.json

An array of objects. Each entry has:

| Field | Type | Description |
|---|---|---|
| `schema` | string | Schema name |
| `schema_to_string` | string | Expected schema toString |
| `origin_airport_disjoint` | boolean | Whether origin airport field is disjoint |
| `destination_airport_disjoint` | boolean | Whether destination airport field is disjoint |
| `outbound_year_month_disjoint` | boolean | Whether outbound year-month field is disjoint |
| `outbound_day_disjoint` | boolean | Whether outbound day field is disjoint |
| `inbound_year_month_disjoint` | boolean | Whether inbound year-month field is disjoint |
| `inbound_day_disjoint` | boolean | Whether inbound day field is disjoint |

## Step 3: Build the Contract Test Runner

### taps_keys_fixtures/runner/__init__.py

Empty file.

### taps_keys_fixtures/runner/test_runner.py

This is the main entry point. It must:

#### CLI Interface

```
python -m taps_keys_fixtures.runner.test_runner --module <name> [--layers <comma-separated>]
```

- `--module` (required): Python module name to validate (e.g. `taps_keys`). The module is imported dynamically via `importlib.import_module()`.
- `--layers` (optional, default `L1,L2,L3`): Comma-separated list of layers to run. Valid values: `L1`, `L2`, `L3`.

#### Module Access

After importing the module, the runner accesses:
- `module.OneWay` -- the OneWay key builder class
- `module.Return` -- the Return key builder class

These are accessed as attributes on the imported module object.

#### Fixture Loading

Load fixture JSON from the package's own `fixtures/` directory using `importlib.resources`:

```python
import importlib.resources
import json

fixtures_pkg = importlib.resources.files("taps_keys_fixtures.fixtures")
encodings = json.loads((fixtures_pkg / "golden_encodings.json").read_text())
signatures = json.loads((fixtures_pkg / "golden_signatures.json").read_text())
```

#### Builder Method Calls

The runner constructs key builder instances by calling methods in **snake_case**. The builder pattern is fluent (each method returns `self`).

For OneWay schema, the builder chain is:

```python
builder = module.OneWay()
builder = (builder
    .origin_airport(entry["origin"])
    .destination_airport(entry["destination"])
    .outbound_departure_year_month(outbound_ym)
    .outbound_departure_day(outbound_day)
    .is_direct(direct_value)
    .marketing_carrier(entry["carrier"])
    .origin_city(...)      # if present in fixture
    .origin_country(...)   # if present in fixture
    .origin(...)           # if present in fixture
    .destination_city(...) # if present in fixture
    .destination_country(...)  # if present in fixture
    .destination(...)      # if present in fixture
)
```

For Return schema, the chain additionally includes:

```python
    .inbound_departure_year_month(inbound_ym)
    .inbound_departure_day(inbound_day)
```

Parse dates from the fixture entry to extract year-month and day components as needed by the builder methods.

#### Wildcard Handling (Set E)

For input set `"E"`, the `is_direct` field is `"*"` (wildcard). The runner must call:

```python
from <module> import KeyBuilder  # or access module.KeyBuilder
builder.is_direct(KeyBuilder.any_direct())
```

Or equivalently, however the module exposes the wildcard sentinel for the `is_direct` field. The runner should try `module.KeyBuilder.any_direct()` as the primary approach. If `KeyBuilder` is not found on the module, fall back to passing the string `"*"` directly.

#### Layer Definitions

**L1 -- Encoding (byte-for-byte)**

For each entry in `golden_encodings.json`:
1. Build the key using the builder chain above
2. Call `builder.encode()` (or equivalent) to get the encoded bytes
3. Compare encoded output byte-for-byte against `entry["encoded_key"]`
4. Compare `str(key)` or `key.to_string()` against `entry["to_string"]`
5. Compare the encoded length against `entry["encoded_length"]`

**L2 -- Signatures and Disjoint Properties**

For each entry in `golden_signatures.json`:
1. Access the schema class (`module.OneWay` or `module.Return` based on `entry["schema"]`)
2. Validate `schema.to_string()` or `str(schema)` matches `entry["schema_to_string"]`
3. Validate each disjoint property:
   - `origin_airport_disjoint`
   - `destination_airport_disjoint`
   - `outbound_year_month_disjoint`
   - `outbound_day_disjoint`
   - `inbound_year_month_disjoint`
   - `inbound_day_disjoint`

Access these as attributes or methods on the schema class (e.g. `schema.origin_airport_disjoint` or `schema.origin_airport_disjoint()`).

**L3 -- Structural: toString, encodedLength, OpenJawFilter**

For each entry in `golden_encodings.json`:
1. Build the key
2. Compare `key.to_string()` against `entry["to_string"]`
3. Compare pipe-delimited toString if available: `key.to_string_pipe()` against `entry["to_string_pipe"]`
4. Compare `key.encoded_length()` against `entry["encoded_length"]`
5. Compare `key.open_jaw_filter()` against `entry["open_jaw_filter"]`

#### Error Handling

Wrap every individual test case in `try/except Exception`. If the implementation under test raises an exception, report it as a failure but **never let the exception dump fixture data to output**. Catch it, record the failure with the error type and message only, and continue to the next test case.

#### Exit Code

- Exit `0` if all requested layers pass with zero failures.
- Exit `1` if any test case fails in any layer.

#### Output Format

Print a summary per layer:

```
[L1] Encoding: 42/42 passed
[L2] Signatures: 6/6 passed
[L3] Structural: 42/42 passed

All layers passed.
```

Or on failure:

```
[L1] Encoding: 40/42 passed, 2 failed
  FAIL: L1 | OneWay | Set A | encoding length mismatch: got 12, expected 14
  FAIL: L1 | Return | Set B | encoding byte mismatch at position 3: got 0x1A, expected 0x1B...

[L1] FAILED
```

Delegate all failure message formatting to `error_formatter.py`.

### taps_keys_fixtures/runner/error_formatter.py

This module formats error messages for test failures. The format is intentionally restrictive to prevent leaking full golden values.

#### Rules

1. **Never reveal the full expected value.** This is the cardinal rule.
2. For byte/string comparison failures, reveal:
   - The layer (`L1`, `L2`, `L3`)
   - The schema name (`OneWay`, `Return`)
   - The input set (`A`, `B`, `C`, `D`, `E`)
   - The nature of the mismatch (length difference, first differing position, boolean flip)
3. For length mismatches: show `got X, expected Y`.
4. For byte mismatches: show the first differing position and the byte values at that position. Show at most the **first 4 characters** of the expected value for prefix comparison. Example: `expected starts with "0x1B..." but got "0x1A..."`.
5. For boolean mismatches: show `got True, expected False` or vice versa.
6. For string mismatches: show `got length X, expected length Y` if lengths differ. If lengths match, show the first differing character position and at most the first 4 characters of the expected string for prefix context.
7. For disjoint property mismatches: show the property name and the boolean flip.
8. For exception failures: show `EXCEPTION: <ErrorType>: <message>` -- never include traceback or fixture data in the message.

#### Public API

```python
def format_encoding_error(layer: str, schema: str, input_set: str,
                          field: str, got: Any, expected: Any) -> str:
    """Format an encoding/value comparison failure."""

def format_disjoint_error(layer: str, schema: str,
                          property_name: str, got: bool, expected: bool) -> str:
    """Format a disjoint property mismatch."""

def format_exception_error(layer: str, schema: str, input_set: str,
                           error: Exception) -> str:
    """Format an implementation exception as a safe failure message."""
```

## Step 4: Build Repository Harness

### pyproject.toml

```toml
[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.build_meta"

[project]
name = "taps-keys-fixtures"
version = "1.0.0"
description = "Golden fixtures and contract test runner for TAPS keys encoding"
requires-python = ">=3.10"
dependencies = []

[tool.setuptools.packages.find]
include = ["taps_keys_fixtures*"]

[tool.setuptools.package-data]
taps_keys_fixtures = ["fixtures/*.json"]
```

No runtime dependencies. The runner uses only the Python standard library.

### Makefile

```makefile
.PHONY: test lint check install

install:
	pip install -e .

test:
	python -m pytest tests/ -v

lint:
	python -m ruff check taps_keys_fixtures/ tests/
	python -m mypy taps_keys_fixtures/

check: lint test
```

### tests/test_runner_selftest.py

A self-test that verifies the runner infrastructure works without requiring an actual implementation module. It should:

1. Test that fixture JSON files load correctly via `importlib.resources`.
2. Test that `golden_encodings.json` is a non-empty list of dicts with the expected keys.
3. Test that `golden_signatures.json` is a non-empty list of dicts with the expected keys.
4. Test that `error_formatter` functions produce strings and never include raw expected values longer than 4 characters.
5. Test that the runner CLI exits with an error when `--module` references a nonexistent module.

Use `pytest` for all tests. No external dependencies beyond pytest.

### README.md

Write a brief README covering:
- What this package is (golden fixtures + contract test runner for TAPS keys)
- How to install: `pip install -e .`
- How to run: `python -m taps_keys_fixtures.runner.test_runner --module <your_module>`
- Layer descriptions (one sentence each for L1, L2, L3)
- Note that fixture files are generated from Java and must not be modified

### CLAUDE.md

```markdown
# CLAUDE.md

## Critical rules

- **Fixture JSON files are immutable.** The files in `taps_keys_fixtures/fixtures/` are generated from the Java reference implementation. Never modify them.
- **Runner error format is intentional.** The error formatter deliberately never reveals full expected values. At most the first 4 characters of an expected value may appear. Do not "improve" error messages by showing full expected output.

## Commands

- `pip install -e .` -- install in editable mode
- `python -m pytest tests/ -v` -- run self-tests
- `python -m taps_keys_fixtures.runner.test_runner --module <name>` -- run contract tests against a module
- `python -m taps_keys_fixtures.runner.test_runner --module <name> --layers L1` -- run only L1
```

## Completion Criteria

When you are done, the following must hold:

1. All files listed in the package structure above exist.
2. `golden_encodings.json` and `golden_signatures.json` are present in `taps_keys_fixtures/fixtures/` (copied from the raw location).
3. `pip install -e .` succeeds with no errors.
4. `python -m pytest tests/ -v` passes (the self-tests).
5. `python -m taps_keys_fixtures.runner.test_runner --module nonexistent_module` exits with a clear error (not a traceback).
6. The runner code handles all input sets A through E, including wildcard handling for Set E.
7. No fixture data (encoded keys, expected strings, golden bytes) appears in error output beyond the 4-character prefix limit.

Run all verification steps before declaring the task complete.
