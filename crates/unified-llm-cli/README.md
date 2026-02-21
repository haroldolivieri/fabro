# unified-llm-cli

A command-line interface for interacting with LLM providers through the [unified-llm](../unified-llm/) library. Supports Anthropic, OpenAI, Google Gemini, and other providers with a single tool.

## Installation

```sh
cargo install --path crates/unified-llm-cli
```

This installs the `ullm` binary.

## Configuration

API keys are read from environment variables. You can set them directly or place them in a `.env` file:

```sh
export ANTHROPIC_API_KEY="sk-..."
export OPENAI_API_KEY="sk-..."
export GEMINI_API_KEY="..."
```

## Commands

### `ullm prompt`

Send a prompt to an LLM and print the response.

```sh
ullm prompt "What is the capital of France?"
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-m, --model <MODEL>` | Model to use (defaults to first catalog model) |
| `-s, --system <TEXT>` | System prompt |
| `-o, --option <KEY=VALUE>` | Generation options: `temperature`, `max_tokens`, `top_p`, or provider-specific keys |
| `-u, --usage` | Show token usage on stderr |
| `--no-stream` | Disable streaming (wait for full response) |

**Stdin support:** Pipe text into `ullm prompt` to use it as input. If both stdin and an argument are provided, they are concatenated.

```sh
echo "Summarize this" | ullm prompt
cat article.txt | ullm prompt "Give me the key points"
```

**Examples:**

```sh
# Use a specific model
ullm prompt -m claude-opus-4-6 "Explain quicksort"

# Set a system prompt
ullm prompt -s "You are a helpful translator" "Translate to French: hello"

# Adjust generation parameters
ullm prompt -o temperature=0.2 -o max_tokens=500 "Write a haiku"

# Show token usage
ullm prompt -u --no-stream "Hello"
```

### `ullm models`

List available models from all providers. Running `ullm models` with no subcommand defaults to `ullm models list`.

```sh
ullm models
```

#### `ullm models list`

```sh
ullm models list
```

**Flags:**

| Flag | Description |
|------|-------------|
| `-p, --provider <NAME>` | Filter models by provider (e.g., `anthropic`, `openai`, `gemini`) |
| `-q, --query <TEXT>` | Search models by ID, display name, or alias (case-insensitive) |

**Examples:**

```sh
# List only Anthropic models
ullm models list --provider anthropic

# Search for models matching "opus"
ullm models list --query opus
```
