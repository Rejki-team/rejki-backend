# ─────────────────────────────────────────────────────────────────────────────
# bun-notification-service — Dockerfile
# Multi-stage: deps → runtime
# ─────────────────────────────────────────────────────────────────────────────

# ── Stage 1: Install dependencies ────────────────────────────────────────────
FROM oven/bun:1-alpine AS deps

WORKDIR /app

COPY package.json bun.lockb* ./
RUN bun install --frozen-lockfile --production

# ── Stage 2: Runtime ──────────────────────────────────────────────────────────
FROM oven/bun:1-alpine

WORKDIR /app

# Non-root user
RUN addgroup -S rejki && adduser -S -G rejki rejki

COPY --from=deps /app/node_modules ./node_modules
COPY src/       ./src/
COPY migrations/ ./migrations/

# Direktori keys akan di-mount via volume
RUN mkdir -p /app/keys && chown -R rejki:rejki /app

USER rejki

ENV NODE_ENV=production

EXPOSE 9000

HEALTHCHECK --interval=15s --timeout=5s --start-period=15s --retries=3 \
    CMD wget -qO- http://localhost:9000/health || exit 1

CMD ["bun", "run", "src/index.ts"]
