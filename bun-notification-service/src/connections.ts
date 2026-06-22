import Redis from "ioredis";
import { Pool } from "pg";
import { config } from "./config";
import { transporter } from "./email";

// Redis connection — shared with consumer.ts
export const redis = new Redis(config.redis_url);

// PostgreSQL connection pool — shared with db.ts
export const pool = new Pool({ connectionString: config.database_url });

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

  console.log("Closing PostgreSQL pool...");
  try {
    await pool.end();
  } catch (e) {
    console.error("PG pool end error:", e);
  }

  console.log("Closing SMTP transporter...");
  try {
    await transporter?.close();
  } catch (e) {
    console.error("SMTP close error:", e);
  }

  console.log("All connections closed");
}
