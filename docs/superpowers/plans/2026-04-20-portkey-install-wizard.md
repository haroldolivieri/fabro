# Portkey Install Wizard Integration — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a collapsible Portkey AI Gateway section to the install wizard's LLM credentials step, storing Portkey config in the vault as environment secrets and wiring it into the server's LLM client initialisation path.

**Architecture:** Portkey variables are stored as `VaultSecretType::Environment` entries (matching how `GITHUB_TOKEN` is stored). `PortkeyConfig` gains a `from_lookup()` method that accepts any `Fn(&str) -> Option<String>`, allowing both `from_env()` (CLI) and vault-aware lookup (server). The server's `build_llm_client()` feeds this lookup with a closure that checks env then vault.

**Tech Stack:** Rust (`fabro-llm`, `fabro-server`), TypeScript/React (`apps/fabro-web`), OpenAPI YAML spec

---

## File Map

| Action | File | Responsibility |
|--------|------|----------------|
| Modify | `lib/crates/fabro-llm/src/portkey.rs` | Add `from_lookup()`, refactor `from_env()` to delegate to it |
| Modify | `lib/crates/fabro-server/src/server_secrets.rs` | Call `PortkeyConfig::from_lookup()` with vault-aware lookup in `build_llm_client()` |
| Modify | `docs/api-reference/fabro-api.yaml` | Add `InstallPortkeyInput` schema; extend `InstallLlmProvidersInput`; relax `minItems` |
| Modify | `lib/crates/fabro-server/src/install.rs` | Add `PortkeyInstallInput` struct; extend `LlmProvidersInput`; update `put_install_llm`, `post_install_finish`, validation |
| Modify | `apps/fabro-web/app/install-config.ts` | Add `PORTKEY_FIELDS` config |
| Modify | `apps/fabro-web/app/install-api.ts` | Extend `putInstallLlm()` to include portkey data |
| Modify | `apps/fabro-web/app/install-app.tsx` | Add `PortkeySection` component; update LLM step state, validation, submit |
| Run | `cd lib/packages/fabro-api-client && bun run generate` | Regenerate TypeScript API client after spec changes |

---

### Task 1: `PortkeyConfig::from_lookup()` — vault-aware config loading

**Files:**
- Modify: `lib/crates/fabro-llm/src/portkey.rs`

- [ ] **Step 1: Write failing tests for `from_lookup()`**

Add to `mod tests` in `portkey.rs`, after the existing tests:

