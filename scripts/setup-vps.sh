#!/usr/bin/env bash
# ─────────────────────────────────────────────────────────────────────────────
# Rejki Backend — One-time VPS Setup Script
# Menyiapkan: Docker, shared infrastructure, user, directory structure
# JALANKAN SEBAGAI ROOT: sudo bash scripts/setup-vps.sh
# ─────────────────────────────────────────────────────────────────────────────
set -euo pipefail

RED='\033[0;31m'; GREEN='\033[0;32m'; YELLOW='\033[1;33m'; NC='\033[0m'
log()  { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
err()  { echo -e "${RED}[✗]${NC} $1"; exit 1; }

# ── 1. Cek root ──────────────────────────────────────────────────────────────
if [[ $EUID -ne 0 ]]; then err "Jalankan sebagai root (sudo)"; fi

# ── 2. System update & packages ──────────────────────────────────────────────
log "System update..."
apt-get update -qq && apt-get upgrade -y -qq

log "Install dependencies..."
apt-get install -y -qq \
    ca-certificates curl gnupg lsb-release \
    ufw git openssl

# ── 3. Install Docker (jika belum ada) ───────────────────────────────────────
if ! command -v docker &>/dev/null; then
    log "Install Docker..."
    install -m 0755 -d /etc/apt/keyrings
    curl -fsSL https://download.docker.com/linux/ubuntu/gpg | \
        gpg --dearmor -o /etc/apt/keyrings/docker.gpg
    echo "deb [arch=$(dpkg --print-architecture) signed-by=/etc/apt/keyrings/docker.gpg] \
        https://download.docker.com/linux/ubuntu $(lsb_release -cs) stable" | \
        tee /etc/apt/sources.list.d/docker.list > /dev/null
    apt-get update -qq && apt-get install -y -qq docker-ce docker-ce-cli containerd.io docker-compose-plugin
    systemctl enable --now docker
    log "Docker installed"
else
    log "Docker already installed"
fi

# ── 4. Create user `rejki` ───────────────────────────────────────────────────
if id "rejki" &>/dev/null; then
    log "User rejki already exists"
else
    useradd -m -d /opt/rejki -s /bin/bash rejki
    log "User rejki created (home: /opt/rejki)"
fi

# Tambahkan rejki ke grup docker agar bisa manage container
usermod -aG docker rejki

# ── 5. Directory structure ───────────────────────────────────────────────────
log "Create directory structure..."
mkdir -p /opt/rejki/{dev,prod,keys,scripts}
mkdir -p /opt/rejki/dev/rejki-backend
chown -R rejki:rejki /opt/rejki

# ── 6. Setup SSH untuk deploy ───────────────────────────────────────────────
log "Setup SSH directory..."
mkdir -p /opt/rejki/.ssh
chmod 700 /opt/rejki/.ssh
touch /opt/rejki/.ssh/authorized_keys
chmod 600 /opt/rejki/.ssh/authorized_keys
chown -R rejki:rejki /opt/rejki/.ssh

warn "=== ACTION REQUIRED ==="
warn "1. Tambahkan public key GitHub Actions ke: /opt/rejki/.ssh/authorized_keys"
warn "2. Generate JWT key pairs:"
warn "   cd /opt/rejki/keys && openssl genrsa -out private.pem 2048"
warn "   openssl rsa -in private.pem -pubout -out public.pem"
warn "3. Generate encryption key: openssl rand -base64 32"
warn "4. Setup .env files di /opt/rejki/dev/ dan /opt/rejki/prod/"
warn "5. Clone repo: git clone <repo-url> /opt/rejki/dev/rejki-backend"
warn "6. Start infrastructure: docker compose -f docker-compose.infra.yml up -d"
warn "7. Setup Cloudflare Tunnel untuk production (lihat dokumentasi)"

# ── 7. UFW Firewall ─────────────────────────────────────────────────────────
log "Configure UFW firewall..."
ufw --force reset
ufw default deny incoming
ufw default allow outgoing

# Hanya buka SSH (port standar, bisa diganti)
ufw allow 22/tcp comment 'SSH'

# Untuk development: bisa akses langsung via IP (jika perlu)
# ufw allow 8081/tcp comment 'Dev API'

# Production: hanya via Cloudflare Tunnel (tidak perlu buka port 80/443)
# Cloudflare Tunnel konek ke internal, jadi tidak perlu port terbuka

ufw --force enable
log "UFW firewall enabled (only SSH open)"

# ── 8. Kernel tweaks untuk VPS 2GB ──────────────────────────────────────────
log "Apply kernel tweaks..."
cat >> /etc/sysctl.conf <<'EOF'

# Rejki Backend — VPS 2GB optimizations
net.core.somaxconn = 1024
net.ipv4.tcp_fastopen = 3
net.ipv4.tcp_tw_reuse = 1
vm.swappiness = 10
vm.vfs_cache_pressure = 50
EOF
sysctl -p

# ── 9. Done ──────────────────────────────────────────────────────────────────
log "=========================================="
log "VPS Setup Complete!"
log "=========================================="
log ""
log "Next steps:"
log "  1. cd /opt/rejki/dev/rejki-backend"
log "  2. cp .env.dev.example /opt/rejki/dev/.env && nano /opt/rejki/dev/.env"
log "  3. cp .env.prod.example /opt/rejki/prod/.env && nano /opt/rejki/prod/.env"
log "  4. docker compose -f docker-compose.infra.yml up -d"
log "  5. docker compose -f docker-compose.dev.yml -p rejki-dev up -d"
log "  6. docker compose -f docker-compose.prod.yml -p rejki-prod --profile prod up -d"
log ""
log "Lihat dokumentasi lengkap: docs/deployment-guide.html"
