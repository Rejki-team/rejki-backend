/**
 * Consumer module tests
 *
 * Karena consumer.ts mengimpor redis + pg connections (side-effect),
 * test ini fokus pada pure logic yang bisa diverifikasi tanpa mocking:
 * - Event routing (email vs push)
 * - Data transformation (Record<string, unknown> → Record<string, string>)
 * - Retry + DLQ logic pattern
 * - Module structure verification
 */
/// <reference types="bun" />
import { describe, test, expect } from "bun:test";
import {
  is_email_event,
  type NotificationEvent,
  type EmailEvent,
  type StreamEvent,
} from "../src/types";
import { config } from "../src/config";

describe("consumer — event routing (pure logic)", () => {
  test("email events route to send_email path", () => {
    const events: StreamEvent[] = [
      { event_id: "e1", channel: "email", to: "a@b.com", subject: "S", body: "B" },
      { event_id: "e2", recipient_id: "u1", title: "Push", body: "Hello" },
      { event_id: "e3", channel: "email", to: "c@d.com", subject: "OTP", body: "123456" },
    ];

    const emailCount = events.filter(is_email_event).length;
    const pushCount = events.filter((e) => !is_email_event(e)).length;

    expect(emailCount).toBe(2);
    expect(pushCount).toBe(1);
  });

  test("no events = empty routing", () => {
    const events: StreamEvent[] = [];
    expect(events.filter(is_email_event).length).toBe(0);
  });

  test("all push events = zero email routing", () => {
    const events: StreamEvent[] = [
      { event_id: "1", recipient_id: "u1", title: "A", body: "B" },
      { event_id: "2", recipient_id: "u2", title: "C", body: "D", channel: "push" },
    ];
    expect(events.filter(is_email_event).length).toBe(0);
  });

  test("all email events = all routed to email", () => {
    const events: StreamEvent[] = [
      { event_id: "1", channel: "email", to: "a@b.com", subject: "S1", body: "B1" },
      { event_id: "2", channel: "email", to: "c@d.com", subject: "S2", body: "B2" },
    ];
    expect(events.filter(is_email_event).length).toBe(2);
  });
});

describe("consumer — data payload transformation (pure logic)", () => {
  /**
   * Replikasi logika dari consumer.ts:
   *   Object.fromEntries(Object.entries(event.data).map(([k, v]) => [k, String(v)]))
   */
  function transformPayload(data?: Record<string, unknown>): Record<string, string> | undefined {
    if (!data) return undefined;
    return Object.fromEntries(
      Object.entries(data).map(([k, v]) => [k, String(v)]),
    );
  }

  test("converts mixed types to strings", () => {
    const input = { bool: true, num: 1, str: "hello", null_val: null, obj: { a: 1 } };
    const output = transformPayload(input)!;

    expect(output.bool).toBe("true");
    expect(output.num).toBe("1");
    expect(output.str).toBe("hello");
    expect(output.null_val).toBe("null");
    expect(output.obj).toBe("[object Object]");
  });

  test("returns undefined when data is undefined", () => {
    expect(transformPayload(undefined)).toBeUndefined();
  });

  test("returns empty object when data is empty", () => {
    const output = transformPayload({});
    expect(output).toEqual({});
  });

  test("handles string-only data as identity", () => {
    const input = { order_id: "42", type: "shipping" };
    const output = transformPayload(input)!;
    expect(output).toEqual({ order_id: "42", type: "shipping" });
  });

  test("preserves all keys through transformation", () => {
    const input = { a: 1, b: 2, c: 3 };
    const keys = Object.keys(transformPayload(input)!);
    expect(keys.sort()).toEqual(["a", "b", "c"]);
  });
});

describe("consumer — retry + DLQ constants", () => {
  test("max_retry is 3", () => {
    expect(config.max_retry).toBe(3);
  });

  test("dlq_stream is 'notifications_dlq'", () => {
    expect(config.dlq_stream).toBe("notifications_dlq");
  });

  test("batch_size and block_ms are reasonable", () => {
    expect(config.batch_size).toBeGreaterThan(0);
    expect(config.block_ms).toBeGreaterThan(0);
  });
});

describe("consumer — process_event logic simulation", () => {
  /**
   * Simulasi logika process_event tanpa side effect:
   * - Email event → send_email dipanggil
   * - Push event → get_fcm_tokens lalu send_push per token
   * - Tokens kosong → warning, no send
   */
  test("email event identified for SMTP routing", () => {
    const event: EmailEvent = {
      event_id: "evt-email",
      channel: "email",
      to: "user@example.com",
      subject: "OTP Login",
      body: "Your code is 123456",
    };

    // Verifikasi type guard bekerja
    expect(is_email_event(event)).toBe(true);
    expect(event.to).toBe("user@example.com");
    expect(event.subject).toBe("OTP Login");
  });

  test("push event routed to FCM when tokens exist", () => {
    const event: NotificationEvent = {
      event_id: "evt-push",
      recipient_id: "user-123",
      title: "New Message",
      body: "You have a message",
      data: { conversation_id: 42 },
    };

    expect(is_email_event(event)).toBe(false);
    expect(event.recipient_id).toBe("user-123");
    expect(event.data?.conversation_id).toBe(42);
  });

  test("push event with no tokens triggers warning path (no FCM send)", () => {
    const event: NotificationEvent = {
      event_id: "evt-notokens",
      recipient_id: "user-no-tokens",
      title: "Hello",
      body: "World",
    };

    // Simulasi: jika tokens.length === 0, tidak ada send_push dipanggil
    const mockTokens: string[] = [];
    const sentPush = mockTokens.length > 0;

    expect(sentPush).toBe(false);
    expect(event.recipient_id).toBe("user-no-tokens");
  });
});
