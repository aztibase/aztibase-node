#!/usr/bin/env bash
set -euo pipefail

# ─────────────────────────────────────────────────────────────────────
# Aztibase Validator — Business-in-a-Box Setup
# One-click validator node for non-technical operators.
# Supports: Ubuntu 22.04+, Debian 12+
# Usage:
#   Interactive:  bash setup-validator.sh
#   Non-interactive:  bash setup-validator.sh --yes --network testnet
#   Dry run:      bash setup-validator.sh --dry-run
#   Uninstall:    bash setup-validator.sh --uninstall
# ─────────────────────────────────────────────────────────────────────

VERSION="1.0.0"
AZTIBASE_USER="aztibase"
AZTIBASE_HOME="/var/lib/aztibase"
AZTIBASE_CONFIG="/etc/aztibase"
AZTIBASE_BIN="/usr/local/bin/aztibase"
ALLOY_BIN="/usr/local/bin/alloy"
ALLOY_CONFIG="/etc/alloy"
ALLOY_SERVICE="alloy"
SERVICE_NAME="aztibase-validator"
REPO_URL="https://github.com/user/aztibase.git"
BRANCH="dev"
LOG_FILE="/tmp/aztibase-setup-$(date +%Y%m%d-%H%M%S).log"

# Defaults (overridden by prompts or flags)
NODE_NAME=""
NETWORK="testnet"
ENABLE_MONITORING="no"
GRAFANA_CLOUD_URL=""
GRAFANA_CLOUD_USER=""
GRAFANA_CLOUD_TOKEN=""
AUTO_YES="no"
DRY_RUN="no"
UNINSTALL="no"

# ── Colors ────────────────────────────────────────────────────────────
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
CYAN='\033[0;36m'
BOLD='\033[1m'
NC='\033[0m'

banner() {
    echo ""
    echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${CYAN}║${NC}  ${BOLD}Aztibase Network — Validator Setup${NC}  v${VERSION}             ${CYAN}║${NC}"
    echo -e "${CYAN}║${NC}  Business-in-a-Box: one command, working validator.     ${CYAN}║${NC}"
    echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}"
    echo ""
}

log() { echo -e "${GREEN}[✓]${NC} $1"; }
warn() { echo -e "${YELLOW}[!]${NC} $1"; }
err() { echo -e "${RED}[✗]${NC} $1" >&2; }
step() { echo -e "\n${BLUE}──── $1 ────${NC}"; }
ask() {
    local prompt="$1" default="$2" var="$3"
    if [ "$AUTO_YES" = "yes" ]; then
        eval "$var='$default'"
        return
    fi
    echo -en "${BOLD}$prompt${NC} [${default}]: "
    read -r input
    eval "$var='${input:-$default}'"
}

ask_yn() {
    local prompt="$1" default="$2"
    if [ "$AUTO_YES" = "yes" ]; then
        [ "$default" = "y" ] && return 0 || return 1
    fi
    echo -en "${BOLD}$prompt${NC} [${default}]: "
    read -r input
    input="${input:-$default}"
    [[ "$input" =~ ^[Yy] ]]
}

run_cmd() {
    if [ "$DRY_RUN" = "yes" ]; then
        echo -e "  ${YELLOW}[dry-run]${NC} $*"
        return 0
    fi
    "$@" >> "$LOG_FILE" 2>&1
}

require_root() {
    if [ "$DRY_RUN" = "yes" ]; then return 0; fi
    if [ "$(id -u)" -ne 0 ]; then
        err "This script must be run as root (use: sudo bash setup-validator.sh)"
        exit 1
    fi
}

