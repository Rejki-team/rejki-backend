import { run_consumer } from "./consumer";

process.on("SIGTERM", () => {
  console.log("SIGTERM received — shutting down");
  process.exit(0);
});

process.on("SIGINT", () => {
  console.log("SIGINT received — shutting down");
  process.exit(0);
});

console.log("bun-notification-service starting...");

run_consumer().catch((err) => {
  console.error("Fatal error in consumer:", err);
  process.exit(1);
});
