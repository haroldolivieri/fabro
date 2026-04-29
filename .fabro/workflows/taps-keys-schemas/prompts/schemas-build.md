# Workflow Agent: Build the taps-keys-schemas Repo

**SCOPE: You must ONLY create and modify files inside `taps-keys-schemas/`. Do not read, write, or interact with `taps-keys-fixtures/` or `taps-keys-python/`. Those repos are handled by separate workflow stages.**

## FIRST: Create .gitignore

Before creating any other file, create `taps-keys-schemas/.gitignore`:
```
dist/
build/
*.egg-info/
__pycache__/
*.pyc
.pytest_cache/
.DS_Store
```

You are building the `taps-keys-schemas` repo -- a language-neutral, single source of truth for Skyscanner's flight pricing key schemas. This repo will be consumed by both the existing Java `taps-keys` library and a new Python port. Your job is to extract schema definitions from the Java source, produce `schemas.json`, write structural validation, build a JSON Schema, and wire up all repo tooling (CLAUDE.md, Makefile, pyproject.toml, add-schema skill, README, CONTRIBUTING).

---

## Section 1 -- Context and Goal

### What is taps-keys?

`taps-keys` is Skyscanner's Java library for encoding flight pricing lookup keys. It defines 142 key schemas (71 one-way, 71 return) that map flight route + date + directionality combinations into compact base-32 encoded strings. These strings are used as lookup keys in the File Cache Proxy (FCP) -- Skyscanner's distributed cache for aggregated pricing data.

### What is the schemas repo?

The `taps-keys-schemas` repo is the canonical definition of what keys exist and how they are structured. It contains:

- **schemas.json** -- all 142 schema definitions with ordered component lists, derived properties (to_string, encoded_length, open_jaw_filter), and component type metadata
- **schemas.schema.json** -- JSON Schema that validates schemas.json
- **validate.py** -- structural validation script that checks schemas.json internally AND cross-references against golden fixture data
- **Claude Code harness** -- CLAUDE.md constitution, add-schema skill, Makefile, pyproject.toml, README, CONTRIBUTING

### Why it exists

Both the Java and Python pipelines must produce byte-for-byte identical keys for the same inputs. Without a shared schema definition, each language maintains its own schema list and the two can drift silently -- a schema added to one but forgotten in the other, a component order changed inconsistently, an encoding width mismatch. The schemas repo makes these failures impossible by construction: schemas live in one place, and both libraries are downstream consumers.

---

## Section 2 -- Input File

The setup step has already run `SchemaExporter` (a Java tool in the fat JAR) against the compiled taps-keys library and written the output to `/tmp/schema_seed.json`. Read this file — it is the authoritative list of all schemas.

**Path:** `/tmp/schema_seed.json`

This file is a JSON array. Each element has:

```json
{
  "name": "AIRPORT_AIRPORT_DAY_OW",
  "prefix": "oneway",
  "to_string": "oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct",
  "encoded_length": 12,
  "open_jaw_filter": "NONE"
}
```

You must process ALL entries in schema_seed.json — the count varies by library version (typically 142 but may change). Do NOT hardcode the count.

### Parsing to_string to recover the component list

The `to_string` field encodes the full component list. Parse it as:

```
prefix + "_" + part1 + "_" + part2 + ...
```

Each `part` is `componentName + DisplayName` (concatenated, no separator between them). Reverse the display name mapping to recover type and name:

| part (suffix match) | Component type | Component name |
|---|---|---|
| ends with `Airport` | AIRPORT | everything before `Airport` |
| ends with `City` | CITY | everything before `City` |
| ends with `Country` | COUNTRY | everything before `Country` |
| ends with `Location` | LOCATION | everything before `Location` |
| ends with `YearMonth` | YEARMONTH | everything before `YearMonth` |
| ends with `Day` | DAY | everything before `Day` |
| exactly `Direct` | DIRECT | "" |
| exactly `MarketingCarrier` | MARKETING_CARRIER | "" |

Example: `to_string = "oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct"`
- Split on `_` after prefix: `["originAirport", "destinationAirport", "outboundDepartureYearMonth", "outboundDepartureDay", "Direct"]`
- `originAirport` → type=AIRPORT, name="origin"
- `destinationAirport` → type=AIRPORT, name="destination"
- `outboundDepartureYearMonth` → type=YEARMONTH, name="outboundDeparture"
- `outboundDepartureDay` → type=DAY, name="outboundDeparture"
- `Direct` → type=DIRECT, name=""

