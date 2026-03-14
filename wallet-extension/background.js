chrome.runtime.onInstalled.addListener(() => {
  chrome.storage.local.get("network", (result) => {
    if (!result.network) {
      chrome.storage.local.set({
        network: {
          name: "Testnet",
          rpc: "https://rpc.aztibase.com",
          chainId: "0xA27B",
        },
      });
    }
  });
});
