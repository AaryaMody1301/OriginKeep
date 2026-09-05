const api = globalThis.browser ?? globalThis.chrome;
const SCRIPT_ID = "originkeep-enhanced-context";
const ORIGINS = ["http://*/*", "https://*/*"];

async function hasEnhancedAccess() {
  return api.permissions.contains({ origins: ORIGINS });
}

async function ensureScriptRegistered() {
  const existing = await api.scripting.getRegisteredContentScripts({ ids: [SCRIPT_ID] });
  if (existing.length) return;
  await api.scripting.registerContentScripts([
    {
      id: SCRIPT_ID,
      matches: ORIGINS,
      js: ["context-capture.js"],
      runAt: "document_start",
      allFrames: false,
      persistAcrossSessions: true,
    },
  ]);
}

async function disableScript() {
  const existing = await api.scripting.getRegisteredContentScripts({ ids: [SCRIPT_ID] });
  if (existing.length) await api.scripting.unregisterContentScripts({ ids: [SCRIPT_ID] });
  await api.permissions.remove({ origins: ORIGINS });
}

async function render() {
  const enabled = await hasEnhancedAccess();
  const status = document.getElementById("status");
  const toggle = document.getElementById("toggle");
  status.textContent = enabled ? "Enhanced context: enabled" : "Enhanced context: disabled";
  toggle.textContent = enabled ? "Disable enhanced context" : "Enable enhanced context";
  if (enabled) await ensureScriptRegistered();
}

document.getElementById("toggle").addEventListener("click", async () => {
  const enabled = await hasEnhancedAccess();
  if (enabled) {
    await disableScript();
  } else {
    const granted = await api.permissions.request({ origins: ORIGINS });
    if (granted) await ensureScriptRegistered();
  }
  await render();
});

render().catch((error) => {
  document.getElementById("status").textContent = `Enhanced context unavailable: ${error}`;
});
