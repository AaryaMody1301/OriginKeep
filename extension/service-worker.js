const HOST_NAME = "com.originkeep.host";

function captureKey(item) {
  return `${chrome.runtime.id}:${item.id}:${item.startTime || "unknown"}`;
}

function fileName(path) {
  return path.split(/[\\/]/).pop() || path;
}

async function sendCapture(item) {
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
  };

  try {
    const response = await chrome.runtime.sendNativeMessage(HOST_NAME, payload);
    if (!response?.ok) {
      console.warn("OriginKeep native host rejected capture", response);
    }
  } catch (error) {
    console.warn("OriginKeep native host unavailable", error);
  }
}

chrome.downloads.onCreated.addListener((item) => {
  void sendCapture(item);
});

chrome.downloads.onChanged.addListener((delta) => {
  if (!delta.state || !["complete", "interrupted"].includes(delta.state.current)) {
    return;
  }

  chrome.downloads.search({ id: delta.id }).then(([item]) => {
    if (item) void sendCapture(item);
  });
});
