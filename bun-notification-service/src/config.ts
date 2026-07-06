import { readFileSync } from "node:fs";

function require_env(key: string): string {
  const val = process.env[key];
  if (!val) throw new Error(`${key} tidak di-set`);
  return val;
}

function opt_env(key: string): string | undefined {
  return process.env[key] || undefined;
}

/**
 * Load Firebase credentials: baca dari file path (production/container)
 * atau dari env var JSON string (testing/CI).
 */
function load_firebase_credentials(): string {
  const path = process.env.FIREBASE_SERVICE_ACCOUNT_PATH;
  if (path) {
    return readFileSync(path, "utf-8");
  }
  // Fallback: langsung dari env var JSON (untuk testing)
  return require_env("FIREBASE_CREDENTIALS_JSON");
}

export const config = {
  redis_url:             require_env("REDIS_URL"),
  database_url:          require_env("DATABASE_URL"),
  firebase_credentials:  load_firebase_credentials(),

  stream_name:     "notifications_stream",
  consumer_group:  "bun-consumers",
  consumer_name:   `bun-worker-${process.pid}`,

  batch_size:      10,
  block_ms:        5_000,
  max_retry:       3,
  dlq_stream:      "notifications_dlq",

  // SMTP — nama env var disamakan dengan docker-compose.dev.yml
  smtp_host:       opt_env("SMTP_HOST"),
  smtp_port:       Number(process.env.SMTP_PORT ?? "587"),
  smtp_user:       opt_env("SMTP_USERNAME"),
  smtp_pass:       opt_env("SMTP_PASSWORD"),
  email_from:      require_env("EMAIL_FROM"),
};
