// Binary standalone belum didukung pada Phase 2 — router() butuh Arc<dyn AuthClient>
// untuk gating (require_auth + require_active_account), hanya tersedia di rejki-app.
// Wiring AuthClient mode standalone ditunda ke Phase 4 (extraction).
fn main() {
    panic!("iklan-pelatihan-service standalone belum didukung — jalankan via rejki-app (lihat Phase 4).");
}
