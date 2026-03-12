const NODES = [
  { name: 'Node 1', rpc: 'http://127.0.0.1:9944' },
  { name: 'Node 2', rpc: 'http://127.0.0.1:9945' },
  { name: 'Node 3', rpc: 'http://127.0.0.1:9946' },
];

function getSelectedRpc() {
  return document.getElementById('nodeSelect').value;
}

async function rpc(method, params, rpcUrl) {
  const url = rpcUrl || getSelectedRpc();
  const resp = await fetch(url, {
    method: 'POST',
    headers: { 'Content-Type': 'application/json' },
    body: JSON.stringify({ jsonrpc: '2.0', method, params: params || [], id: Date.now() }),
  });
  const data = await resp.json();
  if (data.error) throw new Error(data.error.message);
  return data.result;
}

async function fetchMetrics(rpcUrl) {
  const url = (rpcUrl || getSelectedRpc()).replace(/\/$/, '') + '/metrics/json';
  const resp = await fetch(url);
  return resp.json();
}

async function fetchHealth(rpcUrl) {
  const url = (rpcUrl || getSelectedRpc()).replace(/\/$/, '') + '/health';
  const resp = await fetch(url);
  return resp.json();
}

function shortHash(h) {
  if (!h || h.length < 16) return h || '—';
  const clean = h.replace(/^0x/, '');
  return clean.slice(0, 8) + '...' + clean.slice(-8);
}

function formatNumber(n) {
  if (n == null) return '—';
  return Number(n).toLocaleString();
}
