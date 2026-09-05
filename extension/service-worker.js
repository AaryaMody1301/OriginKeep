const api = globalThis.browser ?? globalThis.chrome;
const HOST_NAME = "com.originkeep.host";
const CONTEXT_KEY = "originkeepRecentDownloadContexts";
const CONTEXT_TTL_MS = 2 * 60 * 1000;
let memoryContexts = [];

function captureKey(item) {
  return `${api.runtime.id}:${item.id}:${item.startTime || "unknown"}`;
}

function fileName(path) {
  return String(path || "").split(/[\\/]/).pop() || String(path || "download");
}

function browserName() {
  const agent = globalThis.navigator?.userAgent || "";
  if (agent.includes("Firefox")) return "Firefox";
  if (agent.includes("Edg/")) return "Edge";
  if (agent.includes("Chrome/")) return "Chrome";
  return "WebExtension";
}

async function readContexts() {
  const session = api.storage?.session;
  if (!session) return memoryContexts;
  try {
    const stored = await session.get(CONTEXT_KEY);
    return Array.isArray(stored?.[CONTEXT_KEY]) ? stored[CONTEXT_KEY] : [];
  } catch {
    return memoryContexts;
  }
}

async function writeContexts(contexts) {
  memoryContexts = contexts;
  const session = api.storage?.session;
  if (!session) return;
  try {
    await session.set({ [CONTEXT_KEY]: contexts });
  } catch {
    // Session storage is an optimization; never persist browsing context to disk as fallback.
  }
}

function recent(context) {
  const timestamp = Date.parse(context.capturedAt || "");
  return Number.isFinite(timestamp) && Date.now() - timestamp <= CONTEXT_TTL_MS;
}

async function rememberContext(context) {
  const contexts = (await readContexts()).filter(recent);
  contexts.unshift(context);
  await writeContexts(contexts.slice(0, 30));
}

async function contextFor(item) {
  const contexts = (await readContexts()).filter(recent);
  const urls = new Set([item.url, item.finalUrl].filter(Boolean));
  let index = contexts.findIndex((context) => urls.has(context.href));
  if (index < 0 && item.referrer) {
    index = contexts.findIndex((context) => context.pageUrl === item.referrer);
  }
  if (index < 0) {
    await writeContexts(contexts);
    return null;
  }
  const [match] = contexts.splice(index, 1);
  await writeContexts(contexts);
  return match;
}

async function sendCapture(item) {
  const context = await contextFor(item);
  const fileSize = Number.isFinite(item.fileSize) ? item.fileSize : -1;
  const totalBytes = Number.isFinite(item.totalBytes) ? item.totalBytes : -1;
  const payload = {
    captureKey: captureKey(item),
    browserDownloadId: item.id,
    originalUrl: item.url,
    finalUrl: item.finalUrl || null,
    referrer: item.referrer || context?.pageUrl || null,
    localPath: item.filename,
    fileName: fileName(item.filename),
    mimeType: item.mime || null,
    bytes: fileSize >= 0 ? fileSize : totalBytes >= 0 ? totalBytes : null,
    startedAt: item.startTime || null,
    completedAt: item.endTime || null,
    state: item.state,
    pageTitle: context?.pageTitle || null,
    pageUrl: context?.pageUrl || null,
    linkText: context?.linkText || null,
    contextText: context?.contextText || null,
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

api.runtime.onMessage.addListener((message) => {
  if (message?.type === "originkeep-download-context") {
    void rememberContext(message);
  }
});

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
