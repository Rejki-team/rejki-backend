import { run_consumer } from "./consumer";
import { shutdown_connections } from "./connections";

async function shutdown() {
  console.log("Shutdown signal received — cleaning up connections...");
  try {
    await shutdown_connections();
  } catch (e) {
    console.error("Error during cleanup:", e);
  }
  console.log("Shutdown complete");
  process.exit(0);
}

process.on("SIGTERM", shutdown);
process.on("SIGINT", shutdown);

console.log("bun-notification-service starting...");

run_consumer().catch((err) => {
  console.error("Fatal error in consumer:", err);
  shutdown().then(() => process.exit(1));
});