# ── Parse CLI flags ──────────────────────────────────────────────────
while [ $# -gt 0 ]; do
    case "$1" in
        --yes|-y) AUTO_YES="yes" ;;
        --dry-run) DRY_RUN="yes" ;;
        --uninstall) UNINSTALL="yes" ;;
        --network) NETWORK="$2"; shift ;;
        --network=*) NETWORK="${1#*=}" ;;
        --name) NODE_NAME="$2"; shift ;;
        --name=*) NODE_NAME="${1#*=}" ;;
        --grafana-url) GRAFANA_CLOUD_URL="$2"; shift ;;
        --grafana-url=*) GRAFANA_CLOUD_URL="${1#*=}" ;;
        --grafana-user) GRAFANA_CLOUD_USER="$2"; shift ;;
        --grafana-user=*) GRAFANA_CLOUD_USER="${1#*=}" ;;
        --grafana-token) GRAFANA_CLOUD_TOKEN="$2"; shift ;;
        --grafana-token=*) GRAFANA_CLOUD_TOKEN="${1#*=}" ;;
        --help|-h)
            banner
            echo "Usage: bash setup-validator.sh [OPTIONS]"
            echo ""
            echo "Options:"
            echo "  --yes, -y             Skip all prompts (use defaults)"
            echo "  --dry-run             Show what would be done without making changes"
            echo "  --uninstall           Remove Aztibase validator and all data"
            echo "  --network <name>      Network: testnet or mainnet (default: testnet)"
            echo "  --name <name>         Node name (default: auto-generated)"
            echo "  --grafana-url <url>   Grafana Cloud Prometheus push URL"
            echo "  --grafana-user <id>   Grafana Cloud instance ID"
            echo "  --grafana-token <tok> Grafana Cloud API token"
            echo "  --help, -h            Show this help"
            exit 0
            ;;
        *) err "Unknown option: $1"; exit 1 ;;
    esac
    shift
done

# ── Uninstall ────────────────────────────────────────────────────────
if [ "$UNINSTALL" = "yes" ]; then
    banner
    require_root
    step "Uninstalling Aztibase Validator"

    warn "This will stop the validator and remove ALL data."
    if ! ask_yn "Are you sure? (y/n)" "n"; then
        log "Uninstall cancelled."
        exit 0
    fi

    if systemctl is-active "$SERVICE_NAME" &>/dev/null; then
        log "Stopping validator service..."
        run_cmd systemctl stop "$SERVICE_NAME"
    fi
    if systemctl is-enabled "$SERVICE_NAME" &>/dev/null; then
        run_cmd systemctl disable "$SERVICE_NAME"
    fi
    [ -f "/etc/systemd/system/${SERVICE_NAME}.service" ] && run_cmd rm -f "/etc/systemd/system/${SERVICE_NAME}.service"

    if systemctl is-active "$ALLOY_SERVICE" &>/dev/null; then
        run_cmd systemctl stop "$ALLOY_SERVICE"
        run_cmd systemctl disable "$ALLOY_SERVICE"
    fi
    [ -f "/etc/systemd/system/${ALLOY_SERVICE}.service" ] && run_cmd rm -f "/etc/systemd/system/${ALLOY_SERVICE}.service"

    run_cmd systemctl daemon-reload

    if ask_yn "Delete all node data ($AZTIBASE_HOME)? (y/n)" "n"; then
        run_cmd rm -rf "$AZTIBASE_HOME"
        log "Node data deleted."
    else
        warn "Node data preserved at $AZTIBASE_HOME"
    fi

    [ -d "$AZTIBASE_CONFIG" ] && run_cmd rm -rf "$AZTIBASE_CONFIG"
    [ -d "$ALLOY_CONFIG" ] && run_cmd rm -rf "$ALLOY_CONFIG"
    [ -f "$AZTIBASE_BIN" ] && run_cmd rm -f "$AZTIBASE_BIN"
    [ -f "$ALLOY_BIN" ] && run_cmd rm -f "$ALLOY_BIN"

    if id "$AZTIBASE_USER" &>/dev/null; then
        run_cmd userdel "$AZTIBASE_USER"
        log "System user '$AZTIBASE_USER' removed."
    fi

    log "Uninstall complete."
    exit 0
fi

# ── Main setup ───────────────────────────────────────────────────────
banner

if [ "$DRY_RUN" = "yes" ]; then
    warn "DRY RUN mode — no changes will be made."
    echo ""
else
    require_root
fi

echo "Full log: $LOG_FILE"
echo ""

# ── Step 1: System check ─────────────────────────────────────────────
step "Step 1/7 — Checking your system"

OS_ID=$(. /etc/os-release 2>/dev/null && echo "$ID" || echo "unknown")
OS_VERSION=$(. /etc/os-release 2>/dev/null && echo "$VERSION_ID" || echo "0")
ARCH=$(uname -m)

if [ "$DRY_RUN" = "no" ]; then
    case "$OS_ID" in
        ubuntu)
            if [ "${OS_VERSION%%.*}" -lt 22 ]; then
                err "Ubuntu 22.04 or later required (found $OS_VERSION)"
                exit 1
            fi
            ;;
        debian)
            if [ "${OS_VERSION%%.*}" -lt 12 ]; then
                err "Debian 12 or later required (found $OS_VERSION)"
                exit 1
            fi
            ;;
        *)
            warn "Untested OS: $OS_ID $OS_VERSION. Proceeding anyway."
            ;;
    esac

    if [ "$ARCH" != "x86_64" ] && [ "$ARCH" != "aarch64" ]; then
        err "Unsupported architecture: $ARCH (need x86_64 or aarch64)"
        exit 1
    fi