**Important:** Split on `_` only after stripping the prefix. The prefix itself contains `_` in some cases for return schemas? No — prefix is always `oneway` or `return`, no underscores. Safe to split the full to_string on `_` and treat the first element as prefix, remainder as parts.

### Component type properties

After recovering type and name, add the fixed `bits` and `encoded_length` for each component:

| Type | bits | encoded_length |
|---|---|---|
| AIRPORT | 16 | 4 |
| CITY | 16 | 4 |
| COUNTRY | 16 | 4 |
| LOCATION | 16 | 4 |
| YEARMONTH | 10 | 2 |
| DAY | 5 | 1 |
| DIRECT | 1 | 1 |
| MARKETING_CARRIER | 16 | 4 |

---

## Section 3 -- schemas.json Format

Produce `taps_keys_schemas/schemas.json` with this exact structure:

```json
{
  "oneway": [
    {
      "name": "AIRPORT_AIRPORT_DAY_OW",
      "prefix": "oneway",
      "components": [
        {"type": "AIRPORT", "name": "origin", "bits": 16, "encoded_length": 4},
        {"type": "AIRPORT", "name": "destination", "bits": 16, "encoded_length": 4},
        {"type": "YEARMONTH", "name": "outboundDeparture", "bits": 10, "encoded_length": 2},
        {"type": "DAY", "name": "outboundDeparture", "bits": 5, "encoded_length": 1},
        {"type": "DIRECT", "name": "", "bits": 1, "encoded_length": 1}
      ],
      "to_string": "oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct",
      "encoded_length": 13,
      "open_jaw_filter": "BOTH"
    }
  ],
  "return": [...]
}
```

### Top-level structure

- `"oneway"`: array of 71 schema objects extracted from `Keys.OneWay`
- `"return"`: array of 71 schema objects extracted from `Keys.Return`

### Schema object fields

Each schema object has these fields:

| Field | Type | Description |
|---|---|---|
| `name` | string | Java field name (e.g. `AIRPORT_AIRPORT_DAY_OW`) |
| `prefix` | string | `"oneway"` or `"return"` (from `KeySchema.builder(prefix)`) |
| `components` | array | Ordered list of component objects |
| `to_string` | string | Deterministic toString derivation (see Section 5) |
| `encoded_length` | integer | Sum of all component encoded_lengths |
| `open_jaw_filter` | string | One of `"BOTH"`, `"ORIGIN"`, `"DESTINATION"`, `"NONE"` (see Section 6) |

### Component object fields

Each component object has these fields:

| Field | Type | Description |
|---|---|---|
| `type` | string | Component type from the mapping table |
| `name` | string | Component name from the mapping table (empty string for DIRECT, MARKETING_CARRIER) |
| `bits` | integer | Bit width for this component type |
| `encoded_length` | integer | Number of base-32 characters for this component type |

### Component type properties

| Type | bits | encoded_length |
|---|---|---|
| AIRPORT | 16 | 4 |
| CITY | 16 | 4 |
| COUNTRY | 16 | 4 |
| LOCATION | 16 | 4 |
| YEARMONTH | 10 | 2 |
| DAY | 5 | 1 |
| DIRECT | 1 | 1 |
| MARKETING_CARRIER | 16 | 4 |

---

## Section 4 -- Extraction Rules

For each entry in `/tmp/schema_seed.json`:

1. Take `name`, `prefix`, `to_string`, `encoded_length`, `open_jaw_filter` directly from the seed.
2. Parse `to_string` to reconstruct the `components` array (see the parsing rules in Section 2).
3. Add `bits` and `encoded_length` to each component from the component type properties table.
4. Do NOT re-derive `to_string`, `encoded_length`, or `open_jaw_filter` — copy them verbatim from the seed. They come from the compiled Java library and are already correct.

### Example: Converting a seed entry

Seed entry:
```json
{"name": "CARRIER_CITY_COUNTRY_ANYTIME_OW", "prefix": "oneway",
 "to_string": "oneway_MarketingCarrier_originCity_destinationCountry_outboundDepartureYearMonth_Direct",
 "encoded_length": 15, "open_jaw_filter": "NONE"}
```

