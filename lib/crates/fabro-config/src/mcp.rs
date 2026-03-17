use std::collections::HashMap;
use std::time::Duration;

use serde::{Deserialize, Serialize};

pub fn default_startup_timeout_secs() -> u64 {
    10
}

pub fn default_tool_timeout_secs() -> u64 {
    60
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct McpServerConfig {
    pub name: String,
    pub transport: McpTransport,
    #[serde(default = "default_startup_timeout_secs")]
    pub startup_timeout_secs: u64,
    #[serde(default = "default_tool_timeout_secs")]
    pub tool_timeout_secs: u64,
}

impl McpServerConfig {
    #[must_use]
    pub fn startup_timeout(&self) -> Duration {
        Duration::from_secs(self.startup_timeout_secs)
    }

    #[must_use]
    pub fn tool_timeout(&self) -> Duration {
        Duration::from_secs(self.tool_timeout_secs)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum McpTransport {
    Stdio {
        command: Vec<String>,
        #[serde(default)]
        env: HashMap<String, String>,
    },
    Http {
        url: String,
        #[serde(default)]
        headers: HashMap<String, String>,
    },
    /// MCP server that runs inside a sandbox and is accessed via HTTP preview URL.
    /// During session init, the server is started inside the sandbox and this
    /// variant is resolved into an `Http` transport using the sandbox's preview URL.
    Sandbox {
        command: Vec<String>,
        port: u16,
        #[serde(default)]
        env: HashMap<String, String>,
    },
}

/// MCP server entry as it appears in TOML config files (without a `name` field).
///
/// Converted to [`McpServerConfig`] via [`McpServerEntry::into_config`].
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct McpServerEntry {
    #[serde(flatten)]
    pub transport: McpTransport,
    #[serde(default = "default_startup_timeout_secs")]
    pub startup_timeout_secs: u64,
    #[serde(default = "default_tool_timeout_secs")]
    pub tool_timeout_secs: u64,
}

impl McpServerEntry {
    pub fn into_config(self, name: String) -> McpServerConfig {
        McpServerConfig {
            name,
            transport: self.transport,
            startup_timeout_secs: self.startup_timeout_secs,
            tool_timeout_secs: self.tool_timeout_secs,
        }
    }
}
