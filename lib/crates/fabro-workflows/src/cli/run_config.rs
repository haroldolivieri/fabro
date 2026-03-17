use std::collections::HashMap;
use std::path::Path;

use anyhow::{bail, Context};

// Re-export all config types from fabro-config for backward compatibility.
pub use fabro_config::mcp::{McpServerConfig, McpServerEntry, McpTransport};
pub use fabro_config::run::{
    load_run_config, parse_run_config, resolve_env_refs, resolve_graph_path, AssetsConfig,
    CheckpointConfig, GitHubConfig, LlmConfig, MergeStrategy, PullRequestConfig, RunDefaults,
    SetupConfig, WorkflowRunConfig,
};
pub use fabro_config::sandbox::{
    DaytonaConfig, DaytonaNetwork, DaytonaSnapshotConfig, DockerfileSource, LocalSandboxConfig,
    SandboxConfig, WorktreeMode,
};

/// Expand `$name` placeholders in `source` using the given variable map.
///
/// Identifiers match `[a-zA-Z_][a-zA-Z0-9_]*`. A `$` not followed by an
/// identifier character is left as-is. Undefined variables produce an error.
pub fn expand_vars(source: &str, vars: &HashMap<String, String>) -> anyhow::Result<String> {
    let mut result = String::with_capacity(source.len());
    let bytes = source.as_bytes();
    let len = bytes.len();
    let mut i = 0;

    while i < len {
        if bytes[i] == b'$' {
            let start = i + 1;
            if start < len && bytes[start] == b'$' {
                result.push('$');
                i = start + 1;
            } else if start < len && (bytes[start].is_ascii_alphabetic() || bytes[start] == b'_') {
                let mut end = start + 1;
                while end < len && (bytes[end].is_ascii_alphanumeric() || bytes[end] == b'_') {
                    end += 1;
                }
                let name = &source[start..end];
                match vars.get(name) {
                    Some(value) => result.push_str(value),
                    None => bail!("Undefined variable: ${name}"),
                }
                i = end;
            } else {
                result.push('$');
                i = start;
            }
        } else {
            result.push(source[i..].chars().next().unwrap());
            i += source[i..].chars().next().unwrap().len_utf8();
        }
    }

    Ok(result)
}

/// Run setup commands sequentially in the given directory.
///
/// Each command gets the full `timeout_ms` budget. Commands are executed
/// via `sh -c` so shell features (pipes, redirects, etc.) work.
pub async fn run_setup(setup: &SetupConfig, directory: &Path) -> anyhow::Result<()> {
    let timeout = std::time::Duration::from_millis(setup.timeout_ms.unwrap_or(300_000));

    for cmd in &setup.commands {
        let fut = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(cmd)
            .current_dir(directory)
            .output();

        let output = tokio::time::timeout(timeout, fut)
            .await
            .with_context(|| {
                format!(
                    "Setup command timed out after {}ms: {cmd}",
                    timeout.as_millis()
                )
            })?
            .with_context(|| format!("Failed to execute setup command: {cmd}"))?;

        if !output.status.success() {
            let code = output
                .status
                .code()
                .map_or("unknown".to_string(), |c| c.to_string());
            let stderr = String::from_utf8_lossy(&output.stderr);
            bail!("Setup command failed (exit code {code}): {cmd}\n{stderr}");
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::{DaytonaSnapshotConfig, DockerfileSource};

    #[test]
    fn parse_toml_with_vars() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[vars]
repo_url = "https://github.com/org/repo"
language = "python"
"#;
        let config = parse_run_config(toml).unwrap();
        let vars = config.vars.unwrap();
        assert_eq!(vars["repo_url"], "https://github.com/org/repo");
        assert_eq!(vars["language"], "python");
    }

    #[test]
    fn expand_single_var() {
        let vars = HashMap::from([("name".to_string(), "world".to_string())]);
        assert_eq!(expand_vars("Hello $name", &vars).unwrap(), "Hello world");
    }

    #[test]
    fn expand_multiple_vars() {
        let vars = HashMap::from([
            ("greeting".to_string(), "Hello".to_string()),
            ("name".to_string(), "world".to_string()),
        ]);
        assert_eq!(
            expand_vars("$greeting $name!", &vars).unwrap(),
            "Hello world!"
        );
    }

    #[test]
    fn expand_undefined_var_errors() {
        let vars = HashMap::new();
        let err = expand_vars("Hello $missing", &vars).unwrap_err();
        assert!(
            err.to_string().contains("Undefined variable: $missing"),
            "unexpected error: {err}"
        );
    }

    #[test]
    fn expand_no_vars_passthrough() {
        let vars = HashMap::new();
        assert_eq!(
            expand_vars("no variables here", &vars).unwrap(),
            "no variables here"
        );
    }

    #[test]
    fn expand_dollar_not_followed_by_ident() {
        let vars = HashMap::new();
        assert_eq!(expand_vars("costs $5", &vars).unwrap(), "costs $5");
    }

    #[test]
    fn expand_escaped_dollar() {
        let vars = HashMap::from([("name".to_string(), "world".to_string())]);
        assert_eq!(
            expand_vars("literal $$name here", &vars).unwrap(),
            "literal $name here"
        );
    }

    #[test]
    fn expand_escaped_dollar_at_end() {
        let vars = HashMap::new();
        assert_eq!(expand_vars("trailing $$", &vars).unwrap(), "trailing $");
    }

    #[test]
    fn expand_escaped_dollar_before_non_ident() {
        let vars = HashMap::new();
        assert_eq!(expand_vars("price is $$5", &vars).unwrap(), "price is $5");
    }

    #[test]
    fn parse_toml_with_devcontainer_enabled() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[sandbox]
provider = "daytona"
devcontainer = true
"#;
        let config = parse_run_config(toml).unwrap();
        let sandbox = config.sandbox.unwrap();
        assert_eq!(sandbox.devcontainer, Some(true));
    }

    #[test]
    fn parse_toml_without_devcontainer() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[sandbox]
provider = "daytona"
"#;
        let config = parse_run_config(toml).unwrap();
        let sandbox = config.sandbox.unwrap();
        assert_eq!(sandbox.devcontainer, None);
    }

    #[test]
    fn parse_toml_with_sandbox() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[sandbox]
