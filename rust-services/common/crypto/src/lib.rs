//! Enkripsi data sensitif at-rest — utility bersama (common/crypto).
//!
//! Dipakai oleh auth-service (phone) dan user-service (NIK).
//! Memakai AES-256-GCM. Kunci (KEK) diambil dari environment `DATA_ENCRYPTION_KEY`
//! (32 byte, base64). Untuk envelope encryption penuh (DEK terpisah per record),
//! desain ini dapat diperluas; tahap awal memakai satu kunci simetris langsung,
//! dengan nonce acak per nilai yang disimpan bersama ciphertext.
//!
//! Format tersimpan: base64( nonce(12 byte) || ciphertext ).

use aes_gcm::{
    aead::{rand_core::RngCore, Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose::STANDARD as B64, Engine};

const NONCE_LEN: usize = 12;
const KEY_ENV: &str = "DATA_ENCRYPTION_KEY";

/// Ambil kunci 32-byte dari env (base64). Fail-fast bila tidak valid (Config Standard).
fn load_key() -> anyhow::Result<[u8; 32]> {
    let raw = std::env::var(KEY_ENV).map_err(|_| anyhow::anyhow!("{KEY_ENV} tidak di-set"))?;
    let bytes = B64
        .decode(raw.trim())
        .map_err(|e| anyhow::anyhow!("{KEY_ENV} bukan base64 valid: {e}"))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("{KEY_ENV} harus 32 byte setelah decode base64"))?;
    Ok(arr)
}

/// Enkripsi plaintext → string base64( nonce || ciphertext ).
pub fn encrypt(plaintext: &str) -> anyhow::Result<String> {
    let key = load_key()?;
    let cipher = Aes256Gcm::new((&key).into());

    let mut nonce_bytes = [0u8; NONCE_LEN];
    OsRng.fill_bytes(&mut nonce_bytes);
    let nonce = Nonce::from(nonce_bytes);

    let ciphertext = cipher
        .encrypt(&nonce, plaintext.as_bytes())
        .map_err(|_| anyhow::anyhow!("enkripsi gagal"))?;

    let mut out = Vec::with_capacity(NONCE_LEN + ciphertext.len());
    out.extend_from_slice(&nonce_bytes);
    out.extend_from_slice(&ciphertext);
    Ok(B64.encode(out))
}

/// Dekripsi string base64( nonce || ciphertext ) → plaintext.
pub fn decrypt(stored: &str) -> anyhow::Result<String> {
    let key = load_key()?;
    let cipher = Aes256Gcm::new((&key).into());

    let data = B64
        .decode(stored.trim())
        .map_err(|e| anyhow::anyhow!("nilai tersimpan bukan base64: {e}"))?;
    if data.len() < NONCE_LEN {
        anyhow::bail!("nilai tersimpan terlalu pendek");
    }
    let (nonce_bytes, ciphertext) = data.split_at(NONCE_LEN);
    let nonce_arr: [u8; NONCE_LEN] = nonce_bytes
        .try_into()
        .map_err(|_| anyhow::anyhow!("nonce tersimpan tidak valid"))?;
    let nonce = Nonce::from(nonce_arr);

    let plaintext = cipher
        .decrypt(&nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("dekripsi gagal (kunci salah atau data rusak)"))?;
    String::from_utf8(plaintext).map_err(|e| anyhow::anyhow!("plaintext bukan utf-8: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_test_key() {
        let key = B64.encode([7u8; 32]);
        // SAFETY: tes ini dijalankan secara serial (cargo test -- --test-threads=1
        // atau memang satu-satunya pengguna env var ini di test suite).
        unsafe { std::env::set_var(KEY_ENV, key) };
    }

    #[test]
    fn encrypt_decrypt_round_trip() {
        set_test_key();
        let plain = "3204110502010001";
        let enc = encrypt(plain).expect("encrypt");
        assert_ne!(enc, plain);
        let dec = decrypt(&enc).expect("decrypt");
        assert_eq!(dec, plain);
    }

    #[test]
    fn nonce_makes_ciphertext_unique() {
        set_test_key();
        let a = encrypt("sama").unwrap();
        let b = encrypt("sama").unwrap();
        assert_ne!(a, b, "nonce acak harus membuat ciphertext berbeda");
    }

    #[test]
    fn decrypt_tampered_data_fails() {
        set_test_key();
        let enc = encrypt("secret").unwrap();
        let mut tampered = enc.clone();
        tampered.push('X');
        assert!(decrypt(&tampered).is_err());
    }
}
