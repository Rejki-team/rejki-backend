## ADDED Requirements

### Requirement: Unit test untuk user-service crate, coverage ≥ 85%

Sistem SHALL memiliki unit test inline (`#[cfg(test)] mod tests`) di setiap module user-service:
`domain/entity.rs`, `domain/repository.rs` (trait — no test needed), `application/dto.rs`,
`application/service.rs`, `infrastructure/pg_repository.rs`, `interface/handlers.rs`.

Target coverage sesuai **Acceptance Criteria §2** dan **CLAUDE.md §4.8**:
- **Overall coverage ≥ 85%**
- Application layer ≥ 80%
- Interface layer ≥ 70%
- Infrastructure layer ≥ 60%

**CLAUDEMD Ref:** §2 — "Menerapkan Unit Testing minimal overall coverage ≥ 85%",
§4.8 — "Unit test murni inline `#[cfg(test)]`; integration test di `tests/` pakai DB nyata"

#### Scenario: Domain entity unit test
- **WHEN** `cargo test -p user-service` dijalankan
- **THEN** test `KycSubmissionStatus::from_str`, `as_str`, `DocumentAccessAction::as_str`, `ReviewError` display terverifikasi (≥ 10 assertions)

#### Scenario: DTO validation unit test
- **WHEN** `cargo test -p user-service` dijalankan
- **THEN** test `KycPersonalDataInput` validasi (NIK length, NIK digit-only, required fields), `AdminKycListQuery` defaults, `CommitDocumentInput` validation terverifikasi

#### Scenario: Service layer unit test (mocked repository)
- **WHEN** `cargo test -p user-service` dijalankan
- **THEN** test `submit_kyc` (success flow, NIK immutable rejection, cooldown rejection, region validation failure), `review_kyc` (approve, reject, already reviewed, not found), `commit_document` (valid, invalid mime, IDOR prefix), `admin_list_submissions` (pagination, search, default params) terverifikasi

#### Scenario: Handler unit test
- **WHEN** `cargo test -p user-service` dijalankan
- **THEN** test `submit_kyc` handler (201 created, 422 validation, 401 unauth), `admin_list_kyc` (200 with meta, 403 non-admin, 401 anon), `admin_get_document` (404 not found, 422 invalid kind, 200 OK) terverifikasi

#### Scenario: Repository unit test (SQLite in-memory)
- **WHEN** `cargo test -p user-service` dijalankan
- **THEN** test `find_by_id` (existing, non-existing), `create` (success), `update_profile` (NIK guard, normal update), `review_submission` (pending → approved, already terminal), `admin_list_submissions` (with search, without search, pagination) terverifikasi

#### Scenario: Coverage measurement
- **WHEN** `cargo llvm-cov -p user-service` atau `cargo tarpaulin -p user-service` dijalankan
- **THEN** overall line coverage ≥ 85% dilaporkan

---
