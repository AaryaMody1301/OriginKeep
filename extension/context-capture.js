const api = globalThis.browser ?? globalThis.chrome;

function compactText(value, limit) {
  return String(value || "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, limit);
}

function contextFor(target) {
  const element = target instanceof Element ? target : target?.parentElement;
  if (!element) return null;
  const link = element.closest("a[href]");
  const actionable = link || element.closest("button, [role='button']") || element;
  const parent = actionable.closest("article, li, tr, section, p, div") || actionable.parentElement;
  return {
    type: "originkeep-download-context",
    pageTitle: compactText(document.title, 500),
    pageUrl: location.href.slice(0, 4096),
    href: link?.href?.slice(0, 4096) || null,
    linkText: compactText(actionable.innerText || actionable.textContent, 500) || null,
    contextText: compactText(parent?.innerText || parent?.textContent, 2000) || null,
  };
}

function capture(event) {
  const context = contextFor(event.target);
  if (!context) return;
  try {
    const response = api.runtime.sendMessage(context);
    if (response?.catch) response.catch(() => undefined);
  } catch {
    // Browsing must never be interrupted if the extension context is unavailable.
  }
}

document.addEventListener("pointerdown", capture, true);