fi

log "OS: $OS_ID $OS_VERSION ($ARCH)"

TOTAL_MEM_MB=$(awk '/MemTotal/ {printf "%d", $2/1024}' /proc/meminfo 2>/dev/null || echo "0")
TOTAL_DISK_GB=$(df -BG / 2>/dev/null | awk 'NR==2 {gsub("G",""); print $4}' || echo "0")
CPU_CORES=$(nproc 2>/dev/null || echo "0")

log "Hardware: ${CPU_CORES} CPU cores, ${TOTAL_MEM_MB} MB RAM, ${TOTAL_DISK_GB} GB free disk"

if [ "$TOTAL_MEM_MB" -lt 3500 ] && [ "$DRY_RUN" = "no" ]; then
    warn "Less than 4 GB RAM detected. Validator may struggle under load."
    if ! ask_yn "Continue anyway? (y/n)" "y"; then
        exit 1
    fi
fi

if [ "$TOTAL_DISK_GB" -lt 30 ] && [ "$DRY_RUN" = "no" ]; then
    warn "Less than 30 GB free disk. Aztibase needs ~50 GB for long-running nodes."
    if ! ask_yn "Continue anyway? (y/n)" "y"; then
        exit 1
    fi
fi

# ── Step 2: Configuration ────────────────────────────────────────────
step "Step 2/7 — Configuration"

if [ -z "$NODE_NAME" ]; then
    DEFAULT_NAME="aztibase-$(hostname -s 2>/dev/null || echo 'node')"
    ask "Choose a name for your validator" "$DEFAULT_NAME" NODE_NAME
fi
log "Node name: $NODE_NAME"

if [ "$AUTO_YES" = "no" ]; then
    echo ""
    echo "  Which network do you want to join?"
    echo "    1) testnet  — test tokens, safe to experiment"
    echo "    2) mainnet  — real AZTB tokens, production network"
    echo ""
    ask "Enter 1 or 2" "1" NETWORK_CHOICE
    case "$NETWORK_CHOICE" in
        2|mainnet) NETWORK="mainnet" ;;
        *) NETWORK="testnet" ;;
    esac
fi
log "Network: $NETWORK"

if [ "$AUTO_YES" = "no" ] && [ -z "$GRAFANA_CLOUD_URL" ]; then
    echo ""
    echo "  Free monitoring lets you check your node from any browser."
    echo "  Grafana Cloud free tier: 10,000 metrics, no credit card needed."
    echo "  You can set this up later if you prefer."
    echo ""
    if ask_yn "Enable Grafana Cloud monitoring? (y/n)" "y"; then
        ENABLE_MONITORING="yes"
        echo ""
        echo "  To get your Grafana Cloud credentials:"
        echo "    1. Go to https://grafana.com/auth/sign-up/create-user (free)"
        echo "    2. Create a stack (any region)"
        echo "    3. Go to Connections → Add new connection → Hosted Prometheus"
        echo "    4. Copy the Remote Write Endpoint URL"
        echo "    5. Copy the Username (numeric ID)"
        echo "    6. Generate an API token (Grafana Cloud Access Policy → create token)"
        echo ""
        ask "Prometheus Remote Write URL" "" GRAFANA_CLOUD_URL
        ask "Username (numeric instance ID)" "" GRAFANA_CLOUD_USER
        ask "API token" "" GRAFANA_CLOUD_TOKEN

        if [ -z "$GRAFANA_CLOUD_URL" ] || [ -z "$GRAFANA_CLOUD_USER" ] || [ -z "$GRAFANA_CLOUD_TOKEN" ]; then
            warn "Missing Grafana Cloud credentials. Skipping monitoring setup."
            warn "You can run this script again later to add monitoring."
            ENABLE_MONITORING="no"
        fi
    fi
elif [ -n "$GRAFANA_CLOUD_URL" ]; then
    ENABLE_MONITORING="yes"
fi

# ── Step 3: Install dependencies ─────────────────────────────────────
step "Step 3/7 — Installing dependencies"

log "Updating package lists..."
run_cmd apt-get update -qq

