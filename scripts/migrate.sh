#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Rejki Backend — Run Migrations
# Usage: bash scripts/migrate.sh <dev|prod>
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

ENV="${1:-}"
if [[ -z "$ENV" ]]; then
    echo "Usage: bash scripts/migrate.sh <dev|prod>"
    exit 1
fi

case "$ENV" in
    dev)
        COMPOSE_DIR="/opt/rejki/dev"
        DATABASE_URL_ENV="DATABASE_URL=postgres://rejki:\${POSTGRES_PASSWORD}@postgres:5432/rejki_dev"
        ;;
    prod)
        COMPOSE_DIR="/opt/rejki/prod"
        DATABASE_URL_ENV="DATABASE_URL=postgres://rejki:\${POSTGRES_PASSWORD}@postgres:5432/rejki_prod"
        ;;
    *)
        echo "Unknown environment: $ENV (use dev or prod)"
        exit 1
        ;;
esac

if [[ ! -f "$COMPOSE_DIR/.env" ]]; then
    echo "Error: $COMPOSE_DIR/.env not found!"
    echo "Copy template: cp .env.$ENV.example $COMPOSE_DIR/.env"
    exit 1
fi

cd "$COMPOSE_DIR/rejki-backend/rust-services"

# Export env vars dari .env
set -a
source "$COMPOSE_DIR/.env"
set +a

echo "==> Running migrations for $ENV environment..."
echo "    Database: $DATABASE_URL"

# Map service directory ke schema name
map_schema() {
    case "$1" in
        user-service)            echo "user_svc" ;;
        corporate-comms-service) echo "comms" ;;
        *)                       echo "$1" | sed 's/-service//' | sed 's/-/_/g' ;;
    esac
}

# Run migrations untuk setiap service
for dir in *-service/migrations; do
    if [[ ! -d "$dir" ]]; then continue; fi
    svc=$(basename "$(dirname "$dir")")
    schema=$(map_schema "$svc")
    echo "  → $svc (schema: $schema)"

    # Buat schema jika belum ada
    psql "$DATABASE_URL" -c "CREATE SCHEMA IF NOT EXISTS $schema" 2>/dev/null

    # Run sqlx migration
    sqlx migrate run \
        --source "$dir" \
        --database-url "$DATABASE_URL"
done

echo "==> Migrations complete for $ENV!"