Parse `to_string` parts (after stripping prefix `oneway_`):
- `MarketingCarrier` → type=MARKETING_CARRIER, name=""
- `originCity` → type=CITY, name="origin"
- `destinationCountry` → type=COUNTRY, name="destination"
- `outboundDepartureYearMonth` → type=YEARMONTH, name="outboundDeparture"
- `Direct` → type=DIRECT, name=""

Result in schemas.json:
```json
{
  "name": "CARRIER_CITY_COUNTRY_ANYTIME_OW",
  "prefix": "oneway",
  "components": [
    {"type": "MARKETING_CARRIER", "name": "", "bits": 16, "encoded_length": 4},
    {"type": "CITY", "name": "origin", "bits": 16, "encoded_length": 4},
    {"type": "COUNTRY", "name": "destination", "bits": 16, "encoded_length": 4},
    {"type": "YEARMONTH", "name": "outboundDeparture", "bits": 10, "encoded_length": 2},
    {"type": "DIRECT", "name": "", "bits": 1, "encoded_length": 1}
  ],
  "to_string": "oneway_MarketingCarrier_originCity_destinationCountry_outboundDepartureYearMonth_Direct",
  "encoded_length": 15,
  "open_jaw_filter": "NONE"
}
```

---

## Section 5 -- toString Derivation

Each schema's `to_string` field is deterministically derived from its prefix and component list:

```
prefix + "_" + "_".join(component_name + display_name for each component)
```

Display name mapping:

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

For each component, concatenate `name + display_name` with no separator between them. When `name` is empty (DIRECT, MARKETING_CARRIER), the result is just the display name.

Examples:
- Component `{"type": "AIRPORT", "name": "origin"}` -> `"originAirport"`
- Component `{"type": "YEARMONTH", "name": "outboundDeparture"}` -> `"outboundDepartureYearMonth"`
- Component `{"type": "DIRECT", "name": ""}` -> `"Direct"`
- Component `{"type": "MARKETING_CARRIER", "name": ""}` -> `"MarketingCarrier"`

Full example: prefix `"oneway"`, components `[MARKETING_CARRIER/"", AIRPORT/"origin", AIRPORT/"destination", YEARMONTH/"outboundDeparture", DIRECT/""]` produces:
`oneway_MarketingCarrier_originAirport_destinationAirport_outboundDepartureYearMonth_Direct`

---

## Section 6 -- OpenJawFilter Derivation

The `open_jaw_filter` field is derived from the component list:

**For all oneway schemas:** always `"NONE"`.

**For return schemas:** check for presence of `(AIRPORT, "origin")` and `(AIRPORT, "destination")` components:

| Has AIRPORT/origin | Has AIRPORT/destination | open_jaw_filter |
|---|---|---|
| yes | yes | `"BOTH"` |
| yes | no | `"ORIGIN"` |
| no | yes | `"DESTINATION"` |
| no | no | `"NONE"` |

Note: only AIRPORT type counts for this check. CITY, COUNTRY, and LOCATION do not affect open_jaw_filter.

---

## Section 7 -- validate.py

Create `taps-keys-schemas/scripts/validate.py`. This script validates `schemas.json` structurally AND cross-references against golden fixture data from a separate fixtures directory.

**Interface:**

```bash
python scripts/validate.py --fixtures-dir /path/to/taps-keys-fixtures
```

**Arguments:**
- `--fixtures-dir` (required): path to the directory containing `golden_encodings.json` and `golden_signatures.json`

The script loads schemas.json and schemas.schema.json from `taps_keys_schemas/` (relative to the repo root), and loads fixture files from the provided `--fixtures-dir` path.

### All 13 checks:

### Check 1: JSON Schema conformance

Load `taps_keys_schemas/schemas.schema.json` and validate `taps_keys_schemas/schemas.json` against it using `jsonschema.validate()`.

```python
import jsonschema

with open("taps_keys_schemas/schemas.schema.json") as f:
    json_schema = json.load(f)
with open("taps_keys_schemas/schemas.json") as f:
    schemas = json.load(f)

jsonschema.validate(instance=schemas, schema=json_schema)
```

### Check 2: Schema count

The expected count comes from the schema seed at `/tmp/schema_seed.json` — count entries with `prefix == "oneway"` and `prefix == "return"` separately. This matches whatever the compiled Java library exports (typically 71+71=142 but may change with library updates).

