// Binary standalone belum didukung pada Phase 2.
//
// router() kini membutuhkan Arc<dyn AuthClient> untuk gating fitur (require_auth +
// require_active_account), yang hanya tersedia di Composition Root (rejki-app).
// Wiring AuthClient mode standalone ditunda ke Phase 4 (extraction). Jalankan via rejki-app.
fn main() {
    panic!("iklan-pekerjaan-service standalone belum didukung — jalankan via rejki-app (lihat Phase 4).");
}
