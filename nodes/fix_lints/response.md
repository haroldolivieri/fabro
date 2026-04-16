Both clippy lint errors are fixed. Here's a summary of the changes:

1. **`lib/crates/fabro-server/src/serve.rs:504`** — Added `#[expect(clippy::disallowed_methods, reason = "...")]` to the `std::process::Command::new("bun")` call. This is an intentional synchronous subprocess spawn for the `--watch-web` debug feature that manages a long-lived `bun run dev` child process outside the Tokio runtime, so switching to `tokio::process::Command` is not appropriate here.

2. **`lib/crates/fabro-cli/src/commands/install.rs:161`** — Changed `fabro_server::serve::DEFAULT_TCP_PORT` to `serve::DEFAULT_TCP_PORT`, using the already-imported `use fabro_server::serve;` module path instead of an absolute path, satisfying the `clippy::absolute_paths` lint.