```python
import json as _json
seed = _json.load(open("/tmp/schema_seed.json"))
expected_oneway = sum(1 for s in seed if s["prefix"] == "oneway")
expected_return = sum(1 for s in seed if s["prefix"] == "return")
assert len(schemas["oneway"]) == expected_oneway, \
    f"Expected {expected_oneway} oneway schemas (from seed), got {len(schemas['oneway'])}"
assert len(schemas["return"]) == expected_return, \
    f"Expected {expected_return} return schemas (from seed), got {len(schemas['return'])}"
```

### Check 3: Valid component types

Every component type used in any schema must be one of: `AIRPORT`, `CITY`, `COUNTRY`, `LOCATION`, `YEARMONTH`, `DAY`, `DIRECT`, `MARKETING_CARRIER`.

```python
VALID_TYPES = {"AIRPORT", "CITY", "COUNTRY", "LOCATION", "YEARMONTH", "DAY", "DIRECT", "MARKETING_CARRIER"}

for section in ("oneway", "return"):
    for schema in schemas[section]:
        for comp in schema["components"]:
            assert comp["type"] in VALID_TYPES, f"Invalid component type '{comp['type']}' in {schema['name']}"
```

### Check 4: No duplicate schema names

No schema name appears in both `oneway` and `return`, and no duplicates within either section.

```python
oneway_names = [s["name"] for s in schemas["oneway"]]
return_names = [s["name"] for s in schemas["return"]]
assert len(oneway_names) == len(set(oneway_names)), "Duplicate names in oneway"
assert len(return_names) == len(set(return_names)), "Duplicate names in return"
overlap = set(oneway_names) & set(return_names)
assert not overlap, f"Schema names in both oneway and return: {overlap}"
```

### Check 5: toString derivation

Each schema's `to_string` field must match the deterministic formula:

```
prefix + "_" + "_".join(name + display_name for each component)
```

Display name mapping:

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

```python
DISPLAY_NAMES = {
    "AIRPORT": "Airport",
    "CITY": "City",
    "COUNTRY": "Country",
    "LOCATION": "Location",
    "YEARMONTH": "YearMonth",
    "DAY": "Day",
    "DIRECT": "Direct",
    "MARKETING_CARRIER": "MarketingCarrier",
}

def derive_to_string(prefix, components):
    parts = []
    for comp in components:
        display = DISPLAY_NAMES[comp["type"]]
        parts.append(comp["name"] + display)
    return prefix + "_" + "_".join(parts)

for section in ("oneway", "return"):
    for schema in schemas[section]:
        expected = derive_to_string(schema["prefix"], schema["components"])
        assert schema["to_string"] == expected, \
            f"{schema['name']}: to_string expected '{expected}', got '{schema['to_string']}'"
```

### Check 6: encoded_length consistency

For each schema, `encoded_length` must equal the sum of component `encoded_length` values:

```python
for section in ("oneway", "return"):
    for schema in schemas[section]:
        expected_length = sum(comp["encoded_length"] for comp in schema["components"])
        assert schema["encoded_length"] == expected_length, \
            f"{schema['name']}: encoded_length expected {expected_length}, got {schema['encoded_length']}"
```

### Check 7: OpenJawFilter derivation

For each return schema, verify `open_jaw_filter` matches the derivation from component presence (Section 6). For all oneway schemas, verify `open_jaw_filter` is `"NONE"`.

```python
for schema in schemas["oneway"]:
    assert schema["open_jaw_filter"] == "NONE", \
        f"{schema['name']}: oneway schema must have open_jaw_filter NONE, got '{schema['open_jaw_filter']}'"

for schema in schemas["return"]:
    has_origin_airport = any(
        c["type"] == "AIRPORT" and c["name"] == "origin" for c in schema["components"]
    )
    has_dest_airport = any(
        c["type"] == "AIRPORT" and c["name"] == "destination" for c in schema["components"]
    )
    if has_origin_airport and has_dest_airport:
        expected = "BOTH"
    elif has_origin_airport:
        expected = "ORIGIN"
    elif has_dest_airport:
        expected = "DESTINATION"
    else:
        expected = "NONE"
    assert schema["open_jaw_filter"] == expected, \
        f"{schema['name']}: open_jaw_filter expected '{expected}', got '{schema['open_jaw_filter']}'"
```

