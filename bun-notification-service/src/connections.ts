import Redis from "ioredis";
import { config } from "./config";
import { transporter } from "./email";

// Redis connection — shared with consumer.ts
export const redis = new Redis(config.redis_url);

// Tanpa koneksi Postgres — bun-notification-service stateless untuk DB (Opsi C).

/**
 * Graceful shutdown: close semua koneksi sebelum exit.
 */
export async function shutdown_connections(): Promise<void> {
  console.log("Closing Redis connection...");
  try {
    await redis.quit();
  } catch (e) {
    console.error("Redis quit error:", e);
  }

  console.log("Closing SMTP transporter...");
  try {
    await transporter?.close();
  } catch (e) {
    console.error("SMTP close error:", e);
  }

  console.log("All connections closed");
}
