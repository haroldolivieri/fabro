use crate::execution_env::ExecutionEnvironment;
use crate::profiles::EnvContext;
use crate::subagent::{
    make_close_agent_tool, make_send_input_tool, make_spawn_agent_tool, SessionFactory,
    SubAgentManager,
};
use crate::tool_registry::ToolRegistry;
use std::sync::Arc;
use unified_llm::types::ToolDefinition;

/// Static capabilities of a provider profile.
pub struct ProfileCapabilities {
    pub supports_reasoning: bool,
    pub supports_streaming: bool,
    pub supports_parallel_tool_calls: bool,
    pub context_window_size: usize,
}

pub trait ProviderProfile: Send + Sync {
    fn id(&self) -> &str;
    fn model(&self) -> &str;
    fn tool_registry(&self) -> &ToolRegistry;
    fn tool_registry_mut(&mut self) -> &mut ToolRegistry;
    fn build_system_prompt(
        &self,
        env: &dyn ExecutionEnvironment,
        env_context: &EnvContext,
        project_docs: &[String],
        user_instructions: Option<&str>,
    ) -> String;
    fn capabilities(&self) -> ProfileCapabilities;
    fn knowledge_cutoff(&self) -> &str;

    fn tools(&self) -> Vec<ToolDefinition> {
        self.tool_registry().definitions()
    }

    fn provider_options(&self) -> Option<serde_json::Value> {
        None
    }

    fn supports_reasoning(&self) -> bool {
        self.capabilities().supports_reasoning
    }

    fn supports_streaming(&self) -> bool {
        self.capabilities().supports_streaming
    }

    fn supports_parallel_tool_calls(&self) -> bool {
        self.capabilities().supports_parallel_tool_calls
    }

    fn context_window_size(&self) -> usize {
        self.capabilities().context_window_size
    }

    fn register_subagent_tools(
        &mut self,
        manager: Arc<tokio::sync::Mutex<SubAgentManager>>,
        session_factory: SessionFactory,
        current_depth: usize,
    ) {
        self.tool_registry_mut().register(make_spawn_agent_tool(
            manager.clone(),
            session_factory,
            current_depth,
        ));
        self.tool_registry_mut()
            .register(make_send_input_tool(manager.clone()));
        self.tool_registry_mut()
            .register(crate::subagent::make_wait_tool(manager.clone()));
        self.tool_registry_mut()
            .register(make_close_agent_tool(manager));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_env::ExecutionEnvironment;
    use crate::test_support::MockExecutionEnvironment;

    /// A specialized profile for provider_profile tests that uses distinct id/model
    /// and a custom build_system_prompt (unlike the shared TestProfile).
    struct ProviderTestProfile {
        registry: ToolRegistry,
    }

    impl ProviderTestProfile {
        fn new() -> Self {
            Self {
                registry: ToolRegistry::new(),
            }
        }
    }

    impl ProviderProfile for ProviderTestProfile {
        fn id(&self) -> &str {
            "test-provider"
        }
        fn model(&self) -> &str {
            "test-model"
        }
        fn tool_registry(&self) -> &ToolRegistry {
            &self.registry
        }
        fn tool_registry_mut(&mut self) -> &mut ToolRegistry {
            &mut self.registry
        }
        fn build_system_prompt(
            &self,
            env: &dyn ExecutionEnvironment,
            _env_context: &EnvContext,
            project_docs: &[String],
            user_instructions: Option<&str>,
        ) -> String {
            let base = format!(
                "You are working on {}. Docs: {}",
                env.platform(),
                project_docs.len()
            );
            match user_instructions {
                Some(instructions) => format!("{base}\n\n{instructions}"),
                None => base,
            }
        }
        fn capabilities(&self) -> ProfileCapabilities {
            ProfileCapabilities {
                supports_reasoning: true,
                supports_streaming: true,
                supports_parallel_tool_calls: false,
                context_window_size: 200_000,
            }
        }
        fn knowledge_cutoff(&self) -> &str {
            "May 2025"
        }
    }

    #[test]
    fn profile_id_and_model() {
        let profile = ProviderTestProfile::new();
        assert_eq!(profile.id(), "test-provider");
        assert_eq!(profile.model(), "test-model");
    }

    #[test]
    fn profile_capabilities() {
        let profile = ProviderTestProfile::new();
        assert!(profile.supports_reasoning());
        assert!(profile.supports_streaming());
        assert!(!profile.supports_parallel_tool_calls());
        assert_eq!(profile.context_window_size(), 200_000);
    }

    #[test]
    fn profile_build_system_prompt() {
        let profile = ProviderTestProfile::new();
        let env = MockExecutionEnvironment {
            working_dir: "/home/test",
            platform_str: "linux",
            os_version_str: "Linux 6.1.0".into(),
            ..Default::default()
        };
        let ctx = EnvContext::default();
        let docs = vec!["README.md contents".into()];
        let prompt = profile.build_system_prompt(&env, &ctx, &docs, None);
        assert!(prompt.contains("linux"));
        assert!(prompt.contains("1"));
    }

    #[test]
    fn profile_build_system_prompt_with_user_instructions() {
        let profile = ProviderTestProfile::new();
        let env = MockExecutionEnvironment::default();
        let ctx = EnvContext::default();
        let prompt = profile.build_system_prompt(&env, &ctx, &[], Some("Always use TDD"));
        assert!(prompt.contains("Always use TDD"));
    }

    #[test]
    fn profile_provider_options_none() {
        let profile = ProviderTestProfile::new();
        assert!(profile.provider_options().is_none());
    }

    #[test]
    fn profile_tools_empty_registry() {
        let profile = ProviderTestProfile::new();
        assert!(profile.tools().is_empty());
    }
}
