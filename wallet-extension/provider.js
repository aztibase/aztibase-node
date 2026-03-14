(function () {
  if (window.aztibase) return;

  const pending = new Map();
  let reqId = 0;

  function request(method, params) {
    return new Promise((resolve, reject) => {
      const id = ++reqId;
      pending.set(id, { resolve, reject });
      window.postMessage({ type: "aztibase_request", id, method, params }, "*");
      setTimeout(() => {
        if (pending.has(id)) {
          pending.delete(id);
          reject(new Error("Request timeout"));
        }
      }, 30000);
    });
  }

  window.addEventListener("message", (event) => {
    if (event.source !== window) return;
    if (event.data?.type !== "aztibase_response") return;
    const { id, result, error } = event.data;
    const p = pending.get(id);
    if (!p) return;
    pending.delete(id);
    if (error) p.reject(new Error(error));
    else p.resolve(result);
  });

  window.aztibase = {
    isAztibase: true,
    connect: () => request("connect"),
    disconnect: () => request("disconnect"),
    getBalance: () => request("getBalance"),
    getNonce: () => request("getNonce"),
    getAddress: () => request("getAddress"),
    isConnected: () => request("isConnected"),
    signAndSendTransfer: (to, amount, gasPrice) =>
      request("signAndSendTransfer", { to, amount, gasPrice }),
    signAndSendStake: (amount, gasPrice) =>
      request("signAndSendStake", { amount, gasPrice }),
    signAndSendUnstake: (amount, gasPrice) =>
      request("signAndSendUnstake", { amount, gasPrice }),
    faucetDrip: () => request("faucetDrip"),
  };

  window.dispatchEvent(new Event("aztibase#initialized"));
})();
