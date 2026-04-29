# taps-keys Python — Full Build Prompt

**SCOPE: Only modify files inside `taps-keys-python/`. Do not read, write, or interact with `taps-keys-fixtures/`, `taps-keys-schemas/`, or any other repo.**

## FIRST: Create .gitignore

Before creating any other file, create `taps-keys-python/.gitignore`:
```
dist/
build/
*.egg-info/
__pycache__/
*.pyc
.pytest_cache/
```

You are building the complete `taps-keys-python` repo in one pass: the core encoding library, repo harness, unit tests, and validation scripts. The goal is a Python library that produces encoding results **byte-for-byte identical** to the Java taps-keys library. FCP consumers depend on exact key matching — a single wrong character means a cache miss in production.

---

## Section 1 — Algorithm Reference

Read the encoding seed document at `.fabro/workflows/taps-keys-python/prompts/python-encoding-seed.md` (in the fabro repo root) in full before writing any code. It contains the base-32 alphabet, all 8 component types, 10 worked examples with step-by-step arithmetic, KeyBuilder behaviour, KeySchema.toString() format, signature/disjoint checks, OpenJawFilter derivation, Spark serialisation requirements, anti-patterns, and self-check criteria.

Do not proceed without reading that document. The examples are your test cases.

---

## Section 2 — Schema Data Source

Install the `taps-keys-schemas` package (NOT taps-keys-fixtures). Load schemas.json at runtime using:

```python
from importlib.resources import files
import json

_data = json.loads(files("taps_keys_schemas").joinpath("schemas.json").read_text())
```

**Never bundle a copy of schemas.json** — always import from the `taps_keys_schemas` package.

The Java source contains **71 OneWay + 71 Return = 142 total KeySchema constants**. All must be loadable from `schemas.json`.

---

## Section 3 — Java Source Reference

The encoding algorithm is fully specified in the seed document and the worked examples below. You do NOT need to read any Java source files. All necessary information is self-contained in this prompt and the encoding seed.

---

## Section 4 — Core Library (`src/taps_keys/`)

### 4.1 — encoding.py

**Base-32 alphabet**: `"0123456789abcdefghijklmnopqrstuv"` (32 characters: 10 digits + first 22 lowercase letters). Always lowercase. Never uppercase.

```python
_ALPHABET = "0123456789abcdefghijklmnopqrstuv"

def to_base32(n: int, width: int) -> str:
    if n == 0:
        return "0" * width
    chars = []
    while n > 0:
        chars.append(_ALPHABET[n % 32])
        n //= 32
    return "".join(reversed(chars)).rjust(width, "0")
```

**Component types and encoders:**

| Type | bits | encoded_length | Value range | Encoding |
|---|---|---|---|---|
| AIRPORT | 16 | 4 | [0, 65536] | `to_base32(value, 4)` |
| CITY | 16 | 4 | [0, 65536] | `to_base32(value, 4)` |
| COUNTRY | 16 | 4 | [0, 65536] | `to_base32(value, 4)` |
| LOCATION | 16 | 4 | [0, 65536] | `to_base32(value, 4)` |
| YEARMONTH | 10 | 2 | [0, 1024] | `to_base32((year - 1970) * 12 + (month - 1), 2)` |
| DAY | 5 | 1 | [0, 32] | `to_base32(day_of_month, 1)` |
| DIRECT | 1 | 1 | {0, 1} | `to_base32(1 if direct else 0, 1)` |
| MARKETING_CARRIER | 16 | 4 | [-32768, 32767] | `to_base32(carrier_id + 32768, 4)` — offset by +32768 |

Create one encoder class per component type, each with `encode(value) -> str` and `validate(value) -> None`.

**Range validation**: Raise `ValueError` on out-of-range input for all types. Silent overflow is the worst failure mode.

