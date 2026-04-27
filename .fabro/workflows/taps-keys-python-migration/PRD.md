# PRD: taps-keys Python Migration Workflow

## Introduction

This workflow automates the full migration of Skyscanner's `taps-keys` Java library to Python. `taps-keys` defines 142 key schemas (71 one-way, 71 return) that encode flight route + date + directionality combinations into compact base-32 strings. These strings are lookup keys in the File Cache Proxy (FCP) — Skyscanner's distributed cache for aggregated pricing data. A single wrong character means a cache miss in production.

The workflow is a Fabro graph (`workflow.fabro`) that orchestrates three AI agent phases across three separate GitHub repositories, with script gates enforcing correctness at every boundary. The end result is three pull requests — one per repo — ready for human review.

**Validation chain (what each layer catches):**

| Layer | Where | What it catches |
|---|---|---|
| L1 | Phase 1 + 3 | Fixture JSON loads correctly, contract runner structure is sound |
| L2 | Phase 1 + 3 | Schema signatures and disjoint properties match Java golden values |
| L3 | Phase 1 + 3 | toString, pipe-toString, encodedLength, OpenJawFilter all match |
| L4 | Phase 1 (new) | Fixture JSON values were *correctly generated* — verifies against live Java library |
| L5 | Phase 3 (new) | Python and Java produce identical output on the same inputs, with no fixture file in the chain |

---

## Goals

- Produce a `taps-keys-python` library whose `encode()` output is byte-for-byte identical to the Java library for all 142 schemas × 5 input sets.
- Ensure the golden fixture JSON files cannot silently contain wrong expected values (L4 guard).
- Ensure the Python library matches the production Java binary directly, without relying solely on fixture files (L5 guard).
- Leave all three repos in a state where a human engineer can open the generated PRs and merge with confidence.
- Make the workflow idempotent: re-running after a partial failure should not fail due to existing branches or open PRs.

---

## User Stories

### US-001: Generate golden fixture files from the Java reference implementation

**Description:** As a workflow, I need to generate the ground-truth fixture dataset so that all downstream validation has a reliable reference point.

**Acceptance Criteria:**
- [ ] `setup` script clones `taps-keys-fixtures` and `taps-keys-schemas` from GitHub (or pulls if already present)
- [ ] `tools/taps-keys-fixture-gen/gradlew run` generates `golden_encodings.json` (710 entries: 142 schemas × 5 input sets A–E) and `golden_signatures.json` (142 entries)
- [ ] Count validation: `ENCS == SIGS * 5` — exits 1 on mismatch
- [ ] Files are copied to `/tmp/taps-keys-fixtures/` and removed from the Gradle working directory
- [ ] Workflow fails hard (`exit 1`) if fixture generation fails

---

### US-002: Build the taps-keys-fixtures Python package

**Description:** As a workflow agent, I need to build the `taps-keys-fixtures` package so that Python implementations can be validated against the golden fixture data.

**Acceptance Criteria:**
- [ ] Package lives in `taps-keys-fixtures/` and is structured as per `fixtures-build.md`
- [ ] `golden_encodings.json` and `golden_signatures.json` are in `taps_keys_fixtures/fixtures/` only — NOT at repo root
- [ ] Contract test runner (`taps_keys_fixtures.runner`) supports layers L1, L2, L3
- [ ] Error formatter never reveals more than 4 characters of any expected value
- [ ] `python3 -m pytest tests/ -v` passes (self-tests)
- [ ] `python3 -m build` succeeds
- [ ] `.gitignore` covers `dist/`, `*.egg-info/`, `__pycache__/`, `.pytest_cache/`

---

### US-003: Validate fixture JSON against the production Java library (L4)

**Description:** As a workflow, I need to verify that every entry in `golden_encodings.json` reflects what the real Java library actually produces — not just what the LLM-written fixture generator believed it produced.

