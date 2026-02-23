# CLI Design

## Binary

The `attractor` crate (`crates/attractor/`) gains a `[[bin]]` target named `attractor` alongside its existing library. No separate CLI crate.

New files: `src/main.rs`, `src/cli/mod.rs`, `src/cli/run.rs`, `src/cli/validate.rs`.

New dependencies added to the `attractor` crate: `clap`, `anyhow`, `dotenvy`, `chrono` (all already in workspace).

## Command structure

```
attractor run [OPTIONS] <pipeline.dot>
attractor validate [OPTIONS] <pipeline.dot>
attractor --version
attractor --help
```

## `attractor run`

```
Usage: attractor run [OPTIONS] <PIPELINE>

Arguments:
  <PIPELINE>  Path to a .dot pipeline file

Options:
      --logs-dir <DIR>         Log/artifact directory [default: ./attractor-run-<YYYYMMDD-HHMMSS>]
      --dry-run                Execute with a simulated LLM backend (no API calls)
      --auto-approve           Auto-approve all human-in-the-loop gates
      --resume <CHECKPOINT>    Resume from a checkpoint JSON file
      --model <MODEL>          Override default LLM model for all nodes
      --provider <PROVIDER>    Override default LLM provider (anthropic, openai, gemini)
  -v, --verbose...             Verbosity level (-v summary, -vv full details)
  -h, --help                   Show help
```

## `attractor validate`

```
Usage: attractor validate [OPTIONS] <PIPELINE>

Arguments:
  <PIPELINE>  Path to a .dot pipeline file

Options:
  -h, --help  Show help
```

Parse, transform, and validate only. Prints errors and warnings, exits 0 if no errors.

### Environment variables

| Variable | Purpose |
|----------|---------|
| `ANTHROPIC_API_KEY` | Anthropic API key |
| `OPENAI_API_KEY` | OpenAI API key |
| `GEMINI_API_KEY` | Google Gemini API key |

Loaded via `dotenvy::dotenv().ok()` at startup (`.env` file support, non-fatal if missing).

## Execution flow

### `attractor validate`

```
validate_command(args):
  1. Read .dot file
  2. PipelineBuilder::new().prepare(&source) -> (Graph, Vec<Diagnostic>)
  3. Print "Parsed pipeline: {name} ({n} nodes, {m} edges)"
  4. Print errors (Severity::Error) to stderr
  5. Print warnings (Severity::Warning) to stderr
  6. If errors -> exit 1
  7. Print "Validation: OK", exit 0
```

### `attractor run`

```
main()
  1. dotenvy::dotenv()
  2. parse CLI args (clap)
  3. dispatch to run_command(args) or validate_command(args)

run_command(args):
  1. Read .dot file from args.pipeline
  2. Prepare pipeline
       PipelineBuilder::new().prepare(&source)
       -> (Graph, Vec<Diagnostic>)
  3. Print parsed summary to stdout
       "Parsed pipeline: {name} ({n} nodes, {m} edges)"
       "Goal: {goal}"
  4. Check for validation errors (Severity::Error)
       If any -> print to stderr, exit 1
  5. Print warnings (Severity::Warning) to stderr

  6. Create logs directory
       args.logs_dir or generate ./attractor-run-<YYYYMMDD-HHMMSS>
       fs::create_dir_all()
  7. Build LLM client
       If --dry-run -> skip (no backend set on engine, handlers use dry-run stubs)
       Else -> unified_llm::Client::from_env()
         If no providers configured -> warn to stderr, continue as dry-run
  8. Resolve model/provider
       CLI --model/--provider override > graph-level defaults > auto-detect from available providers
  9. Build CodergenBackend
       Wire up LLM client + coding-agent-loop with ExecutionEnv rooted at cwd
 10. Build PipelineEngine
       Create HandlerRegistry, register built-in handlers
       Set backend on codergen handler
       Create EventEmitter
       If -v or -vv -> attach stderr logging callback (level determines format)
       Set interviewer:
         --auto-approve -> AutoApproveInterviewer
         else           -> ConsoleInterviewer
 11. Execute or resume
       If --resume -> load Checkpoint from file, engine.run_from_checkpoint()
       Else        -> engine.run()
 12. Print result
       "=== Pipeline Result ==="
       "Status: {SUCCESS|FAIL|PARTIAL_SUCCESS|...}"
       "Notes: ..."  (if present)
       "Failure: ..." (if present)
       "Logs: {logs_dir}"
 13. Exit code
       0 if SUCCESS or PARTIAL_SUCCESS
       1 otherwise
```

## Clap types