```rust
    #[test]
    fn from_lookup_returns_none_when_url_missing() {
        let lookup = |k: &str| match k {
            "PORTKEY_API_KEY" => Some("pk-test".to_string()),
            "PORTKEY_PROVIDER" => Some("anthropic".to_string()),
            _ => None,
        };
        assert!(PortkeyConfig::from_lookup(lookup).is_none());
    }

    #[test]
    fn from_lookup_parses_all_required_fields() {
        let lookup = |k: &str| match k {
            "PORTKEY_URL"      => Some("https://api.portkey.ai/v1".to_string()),
            "PORTKEY_API_KEY"  => Some("pk-test".to_string()),
            "PORTKEY_PROVIDER" => Some("anthropic".to_string()),
            _                  => None,
        };
        let config = PortkeyConfig::from_lookup(lookup).unwrap();
        assert_eq!(config.base_url, "https://api.portkey.ai/v1");
        assert_eq!(config.api_key, "pk-test");
        assert_eq!(config.provider, Provider::Anthropic);
        assert!(config.provider_slug.is_none());
        assert!(config.aws.is_none());
    }

    #[test]
    fn from_lookup_parses_optional_fields() {
        let lookup = |k: &str| match k {
            "PORTKEY_URL"                    => Some("https://api.portkey.ai/v1".to_string()),
            "PORTKEY_API_KEY"                => Some("pk-key".to_string()),
            "PORTKEY_PROVIDER"               => Some("anthropic".to_string()),
            "PORTKEY_PROVIDER_SLUG"          => Some("@bedrock-sandbox".to_string()),
            "PORTKEY_CONFIG"                 => Some("cfg-abc".to_string()),
            "PORTKEY_METADATA"               => Some(r#"{"team":"eng"}"#.to_string()),
            "PORTKEY_AWS_ACCESS_KEY_ID"      => Some("AKIA...".to_string()),
            "PORTKEY_AWS_SECRET_ACCESS_KEY"  => Some("secret".to_string()),
            "PORTKEY_AWS_REGION"             => Some("eu-west-1".to_string()),
            _                                => None,
        };
        let config = PortkeyConfig::from_lookup(lookup).unwrap();
        assert_eq!(config.provider_slug.as_deref(), Some("@bedrock-sandbox"));
        assert_eq!(config.config.as_deref(), Some("cfg-abc"));
        assert_eq!(config.aws.as_ref().unwrap().region, "eu-west-1");
    }

    #[test]
    fn from_env_delegates_to_from_lookup() {
        clear_portkey_env();
        std::env::set_var("PORTKEY_URL", "https://api.portkey.ai/v1");
        std::env::set_var("PORTKEY_API_KEY", "pk-env");
        std::env::set_var("PORTKEY_PROVIDER", "openai");
        let config = PortkeyConfig::from_env().unwrap();
        assert_eq!(config.api_key, "pk-env");
        assert_eq!(config.provider, Provider::OpenAi);
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Add `pub mod portkey;` temporarily to `lib/crates/fabro-llm/src/lib.rs` if not already there, then:

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-llm -- from_lookup`
Expected: Compilation error — `from_lookup` not defined.

- [ ] **Step 3: Extract `from_lookup()` and refactor `from_env()` to delegate**

Replace the `from_env()` implementation in `portkey.rs` with:

```rust
    /// Load `PortkeyConfig` using a custom key-lookup function.
    ///
    /// `lookup` is called with env var names and returns `Some(value)` when
    /// found. Use this to read from vault, injected test maps, or any other
    /// source. [`PortkeyConfig::from_env`] delegates here with `std::env::var`.
    ///
    /// Returns `None` if any required variable is missing or invalid.
    #[must_use]
    pub fn from_lookup(lookup: impl Fn(&str) -> Option<String>) -> Option<Self> {
        let base_url = lookup("PORTKEY_URL")?;
        let api_key  = lookup("PORTKEY_API_KEY")?;

        let provider_str = lookup("PORTKEY_PROVIDER")?;
        let provider = match Provider::from_str(&provider_str) {
            Ok(p) => p,
            Err(e) => {
                warn!(value = %provider_str, error = %e, "PORTKEY_PROVIDER is not a valid provider");
                return None;
            }
        };

        let provider_slug = lookup("PORTKEY_PROVIDER_SLUG");
        let config        = lookup("PORTKEY_CONFIG");
        let metadata      = lookup("PORTKEY_METADATA");

        let aws = match (
            lookup("PORTKEY_AWS_ACCESS_KEY_ID"),
            lookup("PORTKEY_AWS_SECRET_ACCESS_KEY"),
        ) {
            (Some(access_key_id), Some(secret_access_key)) => {
                let region        = lookup("PORTKEY_AWS_REGION").unwrap_or_else(|| "us-east-1".to_string());
                let session_token = lookup("PORTKEY_AWS_SESSION_TOKEN");
                Some(AwsCredentials { access_key_id, secret_access_key, region, session_token })
            }
            _ => None,
        };

        Some(Self { base_url, api_key, provider, provider_slug, config, metadata, aws })
    }

    /// Load `PortkeyConfig` from environment variables.
    ///
    /// Delegates to [`PortkeyConfig::from_lookup`] with `std::env::var`.
    #[must_use]
    pub fn from_env() -> Option<Self> {
        Self::from_lookup(|k| std::env::var(k).ok())
    }
```

