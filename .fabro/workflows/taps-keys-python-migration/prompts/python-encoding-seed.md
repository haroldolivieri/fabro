# taps-keys Python Library — Implementation Seed

## What You're Building

A Python library that encodes flight pricing lookup keys for Skyscanner's File Cache Proxy (FCP). Each key is a compact string that identifies a specific flight route + date + directionality combination. The keys must be **byte-for-byte identical** to what the existing Java library produces — FCP consumers depend on exact key matching.

## The Core Concept

A `KeySchema` defines an ordered list of typed components. A `KeyBuilder` accepts values for those components. `Key.encode()` concatenates each component's fixed-width base-32 encoding.

Think of it like a composite database key: `(origin, destination, month, day, isDirect)` — but instead of storing as separate columns, the values are encoded into a single compact string.

## Base-32 Encoding

The alphabet is `0123456789abcdefghijklmnopqrstuv` (32 characters: 10 digits + first 22 lowercase letters). Always lowercase. Never uppercase.

To encode an integer `n` in base-32 with width `w`:
1. Repeatedly divmod by 32, mapping remainders to the alphabet
2. Left-pad with `'0'` to exactly `w` characters

Python has no built-in base-32 string function. You need a custom encoder:

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

## Component Types

Each component type has a fixed encoding width:

| Type | Width (chars) | Value range | Encoding |
|---|---|---|---|
| AIRPORT | 4 | [0, 65536] | `to_base32(value, 4)` |
| CITY | 4 | [0, 65536] | `to_base32(value, 4)` |
| COUNTRY | 4 | [0, 65536] | `to_base32(value, 4)` |
| LOCATION | 4 | [0, 65536] | `to_base32(value, 4)` |
| YEARMONTH | 2 | [0, 1024] | `to_base32((year - 1970) * 12 + (month - 1), 2)` |
| DAY | 1 | [0, 32] | `to_base32(day_of_month, 1)` |
| DIRECT | 1 | {0, 1} | `to_base32(1 if direct else 0, 1)` |
| MARKETING_CARRIER | 4 | input [-32768, 32767] | `to_base32(carrier_id + 32768, 4)` |

**Every type must validate its input range and raise ValueError on out-of-range values — except YEARMONTH, which replicates Java's boundary behavior (see What NOT To Do).** Silent overflow is the worst failure mode — it produces a valid-looking but wrong key.

## Key.encode()

Concatenate each component's encoded string in schema order. That's it.

If a component is a wildcard (trailing only), stop concatenating — wildcards mean "omit the rest."

## Worked Examples

### Example 1: AIRPORT_AIRPORT_DAY_OW

Schema components: `[AIRPORT/origin, AIRPORT/destination, YEARMONTH/outbound, DAY/outbound, DIRECT]`

Inputs: origin=13554, destination=13555, outbound=2018-07-08, isDirect=true

Step-by-step:
```
1. AIRPORT origin: 13554
   13554 ÷ 32 = 423 remainder 18 → 'i'
   423 ÷ 32 = 13 remainder 7 → '7'
   13 ÷ 32 = 0 remainder 13 → 'd'
   Result: "d7i" → pad to 4: "0d7i"

2. AIRPORT destination: 13555
   13555 ÷ 32 = 423 remainder 19 → 'j'
   423 ÷ 32 = 13 remainder 7 → '7'
   13 ÷ 32 = 0 remainder 13 → 'd'
   Result: "d7j" → pad to 4: "0d7j"

3. YEARMONTH outbound: 2018-07-08
   value = (2018 - 1970) * 12 + (7 - 1) = 48*12 + 6 = 582
   582 ÷ 32 = 18 remainder 6 → '6'
   18 ÷ 32 = 0 remainder 18 → 'i'
   Result: "i6" → pad to 2: "i6"

4. DAY outbound: day 8
   8 < 32, single char: "8"

5. DIRECT: true → 1 → "1"

Concatenate: "0d7i" + "0d7j" + "i6" + "8" + "1" = "0d7i0d7ji681"
```

### Example 2: CARRIER_CITY_COUNTRY_ANYTIME_OW

Schema: `[MARKETING_CARRIER/"", CITY/origin, COUNTRY/destination, YEARMONTH/outbound, DIRECT]`

Inputs: carrier=-32480, origin_city=11135, dest_country=11235, outbound=2018-07-08, isDirect=true