### Check 8: Cross-ref -- every schema name in golden_encodings.json exists in schemas.json

```python
all_schema_names = {s["name"] for s in schemas["oneway"]} | {s["name"] for s in schemas["return"]}
fixture_schema_names = {entry["schema"] for entry in encodings}

missing_from_schemas = fixture_schema_names - all_schema_names
assert not missing_from_schemas, f"Schemas in fixtures but not in schemas.json: {missing_from_schemas}"
```

### Check 9: Cross-ref -- every schema in schemas.json has fixture entries

The schema seed comes from the same library version as the golden fixtures, so they must match exactly.

```python
missing_from_fixtures = all_schema_names - fixture_schema_names
assert not missing_from_fixtures, f"Schemas in schemas.json missing from fixtures: {missing_from_fixtures}"
```

### Check 10: Cross-ref -- golden_encodings.json has SETS_PER_SCHEMA entries per schema

The fixture generator produces 13 base sets (A, B, C, D, F, G, H, I, J, K, M, N, Q) plus Set L (per-component hierarchy) and Set E (wildcard isDirect) = 15 sets per schema. The count is self-computed from the generator; do not hardcode 15 — derive it from the fixture data itself.

```python
import json as _json
from collections import Counter

seed = _json.load(open("/tmp/schema_seed.json"))
TOTAL_SCHEMAS = len(seed)
schema_counts = Counter(entry["schema"] for entry in encodings)
# All schemas must have the same number of sets; derive expected from first schema
counts = list(schema_counts.values())
assert len(set(counts)) == 1, f"Inconsistent set counts across schemas: {dict(schema_counts)}"
SETS_PER_SCHEMA = counts[0]
for schema_name, count in schema_counts.items():
    assert count == SETS_PER_SCHEMA, f"Schema {schema_name} has {count} encoding entries, expected {SETS_PER_SCHEMA}"
expected_total = TOTAL_SCHEMAS * SETS_PER_SCHEMA
assert len(encodings) == expected_total, f"Expected {expected_total} encoding entries ({TOTAL_SCHEMAS} x {SETS_PER_SCHEMA}), got {len(encodings)}"
```

### Check 11: Cross-ref -- toString values from fixtures match schemas.json to_string field

For each entry in `golden_encodings.json`, the `schema_to_string` value must match the `to_string` field of the corresponding schema in schemas.json.

```python
schema_lookup = {}
for section in ("oneway", "return"):
    for schema in schemas[section]:
        schema_lookup[schema["name"]] = schema

for entry in encodings:
    schema_name = entry["schema"]
    if schema_name in schema_lookup:
        expected = schema_lookup[schema_name]["to_string"]
        actual = entry["schema_to_string"]
        assert expected == actual, \
            f"{schema_name}: to_string in schemas.json is '{expected}', fixture has '{actual}'"
```

### Check 12: Cross-ref -- golden_signatures.json disjoint values are consistent with component lists

For each entry in `golden_signatures.json`, verify that each disjoint boolean is consistent with whether the schema's component list contains the probed component:

```python
DISJOINT_CHECKS = {
    "origin_airport_disjoint": ("AIRPORT", "origin"),
    "destination_airport_disjoint": ("AIRPORT", "destination"),
    "outbound_year_month_disjoint": ("YEARMONTH", "outboundDeparture"),
    "outbound_day_disjoint": ("DAY", "outboundDeparture"),
    "inbound_year_month_disjoint": ("YEARMONTH", "inboundDeparture"),
    "inbound_day_disjoint": ("DAY", "inboundDeparture"),
}

for entry in signatures:
    schema_name = entry["schema"]
    schema = schema_lookup.get(schema_name)
    if not schema:
        continue  # caught by check 8/9
    component_tuples = [(c["type"], c["name"]) for c in schema["components"]]

    for field, probe in DISJOINT_CHECKS.items():
        expected_disjoint = probe not in component_tuples
        actual_disjoint = entry[field]
        assert expected_disjoint == actual_disjoint, \
            f"{schema_name}: {field} expected {expected_disjoint}, got {actual_disjoint}"
```

### Check 13: Valid JSON structure throughout

All fixture files must be valid JSON with the expected structure:
- `golden_encodings.json`: array of objects, each with at least `schema`, `input_set`, `encoded_key`, `schema_to_string`, `encoded_length`, `open_jaw_filter`. Note: `inbound_date` is present on all entries but is `null` for oneway schemas (which have no inbound components) — treat null as valid.
- `golden_signatures.json`: array of objects, each with at least `schema` and the 6 disjoint fields