log "Installing build tools, SSL, git, curl..."
run_cmd apt-get install -y -qq build-essential pkg-config libssl-dev git curl jq

if ! command -v cargo &>/dev/null; then
    log "Installing Rust toolchain (this takes ~1 minute)..."
    if [ "$DRY_RUN" = "no" ]; then
        curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable >> "$LOG_FILE" 2>&1
        export PATH="$HOME/.cargo/bin:$PATH"
    else
        echo -e "  ${YELLOW}[dry-run]${NC} curl ... | sh (install Rust)"
    fi
else
    log "Rust already installed: $(rustc --version 2>/dev/null || echo 'unknown')"
fi

# ── Step 4: Build Aztibase ───────────────────────────────────────────
step "Step 4/7 — Building Aztibase (5-15 minutes on first run)"

if [ -f "$AZTIBASE_BIN" ]; then
    EXISTING_VER=$("$AZTIBASE_BIN" --version 2>/dev/null || echo "unknown")
    log "Existing binary found: $EXISTING_VER"
    if ! ask_yn "Rebuild from source? (y/n)" "n"; then
        log "Keeping existing binary."
    else
        REBUILD="yes"
    fi
else
    REBUILD="yes"
fi

if [ "${REBUILD:-yes}" = "yes" ]; then
    WORK_DIR=$(mktemp -d)
    log "Cloning repository..."
    run_cmd git clone --depth 1 --branch "$BRANCH" "$REPO_URL" "$WORK_DIR/aztibase"

    log "Compiling release binary (grab a coffee)..."
    if [ "$DRY_RUN" = "no" ]; then
        cd "$WORK_DIR/aztibase"
        CARGO_INCREMENTAL=0 cargo build --release -p aztibase-node >> "$LOG_FILE" 2>&1
        cp target/release/aztibase "$AZTIBASE_BIN"
        chmod +x "$AZTIBASE_BIN"
        cd /
    else
        echo -e "  ${YELLOW}[dry-run]${NC} cargo build --release -p aztibase-node"
    fi

    log "Binary installed: $AZTIBASE_BIN"
    run_cmd rm -rf "$WORK_DIR"
fi

# ── Step 5: Configure node ──────────────────────────────────────────
step "Step 5/7 — Configuring validator node"

if ! id "$AZTIBASE_USER" &>/dev/null && [ "$DRY_RUN" = "no" ]; then
    useradd --system --home-dir "$AZTIBASE_HOME" --shell /usr/sbin/nologin "$AZTIBASE_USER"
    log "Created system user: $AZTIBASE_USER"
fi

run_cmd mkdir -p "$AZTIBASE_HOME" "$AZTIBASE_CONFIG/keys"

NETWORK_FLAG="--testnet"
[ "$NETWORK" = "mainnet" ] && NETWORK_FLAG="--mainnet"

if [ ! -f "$AZTIBASE_CONFIG/keys/validator.json" ] || [ "$DRY_RUN" = "yes" ]; then
    log "Generating validator keypair..."
    if [ "$DRY_RUN" = "no" ]; then
        "$AZTIBASE_BIN" wallet generate --output "$AZTIBASE_CONFIG/keys/validator.json"
        chown "$AZTIBASE_USER:$AZTIBASE_USER" "$AZTIBASE_CONFIG/keys/validator.json"
        chmod 600 "$AZTIBASE_CONFIG/keys/validator.json"
    else
        echo -e "  ${YELLOW}[dry-run]${NC} aztibase wallet generate --output $AZTIBASE_CONFIG/keys/validator.json"
    fi

    echo ""
    echo -e "  ${RED}╔══════════════════════════════════════════════════════╗${NC}"
    echo -e "  ${RED}║  IMPORTANT: Back up your validator key!              ║${NC}"
    echo -e "  ${RED}║  Location: $AZTIBASE_CONFIG/keys/validator.json      ║${NC}"
    echo -e "  ${RED}║  If you lose this file, you lose your validator.     ║${NC}"
    echo -e "  ${RED}╚══════════════════════════════════════════════════════╝${NC}"
    echo ""
else
    log "Validator key already exists. Keeping it."
fi

if [ "$DRY_RUN" = "no" ]; then
    chown -R "$AZTIBASE_USER:$AZTIBASE_USER" "$AZTIBASE_HOME"
fi

BOOT_NODES=""
if [ "$NETWORK" = "testnet" ]; then
    BOOT_NODES="--boot-node /dns4/testnet1.aztibase.com/tcp/30333 --boot-node /dns4/testnet2.aztibase.com/tcp/30333 --boot-node /dns4/testnet3.aztibase.com/tcp/30333"