```
1. CARRIER: -32480 + 32768 = 288
   288 ÷ 32 = 9 remainder 0 → '0'
   9 ÷ 32 = 0 remainder 9 → '9'
   Result: "90" → pad to 4: "0090"

2. CITY origin: 11135
   11135 in base-32 = "arv" → pad to 4: "0arv"

3. COUNTRY destination: 11235
   11235 in base-32 = "av3" → pad to 4: "0av3"

4. YEARMONTH: 582 → "i6"

5. DIRECT: true → "1"

Concatenate: "0090" + "0arv" + "0av3" + "i6" + "1" = "00900arv0av3i61"
```

### Example 3: Wildcard (anyDirect)

Same as Example 1 but isDirect=wildcard (trailing):

```
Components 1-4 encode the same: "0d7i0d7ji68"
Component 5 (DIRECT) is wildcard → stop concatenating
Result: "0d7i0d7ji68" (no trailing "1" or "0")
```

### Example 4: January 1970 (boundary)

AIRPORT_AIRPORT_DAY_OW, origin=13554, dest=13555, outbound=1970-01-01, isDirect=true

```
YEARMONTH: (1970-1970)*12 + (1-1) = 0 → "00"
DAY: 1 → "1"
Result: "0d7i0d7j0011"
```

### Example 5: isDirect=false

AIRPORT_AIRPORT_DAY_OW, origin=13554, dest=13555, outbound=2018-07-08, isDirect=false

```
Same as Example 1 except DIRECT: false → 0 → "0"
Result: "0d7i0d7ji680"
```

### Example 6: Return flight

AIRPORT_AIRPORT_DAY_RTN, origin=13554, dest=13555, outbound=2018-07-08, inbound=2018-08-25, isDirect=true

Schema: `[AIRPORT/origin, AIRPORT/dest, YEARMONTH/outbound, YEARMONTH/inbound, DAY/outbound, DAY/inbound, DIRECT]`

```
1-2: "0d7i" + "0d7j"
3. YEARMONTH outbound: 582 → "i6"
4. YEARMONTH inbound: (2018-1970)*12 + (8-1) = 583 → "i7"
5. DAY outbound: 8 → "8"
6. DAY inbound: 25 → 25 in base-32 = "p"
7. DIRECT: true → "1"

Result: "0d7i0d7ji6i78p1"
```

### Example 7: Negative carrier

CARRIER_AIRPORT_AIRPORT_ANYTIME_OW, carrier=-32480, origin=13554, dest=13555, outbound=2018-07-08, isDirect=true

```
CARRIER: -32480 + 32768 = 288 → "0090"
Result: "0090" + "0d7i" + "0d7j" + "i6" + "1" = "00900d7i0d7ji61"
```

### Example 8: Value zero (boundary)

AIRPORT_AIRPORT_DAY_OW, origin=0, dest=0, outbound=1970-01-01, isDirect=true

```
AIRPORT origin: 0 → "0000"
AIRPORT dest: 0 → "0000"
YEARMONTH: 0 → "00"
DAY: 1 → "1"
DIRECT: true → "1"
Result: "000000000011"
```

### Example 9: Maximum carrier

CARRIER_AIRPORT_AIRPORT_ANYTIME_OW, carrier=32767, origin=13554, dest=13555, outbound=2018-07-08, isDirect=true

```
CARRIER: 32767 + 32768 = 65535 → "1vvv"
Result: "1vvv" + "0d7i" + "0d7j" + "i6" + "1" = "1vvv0d7i0d7ji61"
```

### Example 10: Country + anytime (short key)

COUNTRY_ANYWHERE_DESTINATIONCOUNTRY_ANYTIME_OW, origin_country=11236, dest_country=11235, isDirect=false

Schema: `[COUNTRY/origin, COUNTRY/destination, DIRECT]`

```
COUNTRY origin: 11236 in base-32 = "av4" → "0av4"
COUNTRY dest: 11235 → "0av3"
DIRECT: false → "0"
Result: "0av40av30"
```

## Key.to_string() (Human-Readable Format)

`key.to_string(joiner="-")` joins each component's **raw integer value** with the joiner character. Wildcards are shown as `"*"`.

**The values are integers, NOT formatted dates or booleans:**
- AIRPORT/CITY/COUNTRY/LOCATION → the route node ID integer (e.g. `13554`)
- YEARMONTH → the computed months-since-Jan-1970 integer (e.g. `582`), NOT the date string
- DAY → the day-of-month integer (e.g. `8`)
- DIRECT → `1` for true, `0` for false (not the words "true"/"false")
- MARKETING_CARRIER → the raw carrier ID integer (e.g. `-32480`)

### Worked examples

