# taps-keys-python

Python implementation of Skyscanner's taps-keys encoding library. Produces flight pricing lookup keys for the File Cache Proxy (FCP) that are **byte-for-byte identical** to the Java taps-keys library.

## What it is

Each key is a compact string that identifies a specific flight route + date + directionality combination. FCP consumers depend on exact key matching — a single wrong character means a cache miss in production.

## Installation

```bash
pip install taps-keys
```

The `taps-keys-schemas` package is a required dependency and ships the `schemas.json` data file. It is installed automatically.

## Usage

```python
from taps_keys.keys import OneWay
from datetime import date

key = (
    OneWay.AIRPORT_AIRPORT_DAY_OW
    .key_builder()
    .origin_airport(13554)
    .destination_airport(13555)
    .outbound_departure_year_month(date(2018, 7, 8))
    .outbound_departure_day(date(2018, 7, 8))
    .is_direct(True)
    .build()
    .encode()
)

assert key == "0d7i0d7ji681"
```

### Available schemas

`OneWay` and `Return` are namespace classes. Every attribute is a `KeySchema` instance named after the Java constant (e.g. `OneWay.AIRPORT_AIRPORT_DAY_OW`, `Return.AIRPORT_AIRPORT_DAY_RTN`).

There are 71 one-way and 71 return schemas (142 total).

### Wildcard components

Wildcards represent "any value" for trailing components:

```python
key = (
    OneWay.AIRPORT_AIRPORT_DAY_OW
    .key_builder()
    .origin_airport(13554)
    .destination_airport(13555)
    .outbound_departure_year_month(date(2018, 7, 8))
    .outbound_departure_day(date(2018, 7, 8))
    .any_direct()   # wildcard — DIRECT is omitted from encoded output
    .build()
)

assert key.encode() == "0d7i0d7ji68"
assert key.to_string() == "13554-13555-582-8-*"
```

Wildcards must be trailing — you cannot have a wildcard followed by a concrete value.

### Human-readable format

`key.to_string()` joins the raw integer values (not encoded base-32) with a separator:

```python
key.to_string()    # "13554-13555-582-8-1"
key.to_string("|") # "13554|13555|582|8|1"
```

Values are always integers:
- YEARMONTH → months since Jan 1970 (e.g. July 2018 = 582)
- DAY → day of month (e.g. 8)
- DIRECT → 1 or 0 (not True/False)
- MARKETING_CARRIER → raw signed integer (e.g. -32480), not the +32768 offset

### Schema metadata

```python
schema = OneWay.AIRPORT_AIRPORT_DAY_OW

schema.to_string()       # "oneway_originAirport_destinationAirport_outboundDepartureYearMonth_outboundDepartureDay_Direct"
schema.encoded_length()  # 12
schema.get_open_jaw_filter()  # OpenJawFilter.NONE
schema.signature()       # [(ComponentType.AIRPORT, "origin"), ...]
```

`KeySchema` is picklable — required for PySpark UDF serialisation.

## How schemas.json is loaded

Schemas are loaded at import time from the `taps_keys_schemas` package using `importlib.resources`:

```python
from importlib.resources import files
import json

data = json.loads(files("taps_keys_schemas").joinpath("schemas.json").read_text())
```

Never bundle a local copy of `schemas.json`. Always import from the `taps_keys_schemas` package.

## Base-32 alphabet

`0123456789abcdefghijklmnopqrstuv` — 10 digits + first 22 lowercase letters. Always lowercase. Never uppercase.

## Development

```bash
pip install -e ".[dev]"
make test      # run unit tests
make validate  # run contract runner (L1–L3) + unit tests
make lint      # flake8
```

## Validation layers

| Layer | What it verifies | How to run |
|---|---|---|
| **L1** | `encode()` byte-for-byte vs 2130 golden cases | `make validate` |
| **L2** | `schema.signature()` and disjoint component properties | `make validate` |
| **L3** | `to_string()`, `encoded_length`, `OpenJawFilter` | `make validate` |
| **Unit tests** | Base-32 edge cases, YEARMONTH overflow, builder edge cases, PySpark picklability | `make test` |
| **L5** | Python vs Java on same 2130 inputs live — no fixture files | CI / Fabro workflow |
| **L6** | 14,200 random fuzz inputs (Java generates, Python encodes) | CI / Fabro workflow |

L5 and L6 require the `taps-keys-fixture-gen` Java tool and run automatically in the Fabro migration workflow and in GitHub Actions CI. They are the strongest correctness guarantees: a lookup-table implementation that "passes" L1–L3 will fail on the first L5 input outside the 15 fixed test sets.
