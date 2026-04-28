# taps-keys-schemas

Canonical schema definitions for Skyscanner's `taps-keys` library.

## What is this?

This package is the **single source of truth** for all 142 key schemas used by the `taps-keys` system (71 one-way + 71 return). Each schema defines an ordered list of components that together describe a flight pricing lookup key: origin/destination location types, departure date granularity, and carrier.

The schemas here are extracted from the Java `taps-keys` library and expressed as structured JSON so they can be consumed by both the Java library and any downstream ports (e.g. the Python port).

## What does schemas.json contain?

`taps_keys_schemas/schemas.json` contains all 142 schemas split into `"oneway"` and `"return"` arrays. Each schema object has:

| Field | Description |
|---|---|
| `name` | Java constant name (e.g. `AIRPORT_AIRPORT_DAY_OW`) |
| `prefix` | `"oneway"` or `"return"` |
| `components` | Ordered list of component objects (type, name, bits, encoded_length) |
| `to_string` | Deterministic string representation used to identify the schema |
| `encoded_length` | Total base-32 character length of keys produced by this schema |
| `open_jaw_filter` | `BOTH`, `ORIGIN`, `DESTINATION`, or `NONE` — used for open-jaw flight filtering |

## Who consumes this?

- **Java `taps-keys` library** — the upstream source; schemas here mirror the Java definitions exactly
- **Python `taps-keys` port** — imports this package at runtime to drive key encoding without duplicating schema definitions

## Validation

Run structural validation against golden fixture data:

```bash
make validate FIXTURES_DIR=/path/to/taps-keys-fixtures
```

The validator runs 13 checks covering JSON Schema conformance, schema counts, component type validity, toString derivation, encoded_length arithmetic, open_jaw_filter derivation, and cross-reference against golden encoding and signature fixtures.

## Adding a new schema

Use the Claude Code add-schema skill:

```
.claude/agents/add-schema.md
```

After adding a schema to `schemas.json`, you must regenerate the golden fixture files from the Java library. The Python port will pick up the new schema automatically on the next install.

## Installation

```bash
pip install -e .
```

This makes `taps_keys_schemas` importable and bundles `schemas.json` and `schemas.schema.json` as package data.

## Package structure

```
taps-keys-schemas/
  taps_keys_schemas/
    __init__.py
    schemas.json          ← source of truth
    schemas.schema.json   ← JSON Schema for validation
  scripts/
    validate.py           ← 13-check validator
    generate_schemas.py   ← bootstrap generator (reads Keys.java)
  .claude/
    agents/
      add-schema.md       ← Claude Code skill for adding schemas
  pyproject.toml
  Makefile
  README.md
  CONTRIBUTING.md
  CLAUDE.md
```
