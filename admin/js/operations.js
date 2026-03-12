registerPage('operations', {
  init() {
    document.getElementById('page-operations').innerHTML = `
      <div class="section-title">Faucet</div>
      <div class="card" style="max-width:560px">
        <div class="form-row">
          <div class="form-group">
            <label class="form-label">Recipient Address</label>
            <input class="form-input" id="faucetAddr" placeholder="64-char hex address">
          </div>
          <button class="btn btn-primary" id="faucetBtn" onclick="adminFaucetDrip()">Drip 1M AZTB</button>
        </div>
        <div id="faucetResult"></div>
        <div class="sub" style="margin-top:8px">
          Faucet is node-local: balance only exists on the selected node.
          Cooldown: 60s per address.
        </div>
      </div>

      <div class="section-title" style="margin-top:32px">Chain Parameters</div>
      <div id="chainParamsContainer">
        <button class="btn btn-secondary" onclick="loadChainParams()">Load Parameters</button>
      </div>

      <div class="section-title" style="margin-top:32px">Governance Proposals</div>
      <div id="govContainer">
        <button class="btn btn-secondary" onclick="loadProposals()">Load Proposals</button>
      </div>

      <div class="section-title" style="margin-top:32px">Emission Info</div>
      <div id="emissionContainer">
        <button class="btn btn-secondary" onclick="loadEmission()">Load Emission Info</button>
      </div>
    `;
  },
});

async function adminFaucetDrip() {
  const addr = document.getElementById('faucetAddr').value.trim().replace(/^0x/, '');
  const el = document.getElementById('faucetResult');
  const btn = document.getElementById('faucetBtn');

  if (!/^[0-9a-fA-F]{64}$/.test(addr)) {
    el.innerHTML = '<div class="alert alert-error">Invalid address (64 hex chars required)</div>';
    return;
  }

  btn.disabled = true;
  btn.textContent = 'Sending...';
  try {
    const result = await rpc('aztb_faucetDrip', [addr]);
    const balance = result?.balance || result;
    el.innerHTML = `<div class="alert alert-success">
      Dripped 1,000,000 AZTB. New balance: ${formatNumber(balance)}
    </div>`;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  } finally {
    btn.textContent = 'Drip 1M AZTB';
    setTimeout(() => { btn.disabled = false; }, 60000);
  }
}

async function loadChainParams() {
  const el = document.getElementById('chainParamsContainer');
  try {
    const params = await rpc('aztb_listChainParams');
    if (!params || params.length === 0) {
      el.innerHTML = '<div class="sub">No parameters found</div>';
      return;
    }
    el.innerHTML = `
      <table class="data-table">
        <thead><tr><th>Parameter</th><th>Value</th></tr></thead>
        <tbody>
          ${params.map(p => `<tr><td>${p.key || p.name || p[0]}</td><td>${p.value || p[1] || '—'}</td></tr>`).join('')}
        </tbody>
      </table>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}

async function loadProposals() {
  const el = document.getElementById('govContainer');
  try {
    const proposals = await rpc('aztb_listProposals');
    if (!proposals || proposals.length === 0) {
      el.innerHTML = '<div class="sub">No active proposals</div>';
      return;
    }
    el.innerHTML = `
      <table class="data-table">
        <thead><tr><th>ID</th><th>Type</th><th>Status</th><th>Votes For</th><th>Votes Against</th></tr></thead>
        <tbody>
          ${proposals.map(p => `
            <tr>
              <td>${shortHash(p.id || p.proposal_id)}</td>
              <td>${p.proposal_type || p.kind || '—'}</td>
              <td><span class="badge ${p.executed ? 'badge-green' : 'badge-yellow'}">${p.executed ? 'Executed' : 'Active'}</span></td>
              <td>${formatNumber(p.votes_for)}</td>
              <td>${formatNumber(p.votes_against)}</td>
            </tr>
          `).join('')}
        </tbody>
      </table>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}

async function loadEmission() {
  const el = document.getElementById('emissionContainer');
  try {
    const info = await rpc('aztb_getEmissionInfo');
    el.innerHTML = `
      <div class="alert alert-info">
        <pre style="white-space:pre-wrap;font-size:12px">${JSON.stringify(info, null, 2)}</pre>
      </div>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}
