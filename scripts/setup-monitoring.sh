#!/usr/bin/env bash
set -euo pipefail

# =============================================================================
# Aztibase Network — Monitoring Stack Installer
# Target: Ubuntu 22.04 LTS, 1 vCPU / 2 GB RAM / 25 GB NVMe
# Installs: VictoriaMetrics (single-node) + Grafana OSS
# Safe to run multiple times (idempotent).
# =============================================================================

AZTIBASE_METRICS_ADDR="127.0.0.1:9944"
VM_LISTEN_ADDR="127.0.0.1:8428"
VM_DATA_DIR="/var/lib/victoriametrics"
VM_CONFIG_DIR="/etc/victoriametrics"
GRAFANA_DASHBOARDS_DIR="/var/lib/grafana/dashboards"
VPS_PUBLIC_IP="102.209.21.247"

# Generate a random 16-char admin password if this is a fresh install.
# If grafana.ini already has a custom password we leave it alone below.
GRAFANA_ADMIN_PASS="$(head -c 32 /dev/urandom | base64 | tr -dc 'A-Za-z0-9' | head -c 16)"

# =============================================================================
# 0. Preflight
# =============================================================================
echo ""
echo "============================================================"
echo " [0/7] Preflight checks"
echo "============================================================"

if [[ $EUID -ne 0 ]]; then
  echo "ERROR: This script must be run as root." >&2
  exit 1
fi

apt-get update -qq
apt-get install -y -qq curl wget gpg jq ufw

# =============================================================================
# 1. VictoriaMetrics — single-node binary
# =============================================================================
echo ""
echo "============================================================"
echo " [1/7] Installing VictoriaMetrics"
echo "============================================================"

VM_VERSION="$(curl -fsSL https://api.github.com/repos/VictoriaMetrics/VictoriaMetrics/releases/latest \
  | jq -r '.tag_name')"
VM_ARCHIVE="victoria-metrics-linux-amd64-${VM_VERSION}.tar.gz"
VM_DOWNLOAD_URL="https://github.com/VictoriaMetrics/VictoriaMetrics/releases/download/${VM_VERSION}/${VM_ARCHIVE}"

if [[ ! -f /usr/local/bin/victoria-metrics ]]; then
  echo "Downloading VictoriaMetrics ${VM_VERSION}..."
  TMP_DIR="$(mktemp -d)"
  wget -q --show-progress -O "${TMP_DIR}/${VM_ARCHIVE}" "${VM_DOWNLOAD_URL}"
  tar -xzf "${TMP_DIR}/${VM_ARCHIVE}" -C "${TMP_DIR}"
  install -m 755 "${TMP_DIR}/victoria-metrics-prod" /usr/local/bin/victoria-metrics
  rm -rf "${TMP_DIR}"
  echo "VictoriaMetrics ${VM_VERSION} installed."
else
  echo "VictoriaMetrics binary already present at /usr/local/bin/victoria-metrics — skipping download."
fi

# Data and config directories
mkdir -p "${VM_DATA_DIR}" "${VM_CONFIG_DIR}"

# Scrape configuration
cat > "${VM_CONFIG_DIR}/scrape.yml" <<'SCRAPE_EOF'
global:
  scrape_interval: 5s
  scrape_timeout: 4s

scrape_configs:
  - job_name: aztibase-node
    static_configs:
      - targets:
          - "127.0.0.1:9944"
        labels:
          network: "testnet"
          node: "vps1"
    metrics_path: /metrics
SCRAPE_EOF

echo "Scrape config written to ${VM_CONFIG_DIR}/scrape.yml"

# Dedicated system user
if ! id -u victoriametrics &>/dev/null; then
  useradd --system --no-create-home --shell /usr/sbin/nologin victoriametrics
fi
chown -R victoriametrics:victoriametrics "${VM_DATA_DIR}"

# systemd service
cat > /etc/systemd/system/victoriametrics.service <<SYSTEMD_EOF
[Unit]
Description=VictoriaMetrics single-node time-series database
Documentation=https://docs.victoriametrics.com/
After=network.target
Wants=network.target

[Service]
Type=simple
User=victoriametrics
Group=victoriametrics
Restart=on-failure
RestartSec=5s
ExecStart=/usr/local/bin/victoria-metrics \\
  -storageDataPath=${VM_DATA_DIR} \\
  -httpListenAddr=${VM_LISTEN_ADDR} \\
  -promscrape.config=${VM_CONFIG_DIR}/scrape.yml \\
  -retentionPeriod=6 \\
  -memory.allowedPercent=15 \\
  -loggerLevel=INFO
