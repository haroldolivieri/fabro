use crate::config::SessionConfig;
use crate::execution_env::*;
use crate::profiles::EnvContext;
use crate::provider_profile::{ProfileCapabilities, ProviderProfile};
use crate::session::Session;
use crate::tool_registry::ToolRegistry;
use async_trait::async_trait;
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use unified_llm::client::Client;
use unified_llm::error::SdkError;
use unified_llm::provider::{ProviderAdapter, StreamEventStream};
use unified_llm::types::{FinishReason, Message, Request, Response, Usage};

// --- MockExecutionEnvironment ---

pub(crate) struct MockExecutionEnvironment {
    pub files: HashMap<String, String>,
    pub exec_result: ExecResult,
    pub grep_results: Vec<String>,
    pub glob_results: Vec<String>,
    pub working_dir: &'static str,
    pub platform_str: &'static str,
    pub os_version_str: String,
}

impl Default for MockExecutionEnvironment {
    fn default() -> Self {
        Self {
            files: HashMap::new(),
            exec_result: ExecResult {
                stdout: "mock output".into(),
                stderr: String::new(),
                exit_code: 0,
                timed_out: false,
                duration_ms: 10,
            },
            grep_results: vec![],
            glob_results: vec![],
            working_dir: "/tmp/test",
            platform_str: "darwin",
            os_version_str: "Darwin 24.0.0".into(),
        }
    }
}

#[async_trait]
impl ExecutionEnvironment for MockExecutionEnvironment {
    async fn read_file(
        &self,
        path: &str,
        _offset: Option<usize>,
        _limit: Option<usize>,
    ) -> Result<String, String> {
        self.files
            .get(path)
            .cloned()
            .ok_or_else(|| format!("File not found: {path}"))
    }

    async fn write_file(&self, _path: &str, _content: &str) -> Result<(), String> {
        Ok(())
    }

    async fn delete_file(&self, _path: &str) -> Result<(), String> {
        Ok(())
    }

    async fn file_exists(&self, path: &str) -> Result<bool, String> {
        Ok(self.files.contains_key(path))
    }

    async fn list_directory(
        &self,
        _path: &str,
        _depth: Option<usize>,
    ) -> Result<Vec<DirEntry>, String> {
        Ok(vec![])
    }

    async fn exec_command(
        &self,
        _command: &str,
        _timeout_ms: u64,
        _working_dir: Option<&str>,
        _env_vars: Option<&std::collections::HashMap<String, String>>,
    ) -> Result<ExecResult, String> {
        Ok(self.exec_result.clone())
    }

    async fn grep(
        &self,
        _pattern: &str,
        _path: &str,
        _options: &GrepOptions,
    ) -> Result<Vec<String>, String> {
        Ok(self.grep_results.clone())
    }

    async fn glob(&self, _pattern: &str, _path: Option<&str>) -> Result<Vec<String>, String> {
        Ok(self.glob_results.clone())
    }

    async fn initialize(&self) -> Result<(), String> {
        Ok(())
    }

    async fn cleanup(&self) -> Result<(), String> {
        Ok(())
    }

    fn working_directory(&self) -> &str {
        self.working_dir
    }

    fn platform(&self) -> &str {
        self.platform_str
    }

    fn os_version(&self) -> String {
        self.os_version_str.clone()
    }
}

// --- TestProfile ---

pub(crate) struct TestProfile {
    pub registry: ToolRegistry,
}

impl TestProfile {
    pub fn new() -> Self {
        Self {
            registry: ToolRegistry::new(),
        }
    }

    pub fn with_tools(registry: ToolRegistry) -> Self {
        Self { registry }
    }
}

