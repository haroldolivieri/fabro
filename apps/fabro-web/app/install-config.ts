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
      text: "Your Portkey gateway base URL. For the hosted Portkey service use https://api.portkey.ai/v1. For a self-hosted or enterprise gateway, use that URL with /v1 appended.",
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
      text: "Your Portkey API key. Find it in the Portkey dashboard under Settings → API Keys.",
      url: "https://app.portkey.ai/" as string | null,
      linkText: "app.portkey.ai" as string | null,
    },
  },
  {
    id: "provider" as const,
    label: "Provider",
    envVar: "PORTKEY_PROVIDER",
    required: true,
    isSecret: false,
    placeholder: "anthropic",
    help: {
      text: "The LLM adapter to use — determines the request format. Must match the upstream provider's API. Valid values: anthropic, openai, gemini, kimi, zai, minimax, inception. For AWS Bedrock (Claude models), use anthropic.",
      url: null as string | null,
      linkText: null as string | null,
    },
  },
  {
    id: "provider_slug" as const,
    label: "Provider Slug",
    envVar: "PORTKEY_PROVIDER_SLUG",
    required: false,
    isSecret: false,
    placeholder: "@bedrock-sandbox",
    help: {
      text: "Only needed when the Portkey routing target differs from the provider adapter — e.g. routing to Bedrock, Azure, or any custom Model Catalog provider. Leave blank for direct routing (e.g. Provider = anthropic routes straight to Anthropic). Set this to the slug of your configured Model Catalog provider (e.g. @bedrock-sandbox). Find your slugs in the Portkey Model Catalog.",
      url: "https://app.portkey.ai/model-catalog" as string | null,
      linkText: "app.portkey.ai/model-catalog" as string | null,
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
      text: "A Portkey config ID or inline JSON for advanced routing strategies: fallbacks, load balancing, or conditional routing across providers. Create configs in the Portkey dashboard. Leave blank for simple single-provider routing.",
      url: "https://app.portkey.ai/" as string | null,
      linkText: "app.portkey.ai → Configs" as string | null,
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
