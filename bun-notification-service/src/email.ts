import nodemailer, { type Transporter } from "nodemailer";
import { config } from "./config";

let transporter: Transporter | null = null;

export { transporter };

function get_transporter(): Transporter {
  if (!config.smtp_host) {
    throw new Error("SMTP_HOST tidak di-set — pengiriman email tidak dikonfigurasi");
  }
  if (!transporter) {
    transporter = nodemailer.createTransport({
      host:   config.smtp_host,
      port:   config.smtp_port,
      secure: config.smtp_port === 465, // 465 = implicit TLS
      auth:
        config.smtp_user && config.smtp_pass
          ? { user: config.smtp_user, pass: config.smtp_pass }
          : undefined,
    });
  }
  return transporter;
}

/** Kirim email transaksional (mis. OTP). Melempar bila SMTP tidak dikonfigurasi/gagal. */
export async function send_email(to: string, subject: string, body: string): Promise<void> {
  const tx = get_transporter();
  await tx.sendMail({
    from:    config.email_from,
    to,
    subject,
    text:    body,
  });
}
