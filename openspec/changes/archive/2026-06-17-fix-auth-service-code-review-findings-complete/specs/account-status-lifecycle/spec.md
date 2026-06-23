## MODIFIED Requirements

### Requirement: State machine transition enforced at all mutation points

Sistem SHALL memvalidasi setiap transisi status akun melalui fungsi `can_transition_to()` pada state machine `AccountStatus`. Tidak ada titik mutasi yang boleh mengubah status secara langsung tanpa validasi transisi. Ini berlaku untuk semua call path: `AuthService::transition_status`, `AuthService::set_status_checked`, `AuthClient::set_account_status`, **dan `suspend_one`**.

#### Scenario: Transisi sah berhasil

- **WHEN** status source dan target berada dalam tabel transisi sah
- **THEN** sistem mengizinkan perubahan status

#### Scenario: Transisi tidak sah ditolak

- **WHEN** status source dan target TIDAK berada dalam tabel transisi sah (mis. `PendingVerification -> SuspendedPermanent`)
- **THEN** sistem menolak dengan error "transisi status tidak sah"

#### Scenario: Suspend akun divalidasi terhadap state machine

- **WHEN** admin mencoba menangguhkan akun via `suspend_one` atau `suspend_account`
- **THEN** sistem memeriksa status saat ini dan memvalidasi `can_transition_to(target)` SEBELUM mengeksekusi `set_status`

#### Scenario: Suspend akun dengan status non-Active ditolak

- **WHEN** admin mencoba menangguhkan akun dengan status `PendingVerification`, `ProfileIncomplete`, `PendingKyc`, atau `Rejected`
- **THEN** sistem menolak dengan error transisi tidak sah, tanpa mengubah status