fi

cat > "/etc/systemd/system/${SERVICE_NAME}.service" << UNIT
[Unit]
Description=Aztibase Validator ($NODE_NAME)
Documentation=https://docs.aztibase.com
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=$AZTIBASE_USER
ExecStart=$AZTIBASE_BIN \\
    --data-dir $AZTIBASE_HOME \\
    --validator-key $AZTIBASE_CONFIG/keys/validator.json \\
    --rpc-addr 0.0.0.0:9944 \\
    --listen /ip4/0.0.0.0/tcp/30333 \\
    --listen /ip4/0.0.0.0/udp/30333/quic-v1 \\
    --metrics \\
    $NETWORK_FLAG $BOOT_NODES
Restart=on-failure
RestartSec=10
LimitNOFILE=65535

Environment=RUST_LOG=info

WorkingDirectory=$AZTIBASE_HOME
StateDirectory=aztibase

[Install]
WantedBy=multi-user.target
UNIT

log "Systemd service created: ${SERVICE_NAME}.service"

if [ "$DRY_RUN" = "no" ]; then
    systemctl daemon-reload
    systemctl enable "$SERVICE_NAME" >> "$LOG_FILE" 2>&1
fi

# ── Step 6: Firewall ────────────────────────────────────────────────
step "Step 6/7 — Firewall & networking"

if command -v ufw &>/dev/null; then
    log "Configuring UFW firewall rules..."
    run_cmd ufw allow 30333/tcp comment "Aztibase P2P TCP"
    run_cmd ufw allow 30333/udp comment "Aztibase P2P QUIC"
    run_cmd ufw allow 9944/tcp comment "Aztibase RPC"
    log "Ports opened: 30333 (P2P), 9944 (RPC)"
else
    warn "UFW not found. Make sure ports 30333 and 9944 are open in your firewall."
fi

# ── Step 7: Grafana Cloud monitoring ────────────────────────────────
step "Step 7/7 — Monitoring setup"

if [ "$ENABLE_MONITORING" = "yes" ]; then
    log "Setting up Grafana Cloud monitoring..."

    if [ ! -f "$ALLOY_BIN" ] || [ "$DRY_RUN" = "yes" ]; then
        log "Installing Grafana Alloy (metrics agent)..."
        if [ "$DRY_RUN" = "no" ]; then
            ALLOY_VERSION="1.5.1"
            ALLOY_ARCH="amd64"
            [ "$ARCH" = "aarch64" ] && ALLOY_ARCH="arm64"
            ALLOY_URL="https://github.com/grafana/alloy/releases/download/v${ALLOY_VERSION}/alloy-linux-${ALLOY_ARCH}.zip"
            ALLOY_TMP=$(mktemp -d)
            curl -fsSL "$ALLOY_URL" -o "$ALLOY_TMP/alloy.zip"
            if command -v unzip &>/dev/null; then
                unzip -q "$ALLOY_TMP/alloy.zip" -d "$ALLOY_TMP"
            else
                apt-get install -y -qq unzip >> "$LOG_FILE" 2>&1
                unzip -q "$ALLOY_TMP/alloy.zip" -d "$ALLOY_TMP"
            fi
            cp "$ALLOY_TMP/alloy-linux-${ALLOY_ARCH}" "$ALLOY_BIN"
            chmod +x "$ALLOY_BIN"
            rm -rf "$ALLOY_TMP"
        else
            echo -e "  ${YELLOW}[dry-run]${NC} Download + install Grafana Alloy"
        fi
    fi

    mkdir -p "$ALLOY_CONFIG"

    cat > "$ALLOY_CONFIG/config.alloy" << ALLOY_CFG
// Aztibase Validator — Grafana Cloud metrics pipeline
// Scrapes local /metrics endpoint and pushes to Grafana Cloud

prometheus.scrape "aztibase" {
    targets = [{
        __address__ = "127.0.0.1:9944",
        instance    = "$NODE_NAME",
        network     = "$NETWORK",
    }]
    metrics_path  = "/metrics"
    scrape_interval = "15s"
    forward_to    = [prometheus.remote_write.grafana_cloud.receiver]
}