**Exit behavior:**

- Run all 13 checks. Every check runs even if a previous one failed.
- Print each check result: `"CHECK N: PASS"` or `"CHECK N: FAIL -- message"`.
- Exit 0 if all pass, exit 1 if any fail.

**Dependencies:** `jsonschema` (for Check 1). Add to `pyproject.toml` as a dependency.

---

## Section 8 -- schemas.schema.json

Create `taps_keys_schemas/schemas.schema.json`. This is a JSON Schema that validates the structure of `schemas.json`.

The schema must enforce:

- Top-level object with required keys `"oneway"` and `"return"`, no additional properties
- Each key maps to an array of schema objects
- Each schema object has required fields: `name` (string), `prefix` (string, enum `["oneway", "return"]`), `components` (array, minItems 1), `to_string` (string), `encoded_length` (integer, minimum 1), `open_jaw_filter` (string, enum `["BOTH", "ORIGIN", "DESTINATION", "NONE"]`)
- No additional properties on schema objects
- Each component object has required fields: `type` (string, enum of the 8 valid types), `name` (string), `bits` (integer, minimum 1), `encoded_length` (integer, minimum 1)
- No additional properties on component objects

```json
{
  "$schema": "http://json-schema.org/draft-07/schema#",
  "title": "taps-keys Schema Definitions",
  "description": "Defines all 142 key schemas (71 oneway + 71 return) as ordered component lists with derived properties",
  "type": "object",
  "required": ["oneway", "return"],
  "additionalProperties": false,
  "properties": {
    "oneway": {
      "$ref": "#/$defs/schema_array"
    },
    "return": {
      "$ref": "#/$defs/schema_array"
    }
  },
  "$defs": {
    "schema_array": {
      "type": "array",
      "items": {
        "$ref": "#/$defs/schema_object"
      }
    },
    "schema_object": {
      "type": "object",
      "required": ["name", "prefix", "components", "to_string", "encoded_length", "open_jaw_filter"],
      "additionalProperties": false,
      "properties": {
        "name": {"type": "string"},
        "prefix": {"type": "string", "enum": ["oneway", "return"]},
        "components": {
          "type": "array",
          "minItems": 1,
          "items": {
            "$ref": "#/$defs/component_object"
          }
        },
        "to_string": {"type": "string"},
        "encoded_length": {"type": "integer", "minimum": 1},
        "open_jaw_filter": {"type": "string", "enum": ["BOTH", "ORIGIN", "DESTINATION", "NONE"]}
      }
    },
    "component_object": {
      "type": "object",
      "required": ["type", "name", "bits", "encoded_length"],
      "additionalProperties": false,
      "properties": {
        "type": {
          "type": "string",
          "enum": ["AIRPORT", "CITY", "COUNTRY", "LOCATION", "YEARMONTH", "DAY", "DIRECT", "MARKETING_CARRIER"]
        },
        "name": {"type": "string"},
        "bits": {"type": "integer", "minimum": 1},
        "encoded_length": {"type": "integer", "minimum": 1}
      }
    }
  }
}
```

---

## Section 9 -- Claude Code Harness

After building the core package (schemas.json, schemas.schema.json, validate.py), add the full repo harness. The validation gate runs `make validate`, so the package must be structurally valid.

### CLAUDE.md

Create `taps-keys-schemas/CLAUDE.md` with this constitution for AI agents working in the repo:

- `schemas.json` is the single source of truth for all key schema definitions
- Encoding rules (base-32 alphabet, component widths, carrier offset) are immutable -- they are defined by the Java library and must not change
- Both Java and Python libraries consume this package at runtime
- Always run `make validate` before committing any change
- Adding a new schema: use the add-schema skill (`.claude/agents/add-schema.md`)
- Never edit schemas.json by hand without re-running validation
- The `scripts/validate.py` script requires a `--fixtures-dir` argument pointing to the taps-keys-fixtures directory

### add_schema.py helper script

Create `taps-keys-schemas/scripts/add_schema.py`. This script encodes the derivation formulas so agents and humans don't apply them manually (and can't make formula errors).

**Interface:**

