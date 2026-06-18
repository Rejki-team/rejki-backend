## MODIFIED Requirements

### Requirement: Endpoint profil publik dihapus — semua operasi profil harus terautentikasi

Sistem SHALL memindahkan `GET /api/v1/users/{id}` dari public router ke protected router
dengan middleware autentikasi. Akses tanpa token SHALL mengembalikan **401 Unauthorized**.
Akses dengan token non-pemilik SHALL mengembalikan **404 Not Found** (ownership pattern).
Data sensitif (`phone`, `full_name`, `bio`, `kyc_status`) SHALL tidak dapat diakses oleh
anonymous caller.

**CLAUDEMD Ref:** §4.4 — "3 lapis: autentikasi di middleware, ownership check di repository,
visibility di handler" dan "IDOR → 404, bukan 403. Ownership check di query"

#### Scenario: Anonymous caller mencoba akses profil pengguna
- **WHEN** pemanggil tanpa token auth memanggil `GET /api/v1/users/{uuid}`
- **THEN** sistem mengembalikan **401 Unauthorized**

#### Scenario: Authenticated user mencoba akses profil pengguna lain
- **WHEN** pengguna A (terautentikasi) memanggil `GET /api/v1/users/{user_b_id}`
- **THEN** sistem mengembalikan **404 Not Found** (ownership check: query mencari profil dengan `id = $1 AND auth_id = $2`)

---

### Requirement: Validasi NIK harus berupa 16 digit angka

Sistem SHALL memvalidasi bahwa field NIK hanya berisi karakter digit ASCII (0-9) selain
validasi panjang (tepat 16 karakter). Input non-digit SHALL ditolak dengan error validasi
**422 Unprocessable Entity**.

**CLAUDEMD Ref:** §4.5 — "Validasi & sanitasi semua input I/O eksternal"

#### Scenario: NIK berisi karakter non-digit
- **WHEN** pengguna mengirim `nik: "ABCD1234EFGH5678"` (16 karakter, non-digit)
- **THEN** sistem menolak dengan error "NIK harus 16 digit angka" (422)

#### Scenario: NIK berisi hanya digit
- **WHEN** pengguna mengirim `nik: "3273012345678901"` (16 digit numerik valid)
- **THEN** sistem menerima dan melanjutkan enkripsi

---

### Requirement: Audit trail upload_issued tidak boleh silent discard

Sistem SHALL mempropagasi error dari `log_upload_issued` (INSERT ke `document_access_log`).
Jika audit trail gagal dicatat, operasi penerbitan presigned upload URL SHALL gagal total
(dengan error log) — memastikan rantai audit utuh untuk setiap akses dokumen.

**CLAUDEMD Ref:** §4.5 — "I/O aman: degradasi anggun (mis. Redis down → DB tetap simpan,
push di-skip dengan `warn!`)" — untuk kasus audit, gagal menyimpan audit record adalah
kondisi kritis, bukan degradasi anggun.

#### Scenario: Audit trail INSERT gagal
- **WHEN** `INSERT INTO user_svc.document_access_log` gagal (mis. DB connection pool exhausted)
- **THEN** operasi `POST /me/documents` gagal dengan error **500 Internal Server Error**
  dan `tracing::error!` dicatat — bukan silent discard

---

### Requirement: CSV export mencegah formula injection (CWE-1236)

Sistem SHALL memproteksi CSV export terhadap **CSV formula injection** (CWE-1236). Setiap
cell yang diawali karakter formula (`=`, `+`, `-`, `@`) SHALL diprefix dengan tab (`\t`)
sebelum ditulis ke CSV.

**CLAUDEMD Ref:** §2 — "Zero Security Issue"

#### Scenario: Full name diawali karakter formula
- **WHEN** pengguna memiliki `full_name: "=cmd|'/C calc'!A0"`, dan admin mengekspor CSV
- **THEN** cell di CSV berisi `"\t=cmd|'/C calc'!A0"` — Excel tidak mengeksekusi formula
