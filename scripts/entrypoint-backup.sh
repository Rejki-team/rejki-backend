#!/bin/sh
# rejki-backend — DB Backup Container Entrypoint
# Menjalankan cron dengan schedule dari BACKUP_SCHEDULE env var
# Default: 0 3 * * * (setiap hari jam 03:00)
#
# Env vars:
#   BACKUP_SCHEDULE — cron expression (default: "0 3 * * *")
# ─────────────────────────────────────────────────────────────────────────────
set -e

SCHEDULE="${BACKUP_SCHEDULE:-0 3 * * *}"

# Cek bahwa backup script ada
if [ ! -x /usr/local/bin/backup-db.sh ]; then
    echo "ERROR: /usr/local/bin/backup-db.sh tidak ditemukan atau tidak executable"
    exit 1
fi

# Setup cron
echo "${SCHEDULE} /usr/local/bin/backup-db.sh >> /var/log/backup.log 2>&1" > /etc/crontabs/root
echo "[entrypoint] DB Backup cron scheduled: ${SCHEDULE}"
echo "[entrypoint] Log: /var/log/backup.log"

# Jalankan cron foreground
crond -f -l 2