impl ProviderProfile for TestProfile {
    fn id(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    fn tool_registry(&self) -> &ToolRegistry {
        &self.registry
    }

    fn tool_registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    fn build_system_prompt(
        &self,
        _env: &dyn ExecutionEnvironment,
        _env_context: &EnvContext,
        _project_docs: &[String],
        _user_instructions: Option<&str>,
    ) -> String {
        "You are a test assistant.".into()
    }

    fn capabilities(&self) -> ProfileCapabilities {
        ProfileCapabilities {
            supports_reasoning: false,
            supports_streaming: false,
            supports_parallel_tool_calls: false,
            context_window_size: 200_000,
        }
    }

    fn knowledge_cutoff(&self) -> &str {
        "May 2025"
    }
}

// --- MockLlmProvider ---

pub(crate) struct MockLlmProvider {
    pub responses: Vec<Response>,
    pub call_index: AtomicUsize,
}

impl MockLlmProvider {
    pub fn new(responses: Vec<Response>) -> Self {
        Self {
            responses,
            call_index: AtomicUsize::new(0),
        }
    }
}

#[async_trait]
impl ProviderAdapter for MockLlmProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn complete(&self, _request: &Request) -> Result<Response, SdkError> {
        let idx = self.call_index.fetch_add(1, Ordering::SeqCst);
        if idx < self.responses.len() {
            Ok(self.responses[idx].clone())
        } else {
            Ok(self.responses[self.responses.len() - 1].clone())
        }
    }

    async fn stream(&self, _request: &Request) -> Result<StreamEventStream, SdkError> {
        Err(SdkError::Configuration {
            message: "streaming not supported in mock".into(),
        })
    }
}

// --- Helper functions ---

pub(crate) fn text_response(text: &str) -> Response {
    Response {
        id: format!("resp_{text}"),
        model: "mock-model".into(),
        provider: "mock".into(),
        message: Message::assistant(text),
        finish_reason: FinishReason::Stop,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            ..Default::default()
        },
        raw: None,
        warnings: vec![],
        rate_limit: None,
    }
}

pub(crate) async fn make_client(provider: Arc<dyn ProviderAdapter>) -> Client {
    let mut providers = HashMap::new();
    providers.insert(provider.name().to_string(), provider);
    Client::new(providers, Some("mock".into()), vec![])
}

pub(crate) async fn make_session(responses: Vec<Response>) -> Session {
    let provider = Arc::new(MockLlmProvider::new(responses));
    let client = make_client(provider).await;
    let profile = Arc::new(TestProfile::new());
    let env = Arc::new(MockExecutionEnvironment::default());
    Session::new(client, profile, env, SessionConfig::default())
}

pub(crate) async fn make_session_with_tools(
    responses: Vec<Response>,
    registry: ToolRegistry,
) -> Session {
    let provider = Arc::new(MockLlmProvider::new(responses));
    let client = make_client(provider).await;
    let profile = Arc::new(TestProfile::with_tools(registry));
    let env = Arc::new(MockExecutionEnvironment::default());
    Session::new(client, profile, env, SessionConfig::default())
}

pub(crate) async fn make_session_with_config(
    responses: Vec<Response>,
    config: SessionConfig,
) -> Session {
    let provider = Arc::new(MockLlmProvider::new(responses));
    let client = make_client(provider).await;
    let profile = Arc::new(TestProfile::new());
    let env = Arc::new(MockExecutionEnvironment::default());
    Session::new(client, profile, env, config)
}

pub(crate) async fn make_session_with_tools_and_config(
    responses: Vec<Response>,
    registry: ToolRegistry,
    config: SessionConfig,
) -> Session {
    let provider = Arc::new(MockLlmProvider::new(responses));
    let client = make_client(provider).await;
    let profile = Arc::new(TestProfile::with_tools(registry));
    let env = Arc::new(MockExecutionEnvironment::default());
    Session::new(client, profile, env, config)
}

pub(crate) fn tool_call_response(
    tool_name: &str,
    tool_call_id: &str,
    args: serde_json::Value,
) -> Response {
    use unified_llm::types::{ContentPart, Role, ToolCall};
    Response {
        id: format!("resp_{tool_call_id}"),
        model: "mock-model".into(),
        provider: "mock".into(),
        message: Message {
            role: Role::Assistant,
            content: vec![
                ContentPart::text("Let me use a tool."),
                ContentPart::ToolCall(ToolCall::new(tool_call_id, tool_name, args)),
            ],
            name: None,
            tool_call_id: None,
        },
        finish_reason: FinishReason::ToolCalls,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            ..Default::default()
        },
        raw: None,
        warnings: vec![],
        rate_limit: None,
    }
}

