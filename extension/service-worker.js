const api = globalThis.browser ?? globalThis.chrome;
const HOST_NAME = "com.originkeep.host";
const CONTEXT_KEY = "lastOriginKeepDownloadContext";
const CONTEXT_SCRIPT_ID = "originkeep-context-capture";
const OPTIONAL_ORIGINS = ["http://*/*", "https://*/*"];
const CONTEXT_MAX_AGE_MS = 2 * 60 * 1000;

function captureKey(item) {
  return `${api.runtime.id}:${item.id}:${item.startTime || "unknown"}`;
}

function fileName(path) {
  return path.split(/[\\/]/).pop() || path;
}

function browserName() {
  const value = navigator.userAgent;
  if (value.includes("Firefox/")) return "Firefox";
  if (value.includes("Edg/")) return "Edge";
  if (value.includes("Chrome/")) return "Chrome/Chromium";
  return "WebExtension";
}

async function optionalContextEnabled() {
  try {
    return await api.permissions.contains({ origins: OPTIONAL_ORIGINS });
  } catch {
    return false;
  }
}

async function ensureContextScript() {
  if (!(await optionalContextEnabled())) return false;
  try {
    const existing = await api.scripting.getRegisteredContentScripts({ ids: [CONTEXT_SCRIPT_ID] });
    if (existing.length === 0) {
      await api.scripting.registerContentScripts([
        {
          id: CONTEXT_SCRIPT_ID,
          js: ["context-capture.js"],
          matches: OPTIONAL_ORIGINS,
          runAt: "document_start",
          persistAcrossSessions: true,
        },
      ]);
    }
    return true;
  } catch (error) {
    console.warn("OriginKeep could not register optional context capture", error);
    return false;
  }
}

async function enableRichContext() {
  try {
    const granted = await api.permissions.request({
      permissions: ["tabs"],
      origins: OPTIONAL_ORIGINS,
    });
    if (!granted) {
      await api.action.setBadgeText({ text: "" });
      return;
    }
    await ensureContextScript();
    await api.action.setBadgeBackgroundColor({ color: "#2f855a" });
    await api.action.setBadgeText({ text: "ON" });
    setTimeout(() => void api.action.setBadgeText({ text: "" }), 2500);
  } catch (error) {
    console.warn("OriginKeep context permission request failed", error);
  }
}

api.action.onClicked.addListener(() => {
  void enableRichContext();
});

api.runtime.onMessage.addListener((message) => {
  if (message?.type !== "originkeep-download-context") return undefined;
  const context = {
    pageUrl: typeof message.pageUrl === "string" ? message.pageUrl.slice(0, 4096) : null,
    pageTitle: typeof message.pageTitle === "string" ? message.pageTitle.slice(0, 1000) : null,
    linkText: typeof message.linkText === "string" ? message.linkText.slice(0, 500) : null,
    contextText: typeof message.contextText === "string" ? message.contextText.slice(0, 2000) : null,
    timestamp: Number(message.timestamp) || Date.now(),
  };
  return api.storage.local.set({ [CONTEXT_KEY]: context });
});

function relatedContext(item, context) {
  if (!context || Date.now() - Number(context.timestamp || 0) > CONTEXT_MAX_AGE_MS) return null;
  if (!context.pageUrl) return null;
  if (item.referrer && item.referrer === context.pageUrl) return context;
  try {
    const source = new URL(item.url);
    const page = new URL(context.pageUrl);
    if (!item.referrer && source.origin === page.origin) return context;
  } catch {
    return null;
  }
  return null;
}

async function activeTabContext() {
  try {
    const [tab] = await api.tabs.query({ active: true, lastFocusedWindow: true });
    return tab
      ? {
          pageUrl: typeof tab.url === "string" ? tab.url.slice(0, 4096) : null,
          pageTitle: typeof tab.title === "string" ? tab.title.slice(0, 1000) : null,
        }
      : null;
  } catch {
    return null;
  }
}

async function sendCapture(item) {
  const stored = await api.storage.local.get(CONTEXT_KEY).catch(() => ({}));
  const clicked = relatedContext(item, stored?.[CONTEXT_KEY]);
  const active = clicked ? null : await activeTabContext();
  const payload = {
    captureKey: captureKey(item),
    browserDownloadId: item.id,
    originalUrl: item.url,
    finalUrl: item.finalUrl || null,
    referrer: item.referrer || null,
    localPath: item.filename,
    fileName: fileName(item.filename),
    mimeType: item.mime || null,
    bytes: item.fileSize >= 0 ? item.fileSize : item.totalBytes >= 0 ? item.totalBytes : null,
    startedAt: item.startTime || null,
    completedAt: item.endTime || null,
    state: item.state,
    pageUrl: clicked?.pageUrl || active?.pageUrl || item.referrer || null,
    pageTitle: clicked?.pageTitle || active?.pageTitle || null,
    linkText: clicked?.linkText || null,
    contextText: clicked?.contextText || null,
    browserName: browserName(),
  };

  try {
    const response = await api.runtime.sendNativeMessage(HOST_NAME, payload);
    if (!response?.ok) {
      console.warn("OriginKeep native host rejected capture", response);
    }
  } catch (error) {
    console.warn("OriginKeep native host unavailable", error);
  }
}

api.downloads.onCreated.addListener((item) => {
  void sendCapture(item);
});

api.downloads.onChanged.addListener((delta) => {
  if (!delta.state || !["complete", "interrupted"].includes(delta.state.current)) {
    return;
  }

  api.downloads.search({ id: delta.id }).then(([item]) => {
    if (item) void sendCapture(item);
  });
});

void ensureContextScript();
