# Grafana Cloud Monitoring — Free Tier Setup

Monitor your Aztibase validator from any browser, no self-hosted infrastructure needed.

**Cost:** Free (10,000 metric series, 50 GB logs, 50 GB traces — fits 3+ validators easily).

---

## Step 1: Create a Grafana Cloud Account

1. Go to [https://grafana.com/auth/sign-up/create-user](https://grafana.com/auth/sign-up/create-user)
2. Sign up with email or GitHub (no credit card required)
3. Create a **stack** — choose any name (e.g., `aztibase-validators`) and region

Your stack URL will look like: `https://yourstackname.grafana.net`

## Step 2: Get Your Prometheus Credentials

1. In the Grafana Cloud portal, click **Connections** → **Add new connection**
2. Search for **Hosted Prometheus** (or **Hosted Metrics**)
3. Note down three values:
   - **Remote Write Endpoint**: looks like `https://prometheus-prod-XX-prod-XX.grafana.net/api/prom/push`
   - **Username**: a numeric ID (e.g., `1234567`)
   - **Generate API Token**: click to create one, name it `aztibase-validator`, copy the token

> Save these three values — you'll need them for the setup script.

## Step 3: Run the Validator Setup with Monitoring

If you haven't set up your validator yet:

```bash
sudo bash setup-validator.sh
```

When prompted "Enable Grafana Cloud monitoring?", answer **y** and paste your credentials.

If your validator is already running, re-run the script — it will detect the existing installation and only add monitoring:

```bash
sudo bash setup-validator.sh
```

### Manual Setup (if you prefer)

Install Grafana Alloy:

```bash
ALLOY_VERSION="1.5.1"
curl -fsSL "https://github.com/grafana/alloy/releases/download/v${ALLOY_VERSION}/alloy-linux-amd64.zip" -o /tmp/alloy.zip
unzip /tmp/alloy.zip -d /tmp
sudo cp /tmp/alloy-linux-amd64 /usr/local/bin/alloy
sudo chmod +x /usr/local/bin/alloy
```

Create config at `/etc/alloy/config.alloy`:

```hcl
prometheus.scrape "aztibase" {
    targets = [{
        __address__ = "127.0.0.1:9944",
        instance    = "my-validator",
        network     = "testnet",
    }]
    metrics_path    = "/metrics"
    scrape_interval = "15s"
    forward_to      = [prometheus.remote_write.grafana_cloud.receiver]
}

prometheus.remote_write "grafana_cloud" {
    endpoint {
        url = "YOUR_REMOTE_WRITE_URL"
        basic_auth {
            username = "YOUR_INSTANCE_ID"
            password = "YOUR_API_TOKEN"
        }
    }
}
```

Replace the three `YOUR_*` placeholders with your Grafana Cloud credentials.

Create a systemd service:

```bash
sudo tee /etc/systemd/system/alloy.service << 'EOF'
[Unit]
Description=Grafana Alloy (Aztibase metrics)
After=network-online.target aztibase-validator.service

[Service]
Type=simple
ExecStart=/usr/local/bin/alloy run /etc/alloy/config.alloy
Restart=on-failure
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF

sudo systemctl daemon-reload
sudo systemctl enable --now alloy
```

## Step 4: Import Dashboards

1. Log in to your Grafana Cloud stack (e.g., `https://yourstackname.grafana.net`)
2. Go to **Dashboards** → **New** → **Import**
3. Upload these JSON files from the Aztibase repository:
   - `monitoring/grafana/dashboards/node-health.json` — block height, TPS, peer count, mempool, base fee
   - `monitoring/grafana/dashboards/consensus.json` — rounds, commits, latency, equivocations
4. When prompted for a datasource, select **grafanacloud-prom** (your hosted Prometheus)
5. Click **Import**

Both dashboards will appear under the "Aztibase Network" folder.

## Step 5: Verify Data Flow

After the Alloy agent has been running for ~30 seconds:

1. Go to **Explore** in Grafana Cloud
2. Select your hosted Prometheus datasource
3. Query: `aztibase_execution_block_height`
4. You should see your validator's block height updating

If no data appears:
- Check Alloy is running: `sudo systemctl status alloy`
- Check Alloy logs: `sudo journalctl -u alloy -f`
- Verify your validator exposes metrics: `curl http://localhost:9944/metrics`

## Free Tier Limits

| Resource | Free Allowance | Aztibase Usage (3 nodes) |
|----------|---------------|--------------------------|
| Metric series | 10,000 | ~50-100 per node (~300 total) |
| Metrics retention | 13 months | More than enough |
| Log volume | 50 GB/month | Not used (optional) |
| Trace volume | 50 GB/month | Not used |

You won't come close to the free tier limits with 3 validators.

## Alerting (Optional)

Set up free alerts for node health:

1. Go to **Alerting** → **Alert rules** → **New alert rule**
2. Useful alerts:
   - **Node down**: `up{job="aztibase"} == 0` for 2 minutes
   - **No new blocks**: `rate(aztibase_execution_block_height[5m]) == 0` for 5 minutes
   - **Peer loss**: `aztibase_network_peer_count < 1` for 3 minutes
   - **Equivocation**: `aztibase_consensus_equivocations > 0`
3. Set notification channel to email (configured in your Grafana Cloud profile)
