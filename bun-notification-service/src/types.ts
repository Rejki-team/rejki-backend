/** Event push FCM (default; tanpa field `channel` atau channel !== "email"). */
export interface NotificationEvent {
  event_id:     string;
  recipient_id: string;
  title:        string;
  body:         string;
  data?:        Record<string, unknown>;
  channel?:     string;
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

export interface FcmToken {
  user_id:   string;
  fcm_token: string;
}
