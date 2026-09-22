//! Generator PDF sertifikat pelatihan (F-12, Kelompok 6 P6). Detail teknis
//! murni (font/layout, library `genpdfi`) — tidak tahu storage/repo, hanya
//! menerima data yang sudah di-fetch oleh application layer dan
//! mengembalikan bytes PDF. Implementasi konkret trait domain
//! `CertificateGenerator` (Dependency Inversion, lihat `domain::cert_generator`).

use std::io::Cursor;

use genpdfi::elements::{Break, Image, Paragraph};
use genpdfi::{Alignment, Document, Element as _, Scale};

use crate::domain::cert_generator::{CertificateGenerator, CertificateInput};

const FONT_DIR: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/fonts");
const FONT_NAME: &str = "Tinos";

/// Implementasi `CertificateGenerator` berbasis `genpdfi`. Stateless — tidak
/// perlu config/koneksi, jadi cukup di-instansiasi langsung di call site.
pub struct GenpdfiCertificateGenerator;

impl CertificateGenerator for GenpdfiCertificateGenerator {
    fn generate(&self, input: CertificateInput) -> Result<Vec<u8>, anyhow::Error> {
        generate(input)
    }
}

fn generate(input: CertificateInput) -> Result<Vec<u8>, anyhow::Error> {
    let font_family = genpdfi::fonts::from_files(FONT_DIR, FONT_NAME, None)
        .map_err(|e| anyhow::anyhow!("gagal memuat font sertifikat: {e}"))?;
    let mut doc = Document::new(font_family);
    doc.set_title("Sertifikat Pelatihan");

    doc.push(
        Paragraph::new("COMPLETION CERTIFICATE")
            .aligned(Alignment::Center)
            .styled(genpdfi::style::Effect::Bold),
    );
    doc.push(Break::new(2.0));
    doc.push(
        Paragraph::new(format!("Diberikan kepada: {}", input.peserta_nama))
            .aligned(Alignment::Center),
    );
    doc.push(
        Paragraph::new(format!(
            "atas keikutsertaan dalam pelatihan \"{}\"",
            input.judul_pelatihan
        ))
        .aligned(Alignment::Center),
    );
    doc.push(Break::new(1.0));
    doc.push(
        Paragraph::new(format!("Diselenggarakan oleh: {}", input.nama_perusahaan))
            .aligned(Alignment::Center),
    );
    doc.push(Paragraph::new(input.alamat_perusahaan).aligned(Alignment::Center));
    doc.push(Break::new(2.0));

    if let Some(sig_bytes) = input.signature_image {
        match Image::from_reader(Cursor::new(sig_bytes)) {
            Ok(img) => doc.push(
                img.with_alignment(Alignment::Center)
                    .with_scale(Scale::new(0.3, 0.3)),
            ),
            Err(e) => tracing::warn!(error = ?e, "gagal memuat gambar tanda tangan, dilewati"),
        }
    }

    doc.push(Paragraph::new(input.pejabat_nama).aligned(Alignment::Center));
    doc.push(Break::new(3.0));
    doc.push(Paragraph::new("VERIFIED by REJKI").aligned(Alignment::Center));

    let mut buf = Vec::new();
    doc.render(&mut buf)
        .map_err(|e| anyhow::anyhow!("gagal render PDF sertifikat: {e}"))?;
    Ok(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_input() -> CertificateInput<'static> {
        CertificateInput {
            peserta_nama: "Budi Santoso",
            judul_pelatihan: "Pelatihan K3 Dasar",
            nama_perusahaan: "PT Rejki Academy",
            alamat_perusahaan: "Jl. Merdeka No. 1, Jakarta",
            pejabat_nama: "Siti Aminah",
            signature_image: None,
        }
    }

    #[test]
    fn test_generate_given_no_signature_when_generate_then_valid_pdf_bytes() {
        let pdf = generate(sample_input()).expect("generate should succeed");
        assert!(pdf.starts_with(b"%PDF-"));
        assert!(pdf.len() > 500);
    }

    // P6.5: ekstrak teks PDF nyata (bukan cuma cek non-empty) untuk pastikan
    // teks baku PRD (F-12) benar-benar muncul di halaman render, bukan cuma
    // dikirim ke elemen yang gagal ditata layout-nya.
    #[test]
    fn test_generate_given_valid_input_when_generate_then_standard_text_present() {
        let pdf = generate(sample_input()).expect("generate should succeed");
        let text =
            pdf_extract::extract_text_from_mem(&pdf).expect("text extraction should succeed");

        assert!(text.contains("COMPLETION CERTIFICATE"));
        assert!(text.contains("Budi Santoso"));
        assert!(text.contains("Pelatihan K3 Dasar"));
        assert!(text.contains("PT Rejki Academy"));
        assert!(text.contains("Jl. Merdeka No. 1, Jakarta"));
        assert!(text.contains("Siti Aminah"));
        assert!(text.contains("VERIFIED by REJKI"));
        // P5.4: badge_icon tidak pernah ada sebagai parameter generator sama
        // sekali (lihat `CertificateInput`) — jadi otomatis tidak mungkin
        // muncul di output, tanpa perlu kode pencegahan tambahan.
        assert!(!text.to_lowercase().contains("badge_icon"));
    }
}
