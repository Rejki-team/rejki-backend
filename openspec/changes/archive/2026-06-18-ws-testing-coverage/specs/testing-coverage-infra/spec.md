## ADDED Requirements

### Requirement: CI coverage measurement with llvm-cov
The CI pipeline SHALL include a `coverage` job that measures overall line coverage using `cargo-llvm-cov`, outputs `lcov.info`, and enforces a configurable threshold gate (≥85% target, soft gate at lower thresholds for progressive rollout).

#### Scenario: Coverage job runs on PR
- **WHEN** a pull request is opened against develop or main
- **THEN** the `coverage` job runs after `check` succeeds, measuring workspace-wide coverage with Postgres service container

#### Scenario: Coverage gate fails below threshold
- **WHEN** overall line coverage drops below the configured threshold
- **THEN** the CI step exits with code 1, blocking the PR

#### Scenario: Coverage artifact is uploaded
- **WHEN** the coverage job completes
- **THEN** an `lcov.info` artifact is uploaded with 7-day retention

### Requirement: Unit test coverage for all service crates
Every service crate in the workspace SHALL have unit tests at the application layer using mock repositories and mock client traits, covering create, update, delete, IDOR (ownership check returning 404), and admin operations.

#### Scenario: Application layer unit tests exist per service
- **WHEN** `cargo test --workspace --lib` is executed
- **THEN** all 12 service crates (auth, user, chat, notification, region, storage, iklan-pekerjaan, iklan-pekerja, iklan-barang-bekas, iklan-pelatihan, corporate-comms, report) have unit tests passing

#### Scenario: UserService is tested with full mock stack
- **WHEN** a unit test submits KYC personal data
- **THEN** MockUserRepository + MockTxUserRepository + MockAuthClient + MockRegionClient + MockStorageClient + MockNotificationClient simulate the full transaction boundary without a real database

### Requirement: All integration test files run in CI
The CI pipeline SHALL execute ALL integration test files (`cargo test --workspace --test '*'`), not just a single file, using a real PostgreSQL service container with all 12 schemas initialized.

#### Scenario: All 5 integration test files pass
- **WHEN** `cargo test --workspace --test '*' -- --test-threads=1` runs in CI
- **THEN** all test files (auth_integration_test, barang_gratis_test, bulk_suspend_test, user_admin_kyc_test, user_code_review_fix_test) pass with zero failures

#### Scenario: Dynamic schema initialization covers all 12 services
- **WHEN** the CI integration-test job runs migrations
- **THEN** all 12 PostgreSQL schemas (auth, user_svc, chat, notification, iklan_pekerjaan, iklan_pekerja, iklan_barang_bekas, iklan_pelatihan, region, corporate_comms, report, storage) are created using a dynamic loop from migration folders
