registerPage('accounts', {
  init() {
    document.getElementById('page-accounts').innerHTML = `
      <div class="section-title">Account Lookup</div>
      <div class="form-row" style="max-width:600px">
        <div class="form-group">
          <label class="form-label">Address or TX Hash</label>
          <input class="form-input" id="accountQuery" placeholder="64-char hex">
        </div>
        <button class="btn btn-primary" onclick="queryAccount()">Lookup</button>
      </div>
      <div id="accountResult" style="margin-top:16px"></div>

      <div class="section-title" style="margin-top:32px">Batch Explorer</div>
      <div class="form-row" style="max-width:400px">
        <div class="form-group">
          <label class="form-label">Batch Number</label>
          <input class="form-input" id="batchQuery" type="number" min="0" placeholder="e.g. 100">
        </div>
        <button class="btn btn-primary" onclick="queryBatch()">Load</button>
      </div>
      <div id="batchResult" style="margin-top:16px"></div>
    `;
  },
});

async function queryAccount() {
  const q = document.getElementById('accountQuery').value.trim().replace(/^0x/, '');
  const el = document.getElementById('accountResult');

  if (!/^[0-9a-fA-F]{64}$/.test(q)) {
    el.innerHTML = '<div class="alert alert-error">Invalid input (64 hex chars required)</div>';
    return;
  }

  el.innerHTML = '<div class="sub">Loading...</div>';

  try {
    const tx = await rpc('aztb_getTransactionByHash', [q]);
    if (tx) {
      el.innerHTML = `
        <div class="card">
          <h3>Transaction Found</h3>
          <div class="node-stats">
            <span class="node-stat-label">Hash</span>
            <span class="node-stat-value">${shortHash(tx.hash)}</span>
            ${tx.receipt ? `
              <span class="node-stat-label">Status</span>
              <span class="node-stat-value">
                <span class="badge ${tx.receipt.success ? 'badge-green' : 'badge-red'}">
                  ${tx.receipt.success ? 'Success' : 'Failed'}
                </span>
              </span>
              <span class="node-stat-label">Gas Used</span>
              <span class="node-stat-value">${parseInt(tx.receipt.gasUsed, 16)}</span>
              ${tx.receipt.error ? `
                <span class="node-stat-label">Error</span>
                <span class="node-stat-value" style="color:var(--red)">${tx.receipt.error}</span>
              ` : ''}
            ` : ''}
          </div>
          <div class="sub" style="margin-top:12px;word-break:break-all">
            Raw: ${tx.raw || '—'}
          </div>
        </div>
      `;
      return;
    }
  } catch {}

  try {
    const [balance, nonce] = await Promise.all([
      rpc('aztb_getBalance', [q]),
      rpc('aztb_getNonce', [q]),
    ]);
    el.innerHTML = `
      <div class="card">
        <h3>Account</h3>
        <div class="node-stats">
          <span class="node-stat-label">Address</span>
          <span class="node-stat-value">${shortHash(q)}</span>
          <span class="node-stat-label">Balance</span>
          <span class="node-stat-value">${formatNumber(balance)} AZTB</span>
          <span class="node-stat-label">Nonce</span>
          <span class="node-stat-value">${nonce}</span>
        </div>
      </div>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}

async function queryBatch() {
  const num = document.getElementById('batchQuery').value.trim();
  const el = document.getElementById('batchResult');

  if (!num || isNaN(num)) {
    el.innerHTML = '<div class="alert alert-error">Enter a valid batch number</div>';
    return;
  }

  el.innerHTML = '<div class="sub">Loading...</div>';

  try {
    const block = await rpc('aztb_getBlockByNumber', [parseInt(num)]);
    if (!block) {
      el.innerHTML = '<div class="alert alert-error">Batch not found</div>';
      return;
    }
    el.innerHTML = `
      <div class="card">
        <h3>Batch #${num}</h3>
        <div class="node-stats">
          <span class="node-stat-label">Hash</span>
          <span class="node-stat-value">${shortHash(block.hash)}</span>
          <span class="node-stat-label">State Root</span>
          <span class="node-stat-value">${shortHash(block.state_root)}</span>
          <span class="node-stat-label">TX Count</span>
          <span class="node-stat-value">${block.tx_count || block.transactions?.length || 0}</span>
          <span class="node-stat-label">Timestamp</span>
          <span class="node-stat-value">${block.timestamp ? new Date(block.timestamp).toLocaleString() : '—'}</span>
        </div>
      </div>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}