prometheus.remote_write "grafana_cloud" {
    endpoint {
        url = "$GRAFANA_CLOUD_URL"
        basic_auth {
            username = "$GRAFANA_CLOUD_USER"
            password = "$GRAFANA_CLOUD_TOKEN"
        }
    }
}
ALLOY_CFG

    log "Alloy config written: $ALLOY_CONFIG/config.alloy"

    cat > "/etc/systemd/system/${ALLOY_SERVICE}.service" << ALLOY_UNIT
[Unit]
Description=Grafana Alloy (Aztibase metrics)
After=network-online.target ${SERVICE_NAME}.service
Wants=network-online.target

[Service]
Type=simple
ExecStart=$ALLOY_BIN run $ALLOY_CONFIG/config.alloy
Restart=on-failure
RestartSec=10
User=root

[Install]
WantedBy=multi-user.target
ALLOY_UNIT

    if [ "$DRY_RUN" = "no" ]; then
        systemctl daemon-reload
        systemctl enable "$ALLOY_SERVICE" >> "$LOG_FILE" 2>&1
    fi

    log "Grafana Alloy service created and enabled."
    echo ""
    echo -e "  ${CYAN}To import dashboards in Grafana Cloud:${NC}"
    echo "    1. Log in to https://grafana.com → your stack → Grafana"
    echo "    2. Go to Dashboards → Import"
    echo "    3. Upload the JSON files from the Aztibase repo:"
    echo "       - monitoring/grafana/dashboards/node-health.json"
    echo "       - monitoring/grafana/dashboards/consensus.json"
    echo "    4. Select your Hosted Prometheus datasource"
    echo ""
else
    log "Monitoring skipped. Run this script again to add it later."
fi

# ── Done ─────────────────────────────────────────────────────────────
echo ""
echo -e "${GREEN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${GREEN}║          Aztibase Validator Setup Complete!             ║${NC}"
echo -e "${GREEN}╚══════════════════════════════════════════════════════════╝${NC}"
echo ""
echo -e "  ${BOLD}Node name:${NC}      $NODE_NAME"
echo -e "  ${BOLD}Network:${NC}        $NETWORK"
echo -e "  ${BOLD}Data:${NC}           $AZTIBASE_HOME"
echo -e "  ${BOLD}Validator key:${NC}  $AZTIBASE_CONFIG/keys/validator.json"
echo -e "  ${BOLD}RPC endpoint:${NC}   http://localhost:9944"
echo -e "  ${BOLD}Metrics:${NC}        http://localhost:9944/metrics"
if [ "$ENABLE_MONITORING" = "yes" ]; then
echo -e "  ${BOLD}Monitoring:${NC}     Grafana Cloud (Alloy agent running)"
fi
echo ""
echo -e "  ${BOLD}Quick commands:${NC}"
echo "    sudo systemctl start $SERVICE_NAME      # Start your validator"
echo "    sudo systemctl status $SERVICE_NAME     # Check status"
echo "    sudo journalctl -u $SERVICE_NAME -f     # Watch live logs"
echo "    curl -s http://localhost:9944/health     # Health check"
echo ""

if [ "$DRY_RUN" = "no" ]; then
    log "Starting validator..."
    systemctl start "$SERVICE_NAME"

    if [ "$ENABLE_MONITORING" = "yes" ]; then
        systemctl start "$ALLOY_SERVICE"
    fi

    sleep 3

    if systemctl is-active "$SERVICE_NAME" &>/dev/null; then
        log "Validator is running!"

        HEALTH=$(curl -sf http://localhost:9944/health 2>/dev/null || echo "")
        if [ -n "$HEALTH" ]; then
            log "Health check passed: $HEALTH"
        else
            warn "Health endpoint not responding yet. Give it a few seconds."
        fi
    else
        err "Validator failed to start. Check logs:"
        echo "    sudo journalctl -u $SERVICE_NAME --no-pager -n 20"
    fi

    if [ -f "$AZTIBASE_CONFIG/keys/validator.json" ]; then
        ADDR=$(python3 -c "import json; print(json.load(open('$AZTIBASE_CONFIG/keys/validator.json'))['address'])" 2>/dev/null || echo "(see key file)")
        echo ""
        echo -e "  ${BOLD}Your validator address:${NC} $ADDR"
        if [ "$NETWORK" = "testnet" ]; then
            echo -e "  ${BOLD}Get testnet tokens:${NC}     https://faucet.aztibase.com"
            echo -e "  ${BOLD}Stake to validate:${NC}      Send a Stake transaction for ≥10,000 AZTB"
        fi
    fi
fi

echo ""
echo "  Full setup log: $LOG_FILE"
echo ""