**Acceptance Criteria:**
- [ ] `ValidateFixtures.java` exists at `tools/taps-keys-fixture-gen/src/main/java/net/skyscanner/tools/`
- [ ] For each of the 710 entries, `ValidateFixtures` rebuilds the key using the production `Keys.*` API with the stored input values (`origin`, `destination`, `carrier`, `outbound_date`, `inbound_date`, `is_direct`)
- [ ] Compares: `encoded_key`, `to_string`, `to_string_pipe`, `schema_to_string`, `encoded_length`, `open_jaw_filter`
- [ ] All failures printed before exit — never stops at first failure
- [ ] Exits 1 on any mismatch; exits 0 with `L4: 710/710 passed`
- [ ] Set E wildcard (`is_direct == "wildcard"`) handled via `KeyBuilder.anyDirect()`
- [ ] `validateFixtures` Gradle task wired in `build.gradle`
- [ ] `validate_fixtures_java` workflow node runs after `validate_fixtures` success
- [ ] On L4 failure, workflow routes to `fix_fixture_generator` (not `fix_fixtures`) which fixes `FixtureGenerator.java` and re-runs from `setup`

---

### US-004: Extract all 142 schemas into a language-neutral schemas.json

**Description:** As a workflow agent, I need to extract all schema definitions from `Keys.java` into `taps-keys-schemas` so that both the Java and Python libraries consume a single source of truth.

**Acceptance Criteria:**
- [ ] `taps_keys_schemas/schemas.json` contains exactly 71 oneway + 71 return schemas
- [ ] Each schema has: `name`, `prefix`, `components` (ordered), `to_string`, `encoded_length`, `open_jaw_filter`
- [ ] All 13 checks in `scripts/validate.py` pass (JSON Schema conformance, counts, types, no duplicates, toString derivation, encoded_length, OpenJawFilter, 5 cross-ref checks)
- [ ] `python3 -m build` succeeds
- [ ] `python3 scripts/validate.py --fixtures-dir /tmp/taps-keys-fixtures` exits 0

---

### US-005: Build the taps-keys Python library

**Description:** As a workflow agent, I need to build `taps-keys-python` so that it produces byte-for-byte identical base-32 encoded keys to the Java reference.

