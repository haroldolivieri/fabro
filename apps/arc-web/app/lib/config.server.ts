import { readFileSync } from "node:fs";
import { homedir } from "node:os";
import { join } from "node:path";
import { parse } from "smol-toml";

interface AuthConfig {
  provider: "github" | "insecure_disabled";
  allowed_usernames: string[];
}

interface ApiConfig {
  base_url: string;
  authentication_strategy: "jwt" | "insecure_disabled";
}

interface GitConfig {
  provider: "github";
  app_id: string | null;
  client_id: string | null;
}

export interface AppConfig {
  auth: AuthConfig;
  api: ApiConfig;
  git: GitConfig;
}

const AUTH_DEFAULTS: AuthConfig = {
  provider: "github",
  allowed_usernames: [],
};

const API_DEFAULTS: ApiConfig = {
  base_url: "http://localhost:3000",
  authentication_strategy: "jwt",
};

const GIT_DEFAULTS: GitConfig = {
  provider: "github",
  app_id: null,
  client_id: null,
};

export const ARC_CONFIG_PATH = join(homedir(), ".arc", "arc.toml");

function loadAppConfig(): AppConfig {
  const configPath = ARC_CONFIG_PATH;

  let raw: Record<string, unknown> = {};
  try {
    raw = parse(readFileSync(configPath, "utf-8")) as Record<string, unknown>;
  } catch {
    // File doesn't exist or is unreadable — use defaults
  }

  const rawAuth = (raw.auth ?? {}) as Partial<AuthConfig>;
  const rawApi = (raw.api ?? {}) as Partial<ApiConfig>;
  const rawGit = (raw.git ?? {}) as Partial<GitConfig>;

  return {
    auth: { ...AUTH_DEFAULTS, ...rawAuth },
    api: { ...API_DEFAULTS, ...rawApi },
    git: { ...GIT_DEFAULTS, ...rawGit },
  };
}

/** Loaded once at module init; call reloadAppConfig() to pick up changes. */
let appConfig: AppConfig = loadAppConfig();

export function getAppConfig(): AppConfig {
  return appConfig;
}

export function reloadAppConfig(): void {
  appConfig = loadAppConfig();
}
