export const INSTALL_PROVIDERS = [
  {
    id: "anthropic",
    label: "Anthropic",
    envVar: "ANTHROPIC_API_KEY",
    keyHelp: {
      url: "https://console.anthropic.com/settings/keys",
      text: "Create one in the Anthropic Console under Settings → API keys.",
    },
  },
  {
    id: "openai",
    label: "OpenAI",
    envVar: "OPENAI_API_KEY",
    keyHelp: {
      url: "https://platform.openai.com/api-keys",
      text: "Create one in the OpenAI platform under API keys.",
    },
  },
  {
    id: "gemini",
    label: "Gemini",
    envVar: "GEMINI_API_KEY",
    keyHelp: {
      url: "https://aistudio.google.com/apikey",
      text: "Create one in Google AI Studio.",
    },
  },
] as const;

export const PORTKEY_FIELDS = [
  {
    id: "url" as const,
    label: "Gateway URL",
    envVar: "PORTKEY_URL",
    required: true,
    isSecret: false,
    placeholder: "https://api.portkey.ai/v1",
    help: {
      text: "For most users this is https://api.portkey.ai/v1 — Portkey's hosted service. You'd only need a different URL if you're using a self-hosted or enterprise gateway.",
      url: "https://portkey.ai/docs" as string | null,
      linkText: "portkey.ai/docs" as string | null,
    },
  },
  {
    id: "api_key" as const,
    label: "API Key",
    envVar: "PORTKEY_API_KEY",
    required: true,
    isSecret: true,
    placeholder: "PORTKEY_API_KEY",
    help: {
      text: "Your Portkey API key. Create or copy one from the Portkey API Keys page.",
      url: "https://app.portkey.ai/api-keys" as string | null,
      linkText: "app.portkey.ai/api-keys" as string | null,
    },
  },
  {
    id: "provider_slug" as const,
    label: "Provider Slug",
    envVar: "PORTKEY_PROVIDER_SLUG",
    required: true,
    isSecret: false,
    placeholder: "@openai-prod",
    help: {
      text: "The slug of your configured Portkey Model Catalog provider (e.g. @openai-prod, @bedrock-sandbox). Required when using Model Catalog routing. Optional when using a Config — only needed if the config uses passthrough targets that defer provider selection to the request.",
      url: "https://app.portkey.ai/model-catalog" as string | null,
      linkText: "app.portkey.ai/model-catalog" as string | null,
    },
  },
  {
    id: "provider" as const,
    label: "Provider",
    envVar: "PORTKEY_PROVIDER",
    required: false,
    isSecret: false,
    placeholder: "anthropic",
    help: {
      text: "Optional. Set this to use the provider's native API format and unlock provider-specific features such as Anthropic prompt caching and extended thinking. When left blank, requests use OpenAI-compatible format, which works universally but does not support native features. Valid values: anthropic, openai, gemini, kimi, zai, minimax, inception.",
      url: null as string | null,
      linkText: null as string | null,
    },
  },
  {
    id: "config" as const,
    label: "Config",
    envVar: "PORTKEY_CONFIG",
    required: false,
    isSecret: false,
    placeholder: "cfg-xxxx",
    help: {
      text: "A Portkey config ID for advanced routing strategies: fallbacks, load balancing, or conditional routing across providers. Find or create configs in the Portkey Configs page.",
      url: "https://app.portkey.ai/configs" as string | null,
      linkText: "app.portkey.ai/configs" as string | null,
    },
  },
] as const;

export type PortkeyFieldId = (typeof PORTKEY_FIELDS)[number]["id"];

export type PortkeySelection = Record<PortkeyFieldId, string>;

export function defaultPortkeySelection(): PortkeySelection {
  return { url: "", api_key: "", provider: "", provider_slug: "", config: "" };
}

export const PORTKEY_ENV_ONLY_FIELDS = [
  {
    envVar: "PORTKEY_METADATA",
    description:
      'JSON metadata attached to every request for Portkey observability (e.g. {"team":"eng"}). Set via environment variable only.',
  },
  {
    envVar: "PORTKEY_AWS_ACCESS_KEY_ID / PORTKEY_AWS_SECRET_ACCESS_KEY / PORTKEY_AWS_REGION",
    description:
      "Direct AWS credentials for Bedrock access without a Portkey Model Catalog provider. Set via environment variables only (see docs).",
  },
] as const;
