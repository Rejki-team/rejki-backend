# ─────────────────────────────────────────────────────────────────────────────
# Rejki Backend — Dockerfile untuk rejki-app (Rust Modular Monolith)
# Build context : rust-services/   (lihat docker-compose.yml)
# Multi-stage   : builder → runtime
# Final image   : debian:bookworm-slim, non-root user rejki
# ─────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Builder ──────────────────────────────────────────────────────────
FROM rust:1.82-bookworm AS builder

WORKDIR /app

# Install sqlx-cli untuk offline mode (tidak diperlukan saat runtime)
# dan dependency sistem untuk openssl
RUN apt-get update && apt-get install -y --no-install-recommends \
    pkg-config \
    libssl-dev \
    && rm -rf /var/lib/apt/lists/*

# Copy workspace manifest dulu untuk cache dependency layer
COPY Cargo.toml Cargo.lock ./

# Copy semua Cargo.toml crate (dibutuhkan resolver workspace)
COPY common/config/Cargo.toml         common/config/Cargo.toml
COPY common/errors/Cargo.toml         common/errors/Cargo.toml
COPY common/tracing-setup/Cargo.toml  common/tracing-setup/Cargo.toml
COPY common/auth-middleware/Cargo.toml common/auth-middleware/Cargo.toml
COPY auth-service/Cargo.toml          auth-service/Cargo.toml
COPY auth-service-client/Cargo.toml   auth-service-client/Cargo.toml
COPY user-service/Cargo.toml          user-service/Cargo.toml
COPY user-service-client/Cargo.toml   user-service-client/Cargo.toml
COPY chat-service/Cargo.toml          chat-service/Cargo.toml
COPY chat-service-client/Cargo.toml   chat-service-client/Cargo.toml
COPY notification-service/Cargo.toml  notification-service/Cargo.toml
COPY notification-service-client/Cargo.toml notification-service-client/Cargo.toml
COPY iklan-pekerjaan-service/Cargo.toml     iklan-pekerjaan-service/Cargo.toml
COPY iklan-pekerjaan-service-client/Cargo.toml iklan-pekerjaan-service-client/Cargo.toml
COPY iklan-pekerja-service/Cargo.toml       iklan-pekerja-service/Cargo.toml
COPY iklan-pekerja-service-client/Cargo.toml iklan-pekerja-service-client/Cargo.toml
COPY iklan-barang-bekas-service/Cargo.toml  iklan-barang-bekas-service/Cargo.toml
COPY iklan-barang-bekas-service-client/Cargo.toml iklan-barang-bekas-service-client/Cargo.toml
COPY iklan-pelatihan-service/Cargo.toml     iklan-pelatihan-service/Cargo.toml
COPY iklan-pelatihan-service-client/Cargo.toml iklan-pelatihan-service-client/Cargo.toml
COPY corporate-comms-service/Cargo.toml       corporate-comms-service/Cargo.toml
COPY corporate-comms-service-client/Cargo.toml corporate-comms-service-client/Cargo.toml
COPY report-service/Cargo.toml               report-service/Cargo.toml
COPY report-service-client/Cargo.toml         report-service-client/Cargo.toml
COPY rejki-app/Cargo.toml             rejki-app/Cargo.toml

# Buat src dummy agar cargo bisa fetch dependency tanpa source asli
# Ini memaksimalkan cache layer Docker
RUN mkdir -p common/config/src common/errors/src common/tracing-setup/src common/auth-middleware/src \
    auth-service/src auth-service-client/src \
    user-service/src user-service-client/src \
    chat-service/src chat-service-client/src \
    notification-service/src notification-service-client/src \
    iklan-pekerjaan-service/src iklan-pekerjaan-service-client/src \
    iklan-pekerja-service/src iklan-pekerja-service-client/src \
    iklan-barang-bekas-service/src iklan-barang-bekas-service-client/src \
    iklan-pelatihan-service/src iklan-pelatihan-service-client/src \
    corporate-comms-service/src corporate-comms-service-client/src \
    report-service/src report-service-client/src \
    rejki-app/src \
    && echo "fn main() {}" | tee \
        auth-service/src/main.rs user-service/src/main.rs \
        chat-service/src/main.rs notification-service/src/main.rs \
        iklan-pekerjaan-service/src/main.rs iklan-pekerja-service/src/main.rs \
        iklan-barang-bekas-service/src/main.rs iklan-pelatihan-service/src/main.rs \
        corporate-comms-service/src/main.rs \
        report-service/src/main.rs \
        rejki-app/src/main.rs \
    && for d in common/config common/errors common/tracing-setup common/auth-middleware \
        auth-service-client user-service-client chat-service-client \
        notification-service-client iklan-pekerjaan-service-client \
        iklan-pekerja-service-client iklan-barang-bekas-service-client \
        iklan-pelatihan-service-client corporate-comms-service-client \
        report-service-client; do \
        echo "" > $d/src/lib.rs; \
    done

# Fetch + compile dependencies (di-cache jika Cargo.toml tidak berubah)
RUN cargo build --release -p rejki-app 2>/dev/null || true

# Copy source asli dan build final
COPY . .

# Touch main.rs agar cargo tahu source berubah (bukan hanya dependency)
RUN touch rejki-app/src/main.rs

RUN cargo build --release -p rejki-app

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM debian:bookworm-slim AS runtime

RUN apt-get update && apt-get install -y --no-install-recommends \
    ca-certificates \
    libssl3 \
    wget \
    && rm -rf /var/lib/apt/lists/*

# Non-root user untuk keamanan
RUN useradd --uid 1001 --no-create-home --shell /bin/false rejki

WORKDIR /app

COPY --from=builder /app/target/release/rejki-app /app/rejki-app

# Pastikan direktori keys ada (akan di-mount via volume)
RUN mkdir -p /app/keys && chown -R rejki:rejki /app

USER rejki

EXPOSE 8080

HEALTHCHECK --interval=15s --timeout=5s --start-period=10s --retries=3 \
    CMD wget -qO- http://localhost:8080/health || exit 1

CMD ["/app/rejki-app"]