pub(crate) fn make_echo_tool() -> crate::tool_registry::RegisteredTool {
    use unified_llm::types::ToolDefinition;
    crate::tool_registry::RegisteredTool {
        definition: ToolDefinition {
            name: "echo".into(),
            description: "Echoes the input".into(),
            parameters: serde_json::json!({"type": "object", "properties": {"text": {"type": "string"}}}),
        },
        executor: Arc::new(|args, _env| {
            Box::pin(async move {
                let text = args
                    .get("text")
                    .and_then(|v| v.as_str())
                    .unwrap_or("no text");
                Ok(format!("echo: {text}"))
            })
        }),
    }
}

pub(crate) fn make_error_tool() -> crate::tool_registry::RegisteredTool {
    use unified_llm::types::ToolDefinition;
    crate::tool_registry::RegisteredTool {
        definition: ToolDefinition {
            name: "fail_tool".into(),
            description: "Always fails".into(),
            parameters: serde_json::json!({"type": "object"}),
        },
        executor: Arc::new(|_args, _env| {
            Box::pin(async move { Err("tool execution failed".to_string()) })
        }),
    }
}

// --- ParallelTestProfile ---

pub(crate) struct ParallelTestProfile {
    pub registry: ToolRegistry,
    pub context_window: usize,
}

impl ParallelTestProfile {
    pub fn with_tools(registry: ToolRegistry) -> Self {
        Self {
            registry,
            context_window: 200_000,
        }
    }

    pub fn with_tools_and_context_window(registry: ToolRegistry, context_window: usize) -> Self {
        Self {
            registry,
            context_window,
        }
    }
}

impl ProviderProfile for ParallelTestProfile {
    fn id(&self) -> &str {
        "mock"
    }

    fn model(&self) -> &str {
        "mock-model"
    }

    fn tool_registry(&self) -> &ToolRegistry {
        &self.registry
    }

    fn tool_registry_mut(&mut self) -> &mut ToolRegistry {
        &mut self.registry
    }

    fn build_system_prompt(
        &self,
        _env: &dyn ExecutionEnvironment,
        _env_context: &EnvContext,
        _project_docs: &[String],
        _user_instructions: Option<&str>,
    ) -> String {
        "You are a test assistant.".into()
    }

    fn capabilities(&self) -> ProfileCapabilities {
        ProfileCapabilities {
            supports_reasoning: false,
            supports_streaming: false,
            supports_parallel_tool_calls: true,
            context_window_size: self.context_window,
        }
    }

    fn knowledge_cutoff(&self) -> &str {
        "May 2025"
    }
}

// --- MockErrorProvider ---

pub(crate) struct MockErrorProvider {
    pub error: SdkError,
}

#[async_trait]
impl ProviderAdapter for MockErrorProvider {
    fn name(&self) -> &str {
        "mock"
    }

    async fn complete(&self, _request: &Request) -> Result<Response, SdkError> {
        Err(self.error.clone())
    }

    async fn stream(&self, _request: &Request) -> Result<StreamEventStream, SdkError> {
        Err(SdkError::Configuration {
            message: "streaming not supported in mock".into(),
        })
    }
}

pub(crate) fn multi_tool_call_response(
    calls: Vec<(&str, &str, serde_json::Value)>,
) -> Response {
    use unified_llm::types::{ContentPart, Role, ToolCall};
    let mut content = vec![ContentPart::text("Let me use multiple tools.")];
    for (tool_name, tool_call_id, args) in &calls {
        content.push(ContentPart::ToolCall(ToolCall::new(
            *tool_call_id,
            *tool_name,
            args.clone(),
        )));
    }
    Response {
        id: "resp_multi".into(),
        model: "mock-model".into(),
        provider: "mock".into(),
        message: Message {
            role: Role::Assistant,
            content,
            name: None,
            tool_call_id: None,
        },
        finish_reason: FinishReason::ToolCalls,
        usage: Usage {
            input_tokens: 10,
            output_tokens: 5,
            total_tokens: 15,
            ..Default::default()
        },
        raw: None,
        warnings: vec![],
        rate_limit: None,
    }
}
