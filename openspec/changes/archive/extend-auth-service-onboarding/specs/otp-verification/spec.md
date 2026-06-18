## ADDED Requirements

### Requirement: OTP berbasis purpose dengan satu mekanisme penyimpanan

Sistem SHALL menggunakan satu mekanisme OTP yang dibedakan oleh `purpose` (`register`, `reset_password`, `change_password`). Sistem SHALL menyimpan OTP dalam bentuk ter-hash (bukan plaintext) dan SHALL hanya menyimpan satu OTP aktif per kombinasi pengguna dan `purpose` (pembangkitan baru menggantikan yang lama).

#### Scenario: Pembangkitan OTP untuk sebuah purpose
- **WHEN** sistem membangkitkan OTP untuk pengguna dengan purpose tertentu
- **THEN** sistem menyimpan nilai ter-hash beserta waktu kedaluwarsa, menggantikan OTP aktif sebelumnya untuk pasangan pengguna+purpose yang sama

#### Scenario: Verifikasi OTP yang benar
- **WHEN** pengguna mengirim OTP yang cocok untuk purpose terkait sebelum kedaluwarsa
- **THEN** sistem menerima OTP, menandainya terpakai, dan menjalankan aksi sesuai purpose

#### Scenario: Verifikasi OTP yang salah atau kedaluwarsa
- **WHEN** pengguna mengirim OTP yang salah atau telah melewati masa kedaluwarsa
- **THEN** sistem menolak verifikasi tanpa menjalankan aksi terkait

### Requirement: Kedaluwarsa, batas percobaan, dan rate limit OTP

Sistem SHALL menetapkan masa kedaluwarsa OTP yang pendek. Sistem SHALL membatasi jumlah percobaan verifikasi dan frekuensi permintaan OTP per pengguna untuk mencegah brute-force dan penyalahgunaan.

#### Scenario: Melebihi batas percobaan verifikasi
- **WHEN** pengguna melampaui batas percobaan verifikasi OTP yang diizinkan
- **THEN** sistem menolak percobaan berikutnya untuk periode yang ditentukan dan tidak mengungkap OTP yang benar

#### Scenario: Melebihi batas permintaan OTP
- **WHEN** pengguna meminta OTP melebihi frekuensi yang diizinkan dalam jendela waktu
- **THEN** sistem menolak permintaan tambahan hingga jendela rate limit berlalu

### Requirement: Pengiriman OTP melalui notification-service

Sistem SHALL mengirim OTP ke email pengguna melalui notification-service. Sistem SHALL TIDAK mengembalikan nilai OTP dalam respons API maupun mencatatnya sebagai log permanen.

#### Scenario: OTP dikirim via notification-service
- **WHEN** sebuah OTP perlu dikirim ke pengguna
- **THEN** sistem mendelegasikan pengiriman email ke notification-service dan tidak menyertakan nilai OTP pada respons API

#### Scenario: OTP tidak bocor melalui kanal yang tidak semestinya
- **WHEN** sebuah OTP dibangkitkan
- **THEN** nilai OTP tidak muncul pada body respons API maupun log aplikasi permanen
