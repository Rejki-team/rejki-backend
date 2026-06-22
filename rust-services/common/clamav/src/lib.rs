//! # common-clamav — ClamAV TCP client library
//!
//! Implementasi protokol ClamAV `INSTREAM` untuk scan file upload.
//! Berkomunikasi via TCP ke ClamAV daemon (port 3310).
//!
//! ## Protocol
//! ClamAV INSTREAM:
//! 1. Kirim `zINSTREAM\0` (command)
//! 2. Kirim data dalam chunk: `[4-byte big-endian size][data][4-byte big-endian size][data]...`
//! 3. Kirim terminator: `[0x00, 0x00, 0x00, 0x00]`
//! 4. Baca response: `stream: OK` (clean) atau `stream: {virus_name} FOUND` (infected)
//!
//! ## Example
//! ```rust,no_run
//! use common_clamav::{ClamavClient, ScanResult};
//!
//! # async fn example() {
//! let client = ClamavClient::new("clamav-daemon".into(), 3310);
//! let result = client.scan_bytes(b"dummy file content").await.unwrap();
//! match result {
//!     ScanResult::Clean => println!("File aman"),
//!     ScanResult::Infected(v) => println!("Virus terdeteksi: {}", v),
//! }
//! # }
//! ```

use std::time::Duration;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpStream;

/// Durasi maksimal menunggu hasil scan dari ClamAV
const SCAN_TIMEOUT_SECS: u64 = 120;

/// Durasi maksimal koneksi TCP ke ClamAV
const CONNECT_TIMEOUT_SECS: u64 = 5;

/// Ukuran chunk untuk streaming data ke ClamAV
const CHUNK_SIZE: usize = 4096;

// ── Error ───────────────────────────────────────────────────────────────────

#[derive(Debug, thiserror::Error)]
pub enum ClamavError {
    #[error("ClamAV connection timeout: {0}")]
    ConnectionTimeout(String),

    #[error("ClamAV connection refused: {0}")]
    ConnectionFailed(String),

    #[error("scan timeout (>{0}s)")]
    ScanTimeout(u64),

    #[error("ClamAV scan error: {0}")]
    ScanFailed(String),

    #[error("unexpected ClamAV response: {0}")]
    UnexpectedResponse(String),
}

// ── Result ──────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, PartialEq)]
pub enum ScanResult {
    /// File aman — tidak terdeteksi virus
    Clean,

    /// File terinfeksi — nama virus
    Infected(String),
}

// ── Client ──────────────────────────────────────────────────────────────────

/// ClamAV TCP client untuk scanning file via INSTREAM protocol.
#[derive(Debug, Clone)]
pub struct ClamavClient {
    host: String,
    port: u16,
    timeout: Duration,
}

impl ClamavClient {
    /// Buat ClamAV client baru.
    ///
    /// - `host`: hostname container ClamAV (default: `clamav-daemon`)
    /// - `port`: port TCP (default: `3310`)
    pub fn new(host: String, port: u16) -> Self {
        Self {
            host,
            port,
            timeout: Duration::from_secs(SCAN_TIMEOUT_SECS),
        }
    }

    /// Buat ClamAV client dengan custom timeout.
    pub fn with_timeout(host: String, port: u16, timeout: Duration) -> Self {
        Self {
            host,
            port,
            timeout,
        }
    }

    /// Buat dari env vars (fallback ke default jika tidak diset).
    ///
    /// - `CLAMAV_HOST` (default: `clamav-daemon`)
    /// - `CLAMAV_PORT` (default: `3310`)
    pub fn from_env() -> Self {
        let host = std::env::var("CLAMAV_HOST").unwrap_or_else(|_| "clamav-daemon".into());
        let port = std::env::var("CLAMAV_PORT")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(3310);
        Self::new(host, port)
    }

