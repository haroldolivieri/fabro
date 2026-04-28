# CLAUDE.md — Constitution for AI agents in taps-keys-python

This file governs how AI agents must behave when working in this repository.

## Non-negotiable constraints

### 1. Encoding rules are immutable

The base-32 alphabet, component bit widths, and encoding formulas are derived from the Java source of truth. **Do not change them** without:
- Verifying the change in the Java library first
- Regenerating fixtures in `taps-keys-fixtures`
- Updating `taps-keys-schemas` if the schema structure changes

A single wrong character in a key is a cache miss in production.

### 2. Never bundle schemas.json

Always load schemas at runtime from the `taps_keys_schemas` package using `importlib.resources`:

```python
from importlib.resources import files
import json

data = json.loads(files("taps_keys_schemas").joinpath("schemas.json").read_text())
```

Do not copy, embed, or hardcode any schema data. The `taps-keys-schemas` package is the single source of truth.

### 3. Never write fixture-loading code

The contract runner in `taps-keys-fixtures` handles all fixture validation. Do not write code that reads `golden_encodings.json`, `golden_signatures.json`, or any other fixture file.

### 4. validate.py calls the contract runner, not its own fixture tests

`scripts/validate.py` invokes `taps_keys_fixtures.runner.test_runner`. It does not implement its own fixture-reading logic.

### 5. KeySchema must remain picklable

PySpark ships UDF closures via pickle. `KeySchema` and all objects it contains must be picklable. Test this explicitly. Do not add lambdas, open file handles, or thread locks to `KeySchema`.

### 6. Always run `make validate` before committing

```bash
make validate
```

This runs the contract runner (L1–L3) and unit tests. Both must pass.

## Validation layers

The encoding is verified across six independent layers. Understanding what each catches helps you fix failures correctly:

| Layer | What it checks | Where it runs |
|---|---|---|
| **L1** | `encode()` output byte-for-byte vs 2130 golden cases | contract runner |
| **L2** | `schema.signature()` and 6 disjoint booleans per schema | contract runner |
| **L3** | `to_string()`, `to_string('|')`, `encoded_length`, `OpenJawFilter` | contract runner |
| **Unit tests** | Base-32 boundaries, YEARMONTH overflow, builder edge cases, picklability | pytest |
| **L5** | Python vs Java on same 2130 inputs **live** — no fixture files in chain | Fabro workflow + CI |
| **L6** | 14,200 random fuzz inputs (Java generates, Python encodes) | Fabro workflow + CI |

**L5 and L6 are the strongest guarantees.** They bypass the fixture files entirely and run both the JVM and the Python library against the same inputs in the same process. A port that passes L1–L3 via a lookup table will fail L5 on the first out-of-fixture input.

**If L1–L3 pass but L5 fails**, the Python implementation is algorithmically wrong for inputs outside the 15 hardcoded input sets per schema. Fix the encoding formula, not the tests.

**If L5 passes but L6 fails**, there is an edge case in random input combinations that the 15 fixed sets don't cover (e.g., unusual date/carrier/route combinations). The fuzz output identifies the exact failing input.

## Scope

Only modify files inside `taps-keys-python/`. Do not read, write, or interact with `taps-keys-fixtures/`, `taps-keys-schemas/`, or any other repo.

## Workflow

When making encoding changes:
1. Read `src/taps_keys/encoding.py` before modifying.
2. Verify against the worked examples in `.fabro/workflows/taps-keys-python/prompts/python-encoding-seed.md`.
3. Run `make validate`.

When schemas change:
1. `pip install --upgrade taps-keys-schemas`
2. `make validate`
3. Report what changed and whether all tests passed.
