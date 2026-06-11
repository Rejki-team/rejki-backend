// Binary standalone chat-service belum didukung pada Phase 2.
//
// chat-service::router() membutuhkan Arc<dyn AuthClient> untuk autentikasi WebSocket,
// yang hanya tersedia di Composition Root (rejki-app). Wiring AuthClient untuk mode
// standalone (microservice) ditunda ke Phase 4 (extraction). Untuk saat ini jalankan
// seluruh sistem melalui binary `rejki-app`.
fn main() {
    panic!(
        "chat-service standalone belum didukung — jalankan via rejki-app. \
         Lihat Phase 4 (extraction) untuk wiring AuthClient mode microservice."
    );
}
