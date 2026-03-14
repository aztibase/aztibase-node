(function () {
  const script = document.createElement("script");
  script.src = chrome.runtime.getURL("provider.js");
  script.onload = () => script.remove();
  (document.head || document.documentElement).appendChild(script);

  window.addEventListener("message", async (event) => {
    if (event.source !== window) return;
    if (event.data?.type !== "aztibase_request") return;

    const { id, method, params } = event.data;
    try {
      const result = await chrome.runtime.sendMessage({
        type: "provider_request",
        method,
        params,
      });
      if (result?.error) {
        window.postMessage({ type: "aztibase_response", id, error: result.error }, "*");
      } else {
        window.postMessage({ type: "aztibase_response", id, result: result?.data }, "*");
      }
    } catch (e) {
      window.postMessage({ type: "aztibase_response", id, error: e.message }, "*");
    }
  });
})();