ProtectSystem=full
PrivateTmp=true
NoNewPrivileges=true

[Install]
WantedBy=multi-user.target
SYSTEMD_EOF

systemctl daemon-reload
systemctl enable victoriametrics
systemctl restart victoriametrics

echo "VictoriaMetrics service started (listening on ${VM_LISTEN_ADDR})."

# =============================================================================
# 2. Grafana OSS — official APT repository
# =============================================================================
echo ""
echo "============================================================"
echo " [2/7] Installing Grafana OSS"
echo "============================================================"

if ! dpkg -l grafana &>/dev/null 2>&1; then
  # Add Grafana GPG key (--batch --yes for non-interactive SSH)
  mkdir -p /etc/apt/keyrings
  curl -fsSL https://apt.grafana.com/gpg.key \
    | gpg --batch --yes --dearmor -o /etc/apt/keyrings/grafana.gpg

  # Add Grafana stable APT repository
  echo "deb [signed-by=/etc/apt/keyrings/grafana.gpg] https://apt.grafana.com stable main" \
    > /etc/apt/sources.list.d/grafana.list

  apt-get update -qq
  apt-get install -y grafana
  echo "Grafana installed."
else
  echo "Grafana already installed — skipping package install."
fi

# =============================================================================
# 3. Configure Grafana
# =============================================================================
echo ""
echo "============================================================"
echo " [3/7] Configuring Grafana"
echo "============================================================"

GRAFANA_INI="/etc/grafana/grafana.ini"

# Admin password — only set on first install (file not yet customized).
# We embed the password in grafana.ini under [security].
if grep -q '^admin_password = admin$' "${GRAFANA_INI}" 2>/dev/null || \
   grep -q '^;admin_password' "${GRAFANA_INI}" 2>/dev/null; then
  sed -i "s|^;*admin_password.*|admin_password = ${GRAFANA_ADMIN_PASS}|" "${GRAFANA_INI}"
  echo "Admin password set."
else
  # Already customized — do not overwrite. Generate a placeholder so the
  # summary at the end still shows something meaningful.
  GRAFANA_ADMIN_PASS="<already set — see ${GRAFANA_INI}>"
  echo "Admin password already customized — leaving as-is."
fi

# Bind address (0.0.0.0:3000, firewalled via ufw)
sed -i 's|^;*http_addr.*|http_addr = 0.0.0.0|' "${GRAFANA_INI}"
sed -i 's|^;*http_port.*|http_port = 3000|' "${GRAFANA_INI}"

# Allow embedding (needed for explorer iframe integration)
if grep -q '^\[security\]' "${GRAFANA_INI}"; then
  sed -i '/^\[security\]/,/^\[/{s|^;*allow_embedding.*|allow_embedding = true|}' "${GRAFANA_INI}"
else
  printf '\n[security]\nallow_embedding = true\n' >> "${GRAFANA_INI}"
fi

echo "grafana.ini updated."

# =============================================================================
# 4. Provision datasource — VictoriaMetrics as default Prometheus source
# =============================================================================
echo ""
echo "============================================================"
echo " [4/7] Provisioning Grafana datasource"
echo "============================================================"

mkdir -p /etc/grafana/provisioning/datasources

cat > /etc/grafana/provisioning/datasources/victoriametrics.yml <<'DS_EOF'
apiVersion: 1

datasources:
  - name: VictoriaMetrics
    type: prometheus
    access: proxy
    url: http://127.0.0.1:8428
    isDefault: true
    editable: false
    jsonData:
      timeInterval: "5s"
      httpMethod: POST
DS_EOF

echo "Datasource provisioning config written."

# =============================================================================
# 5. Provision dashboard directory
# =============================================================================
echo ""
echo "============================================================"
echo " [5/7] Provisioning Grafana dashboards"
echo "============================================================"

mkdir -p "${GRAFANA_DASHBOARDS_DIR}"
chown -R grafana:grafana "${GRAFANA_DASHBOARDS_DIR}"

cat > /etc/grafana/provisioning/dashboards/aztibase.yml <<DASH_EOF
apiVersion: 1

providers:
  - name: Aztibase
    orgId: 1
    folder: "Aztibase Network"
    folderUid: aztibase
    type: file
    disableDeletion: false
    updateIntervalSeconds: 30
    allowUiUpdates: true
    options:
      path: ${GRAFANA_DASHBOARDS_DIR}
DASH_EOF

echo "Dashboard provisioning config written."
echo "Place dashboard JSON files in ${GRAFANA_DASHBOARDS_DIR}/ to auto-load them."

