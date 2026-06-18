## ADDED Requirements

### Requirement: Rate limiter trait and implementation are shared across services
The system SHALL provide a shared `RateLimiter` trait in `common/rate-limit` crate, accessible by all services without dependency on auth-service.

#### Scenario: Any service can import the trait
- **WHEN** a service crate adds `common-rate-limit = { workspace = true }` to its Cargo.toml
- **THEN** the `RateLimiter` trait and `OtpRateLimiter` implementation are available for injection

#### Scenario: Auth-service continues to work unchanged
- **WHEN** auth-service re-exports `RateLimiter` from `common/rate-limit`
- **THEN** all existing auth-service rate limit tests PASS without modification

### Requirement: Rate limiting on iklan create endpoints
The system SHALL enforce rate limits on POST create endpoints for all 4 iklan services (pekerjaan, pekerja, barang-bekas, pelatihan).

#### Scenario: User exceeds create rate limit
- **WHEN** user sends more than 30 POST create requests in 15 minutes for iklan-pekerjaan
- **THEN** the system returns HTTP 429 with `Retry-After` header and error code `RATE_LIMITED`

#### Scenario: User within rate limit creates successfully
- **WHEN** user sends a POST create request within the allowed rate
- **THEN** the system returns HTTP 201 Created with the created resource

### Requirement: Rate limiting on report creation
The system SHALL enforce rate limits on POST /reports endpoint to prevent abuse.

#### Scenario: User exceeds report rate limit
- **WHEN** user sends more than 10 report creation requests in 15 minutes
- **THEN** the system returns HTTP 429 with error code `RATE_LIMITED`

### Requirement: Rate limiting on chat message sending
The system SHALL enforce rate limits on POST /chat/messages to prevent spam.

#### Scenario: User exceeds message rate limit
- **WHEN** user sends more than 30 messages in 1 minute
- **THEN** the system returns HTTP 429 with error code `RATE_LIMITED`

### Requirement: Rate limiting on notification sending
The system SHALL enforce rate limits on POST /notif/send to prevent notification abuse.

#### Scenario: User exceeds notification rate limit
- **WHEN** user sends more than 20 notifications in 1 minute
- **THEN** the system returns HTTP 429 with error code `RATE_LIMITED`

### Requirement: Rate limiting on article creation
The system SHALL enforce rate limits on POST /admin/articles create endpoint.

#### Scenario: Admin exceeds article creation rate limit
- **WHEN** admin creates more than 10 articles in 15 minutes
- **THEN** the system returns HTTP 429 with error code `RATE_LIMITED`

### Requirement: Global IP-based rate limit defense-in-depth
The system SHALL enforce a global rate limit of 100 requests per minute per IP address as a defense-in-depth layer at the rejki-app level.

#### Scenario: IP exceeds global rate limit
- **WHEN** any IP sends more than 100 requests in 1 minute to any endpoint
- **THEN** the system returns HTTP 429 with error code `RATE_LIMITED`

#### Scenario: Health check is exempt from global rate limit
- **WHEN** a request is made to GET /health
- **THEN** the rate limiter does not count this request

### Requirement: Rate limiter fail-open behavior
The system SHALL allow all requests when Redis is unavailable, logging a warning.

#### Scenario: Redis is down
- **WHEN** Redis connection fails or REDIS_URL is not configured
- **THEN** the rate limiter returns `true` (allow) for all requests and logs a warning

### Requirement: Response format for rate-limited requests
The system SHALL return a standardized error response for rate-limited requests.

#### Scenario: Rate-limited response format
- **WHEN** a request is rate-limited
- **THEN** the response body contains `{"success": false, "error": {"code": "RATE_LIMITED", "message": "..."}}` with HTTP 429 and `Retry-After` header
