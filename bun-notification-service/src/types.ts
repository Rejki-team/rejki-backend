/** Event push FCM (default; tanpa field `channel` atau channel !== "email"). */
export interface NotificationEvent {
  event_id:     string;
  recipient_id: string;
  title:        string;
  body:         string;
  data?:        Record<string, unknown>;
  channel?:     string;
  /** Token FCM aktif milik recipient_id, sudah di-resolve Rust saat publish (Opsi C) — tidak query Postgres lagi di sini. */
  tokens:       string[];
}

/** Event email transaksional (channel === "email"), mis. OTP dari auth-service. */
export interface EmailEvent {
  event_id: string;
  channel:  "email";
  to:       string;
  subject:  string;
  body:     string;
}

export type StreamEvent = NotificationEvent | EmailEvent;

export function is_email_event(e: StreamEvent): e is EmailEvent {
  return (e as EmailEvent).channel === "email";
}