```
AIRPORT_AIRPORT_DAY_OW, Set A (origin=13554, dest=13555, outbound=2018-07-08, isDirect=true):
  key.to_string()    → "13554-13555-582-8-1"
  key.to_string("|") → "13554|13555|582|8|1"

Same with wildcard isDirect:
  key.to_string()    → "13554-13555-582-8-*"

Same with wildcard day+direct (trailing):
  key.to_string()    → "13554-13555-582-*-*"

CARRIER_AIRPORT_AIRPORT_ANYTIME_OW, Set A (carrier=-32480, origin=13554, dest=13555, outbound=2018-07-08, isDirect=true):
  key.to_string()    → "-32480-13554-13555-582-1"
```

Note: carrier appears as the raw signed integer (-32480), not the offset value (288).

## KeySchema.to_string() (PRODUCTION CRITICAL)

This is used as the FCP file store name. Wrong value = data written to wrong location.

Format: `prefix + "_" + join("_", [name + display_name for each component])`

Display names (exact, PascalCase):
- AIRPORT → "Airport", CITY → "City", COUNTRY → "Country", LOCATION → "Location"
- YEARMONTH → "YearMonth", DAY → "Day"
- DIRECT → "Direct", MARKETING_CARRIER → "MarketingCarrier"

Names: "origin", "destination", "outboundDeparture", "inboundDeparture", or "" (empty for DIRECT/MARKETING_CARRIER)

Concatenation: `name + display_name` (no separator). Empty name means just the display name.

Example: `oneway_MarketingCarrier_originAirport_destinationAirport_outboundDepartureYearMonth_Direct`

## KeyBuilder Behavior

The builder stores ALL setter values in a dict keyed by (type, name) tuples. At build() time, ONLY the schema's components are looked up. Extra values are silently ignored.

This is critical: the caller sets ALL fields regardless of schema. The builder picks the right ones.

At build() time:
1. Iterate schema components in order
2. Look up each (type, name) in the dict
3. Missing → raise error
4. Wildcard → all subsequent must also be wildcard
5. Extra dict entries → silently ignored

## KeySchema.signature() and disjoint checks

`signature()` returns the list of (type, name) pairs. Used with set-disjoint to check if a component is in the schema.

`disjoint(schema.signature(), [(AIRPORT, "origin")])` → True if the schema does NOT contain an AIRPORT origin component.

This drives whether a field goes in the key (disjoint=False, skip from value) or in the protobuf value (disjoint=True, include).

## OpenJawFilter

For return schemas: check if components include (AIRPORT, "origin") and/or (AIRPORT, "destination").
- Both → BOTH
- Origin only → ORIGIN
- Destination only → DESTINATION
- Neither → NONE

All oneway schemas → NONE.

## Spark Serialization

KeySchema must be picklable. PySpark ships UDF closures via pickle. Test this explicitly.

## What NOT To Do

- Never hardcode value→encoding mappings. The encoding must be algorithmic.
- Never produce uppercase letters in base-32 output.
- Raise on out-of-range values for all types EXCEPT: replicate the Java YEARMONTH boundary exactly. Java allows value 1024 (maxValue = 2^10) despite it producing 3 chars ("100") for a 2-char field. This is a known Java edge case — do NOT "fix" it. Replicate the same range check (`<= 2^numBits`) and the same silent overflow behavior. Concretely: `to_base32(1024, 2)` → `"100"` (3 chars, not 2 — Java's `padStart` pads but never truncates, so the result overflows the nominal width).
- Never bundle a copy of schemas.json — import it from the taps-keys-contract package using `importlib.resources`:
  ```python
  from importlib.resources import files
  import json
  schemas = json.loads(files("taps_keys_contract").joinpath("schemas.json").read_text())
  ```
- Never write fixture-loading test code — the contract runner handles that.

## Self-Check Before Submitting

- [ ] Can you encode a value you've never seen in the examples?
- [ ] Does `to_base32(0, 4)` produce `"0000"`?
- [ ] Does `to_base32(13554, 4)` produce `"0d7i"` (lowercase)?
- [ ] Does carrier -32768 encode to `"0000"` and 32767 encode to `"1vvv"`?
- [ ] Does KeySchema.to_string() produce the exact FCP file store name format?
- [ ] Does the builder silently ignore fields not in the schema?
- [ ] Is KeySchema picklable?
- [ ] Does YEARMONTH value 1024 produce `"100"` (3 chars, not a ValueError)?
- [ ] Does your validate.py call the contract runner, NOT your own fixture tests?

## Output Format

Your repo structure must be:
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
