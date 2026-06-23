import { pool } from "./connections";
import type { FcmToken } from "./types";

export async function get_fcm_tokens(user_id: string): Promise<FcmToken[]> {
  const res = await pool.query<FcmToken>(
    "SELECT user_id, fcm_token FROM notification.fcm_tokens WHERE user_id = $1",
    [user_id],
  );
  return res.rows;
}

export async function mark_event_processed(event_id: string): Promise<void> {
  await pool.query(
    `INSERT INTO notification.processed_events (event_id)
     VALUES ($1)
     ON CONFLICT (event_id) DO NOTHING`,
    [event_id],
  );
}

export async function get_fcm_tokens_column(user_id: string): Promise<string[]> {
  const res = await pool.query<{ token: string }>(
    "SELECT token FROM notification.device_tokens WHERE user_id = $1",
    [user_id],
  );
  return res.rows.map(r => r.token);
}
