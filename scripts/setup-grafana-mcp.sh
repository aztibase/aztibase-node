#!/usr/bin/env bash
set -euo pipefail

# Grafana MCP Server Setup for Claude Code
# Connects Claude Code to your Aztibase Grafana instance
# Run this AFTER setup-monitoring.sh has been deployed to VPS

GRAFANA_URL="${1:-http://102.209.21.247:3000}"
echo "=== Grafana MCP Setup for Claude Code ==="
echo "Grafana URL: ${GRAFANA_URL}"
echo ""

# Step 1: Check uv is installed (Python package manager)
if ! command -v uv &>/dev/null; then
    echo "Installing uv (Python package manager)..."
    if command -v pip &>/dev/null; then
        pip install uv
    elif command -v pip3 &>/dev/null; then
        pip3 install uv
    else
        echo "ERROR: pip not found. Install Python first, then run: pip install uv"
        exit 1
    fi
fi
echo "[OK] uv installed: $(uv --version 2>/dev/null || echo 'installed')"

# Step 2: Pre-fetch the MCP server package
echo "Pre-fetching mcp-grafana package..."
uvx --from mcp-grafana mcp-grafana --help >/dev/null 2>&1 || true
echo "[OK] mcp-grafana package cached"

# Step 3: Test Grafana connectivity
echo "Testing Grafana connectivity..."
HTTP_CODE=$(curl -s -o /dev/null -w "%{http_code}" "${GRAFANA_URL}/api/health" 2>/dev/null || echo "000")
if [ "$HTTP_CODE" = "200" ]; then
    echo "[OK] Grafana reachable at ${GRAFANA_URL}"
else
    echo "[WARN] Grafana not reachable (HTTP ${HTTP_CODE}). Deploy monitoring stack to VPS first."
    echo "       Run: ssh root@102.209.21.247 'bash -s' < scripts/setup-monitoring.sh"
    echo ""
    echo "Continuing with config generation anyway..."
fi

# Step 4: Prompt for API token
echo ""
echo "You need a Grafana Service Account token."
echo "The setup-monitoring.sh script prints one at the end."
echo "Or create one manually: Grafana > Administration > Service Accounts > Add token"
echo ""
read -rp "Paste your Grafana API token (or press Enter to skip): " TOKEN

if [ -z "$TOKEN" ]; then
    TOKEN="PASTE_YOUR_TOKEN_HERE"
    echo "[SKIP] No token provided. You'll need to edit the config manually."
fi

# Step 5: Write Claude Code project settings with MCP config
SETTINGS_DIR="$HOME/.claude/projects/c--Users-Lenovo-Documents-Project-Genises"
mkdir -p "$SETTINGS_DIR"

cat > "$SETTINGS_DIR/settings.json" << SETTINGS_EOF
{
  "mcpServers": {
    "grafana": {
      "command": "uvx",
      "args": ["mcp-grafana"],
      "env": {
        "GRAFANA_URL": "${GRAFANA_URL}",
        "GRAFANA_SERVICE_ACCOUNT_TOKEN": "${TOKEN}"
      }
    }
  }
}
SETTINGS_EOF

echo ""
echo "[OK] Claude Code MCP config written to: ${SETTINGS_DIR}/settings.json"
echo ""
echo "=== Setup Complete ==="
echo ""
echo "What Claude Code can now do with your Grafana:"
echo "  - Search and read dashboards"
echo "  - Query Prometheus metrics directly"
echo "  - Check alert status and rules"
echo "  - Investigate chain health from within conversations"
echo ""
echo "Test it by asking Claude: 'Query my Grafana for the current block height'"
echo ""
echo "Deployment order:"
echo "  1. SSH to VPS: ssh root@102.209.21.247"
echo "  2. Run: bash setup-monitoring.sh"
echo "  3. Note the admin password and API token"
echo "  4. Re-run this script with the token"
echo "  5. Restart Claude Code"