provider = "daytona"
"#;
        let config = parse_run_config(toml).unwrap();
        let sandbox = config.sandbox.unwrap();
        assert_eq!(sandbox.provider.as_deref(), Some("daytona"));
        assert!(sandbox.daytona.is_none());
    }

    #[test]
    fn parse_toml_with_daytona_config() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[sandbox]
provider = "daytona"

[sandbox.daytona]
auto_stop_interval = 60

[sandbox.daytona.labels]
project = "fabro"
environment = "ci"
"#;
        let config = parse_run_config(toml).unwrap();
        let sandbox = config.sandbox.unwrap();
        let daytona = sandbox.daytona.unwrap();
        assert_eq!(daytona.auto_stop_interval, Some(60));
        let labels = daytona.labels.unwrap();
        assert_eq!(labels["project"], "fabro");
        assert_eq!(labels["environment"], "ci");
    }

    #[test]
    fn parse_toml_with_daytona_snapshot() {
        let toml = r#"
version = 1
goal = "Run tests"
graph = "workflow.fabro"

[sandbox]
provider = "daytona"

[sandbox.daytona.snapshot]
name = "my-snapshot"
cpu = 4
memory = 8
disk = 32
"#;
        let config = parse_run_config(toml).unwrap();
        let snap = config.sandbox.unwrap().daytona.unwrap().snapshot.unwrap();
        assert_eq!(snap.name, "my-snapshot");
        assert_eq!(snap.cpu, Some(4));
        assert_eq!(snap.memory, Some(8));
        assert_eq!(snap.disk, Some(32));
    }

    #[test]
    fn parse_toml_with_inline_dockerfile() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[sandbox.daytona.snapshot]
name = "custom"
dockerfile = "FROM rust:1.85-slim-bookworm"
"#;
        let config = parse_run_config(toml).unwrap();
        let snap = config.sandbox.unwrap().daytona.unwrap().snapshot.unwrap();
        assert_eq!(
            snap.dockerfile,
            Some(DockerfileSource::Inline(
                "FROM rust:1.85-slim-bookworm".into()
            ))
        );
    }

    #[test]
    fn parse_toml_with_path_dockerfile() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[sandbox.daytona.snapshot]
name = "custom"