- [ ] **Step 4: Run all portkey tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-llm -- portkey`
Expected: All tests pass (including the 4 new `from_lookup` tests).

- [ ] **Step 5: Commit**

```bash
git add lib/crates/fabro-llm/src/portkey.rs
git commit -m "feat(fabro-llm): add PortkeyConfig::from_lookup() for vault-aware config loading"
```

---

### Task 2: Wire vault-aware Portkey config into `build_llm_client()`

**Files:**
- Modify: `lib/crates/fabro-server/src/server_secrets.rs`

- [ ] **Step 1: Write failing test**

Add to `mod tests` in `server_secrets.rs`:

```rust
    #[tokio::test]
    async fn build_llm_client_applies_portkey_from_vault() {
        let dir = tempfile::tempdir().unwrap();
        let mut vault = Vault::load(dir.path().join("secrets.json")).unwrap();
        vault
            .set("PORTKEY_URL", "https://api.portkey.ai/v1", SecretType::Environment, None)
            .unwrap();
        vault
            .set("PORTKEY_API_KEY", "pk-vault-key", SecretType::Environment, None)
            .unwrap();
        vault
            .set("PORTKEY_PROVIDER", "anthropic", SecretType::Environment, None)
            .unwrap();

        let credentials =
            ProviderCredentials::with_env_lookup(Arc::new(AsyncRwLock::new(vault)), |_| None);

        // build_llm_client should not error even with no real provider API keys —
        // PortkeyConfig creates a dummy credential when portkey vars are set.
        let result = credentials.build_llm_client().await;
        assert!(result.is_ok(), "build_llm_client failed: {:?}", result.err());
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-server -- build_llm_client_applies_portkey`
Expected: Test compiles but FAILS — client builds with zero providers (portkey vars not applied).

- [ ] **Step 3: Update `build_llm_client()` to apply Portkey from vault**

In `lib/crates/fabro-server/src/server_secrets.rs`, add the import at the top:

```rust
use fabro_llm::portkey::PortkeyConfig;
```

Replace `build_llm_client()` with:

```rust
    pub(crate) async fn build_llm_client(&self) -> Result<LlmClientResult, String> {
        let resolver =
            CredentialResolver::with_env_lookup(Arc::clone(&self.vault), self.env_lookup.clone());
        let mut api_credentials = Vec::new();
        let mut auth_issues = Vec::new();

        for provider in Provider::ALL {
            match resolver
                .resolve(*provider, CredentialUsage::ApiRequest)
                .await
            {
                Ok(ResolvedCredential::Api(credential)) => api_credentials.push(credential),
                Ok(ResolvedCredential::Cli(_)) | Err(ResolveError::NotConfigured(_)) => {}
                Err(err) => auth_issues.push((*provider, err)),
            }
        }

        // Apply Portkey gateway config if configured — reads from env first,
        // then falls back to vault Environment secrets.
        {
            let vault = self.vault.read().await;
            let env_lookup = Arc::clone(&self.env_lookup);
            if let Some(portkey) =
                PortkeyConfig::from_lookup(|k| env_lookup(k).or_else(|| vault.get(k).map(str::to_string)))
            {
                portkey.apply(&mut api_credentials);
            }
        }

        let client = LlmClient::from_credentials(api_credentials)
            .await
            .map_err(|err| err.to_string())?;

        Ok(LlmClientResult {
            client,
            auth_issues,
        })
    }
```

- [ ] **Step 4: Run test to verify it passes**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-server -- build_llm_client_applies_portkey`
Expected: PASS.

- [ ] **Step 5: Run full server tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-server`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/fabro-server/src/server_secrets.rs
git commit -m "feat(fabro-server): apply Portkey config from vault in build_llm_client()"
```

---

### Task 3: OpenAPI spec — add `InstallPortkeyInput` schema

**Files:**
- Modify: `docs/api-reference/fabro-api.yaml`

- [ ] **Step 1: Add `InstallPortkeyInput` schema**

Find the `InstallLlmProvidersInput` schema (line 2206) and make these two changes:

**Change 1** — relax `providers` to allow 0 items and add optional `portkey` field:

```yaml
    InstallLlmProvidersInput:
      description: LLM providers selected during browser install. Provide direct provider keys, a Portkey gateway config, or both.
      type: object
      required:
        - providers
      properties:
        providers:
          type: array
          minItems: 0
          items:
            $ref: "#/components/schemas/InstallLlmProviderInput"
        portkey:
          description: Optional Portkey AI gateway configuration. When provided, all LLM traffic routes through the Portkey gateway.
          $ref: "#/components/schemas/InstallPortkeyInput"
```

**Change 2** — add `InstallPortkeyInput` schema after `InstallLlmProviderInput` (around line 2230):

```yaml
    InstallPortkeyInput:
      description: Portkey AI gateway configuration collected during browser install.
      type: object
      required:
        - url
        - api_key
        - provider
      properties:
        url:
          type: string
          description: Portkey gateway base URL (e.g. https://api.portkey.ai/v1).
          example: https://api.portkey.ai/v1
        api_key:
          type: string
          description: Portkey API key.
        provider:
          type: string
          description: "Provider adapter to use. One of: anthropic, openai, gemini, kimi, zai, minimax, inception."
          example: anthropic
        provider_slug:
          type: string
          description: "Portkey routing target (e.g. @bedrock-sandbox). Defaults to provider when omitted."
          example: "@bedrock-sandbox"
        config:
          type: string
          description: Portkey config ID or inline JSON for advanced routing (fallbacks, load balancing).
```

- [ ] **Step 2: Regenerate Rust types**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo build -p fabro-api`
Expected: Compiles cleanly — progenitor regenerates Rust types from spec.

- [ ] **Step 3: Regenerate TypeScript client**

Run: `cd lib/packages/fabro-api-client && bun run generate`
Expected: TypeScript client regenerated. `InstallPortkeyInput` type now available.

- [ ] **Step 4: Commit**

```bash
git add docs/api-reference/fabro-api.yaml lib/packages/fabro-api-client/
git commit -m "feat(api): add InstallPortkeyInput schema to install LLM endpoint"
```

---

### Task 4: Server — accept and persist Portkey install config

**Files:**
- Modify: `lib/crates/fabro-server/src/install.rs`

- [ ] **Step 1: Add `PortkeyInstallInput` struct and extend `LlmProvidersInput`**

Find the structs around line 169-178 and add/update:

```rust
#[derive(Clone, Debug, Deserialize, Serialize)]
struct LlmProvidersInput {
    providers: Vec<LlmProviderInput>,
    portkey:   Option<PortkeyInstallInput>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
struct PortkeyInstallInput {
    url:           String,
    api_key:       String,
    provider:      String,
    provider_slug: Option<String>,
    config:        Option<String>,
}
```

- [ ] **Step 2: Update `put_install_llm` validation**

Replace the current validation in `put_install_llm` (lines 507-525) with:

```rust
    // At least one provider key OR a portkey config is required.
    if input.providers.is_empty() && input.portkey.is_none() {
        return install_error_response(
            StatusCode::UNPROCESSABLE_ENTITY,
            "add at least one LLM provider key or configure Portkey",
        );
    }

    for provider in &input.providers {
        if let Some(error) = unsupported_install_provider_error(provider.provider) {
            return install_error_response(StatusCode::UNPROCESSABLE_ENTITY, error);
        }
        if provider.api_key.trim().is_empty() {
            return install_error_response(
                StatusCode::UNPROCESSABLE_ENTITY,
                format!("api_key is required for {}", provider.provider.as_str()),
            );
        }
    }

    if let Some(portkey) = &input.portkey {
        if portkey.url.trim().is_empty() {
            return install_error_response(StatusCode::UNPROCESSABLE_ENTITY, "portkey url is required");
        }
        if portkey.api_key.trim().is_empty() {
            return install_error_response(StatusCode::UNPROCESSABLE_ENTITY, "portkey api_key is required");
        }
        if portkey.provider.trim().is_empty() {
            return install_error_response(StatusCode::UNPROCESSABLE_ENTITY, "portkey provider is required");
        }
    }
```

- [ ] **Step 3: Persist Portkey vars in `post_install_finish`**

After the `for provider in llm.providers` loop (after line 815), add:

```rust
    if let Some(portkey) = llm.portkey {
        for (name, value) in [
            ("PORTKEY_URL",      portkey.url),
            ("PORTKEY_API_KEY",  portkey.api_key),
            ("PORTKEY_PROVIDER", portkey.provider),
        ] {
            vault_secrets.push(VaultSecretWrite {
                name:        name.to_string(),
                value,
                secret_type: VaultSecretType::Environment,
                description: None,
            });
        }
        if let Some(slug) = portkey.provider_slug {
            vault_secrets.push(VaultSecretWrite {
                name:        "PORTKEY_PROVIDER_SLUG".to_string(),
                value:       slug,
                secret_type: VaultSecretType::Environment,
                description: None,
            });
        }
        if let Some(config) = portkey.config {
            vault_secrets.push(VaultSecretWrite {
                name:        "PORTKEY_CONFIG".to_string(),
                value:       config,
                secret_type: VaultSecretType::Environment,
                description: None,
            });
        }
    }
```

- [ ] **Step 4: Build to verify**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo build -p fabro-server`
Expected: Clean build.

- [ ] **Step 5: Run server tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run -p fabro-server`
Expected: All pass.

- [ ] **Step 6: Commit**

```bash
git add lib/crates/fabro-server/src/install.rs
git commit -m "feat(fabro-server): accept and persist Portkey config in install flow"
```

---

### Task 5: `install-config.ts` — Portkey fields config

**Files:**
- Modify: `apps/fabro-web/app/install-config.ts`

- [ ] **Step 1: Add `PORTKEY_FIELDS` config**

Append to `install-config.ts`:

```typescript
export const PORTKEY_FIELDS = [
  {
    id: "url" as const,
    label: "Gateway URL",
    envVar: "PORTKEY_URL",
    required: true,
    isSecret: false,
    placeholder: "https://api.portkey.ai/v1",
    help: {
      text: "Your Portkey gateway base URL. For the hosted Portkey service use https://api.portkey.ai/v1. For a self-hosted or enterprise gateway (e.g. internal Skyscanner instance), use that URL with /v1 appended.",
      url: "https://portkey.ai/docs",
      linkText: "portkey.ai/docs",
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
      url: "https://app.portkey.ai/",
      linkText: "app.portkey.ai",
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
      url: null,
      linkText: null,
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
      text: "The Portkey routing target sent as x-portkey-provider. Leave blank for direct routing (e.g. Provider = anthropic → routes straight to Anthropic). Set this when the target differs from the adapter — e.g. @bedrock-sandbox for AWS Bedrock via Portkey Model Catalog, or @azure-prod for Azure OpenAI. This value is the slug you configured in your Portkey dashboard.",
      url: "https://portkey.ai/docs/product/ai-gateway/virtual-keys",
      linkText: "portkey.ai/docs → Model Catalog",
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
      url: "https://app.portkey.ai/",
      linkText: "app.portkey.ai → Configs",
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
    description: 'JSON metadata attached to every request for Portkey observability (e.g. {"team":"eng"}). Set via environment variable only.',
  },
  {
    envVar: "PORTKEY_AWS_ACCESS_KEY_ID / PORTKEY_AWS_SECRET_ACCESS_KEY / PORTKEY_AWS_REGION",
    description: "Direct AWS credentials for Bedrock access without a Portkey Model Catalog provider. Set via environment variables only (see docs).",
  },
] as const;
```

- [ ] **Step 2: Commit**

```bash
git add apps/fabro-web/app/install-config.ts
git commit -m "feat(install-config): add PORTKEY_FIELDS config for install wizard"
```

---

### Task 6: `install-api.ts` — extend `putInstallLlm` to include portkey

**Files:**
- Modify: `apps/fabro-web/app/install-api.ts`

- [ ] **Step 1: Add `PortkeyInstallData` type and update `putInstallLlm`**

Add the type and update the function in `install-api.ts`:

```typescript
export type PortkeyInstallData = {
  url: string;
  api_key: string;
  provider: string;
  provider_slug?: string;
  config?: string;
};

export async function putInstallLlm(
  token: string,
  providers: InstallLlmProviderInput[],
  portkey?: PortkeyInstallData,
): Promise<void> {
  const response = await installFetch("/install/llm", token, {
    method: "PUT",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ providers, ...(portkey ? { portkey } : {}) }),
  });
  if (!response.ok) {
    throw new Error(await readInstallError(response, "install llm request failed"));
  }
}
```

- [ ] **Step 2: Commit**

```bash
git add apps/fabro-web/app/install-api.ts
git commit -m "feat(install-api): extend putInstallLlm to accept optional portkey config"
```

---

### Task 7: `install-app.tsx` — `PortkeySection` component and LLM step update

**Files:**
- Modify: `apps/fabro-web/app/install-app.tsx`

- [ ] **Step 1: Add `portkeySelection` state**

In `InstallApp`, after the `llmSelection` state (line 89), add:

```typescript
  const [portkeySelection, setPortkeySelection] = useState<PortkeySelection>(
    () => defaultPortkeySelection(),
  );
```

Also update the import at the top to include the new config exports:

```typescript
import {
  INSTALL_PROVIDERS,
  PORTKEY_FIELDS,
  PORTKEY_ENV_ONLY_FIELDS,
  defaultPortkeySelection,
  type PortkeySelection,
} from "./install-config";
```

And update the `install-api.ts` import to include `PortkeyInstallData` and `putInstallLlm`:

```typescript
import {
  // ... existing imports ...
  putInstallLlm,
  type PortkeyInstallData,
} from "./install-api";
```

- [ ] **Step 2: Update LLM step submit logic**

Replace the `onSubmit` handler for `/install/llm` (lines 300-330). The new version builds `portkey` from `portkeySelection` if the required fields are filled, and allows submission with no provider keys when portkey is configured:

```typescript
            onSubmit={async () => {
              const providers = INSTALL_PROVIDERS.map(({ id }) => {
                const current = llmSelection[id] ?? { apiKey: "" };
                return { provider: id, api_key: current.apiKey.trim() };
              }).filter((provider) => provider.api_key.length > 0);

              const portkey: PortkeyInstallData | undefined =
                portkeySelection.url.trim() &&
                portkeySelection.api_key.trim() &&
                portkeySelection.provider.trim()
                  ? {
                      url:           portkeySelection.url.trim(),
                      api_key:       portkeySelection.api_key.trim(),
                      provider:      portkeySelection.provider.trim(),
                      ...(portkeySelection.provider_slug.trim()
                        ? { provider_slug: portkeySelection.provider_slug.trim() }
                        : {}),
                      ...(portkeySelection.config.trim()
                        ? { config: portkeySelection.config.trim() }
                        : {}),
                    }
                  : undefined;

              if (providers.length === 0 && !portkey) {
                setSaveError(
                  "Add at least one provider API key or configure Portkey before continuing.",
                );
                return;
              }

              setSubmitting(true);
              setSaveError(null);
              try {
                await Promise.all(
                  providers.map((provider) => testInstallLlm(installToken, provider)),
                );
                await putInstallLlm(installToken, providers, portkey);
                const nextSession = await getInstallSession(installToken);
                setSessionState({ status: "ready", data: nextSession });
                navigate("/install/github");
              } catch (error) {
                setSaveError(
                  error instanceof Error ? error.message : "Failed to save LLM settings.",
                );
              } finally {
                setSubmitting(false);
              }
            }}
```

- [ ] **Step 3: Add `PortkeySection` component**

Add the `PortkeySection` component after the `ProviderFields` function (after line 1036):

```typescript
function PortkeySection({
  value,
  onChange,
}: {
  value: PortkeySelection;
  onChange: (next: PortkeySelection) => void;
}) {
  const requiredFields = PORTKEY_FIELDS.filter((f) => f.required);
  const optionalFields = PORTKEY_FIELDS.filter((f) => !f.required);

  return (
    <details className="group rounded-lg border border-white/10 bg-panel">
      <summary className="flex cursor-pointer select-none list-none items-center justify-between gap-3 px-4 py-3 text-sm font-medium text-fg [&::-webkit-details-marker]:hidden">
        <span>Portkey AI Gateway</span>
        <ChevronDownIcon className="size-4 shrink-0 text-fg-3 transition-transform group-open:-rotate-180" />
      </summary>

      <div className="space-y-6 border-t border-white/10 px-4 py-4">
        <p className="text-xs/5 text-fg-3">
          Route all LLM traffic through{" "}
          <a
            href="https://portkey.ai"
            target="_blank"
            rel="noopener noreferrer"
            className="text-teal-300 hover:text-teal-500"
          >
            Portkey
          </a>{" "}
          for observability, cost tracking, and provider routing (e.g. AWS Bedrock, Azure OpenAI).
          Filling the three required fields below replaces the need for direct provider API keys
          above.
        </p>

        {requiredFields.map((field) => (
          <div key={field.id}>
            <label htmlFor={`portkey_${field.id}`} className="text-sm font-medium text-fg">
              {field.label}
              <span className="ml-1 text-xs text-fg-3">(required)</span>
            </label>
            <div className="mt-2">
              {field.isSecret ? (
                <PasswordInput
                  id={`portkey_${field.id}`}
                  name={`portkey_${field.id}`}
                  value={value[field.id]}
                  onChange={(next) => onChange({ ...value, [field.id]: next })}
                  placeholder={field.placeholder}
                />
              ) : (
                <input
                  type="text"
                  id={`portkey_${field.id}`}
                  name={`portkey_${field.id}`}
                  value={value[field.id]}
                  onChange={(e) => onChange({ ...value, [field.id]: e.target.value })}
                  className={INPUT_CLASS + " font-mono"}
                  placeholder={field.placeholder}
                  spellCheck={false}
                  autoComplete="off"
                />
              )}
            </div>
            <HelpDisclosure summary="Where do I get this?">
              <p>{field.help.text}</p>
              {field.help.url && field.help.linkText && (
                <ExternalLink href={field.help.url}>{field.help.linkText}</ExternalLink>
              )}
            </HelpDisclosure>
          </div>
        ))}

        {optionalFields.length > 0 && (
          <details className="group/optional">
            <summary className="inline-flex cursor-pointer select-none list-none items-center gap-1 text-xs text-fg-3 hover:text-fg-2 [&::-webkit-details-marker]:hidden">
              <ChevronDownIcon className="size-3.5 shrink-0 transition-transform group-open/optional:-rotate-180" />
              <span>Advanced routing</span>
            </summary>
            <div className="mt-4 space-y-6">
              {optionalFields.map((field) => (
                <div key={field.id}>
                  <label
                    htmlFor={`portkey_${field.id}`}
                    className="text-sm font-medium text-fg"
                  >
                    {field.label}
                    <span className="ml-1 text-xs text-fg-3">(optional)</span>
                  </label>
                  <div className="mt-2">
                    <input
                      type="text"
                      id={`portkey_${field.id}`}
                      name={`portkey_${field.id}`}
                      value={value[field.id]}
                      onChange={(e) => onChange({ ...value, [field.id]: e.target.value })}
                      className={INPUT_CLASS + " font-mono"}
                      placeholder={field.placeholder}
                      spellCheck={false}
                      autoComplete="off"
                    />
                  </div>
                  <HelpDisclosure summary="Where do I get this?">
                    <p>{field.help.text}</p>
                    {field.help.url && field.help.linkText && (
                      <ExternalLink href={field.help.url}>{field.help.linkText}</ExternalLink>
                    )}
                  </HelpDisclosure>
                </div>
              ))}
            </div>
          </details>
        )}

        <details className="group/env">
          <summary className="inline-flex cursor-pointer select-none list-none items-center gap-1 text-xs text-fg-3 hover:text-fg-2 [&::-webkit-details-marker]:hidden">
            <ChevronDownIcon className="size-3.5 shrink-0 transition-transform group-open/env:-rotate-180" />
            <span>Environment-variable-only settings</span>
          </summary>
          <div className="mt-3 space-y-3">
            {PORTKEY_ENV_ONLY_FIELDS.map((field) => (
              <div key={field.envVar} className="rounded-md bg-panel-alt px-3 py-2">
                <p className="font-mono text-xs text-teal-300">{field.envVar}</p>
                <p className="mt-1 text-xs/5 text-fg-3">{field.description}</p>
              </div>
            ))}
          </div>
        </details>
      </div>
    </details>
  );
}
```

- [ ] **Step 4: Render `PortkeySection` in the LLM step**

Find the line `<ProviderFields value={llmSelection} onChange={setLlmSelection} />` (line 333) and replace with:

```typescript
          <div className="space-y-8">
            <ProviderFields value={llmSelection} onChange={setLlmSelection} />
            <PortkeySection value={portkeySelection} onChange={setPortkeySelection} />
          </div>
```

- [ ] **Step 5: Ensure `INPUT_CLASS` is imported**

`INPUT_CLASS` is already imported at the top of `install-app.tsx` from `./components/ui`:

```typescript
import {
  INPUT_CLASS,
  // ... other ui exports
} from "./components/ui";
```

No changes needed — the `PortkeySection` component uses it directly.

- [ ] **Step 6: TypeScript typecheck**

Run: `cd apps/fabro-web && bun run typecheck`
Expected: No type errors.

- [ ] **Step 7: Commit**

```bash
git add apps/fabro-web/app/install-app.tsx
git commit -m "feat(install-app): add PortkeySection to LLM credentials step"
```

---

### Task 8: Rebuild SPA bundle and final verification

**Files:**
- Run: `scripts/refresh-fabro-spa.sh`
- Run: full build + test

- [ ] **Step 1: Rebuild embedded SPA**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && scripts/refresh-fabro-spa.sh`
Expected: New SPA bundle copied to `lib/crates/fabro-spa/assets/`.

- [ ] **Step 2: Full workspace build**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo build --workspace`
Expected: Clean build.

- [ ] **Step 3: Run all tests**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo nextest run --workspace`
Expected: All pass.

- [ ] **Step 4: Run clippy**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo +nightly-2026-04-14 clippy --workspace --all-targets -- -D warnings`
Expected: No warnings.

- [ ] **Step 5: Run fmt check**

Run: `export PATH="$HOME/.cargo/bin:$PATH" && cargo +nightly-2026-04-14 fmt --check --all`
Expected: No formatting issues. If there are, run `cargo +nightly-2026-04-14 fmt --all` first.

- [ ] **Step 6: Commit bundle and any lint fixes**

```bash
git add lib/crates/fabro-spa/assets/ lib/packages/fabro-api-client/
git commit -m "chore: rebuild SPA bundle with Portkey install wizard section"
```

- [ ] **Step 7: Manual smoke test**

```bash
export PATH="$HOME/.cargo/bin:$PATH"
set -a && source .env && set +a
cargo build -p fabro-cli
./target/debug/fabro server start --foreground &
sleep 2
# Open the install URL shown in server output
# Navigate to the LLM step
# Verify the Portkey section appears below the provider fields
# Fill in URL, API key, Provider — verify it saves without provider keys
```
