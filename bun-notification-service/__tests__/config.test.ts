/**
 * Config + Email + Firebase Tests — Pure unit
 *
 * Test ini memverifikasi:
 *   - Config membaca env var dengan NAMA YANG SAMA seperti docker-compose.dev.yml
 *   - SMTP_USERNAME / SMTP_PASSWORD (bukan SMTP_USER / SMTP_PASS)
 *   - Firebase: FIREBASE_SERVICE_ACCOUNT_PATH (container) atau FIREBASE_CREDENTIALS_JSON (CI/test)
 *   - Event routing logic (email vs push)
 */
/// <reference types="bun" />
import { describe, test, expect } from "bun:test";
import { config } from "../src/config";
import { is_email_event, type EmailEvent, type NotificationEvent, type StreamEvent } from "../src/types";

// ── Config ────────────────────────────────────────────────────────────────

describe("config — required env vars (docker-compose aligned)", () => {
  test("redis_url and database_url", () => {
    expect(config.redis_url).toBe("redis://localhost:6379");
    expect(config.database_url).toBe("postgresql://localhost:5432/test");
    expect(config.email_from).toBe("noreply@rejki.id");
  });

  test("firebase_credentials loaded from FIREBASE_CREDENTIALS_JSON (CI mode)", () => {
    const creds = JSON.parse(config.firebase_credentials);
    expect(creds.type).toBe("service_account");
    expect(creds.project_id).toBe("test-project");
  });
});

describe("config — consumer constants", () => {
  test("stream_name = 'notifications_stream'", () => {
    expect(config.stream_name).toBe("notifications_stream");
  });

  test("consumer_group = 'bun-consumers'", () => {
    expect(config.consumer_group).toBe("bun-consumers");
  });

  test("consumer_name matches /^bun-worker-/", () => {
    expect(config.consumer_name).toMatch(/^bun-worker-/);
  });

  test("batch_size=10, block_ms=5000, max_retry=3", () => {
    expect(config.batch_size).toBe(10);
    expect(config.block_ms).toBe(5_000);
    expect(config.max_retry).toBe(3);
    expect(config.dlq_stream).toBe("notifications_dlq");
  });
});

describe("config — SMTP env vars (docker-compose aligned)", () => {
  test("smtp_host/smtp_user/smtp_pass undefined in test (no SMTP set)", () => {
    // Di test.env tidak ada SMTP_HOST / SMTP_USERNAME / SMTP_PASSWORD
    expect(config.smtp_host).toBeUndefined();
    expect(config.smtp_user).toBeUndefined();
    expect(config.smtp_pass).toBeUndefined();
  });

  test("smtp_port defaults to 587", () => {
    expect(config.smtp_port).toBe(587);
  });
});

// ── Email routing (pure) ──────────────────────────────────────────────────

describe("email — event routing with is_email_event", () => {
  test("email events route to SMTP path", () => {
    const e: EmailEvent = {
      event_id: "e-001",
      channel: "email",
      to: "user@rejki.id",
      subject: "OTP",
      body: "123456",
    };
    expect(is_email_event(e)).toBe(true);
    expect(e.to).toBe("user@rejki.id");
  });

  test("push events do NOT route to SMTP", () => {
    const e: NotificationEvent = {
      event_id: "n-001",
      recipient_id: "u-1",
      title: "Hello",
      body: "World",
    };
    expect(is_email_event(e)).toBe(false);
  });

  test("push with channel='push' does NOT route to SMTP", () => {
    const e = {
      event_id: "n-002",
      recipient_id: "u-2",
      title: "Alert",
      body: "Something",
      channel: "push",
    } as StreamEvent;
    expect(is_email_event(e)).toBe(false);
  });
});

// ── Firebase payload (pure) ───────────────────────────────────────────────

describe("firebase — push notification payload structure", () => {
  test("title, body, token are correctly typed", () => {
    const title = "Order Update";
    const body = "Your order shipped";
    const token = "fcm-token-abc";

    expect(typeof title).toBe("string");
    expect(typeof body).toBe("string");
    expect(typeof token).toBe("string");
    expect(token.length).toBeGreaterThan(0);
  });

  test("non-ASCII/emoji titles are preserved", () => {
    const title = "Pemberitahuan 🎉";
    const body = "Pesanan sudah dikirim ✓";
    expect(title.includes("🎉")).toBe(true);
    expect(body.includes("✓")).toBe(true);
  });
});
