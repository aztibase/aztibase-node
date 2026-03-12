registerPage('nodes', {
  init() {
    document.getElementById('page-nodes').innerHTML = `
      <div class="section-title">Network Overview</div>
      <div class="card-grid" id="nodesOverview"></div>
      <div class="section-title">All Nodes</div>
      <div class="card-grid" id="nodesGrid"></div>
    `;
  },

  async refresh() {
    const results = await Promise.allSettled(
      NODES.map(async (node) => {
        try {
          const [health, metrics, info] = await Promise.all([
            fetchHealth(node.rpc),
            fetchMetrics(node.rpc),
            rpc('aztb_nodeInfo', [], node.rpc),
          ]);
          return { ...node, health, metrics, info, online: true };
        } catch {
          return { ...node, online: false };
        }
      })
    );

    const nodes = results.map(r => r.value || r.reason);
    const onlineNodes = nodes.filter(n => n.online);

    updateConnectionStatus(onlineNodes.length > 0);
    renderOverview(nodes);
    renderNodeCards(nodes);
  },
});

function updateConnectionStatus(online) {
  const dot = document.getElementById('connStatus');
  dot.className = 'status-dot ' + (online ? 'online' : 'offline');
}

function renderOverview(nodes) {
  const online = nodes.filter(n => n.online);
  const totalPeers = online.reduce((s, n) => s + (n.metrics?.network?.peer_count || 0), 0);
  const maxHeight = Math.max(0, ...online.map(n => n.metrics?.execution?.block_height || 0));
  const totalTxs = online.reduce((s, n) => s + (n.metrics?.execution?.txs_processed || 0), 0);

  document.getElementById('nodesOverview').innerHTML = `
    <div class="card">
      <h3>Nodes Online</h3>
      <div class="value">${online.length} / ${nodes.length}</div>
    </div>
    <div class="card">
      <h3>Block Height</h3>
      <div class="value">${formatNumber(maxHeight)}</div>
    </div>
    <div class="card">
      <h3>Total Peers</h3>
      <div class="value">${totalPeers}</div>
    </div>
    <div class="card">
      <h3>Total TXs Processed</h3>
      <div class="value">${formatNumber(totalTxs)}</div>
    </div>
  `;
}

function renderNodeCards(nodes) {
  const grid = document.getElementById('nodesGrid');
  grid.innerHTML = nodes.map(n => {
    if (!n.online) {
      return `
        <div class="node-card down">
          <div class="node-header">
            <span class="node-name">${n.name}</span>
            <span class="badge badge-red">OFFLINE</span>
          </div>
          <div class="sub">${n.rpc}</div>
        </div>
      `;
    }
    const m = n.metrics || {};
    const c = m.consensus || {};
    const e = m.execution || {};
    const net = m.network || {};
    const latencyMs = c.last_commit_latency_us ? (c.last_commit_latency_us / 1000).toFixed(1) : '—';

    return `
      <div class="node-card healthy">
        <div class="node-header">
          <span class="node-name">${n.name}</span>
          <span class="badge badge-green">ONLINE</span>
        </div>
        <div class="sub">${n.rpc}</div>
        <div class="node-stats">
          <span class="node-stat-label">Height</span>
          <span class="node-stat-value">${formatNumber(e.block_height)}</span>
          <span class="node-stat-label">Peers</span>
          <span class="node-stat-value">${net.peer_count || 0}</span>
          <span class="node-stat-label">Mempool</span>
          <span class="node-stat-value">${net.mempool_size || 0}</span>
          <span class="node-stat-label">Commits</span>
          <span class="node-stat-value">${formatNumber(c.commits)}</span>
          <span class="node-stat-label">Rounds</span>
          <span class="node-stat-value">${formatNumber(c.rounds_advanced)}</span>
          <span class="node-stat-label">Latency</span>
          <span class="node-stat-value">${latencyMs} ms</span>
          <span class="node-stat-label">Base Fee</span>
          <span class="node-stat-value">${e.base_fee || 1}</span>
          <span class="node-stat-label">Equivocations</span>
          <span class="node-stat-value">${c.equivocations || 0}</span>
        </div>
      </div>
    `;
  }).join('');
}
