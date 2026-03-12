registerPage('validators', {
  init() {
    document.getElementById('page-validators').innerHTML = `
      <div class="section-title">Active Validators</div>
      <div class="card-grid" id="valOverview"></div>
      <table class="data-table" id="valTable">
        <thead>
          <tr>
            <th>Validator ID</th>
            <th>Effective Stake</th>
            <th>Status</th>
          </tr>
        </thead>
        <tbody id="valBody"></tbody>
      </table>
      <div class="section-title" style="margin-top:32px">Staking Lookup</div>
      <div class="form-row">
        <div class="form-group">
          <label class="form-label">Validator Address</label>
          <input class="form-input" id="stakeLookup" placeholder="64-char hex address">
        </div>
        <button class="btn btn-primary" onclick="lookupStake()">Query</button>
      </div>
      <div id="stakeResult"></div>
    `;
  },

  async refresh() {
    try {
      const validators = await rpc('aztb_getActiveValidators');
      renderValidatorOverview(validators);
      renderValidatorTable(validators);
    } catch (e) {
      document.getElementById('valBody').innerHTML =
        `<tr><td colspan="3" style="color:var(--red)">${e.message}</td></tr>`;
    }
  },
});

function renderValidatorOverview(validators) {
  const totalStake = validators.reduce((s, v) => s + Number(v.effective_stake || 0), 0);
  document.getElementById('valOverview').innerHTML = `
    <div class="card">
      <h3>Active Validators</h3>
      <div class="value">${validators.length}</div>
    </div>
    <div class="card">
      <h3>Total Effective Stake</h3>
      <div class="value">${formatNumber(totalStake)}</div>
      <div class="sub">AZTB</div>
    </div>
  `;
}

function renderValidatorTable(validators) {
  document.getElementById('valBody').innerHTML = validators.map(v => {
    const id = v.validator_id || v.address || v;
    const idStr = typeof id === 'string' ? id : JSON.stringify(id);
    const stake = v.effective_stake || v.stake || '—';
    return `
      <tr>
        <td>${shortHash(idStr)}</td>
        <td>${formatNumber(stake)}</td>
        <td><span class="badge badge-green">Active</span></td>
      </tr>
    `;
  }).join('');
}

async function lookupStake() {
  const addr = document.getElementById('stakeLookup').value.trim().replace(/^0x/, '');
  const el = document.getElementById('stakeResult');
  if (!/^[0-9a-fA-F]{64}$/.test(addr)) {
    el.innerHTML = '<div class="alert alert-error">Invalid address (64 hex chars required)</div>';
    return;
  }
  try {
    const [stake, delegation, unbonding] = await Promise.all([
      rpc('aztb_getValidatorStake', [addr]),
      rpc('aztb_getDelegation', [addr]),
      rpc('aztb_getUnbondingStatus', [addr]),
    ]);
    el.innerHTML = `
      <div class="alert alert-info" style="margin-top:12px">
        <strong>Stake:</strong> ${formatNumber(stake)} AZTB<br>
        <strong>Delegation:</strong> ${JSON.stringify(delegation)}<br>
        <strong>Unbonding:</strong> ${JSON.stringify(unbonding)}
      </div>
    `;
  } catch (e) {
    el.innerHTML = `<div class="alert alert-error">${e.message}</div>`;
  }
}
