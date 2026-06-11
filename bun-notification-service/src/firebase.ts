import admin from "firebase-admin";
import { config } from "./config";

let app: admin.app.App | null = null;

export function get_firebase_app(): admin.app.App {
  if (!app) {
    const creds = JSON.parse(config.firebase_credentials);
    app = admin.initializeApp({
      credential: admin.credential.cert(creds),
    });
  }
  return app;
}

export async function send_push(
  token:   string,
  title:   string,
  body:    string,
  data?:   Record<string, string>,
): Promise<void> {
  const messaging = get_firebase_app().messaging();
  await messaging.send({
    token,
    notification: { title, body },
    data,
  });
}
