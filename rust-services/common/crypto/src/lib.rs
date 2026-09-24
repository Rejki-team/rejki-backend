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

// ── Deterministic lookup hash (untuk kolom UNIQUE di atas nilai terenkripsi) ──
//
// `encrypt()`/`decrypt()` di atas memakai nonce acak — ciphertext-nya TIDAK deterministik,
// sehingga tidak bisa dipakai untuk UNIQUE constraint (nilai plaintext yang sama akan
// menghasilkan ciphertext berbeda tiap kali). `hash_lookup()` memakai HMAC-SHA256 dengan
// kunci terpisah (`DATA_HASH_KEY`, bukan `DATA_ENCRYPTION_KEY`) sehingga deterministik dan
// aman dijadikan kolom `UNIQUE` (mis. `phone_hash`), tanpa membocorkan plaintext (satu arah).

use hmac::{Hmac, Mac};
use sha2::Sha256;

type HmacSha256 = Hmac<Sha256>;

const HASH_KEY_ENV: &str = "DATA_HASH_KEY";

fn load_hash_key() -> anyhow::Result<[u8; 32]> {
    let raw =
        std::env::var(HASH_KEY_ENV).map_err(|_| anyhow::anyhow!("{HASH_KEY_ENV} tidak di-set"))?;
    let bytes = B64
        .decode(raw.trim())
        .map_err(|e| anyhow::anyhow!("{HASH_KEY_ENV} bukan base64 valid: {e}"))?;
    let arr: [u8; 32] = bytes
        .as_slice()
        .try_into()
        .map_err(|_| anyhow::anyhow!("{HASH_KEY_ENV} harus 32 byte setelah decode base64"))?;
    Ok(arr)
}

/// Hash deterministik (hex-encoded HMAC-SHA256) untuk lookup/uniqueness — bukan enkripsi.
pub fn hash_lookup(value: &str) -> anyhow::Result<String> {
    let key = load_hash_key()?;
    let mut mac = <HmacSha256 as Mac>::new_from_slice(&key)
        .map_err(|e| anyhow::anyhow!("kunci HMAC tidak valid: {e}"))?;
    mac.update(value.as_bytes());
    Ok(hex::encode(mac.finalize().into_bytes()))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn set_test_key() {
        let key = B64.encode([7u8; 32]);
        // Gunakan std::env::set_var — aman karena test dijalankan serial (--test-threads=1)
        std::env::set_var(KEY_ENV, key);
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

    fn set_test_hash_key() {
        let key = B64.encode([9u8; 32]);
        std::env::set_var(HASH_KEY_ENV, key);
    }

    #[test]
    fn test_hash_lookup_given_same_value_when_hashed_twice_then_deterministic() {
        set_test_hash_key();
        let a = hash_lookup("081234567890").unwrap();
        let b = hash_lookup("081234567890").unwrap();
        assert_eq!(
            a, b,
            "hash_lookup harus deterministik untuk nilai yang sama"
        );
    }

    #[test]
    fn test_hash_lookup_given_different_values_when_hashed_then_different_output() {
        set_test_hash_key();
        let a = hash_lookup("081234567890").unwrap();
        let b = hash_lookup("089999999999").unwrap();
        assert_ne!(a, b);
    }
    // Catatan: tidak ada test "missing key" via `remove_var` di sini — env var bersifat
    // global per-proses dan test lain di modul ini berjalan paralel (thread berbeda,
    // proses sama); meng-unset akan berisiko race dengan test lain yang mengharapkan
    // key tetap terpasang (Hazard #1 — hindari kondisi yang sama di test sendiri).
}
