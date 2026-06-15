// Binary standalone belum didukung pada Phase 2 — router() butuh Arc<dyn AuthClient>
// untuk autentikasi, hanya tersedia di Composition Root (rejki-app).
// Wiring AuthClient mode standalone ditunda ke Phase 4 (extraction).
fn main() {
    panic!(
        "notification-service standalone belum didukung — jalankan via rejki-app (lihat Phase 4)."
    );
}