    /// Scan byte buffer via ClamAV INSTREAM protocol.
    ///
    /// Returns `ScanResult::Clean` atau `ScanResult::Infected(virus_name)`.
    pub async fn scan_bytes(&self, data: &[u8]) -> Result<ScanResult, ClamavError> {
        // ── 1. Connect ke ClamAV daemon ─────────────────────────────────────
        let stream = tokio::time::timeout(
            Duration::from_secs(CONNECT_TIMEOUT_SECS),
            TcpStream::connect((self.host.as_str(), self.port)),
        )
        .await
        .map_err(|_| ClamavError::ConnectionTimeout(format!("{}:{}", self.host, self.port)))?
        .map_err(|e| ClamavError::ConnectionFailed(e.to_string()))?;

        let (mut reader, mut writer) = stream.into_split();

        // ── 2. Kirim command INSTREAM ─────────────────────────────────────
        writer
            .write_all(b"zINSTREAM\0")
            .await
            .map_err(|e| ClamavError::ScanFailed(format!("write INSTREAM: {e}")))?;

        // ── 3. Kirim data dalam chunk ──────────────────────────────────────
        for chunk in data.chunks(CHUNK_SIZE) {
            let size = (chunk.len() as u32).to_be_bytes();
            writer
                .write_all(&size)
                .await
                .map_err(|e| ClamavError::ScanFailed(format!("write size: {e}")))?;
            writer
                .write_all(chunk)
                .await
                .map_err(|e| ClamavError::ScanFailed(format!("write data: {e}")))?;
        }

        // ── 4. Kirim terminator (0-byte chunk) ─────────────────────────────
        writer
            .write_all(&0u32.to_be_bytes())
            .await
            .map_err(|e| ClamavError::ScanFailed(format!("write terminator: {e}")))?;

        writer
            .flush()
            .await
            .map_err(|e| ClamavError::ScanFailed(format!("flush: {e}")))?;

        // ── 5. Baca response ───────────────────────────────────────────────
        let mut buf = vec![0u8; 4096];
        let n = tokio::time::timeout(self.timeout, reader.read(&mut buf))
            .await
            .map_err(|_| ClamavError::ScanTimeout(SCAN_TIMEOUT_SECS))?
            .map_err(|e| ClamavError::ScanFailed(format!("read response: {e}")))?;

        let response = String::from_utf8_lossy(&buf[..n]).trim().to_string();

        // ── 6. Parse response ──────────────────────────────────────────────
        if response.contains("FOUND") {
            // Format: "stream: VirusName FOUND"
            let virus_name = response
                .trim()
                .trim_start_matches("stream: ")
                .trim_end_matches(" FOUND")
                .to_string();
            Ok(ScanResult::Infected(virus_name))
        } else if response.contains("OK") {
            Ok(ScanResult::Clean)
        } else {
            Err(ClamavError::UnexpectedResponse(response))
        }
    }

    /// Ping ClamAV untuk verifikasi koneksi.
    pub async fn ping(&self) -> Result<(), ClamavError> {
        let mut stream = tokio::time::timeout(
            Duration::from_secs(CONNECT_TIMEOUT_SECS),
            TcpStream::connect((self.host.as_str(), self.port)),
        )
        .await
        .map_err(|_| ClamavError::ConnectionTimeout(format!("{}:{}", self.host, self.port)))?
        .map_err(|e| ClamavError::ConnectionFailed(e.to_string()))?;

        stream
            .write_all(b"PING\0")
            .await
            .map_err(|e| ClamavError::ScanFailed(format!("write PING: {e}")))?;

        let mut buf = [0u8; 32];
        stream
            .read(&mut buf)
            .await
            .map_err(|e| ClamavError::ScanFailed(format!("read PONG: {e}")))?;

        let response = String::from_utf8_lossy(&buf);
        if response.starts_with("PONG") {
            Ok(())
        } else {
            Err(ClamavError::UnexpectedResponse(response.to_string()))
        }
    }
}

// ── Tests ───────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_from_env_defaults() {
        // Tanpa env var, harus pakai default
        let client = ClamavClient::from_env();
        assert_eq!(client.host, "clamav-daemon");
        assert_eq!(client.port, 3310);
    }

    #[test]
    fn test_clean_response_parsed() {
        let result = parse_response_helper("stream: OK");
        assert_eq!(result, Ok(ScanResult::Clean));
    }

    #[test]
    fn test_infected_response_parsed() {
        let result = parse_response_helper("stream: Eicar-Test-Signature FOUND");
        assert_eq!(
            result,
            Ok(ScanResult::Infected("Eicar-Test-Signature".into()))
        );
    }

    #[test]
    fn test_unknown_response_error() {
        let result = parse_response_helper("ERROR: Can't open file");
        assert!(result.is_err());
    }

    /// Helper untuk test parsing response tanpa TCP
    fn parse_response_helper(response: &str) -> Result<ScanResult, ClamavError> {
        if response.contains("FOUND") {
            let virus_name = response
                .trim()
                .trim_start_matches("stream: ")
                .trim_end_matches(" FOUND")
                .to_string();
            Ok(ScanResult::Infected(virus_name))
        } else if response.contains("OK") {
            Ok(ScanResult::Clean)
        } else {
            Err(ClamavError::UnexpectedResponse(response.to_string()))
        }
    }
}