```rust
#[derive(Parser)]
#[command(name = "attractor", version, about = "DOT-based pipeline runner for AI workflows")]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Launch a pipeline from a .dot file
    Run(RunArgs),
    /// Parse and validate a pipeline without executing
    Validate(ValidateArgs),
}

#[derive(Args)]
struct RunArgs {
    /// Path to the .dot pipeline file
    pipeline: PathBuf,

    /// Log/artifact directory
    #[arg(long)]
    logs_dir: Option<PathBuf>,

    /// Execute with simulated LLM backend
    #[arg(long)]
    dry_run: bool,

    /// Auto-approve all human gates
    #[arg(long)]
    auto_approve: bool,

    /// Resume from a checkpoint file
    #[arg(long)]
    resume: Option<PathBuf>,

    /// Override default LLM model
    #[arg(long)]
    model: Option<String>,

    /// Override default LLM provider
    #[arg(long)]
    provider: Option<String>,

    /// Verbosity level (-v summary, -vv full details)
    #[arg(short, long, action = clap::ArgAction::Count)]
    verbose: u8,
}

#[derive(Args)]
struct ValidateArgs {
    /// Path to the .dot pipeline file
    pipeline: PathBuf,
}
```

## Verbosity levels

The `-v` flag uses `clap::ArgAction::Count` to support two levels. Both levels write to stderr.

### `-v` (verbose) -- one-line summary per event

Prints the event kind and key identifying fields on a single line. Matches the style of the C reference implementation (kind + node + attempt + optional data), extended with the richer fields available in the Rust `PipelineEvent` enum.

```
[PIPELINE_STARTED] name=Deploy id=abc-123
[STAGE_STARTED] name=build index=1
[STAGE_COMPLETED] name=build index=1 duration=4523ms
[STAGE_RETRYING] name=test index=2 attempt=2 delay=200ms
[STAGE_FAILED] name=test index=2 error="assertion failed" will_retry=true
[PARALLEL_STARTED] branches=3
[PARALLEL_BRANCH_STARTED] branch=lint index=0
[PARALLEL_BRANCH_COMPLETED] branch=lint index=0 duration=1200ms success=true
[PARALLEL_COMPLETED] duration=5100ms succeeded=2 failed=1
[INTERVIEW_STARTED] stage=review_gate question="Approve changes?"
[INTERVIEW_COMPLETED] question="Approve changes?" answer="Approve" duration=12340ms
[INTERVIEW_TIMEOUT] stage=review_gate duration=30000ms
[CHECKPOINT_SAVED] node=test
[PIPELINE_COMPLETED] duration=45230ms artifacts=3
[PIPELINE_FAILED] error="goal gate unsatisfied" duration=32100ms
```

No JSON, no multi-line output. Suitable for tailing in a terminal alongside normal output.

### `-vv` (very verbose) -- full event details

Prints every event as nicely formatted, multi-line output with all fields. Uses indented key-value pairs under a header line.

```
── STAGE_COMPLETED ──────────────────────────
  name:        build
  index:       1
  duration_ms: 4523

── STAGE_FAILED ─────────────────────────────
  name:       test
  index:      2
  error:      assertion failed
  will_retry: true

── PARALLEL_COMPLETED ───────────────────────
  duration_ms:   5100
  success_count: 2
  failure_count: 1

── INTERVIEW_COMPLETED ──────────────────────
  question:    Approve changes?
  answer:      Approve
  duration_ms: 12340
```

Every field on the event variant is printed. This is the "dump everything" mode for debugging pipeline behavior.

## Output conventions

- **stdout**: Pipeline summary, result status.
- **stderr**: Warnings, verbose events, progress messages, LLM provider warnings.

## Error handling

| Condition | Behavior |
|-----------|----------|
| .dot file not found / unreadable | Print error to stderr, exit 1 |
| Parse failure | Print parse error to stderr, exit 1 |
| Validation errors | Print each error to stderr, exit 1 |
| No LLM providers (without --dry-run) | Warn to stderr, continue in dry-run mode |
| Engine execution error | Print error to stderr, exit 1 |
| Pipeline completes with FAIL | Print result, exit 1 |
| Pipeline completes with SUCCESS/PARTIAL_SUCCESS | Print result, exit 0 |

All errors go through `anyhow` at the CLI boundary. The `main` function catches the result and formats it.

## File layout

```
crates/attractor/
  Cargo.toml          -- add [[bin]], clap, anyhow, dotenvy, chrono deps
  src/
    main.rs           -- entry point: dotenvy, clap parse, dispatch
    cli/
      mod.rs          -- re-exports
      run.rs          -- run_command()
      validate.rs     -- validate_command()
    lib.rs            -- existing library (unchanged)
    ...
```

## Cargo.toml changes

Add to `crates/attractor/Cargo.toml`:

```toml
[[bin]]
name = "attractor"
path = "src/main.rs"

[dependencies]
# ... existing deps ...
clap.workspace = true
anyhow.workspace = true
dotenvy.workspace = true
chrono.workspace = true
```

Add `anyhow` to workspace root `Cargo.toml` `[workspace.dependencies]`:

```toml
anyhow = "1"
```

(Already present in workspace deps.)

## What this design does NOT cover

- Web/SSE server mode (`attractor serve`) -- separate subcommand later
- JSON/structured output mode (`--output json`) -- can be added later
- Signal handling (Ctrl-C graceful shutdown) -- defer to follow-up
- Config file support (e.g., `~/.config/attractor/config.toml`) -- not needed yet
