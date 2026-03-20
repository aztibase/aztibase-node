#!/usr/bin/env bash
set -euo pipefail

echo "=== Aztibase VPS Setup ==="

# 1. Create 4GB swap (2GB RAM isn't enough for Rust compilation)
echo "[1/7] Creating swap..."
if [ ! -f /swapfile ]; then
    fallocate -l 4G /swapfile
    chmod 600 /swapfile
    mkswap /swapfile
    swapon /swapfile
    echo '/swapfile none swap sw 0 0' >> /etc/fstab
    echo "Swap created (4GB)"
else
    echo "Swap already exists"
fi

# 2. Install build dependencies
echo "[2/7] Installing dependencies..."
apt-get update -qq
apt-get install -y -qq build-essential pkg-config libssl-dev git curl ufw > /dev/null 2>&1

# 3. Install Rust
echo "[3/7] Installing Rust..."
if ! command -v cargo &> /dev/null; then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain stable
    source "$HOME/.cargo/env"
else
    source "$HOME/.cargo/env"
    echo "Rust already installed: $(rustc --version)"
fi

# 4. Clone and build
echo "[4/7] Cloning and building (this takes ~15-20 minutes)..."
cd /opt
if [ ! -d aztibase ]; then
    git clone https://github.com/Gerrit740/Project-Genesis.git aztibase
else
    cd aztibase && git pull && cd /opt
fi
cd aztibase
cargo build --release -p aztibase-node 2>&1 | tail -5
cp target/release/aztibase /usr/local/bin/aztibase
chmod +x /usr/local/bin/aztibase
echo "Binary installed: $(aztibase --version 2>/dev/null || echo 'ok')"

# 5. Create node config
echo "[5/7] Configuring node..."
mkdir -p /var/lib/aztibase/models
mkdir -p /etc/aztibase

cat > /etc/aztibase/node.toml << 'TOML'
data_dir = "/var/lib/aztibase"
validator_key = "/var/lib/aztibase/keys/validator.json"

[network]
listen_addresses = [
    "/ip4/0.0.0.0/tcp/30333",
    "/ip4/0.0.0.0/udp/30333/quic-v1",
]
boot_nodes = []
idle_timeout_secs = 60
stun_servers = [
    "stun:stun.l.google.com:19302",
    "stun:stun1.l.google.com:19302",
]

[rpc]
listen_addr = "0.0.0.0:9944"
enabled = true

[log]
level = "info"

[ai]
enabled = false
models = []

[metrics]
enabled = true
TOML

# 6. Create systemd service
echo "[6/7] Creating systemd service..."
cat > /etc/systemd/system/aztibase.service << 'SERVICE'
[Unit]
Description=Aztibase Validator Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/aztibase --config /etc/aztibase/node.toml --testnet
Restart=on-failure
RestartSec=5
LimitNOFILE=65535

Environment=RUST_LOG=info

[Install]
WantedBy=multi-user.target
SERVICE

systemctl daemon-reload
systemctl enable aztibase

# 7. Open firewall ports
echo "[7/7] Configuring firewall..."
ufw allow 22/tcp    > /dev/null
ufw allow 30333/tcp > /dev/null
ufw allow 30333/udp > /dev/null
ufw allow 9944/tcp  > /dev/null
ufw --force enable  > /dev/null

echo ""
echo "=== Setup Complete ==="
echo "  Binary:  /usr/local/bin/aztibase"
echo "  Config:  /etc/aztibase/node.toml"
echo "  Data:    /var/lib/aztibase/"
echo "  Service: systemctl start aztibase"
echo ""
echo "Next steps:"
echo "  1. Generate keys:  aztibase wallet generate --validator --output /var/lib/aztibase/keys/validator.json"
echo "  2. Start node:     systemctl start aztibase"
echo "  3. Check status:   systemctl status aztibase"
echo "  4. View logs:      journalctl -u aztibase -f"
echo ""
echo "  Public IP: $(curl -s ifconfig.me)"
echo "  P2P:       /ip4/$(curl -s ifconfig.me)/tcp/30333"
echo "  RPC:       http://$(curl -s ifconfig.me):9944"