[sandbox.daytona.snapshot.dockerfile]
path = "./Dockerfile"
"#;
        let config = parse_run_config(toml).unwrap();
        let snap = config.sandbox.unwrap().daytona.unwrap().snapshot.unwrap();
        assert_eq!(
            snap.dockerfile,
            Some(DockerfileSource::Path {
                path: "./Dockerfile".into()
            })
        );
    }

    #[test]
    fn resolve_dockerfile_replaces_path_with_content() {
        let dir = tempfile::tempdir().unwrap();
        let dockerfile_path = dir.path().join("Dockerfile");
        std::fs::write(&dockerfile_path, "FROM ubuntu:24.04\nRUN apt-get update").unwrap();
        let toml_path = dir.path().join("workflow.toml");
        std::fs::write(
            &toml_path,
            r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[sandbox.daytona.snapshot]
name = "custom"

[sandbox.daytona.snapshot.dockerfile]
path = "./Dockerfile"
"#,
        )
        .unwrap();
        let config = load_run_config(&toml_path).unwrap();
        let snap = config.sandbox.unwrap().daytona.unwrap().snapshot.unwrap();
        assert_eq!(
            snap.dockerfile,
            Some(DockerfileSource::Inline(
                "FROM ubuntu:24.04\nRUN apt-get update".into()
            ))
        );
    }

    #[test]
    fn apply_defaults_fills_missing_fields() {
        let defaults = RunDefaults {
            work_dir: Some("/ws".into()),
            llm: Some(LlmConfig {
                model: Some("m".into()),
                provider: Some("p".into()),
                fallbacks: None,
            }),
            ..Default::default()
        };
        let toml = r#"
version = 1
graph = "workflow.fabro"
"#;
        let mut config = parse_run_config(toml).unwrap();
        config.apply_defaults(&defaults);
        assert_eq!(config.work_dir.as_deref(), Some("/ws"));
        assert_eq!(config.llm.as_ref().unwrap().model.as_deref(), Some("m"));
    }

    #[test]
    fn apply_defaults_task_wins() {
        let defaults = RunDefaults {
            work_dir: Some("/default".into()),
            llm: Some(LlmConfig {
                model: Some("default-m".into()),
                provider: Some("default-p".into()),
                fallbacks: None,
            }),
            ..Default::default()
        };
        let toml = r#"
version = 1
graph = "workflow.fabro"
work_dir = "/task"

[llm]
model = "task-m"
"#;
        let mut config = parse_run_config(toml).unwrap();
        config.apply_defaults(&defaults);
        assert_eq!(config.work_dir.as_deref(), Some("/task"));
        assert_eq!(
            config.llm.as_ref().unwrap().model.as_deref(),
            Some("task-m")
        );
        // provider filled from defaults
        assert_eq!(
            config.llm.as_ref().unwrap().provider.as_deref(),
            Some("default-p")
        );
    }

    #[test]
    fn apply_defaults_merges_vars() {
        let defaults = RunDefaults {
            vars: Some(HashMap::from([
                ("a".into(), "1".into()),
                ("b".into(), "2".into()),
            ])),
            ..Default::default()
        };
        let toml = r#"
version = 1
graph = "workflow.fabro"

[vars]
b = "override"
c = "3"
"#;
        let mut config = parse_run_config(toml).unwrap();
        config.apply_defaults(&defaults);
        let vars = config.vars.unwrap();
        assert_eq!(vars["a"], "1");
        assert_eq!(vars["b"], "override");
        assert_eq!(vars["c"], "3");
    }

    #[test]
    fn apply_defaults_daytona_deep_merge() {
        let defaults = RunDefaults {
            sandbox: Some(SandboxConfig {
                provider: Some("daytona".into()),
                preserve: None,
                devcontainer: None,
                local: None,
                daytona: Some(DaytonaConfig {
                    auto_stop_interval: Some(30),
                    labels: Some(HashMap::from([("env".into(), "prod".into())])),
                    snapshot: Some(DaytonaSnapshotConfig {
                        name: "base".into(),
                        cpu: Some(2),
                        memory: None,
                        disk: None,
                        dockerfile: None,
                    }),
                    network: None,
                }),
                #[cfg(feature = "exedev")]
                exe: None,
                ssh: None,
                env: None,
            }),
            ..Default::default()
        };
        let toml = r#"
version = 1
graph = "workflow.fabro"

[sandbox.daytona]
auto_stop_interval = 60

[sandbox.daytona.labels]
team = "a"
"#;
        let mut config = parse_run_config(toml).unwrap();
        config.apply_defaults(&defaults);
        let d = config.sandbox.unwrap().daytona.unwrap();
        assert_eq!(d.auto_stop_interval, Some(60));
        let labels = d.labels.unwrap();
        assert_eq!(labels["env"], "prod");
        assert_eq!(labels["team"], "a");
        assert_eq!(d.snapshot.unwrap().name, "base");
    }

    #[test]
    fn merge_overlay_basic() {
        let mut base = RunDefaults {
            work_dir: Some("/base".into()),
            llm: Some(LlmConfig {
                model: Some("base-m".into()),
                provider: None,
                fallbacks: None,
            }),
            ..Default::default()
        };
        let overlay = RunDefaults {
            llm: Some(LlmConfig {
                model: None,
                provider: Some("overlay-p".into()),
                fallbacks: None,
            }),
            ..Default::default()
        };
        base.merge_overlay(overlay);
        assert_eq!(base.work_dir.as_deref(), Some("/base"));
        assert_eq!(base.llm.as_ref().unwrap().model.as_deref(), Some("base-m"));
        assert_eq!(
            base.llm.as_ref().unwrap().provider.as_deref(),
            Some("overlay-p")
        );
    }

    #[test]
    fn resolve_graph_path_relative() {
        let toml_path = std::path::Path::new("/home/user/workflows/wf/workflow.toml");
        let dot = resolve_graph_path(toml_path, "workflow.fabro");
        assert_eq!(
            dot,
            std::path::PathBuf::from("/home/user/workflows/wf/workflow.fabro")
        );
    }

    #[test]
    fn resolve_graph_path_absolute() {
        let toml_path = std::path::Path::new("/home/user/workflows/wf/workflow.toml");
        let dot = resolve_graph_path(toml_path, "/absolute/path.fabro");
        assert_eq!(dot, std::path::PathBuf::from("/absolute/path.fabro"));
    }

    #[test]
    fn parse_toml_with_mcp_servers() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[mcp_servers.filesystem]
