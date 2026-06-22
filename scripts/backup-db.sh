#!/bin/bash
# rejki-backend — DB Backup Script
# pg_dump → dual destination: host lokal (safety net) + MinIO (primary)
# 3-2-1 Backup Rule: 3 copies, 2 media, 1 off-container (MinIO volume)
#
# Dijadwalkan via cron di container db-backup (Dockerfile.backup)
# Schedule default: 0 3 * * * (setiap hari jam 3 pagi WIB)
#
# Env vars:
#   PGUSER, PGPASSWORD, PGDATABASE, PGHOST (PostgreSQL connection)
#   MINIO_ENDPOINT, MINIO_ACCESS_KEY, MINIO_SECRET_KEY (MinIO S3)
#   S3_BACKUP_BUCKET (default: rejki-backups)
#   LOCAL_BACKUP_DIR (default: /backups)
#   BACKUP_RETENTION_LOCAL (default: 7 hari)
#   BACKUP_RETENTION_S3   (default: 30 hari)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

# ── Configuration ────────────────────────────────────────────────────────────
: "${PGUSER:=rejki}"
: "${PGPASSWORD:=secret}"
: "${PGDATABASE:=rejki_db}"
: "${PGHOST:=postgres}"
: "${MINIO_ENDPOINT:=http://minio:9000}"
: "${MINIO_ACCESS_KEY:=rejkiadmin}"
: "${MINIO_SECRET_KEY:=rejkiadmin123}"
: "${S3_BACKUP_BUCKET:=rejki-backups}"
: "${LOCAL_BACKUP_DIR:=/backups}"
: "${BACKUP_RETENTION_LOCAL:=7}"
: "${BACKUP_RETENTION_S3:=30}"

# ── Helpers ──────────────────────────────────────────────────────────────────
TIMESTAMP=$(date +%Y%m%d-%H%M%S)
BACKUP_FILE="rejki-pg-${TIMESTAMP}.dump.gz"
TMP_PATH="/tmp/${BACKUP_FILE}"

export PGPASSWORD

log_info()  { echo "[$(date +%H:%M:%S)] INFO  $*"; }
log_error() { echo "[$(date +%H:%M:%S)] ERROR $*" >&2; }
log_warn()  { echo "[$(date +%H:%M:%S)] WARN  $*"; }

# ── 1. pg_dump ──────────────────────────────────────────────────────────────
log_info "Memulai backup database '${PGDATABASE}'..."

if ! pg_dump -h "$PGHOST" -U "$PGUSER" -d "$PGDATABASE" -Fc 2>/dev/null | gzip > "$TMP_PATH"; then
    log_error "pg_dump gagal — database '${PGDATABASE}' tidak bisa di-backup"
    exit 1
fi

FILE_SIZE=$(stat -c%s "$TMP_PATH" 2>/dev/null || stat -f%z "$TMP_PATH" 2>/dev/null || echo "?")
log_info "pg_dump selesai: ${FILE_SIZE} bytes"

# ── 2. Host-mounted volume (safety net) ─────────────────────────────────────
mkdir -p "${LOCAL_BACKUP_DIR}/daily"
cp "$TMP_PATH" "${LOCAL_BACKUP_DIR}/daily/${BACKUP_FILE}"
log_info "Backup lokal     : ${LOCAL_BACKUP_DIR}/daily/${BACKUP_FILE}"

# ── 3. Upload ke MinIO (S3-compatible) ──────────────────────────────────────
if AWS_ACCESS_KEY_ID="$MINIO_ACCESS_KEY" \
   AWS_SECRET_ACCESS_KEY="$MINIO_SECRET_KEY" \
   /usr/bin/aws --endpoint-url "$MINIO_ENDPOINT" s3 cp \
       "$TMP_PATH" "s3://${S3_BACKUP_BUCKET}/daily/${BACKUP_FILE}" \
       --only-show-errors 2>/dev/null; then
    log_info "Upload ke MinIO  : s3://${S3_BACKUP_BUCKET}/daily/${BACKUP_FILE}"
else
    log_warn "Upload ke MinIO gagal — backup hanya tersedia di lokal"
fi

# ── 4. Hapus file temp ─────────────────────────────────────────────────────
rm -f "$TMP_PATH"

# ── 5. Cleanup lokal (host-mounted) > RETENTION_DAYS ────────────────────────
LOCAL_CUTOFF=$(date -d "-${BACKUP_RETENTION_LOCAL} days" +%s 2>/dev/null || echo "")
if [ -n "$LOCAL_CUTOFF" ]; then
    find "${LOCAL_BACKUP_DIR}/daily" -name "rejki-pg-*.dump.gz" -type f | while read -r f; do
        FILE_MTIME=$(stat -c%Y "$f" 2>/dev/null || stat -f%m "$f" 2>/dev/null || echo "0")
        if [ "${FILE_MTIME:-0}" -lt "$LOCAL_CUTOFF" ] 2>/dev/null; then
            rm -f "$f"
            log_info "Cleanup lokal     : ${f##*/} (expired)"
        fi
    done
fi

# ── 6. Cleanup MinIO > RETENTION_S3 ─────────────────────────────────────────
AWS_ACCESS_KEY_ID="$MINIO_ACCESS_KEY" \
AWS_SECRET_ACCESS_KEY="$MINIO_SECRET_KEY" \
/usr/bin/aws --endpoint-url "$MINIO_ENDPOINT" s3 ls \
    "s3://${S3_BACKUP_BUCKET}/daily/" --output text 2>/dev/null \
    | while read -r _ date_str time_str _ _ key; do
        if [ -z "$key" ]; then continue; fi
        FILE_TS=$(date -d "${date_str} ${time_str}" +%s 2>/dev/null || echo "0")
        S3_CUTOFF=$(date -d "-${BACKUP_RETENTION_S3} days" +%s 2>/dev/null || echo "0")
        if [ "${FILE_TS:-0}" -lt "${S3_CUTOFF:-0}" ] 2>/dev/null; then
            AWS_ACCESS_KEY_ID="$MINIO_ACCESS_KEY" \
            AWS_SECRET_ACCESS_KEY="$MINIO_SECRET_KEY" \
            /usr/bin/aws --endpoint-url "$MINIO_ENDPOINT" s3 rm \
                "s3://${S3_BACKUP_BUCKET}/daily/${key##*/}" --only-show-errors 2>/dev/null \
            && log_info "Cleanup MinIO     : ${key##*/} (expired >${BACKUP_RETENTION_S3} hari)"
        fi
    done

log_info "Backup selesai ✅"