**Acceptance Criteria:**
- [ ] Encoding uses alphabet `0123456789abcdefghijklmnopqrstuv` — always lowercase
- [ ] All 8 component types implemented with correct bit widths and range validation
- [ ] YEARMONTH value 1024 produces `"100"` (3 chars — matches Java's `padStart` overflow behaviour)
- [ ] Carrier offset: raw ID + 32768 before encoding
- [ ] Schemas loaded at runtime from `taps_keys_schemas` via `importlib.resources` — never bundled
- [ ] `KeySchema` is picklable (PySpark UDF requirement)
- [ ] `KeyBuilder` silently ignores fields not in the schema; raises on missing required fields
- [ ] Contract runner (L1–L3) passes: `python3 -m taps_keys_fixtures.runner --module taps_keys`
- [ ] `python3 -m pytest tests/ -v` passes

---

### US-006: Validate Python library against Java binary directly (L5)

**Description:** As a workflow, I need to confirm Python matches Java on the same live inputs, removing any dependency on static fixture JSON files for the final parity check.

**Acceptance Criteria:**
- [ ] `EncodeMain.java` exists at `tools/taps-keys-fixture-gen/src/main/java/net/skyscanner/tools/`
- [ ] `EncodeMain` reads fixture inputs from `/tmp/taps-keys-fixtures/golden_encodings.json`, encodes each using the live Java library, writes results (schema, input_set, encoded_key, to_string, to_string_pipe) to `/tmp/java_outputs.json`
- [ ] Output contains **no** values copied from the fixture JSON — only live Java results
- [ ] `encodeMain` Gradle task wired in `build.gradle`
- [ ] L5 Python comparison script runs both libraries on the same inputs and diffs `encoded_key` and `to_string`
- [ ] Exits 1 on any mismatch; exits 0 with `L5: 710/710 Java/Python parity checks passed`
- [ ] `validate_java_parity` workflow node runs after `validate_python` success
- [ ] On L5 failure, workflow routes to `fix_python`

---

### US-007: Publish all three repos with idempotent branch and PR creation

**Description:** As a workflow, I need to push each repo to a feature branch and open a PR so that human engineers can review and merge.

**Acceptance Criteria:**
- [ ] Branch creation is idempotent: `git checkout -b $BRANCH 2>/dev/null || git checkout $BRANCH`
- [ ] Stray files cleaned before `git add .`: root-level `*.json`, `*.egg-info/`, `dist/` directories
- [ ] PR creation is idempotent: `gh pr list --head $BRANCH --state open | grep -q '"number"' || gh pr create ...`
- [ ] All three publish steps succeed on both first run and re-run without manual intervention
- [ ] Branch names: `fabro/golden-fixtures-and-contract-runner`, `fabro/schema-definitions-and-validation`, `fabro/python-taps-keys-encoding-library`
- [ ] Each PR body includes: summary, design decisions, validation results, usage examples

---

## Functional Requirements

- FR-1: `setup` generates fixtures via `tools/taps-keys-fixture-gen` and validates count before proceeding
- FR-2: `build_fixtures` agent builds the `taps-keys-fixtures` Python package including contract runner (L1–L3) and error formatter
- FR-3: `validate_fixtures` gate runs pytest self-tests and `python3 -m build` — routes to `fix_fixtures` on failure
- FR-4: `validate_fixtures_java` gate runs `ValidateFixtures.java` against `/tmp/taps-keys-fixtures/golden_encodings.json` — routes to `fix_fixture_generator` on failure
- FR-5: `fix_fixture_generator` agent fixes `FixtureGenerator.java` and is followed by a full restart from `setup`
- FR-6: `publish_fixtures` creates branch + PR in `taps-keys-fixtures`, installs locally via `pip install -e .`
- FR-7: `build_schemas` agent extracts all 142 schemas from `Keys.java` into `schemas.json` with all derived fields
- FR-8: `validate_schemas` gate runs all 13 validate.py checks — routes to `fix_schemas` on failure
- FR-9: `publish_schemas` creates branch + PR in `taps-keys-schemas`, installs locally
- FR-10: `setup_python` clones `taps-keys-python` and installs schemas only (no fixtures installed at this stage)
- FR-11: `build_python` agent builds the full Python encoding library from the algorithm spec
- FR-12: `validate_python` gate installs fixtures and runs contract runner + pytest — routes to `fix_python` on failure
- FR-13: `validate_java_parity` gate runs `EncodeMain.java` then Python comparison script — routes to `fix_python` on failure
- FR-14: `publish_python` creates branch + PR in `taps-keys-python`, runs e2e smoke test before declaring success

---

## Non-Goals

- No automated merging — PRs are opened for human review only
- No publishing to Artifactory or PyPI — local install only during the workflow run
- No modification of the production `taps-keys` Java library or `Keys.java`
- No support for partial schema sets — all 142 must pass before publishing
- No test coverage for the Java source (the Java library is treated as a black box oracle)
- No CI/CD pipeline configuration in the generated repos (that's a follow-up)

---

## Sandbox Architecture (Split Workflows)

The migration is split into 3 independent single-repo workflows to enable fabro's files-changed feature and sandbox support (both require one GitHub repo per workflow).

### Workflow structure

| Workflow | Repo | Nodes | JAR needed |
|---|---|---|---|
| `taps-keys-fixtures` | `Skyscanner/taps-keys-fixtures` | 10 nodes, 11 edges | Yes (FixtureGenerator, ValidateFixtures) |
| `taps-keys-schemas` | `Skyscanner/taps-keys-schemas` | 8 nodes, 8 edges | No |
| `taps-keys-python` | `Skyscanner/taps-keys-python` | 10 nodes, 12 edges | Yes (EncodeMain, FuzzEncoder) |

### Execution order

```
1. ./gradlew shadowJar  (pre-build Java tools — once)
2. fabro run taps-keys-fixtures  →  review PR  →  merge
3. fabro run taps-keys-schemas   →  review PR  →  merge
4. fabro run taps-keys-python    →  review PR  →  merge
```

### Cross-workflow data flow

No local paths or `/tmp/` state carries between workflows. Each sandbox starts clean.

- **Workflow 2** installs `taps-keys-fixtures` from the merged GitHub repo via `pip install git+ssh://...`
- **Workflow 3** installs both `taps-keys-fixtures` and `taps-keys-schemas` from merged GitHub repos
- Golden JSON files for L5/L6 are extracted from the pip-installed package via `importlib.resources` within each sandbox

### Java tools via pre-built shadow JAR

The Java tools (fixture generator, L4 validator, L5 encoder, L6 fuzz) are pre-built into a single fat JAR (`taps-keys-fixture-gen.jar`). The path is passed to workflows via `[run.sandbox.env]`:

```toml
[run.inputs]
jar_path = "/path/to/taps-keys-fixture-gen.jar"

[run.sandbox.env]
TAPS_KEYS_JAR = "{{ inputs.jar_path }}"
```

Scripts use `java -cp $TAPS_KEYS_JAR net.skyscanner.tools.<ClassName>`. Only a Java runtime is needed in the sandbox — no Gradle, no fabro repo.

### Human review gates

Each workflow ends with a `human_review` node (shape=box, human=true) that pauses execution. The reviewer:
1. Inspects the files-changed diff in fabro's UI
2. Reviews the PR on GitHub
3. Merges the PR
4. Resumes the workflow

This ensures downstream workflows install from merged code, not unreviewed branches.

---

## Technical Considerations

- **Fixture generator location**: `tools/taps-keys-fixture-gen/` relative to the fabro repo root (the workflow's working directory)
- **Fixture files in `/tmp/`**: Shared between phases via `/tmp/taps-keys-fixtures/` — scripts assume this path is stable within a single workflow run
- **Java library version**: `net.skyscanner.taps-keys:taps-keys:0.0.58` — pinned in `tools/taps-keys-fixture-gen/build.gradle`
- **Gson**: Already a dependency (`com.google.code.gson:gson:2.10.1`) — used by `ValidateFixtures.java` and `EncodeMain.java`
- **Python builder calls all setters**: The schema decides which values matter; passing all of `origin_airport`, `origin_city`, `origin_country`, `destination_airport`, etc. on every schema is intentional — extras are silently ignored
- **YEARMONTH overflow (value 1024)**: Produces 3-char `"100"` — this is a known Java edge case that Python must replicate exactly
- **`KeySchema` picklability**: Required for PySpark UDF serialisation — must not use lambda functions or non-picklable closures in `KeySchema`
- **Working directory**: All relative paths (`taps-keys-fixtures/`, `taps-keys-schemas/`, `taps-keys-python/`, `tools/`) are relative to the fabro repo root

---

## Workflow Graph Summary

```
start → setup → build_fixtures → validate_fixtures
                                      ↓ success
                               validate_fixtures_java  ← fix_fixture_generator ← (L4 fail)
                                      ↓ success
                               publish_fixtures → build_schemas → validate_schemas
                                                                       ↓ success
                                                               publish_schemas → setup_python
                                                                                    → build_python
                                                                                    → validate_python
                                                                                          ↓ success
                                                                                   validate_java_parity (L5)
                                                                                          ↓ success
                                                                                   publish_python → exit
```

Fix loops: `fix_fixtures → validate_fixtures`, `fix_schemas → validate_schemas`, `fix_python → validate_python`, `fix_fixture_generator → setup`

---

## Success Metrics

- `L4: 710/710 passed` — all fixture JSON values verified against live Java library
- `L5: 710/710 Java/Python parity checks passed` — Python and Java agree on every encoding, bypass fixture files
- All three PRs created and visible on GitHub for human review
- Workflow is re-runnable: second run with existing branches and open PRs completes without failures

---

## Open Questions

- ~~Should `validate_java_parity` also compare `to_string_pipe`?~~ **Resolved** — L5 now compares `encoded_key`, `to_string`, and `to_string_pipe`.
- Should `fix_fixture_generator` have a `max_visits` guard before the workflow hard-fails (currently 3)?
- After merge, should the workflow verify the published package installs correctly from GitHub (not just locally)?

---

## Remaining Gaps for Production Confidence

The current workflow validates encoding correctness against 5 hardcoded input sets per schema (710 total cases). This is strong but not exhaustive. The following items are needed before `taps-keys-python` can fully replace the Java library in production.

### Gap 1: Limited input coverage

**Problem:** Only 5 input sets (A–E) per schema are tested. Edge cases outside those inputs are unverified — e.g., carrier ID = 0, YEARMONTH at epoch boundaries, AIRPORT IDs near max range, date combinations that don't appear in the fixture sets.

**Recommendation:** Add a randomised fuzz layer (L6) that generates N random valid inputs per schema, encodes with both Java and Python, and compares. This can reuse `EncodeMain.java` by extending it to accept arbitrary JSON inputs. Run as part of the workflow or as a separate CI step.

### Gap 2: No consumer integration test

**Problem:** Nobody calls the Python library from the actual consumer code (e.g., the quote-aggregator FCP writer) to verify that the keys it produces hit the same cache entries as Java. The library is validated in isolation but not in its deployment context.

**Recommendation:** Add an integration test in the quote-aggregator repo (or a test harness) that:
1. Reads a sample of real production key requests (route + date + carrier combinations)
2. Encodes each with both the Java and Python libraries
3. Verifies byte-for-byte identical output
4. Verifies the keys resolve to the same FCP cache paths

### Gap 3: No production traffic shadow test

**Problem:** The strongest guarantee comes from running both libraries on live production traffic and diffing outputs. The fixture-based approach can miss inputs that only appear in production (unusual route node IDs, edge-case dates, new schemas added after fixture generation).

**Recommendation:** Before cutting over, deploy the Python library alongside Java in a shadow mode:
1. For every key encode request, run both Java and Python
2. Log any discrepancy with full input details
3. Run for at least 1 week of production traffic (covers weekday/weekend patterns, different route mixes)
4. Only cut over to Python-only after zero discrepancies across the shadow period

### Gap 4: PySpark serialisation under real Spark

**Problem:** `KeySchema` picklability is unit-tested with `pickle.dumps`/`pickle.loads`, but not verified in an actual PySpark cluster where UDF serialisation, worker distribution, and Python version differences could surface issues.

**Recommendation:** Add a Spark integration test that:
1. Creates a small DataFrame with sample route data
2. Applies a UDF that uses `KeySchema.key_builder()` to encode keys
3. Collects results and compares against expected values
4. Runs on the target Spark version and Python version used in production

### Gap 5: `WebsiteIdFilterKey` coverage

**Problem:** `website_id.py` has basic unit tests but no golden fixture validation against Java's `WebsiteIdFilterKey`. If the encoding differs, Spark partition keys would mismatch.

**Recommendation:** Add `WebsiteIdFilterKey` test cases to the fixture generator and extend L4/L5 to cover them.

### Priority Order

| Priority | Gap | Effort | Risk if skipped |
|---|---|---|---|
| 1 | Gap 3 — Shadow test | Medium (infra setup) | Highest — production cache misses |
| 2 | Gap 2 — Consumer integration | Low (test harness) | High — deployment context bugs |
| 3 | Gap 1 — Fuzz testing (L6) | Low (extend existing tools) | Medium — edge case misses |
| 4 | Gap 5 — WebsiteIdFilterKey | Low (extend fixtures) | Medium — Spark partition mismatch |
| 5 | Gap 4 — Real Spark test | Medium (cluster access) | Low — pickle issues are rare |