**YEARMONTH special case**: maxValue is 1024 (2^10). The range check is `0 <= value <= 1024`. Value 1024 IS valid (does not throw) despite producing 3 chars (`"100"`) for a 2-char field. This is a known Java edge case — replicate the exact boundary (`<= 2^numBits`) and the same silent overflow behaviour. Concretely: `to_base32(1024, 2)` produces `"100"` (3 chars, not 2 — Java's `padStart` pads but never truncates).

### 4.2 — schema.py

- `ComponentType` enum (8 members: AIRPORT, CITY, COUNTRY, LOCATION, YEARMONTH, DAY, DIRECT, MARKETING_CARRIER)
- `Component` dataclass: `(type: ComponentType, name: str)`
- `OpenJawFilter` enum: NONE, ORIGIN, DESTINATION, BOTH

**Key class:**
- `.encode() -> str` — concatenate each component's base-32 encoding in schema order; stop at first wildcard
- `.to_string(joiner: str = "-") -> str` — join component **raw integer values** with the joiner; wildcards shown as `"*"`

**Key.to_string() format (CRITICAL):**

The values are integers, NOT formatted dates or booleans:
- AIRPORT/CITY/COUNTRY/LOCATION: the route node ID integer (e.g. `13554`)
- YEARMONTH: the computed months-since-Jan-1970 integer (e.g. `582`), NOT the date string
- DAY: the day-of-month integer (e.g. `8`)
- DIRECT: `1` for true, `0` for false (not the words "true"/"false")
- MARKETING_CARRIER: the raw carrier ID integer (e.g. `-32480`), NOT the offset value

Examples:
```
AIRPORT_AIRPORT_DAY_OW, inputs (13554, 13555, 2018-07-08, true):
  key.to_string()    → "13554-13555-582-8-1"
  key.to_string("|") → "13554|13555|582|8|1"

With wildcard isDirect:
  key.to_string()    → "13554-13555-582-8-*"
```

**KeyBuilder:**
- Stores ALL setter values in a dict keyed by `(ComponentType, name)` tuples
- At `build()` time, ONLY the schema's components are looked up — extra values are silently ignored
- Missing required value raises error
- Wildcard flag: once set, all subsequent components must also be wildcard

**KeyBuilder setter names** (must match contract runner exactly):

| Python method | Component |
|---|---|
| `origin_airport(int)` | (AIRPORT, "origin") |
| `origin_city(int)` | (CITY, "origin") |
| `origin_country(int)` | (COUNTRY, "origin") |
| `origin(int)` | (LOCATION, "origin") |
| `destination_airport(int)` | (AIRPORT, "destination") |
| `destination_city(int)` | (CITY, "destination") |
| `destination_country(int)` | (COUNTRY, "destination") |
| `destination(int)` | (LOCATION, "destination") |
| `outbound_departure_year_month(date)` | (YEARMONTH, "outboundDeparture") |
| `outbound_departure_day(date)` | (DAY, "outboundDeparture") |
| `inbound_departure_year_month(date)` | (YEARMONTH, "inboundDeparture") |
| `inbound_departure_day(date)` | (DAY, "inboundDeparture") |
| `is_direct(bool)` | (DIRECT, "") |
| `marketing_carrier(int)` | (MARKETING_CARRIER, "") |

The builder must also support `any_direct()` (class method or module-level sentinel) for wildcard DIRECT. Wildcards can only be trailing.

**KeySchema:**
- Ordered component list
- `.key_builder() -> KeyBuilder`
- `.to_string() -> str` — the FCP file store name (PRODUCTION CRITICAL)
- `.signature() -> list[Component]`
- `.encoded_length() -> int`
- `.get_open_jaw_filter() -> OpenJawFilter`
- **Must be picklable** (PySpark ships UDF closures via pickle)

**KeySchema.to_string() format:**

```
prefix + "_" + "_".join(name + display_name for each component)
```

Display names (PascalCase, exact):

| Component type | Display name |
|---|---|
| AIRPORT | `Airport` |
| CITY | `City` |
| COUNTRY | `Country` |
| LOCATION | `Location` |
| YEARMONTH | `YearMonth` |
| DAY | `Day` |
| DIRECT | `Direct` |
| MARKETING_CARRIER | `MarketingCarrier` |

Component names: `"origin"`, `"destination"`, `"outboundDeparture"`, `"inboundDeparture"`, or `""` (empty for DIRECT and MARKETING_CARRIER).

Format is `name + displayName` with no separator. Empty name means just the display name.

Example: `oneway_MarketingCarrier_originAirport_destinationAirport_outboundDepartureYearMonth_Direct`

**OpenJawFilter derivation (exact Java logic):**

```python
if prefix == "return":
    has_origin = any(c.type == AIRPORT and c.name == "origin" for c in components)
    has_dest = any(c.type == AIRPORT and c.name == "destination" for c in components)
    if has_origin and has_dest:
        return OpenJawFilter.BOTH
    elif has_origin:
        return OpenJawFilter.ORIGIN
    elif has_dest:
        return OpenJawFilter.DESTINATION
    else:
        return OpenJawFilter.NONE
else:
    return OpenJawFilter.NONE  # All oneway schemas
```

Only AIRPORT components matter — CITY, COUNTRY, LOCATION do NOT affect OpenJawFilter.

**KeySchema.signature() and disjoint checks:**

`signature()` returns the list of `(type, name)` pairs. Used with set-disjoint to check if a component is in the schema. `disjoint(schema.signature(), [(AIRPORT, "origin")])` returns `True` if the schema does NOT contain an AIRPORT origin component.

### 4.3 — keys.py

Load `schemas.json` from the `taps_keys_schemas` package:

```python
from importlib.resources import files
import json

_data = json.loads(files("taps_keys_schemas").joinpath("schemas.json").read_text())
```

Build `OneWay` and `Return` as namespace objects (or classes with class attributes) where each attribute is a `KeySchema` instance named after the schema's key in schemas.json.

### 4.4 — website_id.py

`encode_to_fixed32(website_id: str) -> bytes` — 4-byte big-endian encoding of the first 4 ASCII characters. Used for Spark partitioning.

### 4.5 — __init__.py

Re-export the public API:

```python
from .keys import OneWay, Return
from .schema import KeySchema, KeyBuilder, Key
from .encoding import to_base32
__all__ = ["OneWay", "Return", "KeySchema", "KeyBuilder", "Key", "to_base32"]
```

---

## Section 5 — Repo Harness

### 5.1 — pyproject.toml

```toml
[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.build_meta"

[project]
name = "taps-keys"
version = "1.0.0"
description = "Python implementation of Skyscanner taps-keys encoding library"
requires-python = ">=3.10"
dependencies = ["taps-keys-schemas>=1.0"]

[tool.setuptools.packages.find]
where = ["src"]
include = ["taps_keys*"]
```

### 5.2 — Makefile

```makefile
.PHONY: validate test lint

validate:
	python scripts/validate.py

test:
	python -m pytest tests/ -v

lint:
	python -m flake8 src/ tests/
```

### 5.3 — README.md

**Always overwrite this file.** Derive the content from the actual code:

1. Read `src/taps_keys/keys.py` to get the actual available schemas and namespaces.
2. Read `src/taps_keys/schema.py` to understand `KeyBuilder`, `Key`, `KeySchema` public API.
3. Read `src/taps_keys/encoding.py` to get the base-32 alphabet and component types.
4. Read `.github/workflows/ci.yml` to list the validation layers that run in CI.
5. Read `scripts/validate.py` to describe how to run validation locally.

The README must accurately reflect the code. Cover: what this library is, installation, a usage example derived from the actual API, available schemas, `to_string()` format, wildcard components, how schemas are loaded, base-32 alphabet, and validation layers with CI vs workflow split.

```python
from taps_keys.keys import OneWay
from datetime import date

key = (OneWay.AIRPORT_AIRPORT_DAY_OW
    .key_builder()
    .origin_airport(13554)
    .destination_airport(13555)
    .outbound_departure_year_month(date(2018, 7, 8))
    .outbound_departure_day(date(2018, 7, 8))
    .is_direct(True)
    .build()
    .encode())

assert key == "0d7i0d7ji681"
```

How schemas.json is loaded from the taps-keys-schemas package at runtime.

### 5.4 — CONTRIBUTING.md

Dev setup, `make validate`, how to sync schemas, how new schemas flow (update schemas.json in taps-keys-schemas repo, then `pip install --upgrade taps-keys-schemas && make validate` here).

### 5.5 — CLAUDE.md

Constitution for AI agents working in this repo:
- Encoding rules are immutable — any change requires updating the schemas repo first
- Never bundle schemas.json — always import from taps-keys-schemas package using `importlib.resources`
- Never write fixture-loading code — the contract runner in taps-keys-fixtures handles all fixture validation
- `scripts/validate.py` must call the contract runner, not its own fixture tests
- KeySchema must remain picklable — required for PySpark UDF serialisation
- Always run `make validate` before committing

## Validation layers

The encoding is validated across six independent layers. Understanding what each catches helps diagnose failures correctly:

| Layer | What | Where |
|---|---|---|
| **L1** | `encode()` byte-for-byte vs 2130 golden cases | contract runner |
| **L2** | `schema.signature()` and 6 disjoint booleans per schema | contract runner |
| **L3** | `to_string()`, `to_string('|')`, `encoded_length`, `OpenJawFilter` | contract runner |
| **Unit tests** | Base-32 boundaries, YEARMONTH overflow, builder edge cases, picklability | pytest |
| **L5** | Python vs Java on same 2130 inputs **live** — no fixture files in chain | Fabro workflow + CI |
| **L6** | 14,200 random fuzz inputs (Java generates, Python encodes) | Fabro workflow + CI |

**L5 and L6 are the strongest guarantees.** A port that passes L1–L3 via a lookup table will fail L5 on the first out-of-fixture input.

**If L1–L3 pass but L5 fails**: the encoding is algorithmically wrong for inputs outside the 15 fixed sets. Fix the formula.

**If L5 passes but L6 fails**: there is an edge case in random combinations (unusual date/carrier boundary). The fuzz output identifies the exact failing input.

### 5.6 — scripts/validate.py

```python
import subprocess
import sys

# Run contract runner from taps-keys-fixtures (Layers L1-L3)
result = subprocess.run(
    [sys.executable, "-m", "taps_keys_fixtures.runner.test_runner",
     "--module", "taps_keys"],
    capture_output=True, text=True
)
print(result.stdout)
if result.returncode != 0:
    print(result.stderr)
    sys.exit(1)

# Run unit tests (Layer 4)
result = subprocess.run(
    [sys.executable, "-m", "pytest", "tests/", "-v"],
    capture_output=True, text=True
)
print(result.stdout)
if result.returncode != 0:
    print(result.stderr)
    sys.exit(1)

print("All validation layers passed.")
```

### 5.7 — .claude/agents/sync-schemas.md

Skill that syncs the local schemas dependency:
1. Runs `pip install -e ../taps-keys-schemas` (install from sibling local repo)
2. Runs `make validate`
3. Reports what changed and whether all tests still pass

Note: In production, this will use `pip install --upgrade taps-keys-schemas` from Artifactory. During development, use the local path.

---

## Section 6 — Unit Tests

Write these from scratch. Do NOT read fixture files from any package.

### 6.1 — tests/test_encoding.py

- `to_base32(0, 4)` produces `"0000"`
- `to_base32(13554, 4)` produces `"0d7i"` (lowercase)
- `to_base32(31, 1)` produces `"v"` (last letter of alphabet)
- `to_base32(32, 2)` produces `"10"` (carry)
- `to_base32(1024, 2)` produces `"100"` (3 chars — YEARMONTH overflow, allowed)
- Carrier -32768 encodes to `"0000"` (min boundary)
- Carrier 32767 encodes to `"1vvv"` (max boundary)
- Carrier -32769 raises `ValueError` (below range)
- Carrier 32768 raises `ValueError` (above range)
- YEARMONTH January 1970 (value 0) encodes to `"00"`
- YEARMONTH July 2018 (value 582) encodes to `"i6"`

### 6.2 — tests/test_schema.py

- Builder silently ignores extra fields not in schema
- Builder raises when required schema field not set
- Wildcards must be trailing (value after wildcard raises)
- KeySchema is picklable (`pickle.dumps`/`pickle.loads` roundtrip)
- `to_string()` matches format: `prefix_componentName_...`
- `encoded_length()` equals sum of component widths
- `key.to_string()` for Set A AIRPORT_AIRPORT_DAY_OW produces `"13554-13555-582-8-1"`
- `key.to_string("|")` produces `"13554|13555|582|8|1"`
- OpenJawFilter: return schema with both airports gives BOTH, oneway always NONE

### 6.3 — tests/test_website_id.py

- `encode_to_fixed32("GBUK")` produces expected 4-byte big-endian bytes
- `encode_to_fixed32` raises for non-4-char input

---

## Section 7 — Repo Structure

```
taps-keys-python/
  pyproject.toml
  Makefile
  README.md
  CONTRIBUTING.md
  CLAUDE.md
  src/taps_keys/
    __init__.py
    encoding.py
    schema.py
    keys.py
    website_id.py
  tests/
    test_encoding.py
    test_schema.py
    test_website_id.py
  scripts/
    validate.py
  .claude/
    agents/
      sync-contract.md
```

---

## Section 8 — Prohibited

- Do not read fixture files from any package (`golden_encodings.json`, `golden_signatures.json`). Implement the encoding algorithm, not a lookup table.
- Do not hardcode value-to-encoding mappings.
- Do not produce uppercase letters in base-32 output.
- Do not bundle schemas.json — import from `taps_keys_schemas` package.
- Do not install or depend on `taps-keys-fixtures`. The contract runner is invoked by `scripts/validate.py` but is not a build dependency.

---

## Section 9 — Self-Checks Before Submitting

Run these mentally or as assertions before declaring completion:

- [ ] `to_base32(0, 4)` produces `"0000"`
- [ ] `to_base32(13554, 4)` produces `"0d7i"` (lowercase only)
- [ ] Carrier -32768 encodes to `"0000"`, carrier 32767 encodes to `"1vvv"`
- [ ] YEARMONTH value 1024 produces `"100"` (3 chars, allowed by range check despite 2-char field width)
- [ ] `key.to_string()` for Set A AIRPORT_AIRPORT_DAY_OW produces `"13554-13555-582-8-1"`
- [ ] `KeySchema.to_string()` for AIRPORT_AIRPORT_DAY_OW produces `"oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct"`
- [ ] Builder silently ignores fields not in schema
- [ ] `KeySchema` survives `pickle.dumps` / `pickle.loads` roundtrip
- [ ] Can you encode a value you have never seen in the examples? (proves algorithmic, not lookup-based)
- [ ] All 142 schemas load from `taps_keys_schemas` without error
- [ ] `validate.py` calls the contract runner, NOT your own fixture tests
- [ ] The contract runner passes L1–L3 for all 2130 fixture cases (142 schemas × 15 input sets)
- [ ] After the workflow validates L1–L3, two further Java-binary parity gates run automatically:
  - **L5**: `EncodeMain` generates live Java outputs for all 2130 cases; Python must match on `encode()`, `to_string()`, and `to_string('|')`
  - **L6**: `FuzzEncoder` generates 14,200 random inputs (seed=42); Python must match on all three output forms
  - These run via the workflow's `validate_java_parity` and `validate_fuzz_parity` nodes — you do not need to implement them, but your encoding must be correct or they will fail
- [ ] The e2e smoke: `OneWay.AIRPORT_AIRPORT_DAY_OW` with LHR→EDI inputs of Set A must produce `"0d7i0d7ji681"`
