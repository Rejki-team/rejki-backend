#!/bin/sh
# rejki-backend — MinIO Initialization Script
# Dijalankan sekali setelah container minio pertama start untuk:
#   1. Setup mc alias
#   2. Buat bucket: rejki-dokumen (file upload), rejki-backups (DB backup)
#   3. Set policy: private
#   4. Setup bucket notification untuk scan-worker
#
# Env vars:
#   MINIO_ENDPOINT, MINIO_ACCESS_KEY, MINIO_SECRET_KEY (MinIO root creds)
#   MINIO_BUCKET (default: rejki-dokumen)
#   S3_BACKUP_BUCKET (default: rejki-backups)
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

: "${MINIO_ENDPOINT:=http://minio:9000}"
: "${MINIO_ACCESS_KEY:=rejkiadmin}"
: "${MINIO_SECRET_KEY:=rejkiadmin123}"
: "${MINIO_BUCKET:=rejki-dokumen}"
: "${S3_BACKUP_BUCKET:=rejki-backups}"
: "${REDIS_ADDR:=redis:6379}"

# Tunggu MinIO siap
echo "Menunggu MinIO di ${MINIO_ENDPOINT}..."
for i in $(seq 1 30); do
    if curl -sf "${MINIO_ENDPOINT}/minio/health/live" > /dev/null 2>&1; then
        echo "MinIO siap setelah ${i}s"
        break
    fi
    if [ "$i" -eq 30 ]; then
        echo "ERROR: Timeout menunggu MinIO"
        exit 1
    fi
    sleep 1
done

# Setup mc alias
mc alias set local "${MINIO_ENDPOINT}" "${MINIO_ACCESS_KEY}" "${MINIO_SECRET_KEY}"

# Buat bucket (ignore-existing = tidak error jika sudah ada)
mc mb "local/${MINIO_BUCKET}" --ignore-existing
mc mb "local/${S3_BACKUP_BUCKET}" --ignore-existing

# Set policy private
mc anonymous set private "local/${MINIO_BUCKET}"
mc anonymous set private "local/${S3_BACKUP_BUCKET}"

echo "✅ MinIO initialized"
echo "   Bucket upload: ${MINIO_BUCKET}"
echo "   Bucket backup: ${S3_BACKUP_BUCKET}"
