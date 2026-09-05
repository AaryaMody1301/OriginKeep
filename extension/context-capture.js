const runtime = (globalThis.browser ?? globalThis.chrome).runtime;

function compactText(value, limit) {
  return String(value || "")
    .replace(/\s+/g, " ")
    .trim()
    .slice(0, limit);
}

document.addEventListener(
  "click",
  (event) => {
    const target = event.target instanceof Element ? event.target.closest("a[href]") : null;
    if (!target) return;
    const container = target.closest("p, li, article, section, div") || target.parentElement;
    void runtime.sendMessage({
      type: "originkeep-download-context",
      pageUrl: location.href,
      pageTitle: document.title,
      linkText: compactText(target.textContent || target.getAttribute("aria-label"), 500),
      contextText: compactText(container?.textContent, 2000),
      timestamp: Date.now(),
    });
  },
  true,
);