```bash
python3 scripts/add_schema.py --prefix oneway --name AIRPORT_AIRPORT_DAY_OW \
  --components "originAirport,destinationAirport,outboundDepartureYearMonth,outboundDepartureDay,Direct"
```

The `--components` argument is a comma-separated list of `to_string` part names (each is `componentName + DisplayName` as it appears in to_string, e.g. `originAirport`, `outboundDepartureYearMonth`, `Direct`, `MarketingCarrier`).

**What the script does:**

1. Parses each component part using the reverse display-name mapping (same table as validate.py Check 5) to recover `type` and `name`.
2. Looks up `bits` and `encoded_length` from the component type properties table.
3. Derives `to_string` = `prefix + "_" + "_".join(parts)`.
4. Derives `encoded_length` = sum of component `encoded_length` values.
5. Derives `open_jaw_filter` using the return-schema AIRPORT presence rule (NONE for oneway).
6. Prints the complete JSON object ready to paste into schemas.json, and also appends it directly to the correct array in `taps_keys_schemas/schemas.json`.
7. Exits 0 on success, exits 1 with a clear error if any component part is unrecognized.

The script must use the SAME display-name mapping and open_jaw_filter logic as `scripts/validate.py` — copy the constants, do not redefine them.

**Example output (printed to stdout before writing):**

```json
{
  "name": "AIRPORT_AIRPORT_DAY_OW",
  "prefix": "oneway",
  "components": [
    {"type": "AIRPORT", "name": "origin", "bits": 16, "encoded_length": 4},
    {"type": "AIRPORT", "name": "destination", "bits": 16, "encoded_length": 4},
    {"type": "YEARMONTH", "name": "outboundDeparture", "bits": 10, "encoded_length": 2},
    {"type": "DAY", "name": "outboundDeparture", "bits": 5, "encoded_length": 1},
    {"type": "DIRECT", "name": "", "bits": 1, "encoded_length": 1}
  ],
  "to_string": "oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct",
  "encoded_length": 12,
  "open_jaw_filter": "NONE"
}
```

### Add-schema skill

Create `taps-keys-schemas/.claude/agents/add-schema.md` with these steps:

1. Ask the user for: prefix (`oneway` or `return`), schema name (Java constant name), and the ordered list of component parts as they appear in the `to_string` format (e.g. `originAirport`, `destinationAirport`, `outboundDepartureYearMonth`, `outboundDepartureDay`, `Direct`).
2. Run the helper script to generate and insert the schema:
   ```bash
   python3 scripts/add_schema.py --prefix <prefix> --name <NAME> --components "<part1>,<part2>,..."
   ```
3. Run `make validate` to verify all 13 structural checks pass.
4. If validation passes, commit the change.
5. Note: golden fixtures must be regenerated from the Java library after adding a schema -- that is a separate step outside this repo.

Do NOT derive `to_string`, `encoded_length`, or `open_jaw_filter` manually. The helper script is the single source of truth for derivation logic — using it prevents formula drift.

### Makefile

Create `taps-keys-schemas/Makefile`:

```makefile
.PHONY: validate build clean add-schema

FIXTURES_DIR ?= ../taps-keys-fixtures

validate:
	python3 scripts/validate.py --fixtures-dir $(FIXTURES_DIR)

build:
	python3 -m build

clean:
	rm -rf dist/ build/ *.egg-info

# Usage: make add-schema PREFIX=oneway NAME=MY_SCHEMA COMPONENTS="originAirport,destinationAirport,Direct"
add-schema:
	python3 scripts/add_schema.py --prefix $(PREFIX) --name $(NAME) --components "$(COMPONENTS)"
```

### pyproject.toml

Create `taps-keys-schemas/pyproject.toml`:

```toml
[build-system]
requires = ["setuptools>=68.0"]
build-backend = "setuptools.build_meta"

[project]
name = "taps-keys-schemas"
version = "1.0.0"
description = "Schema definitions for Skyscanner taps-keys: 142 key schemas consumed by Java and Python libraries"
requires-python = ">=3.10"
dependencies = ["jsonschema>=4.0"]

[tool.setuptools.packages.find]
where = ["."]
include = ["taps_keys_schemas*"]

[tool.setuptools.package-data]
taps_keys_schemas = ["schemas.json", "schemas.schema.json"]
```

### README.md

**Always overwrite this file.** Derive the content from the actual code:

1. Read `taps_keys_schemas/schemas.json` to get the actual schema count (oneway + return).
2. Read `scripts/validate.py` to understand what the 13 checks cover — list them accurately.
3. Read `taps_keys_schemas/schemas.schema.json` to understand the enforced structure.
4. Read `.github/workflows/ci.yml` to describe what CI runs.

The README must accurately reflect the code. Cover: what this repo is, who consumes it, what schemas.json contains (with real counts from the file), how to validate, how to add a schema, installation.

### CONTRIBUTING.md

Create `taps-keys-schemas/CONTRIBUTING.md` covering:

- Dev setup: `pip install -e .` and `pip install jsonschema`
- Running validation: `make validate FIXTURES_DIR=/path/to/taps-keys-fixtures`
- Schema addition workflow: modify schemas.json, run validation, regenerate fixtures from Java
- Do NOT manually edit derived fields (to_string, encoded_length, open_jaw_filter) -- they must match the derivation formulas
- Do NOT modify golden fixture files -- they are generated by the Java library

### __init__.py

Create `taps-keys-schemas/taps_keys_schemas/__init__.py`:

```python
"""taps-keys-schemas: canonical schema definitions for Skyscanner taps-keys."""
```

---

## Section 10 -- Package Structure

Your complete output structure:

```
taps-keys-schemas/
  taps_keys_schemas/
    __init__.py
    schemas.json
    schemas.schema.json
  scripts/
    validate.py
    add_schema.py
  .claude/
    agents/
      add-schema.md
  pyproject.toml
  Makefile
  README.md
  CONTRIBUTING.md
  CLAUDE.md
```

**Total: ~12 files.** The most critical is `schemas.json` (extracted from Java source with all 142 schemas and correctly derived properties).

---

## Section 11 -- Validator Defenses

The `validate.py` script you write will be reviewed by a human. Design it with these principles:

1. **Each check must be simple.** Count comparisons, JSON Schema validation, string formatting, set operations. No complex encoding logic. The validator checks structure and consistency, not encoding correctness.

2. **Each check must be understandable in under 30 seconds.** A reviewer should be able to read a check and immediately verify it is correct. If a check requires more than 30 seconds to understand, simplify it.

3. **No encoding implementation in the validator.** The validator must NEVER implement base-32 encoding, component encoding, or any part of the key encoding algorithm. It validates structural properties only (counts, types, string patterns, cross-references).

4. **Fail fast with specific messages.** Each check prints its result immediately. On failure, the message names the specific check, the specific schema, and the specific discrepancy. No generic "validation failed" messages.

5. **No suppression of failures.** Every check runs even if a previous one failed. The full list of failures is reported at the end so the agent can fix all issues in one retry pass.

6. **Deterministic output.** The same input always produces the same output. No randomness, no timing-dependent checks, no network calls.

---

## Section 12 -- Self-Check Before Signaling Completion

Before signaling completion, verify:

- [ ] Did you read `/tmp/schema_seed.json` and process ALL entries (count matches seed)?
- [ ] Does schemas.json conform to the format in Section 3 (arrays of objects, not dictionaries)?
- [ ] Does every schema object have all 6 required fields (name, prefix, components, to_string, encoded_length, open_jaw_filter)?
- [ ] Does every component object have all 4 required fields (type, name, bits, encoded_length)?
- [ ] Are component bits and encoded_length values correct per the component type properties table?
- [ ] Is to_string derived correctly (prefix + underscore-joined name+DisplayName parts)?
- [ ] Is encoded_length the sum of component encoded_lengths?
- [ ] Is open_jaw_filter derived correctly (NONE for oneway; BOTH/ORIGIN/DESTINATION/NONE for return based on AIRPORT components)?
- [ ] Does schemas.schema.json validate schemas.json without errors?
- [ ] Does `python scripts/validate.py --fixtures-dir <path>` exit 0?
- [ ] Are all harness files present (CLAUDE.md, Makefile, pyproject.toml, README.md, CONTRIBUTING.md, add-schema skill, scripts/add_schema.py)?
- [ ] Does `python3 scripts/add_schema.py --prefix oneway --name TEST --components "originAirport,Direct"` run without error?
- [ ] Does pyproject.toml include schemas.json and schemas.schema.json as package_data?

Read the Java source file, extract all 142 schemas, build the complete repo, and run `make validate` when done.
