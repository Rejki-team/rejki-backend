import Redis from "ioredis";
import { config } from "./config";
import { get_fcm_tokens_column, mark_event_processed } from "./db";
import { send_push } from "./firebase";
import { send_email } from "./email";
import { is_email_event, type StreamEvent, type NotificationEvent } from "./types";

const redis = new Redis(config.redis_url);

async function ensure_consumer_group(): Promise<void> {
  try {
    await redis.xgroup(
      "CREATE",
      config.stream_name,
      config.consumer_group,
      "0",
      "MKSTREAM",
    );
    console.log(`Consumer group '${config.consumer_group}' created`);
  } catch (e: any) {
    if (!e.message?.includes("BUSYGROUP")) throw e;
  }
}

async function process_event(event: StreamEvent): Promise<void> {
  // Email transaksional (mis. OTP) → kirim via SMTP, idempotent per event_id.
  if (is_email_event(event)) {
    await send_email(event.to, event.subject, event.body);
    await mark_event_processed(event.event_id);
    return;
  }

  // Default: push FCM.
  const tokens = await get_fcm_tokens_column(event.recipient_id);
  if (tokens.length === 0) {
    console.warn(`No FCM tokens for user ${event.recipient_id}`);
    return;
  }

  const data = event.data
    ? Object.fromEntries(
        Object.entries(event.data).map(([k, v]) => [k, String(v)])
      )
    : undefined;

  await Promise.all(
    tokens.map((token) =>
      send_push(token, event.title, event.body, data)
    ),
  );

  await mark_event_processed(event.event_id);
}

async function move_to_dlq(id: string, payload: string, error: string): Promise<void> {
  await redis.xadd(config.dlq_stream, "*",
    "original_id", id,
    "payload",     payload,
    "error",       error,
    "failed_at",   new Date().toISOString(),
  );
}

async function reclaim_stale(): Promise<void> {
  // XAUTOCLAIM: ambil alih pesan yang pending > 60 detik dari consumer lain
  const result = await (redis as any).xautoclaim(
    config.stream_name,
    config.consumer_group,
    config.consumer_name,
    60_000,
    "0-0",
    "COUNT",
    config.batch_size,
  );
  const messages: [string, string[]][] = result[1] ?? [];
  for (const [id, fields] of messages) {
    await handle_message(id, fields);
  }
}

async function handle_message(id: string, fields: string[]): Promise<void> {
  const payload_str = fields[fields.indexOf("payload") + 1];
  let retries = 0;

  while (retries <= config.max_retry) {
    try {
      const event: StreamEvent = JSON.parse(payload_str);
      await process_event(event);
      await redis.xack(config.stream_name, config.consumer_group, id);
      return;
    } catch (e: any) {
      retries++;
      if (retries > config.max_retry) {
        console.error(`Event ${id} gagal setelah ${config.max_retry} retry:`, e.message);
        await move_to_dlq(id, payload_str, e.message);
        await redis.xack(config.stream_name, config.consumer_group, id);
      }
    }
  }
}

export async function run_consumer(): Promise<never> {
  await ensure_consumer_group();
  console.log(`Consumer '${config.consumer_name}' started`);

  // Reclaim stale messages dari consumer lain yang crash
  await reclaim_stale();

  while (true) {
    const results = await redis.xreadgroup(
      "GROUP",
      config.consumer_group,
      config.consumer_name,
      "COUNT",
      config.batch_size,
      "BLOCK",
      config.block_ms,
      "STREAMS",
      config.stream_name,
      ">",
    ) as [string, [string, string[]][]][] | null;

    if (!results) continue;

    for (const [_stream, messages] of results) {
      for (const [id, fields] of messages) {
        await handle_message(id, fields);
      }
    }
  }
}
