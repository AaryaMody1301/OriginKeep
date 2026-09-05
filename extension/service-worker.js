const api = globalThis.browser ?? globalThis.chrome;
const HOST_NAME = "com.originkeep.host";
const CONTEXT_MAX_AGE_MS = 20_000;

function captureKey(item) {
  return `${api.runtime.id}:${item.id}:${item.startTime || "unknown"}`;
}

function fileName(path) {
  return path.split(/[\\/]/).pop() || path;
}

function browserName() {
  const ua = globalThis.navigator?.userAgent || "";
  if (/Firefox\//.test(ua)) return "Firefox";
  if (/Edg\//.test(ua)) return "Edge";
  if (/Chrome\//.test(ua)) return "Chrome";
  if (/Safari\//.test(ua)) return "Safari";
  return "Browser";
}

function comparableUrl(value) {
  try {
    const url = new URL(value);
    url.hash = "";
    return url.href;
  } catch {
    return value || "";
  }
}

async function recentClickContext(item) {
  try {
    const stored = await api.storage.local.get("recentDownloadContext");
    const context = stored.recentDownloadContext;
    if (!context || Date.now() - context.capturedAt > CONTEXT_MAX_AGE_MS) return null;
    const referrer = comparableUrl(item.referrer || "");
    const page = comparableUrl(context.pageUrl || "");
    const clicked = comparableUrl(context.href || "");
    const candidate = comparableUrl(item.url || item.finalUrl || "");
    if (referrer && page && referrer !== page && clicked && candidate !== clicked) return null;
    return context;
  } catch {
    return null;
  }
}

async function matchingTabContext(item) {
  if (!api.tabs?.query) return null;
  try {
    const tabs = await api.tabs.query({});
    const referrer = comparableUrl(item.referrer || "");
    const match = tabs.find((tab) => referrer && comparableUrl(tab.url || "") === referrer);
    if (!match) return null;
    return {
      pageTitle: match.title || null,
      pageUrl: match.url || null,
      linkText: null,
      contextText: null,
      contextSource: "referrer-tab-match",
    };
  } catch {
    return null;
  }
}

async function sendCapture(item) {
  const clicked = await recentClickContext(item);
  const tab = clicked ? null : await matchingTabContext(item);
  const context = clicked
    ? {
        pageTitle: clicked.pageTitle || null,
        pageUrl: clicked.pageUrl || null,
        linkText: clicked.linkText || null,
        contextText: clicked.contextText || null,
        contextSource: "enhanced-click",
      }
    : tab;

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
    browserName: browserName(),
    pageTitle: context?.pageTitle || null,
    pageUrl: context?.pageUrl || item.referrer || null,
    linkText: context?.linkText || null,
    contextText: context?.contextText || null,
    contextSource: context?.contextSource || (item.referrer ? "download-referrer" : "download-api"),
  };

  try {
    const response = await api.runtime.sendNativeMessage(HOST_NAME, payload);
    if (!response?.ok) console.warn("OriginKeep native host rejected capture", response);
  } catch (error) {
    console.warn("OriginKeep native host unavailable", error);
  }
}

async function sendFallbackContext(context) {
  try {
    await api.runtime.sendNativeMessage(HOST_NAME, {
      messageType: "contextObservation",
      browserName: browserName(),
      pageTitle: context.pageTitle,
      pageUrl: context.pageUrl,
      linkText: context.linkText,
      contextText: context.contextText,
      contextSource: "safari-fallback",
    });
  } catch (error) {
    console.warn("OriginKeep fallback native context bridge unavailable", error);
  }
}

api.runtime.onMessage.addListener((message, sender) => {
  if (message?.type !== "originkeep-download-context") return undefined;
  const context = {
    pageTitle: String(message.pageTitle || "").slice(0, 500) || null,
    pageUrl: String(message.pageUrl || sender.tab?.url || "").slice(0, 4096) || null,
    href: String(message.href || "").slice(0, 4096) || null,
    linkText: String(message.linkText || "").slice(0, 500) || null,
    contextText: String(message.contextText || "").slice(0, 2000) || null,
    capturedAt: Date.now(),
    tabId: sender.tab?.id ?? null,
  };
  void api.storage.local.set({ recentDownloadContext: context });
  if (!api.downloads?.onCreated) void sendFallbackContext(context);
  return undefined;
});

if (api.downloads?.onCreated && api.downloads?.onChanged && api.downloads?.search) {
  api.downloads.onCreated.addListener((item) => {
    void sendCapture(item);
  });

  api.downloads.onChanged.addListener((delta) => {
    if (!delta.state || !["complete", "interrupted"].includes(delta.state.current)) return;
    api.downloads.search({ id: delta.id }).then(([item]) => {
      if (item) void sendCapture(item);
    });
  });
} else {
  console.info("OriginKeep: automatic download capture is unavailable in this browser; use File Passport adoption instead.");
}
