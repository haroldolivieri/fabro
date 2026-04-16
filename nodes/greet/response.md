Here's a breakdown of the top-level contents of the **Fabro** repository:

### 📁 Directories
| Directory | Description |
|---|---|
| `.ai` | AI-related configuration/assets |
| `.cargo` | Cargo (Rust) configuration |
| `.claude` | Claude AI assistant config |
| `.config` | General config files |
| `.fabro` | Fabro workflow definitions (e.g. `.fabro/workflows/`) |
| `.git` | Git repository metadata |
| `.github` | GitHub Actions CI/CD workflows |
| `apps` | Frontend applications (e.g. `fabro-web` React UI, `marketing` Astro site) |
| `bin` | Binary/script entrypoints |
| `docker` | Dockerfiles and container setup |
| `docs` | Public documentation (Mintlify), including the OpenAPI spec |
| `docs-internal` | Internal strategy docs (logging, events, testing) |
| `evals` | Evaluation harnesses |
| `files-internal` | Internal reference files |
| `installer` | Install scripts/tooling |
| `lib` | Rust crates (`lib/crates/`) and TypeScript packages (`lib/packages/`) |
| `scripts` | Utility/build scripts |
| `test` | Test fixtures, scenarios, and helpers |

### 📄 Key Files
| File | Description |
|---|---|
| `README.md` | Project overview |
| `AGENTS.md` / `CLAUDE.md` | AI assistant guidance (CLAUDE.md is a symlink to AGENTS.md) |
| `CONTRIBUTING.md` | Contribution guidelines |
| `Cargo.toml` / `Cargo.lock` | Rust workspace manifest and lockfile |
| `package.json` / `bun.lock` | Root JS/TS package config and lockfile (Bun) |
| `rustfmt.toml` / `clippy.toml` | Rust formatting and lint config |
| `.env.example` | Example environment variables for local dev |
| `install.sh` / `install.md` | Installer script and docs (symlinked from `apps/marketing/public/`) |
| `LICENSE.md` | Project license |

The repo is a **monorepo** combining Rust (workspace of crates under `lib/crates/`) and TypeScript (apps under `apps/` and packages under `lib/packages/`) — all orchestrated around the Fabro AI workflow platform. 🚀