# Design: Fabro Migration Harness

**Date:** 2026-04-29
**Author:** Harold Olivieri (with Claude)
**Status:** Draft — awaiting review

## 1. Summary

A reusable, installable skill bundle that helps engineers set up Fabro and create migration workflows for porting backend libraries and data pipelines between languages. The harness is Fabro-specific (not engine-agnostic) and targets algorithm/library ports with existing test suites — not web apps, UIs, or full service extractions.

**Fabro source pinned to `fabro-fork`** (not upstream Fabro). The fork includes Portkey gateway support that upstream lacks. Bootstrap skill installs the `fabro` binary from a fabro-fork release/build, not from upstream. This constraint is revisited when upstream gains Portkey support — see Section 15.

**Hybrid architecture** — Claude Code skills for interactive UX, a single Fabro workflow for multi-agent synthesis that does QA + scaffold together. This dogfoods the harness: the harness uses Fabro to author Fabro migration workflows.

Two skills + one Fabro workflow:

- `/migration:bootstrap` — installs Fabro from fork, configures credentials, starts server. Claude Code skill (can't self-bootstrap).
- `/migration:start <name>` — Claude Code skill. Interactive Q&A via AskUserQuestion, writes `docs/migrations/<name>-inputs.json`. Launches `migration-synthesis` Fabro workflow.
- `migration-synthesis` (Fabro workflow, ships with skill bundle) — multi-agent pipeline that:
  1. Analyzes source repo (with access to full scenarios)
  2. Drafts spec (restricted to seeds only — cannot see scenarios)
  3. Validates spec against scenarios (validator has full access)
  4. Loops through fixer on failures (max 3)
  5. Generates N migration workflows + tools + CI from validated spec
  6. Validates generated workflows (fabro validate + parity + anti-pattern scan)
  7. Loops through fixer-scaffold on failures
  8. Human review gate before commit
  9. Atomic commit

**Seeds vs scenarios isolation** (same pattern taps-keys uses for fixtures applied to design): drafter/fixer/generator agents can only see seeds (inputs.json, PATTERNS.md, reference examples, top-level source summary). Validator agents can see scenarios (full source repo, external docs, idiomatic target-lang reference, other migration specs). Prevents drafter from gaming its way to a "valid" spec by pattern-matching without real grounding.

**Output: N migration workflows per spec.** Default 2 (oracle + target). User declares any number of additional custom workflows with `role: custom` + `purpose` + `responsibilities` + optional `validation_layers` and `depends_on`. Agent uses the user's declared purpose/responsibilities as prompt input to synthesize each custom workflow's setup/build/validate/publish stages — applies generic workflow patterns with user-provided specifics. No predefined "contract" role; taps-keys-schemas is just one example of a `custom` repo.

The harness ships with PATTERNS.md documenting the principles, recipes, and gotchas extracted from the taps-keys migration. Scaffold-agent cites PATTERNS.md when generating files.

## 2. Goals and non-goals

### Goals

- Help an engineer with Fabro access go from zero to a running migration workflow in under 30 minutes
- Encode the operational lessons from taps-keys (oracle pattern, validation stack, anti-cheating mechanisms, git workflow gotchas, credential model) so new migrations avoid the discovered pitfalls
- Output repos come ready with Claude Code harness (CLAUDE.md, skills, agents) tailored to the migration's specific needs
- Validation parity between the Fabro workflow and the output repo's CI via a shared Makefile entry point
- Idempotent skills and workflows — safe to re-run after partial failures

### Non-goals

- Web app / UI migrations (separate harness, possibly future work)
- Full service extractions from monoliths (separate harness)
- Porting the harness to other workflow engines (Fabro-only)
- Auto-merging PRs (always human-gated)

## 3. Architecture

### 3.1 Skill bundle layout

```
~/.claude/skills/migration-harness/
├── bootstrap/SKILL.md
├── design/SKILL.md
├── scaffold/SKILL.md
└── reference/
    ├── PATTERNS.md                       ← principles + recipes + case study
    ├── synthesis-workflows/              ← Fabro workflows shipped with skill
    │   ├── design-synthesis/
    │   │   ├── workflow.fabro
    │   │   ├── workflow.toml
    │   │   └── prompts/
    │   │       ├── analyzer.md
    │   │       ├── drafter.md
    │   │       ├── reviewer.md
    │   │       ├── fixer.md
    │   │       └── approver.md
    │   └── scaffold-synthesis/
    │       ├── workflow.fabro
    │       ├── workflow.toml
    │       └── prompts/
    │           ├── generator.md
    │           └── fixer.md
    ├── examples/
    │   ├── 2repo-oracle.fabro            ← abstract reference workflow
    │   ├── 2repo-target.fabro
    │   ├── 3repo-oracle.fabro
    │   ├── 3repo-contract.fabro
    │   ├── 3repo-target.fabro
    │   └── prompts/
    │       ├── oracle-build.md
    │       ├── contract-build.md
    │       └── target-build.md
    ├── tools/
    │   ├── exporters/
    │   │   ├── java/SchemaExporter.java
    │   │   ├── python/exporter.py
    │   │   ├── go/exporter.go
    │   │   └── typescript/exporter.ts
    │   └── runners/
    │       ├── pytest-contract-runner.py
    │       └── cargo-contract-runner.rs
    └── validation-layers.md              ← L1-L6 recipes

Skill `scaffold` copies `reference/synthesis-workflows/*` into the user's
`.fabro/workflows/` on first use (or points at them directly via absolute path).
```

Reference examples are **abstract** — no Skyscanner names, no taps-keys specifics. They encode the shape of each workflow type.

### 3.2 Data flow

```
┌─────────────────────────────────────────────────────────────┐
│ USER (first time on this machine)                           │
│                                                              │
│   /migration:bootstrap                                       │
│       ↓                                                      │
│   installs Fabro, writes server.env, sets vault secrets,    │
│   starts server, runs healthcheck + dummy-workflow smoke    │
│       ↓                                                      │
│   prints: FABRO_SERVER=... (ready)                          │
└─────────────────────────────────────────────────────────────┘

┌─────────────────────────────────────────────────────────────┐
│ USER (per migration)                                         │
│                                                              │
│   /migration:design                                          │
│       ↓ (calls bootstrap healthcheck first)                 │
│       ↓ (spawns repo-analyzer subagent on source repo)      │
│       ↓ (asks clarifying questions one at a time)           │
│       ↓                                                      │
│   writes docs/migrations/<name>-spec.md                      │
│       ↓                                                      │
│   USER reviews spec                                          │
│       ↓                                                      │
│   /migration:scaffold <name>                                 │
│       ↓ (calls bootstrap healthcheck first)                 │
│       ↓ (scaffold-agent reads spec + PATTERNS.md + refs)    │
│       ↓ (writes workflow.fabro, prompts, tools, CI, harness)│
│       ↓ (runs fabro validate <name>-*)                      │
│       ↓                                                      │
│   prints: next-steps checklist (build JAR, export env var,  │
│           run fabro run <name>-oracle, review PR, …)        │
└─────────────────────────────────────────────────────────────┘
```

### 3.3 Location of generated files (in user's working directory)

```
<user's working directory>/
├── .fabro/workflows/<name>-oracle/
│   ├── workflow.fabro
│   ├── workflow.toml
│   └── prompts/oracle-build.md
├── .fabro/workflows/<name>-contract/       (only if spec.backwards_compat_required)
├── .fabro/workflows/<name>-target/
├── tools/<name>-exporter/                  (source-lang source for oracle binary)
├── docs/migrations/<name>-spec.md
└── (after workflow runs)
    └── output repos on GitHub with their own .claude/ harness
```

## 4. Component contracts

### 4.1 `/migration:bootstrap`

**Purpose:** One-time Fabro setup per machine.

**Interactive inputs (AskUserQuestion, one at a time):**
1. LLM provider: Anthropic direct / Portkey / Bedrock-via-Portkey
2. API keys (prompted per provider)
3. Portkey gateway URL (if Portkey) — auto-validates `/v1` suffix
4. Server port (default 32276)
5. Overwrite existing vault secrets? (if present)

**Steps:**
1. Check `fabro --version` — confirm it's the fabro-fork build (detect via version string / `--help` text). If missing or wrong fork, install from fabro-fork:
   - Prompt user for path to fabro-fork checkout (default: `~/Development/Skyscanner/backend/fabro-fork`)
   - Run `cargo build --release --bin fabro` in that checkout
   - Symlink the binary into `~/.local/bin/fabro` (or prompt user for preferred install path)
   - Verify `fabro --version` now reports the fork build
2. Write `~/.fabro/storage/server.env` with chosen credentials (`chmod 600`)
3. `fabro secret set ANTHROPIC_API_KEY <value>` (and related PORTKEY_* / AWS_* as applicable) — worker env allowlist strips these, so vault is mandatory
4. Start server: `fabro server start &` — store PID in state file
5. Hit healthcheck; retry 3× with backoff
6. Run dummy workflow (ships with the skill) to confirm end-to-end
7. Write `~/.fabro-env.sh` with `export FABRO_SERVER=http://127.0.0.1:<port>`

**Idempotent:** safe to re-run. Detects existing state, fills gaps. Does not overwrite secrets unless explicitly confirmed.

**Error paths:** server won't start → print last 20 lines of `~/.fabro/logs/server.log`. Vault write fails → print exact `fabro secret set` command. Healthcheck fails → curl the endpoint, show full response.

**Output (success):**
```
✓ Fabro installed (v<X>)
✓ Server running at http://127.0.0.1:32276 (PID: <pid>)
✓ server.env: <providers>
✓ Vault secrets: <set keys>
✓ End-to-end smoke test: passed
Run:  export FABRO_SERVER=http://127.0.0.1:32276
Next: /migration:design
```

### 4.2 `/migration:design`

**Purpose:** Write a reviewable migration spec via guided Q&A + source-repo analysis.

**Preflight:** calls bootstrap healthcheck; aborts with instructions if Fabro not ready.

**Interactive inputs (one at a time):**
1. Migration short name (kebab-case, becomes `<name>-oracle`, `<name>-target`, etc.)
2. Source repo path (local checkout preferred) or URL + commit ref
3. Source language (auto-detected from repo; user confirms)
4. Target language
5. Oracle type: `compiled-binary-reflection` / `live-service` / `test-suite-replay` / `spec-from-docs`
6. Backwards-compat required? (yes → 3-repo; no → 2-repo)
7. Third-party reference docs (URLs/paths for internal/external libs; optional)
8. Reference target-lang repo (optional — idiomatic patterns)
9. Publish targets (GitHub org/repo per output)
10. Validation layers to include (checkboxes: L1 L2 L3 L4 L5 L6; defaults by migration type)
11. Per-layer comparison mode (exact / epsilon / structural / subset)
12. Fixture storage (inline / lfs / s3 / http)
13. Test environment services needed (docker-compose spec)
14. Known source bugs to replicate in target (free text — the "don't fix bugs" rule)
15. Input set matrix (names + purposes — agent proposes based on spec domain, user accepts/edits)
16. Retry budget: setup/build/fix per-node max attempts

**Side tasks:**
- Spawn repo-analyzer subagent: scans source repo for build system, test framework, public API surface, existing fixtures. Findings reconciled with user answers; discrepancies surfaced.
- Agent proposes sensible defaults for validation layers based on migration type.

**Output:** single file `docs/migrations/<name>-spec.md` — YAML front-matter + markdown body.

### 4.3 `/migration:scaffold <name>`

**Purpose:** Read spec, write all migration files.

**Preflight:** bootstrap healthcheck + spec file exists.

**Steps (scaffold-agent):**
1. Reads: spec, PATTERNS.md, `reference/examples/`, `reference/tools/exporters/<source-lang>/`
2. Writes `.fabro/workflows/<name>-oracle/workflow.fabro` + `workflow.toml` + `prompts/oracle-build.md`
3. Writes `.fabro/workflows/<name>-contract/...` if 3-repo
4. Writes `.fabro/workflows/<name>-target/...`
5. Writes `tools/<name>-exporter/` with source-language-specific skeleton (adapts reference exporter to the spec's entry points). If source-lang not in `reference/tools/exporters/`, spawns `tool-generator` subagent to write from scratch.
6. Writes `.github/workflows/ci.yml` skeletons for each output repo (matching Section 9 validation parity)
7. Proposes output-repo Claude Code skills/agents (dynamic, based on spec domain — see Section 6), uses AskUserQuestion to confirm, writes chosen skills into `.claude/` dirs of each output repo
8. Runs `fabro validate <name>-oracle` (and `-contract`, `-target`); on fail, prints error + path and exits with user-actionable message
9. Prints pre-first-run checklist (Section 10)

**Guardrails in scaffold-agent prompt:**
- Must name all workflows/repos per spec (no invention)
- Must include all validation layers the spec requested, no extras or omissions
- Must embed validation parity Makefile targets (Section 9)
- Must embed git workflow gotchas (Section 8): `core.hooksPath /tmp`, `git fetch origin` before checkout -B, HTTPS for pip installs, etc.
- Must remove `.git` in setup (Fabro checkpoint requirement)
- Must run `fabro validate` before exiting

### 4.4 `repo-analyzer` agent

**Input:** source repo path.

**Reads (lightweight scan, no deep traversal):**
- Build files: `pom.xml`, `build.gradle`, `pyproject.toml`, `go.mod`, `Cargo.toml`, `package.json`
- Top-level README
- Test directories (`tests/`, `src/test/`, `__tests__/`)
- 1-2 sample test files

**Output (structured JSON, consumed by design skill):**
```json
{
  "build_system": "gradle",
  "languages": ["java"],
  "test_framework": "junit5",
  "public_api_surface": ["net.example.Foo", ...],
  "entry_points_for_oracle": ["Foo.encode(...)", ...],
  "has_existing_fixtures": false,
  "suggests": {
    "oracle_type": "compiled-binary-reflection",
    "exporter_strategy": "java-reflection-on-static-fields"
  }
}
```

### 4.5 `scaffold-agent`

**Input:** spec + PATTERNS.md + `reference/` bundle + user's current working directory.

**Reads:** everything above, plus existing `.fabro/workflows/` in the user's repo (to avoid name conflicts).

**Produces:** all workflow files. Does text-level adaptation of reference examples to spec. Never invents validation layers not in spec. Always invokes `fabro validate` before exiting.

### 4.6 `design-synthesis` Fabro workflow (Phase B of design)

**Input:** `docs/migrations/<name>-inputs.json` written by `/migration:design` Phase A.

**Multi-agent graph** (nodes with per-node model pins):

```
start
  └─ setup (script, max_retries=0, goal_gate=true):
     - validate inputs JSON schema
     - clone/checkout source repo at specified ref
     - stage PATTERNS.md + reference/ as agent-readable
  └─ analyzer (agent, model=sonnet):
     - reads source repo, reconciles with user answers
     - proposes validation layer defaults, input set matrix, oracle strategy
     - writes /tmp/analysis.json
  └─ drafter (agent, model=sonnet):
     - synthesizes first-pass spec YAML + markdown body from inputs + analysis
     - writes /tmp/draft-spec.md
  └─ validate-spec (script, goal_gate=true, retry_target="fixer"):
     - schema check (YAML front-matter parses, all required fields)
     - PATTERNS.md anti-pattern scan (no LLM-generated fixtures, no hardcoded counts)
     - consistency check (backwards_compat_required implies 3 output_repos, etc.)
     - completeness check (source_quirks_to_replicate non-empty if analyzer flagged any)
     - exit 0 + specific fail messages
  └─ fixer (agent, model=sonnet, max_visits=3):
     - reads /tmp/draft-spec.md + validate-spec error output
     - addresses each specific failure
     - writes new /tmp/draft-spec.md
     - loop back to validate-spec
  └─ approver (agent, model=opus, max_visits=1):
     - final cross-check: spec internal consistency, coverage gaps vs PATTERNS.md
     - approves or sends back to fixer
  └─ write-spec (script, goal_gate=true):
     - copies /tmp/draft-spec.md to docs/migrations/<name>-spec.md
     - prints path for user review
  └─ exit
```

**Models per node** (cost-tuned):
- analyzer, drafter, fixer → `sonnet` (needs reasoning over code/spec)
- validate-spec → script only (no agent cost)
- approver → `opus` (final quality gate; runs once)

**Retry budget:** fixer max_visits=3 — if still failing, run halts at human review gate with error summary.

### 4.7 `scaffold-synthesis` Fabro workflow (Phase B of scaffold)

**Input:** `docs/migrations/<name>-spec.md` + `reference/` bundle.

```
start
  └─ setup (script, max_retries=0, goal_gate=true):
     - validate spec file present + parses
     - stage reference/ examples as agent-readable
  └─ generator (agent, model=sonnet):
     - reads spec + PATTERNS.md + reference/examples/<2|3>-repo.fabro
     - writes candidate .fabro/workflows/<name>-*, tools/<name>-exporter/, CI YAML
  └─ validate-scaffold (script, goal_gate=true, retry_target="fixer"):
     - fabro validate <name>-oracle (and -contract, -target)
     - Makefile target match: spec.validation.layers ⇔ Makefile .PHONY list
     - CI YAML match: same Makefile targets invoked
     - Output-repo CLAUDE.md forbids output-repo-edits
     - Anti-pattern scan: no ${{ }} in workflow script attrs, no SSH pip installs, etc.
  └─ fixer (agent, model=sonnet, max_visits=3):
     - reads validate-scaffold error output + generated files
     - targeted fixes
  └─ commit-files (script, goal_gate=true):
     - moves staged files into place atomically
     - prints pre-first-run checklist (Section 10)
  └─ exit
```

**Tool-generator escape hatch** (Section 4.8) is invoked by `generator` when source lang not in `reference/tools/exporters/`.

### 4.8 `tool-generator` agent (escape hatch)

**Invoked when:** spec source language isn't in `reference/tools/exporters/<lang>/`.

**Input:** spec, source repo analysis.

**Produces:** idiomatic exporter in source language — reflection-based if language supports it (Java, Go, Python, .NET), otherwise AST-based or manual enumeration with a clear pattern (C++, COBOL, etc.).

## 5. Spec format

Single file per migration: `docs/migrations/<name>-spec.md`. Machine-parseable YAML front-matter + human-readable markdown body.

```markdown
---
name: <migration-name>
date: YYYY-MM-DD

source:
  repo: <local-path-or-url>
  ref: <commit-sha-or-branch>
  language: java
  build_system: gradle
  oracle:
    type: compiled-binary-reflection
    binary_target: tools/<name>-exporter/build/libs/<name>.jar
    env_var: SOURCE_JAR

target:
  language: python
  idiomatic_reference_repo: <url>    # optional

backwards_compat_required: true       # 3-repo if true; 2-repo if false

validation:
  layers:
    - id: L1
      comparison_mode: exact
    - id: L5
      comparison_mode: exact
    - id: L6
      comparison_mode: exact
  input_sets:
    - name: A
      purpose: baseline
    - name: Q
      purpose: mixed-overflow-boundary   # see PATTERNS.md §11
  fuzz_count: 100
  fuzz_seed: 42

fixtures:
  storage: inline                      # inline | lfs | s3 | http
  location: null
  max_inline_size_mb: 10

test_env:
  requires_external: false
  services: []                        # docker-compose services (postgres, redis, kafka, ...)
  mocks: []                           # in-process/localstack mocks: s3, dynamodb, sqs, kinesis

credentials:
  vault: [ANTHROPIC_API_KEY]          # must be in Fabro vault
  env: [PORTKEY_URL, PORTKEY_API_KEY] # server.env only

retry_budget:
  setup_max_retries: 0
  build_max_visits: 4
  fix_max_visits: 3

output_repos:
  # Default: 2 repos (oracle + target). User declares any additional custom repos.
  - name: <name>-oracle         # role=oracle: generate fixtures + contract runner, publish package
    role: oracle
    github: <org>/<name>-oracle
  - name: <name>-target         # role=target: final port in target language
    role: target
    github: <org>/<name>-target
  # Optional user-declared custom repos (any number):
  # - name: <name>-schemas
  #   role: custom
  #   purpose: "one-sentence why this repo exists"
  #   responsibilities:
  #     - "bullet 1"
  #     - "bullet 2"
  #   validation_layers: [L-custom-struct]  # user-named layers
  #   depends_on: [<name>-oracle]            # ordering; synthesis enforces topological order

references:
  - <url1>
  - <url2>

source_quirks_to_replicate:
  - "Field X silently overflows width when value exceeds 1024"

tool_generator_needed: false          # true if source lang unfamiliar
---

# <Migration Name>

## Summary
<one-paragraph what/why>

## Architecture (2-repo / 3-repo)
<diagram + rationale>

## Validation layer plan (L1–LN rationale per layer)

## Oracle strategy
<how the source binary/service/suite acts as oracle>

## Acceptance criteria
- [ ] L1–LN all pass
- [ ] PR opened in each output repo
- [ ] Target library installable via pip/npm/cargo/...
```

## 6. Output repo harness (dynamic per-migration proposal)

### 6.1 Core harness (always generated)

- `CLAUDE.md` — constitution: what this repo is, what it depends on, validation commands (`make validate-all`), prohibited actions
- `.claude/settings.json` — permission allowlist for routine commands
- `README.md` — **derived from code** (fixture counts, layer descriptions), not hardcoded templates
- `CONTRIBUTING.md` — dev setup, validation workflow
- `.gitignore` — language-specific + `.DS_Store`
- `Makefile` — validation entry points + build + clean targets

### 6.2 Optional harness (dynamic proposals)

Scaffold-agent reasons over spec + analyzer findings and proposes **~3-6 skills/agents** tailored to this specific migration. Presents via AskUserQuestion with multi-select. User can add free-text custom suggestions. Only chosen items get written.

**Examples** (not a fixed menu — scaffold-agent generates fresh per migration):
- `add-<entity>` skill for repos with structural entities added over time
- `regenerate-fixtures` skill for oracle-dependent fixture repos
- `upgrade-oracle-version` skill when source is a versioned dep
- `diagnose-parity-failure` agent for byte-level validation domains
- `sync-docs` skill when 3rd-party docs are referenced

## 7. Migration type matrix

| Class | Layer profile | Fixture storage | Test env | Example |
|---|---|---|---|---|
| Algorithm port (byte-exact) | L1+L5+L6 exact | inline | none | taps-keys |
| Numeric/scientific | L1+L5+L6 epsilon | inline | none | stats lib |
| Parser / encoder | L1+L5 exact | inline | none | protobuf→JSON |
| Data pipeline | L1+L5 structural | s3/lfs | spark/k8s | ETL port |
| DB-backed library | L1 exact | inline | postgres | ORM port |
| Cache/KV client | L1 structural | inline | redis | memcache→redis |
| SDK port | L1 structural | inline | mock server | JS→TS SDK |

**Out-of-scope** (explicitly documented):
- Web apps / UIs
- Service extractions from monoliths
- Full-app migrations with complex side effects

## 8. Operational gotchas (embedded in scaffold + PATTERNS.md)

### 8.1 Git workflow

- `git config core.hooksPath /tmp` immediately after every `git init` / `git clone` — disables pre-commit hooks (betterleaks flags base-32 strings as false-positive API keys)
- `git fetch origin` (no arguments) before `git checkout -B BRANCH origin/main` — `git fetch origin main --depth=1` sets FETCH_HEAD but NOT a tracking ref; checkout falls back to orphan
- `git push --force -u origin $BRANCH` (not `--force-with-lease`) — sandboxes start without remote tracking refs; lease pattern has stale info
- `git init` + `git clone --depth=1 <remote>` + `cp -r /tmp/seed/.git .git` pattern to inject history into fresh repo while keeping agent-generated working tree
- Cleanup node removes `.git` from target subdirectory after human review (next run starts fresh; also required for Fabro checkpoint to track agent changes)
- HTTPS for pip installs from GitHub (`git+https://...`), never SSH — sandboxes often lack SSH keys

### 8.2 Retry semantics

- `setup` nodes: `max_retries=0` (must succeed once; failure is terminal — fail-closed)
- `build` agents: `max_visits=4-6` (design limit, not infinite)
- `fix` agents: `max_visits=3-5` (exhaustion escalates to human)
- Validate → fix edge: `retry_target="fix_*"` on validate node (conditional on outcome=success for forward path; unconditional fallback to fix for failure)
- Error output from prior attempts must be fed back into fix agent's context — scaffold embeds `@prompts/fix-feedback.md` pattern

### 8.3 Cloud service mocks

For libraries that talk to S3, DynamoDB, SQS, Kinesis, etc., `test_env.mocks` drives scaffold to write:

- **Python target:** `moto` as a test dependency in `pyproject.toml`, fixtures use `@mock_aws` decorators, test doubles serve sample data
- **CI YAML:** localstack service (`image: localstack/localstack`) for cross-language parity tests
- **Fabro workflow:** setup node starts localstack via docker before validation nodes run, stops after

One spec field (`test_env.mocks: [s3, dynamodb]`) drives both the Python-level mock and the CI-level localstack — validation parity (Section 9) extends to mocked services.

### 8.4 Credentials

- **Vault is mandatory for workers** — worker env allowlist strips everything except PATH/HOME/TMPDIR/etc. `ANTHROPIC_API_KEY`, `PORTKEY_*`, `AWS_*` must be in Fabro vault (`fabro secret set`) in addition to server.env
- `PORTKEY_URL` must include `/v1` suffix — Anthropic adapter appends `/messages`; without `/v1`, 404 every request
- Credentials must be scrubbed from error output before logging to run history (public repo risk)

### 8.5 Multi-repo orchestration

- Output repos publish sequentially with human merge gate between — `workflow A` → merge → `workflow B` (installs A from merged GitHub) → merge → `workflow C`
- `/tmp` state is per-workflow — between workflows, re-extract fixtures from pip-installed package via `importlib.resources`
- `pip install git+https://github.com/<org>/<repo>.git` only works if the repo is public OR CI has org-level PAT

### 8.6 Human gates

- `human=true` nodes with `prompt=` cause agent to auto-run on resume — the prompt MUST include "PROHIBITED: Do not merge the PR. Your responsibility." Otherwise agent auto-merges
- Terminal `publish` nodes use `goal_gate=true` + success-conditional success edge + unconditional cleanup edge

### 8.7 Fixture handling

- Fixture JSON is **oracle data** — never edit by hand
- L4 validation catches fixtures that don't match the source binary (fixture generator correctness guard)
- Input set design matters: 5 sets insufficient; 15+ catches edge cases (taps-keys Set Q pattern)
- Use real-world production data for hierarchical ID fixtures (not synthetic 1/2/3)

### 8.8 Fabro-specific

- `fabro validate <name>` after every `workflow.fabro` edit
- Two-pass minijinja rendering: `{{ }}` in script attributes fails on second pass if variable undefined; `{% raw %}` protects first pass only
- Never put `${{ secrets.* }}` in workflow.fabro scripts — minijinja parses it; rely on runner env instead
- DOT script attribute quoting: single quotes inside the `script="..."` value; unescaped double quotes break parse
- `model_stylesheet="* { model: eu.anthropic.claude-sonnet-4-6; }"` and `stall_timeout="3600s"` in graph attributes of every workflow

### 8.9 Anti-patterns (PATTERNS.md documents, scaffold avoids)

- LLM-generated fixtures (oracle must come from source system)
- Hardcoded counts / constants (use dynamic count from exporter output)
- Source parsing instead of binary reflection (version skew risk)
- Cross-repo file access in build agents (isolation violation)
- Committing binaries bundling internal code to public repos
- Jinja2 template layer on top of Fabro's minijinja (redundant)

### 8.10 Replicate-source-bugs-exactly

When the migration target must match source behavior byte-for-byte, source bugs must be preserved. Spec captures them in `source_quirks_to_replicate`. Scaffold embeds them in target-build prompt with "do NOT fix this; the target must reproduce".

Examples: silent integer overflow, off-by-one edge cases, encoding quirks at boundary values.

## 9. Validation parity (workflow ↔ CI)

**Principle:** Same `spec.validation.layers`, same checks, same commands, in both the Fabro workflow and the output repo's CI.

**Implementation:** scaffold writes a single Makefile in every output repo with one target per layer. Both workflow and CI invoke those targets.

```makefile
.PHONY: validate-l1 validate-l2 validate-l3 validate-l4 validate-l5 validate-l6 validate-all

validate-l1:
	python3 -m <pkg>.runner --layer l1 --fixtures-dir $(FIXTURES_DIR)

validate-l4:
	java -cp $(SOURCE_JAR) net.example.ValidateFixtures $(FIXTURES_DIR)/golden.json

validate-l5:
	java -cp $(SOURCE_JAR) net.example.EncodeMain $(FIXTURES_DIR)/golden.json /tmp/source_out.json
	python3 scripts/compare_parity.py --source /tmp/source_out.json --target <pkg>

validate-l6:
	java -cp $(SOURCE_JAR) net.example.FuzzEncoder --seed $(SEED) --count $(COUNT) --output /tmp/fuzz.json
	python3 scripts/compare_fuzz.py --fuzz /tmp/fuzz.json --target <pkg>

validate-all: validate-l1 validate-l2 validate-l3 validate-l4 validate-l5 validate-l6
```

Fabro workflow calls `make validate-all`; CI YAML calls the same. Scaffold-agent generates both from spec's layer list — guarantees parity. On regeneration, if Makefile and CI drift from spec, refuse and print diff.

## 10. Pre-first-run checklist

Scaffold emits after writing files:

```
Next steps:

1. Build the source oracle exporter:
   cd tools/<name>-exporter && <build-cmd>
2. Set env var:
   export SOURCE_JAR=$(pwd)/tools/<name>-exporter/build/libs/<name>.jar
3. Verify Fabro ready (bootstrap should have done this):
   curl -s http://127.0.0.1:32276/healthz
4. Run oracle workflow:
   fabro run <name>-oracle
5. Review PR → merge → re-run FABRO_SERVER workflow 2 (if 3-repo)
6. Run target workflow:
   fabro run <name>-target
7. Review PR → merge → validate published package installable:
   pip install git+https://github.com/<org>/<name>-target.git
```

Operator prerequisites (scaffold checks before writing):
- [ ] `fabro validate <name>` passes for each generated workflow
- [ ] Credentials present in BOTH vault AND server.env
- [ ] Source oracle repo accessible (local checkout or permissioned remote)
- [ ] Upstream output repos created on GitHub (empty is fine)
- [ ] Fabro server running + healthcheck responding

## 11. PATTERNS.md structure

Ships in `reference/PATTERNS.md`. Scaffold-agent cites specific sections when generating files. Structure:

1. Oracle principles (source = oracle, never LLM; binary reflection preferred)
2. Repo architecture (2-repo vs 3-repo; when to use each)
3. Validation stack (L1-L6 per-layer semantics + example check code)
4. Workflow graph patterns (setup/validate/fix/publish structure)
5. Prompt patterns (SCOPE statement, .gitignore FIRST, always-overwrite, self-check)
6. Tool patterns (binary reflection exporters per language, fixture generators)
7. Fixture storage (inline vs LFS vs S3; max_inline_size threshold)
8. Comparison strategies (exact/epsilon/structural/subset with example impls)
9. Test environment provisioning (docker-compose for external services)
10. Output repo harness patterns (dynamic proposals)
11. Input set design matrix (why 15+ sets; Set Q pattern; real-world IDs for Set L-equivalent)
12. Credential model (vault vs server.env; PORTKEY_URL /v1 requirement)
13. Replicate-source-bugs-exactly principle (examples; spec.source_quirks_to_replicate field)
14. Anti-patterns (LLM fixtures, hardcoded counts, cross-repo access, etc.)
15. Validation parity (shared Makefile entry point)
16. Case study: taps-keys (the reference walkthrough)
17. Gotchas checklist (30 items from the Apr 2026 session)

## 12. Testing and error handling

### Bootstrap tests
- `fabro --version` before/after install
- Server healthcheck responds 200
- End-to-end dummy workflow completes

### Design tests
- Spec file schema validates (YAML front-matter parses)
- repo-analyzer output matches expected JSON shape
- AskUserQuestion inputs round-trip into spec

### Scaffold tests
- `fabro validate <name>` passes for generated workflows (Test: run against taps-keys spec, compare output to existing files — should match with only cosmetic diffs)
- Makefile targets match spec.validation.layers exactly
- CI YAML invokes same targets as workflow.fabro
- Output repo CLAUDE.md forbids output-repo-edits

### Error handling
- Bootstrap failure: clear log output + actionable remediation per failure mode
- Design interrupted mid-flow: partial spec saved, resumable
- Scaffold validation failure: no files written, diff printed, exit non-zero

## 13. Rollout plan

Phase 1 — MVP (this design + implementation plan):
- Bootstrap skill (fabro-fork build + Anthropic-direct path only first)
- Design skill Phase A (Claude skill interactive Q&A → inputs.json)
- Scaffold skill Phase A (Claude skill reads spec → confirms harness proposals)
- `design-synthesis` Fabro workflow with analyzer + drafter + validate-spec + fixer (no approver yet; approver comes Phase 2)
- `scaffold-synthesis` Fabro workflow with generator + validate-scaffold + fixer
- PATTERNS.md v1 (Sections 1-6, 11-14 from Section 11 structure above)
- Smoke-test: design + scaffold regenerate taps-keys-python migration end-to-end via workflows; diff against hand-written version; all validators pass

Phase 2 — parity with taps-keys:
- Add approver node to design-synthesis (opus model, final quality gate)
- 3-repo scaffolding
- Portkey/Bedrock bootstrap paths
- tool-generator escape hatch for non-Java sources
- PATTERNS.md Sections 7-10, 15-17

Phase 3 — broader coverage:
- Additional exporter references (Go, TypeScript, Rust)
- docker-compose scaffolding for test_env.services
- Incremental migration support (`migration_style: incremental`)
- Output repo harness: dynamic proposal engine

## 14. Known limitations (Phase 2+)

The MVP harness targets taps-keys-like algorithm ports + libraries with S3/DynamoDB/SQS/Kinesis mock needs. The following are explicitly deferred to Phase 2 work:

- **Spark Dataset / PySpark interop validation** — no dedicated layer to verify target-lang output deserializes cleanly in an external framework. Workaround: manual smoke test in target-build prompt.
- **Stateful API semantics** — libraries with mutable state (e.g. `reload()` preserving on failure) need sequence-based input sets, not just single-call fixtures. Phase 2 adds `spec.stateful_apis` + sequence-test-runner scaffolding.
- **Exception-type parity** — target must raise mapped exceptions for same bad inputs. Phase 2 adds `spec.exceptions_mapping` + exception-parity validation check.
- **Non-JSON fixture formats** — current prompts assume JSON golden data. CSV/parquet/yaml fixtures require prompt and validator adaptations. Phase 2 adds `spec.fixtures.format: json | csv | parquet | yaml`.
- **Incremental migrations** — module-by-module porting (not one-shot). Phase 2 adds `spec.migration_style: one_shot | incremental`.
- **Performance parity layer (L7)** — ratio-based regression vs source. Phase 2 optional layer.
- **Side-effect validation** — DB writes, message publish comparison. Phase 2 with event-capture pattern.

### Coverage examples (based on target migrations)

| Repo | MVP sufficient? | What's missing |
|---|---|---|
| taps-keys-python | ✓ Full | — (reference implementation) |
| market-partners → Python | Mostly | Need `test_env.mocks: [s3]` (MVP includes this). Stateful `reload()` semantics deferred to Phase 2; workaround: model each reload as a fresh fixture set. |
| Any numeric-algorithm port (scientific lib) | ✓ Full | — (use `comparison_mode: epsilon`) |
| Any DB-backed library | ✓ Full | — (use `test_env.services: [{name: postgres, ...}]`) |
| Any pipeline with Spark/Beam consumer | Partial | Spark interop validation deferred to Phase 2 |

## 15. Open questions

1. **When does the harness switch from fabro-fork to upstream Fabro?** Pinned to fork for now (Portkey gateway support). Switch triggered by: upstream PR merging Portkey support → harness bootstrap detects via `fabro --version` capability probe and offers upgrade path. Until then, bootstrap requires a fabro-fork checkout path.
2. Should bootstrap auto-build Fabro from the fork if missing, or just print install instructions? (Recommend: auto-build via `cargo build --release --bin fabro` in the user-provided fork checkout; print manual steps only if cargo not found)
3. Where does the skill bundle actually live — `~/.claude/skills/` user-level, or published as a superpowers plugin? (Recommend: start at user-level, evolve to plugin)
4. Should `/migration:design` let users resume a partial spec from disk, or start fresh each time? (Recommend: resume if file exists; prompt to overwrite or extend)
