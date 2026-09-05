const runtime = globalThis.browser?.runtime ?? globalThis.chrome?.runtime;

function clamp(value, max) {
  const text = String(value ?? "").replace(/\s+/g, " ").trim();
  return text.length > max ? `${text.slice(0, max)}…` : text;
}

function nearestContext(anchor) {
  const container = anchor.closest("article, section, li, tr, p, figure, main, div");
  return clamp(container?.innerText || anchor.parentElement?.innerText || "", 1500);
}

function capture(event) {
  const target = event.target instanceof Element ? event.target : null;
  const anchor = target?.closest("a[href]");
  if (!anchor) return;

  let href;
  try {
    href = new URL(anchor.href, document.baseURI);
  } catch {
    return;
  }
  if (!['http:', 'https:'].includes(href.protocol)) return;

  void runtime.sendMessage({
    type: "originkeep-download-context",
    href: href.href,
    pageTitle: clamp(document.title, 500),
    pageUrl: location.href,
    linkText: clamp(anchor.innerText || anchor.getAttribute("aria-label") || anchor.textContent || "", 500),
    contextText: nearestContext(anchor),
    capturedAt: new Date().toISOString(),
  }).catch(() => undefined);
}

// Pointer-down runs before browser download creation for normal link activations.
document.addEventListener("pointerdown", capture, true);
document.addEventListener("auxclick", capture, true);