type = "stdio"
command = ["npx", "-y", "@modelcontextprotocol/server-filesystem"]
startup_timeout_secs = 20
tool_timeout_secs = 120
"#;
        let config = parse_run_config(toml).unwrap();
        assert_eq!(config.mcp_servers.len(), 1);
        let entry = &config.mcp_servers["filesystem"];
        assert_eq!(entry.startup_timeout_secs, 20);
        assert_eq!(entry.tool_timeout_secs, 120);
    }

    #[test]
    fn parse_toml_with_sandbox_env() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[sandbox.env]
MY_VAR = "hello"
"#;
        let config = parse_run_config(toml).unwrap();
        let env = config.sandbox.unwrap().env.unwrap();
        assert_eq!(env["MY_VAR"], "hello");
    }

    #[test]
    fn parse_run_config_with_hooks() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[[hooks]]
event = "run_start"
command = "echo starting"
"#;
        let config = parse_run_config(toml).unwrap();
        assert_eq!(config.hooks.len(), 1);
    }

    #[test]
    fn parse_run_config_with_github() {
        let toml = r#"
version = 1
goal = "test"
graph = "workflow.fabro"

[github]
permissions = { contents = "read" }
"#;
        let config = parse_run_config(toml).unwrap();
        let github = config.github.unwrap();
        assert_eq!(github.permissions["contents"], "read");
    }

    #[test]
    fn parse_toml_worktree_modes() {
        for (mode, expected) in [
            ("always", WorktreeMode::Always),
            ("clean", WorktreeMode::Clean),
            ("dirty", WorktreeMode::Dirty),
            ("never", WorktreeMode::Never),
        ] {
            let toml = format!(
                r#"
version = 1
graph = "workflow.fabro"

[sandbox.local]
worktree_mode = "{mode}"
"#
            );
            let config = parse_run_config(&toml).unwrap();
            assert_eq!(
                config.sandbox.unwrap().local.unwrap().worktree_mode,
                expected,
                "failed for mode: {mode}"
            );
        }
    }
}
