## ADDED Requirements

### Requirement: CORS production whitelist

Sistem SHALL menyediakan CORS policy yang berbeda untuk development dan production:
- **Development**: `allow_origin(Any)` — longgar untuk frontend lokal
- **Production**: whitelist origin dari env var `CORS_ALLOWED_ORIGINS` — ketat

#### Scenario: Development — origin any diizinkan
- **GIVEN** `APP_ENV=development`
- **WHEN** request dari origin `http://localhost:5173` atau origin lain
- **THEN** CORS header `Access-Control-Allow-Origin` di-set ke `*`

#### Scenario: Production — only whitelisted origin
- **GIVEN** `APP_ENV=production` dengan `CORS_ALLOWED_ORIGINS=https://rejki.id,https://app.rejki.id`
- **WHEN** request dari origin `https://rejki.id`
- **THEN** response memiliki header `Access-Control-Allow-Origin: https://rejki.id`

#### Scenario: Production — origin tidak dikenal ditolak
- **GIVEN** `APP_ENV=production` dengan whitelist `https://rejki.id`
- **WHEN** request dari origin `https://evil-site.com`
- **THEN** response TIDAK memiliki header `Access-Control-Allow-Origin`

### Requirement: allow_credentials production-only

Sistem SHALL mengaktifkan `allow_credentials(true)` hanya di production (kompatibel dengan whitelist origin). Development tidak mengaktifkan credentials karena tidak kompatibel dengan `allow_origin(Any)`.

#### Scenario: Production — credentials diizinkan
- **GIVEN** `APP_ENV=production`
- **WHEN** request dari whitelisted origin dengan `credentials: include`
- **THEN** response memiliki header `Access-Control-Allow-Credentials: true`

#### Scenario: Development — credentials tidak diizinkan
- **GIVEN** `APP_ENV=development`
- **WHEN** request dari origin manapun
- **THEN** response memiliki `Access-Control-Allow-Origin: *` tanpa `Access-Control-Allow-Credentials`
