/**
 * Types Test — Pure unit test (0 dependencies)
 *
 * Tests: type guards, type shapes, discriminated unions.
 */
/// <reference types="bun" />
import { describe, test, expect } from "bun:test";
import {
  is_email_event,
  type NotificationEvent,
  type EmailEvent,
  type StreamEvent,
  type FcmToken,
} from "../src/types";

describe("is_email_event() type guard", () => {
  test("returns true when channel is 'email'", () => {
    const e: StreamEvent = {
      event_id: "evt-001",
      channel: "email",
      to: "user@example.com",
      subject: "OTP Login",
      body: "Your OTP is 123456",
    };
    expect(is_email_event(e)).toBe(true);
  });

  test("returns false when channel is undefined (push event)", () => {
    const e: StreamEvent = {
      event_id: "evt-001",
      recipient_id: "user-abc",
      title: "New Message",
      body: "You have a new message",
    };
    expect(is_email_event(e)).toBe(false);
  });

  test("returns false when channel is 'push'", () => {
    const e = {
      event_id: "evt-001",
      channel: "push",
      recipient_id: "u1",
      title: "T",
      body: "B",
    } as StreamEvent;
    expect(is_email_event(e)).toBe(false);
  });

  test("returns false when channel is null", () => {
    const e = {
      event_id: "evt-001",
      channel: null as unknown as string | undefined,
    } as StreamEvent;
    expect(is_email_event(e)).toBe(false);
  });

  test("returns false when channel is empty string", () => {
    const e = {
      event_id: "evt-001",
      channel: "",
    } as StreamEvent;
    expect(is_email_event(e)).toBe(false);
  });

  test("returns false for minimal object without channel", () => {
    const e = { event_id: "evt-001" } as unknown as StreamEvent;
    expect(is_email_event(e)).toBe(false);
  });
});

describe("event routing — discriminated union", () => {
  test("separates mixed events correctly", () => {
    const events: StreamEvent[] = [
      { event_id: "1", channel: "email", to: "a@b.com", subject: "S", body: "B" },
      { event_id: "2", recipient_id: "u1", title: "Push", body: "Hello" },
      { event_id: "3", channel: "email", to: "c@d.com", subject: "OTP", body: "123456" },
      { event_id: "4", recipient_id: "u2", title: "T2", body: "B2", channel: "push" },
    ];

    const emailEvents = events.filter(is_email_event);
    const pushEvents = events.filter((e) => !is_email_event(e));

    expect(emailEvents.length).toBe(2);
    expect(pushEvents.length).toBe(2);

    // Type narrowing works
    expect(emailEvents[0].to).toBe("a@b.com");
    expect(emailEvents[1].subject).toBe("OTP");
    expect(pushEvents[0].recipient_id).toBe("u1");
  });

  test("empty array", () => {
    expect([].filter(is_email_event).length).toBe(0);
  });

  test("all email", () => {
    const events: StreamEvent[] = [
      { event_id: "1", channel: "email", to: "a@b.com", subject: "S1", body: "B1" },
      { event_id: "2", channel: "email", to: "c@d.com", subject: "S2", body: "B2" },
    ];
    expect(events.filter(is_email_event).length).toBe(2);
    expect(events.filter((e) => !is_email_event(e)).length).toBe(0);
  });

  test("all push", () => {
    const events: StreamEvent[] = [
      { event_id: "1", recipient_id: "u1", title: "A", body: "B" },
      { event_id: "2", recipient_id: "u2", title: "C", body: "D" },
    ];
    expect(events.filter(is_email_event).length).toBe(0);
    expect(events.filter((e) => !is_email_event(e)).length).toBe(2);
  });
});

describe("NotificationEvent shape", () => {
  test("minimal fields", () => {
    const e: NotificationEvent = {
      event_id: "n-1",
      recipient_id: "u-1",
      title: "Hello",
      body: "World",
    };
    expect(e.event_id).toBe("n-1");
    expect(e.recipient_id).toBe("u-1");
    expect(e.data).toBeUndefined();
    expect(e.channel).toBeUndefined();
  });

  test("with optional data + channel", () => {
    const e: NotificationEvent = {
      event_id: "n-2",
      recipient_id: "u-2",
      title: "Order",
      body: "Shipped",
      data: { order_id: 42, status: "shipping" },
      channel: "push",
    };
    expect(e.data).toEqual({ order_id: 42, status: "shipping" });
    expect(e.channel).toBe("push");
  });

  test("data accepts mixed types", () => {
    const e: NotificationEvent = {
      event_id: "n-3",
      recipient_id: "u-3",
      title: "Mixed",
      body: "Data",
      data: { str: "hello", num: 100, bool: true, nil: null, obj: { n: 1 } },
    };
    expect(e.data?.bool).toBe(true);
    expect(e.data?.nil).toBeNull();
  });
});

describe("EmailEvent shape", () => {
  test("required fields", () => {
    const e: EmailEvent = {
      event_id: "e-1",
      channel: "email",
      to: "test@rejki.id",
      subject: "Welcome!",
      body: "Thanks for registering.",
    };
    expect(e.to).toBe("test@rejki.id");
    expect(e.subject).toBe("Welcome!");
    expect(e.channel).toBe("email");
  });
});

describe("FcmToken shape", () => {
  test("holds user_id + fcm_token", () => {
    const t: FcmToken = { user_id: "u-abc", fcm_token: "tok-xyz" };
    expect(t.user_id).toBe("u-abc");
    expect(t.fcm_token).toBe("tok-xyz");
  });
});
