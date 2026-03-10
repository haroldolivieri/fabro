<div align="left" id="top">
<a href="https://arc.dev"><img alt="Arc" src="docs/logo/dark.svg" height="75"></a>
</div>

## The software factory for small teams of expert engineers

AI coding agents are powerful but unpredictable. You either babysit every step or review a 50-file diff you don't trust. Arc gives you a middle path: define the process as a graph, let agents execute it, and intervene only where it matters.

[![License: MIT](https://img.shields.io/badge/license-MIT-blue)](LICENSE.md)
[![docs](https://img.shields.io/badge/docs-arc.dev-357F9E)](https://arc.dev)

---

## Key Features

- **Deterministic workflow graphs** -- Define pipelines in Graphviz DOT with branching, loops, parallelism, and human gates. Diffable, reviewable, version-controlled.
- **Human-in-the-loop** -- Approval gates pause for human decisions. Steer running agents mid-turn. Interview steps collect structured input. You stay in control without babysitting.
- **Multi-model routing** -- Route each node to the right model and provider with CSS-like stylesheets. Cheap models for boilerplate, frontier models for hard reasoning, with automatic provider fallback.
- **Cloud sandboxes** -- Run agents in isolated Daytona cloud VMs with snapshot-based setup, network controls, and automatic cleanup.
- **SSH access and preview links** -- Shell into running sandboxes with `arc ssh` and expose ports with `arc preview` for live debugging while workflows run.
- **Git checkpointing** -- Every stage commits to a branch. Inspect changes, revert mistakes, or resume interrupted runs exactly where they left off.
- **Automatic retros** -- Each run generates a retrospective with cost, duration, files touched, and an LLM-written narrative rating smoothness and flagging friction points.
- **Comprehensive API** -- `arc serve` exposes a full REST API with SSE event streaming and a React web UI. Run workflows programmatically, build integrations, or operate Arc as a service.
- **Single binary, no runtime** -- One compiled Rust executable with zero dependencies. No Python, no Node, no Docker required to get started.
- **Open source (MIT)** -- Full source code, no vendor lock-in. Self-host, fork, or extend to fit your workflow.

Read the full [documentation](https://arc.dev) for details.

---

## Quick Start

### Install

Download the latest release for your platform:

```bash
# macOS (Apple Silicon)
curl -fsSL https://github.com/brynary/arc/releases/latest/download/arc-aarch64-apple-darwin.tar.gz | tar xz
sudo mv arc /usr/local/bin/

# Linux (x86_64)
curl -fsSL https://github.com/brynary/arc/releases/latest/download/arc-x86_64-unknown-linux-gnu.tar.gz | tar xz
sudo mv arc /usr/local/bin/
```

Or download directly from [GitHub Releases](https://github.com/brynary/arc/releases).

<details>
<summary>Build from source</summary>

Requires [Rust](https://rustup.rs/) (latest stable):

```bash
git clone https://github.com/brynary/arc.git
cd arc
cargo build --release
# Binary is at ./target/release/arc
```

</details>

### Run the setup wizard

```bash
arc install
```

### Initialize your project

```bash
cd my-repo/
arc init
```

### Run your first workflow

```bash
arc run hello
```

---

## Example Workflow

A plan-approve-implement workflow where a human reviews the plan before the agent writes code:

```dot
digraph PlanImplement {
    graph [
        goal="Plan, approve, implement, and simplify a change"
        model_stylesheet="
            *        { llm_model: claude-haiku-4-5; reasoning_effort: low; }
            .coding  { llm_model: claude-sonnet-4-5; reasoning_effort: high; }
        "
    ]

    start [shape=Mdiamond, label="Start"]
    exit  [shape=Msquare, label="Exit"]

    plan      [label="Plan", prompt="Analyze the goal and codebase. Write a step-by-step plan.", reasoning_effort="high"]
    approve   [shape=hexagon, label="Approve Plan"]
    implement [label="Implement", class="coding", prompt="Read plan.md and implement every step."]
    simplify  [label="Simplify", class="coding", prompt="Review the changes for clarity and correctness."]

    start -> plan -> approve
    approve -> implement [label="[A] Approve"]
    approve -> plan      [label="[R] Revise"]
    implement -> simplify -> exit
}
```

Agents run as multi-turn LLM sessions with tool access. Human gates (`hexagon`) pause for approval. The stylesheet routes planning to a cheap model and coding to a frontier model. See the [DOT language reference](https://arc.dev/reference/dot-language) for the full syntax.

---

## System Requirements

- **macOS** (Apple Silicon) or **Linux** (x86_64)
- **At least one LLM provider API key** (Anthropic, OpenAI, or Gemini)
- **Git** (for checkpoint and resume)
- **Rust** (only if building from source)

---

## Help or Feedback

- Read the [documentation](https://arc.dev)
- [Bug reports](https://github.com/brynary/arc/issues) via GitHub Issues
- [Feature requests](https://github.com/brynary/arc/issues) via GitHub Issues
- Email [bryan@qlty.sh](mailto:bryan@qlty.sh) for questions

---

## Contributing

See [CLAUDE.md](CLAUDE.md) for build commands, architecture overview, and development conventions.

---

## License

Arc is licensed under the [MIT License](LICENSE.md).
