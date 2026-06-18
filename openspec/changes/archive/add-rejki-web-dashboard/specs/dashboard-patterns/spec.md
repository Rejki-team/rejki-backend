## ADDED Requirements

### Requirement: Tabel data server-side

Sistem klien SHALL menampilkan data dalam tabel yang mendukung paginasi, pencarian, pengurutan, dan pemfilteran di sisi server. Setiap perubahan halaman, pencarian, atau urutan SHALL memicu permintaan baru ke backend. Sistem klien SHALL menampilkan indikator pemuatan saat data sedang diambil.

#### Scenario: Admin berpindah halaman
- **WHEN** admin mengklik halaman berikutnya pada paginasi
- **THEN** sistem klien meminta data halaman tersebut dari server dan menampilkannya

#### Scenario: Admin mencari data
- **WHEN** admin mengetik kata kunci pada input pencarian
- **THEN** sistem klien mengirim pencarian ke server dan menampilkan hasil yang cocok

### Requirement: Masking dan click-to-view data sensitif

Sistem klien SHALL menampilkan data sensitif (NIK, Foto KTP, Foto Selfie) dalam keadaan ter-mask secara default. Admin SHALL dapat melihat data asli dengan mengklik elemen tersebut, yang SHALL memicu pencatatan akses di backend. Setelah ditutup, data SHALL kembali ke keadaan ter-mask.

#### Scenario: Admin melihat NIK ter-mask
- **WHEN** admin membuka detail pengguna
- **THEN** NIK ditampilkan sebagai karakter ter-mask (contoh: `xxxx...1234`)

#### Scenario: Admin mengklik untuk melihat data asli
- **WHEN** admin mengklik data ter-mask
- **THEN** data asli ditampilkan dan akses dicatat ke backend

### Requirement: Pop-up foto

Sistem klien SHALL menyediakan tampilan pop-up (modal atau lightbox) untuk melihat foto dengan jelas saat diklik dari tabel.

#### Scenario: Admin melihat foto pada pop-up
- **WHEN** admin mengklik foto pada kolom tabel
- **THEN** foto ditampilkan dalam ukuran besar pada pop-up

### Requirement: Progress loading dan pop-up status

Sistem klien SHALL menampilkan indikator pemuatan (skeleton atau spinner) saat data sedang diproses. Setelah proses selesai, sistem klien SHALL menampilkan pop-up status yang menginformasikan keberhasilan atau kegagalan.

#### Scenario: Loading saat fetch data
- **WHEN** halaman pertama kali memuat data
- **THEN** skeleton loader ditampilkan menggantikan tabel hingga data tersedia

#### Scenario: Pop-up hasil aksi
- **WHEN** admin menyelesaikan aksi (mis. menyetujui pengguna)
- **THEN** pop-up status muncul menginformasikan hasil (berhasil atau gagal)

### Requirement: Bulk select untuk aksi massal

Sistem klien SHALL menyediakan checkbox pada setiap baris dan checkbox pilih-semua untuk memilih beberapa item sekaligus. Setelah item dipilih, tombol aksi massal (mis. Suspend) SHALL tersedia.

#### Scenario: Admin memilih beberapa item untuk di-suspend
- **WHEN** admin menceklis beberapa iklan dan menekan tombol Suspend
- **THEN** modal muncul untuk mengisi alasan dan mengunggah bukti sebelum mengirim permintaan massal

### Requirement: Ekspor CSV

Sistem klien SHALL menyediakan tombol ekspor CSV yang memicu unduhan berkas CSV dari backend sesuai filter dan pencarian yang sedang aktif.

#### Scenario: Admin mengekspor data
- **WHEN** admin menekan tombol "Export CSV"
- **THEN** berkas CSV terunduh dengan data sesuai filter yang sedang aktif
