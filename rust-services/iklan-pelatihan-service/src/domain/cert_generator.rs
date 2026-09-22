//! Kontrak generator PDF sertifikat pelatihan (F-12, Kelompok 6 P6). Library
//! PDF/font yang dipakai adalah detail infrastructure (seperti JWT/Redis/S3
//! di CLAUDE.md §3) — use case (application) hanya bergantung pada trait ini
//! (Dependency Inversion), pola sama seperti `IklanPelatihanRepository`.

/// Argumen generator — struct param (bukan 6 argumen lepas), per CLAUDE.md §4.7.
pub struct CertificateInput<'a> {
    pub peserta_nama: &'a str,
    pub judul_pelatihan: &'a str,
    pub nama_perusahaan: &'a str,
    pub alamat_perusahaan: &'a str,
    pub pejabat_nama: &'a str,
    /// Bytes gambar tanda tangan (dari `signature_object_key`), opsional —
    /// pelatihan lama/tanpa tanda tangan tetap bisa menghasilkan sertifikat.
    pub signature_image: Option<&'a [u8]>,
}

pub trait CertificateGenerator: Send + Sync {
    fn generate(&self, input: CertificateInput) -> Result<Vec<u8>, anyhow::Error>;
}
