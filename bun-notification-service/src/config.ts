function require_env(key: string): string {
  const val = process.env[key];
  if (!val) throw new Error(`${key} tidak di-set`);
  return val;
}

function opt_env(key: string): string | undefined {
  return process.env[key] || undefined;
}

export const config = {
  redis_url:             require_env("REDIS_URL"),
  database_url:          require_env("DATABASE_URL"),
  firebase_credentials:  require_env("FIREBASE_CREDENTIALS_JSON"),

  stream_name:     "notifications_stream",
  consumer_group:  "bun-consumers",
  consumer_name:   `bun-worker-${process.pid}`,

  batch_size:      10,
  block_ms:        5_000,
  max_retry:       3,
  dlq_stream:      "notifications_dlq",

  // SMTP (opsional) — untuk email transaksional seperti OTP (channel "email").
  smtp_host:       opt_env("SMTP_HOST"),
  smtp_port:       Number(process.env.SMTP_PORT ?? "587"),
  smtp_user:       opt_env("SMTP_USER"),
  smtp_pass:       opt_env("SMTP_PASS"),
  email_from:      process.env.EMAIL_FROM ?? "no-reply@rejki.id",
};