# =============================================================================
# 6. Firewall — open Grafana port
# =============================================================================
echo ""
echo "============================================================"
echo " [6/7] Configuring firewall"
echo "============================================================"

# Ensure existing SSH rule is in place before enabling ufw
ufw allow 22/tcp   comment 'SSH'    2>/dev/null || true
ufw allow 30333/tcp comment 'Aztibase P2P' 2>/dev/null || true
ufw allow 9944/tcp  comment 'Aztibase RPC (loopback only — consider restricting)' 2>/dev/null || true
ufw allow 3000/tcp  comment 'Grafana web UI' 2>/dev/null || true

# Enable ufw non-interactively (will not drop existing SSH connections)
ufw --force enable

echo "ufw rules applied. Port 3000 open."

# =============================================================================
# 7. Start Grafana + create API service account token
# =============================================================================
echo ""
echo "============================================================"
echo " [7/7] Starting Grafana and generating API token"
echo "============================================================"

systemctl enable grafana-server
systemctl restart grafana-server

# Wait for Grafana to become ready (up to 30 seconds)
echo "Waiting for Grafana to start..."
GRAFANA_READY=0
for i in $(seq 1 30); do
  if curl -sf "http://127.0.0.1:3000/api/health" | grep -q '"database": "ok"' 2>/dev/null; then
    GRAFANA_READY=1
    break
  fi
  sleep 1
done

GRAFANA_API_TOKEN="<unavailable — Grafana did not start in time>"

if [[ ${GRAFANA_READY} -eq 1 ]]; then
  # Resolve the actual admin password (may be the literal string if already set)
  if [[ "${GRAFANA_ADMIN_PASS}" == "<already set"* ]]; then
    echo "Grafana admin password was already set — skipping automated token creation."
    echo "Create the token manually at http://${VPS_PUBLIC_IP}:3000/org/serviceaccounts"
  else
    GRAFANA_AUTH="${GRAFANA_ADMIN_PASS}"

    # Create service account
    SA_RESPONSE="$(curl -sf -X POST \
      -H "Content-Type: application/json" \
      -u "admin:${GRAFANA_AUTH}" \
      "http://127.0.0.1:3000/api/serviceaccounts" \
      -d '{"name":"aztibase-mcp","role":"Viewer","isDisabled":false}' || echo '{}')"

    SA_ID="$(echo "${SA_RESPONSE}" | jq -r '.id // empty')"

    if [[ -n "${SA_ID}" ]]; then
      TOKEN_RESPONSE="$(curl -sf -X POST \
        -H "Content-Type: application/json" \
        -u "admin:${GRAFANA_AUTH}" \
        "http://127.0.0.1:3000/api/serviceaccounts/${SA_ID}/tokens" \
        -d '{"name":"mcp-token","secondsToLive":0}' || echo '{}')"

      GRAFANA_API_TOKEN="$(echo "${TOKEN_RESPONSE}" | jq -r '.key // "<token field missing>"')"
    else
      GRAFANA_API_TOKEN="<service account creation failed — check Grafana logs>"
    fi
  fi
else
  echo "WARNING: Grafana health check timed out. Check 'systemctl status grafana-server'." >&2
fi

# =============================================================================
# Summary
# =============================================================================
echo ""
echo "============================================================"
echo " Monitoring stack installed"
echo "============================================================"
echo " VictoriaMetrics : http://127.0.0.1:8428  (internal only)"
echo " Grafana         : http://${VPS_PUBLIC_IP}:3000"
echo " Admin password  : ${GRAFANA_ADMIN_PASS}"
echo " API token (MCP) : ${GRAFANA_API_TOKEN}"
echo " Scraping        : ${AZTIBASE_METRICS_ADDR}/metrics every 5s"
echo " Retention       : 6 months"
echo " Dashboards dir  : ${GRAFANA_DASHBOARDS_DIR}/"
echo "============================================================"
echo ""
echo "Next steps:"
echo "  1. Copy dashboard JSON files to ${GRAFANA_DASHBOARDS_DIR}/"
echo "     e.g.: scp dashboards/*.json root@${VPS_PUBLIC_IP}:${GRAFANA_DASHBOARDS_DIR}/"
echo "  2. Log in at http://${VPS_PUBLIC_IP}:3000 (admin / <password above>)"
echo "  3. Store the API token in your MCP config."
echo "  4. Verify metrics: curl http://127.0.0.1:8428/metrics"
echo "  5. Verify scrape: curl 'http://127.0.0.1:8428/api/v1/query?query=up'"
echo ""
